// src/routes/inventory.rs
//! Inventory by scanning. Reading is open to every signed-in user; starting,
//! scanning, correcting and finishing need the batch edit permission (the
//! same one as moving bottles and writing off stock).

use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;
use crate::{AppState, auth_handlers, inventory_handlers as inv};
use crate::error::ApiResult;

async fn can_edit(http_request: &HttpRequest, app_state: &web::Data<Arc<AppState>>) -> ApiResult<()> {
    auth_handlers::check_batch_permission_async(http_request, auth_handlers::BatchAction::Edit, &app_state.db_pool).await
}

async fn start_protected(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<inv::StartRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    can_edit(&http_request, &app_state).await?;
    inv::start_session(app_state, body, http_request).await
}

async fn scan_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    body: web::Json<inv::ScanRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    can_edit(&http_request, &app_state).await?;
    inv::scan(app_state, path, body, http_request).await
}

async fn resolve_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    body: web::Json<inv::ResolveRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    can_edit(&http_request, &app_state).await?;
    inv::resolve_item(app_state, path, body, http_request).await
}

async fn close_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    body: Option<web::Json<inv::CloseRequest>>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    can_edit(&http_request, &app_state).await?;
    inv::close_session(app_state, path, body, http_request).await
}

async fn cancel_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    can_edit(&http_request, &app_state).await?;
    inv::cancel_session(app_state, path, http_request).await
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/inventory")
            .route("/sessions", web::get().to(inv::list_sessions))
            .route("/sessions", web::post().to(start_protected))
            .route("/sessions/{id}", web::get().to(inv::get_session_detail))
            .route("/sessions/{id}/scan", web::post().to(scan_protected))
            .route("/sessions/{id}/items/{item_id}/resolve", web::post().to(resolve_protected))
            .route("/sessions/{id}/close", web::post().to(close_protected))
            .route("/sessions/{id}/cancel", web::post().to(cancel_protected))
    );
}
