//! Обработчики для модуля оборудования
//!
//! Включает:
//! - CRUD операции для оборудования
//! - Управление запасными частями
//! - Планирование и учет обслуживания
//! - Загрузка и хранение файлов (мануалы, изображения)
//! - FTS5 полнотекстовый поиск

use actix_web::{web, HttpResponse};
use actix_multipart::Multipart;
use futures_util::StreamExt;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::io::Write;
use std::str::FromStr;
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;

use crate::AppState;
use crate::models::{
    Equipment, CreateEquipmentRequest, UpdateEquipmentRequest,
    EquipmentPart, CreateEquipmentPartRequest, UpdateEquipmentPartRequest,
    EquipmentMaintenance,
    CreateMaintenanceRequest, UpdateMaintenanceRequest, CompleteMaintenanceRequest,
    EquipmentFile, EquipmentDetailResponse
};
use crate::error::{ApiError, ApiResult};
use crate::handlers::{ApiResponse, PaginatedResponse};
use crate::query_builders::{
    SafeQueryBuilder, CountQueryBuilder, FieldWhitelist,
    EquipmentType, MaintenanceType, MaintenanceStatus,
    validate_file_size, validate_mime_type,
};
use crate::equipment_files;

// ==================== КОНСТАНТЫ ====================

// ==================== СТРУКТУРЫ ЗАПРОСОВ ====================

/// Специфичная структура пагинации для оборудования
#[derive(Debug, serde::Deserialize)]
pub struct EquipmentPaginationQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub location: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

impl EquipmentPaginationQuery {
    pub fn normalize(&self) -> (i64, i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * per_page;
        (page, per_page, offset)
    }
}

/// Структура для поискового запроса
#[derive(Debug, serde::Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
}

// ==================== КОНСТАНТЫ (продолжение) ====================

/// Максимальный размер файла (10 МБ)
const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Разрешенные MIME типы для изображений
const ALLOWED_IMAGE_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

/// Разрешенные MIME типы для документов
const ALLOWED_DOC_TYPES: &[&str] = &[
    "application/pdf",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "text/plain",
];


// ==================== ОСНОВНЫЕ CRUD ОПЕРАЦИИ ====================

/// Получение списка оборудования с пагинацией и фильтрами
pub async fn get_equipment(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<EquipmentPaginationQuery>,
) -> ApiResult<HttpResponse> {
    let (page, per_page, offset) = query.normalize();
    let whitelist = FieldWhitelist::for_equipment();

// Подсчет общего количества
    let mut count_builder = CountQueryBuilder::new("equipment")
        .map_err(|e| ApiError::InternalServerError(e))?;
    count_builder.add_condition("deleted_at IS NULL", vec![]);
    apply_equipment_filters(&mut count_builder, &query, &whitelist)?;

    let (count_sql, count_params) = count_builder.build();
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for param in &count_params {
        count_query = count_query.bind(param);
    }
    let total: i64 = count_query.fetch_one(&app_state.db_pool).await?;

    // Выборка данных
    let base_sql = "SELECT * FROM equipment";
    let mut select_builder = SafeQueryBuilder::new(base_sql)
        .map_err(|e| ApiError::InternalServerError(e))?
        .with_whitelist(&whitelist);
    select_builder.add_condition("deleted_at IS NULL", vec![]);
    apply_equipment_filters_safe(&mut select_builder, &query)?;
    
    // ИСПРАВЛЕНО: Теперь используем параметры из запроса, а не хардкод
    let sort_field = query.sort_by.as_deref().unwrap_or("created_at");
    let sort_order = query.sort_order.as_deref().unwrap_or("desc");
    select_builder.order_by(sort_field, sort_order);

    // В вашем query_builders/mod.rs limit принимает i64, приведение к u32 не нужно
    select_builder.limit(per_page);
    select_builder.offset(offset);

    let (select_sql, select_params) = select_builder.build();
    let mut select_query = sqlx::query_as::<_, Equipment>(&select_sql);
    for param in &select_params {
        select_query = select_query.bind(param);
    }
    let equipment = select_query.fetch_all(&app_state.db_pool).await?;

    let total_pages = (total + per_page - 1) / per_page;

    Ok(HttpResponse::Ok().json(ApiResponse::success(PaginatedResponse {
        data: equipment,
        total,
        page,
        per_page,
        total_pages,
    })))
}

/// Получение оборудования по ID с деталями (части, обслуживание, файлы)
pub async fn get_equipment_by_id(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let equipment_id = path.into_inner();

    let equipment: Option<Equipment> = sqlx::query_as(
        "SELECT * FROM equipment WHERE id = ? AND deleted_at IS NULL"
    )
        .bind(&equipment_id)
        .fetch_optional(&app_state.db_pool)
        .await?;

    match equipment {
        Some(e) => {
            // Загружаем связанные данные
            let parts = get_equipment_parts_internal(&app_state.db_pool, &equipment_id).await?;
            let maintenance = get_recent_maintenance_internal(&app_state.db_pool, &equipment_id, 5).await?;
            let files = get_equipment_files_internal(&app_state.db_pool, &equipment_id).await?;

            let response = EquipmentDetailResponse {
                equipment: e,
                parts,
                recent_maintenance: maintenance,
                files,
            };

            Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
        },
        None => Err(ApiError::not_found("Equipment")),
    }
}

/// Создание нового оборудования
pub async fn create_equipment(
    app_state: web::Data<Arc<AppState>>,
    equipment: web::Json<CreateEquipmentRequest>,
    _user_id: String,
) -> ApiResult<HttpResponse> {
    equipment.validate()?;
    validate_equipment_data(&equipment)?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO equipment
           (id, name, type_, quantity, unit, status, location, description, 
            serial_number, manufacturer, model, purchase_date, warranty_until,
            maintenance_interval_days,
            created_by, updated_by, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, 'available', ?, ?, ?, ?, ?, ?, ?, COALESCE(?, 90), ?, ?, ?, ?)"#
    )
        .bind(&id)
        .bind(&equipment.name)
        .bind(&equipment.type_)
        .bind(equipment.quantity)
        .bind(&equipment.unit)
        .bind(&equipment.location)
        .bind(&equipment.description)
        .bind(&equipment.serial_number)
        .bind(&equipment.manufacturer)
        .bind(&equipment.model)
        .bind(&equipment.purchase_date)
        .bind(&equipment.warranty_until)
        .bind(equipment.maintenance_interval_days)
        .bind(&_user_id)
        .bind(&_user_id)
        .bind(&now)
        .bind(&now)
        .execute(&app_state.db_pool)
        .await?;

    let created: Equipment = sqlx::query_as("SELECT * FROM equipment WHERE id = ? AND deleted_at IS NULL")
        .bind(&id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(created)))
}

/// Обновление оборудования
pub async fn update_equipment(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    update: web::Json<UpdateEquipmentRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    update.validate()?;
    let equipment_id = path.into_inner();

    // Проверяем существование
    let existing: Option<Equipment> = sqlx::query_as(
        "SELECT * FROM equipment WHERE id = ? AND deleted_at IS NULL"
    )
        .bind(&equipment_id)
        .fetch_optional(&app_state.db_pool)
        .await?;

    let existing = existing.ok_or_else(|| ApiError::not_found("Equipment"))?;

    // Строим динамический UPDATE
    let mut updates = Vec::new();
    let mut values: Vec<String> = Vec::new();

    macro_rules! add_field {
        ($field:ident, $name:expr) => {
            if let Some(ref val) = update.$field {
                updates.push(concat!($name, " = ?"));
                values.push(val.clone());
            }
        };
    }

    if let Some(ref t) = update.type_ {
        if EquipmentType::from_str(t).is_err() {
            return Err(ApiError::bad_request(&format!(
                "Invalid type: {}. Valid: instrument, glassware, safety, storage, consumable, other", t
            )));
        }
    }

    add_field!(name, "name");
    add_field!(type_, "type_");
    add_field!(purchase_date, "purchase_date");
    add_field!(warranty_until, "warranty_until");
    add_field!(unit, "unit");
    add_field!(location, "location");
    add_field!(description, "description");
    add_field!(status, "status");
    add_field!(serial_number, "serial_number");
    add_field!(manufacturer, "manufacturer");
    add_field!(model, "model");

    if let Some(quantity) = update.quantity {
        updates.push("quantity = ?");
        values.push(quantity.to_string());
    }
    if let Some(days) = update.maintenance_interval_days {
        updates.push("maintenance_interval_days = ?");
        values.push(days.to_string());
    }

    if updates.is_empty() {
        return Err(ApiError::bad_request("No fields to update"));
    }

    updates.push("updated_by = ?");
    updates.push("updated_at = ?");
    values.push(user_id);
    values.push(Utc::now().to_rfc3339());

    let sql = format!("UPDATE equipment SET {} WHERE id = ?", updates.join(", "));

    let mut query = sqlx::query(&sql);
    for value in &values {
        query = query.bind(value);
    }
    query = query.bind(&equipment_id);

    query.execute(&app_state.db_pool).await?;

    if update.maintenance_interval_days.is_some() {
        refresh_maintenance_dates(&app_state.db_pool, &equipment_id).await?;
    }

    // Renamed: its folder (named after it) moves with it
    if update.name.as_deref().map_or(false, |n| n != existing.name) {
        if let Err(e) = equipment_files::relocate_equipment(&app_state.db_pool, &equipment_id).await {
            log::warn!("Equipment {} renamed, files not moved: {}", equipment_id, e);
        }
    }

    let updated: Equipment = sqlx::query_as("SELECT * FROM equipment WHERE id = ? AND deleted_at IS NULL")
        .bind(&equipment_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(updated)))
}

/// Удаление оборудования
pub async fn delete_equipment(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    user_id: String,  // ← ДОБАВИТЬ: пробросить из JWT
) -> ApiResult<HttpResponse> {
    let equipment_id = path.into_inner();

    // Soft delete: FTS hard + equipment soft (parts/maintenance/files сохраняются)
    crate::soft_delete::delete_equipment(&app_state.db_pool, &equipment_id, &user_id).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        (),
        "Equipment deleted successfully".to_string(),
    )))
}

// ==================== ЗАПАСНЫЕ ЧАСТИ ====================

/// Получение списка частей оборудования
pub async fn get_equipment_parts(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let equipment_id = path.into_inner();

    // Проверяем существование оборудования
    check_equipment_exists(&app_state.db_pool, &equipment_id).await?;

    let parts = get_equipment_parts_internal(&app_state.db_pool, &equipment_id).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(parts)))
}

/// Добавление части к оборудованию
pub async fn add_equipment_part(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    part: web::Json<CreateEquipmentPartRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    part.validate()?;
    let equipment_id = path.into_inner();

    check_equipment_exists(&app_state.db_pool, &equipment_id).await?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let status = part.status.as_deref().unwrap_or("good");  // FIXED: "good" matches DB default

    // Validate part status against DB constraint
    let valid_statuses = ["good", "needs_attention", "needs_replacement", "replaced", "missing"];
    if !valid_statuses.contains(&status) {
        return Err(ApiError::bad_request(&format!(
            "Invalid part status: {}. Valid: good, needs_attention, needs_replacement, replaced, missing",
            status
        )));
    }

    sqlx::query(
        r#"INSERT INTO equipment_parts
           (id, equipment_id, name, part_number, manufacturer, quantity, 
            min_quantity, status, last_replaced, next_replacement, notes,
            created_by, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
        .bind(&id)
        .bind(&equipment_id)
        .bind(&part.name)
        .bind(&part.part_number)
        .bind(&part.manufacturer)
        .bind(part.quantity.unwrap_or(1))
        .bind(part.min_quantity.unwrap_or(0))
        .bind(status)
        .bind(&part.last_replaced)
        .bind(&part.next_replacement)
        .bind(&part.notes)
        .bind(&user_id)
        .bind(&now)
        .bind(&now)
        .execute(&app_state.db_pool)
        .await?;

    let created: EquipmentPart = sqlx::query_as(
        "SELECT * FROM equipment_parts WHERE id = ?"
    )
        .bind(&id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(created)))
}

/// Обновление части оборудования
pub async fn update_equipment_part(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    update: web::Json<UpdateEquipmentPartRequest>,
    _user_id: String,
) -> ApiResult<HttpResponse> {
    update.validate()?;
    let (equipment_id, part_id) = path.into_inner();

    check_equipment_exists(&app_state.db_pool, &equipment_id).await?;

    // Проверяем существование части
    let existing: Option<EquipmentPart> = sqlx::query_as(
        "SELECT * FROM equipment_parts WHERE id = ? AND equipment_id = ?"
    )
        .bind(&part_id)
        .bind(&equipment_id)
        .fetch_optional(&app_state.db_pool)
        .await?;

    let existing = existing.ok_or_else(|| ApiError::not_found("Equipment part"))?;

    let mut updates = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(ref name) = update.name {
        updates.push("name = ?");
        values.push(name.clone());
    }
    if let Some(ref part_number) = update.part_number {
        updates.push("part_number = ?");
        values.push(part_number.clone());
    }
    if let Some(ref manufacturer) = update.manufacturer {
        updates.push("manufacturer = ?");
        values.push(manufacturer.clone());
    }
    if let Some(quantity) = update.quantity {
        updates.push("quantity = ?");
        values.push(quantity.to_string());
    }
    if let Some(min_quantity) = update.min_quantity {
        updates.push("min_quantity = ?");
        values.push(min_quantity.to_string());
    }
    if let Some(ref status) = update.status {
        // Validate part status against DB constraint
        let valid_statuses = ["good", "needs_attention", "needs_replacement", "replaced", "missing"];
        if !valid_statuses.contains(&status.as_str()) {
            return Err(ApiError::bad_request(&format!(
                "Invalid part status: {}. Valid: good, needs_attention, needs_replacement, replaced, missing",
                status
            )));
        }
        updates.push("status = ?");
        values.push(status.clone());
    }
    if let Some(ref notes) = update.notes {
        updates.push("notes = ?");
        values.push(notes.clone());
    }

    if updates.is_empty() {
        return Err(ApiError::bad_request("No fields to update"));
    }

    updates.push("updated_at = ?");
    values.push(Utc::now().to_rfc3339());

    let sql = format!("UPDATE equipment_parts SET {} WHERE id = ?", updates.join(", "));

    let mut query = sqlx::query(&sql);
    for value in &values {
        query = query.bind(value);
    }
    query = query.bind(&part_id);
    query.execute(&app_state.db_pool).await?;

    if update.name.as_deref().map_or(false, |n| n != existing.name) {
        if let Err(e) = equipment_files::relocate_equipment(&app_state.db_pool, &equipment_id).await {
            log::warn!("Part {} renamed, files not moved: {}", part_id, e);
        }
    }

    let updated: EquipmentPart = sqlx::query_as(
        "SELECT * FROM equipment_parts WHERE id = ?"
    )
        .bind(&part_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(updated)))
}

/// Удаление части оборудования
pub async fn delete_equipment_part(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
) -> ApiResult<HttpResponse> {
    let (equipment_id, part_id) = path.into_inner();

    check_equipment_exists(&app_state.db_pool, &equipment_id).await?;

    // The part's photos and documents go with it. Left behind, the foreign
    // key (ON DELETE SET NULL) made them equipment photos.
    if equipment_files::delete_part(&app_state.db_pool, &equipment_id, &part_id).await? == 0 {
        return Err(ApiError::not_found("Equipment part"));
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        (),
        "Part deleted successfully".to_string(),
    )))
}

// ==================== ОБСЛУЖИВАНИЕ ====================

/// A day as the date inputs send it (YYYY-MM-DD); an ISO timestamp keeps its day
fn parse_day(value: &str, field: &str) -> ApiResult<String> {
    let v = value.trim();
    let day = v.get(..10).unwrap_or(v);
    chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d")
        .map(|d| d.format("%Y-%m-%d").to_string())
        .map_err(|_| ApiError::bad_request(&format!("{} must be a date (YYYY-MM-DD), got '{}'", field, value)))
}

fn check_maintenance_type(t: &str) -> ApiResult<()> {
    MaintenanceType::from_str(t).map(|_| ()).map_err(|_| ApiError::bad_request(&format!(
        "Invalid maintenance type: {}. Valid: calibration, repair, inspection, cleaning, replacement, other", t
    )))
}

fn check_maintenance_status(s: &str) -> ApiResult<()> {
    MaintenanceStatus::from_str(s).map(|_| ()).map_err(|_| ApiError::bad_request(&format!(
        "Invalid status: {}. Valid: scheduled, in_progress, completed, cancelled", s
    )))
}

fn today() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

/// Keep `last_maintenance` / `next_maintenance` of the equipment in step with
/// its records: last = latest completed; next = nearest planned (scheduled or
/// in progress), else last + interval. Equipment with no records at all keeps
/// an imported last date.
pub async fn refresh_maintenance_dates(pool: &SqlitePool, equipment_id: &str) -> ApiResult<()> {
    let (records, last_done, planned): (i64, Option<String>, Option<String>) = sqlx::query_as(
        r#"SELECT COUNT(*),
                  MAX(CASE WHEN status = 'completed' THEN substr(completed_date, 1, 10) END),
                  MIN(CASE WHEN status IN ('scheduled', 'in_progress') THEN substr(scheduled_date, 1, 10) END)
           FROM equipment_maintenance WHERE equipment_id = ?"#
    ).bind(equipment_id).fetch_one(pool).await?;
    let Some((current_last, interval)) = sqlx::query_as::<_, (Option<String>, Option<i32>)>(
        "SELECT last_maintenance, maintenance_interval_days FROM equipment WHERE id = ?"
    ).bind(equipment_id).fetch_optional(pool).await? else { return Ok(()) };

    let last = if records == 0 { current_last.map(|d| d.chars().take(10).collect::<String>()) } else { last_done };
    let next = planned.or_else(|| {
        let days = interval.unwrap_or(0);
        let last = last.as_deref()?;
        if days <= 0 { return None; }
        chrono::NaiveDate::parse_from_str(last, "%Y-%m-%d").ok()
            .map(|d| (d + chrono::Duration::days(days as i64)).format("%Y-%m-%d").to_string())
    });
    sqlx::query("UPDATE equipment SET last_maintenance = ?, next_maintenance = ? WHERE id = ?")
        .bind(&last).bind(&next).bind(equipment_id)
        .execute(pool).await?;
    Ok(())
}

/// Получение списка обслуживания оборудования
pub async fn get_equipment_maintenance(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let equipment_id = path.into_inner();

    check_equipment_exists(&app_state.db_pool, &equipment_id).await?;

    let maintenance: Vec<EquipmentMaintenance> = sqlx::query_as(
        r#"SELECT * FROM equipment_maintenance 
           WHERE equipment_id = ? 
           ORDER BY scheduled_date DESC"#
    )
        .bind(&equipment_id)
        .fetch_all(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(maintenance)))
}

/// Создание записи об обслуживании
pub async fn create_maintenance(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    maintenance: web::Json<CreateMaintenanceRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    maintenance.validate()?;
    let equipment_id = path.into_inner();

    check_equipment_exists(&app_state.db_pool, &equipment_id).await?;

    check_maintenance_type(&maintenance.maintenance_type)?;
    let scheduled_date = parse_day(&maintenance.scheduled_date, "Date")?;
    let mut completed_date = match maintenance.completed_date.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(d) => Some(parse_day(d, "Completed date")?),
        None => None,
    };
    // A record with a completion date is a completed one (work done earlier
    // than planned is fine: no order between the two dates is required)
    let status = match maintenance.status.as_deref() {
        Some(s) => { check_maintenance_status(s)?; s }
        None if completed_date.is_some() => "completed",
        None => "scheduled",
    };
    if status == "completed" && completed_date.is_none() {
        completed_date = Some(today());
    }
    if status != "completed" {
        completed_date = None;
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO equipment_maintenance
           (id, equipment_id, maintenance_type, status, scheduled_date, completed_date,
            performed_by, description, cost, parts_replaced, notes,
            created_by, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
        .bind(&id)
        .bind(&equipment_id)
        .bind(&maintenance.maintenance_type)
        .bind(status)
        .bind(&scheduled_date)
        .bind(&completed_date)
        .bind(&maintenance.performed_by)
        .bind(&maintenance.description)
        .bind(maintenance.cost)
        .bind(&maintenance.parts_replaced)
        .bind(&maintenance.notes)
        .bind(&user_id)
        .bind(&now)
        .bind(&now)
        .execute(&app_state.db_pool)
        .await?;
    refresh_maintenance_dates(&app_state.db_pool, &equipment_id).await?;

    let created: EquipmentMaintenance = sqlx::query_as(
        "SELECT * FROM equipment_maintenance WHERE id = ?"
    )
        .bind(&id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(created)))
}

/// Обновление записи об обслуживании
pub async fn update_maintenance(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    update: web::Json<UpdateMaintenanceRequest>,
    _user_id: String,
) -> ApiResult<HttpResponse> {
    update.validate()?;
    let (equipment_id, maintenance_id) = path.into_inner();

    check_equipment_exists(&app_state.db_pool, &equipment_id).await?;

    let existing: Option<EquipmentMaintenance> = sqlx::query_as(
        "SELECT * FROM equipment_maintenance WHERE id = ? AND equipment_id = ?"
    )
        .bind(&maintenance_id)
        .bind(&equipment_id)
        .fetch_optional(&app_state.db_pool)
        .await?;

    let existing = existing.ok_or_else(|| ApiError::not_found("Maintenance record"))?;

    let mut updates = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(ref t) = update.maintenance_type {
        check_maintenance_type(t)?;
        updates.push("maintenance_type = ?");
        values.push(t.clone());
    }
    if let Some(ref d) = update.scheduled_date {
        updates.push("scheduled_date = ?");
        values.push(parse_day(d, "Date")?);
    }
    let completed_given = match update.completed_date.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(d) => Some(parse_day(d, "Completed date")?),
        None => None,
    };
    if let Some(ref status) = update.status {
        check_maintenance_status(status)?;
        updates.push("status = ?");
        values.push(status.clone());
        if status == "completed" {
            if completed_given.is_none() && existing.completed_date.is_none() {
                updates.push("completed_date = ?");
                values.push(today());
            }
        } else if completed_given.is_none() {
            // back to planned / cancelled: no completion date
            updates.push("completed_date = NULL");
        }
    }
    if let Some(d) = completed_given {
        updates.push("completed_date = ?");
        values.push(d);
    }
    if let Some(ref performed_by) = update.performed_by {
        updates.push("performed_by = ?");
        values.push(performed_by.clone());
    }
    if let Some(ref description) = update.description {
        updates.push("description = ?");
        values.push(description.clone());
    }
    if let Some(cost) = update.cost {
        updates.push("cost = ?");
        values.push(cost.to_string());
    }
    if let Some(ref parts_replaced) = update.parts_replaced {
        updates.push("parts_replaced = ?");
        values.push(parts_replaced.clone());
    }
    if let Some(ref notes) = update.notes {
        updates.push("notes = ?");
        values.push(notes.clone());
    }

    if updates.is_empty() {
        return Err(ApiError::bad_request("No fields to update"));
    }

    updates.push("updated_at = ?");
    values.push(Utc::now().to_rfc3339());

    let sql = format!(
        "UPDATE equipment_maintenance SET {} WHERE id = ?",
        updates.join(", ")
    );

    let mut query = sqlx::query(&sql);
    for value in &values {
        query = query.bind(value);
    }
    query = query.bind(&maintenance_id);
    query.execute(&app_state.db_pool).await?;
    refresh_maintenance_dates(&app_state.db_pool, &equipment_id).await?;

    let updated: EquipmentMaintenance = sqlx::query_as(
        "SELECT * FROM equipment_maintenance WHERE id = ?"
    )
        .bind(&maintenance_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(updated)))
}

/// Завершение обслуживания
pub async fn complete_maintenance(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    body: web::Json<CompleteMaintenanceRequest>,
    _user_id: String,
) -> ApiResult<HttpResponse> {
    let (equipment_id, maintenance_id) = path.into_inner();

    check_equipment_exists(&app_state.db_pool, &equipment_id).await?;

    let existing: Option<EquipmentMaintenance> = sqlx::query_as(
        "SELECT * FROM equipment_maintenance WHERE id = ? AND equipment_id = ?"
    )
        .bind(&maintenance_id)
        .bind(&equipment_id)
        .fetch_optional(&app_state.db_pool)
        .await?;

    if existing.is_none() {
        return Err(ApiError::not_found("Maintenance record"));
    }

    let completed_date = match body.completed_date.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(d) => parse_day(d, "Completed date")?,
        None => today(),
    };
    let performed_by = body.performed_by.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if performed_by.map_or(false, |s| s.chars().count() > 255) {
        return Err(ApiError::bad_request("Performed by cannot exceed 255 characters"));
    }
    let notes = body.notes.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if notes.map_or(false, |s| s.chars().count() > 1000) {
        return Err(ApiError::bad_request("Notes cannot exceed 1000 characters"));
    }

    sqlx::query(
        r#"UPDATE equipment_maintenance 
           SET status = 'completed', completed_date = ?, performed_by = COALESCE(?, performed_by), 
               notes = COALESCE(?, notes), updated_at = ?
           WHERE id = ?"#
    )
        .bind(&completed_date)
        .bind(performed_by)
        .bind(notes)
        .bind(Utc::now())
        .bind(&maintenance_id)
        .execute(&app_state.db_pool)
        .await?;
    refresh_maintenance_dates(&app_state.db_pool, &equipment_id).await?;

    let updated: EquipmentMaintenance = sqlx::query_as(
        "SELECT * FROM equipment_maintenance WHERE id = ?"
    )
        .bind(&maintenance_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(updated)))
}

/// Удаление записи об обслуживании
pub async fn delete_maintenance(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
) -> ApiResult<HttpResponse> {
    let (equipment_id, maintenance_id) = path.into_inner();

    check_equipment_exists(&app_state.db_pool, &equipment_id).await?;

    let result = sqlx::query(
        "DELETE FROM equipment_maintenance WHERE id = ? AND equipment_id = ?"
    )
        .bind(&maintenance_id)
        .bind(&equipment_id)
        .execute(&app_state.db_pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Maintenance record"));
    }
    refresh_maintenance_dates(&app_state.db_pool, &equipment_id).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        (),
        "Maintenance record deleted successfully".to_string(),
    )))
}

// ==================== ФАЙЛЫ ====================

/// Получение файлов оборудования
pub async fn get_equipment_files(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let equipment_id = path.into_inner();

    check_equipment_exists(&app_state.db_pool, &equipment_id).await?;

    let files = get_equipment_files_internal(&app_state.db_pool, &equipment_id).await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(files)))
}

/// Загрузка файла для оборудования с древовидной структурой папок
/// Структура: 
///   uploads/equipment/{equipment_name}/images/       - фото оборудования
///   uploads/equipment/{equipment_name}/manuals/      - мануалы
///   uploads/equipment/{equipment_name}/parts/{part_name}/images/ - фото запчасти
pub async fn upload_equipment_file(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    mut payload: Multipart,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let equipment_id = path.into_inner();

    // Получаем информацию об оборудовании
    let equipment: Equipment = sqlx::query_as(
        "SELECT * FROM equipment WHERE id = ? AND deleted_at IS NULL"
    )
        .bind(&equipment_id)
        .fetch_optional(&app_state.db_pool)
        .await?
        .ok_or_else(|| ApiError::not_found("Equipment"))?;

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut original_filename: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut form_file_type: Option<String> = None;
    let mut form_description: Option<String> = None;
    let mut form_part_id: Option<String> = None;

    // Читаем все поля формы
    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|e| ApiError::bad_request(&format!("Multipart error: {}", e)))?;

        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name().unwrap_or("");

        match field_name {
            "file" => {
                let filename = content_disposition
                    .get_filename()
                    .ok_or_else(|| ApiError::bad_request("Filename not provided"))?
                    .to_string();

                let mime = field.content_type()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| "application/octet-stream".to_string());

                let all_allowed: Vec<&str> = ALLOWED_IMAGE_TYPES.iter()
                    .chain(ALLOWED_DOC_TYPES.iter())
                    .copied()
                    .collect();

                validate_mime_type(&mime, &all_allowed)?;

                let mut bytes = Vec::new();
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| ApiError::bad_request(&format!("Read error: {}", e)))?;
                    bytes.extend_from_slice(&chunk);
                    validate_file_size(bytes.len(), MAX_FILE_SIZE)?;
                }

                file_bytes = Some(bytes);
                original_filename = Some(filename);
                content_type = Some(mime);
            }
            "file_type" => {
                let mut bytes = Vec::new();
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| ApiError::bad_request(&format!("Read error: {}", e)))?;
                    bytes.extend_from_slice(&chunk);
                }
                if let Ok(value) = String::from_utf8(bytes) {
                    let value = value.trim().to_string();
                    // the values the equipment_files CHECK constraint accepts
                    let valid_types = ["manual", "photo", "certificate", "other"];
                    if valid_types.contains(&value.as_str()) {
                        form_file_type = Some(value);
                    }
                }
            }
            "description" => {
                let mut bytes = Vec::new();
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| ApiError::bad_request(&format!("Read error: {}", e)))?;
                    bytes.extend_from_slice(&chunk);
                }
                if let Ok(value) = String::from_utf8(bytes) {
                    let value = value.trim().to_string();
                    if !value.is_empty() {
                        form_description = Some(value);
                    }
                }
            }
            "part_id" => {
                let mut bytes = Vec::new();
                while let Some(chunk) = field.next().await {
                    let chunk = chunk.map_err(|e| ApiError::bad_request(&format!("Read error: {}", e)))?;
                    bytes.extend_from_slice(&chunk);
                }
                if let Ok(value) = String::from_utf8(bytes) {
                    let value = value.trim().to_string();
                    if !value.is_empty() {
                        form_part_id = Some(value);
                    }
                }
            }
            _ => {}
        }
    }

    let file_bytes = file_bytes.ok_or_else(|| ApiError::bad_request("No file provided"))?;
    let original_filename = original_filename.ok_or_else(|| ApiError::bad_request("No filename"))?;
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    let file_type = form_file_type.unwrap_or_else(|| {
        if ALLOWED_IMAGE_TYPES.contains(&content_type.as_str()) {
            "photo".to_string()  // DB constraint: 'manual', 'certificate', 'photo', 'other'
        } else {
            "other".to_string()
        }
    });

    // equipment/<equipment>/{photos,documents}/, parts/<part>/{photos,documents}/
    let part = match form_part_id {
        Some(ref part_id) => {
            let part: EquipmentPart = sqlx::query_as(
                "SELECT * FROM equipment_parts WHERE id = ? AND equipment_id = ?"
            )
                .bind(part_id)
                .bind(&equipment_id)
                .fetch_optional(&app_state.db_pool)
                .await?
                .ok_or_else(|| ApiError::not_found("Part"))?;
            Some(part)
        }
        None => None,
    };
    let type_dir = equipment_files::file_dir(
        &equipment_files::files_root(),
        &equipment.name,
        &equipment.id,
        part.as_ref().map(|p| (p.name.as_str(), p.id.as_str())),
        &file_type,
    );
    std::fs::create_dir_all(&type_dir)
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create directory: {}", e)))?;
    let file_path = type_dir
        .join(equipment_files::stored_file_name(&original_filename))
        .to_string_lossy()
        .to_string();

    // Извлекаем stored_filename из полного пути
    let stored_filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&original_filename)
        .to_string();

    // Сохраняем файл
    let mut f = std::fs::File::create(&file_path)
        .map_err(|e| ApiError::InternalServerError(format!("Failed to create file: {}", e)))?;
    f.write_all(&file_bytes)
        .map_err(|e| ApiError::InternalServerError(format!("Failed to write file: {}", e)))?;

    // Сохраняем в БД
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let inserted = sqlx::query(
        r#"INSERT INTO equipment_files
           (id, equipment_id, part_id, file_type, original_filename, stored_filename,
            file_path, file_size, mime_type, description, uploaded_by, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
        .bind(&id)
        .bind(&equipment_id)
        .bind(&form_part_id)
        .bind(&file_type)
        .bind(&original_filename)
        .bind(&stored_filename)
        .bind(&file_path)
        .bind(file_bytes.len() as i64)
        .bind(&content_type)
        .bind(&form_description)
        .bind(&user_id)
        .bind(&now)
        .execute(&app_state.db_pool)
        .await;
    if let Err(e) = inserted {
        equipment_files::remove_stored_file(&file_path); // no record, no file
        return Err(e.into());
    }

    let created: EquipmentFile = sqlx::query_as(
        "SELECT * FROM equipment_files WHERE id = ?"
    )
        .bind(&id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(created)))
}

/// Скачивание файла оборудования
/// `?download=1` saves the file instead of showing it
#[derive(Debug, serde::Deserialize)]
pub struct FileViewQuery {
    pub download: Option<String>,
}

/// Content-Disposition for a stored file: PDFs, images and plain text open in
/// the browser (inline), other documents download. The name is sent as UTF-8
/// (`filename*`), with an ASCII `filename` for old clients, so Cyrillic names
/// arrive intact and a quote in a name cannot break the header.
fn file_disposition(original_filename: &str, mime: &str, force_download: bool) -> actix_web::http::header::ContentDisposition {
    use actix_web::http::header::{Charset, ContentDisposition, DispositionParam, DispositionType, ExtendedValue};
    let viewable = mime.starts_with("image/") || mime == "application/pdf" || mime == "text/plain";
    let ascii: String = original_filename.chars()
        .map(|c| if (c.is_ascii_graphic() || c == ' ') && c != '"' && c != '\\' { c } else { '_' })
        .collect();
    ContentDisposition {
        disposition: if viewable && !force_download { DispositionType::Inline } else { DispositionType::Attachment },
        parameters: vec![
            DispositionParam::Filename(ascii),
            DispositionParam::FilenameExt(ExtendedValue {
                charset: Charset::Ext("UTF-8".to_string()),
                language_tag: None,
                value: original_filename.as_bytes().to_vec(),
            }),
        ],
    }
}

pub async fn download_equipment_file(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    query: web::Query<FileViewQuery>,
) -> ApiResult<HttpResponse> {
    let (equipment_id, file_id) = path.into_inner();

    let file: Option<EquipmentFile> = sqlx::query_as(
        "SELECT * FROM equipment_files WHERE id = ? AND equipment_id = ?"
    )
        .bind(&file_id)
        .bind(&equipment_id)
        .fetch_optional(&app_state.db_pool)
        .await?;

    let file = file.ok_or_else(|| ApiError::not_found("File"))?;

    // Читаем файл
    let contents = std::fs::read(&file.file_path)
        .map_err(|e| ApiError::InternalServerError(format!("Failed to read file: {}", e)))?;

    let force_download = query.download.as_deref().map_or(false, |v| v != "0" && v != "false");
    let disposition = file_disposition(&file.original_filename, &file.mime_type, force_download);

    Ok(HttpResponse::Ok()
        .content_type(file.mime_type)
        .insert_header(disposition)
        // private: lab documents are not for shared caches
        .insert_header(("Cache-Control", "private, max-age=3600"))
        .body(contents))
}

/// Удаление файла оборудования
pub async fn delete_equipment_file(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
) -> ApiResult<HttpResponse> {
    let (equipment_id, file_id) = path.into_inner();

    // Получаем информацию о файле
    let file: Option<EquipmentFile> = sqlx::query_as(
        "SELECT * FROM equipment_files WHERE id = ? AND equipment_id = ?"
    )
        .bind(&file_id)
        .bind(&equipment_id)
        .fetch_optional(&app_state.db_pool)
        .await?;

    let file = file.ok_or_else(|| ApiError::not_found("File"))?;

    // Record first, then the file (and folders it leaves empty)
    sqlx::query("DELETE FROM equipment_files WHERE id = ?")
        .bind(&file_id)
        .execute(&app_state.db_pool)
        .await?;
    equipment_files::remove_stored_file(&file.file_path);

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        (),
        "File deleted successfully".to_string(),
    )))
}

// ==================== ПОИСК ====================

/// Полнотекстовый поиск по оборудованию
pub async fn search_equipment(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<SearchQuery>,
) -> ApiResult<HttpResponse> {
    let search_term = query.q.as_deref().unwrap_or("").trim();

    if search_term.is_empty() {
        return Err(ApiError::bad_request("Search query cannot be empty"));
    }

    let limit = query.limit.unwrap_or(20).clamp(1, 100); // LIMIT -1 is "no limit" in SQLite

    // LIKE over the equipment columns. The old FTS branch put the search text
    // into the SQL with format! (a quote broke out of the MATCH string) and
    // read equipment_fts, which the schema never creates; it also listed
    // archived equipment.
    let (condition, params) = crate::query_builders::utils::like_any(
        &["name", "manufacturer", "model", "serial_number", "location", "description"],
        search_term,
    );
    let sql = format!(
        "SELECT * FROM equipment WHERE deleted_at IS NULL AND {} ORDER BY name LIMIT ?",
        condition
    );
    let mut db_query = sqlx::query_as::<_, Equipment>(&sql);
    for p in &params {
        db_query = db_query.bind(p);
    }
    let equipment: Vec<Equipment> = db_query
        .bind(limit)
        .fetch_all(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(equipment)))
}

// ==================== ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ ====================

/// Проверка существования оборудования
async fn check_equipment_exists(pool: &SqlitePool, equipment_id: &str) -> ApiResult<()> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM equipment WHERE id = ?)"
    )
        .bind(equipment_id)
        .fetch_one(pool)
        .await?;

    if !exists {
        return Err(ApiError::not_found("Equipment"));
    }
    Ok(())
}

/// Получение частей оборудования (внутренняя функция)
async fn get_equipment_parts_internal(
    pool: &SqlitePool,
    equipment_id: &str,
) -> ApiResult<Vec<EquipmentPart>> {
    let parts: Vec<EquipmentPart> = sqlx::query_as(
        "SELECT * FROM equipment_parts WHERE equipment_id = ? ORDER BY name"
    )
        .bind(equipment_id)
        .fetch_all(pool)
        .await?;

    Ok(parts)
}

/// Получение недавнего обслуживания (внутренняя функция)
async fn get_recent_maintenance_internal(
    pool: &SqlitePool,
    equipment_id: &str,
    limit: i32,
) -> ApiResult<Vec<EquipmentMaintenance>> {
    let maintenance: Vec<EquipmentMaintenance> = sqlx::query_as(
        r#"SELECT * FROM equipment_maintenance 
           WHERE equipment_id = ? 
           ORDER BY scheduled_date DESC 
           LIMIT ?"#
    )
        .bind(equipment_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

    Ok(maintenance)
}

/// Получение файлов оборудования (внутренняя функция)
async fn get_equipment_files_internal(
    pool: &SqlitePool,
    equipment_id: &str,
) -> ApiResult<Vec<EquipmentFile>> {
    let files: Vec<EquipmentFile> = sqlx::query_as(
        "SELECT * FROM equipment_files WHERE equipment_id = ? ORDER BY created_at DESC"
    )
        .bind(equipment_id)
        .fetch_all(pool)
        .await?;

    Ok(files)
}

/// Получение файлов запчасти
pub async fn get_part_files(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
) -> ApiResult<HttpResponse> {
    let (equipment_id, part_id) = path.into_inner();

    check_equipment_exists(&app_state.db_pool, &equipment_id).await?;

    let files: Vec<EquipmentFile> = sqlx::query_as(
        "SELECT * FROM equipment_files WHERE equipment_id = ? AND part_id = ? ORDER BY created_at DESC"
    )
        .bind(&equipment_id)
        .bind(&part_id)
        .fetch_all(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(files)))
}

/// Применение фильтров к CountQueryBuilder
fn apply_equipment_filters(
    builder: &mut CountQueryBuilder,
    query: &EquipmentPaginationQuery,
    _whitelist: &FieldWhitelist,
) -> Result<(), ApiError> {
    if let Some(ref search) = query.search {
        if !search.trim().is_empty() {
            builder.add_like("name", search);
        }
    }

    if let Some(ref status) = query.status {
        builder.add_exact_match("status", status);
    }

    if let Some(ref type_) = query.type_ {
        builder.add_exact_match("type_", type_);
    }

    if let Some(ref location) = query.location {
        builder.add_exact_match("location", location);
    }

    Ok(())
}
/// Применение фильтров к SafeQueryBuilder
fn apply_equipment_filters_safe(
    builder: &mut SafeQueryBuilder,
    query: &EquipmentPaginationQuery,
) -> Result<(), ApiError> {
    if let Some(ref search) = query.search {
        if !search.trim().is_empty() {
            builder.add_like("name", search);
        }
    }

    if let Some(ref status) = query.status {
        builder.add_exact_match("status", status);
    }

    if let Some(ref type_) = query.type_ {
        builder.add_exact_match("type_", type_);
    }

    if let Some(ref location) = query.location {
        builder.add_exact_match("location", location);
    }

    Ok(())
}
/// Валидация данных оборудования
fn validate_equipment_data(equipment: &CreateEquipmentRequest) -> Result<(), ApiError> {
    if equipment.name.trim().is_empty() {
        return Err(ApiError::bad_request("Name cannot be empty"));
    }

    if equipment.quantity < 1 {
        return Err(ApiError::bad_request("Quantity must be at least 1"));
    }

    // FIXED: FromStr trait is now in scope
    if EquipmentType::from_str(&equipment.type_).is_err() {
        return Err(ApiError::bad_request(&format!(
            "Invalid type: {}. Valid: instrument, glassware, safety, storage, consumable, other",
            equipment.type_
        )));
    }

    Ok(())
}

// ==================== ВСПОМОГАТЕЛЬНЫЕ СТРУКТУРЫ ====================

/// Данные загруженного файла
struct FileUploadData {
    original_filename: String,
    stored_filename: String,
    file_path: String,
    file_size: usize,
    mime_type: String,
    file_type: String,
}

// ==================== ТЕСТЫ ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equipment_type_validation() {
        assert!(EquipmentType::from_str("instrument").is_ok());
        assert!(EquipmentType::from_str("glassware").is_ok());
        assert!(EquipmentType::from_str("safety").is_ok());
        assert!(EquipmentType::from_str("invalid").is_err());
    }

    #[test]
    fn test_part_status_validation() {
        // Part statuses matching DB constraint:
        // status IN ('good', 'needs_attention', 'needs_replacement', 'replaced', 'missing')
        let valid_statuses = ["good", "needs_attention", "needs_replacement", "replaced", "missing"];

        assert!(valid_statuses.contains(&"good"));
        assert!(valid_statuses.contains(&"needs_attention"));
        assert!(valid_statuses.contains(&"needs_replacement"));
        assert!(valid_statuses.contains(&"replaced"));
        assert!(valid_statuses.contains(&"missing"));
        assert!(!valid_statuses.contains(&"invalid"));
        assert!(!valid_statuses.contains(&"available")); // Old value - should fail
    }

    #[test]
    fn maintenance_dates_and_types() {
        assert_eq!(parse_day("2026-10-28", "Date").unwrap(), "2026-10-28");
        assert_eq!(parse_day(" 2026-10-28T09:00:00Z ", "Date").unwrap(), "2026-10-28");
        assert!(parse_day("not a date", "Date").is_err());
        assert!(parse_day("2026-02-30", "Date").is_err());
        assert!(parse_day("", "Date").is_err());
        for t in ["calibration", "repair", "inspection", "cleaning", "replacement", "other"] {
            assert!(check_maintenance_type(t).is_ok(), "{}", t);
        }
        // what the old form sent
        assert!(check_maintenance_type("scheduled").is_err());
        assert!(check_maintenance_type("part_replacement").is_err());
        for s in ["scheduled", "in_progress", "completed", "cancelled"] {
            assert!(check_maintenance_status(s).is_ok(), "{}", s);
        }
        assert!(check_maintenance_status("overdue").is_err());
    }

    #[test]
    fn documents_open_in_the_browser() {
        use actix_web::http::header::{DispositionType, TryIntoHeaderValue};
        let pdf = file_disposition("Руководство HPLC.pdf", "application/pdf", false);
        assert_eq!(pdf.disposition, DispositionType::Inline);
        let v = pdf.clone().try_into_value().unwrap();
        let v = v.to_str().unwrap();
        assert!(v.starts_with("inline"), "{}", v);
        assert!(v.contains("filename*=UTF-8''%D0%A0%D1%83%D0%BA"), "{}", v);
        assert!(v.contains("filename=\"___________ HPLC.pdf\""), "{}", v);
        assert_eq!(file_disposition("a.pdf", "application/pdf", true).disposition, DispositionType::Attachment);
        assert_eq!(file_disposition("a.png", "image/png", false).disposition, DispositionType::Inline);
        assert_eq!(file_disposition("a.docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", false).disposition, DispositionType::Attachment);
        // a quote or line break in a name cannot break the header
        let odd = file_disposition("a\"b\r\nX: y.pdf", "application/pdf", false).try_into_value().unwrap();
        assert!(!odd.to_str().unwrap().contains('\n'));
    }
}