// src/report_handlers.rs
//! Обработчики для системы кастомных репортов
//! Экспорт: CSV и XLSX (параметр request.format = "csv" | "xlsx")

use actix_web::{web, HttpResponse, HttpRequest};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Datelike, Timelike};

use crate::AppState;
use crate::error::{ApiResult, ApiError};
use crate::handlers::ApiResponse;
use crate::query_builders::{
    FieldWhitelist, ReportConfig, ReportFilter, ReportColumn,
    ComparisonOperator, ReportFilterValue,
};

// ==================== SECURITY CONSTANTS ====================

const ALLOWED_SORT_FIELDS: &[&str] = &[
    "id", "reagent_id", "reagent_name", "batch_number", "cat_number",
    "quantity", "original_quantity", "reserved_quantity", "unit",
    "expiry_date", "supplier", "manufacturer", "received_date",
    "status", "created_at", "updated_at", "days_until_expiry",
    "expiration_status",
    "container_count", "opened_count", "placed_count", "unplaced_count",
    "location_summary", "room_names",
];

fn validate_sort_field(field: &str) -> Option<&'static str> {
    ALLOWED_SORT_FIELDS.iter()
        .find(|&&allowed| allowed == field)
        .copied()
}

fn escape_like_pattern(pattern: &str) -> String {
    pattern
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

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
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub days_until_expiry: Option<i64>,
    pub expiration_status: String,
    pub container_count: i64,
    pub opened_count: i64,
    pub placed_count: i64,
    pub unplaced_count: i64,
    pub location_summary: Option<String>,
    pub room_names: Option<String>,
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
    /// Формат экспорта: "csv" (default) | "xlsx". Используется только в /reports/export.
    pub format: Option<String>,
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
        "depleted" => ReportConfig::depleted(),
        "unplaced" => ReportConfig::unplaced(),
        _ => ReportConfig::all_batches(),
    };

    config.preset = preset.to_string();
    config.name = match preset {
        "low_stock" => "Low Stock Report".to_string(),
        "expiring_soon" => "Expiring Soon Report".to_string(),
        "expired" => "Expired Items Report".to_string(),
        "depleted" => "Depleted Batches Report".to_string(),
        "unplaced" => "Unplaced Containers Report".to_string(),
        _ => "All Batches Report".to_string(),
    };

    if let Some(ref filters) = request.filters {
        for filter_req in filters {
            if let Some(filter) = filter_req.to_report_filter() {
                config.filters.push(filter);
            }
        }
    }

    if let Some(ref sort_by) = request.sort_by {
        if validate_sort_field(sort_by).is_some() {
            config.sort_by = Some(sort_by.clone());
        }
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
    WITH container_stats AS (
        SELECT
            bc.batch_id,
            COUNT(DISTINCT bc.id) as container_count,
            SUM(CASE WHEN bc.is_opened = 1 THEN 1 ELSE 0 END) as opened_count,
            SUM(CASE WHEN bp.id IS NOT NULL THEN 1 ELSE 0 END) as placed_count,
            GROUP_CONCAT(DISTINCT rm.name || ' → ' || sz.name || ' → ' || sp.name) as location_summary,
            GROUP_CONCAT(DISTINCT rm.name) as room_names
        FROM batch_containers bc
        LEFT JOIN batch_placements bp ON bp.container_id = bc.id
        LEFT JOIN storage_positions sp ON bp.position_id = sp.id
        LEFT JOIN storage_zones sz ON sp.zone_id = sz.id
        LEFT JOIN rooms rm ON sz.room_id = rm.id
        WHERE bc.status != 'disposed'
        GROUP BY bc.batch_id
    ),
    batch_data AS (
        SELECT
            b.id, b.reagent_id, r.name as reagent_name, b.batch_number, b.cat_number,
            b.quantity, b.original_quantity, b.reserved_quantity, b.unit, b.expiry_date,
            b.supplier, b.manufacturer, b.received_date, b.status, b.notes,
            b.created_at, b.updated_at,
            COALESCE(cs.container_count, 0) as container_count,
            COALESCE(cs.opened_count, 0) as opened_count,
            COALESCE(cs.placed_count, 0) as placed_count,
            COALESCE(cs.container_count, 0) - COALESCE(cs.placed_count, 0) as unplaced_count,
            cs.location_summary,
            cs.room_names,
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
        JOIN reagents r ON b.reagent_id = r.id AND r.deleted_at IS NULL
        LEFT JOIN container_stats cs ON cs.batch_id = b.id
        WHERE b.deleted_at IS NULL
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
            description: "Complete list of active batches (excludes depleted)".to_string(),
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
        AvailablePreset {
            id: "depleted".to_string(),
            name: "Depleted Batches".to_string(),
            description: "Fully consumed batches — archive / history".to_string(),
            default_params: serde_json::json!({}),
        },
        AvailablePreset {
            id: "unplaced".to_string(),
            name: "Unplaced Containers".to_string(),
            description: "Batches with containers not yet assigned to a storage position".to_string(),
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
            field: "location_summary".to_string(),
            label: "Location".to_string(),
            data_type: "text".to_string(),
            operators: vec!["eq".to_string(), "like".to_string(), "is_null".to_string(), "is_not_null".to_string()],
            values: None,
        },
        AvailableField {
            field: "room_names".to_string(),
            label: "Rooms".to_string(),
            data_type: "text".to_string(),
            operators: vec!["like".to_string(), "is_null".to_string(), "is_not_null".to_string()],
            values: None,
        },
        AvailableField {
            field: "container_count".to_string(),
            label: "Containers (total)".to_string(),
            data_type: "number".to_string(),
            operators: vec!["eq".to_string(), "gt".to_string(), "gte".to_string(), "lt".to_string(), "lte".to_string()],
            values: None,
        },
        AvailableField {
            field: "opened_count".to_string(),
            label: "Containers (opened)".to_string(),
            data_type: "number".to_string(),
            operators: vec!["eq".to_string(), "gt".to_string(), "gte".to_string(), "lt".to_string(), "lte".to_string()],
            values: None,
        },
        AvailableField {
            field: "placed_count".to_string(),
            label: "Containers (placed)".to_string(),
            data_type: "number".to_string(),
            operators: vec!["eq".to_string(), "gt".to_string(), "gte".to_string(), "lt".to_string(), "lte".to_string()],
            values: None,
        },
        AvailableField {
            field: "unplaced_count".to_string(),
            label: "Containers (unplaced)".to_string(),
            data_type: "number".to_string(),
            operators: vec!["eq".to_string(), "gt".to_string(), "gte".to_string(), "lt".to_string(), "lte".to_string()],
            values: None,
        },
        AvailableField {
            field: "supplier".to_string(),
            label: "Supplier".to_string(),
            data_type: "text".to_string(),
            operators: vec!["eq".to_string(), "like".to_string()],
            values: None,
        },
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

    let page = request.page.unwrap_or(1).max(1);
    let per_page = request.per_page.unwrap_or(50).clamp(1, 500);
    let offset = (page - 1) * per_page;

    let (where_clause, mut params) = build_filter_sql(&config, &whitelist);

    let mut search_condition = String::new();
    if let Some(ref search) = request.search {
        if !search.trim().is_empty() {
            let escaped = escape_like_pattern(search.trim());
            let pattern = format!("%{}%", escaped);
            search_condition = " AND (reagent_name LIKE ? ESCAPE '\\' OR batch_number LIKE ? ESCAPE '\\' OR supplier LIKE ? ESCAPE '\\' OR COALESCE(location_summary, '') LIKE ? ESCAPE '\\' OR COALESCE(room_names, '') LIKE ? ESCAPE '\\')".to_string();
            for _ in 0..5 {
                params.push(pattern.clone());
            }
        }
    }

    let sort_field = config.sort_by.as_deref()
        .and_then(validate_sort_field)
        .unwrap_or("created_at");
    let sort_order = if config.sort_order == "ASC" { "ASC" } else { "DESC" };

    let count_sql = format!(
        "SELECT COUNT(*) FROM ({} WHERE {}{}) as subquery",
        BASE_REPORT_QUERY, where_clause, search_condition
    );

    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for p in &params {
        count_query = count_query.bind(p);
    }
    let total: i64 = count_query.fetch_one(&app_state.db_pool).await?;

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

// ==================== EXPORT ====================

/// Тянет все строки отчёта без пагинации (общая логика для CSV и XLSX).
async fn fetch_export_data(
    app_state: &AppState,
    request: &GenerateReportRequest,
) -> ApiResult<(ReportConfig, Vec<BatchReportRow>)> {
    let config = build_report_config(request);
    let whitelist = FieldWhitelist::for_reports();

    let (where_clause, mut params) = build_filter_sql(&config, &whitelist);

    let mut search_condition = String::new();
    if let Some(ref search) = request.search {
        if !search.trim().is_empty() {
            let escaped = escape_like_pattern(search.trim());
            let pattern = format!("%{}%", escaped);
            search_condition = " AND (reagent_name LIKE ? ESCAPE '\\' OR batch_number LIKE ? ESCAPE '\\' OR supplier LIKE ? ESCAPE '\\' OR COALESCE(location_summary, '') LIKE ? ESCAPE '\\' OR COALESCE(room_names, '') LIKE ? ESCAPE '\\')".to_string();
            for _ in 0..5 {
                params.push(pattern.clone());
            }
        }
    }

    let sort_field = config.sort_by.as_deref()
        .and_then(validate_sort_field)
        .unwrap_or("created_at");
    let sort_order = if config.sort_order == "ASC" { "ASC" } else { "DESC" };

    // Лимит на экспорт — не более 100k строк за раз (защита от OOM на сервере)
    const EXPORT_LIMIT: i64 = 100_000;

    let data_sql = format!(
        "{} WHERE {}{} ORDER BY {} {} LIMIT {}",
        BASE_REPORT_QUERY, where_clause, search_condition, sort_field, sort_order, EXPORT_LIMIT
    );

    let mut data_query = sqlx::query_as::<_, BatchReportRow>(&data_sql);
    for p in &params {
        data_query = data_query.bind(p);
    }

    let data: Vec<BatchReportRow> = data_query.fetch_all(&app_state.db_pool).await?;
    Ok((config, data))
}

/// Главная точка входа. Ветвит по format: "csv" | "xlsx".
pub async fn export_report(
    app_state: web::Data<Arc<AppState>>,
    request: web::Json<GenerateReportRequest>,
    _http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let format = request.format.as_deref()
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "csv".to_string());

    let (config, data) = fetch_export_data(&app_state, &request).await?;

    match format.as_str() {
        "xlsx" | "excel" => build_xlsx_response(&config, &data, &request),
        _ => Ok(build_csv_response(&config, &data)),
    }
}

// ==================== CSV ====================

fn build_csv_response(config: &ReportConfig, data: &[BatchReportRow]) -> HttpResponse {
    let mut csv_content = String::new();
    csv_content.push('\u{FEFF}'); // BOM для Excel
    csv_content.push_str(
        "ID,Reagent,Batch Number,Quantity,Unit,Expiry Date,Days Left,Status,\
         Containers,Opened,Placed,Unplaced,Location,Rooms,Supplier,Manufacturer,Notes\r\n"
    );

    for row in data {
        csv_content.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\r\n",
            escape_csv_field(&row.id),
            escape_csv_field(&row.reagent_name),
            escape_csv_field(&row.batch_number),
            row.quantity,
            escape_csv_field(&row.unit),
            row.expiry_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(),
            row.days_until_expiry.map(|d| d.to_string()).unwrap_or_default(),
            escape_csv_field(&row.status),
            row.container_count,
            row.opened_count,
            row.placed_count,
            row.unplaced_count,
            escape_csv_field(row.location_summary.as_deref().unwrap_or("")),
            escape_csv_field(row.room_names.as_deref().unwrap_or("")),
            escape_csv_field(row.supplier.as_deref().unwrap_or("")),
            escape_csv_field(row.manufacturer.as_deref().unwrap_or("")),
            escape_csv_field(row.notes.as_deref().unwrap_or("")),
        ));
    }

    let filename = format!("report_{}_{}.csv", config.preset, Utc::now().format("%Y%m%d_%H%M%S"));

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/csv; charset=utf-8"))
        .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", filename)))
        .body(csv_content)
}

// ==================== XLSX ====================

fn build_xlsx_response(
    config: &ReportConfig,
    data: &[BatchReportRow],
    request: &GenerateReportRequest,
) -> ApiResult<HttpResponse> {
    use rust_xlsxwriter::{Workbook, Format, FormatAlign, ExcelDateTime, Color};

    let mut wb = Workbook::new();

    // Форматы
    let header_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xE2E8F0))
        .set_align(FormatAlign::Center)
        .set_border(rust_xlsxwriter::FormatBorder::Thin);

    let date_fmt = Format::new().set_num_format("yyyy-mm-dd");

    // === Лист "Data" ===
    let sheet_name = if config.preset.len() > 31 {
        &config.preset[..31]
    } else {
        &config.preset
    };

    let ws = wb.add_worksheet();
    ws.set_name(sheet_name)
        .map_err(|e| ApiError::InternalServerError(format!("XLSX sheet name: {}", e)))?;

    let headers = [
        "ID", "Reagent", "Batch Number", "Quantity", "Unit", "Expiry Date",
        "Days Left", "Status", "Containers", "Opened", "Placed", "Unplaced",
        "Location", "Rooms", "Supplier", "Manufacturer", "Notes",
    ];

    for (col, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col as u16, *h, &header_fmt)
            .map_err(|e| ApiError::InternalServerError(format!("XLSX header: {}", e)))?;
    }

    for (row_idx, row) in data.iter().enumerate() {
        let r = (row_idx + 1) as u32;

        ws.write_string(r, 0, &row.id).ok();
        ws.write_string(r, 1, &row.reagent_name).ok();
        ws.write_string(r, 2, &row.batch_number).ok();
        ws.write_number(r, 3, row.quantity).ok();
        ws.write_string(r, 4, &row.unit).ok();

        // Expiry date — как настоящая дата-ячейка
        if let Some(d) = row.expiry_date {
            let nd = d.naive_utc();
            if let Ok(edt) = ExcelDateTime::from_ymd(
                nd.year() as u16,
                nd.month() as u8,
                nd.day() as u8,
            ) {
                ws.write_datetime_with_format(r, 5, &edt, &date_fmt).ok();
            }
        }

        if let Some(d) = row.days_until_expiry {
            ws.write_number(r, 6, d as f64).ok();
        }

        ws.write_string(r, 7, &row.status).ok();
        ws.write_number(r, 8, row.container_count as f64).ok();
        ws.write_number(r, 9, row.opened_count as f64).ok();
        ws.write_number(r, 10, row.placed_count as f64).ok();
        ws.write_number(r, 11, row.unplaced_count as f64).ok();
        ws.write_string(r, 12, row.location_summary.as_deref().unwrap_or("")).ok();
        ws.write_string(r, 13, row.room_names.as_deref().unwrap_or("")).ok();
        ws.write_string(r, 14, row.supplier.as_deref().unwrap_or("")).ok();
        ws.write_string(r, 15, row.manufacturer.as_deref().unwrap_or("")).ok();
        ws.write_string(r, 16, row.notes.as_deref().unwrap_or("")).ok();
    }

    // Закрепляем шапку и автоширина колонок
    ws.set_freeze_panes(1, 0).ok();
    ws.autofit();

    // === Лист "Info" — параметры отчёта ===
    let info_ws = wb.add_worksheet();
    info_ws.set_name("Info")
        .map_err(|e| ApiError::InternalServerError(format!("XLSX info sheet: {}", e)))?;

    let label_fmt = Format::new().set_bold();

    let mut row: u32 = 0;

    info_ws.write_with_format(row, 0, "Report", &label_fmt).ok();
    info_ws.write_string(row, 1, &config.name).ok();
    row += 1;

    info_ws.write_with_format(row, 0, "Preset", &label_fmt).ok();
    info_ws.write_string(row, 1, &config.preset).ok();
    row += 1;

info_ws.write_with_format(row, 0, "Generated", &label_fmt).ok();
    let now = Utc::now();
    let dt_fmt = Format::new().set_num_format("yyyy-mm-dd hh:mm:ss");
    let dt_built = ExcelDateTime::from_ymd(
            now.year() as u16,
            now.month() as u8,
            now.day() as u8,
        )
        .and_then(|d| d.and_hms(
            now.hour() as u16,
            now.minute() as u8,
            now.second() as f64,
        ));

    if let Ok(edt) = dt_built {
        info_ws.write_datetime_with_format(row, 1, &edt, &dt_fmt).ok();
    } else {
        info_ws.write_string(row, 1, &now.to_rfc3339()).ok();
    }
    row += 1;

    info_ws.write_with_format(row, 0, "Total rows", &label_fmt).ok();
    info_ws.write_number(row, 1, data.len() as f64).ok();
    row += 1;

    if let Some(s) = &request.search {
        if !s.trim().is_empty() {
            info_ws.write_with_format(row, 0, "Search", &label_fmt).ok();
            info_ws.write_string(row, 1, s).ok();
            row += 1;
        }
    }

    if let Some(sort_by) = &request.sort_by {
        info_ws.write_with_format(row, 0, "Sort", &label_fmt).ok();
        let sort_order = request.sort_order.as_deref().unwrap_or("DESC");
        info_ws.write_string(row, 1, &format!("{} {}", sort_by, sort_order)).ok();
        row += 1;
    }

    if !config.filters.is_empty() {
        row += 1;
        info_ws.write_with_format(row, 0, "Filters", &label_fmt).ok();
        row += 1;
        for f in &config.filters {
            info_ws.write_string(row, 0, &f.field).ok();
            let value_str = format_filter_value(&f.value);
            info_ws.write_string(
                row,
                1,
                &format!("{} {}", format_operator(&f.operator), value_str),
            ).ok();
            row += 1;
        }
    }

    info_ws.set_column_width(0, 20.0).ok();
    info_ws.set_column_width(1, 50.0).ok();

    // Сериализуем в буфер
    let buf = wb.save_to_buffer()
        .map_err(|e| ApiError::InternalServerError(format!("XLSX save: {}", e)))?;

    let filename = format!(
        "report_{}_{}.xlsx",
        config.preset,
        Utc::now().format("%Y%m%d_%H%M%S")
    );

    Ok(HttpResponse::Ok()
        .insert_header(("Content-Type", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"))
        .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", filename)))
        .body(buf))
}

fn format_operator(op: &ComparisonOperator) -> &'static str {
    match op {
        ComparisonOperator::Eq => "=",
        ComparisonOperator::Ne => "≠",
        ComparisonOperator::Gt => ">",
        ComparisonOperator::Gte => "≥",
        ComparisonOperator::Lt => "<",
        ComparisonOperator::Lte => "≤",
        ComparisonOperator::Like => "contains",
        ComparisonOperator::In => "in",
        ComparisonOperator::NotIn => "not in",
        ComparisonOperator::IsNull => "is empty",
        ComparisonOperator::IsNotNull => "is not empty",
        ComparisonOperator::Between => "between",
    }
}

fn format_filter_value(v: &ReportFilterValue) -> String {
    match v {
        ReportFilterValue::Exact(s) => s.clone(),
        ReportFilterValue::Contains(s) => s.clone(),
        ReportFilterValue::Number(n) => format!("{}", n),
        ReportFilterValue::List(list) => list.join(", "),
        ReportFilterValue::Range { from, to } => format!("{} … {}", from, to),
        ReportFilterValue::Null => String::new(),
    }
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_sort_field() {
        assert_eq!(validate_sort_field("created_at"), Some("created_at"));
        assert_eq!(validate_sort_field("quantity"), Some("quantity"));
        assert_eq!(validate_sort_field("reagent_name"), Some("reagent_name"));
        assert_eq!(validate_sort_field("container_count"), Some("container_count"));
        assert_eq!(validate_sort_field("opened_count"), Some("opened_count"));
        assert_eq!(validate_sort_field("placed_count"), Some("placed_count"));
        assert_eq!(validate_sort_field("unplaced_count"), Some("unplaced_count"));
        assert_eq!(validate_sort_field("location_summary"), Some("location_summary"));
        assert_eq!(validate_sort_field("room_names"), Some("room_names"));
        assert_eq!(validate_sort_field("location"), None);
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
