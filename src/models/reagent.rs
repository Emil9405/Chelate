// src/models/reagent.rs
use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, Utc};

// ==================== REAGENT ====================

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Reagent {
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
    /// 'public' | 'hidden' — скрытые реагенты не показываются в дефолтном списке
    #[serde(default = "default_visibility")]
    #[sqlx(default)]
    pub visibility: String,
    // Cached aggregation fields (обновляются триггерами при изменении batches)
    pub total_quantity: f64,
    pub batches_count: i64,
    pub primary_unit: Option<String>,
    // Audit fields
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(default)]
    pub deleted_at: Option<DateTime<Utc>>,

}

fn default_visibility() -> String {
    "public".to_string()
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct CreateReagentRequest {
    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: String,

    #[validate(length(max = 500, message = "Formula cannot exceed 500 characters"))]
    pub formula: Option<String>,

    #[validate(length(max = 50, message = "CAS number cannot exceed 50 characters"))]
    pub cas_number: Option<String>,

    #[validate(length(max = 255, message = "Manufacturer cannot exceed 255 characters"))]
    pub manufacturer: Option<String>,

    #[validate(range(min = 0.0001, message = "Molecular weight must be positive (>0)"))]
    pub molecular_weight: Option<f64>,

    #[validate(length(max = 50, message = "Physical state cannot exceed 50 characters"))]
    pub physical_state: Option<String>,

    #[validate(length(max = 1000, message = "Description cannot exceed 1000 characters"))]
    pub description: Option<String>,

    #[validate(length(max = 255, message = "Storage conditions cannot exceed 255 characters"))]
    pub storage_conditions: Option<String>,

    #[validate(length(max = 255, message = "Appearance cannot exceed 255 characters"))]
    pub appearance: Option<String>,

    #[validate(length(max = 100, message = "Hazard pictograms cannot exceed 100 characters"))]
    pub hazard_pictograms: Option<String>,

    /// 'public' (default) или 'hidden'
    pub visibility: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateReagentRequest {
    #[validate(length(min = 1, max = 255, message = "Name must be between 1 and 255 characters"))]
    pub name: Option<String>,

    #[validate(length(max = 500, message = "Formula cannot exceed 500 characters"))]
    pub formula: Option<String>,

    #[validate(length(max = 50, message = "CAS number cannot exceed 50 characters"))]
    pub cas_number: Option<String>,

    #[validate(length(max = 255, message = "Manufacturer cannot exceed 255 characters"))]
    pub manufacturer: Option<String>,

    #[validate(range(min = 0.0001, message = "Molecular weight must be positive (>0)"))]
    pub molecular_weight: Option<f64>,

    #[validate(length(max = 50, message = "Physical state cannot exceed 50 characters"))]
    pub physical_state: Option<String>,

    #[validate(length(max = 1000, message = "Description cannot exceed 1000 characters"))]
    pub description: Option<String>,

    #[validate(length(max = 255, message = "Storage conditions cannot exceed 255 characters"))]
    pub storage_conditions: Option<String>,

    #[validate(length(max = 255, message = "Appearance cannot exceed 255 characters"))]
    pub appearance: Option<String>,

    #[validate(length(max = 100, message = "Hazard pictograms cannot exceed 100 characters"))]
    pub hazard_pictograms: Option<String>,

    pub status: Option<String>,

    /// 'public' или 'hidden' — визибилити меняется через тот же endpoint
    pub visibility: Option<String>,
}

// ==================== REAGENT WITH STOCK (legacy compatibility) ====================

/// Для обратной совместимости со старым API
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ReagentWithStock {
    // Reagent fields
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
    #[sqlx(default)]
    pub visibility: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    // Stock fields
    pub total_quantity: f64,
    pub batches_count: i64,
    pub primary_unit: Option<String>,
    // Computed fields (for backward compatibility)
    #[sqlx(default)]
    pub reserved_quantity: f64,
    #[sqlx(default)]
    pub available_quantity: f64,
    #[sqlx(default)]
    pub total_display: String,
}

impl From<Reagent> for ReagentWithStock {
    fn from(r: Reagent) -> Self {
        let total_display = if r.total_quantity > 0.0 {
            smart_format_quantity(r.total_quantity, r.primary_unit.as_deref().unwrap_or(""))
        } else {
            "No stock".to_string()
        };
        
        Self {
            id: r.id,
            name: r.name,
            formula: r.formula,
            cas_number: r.cas_number,
            manufacturer: r.manufacturer,
            molecular_weight: r.molecular_weight,
            physical_state: r.physical_state,
            description: r.description,
            storage_conditions: r.storage_conditions,
            appearance: r.appearance,
            hazard_pictograms: r.hazard_pictograms,
            status: r.status,
            visibility: r.visibility,
            created_by: r.created_by,
            updated_by: r.updated_by,
            created_at: r.created_at,
            updated_at: r.updated_at,
            total_quantity: r.total_quantity,
            batches_count: r.batches_count,
            primary_unit: r.primary_unit,
            reserved_quantity: 0.0,
            available_quantity: r.total_quantity,
            total_display,
            deleted_at: r.deleted_at,
        }
    }
}

// ==================== SMART QUANTITY FORMATTING ====================
//
// Переводит количество из базовой единицы (g / mL) в самую удобную
// для отображения: 100000 mL → "100 L", 0.5 g → "500 mg", 50 g → "50 g".
//
// Правила:
//   - Граница переключения: 1000 / 0.001 (при 999 — 999 g, при 1000 — 1 kg).
//   - Десятичные знаки: умно — целые без дробной части, дробные до 2 знаков.
//
// Если base_unit не распознан — возвращаем значение как есть (без конвертации).

fn format_smart_number(value: f64) -> String {
    // Отсекаем "почти целые": 99.995 тоже целое при 2 знаках
    let rounded = (value * 100.0).round() / 100.0;
    if (rounded.fract()).abs() < 0.005 {
        format!("{:.0}", rounded)
    } else {
        // Убираем хвостовой ноль: "99.50" → оставляем, "99.5" тоже ок;
        // но для консистентности лучше 2 знака
        format!("{:.2}", rounded)
    }
}

pub fn smart_format_quantity(value: f64, base_unit: &str) -> String {
    if value <= 0.0 {
        return format!("0 {}", base_unit);
    }

    match base_unit {
        // Масса, база — граммы
        "g" => {
            if value >= 1000.0 {
                format!("{} kg", format_smart_number(value / 1000.0))
            } else if value >= 1.0 {
                format!("{} g", format_smart_number(value))
            } else if value >= 0.001 {
                format!("{} mg", format_smart_number(value * 1000.0))
            } else {
                format!("{} μg", format_smart_number(value * 1_000_000.0))
            }
        }
        // Объём, база — миллилитры
        "mL" | "ml" => {
            if value >= 1000.0 {
                format!("{} L", format_smart_number(value / 1000.0))
            } else if value >= 1.0 {
                format!("{} mL", format_smart_number(value))
            } else {
                format!("{} μL", format_smart_number(value * 1000.0))
            }
        }
        // Прочие единицы (pcs, mol и т.д.) — без конвертации
        other => format!("{} {}", format_smart_number(value), other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_format_volume() {
        assert_eq!(smart_format_quantity(100_000.0, "mL"), "100 L");
        assert_eq!(smart_format_quantity(1000.0, "mL"), "1 L");
        assert_eq!(smart_format_quantity(1500.0, "mL"), "1.5 L");
        assert_eq!(smart_format_quantity(999.0, "mL"), "999 mL");
        assert_eq!(smart_format_quantity(500.0, "mL"), "500 mL");
        assert_eq!(smart_format_quantity(0.5, "mL"), "500 μL");
    }

    #[test]
    fn test_smart_format_mass() {
        assert_eq!(smart_format_quantity(1500.0, "g"), "1.5 kg");
        assert_eq!(smart_format_quantity(999.0, "g"), "999 g");
        assert_eq!(smart_format_quantity(50.0, "g"), "50 g");
        assert_eq!(smart_format_quantity(0.5, "g"), "500 mg");
        assert_eq!(smart_format_quantity(0.0005, "g"), "500 μg");
    }

    #[test]
    fn test_smart_format_decimals() {
        // Целое — без дроби
        assert_eq!(smart_format_quantity(100.0, "mL"), "100 mL");
        // Дробное — 2 знака
        assert_eq!(smart_format_quantity(99.5, "mL"), "99.5 mL");
        assert_eq!(smart_format_quantity(99.55, "mL"), "99.55 mL");
    }

    #[test]
    fn test_smart_format_edge_cases() {
        assert_eq!(smart_format_quantity(0.0, "mL"), "0 mL");
        assert_eq!(smart_format_quantity(-1.0, "g"), "0 g");
        // Неизвестная единица — pass-through
        assert_eq!(smart_format_quantity(5.0, "pcs"), "5 pcs");
    }
}
