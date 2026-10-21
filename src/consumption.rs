// src/consumption.rs
//! Single write-off path for reagent stock.
//!
//! Every way of using a reagent (container "Use", batch-level use with
//! auto-distribution, dispense by units, experiment consumption) goes through
//! `consume_from_batch`, so they all share:
//!   - one transaction with the write lock taken up front (no lost updates),
//!   - availability checks against containers (physical truth) and reservations,
//!   - FIFO draw from containers: opened first, then sealed, by sequence number,
//!   - one status machine for containers and batches,
//!   - a usage_logs row for every write-off.
//!
//! Audit entries are written by callers *after* commit (they need the request).

use chrono::Utc;
use serde::Serialize;
use sqlx::SqliteConnection;
use uuid::Uuid;

use crate::audit::ChangeSet;
use crate::error::{ApiError, ApiResult};

/// Tolerance for float comparisons of quantities
const QTY_EPS: f64 = 1e-9;
/// A container/batch at or below this quantity counts as empty (unchanged legacy threshold)
const EMPTY_EPS: f64 = 0.001;
/// Batch is low_stock at or below this share of its original quantity
pub const LOW_STOCK_RATIO: f64 = 0.2;

/// Strip float noise (0.30000000000000004 -> 0.3) before storing
fn tidy(x: f64) -> f64 {
    let r = (x * 1e9).round() / 1e9;
    if r.abs() < QTY_EPS { 0.0 } else { r }
}

// ==================== STATUS MACHINE ====================

pub fn compute_container_status(quantity: f64, original_quantity: f64) -> &'static str {
    if quantity <= EMPTY_EPS {
        "empty"
    } else if (quantity - original_quantity).abs() < EMPTY_EPS {
        "full"
    } else {
        "partial"
    }
}

/// Batch status after a quantity change.
/// Depleted when every container is gone or the quantity is effectively zero;
/// an expired batch stays expired; otherwise the 20% low-stock rule.
pub fn compute_batch_status(
    new_quantity: f64,
    original_quantity: f64,
    has_containers: bool,
    active_containers: i64,
    status_before: &str,
) -> &'static str {
    if (has_containers && active_containers == 0) || new_quantity <= EMPTY_EPS {
        "depleted"
    } else if status_before == "expired" {
        "expired"
    } else if original_quantity > 0.0 && new_quantity <= original_quantity * LOW_STOCK_RATIO {
        "low_stock"
    } else {
        "available"
    }
}

// ==================== REQUEST / RESULT ====================

pub struct ConsumeRequest<'a> {
    pub batch_id: &'a str,
    pub quantity: f64,
    pub user_id: Option<&'a str>,
    /// Draw only from this container; None = auto-distribute across containers
    pub container_id: Option<&'a str>,
    /// Auto-distribute only across these containers (e.g. a selected group)
    pub container_ids: Option<&'a [String]>,
    pub purpose: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub experiment_id: Option<&'a str>,
    /// Part of batch.reserved_quantity this write-off fulfils (experiments)
    pub release_reserved: f64,
    /// Experiments may consume stock reserved before it expired
    pub allow_expired: bool,
}

impl<'a> ConsumeRequest<'a> {
    pub fn new(batch_id: &'a str, quantity: f64, user_id: Option<&'a str>) -> Self {
        Self {
            batch_id,
            quantity,
            user_id,
            container_id: None,
            container_ids: None,
            purpose: None,
            notes: None,
            experiment_id: None,
            release_reserved: 0.0,
            allow_expired: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerDraw {
    pub container_id: String,
    pub sequence_number: i64,
    pub quantity_before: f64,
    pub quantity_after: f64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsumeResult {
    pub usage_id: String,
    pub batch_id: String,
    pub reagent_id: String,
    pub reagent_name: String,
    pub batch_number: String,
    pub unit: String,
    pub quantity_used: f64,
    pub batch_quantity_before: f64,
    pub batch_quantity_after: f64,
    pub status_before: String,
    pub status_after: String,
    pub containers: Vec<ContainerDraw>,
}

impl ConsumeResult {
    pub fn audit_changes(&self) -> ChangeSet {
        let mut cs = ChangeSet::new();
        cs.add_f64("quantity", self.batch_quantity_before, self.batch_quantity_after);
        if self.status_before != self.status_after {
            cs.add("status", &self.status_before, &self.status_after);
        }
        for c in &self.containers {
            cs.add_f64(
                &format!("container #{} quantity", c.sequence_number),
                c.quantity_before,
                c.quantity_after,
            );
        }
        cs
    }

    /// Audit text, e.g. `Used 2 g from reagent "Acetone" batch A-1, containers #1, #2 (remaining: 8 g)`
    pub fn describe(&self, context: Option<&str>) -> String {
        let from = match self.containers.len() {
            0 => String::new(),
            1 => format!(", container #{}", self.containers[0].sequence_number),
            _ => format!(
                ", containers {}",
                self.containers.iter()
                    .map(|c| format!("#{}", c.sequence_number))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        let ctx = context.map(|c| format!(" {}", c)).unwrap_or_default();
        format!(
            "Used {} {} from reagent \"{}\" batch {}{}{} (remaining: {} {})",
            self.quantity_used, self.unit, self.reagent_name, self.batch_number,
            from, ctx, self.batch_quantity_after, self.unit
        )
    }
}

// ==================== CORE ====================

#[derive(sqlx::FromRow)]
struct BatchRow {
    reagent_id: String,
    reagent_name: String,
    batch_number: String,
    unit: String,
    quantity: f64,
    original_quantity: f64,
    reserved_quantity: f64,
    status: String,
}

#[derive(sqlx::FromRow)]
struct ContainerRow {
    id: String,
    sequence_number: i64,
    quantity: f64,
    original_quantity: f64,
    is_opened: bool,
    status: String,
}

/// Write off `req.quantity` from a batch. Must run inside a transaction:
/// pass `&mut *tx`. Nothing is committed here.
pub async fn consume_from_batch(
    conn: &mut SqliteConnection,
    req: ConsumeRequest<'_>,
) -> ApiResult<ConsumeResult> {
    if !(req.quantity.is_finite() && req.quantity > 0.0) {
        return Err(ApiError::bad_request("Quantity must be a positive number"));
    }
    let now = Utc::now();

    // 1. Take the write lock first. A transaction whose first statement is a
    //    write holds the lock from here on, so concurrent write-offs serialize
    //    and every read below sees the latest committed state.
    let touched = sqlx::query(
        "UPDATE batches SET updated_at = updated_at WHERE id = ? AND deleted_at IS NULL"
    )
        .bind(req.batch_id)
        .execute(&mut *conn)
        .await?
        .rows_affected();
    if touched == 0 {
        return Err(ApiError::batch_not_found(req.batch_id));
    }

    // 2. Batch state
    let batch: BatchRow = sqlx::query_as(
        "SELECT b.reagent_id, r.name AS reagent_name, b.batch_number, b.unit, b.quantity, \
                COALESCE(b.original_quantity, b.quantity) AS original_quantity, \
                COALESCE(b.reserved_quantity, 0) AS reserved_quantity, b.status \
         FROM batches b JOIN reagents r ON r.id = b.reagent_id \
         WHERE b.id = ? AND r.deleted_at IS NULL"
    )
        .bind(req.batch_id)
        .fetch_optional(&mut *conn)
        .await?
        .ok_or_else(|| ApiError::batch_not_found(req.batch_id))?;

    match batch.status.as_str() {
        "available" | "low_stock" | "in_use" => {}
        "expired" if req.allow_expired => {}
        "expired" => return Err(ApiError::bad_request(&format!(
            "Batch {} is expired and cannot be used", batch.batch_number
        ))),
        "depleted" => return Err(ApiError::bad_request(&format!(
            "Batch {} is depleted", batch.batch_number
        ))),
        other => return Err(ApiError::bad_request(&format!(
            "Batch {} is not available for use (status '{}')", batch.batch_number, other
        ))),
    }

    // 3. Reservations: stock reserved for experiments is not free to use,
    //    except the part this write-off fulfils.
    let release = req.release_reserved.max(0.0).min(batch.reserved_quantity);
    let reserved_for_others = (batch.reserved_quantity - release).max(0.0);
    let usable = batch.quantity - reserved_for_others;
    if req.quantity > usable + QTY_EPS {
        return Err(ApiError::BadRequest(if reserved_for_others > QTY_EPS {
            format!(
                "Insufficient quantity in batch {}: requested {} {}, available {} {} ({} {} reserved for experiments)",
                batch.batch_number, req.quantity, batch.unit, tidy(usable.max(0.0)), batch.unit,
                tidy(reserved_for_others), batch.unit
            )
        } else {
            format!(
                "Insufficient quantity in batch {}: requested {} {}, available {} {}",
                batch.batch_number, req.quantity, batch.unit, tidy(batch.quantity.max(0.0)), batch.unit
            )
        }));
    }

    // 4. Containers (physical truth)
    let (total_containers,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM batch_containers WHERE batch_id = ?"
    )
        .bind(req.batch_id)
        .fetch_one(&mut *conn)
        .await?;

    let mut draws: Vec<ContainerDraw> = Vec::new();

    if total_containers > 0 {
        let candidates: Vec<ContainerRow> = if let Some(cid) = req.container_id {
            let c: ContainerRow = sqlx::query_as(
                "SELECT id, sequence_number, quantity, original_quantity, is_opened, status \
                 FROM batch_containers WHERE id = ? AND batch_id = ?"
            )
                .bind(cid)
                .bind(req.batch_id)
                .fetch_optional(&mut *conn)
                .await?
                .ok_or_else(|| ApiError::not_found("Container"))?;
            if c.status == "empty" || c.status == "disposed" {
                return Err(ApiError::bad_request(&format!(
                    "Container #{} is {}", c.sequence_number, c.status
                )));
            }
            vec![c]
        } else {
            let only = match req.container_ids {
                Some(ids) if !ids.is_empty() => format!(
                    " AND id IN ({})", vec!["?"; ids.len()].join(", ")
                ),
                _ => String::new(),
            };
            let sql = format!(
                "SELECT id, sequence_number, quantity, original_quantity, is_opened, status \
                 FROM batch_containers \
                 WHERE batch_id = ? AND status NOT IN ('empty', 'disposed') AND quantity > 0{} \
                 ORDER BY is_opened DESC, sequence_number ASC",
                only
            );
            let mut q = sqlx::query_as::<_, ContainerRow>(&sql).bind(req.batch_id);
            if let Some(ids) = req.container_ids {
                for id in ids {
                    q = q.bind(id);
                }
            }
            q.fetch_all(&mut *conn).await?
        };

        let in_containers: f64 = candidates.iter().map(|c| c.quantity).sum();
        if req.quantity > in_containers + QTY_EPS {
            return Err(ApiError::BadRequest(match req.container_id {
                Some(_) => format!(
                    "Insufficient quantity in container #{}: requested {} {}, available {} {}",
                    candidates[0].sequence_number, req.quantity, batch.unit,
                    tidy(in_containers), batch.unit
                ),
                None => format!(
                    "Insufficient quantity in {} of batch {}: requested {} {}, available {} {}",
                    if req.container_ids.is_some() { "selected containers" } else { "containers" },
                    batch.batch_number, req.quantity, batch.unit, tidy(in_containers), batch.unit
                ),
            }));
        }

        let mut remaining = req.quantity;
        for c in candidates {
            if remaining <= QTY_EPS {
                break;
            }
            let take = if remaining >= c.quantity - QTY_EPS { c.quantity } else { remaining };
            remaining -= take;

            let after = tidy((c.quantity - take).max(0.0));
            let status = compute_container_status(after, c.original_quantity);

            if c.is_opened {
                sqlx::query(
                    "UPDATE batch_containers SET quantity = ?, status = ?, updated_at = ? WHERE id = ?"
                )
                    .bind(after).bind(status).bind(now).bind(&c.id)
                    .execute(&mut *conn)
                    .await?;
            } else {
                sqlx::query(
                    "UPDATE batch_containers \
                     SET quantity = ?, status = ?, is_opened = 1, opened_at = ?, opened_by = ?, updated_at = ? \
                     WHERE id = ?"
                )
                    .bind(after).bind(status).bind(now).bind(req.user_id).bind(now).bind(&c.id)
                    .execute(&mut *conn)
                    .await?;
            }

            draws.push(ContainerDraw {
                container_id: c.id,
                sequence_number: c.sequence_number,
                quantity_before: c.quantity,
                quantity_after: after,
                status: status.to_string(),
            });
        }
    } else if req.container_id.is_some() {
        return Err(ApiError::not_found("Container"));
    }

    // 5. Batch quantity + status
    let new_quantity = tidy((batch.quantity - req.quantity).max(0.0));
    let active_containers: i64 = if total_containers > 0 {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM batch_containers WHERE batch_id = ? AND status NOT IN ('empty', 'disposed')"
        )
            .bind(req.batch_id)
            .fetch_one(&mut *conn)
            .await?
    } else {
        0
    };
    let status_after = compute_batch_status(
        new_quantity, batch.original_quantity, total_containers > 0, active_containers, &batch.status,
    );

    sqlx::query(
        "UPDATE batches SET quantity = ?, status = ?, \
                reserved_quantity = MAX(0, COALESCE(reserved_quantity, 0) - ?), \
                updated_by = COALESCE(?, updated_by), updated_at = ? \
         WHERE id = ?"
    )
        .bind(new_quantity)
        .bind(status_after)
        .bind(release)
        .bind(req.user_id)
        .bind(now)
        .bind(req.batch_id)
        .execute(&mut *conn)
        .await?;

    // 6. Usage log
    let usage_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO usage_logs \
         (id, reagent_id, batch_id, user_id, experiment_id, quantity_used, unit, purpose, notes, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
        .bind(&usage_id)
        .bind(&batch.reagent_id)
        .bind(req.batch_id)
        .bind(req.user_id)
        .bind(req.experiment_id)
        .bind(req.quantity)
        .bind(&batch.unit)
        .bind(req.purpose.map(|p| p.chars().take(500).collect::<String>()))
        .bind(req.notes)
        .bind(now)
        .execute(&mut *conn)
        .await?;

    Ok(ConsumeResult {
        usage_id,
        batch_id: req.batch_id.to_string(),
        reagent_id: batch.reagent_id,
        reagent_name: batch.reagent_name,
        batch_number: batch.batch_number,
        unit: batch.unit,
        quantity_used: req.quantity,
        batch_quantity_before: batch.quantity,
        batch_quantity_after: new_quantity,
        status_before: batch.status,
        status_after: status_after.to_string(),
        containers: draws,
    })
}

// ==================== EXPERIMENTS ====================

/// Consume one experiment reagent link. Marks it consumed atomically, so two
/// concurrent requests cannot write the same link off twice.
/// Returns Ok(None) if the link was already consumed or has no quantity.
pub async fn consume_experiment_link(
    conn: &mut SqliteConnection,
    link_id: &str,
    batch_id: &str,
    planned_quantity: Option<f64>,
    experiment_id: &str,
    experiment_title: &str,
    user_id: Option<&str>,
) -> ApiResult<Option<ConsumeResult>> {
    let marked = sqlx::query(
        "UPDATE experiment_reagents SET is_consumed = 1 WHERE id = ? AND is_consumed = 0"
    )
        .bind(link_id)
        .execute(&mut *conn)
        .await?
        .rows_affected();
    if marked == 0 {
        return Ok(None);
    }

    let qty = planned_quantity.unwrap_or(0.0);
    if qty <= 0.0 {
        return Ok(None);
    }

    let purpose = format!("Experiment: {}", experiment_title);
    let mut req = ConsumeRequest::new(batch_id, qty, user_id);
    req.purpose = Some(&purpose);
    req.experiment_id = Some(experiment_id);
    req.release_reserved = qty;
    req.allow_expired = true;

    consume_from_batch(conn, req).await.map(Some)
}

/// Pending (not yet consumed) reagent links of an experiment
#[derive(Debug, sqlx::FromRow)]
pub struct PendingLink {
    pub id: String,
    pub batch_id: String,
    pub planned_quantity: Option<f64>,
}

pub async fn pending_experiment_links(
    conn: &mut SqliteConnection,
    experiment_id: &str,
) -> ApiResult<Vec<PendingLink>> {
    Ok(sqlx::query_as(
        "SELECT id, batch_id, planned_quantity FROM experiment_reagents \
         WHERE experiment_id = ? AND is_consumed = 0 ORDER BY created_at"
    )
        .bind(experiment_id)
        .fetch_all(&mut *conn)
        .await?)
}

/// Consume all pending links of an experiment (interactive completion).
/// Any failure aborts the whole operation: the caller's transaction is not committed.
pub async fn consume_experiment(
    conn: &mut SqliteConnection,
    experiment_id: &str,
    experiment_title: &str,
    user_id: Option<&str>,
) -> ApiResult<Vec<ConsumeResult>> {
    let links = pending_experiment_links(&mut *conn, experiment_id).await?;
    let mut results = Vec::new();
    for link in links {
        let res = consume_experiment_link(
            &mut *conn, &link.id, &link.batch_id, link.planned_quantity,
            experiment_id, experiment_title, user_id,
        )
            .await
            .map_err(|e| match e {
                ApiError::BadRequest(msg) => ApiError::BadRequest(format!(
                    "Cannot complete experiment: {}", msg
                )),
                other => other,
            })?;
        if let Some(r) = res {
            results.push(r);
        }
    }
    Ok(results)
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        for ddl in [
            "CREATE TABLE reagents (id TEXT PRIMARY KEY, name TEXT, deleted_at TEXT)",
            "CREATE TABLE batches (id TEXT PRIMARY KEY, reagent_id TEXT, batch_number TEXT, unit TEXT, \
             quantity REAL, original_quantity REAL, reserved_quantity REAL DEFAULT 0, status TEXT, \
             deleted_at TEXT, updated_by TEXT, updated_at TEXT)",
            "CREATE TABLE batch_containers (id TEXT PRIMARY KEY, batch_id TEXT, sequence_number INTEGER, \
             quantity REAL, original_quantity REAL, is_opened INTEGER DEFAULT 0, opened_at TEXT, \
             opened_by TEXT, status TEXT, updated_at TEXT)",
            "CREATE TABLE usage_logs (id TEXT PRIMARY KEY, reagent_id TEXT, batch_id TEXT, user_id TEXT, \
             experiment_id TEXT, quantity_used REAL CHECK(quantity_used > 0), unit TEXT, purpose TEXT, \
             notes TEXT, created_at TEXT)",
            "CREATE TABLE experiment_reagents (id TEXT PRIMARY KEY, experiment_id TEXT, batch_id TEXT, \
             planned_quantity REAL, is_consumed INTEGER DEFAULT 0, created_at TEXT)",
            "INSERT INTO reagents VALUES ('r1', 'Acetone', NULL)",
        ] {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }
        pool
    }

    async fn add_batch(pool: &SqlitePool, id: &str, qty: f64, reserved: f64, status: &str) {
        sqlx::query("INSERT INTO batches VALUES (?, 'r1', ?, 'g', ?, ?, ?, ?, NULL, NULL, NULL)")
            .bind(id).bind(format!("B-{}", id)).bind(qty).bind(qty).bind(reserved).bind(status)
            .execute(pool).await.unwrap();
    }

    async fn add_container(pool: &SqlitePool, id: &str, batch: &str, seq: i64, qty: f64, opened: bool) {
        let status = if opened { "partial" } else { "full" };
        sqlx::query("INSERT INTO batch_containers VALUES (?, ?, ?, ?, 1.0, ?, NULL, NULL, ?, NULL)")
            .bind(id).bind(batch).bind(seq).bind(qty).bind(opened).bind(status)
            .execute(pool).await.unwrap();
    }

    async fn container_qty(pool: &SqlitePool, id: &str) -> f64 {
        sqlx::query_scalar("SELECT quantity FROM batch_containers WHERE id = ?")
            .bind(id).fetch_one(pool).await.unwrap()
    }

    async fn batch_state(pool: &SqlitePool, id: &str) -> (f64, String, f64) {
        sqlx::query_as("SELECT quantity, status, reserved_quantity FROM batches WHERE id = ?")
            .bind(id).fetch_one(pool).await.unwrap()
    }

    #[tokio::test]
    async fn draws_opened_containers_first_then_by_sequence() {
        let pool = setup().await;
        add_batch(&pool, "b", 3.0, 0.0, "available").await;
        add_container(&pool, "c1", "b", 1, 1.0, false).await;
        add_container(&pool, "c2", "b", 2, 0.5, true).await;
        add_container(&pool, "c3", "b", 3, 1.0, false).await;

        let mut tx = pool.begin().await.unwrap();
        let r = consume_from_batch(&mut *tx, ConsumeRequest::new("b", 1.2, Some("u"))).await.unwrap();
        tx.commit().await.unwrap();

        let order: Vec<i64> = r.containers.iter().map(|c| c.sequence_number).collect();
        assert_eq!(order, vec![2, 1]);
        assert_eq!(container_qty(&pool, "c2").await, 0.0);
        assert!((container_qty(&pool, "c1").await - 0.3).abs() < 1e-12);
        assert_eq!(container_qty(&pool, "c3").await, 1.0);
        let (q, s, _) = batch_state(&pool, "b").await;
        assert!((q - 1.8).abs() < 1e-12);
        assert_eq!(s, "available");
    }

    #[tokio::test]
    async fn group_selection_limits_the_draw() {
        let pool = setup().await;
        add_batch(&pool, "b", 3.0, 0.0, "available").await;
        add_container(&pool, "c1", "b", 1, 1.0, true).await;
        add_container(&pool, "c2", "b", 2, 1.0, false).await;
        add_container(&pool, "c3", "b", 3, 1.0, false).await;

        let group = vec!["c2".to_string(), "c3".to_string()];
        let mut tx = pool.begin().await.unwrap();
        let mut req = ConsumeRequest::new("b", 1.5, None);
        req.container_ids = Some(&group);
        consume_from_batch(&mut *tx, req).await.unwrap();
        tx.commit().await.unwrap();

        assert_eq!(container_qty(&pool, "c1").await, 1.0);
        assert_eq!(container_qty(&pool, "c2").await, 0.0);
        assert_eq!(container_qty(&pool, "c3").await, 0.5);
    }

    #[tokio::test]
    async fn rejects_more_than_containers_hold_and_changes_nothing() {
        let pool = setup().await;
        add_batch(&pool, "b", 2.0, 0.0, "available").await;
        add_container(&pool, "c1", "b", 1, 1.0, false).await;
        add_container(&pool, "c2", "b", 2, 1.0, false).await;

        let mut tx = pool.begin().await.unwrap();
        let err = consume_from_batch(&mut *tx, ConsumeRequest::new("b", 2.5, None)).await;
        assert!(err.is_err());
        drop(tx); // rollback
        assert_eq!(container_qty(&pool, "c1").await, 1.0);
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM usage_logs").fetch_one(&pool).await.unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn reserved_stock_is_protected_except_for_its_experiment() {
        let pool = setup().await;
        add_batch(&pool, "b", 10.0, 6.0, "available").await;

        let mut tx = pool.begin().await.unwrap();
        assert!(consume_from_batch(&mut *tx, ConsumeRequest::new("b", 5.0, None)).await.is_err());
        drop(tx);

        let mut tx = pool.begin().await.unwrap();
        let mut req = ConsumeRequest::new("b", 6.0, None);
        req.release_reserved = 6.0;
        consume_from_batch(&mut *tx, req).await.unwrap();
        tx.commit().await.unwrap();
        let (q, s, reserved) = batch_state(&pool, "b").await;
        assert_eq!((q, reserved), (4.0, 0.0));
        assert_eq!(s, "available");
    }

    #[tokio::test]
    async fn last_container_depletes_batch_and_float_noise_is_cleaned() {
        let pool = setup().await;
        add_batch(&pool, "b", 0.3, 0.0, "available").await;
        for (i, id) in ["c1", "c2", "c3"].iter().enumerate() {
            add_container(&pool, id, "b", i as i64 + 1, 0.1, false).await;
        }
        let mut tx = pool.begin().await.unwrap();
        let r = consume_from_batch(&mut *tx, ConsumeRequest::new("b", 0.3, None)).await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(r.containers.len(), 3);
        let (q, s, _) = batch_state(&pool, "b").await;
        assert_eq!(q, 0.0);
        assert_eq!(s, "depleted");
    }

    #[tokio::test]
    async fn expired_batch_only_for_experiments() {
        let pool = setup().await;
        add_batch(&pool, "b", 5.0, 0.0, "expired").await;
        let mut tx = pool.begin().await.unwrap();
        assert!(consume_from_batch(&mut *tx, ConsumeRequest::new("b", 1.0, None)).await.is_err());
        let mut req = ConsumeRequest::new("b", 1.0, None);
        req.allow_expired = true;
        let r = consume_from_batch(&mut *tx, req).await.unwrap();
        assert_eq!(r.status_after, "expired");
    }

    #[tokio::test]
    async fn experiment_link_is_consumed_only_once() {
        let pool = setup().await;
        add_batch(&pool, "b", 10.0, 2.0, "available").await;
        sqlx::query("INSERT INTO experiment_reagents VALUES ('l1', 'e1', 'b', 2.0, 0, datetime('now'))")
            .execute(&pool).await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let first = consume_experiment_link(&mut *tx, "l1", "b", Some(2.0), "e1", "Titration", None).await.unwrap();
        let second = consume_experiment_link(&mut *tx, "l1", "b", Some(2.0), "e1", "Titration", None).await.unwrap();
        tx.commit().await.unwrap();

        assert!(first.is_some());
        assert!(second.is_none());
        let (q, _, reserved) = batch_state(&pool, "b").await;
        assert_eq!((q, reserved), (8.0, 0.0));
    }

    #[test]
    fn batch_status_rules() {
        assert_eq!(compute_batch_status(5.0, 10.0, false, 0, "available"), "available");
        assert_eq!(compute_batch_status(2.0, 10.0, false, 0, "available"), "low_stock");
        assert_eq!(compute_batch_status(0.0005, 10.0, false, 0, "low_stock"), "depleted");
        assert_eq!(compute_batch_status(3.0, 10.0, true, 0, "available"), "depleted");
        assert_eq!(compute_batch_status(3.0, 10.0, false, 0, "expired"), "expired");
    }
}
