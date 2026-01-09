// src/report_handlers.rs
//! Обработчики для системы кастомных репортов

use actix_web::{web, HttpResponse, HttpRequest};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::AppState;
use crate::error::{ApiError, ApiResult};
use crate::handlers::ApiResponse;
use crate::query_builders::{
    FieldWhitelist, ReportConfig, ReportFilter, ReportColumn,
    ComparisonOperator, ReportFilterValue,
};

// ==================== SECURITY CONSTANTS ====================

/// Разрешённые поля для сортировки - защита от SQL-инъекций
const ALLOWED_SORT_FIELDS: &[&str] = &[
    "id", "reagent_id", "reagent_name", "batch_number", "cat_number",
    "quantity", "original_quantity", "reserved_quantity", "unit",
    "expiry_date", "supplier", "manufacturer", "received_date",
    "status", "location", "created_at", "updated_at", "days_until_expiry",
    "expiration_status",
];

/// Валидация поля сортировки
fn validate_sort_field(field: &str) -> Option<&'static str> {
    ALLOWED_SORT_FIELDS.iter()
        .find(|&&allowed| allowed == field)
        .copied()
}

/// Экранирование спецсимволов LIKE для предотвращения LIKE-инъекций
fn escape_like_pattern(pattern: &str) -> String {
    pattern
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Экранирование CSV-полей (обработка запятых, кавычек и переносов строк)
fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

// ==================== RESPONSE STRUCTURES ====================

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BatchReportRow {
    pub id: String,
    pub reagent_id: String,
    pub reagent_name: String,
    pub batch_number: String,
    pub cat_number: Option<String>,
    pub quantity: f64,
    pub original_quantity: f64,
    pub reserved_quantity: f64,
    pub unit: String,
    pub expiry_date: Option<DateTime<Utc>>,
    pub supplier: Option<String>,
    pub manufacturer: Option<String>,
    pub received_date: DateTime<Utc>,
    pub status: String,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub days_until_expiry: Option<i64>,
    pub expiration_status: String,
}

#[derive(Debug, Serialize)]
pub struct ReportMetadata {
    pub name: String,
    pub description: Option<String>,
    pub preset: String,
    pub total_items: i64,
    pub generated_at: DateTime<Utc>,
    pub columns: Vec<ReportColumn>,
}

#[derive(Debug, Serialize)]
pub struct ReportResponse {
    pub metadata: ReportMetadata,
    pub data: Vec<BatchReportRow>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub total_pages: i64,
}

#[derive(Debug, Serialize)]
pub struct AvailablePreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub default_params: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct AvailableField {
    pub field: String,
    pub label: String,
    pub data_type: String,
    pub operators: Vec<String>,
    pub values: Option<Vec<String>>,
}

// ==================== REQUEST STRUCTURES ====================

#[derive(Debug, Deserialize)]
pub struct GenerateReportRequest {
    pub preset: Option<String>,
    pub preset_params: Option<serde_json::Map<String, serde_json::Value>>,
    pub filters: Option<Vec<ReportFilterRequest>>,
    pub columns: Option<Vec<String>>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReportFilterRequest {
    pub field: String,
    pub operator: String,
    pub value: serde_json::Value,
}

impl ReportFilterRequest {
    pub fn to_report_filter(&self) -> Option<ReportFilter> {
        let operator = match self.operator.as_str() {
            "eq" | "=" => ComparisonOperator::Eq,
            "ne" | "!=" => ComparisonOperator::Ne,
            "gt" | ">" => ComparisonOperator::Gt,
            "gte" | ">=" => ComparisonOperator::Gte,
            "lt" | "<" => ComparisonOperator::Lt,
            "lte" | "<=" => ComparisonOperator::Lte,
            "like" | "contains" => ComparisonOperator::Like,
            "in" => ComparisonOperator::In,
            "not_in" => ComparisonOperator::NotIn,
            "is_null" => ComparisonOperator::IsNull,
            "is_not_null" => ComparisonOperator::IsNotNull,
            _ => return None,
        };

        let value = match &self.value {
            serde_json::Value::String(s) => {
                if matches!(operator, ComparisonOperator::Gt | ComparisonOperator::Gte | 
                                      ComparisonOperator::Lt | ComparisonOperator::Lte) {
                    if let Ok(n) = s.parse::<f64>() {
                        ReportFilterValue::Number(n)
                    } else {
                        ReportFilterValue::Exact(s.clone())
                    }
                } else if operator == ComparisonOperator::Like {
                    ReportFilterValue::Contains(s.clone())
                } else {
                    if let Ok(n) = s.parse::<f64>() {
                        if s.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
                            ReportFilterValue::Number(n)
                        } else {
                            ReportFilterValue::Exact(s.clone())
                        }
                    } else {
                        ReportFilterValue::Exact(s.clone())
                    }
                }
            },
            serde_json::Value::Number(n) => {
                ReportFilterValue::Number(n.as_f64().unwrap_or(0.0))
            },
            serde_json::Value::Array(arr) => {
                let list: Vec<String> = arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                ReportFilterValue::List(list)
            },
            serde_json::Value::Object(obj) => {
                if obj.contains_key("from") || obj.contains_key("to") {
                    ReportFilterValue::Range {
                        from: obj.get("from").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        to: obj.get("to").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    }
                } else {
                    return None;
                }
            },
            serde_json::Value::Null => ReportFilterValue::Null,
            _ => return None,
        };

        Some(ReportFilter {
            field: self.field.clone(),
            operator,
            value,
        })
    }
}

// ==================== HELPER FUNCTIONS ====================

fn build_report_config(request: &GenerateReportRequest) -> ReportConfig {
    let preset = request.preset.as_deref().unwrap_or("all_batches");
    
    let mut config = match preset {
        "low_stock" => {
            let threshold = request.preset_params.as_ref()
                .and_then(|p| p.get("threshold"))
                .and_then(|v| v.as_f64())
                .unwrap_or(10.0);
            ReportConfig::low_stock(threshold)
        },
        "expiring_soon" => {
            let days = request.preset_params.as_ref()
                .and_then(|p| p.get("days"))
                .and_then(|v| v.as_i64())
                .unwrap_or(30);
            ReportConfig::expiring_soon(days)
        },
        "expired" => ReportConfig::expired(),
        _ => ReportConfig::all_batches(),
    };

    config.preset = preset.to_string();
    config.name = match preset {
        "low_stock" => "Low Stock Report".to_string(),
        "expiring_soon" => "Expiring Soon Report".to_string(),
        "expired" => "Expired Items Report".to_string(),
        _ => "All Batches Report".to_string(),
    };

    // Добавляем кастомные фильтры
    if let Some(ref filters) = request.filters {
        for filter_req in filters {
            if let Some(filter) = filter_req.to_report_filter() {
                config.filters.push(filter);
            }
        }
    }

    // ✅ ИСПРАВЛЕНО: Валидация сортировки через whitelist
    if let Some(ref sort_by) = request.sort_by {
        if validate_sort_field(sort_by).is_some() {
            config.sort_by = Some(sort_by.clone());
        }
        // Если поле невалидно - используем дефолт (created_at)
    }
    if let Some(ref sort_order) = request.sort_order {
        config.sort_order = sort_order.to_uppercase();
    }

    config
}

fn build_filter_sql(config: &ReportConfig, whitelist: &FieldWhitelist) -> (String, Vec<String>) {
    let (where_clause, params) = config.build_where_clause(whitelist);
    (where_clause, params)
}

// ==================== BASE QUERY ====================

const BASE_REPORT_QUERY: &str = r#"
    WITH batch_data AS (
        SELECT 
            b.id, b.reagent_id, r.name as reagent_name, b.batch_number, b.cat_number,
            b.quantity, b.original_quantity, b.reserved_quantity, b.unit, b.expiry_date,
            b.supplier, b.manufacturer, b.received_date, b.status, b.location, b.notes,
            b.created_at, b.updated_at,
            CASE WHEN b.expiry_date IS NULL THEN NULL
                 ELSE CAST((julianday(b.expiry_date) - julianday('now')) AS INTEGER)
            END as days_until_expiry,
            CASE WHEN b.expiry_date IS NULL THEN 'unknown'
                 WHEN julianday(b.expiry_date) < julianday('now') THEN 'expired'
                 WHEN julianday(b.expiry_date) - julianday('now') <= 7 THEN 'critical'
                 WHEN julianday(b.expiry_date) - julianday('now') <= 30 THEN 'warning'
                 ELSE 'ok'
            END as expiration_status
        FROM batches b
        JOIN reagents r ON b.reagent_id = r.id
    )
    SELECT * FROM batch_data
"#;

// ==================== HANDLERS ====================

pub async fn get_report_presets(
    _app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let presets = vec![
        AvailablePreset {
            id: "all_batches".to_string(),
            name: "All Batches".to_string(),
            description: "Complete list of all batches".to_string(),
            default_params: serde_json::json!({}),
        },
        AvailablePreset {
            id: "low_stock".to_string(),
            name: "Low Stock Items".to_string(),
            description: "Batches with quantity below threshold".to_string(),
            default_params: serde_json::json!({ "threshold": 10 }),
        },
        AvailablePreset {
            id: "expiring_soon".to_string(),
            name: "Expiring Soon".to_string(),
            description: "Batches expiring within specified days".to_string(),
            default_params: serde_json::json!({ "days": 30 }),
        },
        AvailablePreset {
            id: "expired".to_string(),
            name: "Expired Items".to_string(),
            description: "Batches that have expired".to_string(),
            default_params: serde_json::json!({}),
        },
    ];

    Ok(HttpResponse::Ok().json(ApiResponse::success(presets)))
}

pub async fn get_report_fields(
    _app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let fields = vec![
        AvailableField {
            field: "status".to_string(),
            label: "Status".to_string(),
            data_type: "enum".to_string(),
            operators: vec!["eq".to_string(), "ne".to_string(), "in".to_string()],
            // ✅ ИСПРАВЛЕНО: добавлен low_stock
            values: Some(vec![
                "available".to_string(), 
                "low_stock".to_string(),
                "reserved".to_string(), 
                "expired".to_string(), 
                "depleted".to_string()
            ]),
        },
        AvailableField {
            field: "quantity".to_string(),
            label: "Quantity".to_string(),
            data_type: "number".to_string(),
            operators: vec!["eq".to_string(), "gt".to_string(), "gte".to_string(), "lt".to_string(), "lte".to_string()],
            values: None,
        },
        AvailableField {
            field: "expiry_date".to_string(),
            label: "Expiry Date".to_string(),
            data_type: "date".to_string(),
            operators: vec!["eq".to_string(), "gt".to_string(), "lt".to_string(), "is_null".to_string()],
            values: None,
        },
        AvailableField {
            field: "days_until_expiry".to_string(),
            label: "Days Until Expiry".to_string(),
            data_type: "number".to_string(),
            operators: vec!["eq".to_string(), "gt".to_string(), "gte".to_string(), "lt".to_string(), "lte".to_string()],
            values: None,
        },
        AvailableField {
            field: "location".to_string(),
            label: "Location".to_string(),
            data_type: "text".to_string(),
            operators: vec!["eq".to_string(), "like".to_string(), "is_null".to_string()],
            values: None,
        },
        AvailableField {
            field: "supplier".to_string(),
            label: "Supplier".to_string(),
            data_type: "text".to_string(),
            operators: vec!["eq".to_string(), "like".to_string()],
            values: None,
        },
        // ✅ ДОБАВЛЕНО: дополнительные полезные поля
        AvailableField {
            field: "manufacturer".to_string(),
            label: "Manufacturer".to_string(),
            data_type: "text".to_string(),
            operators: vec!["eq".to_string(), "like".to_string()],
            values: None,
        },
        AvailableField {
            field: "reagent_name".to_string(),
            label: "Reagent Name".to_string(),
            data_type: "text".to_string(),
            operators: vec!["eq".to_string(), "like".to_string()],
            values: None,
        },
    ];

    Ok(HttpResponse::Ok().json(ApiResponse::success(fields)))
}

pub async fn get_report_columns(
    _app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let columns = ReportConfig::default_batch_columns();
    Ok(HttpResponse::Ok().json(ApiResponse::success(columns)))
}

pub async fn generate_report(
    app_state: web::Data<Arc<AppState>>,
    request: web::Json<GenerateReportRequest>,
    _http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let config = build_report_config(&request);
    let whitelist = FieldWhitelist::for_reports();

    // Пагинация
    let page = request.page.unwrap_or(1).max(1);
    let per_page = request.per_page.unwrap_or(50).clamp(1, 500);
    let offset = (page - 1) * per_page;

    // Строим WHERE условия
    let (where_clause, mut params) = build_filter_sql(&config, &whitelist);
    
    // ✅ ИСПРАВЛЕНО: Добавляем поиск с экранированием LIKE-спецсимволов
    let mut search_condition = String::new();
    if let Some(ref search) = request.search {
        if !search.trim().is_empty() {
            let escaped = escape_like_pattern(search.trim());
            let pattern = format!("%{}%", escaped);
            search_condition = " AND (reagent_name LIKE ? ESCAPE '\\' OR batch_number LIKE ? ESCAPE '\\' OR supplier LIKE ? ESCAPE '\\' OR location LIKE ? ESCAPE '\\')".to_string();
            params.push(pattern.clone());
            params.push(pattern.clone());
            params.push(pattern.clone());
            params.push(pattern);
        }
    }

    // ✅ ИСПРАВЛЕНО: Валидация сортировки через whitelist
    let sort_field = config.sort_by.as_deref()
        .and_then(validate_sort_field)
        .unwrap_or("created_at");
    let sort_order = if config.sort_order == "ASC" { "ASC" } else { "DESC" };

    // COUNT запрос
    let count_sql = format!(
        "{} WHERE {}{}",
        BASE_REPORT_QUERY, where_clause, search_condition
    );
    let count_sql = format!(
        "SELECT COUNT(*) FROM ({}) as subquery",
        count_sql
    );
    
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for p in &params {
        count_query = count_query.bind(p);
    }
    let total: i64 = count_query.fetch_one(&app_state.db_pool).await?;

    // DATA запрос
    let data_sql = format!(
        "{} WHERE {}{} ORDER BY {} {} LIMIT ? OFFSET ?",
        BASE_REPORT_QUERY, where_clause, search_condition, sort_field, sort_order
    );

    let mut data_query = sqlx::query_as::<_, BatchReportRow>(&data_sql);
    for p in &params {
        data_query = data_query.bind(p);
    }
    data_query = data_query.bind(per_page).bind(offset);
    
    let data: Vec<BatchReportRow> = data_query.fetch_all(&app_state.db_pool).await?;

    let total_pages = if per_page > 0 { (total + per_page - 1) / per_page } else { 1 };

    let response = ReportResponse {
        metadata: ReportMetadata {
            name: config.name.clone(),
            description: config.description.clone(),
            preset: config.preset.clone(),
            total_items: total,
            generated_at: Utc::now(),
            columns: config.columns.clone(),
        },
        data,
        pagination: PaginationInfo {
            page,
            per_page,
            total,
            total_pages,
        },
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

pub async fn export_report(
    app_state: web::Data<Arc<AppState>>,
    request: web::Json<GenerateReportRequest>,
    _http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let config = build_report_config(&request);
    let whitelist = FieldWhitelist::for_reports();

    let (where_clause, mut params) = build_filter_sql(&config, &whitelist);
    
    // ✅ ИСПРАВЛЕНО: Добавляем поиск с экранированием
    let mut search_condition = String::new();
    if let Some(ref search) = request.search {
        if !search.trim().is_empty() {
            let escaped = escape_like_pattern(search.trim());
            let pattern = format!("%{}%", escaped);
            search_condition = " AND (reagent_name LIKE ? ESCAPE '\\' OR batch_number LIKE ? ESCAPE '\\' OR supplier LIKE ? ESCAPE '\\' OR location LIKE ? ESCAPE '\\')".to_string();
            params.push(pattern.clone());
            params.push(pattern.clone());
            params.push(pattern.clone());
            params.push(pattern);
        }
    }

    // ✅ ИСПРАВЛЕНО: Валидация сортировки
    let sort_field = config.sort_by.as_deref()
        .and_then(validate_sort_field)
        .unwrap_or("created_at");
    let sort_order = if config.sort_order == "ASC" { "ASC" } else { "DESC" };

    // Запрос без пагинации для экспорта
    let data_sql = format!(
        "{} WHERE {}{} ORDER BY {} {}",
        BASE_REPORT_QUERY, where_clause, search_condition, sort_field, sort_order
    );

    let mut data_query = sqlx::query_as::<_, BatchReportRow>(&data_sql);
    for p in &params {
        data_query = data_query.bind(p);
    }
    
    let data: Vec<BatchReportRow> = data_query.fetch_all(&app_state.db_pool).await?;

    // ✅ ИСПРАВЛЕНО: Генерируем CSV с правильным экранированием
    let mut csv_content = String::new();
    // BOM для корректного отображения UTF-8 в Excel
    csv_content.push('\u{FEFF}');
    csv_content.push_str("ID,Reagent,Batch Number,Quantity,Unit,Expiry Date,Status,Location,Supplier,Notes\n");
    
    for row in &data {
        csv_content.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            escape_csv_field(&row.id),
            escape_csv_field(&row.reagent_name),
            escape_csv_field(&row.batch_number),
            row.quantity,
            escape_csv_field(&row.unit),
            row.expiry_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(),
            escape_csv_field(&row.status),
            escape_csv_field(row.location.as_deref().unwrap_or("")),
            escape_csv_field(row.supplier.as_deref().unwrap_or("")),
            escape_csv_field(row.notes.as_deref().unwrap_or("")),
        ));
    }

    let filename = format!("report_{}_{}.csv", config.preset, Utc::now().format("%Y%m%d_%H%M%S"));

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "text/csv; charset=utf-8"))
        .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", filename)))
        .body(csv_content))
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_sort_field() {
        // Валидные поля
        assert_eq!(validate_sort_field("created_at"), Some("created_at"));
        assert_eq!(validate_sort_field("quantity"), Some("quantity"));
        assert_eq!(validate_sort_field("reagent_name"), Some("reagent_name"));
        
        // SQL-инъекции блокируются
        assert_eq!(validate_sort_field("created_at; DROP TABLE users"), None);
        assert_eq!(validate_sort_field("1=1 OR 1=1"), None);
        assert_eq!(validate_sort_field("password"), None);
        assert_eq!(validate_sort_field(""), None);
        assert_eq!(validate_sort_field("' OR '1'='1"), None);
    }

    #[test]
    fn test_escape_like_pattern() {
        assert_eq!(escape_like_pattern("100%"), "100\\%");
        assert_eq!(escape_like_pattern("test_value"), "test\\_value");
        assert_eq!(escape_like_pattern("a\\b"), "a\\\\b");
        assert_eq!(escape_like_pattern("normal"), "normal");
        assert_eq!(escape_like_pattern("%_%"), "\\%\\_\\%");
    }

    #[test]
    fn test_escape_csv_field() {
        assert_eq!(escape_csv_field("simple"), "simple");
        assert_eq!(escape_csv_field("with,comma"), "\"with,comma\"");
        assert_eq!(escape_csv_field("with\"quote"), "\"with\"\"quote\"");
        assert_eq!(escape_csv_field("with\nnewline"), "\"with\nnewline\"");
        assert_eq!(escape_csv_field("combo,\"\n"), "\"combo,\"\"\n\"");
    }

    #[test]
    fn test_report_filter_request_conversion() {
        let req = ReportFilterRequest {
            field: "quantity".to_string(),
            operator: "gt".to_string(),
            value: serde_json::json!(10),
        };
        
        let filter = req.to_report_filter();
        assert!(filter.is_some());
        let f = filter.unwrap();
        assert_eq!(f.field, "quantity");
        assert_eq!(f.operator, ComparisonOperator::Gt);
    }

    #[test]
    fn test_invalid_operator_returns_none() {
        let req = ReportFilterRequest {
            field: "status".to_string(),
            operator: "INVALID_OP".to_string(),
            value: serde_json::json!("test"),
        };
        
        assert!(req.to_report_filter().is_none());
    }
}