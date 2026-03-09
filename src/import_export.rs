// src/import_export.rs
//! Optimized import/export with query_builders integration
//! OPTIMIZATIONS v2 (BULK INSERT):
//! - Preload users map (avoid N queries for owner lookup)
//! - Preload reagents map (avoid SELECT after INSERT)
//! - BULK INSERT: 60-80 rows per query instead of 1 (10-50x faster)
//! - PRAGMA optimizations for SQLite (WAL, cache, mmap)
//! - Two-phase: prepare all data first, then bulk write
//! - FIX: Correct date parsing from Excel (avoids 1970 issue)
//! Expected: 5,000-15,000 items/sec (vs 350 items/sec)

use actix_web::{web, HttpResponse, HttpRequest};
use actix_multipart::Multipart;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize, Deserializer}; // Added Deserializer
use sqlx::SqlitePool;
use std::sync::Arc;
use calamine::{Reader, open_workbook, RangeDeserializerBuilder, Xlsx, XlsxError};
use std::path::PathBuf;
use std::fs;
use std::io::Write;
use uuid::Uuid;
use std::time::Instant;
use std::collections::HashMap;
use sqlx::Row;
use chrono::{Utc, NaiveDate, NaiveDateTime, DateTime}; // Added Chrono types
use crate::{AppState, error::{ApiResult, ApiError}, handlers::ApiResponse};
use crate::query_builders::{SafeQueryBuilder, FieldWhitelist};
use crate::auth::get_current_user;

// ==========================================
// CUSTOM DESERIALIZER (FIX FOR DATE ISSUE)
// ==========================================

/// Десериализует дату из разных форматов (Excel float, String DD.MM.YYYY, ISO) в ISO String
fn deserialize_flexible_date<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DateValue {
        Float(f64),
        Int(i64),
        String(String),
    }

    let value: Option<DateValue> = Option::deserialize(deserializer)?;

    match value {
        Some(DateValue::Float(f)) => {
            // Excel stores dates as days since Dec 30, 1899.
            // Unix epoch (1970-01-01) is 25569 days after Excel epoch.
            // Formula: (ExcelDays - 25569) * 86400 seconds
            let seconds = (f - 25569.0) * 86400.0;
            // Handle negative or invalid timestamps gracefully
            if seconds >= 0.0 {
                if let Some(dt) = DateTime::from_timestamp(seconds as i64, 0) {
                    return Ok(Some(dt.naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string()));
                }
            }
            Ok(None)
        },
        Some(DateValue::Int(i)) => {
            // Same logic if Excel passes it as integer
            let seconds = (i as f64 - 25569.0) * 86400.0;
            if seconds >= 0.0 {
                if let Some(dt) = DateTime::from_timestamp(seconds as i64, 0) {
                    return Ok(Some(dt.naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string()));
                }
            }
            Ok(None)
        },
        Some(DateValue::String(s)) => {
            let s = s.trim();
            if s.is_empty() {
                return Ok(None);
            }
            // Try different formats: DD.MM.YYYY, YYYY-MM-DD, DD/MM/YYYY
            let formats = [
                "%Y-%m-%d",
                "%d.%m.%Y",
                "%d/%m/%Y",
                "%Y/%m/%d",
                "%Y-%m-%dT%H:%M:%S",
                "%Y-%m-%dT%H:%M:%SZ",
            ];

            for fmt in formats {
                if let Ok(dt) = NaiveDate::parse_from_str(s, fmt) {
                    return Ok(Some(dt.format("%Y-%m-%dT00:00:00").to_string()));
                }
                // Also try parsing as DateTime for ISO strings with time
                if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
                    return Ok(Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string()));
                }
            }
            
            // If strictly preserving original string if parse fails (fallback)
            Ok(Some(s.to_string()))
        },
        None => Ok(None),
    }
}

// ==========================================
// MODELS (DTOs)
// ==========================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ReagentImportDto {
    #[serde(alias = "Name", alias = "reagent_name", alias = "Название")]
    pub name: String,
    
    #[serde(alias = "Formula", alias = "chemical_formula", alias = "Формула")]
    pub formula: Option<String>,
    
    #[serde(alias = "CAS", alias = "cas", alias = "cas_number", alias = "CAS Number")]
    pub cas_number: Option<String>,
    
    #[serde(alias = "Molecular weight", alias = "MW", alias = "Molecular Weight", alias = "Mol. Weight")]
    pub molecular_weight: Option<f64>,
    
    #[serde(alias = "Manufacturer", alias = "manufacturer", alias = "Производитель")]
    pub manufacturer: Option<String>,
    
    #[serde(alias = "Description", alias = "description", alias = "Описание")]
    pub description: Option<String>,
    
    #[serde(alias = "Catalog Number", alias = "cat_number", alias = "Catalogue No", alias = "Catalog #")]
    pub catalog_number: Option<String>,

    #[serde(alias = "Storage_cond", alias = "Storage", alias = "Storage conditions", alias = "Safety")]
    pub storage: Option<String>, 
    
    #[serde(alias = "Appearance", alias = "Color")]
    pub appearance: Option<String>,

    #[serde(alias = "Added by", alias = "User", alias = "Owner", alias = "Владелец")]
    pub owner: Option<String>,

    #[serde(alias = "Added at", alias = "Date added", alias = "created_at")]
    pub added_at: Option<String>,

    // Batch fields
    #[serde(alias = "Lot number", alias = "Lot Number", alias = "batch_number", alias = "Партия")]
    pub batch_number: Option<String>,
    
    #[serde(alias = "Pack_size", alias = "Pack size", alias = "Pack Size", alias = "PackSize", alias = "pack_size", alias = "Unit Size", alias = "UnitSize")]
    pub pack_size: Option<f64>,
    
    #[serde(alias = "Quantity", alias = "quantity", alias = "Количество")]
    pub quantity: Option<f64>,
    
    #[serde(alias = "Units", alias = "units", alias = "Unit", alias = "unit", alias = "Единицы",)]
    pub units: Option<String>,
    
    #[serde(alias = "Expiry Date", alias = "expiry_date", alias = "expiration_date", alias = "Срок годности")]
    #[serde(default, deserialize_with = "deserialize_flexible_date")] 
    pub expiry_date: Option<String>,
    
    #[serde(alias = "Place", alias = "Location", alias = "location", alias = "Место хранения")]
    pub location: Option<String>,

    #[serde(alias = "Hazard", alias = "hazard_pictograms", alias = "GHS", alias = "Pictograms", alias = "Hazard Pictograms")]
    pub hazard_pictograms: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchImportDto {
    #[serde(alias = "Reagent Name", alias = "reagent_name")]
    pub reagent_name: String,
    #[serde(alias = "Batch Number", alias = "batch_number", alias = "Lot Number", alias = "Lot number")]
    pub batch_number: String,
    #[serde(alias = "Catalog Number", alias = "cat_number", alias = "Catalogue No", alias = "Catalog #")]
    pub cat_number: Option<String>,
    pub supplier: Option<String>,
    #[serde(alias = "quantity", alias = "Quantity", alias = "Amount")]
    pub quantity: f64, 
    #[serde(alias = "unit", alias = "Unit", alias = "units", alias = "Units", alias = "Umits")]
    pub units: String,
    #[serde(alias = "Pack_size", alias = "Pack size", alias = "Pack Size", alias = "PackSize", alias = "pack_size", alias = "Unit Size")]
    pub pack_size: Option<f64>,
    
    #[serde(default, deserialize_with = "deserialize_flexible_date")] // <--- ПРИМЕНЕНО ЗДЕСЬ
    pub expiration_date: Option<String>,
    
    pub location: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EquipmentImportDto {
    pub name: String,
    #[serde(alias = "type")]
    pub equipment_type: String,
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub quantity: Option<i32>,
    pub unit: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
}

// ==========================================
// HELPERS
// ==========================================

async fn save_multipart_to_temp(mut payload: Multipart) -> ApiResult<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let file_name = format!("lims_import_{}.xlsx", Uuid::new_v4());
    let file_path = temp_dir.join(file_name);

    let mut f = fs::File::create(&file_path)
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create temp file: {}", e)))?;

    while let Ok(Some(mut field)) = payload.try_next().await {
        if field.content_disposition().get_filename().is_some() {
            while let Some(chunk) = field.next().await {
                let data = chunk.map_err(|e| ApiError::BadRequest(e.to_string()))?;
                f.write_all(&data)
                    .map_err(|e| ApiError::InternalServerError(format!("Failed to write to temp file: {}", e)))?;
            }
            return Ok(file_path);
        }
    }
    Err(ApiError::BadRequest("No file found in request".to_string()))
}

/// Preload all users into HashMap (username lowercase -> id)
async fn preload_users(pool: &SqlitePool) -> ApiResult<HashMap<String, String>> {
    let rows = sqlx::query("SELECT username, id FROM users")
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to preload users: {}", e)))?;
    
    let map: HashMap<String, String> = rows
        .into_iter()
        .map(|row| {
            (
                row.get::<String, _>("username").trim().to_lowercase(),
                row.get::<String, _>("id")
            )
        })
        .collect();
    
    Ok(map)
}

/// Preload all reagents into HashMap (name lowercase -> id)
async fn preload_reagents(pool: &SqlitePool) -> ApiResult<HashMap<String, String>> {
    let rows = sqlx::query("SELECT name, id FROM reagents WHERE deleted_at IS NULL")
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to preload reagents: {}", e)))?;
    
    let map: HashMap<String, String> = rows
        .into_iter()
        .map(|row| {
            (
                row.get::<String, _>("name").trim().to_lowercase(), 
                row.get::<String, _>("id")
            )
        })
        .collect();
    
    Ok(map)
}

// ==========================================
// PRAGMA OPTIMIZATION (for bulk imports)
// ==========================================

/// Apply SQLite PRAGMA settings for faster bulk imports
async fn optimize_sqlite_for_bulk(pool: &SqlitePool) -> ApiResult<()> {
    // WAL mode for better concurrent writes
    sqlx::query("PRAGMA journal_mode = WAL").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    // Don't wait for disk sync on every write  
    sqlx::query("PRAGMA synchronous = NORMAL").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    // 64MB cache
    sqlx::query("PRAGMA cache_size = -64000").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    // Keep temp tables in memory
    sqlx::query("PRAGMA temp_store = MEMORY").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    // 256MB mmap for faster reads
    sqlx::query("PRAGMA mmap_size = 268435456").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    Ok(())
}

// ==========================================
// PREPARED STRUCTS FOR BULK INSERT
// ==========================================

struct PreparedReagent {
    id: String,
    name: String,
    formula: Option<String>,
    cas_number: Option<String>,
    manufacturer: Option<String>,
    description: Option<String>,
    storage: Option<String>,
    appearance: Option<String>,
    hazard_pictograms: Option<String>,
    molecular_weight: Option<f64>,
    owner_id: String,
    created_at: String,
}

struct PreparedBatch {
    id: String,
    reagent_id: String,
    batch_number: String,
    cat_number: Option<String>,
    quantity: f64,
    unit: String,
    pack_size: Option<f64>,
    expiry_date: Option<String>,
    location: Option<String>,
    owner_id: String,
}

// ==========================================
// REAGENTS IMPORT (OPTIMIZED)
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
    
    // Preload all users and reagents ONCE
    let users_map = preload_users(pool).await?;
    let mut reagents_map = preload_reagents(pool).await?;
    
    log::info!("📦 Preloaded {} users, {} reagents", users_map.len(), reagents_map.len());
    
    // PHASE 1: Prepare all data (no DB calls)
    let mut prepared_reagents: Vec<PreparedReagent> = Vec::with_capacity(total_items);
    let mut prepared_batches: Vec<PreparedBatch> = Vec::new();
    
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
        
        // Prepare batch if present
        if let (Some(batch_num), Some(qty), Some(unit)) = (&r.batch_number, r.quantity, &r.units) {
            if !batch_num.trim().is_empty() && qty > 0.0 {
                prepared_batches.push(PreparedBatch {
                    id: Uuid::new_v4().to_string(),
                    reagent_id: reagent_id.clone(),
                    batch_number: batch_num.trim().to_string(),
                    cat_number: r.catalog_number.clone(),
                    quantity: qty,
                    unit: unit.clone(),
                    pack_size: r.pack_size,
                    expiry_date: r.expiry_date.clone(),
                    location: r.location.clone(),
                    owner_id: owner_id,
                });
            }
        }
    }
    
    log::info!("📋 Prepared {} reagents, {} batches for bulk insert", prepared_reagents.len(), prepared_batches.len());
    
    // === PRAGMA BEFORE TRANSACTION ===
    sqlx::query("PRAGMA synchronous = OFF").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    // === SINGLE TRANSACTION FOR ENTIRE IMPORT ===
    let mut tx = pool.begin().await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    // PHASE 2: Bulk insert reagents
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
                .bind(&r.manufacturer)
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
    
    // PHASE 3: Bulk insert batches
    const BATCH_CHUNK_SIZE: usize = 60;
    let mut processed_batches = 0;
    let now = Utc::now().to_rfc3339();
    
    for chunk in prepared_batches.chunks(BATCH_CHUNK_SIZE) {
        let values_clause: String = chunk.iter()
            .map(|_| "(?,?,?,?,?,?,0.0,?,?,?,?,'available',?,?,?,?,?)")
            .collect::<Vec<_>>()
            .join(",");
        
        let sql = format!(
            r#"INSERT INTO batches (
                id, reagent_id, batch_number, cat_number, quantity, original_quantity,
                reserved_quantity, unit, pack_size, expiry_date, location, status,
                received_date, created_at, updated_at, created_by, updated_by
            ) VALUES {}
            ON CONFLICT(reagent_id, batch_number) DO UPDATE SET 
                quantity = quantity + excluded.quantity,
                original_quantity = original_quantity + excluded.original_quantity,
                pack_size = COALESCE(excluded.pack_size, pack_size),
                cat_number = COALESCE(excluded.cat_number, cat_number)"#,
            values_clause
        );
        
        let mut query = sqlx::query(&sql);
        for b in chunk {
            query = query
                .bind(&b.id)
                .bind(&b.reagent_id)
                .bind(&b.batch_number)
                .bind(&b.cat_number)
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
    
    // === SINGLE COMMIT AT THE END ===
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
    
    log::info!("✅ BULK import completed in {:.2?}. {} items at {:.0} items/sec", elapsed, total_items, rate);

    Ok(total_items)
}

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

// ==========================================
// BATCHES IMPORT (OPTIMIZED)
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
                cat_number = COALESCE(excluded.cat_number, cat_number)"#,
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

// ==========================================
// EQUIPMENT IMPORT (OPTIMIZED)
// ==========================================

pub async fn import_equipment_json(app_state: web::Data<Arc<AppState>>, body: web::Json<Vec<EquipmentImportDto>>) -> ApiResult<HttpResponse> {
    let count = import_equipment_logic(&app_state.db_pool, body.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_with_message((), format!("Imported {} equipment", count))))
}

pub async fn import_equipment_excel(app_state: web::Data<Arc<AppState>>, payload: Multipart) -> ApiResult<HttpResponse> {
    let file_path = save_multipart_to_temp(payload).await?;
    let path_clone = file_path.clone();
    let items_res = web::block(move || {
        let mut workbook: Xlsx<_> = open_workbook(&path_clone).map_err(|e: XlsxError| e.to_string())?;
        let range = workbook.worksheet_range_at(0).ok_or("Empty")?.map_err(|e| e.to_string())?;
        let mut list = Vec::new();
        let iter = RangeDeserializerBuilder::new().from_range(&range).map_err(|e| e.to_string())?;
        for res in iter { if let Ok(r) = res { list.push(r); } }
        Ok::<Vec<EquipmentImportDto>, String>(list)
    }).await.map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    match items_res {
        Ok(items) => {
            let count = import_equipment_logic(&app_state.db_pool, items).await?;
            let _ = fs::remove_file(file_path);
            Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_with_message((), format!("Imported {} equipment", count))))
        },
        Err(e) => { let _ = fs::remove_file(file_path); Err(ApiError::BadRequest(e)) }
    }
}

pub async fn import_equipment(app_state: web::Data<Arc<AppState>>, body: web::Json<Vec<EquipmentImportDto>>) -> ApiResult<HttpResponse> {
    import_equipment_json(app_state, body).await
}

async fn import_equipment_logic(pool: &SqlitePool, items: Vec<EquipmentImportDto>) -> ApiResult<usize> {
    let total_items = items.len();
    let start_time = Instant::now();
    
    log::info!("🚀 Starting BULK equipment import of {} items...", total_items);
    
    // Apply PRAGMA optimizations
    optimize_sqlite_for_bulk(pool).await?;
    
    // Prepare equipment data
    struct PrepEquip {
        id: String,
        name: String,
        eq_type: String,
        serial_number: Option<String>,
        manufacturer: Option<String>,
        location: Option<String>,
        description: Option<String>,
    }
    
    let valid_types = ["equipment", "labware", "instrument", "glassware", "safety", "storage", "consumable", "other"];
    
    let prepared: Vec<PrepEquip> = items.iter()
        .filter(|item| !item.name.trim().is_empty())
        .map(|item| {
            let eq_type = if valid_types.contains(&item.equipment_type.to_lowercase().as_str()) {
                item.equipment_type.to_lowercase()
            } else {
                "other".to_string()
            };
            PrepEquip {
                id: Uuid::new_v4().to_string(),
                name: item.name.trim().to_string(),
                eq_type,
                serial_number: item.serial_number.clone(),
                manufacturer: item.manufacturer.clone(),
                location: item.location.clone(),
                description: item.description.clone(),
            }
        })
        .collect();
    
    log::info!("📋 Prepared {} equipment items for bulk insert", prepared.len());
    
    // === PRAGMA BEFORE TRANSACTION ===
    sqlx::query("PRAGMA synchronous = OFF").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    // === SINGLE TRANSACTION ===
    let mut tx = pool.begin().await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    const CHUNK_SIZE: usize = 100;
    let mut processed = 0;
    
    for chunk in prepared.chunks(CHUNK_SIZE) {
        let values_clause: String = chunk.iter()
            .map(|_| "(?,?,?,?,?,'available',?,?,datetime('now'),datetime('now'))")
            .collect::<Vec<_>>()
            .join(",");
        
        let sql = format!(
            r#"INSERT INTO equipment (
                id, name, type_, serial_number, manufacturer, 
                status, location, description, 
                created_at, updated_at
            ) VALUES {}
            ON CONFLICT(serial_number) WHERE serial_number IS NOT NULL 
            DO UPDATE SET name = excluded.name, updated_at = datetime('now')"#,
            values_clause
        );
        
        let mut query = sqlx::query(&sql);
        for e in chunk {
            query = query
                .bind(&e.id)
                .bind(&e.name)
                .bind(&e.eq_type)
                .bind(&e.serial_number)
                .bind(&e.manufacturer)
                .bind(&e.location)
                .bind(&e.description);
        }
        
        query.execute(&mut *tx).await
            .map_err(|e| ApiError::InternalServerError(format!("Bulk equipment insert failed: {}", e)))?;
        
        processed += chunk.len();
        if processed % 50000 == 0 {
            log::info!("📥 Equipment: {}/{}", processed, prepared.len());
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
    log::info!("✅ BULK equipment import completed in {:.2?}. {} items at {:.0} items/sec", elapsed, total_items, rate);
    
    Ok(total_items)
}

pub async fn export_equipment(app_state: web::Data<Arc<AppState>>) -> ApiResult<HttpResponse> {
    let whitelist = FieldWhitelist::for_equipment();
    let builder = SafeQueryBuilder::new("SELECT * FROM equipment")
        .map_err(|e| ApiError::InternalServerError(e))?
        .with_whitelist(&whitelist);
    
    let (sql, _) = builder.build();
    let equipment = sqlx::query_as::<_, crate::models::Equipment>(&sql)
        .fetch_all(&app_state.db_pool)
        .await?;
    Ok(HttpResponse::Ok().json(equipment))
}