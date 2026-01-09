// src/import_export.rs
//! Optimized import/export with query_builders integration
//! OPTIMIZATIONS:
//! - Preload users map (avoid N queries for owner lookup)
//! - Preload reagents map (avoid SELECT after INSERT)
//! - Remove unnecessary sleep between chunks
//! - Use generated IDs directly for batches
//! - FIX: Correct date parsing from Excel (avoids 1970 issue)

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
use chrono::{Utc, NaiveDate, NaiveDateTime}; // Added Chrono types
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
                if let Some(dt) = NaiveDateTime::from_timestamp_opt(seconds as i64, 0) {
                    return Ok(Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string()));
                }
            }
            Ok(None)
        },
        Some(DateValue::Int(i)) => {
            // Same logic if Excel passes it as integer
            let seconds = (i as f64 - 25569.0) * 86400.0;
            if seconds >= 0.0 {
                if let Some(dt) = NaiveDateTime::from_timestamp_opt(seconds as i64, 0) {
                    return Ok(Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string()));
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
    
    #[serde(alias = "Quantiy in pcs", alias = "Quantity in pcs")]
    pub quantity_pcs: Option<String>,
    
    #[serde(alias = "Quantity", alias = "quantity", alias = "Количество")]
    pub quantity: Option<f64>,
    
    #[serde(alias = "Units", alias = "units", alias = "Unit", alias = "unit", alias = "Единицы")]
    pub units: Option<String>,
    
    #[serde(alias = "Expiry Date", alias = "expiry_date", alias = "expiration_date", alias = "Срок годности")]
    #[serde(default, deserialize_with = "deserialize_flexible_date")] // <--- ПРИМЕНЕНО ЗДЕСЬ
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
    #[serde(alias = "Batch Number", alias = "batch_number")]
    pub batch_number: String,
    pub supplier: Option<String>,
    #[serde(alias = "quantity", alias = "Quantity", alias = "Amount")]
    pub quantity: f64, 
    #[serde(alias = "unit", alias = "Unit", alias = "units")]
    pub units: String,
    
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
    let rows = sqlx::query("SELECT name, id FROM reagents")
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
    let mut processed_count = 0;
    let start_time = Instant::now();
    
    log::info!("🚀 Starting optimized import of {} reagents...", total_items);
    
    // OPTIMIZATION 1: Preload all users and reagents ONCE before loop
    let users_map = preload_users(pool).await?;
    let mut reagents_map = preload_reagents(pool).await?;
    
    log::info!("📦 Preloaded {} users, {} reagents", users_map.len(), reagents_map.len());
    
    for (chunk_idx, chunk) in reagents.chunks(3000).enumerate() {
        let mut tx = pool.begin().await
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        for r in chunk {
            let name = r.name.trim();
            if name.is_empty() { continue; }
            
            let name_key = name.to_lowercase();

            // OPTIMIZATION 2: Lookup owner from preloaded map (no DB query!)
            let owner_id = r.owner.as_ref()
                .and_then(|o| users_map.get(&o.trim().to_lowercase()))
                .cloned()
                .unwrap_or_else(|| current_user_id.clone());

            let created_at_val = r.added_at.as_ref()
                .filter(|s| !s.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| Utc::now().to_rfc3339());

            // OPTIMIZATION 3: Check if reagent exists, reuse ID or generate new
            let reagent_id = if let Some(existing_id) = reagents_map.get(&name_key) {
                existing_id.clone()
            } else {
                let new_id = Uuid::new_v4().to_string();
                reagents_map.insert(name_key.clone(), new_id.clone());
                new_id
            };

            // Insert or update reagent
            sqlx::query(
                r#"INSERT INTO reagents (
                    id, name, formula, cas_number, manufacturer, description,
                    storage_conditions, appearance, hazard_pictograms, status, molecular_weight, 
                    created_by, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'active', ?, ?, ?, datetime('now'))
                ON CONFLICT(name) DO UPDATE SET 
                    formula = COALESCE(excluded.formula, formula),
                    cas_number = COALESCE(excluded.cas_number, cas_number),
                    manufacturer = COALESCE(excluded.manufacturer, manufacturer),
                    description = COALESCE(excluded.description, description),
                    storage_conditions = COALESCE(excluded.storage_conditions, storage_conditions),
                    appearance = COALESCE(excluded.appearance, appearance),
                    hazard_pictograms = COALESCE(excluded.hazard_pictograms, hazard_pictograms),
                    molecular_weight = COALESCE(excluded.molecular_weight, molecular_weight),
                    updated_at = datetime('now')
                "#
            )
            .bind(&reagent_id)
            .bind(name)
            .bind(&r.formula)
            .bind(&r.cas_number)
            .bind(&r.manufacturer)
            .bind(&r.description)
            .bind(&r.storage)
            .bind(&r.appearance)
            .bind(&r.hazard_pictograms)
            .bind(&r.molecular_weight)
            .bind(&owner_id)
            .bind(&created_at_val)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to insert reagent '{}': {}", name, e)))?;

            // Insert batch if data present
            // OPTIMIZATION 4: Use reagent_id directly (no SELECT needed!)
            if let (Some(batch_num), Some(qty), Some(unit)) = (&r.batch_number, r.quantity, &r.units) {
                if !batch_num.trim().is_empty() && qty > 0.0 {
                    sqlx::query(
                        r#"INSERT INTO batches (
                            id, reagent_id, batch_number, quantity, original_quantity,
                            reserved_quantity, unit, expiry_date, location, status,
                            received_date, created_at, updated_at, created_by, updated_by
                        ) VALUES (?, ?, ?, ?, ?, 0.0, ?, ?, ?, 'available', datetime('now'), datetime('now'), datetime('now'), ?, ?)
                        ON CONFLICT(reagent_id, batch_number) DO UPDATE SET 
                            quantity = quantity + excluded.quantity,
                            original_quantity = original_quantity + excluded.original_quantity
                        "#
                    )
                    .bind(Uuid::new_v4().to_string())
                    .bind(&reagent_id)  
                    .bind(batch_num.trim())
                    .bind(qty)
                    .bind(qty)
                    .bind(unit)
                    .bind(&r.expiry_date) // This now contains ISO string or None, not Excel float
                    .bind(&r.location)
                    .bind(&current_user_id)
                    .bind(&current_user_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| ApiError::InternalServerError(format!("Failed to insert batch: {}", e)))?;
                }
            }
        }

        tx.commit().await
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
        
        processed_count += chunk.len();
        log::info!("📥 Chunk {} done. Progress: {}/{}", chunk_idx + 1, processed_count, total_items);
    }

    let elapsed = start_time.elapsed();
    let rate = if elapsed.as_secs_f64() > 0.0 { 
        processed_count as f64 / elapsed.as_secs_f64() 
    } else { 
        0.0 
    };
    
    log::info!("✅ Import completed in {:.2?}. {} items at {:.0} items/sec", elapsed, processed_count, rate);

    Ok(processed_count)
}

pub async fn export_reagents(app_state: web::Data<Arc<AppState>>) -> ApiResult<HttpResponse> {
    let whitelist = FieldWhitelist::for_reagents();
    let builder = SafeQueryBuilder::new("SELECT * FROM reagents")
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
    let mut processed_count = 0;
    let start_time = Instant::now();
    
    log::info!("🚀 Starting optimized batch import of {} items...", total_items);
    
    // Preload reagents map
    let mut reagent_map = preload_reagents(pool).await?;
    
    for (chunk_idx, chunk) in batches.chunks(5000).enumerate() {
        let mut tx = pool.begin().await.map_err(|e| ApiError::InternalServerError(e.to_string()))?;

        for b in chunk {
            let r_name_raw = b.reagent_name.trim();
            if b.batch_number.trim().is_empty() || r_name_raw.is_empty() { continue; }
            
            let r_name_key = r_name_raw.to_lowercase();
            
            // Get existing reagent ID or create new one
            let r_id = if let Some(id) = reagent_map.get(&r_name_key) {
                id.clone()
            } else {
                let new_id = Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO reagents (id, name, status, created_at, updated_at) VALUES (?, ?, 'active', datetime('now'), datetime('now'))")
                    .bind(&new_id)
                    .bind(r_name_raw)
                    .execute(&mut *tx).await?;
                reagent_map.insert(r_name_key, new_id.clone());
                new_id
            };

            sqlx::query(
                r#"INSERT INTO batches (
                    id, reagent_id, batch_number, supplier, 
                    quantity, original_quantity, reserved_quantity,
                    unit, expiry_date, received_date,
                    location, notes, created_at, updated_at, status
                )
                VALUES (?, ?, ?, ?, ?, ?, 0.0, ?, ?, datetime('now'), ?, ?, datetime('now'), datetime('now'), 'available')
                ON CONFLICT(reagent_id, batch_number) DO UPDATE SET 
                    quantity = quantity + excluded.quantity,
                    original_quantity = original_quantity + excluded.original_quantity
                "#
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&r_id)
            .bind(b.batch_number.trim())
            .bind(&b.supplier)
            .bind(b.quantity) 
            .bind(b.quantity) 
            .bind(&b.units)
            .bind(&b.expiration_date) // ISO String or None
            .bind(&b.location)
            .bind(&b.notes)
            .execute(&mut *tx).await.map_err(|e| ApiError::InternalServerError(e.to_string()))?;
        }
        
        tx.commit().await.map_err(|e| ApiError::InternalServerError(e.to_string()))?;
        processed_count += chunk.len();
        log::info!("📥 Chunk {} done. Progress: {}/{}", chunk_idx + 1, processed_count, total_items);
    }
    
    let elapsed = start_time.elapsed();
    log::info!("✅ Batch import completed in {:.2?}. {} items", elapsed, processed_count);
    
    Ok(processed_count)
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
    let mut processed_count = 0;
    let start_time = Instant::now();
    
    log::info!("🚀 Starting equipment import of {} items...", total_items);
    
    for (chunk_idx, chunk) in items.chunks(3000).enumerate() {
        let mut tx = pool.begin().await.map_err(|e| ApiError::InternalServerError(e.to_string()))?;
        
        for item in chunk {
            let name = item.name.trim();
            if name.is_empty() { continue; }
            
            let valid_types = ["equipment", "labware", "instrument", "glassware", "safety", "storage", "consumable", "other"];
            let eq_type = if valid_types.contains(&item.equipment_type.to_lowercase().as_str()) {
                item.equipment_type.to_lowercase()
            } else {
                "other".to_string()
            };
            
            sqlx::query(
                r#"INSERT INTO equipment (
                    id, name, type_, serial_number, manufacturer, 
                    status, location, description, 
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, 'available', ?, ?, datetime('now'), datetime('now')) 
                ON CONFLICT(serial_number) WHERE serial_number IS NOT NULL 
                DO UPDATE SET name = excluded.name, updated_at = datetime('now')"#
            )
            .bind(Uuid::new_v4().to_string())
            .bind(name)
            .bind(&eq_type)
            .bind(&item.serial_number)
            .bind(&item.manufacturer)
            .bind(&item.location)
            .bind(&item.description)
            .execute(&mut *tx)
            .await
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
        }
        
        tx.commit().await.map_err(|e| ApiError::InternalServerError(e.to_string()))?;
        processed_count += chunk.len();
        log::info!("📥 Chunk {} done. Progress: {}/{}", chunk_idx + 1, processed_count, total_items);
    }
    
    let elapsed = start_time.elapsed();
    log::info!("✅ Equipment import completed in {:.2?}. {} items", elapsed, processed_count);
    
    Ok(processed_count)
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