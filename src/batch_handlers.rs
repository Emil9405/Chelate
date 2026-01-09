// src/batch_handlers.rs
//! Обработчики для партий реагентов
//! ОБНОВЛЕНО: интеграция с query_builders для безопасных SQL-запросов

use actix_web::{web, HttpResponse};
use std::sync::Arc;
use crate::AppState;
use crate::models::*;
use crate::error::{ApiError, ApiResult, validate_quantity, validate_unit};
use crate::handlers::{ApiResponse, PaginatedResponse};
use crate::validator::{CustomValidate, UnitConverter};
use crate::query_builders::{SafeQueryBuilder, FieldWhitelist};
use chrono::{Utc, DateTime};
use uuid::Uuid;
use validator::Validate;
use serde::Serialize;

// ==================== RESPONSE STRUCTURES ====================

/// Партия с расширенной информацией (статус срока годности, конвертация)
#[derive(Debug, Serialize)]
pub struct BatchResponse {
    pub id: String,
    pub reagent_id: String,
    pub lot_number: Option<String>,
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
    pub expiration_status: String,
    pub days_until_expiration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converted_quantity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converted_unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_unit: Option<String>,
}

/// Партия с именем реагента
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BatchWithReagent {
    pub id: String,
    pub reagent_id: String,
    pub lot_number: Option<String>,
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
    pub reagent_name: String,
}

/// Расширенный ответ партии с реагентом
#[derive(Debug, Serialize)]
pub struct BatchWithReagentResponse {
    pub id: String,
    pub reagent_id: String,
    pub reagent_name: String,
    pub lot_number: Option<String>,
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

// ==================== EXPIRATION STATUS ====================

const EXPIRY_CRITICAL_DAYS: i64 = 7;
const EXPIRY_WARNING_DAYS: i64 = 30;

fn calculate_expiration_status(expiry_date: Option<DateTime<Utc>>) -> (String, Option<i64>) {
    match expiry_date {
        None => ("unknown".to_string(), None),
        Some(date) => {
            let now = Utc::now();
            let days = (date - now).num_days();
            let status = if days < 0 {
                "expired"
            } else if days <= EXPIRY_CRITICAL_DAYS {
                "expiring_critical"
            } else if days <= EXPIRY_WARNING_DAYS {
                "expiring_soon"
            } else {
                "ok"
            };
            (status.to_string(), Some(days))
        }
    }
}

// ==================== UNIT CONVERSION ====================

fn convert_quantity(quantity: f64, from_unit: &str, to_unit: &str) -> Result<f64, String> {
    if from_unit == to_unit {
        return Ok(quantity);
    }
    let converter = UnitConverter::new();
    converter.convert(quantity, from_unit, to_unit)
}

// ==================== BATCH QUERY ====================

#[derive(Debug, serde::Deserialize)]
pub struct BatchQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
    pub status: Option<String>,
    pub unit: Option<String>,
}

impl BatchQuery {
    pub fn normalize(&self) -> (i64, i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * per_page;
        (page, per_page, offset)
    }
}

// ==================== WHITELIST для партий с JOIN ====================

fn get_batch_join_whitelist() -> FieldWhitelist {
    FieldWhitelist::new("batches",&[
        // Поля batches (с алиасом b.)
        "b.id", "b.reagent_id", "b.batch_number", "b.lot_number", "b.cat_number",
        "b.quantity", "b.original_quantity", "b.reserved_quantity", "b.unit",
        "b.expiry_date", "b.supplier", "b.manufacturer", "b.received_date",
        "b.status", "b.location", "b.notes", "b.created_at", "b.updated_at",
        "r.name", "r.id", "r.formula", "r.cas_number",
    ])
}

// ==================== BATCH CRUD ====================

/// Получить все партии с пагинацией
/// Использует SafeQueryBuilder для безопасных SQL-запросов
pub async fn get_all_batches(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<BatchQuery>,
) -> ApiResult<HttpResponse> {
    let (page, per_page, _offset) = query.normalize();

    let whitelist = get_batch_join_whitelist();
    
    // Безопасное построение запроса через SafeQueryBuilder
    // Примечание: SafeQueryBuilder из mod.rs принимает base_query
    let base_query = "SELECT b.*, r.name as reagent_name FROM batches b JOIN reagents r ON b.reagent_id = r.id";
    let mut builder = crate::query_builders::SafeQueryBuilder::new(base_query)
        .map_err(|e| ApiError::bad_request(&e))?
        .with_whitelist(&whitelist);

    // Добавляем условия поиска
    if let Some(ref search) = query.search {
        let trimmed = search.trim();
        if !trimmed.is_empty() {
            // Для сложного OR условия используем add_condition
            let pattern = format!("%{}%", trimmed);
            let or_condition = "(b.batch_number LIKE ? OR r.name LIKE ? OR b.cat_number LIKE ? OR b.supplier LIKE ?)";
            builder.add_condition(or_condition, vec![
                pattern.clone(), 
                pattern.clone(), 
                pattern.clone(), 
                pattern
            ]);
        }
    }

    if let Some(ref status) = query.status {
        builder.add_exact_match("b.status", status);
    }

    // Сортировка и пагинация
    builder
        .order_by("b.created_at", "DESC")
        .limit(per_page)
        .offset((page - 1) * per_page);

    // Построение запросов
    let (count_sql, count_params) = builder.build_count();
    let (select_sql, select_params) = builder.build();

    // Выполнение COUNT запроса
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for p in &count_params {
        count_query = count_query.bind(p);
    }
    let total: i64 = count_query.fetch_one(&app_state.db_pool).await?;

    // Выполнение SELECT запроса
    let mut select_query = sqlx::query_as::<_, BatchWithReagent>(&select_sql);
    for p in &select_params {
        select_query = select_query.bind(p);
    }
    let batches: Vec<BatchWithReagent> = select_query.fetch_all(&app_state.db_pool).await?;

    // Transform to response with expiration status
    let response_batches: Vec<BatchWithReagentResponse> = batches
        .into_iter()
        .map(|b| {
            let (expiration_status, days_until_expiration) = calculate_expiration_status(b.expiry_date);
            BatchWithReagentResponse {
                id: b.id,
                reagent_id: b.reagent_id,
                reagent_name: b.reagent_name,
                lot_number: b.lot_number,
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
                days_until_expiration,
            }
        })
        .collect();

    let total_pages = (total + per_page - 1) / per_page;

    Ok(HttpResponse::Ok().json(ApiResponse::success(PaginatedResponse {
        data: response_batches,
        total,
        page,
        per_page,
        total_pages,
    })))
}

/// Получить одну партию по ID
pub async fn get_batch(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
) -> ApiResult<HttpResponse> {
    let (reagent_id, batch_id) = path.into_inner();

    let whitelist = FieldWhitelist::for_batches();
    let mut builder = crate::query_builders::SafeQueryBuilder::new("SELECT * FROM batches")
        .map_err(|e| ApiError::bad_request(&e))?
        .with_whitelist(&whitelist);

    builder
        .add_exact_match("id", &batch_id)
        .add_exact_match("reagent_id", &reagent_id);

    let (sql, params) = builder.build();
    
    let mut query = sqlx::query_as::<_, Batch>(&sql);
    for p in &params {
        query = query.bind(p);
    }

    let batch = query
        .fetch_optional(&app_state.db_pool)
        .await?
        .ok_or_else(|| ApiError::not_found("Batch"))?;

    let (expiration_status, days_until_expiration) = calculate_expiration_status(batch.expiry_date);
    
    let response = BatchResponse {
        id: batch.id,
        reagent_id: batch.reagent_id,
        lot_number: batch.lot_number,
        batch_number: batch.batch_number,
        cat_number: batch.cat_number,
        quantity: batch.quantity,
        original_quantity: batch.original_quantity,
        reserved_quantity: batch.reserved_quantity,
        unit: batch.unit,
        expiry_date: batch.expiry_date,
        supplier: batch.supplier,
        manufacturer: batch.manufacturer,
        received_date: batch.received_date,
        status: batch.status,
        location: batch.location,
        notes: batch.notes,
        created_by: batch.created_by,
        updated_by: batch.updated_by,
        created_at: batch.created_at,
        updated_at: batch.updated_at,
        expiration_status,
        days_until_expiration,
        converted_quantity: None,
        converted_unit: None,
        original_unit: None,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

/// Создать новую партию
pub async fn create_batch(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    batch_data: web::Json<CreateBatchRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let reagent_id = path.into_inner();
    
    // Валидация
    batch_data.validate().map_err(|e| ApiError::ValidationError(e.to_string()))?;
    
    let custom_validation = batch_data.custom_validate();
    if !custom_validation.is_valid() {
        return Err(custom_validation.to_api_error());
    }

    // Проверка существования реагента
    let _: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ?")
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Reagent"))?;

    let now = Utc::now();
    let batch_id = Uuid::new_v4().to_string();
    let received_date = batch_data.received_date.unwrap_or(now);

    sqlx::query(
        r#"INSERT INTO batches (
            id, reagent_id, lot_number, batch_number, cat_number,
            quantity, original_quantity, reserved_quantity, unit,
            expiry_date, supplier, manufacturer, received_date,
            status, location, notes, created_by, updated_by,
            created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, 0.0, ?, ?, ?, ?, ?, 'available', ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&batch_id)
    .bind(&reagent_id)
    .bind(&batch_data.lot_number)
    .bind(&batch_data.batch_number)
    .bind(&batch_data.cat_number)
    .bind(batch_data.quantity)
    .bind(batch_data.quantity)  // original_quantity
    .bind(&batch_data.unit)
    .bind(&batch_data.expiry_date)
    .bind(&batch_data.supplier)
    .bind(&batch_data.manufacturer)
    .bind(&received_date)
    .bind(&batch_data.location)
    .bind(&batch_data.notes)
    .bind(&user_id)
    .bind(&user_id)
    .bind(&now)
    .bind(&now)
    .execute(&app_state.db_pool)
    .await?;

    let batch: Batch = sqlx::query_as("SELECT * FROM batches WHERE id = ?")
        .bind(&batch_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    let (expiration_status, days_until_expiration) = calculate_expiration_status(batch.expiry_date);

    let response = BatchResponse {
        id: batch.id,
        reagent_id: batch.reagent_id,
        lot_number: batch.lot_number,
        batch_number: batch.batch_number,
        cat_number: batch.cat_number,
        quantity: batch.quantity,
        original_quantity: batch.original_quantity,
        reserved_quantity: batch.reserved_quantity,
        unit: batch.unit,
        expiry_date: batch.expiry_date,
        supplier: batch.supplier,
        manufacturer: batch.manufacturer,
        received_date: batch.received_date,
        status: batch.status,
        location: batch.location,
        notes: batch.notes,
        created_by: batch.created_by,
        updated_by: batch.updated_by,
        created_at: batch.created_at,
        updated_at: batch.updated_at,
        expiration_status,
        days_until_expiration,
        converted_quantity: None,
        converted_unit: None,
        original_unit: None,
    };

    Ok(HttpResponse::Created().json(ApiResponse::success(response)))
}

/// Обновить партию
pub async fn update_batch(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    batch_data: web::Json<UpdateBatchRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let (reagent_id, batch_id) = path.into_inner();
    
    batch_data.validate().map_err(|e| ApiError::ValidationError(e.to_string()))?;

    // Проверка существования
    let existing: Batch = sqlx::query_as("SELECT * FROM batches WHERE id = ? AND reagent_id = ?")
        .bind(&batch_id)
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Batch"))?;

    let now = Utc::now();

    sqlx::query(
        r#"UPDATE batches SET
            lot_number = COALESCE(?, lot_number),
            batch_number = COALESCE(?, batch_number),
            cat_number = COALESCE(?, cat_number),
            quantity = COALESCE(?, quantity),
            unit = COALESCE(?, unit),
            expiry_date = COALESCE(?, expiry_date),
            supplier = COALESCE(?, supplier),
            manufacturer = COALESCE(?, manufacturer),
            status = COALESCE(?, status),
            location = COALESCE(?, location),
            notes = COALESCE(?, notes),
            updated_by = ?,
            updated_at = ?
        WHERE id = ? AND reagent_id = ?"#,
    )
    .bind(&batch_data.lot_number)
    .bind(&batch_data.batch_number)
    .bind(&batch_data.cat_number)
    .bind(&batch_data.quantity)
    .bind(&batch_data.unit)
    .bind(&batch_data.expiry_date)
    .bind(&batch_data.supplier)
    .bind(&batch_data.manufacturer)
    .bind(&batch_data.status)
    .bind(&batch_data.location)
    .bind(&batch_data.notes)
    .bind(&user_id)
    .bind(&now)
    .bind(&batch_id)
    .bind(&reagent_id)
    .execute(&app_state.db_pool)
    .await?;

    let batch: Batch = sqlx::query_as("SELECT * FROM batches WHERE id = ?")
        .bind(&batch_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    let (expiration_status, days_until_expiration) = calculate_expiration_status(batch.expiry_date);

    let response = BatchResponse {
        id: batch.id,
        reagent_id: batch.reagent_id,
        lot_number: batch.lot_number,
        batch_number: batch.batch_number,
        cat_number: batch.cat_number,
        quantity: batch.quantity,
        original_quantity: batch.original_quantity,
        reserved_quantity: batch.reserved_quantity,
        unit: batch.unit,
        expiry_date: batch.expiry_date,
        supplier: batch.supplier,
        manufacturer: batch.manufacturer,
        received_date: batch.received_date,
        status: batch.status,
        location: batch.location,
        notes: batch.notes,
        created_by: batch.created_by,
        updated_by: batch.updated_by,
        created_at: batch.created_at,
        updated_at: batch.updated_at,
        expiration_status,
        days_until_expiration,
        converted_quantity: None,
        converted_unit: None,
        original_unit: None,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

/// Удалить партию
pub async fn delete_batch(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let (reagent_id, batch_id) = path.into_inner();

    let result = sqlx::query("DELETE FROM batches WHERE id = ? AND reagent_id = ?")
        .bind(&batch_id)
        .bind(&reagent_id)
        .execute(&app_state.db_pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Batch"));
    }

    log::info!("🗑️ Batch {} deleted by user {}", batch_id, user_id);

    Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_with_message((), "Batch deleted successfully".to_string())))
}

// ==================== EXPIRING BATCHES ====================

#[derive(Debug, serde::Deserialize)]
pub struct ExpiringQuery {
    pub days: Option<i64>,
}

/// Получить партии с истекающим сроком годности
pub async fn get_expiring_batches(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<ExpiringQuery>,
) -> ApiResult<HttpResponse> {
    let days = query.days.unwrap_or(30);
    let expiry_threshold = Utc::now() + chrono::Duration::days(days);

    let whitelist = get_batch_join_whitelist();
    let base_query = "SELECT b.*, r.name as reagent_name FROM batches b JOIN reagents r ON b.reagent_id = r.id";
    let mut builder = crate::query_builders::SafeQueryBuilder::new(base_query)
        .map_err(|e| ApiError::bad_request(&e))?
        .with_whitelist(&whitelist);

    builder
        .add_is_not_null("b.expiry_date")
        .add_comparison("b.expiry_date", "<=", expiry_threshold.to_rfc3339())
        .add_exact_match("b.status", "available")
        .order_by("b.expiry_date", "ASC");

    let (sql, params) = builder.build();

    let mut select_query = sqlx::query_as::<_, BatchWithReagent>(&sql);
    for p in &params {
        select_query = select_query.bind(p);
    }
    let batches: Vec<BatchWithReagent> = select_query.fetch_all(&app_state.db_pool).await?;

    let response: Vec<BatchWithReagentResponse> = batches
        .into_iter()
        .map(|b| {
            let (expiration_status, days_until_expiration) = calculate_expiration_status(b.expiry_date);
            BatchWithReagentResponse {
                id: b.id,
                reagent_id: b.reagent_id,
                reagent_name: b.reagent_name,
                lot_number: b.lot_number,
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
                days_until_expiration,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

// ==================== LOW STOCK BATCHES ====================

#[derive(Debug, serde::Deserialize)]
pub struct LowStockQuery {
    pub threshold: Option<f64>,
}

/// Получить партии с низким остатком
pub async fn get_low_stock_batches(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<LowStockQuery>,
) -> ApiResult<HttpResponse> {
    let threshold_percentage = query.threshold.unwrap_or(20.0);

    // Для сложного условия используем raw SQL, но безопасно
    let batches: Vec<BatchWithReagent> = sqlx::query_as(r#"
        SELECT b.*, r.name as reagent_name
        FROM batches b
        JOIN reagents r ON b.reagent_id = r.id
        WHERE b.status = 'available'
          AND b.original_quantity > 0
          AND (b.quantity / b.original_quantity * 100) <= ?
        ORDER BY (b.quantity / b.original_quantity) ASC
    "#)
        .bind(threshold_percentage)
        .fetch_all(&app_state.db_pool)
        .await?;

    let response: Vec<BatchWithReagentResponse> = batches
        .into_iter()
        .map(|b| {
            let (expiration_status, days_until_expiration) = calculate_expiration_status(b.expiry_date);
            BatchWithReagentResponse {
                id: b.id,
                reagent_id: b.reagent_id,
                reagent_name: b.reagent_name,
                lot_number: b.lot_number,
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
                days_until_expiration,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

// ==================== UNIT CONVERSION ENDPOINT ====================

#[derive(Debug, serde::Deserialize)]
pub struct ConvertUnitRequest {
    pub quantity: f64,
    pub from_unit: String,
    pub to_unit: String,
}

#[derive(Debug, Serialize)]
pub struct ConvertUnitResponse {
    pub original_quantity: f64,
    pub original_unit: String,
    pub converted_quantity: f64,
    pub converted_unit: String,
}

pub async fn convert_units(
    request: web::Json<ConvertUnitRequest>,
) -> ApiResult<HttpResponse> {
    let converter = UnitConverter::new();
    
    let converted = converter
        .convert(request.quantity, &request.from_unit, &request.to_unit)
        .map_err(|e| ApiError::bad_request(&e))?;

    let response = ConvertUnitResponse {
        original_quantity: request.quantity,
        original_unit: request.from_unit.clone(),
        converted_quantity: converted,
        converted_unit: request.to_unit.clone(),
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

// ==================== BATCHES FOR REAGENT ====================

pub async fn get_batches_for_reagent(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    query: web::Query<BatchQuery>,
) -> ApiResult<HttpResponse> {
    let reagent_id = path.into_inner();
    let (page, per_page, _offset) = query.normalize();

    // Проверка существования реагента
    let _: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ?")
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Reagent"))?;

    let whitelist = FieldWhitelist::for_batches();
    let mut builder = crate::query_builders::SafeQueryBuilder::new("SELECT * FROM batches b")
        .map_err(|e| ApiError::bad_request(&e))?
        .with_whitelist(&whitelist);

    builder.add_exact_match("reagent_id", &reagent_id);

    if let Some(ref status) = query.status {
        builder.add_exact_match("status", status);
    }

    builder
        .order_by("received_date", "DESC")
        .limit(per_page)
        .offset((page - 1) * per_page);

    // Count
    let (count_sql, count_params) = builder.build_count();
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for p in &count_params {
        count_query = count_query.bind(p);
    }
    let total: i64 = count_query.fetch_one(&app_state.db_pool).await?;

    // Select
    let (sql, params) = builder.build();
    let mut select_query = sqlx::query_as::<_, Batch>(&sql);
    for p in &params {
        select_query = select_query.bind(p);
    }
    let batches: Vec<Batch> = select_query.fetch_all(&app_state.db_pool).await?;

    // Transform
    let response_batches: Vec<BatchResponse> = batches
        .into_iter()
        .map(|b| {
            let (expiration_status, days_until_expiration) = calculate_expiration_status(b.expiry_date);
            BatchResponse {
                id: b.id,
                reagent_id: b.reagent_id,
                lot_number: b.lot_number,
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
                created_by: b.created_by,
                updated_by: b.updated_by,
                created_at: b.created_at,
                updated_at: b.updated_at,
                expiration_status,
                days_until_expiration,
                converted_quantity: None,
                converted_unit: None,
                original_unit: None,
            }
        })
        .collect();

    let total_pages = (total + per_page - 1) / per_page;

    Ok(HttpResponse::Ok().json(ApiResponse::success(PaginatedResponse {
        data: response_batches,
        total,
        page,
        per_page,
        total_pages,
    })))
}