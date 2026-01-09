// src/filter_handlers.rs

use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::query_builders::{
    FilterGroup, FieldWhitelist, Filter, FilterItem,
};
use crate::handlers::PaginatedResponse;
use crate::error::{ApiError, ApiResult};
use crate::models::{Experiment, Batch};

// ==================== КОНСТАНТЫ БЕЗОПАСНОСТИ ====================

/// Разрешённые поля сортировки для batches
const BATCH_SORT_FIELDS: &[&str] = &[
    "b.id", "b.reagent_id", "b.batch_number", "b.quantity",
    "b.original_quantity", "b.reserved_quantity", "b.expiry_date",
    "b.supplier", "b.manufacturer", "b.received_date", "b.status",
    "b.location", "b.created_at", "b.updated_at",
    "reagent_name", "days_until_expiry",
];

/// Разрешённые поля сортировки для experiments
const EXPERIMENT_SORT_FIELDS: &[&str] = &[
    "id", "title", "experiment_date", "experiment_type",
    "instructor", "student_group", "status", "room_id",
    "created_at", "updated_at",
];

/// Валидация поля сортировки
fn validate_sort_field<'a>(field: &'a str, allowed: &[&str]) -> Option<&'a str> {
    if allowed.iter().any(|&f| f == field) {
        Some(field)
    } else {
        None
    }
}

/// Экранирование спецсимволов LIKE
fn escape_like_pattern(pattern: &str) -> String {
    pattern
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

// === Структура для чтения из БД ===
#[derive(Debug, sqlx::FromRow)]
struct BatchFromDb {
    pub id: String,
    pub reagent_id: String,
    pub batch_number: String,
    pub cat_number: Option<String>,
    pub quantity: f64,
    pub original_quantity: f64,
    pub reserved_quantity: f64,
    pub unit: String,
    pub expiry_date: Option<DateTime<Utc>>,
    pub supplier: Option<String>,
    pub manufacturer: Option<String>,
    pub received_date: DateTime<Utc>,
    pub status: String,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub reagent_name: Option<String>,
    pub days_until_expiry: Option<i64>,
}

// === Структура для ответа API ===
#[derive(Debug, Serialize)]
pub struct BatchFilterResponse {
    pub id: String,
    pub reagent_id: String,
    pub reagent_name: String,
    pub batch_number: String,
    pub cat_number: Option<String>,
    pub quantity: f64,
    pub original_quantity: f64,
    pub reserved_quantity: f64,
    pub unit: String,
    pub expiry_date: Option<DateTime<Utc>>,
    pub supplier: Option<String>,
    pub manufacturer: Option<String>,
    pub received_date: DateTime<Utc>,
    pub status: String,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expiration_status: String,
    pub days_until_expiration: Option<i64>,
}

// === Константы для статуса срока годности ===
const EXPIRY_CRITICAL_DAYS: i64 = 7;
const EXPIRY_WARNING_DAYS: i64 = 30;

fn calculate_expiration_status(days_until_expiry: Option<i64>) -> String {
    match days_until_expiry {
        None => "unknown".to_string(),
        Some(days) => {
            if days < 0 {
                "expired"
            } else if days <= EXPIRY_CRITICAL_DAYS {
                "expiring_critical"
            } else if days <= EXPIRY_WARNING_DAYS {
                "expiring_soon"
            } else {
                "ok"
            }.to_string()
        }
    }
}

impl From<BatchFromDb> for BatchFilterResponse {
    fn from(b: BatchFromDb) -> Self {
        let expiration_status = calculate_expiration_status(b.days_until_expiry);
        BatchFilterResponse {
            id: b.id,
            reagent_id: b.reagent_id,
            reagent_name: b.reagent_name.unwrap_or_default(),
            batch_number: b.batch_number,
            cat_number: b.cat_number,
            quantity: b.quantity,
            original_quantity: b.original_quantity,
            reserved_quantity: b.reserved_quantity,
            unit: b.unit,
            expiry_date: b.expiry_date,
            supplier: b.supplier,
            manufacturer: b.manufacturer,
            received_date: b.received_date,
            status: b.status,
            location: b.location,
            notes: b.notes,
            created_at: b.created_at,
            updated_at: b.updated_at,
            expiration_status,
            days_until_expiration: b.days_until_expiry,
        }
    }
}

// === Запрос с фильтрами ===
#[derive(Debug, Deserialize)]
pub struct AdvancedFilterRequest {
    pub filters: Option<FilterGroup>,
    pub search: Option<String>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    pub sort_by: Option<String>,
    #[serde(default = "default_sort_order")]
    pub sort_order: String,
}

fn default_page() -> i64 { 1 }
fn default_per_page() -> i64 { 20 }
fn default_sort_order() -> String { "DESC".to_string() }

// === Фильтрация партий ===
pub async fn get_batches_filtered(
    pool: web::Data<SqlitePool>,
    body: web::Json<AdvancedFilterRequest>,
) -> ApiResult<HttpResponse> {
    let whitelist = FieldWhitelist::for_batches();
    let offset = (body.page - 1) * body.per_page;

    // Базовый SQL запрос
    let base_sql = r#"
        SELECT 
            b.id, b.reagent_id, b.batch_number, b.cat_number, b.quantity,
            b.original_quantity, b.reserved_quantity, b.unit, b.expiry_date,
            b.supplier, b.manufacturer, b.received_date, b.status, b.location,
            b.notes, b.created_by, b.updated_by, b.created_at, b.updated_at,
            r.name as reagent_name,
            CAST(julianday(b.expiry_date) - julianday('now') AS INTEGER) as days_until_expiry
        FROM batches b
        LEFT JOIN reagents r ON b.reagent_id = r.id
    "#;

    let mut conditions: Vec<String> = vec!["1=1".to_string()];
    let mut params: Vec<String> = Vec::new();

    // Применяем фильтры через FilterBuilder
    if let Some(ref filters) = body.filters {
        let filter_builder = crate::query_builders::FilterBuilder::new()
            .with_whitelist(&whitelist);
        if let Ok((cond, filter_params)) = filter_builder.build_condition(filters) {
            if !cond.is_empty() {
                conditions.push(cond);
                params.extend(filter_params);
            }
        }
    }

    // ✅ ИСПРАВЛЕНО: Поиск с экранированием LIKE-спецсимволов
    if let Some(ref search) = body.search {
        if !search.trim().is_empty() {
            let escaped = escape_like_pattern(search.trim());
            let search_pattern = format!("%{}%", escaped);
            conditions.push(
                "(r.name LIKE ? ESCAPE '\\' OR b.batch_number LIKE ? ESCAPE '\\' \
                 OR b.cat_number LIKE ? ESCAPE '\\' OR b.supplier LIKE ? ESCAPE '\\')".to_string()
            );
            params.push(search_pattern.clone());
            params.push(search_pattern.clone());
            params.push(search_pattern.clone());
            params.push(search_pattern);
        }
    }

    // ✅ ИСПРАВЛЕНО: Валидация поля сортировки через whitelist
    let sort_field = body.sort_by.as_deref()
        .and_then(|f| validate_sort_field(f, BATCH_SORT_FIELDS))
        .unwrap_or("b.created_at");
    let sort_order = if body.sort_order.to_uppercase() == "ASC" { "ASC" } else { "DESC" };

    let sql = format!(
        "{} WHERE {} ORDER BY {} {} LIMIT ? OFFSET ?",
        base_sql,
        conditions.join(" AND "),
        sort_field,
        sort_order
    );

    // Выполняем запрос
    let mut query = sqlx::query_as::<_, BatchFromDb>(&sql);
    for param in &params {
        query = query.bind(param);
    }
    query = query.bind(body.per_page).bind(offset);

    let batches_db: Vec<BatchFromDb> = query.fetch_all(pool.get_ref()).await?;
    let batches: Vec<BatchFilterResponse> = batches_db.into_iter().map(Into::into).collect();

    // Подсчёт общего количества
    let count_sql = format!(
        "SELECT COUNT(*) FROM batches b LEFT JOIN reagents r ON b.reagent_id = r.id WHERE {}",
        conditions.join(" AND ")
    );
    
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for param in &params {
        count_query = count_query.bind(param);
    }
    let total: i64 = count_query.fetch_one(pool.get_ref()).await?;

    let total_pages = if body.per_page > 0 { (total + body.per_page - 1) / body.per_page } else { 1 };

    Ok(HttpResponse::Ok().json(PaginatedResponse {
        data: batches,
        total,
        page: body.page,
        per_page: body.per_page,
        total_pages,
    }))
}

// === Пресеты ===
pub async fn get_batches_by_preset(
    pool: web::Data<SqlitePool>,
    preset: web::Path<String>,
    query: web::Query<crate::handlers::PaginationQuery>,
) -> ApiResult<HttpResponse> {
    let filters = match preset.as_str() {
        "low_stock" => FilterGroup::and(vec![
            FilterItem::filter(Filter::lte("quantity", 10.0)),
            FilterItem::filter(Filter::eq("status", "available")),
        ]),
        "expiring_soon" => FilterGroup::and(vec![
            FilterItem::filter(Filter::between_numbers("days_until_expiry", 0.0, 30.0)),
            FilterItem::filter(Filter::neq("status", "expired")),
        ]),
        "expired" => FilterGroup::or(vec![
            FilterItem::filter(Filter::eq("status", "expired")),
            FilterItem::filter(Filter::lt("days_until_expiry", 0.0)),
        ]),
        "available" => FilterGroup::and(vec![
            FilterItem::filter(Filter::eq("status", "available")),
            FilterItem::filter(Filter::gt("quantity", 0.0)),
        ]),
        _ => return Err(ApiError::bad_request("Unknown preset")),
    };

    let req = AdvancedFilterRequest {
        filters: Some(filters),
        search: None,
        page: query.page.unwrap_or(1),
        per_page: query.per_page.unwrap_or(20),
        sort_by: query.sort_by.clone(),
        sort_order: query.sort_order.clone().unwrap_or("DESC".to_string()),
    };

    get_batches_filtered(pool, web::Json(req)).await
}

// === Фильтрация экспериментов ===
pub async fn get_experiments_filtered(
    pool: web::Data<SqlitePool>,
    body: web::Json<AdvancedFilterRequest>,
) -> ApiResult<HttpResponse> {
    let whitelist = FieldWhitelist::for_experiments();
    let offset = (body.page - 1) * body.per_page;

    let mut conditions: Vec<String> = vec!["1=1".to_string()];
    let mut params: Vec<String> = Vec::new();

    // Применяем фильтры
    if let Some(ref filters) = body.filters {
        let filter_builder = crate::query_builders::FilterBuilder::new()
            .with_whitelist(&whitelist);
        if let Ok((cond, filter_params)) = filter_builder.build_condition(filters) {
            if !cond.is_empty() {
                conditions.push(cond);
                params.extend(filter_params);
            }
        }
    }

    // ✅ ИСПРАВЛЕНО: Поиск с экранированием
    if let Some(ref search) = body.search {
        if !search.trim().is_empty() {
            let escaped = escape_like_pattern(search.trim());
            let search_pattern = format!("%{}%", escaped);
            conditions.push(
                "(title LIKE ? ESCAPE '\\' OR description LIKE ? ESCAPE '\\' \
                 OR instructor LIKE ? ESCAPE '\\' OR student_group LIKE ? ESCAPE '\\')".to_string()
            );
            params.push(search_pattern.clone());
            params.push(search_pattern.clone());
            params.push(search_pattern.clone());
            params.push(search_pattern);
        }
    }

    // ✅ ИСПРАВЛЕНО: Валидация сортировки
    let sort_field = body.sort_by.as_deref()
        .and_then(|f| validate_sort_field(f, EXPERIMENT_SORT_FIELDS))
        .unwrap_or("created_at");
    let sort_order = if body.sort_order.to_uppercase() == "ASC" { "ASC" } else { "DESC" };

    let sql = format!(
        "SELECT * FROM experiments WHERE {} ORDER BY {} {} LIMIT ? OFFSET ?",
        conditions.join(" AND "),
        sort_field,
        sort_order
    );

    let mut query = sqlx::query_as::<_, Experiment>(&sql);
    for param in &params {
        query = query.bind(param);
    }
    query = query.bind(body.per_page).bind(offset);

    let experiments: Vec<Experiment> = query.fetch_all(pool.get_ref()).await?;

    // Подсчёт
    let count_sql = format!(
        "SELECT COUNT(*) FROM experiments WHERE {}",
        conditions.join(" AND ")
    );
    
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for param in &params {
        count_query = count_query.bind(param);
    }
    let total: i64 = count_query.fetch_one(pool.get_ref()).await?;

    let total_pages = if body.per_page > 0 { (total + body.per_page - 1) / body.per_page } else { 1 };

    Ok(HttpResponse::Ok().json(PaginatedResponse {
        data: experiments,
        total,
        page: body.page,
        per_page: body.per_page,
        total_pages,
    }))
}

// ==================== ТЕСТЫ БЕЗОПАСНОСТИ ====================

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_sort_field_validation() {
        // Валидные поля
        assert!(validate_sort_field("b.created_at", BATCH_SORT_FIELDS).is_some());
        assert!(validate_sort_field("b.quantity", BATCH_SORT_FIELDS).is_some());
        assert!(validate_sort_field("created_at", EXPERIMENT_SORT_FIELDS).is_some());
        
        // SQL-инъекции блокируются
        assert!(validate_sort_field("b.created_at; DROP TABLE users", BATCH_SORT_FIELDS).is_none());
        assert!(validate_sort_field("1=1 OR 1=1", BATCH_SORT_FIELDS).is_none());
        assert!(validate_sort_field("password", BATCH_SORT_FIELDS).is_none());
        assert!(validate_sort_field("", BATCH_SORT_FIELDS).is_none());
    }

    #[test]
    fn test_like_escape() {
        assert_eq!(escape_like_pattern("100%"), "100\\%");
        assert_eq!(escape_like_pattern("test_value"), "test\\_value");
        assert_eq!(escape_like_pattern("a\\b"), "a\\\\b");
        assert_eq!(escape_like_pattern("normal"), "normal");
        assert_eq!(escape_like_pattern("%_%"), "\\%\\_\\%");
    }

    #[test]
    fn test_expiration_status() {
        assert_eq!(calculate_expiration_status(None), "unknown");
        assert_eq!(calculate_expiration_status(Some(-1)), "expired");
        assert_eq!(calculate_expiration_status(Some(0)), "expiring_critical");
        assert_eq!(calculate_expiration_status(Some(7)), "expiring_critical");
        assert_eq!(calculate_expiration_status(Some(8)), "expiring_soon");
        assert_eq!(calculate_expiration_status(Some(30)), "expiring_soon");
        assert_eq!(calculate_expiration_status(Some(31)), "ok");
    }
}