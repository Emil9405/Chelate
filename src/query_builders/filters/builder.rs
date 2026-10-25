// src/query_builders/filters/builder.rs
//! Безопасный построитель фильтров с защитой от SQL-инъекций
//!
//! ВСЕ значения передаются ТОЛЬКО через sqlx::bind
//! Имена колонок ТОЛЬКО через whitelist
//! Операторы - строго типизированные enum

use sqlx::{QueryBuilder, Sqlite};
use log::warn;

use super::{Filter, FilterGroup, FilterItem, FilterOperator, FilterValue, FieldWhitelist};
use crate::query_builders::utils::{like_contains, like_prefix, like_suffix, like_sql, is_safe_report_field};

/// Построитель фильтров с защитой от SQL-инъекций
pub struct FilterBuilder<'a> {
    whitelist: Option<&'a FieldWhitelist>,
}

impl<'a> FilterBuilder<'a> {
    pub fn new() -> Self {
        Self { whitelist: None }
    }

    pub fn with_whitelist(mut self, whitelist: &'a FieldWhitelist) -> Self {
        self.whitelist = Some(whitelist);
        self
    }

    // ==================== СТРОКОВАЯ ГЕНЕРАЦИЯ (для legacy) ====================

    /// Генерация условия и параметров для строковых запросов
    pub fn build_condition(&self, group: &FilterGroup) -> Result<(String, Vec<String>), String> {
        group.validate(self.whitelist)?;
        self.build_condition_internal(group)
    }

    fn build_condition_internal(&self, group: &FilterGroup) -> Result<(String, Vec<String>), String> {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        for item in &group.items {
            match item {
                FilterItem::Single(filter) => {
                    if !filter.enabled { continue; }
                    let (cond, filter_params) = self.build_single_condition(filter)?;
                    if !cond.is_empty() {
                        conditions.push(cond);
                        params.extend(filter_params);
                    }
                }
                FilterItem::Group(nested) => {
                    let (nested_cond, nested_params) = self.build_condition_internal(nested)?;
                    if !nested_cond.is_empty() {
                        conditions.push(format!("({})", nested_cond));
                        params.extend(nested_params);
                    }
                }
            }
        }

        if conditions.is_empty() {
            return Ok((String::new(), Vec::new()));
        }

        let sql = conditions.join(&format!(" {} ", group.group_type.as_sql()));
        Ok((sql, params))
    }

    fn build_single_condition(&self, filter: &Filter) -> Result<(String, Vec<String>), String> {
        let field = &filter.field;

        // Валидация поля через whitelist
        if !self.is_field_safe(field) {
            warn!("Field '{}' not allowed, skipping", field);
            return Ok((String::new(), Vec::new()));
        }

        match filter.operator {
            FilterOperator::IsNull => Ok((format!("{} IS NULL", field), Vec::new())),
            FilterOperator::IsNotNull => Ok((format!("{} IS NOT NULL", field), Vec::new())),
            
            FilterOperator::Eq | FilterOperator::Neq | FilterOperator::Gt |
            FilterOperator::Gte | FilterOperator::Lt | FilterOperator::Lte => {
                let value = filter.value.as_ref()
                    .ok_or_else(|| format!("Value required for operator {:?}", filter.operator))?;
                let param = value.to_string_value()
                    .map_err(|e| e.to_string())?;
                Ok((format!("{} {} ?", field, filter.operator.as_sql()), vec![param]))
            }
            
            FilterOperator::Like => {
                let value = filter.value.as_ref().ok_or("Value required for LIKE")?;
                let param = value.to_string_value().map_err(|e| e.to_string())?;
                Ok((like_sql(field), vec![like_contains(&param)]))
            }
            
            FilterOperator::StartsWith => {
                let value = filter.value.as_ref().ok_or("Value required for STARTS_WITH")?;
                let param = value.to_string_value().map_err(|e| e.to_string())?;
                Ok((like_sql(field), vec![like_prefix(&param)]))
            }
            
            FilterOperator::EndsWith => {
                let value = filter.value.as_ref().ok_or("Value required for ENDS_WITH")?;
                let param = value.to_string_value().map_err(|e| e.to_string())?;
                Ok((like_sql(field), vec![like_suffix(&param)]))
            }
            
            FilterOperator::In | FilterOperator::NotIn => {
                let values = filter.value.as_ref()
                    .ok_or("Value required for IN/NOT IN")?
                    .as_string_array()
                    .map_err(|e| e.to_string())?;
                
                if values.is_empty() {
                    // IN () - всегда false, NOT IN () - всегда true
                    return Ok((
                        if filter.operator == FilterOperator::In { "1=0" } else { "1=1" }.to_string(),
                        Vec::new()
                    ));
                }
                
                let placeholders: Vec<&str> = (0..values.len()).map(|_| "?").collect();
                let op = if filter.operator == FilterOperator::In { "IN" } else { "NOT IN" };
                Ok((format!("{} {} ({})", field, op, placeholders.join(", ")), values))
            }
            
            FilterOperator::Between | FilterOperator::NotBetween => {
                let (from, to) = filter.value.as_ref()
                    .ok_or("Value required for BETWEEN")?
                    .as_range_strings()
                    .map_err(|e| e.to_string())?;
                let op = if filter.operator == FilterOperator::Between { "BETWEEN" } else { "NOT BETWEEN" };
                Ok((format!("{} {} ? AND ?", field, op), vec![from, to]))
            }
        }
    }

    // ==================== SQLX QUERYBUILDER ИНТЕГРАЦИЯ ====================

    /// Применение фильтров к sqlx::QueryBuilder с биндингом параметров
    pub fn build_condition_with_bindings<'q>(
        &self,
        group: &FilterGroup,
        builder: &mut QueryBuilder<'q, Sqlite>,
        has_where: &mut bool,
        condition_count: &mut usize,
    ) -> Result<bool, String> {
        group.validate(self.whitelist)?;

        let items: Vec<&FilterItem> = group.items.iter()
            .filter(|item| match item {
                FilterItem::Single(f) => f.enabled,
                FilterItem::Group(_) => true
            })
            .collect();

        if items.is_empty() {
            return Ok(false);
        }

        if *has_where {
            builder.push(" AND ");
        } else {
            builder.push(" WHERE ");
            *has_where = true;
        }

        *condition_count += 1;
        builder.push("(");

        let mut first = true;
        for item in items {
            if !first {
                builder.push(&format!(" {} ", group.group_type.as_sql()));
            }
            first = false;

            match item {
                FilterItem::Single(filter) => {
                    self.apply_single_filter(filter, builder)?;
                }
                FilterItem::Group(nested) => {
                    builder.push("(");
                    self.apply_group_recursive(nested, builder)?;
                    builder.push(")");
                }
            }
        }

        builder.push(")");
        Ok(true)
    }

    fn apply_group_recursive(&self, group: &FilterGroup, builder: &mut QueryBuilder<'_, Sqlite>) -> Result<(), String> {
        let items: Vec<&FilterItem> = group.items.iter()
            .filter(|item| match item {
                FilterItem::Single(f) => f.enabled,
                FilterItem::Group(_) => true
            })
            .collect();

        if items.is_empty() {
            builder.push("1=1");
            return Ok(());
        }

        let mut first = true;
        for item in items {
            if !first {
                builder.push(&format!(" {} ", group.group_type.as_sql()));
            }
            first = false;

            match item {
                FilterItem::Single(filter) => {
                    self.apply_single_filter(filter, builder)?;
                }
                FilterItem::Group(nested) => {
                    builder.push("(");
                    self.apply_group_recursive(nested, builder)?;
                    builder.push(")");
                }
            }
        }
        Ok(())
    }

    fn apply_single_filter(&self, filter: &Filter, builder: &mut QueryBuilder<'_, Sqlite>) -> Result<(), String> {
        let field = &filter.field;

        // Валидация поля - КРИТИЧНО для безопасности
        if !self.is_field_safe(field) {
            warn!("Field '{}' not allowed, using 1=1", field);
            builder.push("1=1");
            return Ok(());
        }

        match filter.operator {
            FilterOperator::IsNull => {
                builder.push(field);
                builder.push(" IS NULL");
            }
            FilterOperator::IsNotNull => {
                builder.push(field);
                builder.push(" IS NOT NULL");
            }
            FilterOperator::Eq | FilterOperator::Neq | FilterOperator::Gt |
            FilterOperator::Gte | FilterOperator::Lt | FilterOperator::Lte => {
                let value = filter.value.as_ref()
                    .ok_or_else(|| format!("Value required for {:?}", filter.operator))?;
                builder.push(field);
                builder.push(&format!(" {} ", filter.operator.as_sql()));
                self.push_bind_value(builder, value)?;
            }
            FilterOperator::Like => {
                let value = filter.value.as_ref().ok_or("Value required for LIKE")?;
                let param = value.to_string_value().map_err(|e| e.to_string())?;
                builder.push(field);
                builder.push(" LIKE ");
                builder.push_bind(like_contains(&param));
                builder.push(" ESCAPE '\\'");
            }
            FilterOperator::StartsWith => {
                let value = filter.value.as_ref().ok_or("Value required")?;
                let param = value.to_string_value().map_err(|e| e.to_string())?;
                builder.push(field);
                builder.push(" LIKE ");
                builder.push_bind(like_prefix(&param));
                builder.push(" ESCAPE '\\'");
            }
            FilterOperator::EndsWith => {
                let value = filter.value.as_ref().ok_or("Value required")?;
                let param = value.to_string_value().map_err(|e| e.to_string())?;
                builder.push(field);
                builder.push(" LIKE ");
                builder.push_bind(like_suffix(&param));
                builder.push(" ESCAPE '\\'");
            }
            FilterOperator::In | FilterOperator::NotIn => {
                let values = filter.value.as_ref()
                    .ok_or("Value required for IN/NOT IN")?
                    .as_string_array()
                    .map_err(|e| e.to_string())?;
                
                if values.is_empty() {
                    builder.push(if filter.operator == FilterOperator::In { "1=0" } else { "1=1" });
                    return Ok(());
                }
                
                builder.push(field);
                builder.push(if filter.operator == FilterOperator::In { " IN (" } else { " NOT IN (" });
                
                let mut first_val = true;
                for val in values {
                    if !first_val { builder.push(", "); }
                    first_val = false;
                    builder.push_bind(val);
                }
                builder.push(")");
            }
            FilterOperator::Between | FilterOperator::NotBetween => {
                let (from, to) = filter.value.as_ref()
                    .ok_or("Value required for BETWEEN")?
                    .as_range_strings()
                    .map_err(|e| e.to_string())?;
                
                builder.push(field);
                builder.push(if filter.operator == FilterOperator::Between { " BETWEEN " } else { " NOT BETWEEN " });
                builder.push_bind(from);
                builder.push(" AND ");
                builder.push_bind(to);
            }
        }
        Ok(())
    }

    /// Биндинг значения в зависимости от типа
    fn push_bind_value(&self, builder: &mut QueryBuilder<'_, Sqlite>, value: &FilterValue) -> Result<(), String> {
        match value {
            FilterValue::String(s) => { builder.push_bind(s.clone()); }
            FilterValue::Integer(n) => { builder.push_bind(*n); }
            FilterValue::Float(n) => { builder.push_bind(*n); }
            FilterValue::Boolean(b) => { builder.push_bind(if *b { 1i64 } else { 0i64 }); }
            FilterValue::Null => { builder.push("NULL"); }
            _ => return Err("Cannot bind array/range as single value".to_string()),
        }
        Ok(())
    }

    // ==================== ВСПОМОГАТЕЛЬНЫЕ МЕТОДЫ ====================

    #[inline]
    fn is_field_safe(&self, field: &str) -> bool {
        if let Some(wl) = self.whitelist {
            wl.is_allowed(field)
        } else {
            is_safe_report_field(field)
        }
    }
}

impl Default for FilterBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== ТЕСТЫ ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query_builders::filters::{Filter, FilterGroup, FilterItem, GroupType};

    #[test]
    fn test_filter_builder_simple() {
        let group = FilterGroup::and(vec![
            FilterItem::filter(Filter::eq("status", "active")),
            FilterItem::filter(Filter::gte("quantity", 10.0)),
        ]);
        
        let whitelist = FieldWhitelist::for_batches();
        let builder = FilterBuilder::new().with_whitelist(&whitelist);
        let result = builder.build_condition(&group);
        
        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("status = ?"));
        assert!(sql.contains("quantity >= ?"));
        assert!(sql.contains("AND"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_filter_builder_nested() {
        let group = FilterGroup::and(vec![
            FilterItem::filter(Filter::eq("status", "active")),
            FilterItem::group(FilterGroup::or(vec![
                FilterItem::filter(Filter::gte("quantity", 10.0)),
                FilterItem::filter(Filter::is_null("expiry_date")),
            ])),
        ]);
        
        let whitelist = FieldWhitelist::for_batches();
        let builder = FilterBuilder::new().with_whitelist(&whitelist);
        let result = builder.build_condition(&group);
        
        assert!(result.is_ok());
        let (sql, _) = result.unwrap();
        assert!(sql.contains("AND"));
        assert!(sql.contains("OR"));
        assert!(sql.contains("IS NULL"));
    }

    #[test]
    fn test_filter_builder_in_clause() {
        let group = FilterGroup::and(vec![
            FilterItem::filter(Filter::in_list("status", vec!["active".to_string(), "reserved".to_string()])),
        ]);
        
        let whitelist = FieldWhitelist::for_batches();
        let builder = FilterBuilder::new().with_whitelist(&whitelist);
        let result = builder.build_condition(&group);
        
        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("IN (?, ?)"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_filter_builder_between() {
        let group = FilterGroup::and(vec![
            FilterItem::filter(Filter::between_numbers("quantity", 10.0, 100.0)),
        ]);
        
        let whitelist = FieldWhitelist::for_batches();
        let builder = FilterBuilder::new().with_whitelist(&whitelist);
        let result = builder.build_condition(&group);
        
        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("BETWEEN ? AND ?"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_filter_builder_rejects_invalid_field() {
        let group = FilterGroup::and(vec![
            FilterItem::filter(Filter::eq("password", "secret")),
        ]);
        
        let whitelist = FieldWhitelist::for_batches();
        let builder = FilterBuilder::new().with_whitelist(&whitelist);
        let result = builder.build_condition(&group);
        
        // Должен игнорировать невалидное поле
        assert!(result.is_ok());
        let (sql, _) = result.unwrap();
        assert!(sql.is_empty() || !sql.contains("password"));
    }

    #[test]
    fn test_filter_builder_like_escaping() {
        let group = FilterGroup::and(vec![
            FilterItem::filter(Filter::like("notes", "test%value_special")),
        ]);
        
        let whitelist = FieldWhitelist::for_batches();
        let builder = FilterBuilder::new().with_whitelist(&whitelist);
        let result = builder.build_condition(&group);
        
        assert!(result.is_ok());
        let (_, params) = result.unwrap();
        assert_eq!(params.len(), 1);
        // Проверяем экранирование
        assert!(params[0].contains("\\%"));
        assert!(params[0].contains("\\_"));
    }

    #[test]
    fn test_filter_builder_empty_in() {
        let group = FilterGroup::and(vec![
            FilterItem::filter(Filter::in_list("status", vec![])),
        ]);
        
        let whitelist = FieldWhitelist::for_batches();
        let builder = FilterBuilder::new().with_whitelist(&whitelist);
        let result = builder.build_condition(&group);
        
        assert!(result.is_ok());
        let (sql, params) = result.unwrap();
        assert!(sql.contains("1=0")); // Пустой IN всегда false
        assert!(params.is_empty());
    }
}
