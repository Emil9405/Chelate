// src/routes/archive.rs
//! Маршруты архива (корзины) удалённых сущностей.
//!
//! Все эндпоинты требуют роль `admin`.
//! Действия логируются в audit_logs.

use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;
use crate::{AppState, auth, archive_handlers, audit};
use crate::audit::ChangeSet;
use crate::error::{ApiError, ApiResult};

// ============================================================
//                  ADMIN GUARD
// ============================================================

fn require_admin(req: &HttpRequest) -> ApiResult<auth::Claims> {
    let claims = auth::get_current_user(req)?;
    if !claims.role.can_manage_archive() {
        return Err(ApiError::Forbidden("Admin access required for archive operations".to_string()));
    }
    Ok(claims)
}

// ============================================================
//                PROTECTED WRAPPERS
// ============================================================

async fn get_archive_stats(
    app_state: web::Data<Arc<AppState>>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    require_admin(&req)?;
    archive_handlers::get_archive_stats(app_state).await
}

async fn get_archived_reagents(
    app_state: web::Data<Arc<AppState>>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    require_admin(&req)?;
    archive_handlers::get_archived_reagents(app_state).await
}

async fn get_archived_equipment(
    app_state: web::Data<Arc<AppState>>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    require_admin(&req)?;
    archive_handlers::get_archived_equipment(app_state).await
}

async fn restore_item(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<archive_handlers::ArchiveActionRequest>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = require_admin(&req)?;
    let user_id = claims.sub.clone();
    let entity_type = body.entity_type.clone();
    let entity_id = body.id.clone();

    let response = archive_handlers::restore_item(
        app_state.clone(), body, user_id.clone(),
    ).await?;

    let mut cs = ChangeSet::new();
    cs.created("action", "restore");
    cs.created("entity_type", &entity_type);
    cs.created("entity_id", &entity_id);

    audit::audit_with_changes(
        &app_state.db_pool, &user_id,
        "restore", &entity_type, &entity_id,
        &format!("Restored {} from archive", entity_type),
        &cs, &req,
    ).await;

    Ok(response)
}

async fn hard_delete_item(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<archive_handlers::ArchiveActionRequest>,
    req: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = require_admin(&req)?;
    let user_id = claims.sub.clone();

    let entity_type = body.entity_type.clone();
    let entity_id = body.id.clone();

    let response = archive_handlers::hard_delete_item(
        app_state.clone(), body, user_id.clone(),
    ).await?;

    let mut cs = ChangeSet::new();
    cs.deleted("action", "hard_delete");
    cs.deleted("entity_type", &entity_type);
    cs.deleted("entity_id", &entity_id);

    audit::audit_with_changes(
        &app_state.db_pool, &user_id,
        "hard_delete", &format!("archive_{}", entity_type),
        &entity_id,
        &format!("Permanently deleted: {} {}", entity_type, entity_id),
        &cs, &req,
    ).await;

    Ok(response)
}

// ============================================================
//                     ROUTES
// ============================================================

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/archive")
            .route("/stats", web::get().to(get_archive_stats))
            .route("/reagents", web::get().to(get_archived_reagents))
            .route("/equipment", web::get().to(get_archived_equipment))
            .route("/restore", web::post().to(restore_item))
            .route("/hard-delete", web::delete().to(hard_delete_item))
    );
}
