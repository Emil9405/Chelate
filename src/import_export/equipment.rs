// src/import_export/equipment.rs
//! Equipment import/export handlers

use actix_web::{web, HttpResponse};
use actix_multipart::Multipart;
use sqlx::SqlitePool;
use std::sync::Arc;
use calamine::{Reader, open_workbook, RangeDeserializerBuilder, Xlsx, XlsxError};
use std::fs;
use uuid::Uuid;
use std::time::Instant;
use crate::{AppState, error::{ApiResult, ApiError}, handlers::ApiResponse};
use crate::query_builders::{SafeQueryBuilder, FieldWhitelist};
use super::dto::EquipmentImportDto;
use super::{save_multipart_to_temp, optimize_sqlite_for_bulk};

// ==========================================
// EQUIPMENT IMPORT
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

// ==========================================
// EQUIPMENT EXPORT
// ==========================================

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
