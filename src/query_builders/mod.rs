// src/query_builders/mod.rs
//! Query builders для безопасного построения SQL запросов

pub mod filters;
pub mod fts;
pub mod utils; 
pub mod whitelist; // Утилиты: валидация полей, экранирование, файлы

// Re-export основных типов
pub use filters::{
    FieldWhitelist, FilterBuilder, FilterGroup, Filter, FilterItem, FilterValue,
    ComparisonOperator, ReportFilterValue, ReportFilter, ReportColumn, ReportPreset, ReportConfig,
};
pub use fts::{FtsQueryBuilder, escape_fts_query};

use serde::{Serialize, Deserialize};
use strum::{EnumString, Display, AsRefStr};



// ==================== EQUIPMENT ENUMS ====================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, Display, AsRefStr)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EquipmentStatus {
    Available,
    InUse,
    Maintenance,
    Broken,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, Display, AsRefStr)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EquipmentType {
    Instrument,
    Glassware,
    Safety,
    Storage,
    Consumable,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, Display, AsRefStr)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EquipmentFileType {
    Manual,
    Certificate,
    Photo,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, Display, AsRefStr)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceType {
    Calibration,
    Repair,
    Inspection,
    Cleaning,
    Replacement,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, Display, AsRefStr)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceStatus {
    Scheduled,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, Display, AsRefStr)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PartStatus {
    Available,
    LowStock,
    OutOfStock,
    Ordered,
}

// ==================== SAFE QUERY BUILDER ====================

/// Безопасный построитель SELECT запросов
pub struct SafeQueryBuilder<'a> {
    base_query: &'a str,
    conditions: Vec<String>,
    params: Vec<String>,
    whitelist: Option<&'a FieldWhitelist>,
    order_by: Option<(String, String)>,
    limit: Option<i64>,
    offset: Option<i64>,
}

impl<'a> SafeQueryBuilder<'a> {
    pub fn new(base_query: &'a str) -> Result<Self, String> {
        if base_query.is_empty() {
            return Err("Base query cannot be empty".to_string());
        }
        Ok(Self {
            base_query,
            conditions: Vec::new(),
            params: Vec::new(),
            whitelist: None,
            order_by: None,
            limit: None,
            offset: None,
        })
    }

    pub fn with_whitelist(mut self, whitelist: &'a FieldWhitelist) -> Self {
        self.whitelist = Some(whitelist);
        self
    }

    pub fn add_condition(&mut self, condition: &str, params: Vec<String>) -> &mut Self {
        self.conditions.push(condition.to_string());
        self.params.extend(params);
        self
    }

    pub fn add_exact_match(&mut self, field: &str, value: impl Into<String>) -> &mut Self {
        if self.is_field_allowed(field) {
            self.conditions.push(format!("{} = ?", field));
            self.params.push(value.into());
        }
        self
    }

    pub fn add_like(&mut self, field: &str, pattern: impl Into<String>) -> &mut Self {
        if self.is_field_allowed(field) {
            let p = pattern.into();
            let escaped = if p.contains('%') { p } else { format!("%{}%", p) };
            self.conditions.push(format!("{} LIKE ?", field));
            self.params.push(escaped);
        }
        self
    }

    pub fn add_comparison(&mut self, field: &str, op: &str, value: impl ToString) -> &mut Self {
        let valid_ops = ["=", "!=", "<", ">", "<=", ">=", "<>"];
        if self.is_field_allowed(field) && valid_ops.contains(&op) {
            self.conditions.push(format!("{} {} ?", field, op));
            self.params.push(value.to_string());
        }
        self
    }

    pub fn add_is_null(&mut self, field: &str) -> &mut Self {
        if self.is_field_allowed(field) {
            self.conditions.push(format!("{} IS NULL", field));
        }
        self
    }

    pub fn add_is_not_null(&mut self, field: &str) -> &mut Self {
        if self.is_field_allowed(field) {
            self.conditions.push(format!("{} IS NOT NULL", field));
        }
        self
    }

    pub fn add_in_clause(&mut self, field: &str, values: &[impl ToString]) -> &mut Self {
        if self.is_field_allowed(field) && !values.is_empty() {
            let placeholders: Vec<_> = values.iter().map(|_| "?").collect();
            self.conditions.push(format!("{} IN ({})", field, placeholders.join(", ")));
            for v in values {
                self.params.push(v.to_string());
            }
        }
        self
    }

    pub fn add_between(&mut self, field: &str, from: impl ToString, to: impl ToString) -> &mut Self {
        if self.is_field_allowed(field) {
            self.conditions.push(format!("{} BETWEEN ? AND ?", field));
            self.params.push(from.to_string());
            self.params.push(to.to_string());
        }
        self
    }

    pub fn order_by(&mut self, field: &str, direction: &str) -> &mut Self {
        if self.is_field_allowed(field) {
            let dir = if direction.to_uppercase() == "ASC" { "ASC" } else { "DESC" };
            self.order_by = Some((field.to_string(), dir.to_string()));
        }
        self
    }

    pub fn limit(&mut self, limit: i64) -> &mut Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(&mut self, offset: i64) -> &mut Self {
        self.offset = Some(offset);
        self
    }

    pub fn build(&self) -> (String, Vec<String>) {
        let mut sql = self.base_query.to_string();
        
        if !self.conditions.is_empty() {
            // Проверяем, есть ли уже WHERE в базовом запросе
            if sql.to_uppercase().contains("WHERE") {
                sql.push_str(" AND ");
            } else {
                sql.push_str(" WHERE ");
            }
            sql.push_str(&self.conditions.join(" AND "));
        }
        
        if let Some((field, dir)) = &self.order_by {
            sql.push_str(&format!(" ORDER BY {} {}", field, dir));
        }
        
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }
        
        (sql, self.params.clone())
    }

    pub fn build_count(&self) -> (String, Vec<String>) {
        let mut sql = format!("SELECT COUNT(*) as count FROM ({}) as subquery", self.base_query);
        
        if !self.conditions.is_empty() {
            if self.base_query.to_uppercase().contains("WHERE") {
                sql = format!("SELECT COUNT(*) as count FROM ({} AND {}) as subquery", 
                    self.base_query, self.conditions.join(" AND "));
            } else {
                sql = format!("SELECT COUNT(*) as count FROM ({} WHERE {}) as subquery", 
                    self.base_query, self.conditions.join(" AND "));
            }
        }
        
        (sql, self.params.clone())
    }

    pub fn has_conditions(&self) -> bool {
        !self.conditions.is_empty()
    }

    fn is_field_allowed(&self, field: &str) -> bool {
        match &self.whitelist {
            Some(wl) => wl.is_allowed(field),
            None => true,
        }
    }
}

// ==================== COUNT QUERY BUILDER ====================

/// Построитель COUNT запросов
pub struct CountQueryBuilder<'a> {
    table: &'a str,
    conditions: Vec<String>,
    params: Vec<String>,
    whitelist: Option<&'a FieldWhitelist>,
}

impl<'a> CountQueryBuilder<'a> {
    pub fn new(table: &'a str) -> Result<Self, String> {
        if table.is_empty() {
            return Err("Table name cannot be empty".to_string());
        }
        Ok(Self {
            table,
            conditions: Vec::new(),
            params: Vec::new(),
            whitelist: None,
        })
    }

    pub fn with_whitelist(mut self, whitelist: &'a FieldWhitelist) -> Self {
        self.whitelist = Some(whitelist);
        self
    }

    pub fn add_exact_match(&mut self, field: &str, value: impl Into<String>) -> &mut Self {
        if self.is_field_allowed(field) {
            self.conditions.push(format!("{} = ?", field));
            self.params.push(value.into());
        }
        self
    }

    pub fn add_comparison(&mut self, field: &str, op: &str, value: impl ToString) -> &mut Self {
        let valid_ops = ["=", "!=", "<", ">", "<=", ">=", "<>"];
        if self.is_field_allowed(field) && valid_ops.contains(&op) {
            self.conditions.push(format!("{} {} ?", field, op));
            self.params.push(value.to_string());
        }
        self
    }

    pub fn add_like(&mut self, field: &str, pattern: impl Into<String>) -> &mut Self {
        if self.is_field_allowed(field) {
            let p = pattern.into();
            let escaped = if p.contains('%') { p } else { format!("%{}%", p) };
            self.conditions.push(format!("{} LIKE ?", field));
            self.params.push(escaped);
        }
        self
    }

    pub fn add_is_null(&mut self, field: &str) -> &mut Self {
        if self.is_field_allowed(field) {
            self.conditions.push(format!("{} IS NULL", field));
        }
        self
    }

    pub fn add_is_not_null(&mut self, field: &str) -> &mut Self {
        if self.is_field_allowed(field) {
            self.conditions.push(format!("{} IS NOT NULL", field));
        }
        self
    }

    pub fn add_in_clause(&mut self, field: &str, values: &[impl ToString]) -> &mut Self {
        if self.is_field_allowed(field) && !values.is_empty() {
            let placeholders: Vec<_> = values.iter().map(|_| "?").collect();
            self.conditions.push(format!("{} IN ({})", field, placeholders.join(", ")));
            for v in values {
                self.params.push(v.to_string());
            }
        }
        self
    }

    pub fn add_between(&mut self, field: &str, from: impl ToString, to: impl ToString) -> &mut Self {
        if self.is_field_allowed(field) {
            self.conditions.push(format!("{} BETWEEN ? AND ?", field));
            self.params.push(from.to_string());
            self.params.push(to.to_string());
        }
        self
    }

    pub fn add_condition(&mut self, condition: &str, params: Vec<String>) -> &mut Self {
        self.conditions.push(condition.to_string());
        self.params.extend(params);
        self
    }

    pub fn build(&self) -> (String, Vec<String>) {
        let mut sql = format!("SELECT COUNT(*) as count FROM {}", self.table);
        
        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.conditions.join(" AND "));
        }
        
        (sql, self.params.clone())
    }

    pub fn sql(&self) -> String {
        self.build().0
    }

    pub fn params(&self) -> Vec<String> {
        self.build().1
    }

    pub fn has_conditions(&self) -> bool {
        !self.conditions.is_empty()
    }

    fn is_field_allowed(&self, field: &str) -> bool {
        match &self.whitelist {
            Some(wl) => wl.is_allowed(field),
            None => true,
        }
    }
}

// ==================== HELPER FUNCTIONS ====================

/// Генерация уникального имени файла
pub fn generate_unique_filename(original: &str) -> String {
    let ext = std::path::Path::new(original)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin");
    format!("{}_{}.{}", uuid::Uuid::new_v4(), chrono::Utc::now().timestamp(), ext)
}

/// Валидация размера файла
pub fn validate_file_size(size: usize, max_size: usize) -> Result<(), String> {
    if size > max_size {
        Err(format!("File size {} exceeds maximum allowed size {}", size, max_size))
    } else {
        Ok(())
    }
}

/// Валидация MIME типа
pub fn validate_mime_type(mime: &str, allowed: &[&str]) -> Result<(), String> {
    if allowed.iter().any(|&a| mime.starts_with(a)) {
        Ok(())
    } else {
        Err(format!("MIME type '{}' is not allowed", mime))
    }
}

// ==================== MAINTENANCE VALIDATOR ====================

/// Валидатор для записей обслуживания
pub struct MaintenanceValidator;

impl MaintenanceValidator {
    pub fn validate_date_format(date: &str) -> Result<(), String> {
        if date.len() >= 10 && date.chars().nth(4) == Some('-') && date.chars().nth(7) == Some('-') {
            Ok(())
        } else {
            Err(format!("Invalid date format: {}", date))
        }
    }

    pub fn validate_time_range(start: &str, end: &str) -> Result<(), String> {
        if start > end {
            Err(format!("Start date {} is after end date {}", start, end))
        } else {
            Ok(())
        }
    }
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_equipment_status_from_str() {
        assert_eq!(EquipmentStatus::from_str("available").unwrap(), EquipmentStatus::Available);
        assert_eq!(EquipmentStatus::from_str("in_use").unwrap(), EquipmentStatus::InUse);
        assert_eq!(EquipmentStatus::from_str("maintenance").unwrap(), EquipmentStatus::Maintenance);
    }

    #[test]
    fn test_equipment_status_display() {
        assert_eq!(EquipmentStatus::Available.to_string(), "available");
        assert_eq!(EquipmentStatus::InUse.to_string(), "in_use");
    }

    #[test]
    fn test_equipment_status_as_ref() {
        let status = EquipmentStatus::Maintenance;
        let s: &str = status.as_ref();
        assert_eq!(s, "maintenance");
    }

    #[test]
    fn test_maintenance_status_roundtrip() {
        for status in [
            MaintenanceStatus::Scheduled,
            MaintenanceStatus::InProgress,
            MaintenanceStatus::Completed,
            MaintenanceStatus::Cancelled,
        ] {
            let s = status.to_string();
            let parsed = MaintenanceStatus::from_str(&s).unwrap();
            assert_eq!(status, parsed);
        }
    }
}
