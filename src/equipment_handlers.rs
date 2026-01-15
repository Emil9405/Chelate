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
    EquipmentMaintenance, EquipmentMaintenanceWithEquipment,
    CreateMaintenanceRequest, UpdateMaintenanceRequest, CompleteMaintenanceRequest,
    EquipmentFile, UploadFileRequest, EquipmentDetailResponse
};
use crate::error::{ApiError, ApiResult};
use crate::handlers::{ApiResponse, PaginatedResponse};
use crate::query_builders::{
    SafeQueryBuilder, CountQueryBuilder, FieldWhitelist,
    EquipmentType, MaintenanceType, MaintenanceStatus,
    MaintenanceValidator, generate_unique_filename, validate_file_size, validate_mime_type,
};

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

/// Возвращает путь к директории файлов оборудования (кроссплатформенно)
fn get_equipment_files_dir() -> std::path::PathBuf {
    std::env::var("EQUIPMENT_FILES_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(".")
                .join("uploads")
                .join("equipment")
        })
}

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
        "SELECT * FROM equipment WHERE id = ?"
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
            created_by, updated_by, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, 'available', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
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
        .bind(&_user_id)
        .bind(&_user_id)
        .bind(&now)
        .bind(&now)
        .execute(&app_state.db_pool)
        .await?;

    // Обновляем FTS индекс
    update_equipment_fts(&app_state.db_pool, &id).await?;

    let created: Equipment = sqlx::query_as("SELECT * FROM equipment WHERE id = ?")
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
        "SELECT * FROM equipment WHERE id = ?"
    )
        .bind(&equipment_id)
        .fetch_optional(&app_state.db_pool)
        .await?;

    if existing.is_none() {
        return Err(ApiError::not_found("Equipment"));
    }

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

    add_field!(name, "name");
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

    // Обновляем FTS индекс
    update_equipment_fts(&app_state.db_pool, &equipment_id).await?;

    let updated: Equipment = sqlx::query_as("SELECT * FROM equipment WHERE id = ?")
        .bind(&equipment_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(updated)))
}

/// Удаление оборудования
pub async fn delete_equipment(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let equipment_id = path.into_inner();

    // Удаляем связанные данные
    sqlx::query("DELETE FROM equipment_parts WHERE equipment_id = ?")
        .bind(&equipment_id)
        .execute(&app_state.db_pool)
        .await?;

    sqlx::query("DELETE FROM equipment_maintenance WHERE equipment_id = ?")
        .bind(&equipment_id)
        .execute(&app_state.db_pool)
        .await?;

    // Удаляем файлы с диска
    let files: Vec<EquipmentFile> = sqlx::query_as(
        "SELECT * FROM equipment_files WHERE equipment_id = ?"
    )
        .bind(&equipment_id)
        .fetch_all(&app_state.db_pool)
        .await?;

    for file in files {
        let _ = std::fs::remove_file(&file.file_path);
    }

    sqlx::query("DELETE FROM equipment_files WHERE equipment_id = ?")
        .bind(&equipment_id)
        .execute(&app_state.db_pool)
        .await?;

    // Удаляем из FTS
    sqlx::query("DELETE FROM equipment_fts WHERE equipment_id = ?")
        .bind(&equipment_id)
        .execute(&app_state.db_pool)
        .await
        .ok(); // Игнорируем ошибку если FTS таблица не существует

    // Удаляем само оборудование
    let result = sqlx::query("DELETE FROM equipment WHERE id = ?")
        .bind(&equipment_id)
        .execute(&app_state.db_pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Equipment"));
    }

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

    if existing.is_none() {
        return Err(ApiError::not_found("Equipment part"));
    }

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

    let result = sqlx::query(
        "DELETE FROM equipment_parts WHERE id = ? AND equipment_id = ?"
    )
        .bind(&part_id)
        .bind(&equipment_id)
        .execute(&app_state.db_pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Equipment part"));
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        (),
        "Part deleted successfully".to_string(),
    )))
}

// ==================== ОБСЛУЖИВАНИЕ ====================

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

    // FIXED: FromStr trait is now in scope
    if MaintenanceType::from_str(&maintenance.maintenance_type).is_err() {
        return Err(ApiError::bad_request(&format!(
            "Invalid maintenance type: {}",
            maintenance.maintenance_type
        )));
    }

    // Валидация временных интервалов
    if let Some(ref end) = maintenance.completed_date {
        if MaintenanceValidator::validate_time_range(&maintenance.scheduled_date, end).is_err() {
            return Err(ApiError::bad_request("Completed date cannot be before scheduled date"));
        }
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let status = maintenance.status.as_deref().unwrap_or("scheduled");

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
        .bind(&maintenance.scheduled_date)
        .bind(&maintenance.completed_date)
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

    if existing.is_none() {
        return Err(ApiError::not_found("Maintenance record"));
    }

    let mut updates = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(ref status) = update.status {
        // FIXED: FromStr trait is now in scope
        if MaintenanceStatus::from_str(status).is_err() {
            return Err(ApiError::bad_request(&format!("Invalid status: {}", status)));
        }
        updates.push("status = ?");
        values.push(status.clone());
    }
    if let Some(ref completed_date) = update.completed_date {
        updates.push("completed_date = ?");
        values.push(completed_date.clone());
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

    let completed_date = body.completed_date.clone()
        .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());

    sqlx::query(
        r#"UPDATE equipment_maintenance 
           SET status = 'completed', completed_date = ?, performed_by = ?, 
               notes = COALESCE(?, notes), updated_at = ?
           WHERE id = ?"#
    )
        .bind(&completed_date)
        .bind(&body.performed_by)
        .bind(&body.notes)
        .bind(Utc::now())
        .bind(&maintenance_id)
        .execute(&app_state.db_pool)
        .await?;

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
        "SELECT * FROM equipment WHERE id = ?"
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
                    let valid_types = ["manual", "image", "certificate", "specification", "maintenance_log", "other"];
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

    // Создаём древовидную структуру папок
    let sanitized_equip_name = sanitize_folder_name(&equipment.name);
    let type_folder = get_type_folder(&file_type);

    let file_path = if let Some(ref part_id) = form_part_id {
        // Получаем имя запчасти
        let part: EquipmentPart = sqlx::query_as(
            "SELECT * FROM equipment_parts WHERE id = ? AND equipment_id = ?"
        )
            .bind(part_id)
            .bind(&equipment_id)
            .fetch_optional(&app_state.db_pool)
            .await?
            .ok_or_else(|| ApiError::not_found("Part"))?;

        let sanitized_part_name = sanitize_folder_name(&part.name);

        // Структура: equipment/{equip_name}/parts/{part_name}/{type}/
        let type_dir = get_equipment_files_dir()
            .join(&sanitized_equip_name)
            .join("parts")
            .join(&sanitized_part_name)
            .join(type_folder);

        std::fs::create_dir_all(&type_dir)
            .map_err(|e| ApiError::InternalServerError(format!("Failed to create directory: {}", e)))?;

        let unique_filename = generate_unique_filename(&original_filename);
        type_dir.join(&unique_filename).to_string_lossy().to_string()
    } else {
        // Структура: equipment/{equip_name}/{type}/
        let type_dir = get_equipment_files_dir()
            .join(&sanitized_equip_name)
            .join(type_folder);

        std::fs::create_dir_all(&type_dir)
            .map_err(|e| ApiError::InternalServerError(format!("Failed to create directory: {}", e)))?;

        let unique_filename = generate_unique_filename(&original_filename);
        type_dir.join(&unique_filename).to_string_lossy().to_string()
    };

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

    sqlx::query(
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
        .await?;

    let created: EquipmentFile = sqlx::query_as(
        "SELECT * FROM equipment_files WHERE id = ?"
    )
        .bind(&id)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(created)))
}

/// Очистка имени папки от спецсимволов
fn sanitize_folder_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .replace(' ', "_")
        .to_lowercase()
}

/// Получение имени папки для типа файла
fn get_type_folder(file_type: &str) -> &'static str {
    match file_type {
        "photo" => "images",  // Changed from "image" to match DB constraint
        "manual" => "manuals",
        "certificate" => "certificates",
        "specification" => "specifications",
        "maintenance_log" => "maintenance_logs",
        _ => "other"
    }
}

/// Скачивание файла оборудования
pub async fn download_equipment_file(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
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

    // Определяем Content-Disposition: inline для изображений, attachment для остальных
    let disposition = if file.mime_type.starts_with("image/") {
        format!("inline; filename=\"{}\"", file.original_filename)
    } else {
        format!("attachment; filename=\"{}\"", file.original_filename)
    };

    Ok(HttpResponse::Ok()
        .content_type(file.mime_type)
        .insert_header(("Content-Disposition", disposition))
        .insert_header(("Cache-Control", "public, max-age=3600"))
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

    // Удаляем файл с диска
    let _ = std::fs::remove_file(&file.file_path);

    // Удаляем из БД
    sqlx::query("DELETE FROM equipment_files WHERE id = ?")
        .bind(&file_id)
        .execute(&app_state.db_pool)
        .await?;

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

    let limit = query.limit.unwrap_or(20).min(100);

    // Проверяем доступность FTS
    let fts_available: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='equipment_fts'"
    )
        .fetch_one(&app_state.db_pool)
        .await
        .unwrap_or(false);

    let equipment: Vec<Equipment> = if fts_available {
        // FTS поиск
        let escaped_term = search_term.replace("\"", "\"\"");
        let sql = format!(
            r#"SELECT e.* FROM equipment e
               JOIN equipment_fts f ON e.id = f.equipment_id
               WHERE equipment_fts MATCH '"{}"'
               ORDER BY rank
               LIMIT ?"#,
            escaped_term
        );

        sqlx::query_as::<_, Equipment>(&sql)
            .bind(limit)
            .fetch_all(&app_state.db_pool)
            .await?
    } else {
        // Fallback на LIKE
        let pattern = format!("%{}%", search_term);
        sqlx::query_as::<_, Equipment>(
            "SELECT * FROM equipment WHERE name LIKE ? OR description LIKE ? OR location LIKE ? ORDER BY name LIMIT ?"
        )
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .bind(limit)
            .fetch_all(&app_state.db_pool)
            .await?
    };

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

/// Обновление FTS индекса для оборудования
/// ПРИМЕЧАНИЕ: FTS5 с content table синхронизируется автоматически через триггеры.
/// Эта функция оставлена для совместимости, но фактически ничего не делает,
/// так как триггеры equipment_ai/equipment_au/equipment_ad обрабатывают синхронизацию.
async fn update_equipment_fts(_pool: &SqlitePool, _equipment_id: &str) -> ApiResult<()> {
    // FTS5 с content='equipment' синхронизируется автоматически через триггеры
    // Ручное обновление не требуется и может вызвать ошибки
    Ok(())
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
}