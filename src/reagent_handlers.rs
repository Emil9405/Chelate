// src/reagent_handlers.rs

use actix_web::{web, HttpResponse};
use std::sync::Arc;
use crate::AppState;
use crate::models::*;
use crate::error::{ApiError, ApiResult};
use crate::handlers::{
    ApiResponse, PaginationQuery,
    PaginatedResponseWithSort, PaginationInfo, SortingInfo, CursorPaginatedResponse
};
use crate::validator::FieldValidator;
use uuid::Uuid;
use validator::Validate;
use serde::{Serialize, Deserialize};
use log::warn;

// ==================== SORTING WHITELIST ====================

/// Whitelist разрешённых полей сортировки: (api_field, sql_column)
const ALLOWED_REAGENT_SORT_FIELDS: &[(&str, &str)] = &[
    ("name", "r.name"),
    ("formula", "r.formula"),
    ("cas_number", "r.cas_number"),
    ("manufacturer", "r.manufacturer"),
    ("status", "r.status"),
    ("created_at", "r.created_at"),
    ("updated_at", "r.updated_at"),
    ("total_quantity", "total_quantity"),
    ("available_quantity", "available_quantity"),
    ("batches_count", "batches_count"),
    ("molecular_weight", "r.molecular_weight"),
];

/// Валидация поля сортировки - возвращает SQL колонку или дефолт
fn validate_sort_field(field: &str) -> &'static str {
    ALLOWED_REAGENT_SORT_FIELDS
        .iter()
        .find(|(api, _)| *api == field)
        .map(|(_, sql)| *sql)
        .unwrap_or("r.created_at")
}

/// Валидация порядка сортировки
fn validate_sort_order(order: &str) -> &'static str {
    match order.to_uppercase().as_str() {
        "ASC" => "ASC",
        _ => "DESC",
    }
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
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub total_quantity: f64,
    pub reserved_quantity: f64,
    pub available_quantity: f64,
    pub batches_count: i64,
    #[sqlx(default)]
    pub total_display: String,
    pub unit: Option<String>, 
}

#[derive(Debug, Serialize)]
pub struct ReagentWithStockResponse {
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
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub total_quantity: f64,
    pub total_unit: String,
    pub batches_count: i64,
    pub available_batches: i64,
    pub reserved_quantity: f64,
    pub low_stock: bool,
    pub expiring_soon_count: i64,
    pub expired_count: i64,
    pub batches: Vec<Batch>,
}

#[derive(Debug, sqlx::FromRow)]
struct ReagentStockAggregation {
    pub total_quantity: Option<f64>,
    pub reserved_quantity: Option<f64>,
    pub original_quantity: Option<f64>,
    pub batches_count: i64,
    pub available_batches: i64,
    pub expiring_soon_count: i64,
    pub expired_count: i64,
    pub primary_unit: Option<String>,
}

// ==================== FTS HELPERS ====================

async fn is_fts_available(pool: &sqlx::SqlitePool) -> bool {
    let result: Result<(i64,), _> = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='reagents_fts'"
    ).fetch_one(pool).await;
    
    matches!(result, Ok((count,)) if count > 0)
}

fn escape_fts_query(query: &str) -> String {
    query
        .chars()
        .filter(|c| !matches!(c, '(' | ')' | '*' | '"' | ':' | '^' | '-' | '+' | '~' | '&' | '|'))
        .collect::<String>()
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_search_condition(search: &str, use_fts: bool, table_alias: &str) -> (String, Vec<String>) {
    let search_trimmed = search.trim();
    if search_trimmed.is_empty() {
        return (String::new(), Vec::new());
    }

    if use_fts {
        let escaped = escape_fts_query(search_trimmed);
        let fts_query = escaped
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|word| format!("{}*", word))
            .collect::<Vec<_>>()
            .join(" ");

        if fts_query.is_empty() {
            return (String::new(), Vec::new());
        }

        let pattern = format!("%{}%", search_trimmed);
        let condition = format!(
            "({}.rowid IN (SELECT rowid FROM reagents_fts WHERE reagents_fts MATCH ?) \
             OR EXISTS (SELECT 1 FROM batches bs WHERE bs.reagent_id = {}.id AND \
             (bs.batch_number LIKE ? OR bs.cat_number LIKE ? OR bs.supplier LIKE ?)))",
            table_alias, table_alias
        );

        (condition, vec![fts_query, pattern.clone(), pattern.clone(), pattern])
    } else {
        let pattern = format!("%{}%", search_trimmed);
        let condition = format!(
            "({}.name LIKE ? OR {}.formula LIKE ? OR {}.cas_number LIKE ? OR {}.manufacturer LIKE ? OR {}.storage_conditions LIKE ? OR {}.appearance LIKE ? OR {}.hazard_pictograms LIKE ? \
             OR EXISTS (SELECT 1 FROM batches bs WHERE bs.reagent_id = {}.id AND \
             (bs.batch_number LIKE ? OR bs.cat_number LIKE ? OR bs.supplier LIKE ?)))",
            table_alias, table_alias, table_alias, table_alias,
            table_alias, table_alias, table_alias,
            table_alias
        );

        let mut params = Vec::with_capacity(10);
        for _ in 0..10 {
            params.push(pattern.clone());
        }

        (condition, params)
    }
}

// ==================== REAGENT CRUD ====================

pub async fn get_reagents(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<PaginationQuery>,
) -> ApiResult<HttpResponse> {
    let (page, per_page, offset) = query.normalize();
    
    let fts_available = is_fts_available(&app_state.db_pool).await;
    let mut count_sql = "SELECT COUNT(DISTINCT r.id) FROM reagents r WHERE 1=1".to_string();
    let mut count_params: Vec<String> = Vec::new();

    if let Some(ref search) = query.search {
        let (condition, params) = build_search_condition(search, fts_available, "r");
        if !condition.is_empty() {
            count_sql.push_str(" AND ");
            count_sql.push_str(&condition);
            count_params.extend(params);
        }
    }
    if let Some(ref status) = query.status {
        count_sql.push_str(" AND r.status = ?");
        count_params.push(status.clone());
    }

    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for param in &count_params {
        count_query = count_query.bind(param);
    }
    
    let total: i64 = count_query
        .fetch_one(&app_state.db_pool)
        .await?;

    // Ð˜Ð¡ÐŸÐ ÐÐ’Ð›Ð•ÐÐž: Ð”Ð¾Ð±Ð°Ð²Ð»ÐµÐ½Ñ‹ CAST(... AS REAL) Ð²Ð¾ Ð²ÑÐµÑ… Ð¼ÐµÑÑ‚Ð°Ñ… ÑÑƒÐ¼Ð¼Ð¸Ñ€Ð¾Ð²Ð°Ð½Ð¸Ñ, Ð²ÐºÐ»ÑŽÑ‡Ð°Ñ total_display
let base_query = r#"
        SELECT 
            r.id, r.name, r.formula, r.cas_number, r.manufacturer,
            r.molecular_weight, r.physical_state, r.description, r.storage_conditions, r.appearance, r.hazard_pictograms, r.status,
            r.created_by, r.updated_by, r.created_at, r.updated_at,
            CAST(COALESCE(SUM(CASE WHEN b.status = 'available' THEN b.quantity ELSE 0.0 END), 0.0) AS REAL) as total_quantity,
            CAST(COALESCE(SUM(CASE WHEN b.status = 'available' THEN b.reserved_quantity ELSE 0.0 END), 0.0) AS REAL) as reserved_quantity,
            CAST(COALESCE(SUM(CASE WHEN b.status = 'available' THEN b.quantity - b.reserved_quantity ELSE 0.0 END), 0.0) AS REAL) as available_quantity,
            COUNT(b.id) as batches_count,
            (SELECT bu.unit FROM batches bu WHERE bu.reagent_id = r.id AND bu.status = 'available' LIMIT 1) as unit,
            CASE 
                WHEN COALESCE(SUM(CASE WHEN b.status = 'available' THEN b.quantity ELSE 0.0 END), 0.0) > 0 
                THEN CAST(ROUND(CAST(COALESCE(SUM(CASE WHEN b.status = 'available' THEN b.quantity ELSE 0.0 END), 0.0) AS REAL), 2) AS TEXT) 
                     || ' ' || COALESCE((SELECT bu.unit FROM batches bu WHERE bu.reagent_id = r.id AND bu.status = 'available' LIMIT 1), '')
                ELSE 'No stock'
            END as total_display
        FROM reagents r
        LEFT JOIN batches b ON r.id = b.reagent_id
        WHERE 1=1
    "#;

    let mut sql = base_query.to_string();
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(ref search) = query.search {
        let (condition, params) = build_search_condition(search, fts_available, "r");
        if !condition.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&condition);
            bind_values.extend(params);
        }
    }

    if let Some(ref status) = query.status {
        sql.push_str(" AND r.status = ?");
        bind_values.push(status.clone());
    }

    // Динамическая сортировка с whitelist защитой
    let sort_field = query.sort_by.as_deref().unwrap_or("created_at");
    let sort_order = query.sort_order.as_deref().unwrap_or("DESC");
    let validated_field = validate_sort_field(sort_field);
    let validated_order = validate_sort_order(sort_order);
    
    sql.push_str(&format!(" GROUP BY r.id ORDER BY {} {} LIMIT ? OFFSET ?", 
        validated_field, validated_order));

    let mut query_builder = sqlx::query_as::<_, ReagentListItem>(&sql);
    for value in bind_values {
        query_builder = query_builder.bind(value);
    }
    query_builder = query_builder.bind(per_page).bind(offset);

    let reagents: Vec<ReagentListItem> = query_builder
        .fetch_all(&app_state.db_pool)
        .await?;

    // Новый формат ответа с pagination и sorting
    Ok(HttpResponse::Ok().json(ApiResponse::success(PaginatedResponseWithSort {
        data: reagents,
        pagination: PaginationInfo::new(total, page, per_page),
        sorting: SortingInfo {
            sort_by: sort_field.to_string(),
            sort_order: validated_order.to_string(),
        },
    })))
}

pub async fn get_reagent_by_id(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let reagent_id = path.into_inner();

    let reagent: Option<Reagent> = sqlx::query_as(
        "SELECT * FROM reagents WHERE id = ?"
    )
        .bind(&reagent_id)
        .fetch_optional(&app_state.db_pool)
        .await?;

    let reagent = match reagent {
        Some(r) => r,
        None => return Err(ApiError::not_found("Reagent")),
    };

    // Ð˜Ð¡ÐŸÐ ÐÐ’Ð›Ð•ÐÐž: Ð”Ð¾Ð±Ð°Ð²Ð»ÐµÐ½ CAST(... AS REAL) Ð´Ð»Ñ ÑÐ¾Ð²Ð¼ÐµÑÑ‚Ð¸Ð¼Ð¾ÑÑ‚Ð¸ Ñ‚Ð¸Ð¿Ð¾Ð² SQLx
    let aggregation: ReagentStockAggregation = sqlx::query_as(
        r#"SELECT
            CAST(COALESCE(SUM(CASE WHEN status = 'available' THEN quantity ELSE 0 END), 0) AS REAL) as total_quantity,
            CAST(COALESCE(SUM(CASE WHEN status = 'available' THEN reserved_quantity ELSE 0 END), 0) AS REAL) as reserved_quantity,
            CAST(COALESCE(SUM(CASE WHEN status = 'available' THEN original_quantity ELSE 0 END), 0) AS REAL) as original_quantity,
            COUNT(*) as batches_count,
            SUM(CASE WHEN status = 'available' AND quantity > 0 THEN 1 ELSE 0 END) as available_batches,
            SUM(CASE WHEN expiry_date IS NOT NULL AND expiry_date <= datetime('now', '+30 days') AND expiry_date > datetime('now') AND status = 'available' THEN 1 ELSE 0 END) as expiring_soon_count,
            SUM(CASE WHEN expiry_date IS NOT NULL AND expiry_date <= datetime('now') THEN 1 ELSE 0 END) as expired_count,
            (SELECT unit FROM batches WHERE reagent_id = ? AND status = 'available' LIMIT 1) as primary_unit
           FROM batches
           WHERE reagent_id = ?"#
    )
        .bind(&reagent_id)
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    // Ð˜Ð¡ÐŸÐ ÐÐ’Ð›Ð•ÐÐž: Ð”Ð¾Ð¿Ð¾Ð»Ð½Ð¸Ñ‚ÐµÐ»ÑŒÐ½Ð¾ Ð·Ð°Ð³Ñ€ÑƒÐ¶Ð°ÐµÐ¼ ÑÐ°Ð¼Ð¸ Ð¿Ð°Ñ€Ñ‚Ð¸Ð¸
    let batches: Vec<Batch> = sqlx::query_as(
        "SELECT * FROM batches WHERE reagent_id = ? ORDER BY expiry_date ASC"
    )
        .bind(&reagent_id)
        .fetch_all(&app_state.db_pool)
        .await?;

    let total_qty = aggregation.total_quantity.unwrap_or(0.0);
    let original_qty = aggregation.original_quantity.unwrap_or(0.0);
    let low_stock = if original_qty > 0.0 {
        (total_qty / original_qty) < 0.2
    } else {
        false
    };

    let response = ReagentWithStockResponse {
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
        total_unit: aggregation.primary_unit.unwrap_or_else(|| "N/A".to_string()),
        batches_count: aggregation.batches_count,
        available_batches: aggregation.available_batches,
        reserved_quantity: aggregation.reserved_quantity.unwrap_or(0.0),
        low_stock,
        expiring_soon_count: aggregation.expiring_soon_count,
        expired_count: aggregation.expired_count,
        batches, // ÐŸÐµÑ€ÐµÐ´Ð°ÐµÐ¼ Ð·Ð°Ð³Ñ€ÑƒÐ¶ÐµÐ½Ð½Ñ‹Ðµ Ð¿Ð°Ñ€Ñ‚Ð¸Ð¸
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

// src/reagent_handlers.rs

pub async fn create_reagent(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<CreateReagentRequest>,
    user_id: String, // ID Ð¿Ð¾Ð»ÑŒÐ·Ð¾Ð²Ð°Ñ‚ÐµÐ»Ñ Ð¸Ð· Ñ‚Ð¾ÐºÐµÐ½Ð° (Ð²Ð¼ÐµÑÑ‚Ð¾ owner_id)
) -> ApiResult<HttpResponse> {
    body.validate()
        .map_err(|e| ApiError::bad_request(&format!("Validation failed: {}", e)))?;

    if let Some(ref cas) = body.cas_number {
        let cas_trimmed = cas.trim();
        if !cas_trimmed.is_empty() {
            FieldValidator::cas_number(cas_trimmed)
                .map_err(|e| ApiError::bad_request(&e))?;
        }
    }

    let id = Uuid::new_v4().to_string();

    // Ð˜Ð¡ÐŸÐ ÐÐ’Ð›Ð•ÐÐ˜Ð• Ð—Ð”Ð•Ð¡Ð¬:
    // 1. created_at Ð¸ updated_at Ð·Ð°Ð¿Ð¾Ð»Ð½ÑÑŽÑ‚ÑÑ Ñ„ÑƒÐ½ÐºÑ†Ð¸ÐµÐ¹ datetime('now')
    // 2. created_by Ð·Ð°Ð¿Ð¾Ð»Ð½ÑÐµÑ‚ÑÑ Ð¿ÐµÑ€ÐµÐ¼ÐµÐ½Ð½Ð¾Ð¹ user_id
    sqlx::query(
        r#"
        INSERT INTO reagents (
            id, name, formula, cas_number, manufacturer, 
            molecular_weight, physical_state, description, 
            storage_conditions, appearance, hazard_pictograms, 
            status, created_by, created_at, updated_at
        )
        VALUES (
            ?, ?, ?, ?, ?, 
            ?, ?, ?, 
            ?, ?, ?, 
            'active', ?, datetime('now'), datetime('now')
        )
        "#,
    )
        .bind(&id)
        .bind(&body.name)
        .bind(&body.formula)
        .bind(&body.cas_number)
        .bind(&body.manufacturer)
        .bind(&body.molecular_weight) // Ð£Ð±ÐµÐ´Ð¸Ñ‚ÐµÑÑŒ, Ñ‡Ñ‚Ð¾ ÑÑ‚Ð¾ Ð¿Ð¾Ð»Ðµ ÐµÑÑ‚ÑŒ Ð² CreateReagentRequest Ð² models.rs
        .bind(&body.physical_state)
        .bind(&body.description)
        .bind(&body.storage_conditions)
        .bind(&body.appearance)
        .bind(&body.hazard_pictograms)
        .bind(&user_id)
        .execute(&app_state.db_pool)
        .await?;

    // ÐžÐ±Ð½Ð¾Ð²Ð»ÑÐµÐ¼ Ð¸Ð½Ð´ÐµÐºÑ Ð¿Ð¾Ð¸ÑÐºÐ°
    let _ = sqlx::query(
        "INSERT INTO reagents_fts(rowid, name, formula, cas_number, manufacturer, description, storage_conditions, appearance, hazard_pictograms) \
         SELECT rowid, name, formula, cas_number, manufacturer, description, storage_conditions, appearance, hazard_pictograms FROM reagents WHERE id = ?"
    )
        .bind(&id)
        .execute(&app_state.db_pool)
        .await;

    // Ð’Ð¾Ð·Ð²Ñ€Ð°Ñ‰Ð°ÐµÐ¼ ÑÐ¾Ð·Ð´Ð°Ð½Ð½Ñ‹Ð¹ Ñ€ÐµÐ°Ð³ÐµÐ½Ñ‚
    let reagent: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ?")
        .bind(&id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success_with_message(
        reagent,
        "Reagent created successfully".to_string(),
    )))
}

pub async fn update_reagent(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    body: web::Json<UpdateReagentRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let reagent_id = path.into_inner();

    body.validate()
        .map_err(|e| {
            warn!("Validation failed for reagent update {}: {}", reagent_id, e);
            ApiError::bad_request(&format!("Validation failed: {}", e))
        })?;

    let existing: Option<Reagent> = sqlx::query_as(
        "SELECT * FROM reagents WHERE id = ?"
    )
        .bind(&reagent_id)
        .fetch_optional(&app_state.db_pool)
        .await?;

    if existing.is_none() {
        return Err(ApiError::not_found("Reagent"));
    }

    if let Some(ref cas) = body.cas_number {
        let cas_trimmed = cas.trim();
        if !cas_trimmed.is_empty() {
            FieldValidator::cas_number(cas_trimmed)
                .map_err(|e| ApiError::bad_request(&e))?;
        }
    }

    let mut updates = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(name) = &body.name {
        updates.push("name = ?");
        values.push(name.clone());
    }
    if let Some(formula) = &body.formula {
        updates.push("formula = ?");
        values.push(formula.clone());
    }
    if let Some(cas) = &body.cas_number {
        updates.push("cas_number = ?");
        values.push(cas.clone());
    }
    if let Some(manufacturer) = &body.manufacturer {
        updates.push("manufacturer = ?");
        values.push(manufacturer.clone());
    }
    if let Some(physical_state) = &body.physical_state {
        updates.push("physical_state = ?");
        values.push(physical_state.clone());
    }
    if let Some(description) = &body.description {
        updates.push("description = ?");
        values.push(description.clone());
    }
    if let Some(molecular_weight) = body.molecular_weight {
        updates.push("molecular_weight = ?");
        values.push(molecular_weight.to_string());
    }
    if let Some(storage_conditions) = &body.storage_conditions {
        updates.push("storage_conditions = ?");
        values.push(storage_conditions.clone());
    }
    if let Some(appearance) = &body.appearance {
        updates.push("appearance = ?");
        values.push(appearance.clone());
    }
    if let Some(hazard_pictograms) = &body.hazard_pictograms {
        updates.push("hazard_pictograms = ?");
        values.push(hazard_pictograms.clone());
    }
    
    if let Some(status) = &body.status {
        let valid_statuses = ["active", "inactive", "discontinued"];
        if !valid_statuses.contains(&status.as_str()) {
            return Err(ApiError::bad_request(&format!(
                "Invalid status '{}'. Must be one of: {}",
                status,
                valid_statuses.join(", ")
            )));
        }
        updates.push("status = ?");
        values.push(status.clone());
    }

    if updates.is_empty() {
        return Err(ApiError::bad_request("No fields to update"));
    }

    updates.push("updated_by = ?");
    updates.push("updated_at = datetime('now')");
    values.push(user_id);
    // Note: updated_at uses datetime('now') directly in SQL, no bind needed

    let sql = format!(
        "UPDATE reagents SET {} WHERE id = ?",
        updates.join(", ")
    );

    let mut query = sqlx::query(&sql);
    for value in values {
        query = query.bind(value);
    }
    query = query.bind(&reagent_id);

    query.execute(&app_state.db_pool).await?;

    let _ = sqlx::query("DELETE FROM reagents_fts WHERE rowid = (SELECT rowid FROM reagents WHERE id = ?)")
        .bind(&reagent_id)
        .execute(&app_state.db_pool)
        .await;
    
    let _ = sqlx::query(
        "INSERT INTO reagents_fts(rowid, name, formula, cas_number, manufacturer, description) \
         SELECT rowid, name, formula, cas_number, manufacturer, description FROM reagents WHERE id = ?"
    )
        .bind(&reagent_id)
        .execute(&app_state.db_pool)
        .await;

    let updated: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ?")
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(updated)))
}

pub async fn delete_reagent(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let reagent_id = path.into_inner();

    let batches: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM batches WHERE reagent_id = ?"
    )
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    if batches.0 > 0 {
        return Err(ApiError::bad_request(&format!(
            "Cannot delete reagent with {} existing batches",
            batches.0
        )));
    }

    let _ = sqlx::query("DELETE FROM reagents_fts WHERE rowid = (SELECT rowid FROM reagents WHERE id = ?)")
        .bind(&reagent_id)
        .execute(&app_state.db_pool)
        .await;

    let result = sqlx::query("DELETE FROM reagents WHERE id = ?")
        .bind(&reagent_id)
        .execute(&app_state.db_pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Reagent"));
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        (),
        "Reagent deleted successfully".to_string(),
    )))
}

pub async fn search_reagents(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<SearchQuery>,
) -> ApiResult<HttpResponse> {
    let search_term = query.q.as_ref().map(|s| s.trim()).unwrap_or("");
    let limit = query.limit.unwrap_or(10).clamp(1, 50);

    if search_term.is_empty() {
        return Ok(HttpResponse::Ok().json(ApiResponse::success(Vec::<Reagent>::new())));
    }

    let fts_available = is_fts_available(&app_state.db_pool).await;

    let reagents: Vec<Reagent> = if fts_available {
        let fts_query = escape_fts_query(search_term)
            .split_whitespace()
            .map(|word| format!("{}*", word))
            .collect::<Vec<_>>()
            .join(" ");

        if fts_query.is_empty() {
            return Ok(HttpResponse::Ok().json(ApiResponse::success(Vec::<Reagent>::new())));
        }

        sqlx::query_as(
            r#"SELECT r.* FROM reagents r
               WHERE r.rowid IN (
                   SELECT rowid FROM reagents_fts WHERE reagents_fts MATCH ?
               )
               AND r.status = 'active'
               ORDER BY r.name
               LIMIT ?"#
        )
            .bind(&fts_query)
            .bind(limit)
            .fetch_all(&app_state.db_pool)
            .await?
    } else {
        let search_pattern = format!("%{}%", search_term);

        sqlx::query_as(
            r#"SELECT * FROM reagents
               WHERE (name LIKE ? OR formula LIKE ? OR cas_number LIKE ? OR manufacturer LIKE ?)
                 AND status = 'active'
               ORDER BY name
               LIMIT ?"#
        )
            .bind(&search_pattern)
            .bind(&search_pattern)
            .bind(&search_pattern)
            .bind(&search_pattern)
            .bind(limit)
            .fetch_all(&app_state.db_pool)
            .await?
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(reagents)))
}

// ==================== REAGENT STOCK SUMMARY ====================

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ReagentStockSummary {
    pub reagent_id: String,
    pub reagent_name: String,
    pub total_quantity: f64,
    pub unit: String,
    pub batches_count: i64,
    pub low_stock: bool,
    pub has_expiring: bool,
}

pub async fn get_reagents_stock_summary(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let summary: Vec<ReagentStockSummary> = sqlx::query_as(
        r#"SELECT 
            r.id as reagent_id,
            r.name as reagent_name,
            COALESCE(SUM(CASE WHEN b.status = 'available' THEN b.quantity ELSE 0 END), 0) as total_quantity,
            COALESCE((SELECT bu.unit FROM batches bu WHERE bu.reagent_id = r.id LIMIT 1), 'N/A') as unit,
            COUNT(b.id) as batches_count,
            CASE 
                WHEN COALESCE(SUM(b.original_quantity), 0) > 0 
                     AND (COALESCE(SUM(b.quantity), 0) / COALESCE(SUM(b.original_quantity), 1)) < 0.2 
                THEN 1 ELSE 0 
            END as low_stock,
            CASE 
                WHEN SUM(CASE WHEN b.expiry_date <= datetime('now', '+30 days') AND b.status = 'available' THEN 1 ELSE 0 END) > 0 
                THEN 1 ELSE 0 
            END as has_expiring
           FROM reagents r
           LEFT JOIN batches b ON r.id = b.reagent_id
           WHERE r.status = 'active'
           GROUP BY r.id, r.name
           ORDER BY r.name"#
    )
        .fetch_all(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(summary)))
}

pub async fn rebuild_fts_index(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    sqlx::query("DELETE FROM reagents_fts")
        .execute(&app_state.db_pool)
        .await?;

    let result = sqlx::query(
        r#"
        INSERT INTO reagents_fts(rowid, name, formula, cas_number, manufacturer, description)
        SELECT rowid, name, formula, cas_number, manufacturer, description
        FROM reagents
        "#,
    )
        .execute(&app_state.db_pool)
        .await?;

    sqlx::query("INSERT INTO reagents_fts(reagents_fts) VALUES('optimize')")
        .execute(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        result.rows_affected(),
        format!("FTS index rebuilt with {} reagents", result.rows_affected()),
    )))
}

// ==================== TYPES ====================

#[derive(Debug, serde::Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
}
// ==================== PAGINATION METADATA ====================

/// Метаданные о доступных полях сортировки для реагентов
#[derive(Debug, Serialize)]
pub struct SortFieldMeta {
    pub field: &'static str,
    pub label: &'static str,
    pub data_type: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ReagentsPaginationMeta {
    pub available_sort_fields: Vec<SortFieldMeta>,
    pub default_sort_field: &'static str,
    pub default_sort_order: &'static str,
    pub default_per_page: i64,
    pub max_per_page: i64,
}

/// GET /api/v1/reagents/pagination-meta - Метаданные для UI пагинации
pub async fn get_reagents_pagination_meta() -> ApiResult<HttpResponse> {
    let fields = vec![
        SortFieldMeta { field: "name", label: "Название", data_type: "string" },
        SortFieldMeta { field: "formula", label: "Формула", data_type: "string" },
        SortFieldMeta { field: "cas_number", label: "CAS номер", data_type: "string" },
        SortFieldMeta { field: "manufacturer", label: "Производитель", data_type: "string" },
        SortFieldMeta { field: "status", label: "Статус", data_type: "string" },
        SortFieldMeta { field: "created_at", label: "Дата создания", data_type: "datetime" },
        SortFieldMeta { field: "updated_at", label: "Дата обновления", data_type: "datetime" },
        SortFieldMeta { field: "total_quantity", label: "Общее количество", data_type: "number" },
        SortFieldMeta { field: "available_quantity", label: "Доступное количество", data_type: "number" },
        SortFieldMeta { field: "batches_count", label: "Количество партий", data_type: "number" },
        SortFieldMeta { field: "molecular_weight", label: "Молекулярная масса", data_type: "number" },
    ];

    let meta = ReagentsPaginationMeta {
        available_sort_fields: fields,
        default_sort_field: "created_at",
        default_sort_order: "DESC",
        default_per_page: 20,
        max_per_page: 100,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(meta)))
}

// ==================== CURSOR-BASED PAGINATION ====================

#[derive(Debug, Deserialize)]
pub struct CursorPaginationQuery {
    /// Курсор - ID последнего загруженного элемента
    pub cursor: Option<String>,
    /// Количество элементов (по умолчанию 30)
    pub limit: Option<i64>,
    /// Поиск
    pub search: Option<String>,
    /// Фильтр по статусу
    pub status: Option<String>,
    /// Фильтр по производителю
    pub manufacturer: Option<String>,
    /// Направление: "next" (вперёд) или "prev" (назад)
    pub direction: Option<String>,
}

/// GET /api/v1/reagents/cursor - Cursor-based пагинация (эффективна для 100k+ записей)
pub async fn get_reagents_cursor(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<CursorPaginationQuery>,
) -> ApiResult<HttpResponse> {
    let limit = query.limit.unwrap_or(30).clamp(1, 100);
    let direction = query.direction.as_deref().unwrap_or("next");
    
    // Подсчёт общего количества
    let mut count_sql = "SELECT COUNT(*) FROM reagents WHERE 1=1".to_string();
    let mut count_params: Vec<String> = Vec::new();
    
    if let Some(ref search) = query.search {
        let pattern = format!("%{}%", search.trim());
        count_sql.push_str(" AND (name LIKE ? OR formula LIKE ? OR cas_number LIKE ? OR manufacturer LIKE ?)");
        for _ in 0..4 {
            count_params.push(pattern.clone());
        }
    }
    if let Some(ref status) = query.status {
        count_sql.push_str(" AND status = ?");
        count_params.push(status.clone());
    }
    if let Some(ref manufacturer) = query.manufacturer {
        count_sql.push_str(" AND manufacturer = ?");
        count_params.push(manufacturer.clone());
    }
    
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for p in &count_params {
        count_query = count_query.bind(p);
    }
    let total: i64 = count_query.fetch_one(&app_state.db_pool).await?;
    
    // Основной запрос с курсором
    let base_sql = r#"
        SELECT 
            r.id, r.name, r.formula, r.cas_number, r.manufacturer,
            r.molecular_weight, r.physical_state, r.description, 
            r.storage_conditions, r.appearance, r.hazard_pictograms, r.status,
            r.created_by, r.updated_by, r.created_at, r.updated_at,
            CAST(COALESCE(SUM(CASE WHEN b.status = 'available' THEN b.quantity ELSE 0.0 END), 0.0) AS REAL) as total_quantity,
            CAST(COALESCE(SUM(CASE WHEN b.status = 'available' THEN b.reserved_quantity ELSE 0.0 END), 0.0) AS REAL) as reserved_quantity,
            CAST(COALESCE(SUM(CASE WHEN b.status = 'available' THEN b.quantity - b.reserved_quantity ELSE 0.0 END), 0.0) AS REAL) as available_quantity,
            COUNT(b.id) as batches_count,
            (SELECT bu.unit FROM batches bu WHERE bu.reagent_id = r.id AND bu.status = 'available' LIMIT 1) as unit,
            CASE 
                WHEN COALESCE(SUM(CASE WHEN b.status = 'available' THEN b.quantity ELSE 0.0 END), 0.0) > 0 
                THEN CAST(ROUND(CAST(COALESCE(SUM(CASE WHEN b.status = 'available' THEN b.quantity ELSE 0.0 END), 0.0) AS REAL), 2) AS TEXT) 
                     || ' ' || COALESCE((SELECT bu.unit FROM batches bu WHERE bu.reagent_id = r.id AND bu.status = 'available' LIMIT 1), '')
                ELSE 'No stock'
            END as total_display
        FROM reagents r
        LEFT JOIN batches b ON r.id = b.reagent_id
        WHERE 1=1
    "#;
    
    let mut sql = base_sql.to_string();
    let mut params: Vec<String> = Vec::new();
    
    // Фильтры
    if let Some(ref search) = query.search {
        let pattern = format!("%{}%", search.trim());
        sql.push_str(" AND (r.name LIKE ? OR r.formula LIKE ? OR r.cas_number LIKE ? OR r.manufacturer LIKE ?)");
        for _ in 0..4 {
            params.push(pattern.clone());
        }
    }
    if let Some(ref status) = query.status {
        sql.push_str(" AND r.status = ?");
        params.push(status.clone());
    }
    if let Some(ref manufacturer) = query.manufacturer {
        sql.push_str(" AND r.manufacturer = ?");
        params.push(manufacturer.clone());
    }
    
    // Cursor condition - эффективнее чем OFFSET
    if let Some(ref cursor) = query.cursor {
        if direction == "prev" {
            sql.push_str(" AND r.created_at > (SELECT created_at FROM reagents WHERE id = ?)");
        } else {
            sql.push_str(" AND r.created_at < (SELECT created_at FROM reagents WHERE id = ?)");
        }
        params.push(cursor.clone());
    }
    
    sql.push_str(" GROUP BY r.id");
    
    // Порядок сортировки
    if direction == "prev" {
        sql.push_str(" ORDER BY r.created_at ASC");
    } else {
        sql.push_str(" ORDER BY r.created_at DESC");
    }
    
    sql.push_str(" LIMIT ?");
    
    let mut db_query = sqlx::query_as::<_, ReagentListItem>(&sql);
    for p in &params {
        db_query = db_query.bind(p);
    }
    db_query = db_query.bind(limit + 1); // +1 чтобы проверить has_more
    
    let mut reagents: Vec<ReagentListItem> = db_query
        .fetch_all(&app_state.db_pool)
        .await?;
    
    // Определяем есть ли ещё данные
    let has_more = reagents.len() > limit as usize;
    if has_more {
        reagents.pop(); // Убираем лишний элемент
    }
    
    // Для prev направления нужно перевернуть результат
    if direction == "prev" {
        reagents.reverse();
    }
    
    // Курсоры для навигации
    let next_cursor = if has_more || direction == "prev" {
        reagents.last().map(|r| r.id.clone())
    } else {
        None
    };
    
    let prev_cursor = reagents.first().map(|r| r.id.clone());
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(CursorPaginatedResponse {
        data: reagents,
        next_cursor,
        prev_cursor,
        has_more,
        total,
    })))
}