// src/import_export/mod.rs
//! Optimized import/export with query_builders integration
//! OPTIMIZATIONS v2 (BULK INSERT):
//! - Preload users map (avoid N queries for owner lookup)
//! - Preload reagents map (avoid SELECT after INSERT)
//! - BULK INSERT: 60-80 rows per query instead of 1 (10-50x faster)
//! - PRAGMA optimizations for SQLite (WAL, cache, mmap)
//! - Two-phase: prepare all data first, then bulk write
//! - FIX: Correct date parsing from Excel (avoids 1970 issue)
//! - Auto-create storage locations (Room → Zone → Position) during import
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

/// Preload max sequence_number per batch: batch_id → max_seq
pub(crate) async fn preload_container_max_sequences(pool: &SqlitePool) -> ApiResult<HashMap<String, i64>> {
    let rows = sqlx::query(
        "SELECT batch_id, MAX(sequence_number) as max_seq FROM batch_containers GROUP BY batch_id"
    )
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to preload container sequences: {}", e)))?;
    
    let map: HashMap<String, i64> = rows
        .into_iter()
        .map(|row| {
            let batch_id: String = row.get("batch_id");
            let max_seq: i64 = row.get("max_seq");
            (batch_id, max_seq)
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

/// Parse location path and return ORIGINAL-CASE parts: (room, zone, position)
pub(crate) fn parse_location_parts(path: &str) -> Option<(String, String, String)> {
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
        let room = parts[0].trim().to_string();
        let zone = parts[1].trim().to_string();
        let position = parts[2].trim().to_string();
        if room.is_empty() || zone.is_empty() || position.is_empty() {
            return None;
        }
        Some((room, zone, position))
    } else {
        None
    }
}

// ==========================================
// AUTO-CREATE STORAGE LOCATIONS
// ==========================================

/// Ensure all storage locations from import data exist.
/// Creates missing rooms → zones → positions automatically.
/// Updates position_lookup with newly created entries.
pub(crate) async fn ensure_storage_locations(
    pool: &SqlitePool,
    locations: &[String],
    position_lookup: &mut HashMap<String, String>,
) -> ApiResult<()> {
    // Collect unique location paths that don't exist yet
    let mut needed: Vec<(String, String, String)> = Vec::new(); // (room, zone, position) original case
    let mut seen_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    
    for loc in locations {
        if let Some(key) = parse_location_path(loc) {
            if !position_lookup.contains_key(&key) && seen_keys.insert(key) {
                if let Some(parts) = parse_location_parts(loc) {
                    needed.push(parts);
                }
            }
        }
    }
    
    if needed.is_empty() {
        return Ok(());
    }
    
    log::info!("🏗️ Auto-creating {} missing storage locations...", needed.len());
    
    // Preload existing rooms: name_lower → id
    let room_rows = sqlx::query("SELECT id, name FROM rooms")
        .fetch_all(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    let mut rooms_map: HashMap<String, String> = room_rows.into_iter()
        .map(|r| (r.get::<String, _>("name").trim().to_lowercase(), r.get::<String, _>("id")))
        .collect();
    
    // Preload existing zones: "room_id|zone_name_lower" → zone_id
    let zone_rows = sqlx::query("SELECT id, room_id, name FROM storage_zones")
        .fetch_all(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    let mut zones_map: HashMap<String, String> = zone_rows.into_iter()
        .map(|r| {
            let room_id: String = r.get("room_id");
            let name: String = r.get("name");
            (format!("{}|{}", room_id, name.trim().to_lowercase()), r.get::<String, _>("id"))
        })
        .collect();
    
    // Preload existing positions: "zone_id|pos_name_lower" → position_id
    let pos_rows = sqlx::query("SELECT id, zone_id, name FROM storage_positions")
        .fetch_all(pool).await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    let mut positions_map: HashMap<String, String> = pos_rows.into_iter()
        .map(|r| {
            let zone_id: String = r.get("zone_id");
            let name: String = r.get("name");
            (format!("{}|{}", zone_id, name.trim().to_lowercase()), r.get::<String, _>("id"))
        })
        .collect();
    
    let now = chrono::Utc::now();
    
    // Collect new rooms/zones/positions to create
    struct NewRoom { id: String, name: String }
    struct NewZone { id: String, room_id: String, name: String }
    struct NewPosition { id: String, zone_id: String, name: String, lookup_key: String }
    
    let mut new_rooms: Vec<NewRoom> = Vec::new();
    let mut new_zones: Vec<NewZone> = Vec::new();
    let mut new_positions: Vec<NewPosition> = Vec::new();
    
    for (room_name, zone_name, pos_name) in &needed {
        let room_key = room_name.to_lowercase();
        
        // Ensure room exists
        let room_id = if let Some(id) = rooms_map.get(&room_key) {
            id.clone()
        } else {
            let id = Uuid::new_v4().to_string();
            rooms_map.insert(room_key.clone(), id.clone());
            new_rooms.push(NewRoom { id: id.clone(), name: room_name.clone() });
            id
        };
        
        // Ensure zone exists
        let zone_key = format!("{}|{}", room_id, zone_name.to_lowercase());
        let zone_id = if let Some(id) = zones_map.get(&zone_key) {
            id.clone()
        } else {
            let id = Uuid::new_v4().to_string();
            zones_map.insert(zone_key, id.clone());
            new_zones.push(NewZone { id: id.clone(), room_id: room_id.clone(), name: zone_name.clone() });
            id
        };
        
        // Ensure position exists
        let pos_key = format!("{}|{}", zone_id, pos_name.to_lowercase());
        let lookup_key = format!("{}|{}|{}", room_name.to_lowercase(), zone_name.to_lowercase(), pos_name.to_lowercase());
        
        if let Some(id) = positions_map.get(&pos_key) {
            // Position exists but wasn't in position_lookup (maybe status != 'available')
            position_lookup.insert(lookup_key, id.clone());
        } else {
            let id = Uuid::new_v4().to_string();
            positions_map.insert(pos_key, id.clone());
            position_lookup.insert(lookup_key.clone(), id.clone());
            new_positions.push(NewPosition { id, zone_id, name: pos_name.clone(), lookup_key });
        }
    }
    
    // Bulk insert in a transaction
    if new_rooms.is_empty() && new_zones.is_empty() && new_positions.is_empty() {
        return Ok(());
    }
    
    let mut tx = pool.begin().await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    // Insert rooms
    for room in &new_rooms {
        sqlx::query(
            "INSERT OR IGNORE INTO rooms (id, name, status, created_at, updated_at) VALUES (?,?,'available',?,?)"
        )
            .bind(&room.id)
            .bind(&room.name)
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx).await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to create room '{}': {}", room.name, e)))?;
    }
    
    // Insert zones
    for zone in &new_zones {
        sqlx::query(
            "INSERT OR IGNORE INTO storage_zones (id, room_id, name, zone_type, status, sort_order, is_locked, created_at, updated_at) VALUES (?,?,?,'other','available',0,0,?,?)"
        )
            .bind(&zone.id)
            .bind(&zone.room_id)
            .bind(&zone.name)
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx).await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to create zone '{}': {}", zone.name, e)))?;
    }
    
    // Insert positions
    for pos in &new_positions {
        sqlx::query(
            "INSERT OR IGNORE INTO storage_positions (id, zone_id, name, current_count, sort_order, status, created_at, updated_at) VALUES (?,?,?,0,0,'available',?,?)"
        )
            .bind(&pos.id)
            .bind(&pos.zone_id)
            .bind(&pos.name)
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx).await
            .map_err(|e| ApiError::InternalServerError(format!("Failed to create position '{}': {}", pos.name, e)))?;
    }
    
    tx.commit().await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
    
    log::info!(
        "🏗️ Created {} rooms, {} zones, {} positions",
        new_rooms.len(), new_zones.len(), new_positions.len()
    );
    
    Ok(())
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
