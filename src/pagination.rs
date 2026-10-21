// src/pagination.rs
//! Page-based (offset) pagination for the reagents list.
//! Cursor/keyset pagination was removed: the frontend never used it.

use serde::{Deserialize, Serialize};

// ==================== QUERY PARAMETERS ====================

#[derive(Debug, Deserialize, Clone, Default)]
pub struct HybridPaginationQuery {
    // Page-based
    pub page: Option<i64>,
    pub per_page: Option<i64>,

    // Filters
    pub search: Option<String>,     // Backend naming
    pub q: Option<String>,          // Frontend naming (alias for search)
    pub status: Option<String>,
    pub manufacturer: Option<String>,
    pub has_stock: Option<bool>,
    pub room_id: Option<String>,
    pub zone_id: Option<String>,
    pub position_id: Option<String>,
    /// Include reagents with visibility='hidden'. Defaults to false.
    pub include_hidden: Option<bool>,

    // Sorting
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

impl HybridPaginationQuery {
    pub fn normalize(&self) -> (i64, i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(50).clamp(1, 100);
        let offset = (page - 1) * per_page;
        (page, per_page, offset)
    }

    pub fn sort_by(&self) -> &str {
        self.sort_by.as_deref().unwrap_or("total_quantity")
    }

    pub fn sort_order(&self) -> &str {
        self.sort_order.as_deref().unwrap_or("DESC")
    }

    /// Search term (supports both `search` and `q` parameters)
    pub fn get_search(&self) -> Option<&str> {
        self.search.as_deref()
            .or(self.q.as_deref())
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
    }
}

// ==================== RESPONSE STRUCTURES ====================

#[derive(Debug, Serialize)]
pub struct HybridPaginationInfo {
    pub total: i64,
    pub page: Option<i64>,
    pub per_page: i64,
    pub total_pages: Option<i64>,
    pub has_next: bool,
    pub has_prev: bool,
}

impl HybridPaginationInfo {
    pub fn from_page(total: i64, page: i64, per_page: i64) -> Self {
        let total_pages = (total + per_page - 1) / per_page;
        Self {
            total,
            page: Some(page),
            per_page,
            total_pages: Some(total_pages),
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SortingInfo {
    pub sort_by: String,
    pub sort_order: String,
}

#[derive(Debug, Serialize)]
pub struct HybridPaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: HybridPaginationInfo,
    pub sorting: SortingInfo,
}

// ==================== QUERY BUILDER ====================

/// Builds the filtered COUNT query and the LIMIT/OFFSET page query.
///
/// Filter params are shared by both queries, so COUNT and the page
/// always see exactly the same WHERE clause.
pub struct CtePaginationBuilder {
    table: String,
    select_columns: String,
    sort_column: String,
    sort_order: String,
    conditions: Vec<String>,
    filter_params: Vec<String>,
    limit: i64,
}

impl CtePaginationBuilder {
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            select_columns: "*".to_string(),
            sort_column: "total_quantity".to_string(),
            sort_order: "DESC".to_string(),
            conditions: Vec::new(),
            filter_params: Vec::new(),
            limit: 50,
        }
    }

    pub fn select(mut self, columns: &str) -> Self {
        self.select_columns = columns.to_string();
        self
    }

    /// `column` must come from a whitelist (see `ReagentSortWhitelist`).
    pub fn sort(mut self, column: &str, order: &str) -> Self {
        self.sort_column = column.to_string();
        self.sort_order = if order.to_uppercase() == "ASC" { "ASC".to_string() } else { "DESC".to_string() };
        self
    }

    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = limit;
        self
    }

    pub fn add_condition(&mut self, condition: &str, param: String) -> &mut Self {
        self.conditions.push(condition.to_string());
        self.filter_params.push(param);
        self
    }

    pub fn add_raw_condition(&mut self, condition: &str) -> &mut Self {
        self.conditions.push(condition.to_string());
        self
    }

    /// Adds a condition with several params (e.g. multi-column LIKE search)
    pub fn add_search(&mut self, condition: &str, params: Vec<String>) -> &mut Self {
        self.conditions.push(condition.to_string());
        self.filter_params.extend(params);
        self
    }

    /// COUNT query (filters only, no limit)
    pub fn build_count(&self) -> (String, Vec<String>) {
        let mut sql = format!("SELECT COUNT(*) FROM {}", self.table);

        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.conditions.join(" AND "));
        }

        (sql, self.filter_params.clone())
    }

    /// Page query with LIMIT/OFFSET
    pub fn build_simple(&self, offset: i64) -> (String, Vec<String>) {
        let secondary_order = if self.sort_order == "DESC" { "DESC" } else { "ASC" };

        let where_clause = if self.conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", self.conditions.join(" AND "))
        };

        let sql = format!(r#"
            SELECT {select_cols}
            FROM {table}
            {where_clause}
            ORDER BY {sort_col} {sort_order}, id {secondary_order}
            LIMIT ? OFFSET ?
        "#,
                          select_cols = self.select_columns,
                          table = self.table,
                          where_clause = where_clause,
                          sort_col = self.sort_column,
                          sort_order = self.sort_order,
                          secondary_order = secondary_order,
        );

        let mut all_params = self.filter_params.clone();
        all_params.push(self.limit.to_string());
        all_params.push(offset.to_string());

        (sql.trim().to_string(), all_params)
    }
}

// ==================== SORT FIELD WHITELIST ====================

pub struct ReagentSortWhitelist;

impl ReagentSortWhitelist {
    /// Allowed sort fields: (api_field, sql_column)
    const ALLOWED: &'static [(&'static str, &'static str)] = &[
        ("name", "name"),
        ("formula", "formula"),
        ("cas_number", "cas_number"),
        ("manufacturer", "manufacturer"),
        ("status", "status"),
        ("created_at", "created_at"),
        ("updated_at", "updated_at"),
        ("total_quantity", "total_quantity"),
        ("batches_count", "batches_count"),
        ("molecular_weight", "molecular_weight"),
    ];

    pub fn validate(field: &str) -> &'static str {
        Self::ALLOWED
            .iter()
            .find(|(api, _)| *api == field)
            .map(|(_, sql)| *sql)
            .unwrap_or("total_quantity")
    }

    pub fn validate_order(order: &str) -> &'static str {
        match order.to_uppercase().as_str() {
            "ASC" => "ASC",
            _ => "DESC",
        }
    }
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_simple() {
        let mut builder = CtePaginationBuilder::new("reagents")
            .sort("total_quantity", "DESC")
            .limit(50);

        builder.add_condition("status = ?", "active".to_string());

        let (sql, params) = builder.build_simple(0);

        assert!(sql.contains("ORDER BY total_quantity DESC"));
        assert!(sql.contains("LIMIT ? OFFSET ?"));
        assert_eq!(params.len(), 3); // status, limit, offset
    }

    #[test]
    fn test_builder_with_search() {
        let mut builder = CtePaginationBuilder::new("reagents")
            .sort("total_quantity", "DESC")
            .limit(50);

        builder.add_search(
            "(name LIKE ? OR formula LIKE ?)",
            vec!["%test%".to_string(), "%test%".to_string()]
        );

        let (count_sql, count_params) = builder.build_count();
        assert!(count_sql.contains("name LIKE ?"));
        assert_eq!(count_params.len(), 2); // 2 search params

        let (_sql, params) = builder.build_simple(0);
        assert_eq!(params.len(), 4); // 2 search + limit + offset
    }

    #[test]
    fn test_sort_whitelist() {
        assert_eq!(ReagentSortWhitelist::validate("total_quantity"), "total_quantity");
        assert_eq!(ReagentSortWhitelist::validate("name"), "name");
        assert_eq!(ReagentSortWhitelist::validate("invalid_field"), "total_quantity");
        assert_eq!(ReagentSortWhitelist::validate("name; DROP TABLE users"), "total_quantity");
        assert_eq!(ReagentSortWhitelist::validate_order("asc"), "ASC");
        assert_eq!(ReagentSortWhitelist::validate_order("DESC; --"), "DESC");
    }

    #[test]
    fn test_query_get_search() {
        let query = HybridPaginationQuery {
            search: Some("acetone".to_string()),
            ..Default::default()
        };
        assert_eq!(query.get_search(), Some("acetone"));

        let query2 = HybridPaginationQuery {
            q: Some("benzene".to_string()),
            ..Default::default()
        };
        assert_eq!(query2.get_search(), Some("benzene"));

        let query3 = HybridPaginationQuery {
            search: Some("   ".to_string()),
            ..Default::default()
        };
        assert_eq!(query3.get_search(), None);
    }
}
