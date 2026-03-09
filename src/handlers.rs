// src/handlers.rs
use actix_web::{web, HttpResponse, HttpRequest};
use uuid::Uuid;
use validator::Validate;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use crate::jwt_rotation::{get_rotation_stats, rotate_jwt_secret};
use chrono::{DateTime, Utc};
use crate::AppState;
use crate::models::{Reagent, Batch};
use crate::error::{ApiError, ApiResult, validate_quantity};
use crate::auth::get_current_user;
use crate::audit::ChangeSet;
use std::env;

// ==================== COMMON STRUCTURES ====================

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

// ==================== ENHANCED PAGINATION STRUCTURES ====================

/// Расширенный ответ с пагинацией и информацией о сортировке
#[derive(Debug, Serialize)]
pub struct PaginatedResponseWithSort<T> {
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
    pub sorting: SortingInfo,
}

/// Информация о пагинации
#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
    pub has_next: bool,
    pub has_prev: bool,
}

impl PaginationInfo {
    pub fn new(total: i64, page: i64, per_page: i64) -> Self {
        let total_pages = if total == 0 { 1 } else { (total + per_page - 1) / per_page };
        Self {
            total,
            page,
            per_page,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

/// Информация о сортировке
#[derive(Debug, Serialize)]
pub struct SortingInfo {
    pub sort_by: String,
    pub sort_order: String,
}

/// Cursor-based пагинация для больших данных
#[derive(Debug, Serialize)]
pub struct CursorPaginatedResponse<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
    pub has_more: bool,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
    pub status: Option<String>,
    pub manufacturer: Option<String>,
    pub cas_number: Option<String>,
    pub has_stock: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub category: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
}

impl PaginationQuery {
    pub fn normalize(&self) -> (i64, i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * per_page;
        (page, per_page, offset)
    }
}

#[derive(Debug, Serialize)]
pub struct ReagentWithBatches {
    #[serde(flatten)]
    pub reagent: Reagent,
    pub batches: Vec<Batch>,
}

// ==================== REAGENT WITH BATCHES ====================

pub async fn get_reagent_with_batches(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let reagent_id = path.into_inner();

    let reagent: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ? AND deleted_at IS NULL")
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::reagent_not_found(&reagent_id))?;

    let batches: Vec<Batch> = sqlx::query_as(
        "SELECT * FROM batches WHERE reagent_id = ? ORDER BY received_date DESC"
    )
        .bind(&reagent_id)
        .fetch_all(&app_state.db_pool)
        .await?;

    let response = ReagentWithBatches { reagent, batches };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

// ==================== USAGE TRACKING ====================

#[derive(Debug, Deserialize, Validate)]
pub struct UseReagentRequest {
    #[validate(range(min = 0.0, message = "Quantity must be positive"))]
    pub quantity_used: f64,
    #[validate(length(max = 500, message = "Purpose cannot exceed 500 characters"))]
    pub purpose: Option<String>,
    #[validate(length(max = 1000, message = "Notes cannot exceed 1000 characters"))]
    pub notes: Option<String>,
    /// Optional: placement_id to deduct from specific room location
    pub placement_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UsageLog {
    pub id: String,
    pub batch_id: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub quantity_used: f64,
    pub unit: Option<String>,
    pub purpose: Option<String>,
    pub notes: Option<String>,
    pub used_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

pub async fn use_reagent(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    request: web::Json<UseReagentRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let (reagent_id, batch_id) = path.into_inner();
    request.validate()?;

    let claims = get_current_user(&http_request)?;
    validate_quantity(request.quantity_used)?;

    let reagent: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ? AND deleted_at IS NULL")
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::reagent_not_found(&reagent_id))?;

    let batch: Batch = sqlx::query_as("SELECT * FROM batches WHERE id = ? AND reagent_id = ?")
        .bind(&batch_id)
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::batch_not_found(&batch_id))?;

    if batch.status != "available" {
        return Err(ApiError::BadRequest("Batch is not available for use".to_string()));
    }

    if request.quantity_used > batch.quantity {
        return Err(ApiError::insufficient_quantity(batch.quantity, request.quantity_used));
    }

    let now = Utc::now();
    let usage_id = Uuid::new_v4().to_string();
    let mut tx = app_state.db_pool.begin().await?;

    sqlx::query(
        r#"INSERT INTO usage_logs (id, reagent_id, batch_id, user_id, quantity_used, unit, purpose, notes, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
        .bind(&usage_id)
        .bind(&reagent_id)
        .bind(&batch_id)
        .bind(&claims.sub)
        .bind(request.quantity_used)
        .bind(&batch.unit)
        .bind(&request.purpose)
        .bind(&request.notes)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

    let new_quantity = batch.quantity - request.quantity_used;
    let new_status = if new_quantity <= 0.0 { "depleted" } else { "available" };

    sqlx::query("UPDATE batches SET quantity = ?, status = ?, updated_at = ? WHERE id = ?")
        .bind(new_quantity.max(0.0))
        .bind(new_status)
        .bind(&now)
        .bind(&batch_id)
        .execute(&mut *tx)
        .await?;

    // Deduct from specific placement if placement_id is provided
    if let Some(ref placement_id) = request.placement_id {
        let placement: Option<(String, f64)> = sqlx::query_as(
            "SELECT id, quantity FROM batch_placements WHERE id = ? AND batch_id = ?"
        )
        .bind(placement_id)
        .bind(&batch_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some((_pid, p_qty)) = placement {
            let new_p_qty = p_qty - request.quantity_used;
            if new_p_qty <= 0.001 {
                // Remove empty placement
                sqlx::query("DELETE FROM batch_placements WHERE id = ?")
                    .bind(placement_id)
                    .execute(&mut *tx)
                    .await?;
            } else {
                sqlx::query("UPDATE batch_placements SET quantity = ?, updated_at = ? WHERE id = ?")
                    .bind(new_p_qty.max(0.0))
                    .bind(&now)
                    .bind(placement_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        // If placement not found, still proceed (backward compat — batch total was already decremented)
    }

    tx.commit().await?;

    // Detailed audit with reagent name, batch number, and quantity change
    let mut cs = ChangeSet::new();
    cs.add_f64("quantity", batch.quantity, new_quantity.max(0.0));
    if batch.status != new_status {
        cs.add("status", &batch.status, new_status);
    }

    crate::audit::audit_with_changes(
        &app_state.db_pool, &claims.sub, "use_reagent", "batch", &batch_id,
        &format!(
            "Used {} {} from reagent \"{}\" batch {} (remaining: {} {})",
            request.quantity_used, batch.unit, reagent.name, batch.batch_number,
            new_quantity.max(0.0), batch.unit
        ),
        &cs, &http_request,
    ).await;

    log::info!(
        "User {} used {} {} from reagent \"{}\" batch {} (reagent_id: {}, batch_id: {})",
        claims.username, request.quantity_used, batch.unit, reagent.name, batch.batch_number, reagent_id, batch_id
    );

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        serde_json::json!({
            "usage_id": usage_id,
            "remaining_quantity": new_quantity.max(0.0),
            "status": new_status
        }),
        "Reagent usage recorded successfully".to_string(),
    )))
}

pub async fn get_usage_history(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    query: web::Query<PaginationQuery>,
) -> ApiResult<HttpResponse> {
    let (reagent_id, batch_id) = path.into_inner();
    let (page, per_page, offset) = query.normalize();

    let _reagent: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ? AND deleted_at IS NULL")
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::reagent_not_found(&reagent_id))?;

    let _batch: Batch = sqlx::query_as("SELECT * FROM batches WHERE id = ? AND reagent_id = ?")
        .bind(&batch_id)
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::batch_not_found(&batch_id))?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM usage_logs WHERE batch_id = ?")
        .bind(&batch_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    let usage_logs: Vec<UsageLog> = sqlx::query_as(
        r#"SELECT
            ul.id,
            ul.batch_id,
            ul.user_id,
            u.username as username,
            ul.quantity_used,
            b.unit as unit,
            ul.purpose,
            ul.notes,
            ul.created_at as used_at,
            ul.created_at
           FROM usage_logs ul
           LEFT JOIN users u ON ul.user_id = u.id
           LEFT JOIN batches b ON ul.batch_id = b.id
           WHERE ul.batch_id = ?
           ORDER BY ul.created_at DESC
           LIMIT ? OFFSET ?"#
    )
        .bind(&batch_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(&app_state.db_pool)
        .await?;

    let total_pages = (total.0 + per_page - 1) / per_page;

    let response = PaginatedResponse {
        data: usage_logs,
        total: total.0,
        page,
        per_page,
        total_pages,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

// ==================== DASHBOARD STATISTICS ====================

pub async fn get_dashboard_stats(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    #[derive(Debug, Serialize)]
    struct DashboardStats {
        total_reagents: i64,
        total_batches: i64,
        low_stock: i64,
        expiring_soon: i64,
        total_equipment: i64,
        equipment_alerts: i64,
        active_experiments: i64,
    }

    let total_reagents: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reagents WHERE status = 'active' AND deleted_at IS NULL")
        .fetch_one(&app_state.db_pool)
        .await?;

    let total_batches: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM batches WHERE deleted_at IS NULL")
        .fetch_one(&app_state.db_pool)
        .await?;

    let low_stock: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM batches WHERE quantity <= 10 AND status = 'available' AND deleted_at IS NULL AND reagent_id NOT IN (SELECT id FROM reagents WHERE deleted_at IS NOT NULL)")
        .fetch_one(&app_state.db_pool)
        .await?;

    let expiring_soon: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM batches WHERE expiry_date IS NOT NULL AND expiry_date <= datetime('now', '+30 days') AND status = 'available' AND deleted_at IS NULL AND reagent_id NOT IN (SELECT id FROM reagents WHERE deleted_at IS NOT NULL)"
    )
        .fetch_one(&app_state.db_pool)
        .await?;

    // Equipment: total count
    let total_equipment: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM equipment WHERE status != 'retired'"
    )
        .fetch_one(&app_state.db_pool)
        .await
        .unwrap_or((0,));

    // Equipment alerts: maintenance + damaged + calibration
    let equipment_alerts: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM equipment WHERE status IN ('maintenance', 'damaged', 'calibration')"
    )
        .fetch_one(&app_state.db_pool)
        .await
        .unwrap_or((0,));

    // Active experiments: in_progress + planned
    let active_experiments: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM experiments WHERE status IN ('in_progress', 'planned')"
    )
        .fetch_one(&app_state.db_pool)
        .await
        .unwrap_or((0,));

    let stats = DashboardStats {
        total_reagents: total_reagents.0,
        total_batches: total_batches.0,
        low_stock: low_stock.0,
        expiring_soon: expiring_soon.0,
        total_equipment: total_equipment.0,
        equipment_alerts: equipment_alerts.0,
        active_experiments: active_experiments.0,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(stats)))
}

// ==================== RECENT ACTIVITY (from audit_logs) ====================

#[derive(Debug, Serialize)]
pub struct ActivityItem {
    pub id: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub description: Option<String>,
    pub username: Option<String>,
    pub created_at: String,
}

pub async fn get_recent_activity(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let limit = 15i64;

    let rows: Vec<(String, String, String, Option<String>, Option<String>, Option<String>, String)> =
        sqlx::query_as(
            r#"SELECT
                a.id, a.action, a.entity_type, a.entity_id,
                a.description, u.username, a.created_at
            FROM audit_logs a
            LEFT JOIN users u ON a.user_id = u.id
            ORDER BY a.created_at DESC
            LIMIT ?"#
        )
        .bind(limit)
        .fetch_all(&app_state.db_pool)
        .await?;

    let activities: Vec<ActivityItem> = rows.into_iter()
        .map(|(id, action, entity_type, entity_id, description, username, created_at)| {
            ActivityItem { id, action, entity_type, entity_id, description, username, created_at }
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(activities)))
}

// ==================== USAGE TRENDS (for charts) ====================

#[derive(Debug, Serialize)]
pub struct UsageTrendPoint {
    pub date: String,
    pub usage_count: i64,
    pub total_quantity: f64,
}

#[derive(Debug, Serialize)]
pub struct ExpiringWeekPoint {
    pub week_label: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct DashboardTrends {
    pub usage_by_day: Vec<UsageTrendPoint>,
    pub expiring_by_week: Vec<ExpiringWeekPoint>,
}

pub async fn get_dashboard_trends(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    // Usage per day (last 30 days)
    let usage_rows: Vec<(String, i64, f64)> = sqlx::query_as(
        r#"SELECT
            DATE(created_at) as day,
            COUNT(*) as usage_count,
            COALESCE(SUM(quantity_used), 0) as total_quantity
        FROM usage_logs
        WHERE created_at >= datetime('now', '-30 days')
        GROUP BY DATE(created_at)
        ORDER BY day ASC"#
    )
    .fetch_all(&app_state.db_pool)
    .await?;

    let usage_by_day: Vec<UsageTrendPoint> = usage_rows.into_iter()
        .map(|(date, usage_count, total_quantity)| UsageTrendPoint { date, usage_count, total_quantity })
        .collect();

    // Batches expiring in next 4 weeks
    let expiring_rows: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT
            CASE
                WHEN expiry_date <= datetime('now', '+7 days') THEN 'This week'
                WHEN expiry_date <= datetime('now', '+14 days') THEN 'Week 2'
                WHEN expiry_date <= datetime('now', '+21 days') THEN 'Week 3'
                WHEN expiry_date <= datetime('now', '+28 days') THEN 'Week 4'
            END as week_label,
            COUNT(*) as cnt
        FROM batches
        WHERE expiry_date IS NOT NULL
          AND expiry_date <= datetime('now', '+28 days')
          AND expiry_date > datetime('now')
          AND status = 'available'
        GROUP BY week_label
        ORDER BY CASE week_label
            WHEN 'This week' THEN 1 WHEN 'Week 2' THEN 2
            WHEN 'Week 3' THEN 3 WHEN 'Week 4' THEN 4
        END"#
    )
    .fetch_all(&app_state.db_pool)
    .await?;

    let expiring_by_week: Vec<ExpiringWeekPoint> = expiring_rows.into_iter()
        .map(|(week_label, count)| ExpiringWeekPoint { week_label, count })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(DashboardTrends { usage_by_day, expiring_by_week })))
}
// ==================== JWT ROTATION ====================

pub async fn get_jwt_rotation_status(
    app_state: web::Data<Arc<AppState>>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;

    if claims.role.as_str() != "admin" {
        return Err(ApiError::Forbidden(
            "Only administrators can view JWT rotation status".to_string()
        ));
    }

    let stats = get_rotation_stats(&app_state.db_pool).await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to get rotation stats: {}", e)))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(stats)))
}

pub async fn force_jwt_rotation(
    app_state: web::Data<Arc<AppState>>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;

    if claims.role.as_str() != "admin" {
        return Err(ApiError::Forbidden(
            "Only administrators can force JWT rotation".to_string()
        ));
    }

    let env_file = env::var("ENV_FILE").unwrap_or_else(|_| ".env".to_string());
    let new_secret = rotate_jwt_secret(&app_state.db_pool, &env_file).await
        .map_err(|e| ApiError::InternalServerError(format!("Failed to rotate JWT: {}", e)))?;

    log::warn!("Manual JWT rotation triggered by user: {}", claims.username);
crate::audit::audit(
    &app_state.db_pool, &claims.sub, "jwt_rotation", "system", "jwt",
    "Manual JWT rotation triggered", &http_request
).await;

    #[derive(serde::Serialize)]
    struct RotationResponse {
        message: String,
        secret_length: usize,
        warning: String,
    }

    let response = RotationResponse {
        message: "JWT secret rotated successfully".to_string(),
        secret_length: new_secret.len(),
        warning: "Application restart recommended to load new JWT secret".to_string(),
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}