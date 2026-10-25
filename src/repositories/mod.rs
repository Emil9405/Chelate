// src/repositories/mod.rs
//! Репозитории для работы с базой данных (FIXED)

use async_trait::async_trait;
use sqlx::SqlitePool;
use serde::{Serialize, de::DeserializeOwned};
use crate::error::{ApiError, ApiResult};
use crate::handlers::{PaginatedResponse, PaginationQuery};

/// Базовый trait для CRUD операций
#[async_trait]
pub trait CrudRepository<T, CreateDto, UpdateDto>: Send + Sync
where
    T: Serialize + DeserializeOwned + Send + Unpin + for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    CreateDto: Send,
    UpdateDto: Send,
{
    /// Имя таблицы в базе данных
    fn table_name(&self) -> &'static str;

    /// Имя поля с ID (по умолчанию "id")
    fn id_field(&self) -> &'static str {
        "id"
    }

    /// Поля для поиска (FTS)
    fn search_fields(&self) -> Vec<&'static str> {
        vec![]
    }

    /// Поле для сортировки по умолчанию
    fn default_sort_field(&self) -> &'static str {
        "created_at"
    }

    /// Создать новую запись
    async fn create(&self, pool: &SqlitePool, data: CreateDto, user_id: &str) -> ApiResult<T>;

    /// Получить запись по ID
    async fn get_by_id(&self, pool: &SqlitePool, id: &str) -> ApiResult<Option<T>> {
        let query = format!(
            "SELECT * FROM {} WHERE {} = ?",
            self.table_name(),
            self.id_field()
        );

        let result = sqlx::query_as::<_, T>(&query)
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(result)
    }

    /// Обновить запись
    async fn update(&self, pool: &SqlitePool, id: &str, data: UpdateDto, user_id: &str) -> ApiResult<T>;

    /// Удалить запись
    async fn delete(&self, pool: &SqlitePool, id: &str) -> ApiResult<()> {
        let query = format!(
            "DELETE FROM {} WHERE {} = ?",
            self.table_name(),
            self.id_field()
        );

        let result = sqlx::query(&query)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(ApiError::not_found(self.table_name()));
        }

        Ok(())
    }

    /// Получить список с пагинацией
    async fn get_paginated(
        &self,
        pool: &SqlitePool,
        query: &PaginationQuery,
    ) -> ApiResult<PaginatedResponse<T>> {
        use crate::query_builders::{SafeQueryBuilder, CountQueryBuilder};

        let (page, per_page, offset) = query.normalize();
        let search_fields = self.search_fields();

        // === COUNT QUERY ===
        let mut count_builder = CountQueryBuilder::new(self.table_name())
            .map_err(|e| ApiError::bad_request(&e))?;

        // Apply search using LIKE conditions
        if let Some(ref search) = query.search {
            let (condition, params) =
                crate::query_builders::utils::like_any(&search_fields, search);
            if !condition.is_empty() {
                count_builder.add_condition(&condition, params);
            }
        }

        // Apply status filter
        if let Some(ref status) = query.status {
            count_builder.add_exact_match("status", status.as_str());
        }

        // Execute count query
        let (count_sql, count_params) = count_builder.build();
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        for param in &count_params {
            count_query = count_query.bind(param);
        }
        let total: i64 = count_query.fetch_one(pool).await?;

        // === SELECT QUERY ===
        let select_base = format!("SELECT * FROM {}", self.table_name());
        let mut data_builder = SafeQueryBuilder::new(&select_base)
            .map_err(|e| ApiError::bad_request(&e))?;

        // Apply search using LIKE conditions
        if let Some(ref search) = query.search {
            let (condition, params) =
                crate::query_builders::utils::like_any(&search_fields, search);
            if !condition.is_empty() {
                data_builder.add_condition(&condition, params);
            }
        }

        // Apply status filter
        if let Some(ref status) = query.status {
            data_builder.add_exact_match("status", status.as_str());
        }

        // Apply sorting and pagination
        data_builder
            .order_by(self.default_sort_field(), query.sort_order.as_deref().unwrap_or("DESC"))
            .limit(per_page)
            .offset(offset);

        // Execute select query
        let (select_sql, select_params) = data_builder.build();
        let mut select_query = sqlx::query_as::<_, T>(&select_sql);
        for param in &select_params {
            select_query = select_query.bind(param);
        }
        let data: Vec<T> = select_query.fetch_all(pool).await?;

        let total_pages = (total as f64 / per_page as f64).ceil() as i64;

        Ok(PaginatedResponse {
            data,
            total,
            page,
            per_page,
            total_pages,
        })
    }
}

/// Макрос для быстрого создания репозитория
#[macro_export]
macro_rules! impl_basic_repository {
    (
        $repo:ident,
        $entity:ty,
        $table:expr,
        $search_fields:expr
    ) => {
        pub struct $repo;

        impl $repo {
            pub fn new() -> Self {
                Self
            }
        }

        #[async_trait::async_trait]
        impl CrudRepository<$entity, (), ()> for $repo {
            fn table_name(&self) -> &'static str {
                $table
            }

            fn search_fields(&self) -> Vec<&'static str> {
                $search_fields.to_vec()
            }

            async fn create(&self, _pool: &SqlitePool, _data: (), _user_id: &str) -> ApiResult<$entity> {
                unimplemented!("Create method must be implemented")
            }

            async fn update(&self, _pool: &SqlitePool, _id: &str, _data: (), _user_id: &str) -> ApiResult<$entity> {
                unimplemented!("Update method must be implemented")
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_trait() {
        // Базовые тесты будут добавлены при интеграции
    }
}