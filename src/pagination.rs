// src/pagination.rs
//! Гибридная пагинация: page-based + cursor-based (keyset)
//! Оптимизировано для 270,000+ записей

use serde::{Deserialize, Serialize};

// ==================== CURSOR ENCODING ====================
// Используем простой hex encoding без внешних зависимостей

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn from_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Кодирует cursor: (sort_value, id) -> hex
pub fn encode_cursor(sort_value: f64, id: &str) -> String {
    let raw = format!("{}|{}", sort_value, id);
    to_hex(raw.as_bytes())
}

/// Декодирует cursor: hex -> (sort_value, id)
pub fn decode_cursor(cursor: &str) -> Option<(f64, String)> {
    let bytes = from_hex(cursor)?;
    let raw = String::from_utf8(bytes).ok()?;
    let mut parts = raw.splitn(2, '|');

    let value: f64 = parts.next()?.parse().ok()?;
    let id = parts.next()?.to_string();

    Some((value, id))
}

/// Кодирует cursor для created_at: (timestamp_micros, id) -> hex
pub fn encode_cursor_datetime(timestamp_micros: i64, id: &str) -> String {
    let raw = format!("{}|{}", timestamp_micros, id);
    to_hex(raw.as_bytes())
}

/// Декодирует cursor для created_at
pub fn decode_cursor_datetime(cursor: &str) -> Option<(i64, String)> {
    let bytes = from_hex(cursor)?;
    let raw = String::from_utf8(bytes).ok()?;
    let mut parts = raw.splitn(2, '|');

    let ts: i64 = parts.next()?.parse().ok()?;
    let id = parts.next()?.to_string();

    Some((ts, id))
}

// ==================== QUERY PARAMETERS ====================

#[derive(Debug, Deserialize, Clone)]
pub struct HybridPaginationQuery {
    // Page-based
    pub page: Option<i64>,
    pub per_page: Option<i64>,

    // Cursor-based
    pub cursor: Option<String>,
    pub direction: Option<String>,  // "next" | "prev"

    // Filters
    pub search: Option<String>,     // Backend naming
    pub q: Option<String>,          // Frontend naming (alias for search)
    pub status: Option<String>,
    pub manufacturer: Option<String>,
    pub has_stock: Option<bool>,
    pub room_id: Option<String>,
    pub zone_id: Option<String>,
    pub position_id: Option<String>,
    /// Включать ли скрытые реагенты (visibility='hidden'). Дефолт false.
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

    pub fn is_cursor_mode(&self) -> bool {
        self.cursor.is_some()
    }

    pub fn direction(&self) -> &str {
        self.direction.as_deref().unwrap_or("next")
    }

    pub fn sort_by(&self) -> &str {
        self.sort_by.as_deref().unwrap_or("total_quantity")
    }

    pub fn sort_order(&self) -> &str {
        self.sort_order.as_deref().unwrap_or("DESC")
    }

    pub fn is_desc(&self) -> bool {
        self.sort_order().to_uppercase() == "DESC"
    }

    /// Получить поисковый запрос (поддержка обоих параметров: search и q)
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
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
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
            next_cursor: None,
            prev_cursor: None,
        }
    }

    pub fn from_cursor(
        total: i64,
        per_page: i64,
        has_next: bool,
        has_prev: bool,
        next_cursor: Option<String>,
        prev_cursor: Option<String>,
    ) -> Self {
        Self {
            total,
            page: None,
            per_page,
            total_pages: None,
            has_next,
            has_prev,
            next_cursor,
            prev_cursor,
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

// ==================== CTE PAGINATION BUILDER ====================

/// Построитель SQL запросов с CTE для Deferred Join паттерна
///
/// Генерирует запрос вида:
/// ```sql
/// WITH ids AS (
///     SELECT id, total_quantity
///     FROM reagents
///     WHERE <filters> AND <keyset_condition>
///     ORDER BY total_quantity DESC, id ASC
///     LIMIT 51
/// )
/// SELECT r.* FROM reagents r
/// INNER JOIN ids ON r.id = ids.id
/// ORDER BY ids.total_quantity DESC, ids.id ASC
/// ```
pub struct CtePaginationBuilder {
    table: String,
    select_columns: String,
    sort_column: String,
    sort_order: String,
    conditions: Vec<String>,
    filter_params: Vec<String>,      // Параметры для фильтров (используются в COUNT)
    keyset_params: Vec<String>,       // Параметры для keyset (НЕ используются в COUNT)
    limit: i64,
    keyset_condition: Option<String>,
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
            keyset_params: Vec::new(),
            limit: 50,
            keyset_condition: None,
        }
    }

    pub fn select(mut self, columns: &str) -> Self {
        self.select_columns = columns.to_string();
        self
    }

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

    /// Добавляет условие с множественными параметрами (для LIKE поиска)
    pub fn add_search(&mut self, condition: &str, params: Vec<String>) -> &mut Self {
        self.conditions.push(condition.to_string());
        self.filter_params.extend(params);
        self
    }

    /// Добавляет keyset условие для cursor-based пагинации
    ///
    /// Для DESC: (sort_col, id) < (cursor_value, cursor_id)
    /// Для ASC: (sort_col, id) > (cursor_value, cursor_id)
    pub fn keyset_after(&mut self, cursor_value: f64, cursor_id: &str, is_desc: bool, direction: &str) -> &mut Self {
        // Определяем операторы в зависимости от направления и порядка
        let (op1, op2) = match (direction, is_desc) {
            ("prev", true)  => (">", ">"),  // DESC + prev = идём к большим значениям
            ("prev", false) => ("<", "<"),  // ASC + prev = идём к меньшим
            (_, true)       => ("<", "<"),  // DESC + next = идём к меньшим
            (_, false)      => (">", ">"),  // ASC + next = идём к большим
        };

        self.keyset_condition = Some(format!(
            "(({} {} ?) OR ({} = ? AND id {} ?))",
            self.sort_column, op1, self.sort_column, op2
        ));
        self.keyset_params.push(cursor_value.to_string());
        self.keyset_params.push(cursor_value.to_string());
        self.keyset_params.push(cursor_id.to_string());
        self
    }

    /// Строит COUNT запрос (без keyset, без limit)
    pub fn build_count(&self) -> (String, Vec<String>) {
        let mut sql = format!("SELECT COUNT(*) FROM {}", self.table);

        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.conditions.join(" AND "));
        }

        // Возвращаем только параметры для фильтров (без keyset)
        (sql, self.filter_params.clone())
    }

    /// Строит основной запрос с CTE для Deferred Join
    pub fn build_cte(&self, direction: &str, is_desc: bool) -> (String, Vec<String>) {
        // Определяем порядок сортировки
        let order_dir = if direction == "prev" {
            if is_desc { "ASC" } else { "DESC" }
        } else {
            &self.sort_order
        };

        let secondary_order = if order_dir == "DESC" { "DESC" } else { "ASC" };

        // Собираем WHERE
        let mut where_parts = self.conditions.clone();
        if let Some(ref keyset) = self.keyset_condition {
            where_parts.push(keyset.clone());
        }

        let where_clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_parts.join(" AND "))
        };

        // CTE запрос
        let sql = format!(r#"
            WITH ids AS (
                SELECT id, {sort_col}
                FROM {table}
                {where_clause}
                ORDER BY {sort_col} {order_dir}, id {secondary_order}
                LIMIT ?
            )
            SELECT {select_cols}
            FROM {table} r
            INNER JOIN ids ON r.id = ids.id
            ORDER BY ids.{sort_col} {order_dir}, ids.id {secondary_order}
        "#,
                          sort_col = self.sort_column,
                          table = self.table,
                          where_clause = where_clause,
                          order_dir = order_dir,
                          secondary_order = secondary_order,
                          select_cols = self.select_columns.replace("*", "r.*"),
        );

        // Собираем все параметры: filter + keyset + limit
        let mut all_params = self.filter_params.clone();
        all_params.extend(self.keyset_params.clone());
        all_params.push((self.limit + 1).to_string()); // +1 для определения has_more

        (sql.trim().to_string(), all_params)
    }

    /// Строит простой запрос без CTE (для offset-based)
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
    /// Whitelist разрешённых полей сортировки: (api_field, sql_column)
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

    /// Проверяет, поддерживает ли поле keyset pagination
    pub fn supports_keyset(field: &str) -> bool {
        matches!(field, "total_quantity" | "created_at" | "name" | "batches_count")
    }
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_encode_decode() {
        let original = "hello world";
        let hex = to_hex(original.as_bytes());
        let decoded = from_hex(&hex).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), original);
    }

    #[test]
    fn test_cursor_encode_decode() {
        let encoded = encode_cursor(123.45, "abc-123");
        let (value, id) = decode_cursor(&encoded).unwrap();

        assert!((value - 123.45).abs() < 0.001);
        assert_eq!(id, "abc-123");
    }

    #[test]
    fn test_cursor_datetime() {
        let ts = 1704067200000000i64; // 2024-01-01
        let encoded = encode_cursor_datetime(ts, "xyz-789");
        let (decoded_ts, decoded_id) = decode_cursor_datetime(&encoded).unwrap();

        assert_eq!(decoded_ts, ts);
        assert_eq!(decoded_id, "xyz-789");
    }

    #[test]
    fn test_cte_builder_simple() {
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
    fn test_cte_builder_with_keyset() {
        let mut builder = CtePaginationBuilder::new("reagents")
            .sort("total_quantity", "DESC")
            .limit(50);

        builder
            .add_condition("status = ?", "active".to_string())
            .keyset_after(100.5, "abc-123", true, "next");

        let (sql, params) = builder.build_cte("next", true);

        assert!(sql.contains("WITH ids AS"));
        assert!(sql.contains("INNER JOIN ids"));
        assert_eq!(params.len(), 5); // status + 3 keyset + limit
    }

    #[test]
    fn test_cte_builder_with_search() {
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

        let (sql, params) = builder.build_simple(0);
        assert_eq!(params.len(), 4); // 2 search + limit + offset
    }

    #[test]
    fn test_sort_whitelist() {
        assert_eq!(ReagentSortWhitelist::validate("total_quantity"), "total_quantity");
        assert_eq!(ReagentSortWhitelist::validate("name"), "name");
        assert_eq!(ReagentSortWhitelist::validate("invalid_field"), "total_quantity");

        assert!(ReagentSortWhitelist::supports_keyset("total_quantity"));
        assert!(ReagentSortWhitelist::supports_keyset("created_at"));
        assert!(!ReagentSortWhitelist::supports_keyset("formula"));
    }

    #[test]
    fn test_query_get_search() {
        // Test with 'search' parameter
        let query = HybridPaginationQuery {
            page: None,
            per_page: None,
            cursor: None,
            direction: None,
            search: Some("acetone".to_string()),
            q: None,
            status: None,
            manufacturer: None,
            has_stock: None,
            sort_by: None,
            sort_order: None,
        };
        assert_eq!(query.get_search(), Some("acetone"));

        // Test with 'q' parameter (frontend alias)
        let query2 = HybridPaginationQuery {
            page: None,
            per_page: None,
            cursor: None,
            direction: None,
            search: None,
            q: Some("benzene".to_string()),
            status: None,
            manufacturer: None,
            has_stock: None,
            sort_by: None,
            sort_order: None,
        };
        assert_eq!(query2.get_search(), Some("benzene"));

        // Test with empty string
        let query3 = HybridPaginationQuery {
            page: None,
            per_page: None,
            cursor: None,
            direction: None,
            search: Some("   ".to_string()),
            q: None,
            status: None,
            manufacturer: None,
            has_stock: None,
            sort_by: None,
            sort_order: None,
        };
        assert_eq!(query3.get_search(), None);
    }
}