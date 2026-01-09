// src/query_builders/fts/mod.rs
//! Full-Text Search построитель запросов
//! ✅ ИСПРАВЛЕНО: Заменено .id на .rowid для FTS таблиц (обязательно для SQLite)

pub mod config;

use sqlx::SqlitePool;

pub struct FtsQueryBuilder;

impl FtsQueryBuilder {
    /// Проверить доступность FTS таблицы для реагентов
    pub async fn check_fts_available(pool: &SqlitePool) -> bool {
        let result: Result<(i64,), _> = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='reagents_fts'"
        ).fetch_one(pool).await;
        matches!(result, Ok((count,)) if count > 0)
    }

    /// Проверить доступность произвольной FTS таблицы
    pub async fn check_fts_table_available(pool: &SqlitePool, fts_table: &str) -> bool {
        let query = format!(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
            fts_table.replace('\'', "''") // Экранирование для безопасности
        );
        let result: Result<(i64,), _> = sqlx::query_as(&query).fetch_one(pool).await;
        matches!(result, Ok((count,)) if count > 0)
    }

    /// Построить FTS запрос (очистка и форматирование)
    pub fn build_fts_query(search: &str) -> String {
        // Удаляем спецсимволы, которые могут сломать синтаксис FTS
        let cleaned = search
            .chars()
            .filter(|c| !matches!(c, '(' | ')' | '*' | '"' | ':' | '^' | '-' | '+' | '~' | '&' | '|'))
            .collect::<String>();
        
        // Разбиваем на слова и добавляем * к каждому (поиск по началу слова)
        cleaned
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|word| format!("{}*", word))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Публичная обертка для эскейпинга
    pub fn escape_fts_query(query: &str) -> String {
        Self::build_fts_query(query)
    }

    /// Построить общее условие поиска (FTS или LIKE)
    pub fn build_search_condition(
        search: &str,
        use_fts: bool,
        fts_table: &str,
        like_fields: &[&str],
        table_alias: &str,
    ) -> (String, Vec<String>) {
        let search_trimmed = search.trim();
        if search_trimmed.is_empty() {
            return (String::new(), Vec::new());
        }

        if use_fts {
            let fts_query = Self::build_fts_query(search_trimmed);
            if fts_query.is_empty() {
                return (String::new(), Vec::new());
            }
            
            // ✅ ИСПРАВЛЕНИЕ: Используем rowid для связи с FTS
            // main_table.rowid = fts_table.rowid
            let condition = format!(
                "{}.rowid IN (SELECT rowid FROM {} WHERE {} MATCH ?)",
                table_alias, fts_table, fts_table
            );
            (condition, vec![fts_query])
        } else {
            // Fallback на LIKE
            let pattern = format!("%{}%", search_trimmed);
            
            if like_fields.is_empty() {
                return (String::new(), Vec::new());
            }
            
            let conditions: Vec<String> = like_fields
                .iter()
                .map(|f| format!("{}.{} LIKE ?", table_alias, f))
                .collect();
            
            let params: Vec<String> = like_fields
                .iter()
                .map(|_| pattern.clone())
                .collect();
            
            (format!("({})", conditions.join(" OR ")), params)
        }
    }

    /// Специализированный построитель для реагентов (поиск по реагентам + партиям)
    pub fn build_reagent_search_condition(
        search: &str,
        use_fts: bool,
        table_alias: &str,
    ) -> (String, Vec<String>) {
        let search_trimmed = search.trim();
        if search_trimmed.is_empty() {
            return (String::new(), Vec::new());
        }

        let pattern = format!("%{}%", search_trimmed);

        if use_fts {
            let fts_query = Self::build_fts_query(search_trimmed);
            if fts_query.is_empty() {
                return (String::new(), Vec::new());
            }
            
            // ✅ ИСПРАВЛЕНИЕ: 
            // 1. Для FTS используем .rowid (так как это виртуальная таблица)
            // 2. Для связи с batches используем .id (так как там внешний ключ reagent_id ссылается на UUID)
            let condition = format!(
                "({}.rowid IN (SELECT rowid FROM reagents_fts WHERE reagents_fts MATCH ?) \
                 OR EXISTS (SELECT 1 FROM batches bs WHERE bs.reagent_id = {}.id AND \
                 (bs.batch_number LIKE ? OR bs.cat_number LIKE ? OR bs.supplier LIKE ?)))",
                table_alias, table_alias
            );
            
            (condition, vec![fts_query, pattern.clone(), pattern.clone(), pattern])
        } else {
            // Обычный LIKE поиск по всем полям
            let condition = format!(
                "({}.name LIKE ? OR {}.formula LIKE ? OR {}.cas_number LIKE ? OR {}.manufacturer LIKE ? \
                 OR EXISTS (SELECT 1 FROM batches bs WHERE bs.reagent_id = {}.id AND \
                 (bs.batch_number LIKE ? OR bs.cat_number LIKE ? OR bs.supplier LIKE ?)))",
                table_alias, table_alias, table_alias, table_alias, table_alias
            );
            
            (condition, vec![
                pattern.clone(), pattern.clone(), pattern.clone(), pattern.clone(), // Поля реагента
                pattern.clone(), pattern.clone(), pattern // Поля партии
            ])
        }
    }
}

/// Глобальная функция-хелпер, если она используется в других местах напрямую
pub fn escape_fts_query(query: &str) -> String {
    FtsQueryBuilder::build_fts_query(query)
}