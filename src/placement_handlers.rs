// src/placement_handlers.rs
//! Placement handlers (v3 — container-based)
//! Most logic moved to container_handlers.rs
//!
//! Remaining endpoints for backward compatibility:
//!   GET  /api/batches/{batch_id}/placements  — containers with locations for a batch
//!   GET  /api/rooms/{room_id}/placements     — all placed containers in a room
//!   GET  /api/rooms/{room_id}/inventory       — room inventory (delegated to container_handlers)
//!
//! New container-level endpoints are in container_handlers.rs

use actix_web::{web, HttpResponse};
use std::sync::Arc;
use crate::AppState;
use crate::models::*;
use crate::error::{ApiError, ApiResult};
use crate::handlers::ApiResponse;
use serde::Serialize;

// ==================== SQL for PlacementWithLocation (v3) ====================

/// SELECT containers with LEFT JOIN to placement + location
/// Returns PlacementWithLocation (backward-compat PlacementWithRoom)
const CONTAINER_PLACEMENT_SELECT: &str = r#"
    SELECT
        bc.id as container_id,
        bc.sequence_number,
        bc.quantity as container_quantity,
        bc.is_opened,
        bc.status as container_status,
        bp.id as placement_id,
        bp.position_id,
        bp.placed_by,
        sp.name as position_name,
        sp.position_label,
        sz.id as zone_id,
        sz.name as zone_name,
        sz.zone_type,
        rm.id as room_id,
        rm.name as room_name,
        rm.color as room_color
    FROM batch_containers bc
    LEFT JOIN batch_placements bp ON bp.container_id = bc.id
    LEFT JOIN storage_positions sp ON bp.position_id = sp.id
    LEFT JOIN storage_zones sz ON sp.zone_id = sz.id
    LEFT JOIN rooms rm ON sz.room_id = rm.id
"#;

// ==================== RESPONSE ====================

#[derive(Debug, Serialize)]
pub struct BatchPlacementsResponse {
    pub batch_id: String,
    pub total_quantity: f64,
    pub unit: String,
    pub container_count: i64,
    pub placed_count: i64,
    pub unplaced_count: i64,
    pub containers: Vec<PlacementWithRoom>,
}

// ==================== GET PLACEMENTS FOR BATCH ====================
// Backward-compat: returns containers with their placement info

pub async fn get_batch_placements(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let batch_id = path.into_inner();

    let batch: Batch = sqlx::query_as(
        "SELECT * FROM batches WHERE id = ? AND deleted_at IS NULL"
    )
    .bind(&batch_id)
    .fetch_one(&app_state.db_pool)
    .await
    .map_err(|_| ApiError::batch_not_found(&batch_id))?;

    let sql = format!(
        "{} WHERE bc.batch_id = ? ORDER BY bc.sequence_number",
        CONTAINER_PLACEMENT_SELECT
    );
    let containers: Vec<PlacementWithRoom> = sqlx::query_as(&sql)
        .bind(&batch_id)
        .fetch_all(&app_state.db_pool)
        .await?;

    let total = containers.len() as i64;
    let placed = containers.iter().filter(|c| c.is_placed()).count() as i64;

    Ok(HttpResponse::Ok().json(ApiResponse::success(BatchPlacementsResponse {
        batch_id: batch.id,
        total_quantity: batch.quantity,
        unit: batch.unit,
        container_count: total,
        placed_count: placed,
        unplaced_count: total - placed,
        containers,
    })))
}

// ==================== GET PLACEMENTS FOR ROOM ====================

pub async fn get_room_placements(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let room_id = path.into_inner();

    // Validate room exists
    sqlx::query_as::<_, Room>("SELECT * FROM rooms WHERE id = ?")
        .bind(&room_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Room"))?;

    // Only placed containers (INNER JOIN on batch_placements)
    let placements: Vec<PlacementWithRoom> = sqlx::query_as(
        r#"SELECT
            bc.id as container_id,
            bc.sequence_number,
            bc.quantity as container_quantity,
            bc.is_opened,
            bc.status as container_status,
            bp.id as placement_id,
            bp.position_id,
            bp.placed_by,
            sp.name as position_name,
            sp.position_label,
            sz.id as zone_id,
            sz.name as zone_name,
            sz.zone_type,
            rm.id as room_id,
            rm.name as room_name,
            rm.color as room_color
        FROM batch_containers bc
        JOIN batch_placements bp ON bp.container_id = bc.id
        JOIN storage_positions sp ON bp.position_id = sp.id
        JOIN storage_zones sz ON sp.zone_id = sz.id
        JOIN rooms rm ON sz.room_id = rm.id
        WHERE rm.id = ? AND bc.status != 'disposed'
        ORDER BY sz.sort_order, sp.sort_order, bc.sequence_number"#
    )
    .bind(&room_id)
    .fetch_all(&app_state.db_pool)
    .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(placements)))
}
