// src/query_builders/filters/whitelist.rs
//! Белый список полей для защиты от SQL-инъекций через имена колонок

use std::collections::HashSet;

// ==================== КОНФИГУРАЦИЯ ВАЛИДАЦИИ ====================

#[derive(Debug, Clone)]
pub struct FieldConfig {
    pub max_field_length: usize,
    pub min_field_length: usize,
    pub reserved_words: HashSet<String>,
    pub allow_dot: bool,
    pub allow_brackets: bool,
    pub allow_leading_underscore: bool,
}

impl Default for FieldConfig {
    fn default() -> Self {
        let reserved: HashSet<String> = [
            "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER",
            "UNION", "JOIN", "ORDER", "GROUP", "HAVING", "EXISTS", "AND", "OR", "NOT", "NULL", "AS",
            "TABLE", "INDEX", "VIEW", "TRIGGER", "PROCEDURE", "FUNCTION", "INTO", "VALUES", "SET",
            "EXEC", "EXECUTE", "DECLARE", "GRANT", "REVOKE", "COMMIT", "ROLLBACK", "SAVEPOINT",
            "TRUNCATE", "REPLACE", "MERGE", "CALL", "EXPLAIN", "DESCRIBE", "SHOW", "USE", "BEGIN",
        ].iter().map(|s| s.to_string()).collect();

        Self {
            max_field_length: 64,
            min_field_length: 1,
            reserved_words: reserved,
            allow_dot: false,
            allow_brackets: false,
            allow_leading_underscore: false,
        }
    }
}

impl FieldConfig {
    pub fn for_table_names() -> Self {
        Self { max_field_length: 128, allow_dot: true, ..Default::default() }
    }

    pub fn strict() -> Self {
        Self { max_field_length: 32, min_field_length: 2, ..Default::default() }
    }

    pub fn for_reports() -> Self {
        Self { max_field_length: 64, allow_dot: true, ..Default::default() }
    }
}

// ==================== ОШИБКИ ВАЛИДАЦИИ ====================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValidationError {
    Empty,
    TooShort(usize),
    TooLong(usize),
    InvalidStart,
    InvalidCharacter(char),
    ReservedWord(String),
    InvalidFormat(String),
    ConsecutiveUnderscores,
    NotInWhitelist(String),
}

impl std::fmt::Display for FieldValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "Field name cannot be empty"),
            Self::TooShort(min) => write!(f, "Field name too short (min: {})", min),
            Self::TooLong(max) => write!(f, "Field name too long (max: {})", max),
            Self::InvalidStart => write!(f, "Field name must start with a letter"),
            Self::InvalidCharacter(c) => write!(f, "Invalid character: '{}'", c),
            Self::ReservedWord(w) => write!(f, "Reserved SQL word: '{}'", w),
            Self::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            Self::ConsecutiveUnderscores => write!(f, "Cannot have consecutive underscores"),
            Self::NotInWhitelist(field) => write!(f, "Field '{}' not in whitelist", field),
        }
    }
}

impl std::error::Error for FieldValidationError {}

// ==================== БЕЛЫЙ СПИСОК ====================

#[derive(Debug, Clone)]
pub struct FieldWhitelist {
    allowed_fields: HashSet<String>,
    config: FieldConfig,
}

impl FieldWhitelist {
    pub fn new(fields: &[&str]) -> Self {
        Self {
            allowed_fields: fields.iter().map(|s| s.to_string()).collect(),
            config: FieldConfig::default(),
        }
    }

    pub fn with_config(fields: &[&str], config: FieldConfig) -> Self {
        Self {
            allowed_fields: fields.iter().map(|s| s.to_string()).collect(),
            config,
        }
    }

    // ==================== ПРЕДУСТАНОВЛЕННЫЕ WHITELIST ====================

    pub fn for_reagents() -> Self {
        Self::new(&[
            "id", "name", "formula", "cas_number", "manufacturer", "molecular_weight",
            "physical_state", "description", "status", "visibility",
            "created_by", "updated_by", "created_at",
            "updated_at", "total_quantity", "reserved_quantity", "available_quantity",
            "batches_count", "total_display",
        ])
    }

    pub fn for_experiments() -> Self {
        Self::new(&[
            "id", "title", "description", "experiment_date", "instructor", "student_group",
            "location", "status", "protocol", "start_date", "end_date", "results", "notes",
            "experiment_type", "room_id", "created_by", "updated_by", "created_at", "updated_at",
        ])
    }

    pub fn for_rooms() -> Self {
        Self::new(&[
            "id", "name", "description", "capacity", "color", "status",
            "created_by", "updated_by", "created_at", "updated_at",
        ])
    }

    pub fn for_batches() -> Self {
        Self::new(&[
            "id", "reagent_id", "batch_number", "quantity", "original_quantity", "reserved_quantity",
            "unit", "expiry_date", "supplier", "manufacturer", "received_date", "status", "location",
            "notes", "cat_number", "created_by", "updated_by", "created_at", "updated_at",
        ])
    }

    pub fn for_equipment() -> Self {
        Self::new(&[
            "id", "name", "model", "serial_number", "manufacturer", "description", "type_",
            "status", "location", "purchase_date", "warranty_until", "last_maintenance",
            "next_maintenance", "maintenance_interval_days", "notes",
            "created_by", "updated_by", "created_at", "updated_at",
        ])
    }

    pub fn for_equipment_parts() -> Self {
        Self::new(&[
            "id", "equipment_id", "name", "part_number", "quantity", "status", "description",
            "last_replacement", "next_replacement", "replacement_interval_days", "image_path", "notes",
            "created_by", "updated_by", "created_at", "updated_at",
        ])
    }

    pub fn for_equipment_maintenance() -> Self {
        Self::new(&[
            "id", "equipment_id", "maintenance_type", "scheduled_date", "completed_date",
            "performed_by", "status", "description", "cost", "notes", "part_id",
            "created_by", "updated_by", "created_at", "updated_at",
        ])
    }

    pub fn for_equipment_files() -> Self {
        Self::new(&[
            "id", "equipment_id", "file_type", "filename", "original_filename", "file_path",
            "file_size", "mime_type", "description", "uploaded_by", "uploaded_at",
        ])
    }

    pub fn for_reports() -> Self {
        Self::with_config(&[
            "id", "reagent_id", "reagent_name", "batch_number", "cat_number", "quantity",
            "original_quantity", "reserved_quantity", "unit", "expiry_date", "supplier",
            "manufacturer", "received_date", "status", "notes", "days_until_expiry",
            "expiration_status", "created_at", "updated_at",
            // Container-aware поля (вычисляются из CTE container_stats)
            "container_count", "opened_count", "placed_count", "unplaced_count",
            "location_summary", "room_names",
            // Алиасы с таблицами
            "b.id", "b.reagent_id", "b.batch_number", "b.cat_number", "b.quantity",
            "b.original_quantity", "b.reserved_quantity", "b.unit", "b.expiry_date",
            "b.supplier", "b.manufacturer", "b.received_date", "b.status",
            "b.notes", "b.created_at", "b.updated_at",
            "r.name", "r.id", "r.formula", "r.cas_number",
        ], FieldConfig::for_reports())
    }

    // ==================== МЕТОДЫ ====================

    /// Проверка, разрешено ли поле
    #[inline]
    pub fn is_allowed(&self, field: &str) -> bool {
        crate::query_builders::utils::validate_field_name_detailed(field, &self.config).is_ok()
            && self.allowed_fields.contains(field)
    }

    /// Валидация поля с возвратом ошибки
    pub fn validate(&self, field: &str) -> Result<(), String> {
        crate::query_builders::utils::validate_field_name_detailed(field, &self.config)
            .map_err(|e| format!("Field '{}': {}", field, e))?;
        
        if !self.allowed_fields.contains(field) {
            return Err(format!("Field '{}' not in whitelist", field));
        }
        Ok(())
    }

    /// Фильтрация списка полей - возвращает только разрешённые
    pub fn filter_fields<'a>(&self, fields: &[&'a str]) -> Vec<&'a str> {
        fields.iter().filter(|&&f| self.is_allowed(f)).copied().collect()
    }

    /// Добавление поля в whitelist
    pub fn add_field(&mut self, field: &str) {
        self.allowed_fields.insert(field.to_string());
    }

    /// Удаление поля из whitelist
    pub fn remove_field(&mut self, field: &str) {
        self.allowed_fields.remove(field);
    }

    /// Получение всех разрешённых полей
    pub fn get_allowed_fields(&self) -> &HashSet<String> {
        &self.allowed_fields
    }

    /// Конфигурация
    pub fn config(&self) -> &FieldConfig {
        &self.config
    }
}

// ==================== ТЕСТЫ ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitelist_basic() {
        let whitelist = FieldWhitelist::new(&["id", "name", "status"]);
        assert!(whitelist.is_allowed("id"));
        assert!(whitelist.is_allowed("name"));
        assert!(!whitelist.is_allowed("password"));
        assert!(!whitelist.is_allowed("SELECT"));
    }

    #[test]
    fn test_whitelist_for_batches() {
        let whitelist = FieldWhitelist::for_batches();
        assert!(whitelist.is_allowed("quantity"));
        assert!(whitelist.is_allowed("status"));
        assert!(whitelist.is_allowed("expiry_date"));
        assert!(!whitelist.is_allowed("password"));
        assert!(!whitelist.is_allowed("DROP"));
    }

    #[test]
    fn test_whitelist_validate() {
        let whitelist = FieldWhitelist::for_batches();
        assert!(whitelist.validate("quantity").is_ok());
        assert!(whitelist.validate("password").is_err());
    }

    #[test]
    fn test_whitelist_filter_fields() {
        let whitelist = FieldWhitelist::new(&["id", "name"]);
        let fields = vec!["id", "name", "password", "DROP"];
        let filtered = whitelist.filter_fields(&fields);
        assert_eq!(filtered, vec!["id", "name"]);
    }

    #[test]
    fn test_whitelist_modify() {
        let mut whitelist = FieldWhitelist::new(&["id"]);
        assert!(!whitelist.is_allowed("new_field"));
        whitelist.add_field("new_field");
        assert!(whitelist.is_allowed("new_field"));
        whitelist.remove_field("new_field");
        assert!(!whitelist.is_allowed("new_field"));
    }

    #[test]
    fn test_for_reports_with_aliases() {
        let whitelist = FieldWhitelist::for_reports();
        assert!(whitelist.is_allowed("b.quantity"));
        assert!(whitelist.is_allowed("r.name"));
        assert!(!whitelist.is_allowed("x.unknown"));
    }
}
