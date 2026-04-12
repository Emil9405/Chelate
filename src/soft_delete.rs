// src/soft_delete.rs
//! Универсальный модуль soft delete с каскадированием и транзакциями.
//!
//! Предоставляет переиспользуемые building blocks для любой сущности:
//! reagents, batches, equipment и т.д.
//!
//! ## Архитектура
//!
//! 1. **Inner functions** (`_inner`) — работают с `&mut SqliteConnection` (для транзакций)
//! 2. **Public wrappers** — принимают `&SqlitePool` для standalone вызовов
//! 3. **SoftDeleteChain** — builder, выполняет все шаги в одной транзакции
//! 4. **Domain helpers** — готовые функции (reagent, batch, equipment)
//! 5. **Counting helpers** — подсчёт активных placements для storage
//!
//! ## Добавление soft delete для новой сущности
//!
//! 1. Добавить таблицу/колонки в `KNOWN_TABLES` / `KNOWN_COLUMNS`
//! 2. Добавить `deleted_at DATETIME` в миграцию
//! 3. Описать каскад через `SoftDeleteChain`
//! 4. Добавить `AND deleted_at IS NULL` во все SELECT запросы
//!
//! ## Примеры
//!
//! ```rust
//! // Reagent — полный каскад через готовую функцию:
//! soft_delete::delete_reagent(pool, &id, &user_id).await?;
//!
//! // Equipment — готовая функция:
//! soft_delete::delete_equipment(pool, &id, &user_id).await?;
//!
//! // Кастомный каскад для новой сущности:
//! let result = SoftDeleteChain::new(pool)
//!     .cascade_hard_delete("child_records", "parent_id", &id)
//!     .mark_deleted("parents", &id, &user_id)
//!     .execute()  // ← выполняется в транзакции
//!     .await?;
//! ```

use sqlx::{SqliteConnection, SqlitePool};
use crate::error::{ApiError, ApiResult};

// ============================================================
//                    WHITELIST VALIDATION
// ============================================================

const KNOWN_TABLES: &[&str] = &[
    "reagents",
    "batches",
    "batch_containers",
    "batch_placements",
    "equipment",
    "equipment_parts",
    "equipment_maintenance",
    "equipment_files",
    "equipment_fts",
    "experiments",
    "experiment_reagents",
    "storage_positions",
    "storage_zones",
    "rooms",
];

const KNOWN_COLUMNS: &[&str] = &[
    "id",
    "reagent_id",
    "batch_id",
    "container_id",
    "equipment_id",
    "position_id",
    "zone_id",
    "room_id",
    "experiment_id",
];

fn validate_identifier(name: &str, allowed: &[&str], kind: &str) -> ApiResult<()> {
    if allowed.contains(&name) {
        Ok(())
    } else {
        Err(ApiError::bad_request(&format!(
            "Unknown {} in soft_delete: '{}'. Add it to the whitelist.",
            kind, name
        )))
    }
}

// ============================================================
//                    CASCADE RESULT
// ============================================================

/// Результат каскадного удаления
#[derive(Debug, Default, Clone)]
pub struct CascadeResult {
    pub total_affected: u64,
    pub steps: Vec<(String, u64)>,
}

impl CascadeResult {
    fn add(&mut self, label: String, rows: u64) {
        self.total_affected += rows;
        self.steps.push((label, rows));
    }
}

impl std::fmt::Display for CascadeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parts: Vec<String> = self.steps.iter()
            .filter(|(_, rows)| *rows > 0)
            .map(|(label, rows)| format!("{}: {}", label, rows))
            .collect();
        write!(f, "{}", parts.join(", "))
    }
}

// ============================================================
//          INNER FUNCTIONS (работают с &mut SqliteConnection)
// ============================================================
//
// Все SQL-операции реализованы как _inner функции, принимающие
// &mut SqliteConnection. Это позволяет:
// - SoftDeleteChain::execute() передавать &mut *tx (транзакция)
// - Публичные обёртки вызывать через pool.acquire()

async fn mark_deleted_inner(
    conn: &mut SqliteConnection,
    table: &str,
    id: &str,
    user_id: &str,
) -> ApiResult<u64> {
    validate_identifier(table, KNOWN_TABLES, "table")?;
    let sql = format!(
        "UPDATE {} SET deleted_at = datetime('now'), updated_by = ? \
         WHERE id = ? AND deleted_at IS NULL", table
    );
    Ok(sqlx::query(&sql).bind(user_id).bind(id)
        .execute(&mut *conn).await?.rows_affected())
}

async fn mark_deleted_with_status_inner(
    conn: &mut SqliteConnection,
    table: &str,
    id: &str,
    user_id: &str,
    status: &str,
) -> ApiResult<u64> {
    validate_identifier(table, KNOWN_TABLES, "table")?;
    let sql = format!(
        "UPDATE {} SET deleted_at = datetime('now'), updated_by = ?, status = ? \
         WHERE id = ? AND deleted_at IS NULL", table
    );
    Ok(sqlx::query(&sql).bind(user_id).bind(status).bind(id)
        .execute(&mut *conn).await?.rows_affected())
}

async fn cascade_mark_deleted_inner(
    conn: &mut SqliteConnection,
    child_table: &str,
    fk_column: &str,
    parent_id: &str,
    user_id: &str,
) -> ApiResult<u64> {
    validate_identifier(child_table, KNOWN_TABLES, "table")?;
    validate_identifier(fk_column, KNOWN_COLUMNS, "column")?;
    let sql = format!(
        "UPDATE {} SET deleted_at = datetime('now'), updated_by = ? \
         WHERE {} = ? AND deleted_at IS NULL", child_table, fk_column
    );
    Ok(sqlx::query(&sql).bind(user_id).bind(parent_id)
        .execute(&mut *conn).await?.rows_affected())
}

async fn cascade_set_status_inner(
    conn: &mut SqliteConnection,
    child_table: &str,
    fk_column: &str,
    parent_id: &str,
    new_status: &str,
) -> ApiResult<u64> {
    validate_identifier(child_table, KNOWN_TABLES, "table")?;
    validate_identifier(fk_column, KNOWN_COLUMNS, "column")?;
    let sql = format!(
        "UPDATE {} SET status = ?, updated_at = datetime('now') \
         WHERE {} = ? AND status != ?", child_table, fk_column
    );
    Ok(sqlx::query(&sql).bind(new_status).bind(parent_id).bind(new_status)
        .execute(&mut *conn).await?.rows_affected())
}

async fn cascade_hard_delete_inner(
    conn: &mut SqliteConnection,
    child_table: &str,
    fk_column: &str,
    parent_id: &str,
) -> ApiResult<u64> {
    validate_identifier(child_table, KNOWN_TABLES, "table")?;
    validate_identifier(fk_column, KNOWN_COLUMNS, "column")?;
    let sql = format!("DELETE FROM {} WHERE {} = ?", child_table, fk_column);
    Ok(sqlx::query(&sql).bind(parent_id)
        .execute(&mut *conn).await?.rows_affected())
}

/// 2-уровневый каскад: DELETE через subquery
async fn cascade_delete_via_inner(
    conn: &mut SqliteConnection,
    target_table: &str, target_fk: &str,
    via_table: &str, via_id_column: &str, via_fk: &str,
    parent_id: &str,
) -> ApiResult<u64> {
    validate_identifier(target_table, KNOWN_TABLES, "table")?;
    validate_identifier(target_fk, KNOWN_COLUMNS, "column")?;
    validate_identifier(via_table, KNOWN_TABLES, "table")?;
    validate_identifier(via_id_column, KNOWN_COLUMNS, "column")?;
    validate_identifier(via_fk, KNOWN_COLUMNS, "column")?;
    let sql = format!(
        "DELETE FROM {} WHERE {} IN (SELECT {} FROM {} WHERE {} = ?)",
        target_table, target_fk, via_id_column, via_table, via_fk
    );
    Ok(sqlx::query(&sql).bind(parent_id)
        .execute(&mut *conn).await?.rows_affected())
}

/// 2-уровневый каскад: UPDATE status через subquery
async fn cascade_set_status_via_inner(
    conn: &mut SqliteConnection,
    target_table: &str, target_fk: &str,
    via_table: &str, via_id_column: &str, via_fk: &str,
    parent_id: &str, new_status: &str,
) -> ApiResult<u64> {
    validate_identifier(target_table, KNOWN_TABLES, "table")?;
    validate_identifier(target_fk, KNOWN_COLUMNS, "column")?;
    validate_identifier(via_table, KNOWN_TABLES, "table")?;
    validate_identifier(via_id_column, KNOWN_COLUMNS, "column")?;
    validate_identifier(via_fk, KNOWN_COLUMNS, "column")?;
    let sql = format!(
        "UPDATE {} SET status = ?, updated_at = datetime('now') \
         WHERE {} IN (SELECT {} FROM {} WHERE {} = ?) AND status != ?",
        target_table, target_fk, via_id_column, via_table, via_fk
    );
    Ok(sqlx::query(&sql).bind(new_status).bind(parent_id).bind(new_status)
        .execute(&mut *conn).await?.rows_affected())
}

/// 3-уровневый каскад: DELETE через JOIN в subquery
///
/// ```sql
/// DELETE FROM {target} WHERE {target_fk} IN (
///     SELECT v1.{v1_id} FROM {v1_table} v1
///     JOIN {v2_table} v2 ON v1.{v1_join} = v2.{v2_join}
///     WHERE v2.{v2_fk} = ?
/// )
/// ```
async fn cascade_delete_via_join_inner(
    conn: &mut SqliteConnection,
    target_table: &str, target_fk: &str,
    v1_table: &str, v1_id: &str, v1_join_col: &str,
    v2_table: &str, v2_join_col: &str, v2_fk: &str,
    parent_id: &str,
) -> ApiResult<u64> {
    validate_identifier(target_table, KNOWN_TABLES, "table")?;
    validate_identifier(target_fk, KNOWN_COLUMNS, "column")?;
    validate_identifier(v1_table, KNOWN_TABLES, "table")?;
    validate_identifier(v1_id, KNOWN_COLUMNS, "column")?;
    validate_identifier(v1_join_col, KNOWN_COLUMNS, "column")?;
    validate_identifier(v2_table, KNOWN_TABLES, "table")?;
    validate_identifier(v2_join_col, KNOWN_COLUMNS, "column")?;
    validate_identifier(v2_fk, KNOWN_COLUMNS, "column")?;
    let sql = format!(
        "DELETE FROM {target} WHERE {tfk} IN (\
            SELECT v1.{v1id} FROM {v1} v1 \
            JOIN {v2} v2 ON v1.{v1j} = v2.{v2j} \
            WHERE v2.{v2f} = ?)",
        target = target_table, tfk = target_fk,
        v1id = v1_id, v1 = v1_table, v1j = v1_join_col,
        v2 = v2_table, v2j = v2_join_col, v2f = v2_fk,
    );
    Ok(sqlx::query(&sql).bind(parent_id)
        .execute(&mut *conn).await?.rows_affected())
}

/// 3-уровневый каскад: UPDATE status через JOIN в subquery
async fn cascade_set_status_via_join_inner(
    conn: &mut SqliteConnection,
    target_table: &str, target_fk: &str,
    v1_table: &str, v1_id: &str, v1_join_col: &str,
    v2_table: &str, v2_join_col: &str, v2_fk: &str,
    parent_id: &str, new_status: &str,
) -> ApiResult<u64> {
    validate_identifier(target_table, KNOWN_TABLES, "table")?;
    validate_identifier(target_fk, KNOWN_COLUMNS, "column")?;
    validate_identifier(v1_table, KNOWN_TABLES, "table")?;
    validate_identifier(v1_id, KNOWN_COLUMNS, "column")?;
    validate_identifier(v1_join_col, KNOWN_COLUMNS, "column")?;
    validate_identifier(v2_table, KNOWN_TABLES, "table")?;
    validate_identifier(v2_join_col, KNOWN_COLUMNS, "column")?;
    validate_identifier(v2_fk, KNOWN_COLUMNS, "column")?;
    let sql = format!(
        "UPDATE {target} SET status = ?, updated_at = datetime('now') \
         WHERE {tfk} IN (\
            SELECT v1.{v1id} FROM {v1} v1 \
            JOIN {v2} v2 ON v1.{v1j} = v2.{v2j} \
            WHERE v2.{v2f} = ?) AND status != ?",
        target = target_table, tfk = target_fk,
        v1id = v1_id, v1 = v1_table, v1j = v1_join_col,
        v2 = v2_table, v2j = v2_join_col, v2f = v2_fk,
    );
    Ok(sqlx::query(&sql).bind(new_status).bind(parent_id).bind(new_status)
        .execute(&mut *conn).await?.rows_affected())
}

// ============================================================
//         PUBLIC WRAPPERS (принимают &SqlitePool)
// ============================================================
//
// Для standalone вызовов без chain, когда транзакция не нужна.

/// SET deleted_at + updated_by на одной записи
pub async fn mark_deleted(pool: &SqlitePool, table: &str, id: &str, user_id: &str) -> ApiResult<u64> {
    let mut conn = pool.acquire().await.map_err(|e| ApiError::internal(&e.to_string()))?;
    mark_deleted_inner(&mut conn, table, id, user_id).await
}

/// SET deleted_at + updated_by + status на одной записи
pub async fn mark_deleted_with_status(pool: &SqlitePool, table: &str, id: &str, user_id: &str, status: &str) -> ApiResult<u64> {
    let mut conn = pool.acquire().await.map_err(|e| ApiError::internal(&e.to_string()))?;
    mark_deleted_with_status_inner(&mut conn, table, id, user_id, status).await
}

/// Каскадный soft delete дочерних записей по FK
pub async fn cascade_mark_deleted(pool: &SqlitePool, child_table: &str, fk_column: &str, parent_id: &str, user_id: &str) -> ApiResult<u64> {
    let mut conn = pool.acquire().await.map_err(|e| ApiError::internal(&e.to_string()))?;
    cascade_mark_deleted_inner(&mut conn, child_table, fk_column, parent_id, user_id).await
}

/// Каскадное обновление статуса дочерних записей по FK
pub async fn cascade_set_status(pool: &SqlitePool, child_table: &str, fk_column: &str, parent_id: &str, new_status: &str) -> ApiResult<u64> {
    let mut conn = pool.acquire().await.map_err(|e| ApiError::internal(&e.to_string()))?;
    cascade_set_status_inner(&mut conn, child_table, fk_column, parent_id, new_status).await
}

/// Каскадный hard delete дочерних записей по FK
pub async fn cascade_hard_delete(pool: &SqlitePool, child_table: &str, fk_column: &str, parent_id: &str) -> ApiResult<u64> {
    let mut conn = pool.acquire().await.map_err(|e| ApiError::internal(&e.to_string()))?;
    cascade_hard_delete_inner(&mut conn, child_table, fk_column, parent_id).await
}

/// 2-уровневый каскад: DELETE через subquery
pub async fn cascade_delete_via(pool: &SqlitePool, target_table: &str, target_fk: &str, via_table: &str, via_id_column: &str, via_fk: &str, parent_id: &str) -> ApiResult<u64> {
    let mut conn = pool.acquire().await.map_err(|e| ApiError::internal(&e.to_string()))?;
    cascade_delete_via_inner(&mut conn, target_table, target_fk, via_table, via_id_column, via_fk, parent_id).await
}

/// 2-уровневый каскад: UPDATE status через subquery
pub async fn cascade_set_status_via(pool: &SqlitePool, target_table: &str, target_fk: &str, via_table: &str, via_id_column: &str, via_fk: &str, parent_id: &str, new_status: &str) -> ApiResult<u64> {
    let mut conn = pool.acquire().await.map_err(|e| ApiError::internal(&e.to_string()))?;
    cascade_set_status_via_inner(&mut conn, target_table, target_fk, via_table, via_id_column, via_fk, parent_id, new_status).await
}

/// 3-уровневый каскад: DELETE через JOIN
pub async fn cascade_delete_via_join(
    pool: &SqlitePool,
    target_table: &str, target_fk: &str,
    v1_table: &str, v1_id: &str, v1_join_col: &str,
    v2_table: &str, v2_join_col: &str, v2_fk: &str,
    parent_id: &str,
) -> ApiResult<u64> {
    let mut conn = pool.acquire().await.map_err(|e| ApiError::internal(&e.to_string()))?;
    cascade_delete_via_join_inner(&mut conn, target_table, target_fk, v1_table, v1_id, v1_join_col, v2_table, v2_join_col, v2_fk, parent_id).await
}

/// 3-уровневый каскад: UPDATE status через JOIN
pub async fn cascade_set_status_via_join(
    pool: &SqlitePool,
    target_table: &str, target_fk: &str,
    v1_table: &str, v1_id: &str, v1_join_col: &str,
    v2_table: &str, v2_join_col: &str, v2_fk: &str,
    parent_id: &str, new_status: &str,
) -> ApiResult<u64> {
    let mut conn = pool.acquire().await.map_err(|e| ApiError::internal(&e.to_string()))?;
    cascade_set_status_via_join_inner(&mut conn, target_table, target_fk, v1_table, v1_id, v1_join_col, v2_table, v2_join_col, v2_fk, parent_id, new_status).await
}

// ============================================================
//          SOFT DELETE CHAIN (BUILDER + TRANSACTION)
// ============================================================

enum ChainStep {
    MarkDeleted {
        table: &'static str, id: String, user_id: String,
    },
    MarkDeletedWithStatus {
        table: &'static str, id: String, user_id: String, status: &'static str,
    },
    CascadeMarkDeleted {
        child_table: &'static str, fk_column: &'static str,
        parent_id: String, user_id: String,
    },
    CascadeSetStatus {
        child_table: &'static str, fk_column: &'static str,
        parent_id: String, new_status: &'static str,
    },
    CascadeHardDelete {
        child_table: &'static str, fk_column: &'static str,
        parent_id: String,
    },
    CascadeDeleteVia {
        target_table: &'static str, target_fk: &'static str,
        via_table: &'static str, via_id: &'static str, via_fk: &'static str,
        parent_id: String,
    },
    CascadeSetStatusVia {
        target_table: &'static str, target_fk: &'static str,
        via_table: &'static str, via_id: &'static str, via_fk: &'static str,
        parent_id: String, new_status: &'static str,
    },
    CascadeDeleteViaJoin {
        target_table: &'static str, target_fk: &'static str,
        v1_table: &'static str, v1_id: &'static str, v1_join_col: &'static str,
        v2_table: &'static str, v2_join_col: &'static str, v2_fk: &'static str,
        parent_id: String,
    },
    CascadeSetStatusViaJoin {
        target_table: &'static str, target_fk: &'static str,
        v1_table: &'static str, v1_id: &'static str, v1_join_col: &'static str,
        v2_table: &'static str, v2_join_col: &'static str, v2_fk: &'static str,
        parent_id: String, new_status: &'static str,
    },
}

/// Декларативный builder для цепочки каскадного soft delete.
///
/// Все шаги выполняются **в одной транзакции** — либо все успешно,
/// либо все откатываются. Шаги выполняются в порядке добавления.
pub struct SoftDeleteChain<'a> {
    pool: &'a SqlitePool,
    steps: Vec<ChainStep>,
}

impl<'a> SoftDeleteChain<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool, steps: Vec::new() }
    }

    /// Пометить запись deleted_at
    pub fn mark_deleted(mut self, table: &'static str, id: &str, user_id: &str) -> Self {
        self.steps.push(ChainStep::MarkDeleted {
            table, id: id.into(), user_id: user_id.into(),
        });
        self
    }

    /// Пометить запись deleted_at + сменить status
    pub fn mark_deleted_with_status(
        mut self, table: &'static str, id: &str, user_id: &str, status: &'static str,
    ) -> Self {
        self.steps.push(ChainStep::MarkDeletedWithStatus {
            table, id: id.into(), user_id: user_id.into(), status,
        });
        self
    }

    /// Каскадный soft delete дочерних записей по FK
    pub fn cascade_soft_delete(
        mut self, child_table: &'static str, fk_column: &'static str,
        parent_id: &str, user_id: &str,
    ) -> Self {
        self.steps.push(ChainStep::CascadeMarkDeleted {
            child_table, fk_column, parent_id: parent_id.into(), user_id: user_id.into(),
        });
        self
    }

    /// Каскадное обновление статуса
    pub fn cascade_set_status(
        mut self, child_table: &'static str, fk_column: &'static str,
        parent_id: &str, new_status: &'static str,
    ) -> Self {
        self.steps.push(ChainStep::CascadeSetStatus {
            child_table, fk_column, parent_id: parent_id.into(), new_status,
        });
        self
    }

    /// Каскадный hard delete
    pub fn cascade_hard_delete(
        mut self, child_table: &'static str, fk_column: &'static str, parent_id: &str,
    ) -> Self {
        self.steps.push(ChainStep::CascadeHardDelete {
            child_table, fk_column, parent_id: parent_id.into(),
        });
        self
    }

    /// 2-уровневый каскад: delete через subquery
    pub fn cascade_delete_via(
        mut self,
        target_table: &'static str, target_fk: &'static str,
        via_table: &'static str, via_id: &'static str, via_fk: &'static str,
        parent_id: &str,
    ) -> Self {
        self.steps.push(ChainStep::CascadeDeleteVia {
            target_table, target_fk, via_table, via_id, via_fk, parent_id: parent_id.into(),
        });
        self
    }

    /// 2-уровневый каскад: update status через subquery
    pub fn cascade_set_status_via(
        mut self,
        target_table: &'static str, target_fk: &'static str,
        via_table: &'static str, via_id: &'static str, via_fk: &'static str,
        parent_id: &str, new_status: &'static str,
    ) -> Self {
        self.steps.push(ChainStep::CascadeSetStatusVia {
            target_table, target_fk, via_table, via_id, via_fk,
            parent_id: parent_id.into(), new_status,
        });
        self
    }

    /// 3-уровневый каскад: delete через JOIN (target ← v1 ← v2)
    pub fn cascade_delete_via_join(
        mut self,
        target_table: &'static str, target_fk: &'static str,
        v1_table: &'static str, v1_id: &'static str, v1_join_col: &'static str,
        v2_table: &'static str, v2_join_col: &'static str, v2_fk: &'static str,
        parent_id: &str,
    ) -> Self {
        self.steps.push(ChainStep::CascadeDeleteViaJoin {
            target_table, target_fk,
            v1_table, v1_id, v1_join_col,
            v2_table, v2_join_col, v2_fk,
            parent_id: parent_id.into(),
        });
        self
    }

    /// 3-уровневый каскад: update status через JOIN (target ← v1 ← v2)
    pub fn cascade_set_status_via_join(
        mut self,
        target_table: &'static str, target_fk: &'static str,
        v1_table: &'static str, v1_id: &'static str, v1_join_col: &'static str,
        v2_table: &'static str, v2_join_col: &'static str, v2_fk: &'static str,
        parent_id: &str, new_status: &'static str,
    ) -> Self {
        self.steps.push(ChainStep::CascadeSetStatusViaJoin {
            target_table, target_fk,
            v1_table, v1_id, v1_join_col,
            v2_table, v2_join_col, v2_fk,
            parent_id: parent_id.into(), new_status,
        });
        self
    }

    /// Выполнить все шаги каскада в одной транзакции.
    ///
    /// Если любой шаг fails — все предыдущие откатываются.
    pub async fn execute(self) -> ApiResult<CascadeResult> {
        let mut tx = self.pool.begin().await
            .map_err(|e| ApiError::internal(&format!("Transaction begin failed: {}", e)))?;
        let mut result = CascadeResult::default();

        for step in self.steps {
            match step {
                ChainStep::MarkDeleted { table, id, user_id } => {
                    let rows = mark_deleted_inner(&mut *tx, table, &id, &user_id).await?;
                    result.add(format!("soft_delete {}", table), rows);
                }
                ChainStep::MarkDeletedWithStatus { table, id, user_id, status } => {
                    let rows = mark_deleted_with_status_inner(&mut *tx, table, &id, &user_id, status).await?;
                    result.add(format!("soft_delete {} → {}", table, status), rows);
                }
                ChainStep::CascadeMarkDeleted { child_table, fk_column, parent_id, user_id } => {
                    let rows = cascade_mark_deleted_inner(&mut *tx, child_table, fk_column, &parent_id, &user_id).await?;
                    result.add(format!("cascade soft_delete {}", child_table), rows);
                }
                ChainStep::CascadeSetStatus { child_table, fk_column, parent_id, new_status } => {
                    let rows = cascade_set_status_inner(&mut *tx, child_table, fk_column, &parent_id, new_status).await?;
                    result.add(format!("cascade status {} → {}", child_table, new_status), rows);
                }
                ChainStep::CascadeHardDelete { child_table, fk_column, parent_id } => {
                    let rows = cascade_hard_delete_inner(&mut *tx, child_table, fk_column, &parent_id).await?;
                    result.add(format!("cascade delete {}", child_table), rows);
                }
                ChainStep::CascadeDeleteVia { target_table, target_fk, via_table, via_id, via_fk, parent_id } => {
                    let rows = cascade_delete_via_inner(&mut *tx, target_table, target_fk, via_table, via_id, via_fk, &parent_id).await?;
                    result.add(format!("delete {} via {}", target_table, via_table), rows);
                }
                ChainStep::CascadeSetStatusVia { target_table, target_fk, via_table, via_id, via_fk, parent_id, new_status } => {
                    let rows = cascade_set_status_via_inner(&mut *tx, target_table, target_fk, via_table, via_id, via_fk, &parent_id, new_status).await?;
                    result.add(format!("status {} via {} → {}", target_table, via_table, new_status), rows);
                }
                ChainStep::CascadeDeleteViaJoin { target_table, target_fk, v1_table, v1_id, v1_join_col, v2_table, v2_join_col, v2_fk, parent_id } => {
                    let rows = cascade_delete_via_join_inner(&mut *tx, target_table, target_fk, v1_table, v1_id, v1_join_col, v2_table, v2_join_col, v2_fk, &parent_id).await?;
                    result.add(format!("delete {} via {}⇢{}", target_table, v1_table, v2_table), rows);
                }
                ChainStep::CascadeSetStatusViaJoin { target_table, target_fk, v1_table, v1_id, v1_join_col, v2_table, v2_join_col, v2_fk, parent_id, new_status } => {
                    let rows = cascade_set_status_via_join_inner(&mut *tx, target_table, target_fk, v1_table, v1_id, v1_join_col, v2_table, v2_join_col, v2_fk, &parent_id, new_status).await?;
                    result.add(format!("status {} via {}⇢{} → {}", target_table, v1_table, v2_table, new_status), rows);
                }
            }
        }

        tx.commit().await
            .map_err(|e| ApiError::internal(&format!("Transaction commit failed: {}", e)))?;

        Ok(result)
    }
}

// ============================================================
//              DOMAIN: REAGENT (готовая цепочка)
// ============================================================

/// Soft delete реагента с полным каскадом (в транзакции):
/// placements → containers(disposed) → batches(deleted) → reagent(deleted+inactive)
///
/// SQL для шага 1 (placements):
/// ```sql
/// DELETE FROM batch_placements WHERE container_id IN (
///     SELECT v1.id FROM batch_containers v1
///     JOIN batches v2 ON v1.batch_id = v2.id
///     WHERE v2.reagent_id = ?
/// )
/// ```
pub async fn delete_reagent(
    pool: &SqlitePool,
    reagent_id: &str,
    user_id: &str,
) -> ApiResult<CascadeResult> {
    let result = SoftDeleteChain::new(pool)
        // 1. placements: batch_placements ← batch_containers ← batches WHERE reagent_id
        .cascade_delete_via_join(
            "batch_placements", "container_id",   // target
            "batch_containers", "id", "batch_id", // v1: bc.id, ON bc.batch_id =
            "batches", "id", "reagent_id",        // v2: b.id, WHERE b.reagent_id = ?
            reagent_id,
        )
        // 2. containers → disposed: через JOIN batches WHERE reagent_id
        .cascade_set_status_via_join(
            "batch_containers", "id",
            "batch_containers", "id", "batch_id",
            "batches", "id", "reagent_id",
            reagent_id, "disposed",
        )
        // 3. batches → soft delete (прямой FK)
        .cascade_soft_delete("batches", "reagent_id", reagent_id, user_id)
        // 4. reagent → soft delete + inactive
        .mark_deleted_with_status("reagents", reagent_id, user_id, "inactive")
        .execute()
        .await?;

    log::info!("🗑️ Reagent {} soft-deleted [{}]", reagent_id, result);
    Ok(result)
}

/// Soft delete батча с каскадом (в транзакции):
/// placements → containers(disposed) → batch(deleted)
pub async fn delete_batch(
    pool: &SqlitePool,
    batch_id: &str,
    user_id: &str,
) -> ApiResult<CascadeResult> {
    let result = SoftDeleteChain::new(pool)
        // 1. placements: batch_placements ← batch_containers WHERE batch_id
        .cascade_delete_via(
            "batch_placements", "container_id",
            "batch_containers", "id", "batch_id",
            batch_id,
        )
        // 2. containers → disposed
        .cascade_set_status("batch_containers", "batch_id", batch_id, "disposed")
        // 3. batch → soft delete
        .mark_deleted("batches", batch_id, user_id)
        .execute()
        .await?;

    log::info!("🗑️ Batch {} soft-deleted [{}]", batch_id, result);
    Ok(result)
}

// ============================================================
//              DOMAIN: EQUIPMENT (готовая цепочка)
// ============================================================
//
// Требует миграции:
//   ALTER TABLE equipment ADD COLUMN deleted_at DATETIME
//
// Дочерние таблицы (parts, maintenance, files) НЕ удаляются —
// они становятся невидимыми через JOIN на equipment.deleted_at IS NULL.
// При restore (deleted_at = NULL) вся история возвращается.
//
// Единственное исключение — equipment_fts: FTS-поиск не фильтрует
// по deleted_at, поэтому записи удаляются hard.

/// Soft delete оборудования (в транзакции):
/// fts(hard) → equipment(deleted+retired)
///
/// Дочерние parts/maintenance/files сохраняются для возможного restore.
/// ⚠️ Физические файлы с диска нужно удалять отдельно, если restore не планируется.
pub async fn delete_equipment(
    pool: &SqlitePool,
    equipment_id: &str,
    user_id: &str,
) -> ApiResult<CascadeResult> {
    let result = SoftDeleteChain::new(pool)
        // FTS — hard delete, чтобы удалённое не появлялось в поиске
        .cascade_hard_delete("equipment_fts", "equipment_id", equipment_id)
        // equipment → soft delete + retired
        .mark_deleted_with_status("equipment", equipment_id, user_id, "retired")
        .execute()
        .await?;

    log::info!("🗑️ Equipment {} soft-deleted [{}]", equipment_id, result);
    Ok(result)
}

// ============================================================
//        ACTIVE PLACEMENT COUNTING (для storage)
// ============================================================

/// Активные предметы на позиции (live batch + non-disposed container)
pub async fn count_active_placements_for_position(
    pool: &SqlitePool,
    position_id: &str,
) -> ApiResult<i64> {
    let (count,): (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*) FROM batch_placements bp
           JOIN batch_containers bc ON bp.container_id = bc.id
           JOIN batches b ON bc.batch_id = b.id
           WHERE bp.position_id = ?
             AND b.deleted_at IS NULL
             AND bc.status != 'disposed'"#
    ).bind(position_id).fetch_one(pool).await?;
    Ok(count)
}

/// Активные предметы в зоне
pub async fn count_active_placements_for_zone(
    pool: &SqlitePool,
    zone_id: &str,
) -> ApiResult<i64> {
    let (count,): (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*) FROM batch_placements bp
           JOIN batch_containers bc ON bp.container_id = bc.id
           JOIN batches b ON bc.batch_id = b.id
           WHERE bp.position_id IN (
               SELECT id FROM storage_positions WHERE zone_id = ?
           )
           AND b.deleted_at IS NULL
           AND bc.status != 'disposed'"#
    ).bind(zone_id).fetch_one(pool).await?;
    Ok(count)
}

/// Активные предметы в комнате
pub async fn count_active_placements_for_room(
    pool: &SqlitePool,
    room_id: &str,
) -> ApiResult<i64> {
    let (count,): (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*) FROM batch_placements bp
           JOIN batch_containers bc ON bp.container_id = bc.id
           JOIN batches b ON bc.batch_id = b.id
           JOIN storage_positions sp ON bp.position_id = sp.id
           JOIN storage_zones sz ON sp.zone_id = sz.id
           WHERE sz.room_id = ?
             AND b.deleted_at IS NULL
             AND bc.status != 'disposed'"#
    ).bind(room_id).fetch_one(pool).await?;
    Ok(count)
}

/// Подсчёт по каждой позиции — для hierarchy view (HashMap<position_id, count>)
pub async fn count_active_placements_by_position(
    pool: &SqlitePool,
) -> ApiResult<std::collections::HashMap<String, i64>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"SELECT bp.position_id, COUNT(DISTINCT bc.batch_id)
           FROM batch_placements bp
           JOIN batch_containers bc ON bp.container_id = bc.id
           JOIN batches b ON bc.batch_id = b.id
           WHERE b.deleted_at IS NULL
             AND bc.status != 'disposed'
           GROUP BY bp.position_id"#
    ).fetch_all(pool).await?;
    Ok(rows.into_iter().collect())
}

// ============================================================
//                   CLEANUP UTILITIES
// ============================================================

/// Очистка осиротевших placements и контейнеров.
///
/// Безопасно вызывать при старте или по расписанию.
/// Ошибки НЕ проглатываются — вызывающий код должен их обработать.
pub async fn cleanup_orphaned(pool: &SqlitePool) -> ApiResult<CascadeResult> {
    let mut result = CascadeResult::default();

    let r1 = sqlx::query(
        "DELETE FROM batch_placements WHERE container_id IN \
         (SELECT id FROM batch_containers WHERE status = 'disposed')"
    ).execute(pool).await?;
    result.add("placements → disposed containers".into(), r1.rows_affected());

    let r2 = sqlx::query(
        "DELETE FROM batch_placements WHERE container_id IN ( \
            SELECT bc.id FROM batch_containers bc \
            JOIN batches b ON bc.batch_id = b.id \
            WHERE b.deleted_at IS NOT NULL)"
    ).execute(pool).await?;
    result.add("placements → deleted batches".into(), r2.rows_affected());

    let r3 = sqlx::query(
        "UPDATE batch_containers SET status = 'disposed', updated_at = datetime('now') \
         WHERE batch_id IN (SELECT id FROM batches WHERE deleted_at IS NOT NULL) \
           AND status != 'disposed'"
    ).execute(pool).await?;
    result.add("containers → disposed".into(), r3.rows_affected());

    if result.total_affected > 0 {
        log::info!("🧹 Orphan cleanup: {}", result);
    }

    Ok(result)
}

// ============================================================
//                  RESTORE (отмена soft delete)
// ============================================================

/// Восстановить одну запись: снимает deleted_at, возвращает статус
pub async fn restore_record(
    pool: &SqlitePool,
    table: &str,
    id: &str,
    user_id: &str,
    active_status: &str,
) -> ApiResult<u64> {
    validate_identifier(table, KNOWN_TABLES, "table")?;
    let sql = format!(
        "UPDATE {} SET deleted_at = NULL, updated_by = ?, status = ?, \
         updated_at = datetime('now') WHERE id = ? AND deleted_at IS NOT NULL",
        table
    );
    Ok(sqlx::query(&sql)
        .bind(user_id).bind(active_status).bind(id)
        .execute(pool).await?.rows_affected())
}

/// Восстановить реагент с каскадным restore батчей + пересчёт кэша
pub async fn restore_reagent(
    pool: &SqlitePool,
    reagent_id: &str,
    user_id: &str,
) -> ApiResult<u64> {
    // 1. Restore самого реагента
    let rows = restore_record(pool, "reagents", reagent_id, user_id, "active").await?;
    if rows == 0 {
        return Err(ApiError::not_found("Archived reagent"));
    }

    // 2. Restore батчей этого реагента
    sqlx::query(
        "UPDATE batches SET deleted_at = NULL, updated_by = ?, status = 'available', \
         updated_at = datetime('now') WHERE reagent_id = ? AND deleted_at IS NOT NULL"
    )
    .bind(user_id).bind(reagent_id)
    .execute(pool).await?;

    // 3. Вернуть контейнеры из disposed → sealed
    //    (только те, что были disposed при удалении реагента)
    sqlx::query(
        "UPDATE batch_containers SET status = 'sealed', updated_at = datetime('now') \
         WHERE batch_id IN (SELECT id FROM batches WHERE reagent_id = ?) \
         AND status = 'disposed'"
    )
    .bind(reagent_id)
    .execute(pool).await?;

    // 4. Пересчитать кэш реагента
    crate::reagent_handlers::refresh_reagent_cache(pool, reagent_id).await?;

    log::info!("♻️ Reagent {} restored by {}", reagent_id, user_id);
    Ok(rows)
}

/// Восстановить оборудование
pub async fn restore_equipment(
    pool: &SqlitePool,
    equipment_id: &str,
    user_id: &str,
) -> ApiResult<u64> {
    let rows = restore_record(pool, "equipment", equipment_id, user_id, "available").await?;
    if rows == 0 {
        return Err(ApiError::not_found("Archived equipment"));
    }

    // FTS пересоздаётся автоматически через триггеры при изменении equipment,
    // но запись была удалена hard — нужна ре-индексация.
    // Безопасный вариант: пересоздать FTS-запись из текущих данных
    let _ = sqlx::query(
        "INSERT OR REPLACE INTO equipment_fts(equipment_id, name, type_, manufacturer, model, serial_number, location, description) \
         SELECT id, name, type_, manufacturer, model, serial_number, location, description \
         FROM equipment WHERE id = ?"
    )
    .bind(equipment_id)
    .execute(pool).await; // .ok() — FTS может не существовать

    log::info!("♻️ Equipment {} restored by {}", equipment_id, user_id);
    Ok(rows)
}

// ============================================================
//                  HARD DELETE (окончательное)
// ============================================================

/// Окончательно удалить запись из БД.
///
/// ⚠️ Удаляет ТОЛЬКО записи с `deleted_at IS NOT NULL` (уже soft-deleted).
/// FK каскады (`ON DELETE CASCADE`) автоматически удалят дочерние записи.
pub async fn hard_delete_record(
    pool: &SqlitePool,
    table: &str,
    id: &str,
) -> ApiResult<u64> {
    validate_identifier(table, KNOWN_TABLES, "table")?;
    let sql = format!(
        "DELETE FROM {} WHERE id = ? AND deleted_at IS NOT NULL",
        table
    );
    Ok(sqlx::query(&sql).bind(id).execute(pool).await?.rows_affected())
}

// ============================================================
//              PERFORMANCE INDEXES (для db.rs)
// ============================================================

/// SQL-запросы для создания индексов, ускоряющих soft delete фильтрацию.
/// Вызывать из `ensure_performance_indexes()` в db.rs.
pub const SOFT_DELETE_INDEXES: &[&str] = &[
    // Partial indexes — покрывают WHERE deleted_at IS NULL (самые частые запросы)
    "CREATE INDEX IF NOT EXISTS idx_batches_active ON batches(id) WHERE deleted_at IS NULL",
    "CREATE INDEX IF NOT EXISTS idx_reagents_active ON reagents(id) WHERE deleted_at IS NULL",
    "CREATE INDEX IF NOT EXISTS idx_equipment_active ON equipment(id) WHERE deleted_at IS NULL",
    // Status index для batch_containers (фильтр status != 'disposed')
    "CREATE INDEX IF NOT EXISTS idx_batch_containers_status ON batch_containers(status)",
];
