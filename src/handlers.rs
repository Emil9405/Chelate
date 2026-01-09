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

    let reagent: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ?")
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

    let _reagent: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ?")
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
        r#"INSERT INTO usage_logs (id, batch_id, user_id, quantity_used, purpose, notes, used_at, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#
    )
        .bind(&usage_id)
        .bind(&batch_id)
        .bind(&claims.sub)
        .bind(request.quantity_used)
        .bind(&request.purpose)
        .bind(&request.notes)
        .bind(&now)
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

    tx.commit().await?;

    log::info!(
        "User {} used {} {} from batch {} (reagent {})",
        claims.username, request.quantity_used, batch.unit, batch_id, reagent_id
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

    let _reagent: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ?")
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
            ul.used_at,
            ul.created_at
           FROM usage_logs ul
           LEFT JOIN users u ON ul.user_id = u.id
           LEFT JOIN batches b ON ul.batch_id = b.id
           WHERE ul.batch_id = ?
           ORDER BY ul.used_at DESC
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
    }

    let total_reagents: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reagents WHERE status = 'active'")
        .fetch_one(&app_state.db_pool)
        .await?;

    let total_batches: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM batches")
        .fetch_one(&app_state.db_pool)
        .await?;

    let low_stock: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM batches WHERE quantity <= 10 AND status = 'available'")
        .fetch_one(&app_state.db_pool)
        .await?;

    let expiring_soon: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM batches WHERE expiry_date IS NOT NULL AND expiry_date <= datetime('now', '+30 days') AND status = 'available'"
    )
        .fetch_one(&app_state.db_pool)
        .await?;

    let stats = DashboardStats {
        total_reagents: total_reagents.0,
        total_batches: total_batches.0,
        low_stock: low_stock.0,
        expiring_soon: expiring_soon.0,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(stats)))
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