// src/query_builders/utils/mod.rs
//! Утилиты: валидация полей, экранирование, файлы

pub mod validators;

use std::collections::HashSet;
use crate::query_builders::filters::whitelist::{FieldConfig, FieldValidationError};

// ==================== ЭКРАНИРОВАНИЕ ====================

/// Экранирование спецсимволов LIKE
pub fn escape_like_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
        .replace('[', "\\[")
}

/// Sanitize user input for FTS5 MATCH — whitelist approach.
/// Keeps only alphanumeric + space. Adds * for prefix search.
/// Returns ready-to-use FTS5 MATCH string, or empty if no valid tokens.
pub fn sanitize_fts_query(query: &str) -> String {
    query
        .chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|word| format!("{}*", word))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Backward compat alias
#[deprecated(note = "Use sanitize_fts_query instead")]
pub fn escape_fts_query(query: &str) -> String {
    sanitize_fts_query(query)
}
// ==================== ВАЛИДАЦИЯ ПОЛЕЙ ====================

/// Детальная валидация имени поля
pub fn validate_field_name_detailed(field: &str, config: &FieldConfig) -> Result<(), FieldValidationError> {
    if field.is_empty() {
        return Err(FieldValidationError::Empty);
    }
    if field.len() < config.min_field_length {
        return Err(FieldValidationError::TooShort(config.min_field_length));
    }
    if field.len() > config.max_field_length {
        return Err(FieldValidationError::TooLong(config.max_field_length));
    }
    if config.reserved_words.contains(&field.to_uppercase()) {
        return Err(FieldValidationError::ReservedWord(field.to_string()));
    }
    if field.contains("__") {
        return Err(FieldValidationError::ConsecutiveUnderscores);
    }

    let mut prev_char = '\0';
    for (i, c) in field.chars().enumerate() {
        if i == 0 {
            if !(config.allow_leading_underscore && c == '_') && !c.is_ascii_alphabetic() {
                return Err(FieldValidationError::InvalidStart);
            }
        } else {
            let valid = c.is_ascii_alphanumeric() || c == '_'
                || (c == '.' && config.allow_dot)
                || ((c == '[' || c == ']') && config.allow_brackets);
            if !valid {
                return Err(FieldValidationError::InvalidCharacter(c));
            }
        }
        prev_char = c;
    }
    
    if prev_char == '_' && !config.allow_leading_underscore {
        return Err(FieldValidationError::InvalidFormat("Should not end with underscore".to_string()));
    }
    
    Ok(())
}

/// Проверка безопасности имени поля (базовая конфигурация)
#[inline]
pub fn is_safe_field_name(field: &str) -> bool {
    validate_field_name_detailed(field, &FieldConfig::default()).is_ok()
}

/// Валидация имени поля с возвратом ошибки
#[inline]
pub fn validate_field_name(field: &str) -> Result<(), String> {
    validate_field_name_detailed(field, &FieldConfig::default())
        .map_err(|e: FieldValidationError| e.to_string())
}

/// Проверка безопасности имени таблицы (разрешены точки)
#[inline]
pub fn is_safe_table_name(table: &str) -> bool {
    validate_field_name_detailed(table, &FieldConfig::for_table_names()).is_ok()
}

/// Проверка безопасности поля для отчётов (разрешены точки для алиасов)
#[inline]
pub fn is_safe_report_field(field: &str) -> bool {
    validate_field_name_detailed(field, &FieldConfig::for_reports()).is_ok()
}

/// Проверка валидности порядка сортировки
#[inline]
pub fn is_valid_sort_order(order: &str) -> bool {
    matches!(order.to_uppercase().as_str(), "ASC" | "DESC")
}

/// Нормализация порядка сортировки
#[inline]
pub fn normalize_sort_order(order: &str) -> &'static str {
    match order.to_uppercase().as_str() {
        "ASC" => "ASC",
        _ => "DESC",
    }
}

// ==================== УТИЛИТЫ ДЛЯ ФАЙЛОВ ====================

/// Генерация уникального имени файла
pub fn generate_unique_filename(original: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    
    let extension = std::path::Path::new(original)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin");
    
    let base_name: String = original
        .trim_end_matches(&format!(".{}", extension))
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(50)
        .collect();
    
    format!("{}_{}.{}", base_name, timestamp, extension)
}

/// Валидация размера файла
pub fn validate_file_size(size: usize, max_size_mb: usize) -> Result<(), String> {
    let max_bytes = max_size_mb * 1024 * 1024;
    if size > max_bytes {
        return Err(format!(
            "File size ({:.2} MB) exceeds maximum allowed ({} MB)",
            size as f64 / (1024.0 * 1024.0),
            max_size_mb
        ));
    }
    Ok(())
}

/// Валидация MIME-типа
pub fn validate_mime_type(mime_type: &str, allowed: &[&str]) -> Result<(), String> {
    if !allowed.contains(&mime_type) {
        return Err(format!(
            "MIME type '{}' is not allowed. Allowed types: {:?}",
            mime_type, allowed
        ));
    }
    Ok(())
}

// ==================== КОМНАТЫ ПО УМОЛЧАНИЮ ====================

/// Список комнат по умолчанию (id, name, description, capacity, color)
pub fn get_default_rooms() -> Vec<(&'static str, &'static str, &'static str, i32, &'static str)> {
    vec![
        ("room-101", "Lab 101", "Main chemistry laboratory", 30, "#667eea"),
        ("room-102", "Lab 102", "Organic chemistry lab", 25, "#48bb78"),
        ("room-103", "Lab 103", "Analytical chemistry lab", 20, "#ed8936"),
        ("room-104", "Prep Room", "Preparation and storage", 10, "#9f7aea"),
        ("room-105", "Seminar Room", "For theoretical classes", 40, "#38b2ac"),
    ]
}

// ==================== ТЕСТЫ ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_like_value() {
        assert_eq!(escape_like_value("test"), "test");
        assert_eq!(escape_like_value("test%value"), "test\\%value");
        assert_eq!(escape_like_value("test_value"), "test\\_value");
        assert_eq!(escape_like_value("test[value"), "test\\[value");
    }
#[test]
fn test_sanitize_fts_query() {
    assert_eq!(sanitize_fts_query("hello world"), "hello* world*");
    assert_eq!(sanitize_fts_query("hello (world)"), "hello* world*");
    assert_eq!(sanitize_fts_query("test*query"), "testquery*");
    assert_eq!(sanitize_fts_query("a+b-c"), "abc*");
    // Previously crashing characters:
    assert_eq!(sanitize_fts_query("!"), "");
    assert_eq!(sanitize_fts_query("@#$%"), "");
    assert_eq!(sanitize_fts_query("H2SO4!"), "H2SO4*");
    assert_eq!(sanitize_fts_query("test.value"), "testvalue*");
    assert_eq!(sanitize_fts_query("name='x'; DROP TABLE--"), "namex* DROP* TABLE*");
}

    #[test]
    fn test_field_validation() {
        assert!(is_safe_field_name("status"));
        assert!(is_safe_field_name("created_at"));
        assert!(is_safe_field_name("field123"));
        
        assert!(!is_safe_field_name("SELECT"));
        assert!(!is_safe_field_name("DROP"));
        assert!(!is_safe_field_name(""));
        assert!(!is_safe_field_name("field.name")); // Точки запрещены по умолчанию
        assert!(!is_safe_field_name("123field")); // Не может начинаться с цифры
    }

    #[test]
    fn test_table_name_validation() {
        assert!(is_safe_table_name("experiments"));
        assert!(is_safe_table_name("schema.table")); // Точки разрешены
        assert!(!is_safe_table_name("DROP"));
    }

    #[test]
    fn test_report_field_validation() {
        assert!(is_safe_report_field("b.quantity"));
        assert!(is_safe_report_field("r.name"));
        assert!(!is_safe_report_field("SELECT"));
    }

    #[test]
    fn test_sort_order() {
        assert!(is_valid_sort_order("ASC"));
        assert!(is_valid_sort_order("desc"));
        assert!(!is_valid_sort_order("RANDOM"));

        assert_eq!(normalize_sort_order("asc"), "ASC");
        assert_eq!(normalize_sort_order("DESC"), "DESC");
        assert_eq!(normalize_sort_order("invalid"), "DESC");
    }

    #[test]
    fn test_generate_unique_filename() {
        let filename = generate_unique_filename("test document.pdf");
        assert!(filename.ends_with(".pdf"));
        assert!(filename.contains("_"));
        assert!(!filename.contains(" "));
    }

    #[test]
    fn test_validate_file_size() {
        assert!(validate_file_size(1024, 1).is_ok());
        assert!(validate_file_size(2 * 1024 * 1024, 1).is_err());
    }

    #[test]
    fn test_validate_mime_type() {
        let allowed = &["image/jpeg", "image/png"];
        assert!(validate_mime_type("image/jpeg", allowed).is_ok());
        assert!(validate_mime_type("application/pdf", allowed).is_err());
    }
}
