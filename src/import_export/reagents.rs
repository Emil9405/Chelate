// src/import_export/reagents.rs
//! Reagent import/export handlers

use actix_web::{web, HttpResponse, HttpRequest};
use actix_multipart::Multipart;
use sqlx::SqlitePool;
use std::sync::Arc;
use calamine::{Reader, open_workbook, RangeDeserializerBuilder, Xlsx, XlsxError};
use std::fs;
use uuid::Uuid;
use std::time::Instant;
use chrono::Utc;
use crate::{AppState, error::{ApiResult, ApiError}, handlers::ApiResponse};
use crate::query_builders::{SafeQueryBuilder, FieldWhitelist};
use crate::auth::get_current_user;
use super::dto::{ReagentImportDto, PreparedReagent, PreparedBatch, PreparedContainer, PreparedPlacement};
use super::{
    save_multipart_to_temp, preload_users, preload_reagents, preload_batches,
    preload_position_lookup, preload_container_max_sequences, parse_location_path,
    ensure_storage_locations, optimize_sqlite_for_bulk,
};

// ==========================================
// REAGENTS IMPORT
// ==========================================

pub async fn import_reagents_excel(
    app_state: web::Data<Arc<AppState>>,
    payload: Multipart,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&req)?;
    let current_user_id = claims.sub;

    let file_path = save_multipart_to_temp(payload).await?;
    let path_clone = file_path.clone();
    
    let reagents_result = web::block(move || {
        let mut workbook: Xlsx<_> = open_workbook(&path_clone)
            .map_err(|e: XlsxError| format!("Excel error: {}", e))?;
        
        let range = workbook.worksheet_range_at(0)
            .ok_or("Excel file is empty".to_string())?
            .map_err(|e| e.to_string())?;

        let mut reagents = Vec::new();
        let iter = RangeDeserializerBuilder::new().from_range(&range)
            .map_err(|e| format!("Header error: {}", e))?;

        let mut errors = Vec::new();

        for (i, result) in iter.enumerate() {
            match result {
                Ok(record) => reagents.push(record),
                Err(e) => {
                    let err_msg = format!("Row {}: {}", i + 2, e);
                    log::warn!("⚠️ Import Warning: {}", err_msg);
                    errors.push(err_msg);
                }
            }
        }
        
        if reagents.is_empty() {
            let error_details = errors.first().map(|s| s.as_str()).unwrap_or("Check column headers");
            return Err(format!("Failed to import. No valid rows. Error: {}", error_details));
        }

        Ok::<Vec<ReagentImportDto>, String>(reagents)
    }).await.map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    let reagents = match reagents_result {
        Ok(r) => r,
        Err(e) => {
            let _ = fs::remove_file(file_path);
            return Err(ApiError::BadRequest(e));
        }
    };

    let imported_count = import_reagents_logic(&app_state.db_pool, reagents, current_user_id).await;
    let _ = fs::remove_file(file_path);

    let count = imported_count?;
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_with_message((), format!("Imported {} items", count))))
}

pub async fn import_reagents_json(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<Vec<ReagentImportDto>>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&req)?;
    let count = import_reagents_logic(&app_state.db_pool, body.into_inner(), claims.sub).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_with_message((), format!("Imported {} reagents", count))))
}

pub async fn import_reagents(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<Vec<ReagentImportDto>>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    import_reagents_json(app_state, body, req).await
}

async fn import_reagents_logic(pool: &SqlitePool, reagents: Vec<ReagentImportDto>, current_user_id: String) -> ApiResult<usize> {
    let total_items = reagents.len();
    let start_time = Instant::now();
    
    log::info!("🚀 Starting BULK import of {} reagents...", total_items);
    
    // Apply PRAGMA optimizations
    optimize_sqlite_for_bulk(pool).await?;
    
    // Preload all lookups ONCE
    let users_map = preload_users(pool).await?;
    let mut reagents_map = preload_reagents(pool).await?;
    let mut batches_map = preload_batches(pool).await?;
    let mut position_lookup = preload_position_lookup(pool).await?;
    let container_seqs = preload_container_max_sequences(pool).await?;
    
    // Auto-create missing rooms/zones/positions from import data
    let all_locations: Vec<String> = reagents.iter()
        .filter_map(|r| r.location.clone())
        .filter(|s| !s.trim().is_empty())
        .collect();
    ensure_storage_locations(pool, &all_locations, &mut position_lookup).await?;
    
    // Running sequence counter per batch_id (initialized from DB)
    let mut next_seq: std::collections::HashMap<String, i64> = container_seqs
        .into_iter()
        .map(|(bid, max)| (bid, max + 1))
        .collect();
    
    log::info!(
        "📦 Preloaded {} users, {} reagents, {} batches, {} positions",
        users_map.len(), reagents_map.len(), batches_map.len(), position_lookup.len()
    );
    
    // =============================================
    // PHASE 1: Prepare all data (no DB calls)
    // =============================================
    let mut prepared_reagents: Vec<PreparedReagent> = Vec::with_capacity(total_items);
    let mut prepared_batches: Vec<PreparedBatch> = Vec::new();
    let mut prepared_containers: Vec<PreparedContainer> = Vec::new();
    let mut prepared_placements: Vec<PreparedPlacement> = Vec::new();
    let mut seen_reagent_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    
    for r in &reagents {
        let name = r.name.trim();
        if name.is_empty() { continue; }
        
        let name_key = name.to_lowercase();
        
        let owner_id = r.owner.as_ref()
            .and_then(|o| users_map.get(&o.trim().to_lowercase()))
            .cloned()
            .unwrap_or_else(|| current_user_id.clone());
        
        let created_at = r.added_at.clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        
        let reagent_id = reagents_map
            .entry(name_key)
            .or_insert_with(|| Uuid::new_v4().to_string())
            .clone();
        
        // Only prepare each reagent once (same name → same id)
        if seen_reagent_ids.insert(reagent_id.clone()) {
            prepared_reagents.push(PreparedReagent {
                id: reagent_id.clone(),
                name: name.to_string(),
                formula: r.formula.clone(),
                cas_number: r.cas_number.clone(),
                manufacturer: r.manufacturer.clone(),
                description: r.description.clone(),
                storage: r.storage.clone(),
                appearance: r.appearance.clone(),
                hazard_pictograms: r.hazard_pictograms.clone(),
                molecular_weight: r.molecular_weight,
                owner_id: owner_id.clone(),
                created_at,
            });
        }
        
        // Prepare batch if batch data present
        if let (Some(batch_num), Some(qty), Some(unit)) = (&r.batch_number, r.quantity, &r.units) {
            if batch_num.trim().is_empty() || qty <= 0.0 { continue; }
            
            let batch_number_trimmed = batch_num.trim().to_string();
            let batch_key = (reagent_id.clone(), batch_number_trimmed.to_lowercase());
            
            // Check if batch already exists (in DB or earlier in this import)
            let is_new = !batches_map.contains_key(&batch_key);
            let batch_id = batches_map
                .entry(batch_key)
                .or_insert_with(|| Uuid::new_v4().to_string())
                .clone();
            
            // Resolve location path → position_id
            let position_id = r.location.as_ref()
                .and_then(|loc| parse_location_path(loc))
                .and_then(|key| position_lookup.get(&key).cloned());
            
            let container_count = r.container_count.unwrap_or(1).max(1);
            
            prepared_batches.push(PreparedBatch {
                id: batch_id.clone(),
                reagent_id: reagent_id.clone(),
                batch_number: batch_number_trimmed,
                cat_number: r.catalog_number.clone(),
                manufacturer: r.manufacturer.clone(),
                quantity: qty,
                unit: unit.clone(),
                pack_size: r.pack_size,
                expiry_date: r.expiry_date.clone(),
                location: r.location.clone(),
                owner_id: owner_id.clone(),
                is_new,
                container_count,
                position_id: position_id.clone(),
            });
            
            // ============================================================
            // FIX: ALWAYS create containers for EVERY row, not just new batches.
            // Each row represents a physical presence at a specific location.
            // ============================================================
            let qty_per_container = qty / container_count as f64;
            let seq_start = next_seq.entry(batch_id.clone()).or_insert(1);
            
            for _i in 0..container_count {
                let container_id = Uuid::new_v4().to_string();
                let seq = *seq_start;
                *seq_start += 1;
                
                prepared_containers.push(PreparedContainer {
                    id: container_id.clone(),
                    batch_id: batch_id.clone(),
                    sequence_number: seq,
                    quantity: qty_per_container,
                    original_quantity: qty_per_container,
                });
                
                // Place container if position resolved
                if let Some(ref pos_id) = position_id {
                    prepared_placements.push(PreparedPlacement {
                        id: Uuid::new_v4().to_string(),
                        container_id,
                        position_id: pos_id.clone(),
                        placed_by: owner_id.clone(),
                    });
                }
            }
        }
    }
    
    log::info!(
        "📋 Prepared {} reagents, {} batches ({} new), {} containers, {} placements",
        prepared_reagents.len(),
        prepared_batches.len(),
        prepared_batches.iter().filter(|b| b.is_new).count(),
        prepared_containers.len(),
        prepared_placements.len(),
    );
    
    // =============================================
    // === PRAGMA BEFORE TRANSACTION ===
    // =============================================
    sqlx::query("PRAGMA synchronous = OFF").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    let mut tx = pool.begin().await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    // =============================================
    // PHASE 2: Bulk insert reagents
    // =============================================
    const REAGENT_CHUNK_SIZE: usize = 70;
    let mut processed_reagents = 0;
    
    for chunk in prepared_reagents.chunks(REAGENT_CHUNK_SIZE) {
        let values_clause: String = chunk.iter()
            .map(|_| "(?,?,?,?,?,?,?,?,?,?,?,?,?,datetime('now'))")
            .collect::<Vec<_>>()
            .join(",");
        
        let sql = format!(
            r#"INSERT INTO reagents (
                id, name, formula, cas_number, manufacturer, description,
                storage_conditions, appearance, hazard_pictograms, status, 
                molecular_weight, created_by, created_at, updated_at
            ) VALUES {}
            ON CONFLICT(name) DO UPDATE SET 
                formula = COALESCE(excluded.formula, formula),
                cas_number = COALESCE(excluded.cas_number, cas_number),
                manufacturer = COALESCE(excluded.manufacturer, manufacturer),
                description = COALESCE(excluded.description, description),
                storage_conditions = COALESCE(excluded.storage_conditions, storage_conditions),
                appearance = COALESCE(excluded.appearance, appearance),
                hazard_pictograms = COALESCE(excluded.hazard_pictograms, hazard_pictograms),
                molecular_weight = COALESCE(excluded.molecular_weight, molecular_weight),
                deleted_at = NULL,
                updated_at = datetime('now')"#,
            values_clause
        );
        
        let mut query = sqlx::query(&sql);
        for r in chunk {
            query = query
                .bind(&r.id)
                .bind(&r.name)
                .bind(&r.formula)
                .bind(&r.cas_number)
                .bind(None::<String>) // manufacturer now lives on batch, not reagent
                .bind(&r.description)
                .bind(&r.storage)
                .bind(&r.appearance)
                .bind(&r.hazard_pictograms)
                .bind("active")
                .bind(&r.molecular_weight)
                .bind(&r.owner_id)
                .bind(&r.created_at);
        }
        
        query.execute(&mut *tx).await
            .map_err(|e| ApiError::InternalServerError(format!("Bulk reagent insert failed: {}", e)))?;
        
        processed_reagents += chunk.len();
        if processed_reagents % 50000 == 0 {
            log::info!("📥 Reagents: {}/{}", processed_reagents, prepared_reagents.len());
        }
    }
    log::info!("📥 Reagents complete: {}", processed_reagents);
    
    // =============================================
    // PHASE 3: Bulk insert batches
    // =============================================
    const BATCH_CHUNK_SIZE: usize = 60;
    let mut processed_batches = 0;
    let now = Utc::now().to_rfc3339();
    
    for chunk in prepared_batches.chunks(BATCH_CHUNK_SIZE) {
        let values_clause: String = chunk.iter()
            .map(|_| "(?,?,?,?,?,?,?,0.0,?,?,?,?,'available',?,?,?,?,?)")
            .collect::<Vec<_>>()
            .join(",");
        
        let sql = format!(
            r#"INSERT INTO batches (
                id, reagent_id, batch_number, cat_number, manufacturer, quantity, original_quantity,
                reserved_quantity, unit, pack_size, expiry_date, location, status,
                received_date, created_at, updated_at, created_by, updated_by
            ) VALUES {}
            ON CONFLICT(reagent_id, batch_number) DO UPDATE SET 
                quantity = quantity + excluded.quantity,
                original_quantity = original_quantity + excluded.original_quantity,
                pack_size = COALESCE(excluded.pack_size, pack_size),
                cat_number = COALESCE(excluded.cat_number, cat_number),
                manufacturer = COALESCE(excluded.manufacturer, manufacturer),
                deleted_at = NULL"#,
            values_clause
        );
        
        let mut query = sqlx::query(&sql);
        for b in chunk {
            query = query
                .bind(&b.id)
                .bind(&b.reagent_id)
                .bind(&b.batch_number)
                .bind(&b.cat_number)
                .bind(&b.manufacturer)
                .bind(b.quantity)
                .bind(b.quantity)
                .bind(&b.unit)
                .bind(&b.pack_size)
                .bind(&b.expiry_date)
                .bind(&b.location)
                .bind(&now)
                .bind(&now)
                .bind(&now)
                .bind(&b.owner_id)
                .bind(&b.owner_id);
        }
        
        query.execute(&mut *tx).await
            .map_err(|e| ApiError::InternalServerError(format!("Bulk batch insert failed: {}", e)))?;
        
        processed_batches += chunk.len();
        if processed_batches % 50000 == 0 {
            log::info!("📥 Batches: {}/{}", processed_batches, prepared_batches.len());
        }
    }
    log::info!("📥 Batches complete: {}", processed_batches);
    
    // =============================================
    // PHASE 4: Bulk insert containers
    // =============================================
    const CONTAINER_CHUNK_SIZE: usize = 80;
    let mut processed_containers = 0;
    
    for chunk in prepared_containers.chunks(CONTAINER_CHUNK_SIZE) {
        let values_clause: String = chunk.iter()
            .map(|_| "(?,?,?,?,?,0,'full',NULL,datetime('now'),datetime('now'))")
            .collect::<Vec<_>>()
            .join(",");
        
        let sql = format!(
            r#"INSERT INTO batch_containers (
                id, batch_id, sequence_number, quantity, original_quantity,
                is_opened, status, notes, created_at, updated_at
            ) VALUES {}"#,
            values_clause
        );
        
        let mut query = sqlx::query(&sql);
        for c in chunk {
            query = query
                .bind(&c.id)
                .bind(&c.batch_id)
                .bind(c.sequence_number)
                .bind(c.quantity)
                .bind(c.original_quantity);
        }
        
        query.execute(&mut *tx).await
            .map_err(|e| ApiError::InternalServerError(format!("Bulk container insert failed: {}", e)))?;
        
        processed_containers += chunk.len();
        if processed_containers % 50000 == 0 {
            log::info!("📥 Containers: {}/{}", processed_containers, prepared_containers.len());
        }
    }
    log::info!("📥 Containers complete: {}", processed_containers);
    
    // =============================================
    // PHASE 5: Bulk insert placements (resolved locations only)
    // =============================================
    const PLACEMENT_CHUNK_SIZE: usize = 100;
    let mut processed_placements = 0;
    
    for chunk in prepared_placements.chunks(PLACEMENT_CHUNK_SIZE) {
        let values_clause: String = chunk.iter()
            .map(|_| "(?,?,?,?,datetime('now'),NULL)")
            .collect::<Vec<_>>()
            .join(",");
        
        let sql = format!(
            r#"INSERT INTO batch_placements (
                id, container_id, position_id, placed_by, placed_at, notes
            ) VALUES {}"#,
            values_clause
        );
        
        let mut query = sqlx::query(&sql);
        for p in chunk {
            query = query
                .bind(&p.id)
                .bind(&p.container_id)
                .bind(&p.position_id)
                .bind(&p.placed_by);
        }
        
        query.execute(&mut *tx).await
            .map_err(|e| ApiError::InternalServerError(format!("Bulk placement insert failed: {}", e)))?;
        
        processed_placements += chunk.len();
        if processed_placements % 50000 == 0 {
            log::info!("📥 Placements: {}/{}", processed_placements, prepared_placements.len());
        }
    }
    log::info!("📥 Placements complete: {}", processed_placements);
    
    // =============================================
    // === SINGLE COMMIT AT THE END ===
    // =============================================
    tx.commit().await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    // Restore safe mode
    sqlx::query("PRAGMA synchronous = NORMAL").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    let elapsed = start_time.elapsed();
    let rate = if elapsed.as_secs_f64() > 0.0 { 
        total_items as f64 / elapsed.as_secs_f64() 
    } else { 
        0.0 
    };
    
    log::info!(
        "✅ BULK import completed in {:.2?}. {} reagents, {} batches, {} containers, {} placements at {:.0} items/sec",
        elapsed, processed_reagents, processed_batches, processed_containers, processed_placements, rate
    );

    Ok(total_items)
}

// ==========================================
// REAGENTS EXPORT
// ==========================================

pub async fn export_reagents(app_state: web::Data<Arc<AppState>>) -> ApiResult<HttpResponse> {
    let whitelist = FieldWhitelist::for_reagents();
    let builder = SafeQueryBuilder::new("SELECT * FROM reagents WHERE deleted_at IS NULL")
        .map_err(|e| ApiError::InternalServerError(e))?
        .with_whitelist(&whitelist);
    
    let (sql, _) = builder.build();
    
    let reagents = sqlx::query_as::<_, crate::models::Reagent>(&sql)
        .fetch_all(&app_state.db_pool)
        .await?;
    
    Ok(HttpResponse::Ok().json(reagents))
}
