// src/reagent_stock.rs
//! Single source of truth for reagent stock math.
//!
//! The cached columns `reagents.total_quantity`, `batches_count` and
//! `primary_unit` are recomputed from batches in several places (triggers,
//! startup init, manual rebuild, per-reagent refresh). All of them must use
//! the same status filter and the same unit conversion, so the SQL fragments
//! live here and are spliced into those queries.

/// Batch statuses that count as usable stock.
/// `low_stock` is still stock: it must not disappear from reagent totals.
pub const STOCK_STATUSES_SQL: &str = "('available', 'low_stock', 'in_use')";

/// Factor converting `unit` to the base unit (g for mass, mL for volume).
/// Units outside mass/volume (pcs, mol, ...) pass through unchanged (x1).
/// Both the Greek mu (U+03BC) and the micro sign (U+00B5) are accepted.
pub const UNIT_FACTOR_SQL: &str = "CASE LOWER(unit) \
    WHEN 't' THEN 1000000.0 WHEN 'kg' THEN 1000.0 WHEN 'g' THEN 1.0 WHEN 'mg' THEN 0.001 \
    WHEN 'μg' THEN 0.000001 WHEN 'µg' THEN 0.000001 WHEN 'ug' THEN 0.000001 \
    WHEN 'l' THEN 1000.0 WHEN 'ml' THEN 1.0 \
    WHEN 'μl' THEN 0.001 WHEN 'µl' THEN 0.001 WHEN 'ul' THEN 0.001 \
    ELSE 1.0 END";

/// Base unit for `unit` ('g', 'mL', or the unit itself when not convertible).
pub const BASE_UNIT_SQL: &str = "CASE \
    WHEN LOWER(unit) IN ('t','kg','g','mg','μg','µg','ug') THEN 'g' \
    WHEN LOWER(unit) IN ('l','ml','μl','µl','ul') THEN 'mL' \
    ELSE unit END";

/// SET clause that recomputes the cached stock columns for one reagent.
///
/// `reagent_ref` is the SQL expression identifying the reagent:
/// `"NEW.reagent_id"` / `"OLD.reagent_id"` inside triggers, `"reagents.id"`
/// for a table-wide rebuild, or `"?"` (then bind the id 3 times, before the
/// WHERE clause binds).
pub fn cache_set_clause(reagent_ref: &str) -> String {
    format!(
        "total_quantity = (\
            SELECT COALESCE(SUM(quantity * {factor}), 0) FROM batches \
            WHERE reagent_id = {r} AND status IN {statuses} AND deleted_at IS NULL\
         ), \
         batches_count = (\
            SELECT COUNT(*) FROM batches \
            WHERE reagent_id = {r} AND status IN {statuses} AND deleted_at IS NULL\
         ), \
         primary_unit = (\
            SELECT {base} FROM batches \
            WHERE reagent_id = {r} AND status IN {statuses} AND deleted_at IS NULL \
            ORDER BY created_at ASC LIMIT 1\
         ), \
         updated_at = datetime('now')",
        factor = UNIT_FACTOR_SQL,
        base = BASE_UNIT_SQL,
        statuses = STOCK_STATUSES_SQL,
        r = reagent_ref,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_clause_uses_reagent_ref_three_times() {
        let sql = cache_set_clause("?");
        assert_eq!(sql.matches('?').count(), 3);
        assert!(sql.contains("'low_stock'"));
    }

    #[tokio::test]
    async fn totals_convert_units_and_include_low_stock() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE batches (reagent_id TEXT, quantity REAL, unit TEXT, status TEXT, \
             deleted_at TEXT, created_at TEXT)",
        ).execute(&pool).await.unwrap();
        sqlx::query(
            "CREATE TABLE reagents (id TEXT, total_quantity REAL, batches_count INTEGER, \
             primary_unit TEXT, updated_at TEXT)",
        ).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO reagents VALUES ('r1', 0, 0, NULL, NULL)").execute(&pool).await.unwrap();
        for (q, u, s, d) in [
            (1.0, "kg", "available", None),
            (500.0, "mg", "low_stock", None),
            (2.0, "µg", "available", None),
            (5.0, "g", "depleted", None),
            (7.0, "g", "expired", None),
            (9.0, "g", "available", Some("2026-01-01")),
        ] {
            sqlx::query("INSERT INTO batches VALUES ('r1', ?, ?, ?, ?, datetime('now'))")
                .bind(q).bind(u).bind(s).bind(d)
                .execute(&pool).await.unwrap();
        }

        let sql = format!("UPDATE reagents SET {} WHERE id = 'r1'", cache_set_clause("reagents.id"));
        sqlx::query(&sql).execute(&pool).await.unwrap();

        let (total, count, unit): (f64, i64, Option<String>) = sqlx::query_as(
            "SELECT total_quantity, batches_count, primary_unit FROM reagents WHERE id = 'r1'",
        ).fetch_one(&pool).await.unwrap();

        assert!((total - 1000.500002).abs() < 1e-9, "total = {}", total);
        assert_eq!(count, 3);
        assert_eq!(unit.as_deref(), Some("g"));
    }
}
