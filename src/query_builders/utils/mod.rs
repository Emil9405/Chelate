// src/query_builders/utils/mod.rs
//! Утилиты: валидация полей, экранирование, файлы

pub mod validators;

use std::collections::HashSet;
use crate::query_builders::filters::whitelist::{FieldConfig, FieldValidationError};

// ==================== ЭКРАНИРОВАНИЕ ====================

/// Escape `\`, `%` and `_` so the value matches literally in a LIKE pattern.
///
/// SQLite has no default LIKE escape character: the pattern only works in a
/// clause that names one, so build the SQL with [`like_sql`] / [`like_any`]
/// (`... LIKE ? ESCAPE '\'`). Without it `\_` is two literal characters and a
/// search for `Test_value` finds nothing.
pub fn escape_like_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// `field LIKE ? ESCAPE '\'` — the only LIKE form for escaped user input.
/// `field` is SQL text: pass a column name from code, never user input.
pub fn like_sql(field: &str) -> String {
    format!("{} LIKE ? ESCAPE '\\'", field)
}

/// Pattern for "contains": `%value%` with the value escaped.
pub fn like_contains(value: &str) -> String {
    format!("%{}%", escape_like_value(value))
}

/// Pattern for "starts with": `value%` with the value escaped.
pub fn like_prefix(value: &str) -> String {
    format!("{}%", escape_like_value(value))
}

/// Pattern for "ends with": `%value` with the value escaped.
pub fn like_suffix(value: &str) -> String {
    format!("%{}", escape_like_value(value))
}

/// `(a LIKE ? ESCAPE '\' OR b LIKE ? ESCAPE '\' ...)` and one "contains"
/// parameter per column. Column names that are not plain identifiers
/// (`name`, `b.batch_number`) are skipped, as a whitelist would; returns an
/// empty condition when the value is blank or no column is left.
pub fn like_any(fields: &[&str], value: &str) -> (String, Vec<String>) {
    let value = value.trim();
    let fields: Vec<&str> = fields
        .iter()
        .copied()
        .filter(|f| {
            let ok = is_safe_report_field(f);
            debug_assert!(ok, "like_any: '{}' is not a plain column name", f);
            ok
        })
        .collect();
    if value.is_empty() || fields.is_empty() {
        return (String::new(), Vec::new());
    }
    let pattern = like_contains(value);
    let sql = fields.iter().map(|f| like_sql(f)).collect::<Vec<_>>().join(" OR ");
    (format!("({})", sql), vec![pattern; fields.len()])
}

/// At most this many search words reach FTS5; the rest are dropped.
pub const MAX_FTS_TERMS: usize = 16;

/// User input → FTS5 MATCH expression (bind it as a parameter).
///
/// Each whitespace-separated word becomes one quoted phrase of its
/// letter/digit runs, the last one a prefix: `64-17-5` → `"64 17 5"*`,
/// `sodium chloride` → `"sodium"* "chloride"*` (both words, any order).
/// This mirrors the `unicode61` tokenizer, which splits stored text on
/// punctuation too, so a CAS number or "95-97%" finds its row; joining the
/// pieces (`64175*`) would find nothing.
///
/// Only letters and digits survive and every term is quoted, so FTS5
/// operators (`AND`, `OR`, `NOT`, `NEAR`, `column:`, `^`, `*`, `(`) can't
/// appear: a bare `NOT*` is a syntax error that used to fail the request.
/// Returns an empty string when nothing searchable is left.
pub fn sanitize_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter_map(|word| {
            let tokens: Vec<&str> = word
                .split(|c: char| !c.is_alphanumeric())
                .filter(|t| !t.is_empty())
                .collect();
            (!tokens.is_empty()).then(|| format!("\"{}\"*", tokens.join(" ")))
        })
        .take(MAX_FTS_TERMS)
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
        assert_eq!(escape_like_value("a\\b"), "a\\\\b");
        // SQLite LIKE has no [..] classes: '[' is an ordinary character
        assert_eq!(escape_like_value("test[value"), "test[value");
    }

    #[test]
    fn test_like_helpers() {
        assert_eq!(like_sql("b.batch_number"), "b.batch_number LIKE ? ESCAPE '\\'");
        assert_eq!(like_contains("50%"), "%50\\%%");
        assert_eq!(like_prefix("A_1"), "A\\_1%");
        assert_eq!(like_suffix("x"), "%x");

        let (sql, params) = like_any(&["name", "b.cat_number"], " 1.05426 ");
        assert_eq!(sql, "(name LIKE ? ESCAPE '\\' OR b.cat_number LIKE ? ESCAPE '\\')");
        assert_eq!(params, vec!["%1.05426%".to_string(), "%1.05426%".to_string()]);

        assert_eq!(like_any(&["name"], "   "), (String::new(), Vec::new()));
        assert_eq!(like_any(&[], "x"), (String::new(), Vec::new()));
    }

    /// The patterns must find exactly the literal text in SQLite.
    #[tokio::test]
    async fn test_like_patterns_in_sqlite() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE t (v TEXT)").execute(&pool).await.unwrap();
        for v in ["Test_value", "TestXvalue", "50% w/w", "500 mL", "a\\b", "[x]"] {
            sqlx::query("INSERT INTO t VALUES (?)").bind(v).execute(&pool).await.unwrap();
        }
        let find = |q: &'static str| {
            let pool = pool.clone();
            async move {
                let (sql, params) = like_any(&["v"], q);
                let sql = format!("SELECT v FROM t WHERE {} ORDER BY v", sql);
                let mut query = sqlx::query_scalar::<_, String>(&sql);
                for p in &params { query = query.bind(p); }
                query.fetch_all(&pool).await.unwrap()
            }
        };
        assert_eq!(find("Test_value").await, vec!["Test_value"]);
        assert_eq!(find("50%").await, vec!["50% w/w"]);
        assert_eq!(find("a\\b").await, vec!["a\\b"]);
        assert_eq!(find("[x").await, vec!["[x]"]);
        assert_eq!(find("%").await, vec!["50% w/w"]);
        assert_eq!(find("_").await, vec!["Test_value"]);
    }

    #[test]
    fn test_sanitize_fts_query() {
        assert_eq!(sanitize_fts_query("hello world"), "\"hello\"* \"world\"*");
        assert_eq!(sanitize_fts_query("hello (world)"), "\"hello\"* \"world\"*");
        // Punctuation inside a word splits it the way unicode61 splits stored text
        assert_eq!(sanitize_fts_query("64-17-5"), "\"64 17 5\"*");
        assert_eq!(sanitize_fts_query("test*query"), "\"test query\"*");
        assert_eq!(sanitize_fts_query("a+b-c"), "\"a b c\"*");
        assert_eq!(sanitize_fts_query("test.value"), "\"test value\"*");
        assert_eq!(sanitize_fts_query("N,N-Dimethylformamide"), "\"N N Dimethylformamide\"*");
        assert_eq!(sanitize_fts_query("H2SO4!"), "\"H2SO4\"*");
        assert_eq!(sanitize_fts_query("Этанол"), "\"Этанол\"*");
        // Nothing searchable
        assert_eq!(sanitize_fts_query(""), "");
        assert_eq!(sanitize_fts_query("!"), "");
        assert_eq!(sanitize_fts_query("@#$% \"\" ()"), "");
        // FTS5 syntax can't get through: operators are quoted terms
        assert_eq!(sanitize_fts_query("NOT"), "\"NOT\"*");
        assert_eq!(sanitize_fts_query("acid OR NEAR"), "\"acid\"* \"OR\"* \"NEAR\"*");
        assert_eq!(sanitize_fts_query("name:x ^y"), "\"name x\"* \"y\"*");
        assert_eq!(sanitize_fts_query("\"a\" OR \"b"), "\"a\"* \"OR\"* \"b\"*");
        assert_eq!(sanitize_fts_query("name='x'; DROP TABLE--"), "\"name x\"* \"DROP\"* \"TABLE\"*");
        // Term cap
        let long = "w ".repeat(100);
        assert_eq!(sanitize_fts_query(&long).matches('*').count(), MAX_FTS_TERMS);
    }

    /// Every query must parse in FTS5 and find what a person expects.
    #[tokio::test]
    async fn test_fts_query_in_sqlite() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE VIRTUAL TABLE f USING fts5(name, cas_number, formula, \
             tokenize='unicode61 remove_diacritics 1')",
        ).execute(&pool).await.unwrap();
        for (n, c, f) in [
            ("Ethanol absolute", "64-17-5", "C2H6O"),
            ("Sodium chloride", "7647-14-5", "NaCl"),
            ("Sulfuric acid 95-97%", "7664-93-9", "H2SO4"),
            ("N,N-Dimethylformamide", "68-12-2", "C3H7NO"),
            ("Not a real reagent", "", ""),
        ] {
            sqlx::query("INSERT INTO f VALUES (?, ?, ?)")
                .bind(n).bind(c).bind(f).execute(&pool).await.unwrap();
        }
        let find = |q: &'static str| {
            let pool = pool.clone();
            async move {
                sqlx::query_scalar::<_, String>("SELECT name FROM f WHERE f MATCH ? ORDER BY name")
                    .bind(sanitize_fts_query(q))
                    .fetch_all(&pool).await
                    .unwrap_or_else(|e| panic!("{:?} failed: {}", q, e))
            }
        };
        assert_eq!(find("64-17-5").await, vec!["Ethanol absolute"]);
        assert_eq!(find("64-17").await, vec!["Ethanol absolute"]);
        assert_eq!(find("7647").await, vec!["Sodium chloride"]);
        assert_eq!(find("chloride sod").await, vec!["Sodium chloride"]);
        assert_eq!(find("95-97%").await, vec!["Sulfuric acid 95-97%"]);
        assert_eq!(find("h2so4").await, vec!["Sulfuric acid 95-97%"]);
        assert_eq!(find("N,N-Dimethyl").await, vec!["N,N-Dimethylformamide"]);
        assert_eq!(find("NOT").await, vec!["Not a real reagent"]);
        for q in ["OR", "AND", "NEAR", "acid OR", "a\"b", "x:y", "^a", "(a", "NEAR(a b)", "a*b-"] {
            find(q).await; // parses
        }
        // Nothing searchable: callers skip the MATCH
        assert_eq!(sanitize_fts_query("\" * ( ) -"), "");
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
