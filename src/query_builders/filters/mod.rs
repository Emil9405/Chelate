// src/query_builders/filters/mod.rs
//! Фильтры и whitelist для безопасных запросов

use serde::{Serialize, Deserialize};
use std::collections::HashSet;
pub mod whitelist;

// ==================== FIELD WHITELIST ====================

#[derive(Debug, Clone)]
pub struct FieldWhitelist {
    fields: HashSet<String>,
    table: String,
}

impl FieldWhitelist {
    pub fn new(table: &str, fields: &[&str]) -> Self {
        Self {
            table: table.to_string(),
            fields: fields.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn is_allowed(&self, field: &str) -> bool {
        let clean_field = field.split('.').last().unwrap_or(field);
        self.fields.contains(clean_field) || self.fields.contains(field)
    }

    pub fn for_batches() -> Self {
        Self::new("batches", &[
            "id", "reagent_id", "batch_number", "cat_number", "quantity",
            "original_quantity", "reserved_quantity", "unit", "expiry_date",
            "supplier", "manufacturer", "received_date", "status", "location",
            "notes", "created_by", "updated_by", "created_at", "updated_at",
            "days_until_expiry", "reagent_name",
        ])
    }

    pub fn for_reagents() -> Self {
        Self::new("reagents", &[
            "id", "name", "formula", "cas_number", "manufacturer",
            "molecular_weight", "physical_state", "description", "status",
            "created_by", "updated_by", "created_at", "updated_at",
        ])
    }

    pub fn for_experiments() -> Self {
        Self::new("experiments", &[
            "id", "title", "description", "experiment_date", "experiment_type",
            "instructor", "student_group", "location", "status", "room_id",
            "created_by", "updated_by", "created_at", "updated_at",
        ])
    }

    pub fn for_equipment() -> Self {
        Self::new("equipment", &[
            "id", "name", "type_", "quantity", "unit", "status", "location",
            "description", "serial_number", "manufacturer", "model",
            "purchase_date", "warranty_until", "created_by", "updated_by",
            "created_at", "updated_at",
        ])
    }

    pub fn for_rooms() -> Self {
        Self::new("rooms", &[
            "id", "name", "description", "capacity", "color", "status",
            "created_by", "updated_by", "created_at", "updated_at",
        ])
    }

    pub fn for_users() -> Self {
        Self::new("users", &[
            "id", "username", "email", "role", "is_active",
            "created_at", "updated_at",
        ])
    }
    
    pub fn for_reports() -> Self {
        Self::new("batches", &[
            "id", "reagent_id", "batch_number", "cat_number", "quantity",
            "original_quantity", "reserved_quantity", "unit", "expiry_date",
            "supplier", "manufacturer", "received_date", "status", "location",
            "notes", "created_at", "updated_at", "days_until_expiry",
            "reagent_name", "expiration_status",
        ])
    }
}

// ==================== FILTER TYPES ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    Eq, Neq, Lt, Lte, Gt, Gte, Like, NotLike, In, NotIn, IsNull, IsNotNull, Between,
}

impl FilterOperator {
    pub fn to_sql(&self) -> &'static str {
        match self {
            FilterOperator::Eq => "=",
            FilterOperator::Neq => "!=",
            FilterOperator::Lt => "<",
            FilterOperator::Lte => "<=",
            FilterOperator::Gt => ">",
            FilterOperator::Gte => ">=",
            FilterOperator::Like => "LIKE",
            FilterOperator::NotLike => "NOT LIKE",
            FilterOperator::In => "IN",
            FilterOperator::NotIn => "NOT IN",
            FilterOperator::IsNull => "IS NULL",
            FilterOperator::IsNotNull => "IS NOT NULL",
            FilterOperator::Between => "BETWEEN",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    String(String),
    Number(f64),
    Integer(i64),
    Boolean(bool),
    Array(Vec<String>),
    Null,
}

impl FilterValue {
    pub fn to_string_value(&self) -> String {
        match self {
            FilterValue::String(s) => s.clone(),
            FilterValue::Number(n) => n.to_string(),
            FilterValue::Integer(i) => i.to_string(),
            FilterValue::Boolean(b) => if *b { "1" } else { "0" }.to_string(),
            FilterValue::Array(arr) => arr.join(","),
            FilterValue::Null => "NULL".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: FilterValue,
}

impl Filter {
    pub fn eq(field: &str, value: impl Into<FilterValue>) -> Self {
        Self { field: field.to_string(), operator: FilterOperator::Eq, value: value.into() }
    }
    pub fn neq(field: &str, value: impl Into<FilterValue>) -> Self {
        Self { field: field.to_string(), operator: FilterOperator::Neq, value: value.into() }
    }
    pub fn lt(field: &str, value: impl Into<FilterValue>) -> Self {
        Self { field: field.to_string(), operator: FilterOperator::Lt, value: value.into() }
    }
    pub fn lte(field: &str, value: impl Into<FilterValue>) -> Self {
        Self { field: field.to_string(), operator: FilterOperator::Lte, value: value.into() }
    }
    pub fn gt(field: &str, value: impl Into<FilterValue>) -> Self {
        Self { field: field.to_string(), operator: FilterOperator::Gt, value: value.into() }
    }
    pub fn gte(field: &str, value: impl Into<FilterValue>) -> Self {
        Self { field: field.to_string(), operator: FilterOperator::Gte, value: value.into() }
    }
    pub fn like(field: &str, value: impl Into<String>) -> Self {
        Self { field: field.to_string(), operator: FilterOperator::Like, value: FilterValue::String(value.into()) }
    }
    pub fn between(field: &str, from: &str, to: &str) -> Self {
        Self { field: field.to_string(), operator: FilterOperator::Between, value: FilterValue::Array(vec![from.to_string(), to.to_string()]) }
    }
    pub fn between_numbers(field: &str, from: f64, to: f64) -> Self {
        Self { field: field.to_string(), operator: FilterOperator::Between, value: FilterValue::Array(vec![from.to_string(), to.to_string()]) }
    }
    pub fn is_null(field: &str) -> Self {
        Self { field: field.to_string(), operator: FilterOperator::IsNull, value: FilterValue::Null }
    }
    pub fn is_not_null(field: &str) -> Self {
        Self { field: field.to_string(), operator: FilterOperator::IsNotNull, value: FilterValue::Null }
    }
}

impl From<String> for FilterValue { fn from(s: String) -> Self { FilterValue::String(s) } }
impl From<&str> for FilterValue { fn from(s: &str) -> Self { FilterValue::String(s.to_string()) } }
impl From<f64> for FilterValue { fn from(n: f64) -> Self { FilterValue::Number(n) } }
impl From<i64> for FilterValue { fn from(i: i64) -> Self { FilterValue::Integer(i) } }
impl From<bool> for FilterValue { fn from(b: bool) -> Self { FilterValue::Boolean(b) } }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterItem {
    Filter(Filter),
    Group(FilterGroup),
}

impl FilterItem {
    pub fn filter(f: Filter) -> Self { FilterItem::Filter(f) }
    pub fn group(g: FilterGroup) -> Self { FilterItem::Group(g) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterGroup {
    pub logic: String,
    pub items: Vec<FilterItem>,
}

impl FilterGroup {
    pub fn and(items: Vec<FilterItem>) -> Self { Self { logic: "AND".to_string(), items } }
    pub fn or(items: Vec<FilterItem>) -> Self { Self { logic: "OR".to_string(), items } }
}

// ==================== FILTER BUILDER ====================

pub struct FilterBuilder<'a> {
    whitelist: Option<&'a FieldWhitelist>,
}

impl<'a> FilterBuilder<'a> {
    pub fn new() -> Self { Self { whitelist: None } }

    pub fn with_whitelist(mut self, whitelist: &'a FieldWhitelist) -> Self {
        self.whitelist = Some(whitelist);
        self
    }

    pub fn build_condition(&self, group: &FilterGroup) -> Result<(String, Vec<String>), String> {
        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<String> = Vec::new();

        for item in &group.items {
            match item {
                FilterItem::Filter(f) => {
                    if let Some(wl) = self.whitelist {
                        if !wl.is_allowed(&f.field) { continue; }
                    }
                    let (cond, p) = self.build_filter_condition(f)?;
                    if !cond.is_empty() {
                        conditions.push(cond);
                        params.extend(p);
                    }
                }
                FilterItem::Group(g) => {
                    let (cond, p) = self.build_condition(g)?;
                    if !cond.is_empty() {
                        conditions.push(format!("({})", cond));
                        params.extend(p);
                    }
                }
            }
        }

        if conditions.is_empty() { return Ok((String::new(), Vec::new())); }
        Ok((conditions.join(&format!(" {} ", group.logic)), params))
    }

    fn build_filter_condition(&self, filter: &Filter) -> Result<(String, Vec<String>), String> {
        let mut params: Vec<String> = Vec::new();
        let condition = match &filter.operator {
            FilterOperator::IsNull => format!("{} IS NULL", filter.field),
            FilterOperator::IsNotNull => format!("{} IS NOT NULL", filter.field),
            FilterOperator::In | FilterOperator::NotIn => {
                if let FilterValue::Array(arr) = &filter.value {
                    let placeholders: Vec<_> = arr.iter().map(|_| "?").collect();
                    params.extend(arr.clone());
                    format!("{} {} ({})", filter.field, filter.operator.to_sql(), placeholders.join(", "))
                } else {
                    params.push(filter.value.to_string_value());
                    format!("{} {} (?)", filter.field, filter.operator.to_sql())
                }
            }
            FilterOperator::Between => {
                if let FilterValue::Array(arr) = &filter.value {
                    if arr.len() >= 2 {
                        params.push(arr[0].clone());
                        params.push(arr[1].clone());
                        format!("{} BETWEEN ? AND ?", filter.field)
                    } else { return Ok((String::new(), Vec::new())); }
                } else { return Ok((String::new(), Vec::new())); }
            }
            FilterOperator::Like | FilterOperator::NotLike => {
                let pattern = match &filter.value {
                    FilterValue::String(s) => if s.contains('%') { s.clone() } else { format!("%{}%", s) },
                    _ => format!("%{}%", filter.value.to_string_value()),
                };
                params.push(pattern);
                format!("{} {} ?", filter.field, filter.operator.to_sql())
            }
            _ => {
                params.push(filter.value.to_string_value());
                format!("{} {} ?", filter.field, filter.operator.to_sql())
            }
        };
        Ok((condition, params))
    }
}

impl Default for FilterBuilder<'_> {
    fn default() -> Self { Self::new() }
}

// ==================== REPORT TYPES ====================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    Eq, Ne, Lt, Lte, Gt, Gte, Like, In, NotIn, IsNull, IsNotNull, Between,
}

impl ComparisonOperator {
    pub fn to_sql(&self) -> &'static str {
        match self {
            ComparisonOperator::Eq => "=",
            ComparisonOperator::Ne => "!=",
            ComparisonOperator::Lt => "<",
            ComparisonOperator::Lte => "<=",
            ComparisonOperator::Gt => ">",
            ComparisonOperator::Gte => ">=",
            ComparisonOperator::Like => "LIKE",
            ComparisonOperator::In => "IN",
            ComparisonOperator::NotIn => "NOT IN",
            ComparisonOperator::IsNull => "IS NULL",
            ComparisonOperator::IsNotNull => "IS NOT NULL",
            ComparisonOperator::Between => "BETWEEN",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReportFilterValue {
    Exact(String),
    Number(f64),
    Contains(String),
    List(Vec<String>),
    Range { from: String, to: String },
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportFilter {
    pub field: String,
    pub operator: ComparisonOperator,
    pub value: ReportFilterValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportColumn {
    pub field: String,
    pub label: String,
    #[serde(default)]
    pub sortable: bool,
    #[serde(default)]
    pub filterable: bool,
    #[serde(default)]
    pub visible: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

impl ReportColumn {
    pub fn new(field: &str, label: &str) -> Self {
        Self { field: field.to_string(), label: label.to_string(), sortable: true, filterable: true, visible: true, format: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub filters: Vec<ReportFilter>,
    pub columns: Vec<ReportColumn>,
    pub default_sort: Option<String>,
    pub default_sort_order: Option<String>,
}

impl ReportPreset {
    pub fn new(id: &str, name: &str, description: &str) -> Self {
        Self { id: id.to_string(), name: name.to_string(), description: description.to_string(), filters: Vec::new(), columns: Vec::new(), default_sort: None, default_sort_order: None }
    }
    pub fn as_str(&self) -> &str { &self.id }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    pub preset: String,
    pub name: String,
    pub description: Option<String>,
    pub filters: Vec<ReportFilter>,
    pub columns: Vec<ReportColumn>,
    pub sort_by: Option<String>,
    pub sort_order: String,
    pub page: i64,
    pub per_page: i64,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self { preset: "custom".to_string(), name: "Custom Report".to_string(), description: None, filters: Vec::new(), columns: Self::default_batch_columns(), sort_by: None, sort_order: "DESC".to_string(), page: 1, per_page: 20 }
    }
}

impl ReportConfig {
    pub fn new(preset: &str) -> Self { Self { preset: preset.to_string(), ..Default::default() } }

    pub fn default_batch_columns() -> Vec<ReportColumn> {
        vec![
            ReportColumn::new("reagent_name", "Reagent"),
            ReportColumn::new("batch_number", "Batch #"),
            ReportColumn::new("quantity", "Quantity"),
            ReportColumn::new("unit", "Unit"),
            ReportColumn::new("expiry_date", "Expiry Date"),
            ReportColumn::new("status", "Status"),
            ReportColumn::new("location", "Location"),
        ]
    }

    pub fn all_batches() -> Self { Self::new("all_batches") }
    
    pub fn low_stock(threshold: f64) -> Self {
        let mut config = Self::new("low_stock");
        config.filters.push(ReportFilter {
            field: "quantity".to_string(),
            operator: ComparisonOperator::Lte,
            value: ReportFilterValue::Number(threshold),
        });
        config
    }
    
    pub fn expiring_soon(days: i64) -> Self {
        let mut config = Self::new("expiring_soon");
        config.filters.push(ReportFilter {
            field: "days_until_expiry".to_string(),
            operator: ComparisonOperator::Lte,
            value: ReportFilterValue::Number(days as f64),
        });
        config.filters.push(ReportFilter {
            field: "days_until_expiry".to_string(),
            operator: ComparisonOperator::Gte,
            value: ReportFilterValue::Number(0.0),
        });
        config
    }
    
    pub fn expired() -> Self {
        let mut config = Self::new("expired");
        config.filters.push(ReportFilter {
            field: "days_until_expiry".to_string(),
            operator: ComparisonOperator::Lt,
            value: ReportFilterValue::Number(0.0),
        });
        config
    }

    pub fn build_where_clause(&self, whitelist: &FieldWhitelist) -> (String, Vec<String>) {
        if self.filters.is_empty() { return ("1=1".to_string(), Vec::new()); }
        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<String> = Vec::new();
        for filter in &self.filters {
            if whitelist.is_allowed(&filter.field) {
                let (cond, p) = build_report_filter_condition(filter);
                if !cond.is_empty() {
                    conditions.push(cond);
                    params.extend(p);
                }
            }
        }
        if conditions.is_empty() { ("1=1".to_string(), Vec::new()) } else { (conditions.join(" AND "), params) }
    }
}

fn build_report_filter_condition(filter: &ReportFilter) -> (String, Vec<String>) {
    let mut params: Vec<String> = Vec::new();
    let condition = match &filter.operator {
        ComparisonOperator::IsNull => format!("{} IS NULL", filter.field),
        ComparisonOperator::IsNotNull => format!("{} IS NOT NULL", filter.field),
        ComparisonOperator::In | ComparisonOperator::NotIn => {
            if let ReportFilterValue::List(arr) = &filter.value {
                let placeholders: Vec<_> = arr.iter().map(|_| "?").collect();
                params.extend(arr.clone());
                format!("{} {} ({})", filter.field, filter.operator.to_sql(), placeholders.join(", "))
            } else { return (String::new(), Vec::new()); }
        }
        ComparisonOperator::Between => {
            if let ReportFilterValue::Range { from, to } = &filter.value {
                params.push(from.clone());
                params.push(to.clone());
                format!("{} BETWEEN ? AND ?", filter.field)
            } else { return (String::new(), Vec::new()); }
        }
        ComparisonOperator::Like => {
            let pattern = match &filter.value {
                ReportFilterValue::Contains(s) => format!("%{}%", s),
                ReportFilterValue::Exact(s) => format!("%{}%", s),
                _ => return (String::new(), Vec::new()),
            };
            params.push(pattern);
            format!("{} LIKE ?", filter.field)
        }
        _ => {
            let val = match &filter.value {
                ReportFilterValue::Exact(s) => s.clone(),
                ReportFilterValue::Number(n) => n.to_string(),
                ReportFilterValue::Contains(s) => s.clone(),
                _ => return (String::new(), Vec::new()),
            };
            params.push(val);
            format!("{} {} ?", filter.field, filter.operator.to_sql())
        }
    };
    (condition, params)
}
