// src/storage_handlers.rs
//! Обработчики для управления зонами и позициями хранения
//! Иерархия: Room → StorageZone (шкаф/холодильник) → StoragePosition (полка/ящик)

use actix_web::{web, HttpResponse};
use std::sync::Arc;
use crate::AppState;
use crate::models::storage_zone::*;
use crate::error::{ApiError, ApiResult};
use crate::handlers::ApiResponse;
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;
use log::info;
use sqlx::SqlitePool;

// ============================================================
//                    STORAGE ZONES CRUD
// ============================================================

/// GET /api/storage/zones?room_id=xxx
pub async fn get_storage_zones(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<HttpResponse> {
    let zones = if let Some(room_id) = query.get("room_id") {
        let room_exists: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM rooms WHERE id = ?"
        ).bind(room_id).fetch_optional(&app_state.db_pool).await?;

        if room_exists.is_none() {
            return Err(ApiError::not_found("Room"));
        }

        sqlx::query_as::<_, StorageZone>(
            "SELECT * FROM storage_zones WHERE room_id = ? ORDER BY sort_order ASC, name ASC"
        ).bind(room_id).fetch_all(&app_state.db_pool).await?
    } else {
        sqlx::query_as::<_, StorageZone>(
            "SELECT * FROM storage_zones ORDER BY room_id, sort_order ASC, name ASC"
        ).fetch_all(&app_state.db_pool).await?
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(zones)))
}

/// GET /api/storage/zones/{id}
pub async fn get_storage_zone(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let zone_id = path.into_inner();

    let zone: Option<StorageZone> = sqlx::query_as(
        "SELECT * FROM storage_zones WHERE id = ?"
    ).bind(&zone_id).fetch_optional(&app_state.db_pool).await?;

    match zone {
        Some(z) => Ok(HttpResponse::Ok().json(ApiResponse::success(z))),
        None => Err(ApiError::not_found("Storage zone")),
    }
}

/// POST /api/storage/zones
pub async fn create_storage_zone(
    app_state: web::Data<Arc<AppState>>,
    data: web::Json<CreateStorageZoneRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    data.validate()?;

    if !StorageZoneType::is_valid(&data.zone_type) {
        return Err(ApiError::bad_request("Invalid zone type"));
    }

    if let Some(ref cond) = data.storage_condition {
        if !StorageCondition::is_valid(cond) {
            return Err(ApiError::bad_request("Invalid storage condition"));
        }
    }

    let room_exists: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM rooms WHERE id = ?"
    ).bind(&data.room_id).fetch_optional(&app_state.db_pool).await?;

    if room_exists.is_none() {
        return Err(ApiError::not_found("Room"));
    }

    let duplicate: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM storage_zones WHERE room_id = ? AND LOWER(name) = LOWER(?)"
    ).bind(&data.room_id).bind(&data.name)
    .fetch_optional(&app_state.db_pool).await?;

    if duplicate.is_some() {
        return Err(ApiError::bad_request("A storage zone with this name already exists in this room"));
    }

    if let (Some(min), Some(max)) = (data.temperature_min, data.temperature_max) {
        if min > max {
            return Err(ApiError::bad_request("Minimum temperature cannot be greater than maximum"));
        }
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let is_locked = if data.is_locked.unwrap_or(false) { 1 } else { 0 };
    let sort_order = data.sort_order.unwrap_or(0);

    sqlx::query(
        r#"INSERT INTO storage_zones 
           (id, room_id, name, zone_type, storage_condition, description,
            temperature_min, temperature_max, is_locked, sort_order, status,
            created_by, updated_by, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'available', ?, ?, ?, ?)"#
    )
    .bind(&id).bind(&data.room_id).bind(&data.name).bind(&data.zone_type)
    .bind(&data.storage_condition).bind(&data.description).bind(&data.temperature_min)
    .bind(&data.temperature_max).bind(is_locked).bind(sort_order).bind(&user_id)
    .bind(&user_id).bind(&now).bind(&now)
    .execute(&app_state.db_pool).await?;

    let created: StorageZone = sqlx::query_as("SELECT * FROM storage_zones WHERE id = ?")
        .bind(&id).fetch_one(&app_state.db_pool).await?;

    info!("🗄️ Created storage zone: {} ({}) in room {}", data.name, data.zone_type, data.room_id);
    Ok(HttpResponse::Created().json(ApiResponse::success(created)))
}

/// PUT /api/storage/zones/{id}
pub async fn update_storage_zone(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    data: web::Json<UpdateStorageZoneRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    data.validate()?;
    let zone_id = path.into_inner();

    let existing: Option<StorageZone> = sqlx::query_as(
        "SELECT * FROM storage_zones WHERE id = ?"
    ).bind(&zone_id).fetch_optional(&app_state.db_pool).await?;

    let existing = existing.ok_or_else(|| ApiError::not_found("Storage zone"))?;

    if let Some(ref zt) = data.zone_type {
        if !StorageZoneType::is_valid(zt) { return Err(ApiError::bad_request("Invalid zone type")); }
    }

    if let Some(ref cond) = data.storage_condition {
        if !StorageCondition::is_valid(cond) { return Err(ApiError::bad_request("Invalid storage condition")); }
    }

    if let Some(ref new_name) = data.name {
        if new_name.to_lowercase() != existing.name.to_lowercase() {
            let dup: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM storage_zones WHERE room_id = ? AND LOWER(name) = LOWER(?) AND id != ?"
            ).bind(&existing.room_id).bind(new_name).bind(&zone_id)
            .fetch_optional(&app_state.db_pool).await?;

            if dup.is_some() {
                return Err(ApiError::bad_request("A storage zone with this name already exists in this room"));
            }
        }
    }

    let now = Utc::now();
    let name = data.name.as_ref().unwrap_or(&existing.name);
    let zone_type = data.zone_type.as_ref().unwrap_or(&existing.zone_type);
    let storage_condition = data.storage_condition.clone().or(existing.storage_condition);
    let description = data.description.clone().or(existing.description);
    let temp_min = data.temperature_min.or(existing.temperature_min);
    let temp_max = data.temperature_max.or(existing.temperature_max);
    let is_locked = data.is_locked.map(|b| if b { 1 } else { 0 }).unwrap_or(existing.is_locked);
    let sort_order = data.sort_order.unwrap_or(existing.sort_order);
    let status = data.status.as_ref().unwrap_or(&existing.status);

    sqlx::query(
        r#"UPDATE storage_zones 
           SET name = ?, zone_type = ?, storage_condition = ?, description = ?,
               temperature_min = ?, temperature_max = ?, is_locked = ?, 
               sort_order = ?, status = ?, updated_by = ?, updated_at = ?
           WHERE id = ?"#
    )
    .bind(name).bind(zone_type).bind(&storage_condition).bind(&description).bind(&temp_min)
    .bind(&temp_max).bind(is_locked).bind(sort_order).bind(status).bind(&user_id)
    .bind(&now).bind(&zone_id)
    .execute(&app_state.db_pool).await?;

    let updated: StorageZone = sqlx::query_as("SELECT * FROM storage_zones WHERE id = ?")
        .bind(&zone_id).fetch_one(&app_state.db_pool).await?;

    info!("🗄️ Updated storage zone: {} ({})", updated.name, zone_id);
    Ok(HttpResponse::Ok().json(ApiResponse::success(updated)))
}

/// DELETE /api/storage/zones/{id}
pub async fn delete_storage_zone(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let zone_id = path.into_inner();

    let items_count: (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*) FROM batch_placements 
           WHERE position_id IN (SELECT id FROM storage_positions WHERE zone_id = ?)"#
    ).bind(&zone_id).fetch_one(&app_state.db_pool).await?;

    if items_count.0 > 0 {
        return Err(ApiError::bad_request(&format!(
            "Cannot delete: {} items are stored in this zone. Move them first.", items_count.0
        )));
    }

    sqlx::query("DELETE FROM storage_positions WHERE zone_id = ?").bind(&zone_id).execute(&app_state.db_pool).await?;
    let result = sqlx::query("DELETE FROM storage_zones WHERE id = ?").bind(&zone_id).execute(&app_state.db_pool).await?;

    if result.rows_affected() == 0 { return Err(ApiError::not_found("Storage zone")); }

    info!("🗄️ Deleted storage zone: {}", zone_id);
    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message((), "Storage zone deleted successfully".to_string())))
}

// ============================================================
//                  STORAGE POSITIONS CRUD
// ============================================================

pub async fn get_storage_positions(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<HttpResponse> {
    let positions = if let Some(zone_id) = query.get("zone_id") {
        sqlx::query_as::<_, StoragePosition>(
            "SELECT * FROM storage_positions WHERE zone_id = ? ORDER BY sort_order ASC, name ASC"
        ).bind(zone_id).fetch_all(&app_state.db_pool).await?
    } else {
        sqlx::query_as::<_, StoragePosition>(
            "SELECT * FROM storage_positions ORDER BY zone_id, sort_order ASC, name ASC"
        ).fetch_all(&app_state.db_pool).await?
    };
    Ok(HttpResponse::Ok().json(ApiResponse::success(positions)))
}

pub async fn get_storage_position(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let pos_id = path.into_inner();
    let pos: Option<StoragePosition> = sqlx::query_as("SELECT * FROM storage_positions WHERE id = ?")
        .bind(&pos_id).fetch_optional(&app_state.db_pool).await?;

    match pos {
        Some(p) => Ok(HttpResponse::Ok().json(ApiResponse::success(p))),
        None => Err(ApiError::not_found("Storage position")),
    }
}

pub async fn create_storage_position(
    app_state: web::Data<Arc<AppState>>,
    data: web::Json<CreateStoragePositionRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    data.validate()?;

    let zone_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM storage_zones WHERE id = ?")
        .bind(&data.zone_id).fetch_optional(&app_state.db_pool).await?;

    if zone_exists.is_none() { return Err(ApiError::not_found("Storage zone")); }

    let dup: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM storage_positions WHERE zone_id = ? AND LOWER(name) = LOWER(?)"
    ).bind(&data.zone_id).bind(&data.name).fetch_optional(&app_state.db_pool).await?;

    if dup.is_some() { return Err(ApiError::bad_request("A position with this name already exists in this zone")); }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let sort_order = data.sort_order.unwrap_or(0);

    sqlx::query(
        r#"INSERT INTO storage_positions 
           (id, zone_id, name, position_label, max_capacity, current_count,
            sort_order, description, status, created_by, updated_by, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, 0, ?, ?, 'available', ?, ?, ?, ?)"#
    )
    .bind(&id).bind(&data.zone_id).bind(&data.name).bind(&data.position_label).bind(&data.max_capacity)
    .bind(sort_order).bind(&data.description).bind(&user_id).bind(&user_id).bind(&now).bind(&now)
    .execute(&app_state.db_pool).await?;

    let created: StoragePosition = sqlx::query_as("SELECT * FROM storage_positions WHERE id = ?")
        .bind(&id).fetch_one(&app_state.db_pool).await?;

    info!("📦 Created storage position: {} in zone {}", data.name, data.zone_id);
    Ok(HttpResponse::Created().json(ApiResponse::success(created)))
}

pub async fn update_storage_position(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    data: web::Json<UpdateStoragePositionRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    data.validate()?;
    let pos_id = path.into_inner();

    let existing: Option<StoragePosition> = sqlx::query_as("SELECT * FROM storage_positions WHERE id = ?")
        .bind(&pos_id).fetch_optional(&app_state.db_pool).await?;
    let existing = existing.ok_or_else(|| ApiError::not_found("Storage position"))?;

    if let Some(ref new_name) = data.name {
        if new_name.to_lowercase() != existing.name.to_lowercase() {
            let dup: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM storage_positions WHERE zone_id = ? AND LOWER(name) = LOWER(?) AND id != ?"
            ).bind(&existing.zone_id).bind(new_name).bind(&pos_id).fetch_optional(&app_state.db_pool).await?;

            if dup.is_some() { return Err(ApiError::bad_request("A position with this name already exists in this zone")); }
        }
    }

    let now = Utc::now();
    let name = data.name.as_ref().unwrap_or(&existing.name);
    let label = data.position_label.clone().or(existing.position_label);
    let max_cap = data.max_capacity.or(existing.max_capacity);
    let sort_order = data.sort_order.unwrap_or(existing.sort_order);
    let description = data.description.clone().or(existing.description);
    let status = data.status.as_ref().unwrap_or(&existing.status);

    sqlx::query(
        r#"UPDATE storage_positions 
           SET name = ?, position_label = ?, max_capacity = ?, sort_order = ?, description = ?, status = ?,
               updated_by = ?, updated_at = ? WHERE id = ?"#
    )
    .bind(name).bind(&label).bind(&max_cap).bind(sort_order).bind(&description)
    .bind(status).bind(&user_id).bind(&now).bind(&pos_id)
    .execute(&app_state.db_pool).await?;

    let updated: StoragePosition = sqlx::query_as("SELECT * FROM storage_positions WHERE id = ?")
        .bind(&pos_id).fetch_one(&app_state.db_pool).await?;

    info!("📦 Updated storage position: {} ({})", updated.name, pos_id);
    Ok(HttpResponse::Ok().json(ApiResponse::success(updated)))
}

pub async fn delete_storage_position(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let pos_id = path.into_inner();

    let items_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM batch_placements WHERE position_id = ?"
    ).bind(&pos_id).fetch_one(&app_state.db_pool).await?;

    if items_count.0 > 0 {
        return Err(ApiError::bad_request(&format!("Cannot delete: {} items are at this position. Move them first.", items_count.0)));
    }

    let result = sqlx::query("DELETE FROM storage_positions WHERE id = ?").bind(&pos_id).execute(&app_state.db_pool).await?;

    if result.rows_affected() == 0 { return Err(ApiError::not_found("Storage position")); }

    info!("📦 Deleted storage position: {}", pos_id);
    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message((), "Storage position deleted successfully".to_string())))
}

// ============================================================
//              HIERARCHY & LOCATION QUERIES
// ============================================================

pub async fn get_storage_hierarchy(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let rooms: Vec<crate::models::Room> = sqlx::query_as("SELECT * FROM rooms ORDER BY name ASC").fetch_all(&app_state.db_pool).await?;
    let zones: Vec<StorageZone> = sqlx::query_as("SELECT * FROM storage_zones ORDER BY sort_order ASC, name ASC").fetch_all(&app_state.db_pool).await?;
    let positions: Vec<StoragePosition> = sqlx::query_as("SELECT * FROM storage_positions ORDER BY sort_order ASC, name ASC").fetch_all(&app_state.db_pool).await?;

    // ИСПРАВЛЕНИЕ: COUNT(DISTINCT bc.batch_id) через JOIN
    let position_counts: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT bp.position_id, COUNT(DISTINCT bc.batch_id) 
           FROM batch_placements bp
           JOIN batch_containers bc ON bp.container_id = bc.id
           GROUP BY bp.position_id"#
    ).fetch_all(&app_state.db_pool).await?;

    let count_map: std::collections::HashMap<String, i64> = position_counts.into_iter().collect();
    let mut result: Vec<RoomWithStorage> = Vec::new();

    for room in rooms {
        let room_zones: Vec<StorageZoneWithPositions> = zones.iter().filter(|z| z.room_id == room.id).map(|zone| {
            let zone_positions: Vec<StoragePosition> = positions.iter().filter(|p| p.zone_id == zone.id).cloned().collect();
            let items_count: i64 = zone_positions.iter().map(|p| count_map.get(&p.id).copied().unwrap_or(0)).sum();

            StorageZoneWithPositions { zone: zone.clone(), positions: zone_positions, items_count }
        }).collect();

        let total_items: i64 = room_zones.iter().map(|z| z.items_count).sum();
        result.push(RoomWithStorage {
            id: room.id, name: room.name, description: room.description, capacity: room.capacity,
            color: room.color, status: room.status, zones: room_zones, total_items,
        });
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success(result)))
}

pub async fn get_location_path(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let position_id = path.into_inner();
    let location: Option<StorageLocationPath> = sqlx::query_as::<_, (
        String, String, String, String, String, String, String, Option<String>,
    )>(
        r#"SELECT r.id, r.name, sz.id, sz.name, sz.zone_type, 
                  sp.id, sp.name, sp.position_label
           FROM storage_positions sp
           JOIN storage_zones sz ON sp.zone_id = sz.id
           JOIN rooms r ON sz.room_id = r.id
           WHERE sp.id = ?"#
    ).bind(&position_id).fetch_optional(&app_state.db_pool).await?
    .map(|(room_id, room_name, zone_id, zone_name, zone_type, pos_id, pos_name, pos_label)| {
        let full_path = format!("{} → {} → {}", room_name, zone_name, pos_name);
        StorageLocationPath {
            room_id, room_name, zone_id: Some(zone_id), zone_name: Some(zone_name),
            zone_type: Some(zone_type), position_id: Some(pos_id), position_name: Some(pos_name),
            position_label: pos_label, full_path,
        }
    });

    match location {
        Some(loc) => Ok(HttpResponse::Ok().json(ApiResponse::success(loc))),
        None => Err(ApiError::not_found("Storage position")),
    }
}

pub async fn get_position_items(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let pos_id = path.into_inner();

    let pos_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM storage_positions WHERE id = ?")
        .bind(&pos_id).fetch_optional(&app_state.db_pool).await?;

    if pos_exists.is_none() { return Err(ApiError::not_found("Storage position")); }

    // ИСПОЛЬЗУЕМ НОВУЮ СТРУКТУРУ КОНТЕЙНЕРОВ (V3)
    let items: Vec<crate::models::PositionInventoryItem> = sqlx::query_as(
        r#"SELECT
            bc.id as container_id,
            bc.sequence_number,
            bc.quantity as container_quantity,
            bc.is_opened,
            bc.status as container_status,
            sp.id as position_id,
            sp.name as position_name,
            b.id as batch_id,
            b.batch_number,
            b.lot_number,
            b.unit,
            b.quantity as total_quantity,
            b.expiry_date,
            b.status as batch_status,
            rg.id as reagent_id,
            rg.name as reagent_name,
            rg.formula,
            rg.cas_number,
            rg.hazard_pictograms
        FROM batch_placements bp
        JOIN batch_containers bc ON bp.container_id = bc.id
        JOIN storage_positions sp ON bp.position_id = sp.id
        JOIN batches b ON bc.batch_id = b.id
        JOIN reagents rg ON b.reagent_id = rg.id
        WHERE bp.position_id = ? AND b.deleted_at IS NULL AND bc.status != 'disposed'
        ORDER BY rg.name ASC, b.batch_number ASC, bc.sequence_number ASC"#
    )
    .bind(&pos_id)
    .fetch_all(&app_state.db_pool)
    .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(items)))
}

/// GET /api/storage/zones/{id}/items
/// Get all batches stored in all positions of a given zone
pub async fn get_zone_items(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let zone_id = path.into_inner();

    let zone_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM storage_zones WHERE id = ?")
        .bind(&zone_id).fetch_optional(&app_state.db_pool).await?;

    if zone_exists.is_none() { return Err(ApiError::not_found("Storage zone")); }

    // ИСПОЛЬЗУЕМ НОВУЮ СТРУКТУРУ КОНТЕЙНЕРОВ (V3)
    let items: Vec<crate::models::PositionInventoryItem> = sqlx::query_as(
        r#"SELECT
            bc.id as container_id,
            bc.sequence_number,
            bc.quantity as container_quantity,
            bc.is_opened,
            bc.status as container_status,
            sp.id as position_id,
            sp.name as position_name,
            b.id as batch_id,
            b.batch_number,
            b.lot_number,
            b.unit,
            b.quantity as total_quantity,
            b.expiry_date,
            b.status as batch_status,
            rg.id as reagent_id,
            rg.name as reagent_name,
            rg.formula,
            rg.cas_number,
            rg.hazard_pictograms
        FROM batch_placements bp
        JOIN batch_containers bc ON bp.container_id = bc.id
        JOIN storage_positions sp ON bp.position_id = sp.id
        JOIN batches b ON bc.batch_id = b.id
        JOIN reagents rg ON b.reagent_id = rg.id
        WHERE sp.zone_id = ? AND b.deleted_at IS NULL AND bc.status != 'disposed'
        ORDER BY sp.sort_order ASC, sp.name ASC, rg.name ASC, b.batch_number ASC, bc.sequence_number ASC"#
    )
    .bind(&zone_id)
    .fetch_all(&app_state.db_pool)
    .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(items)))
}

/// Utility for updating position count cache
#[allow(dead_code)]
pub async fn update_position_count(pool: &SqlitePool, position_id: &str) -> ApiResult<()> {
    // ИСПРАВЛЕНИЕ: Подсчет уникальных батчей через batch_containers
    sqlx::query(
        r#"UPDATE storage_positions 
           SET current_count = (
               SELECT COUNT(DISTINCT bc.batch_id) 
               FROM batch_placements bp 
               JOIN batch_containers bc ON bp.container_id = bc.id 
               WHERE bp.position_id = ?
           )
           WHERE id = ?"#
    ).bind(position_id).bind(position_id).execute(pool).await?;
    Ok(())
}

pub async fn search_storage_locations(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<HttpResponse> {
    let search = query.get("q").map(|s| s.as_str()).unwrap_or("");
    if search.is_empty() { return Err(ApiError::bad_request("Search query 'q' is required")); }

    let pattern = format!("%{}%", search);
    let results: Vec<serde_json::Value> = sqlx::query_as::<_, (
        String, String, Option<String>, String, String, String, String, String,
    )>(
        r#"SELECT sp.id, sp.name, sp.position_label, sz.name, sz.zone_type,
                  r.name, r.id, sz.id
           FROM storage_positions sp
           JOIN storage_zones sz ON sp.zone_id = sz.id
           JOIN rooms r ON sz.room_id = r.id
           WHERE sp.name LIKE ? OR sp.position_label LIKE ? OR sz.name LIKE ? OR r.name LIKE ?
           ORDER BY r.name, sz.sort_order, sp.sort_order LIMIT 20"#
    )
    .bind(&pattern).bind(&pattern).bind(&pattern).bind(&pattern)
    .fetch_all(&app_state.db_pool).await?.into_iter()
    .map(|(pos_id, pos_name, pos_label, zone_name, zone_type, room_name, room_id, zone_id)| {
        let full_path = format!("{} → {} → {}", room_name, zone_name, pos_name);
        serde_json::json!({
            "position_id": pos_id, "position_name": pos_name, "position_label": pos_label,
            "zone_id": zone_id, "zone_name": zone_name, "zone_type": zone_type,
            "room_id": room_id, "room_name": room_name, "full_path": full_path,
        })
    }).collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(results)))
}