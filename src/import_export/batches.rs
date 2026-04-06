// src/import_export/batches.rs
//! Batch import/export handlers

use actix_web::{web, HttpResponse};
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
use super::dto::BatchImportDto;
use super::{save_multipart_to_temp, preload_reagents, optimize_sqlite_for_bulk};

// ==========================================
// BATCHES IMPORT
// ==========================================

pub async fn import_batches_json(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<Vec<BatchImportDto>>,
) -> ApiResult<HttpResponse> {
    let count = import_batches_logic(&app_state.db_pool, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_with_message((), format!("Imported {} batches", count))))
}

pub async fn import_batches_excel(
    app_state: web::Data<Arc<AppState>>,
    payload: Multipart,
) -> ApiResult<HttpResponse> {
    let file_path = save_multipart_to_temp(payload).await?;
    let path_clone = file_path.clone();

    let batches_result = web::block(move || {
        let mut workbook: Xlsx<_> = open_workbook(&path_clone)
            .map_err(|e: XlsxError| e.to_string())?;
        let range = workbook.worksheet_range_at(0)
            .ok_or("Empty")?
            .map_err(|e| e.to_string())?;
        let mut list = Vec::new();
        let iter = RangeDeserializerBuilder::new().from_range(&range)
            .map_err(|e| e.to_string())?;
        for res in iter {
            match res {
                Ok(r) => list.push(r),
                Err(e) => log::warn!("Skipping row due to error: {}", e),
            }
        }
        Ok::<Vec<BatchImportDto>, String>(list)
    }).await.map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    match batches_result {
        Ok(batches) => {
            let count = import_batches_logic(&app_state.db_pool, batches).await?;
            let _ = fs::remove_file(file_path);
            Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_with_message((), format!("Imported {} batches", count))))
        }
        Err(e) => {
            let _ = fs::remove_file(file_path);
            Err(ApiError::BadRequest(e))
        }
    }
}

pub async fn import_batches(app_state: web::Data<Arc<AppState>>, body: web::Json<Vec<BatchImportDto>>) -> ApiResult<HttpResponse> {
    import_batches_json(app_state, body).await
}

async fn import_batches_logic(pool: &SqlitePool, batches: Vec<BatchImportDto>) -> ApiResult<usize> {
    let total_items = batches.len();
    let start_time = Instant::now();
    
    log::info!("🚀 Starting BULK batch import of {} items...", total_items);
    
    // Apply PRAGMA optimizations
    optimize_sqlite_for_bulk(pool).await?;
    
    // Preload reagents map
    let mut reagent_map = preload_reagents(pool).await?;
    
    // PHASE 1: Find and create missing reagents first
    let mut new_reagents: Vec<(String, String)> = Vec::new(); // (id, name)
    for b in &batches {
        let r_name_raw = b.reagent_name.trim();
        if r_name_raw.is_empty() { continue; }
        
        let r_name_key = r_name_raw.to_lowercase();
        if !reagent_map.contains_key(&r_name_key) {
            let new_id = Uuid::new_v4().to_string();
            reagent_map.insert(r_name_key, new_id.clone());
            new_reagents.push((new_id, r_name_raw.to_string()));
        }
    }
    
    // Bulk insert new reagents in single transaction
    if !new_reagents.is_empty() {
        log::info!("📦 Creating {} new reagents...", new_reagents.len());
        
        sqlx::query("PRAGMA synchronous = OFF").execute(pool).await
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
        
        let mut tx = pool.begin().await
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
        
        const REAGENT_CHUNK: usize = 200;
        for chunk in new_reagents.chunks(REAGENT_CHUNK) {
            let values_clause: String = chunk.iter()
                .map(|_| "(?,?,'active',datetime('now'),datetime('now'))")
                .collect::<Vec<_>>()
                .join(",");
            
            let sql = format!(
                "INSERT OR IGNORE INTO reagents (id, name, status, created_at, updated_at) VALUES {}",
                values_clause
            );
            
            let mut query = sqlx::query(&sql);
            for (id, name) in chunk {
                query = query.bind(id).bind(name);
            }
            
            query.execute(&mut *tx).await
                .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
        }
        
        tx.commit().await
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    }
    
    // PHASE 2: Prepare batches with resolved reagent IDs
    struct PrepBatch {
        id: String,
        reagent_id: String,
        batch_number: String,
        cat_number: Option<String>,
        supplier: Option<String>,
        quantity: f64,
        units: String,
        pack_size: Option<f64>,
        expiration_date: Option<String>,
        location: Option<String>,
        notes: Option<String>,
    }
    
    let mut prepared: Vec<PrepBatch> = Vec::with_capacity(total_items);
    for b in &batches {
        let r_name_raw = b.reagent_name.trim();
        if b.batch_number.trim().is_empty() || r_name_raw.is_empty() { continue; }
        
        let r_name_key = r_name_raw.to_lowercase();
        let r_id = reagent_map.get(&r_name_key).cloned().unwrap_or_default();
        if r_id.is_empty() { continue; }
        
        prepared.push(PrepBatch {
            id: Uuid::new_v4().to_string(),
            reagent_id: r_id,
            batch_number: b.batch_number.trim().to_string(),
            cat_number: b.cat_number.clone(),
            supplier: b.supplier.clone(),
            quantity: b.quantity,
            units: b.units.clone(),
            pack_size: b.pack_size,
            expiration_date: b.expiration_date.clone(),
            location: b.location.clone(),
            notes: b.notes.clone(),
        });
    }
    
    log::info!("📋 Prepared {} batches for bulk insert", prepared.len());
    
    // === PRAGMA BEFORE TRANSACTION ===
    sqlx::query("PRAGMA synchronous = OFF").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    // === SINGLE TRANSACTION FOR ENTIRE IMPORT ===
    let mut tx = pool.begin().await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    const BATCH_CHUNK: usize = 60;
    let mut processed = 0;
    let now = Utc::now().to_rfc3339();
    
    for chunk in prepared.chunks(BATCH_CHUNK) {
        let values_clause: String = chunk.iter()
            .map(|_| "(?,?,?,?,?,?,?,0.0,?,?,?,?,?,?,datetime('now'),'available')")
            .collect::<Vec<_>>()
            .join(",");
        
        let sql = format!(
            r#"INSERT INTO batches (
                id, reagent_id, batch_number, cat_number, supplier, 
                quantity, original_quantity, reserved_quantity,
                unit, pack_size, expiry_date, received_date,
                location, notes, updated_at, status
            ) VALUES {}
            ON CONFLICT(reagent_id, batch_number) DO UPDATE SET 
                quantity = quantity + excluded.quantity,
                original_quantity = original_quantity + excluded.original_quantity,
                pack_size = COALESCE(excluded.pack_size, pack_size),
                cat_number = COALESCE(excluded.cat_number, cat_number),
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
                .bind(&b.supplier)
                .bind(b.quantity)
                .bind(b.quantity)
                .bind(&b.units)
                .bind(&b.pack_size)
                .bind(&b.expiration_date)
                .bind(&now)
                .bind(&b.location)
                .bind(&b.notes);
        }
        
        query.execute(&mut *tx).await
            .map_err(|e| ApiError::InternalServerError(format!("Bulk batch insert failed: {}", e)))?;
        
        processed += chunk.len();
        if processed % 50000 == 0 {
            log::info!("📥 Batches: {}/{}", processed, prepared.len());
        }
    }
    
    // === SINGLE COMMIT ===
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
    log::info!("✅ BULK batch import completed in {:.2?}. {} items at {:.0} items/sec", elapsed, total_items, rate);
    
    Ok(total_items)
}

// ==========================================
// BATCHES EXPORT
// ==========================================

pub async fn export_batches(app_state: web::Data<Arc<AppState>>) -> ApiResult<HttpResponse> {
    let whitelist = FieldWhitelist::for_batches();
    let builder = SafeQueryBuilder::new("SELECT * FROM batches")
        .map_err(|e| ApiError::InternalServerError(e))?
        .with_whitelist(&whitelist);
    
    let (sql, _) = builder.build();
    let batches = sqlx::query_as::<_, crate::models::Batch>(&sql)
        .fetch_all(&app_state.db_pool)
        .await?;
    Ok(HttpResponse::Ok().json(batches))
}
