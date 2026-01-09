// src/models/equipment.rs
use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, Utc};

// ==================== EQUIPMENT (ОБОРУДОВАНИЕ) ====================

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Equipment {
    pub id: String,
    pub name: String,
    #[sqlx(rename = "type_")]
    #[serde(rename = "type_")]
    pub type_: String,
    pub quantity: i32,
    pub unit: Option<String>,
    pub status: String,
    pub location: Option<String>,
    pub description: Option<String>,
    // Дополнительные поля
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub purchase_date: Option<String>,
    pub warranty_until: Option<String>,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct CreateEquipmentRequest {
    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: String,

    #[validate(length(min = 1, max = 50, message = "Type must be 'equipment' or 'labware'"))]
    #[serde(rename = "type_")]
    pub type_: String,

    #[validate(range(min = 1, message = "Quantity must be at least 1"))]
    pub quantity: i32,

    #[validate(length(max = 20, message = "Unit cannot exceed 20 characters"))]
    pub unit: Option<String>,

    #[validate(length(max = 255, message = "Location cannot exceed 255 characters"))]
    pub location: Option<String>,

    #[validate(length(max = 1000, message = "Description cannot exceed 1000 characters"))]
    pub description: Option<String>,

    // Дополнительные поля
    #[validate(length(max = 100, message = "Serial number cannot exceed 100 characters"))]
    pub serial_number: Option<String>,

    #[validate(length(max = 255, message = "Manufacturer cannot exceed 255 characters"))]
    pub manufacturer: Option<String>,

    #[validate(length(max = 255, message = "Model cannot exceed 255 characters"))]
    pub model: Option<String>,

    pub purchase_date: Option<String>,
    pub warranty_until: Option<String>,
}

/// Расширенный запрос на создание (с большим списком допустимых типов)
#[derive(Debug, Deserialize, Validate, Clone)]
pub struct CreateEquipmentRequestExtended {
    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: String,

    #[validate(length(min = 1, max = 50, message = "Type must be 'equipment', 'labware', 'instrument', or 'consumable'"))]
    #[serde(rename = "type_")]
    pub type_: String,

    #[validate(range(min = 1, message = "Quantity must be at least 1"))]
    pub quantity: i32,

    #[validate(length(max = 20, message = "Unit cannot exceed 20 characters"))]
    pub unit: Option<String>,

    #[validate(length(max = 255, message = "Location cannot exceed 255 characters"))]
    pub location: Option<String>,

    #[validate(length(max = 1000, message = "Description cannot exceed 1000 characters"))]
    pub description: Option<String>,

    #[validate(length(max = 100, message = "Serial number cannot exceed 100 characters"))]
    pub serial_number: Option<String>,

    #[validate(length(max = 255, message = "Manufacturer cannot exceed 255 characters"))]
    pub manufacturer: Option<String>,

    #[validate(length(max = 255, message = "Model cannot exceed 255 characters"))]
    pub model: Option<String>,

    pub purchase_date: Option<String>,
    pub warranty_until: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateEquipmentRequest {
    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: Option<String>,

    #[validate(length(max = 20, message = "Unit cannot exceed 20 characters"))]
    pub unit: Option<String>,

    #[validate(length(max = 255, message = "Location cannot exceed 255 characters"))]
    pub location: Option<String>,

    #[validate(length(max = 1000, message = "Description cannot exceed 1000 characters"))]
    pub description: Option<String>,

    pub status: Option<String>,

    pub quantity: Option<i32>,

    #[validate(length(max = 100, message = "Serial number cannot exceed 100 characters"))]
    pub serial_number: Option<String>,

    #[validate(length(max = 255, message = "Manufacturer cannot exceed 255 characters"))]
    pub manufacturer: Option<String>,

    #[validate(length(max = 255, message = "Model cannot exceed 255 characters"))]
    pub model: Option<String>,

    pub purchase_date: Option<String>,
    pub warranty_until: Option<String>,
}

pub type UpdateEquipmentRequestExtended = UpdateEquipmentRequest;

// ==================== PARTS (ЗАПЧАСТИ) ====================

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct EquipmentPart {
    pub id: String,
    pub equipment_id: String,
    pub name: String,
    pub part_number: Option<String>,
    pub manufacturer: Option<String>,
    pub quantity: i32,
    pub min_quantity: i32,
    pub status: String,
    pub last_replaced: Option<String>,
    pub next_replacement: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct CreateEquipmentPartRequest {
    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: String,

    #[validate(length(max = 100, message = "Part number cannot exceed 100 characters"))]
    pub part_number: Option<String>,

    #[validate(length(max = 255, message = "Manufacturer cannot exceed 255 characters"))]
    pub manufacturer: Option<String>,

    pub quantity: Option<i32>,
    pub min_quantity: Option<i32>,

    #[validate(length(max = 50, message = "Status cannot exceed 50 characters"))]
    pub status: Option<String>,

    pub last_replaced: Option<String>,
    pub next_replacement: Option<String>,

    #[validate(length(max = 1000, message = "Notes cannot exceed 1000 characters"))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateEquipmentPartRequest {
    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: Option<String>,

    #[validate(length(max = 100, message = "Part number cannot exceed 100 characters"))]
    pub part_number: Option<String>,

    #[validate(length(max = 255, message = "Manufacturer cannot exceed 255 characters"))]
    pub manufacturer: Option<String>,

    pub quantity: Option<i32>,
    pub min_quantity: Option<i32>,

    #[validate(length(max = 50, message = "Status cannot exceed 50 characters"))]
    pub status: Option<String>,

    pub last_replaced: Option<String>,
    pub next_replacement: Option<String>,

    #[validate(length(max = 1000, message = "Notes cannot exceed 1000 characters"))]
    pub notes: Option<String>,
}

// ==================== MAINTENANCE (ОБСЛУЖИВАНИЕ) ====================

/// Запись об обслуживании оборудования
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct EquipmentMaintenance {
    pub id: String,
    pub equipment_id: String,
    pub maintenance_type: String,
    pub status: String,
    pub scheduled_date: String,
    pub completed_date: Option<String>,
    pub performed_by: Option<String>,
    pub description: Option<String>,
    pub cost: Option<f64>,
    pub parts_replaced: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Обслуживание с информацией об оборудовании (Исправлена для JOIN запросов)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct EquipmentMaintenanceWithEquipment {
    // Поля из maintenance
    pub id: String,
    pub equipment_id: String,
    pub maintenance_type: String,
    pub status: String,
    pub scheduled_date: String,
    pub completed_date: Option<String>,
    pub performed_by: Option<String>,
    pub description: Option<String>,
    pub cost: Option<f64>,
    pub parts_replaced: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Поля из equipment (добавляются через JOIN)
    pub equipment_name: String,
    pub equipment_location: Option<String>,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct CreateMaintenanceRequest {
    #[validate(length(min = 1, max = 50, message = "Maintenance type is required"))]
    pub maintenance_type: String,

    #[validate(length(max = 50, message = "Status cannot exceed 50 characters"))]
    pub status: Option<String>,

    pub scheduled_date: String,
    pub completed_date: Option<String>,

    #[validate(length(max = 255, message = "Performed by cannot exceed 255 characters"))]
    pub performed_by: Option<String>,

    #[validate(length(max = 2000, message = "Description cannot exceed 2000 characters"))]
    pub description: Option<String>,

    pub cost: Option<f64>,

    #[validate(length(max = 1000, message = "Parts replaced cannot exceed 1000 characters"))]
    pub parts_replaced: Option<String>,

    #[validate(length(max = 1000, message = "Notes cannot exceed 1000 characters"))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateMaintenanceRequest {
    #[validate(length(max = 50, message = "Status cannot exceed 50 characters"))]
    pub status: Option<String>,

    pub completed_date: Option<String>,

    #[validate(length(max = 255, message = "Performed by cannot exceed 255 characters"))]
    pub performed_by: Option<String>,

    #[validate(length(max = 2000, message = "Description cannot exceed 2000 characters"))]
    pub description: Option<String>,

    pub cost: Option<f64>,

    #[validate(length(max = 1000, message = "Parts replaced cannot exceed 1000 characters"))]
    pub parts_replaced: Option<String>,

    #[validate(length(max = 1000, message = "Notes cannot exceed 1000 characters"))]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteMaintenanceRequest {
    pub completed_date: Option<String>,
    pub performed_by: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpcomingMaintenanceQuery {
    pub days: Option<i32>,
    pub limit: Option<i32>,
}

// ==================== FILES (ФАЙЛЫ) ====================

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct EquipmentFile {
    pub id: String,
    pub equipment_id: String,
    pub part_id: Option<String>,
    pub file_type: String,
    pub original_filename: String,
    pub stored_filename: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub description: Option<String>,
    pub uploaded_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UploadFileRequest {
    #[validate(length(max = 50, message = "File type cannot exceed 50 characters"))]
    pub file_type: Option<String>,

    #[validate(length(max = 500, message = "Description cannot exceed 500 characters"))]
    pub description: Option<String>,
}

// ==================== DETAIL RESPONSE ====================

/// Детальный ответ с оборудованием и связанными данными
#[derive(Debug, Serialize)]
pub struct EquipmentDetailResponse {
    #[serde(flatten)]
    pub equipment: Equipment,
    pub parts: Vec<EquipmentPart>,
    pub recent_maintenance: Vec<EquipmentMaintenance>,
    pub files: Vec<EquipmentFile>,
}