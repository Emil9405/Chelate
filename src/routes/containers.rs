// src/routes/containers.rs
use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;
use crate::{AppState, auth_handlers, container_handlers};
use crate::error::ApiResult;

// ==================== PROTECTED WRAPPERS ====================

async fn place_container_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<crate::models::batch_container::PlaceContainerRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, auth_handlers::BatchAction::Edit, &app_state.db_pool).await?;
    container_handlers::place_container(app_state, path, request, http_request).await
}

async fn move_container_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<crate::models::batch_container::MoveContainerRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, auth_handlers::BatchAction::Edit, &app_state.db_pool).await?;
    container_handlers::move_container(app_state, path, request, http_request).await
}

async fn unplace_container_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, auth_handlers::BatchAction::Edit, &app_state.db_pool).await?;
    container_handlers::unplace_container(app_state, path, http_request).await
}

async fn use_from_container_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<crate::models::batch_container::UseFromContainerRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, auth_handlers::BatchAction::Edit, &app_state.db_pool).await?;
    container_handlers::use_from_container(app_state, path, request, http_request).await
}

async fn dispose_container_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, auth_handlers::BatchAction::Delete, &app_state.db_pool).await?;
    container_handlers::dispose_container(app_state, path, http_request).await
}

async fn place_containers_bulk_protected(
    app_state: web::Data<Arc<AppState>>,
    request: web::Json<container_handlers::BulkPlaceRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, auth_handlers::BatchAction::Edit, &app_state.db_pool).await?;
    container_handlers::place_containers_bulk(app_state, request, http_request).await
}

async fn move_containers_bulk_protected(
    app_state: web::Data<Arc<AppState>>,
    request: web::Json<container_handlers::BulkMoveRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, auth_handlers::BatchAction::Edit, &app_state.db_pool).await?;
    container_handlers::move_containers_bulk(app_state, request, http_request).await
}

// ==================== ROUTES ====================

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/containers")
            // Bulk operations (static routes FIRST)
            .route("/place-bulk", web::post().to(place_containers_bulk_protected))
            .route("/move-bulk", web::post().to(move_containers_bulk_protected))
            // Per-container operations (dynamic routes after)
            .route("/{container_id}/place", web::post().to(place_container_protected))
            .route("/{container_id}/move", web::put().to(move_container_protected))
            .route("/{container_id}/unplace", web::delete().to(unplace_container_protected))
            .route("/{container_id}/use", web::post().to(use_from_container_protected))
            .route("/{container_id}", web::delete().to(dispose_container_protected))
    );
}
