// src/models/storage_zone.rs
//! Модели зон хранения (шкафы, холодильники, вытяжные шкафы и т.д.)

use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, Utc};

/// Тип зоны хранения
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageZoneType {
    Cabinet,         // Обычный шкаф
    Refrigerator,    // Холодильник (+2..+8°C)
    Freezer,         // Морозильник (-20°C и ниже)
    FumeHood,        // Вытяжной шкаф
    SafetyCabinet,   // Шкаф для ЛВЖ/кислот/щелочей
    Desiccator,      // Эксикатор
    Shelf,           // Открытый стеллаж
    Drawer,          // Тумба с ящиками
    Other,           // Другое
}

impl StorageZoneType {
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageZoneType::Cabinet => "cabinet",
            StorageZoneType::Refrigerator => "refrigerator",
            StorageZoneType::Freezer => "freezer",
            StorageZoneType::FumeHood => "fume_hood",
            StorageZoneType::SafetyCabinet => "safety_cabinet",
            StorageZoneType::Desiccator => "desiccator",
            StorageZoneType::Shelf => "shelf",
            StorageZoneType::Drawer => "drawer",
            StorageZoneType::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "cabinet" => Some(StorageZoneType::Cabinet),
            "refrigerator" => Some(StorageZoneType::Refrigerator),
            "freezer" => Some(StorageZoneType::Freezer),
            "fume_hood" => Some(StorageZoneType::FumeHood),
            "safety_cabinet" => Some(StorageZoneType::SafetyCabinet),
            "desiccator" => Some(StorageZoneType::Desiccator),
            "shelf" => Some(StorageZoneType::Shelf),
            "drawer" => Some(StorageZoneType::Drawer),
            "other" => Some(StorageZoneType::Other),
            _ => None,
        }
    }

    pub fn is_valid(s: &str) -> bool {
        Self::from_str(s).is_some()
    }

    pub const fn all_values() -> &'static [&'static str] {
        &[
            "cabinet", "refrigerator", "freezer", "fume_hood",
            "safety_cabinet", "desiccator", "shelf", "drawer", "other",
        ]
    }

    /// Человекочитаемое название на английском
    pub fn display_name(&self) -> &'static str {
        match self {
            StorageZoneType::Cabinet => "Cabinet",
            StorageZoneType::Refrigerator => "Refrigerator",
            StorageZoneType::Freezer => "Freezer",
            StorageZoneType::FumeHood => "Fume Hood",
            StorageZoneType::SafetyCabinet => "Safety Cabinet",
            StorageZoneType::Desiccator => "Desiccator",
            StorageZoneType::Shelf => "Shelf Unit",
            StorageZoneType::Drawer => "Drawer Unit",
            StorageZoneType::Other => "Other",
        }
    }
}

/// Условия хранения для зоны
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageCondition {
    RoomTemperature,  // Комнатная температура
    Cool,             // Прохладное (+2..+8°C)
    Frozen,           // Замороженное (-20°C)
    DeepFrozen,       // Глубокая заморозка (-80°C)
    Ventilated,       // Вентилируемое (вытяжка)
    DryStorage,       // Сухое хранение
    LightProtected,   // Защита от света
}

impl StorageCondition {
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageCondition::RoomTemperature => "room_temperature",
            StorageCondition::Cool => "cool",
            StorageCondition::Frozen => "frozen",
            StorageCondition::DeepFrozen => "deep_frozen",
            StorageCondition::Ventilated => "ventilated",
            StorageCondition::DryStorage => "dry_storage",
            StorageCondition::LightProtected => "light_protected",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "room_temperature" => Some(StorageCondition::RoomTemperature),
            "cool" => Some(StorageCondition::Cool),
            "frozen" => Some(StorageCondition::Frozen),
            "deep_frozen" => Some(StorageCondition::DeepFrozen),
            "ventilated" => Some(StorageCondition::Ventilated),
            "dry_storage" => Some(StorageCondition::DryStorage),
            "light_protected" => Some(StorageCondition::LightProtected),
            _ => None,
        }
    }

    pub fn is_valid(s: &str) -> bool {
        Self::from_str(s).is_some()
    }

    pub const fn all_values() -> &'static [&'static str] {
        &[
            "room_temperature", "cool", "frozen", "deep_frozen",
            "ventilated", "dry_storage", "light_protected",
        ]
    }
}

/// Зона хранения (шкаф, холодильник и т.д.)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct StorageZone {
    pub id: String,
    pub room_id: String,
    pub name: String,
    pub zone_type: String,
    pub storage_condition: Option<String>,
    pub description: Option<String>,
    pub temperature_min: Option<f64>,
    pub temperature_max: Option<f64>,
    pub is_locked: i32,          // 0/1 - замкнутый шкаф
    pub sort_order: i32,
    pub status: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Позиция хранения (полка, ящик, отсек)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct StoragePosition {
    pub id: String,
    pub zone_id: String,
    pub name: String,
    pub position_label: Option<String>,  // "A1", "Shelf 3", etc.
    pub max_capacity: Option<i32>,       // Максимум единиц хранения
    pub current_count: i32,              // Кэшированное количество предметов
    pub sort_order: i32,
    pub description: Option<String>,
    pub status: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// === Request DTOs ===

#[derive(Debug, Deserialize, Validate)]
pub struct CreateStorageZoneRequest {
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: String,
    pub room_id: String,
    pub zone_type: String,
    pub storage_condition: Option<String>,
    #[validate(length(max = 500, message = "Description max 500 characters"))]
    pub description: Option<String>,
    pub temperature_min: Option<f64>,
    pub temperature_max: Option<f64>,
    pub is_locked: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateStorageZoneRequest {
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: Option<String>,
    pub zone_type: Option<String>,
    pub storage_condition: Option<String>,
    #[validate(length(max = 500, message = "Description max 500 characters"))]
    pub description: Option<String>,
    pub temperature_min: Option<f64>,
    pub temperature_max: Option<f64>,
    pub is_locked: Option<bool>,
    pub sort_order: Option<i32>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateStoragePositionRequest {
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: String,
    pub zone_id: String,
    #[validate(length(max = 20, message = "Label max 20 characters"))]
    pub position_label: Option<String>,
    pub max_capacity: Option<i32>,
    pub sort_order: Option<i32>,
    #[validate(length(max = 500, message = "Description max 500 characters"))]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateStoragePositionRequest {
    #[validate(length(min = 1, max = 100, message = "Name must be 1-100 characters"))]
    pub name: Option<String>,
    #[validate(length(max = 20, message = "Label max 20 characters"))]
    pub position_label: Option<String>,
    pub max_capacity: Option<i32>,
    pub sort_order: Option<i32>,
    #[validate(length(max = 500, message = "Description max 500 characters"))]
    pub description: Option<String>,
    pub status: Option<String>,
}

// === Расширенные DTO для иерархического отображения ===

/// Зона хранения с вложенными позициями
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StorageZoneWithPositions {
    #[serde(flatten)]
    pub zone: StorageZone,
    pub positions: Vec<StoragePosition>,
    pub items_count: i64,  // Суммарное кол-во предметов в зоне
}

/// Комната с вложенными зонами хранения
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoomWithStorage {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub capacity: Option<i32>,
    pub color: Option<String>,
    pub status: String,
    pub zones: Vec<StorageZoneWithPositions>,
    pub total_items: i64,
}

/// Полный путь к месту хранения (для отображения)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StorageLocationPath {
    pub room_id: String,
    pub room_name: String,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub zone_type: Option<String>,
    pub position_id: Option<String>,
    pub position_name: Option<String>,
    pub position_label: Option<String>,
    /// Форматированный путь: "Lab 104 → Cabinet A → Shelf 3"
    pub full_path: String,
}
