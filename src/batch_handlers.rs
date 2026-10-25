// src/batch_handlers.rs
//! Обработчики для партий реагентов
//! ОБНОВЛЕНО: интеграция с query_builders для безопасных SQL-запросов

use actix_web::{web, HttpResponse, HttpRequest};
use std::sync::Arc;
use crate::AppState;
use crate::models::*;
use crate::error::{ApiError, ApiResult, validate_quantity, validate_unit};
use crate::auth::get_current_user;
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
    pub pack_size: Option<f64>,
    pub pack_count: Option<i64>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placements: Option<Vec<PlacementWithRoom>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unplaced_quantity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opened_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placed_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unplaced_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_summary: Option<String>,
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
    pub pack_size: Option<f64>,
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
    pub deleted_at: Option<DateTime<Utc>>,
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
    pub pack_size: Option<f64>,
    pub pack_count: Option<i64>,
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

// ==================== PACK COUNT CALCULATION ====================

/// Вычисляет количество упаковок: ceil(quantity / pack_size)
fn calculate_pack_count(quantity: f64, pack_size: Option<f64>) -> Option<i64> {
    pack_size.map(|ps| (quantity / ps).ceil() as i64)
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

    // Исключаем удалённые батчи
    builder.add_condition("b.deleted_at IS NULL", vec![]);

    // Скрываем depleted по умолчанию — их видно только в Reports
    // Если пользователь явно запросил ?status=depleted — пропускаем фильтр
    if query.status.as_deref() != Some("depleted") {
        builder.add_condition("b.status != 'depleted'", vec![]);
    }

    // Добавляем условия поиска
    if let Some(ref search) = query.search {
        let trimmed = search.trim();
        if !trimmed.is_empty() {
            let (or_condition, params) = crate::query_builders::utils::like_any(
                &["b.batch_number", "r.name", "b.cat_number", "b.supplier"],
                trimmed,
            );
            builder.add_condition(&or_condition, params);
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
    let batch_ids: Vec<&str> = batches.iter().map(|b| b.id.as_str()).collect();
    
    // Container stats per batch
    let container_stats_map = if !batch_ids.is_empty() {
        let placeholders = batch_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        #[derive(sqlx::FromRow)]
        struct CStats {
            batch_id: String,
            container_count: i64,
            opened_count: i64,
            placed_count: i64,
        }
        let sql = format!(
            r#"SELECT 
                bc.batch_id, 
                COUNT(*) as container_count, 
                SUM(CASE WHEN bc.is_opened = 1 THEN 1 ELSE 0 END) as opened_count, 
                SUM(CASE WHEN bp.id IS NOT NULL THEN 1 ELSE 0 END) as placed_count 
            FROM batch_containers bc 
            LEFT JOIN batch_placements bp ON bp.container_id = bc.id 
            WHERE bc.batch_id IN ({}) AND bc.status NOT IN ('disposed', 'empty') 
            GROUP BY bc.batch_id"#,
            placeholders
        );
        let mut q = sqlx::query_as::<_, CStats>(&sql);
        for id in &batch_ids { q = q.bind(id); }
        let rows: Vec<CStats> = q.fetch_all(&app_state.db_pool).await.unwrap_or_default();
        let mut map: std::collections::HashMap<String, (i64, i64, i64)> = std::collections::HashMap::new();
        for r in rows { map.insert(r.batch_id, (r.container_count, r.opened_count, r.placed_count)); }
        map
    } else {
        std::collections::HashMap::new()
    };

    // Location summary per batch
    let location_map: std::collections::HashMap<String, String> = if !batch_ids.is_empty() {
        let placeholders = batch_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        #[derive(sqlx::FromRow)]
        struct LocRow { batch_id: String, loc_summary: String }
        let sql = format!(
            r#"SELECT bc.batch_id,
            GROUP_CONCAT(DISTINCT r.name || ' → ' || sz.name || ' → ' || sp.name) as loc_summary
            FROM batch_containers bc
            JOIN batch_placements bp ON bp.container_id = bc.id
            JOIN storage_positions sp ON sp.id = bp.position_id
            JOIN storage_zones sz ON sz.id = sp.zone_id
            JOIN rooms r ON r.id = sz.room_id
            WHERE bc.batch_id IN ({}) AND bc.status != 'disposed'
            GROUP BY bc.batch_id"#,
            placeholders
        );
        let mut q = sqlx::query_as::<_, LocRow>(&sql);
        for id in &batch_ids { q = q.bind(id); }
        q.fetch_all(&app_state.db_pool).await.unwrap_or_default()
            .into_iter().map(|r| (r.batch_id, r.loc_summary)).collect()
    } else {
        std::collections::HashMap::new()
    };

    let response_batches: Vec<BatchResponse> = batches
        .into_iter()
        .map(|b| {
            let (expiration_status, days_until_expiration) = calculate_expiration_status(b.expiry_date);
            let (cnt_count, cnt_opened, cnt_placed) =
                container_stats_map.get(&b.id).copied().unwrap_or((0, 0, 0));
            let pack_count = if cnt_count > 0 {
                Some(cnt_count)
            } else {
                calculate_pack_count(b.quantity, b.pack_size)
            };
    let loc_sum = location_map.get(&b.id).cloned();        
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
                pack_size: b.pack_size,
                pack_count,
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
                placements: None,
                unplaced_quantity: None,
                container_count: if cnt_count > 0 { Some(cnt_count) } else { None },
                location_summary: loc_sum,
                opened_count: if cnt_count > 0 { Some(cnt_opened) } else { None },
                placed_count: if cnt_count > 0 { Some(cnt_placed) } else { None },
                unplaced_count: if cnt_count > 0 { Some(cnt_count - cnt_placed) } else { None },
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
        .add_exact_match("reagent_id", &reagent_id)
        .add_condition("deleted_at IS NULL", vec![]);

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
    let pack_count = calculate_pack_count(batch.quantity, batch.pack_size);
    
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
        pack_size: batch.pack_size,
        pack_count,
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
        placements: None,
        unplaced_quantity: None,
        container_count: None,
        opened_count: None,
        placed_count: None,
        unplaced_count: None,
        location_summary: None,
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
    let _: Reagent = sqlx::query_as("SELECT * FROM reagents WHERE id = ? AND deleted_at IS NULL")
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Reagent"))?;

    let now = Utc::now();
    let batch_id = Uuid::new_v4().to_string();
    let received_date = batch_data.received_date.unwrap_or(now);
    
    let deleted_dup: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM batches WHERE reagent_id = ? AND batch_number = ? AND deleted_at IS NOT NULL"
    )
    .bind(&reagent_id)
    .bind(&batch_data.batch_number)
    .fetch_optional(&app_state.db_pool)
    .await?;

    if let Some((old_id,)) = deleted_dup {
        sqlx::query(
            "UPDATE batches SET batch_number = batch_number || '_deleted_' || id WHERE id = ?"
        )
        .bind(&old_id)
        .execute(&app_state.db_pool)
        .await?;
    }
    
    sqlx::query(
        r#"INSERT INTO batches (
            id, reagent_id, lot_number, batch_number, cat_number,
            quantity, original_quantity, reserved_quantity, unit, pack_size,
            expiry_date, supplier, manufacturer, received_date,
            status, location, notes, created_by, updated_by,
            created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, 0.0, ?, ?, ?, ?, ?, ?, 'available', ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&batch_id)
    .bind(&reagent_id)
    .bind(&batch_data.lot_number)
    .bind(&batch_data.batch_number)
    .bind(&batch_data.cat_number)
    .bind(batch_data.quantity)
    .bind(batch_data.quantity)  // original_quantity
    .bind(&batch_data.unit)
    .bind(&batch_data.pack_size)
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
    let pack_count = calculate_pack_count(batch.quantity, batch.pack_size);

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
        pack_size: batch.pack_size,
        pack_count,
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
        placements: None,
        unplaced_quantity: None,
        container_count: None,
        opened_count: None,
        placed_count: None,
        unplaced_count: None,
        location_summary: None,
    };

    Ok(HttpResponse::Created().json(ApiResponse::success(response)))
}

/// Обновить партию
/// Uses dynamic SET builder (like update_reagent) instead of COALESCE
/// so that empty-string values can actually clear nullable text fields.
pub async fn update_batch(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    batch_data: web::Json<UpdateBatchRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let (reagent_id, batch_id) = path.into_inner();
    
    batch_data.validate().map_err(|e| ApiError::ValidationError(e.to_string()))?;

    // Проверка существования
    let existing: Batch = sqlx::query_as("SELECT * FROM batches WHERE id = ? AND reagent_id = ? AND deleted_at IS NULL")
        .bind(&batch_id)
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Batch"))?;

    let now = Utc::now();

    // Build dynamic SET clause — only include fields present in request
    let mut sets: Vec<String> = Vec::new();
    
    // We'll collect typed bind values separately since they have different types
    // Strategy: build the full SQL then bind in order

    // Track which fields to bind (in order)
    #[derive(Debug)]
    enum BindValue {
        Str(String),
        OptStr(Option<String>),
        F64(f64),
        OptF64(Option<f64>),
        DateTime(chrono::DateTime<Utc>),
        OptDateTime(Option<chrono::DateTime<Utc>>),
    }

    let mut binds: Vec<BindValue> = Vec::new();

    macro_rules! upd_str {
        ($field:ident, $col:expr) => {
            if let Some(ref v) = batch_data.$field {
                sets.push(format!("{} = ?", $col));
                // Empty string → NULL in DB for cleaner semantics
                if v.is_empty() {
                    binds.push(BindValue::OptStr(None));
                } else {
                    binds.push(BindValue::OptStr(Some(v.clone())));
                }
            }
        };
    }

    macro_rules! upd_f64 {
        ($field:ident, $col:expr) => {
            if let Some(v) = batch_data.$field {
                sets.push(format!("{} = ?", $col));
                binds.push(BindValue::F64(v));
            }
        };
    }

    macro_rules! upd_dt {
        ($field:ident, $col:expr) => {
            if let Some(ref v) = batch_data.$field {
                sets.push(format!("{} = ?", $col));
                binds.push(BindValue::DateTime(*v));
            }
        };
    }

    upd_str!(lot_number, "lot_number");
    upd_str!(batch_number, "batch_number");
    upd_str!(cat_number, "cat_number");
    upd_f64!(quantity, "quantity");
    upd_str!(unit, "unit");
    upd_f64!(pack_size, "pack_size");
    upd_dt!(expiry_date, "expiry_date");
    upd_str!(supplier, "supplier");
    upd_str!(manufacturer, "manufacturer");
    upd_str!(status, "status");
    upd_str!(location, "location");
    upd_str!(notes, "notes");
    upd_dt!(received_date, "received_date");

    if sets.is_empty() {
        return Err(ApiError::BadRequest("No fields to update".to_string()));
    }

    // Always update audit fields
    sets.push("updated_by = ?".to_string());
    sets.push("updated_at = ?".to_string());

    let sql = format!(
        "UPDATE batches SET {} WHERE id = ? AND reagent_id = ?",
        sets.join(", ")
    );

    let mut query = sqlx::query(&sql);

    // Bind all dynamic values in order
    for b in &binds {
        match b {
            BindValue::Str(v) => { query = query.bind(v); }
            BindValue::OptStr(v) => { query = query.bind(v); }
            BindValue::F64(v) => { query = query.bind(v); }
            BindValue::OptF64(v) => { query = query.bind(v); }
            BindValue::DateTime(v) => { query = query.bind(v); }
            BindValue::OptDateTime(v) => { query = query.bind(v); }
        }
    }

    // Bind audit + WHERE params
    query = query.bind(&user_id);
    query = query.bind(&now);
    query = query.bind(&batch_id);
    query = query.bind(&reagent_id);

    query.execute(&app_state.db_pool).await?;
    // === Sync containers when quantity changes ===
    if let Some(new_qty) = batch_data.quantity {
        let old_qty = existing.quantity;
        if (new_qty - old_qty).abs() > 0.001 {
            // Check if containers exist for this batch
            let container_total: Option<(f64,)> = sqlx::query_as(
                "SELECT COALESCE(SUM(original_quantity), 0) FROM batch_containers WHERE batch_id = ? AND status != 'disposed'"
            )
                .bind(&batch_id)
                .fetch_optional(&app_state.db_pool)
                .await?;

            if let Some((current_container_total,)) = container_total {
                if current_container_total > 0.0 {
                    let delta = new_qty - current_container_total;
                    if delta > 0.001 {
                        // Quantity increased → create new container(s) for the difference
                        let pack_size = batch_data.pack_size
                            .or(existing.pack_size)
                            .unwrap_or(delta);
                        let mut remaining = delta;
                        let max_seq: (i64,) = sqlx::query_as(
                            "SELECT COALESCE(MAX(sequence_number), 0) FROM batch_containers WHERE batch_id = ?"
                        )
                            .bind(&batch_id)
                            .fetch_one(&app_state.db_pool)
                            .await?;
                        let mut seq = max_seq.0;

                        while remaining > 0.001 {
                            seq += 1;
                            let qty = if remaining >= pack_size { pack_size } else { remaining };
                            let container_id = Uuid::new_v4().to_string();
                            sqlx::query(
                                "INSERT INTO batch_containers (id, batch_id, sequence_number, quantity, original_quantity, is_opened, status, created_at, updated_at) \
                                VALUES (?, ?, ?, ?, ?, 0, 'full', datetime('now'), datetime('now'))"
                            )
                                .bind(&container_id)
                                .bind(&batch_id)
                                .bind(seq)
                                .bind(qty)
                                .bind(qty)
                                .execute(&app_state.db_pool)
                                .await?;
                            remaining -= qty;
                        }
                        log::info!("Created new containers for batch {} (delta: +{})", batch_id, delta);
                    } else if delta < -0.001 {
                        // Quantity decreased → log warning, don't auto-delete containers
                        log::warn!(
                            "Batch {} quantity decreased by {}, but containers total {}. Manual container adjustment may be needed.",
                            batch_id, delta.abs(), current_container_total
                        );
                    }
                }
            }
        }
    }

    let batch: Batch = sqlx::query_as("SELECT * FROM batches WHERE id = ?")
        .bind(&batch_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    let (expiration_status, days_until_expiration) = calculate_expiration_status(batch.expiry_date);
    let pack_count = calculate_pack_count(batch.quantity, batch.pack_size);

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
        pack_size: batch.pack_size,
        pack_count,
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
        placements: None,
        unplaced_quantity: None,
        container_count: None,
        opened_count: None,
        placed_count: None,
        unplaced_count: None,
        location_summary: None,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

/// Удалить партию (soft delete)
pub async fn delete_batch(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let (reagent_id, batch_id) = path.into_inner();

    let _: Batch = sqlx::query_as("SELECT * FROM batches WHERE id = ? AND reagent_id = ? AND deleted_at IS NULL")
        .bind(&batch_id)
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Batch"))?;

    // Каскад в транзакции: placements → containers → batch
    crate::soft_delete::delete_batch(&app_state.db_pool, &batch_id, &user_id).await?;

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

    // Исключаем удалённые батчи
    builder.add_condition("b.deleted_at IS NULL", vec![]);

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
            let pack_count = calculate_pack_count(b.quantity, b.pack_size);
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
                pack_size: b.pack_size,
                pack_count,
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
    // Percent of the batch's original quantity; defaults to the server-wide rule
    let threshold_percentage = query.threshold
        .filter(|t| t.is_finite() && *t > 0.0 && *t <= 100.0)
        .unwrap_or_else(crate::consumption::low_stock_percent);

    // Для сложного условия используем raw SQL, но безопасно
    let batches: Vec<BatchWithReagent> = sqlx::query_as(r#"
        SELECT b.*, r.name as reagent_name
        FROM batches b
        JOIN reagents r ON b.reagent_id = r.id
        WHERE b.status IN ('available', 'low_stock', 'in_use')
          AND b.deleted_at IS NULL
          AND r.deleted_at IS NULL
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
            let pack_count = calculate_pack_count(b.quantity, b.pack_size);
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
                pack_size: b.pack_size,
                pack_count,
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

    // Исключаем удалённые батчи
    builder.add_condition("deleted_at IS NULL", vec![]);

    // Скрываем depleted по умолчанию — их видно только в Reports
    if query.status.as_deref() != Some("depleted") {
        builder.add_condition("status != 'depleted'", vec![]);
    }

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

    // === Load container stats per batch ===
    let batch_ids: Vec<&str> = batches.iter().map(|b| b.id.as_str()).collect();
    let container_stats_map = if !batch_ids.is_empty() {
        let placeholders = batch_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        #[derive(sqlx::FromRow)]
        struct CStats {
            batch_id: String,
            container_count: i64,
            opened_count: i64,
            placed_count: i64,
        }
        let sql = format!(
            r#"SELECT 
                bc.batch_id, 
                COUNT(*) as container_count, 
                SUM(CASE WHEN bc.is_opened = 1 THEN 1 ELSE 0 END) as opened_count, 
                SUM(CASE WHEN bp.id IS NOT NULL THEN 1 ELSE 0 END) as placed_count 
            FROM batch_containers bc 
            LEFT JOIN batch_placements bp ON bp.container_id = bc.id 
            WHERE bc.batch_id IN ({}) AND bc.status NOT IN ('disposed', 'empty') 
            GROUP BY bc.batch_id"#,
            placeholders
        );
        let mut q = sqlx::query_as::<_, CStats>(&sql);
        for id in &batch_ids { q = q.bind(id); }
        let rows: Vec<CStats> = q.fetch_all(&app_state.db_pool).await.unwrap_or_default();
        let mut map: std::collections::HashMap<String, (i64, i64, i64)> = std::collections::HashMap::new();
        for r in rows { map.insert(r.batch_id, (r.container_count, r.opened_count, r.placed_count)); }
        map
    } else {
        std::collections::HashMap::new()
    };

    // === Load location summary per batch ===
    let location_map: std::collections::HashMap<String, String> = if !batch_ids.is_empty() {
        let placeholders = batch_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        #[derive(sqlx::FromRow)]
        struct LocRow { batch_id: String, loc_summary: String }
        let sql = format!(
            r#"SELECT bc.batch_id,
            GROUP_CONCAT(DISTINCT r.name || ' → ' || sz.name || ' → ' || sp.name) as loc_summary
            FROM batch_containers bc
            JOIN batch_placements bp ON bp.container_id = bc.id
            JOIN storage_positions sp ON sp.id = bp.position_id
            JOIN storage_zones sz ON sz.id = sp.zone_id
            JOIN rooms r ON r.id = sz.room_id
            WHERE bc.batch_id IN ({}) AND bc.status != 'disposed'
            GROUP BY bc.batch_id"#,
            placeholders
        );
        let mut q = sqlx::query_as::<_, LocRow>(&sql);
        for id in &batch_ids { q = q.bind(id); }
        match q.fetch_all(&app_state.db_pool).await {
    Ok(rows) => {
        log::info!("Location map loaded: {} entries", rows.len());
        rows
    }
    Err(e) => {
        log::error!("Location map query failed: {}", e);
        vec![]
    }
}
            .into_iter().map(|r| (r.batch_id, r.loc_summary)).collect()
    } else {
        std::collections::HashMap::new()
    };

    // Transform with container stats
    let response_batches: Vec<BatchResponse> = batches
        .into_iter()
        .map(|b| {
            let (expiration_status, days_until_expiration) = calculate_expiration_status(b.expiry_date);
            let (cnt_count, cnt_opened, cnt_placed) =
                container_stats_map.get(&b.id).copied().unwrap_or((0, 0, 0));
            let pack_count = if cnt_count > 0 {
                Some(cnt_count)
            } else {
                calculate_pack_count(b.quantity, b.pack_size)
            };

    let loc_sum = location_map.get(&b.id).cloned();
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
                pack_size: b.pack_size,
                pack_count,
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
                placements: None,
                unplaced_quantity: None,
                container_count: if cnt_count > 0 { Some(cnt_count) } else { None },
                opened_count: if cnt_count > 0 { Some(cnt_opened) } else { None },
                placed_count: if cnt_count > 0 { Some(cnt_placed) } else { None },
                unplaced_count: if cnt_count > 0 { Some(cnt_count - cnt_placed) } else { None },
                location_summary: loc_sum,
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

// ==================== ШТУЧНОЕ СПИСАНИЕ (DISPENSE BY UNITS) ====================

/// Запрос на штучное списание
/// units_to_dispense - количество единиц (штук/бутылок/упаковок)
/// При списании: quantity -= units_to_dispense * pack_size
#[derive(Debug, serde::Deserialize, Validate)]
pub struct DispenseUnitsRequest {
    /// Количество единиц для списания (минимум 1)
    #[validate(range(min = 1, message = "Units to dispense must be at least 1"))]
    pub units_to_dispense: i64,
    
    /// Назначение использования
    #[validate(length(max = 500, message = "Purpose cannot exceed 500 characters"))]
    pub purpose: Option<String>,
    
    /// Дополнительные заметки
    #[validate(length(max = 1000, message = "Notes cannot exceed 1000 characters"))]
    pub notes: Option<String>,

    /// Explicit confirmation to use an expired batch
    pub allow_expired: Option<bool>,
    /// Required when allow_expired is set
    #[validate(length(max = 500, message = "Reason cannot exceed 500 characters"))]
    pub expired_reason: Option<String>,
}

/// Ответ на штучное списание
#[derive(Debug, Serialize)]
pub struct DispenseUnitsResponse {
    /// ID записи использования
    pub usage_id: String,
    /// Списано единиц
    pub units_dispensed: i64,
    /// Списано quantity (в базовых единицах)
    pub quantity_dispensed: f64,
    /// Единица измерения
    pub unit: String,
    /// Оставшееся quantity
    pub remaining_quantity: f64,
    /// Оставшееся количество единиц (упаковок)
    pub remaining_units: i64,
    /// Новый статус батча
    pub status: String,
}

/// Штучное списание из батча
/// 
/// POST /api/reagents/{reagent_id}/batches/{batch_id}/dispense-units
/// 
/// Логика: если батч содержит 10 единиц по 1000г (pack_size=1000, quantity=10000),
/// при dispense_units=1 -> quantity уменьшается на 1000, остается 9000г (9 единиц)
pub async fn dispense_units(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    request: web::Json<DispenseUnitsRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let (reagent_id, batch_id) = path.into_inner();
    request.validate().map_err(|e| ApiError::ValidationError(e.to_string()))?;
    let claims = get_current_user(&http_request)?;

    let batch: Batch = sqlx::query_as(
        "SELECT * FROM batches WHERE id = ? AND reagent_id = ? AND deleted_at IS NULL"
    )
        .bind(&batch_id)
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::batch_not_found(&batch_id))?;

    let pack_size = match batch.pack_size {
        Some(ps) if ps > 0.0 => ps,
        Some(_) => return Err(ApiError::BadRequest(
            "Invalid pack_size: must be greater than 0".to_string()
        )),
        None => return Err(ApiError::BadRequest(
            "Cannot dispense by units: pack_size is not set for this batch. \
             Use regular quantity-based dispensing (/use endpoint) instead.".to_string()
        )),
    };

    let quantity_to_dispense = request.units_to_dispense as f64 * pack_size;

    let mut tx = app_state.db_pool.begin().await?;
    let mut req = crate::consumption::ConsumeRequest::new(
        &batch_id, quantity_to_dispense, Some(&claims.sub),
    );
    req.purpose = request.purpose.as_deref();
    req.notes = request.notes.as_deref();
    req.allow_expired = request.allow_expired.unwrap_or(false);
    req.expired_reason = request.expired_reason.as_deref();
    let result = crate::consumption::consume_from_batch(&mut *tx, req).await?;
    tx.commit().await?;

    let context = format!("({} unit(s) of {} {})", request.units_to_dispense, pack_size, result.unit);
    crate::audit::audit_with_changes(
        &app_state.db_pool, &claims.sub, "use_reagent", "batch", &batch_id,
        &result.describe(Some(&context)), &result.audit_changes(), &http_request,
    ).await;

    log::info!("User {}: {}", claims.username, result.describe(Some(&context)));

    let response = DispenseUnitsResponse {
        usage_id: result.usage_id.clone(),
        units_dispensed: request.units_to_dispense,
        quantity_dispensed: quantity_to_dispense,
        unit: result.unit.clone(),
        remaining_quantity: result.batch_quantity_after,
        remaining_units: whole_units(result.batch_quantity_after, pack_size),
        status: result.status_after.clone(),
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        response,
        format!("Successfully dispensed {} unit(s)", request.units_to_dispense),
    )))
}

/// Whole packs in `quantity`, robust to float noise (0.3 / 0.1 = 2.999... -> 3)
fn whole_units(quantity: f64, pack_size: f64) -> i64 {
    ((quantity / pack_size) + 1e-9).floor().max(0.0) as i64
}

/// Получить информацию о доступных единицах в батче
/// GET /api/reagents/{reagent_id}/batches/{batch_id}/units-info
pub async fn get_batch_units_info(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
) -> ApiResult<HttpResponse> {
    let (reagent_id, batch_id) = path.into_inner();

    let batch: Batch = sqlx::query_as(
        "SELECT * FROM batches WHERE id = ? AND reagent_id = ? AND deleted_at IS NULL"
    )
        .bind(&batch_id)
        .bind(&reagent_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::batch_not_found(&batch_id))?;

    #[derive(Debug, Serialize)]
    struct UnitsInfo {
        batch_id: String,
        /// Общее количество в базовых единицах
        total_quantity: f64,
        /// Зарезервированное количество
        reserved_quantity: f64,
        /// Доступное количество (total - reserved)
        available_quantity: f64,
        /// Единица измерения
        unit: String,
        /// Размер одной упаковки/единицы
        pack_size: Option<f64>,
        /// Общее количество целых единиц
        total_units: Option<i64>,
        /// Доступное количество целых единиц для списания
        available_units: Option<i64>,
        /// Можно ли использовать штучное списание
        can_dispense_by_units: bool,
        /// Статус батча
        status: String,
    }

    let available_quantity = batch.quantity - batch.reserved_quantity;
    
    let (total_units, available_units, can_dispense) = match batch.pack_size {
        Some(ps) if ps > 0.0 => (
            Some(whole_units(batch.quantity, ps)),
            Some(whole_units(available_quantity, ps)),
            true,
        ),
        _ => (None, None, false),
    };

    let info = UnitsInfo {
        batch_id: batch.id,
        total_quantity: batch.quantity,
        reserved_quantity: batch.reserved_quantity,
        available_quantity,
        unit: batch.unit,
        pack_size: batch.pack_size,
        total_units,
        available_units,
        can_dispense_by_units: can_dispense,
        status: batch.status,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(info)))
}

// ==================== REQUALIFICATION (EXTEND SHELF LIFE) ====================

#[derive(Debug, serde::Deserialize, Validate)]
pub struct RequalifyBatchRequest {
    /// New expiry / retest date; must be in the future
    pub new_expiry_date: DateTime<Utc>,
    /// Basis for the extension, e.g. "assay by titration 99.2%, meets spec"
    #[validate(length(min = 1, max = 500, message = "Reason must be 1-500 characters"))]
    pub reason: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BatchRequalification {
    pub id: String,
    pub batch_id: String,
    pub old_expiry_date: Option<DateTime<Utc>>,
    pub new_expiry_date: DateTime<Utc>,
    pub old_status: String,
    pub new_status: String,
    pub reason: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// POST /api/v1/batches/{batch_id}/requalify
/// Extends the expiry (retest) date after the batch was checked and found fit
/// for use. Recomputes the status (expired -> available/low_stock) and keeps a
/// durable record of who extended it, from/to which date and why.
pub async fn requalify_batch(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<RequalifyBatchRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    request.validate()?;
    let claims = get_current_user(&http_request)?;
    let batch_id = path.into_inner();
    let reason = request.reason.trim().to_string();
    if reason.is_empty() {
        return Err(ApiError::bad_request("A reason is required to extend the expiry date"));
    }
    let now = Utc::now();
    if request.new_expiry_date <= now {
        return Err(ApiError::bad_request("New expiry date must be in the future"));
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        batch_number: String,
        reagent_name: String,
        quantity: f64,
        original_quantity: f64,
        status: String,
        expiry_date: Option<DateTime<Utc>>,
    }

    let mut tx = app_state.db_pool.begin().await?;

    // First statement is a write: take the lock before reading
    let touched = sqlx::query(
        "UPDATE batches SET updated_at = updated_at WHERE id = ? AND deleted_at IS NULL"
    )
        .bind(&batch_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if touched == 0 {
        return Err(ApiError::batch_not_found(&batch_id));
    }

    let b: Row = sqlx::query_as(
        "SELECT b.batch_number, r.name AS reagent_name, b.quantity, \
                COALESCE(b.original_quantity, b.quantity) AS original_quantity, b.status, b.expiry_date \
         FROM batches b JOIN reagents r ON r.id = b.reagent_id WHERE b.id = ?"
    )
        .bind(&batch_id)
        .fetch_one(&mut *tx)
        .await?;

    if b.status == "depleted" {
        return Err(ApiError::bad_request("Batch is depleted; nothing to requalify"));
    }

    let (total_containers, active_containers): (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(CASE WHEN status NOT IN ('empty','disposed') THEN 1 ELSE 0 END), 0) \
         FROM batch_containers WHERE batch_id = ?"
    )
        .bind(&batch_id)
        .fetch_one(&mut *tx)
        .await?;

    // Status as if never expired: same rules as every write-off
    let new_status = crate::consumption::compute_batch_status(
        b.quantity, b.original_quantity, total_containers > 0, active_containers, "available",
    );

    sqlx::query(
        "UPDATE batches SET expiry_date = ?, status = ?, updated_by = ?, updated_at = ? WHERE id = ?"
    )
        .bind(request.new_expiry_date)
        .bind(new_status)
        .bind(&claims.sub)
        .bind(now)
        .bind(&batch_id)
        .execute(&mut *tx)
        .await?;

    let record_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO batch_requalifications \
         (id, batch_id, old_expiry_date, new_expiry_date, old_status, new_status, reason, user_id, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
        .bind(&record_id)
        .bind(&batch_id)
        .bind(b.expiry_date)
        .bind(request.new_expiry_date)
        .bind(&b.status)
        .bind(new_status)
        .bind(&reason)
        .bind(&claims.sub)
        .bind(now)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let fmt = |d: Option<DateTime<Utc>>| d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "none".into());
    let mut cs = crate::audit::ChangeSet::new();
    cs.add("expiry_date", &fmt(b.expiry_date), &fmt(Some(request.new_expiry_date)));
    if b.status != new_status {
        cs.add("status", &b.status, new_status);
    }
    cs.created("requalification_reason", &reason);
    crate::audit::audit_with_changes(
        &app_state.db_pool, &claims.sub, "requalify", "batch", &batch_id,
        &format!(
            "Shelf life of reagent \"{}\" batch {} extended: {} -> {} (reason: {})",
            b.reagent_name, b.batch_number, fmt(b.expiry_date), fmt(Some(request.new_expiry_date)), reason
        ),
        &cs, &http_request,
    ).await;

    log::info!(
        "User {} requalified batch {} ({}): expiry {} -> {}, status {} -> {}",
        claims.username, b.batch_number, batch_id, fmt(b.expiry_date),
        fmt(Some(request.new_expiry_date)), b.status, new_status
    );

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        serde_json::json!({
            "id": record_id,
            "batch_id": batch_id,
            "old_expiry_date": b.expiry_date,
            "new_expiry_date": request.new_expiry_date,
            "old_status": b.status,
            "new_status": new_status,
            "reason": reason,
        }),
        "Expiry date extended".to_string(),
    )))
}

/// GET /api/v1/batches/{batch_id}/requalifications — newest first
pub async fn get_batch_requalifications(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let batch_id = path.into_inner();
    let rows: Vec<BatchRequalification> = sqlx::query_as(
        "SELECT q.id, q.batch_id, q.old_expiry_date, q.new_expiry_date, q.old_status, q.new_status, \
                q.reason, q.user_id, u.username, q.created_at \
         FROM batch_requalifications q LEFT JOIN users u ON u.id = q.user_id \
         WHERE q.batch_id = ? ORDER BY q.created_at DESC"
    )
        .bind(&batch_id)
        .fetch_all(&app_state.db_pool)
        .await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(rows)))
}
