// src/import_export/mod.rs
//! Optimized import/export with query_builders integration
//! OPTIMIZATIONS v2 (BULK INSERT):
//! - Preload users map (avoid N queries for owner lookup)
//! - Preload reagents map (avoid SELECT after INSERT)
//! - BULK INSERT: 60-80 rows per query instead of 1 (10-50x faster)
//! - PRAGMA optimizations for SQLite (WAL, cache, mmap)
//! - Two-phase: prepare all data first, then bulk write
//! - FIX: Correct date parsing from Excel (avoids 1970 issue)
//! Expected: 5,000-15,000 items/sec (vs 350 items/sec)

mod dto;
mod reagents;
mod batches;
mod equipment;

// Re-export DTOs
pub use dto::{ReagentImportDto, BatchImportDto, EquipmentImportDto};

// Re-export handlers
pub use reagents::{import_reagents_excel, import_reagents_json, import_reagents, export_reagents};
pub use batches::{import_batches_excel, import_batches_json, import_batches, export_batches};
pub use equipment::{import_equipment_excel, import_equipment_json, import_equipment, export_equipment};

use actix_multipart::Multipart;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Deserializer};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use std::io::Write;
use uuid::Uuid;
use sqlx::Row;
use chrono::{NaiveDate, NaiveDateTime, DateTime};
use crate::error::{ApiResult, ApiError};

// ==========================================
// CUSTOM DESERIALIZER (FIX FOR DATE ISSUE)
// ==========================================

/// Десериализует дату из разных форматов (Excel float, String DD.MM.YYYY, ISO) в ISO String
pub(crate) fn deserialize_flexible_date<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
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
            let seconds = (f - 25569.0) * 86400.0;
            if seconds >= 0.0 {
                if let Some(dt) = DateTime::from_timestamp(seconds as i64, 0) {
                    return Ok(Some(dt.naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string()));
                }
            }
            Ok(None)
        },
        Some(DateValue::Int(i)) => {
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
                if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
                    return Ok(Some(dt.format("%Y-%m-%dT%H:%M:%S").to_string()));
                }
            }
            
            Ok(Some(s.to_string()))
        },
        None => Ok(None),
    }
}

// ==========================================
// HELPERS
// ==========================================

pub(crate) async fn save_multipart_to_temp(mut payload: Multipart) -> ApiResult<PathBuf> {
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
pub(crate) async fn preload_users(pool: &SqlitePool) -> ApiResult<HashMap<String, String>> {
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
pub(crate) async fn preload_reagents(pool: &SqlitePool) -> ApiResult<HashMap<String, String>> {
    // Include soft-deleted: ON CONFLICT(name) matches them, so we need their real IDs
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

/// Preload existing batches: (reagent_id, batch_number_lower) → batch_id
pub(crate) async fn preload_batches(pool: &SqlitePool) -> ApiResult<HashMap<(String, String), String>> {
    let rows = sqlx::query("SELECT id, reagent_id, batch_number FROM batches")
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to preload batches: {}", e)))?;
    
    let map: HashMap<(String, String), String> = rows
        .into_iter()
        .map(|row| {
            let reagent_id: String = row.get("reagent_id");
            let batch_number: String = row.get("batch_number");
            let id: String = row.get("id");
            ((reagent_id, batch_number.trim().to_lowercase()), id)
        })
        .collect();
    
    Ok(map)
}

/// Preload storage position lookup: "room_lower|zone_lower|position_lower" → position_id
pub(crate) async fn preload_position_lookup(pool: &SqlitePool) -> ApiResult<HashMap<String, String>> {
    let rows = sqlx::query(
        r#"SELECT sp.id AS position_id, r.name AS room_name, sz.name AS zone_name, sp.name AS position_name
           FROM storage_positions sp
           JOIN storage_zones sz ON sp.zone_id = sz.id
           JOIN rooms r ON sz.room_id = r.id
           WHERE sp.status = 'available'"#
    )
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to preload positions: {}", e)))?;
    
    let map: HashMap<String, String> = rows
        .into_iter()
        .map(|row| {
            let room: String = row.get("room_name");
            let zone: String = row.get("zone_name");
            let pos: String = row.get("position_name");
            let key = format!("{}|{}|{}", room.trim().to_lowercase(), zone.trim().to_lowercase(), pos.trim().to_lowercase());
            (key, row.get::<String, _>("position_id"))
        })
        .collect();
    
    Ok(map)
}

/// Parse location path "Room → Zone → Position" into lookup key
/// Supports separators: →, ->, /
/// Returns None if path doesn't have exactly 3 parts
pub(crate) fn parse_location_path(path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    
    let parts: Vec<&str> = if path.contains('→') {
        path.split('→').collect()
    } else if path.contains("->") {
        path.split("->").collect()
    } else if path.contains('/') {
        path.split('/').collect()
    } else {
        return None;
    };
    
    if parts.len() >= 3 {
        let room = parts[0].trim().to_lowercase();
        let zone = parts[1].trim().to_lowercase();
        let position = parts[2].trim().to_lowercase();
        if room.is_empty() || zone.is_empty() || position.is_empty() {
            return None;
        }
        Some(format!("{}|{}|{}", room, zone, position))
    } else {
        None
    }
}

// ==========================================
// PRAGMA OPTIMIZATION (for bulk imports)
// ==========================================

/// Apply SQLite PRAGMA settings for faster bulk imports
pub(crate) async fn optimize_sqlite_for_bulk(pool: &SqlitePool) -> ApiResult<()> {
    sqlx::query("PRAGMA journal_mode = WAL").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    sqlx::query("PRAGMA synchronous = NORMAL").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    sqlx::query("PRAGMA cache_size = -64000").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    sqlx::query("PRAGMA temp_store = MEMORY").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    sqlx::query("PRAGMA mmap_size = 268435456").execute(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    Ok(())
}
