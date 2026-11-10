// src/routes/storage.rs
use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;
use crate::{AppState, auth, auth_handlers, storage_handlers};
use crate::error::ApiResult;

// ==================== PROTECTED WRAPPERS ====================

/// Zones and positions are the inner layout of a room, so changing them needs
/// the same permission as editing a room (`edit_room`: admin and researcher by
/// default, or the user's custom permissions). Reading stays open to everyone.
async fn can_edit_layout(http_request: &HttpRequest, app_state: &web::Data<Arc<AppState>>) -> ApiResult<()> {
    auth_handlers::check_room_permission(http_request, auth_handlers::RoomAction::Edit, &app_state.db_pool).await
}

async fn create_storage_zone_protected(http_request: HttpRequest, app_state: web::Data<Arc<AppState>>, data: web::Json<crate::models::CreateStorageZoneRequest>) -> ApiResult<HttpResponse> {
    can_edit_layout(&http_request, &app_state).await?;
    let claims = auth::get_current_user(&http_request)?;
    storage_handlers::create_storage_zone(app_state, data, claims.sub).await
}
async fn update_storage_zone_protected(http_request: HttpRequest, app_state: web::Data<Arc<AppState>>, path: web::Path<String>, data: web::Json<crate::models::UpdateStorageZoneRequest>) -> ApiResult<HttpResponse> {
    can_edit_layout(&http_request, &app_state).await?;
    let claims = auth::get_current_user(&http_request)?;
    storage_handlers::update_storage_zone(app_state, path, data, claims.sub).await
}
async fn delete_storage_zone_protected(http_request: HttpRequest, app_state: web::Data<Arc<AppState>>, path: web::Path<String>) -> ApiResult<HttpResponse> {
    can_edit_layout(&http_request, &app_state).await?;
    let _claims = auth::get_current_user(&http_request)?;
    storage_handlers::delete_storage_zone(app_state, path).await
}
async fn create_storage_position_protected(http_request: HttpRequest, app_state: web::Data<Arc<AppState>>, data: web::Json<crate::models::CreateStoragePositionRequest>) -> ApiResult<HttpResponse> {
    can_edit_layout(&http_request, &app_state).await?;
    let claims = auth::get_current_user(&http_request)?;
    storage_handlers::create_storage_position(app_state, data, claims.sub).await
}
async fn update_storage_position_protected(http_request: HttpRequest, app_state: web::Data<Arc<AppState>>, path: web::Path<String>, data: web::Json<crate::models::UpdateStoragePositionRequest>) -> ApiResult<HttpResponse> {
    can_edit_layout(&http_request, &app_state).await?;
    let claims = auth::get_current_user(&http_request)?;
    storage_handlers::update_storage_position(app_state, path, data, claims.sub).await
}
async fn delete_storage_position_protected(http_request: HttpRequest, app_state: web::Data<Arc<AppState>>, path: web::Path<String>) -> ApiResult<HttpResponse> {
    can_edit_layout(&http_request, &app_state).await?;
    let _claims = auth::get_current_user(&http_request)?;
    storage_handlers::delete_storage_position(app_state, path).await
}

// ==================== ROUTES ====================

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/storage")
            .route("/zones", web::get().to(storage_handlers::get_storage_zones))
            .route("/zones", web::post().to(create_storage_zone_protected))
            .route("/zones/{id}", web::get().to(storage_handlers::get_storage_zone))
            .route("/zones/{id}", web::put().to(update_storage_zone_protected))
            .route("/zones/{id}", web::delete().to(delete_storage_zone_protected))
            .route("/zones/{id}/items", web::get().to(storage_handlers::get_zone_items))
            .route("/positions", web::get().to(storage_handlers::get_storage_positions))
            .route("/positions", web::post().to(create_storage_position_protected))
            .route("/positions/{id}", web::get().to(storage_handlers::get_storage_position))
            .route("/positions/{id}", web::put().to(update_storage_position_protected))
            .route("/positions/{id}", web::delete().to(delete_storage_position_protected))
            .route("/positions/{id}/items", web::get().to(storage_handlers::get_position_items))
            .route("/hierarchy", web::get().to(storage_handlers::get_storage_hierarchy))
            .route("/location-path/{id}", web::get().to(storage_handlers::get_location_path))
            .route("/search", web::get().to(storage_handlers::search_storage_locations))
    );
}
