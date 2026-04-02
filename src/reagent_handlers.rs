// src/reagent_handlers.rs
//! Обработчики для реагентов с гибридной пагинацией
//! Оптимизировано для 270,000+ записей
//! ✅ FTS5 поиск с автоматическим fallback на LIKE

use actix_web::{web, HttpResponse};
use std::sync::Arc;
use crate::AppState;
use crate::models::*;
use crate::error::{ApiError, ApiResult};
use crate::handlers::ApiResponse;
use crate::validator::FieldValidator;
use crate::pagination::{
    HybridPaginationQuery, HybridPaginatedResponse, HybridPaginationInfo, SortingInfo,
    CtePaginationBuilder, ReagentSortWhitelist,
    encode_cursor, decode_cursor,
};
use uuid::Uuid;
use validator::Validate;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

// ==================== FTS SEARCH HELPER ====================

/// Проверка доступности FTS таблицы (кэшируется при старте)
async fn check_fts_available(pool: &sqlx::SqlitePool) -> bool {
    let result: Result<(i64,), _> = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='reagents_fts'"
    ).fetch_one(pool).await;
    matches!(result, Ok((count,)) if count > 0)
}

/// Построение FTS запроса (очистка спецсимволов + prefix search)
fn build_fts_query(search: &str) -> String {
    crate::query_builders::utils::sanitize_fts_query(search)
}

/// Добавляет условие поиска с FTS или LIKE fallback
/// Поля поиска: name, cas_number, formula
fn add_search_condition_with_fts(
    builder: &mut CtePaginationBuilder,
    search: &str,
    use_fts: bool,
) {
    let search_trimmed = search.trim();
    if search_trimmed.is_empty() {
        return;
    }

    if use_fts {
        let fts_query = build_fts_query(search_trimmed);
        if fts_query.is_empty() {
            return;
        }

        // FTS5 поиск через rowid (быстрый, O(log n))
        // reagents_fts индексирует: name, formula, cas_number
        builder.add_search(
            "rowid IN (SELECT rowid FROM reagents_fts WHERE reagents_fts MATCH ?)",
            vec![fts_query]
        );
    } else {
        // Fallback на LIKE (медленнее, но работает без FTS)
        let pattern = format!("%{}%", search_trimmed);
        builder.add_search(
            "(name LIKE ? OR cas_number LIKE ? OR formula LIKE ?)",
            vec![pattern.clone(), pattern.clone(), pattern]
        );
    }
}

/// Legacy: простой LIKE поиск (для обратной совместимости)
fn add_search_condition(builder: &mut CtePaginationBuilder, pattern: &str) {
    // Добавляем условие поиска с 4 параметрами
    builder.add_search(
        "(name LIKE ? OR formula LIKE ? OR cas_number LIKE ? OR manufacturer LIKE ?)",
        vec![pattern.to_string(); 4]
    );
}

// ==================== RESPONSE STRUCTURES ====================

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ReagentListItem {
    pub id: String,
    pub name: String,
    pub formula: Option<String>,
    pub cas_number: Option<String>,
    pub manufacturer: Option<String>,
    pub molecular_weight: Option<f64>,
    pub physical_state: Option<String>,
    pub description: Option<String>,
    pub storage_conditions: Option<String>,
    pub appearance: Option<String>,
    pub hazard_pictograms: Option<String>,
    pub status: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Cached fields (из таблицы reagents, без JOIN)
    pub total_quantity: f64,
    pub batches_count: i64,
    pub primary_unit: Option<String>,
    
}

#[derive(Debug, Serialize)]
pub struct ReagentDetailResponse {
    pub id: String,
    pub name: String,
    pub formula: Option<String>,
    pub cas_number: Option<String>,
    pub manufacturer: Option<String>,
    pub molecular_weight: Option<f64>,
    pub physical_state: Option<String>,
    pub description: Option<String>,
    pub storage_conditions: Option<String>,
    pub appearance: Option<String>,
    pub hazard_pictograms: Option<String>,
    pub status: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Stock info (вычисляется на лету для одного реагента)
    pub total_quantity: f64,
    pub total_unit: String,
    pub batches_count: i64,
    pub available_batches: i64,
    pub reserved_quantity: f64,
    pub available_quantity: f64,
    pub low_stock: bool,
    pub expiring_soon_count: i64,
    pub expired_count: i64,
    pub batches: Vec<Batch>,
}

#[derive(Debug, sqlx::FromRow)]
struct StockAggregation {
    pub total_quantity: Option<f64>,
    pub reserved_quantity: Option<f64>,
    pub original_quantity: Option<f64>,
    pub batches_count: i64,
    pub available_batches: i64,
    pub expiring_soon_count: i64,
    pub expired_count: i64,
    pub primary_unit: Option<String>,
   
}

// ==================== MAIN GET REAGENTS ====================

/// Получение списка реагентов с гибридной пагинацией
///
/// Поддерживает:
/// - Page-based: ?page=1&per_page=50
/// - Cursor-based: ?cursor=xxx&direction=next
/// - FTS поиск: ?search=acetone (или ?q=acetone)
///
/// Сортировка по total_quantity использует индекс напрямую (O(log n))
pub async fn get_reagents(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<HybridPaginationQuery>,
) -> ApiResult<HttpResponse> {
    let pool = &app_state.db_pool;

    let (page, per_page, offset) = query.normalize();
    let sort_by = ReagentSortWhitelist::validate(query.sort_by());
    let sort_order = ReagentSortWhitelist::validate_order(query.sort_order());
    let is_desc = sort_order == "DESC";
    let direction = query.direction();

    // ===== ПРОВЕРКА FTS =====
    // Проверяем доступность FTS таблицы один раз
    let use_fts = check_fts_available(pool).await;

    // ===== BUILD CONDITIONS =====
    let mut builder = CtePaginationBuilder::new("reagents")
        .select("id, name, formula, cas_number, manufacturer, molecular_weight, \
                 physical_state, description, storage_conditions, appearance, \
                 hazard_pictograms, status, created_by, updated_by, created_at, \
                 updated_at, total_quantity, batches_count, primary_unit")
        .sort(sort_by, sort_order)
        .limit(per_page);
        
    // Exclude soft-deleted reagents
    builder.add_raw_condition("deleted_at IS NULL");

    // ===== SEARCH FILTER (FTS с fallback на LIKE) =====
    // Поддержка обоих параметров: search и q (для совместимости с фронтендом)
    let search_term = query.search.as_ref()
        .or(query.q.as_ref())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    if let Some(search) = search_term {
        add_search_condition_with_fts(&mut builder, search, use_fts);
    }

    // Status filter
    if let Some(ref status) = query.status {
        builder.add_condition("status = ?", status.clone());
    }

    // Manufacturer filter
    if let Some(ref manufacturer) = query.manufacturer {
        builder.add_condition("manufacturer = ?", manufacturer.clone());
    }

    // Has stock filter
    if let Some(has_stock) = query.has_stock {
        if has_stock {
            builder.add_raw_condition("total_quantity > 0");
        } else {
            builder.add_raw_condition("total_quantity = 0");
        }
    }

    // ===== COUNT =====
    let (count_sql, count_params) = builder.build_count();
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for p in &count_params {
        count_query = count_query.bind(p);
    }
    let total: i64 = count_query.fetch_one(pool).await?;

    // ===== FETCH DATA =====
    let use_cursor = query.is_cursor_mode() && ReagentSortWhitelist::supports_keyset(sort_by);

    let mut reagents: Vec<ReagentListItem> = if use_cursor {
        // Cursor-based (keyset) pagination
        if let Some(ref cursor) = query.cursor {
            if let Some((cursor_value, cursor_id)) = decode_cursor(cursor) {
                builder.keyset_after(cursor_value, &cursor_id, is_desc, direction);
            }
        }

        let (sql, params) = builder.build_cte(direction, is_desc);

        let mut db_query = sqlx::query_as::<_, ReagentListItem>(&sql);
        for p in &params {
            db_query = db_query.bind(p);
        }

        db_query.fetch_all(pool).await?
    } else {
        // Page-based (offset) pagination
        let (sql, params) = builder.build_simple(offset);

        let mut db_query = sqlx::query_as::<_, ReagentListItem>(&sql);
        for p in &params {
            db_query = db_query.bind(p);
        }

        db_query.fetch_all(pool).await?
    };

    // ===== PAGINATION STATE =====
    let pagination = if use_cursor {
        let has_more = reagents.len() > per_page as usize;
        if has_more {
            reagents.pop();
        }

        // Reverse if going backwards
        if direction == "prev" {
            reagents.reverse();
        }

        let has_next = if direction == "prev" { query.cursor.is_some() } else { has_more };
        let has_prev = if direction == "prev" { has_more } else { query.cursor.is_some() };

        let next_cursor = if has_next {
            reagents.last().map(|r| encode_cursor(r.total_quantity, &r.id))
        } else {
            None
        };

        let prev_cursor = if has_prev {
            reagents.first().map(|r| encode_cursor(r.total_quantity, &r.id))
        } else {
            None
        };

        HybridPaginationInfo::from_cursor(total, per_page, has_next, has_prev, next_cursor, prev_cursor)
    } else {
        HybridPaginationInfo::from_page(total, page, per_page)
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(HybridPaginatedResponse {
        data: reagents,
        pagination,
        sorting: SortingInfo {
            sort_by: sort_by.to_string(),
            sort_order: sort_order.to_string(),
        },
    })))
}

// ==================== SEARCH (autocomplete) ====================

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<i64>,
}

pub async fn search_reagents(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<SearchQuery>,
) -> ApiResult<HttpResponse> {
    let q = query.q.trim();
    if q.is_empty() {
        return Ok(HttpResponse::Ok().json(ApiResponse::success(Vec::<ReagentListItem>::new())));
    }

    let limit = query.limit.unwrap_or(10).min(50);
    let pool = &app_state.db_pool;

    // Проверяем FTS
    let use_fts = check_fts_available(pool).await;

    let reagents: Vec<ReagentListItem> = if use_fts {
        let fts_query = build_fts_query(q);
        if fts_query.is_empty() {
            return Ok(HttpResponse::Ok().json(ApiResponse::success(Vec::<ReagentListItem>::new())));
        }

        sqlx::query_as::<_, ReagentListItem>(
            r#"SELECT id, name, formula, cas_number, manufacturer, molecular_weight,
                      physical_state, description, storage_conditions, appearance,
                      hazard_pictograms, status, created_by, updated_by, created_at,
                      updated_at, total_quantity, batches_count, primary_unit
               FROM reagents
               WHERE rowid IN (SELECT rowid FROM reagents_fts WHERE reagents_fts MATCH ?)
               AND deleted_at IS NULL
               ORDER BY total_quantity DESC
               LIMIT ?"#
        )
            .bind(&fts_query)
            .bind(limit)
            .fetch_all(pool)
            .await?
    } else {
        let pattern = format!("%{}%", q);
        sqlx::query_as::<_, ReagentListItem>(
            r#"SELECT id, name, formula, cas_number, manufacturer, molecular_weight,
                      physical_state, description, storage_conditions, appearance,
                      hazard_pictograms, status, created_by, updated_by, created_at,
                      updated_at, total_quantity, batches_count, primary_unit
               FROM reagents
               WHERE (name LIKE ? OR cas_number LIKE ? OR formula LIKE ?)
               AND deleted_at IS NULL
               ORDER BY total_quantity DESC
               LIMIT ?"#
        )
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(limit)
            .fetch_all(pool)
            .await?
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(reagents)))
}

// ==================== GET BY ID ====================

pub async fn get_reagent_by_id(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let pool = &app_state.db_pool;

    let reagent: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ? AND deleted_at IS NULL")
        .bind(&id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found("Reagent"))?;

    // Получаем агрегированные данные по батчам
    let stock: StockAggregation = sqlx::query_as(r#"
        SELECT
            COALESCE(SUM(CASE WHEN status = 'available' THEN quantity ELSE 0 END), 0) as total_quantity,
            COALESCE(SUM(CASE WHEN status = 'reserved' THEN quantity ELSE 0 END), 0) as reserved_quantity,
            COALESCE(SUM(original_quantity), 0) as original_quantity,
            COUNT(*) as batches_count,
            COUNT(CASE WHEN status = 'available' THEN 1 END) as available_batches,
            COUNT(CASE WHEN expiry_date IS NOT NULL AND expiry_date <= date('now', '+30 days') AND expiry_date > date('now') THEN 1 END) as expiring_soon_count,
            COUNT(CASE WHEN expiry_date IS NOT NULL AND expiry_date <= date('now') THEN 1 END) as expired_count,
            (SELECT unit FROM batches WHERE reagent_id = ? AND status = 'available' AND deleted_at IS NULL LIMIT 1) as primary_unit
            FROM batches WHERE reagent_id = ? AND deleted_at IS NULL
    "#)
        .bind(&id)
        .bind(&id)
        .fetch_one(pool)
        .await?;

    let batches: Vec<Batch> = sqlx::query_as("SELECT * FROM batches WHERE reagent_id = ? AND deleted_at IS NULL ORDER BY created_at DESC")
        .bind(&id)
        .fetch_all(pool)
        .await?;

    let total_qty = stock.total_quantity.unwrap_or(0.0);
    let reserved_qty = stock.reserved_quantity.unwrap_or(0.0);

    let response = ReagentDetailResponse {
        id: reagent.id,
        name: reagent.name,
        formula: reagent.formula,
        cas_number: reagent.cas_number,
        manufacturer: reagent.manufacturer,
        molecular_weight: reagent.molecular_weight,
        physical_state: reagent.physical_state,
        description: reagent.description,
        storage_conditions: reagent.storage_conditions,
        appearance: reagent.appearance,
        hazard_pictograms: reagent.hazard_pictograms,
        status: reagent.status,
        created_by: reagent.created_by,
        updated_by: reagent.updated_by,
        created_at: reagent.created_at,
        updated_at: reagent.updated_at,
        total_quantity: total_qty,
        total_unit: stock.primary_unit.clone().unwrap_or_default(),
        batches_count: stock.batches_count,
        available_batches: stock.available_batches,
        reserved_quantity: reserved_qty,
        available_quantity: total_qty - reserved_qty,
        low_stock: total_qty < 10.0 && total_qty > 0.0,
        expiring_soon_count: stock.expiring_soon_count,
        expired_count: stock.expired_count,
        batches,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

// ==================== CREATE ====================

pub async fn create_reagent(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<CreateReagentRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    body.validate().map_err(|e| ApiError::bad_request(&e.to_string()))?;

    if let Some(ref cas) = body.cas_number {
        if !cas.trim().is_empty() {
            FieldValidator::cas_number(cas.trim()).map_err(|e| ApiError::bad_request(&e))?;
        }
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    // Free up the name if a soft-deleted reagent holds it
    let deleted_dup: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM reagents WHERE LOWER(name) = LOWER(?) AND deleted_at IS NOT NULL"
    )
    .bind(&body.name)
    .fetch_optional(&app_state.db_pool)
    .await?;

    if let Some((old_id,)) = deleted_dup {
        sqlx::query(
            "UPDATE reagents SET name = name || '_deleted_' || id WHERE id = ?"
        )
        .bind(&old_id)
        .execute(&app_state.db_pool)
        .await?;
    }

    sqlx::query(r#"
        INSERT INTO reagents (
            id, name, formula, cas_number, manufacturer, molecular_weight,
            physical_state, description, storage_conditions, appearance,
            hazard_pictograms, status, total_quantity, batches_count,
            created_by, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'active', 0, 0, ?, ?, ?)
    "#)
        .bind(&id)
        .bind(&body.name)
        .bind(&body.formula)
        .bind(&body.cas_number)
        .bind(&body.manufacturer)
        .bind(&body.molecular_weight)
        .bind(&body.physical_state)
        .bind(&body.description)
        .bind(&body.storage_conditions)
        .bind(&body.appearance)
        .bind(&body.hazard_pictograms)
        .bind(&user_id)
        .bind(&now)
        .bind(&now)
        .execute(&app_state.db_pool)
        .await?;

    let reagent: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ?")
        .bind(&id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success_with_message(
        reagent,
        "Reagent created successfully".to_string(),
    )))
}

// ==================== UPDATE ====================

pub async fn update_reagent(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    body: web::Json<UpdateReagentRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let pool = &app_state.db_pool;

    body.validate().map_err(|e| ApiError::bad_request(&e.to_string()))?;

    let _: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ? AND deleted_at IS NULL")
        .bind(&id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found("Reagent"))?;

    if let Some(ref cas) = body.cas_number {
        if !cas.trim().is_empty() {
            FieldValidator::cas_number(cas.trim()).map_err(|e| ApiError::bad_request(&e))?;
        }
    }

    let mut sets = Vec::new();
    let mut vals: Vec<Option<String>> = Vec::new();

    // upd! maps Some("") to None (clear → NULL) and Some(value) to Some(value)
    macro_rules! upd {
        ($f:ident, $c:expr) => {
            if let Some(ref v) = body.$f {
                sets.push(concat!($c, " = ?"));
                if v.is_empty() {
                    vals.push(None);
                } else {
                    vals.push(Some(v.clone()));
                }
            }
        };
    }

    upd!(name, "name");
    upd!(formula, "formula");
    upd!(cas_number, "cas_number");
    upd!(manufacturer, "manufacturer");
    upd!(physical_state, "physical_state");
    upd!(description, "description");
    upd!(storage_conditions, "storage_conditions");
    upd!(appearance, "appearance");
    upd!(hazard_pictograms, "hazard_pictograms");
    upd!(status, "status");

    if let Some(mw) = body.molecular_weight {
        sets.push("molecular_weight = ?");
        vals.push(Some(mw.to_string()));
    }

    if sets.is_empty() {
        return Err(ApiError::bad_request("No fields to update"));
    }

    sets.push("updated_by = ?");
    vals.push(Some(user_id));
    sets.push("updated_at = datetime('now')");

    let sql = format!("UPDATE reagents SET {} WHERE id = ?", sets.join(", "));
    let mut q = sqlx::query(&sql);
    for v in vals { q = q.bind(v); }
    q = q.bind(&id);
    q.execute(pool).await?;

    let reagent: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ?")
        .bind(&id)
        .fetch_one(pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        reagent,
        "Reagent updated successfully".to_string(),
    )))
}


// ==================== DELETE (SOFT) ====================

pub async fn delete_reagent(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let id = path.into_inner();
    let pool = &app_state.db_pool;

    let _: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ? AND deleted_at IS NULL")
        .bind(&id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found("Reagent"))?;

    // Soft delete — устанавливаем deleted_at
    sqlx::query("UPDATE reagents SET deleted_at = datetime('now'), updated_by = ?, status = 'inactive' WHERE id = ?")
        .bind(&user_id)
        .bind(&id)
        .execute(pool)
        .await?;

    // Soft delete всех батчей этого реагента (если ещё не удалены)
    sqlx::query("UPDATE batches SET deleted_at = datetime('now'), updated_by = ? WHERE reagent_id = ? AND deleted_at IS NULL")
        .bind(&user_id)
        .bind(&id)
        .execute(pool)
        .await?;

    log::info!("🗑️ Reagent {} soft-deleted by user {}", id, user_id);

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        serde_json::json!({"id": id}),
        "Reagent deleted successfully".to_string(),
    )))
}

// ==================== CACHE MANAGEMENT ====================

/// Пересчитать кэш для конкретного реагента
pub async fn refresh_reagent_cache(pool: &sqlx::SqlitePool, reagent_id: &str) -> ApiResult<()> {
    sqlx::query(r#"
        UPDATE reagents SET
            total_quantity = (
                SELECT COALESCE(SUM(quantity), 0)
                FROM batches
                WHERE reagent_id = ? AND status = 'available' AND deleted_at IS NULL
            ),
            batches_count = (
                SELECT COUNT(*)
                FROM batches
                WHERE reagent_id = ? AND status = 'available' AND deleted_at IS NULL
            ),
            primary_unit = (
                SELECT unit
                FROM batches
                WHERE reagent_id = ? AND status = 'available' AND deleted_at IS NULL
                LIMIT 1
            ),
            updated_at = datetime('now')
        WHERE id = ?
    "#)
        .bind(reagent_id)
        .bind(reagent_id)
        .bind(reagent_id)
        .bind(reagent_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Полная перестройка кэша (для maintenance)
pub async fn rebuild_cache(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let start = std::time::Instant::now();

    let result = sqlx::query(r#"
        UPDATE reagents SET
            total_quantity = (
                SELECT COALESCE(SUM(quantity), 0)
                FROM batches
                WHERE reagent_id = reagents.id AND status = 'available' AND deleted_at IS NULL
            ),
            batches_count = (
                SELECT COUNT(*)
                FROM batches
                WHERE reagent_id = reagents.id AND status = 'available' AND deleted_at IS NULL      
            ),
            primary_unit = (
                SELECT unit
                FROM batches
                WHERE reagent_id = reagents.id AND status = 'available' AND deleted_at IS NULL
                LIMIT 1
            ),
            updated_at = datetime('now')
    "#)
        .execute(&app_state.db_pool)
        .await?;

    let elapsed = start.elapsed();

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        serde_json::json!({
            "rows_updated": result.rows_affected(),
            "duration_ms": elapsed.as_millis()
        }),
        format!("Cache rebuilt: {} reagents in {:?}", result.rows_affected(), elapsed),
    )))
}

// ==================== GET REAGENT WITH BATCHES (legacy compatibility) ====================

pub async fn get_reagent_with_batches(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    // Перенаправляем на get_reagent_by_id
    get_reagent_by_id(app_state, path).await
}