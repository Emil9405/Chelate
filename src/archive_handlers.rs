// src/archive_handlers.rs
//! Обработчики для архива (корзины) удалённых сущностей.
//!
//! Эндпоинты:
//!   GET    /api/v1/admin/archive/reagents    — список удалённых реагентов
//!   GET    /api/v1/admin/archive/equipment   — список удалённого оборудования
//!   POST   /api/v1/admin/archive/restore     — восстановить запись
//!   DELETE /api/v1/admin/archive/hard-delete  — окончательно удалить
//!
//! Все эндпоинты требуют роль `admin`.

use actix_web::{web, HttpResponse};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::AppState;
use crate::error::{ApiError, ApiResult};
use crate::handlers::ApiResponse;

// ============================================================
//                      DTO
// ============================================================

/// Элемент архива — единый формат для всех типов сущностей
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ArchivedItem {
    pub id: String,
    pub name: String,
    pub status: String,
    pub deleted_at: Option<DateTime<Utc>>,
    pub updated_by: Option<String>,
    /// Имя пользователя, удалившего запись (JOIN на users)
    pub deleted_by_name: Option<String>,
}

/// Подробный элемент архива реагента (с дополнительными полями)
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ArchivedReagent {
    pub id: String,
    pub name: String,
    pub formula: Option<String>,
    pub cas_number: Option<String>,
    pub manufacturer: Option<String>,
    pub status: String,
    pub deleted_at: Option<DateTime<Utc>>,
    pub updated_by: Option<String>,
    pub deleted_by_name: Option<String>,
    /// Количество soft-deleted батчей этого реагента
    pub deleted_batches_count: i64,
}

/// Подробный элемент архива оборудования
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ArchivedEquipment {
    pub id: String,
    pub name: String,
    pub type_: String,
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub location: Option<String>,
    pub status: String,
    pub deleted_at: Option<DateTime<Utc>>,
    pub updated_by: Option<String>,
    pub deleted_by_name: Option<String>,
}

/// Запрос на восстановление или окончательное удаление
#[derive(Debug, Deserialize)]
pub struct ArchiveActionRequest {
    pub id: String,
    /// "reagent" | "equipment"
    pub entity_type: String,
}

/// Статистика архива
#[derive(Debug, Serialize)]
pub struct ArchiveStats {
    pub reagents_count: i64,
    pub equipment_count: i64,
    pub total: i64,
}

// ============================================================
//                  GET ARCHIVED ITEMS
// ============================================================

/// GET /api/v1/admin/archive/stats
pub async fn get_archive_stats(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let pool = &app_state.db_pool;

    let (reagents_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM reagents WHERE deleted_at IS NOT NULL"
    ).fetch_one(pool).await?;

    let (equipment_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM equipment WHERE deleted_at IS NOT NULL"
    ).fetch_one(pool).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(ArchiveStats {
        reagents_count,
        equipment_count,
        total: reagents_count + equipment_count,
    })))
}

/// GET /api/v1/admin/archive/reagents
pub async fn get_archived_reagents(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let items: Vec<ArchivedReagent> = sqlx::query_as(
        r#"SELECT
            r.id,
            r.name,
            r.formula,
            r.cas_number,
            r.manufacturer,
            r.status,
            r.deleted_at,
            r.updated_by,
            u.username as deleted_by_name,
            (SELECT COUNT(*) FROM batches WHERE reagent_id = r.id AND deleted_at IS NOT NULL) as deleted_batches_count
        FROM reagents r
        LEFT JOIN users u ON r.updated_by = u.id
        WHERE r.deleted_at IS NOT NULL
        ORDER BY r.deleted_at DESC"#
    )
    .fetch_all(&app_state.db_pool)
    .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(items)))
}

/// GET /api/v1/admin/archive/equipment
pub async fn get_archived_equipment(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let items: Vec<ArchivedEquipment> = sqlx::query_as(
        r#"SELECT
            e.id,
            e.name,
            e.type_,
            e.serial_number,
            e.manufacturer,
            e.location,
            e.status,
            e.deleted_at,
            e.updated_by,
            u.username as deleted_by_name
        FROM equipment e
        LEFT JOIN users u ON e.updated_by = u.id
        WHERE e.deleted_at IS NOT NULL
        ORDER BY e.deleted_at DESC"#
    )
    .fetch_all(&app_state.db_pool)
    .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(items)))
}

// ============================================================
//                     RESTORE
// ============================================================

/// POST /api/v1/admin/archive/restore
pub async fn restore_item(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<ArchiveActionRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let pool = &app_state.db_pool;

    match body.entity_type.as_str() {
        "reagent" => {
            // Проверка конфликта имён: может существовать active реагент с тем же именем
            check_name_conflict(pool, "reagents", &body.id).await?;
            crate::soft_delete::restore_reagent(pool, &body.id, &user_id).await?;
        }
        "equipment" => {
            crate::soft_delete::restore_equipment(pool, &body.id, &user_id).await?;
        }
        other => {
            return Err(ApiError::bad_request(&format!(
                "Unknown entity_type: '{}'. Use 'reagent' or 'equipment'.", other
            )));
        }
    }

    log::info!(
        "♻️ Archive: {} '{}' restored by user {}",
        body.entity_type, body.id, user_id
    );

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        serde_json::json!({"id": &body.id, "entity_type": &body.entity_type}),
        format!("{} restored successfully", capitalize(&body.entity_type)),
    )))
}

// ============================================================
//                   HARD DELETE
// ============================================================

/// DELETE /api/v1/admin/archive/hard-delete
pub async fn hard_delete_item(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<ArchiveActionRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let pool = &app_state.db_pool;

    let rows = match body.entity_type.as_str() {
        "reagent" => {
            crate::soft_delete::hard_delete_record(pool, "reagents", &body.id).await?
        }
        "equipment" => {
            // Удаляем физические файлы с диска перед hard delete
            cleanup_equipment_files(pool, &body.id).await;
            crate::soft_delete::hard_delete_record(pool, "equipment", &body.id).await?
        }
        other => {
            return Err(ApiError::bad_request(&format!(
                "Unknown entity_type: '{}'. Use 'reagent' or 'equipment'.", other
            )));
        }
    };

    if rows == 0 {
        return Err(ApiError::not_found("Archived item"));
    }

    log::info!(
        "🗑️ Archive: {} '{}' permanently deleted by user {}",
        body.entity_type, body.id, user_id
    );

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        serde_json::json!({"id": &body.id, "entity_type": &body.entity_type}),
        format!("{} permanently deleted", capitalize(&body.entity_type)),
    )))
}

// ============================================================
//                   HELPERS
// ============================================================

/// Проверка: нет ли активного реагента с тем же именем
async fn check_name_conflict(
    pool: &SqlitePool,
    table: &str,
    id: &str,
) -> ApiResult<()> {
    if table != "reagents" {
        return Ok(());
    }

    // Получаем имя удалённого реагента
    let deleted_name: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM reagents WHERE id = ? AND deleted_at IS NOT NULL"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    let name = match deleted_name {
        Some((name,)) => name,
        None => return Err(ApiError::not_found("Archived reagent")),
    };

    // Проверяем, нет ли активного реагента с тем же именем
    let conflict: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM reagents WHERE LOWER(name) = LOWER(?) AND deleted_at IS NULL AND id != ?"
    )
    .bind(&name)
    .bind(id)
    .fetch_optional(pool)
    .await?;

    if conflict.is_some() {
        return Err(ApiError::bad_request(&format!(
            "Cannot restore: an active reagent with name '{}' already exists", name
        )));
    }

    Ok(())
}

/// Удалить физические файлы оборудования с диска
async fn cleanup_equipment_files(pool: &SqlitePool, equipment_id: &str) {
    if let Ok(files) = sqlx::query_as::<_, (String,)>(
        "SELECT file_path FROM equipment_files WHERE equipment_id = ?"
    )
    .bind(equipment_id)
    .fetch_all(pool)
    .await
    {
        for (path,) in files {
            if let Err(e) = std::fs::remove_file(&path) {
                log::warn!("Failed to remove file {}: {}", path, e);
            }
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}
