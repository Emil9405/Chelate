// src/container_handlers.rs
//! Handlers for batch containers — physical jars/bottles
//!
//! Endpoints:
//!   GET    /api/batches/{batch_id}/containers              — list containers for batch
//!   POST   /api/batches/{batch_id}/containers              — create single container
//!   POST   /api/batches/{batch_id}/containers/split        — auto-split batch by pack_size
//!   POST   /api/containers/{container_id}/place            — place container at position
//!   PUT    /api/containers/{container_id}/move              — move container to new position
//!   DELETE /api/containers/{container_id}/unplace           — remove from position (back to unplaced)
//!   POST   /api/containers/{container_id}/use               — dispense from container
//!   DELETE /api/containers/{container_id}                   — dispose of empty container

use actix_web::{web, HttpResponse, HttpRequest};
use std::sync::Arc;
use crate::AppState;
use crate::models::*;
use crate::error::{ApiError, ApiResult};
use crate::handlers::ApiResponse;
use crate::auth::get_current_user;
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;
use serde::{Serialize, Deserialize};
use log::info;

// ==================== SQL FRAGMENTS ====================

/// SELECT for ContainerWithLocation: LEFT JOIN to get placement + location
const CONTAINER_WITH_LOCATION_SELECT: &str = r#"
    SELECT
        bc.id,
        bc.batch_id,
        bc.sequence_number,
        bc.quantity,
        bc.original_quantity,
        bc.is_opened,
        bc.opened_at,
        bc.opened_by,
        bc.status as container_status,
        bc.notes as container_notes,
        bc.created_at as container_created_at,
        bc.updated_at as container_updated_at,
        bp.id as placement_id,
        bp.position_id,
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

// ==================== HELPERS ====================

/// Fetch batch or 404
async fn get_batch_or_404(pool: &sqlx::SqlitePool, batch_id: &str) -> ApiResult<Batch> {
    sqlx::query_as::<_, Batch>(
        "SELECT * FROM batches WHERE id = ? AND deleted_at IS NULL"
    )
    .bind(batch_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::batch_not_found(batch_id))
}

/// Fetch container or 404
async fn get_container_or_404(pool: &sqlx::SqlitePool, container_id: &str) -> ApiResult<BatchContainer> {
    sqlx::query_as::<_, BatchContainer>(
        "SELECT * FROM batch_containers WHERE id = ?"
    )
    .bind(container_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::not_found("Container"))
}

/// Validate position exists
async fn validate_position(pool: &sqlx::SqlitePool, position_id: &str) -> ApiResult<StoragePosition> {
    sqlx::query_as::<_, StoragePosition>(
        "SELECT * FROM storage_positions WHERE id = ?"
    )
    .bind(position_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::not_found("Storage position"))
}

/// Compute container status from quantity vs original
fn compute_container_status(quantity: f64, original_quantity: f64) -> &'static str {
    if quantity <= 0.001 {
        "empty"
    } else if (quantity - original_quantity).abs() < 0.001 {
        "full"
    } else {
        "partial"
    }
}

/// Get next sequence number for batch
async fn next_sequence(pool: &sqlx::SqlitePool, batch_id: &str) -> ApiResult<i64> {
    let result: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sequence_number), 0) FROM batch_containers WHERE batch_id = ?"
    )
    .bind(batch_id)
    .fetch_one(pool)
    .await?;
    Ok(result.0 + 1)
}

/// Container stats for a batch
async fn get_container_stats(pool: &sqlx::SqlitePool, batch_id: &str) -> ApiResult<ContainerStats> {
    #[derive(sqlx::FromRow)]
    struct RawStats {
        container_count: i64,
        opened_count: i64,
        empty_count: i64,
        placed_count: i64,
    }

    let stats: RawStats = sqlx::query_as(
        r#"SELECT
            COUNT(*) as container_count,
            SUM(CASE WHEN bc.is_opened = 1 THEN 1 ELSE 0 END) as opened_count,
            SUM(CASE WHEN bc.status = 'empty' THEN 1 ELSE 0 END) as empty_count,
            SUM(CASE WHEN bp.id IS NOT NULL THEN 1 ELSE 0 END) as placed_count
        FROM batch_containers bc
        LEFT JOIN batch_placements bp ON bp.container_id = bc.id
        WHERE bc.batch_id = ?"#
    )
    .bind(batch_id)
    .fetch_one(pool)
    .await?;

    Ok(ContainerStats {
        container_count: stats.container_count,
        opened_count: stats.opened_count,
        sealed_count: stats.container_count - stats.opened_count,
        placed_count: stats.placed_count,
        unplaced_count: stats.container_count - stats.placed_count,
        empty_count: stats.empty_count,
    })
}

// ==================== RESPONSE ====================

#[derive(Debug, Serialize)]
pub struct BatchContainersResponse {
    pub batch_id: String,
    pub batch_quantity: f64,
    pub unit: String,
    pub pack_size: Option<f64>,
    pub stats: ContainerStats,
    pub containers: Vec<ContainerWithLocation>,
}
// ==================== REQUESTS ====================
#[derive(Debug, Deserialize)]
    pub struct BulkPlaceRequest {
    pub container_ids: Vec<String>,
    pub position_id: String,
}
#[derive(Debug, Deserialize)]
pub struct BulkMoveRequest {
    pub container_ids: Vec<String>,
    pub new_position_id: String,
}
// ==================== LIST CONTAINERS FOR BATCH ====================

pub async fn get_batch_containers(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let batch_id = path.into_inner();
    let batch = get_batch_or_404(&app_state.db_pool, &batch_id).await?;

    let sql = format!(
        "{} WHERE bc.batch_id = ? ORDER BY bc.sequence_number",
        CONTAINER_WITH_LOCATION_SELECT
    );
    let containers: Vec<ContainerWithLocation> = sqlx::query_as(&sql)
        .bind(&batch_id)
        .fetch_all(&app_state.db_pool)
        .await?;

    let stats = get_container_stats(&app_state.db_pool, &batch_id).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(BatchContainersResponse {
        batch_id: batch.id,
        batch_quantity: batch.quantity,
        unit: batch.unit,
        pack_size: batch.pack_size,
        stats,
        containers,
    })))
}

// ==================== CREATE SINGLE CONTAINER ====================

pub async fn create_container(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<CreateContainerRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    request.validate()?;
    let batch_id = path.into_inner();
    let _claims = get_current_user(&http_request)?;
    let batch = get_batch_or_404(&app_state.db_pool, &batch_id).await?;
    let now = Utc::now();

    // Check: sum of container quantities must not exceed batch quantity
    let existing_sum: (f64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(quantity), 0.0) FROM batch_containers WHERE batch_id = ?"
    )
    .bind(&batch_id)
    .fetch_one(&app_state.db_pool)
    .await?;

    if existing_sum.0 + request.quantity > batch.quantity + 0.001 {
        return Err(ApiError::bad_request(&format!(
            "Cannot create container with {:.2} {}. Already in containers: {:.2}, batch total: {:.2}",
            request.quantity, batch.unit, existing_sum.0, batch.quantity
        )));
    }

    let seq = next_sequence(&app_state.db_pool, &batch_id).await?;
    let id = Uuid::new_v4().to_string();

    sqlx::query(
        r#"INSERT INTO batch_containers
            (id, batch_id, sequence_number, quantity, original_quantity, is_opened, status, notes, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, 0, 'full', ?, ?, ?)"#
    )
    .bind(&id)
    .bind(&batch_id)
    .bind(seq)
    .bind(request.quantity)
    .bind(request.quantity)
    .bind(&request.notes)
    .bind(&now)
    .bind(&now)
    .execute(&app_state.db_pool)
    .await?;

    let sql = format!("{} WHERE bc.id = ?", CONTAINER_WITH_LOCATION_SELECT);
    let created: ContainerWithLocation = sqlx::query_as(&sql)
        .bind(&id)
        .fetch_one(&app_state.db_pool)
        .await?;

    info!("📦 Container #{} created for batch {} ({:.2} {})", seq, batch.batch_number, request.quantity, batch.unit);

    Ok(HttpResponse::Created().json(ApiResponse::success(created)))
}

// ==================== SPLIT BATCH INTO CONTAINERS ====================

pub async fn split_batch_into_containers(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<SplitBatchRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    request.validate()?;
    let batch_id = path.into_inner();
    let _claims = get_current_user(&http_request)?;
    let batch = get_batch_or_404(&app_state.db_pool, &batch_id).await?;
    let now = Utc::now();

    // Check: no existing containers (split is a one-time operation)
    let existing_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM batch_containers WHERE batch_id = ?"
    )
    .bind(&batch_id)
    .fetch_one(&app_state.db_pool)
    .await?;

    if existing_count.0 > 0 {
        return Err(ApiError::bad_request(
            "Batch already has containers. Use create_container to add more, or delete existing ones first."
        ));
    }

    let pack_size = request.pack_size;
    let total = batch.quantity;
    let n_units = (total / pack_size).ceil() as i64;

    if n_units > 1000 {
        return Err(ApiError::bad_request(
            "Split would create more than 1000 containers. Check pack_size value."
        ));
    }

    let mut tx = app_state.db_pool.begin().await?;
    let mut created_ids: Vec<String> = Vec::with_capacity(n_units as usize);

    let mut remaining = total;
    for seq in 1..=n_units {
        let qty = if remaining >= pack_size {
            pack_size
        } else {
            remaining // last container gets the remainder
        };
        remaining -= qty;

        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"INSERT INTO batch_containers
                (id, batch_id, sequence_number, quantity, original_quantity, is_opened, status, notes, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, 0, 'full', NULL, ?, ?)"#
        )
        .bind(&id)
        .bind(&batch_id)
        .bind(seq)
        .bind(qty)
        .bind(qty)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        created_ids.push(id);
    }

    // Update batch pack_size if it wasn't set
    if batch.pack_size.is_none() {
        sqlx::query("UPDATE batches SET pack_size = ?, updated_at = ? WHERE id = ?")
            .bind(pack_size)
            .bind(&now)
            .bind(&batch_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    info!(
        "📦 Batch {} split into {} containers (pack_size={:.2} {})",
        batch.batch_number, n_units, pack_size, batch.unit
    );

    // Return full list
    let sql = format!(
        "{} WHERE bc.batch_id = ? ORDER BY bc.sequence_number",
        CONTAINER_WITH_LOCATION_SELECT
    );
    let containers: Vec<ContainerWithLocation> = sqlx::query_as(&sql)
        .bind(&batch_id)
        .fetch_all(&app_state.db_pool)
        .await?;

    let stats = get_container_stats(&app_state.db_pool, &batch_id).await?;

    Ok(HttpResponse::Created().json(ApiResponse::success_with_message(
        BatchContainersResponse {
            batch_id: batch.id,
            batch_quantity: batch.quantity,
            unit: batch.unit,
            pack_size: Some(pack_size),
            stats,
            containers,
        },
        format!("Batch split into {} containers", n_units),
    )))
}

// ==================== PLACE CONTAINER AT POSITION ====================

pub async fn place_container(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<PlaceContainerRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let container_id = path.into_inner();
    let claims = get_current_user(&http_request)?;
    let now = Utc::now();

    let container = get_container_or_404(&app_state.db_pool, &container_id).await?;
    let position = validate_position(&app_state.db_pool, &request.position_id).await?;

    // Check: container not already placed
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM batch_placements WHERE container_id = ?"
    )
    .bind(&container_id)
    .fetch_optional(&app_state.db_pool)
    .await?;

    if existing.is_some() {
        return Err(ApiError::bad_request(
            "Container is already placed. Use move to change position, or unplace first."
        ));
    }

    let placement_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT INTO batch_placements (id, container_id, position_id, placed_by, placed_at, notes)
           VALUES (?, ?, ?, ?, ?, ?)"#
    )
    .bind(&placement_id)
    .bind(&container_id)
    .bind(&request.position_id)
    .bind(&claims.sub)
    .bind(&now)
    .bind(&request.notes)
    .execute(&app_state.db_pool)
    .await?;

    let sql = format!("{} WHERE bc.id = ?", CONTAINER_WITH_LOCATION_SELECT);
    let result: ContainerWithLocation = sqlx::query_as(&sql)
        .bind(&container_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    info!("📍 Container #{} placed at {}", container.sequence_number, position.name);

    Ok(HttpResponse::Ok().json(ApiResponse::success(result)))
}

 //==================== BULK PLACE & MOVE CONTAINERS ====================

pub async fn place_containers_bulk(
    app_state: web::Data<Arc<AppState>>,
    request: web::Json<BulkPlaceRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;
    let now = Utc::now();
    let mut tx = app_state.db_pool.begin().await?;
    let mut placed = 0i64;

    for cid in &request.container_ids {
        // Skip already placed
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM batch_placements WHERE container_id = ?"
        ).bind(cid).fetch_optional(&mut *tx).await?;

        if existing.is_some() { continue; }

        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO batch_placements (id, container_id, position_id, placed_by, placed_at, notes) VALUES (?, ?, ?, ?, ?, NULL)"
        )
        .bind(&id).bind(cid).bind(&request.position_id)
        .bind(&claims.sub).bind(&now)
        .execute(&mut *tx).await?;
        placed += 1;
    }

    tx.commit().await?;
    info!("📍 Bulk placed {} containers at position {}", placed, request.position_id);

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        serde_json::json!({ "placed": placed }),
        format!("Placed {} containers", placed),
    )))
}

pub async fn move_containers_bulk(
    app_state: web::Data<Arc<AppState>>,
    request: web::Json<BulkMoveRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;
    let now = Utc::now();
    let mut tx = app_state.db_pool.begin().await?;
    let mut moved = 0i64;

    for cid in &request.container_ids {
        let result = sqlx::query(
            "UPDATE batch_placements SET position_id = ?, placed_by = ?, placed_at = ? WHERE container_id = ?"
        )
        .bind(&request.new_position_id).bind(&claims.sub).bind(&now).bind(cid)
        .execute(&mut *tx).await?;
        moved += result.rows_affected() as i64;
    }

    tx.commit().await?;
    info!("📍 Bulk moved {} containers to position {}", moved, request.new_position_id);

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        serde_json::json!({ "moved": moved }),
        format!("Moved {} containers", moved),
    )))
}

// ==================== MOVE CONTAINER ====================

pub async fn move_container(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<MoveContainerRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let container_id = path.into_inner();
    let claims = get_current_user(&http_request)?;
    let now = Utc::now();

    let container = get_container_or_404(&app_state.db_pool, &container_id).await?;
    let new_position = validate_position(&app_state.db_pool, &request.new_position_id).await?;

    // Find existing placement
    let placement: BatchPlacement = sqlx::query_as(
        "SELECT * FROM batch_placements WHERE container_id = ?"
    )
    .bind(&container_id)
    .fetch_one(&app_state.db_pool)
    .await
    .map_err(|_| ApiError::bad_request("Container is not placed anywhere. Use place first."))?;

    // Just update position_id — no quantity logic needed
    sqlx::query(
        "UPDATE batch_placements SET position_id = ?, placed_by = ?, placed_at = ? WHERE id = ?"
    )
    .bind(&request.new_position_id)
    .bind(&claims.sub)
    .bind(&now)
    .bind(&placement.id)
    .execute(&app_state.db_pool)
    .await?;

    let sql = format!("{} WHERE bc.id = ?", CONTAINER_WITH_LOCATION_SELECT);
    let result: ContainerWithLocation = sqlx::query_as(&sql)
        .bind(&container_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    info!(
        "📍 Container #{} moved to {}",
        container.sequence_number, new_position.name
    );

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        result,
        format!("Container moved to {}", new_position.name),
    )))
}

// ==================== UNPLACE CONTAINER ====================

pub async fn unplace_container(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let container_id = path.into_inner();
    let _claims = get_current_user(&http_request)?;

    let result = sqlx::query("DELETE FROM batch_placements WHERE container_id = ?")
        .bind(&container_id)
        .execute(&app_state.db_pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::bad_request("Container is not placed anywhere."));
    }

    info!("📍 Container {} unplaced", container_id);

    Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_with_message(
        (),
        "Container removed from position".to_string(),
    )))
}

// ==================== USE FROM CONTAINER (DISPENSE) ====================

pub async fn use_from_container(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<UseFromContainerRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    request.validate()?;
    let container_id = path.into_inner();
    let claims = get_current_user(&http_request)?;
    let now = Utc::now();

    let container = get_container_or_404(&app_state.db_pool, &container_id).await?;
    let batch = get_batch_or_404(&app_state.db_pool, &container.batch_id).await?;

    // Validate: enough in this container
    if request.quantity > container.quantity + 0.001 {
        return Err(ApiError::bad_request(&format!(
            "Insufficient quantity in container #{}: available {:.2} {}, requested {:.2}",
            container.sequence_number, container.quantity, batch.unit, request.quantity
        )));
    }

    let mut tx = app_state.db_pool.begin().await?;

    // 1. Update container quantity + mark opened
    let new_container_qty = (container.quantity - request.quantity).max(0.0);
    let new_status = compute_container_status(new_container_qty, container.original_quantity);

    if !container.is_opened {
        // First time opening
        sqlx::query(
            r#"UPDATE batch_containers 
               SET quantity = ?, is_opened = 1, opened_at = ?, opened_by = ?,
                   status = ?, updated_at = ?
               WHERE id = ?"#
        )
        .bind(new_container_qty)
        .bind(&now)
        .bind(&claims.sub)
        .bind(new_status)
        .bind(&now)
        .bind(&container_id)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE batch_containers SET quantity = ?, status = ?, updated_at = ? WHERE id = ?"
        )
        .bind(new_container_qty)
        .bind(new_status)
        .bind(&now)
        .bind(&container_id)
        .execute(&mut *tx)
        .await?;
    }

    // 2. Sync batch total quantity
    let new_batch_qty = (batch.quantity - request.quantity).max(0.0);
    let batch_status = if new_batch_qty <= 0.0 {
        "depleted"
    } else if let Some(ps) = batch.pack_size {
        if new_batch_qty <= ps { "low_stock" } else { "available" }
    } else {
        "available"
    };

    sqlx::query(
        "UPDATE batches SET quantity = ?, status = ?, updated_by = ?, updated_at = ? WHERE id = ?"
    )
    .bind(new_batch_qty)
    .bind(batch_status)
    .bind(&claims.sub)
    .bind(&now)
    .bind(&batch.id)
    .execute(&mut *tx)
    .await?;

    // 3. Log usage
    let usage_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT INTO usage_logs (id, reagent_id, batch_id, user_id, quantity_used, unit, purpose, notes, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(&usage_id)
    .bind(&batch.reagent_id)
    .bind(&batch.id)
    .bind(&claims.sub)
    .bind(request.quantity)
    .bind(&batch.unit)
    .bind(&request.purpose)
    .bind(&request.notes)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    info!(
        "📦 Used {:.2} {} from container #{} (batch {}). Container: {:.2} → {:.2}. Batch: {:.2} → {:.2}",
        request.quantity, batch.unit,
        container.sequence_number, batch.batch_number,
        container.quantity, new_container_qty,
        batch.quantity, new_batch_qty,
    );

    #[derive(Serialize)]
    struct UseResponse {
        usage_id: String,
        container_id: String,
        quantity_used: f64,
        container_remaining: f64,
        container_status: String,
        is_opened: bool,
        batch_remaining: f64,
        batch_status: String,
        unit: String,
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        UseResponse {
            usage_id,
            container_id: container.id,
            quantity_used: request.quantity,
            container_remaining: new_container_qty,
            container_status: new_status.to_string(),
            is_opened: true,
            batch_remaining: new_batch_qty,
            batch_status: batch_status.to_string(),
            unit: batch.unit,
        },
        format!("Dispensed {:.2} from container #{}", request.quantity, container.sequence_number),
    )))
}

// ==================== DISPOSE EMPTY CONTAINER ====================

pub async fn dispose_container(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let container_id = path.into_inner();
    let _claims = get_current_user(&http_request)?;
    let now = Utc::now();

    let container = get_container_or_404(&app_state.db_pool, &container_id).await?;

    if container.quantity > 0.001 {
        return Err(ApiError::bad_request(&format!(
            "Container still has {:.2} remaining. Use or empty it before disposing.",
            container.quantity
        )));
    }

    // Remove placement if exists
    sqlx::query("DELETE FROM batch_placements WHERE container_id = ?")
        .bind(&container_id)
        .execute(&app_state.db_pool)
        .await?;

    // Mark as disposed (or delete — your choice)
    sqlx::query("UPDATE batch_containers SET status = 'disposed', updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&container_id)
        .execute(&app_state.db_pool)
        .await?;

    info!("🗑️ Container #{} disposed", container.sequence_number);

    Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_with_message(
        (),
        "Empty container disposed".to_string(),
    )))
}

// ==================== ROOM INVENTORY (updated for containers) ====================

pub async fn get_room_inventory(
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

    let inventory: Vec<PositionInventoryItem> = sqlx::query_as(
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
        JOIN storage_zones sz ON sp.zone_id = sz.id
        JOIN batches b ON bc.batch_id = b.id
        JOIN reagents rg ON b.reagent_id = rg.id
        WHERE sz.room_id = ? AND b.deleted_at IS NULL AND bc.status != 'disposed'
        ORDER BY sz.sort_order, sp.sort_order, rg.name, b.batch_number, bc.sequence_number"#
    )
    .bind(&room_id)
    .fetch_all(&app_state.db_pool)
    .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(inventory)))
}
