// src/container_history.rs
//! Bottle history: an append-only journal of what happened to each container.
//!
//! Every placement change (place / move / unplace), every draw from a bottle
//! (opened / used / emptied), disposal, and every inventory sighting
//! ("seen", "missing") is one row in `container_events`. Rows are written in
//! the same transaction as the change itself, so the journal cannot disagree
//! with the current state.
//!
//! Locations are stored twice: as position ids and as a text path
//! ("Lab 101 / Cabinet A / Shelf 1") taken at the time of the event, so the
//! history stays readable after shelves are renamed or deleted. Unlike
//! audit_logs (pruned after 90 days) this table is never pruned.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

pub mod event {
    pub const PLACED: &str = "placed";
    pub const MOVED: &str = "moved";
    pub const UNPLACED: &str = "unplaced";
    pub const OPENED: &str = "opened";
    pub const USED: &str = "used";
    pub const EMPTIED: &str = "emptied";
    pub const DISPOSED: &str = "disposed";
    pub const SEEN: &str = "seen";
    pub const MISSING: &str = "missing";
}

pub mod source {
    pub const WEB: &str = "web";
    pub const SCAN: &str = "scan";
    pub const BULK: &str = "bulk";
    pub const INVENTORY: &str = "inventory";
    pub const EXPERIMENT: &str = "experiment";
    pub const SYSTEM: &str = "system";
    pub const IMPORT: &str = "import";
}

/// Source reported by the client: only "scan" is trusted, anything else is "web"
pub fn client_source(value: Option<&str>) -> &'static str {
    match value {
        Some(s) if s.eq_ignore_ascii_case(source::SCAN) => source::SCAN,
        _ => source::WEB,
    }
}

pub const MIGRATIONS: &[&str] = &[
    r#"CREATE TABLE IF NOT EXISTS container_events (
        id TEXT PRIMARY KEY,
        container_id TEXT NOT NULL,
        batch_id TEXT NOT NULL,
        event_type TEXT NOT NULL CHECK(event_type IN
            ('placed','moved','unplaced','opened','used','emptied','disposed','seen','missing')),
        from_position_id TEXT,
        from_location TEXT,
        to_position_id TEXT,
        to_location TEXT,
        quantity REAL,
        quantity_after REAL,
        unit TEXT,
        source TEXT NOT NULL DEFAULT 'web',
        session_id TEXT,
        user_id TEXT,
        note TEXT,
        created_at DATETIME NOT NULL
    )"#,
    "CREATE INDEX IF NOT EXISTS idx_container_events_container ON container_events(container_id, created_at)",
    "CREATE INDEX IF NOT EXISTS idx_container_events_session ON container_events(session_id)",
    r#"CREATE TABLE IF NOT EXISTS inventory_sessions (
        id TEXT PRIMARY KEY,
        scope_type TEXT NOT NULL CHECK(scope_type IN ('room','zone','position')),
        scope_id TEXT NOT NULL,
        scope_name TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'open' CHECK(status IN ('open','closed','cancelled')),
        notes TEXT,
        started_by TEXT,
        started_at DATETIME NOT NULL,
        closed_by TEXT,
        closed_at DATETIME,
        expected_count INTEGER,
        found_count INTEGER,
        misplaced_count INTEGER,
        unplaced_count INTEGER,
        inactive_count INTEGER,
        unknown_count INTEGER,
        missing_count INTEGER
    )"#,
    "CREATE INDEX IF NOT EXISTS idx_inventory_sessions_status ON inventory_sessions(status, started_at)",
    r#"CREATE TABLE IF NOT EXISTS inventory_items (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        container_id TEXT,
        raw_code TEXT,
        position_id TEXT,
        location TEXT,
        result TEXT NOT NULL CHECK(result IN
            ('found','misplaced','unplaced','inactive','unknown','missing')),
        expected_position_id TEXT,
        expected_location TEXT,
        resolution TEXT CHECK(resolution IS NULL OR resolution IN ('moved_here','placed_here','unplaced')),
        scanned_by TEXT,
        scanned_at DATETIME NOT NULL,
        UNIQUE(session_id, container_id)
    )"#,
    "CREATE INDEX IF NOT EXISTS idx_inventory_items_session ON inventory_items(session_id)",
];

/// "Room / Zone / Position" for a position id (bound once)
pub const POSITION_PATH_SQL: &str = "(SELECT rm.name || ' / ' || sz.name || ' / ' || sp.name \
     FROM storage_positions sp \
     JOIN storage_zones sz ON sz.id = sp.zone_id \
     JOIN rooms rm ON rm.id = sz.room_id \
     WHERE sp.id = ?)";

// ==================== WRITE ====================

#[derive(Debug, Default)]
pub struct NewEvent<'a> {
    pub container_id: &'a str,
    pub event_type: &'a str,
    pub from_position: Option<&'a str>,
    pub to_position: Option<&'a str>,
    pub quantity: Option<f64>,
    pub quantity_after: Option<f64>,
    pub source: &'a str,
    pub session_id: Option<&'a str>,
    pub user_id: Option<&'a str>,
    pub note: Option<&'a str>,
}

impl<'a> NewEvent<'a> {
    pub fn new(container_id: &'a str, event_type: &'a str, source: &'a str, user_id: Option<&'a str>) -> Self {
        Self { container_id, event_type, source, user_id, ..Default::default() }
    }
}

/// Appends one event. batch_id and unit are taken from the container,
/// location paths from the position ids.
pub async fn record_event(conn: &mut SqliteConnection, ev: &NewEvent<'_>) -> ApiResult<()> {
    let from_loc = if ev.from_position.is_some() { POSITION_PATH_SQL } else { "NULL" };
    let to_loc = if ev.to_position.is_some() { POSITION_PATH_SQL } else { "NULL" };
    let sql = format!(
        "INSERT INTO container_events (id, container_id, batch_id, event_type, \
            from_position_id, from_location, to_position_id, to_location, \
            quantity, quantity_after, unit, source, session_id, user_id, note, created_at) \
         SELECT ?, bc.id, bc.batch_id, ?, ?, {from_loc}, ?, {to_loc}, ?, ?, b.unit, ?, ?, ?, ?, ? \
         FROM batch_containers bc JOIN batches b ON b.id = bc.batch_id \
         WHERE bc.id = ?"
    );
    let mut q = sqlx::query(&sql)
        .bind(Uuid::new_v4().to_string())
        .bind(ev.event_type)
        .bind(ev.from_position);
    if let Some(p) = ev.from_position {
        q = q.bind(p);
    }
    q = q.bind(ev.to_position);
    if let Some(p) = ev.to_position {
        q = q.bind(p);
    }
    let done = q
        .bind(ev.quantity)
        .bind(ev.quantity_after)
        .bind(ev.source)
        .bind(ev.session_id)
        .bind(ev.user_id)
        .bind(ev.note)
        .bind(Utc::now())
        .bind(ev.container_id)
        .execute(&mut *conn)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::not_found("Container"));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlacementChange {
    pub from: Option<String>,
    pub to: Option<String>,
}

impl PlacementChange {
    pub fn changed(&self) -> bool {
        self.from != self.to
    }
}

/// Sets (Some) or clears (None) the container's position and journals it.
/// The single write path for batch_placements: placed / moved / unplaced.
/// Moving to the position it already has is a no-op (no event).
pub async fn change_placement(
    conn: &mut SqliteConnection,
    container_id: &str,
    to: Option<&str>,
    user_id: Option<&str>,
    source: &str,
    session_id: Option<&str>,
    placement_notes: Option<&str>,
) -> ApiResult<PlacementChange> {
    let from: Option<String> = sqlx::query_scalar(
        "SELECT position_id FROM batch_placements WHERE container_id = ?",
    )
    .bind(container_id)
    .fetch_optional(&mut *conn)
    .await?;

    let change = PlacementChange { from: from.clone(), to: to.map(str::to_string) };
    if !change.changed() {
        return Ok(change);
    }
    let now = Utc::now();

    let event_type = match (&from, to) {
        (None, Some(pos)) => {
            sqlx::query(
                "INSERT INTO batch_placements (id, container_id, position_id, placed_by, placed_at, notes) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(container_id)
            .bind(pos)
            .bind(user_id)
            .bind(now)
            .bind(placement_notes)
            .execute(&mut *conn)
            .await?;
            event::PLACED
        }
        (Some(_), Some(pos)) => {
            sqlx::query(
                "UPDATE batch_placements SET position_id = ?, placed_by = ?, placed_at = ? WHERE container_id = ?",
            )
            .bind(pos)
            .bind(user_id)
            .bind(now)
            .bind(container_id)
            .execute(&mut *conn)
            .await?;
            event::MOVED
        }
        (Some(_), None) => {
            sqlx::query("DELETE FROM batch_placements WHERE container_id = ?")
                .bind(container_id)
                .execute(&mut *conn)
                .await?;
            event::UNPLACED
        }
        (None, None) => unreachable!("no-op handled above"),
    };

    let mut ev = NewEvent::new(container_id, event_type, source, user_id);
    ev.from_position = from.as_deref();
    ev.to_position = to;
    ev.session_id = session_id;
    record_event(conn, &ev).await?;
    Ok(change)
}

/// Journals placements that have no event yet (rows written before the
/// journal existed, or by a bulk import) as a "placed" event dated at the
/// placement time. Idempotent.
pub async fn record_unlogged_placements(
    conn: &mut SqliteConnection,
    source: &str,
    note: &str,
) -> ApiResult<u64> {
    let sql = format!(
        "INSERT INTO container_events (id, container_id, batch_id, event_type, \
            to_position_id, to_location, unit, source, user_id, note, created_at) \
         SELECT lower(hex(randomblob(16))), bp.container_id, bc.batch_id, 'placed', \
            bp.position_id, {path}, b.unit, ?, bp.placed_by, ?, COALESCE(bp.placed_at, datetime('now')) \
         FROM batch_placements bp \
         JOIN batch_containers bc ON bc.id = bp.container_id \
         JOIN batches b ON b.id = bc.batch_id \
         WHERE NOT EXISTS (SELECT 1 FROM container_events e WHERE e.container_id = bp.container_id)",
        path = POSITION_PATH_SQL.replace("WHERE sp.id = ?", "WHERE sp.id = bp.position_id"),
    );
    let done = sqlx::query(&sql).bind(source).bind(note).execute(&mut *conn).await?;
    Ok(done.rows_affected())
}

pub async fn run_migrations(pool: &SqlitePool) -> ApiResult<()> {
    let mut tx = pool.begin().await?;
    for ddl in MIGRATIONS {
        sqlx::query(ddl).execute(&mut *tx).await?;
    }
    let n = record_unlogged_placements(
        &mut tx, source::SYSTEM, "Placement recorded before movement history existed",
    ).await?;
    tx.commit().await?;
    if n > 0 {
        log::info!("Container history: journaled {} existing placements", n);
    }
    Ok(())
}

// ==================== READ ====================

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ContainerEventRow {
    pub id: String,
    pub container_id: String,
    pub batch_id: String,
    pub event_type: String,
    pub from_position_id: Option<String>,
    pub from_location: Option<String>,
    pub to_position_id: Option<String>,
    pub to_location: Option<String>,
    pub quantity: Option<f64>,
    pub quantity_after: Option<f64>,
    pub unit: Option<String>,
    pub source: String,
    pub session_id: Option<String>,
    pub session_scope: Option<String>,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Newest first
pub async fn container_history(pool: &SqlitePool, container_id: &str, limit: i64) -> ApiResult<Vec<ContainerEventRow>> {
    let rows = sqlx::query_as::<_, ContainerEventRow>(
        "SELECT e.id, e.container_id, e.batch_id, e.event_type, e.from_position_id, e.from_location, \
                e.to_position_id, e.to_location, e.quantity, e.quantity_after, e.unit, e.source, \
                e.session_id, s.scope_name AS session_scope, e.user_id, u.username, e.note, e.created_at \
         FROM container_events e \
         LEFT JOIN users u ON u.id = e.user_id \
         LEFT JOIN inventory_sessions s ON s.id = e.session_id \
         WHERE e.container_id = ? \
         ORDER BY e.created_at DESC, e.rowid DESC \
         LIMIT ?",
    )
    .bind(container_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ==================== TESTS ====================

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Minimal storage + container schema plus the journal tables
    pub(crate) async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        for ddl in [
            "CREATE TABLE users (id TEXT PRIMARY KEY, username TEXT)",
            "CREATE TABLE reagents (id TEXT PRIMARY KEY, name TEXT, formula TEXT, cas_number TEXT, \
             hazard_pictograms TEXT, storage_conditions TEXT, deleted_at TEXT)",
            "CREATE TABLE batches (id TEXT PRIMARY KEY, reagent_id TEXT, batch_number TEXT, lot_number TEXT, \
             unit TEXT, status TEXT, quantity REAL, reserved_quantity REAL, expiry_date TEXT, \
             received_date TEXT, deleted_at TEXT)",
            "CREATE TABLE batch_containers (id TEXT PRIMARY KEY, batch_id TEXT, sequence_number INTEGER, \
             quantity REAL, original_quantity REAL, status TEXT, is_opened INTEGER)",
            "CREATE TABLE batch_placements (id TEXT PRIMARY KEY, container_id TEXT UNIQUE, position_id TEXT, \
             placed_by TEXT, placed_at TEXT, notes TEXT)",
            "CREATE TABLE rooms (id TEXT PRIMARY KEY, name TEXT, color TEXT)",
            "CREATE TABLE storage_zones (id TEXT PRIMARY KEY, room_id TEXT, name TEXT, sort_order INTEGER DEFAULT 0)",
            "CREATE TABLE storage_positions (id TEXT PRIMARY KEY, zone_id TEXT, name TEXT, sort_order INTEGER DEFAULT 0)",
            "INSERT INTO users VALUES ('u1', 'emil')",
            "INSERT INTO reagents VALUES ('r1', 'Benzaldehyde', 'C7H6O', '100-52-7', NULL, NULL, NULL)",
            "INSERT INTO batches VALUES ('b1', 'r1', 'BZ-1', NULL, 'mL', 'available', 37.5, 0, NULL, \
             '2026-01-10T00:00:00Z', NULL)",
            "INSERT INTO rooms VALUES ('room', 'Lab 101', NULL), ('room2', 'Lab 102', NULL)",
            "INSERT INTO storage_zones VALUES ('z1', 'room', 'Cabinet A', 0), ('z2', 'room2', 'Fridge', 0)",
            "INSERT INTO storage_positions VALUES ('p1', 'z1', 'Shelf 1', 0), ('p2', 'z1', 'Shelf 2', 1), \
             ('p3', 'z2', 'Door', 0)",
            "INSERT INTO batch_containers VALUES ('c1', 'b1', 1, 12.5, 12.5, 'full', 0), \
             ('c2', 'b1', 2, 12.5, 12.5, 'full', 0), ('c3', 'b1', 3, 12.5, 12.5, 'full', 0), \
             ('c4', 'b1', 4, 0, 12.5, 'empty', 1)",
        ] {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }
        for ddl in MIGRATIONS {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }
        pool
    }

    #[test]
    fn only_scan_is_taken_from_the_client() {
        assert_eq!(client_source(Some("scan")), "scan");
        assert_eq!(client_source(Some("SCAN")), "scan");
        assert_eq!(client_source(Some("system")), "web");
        assert_eq!(client_source(None), "web");
    }

    #[tokio::test]
    async fn placement_changes_are_journaled_with_location_paths() {
        let pool = setup().await;
        let mut conn = pool.acquire().await.unwrap();
        let c = change_placement(&mut conn, "c1", Some("p1"), Some("u1"), source::SCAN, None, None).await.unwrap();
        assert_eq!(c.from, None);
        // Same shelf again: nothing happens
        let same = change_placement(&mut conn, "c1", Some("p1"), Some("u1"), source::WEB, None, None).await.unwrap();
        assert!(!same.changed());
        change_placement(&mut conn, "c1", Some("p3"), Some("u1"), source::WEB, None, None).await.unwrap();
        change_placement(&mut conn, "c1", None, Some("u1"), source::WEB, None, None).await.unwrap();
        drop(conn);

        let h = container_history(&pool, "c1", 50).await.unwrap();
        let summary: Vec<(String, Option<String>, Option<String>, String)> = h.iter().rev()
            .map(|e| (e.event_type.clone(), e.from_location.clone(), e.to_location.clone(), e.source.clone()))
            .collect();
        assert_eq!(summary, vec![
            ("placed".into(), None, Some("Lab 101 / Cabinet A / Shelf 1".into()), "scan".into()),
            ("moved".into(), Some("Lab 101 / Cabinet A / Shelf 1".into()), Some("Lab 102 / Fridge / Door".into()), "web".into()),
            ("unplaced".into(), Some("Lab 102 / Fridge / Door".into()), None, "web".into()),
        ]);
        assert_eq!(h[0].username.as_deref(), Some("emil"));
        assert_eq!(h[0].unit.as_deref(), Some("mL"));

        // The path is a snapshot: renaming the shelf does not rewrite history
        sqlx::query("UPDATE storage_positions SET name = 'Top shelf' WHERE id = 'p1'").execute(&pool).await.unwrap();
        let h2 = container_history(&pool, "c1", 50).await.unwrap();
        assert_eq!(h2.last().unwrap().to_location.as_deref(), Some("Lab 101 / Cabinet A / Shelf 1"));
    }

    #[tokio::test]
    async fn existing_placements_are_backfilled_once() {
        let pool = setup().await;
        sqlx::query("INSERT INTO batch_placements VALUES ('pl1', 'c2', 'p2', 'u1', '2026-03-01T10:00:00Z', NULL)")
            .execute(&pool).await.unwrap();
        let mut conn = pool.acquire().await.unwrap();
        assert_eq!(record_unlogged_placements(&mut conn, source::SYSTEM, "before").await.unwrap(), 1);
        assert_eq!(record_unlogged_placements(&mut conn, source::SYSTEM, "before").await.unwrap(), 0);
        drop(conn);
        let h = container_history(&pool, "c2", 10).await.unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].to_location.as_deref(), Some("Lab 101 / Cabinet A / Shelf 2"));
        assert_eq!(h[0].created_at.to_rfc3339(), "2026-03-01T10:00:00+00:00");
    }
}
