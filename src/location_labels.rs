// src/location_labels.rs
//! QR labels for storage locations (room, cabinet/zone, shelf/position) and
//! the scan lookup for them.
//!
//! The symbol encodes `CHL:L:<kind>:<id as 32 uppercase hex>`, kind being
//! `R` (room), `Z` (zone, i.e. cabinet) or `P` (position, i.e. shelf). Bottle
//! labels use `CHL:C:` (Data Matrix); the shared `CHL:` prefix tells our
//! labels apart from manufacturer codes, the second part tells bottles from
//! places.
//!
//! Each label also prints a short code, the kind letter plus the first 8 hex
//! digits ("P 1A2B3C4D"), so a place can still be found by typing. A typed
//! place code always starts with R, Z or P, which are not hex digits, so it
//! can never be mistaken for the short code of a bottle.

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::ApiResponse;
use crate::label_handlers::{compact_id, ContainerLabel, ContainerRow, CONTAINER_ROW_SELECT, LABEL_ORDER, SHORT_CODE_LEN};
use crate::AppState;

pub const LOCATION_PREFIX: &str = "CHL:L:";
/// Labels per request
const MAX_LABELS: i64 = 1000;
/// Bottles listed on a scanned shelf
const MAX_SHELF_BOTTLES: i64 = 300;

// ==================== PAYLOAD FORMAT ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LocationKind {
    Room,
    Zone,
    Position,
}

impl LocationKind {
    pub fn letter(self) -> char {
        match self {
            LocationKind::Room => 'R',
            LocationKind::Zone => 'Z',
            LocationKind::Position => 'P',
        }
    }

    fn from_letter(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'R' => Some(LocationKind::Room),
            'Z' => Some(LocationKind::Zone),
            'P' => Some(LocationKind::Position),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            LocationKind::Room => "room",
            LocationKind::Zone => "zone",
            LocationKind::Position => "position",
        }
    }
}

pub fn location_payload(kind: LocationKind, id: &str) -> String {
    format!("{}{}:{}", LOCATION_PREFIX, kind.letter(), compact_id(id))
}

pub fn location_short_code(kind: LocationKind, id: &str) -> String {
    let hex: String = compact_id(id).chars().take(SHORT_CODE_LEN).collect();
    format!("{}{}", kind.letter(), hex)
}

#[derive(Debug, PartialEq)]
pub enum LocationRef {
    /// Exact id (hyphenated lowercase for UUIDs)
    Id(String),
    /// Hex prefix typed by hand, lowercase
    Prefix(String),
}

#[derive(Debug, PartialEq)]
pub struct LocationQuery {
    pub kind: LocationKind,
    pub target: LocationRef,
}

/// Turns a scanned or typed place code into a lookup; None for anything else
/// (bottle labels, manufacturer codes, words).
pub fn parse_location_code(raw: &str) -> Option<LocationQuery> {
    let s: String = raw.chars().filter(|c| !c.is_control()).collect();
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    if s.len() > LOCATION_PREFIX.len()
        && s.is_char_boundary(LOCATION_PREFIX.len())
        && s[..LOCATION_PREFIX.len()].eq_ignore_ascii_case(LOCATION_PREFIX)
    {
        let rest = &s[LOCATION_PREFIX.len()..];
        let mut chars = rest.chars();
        let kind = LocationKind::from_letter(chars.next()?)?;
        let token = chars.as_str().strip_prefix(':')?.trim();
        if token.is_empty() {
            return None;
        }
        let target = match Uuid::parse_str(token) {
            Ok(u) => LocationRef::Id(u.hyphenated().to_string()),
            Err(_) => LocationRef::Id(token.to_string()),
        };
        return Some(LocationQuery { kind, target });
    }

    // Typed: "P1A2B3C4D", "p 1a2b3c4d", "Z-1A2B 3C4D"
    let mut chars = s.chars();
    let kind = LocationKind::from_letter(chars.next()?)?;
    let digits: String = chars.filter(|c| !matches!(c, ' ' | '-' | ':' | '·')).collect();
    if (6..=12).contains(&digits.len()) && digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(LocationQuery { kind, target: LocationRef::Prefix(digits.to_ascii_lowercase()) });
    }
    None
}

// ==================== DATA ====================

/// What a printed label or a scan result says about one place
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LocationRow {
    pub id: String,
    pub name: String,
    /// "Lab 101 / Cabinet A / Shelf 1"
    pub path: String,
    pub room_id: String,
    pub room_name: String,
    pub room_color: Option<String>,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub zone_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LocationLabel {
    pub kind: LocationKind,
    #[serde(flatten)]
    pub row: LocationRow,
    /// Text to encode in the QR symbol
    pub payload: String,
    /// Printed under the symbol
    pub short_code: String,
}

impl LocationLabel {
    fn new(kind: LocationKind, row: LocationRow) -> Self {
        let payload = location_payload(kind, &row.id);
        let short_code = location_short_code(kind, &row.id);
        Self { kind, row, payload, short_code }
    }
}

#[derive(Debug, Serialize)]
pub struct LocationChild {
    pub kind: LocationKind,
    pub id: String,
    pub name: String,
    pub bottle_count: i64,
}

#[derive(Debug, Serialize)]
pub struct LocationDetail {
    #[serde(flatten)]
    pub label: LocationLabel,
    /// Active bottles recorded inside (empty and disposed ones are not counted)
    pub bottle_count: i64,
    /// Zones of a room, shelves of a zone
    pub children: Vec<LocationChild>,
    /// Bottles on a shelf (shelves only)
    pub bottles: Vec<ContainerLabel>,
}

/// Row sources aliased like the inventory scope queries: sp, sz, rm
const LOCATION_SELECT: [&str; 3] = [
    // room
    "SELECT rm.id, rm.name, rm.name AS path, rm.id AS room_id, rm.name AS room_name, rm.color AS room_color, \
            NULL AS zone_id, NULL AS zone_name, NULL AS zone_type \
     FROM rooms rm",
    // zone
    "SELECT sz.id, sz.name, rm.name || ' / ' || sz.name AS path, rm.id AS room_id, rm.name AS room_name, \
            rm.color AS room_color, sz.id AS zone_id, sz.name AS zone_name, sz.zone_type \
     FROM storage_zones sz JOIN rooms rm ON rm.id = sz.room_id",
    // position
    "SELECT sp.id, sp.name, rm.name || ' / ' || sz.name || ' / ' || sp.name AS path, rm.id AS room_id, \
            rm.name AS room_name, rm.color AS room_color, sz.id AS zone_id, sz.name AS zone_name, sz.zone_type \
     FROM storage_positions sp JOIN storage_zones sz ON sz.id = sp.zone_id JOIN rooms rm ON rm.id = sz.room_id",
];

fn select_and_alias(kind: LocationKind) -> (&'static str, &'static str) {
    match kind {
        LocationKind::Room => (LOCATION_SELECT[0], "rm"),
        LocationKind::Zone => (LOCATION_SELECT[1], "sz"),
        LocationKind::Position => (LOCATION_SELECT[2], "sp"),
    }
}

/// Order that keeps a room's cabinets and a cabinet's shelves as in the UI
fn order_for(kind: LocationKind) -> &'static str {
    match kind {
        LocationKind::Room => "ORDER BY rm.name COLLATE NOCASE",
        LocationKind::Zone => "ORDER BY rm.name COLLATE NOCASE, sz.sort_order, sz.name COLLATE NOCASE",
        LocationKind::Position => {
            "ORDER BY rm.name COLLATE NOCASE, sz.sort_order, sz.name COLLATE NOCASE, sp.sort_order, sp.name COLLATE NOCASE"
        }
    }
}

pub async fn find_locations(pool: &SqlitePool, q: &LocationQuery) -> ApiResult<Vec<LocationLabel>> {
    let (select, alias) = select_and_alias(q.kind);
    let rows: Vec<LocationRow> = match &q.target {
        LocationRef::Id(id) => {
            let sql = format!("{} WHERE lower({}.id) = lower(?)", select, alias);
            sqlx::query_as(&sql).bind(id).fetch_all(pool).await?
        }
        LocationRef::Prefix(prefix) => {
            // prefix is validated hex, so it holds no LIKE wildcards
            let sql = format!("{} WHERE lower(replace({}.id, '-', '')) LIKE ? {} LIMIT 10", select, alias, order_for(q.kind));
            sqlx::query_as(&sql).bind(format!("{}%", prefix)).fetch_all(pool).await?
        }
    };
    Ok(rows.into_iter().map(|r| LocationLabel::new(q.kind, r)).collect())
}

/// Active bottles recorded under a place. Same rule as the inventory's
/// "expected" set: placed, not empty, not disposed, batch and reagent alive.
const ACTIVE_JOINS: &str = "FROM batch_placements bp \
     JOIN batch_containers bc ON bc.id = bp.container_id \
     JOIN batches b ON b.id = bc.batch_id AND b.deleted_at IS NULL \
     JOIN reagents r ON r.id = b.reagent_id AND r.deleted_at IS NULL";
const ACTIVE_STATUS: &str = "bc.status NOT IN ('empty', 'disposed')";

fn scope_condition(kind: LocationKind) -> &'static str {
    match kind {
        LocationKind::Room => "sz.room_id = ?",
        LocationKind::Zone => "sp.zone_id = ?",
        LocationKind::Position => "sp.id = ?",
    }
}

async fn bottle_count(pool: &SqlitePool, kind: LocationKind, id: &str) -> ApiResult<i64> {
    let sql = format!(
        "SELECT COUNT(*) {} JOIN storage_positions sp ON sp.id = bp.position_id \
         JOIN storage_zones sz ON sz.id = sp.zone_id WHERE {} AND {}",
        ACTIVE_JOINS, ACTIVE_STATUS, scope_condition(kind)
    );
    Ok(sqlx::query_scalar(&sql).bind(id).fetch_one(pool).await?)
}

async fn children(pool: &SqlitePool, kind: LocationKind, id: &str) -> ApiResult<Vec<LocationChild>> {
    let (child_kind, sql) = match kind {
        LocationKind::Room => (
            LocationKind::Zone,
            format!(
                "SELECT sz.id, sz.name, (SELECT COUNT(*) {} JOIN storage_positions spx ON spx.id = bp.position_id \
                 WHERE {} AND spx.zone_id = sz.id) AS n \
                 FROM storage_zones sz WHERE sz.room_id = ? ORDER BY sz.sort_order, sz.name COLLATE NOCASE",
                ACTIVE_JOINS, ACTIVE_STATUS
            ),
        ),
        LocationKind::Zone => (
            LocationKind::Position,
            format!(
                "SELECT sp.id, sp.name, (SELECT COUNT(*) {} WHERE {} AND bp.position_id = sp.id) AS n \
                 FROM storage_positions sp WHERE sp.zone_id = ? ORDER BY sp.sort_order, sp.name COLLATE NOCASE",
                ACTIVE_JOINS, ACTIVE_STATUS
            ),
        ),
        LocationKind::Position => return Ok(Vec::new()),
    };
    let rows: Vec<(String, String, i64)> = sqlx::query_as(&sql).bind(id).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|(id, name, bottle_count)| LocationChild { kind: child_kind, id, name, bottle_count })
        .collect())
}

async fn shelf_bottles(pool: &SqlitePool, position_id: &str) -> ApiResult<Vec<ContainerLabel>> {
    let sql = format!(
        "{} WHERE bp.position_id = ? AND bc.status NOT IN ('empty', 'disposed') {} LIMIT ?",
        CONTAINER_ROW_SELECT, LABEL_ORDER
    );
    let rows: Vec<ContainerRow> = sqlx::query_as(&sql).bind(position_id).bind(MAX_SHELF_BOTTLES).fetch_all(pool).await?;
    Ok(rows.into_iter().map(ContainerLabel::from).collect())
}

pub async fn location_detail(pool: &SqlitePool, label: LocationLabel) -> ApiResult<LocationDetail> {
    let kind = label.kind;
    let id = label.row.id.clone();
    let count = bottle_count(pool, kind, &id).await?;
    let kids = children(pool, kind, &id).await?;
    let bottles = if kind == LocationKind::Position { shelf_bottles(pool, &id).await? } else { Vec::new() };
    Ok(LocationDetail { label, bottle_count: count, children: kids, bottles })
}

// ==================== LABEL SELECTION ====================

#[derive(Debug, Deserialize, Default)]
pub struct LocationLabelsRequest {
    pub room_id: Option<String>,
    pub zone_id: Option<String>,
    pub position_id: Option<String>,
    /// Also label the cabinets and shelves inside a room, the shelves inside a cabinet
    pub include_children: Option<bool>,
}

pub async fn select_location_labels(pool: &SqlitePool, req: &LocationLabelsRequest) -> ApiResult<Vec<LocationLabel>> {
    let nonempty = |v: &Option<String>| v.as_deref().filter(|s| !s.is_empty()).map(str::to_string);
    let (room, zone, position) = (nonempty(&req.room_id), nonempty(&req.zone_id), nonempty(&req.position_id));
    if room.is_none() && zone.is_none() && position.is_none() {
        return Err(ApiError::bad_request("Select a room, a cabinet (zone) or a shelf (position)"));
    }
    let with_children = req.include_children.unwrap_or(false);

    let mut out: Vec<LocationLabel> = Vec::new();
    let fetch = |kind: LocationKind, cond: &'static str| {
        let (select, _) = select_and_alias(kind);
        format!("{} WHERE {} {} LIMIT {}", select, cond, order_for(kind), MAX_LABELS)
    };

    if let Some(id) = &room {
        let rows: Vec<LocationRow> = sqlx::query_as(&fetch(LocationKind::Room, "rm.id = ?")).bind(id).fetch_all(pool).await?;
        out.extend(rows.into_iter().map(|r| LocationLabel::new(LocationKind::Room, r)));
        if with_children {
            let zones: Vec<LocationRow> = sqlx::query_as(&fetch(LocationKind::Zone, "rm.id = ?")).bind(id).fetch_all(pool).await?;
            let shelves: Vec<LocationRow> = sqlx::query_as(&fetch(LocationKind::Position, "rm.id = ?")).bind(id).fetch_all(pool).await?;
            push_tree(&mut out, zones, shelves);
        }
    }
    if let Some(id) = &zone {
        let rows: Vec<LocationRow> = sqlx::query_as(&fetch(LocationKind::Zone, "sz.id = ?")).bind(id).fetch_all(pool).await?;
        out.extend(rows.into_iter().map(|r| LocationLabel::new(LocationKind::Zone, r)));
        if with_children {
            let shelves: Vec<LocationRow> = sqlx::query_as(&fetch(LocationKind::Position, "sz.id = ?")).bind(id).fetch_all(pool).await?;
            out.extend(shelves.into_iter().map(|r| LocationLabel::new(LocationKind::Position, r)));
        }
    }
    if let Some(id) = &position {
        let rows: Vec<LocationRow> = sqlx::query_as(&fetch(LocationKind::Position, "sp.id = ?")).bind(id).fetch_all(pool).await?;
        out.extend(rows.into_iter().map(|r| LocationLabel::new(LocationKind::Position, r)));
    }
    if out.is_empty() {
        return Err(ApiError::not_found("Storage location"));
    }
    out.truncate(MAX_LABELS as usize);
    Ok(out)
}

/// Cabinets of a room, each followed by its shelves
fn push_tree(out: &mut Vec<LocationLabel>, zones: Vec<LocationRow>, shelves: Vec<LocationRow>) {
    let mut shelves: Vec<Option<LocationRow>> = shelves.into_iter().map(Some).collect();
    for zone in zones {
        let zone_id = zone.id.clone();
        out.push(LocationLabel::new(LocationKind::Zone, zone));
        for slot in shelves.iter_mut() {
            if slot.as_ref().map(|s| s.zone_id.as_deref() == Some(zone_id.as_str())).unwrap_or(false) {
                if let Some(s) = slot.take() {
                    out.push(LocationLabel::new(LocationKind::Position, s));
                }
            }
        }
    }
}

// ==================== HANDLER ====================

/// POST /api/v1/labels/locations
pub async fn get_location_labels(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<LocationLabelsRequest>,
) -> ApiResult<HttpResponse> {
    let labels = select_location_labels(&app_state.db_pool, &body).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(labels)))
}

/// Resolves a scanned place code into the place with its contents
pub async fn resolve_location(pool: &SqlitePool, q: &LocationQuery) -> ApiResult<LocationDetail> {
    let mut found = find_locations(pool, q).await?;
    match found.len() {
        0 => Err(ApiError::NotFound(
            "No storage location with this label. It may have been deleted.".to_string(),
        )),
        1 => location_detail(pool, found.remove(0)).await,
        _ => Err(ApiError::BadRequest(format!(
            "Several {}s share this short code. Scan the QR symbol instead.",
            q.kind.as_str()
        ))),
    }
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "1a2b3c4d-5e6f-4a1b-8c2d-3e4f5a6b7c8d";

    #[test]
    fn payload_and_short_code() {
        assert_eq!(location_payload(LocationKind::Position, ID), "CHL:L:P:1A2B3C4D5E6F4A1B8C2D3E4F5A6B7C8D");
        assert_eq!(location_payload(LocationKind::Zone, "legacy-1"), "CHL:L:Z:legacy-1");
        assert_eq!(location_short_code(LocationKind::Room, ID), "R1A2B3C4D");
    }

    #[test]
    fn parses_scanned_and_typed_place_codes() {
        let id = |k| Some(LocationQuery { kind: k, target: LocationRef::Id(ID.to_string()) });
        assert_eq!(parse_location_code("CHL:L:P:1A2B3C4D5E6F4A1B8C2D3E4F5A6B7C8D"), id(LocationKind::Position));
        assert_eq!(parse_location_code("chl:l:z:1a2b3c4d5e6f4a1b8c2d3e4f5a6b7c8d\r\n"), id(LocationKind::Zone));
        assert_eq!(parse_location_code(&format!("CHL:L:R:{}", ID)), id(LocationKind::Room));
        assert_eq!(
            parse_location_code("CHL:L:P:legacy-9"),
            Some(LocationQuery { kind: LocationKind::Position, target: LocationRef::Id("legacy-9".into()) })
        );

        let pre = |k, p: &str| Some(LocationQuery { kind: k, target: LocationRef::Prefix(p.to_string()) });
        assert_eq!(parse_location_code("P1A2B3C4D"), pre(LocationKind::Position, "1a2b3c4d"));
        assert_eq!(parse_location_code(" z 1a2b-3c4d "), pre(LocationKind::Zone, "1a2b3c4d"));
        assert_eq!(parse_location_code("R:1A2B3C4D"), pre(LocationKind::Room, "1a2b3c4d"));

        // Not place codes: bottle labels and short codes, GS1, words, junk
        assert_eq!(parse_location_code("CHL:C:24870DF5F5714DC7A4C4DC02B225C49A"), None);
        assert_eq!(parse_location_code("24870DF5"), None);
        assert_eq!(parse_location_code("C3F9A1C2"), None);
        assert_eq!(parse_location_code("010040123456789017270131101234"), None);
        assert_eq!(parse_location_code("Position"), None);
        assert_eq!(parse_location_code("P1A2"), None);
        assert_eq!(parse_location_code("CHL:L:"), None);
        assert_eq!(parse_location_code("CHL:L:X:1A2B3C4D"), None);
        assert_eq!(parse_location_code("CHL:L:P1A2B3C4D"), None);
        assert_eq!(parse_location_code("   "), None);
    }

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        for ddl in [
            "CREATE TABLE reagents (id TEXT PRIMARY KEY, name TEXT, formula TEXT, cas_number TEXT, \
             hazard_pictograms TEXT, storage_conditions TEXT, deleted_at TEXT)",
            "CREATE TABLE batches (id TEXT PRIMARY KEY, reagent_id TEXT, batch_number TEXT, lot_number TEXT, \
             unit TEXT, status TEXT, quantity REAL, reserved_quantity REAL, expiry_date TEXT, \
             received_date TEXT, deleted_at TEXT)",
            "CREATE TABLE batch_containers (id TEXT PRIMARY KEY, batch_id TEXT, sequence_number INTEGER, \
             quantity REAL, original_quantity REAL, status TEXT, is_opened INTEGER)",
            "CREATE TABLE batch_placements (id TEXT PRIMARY KEY, container_id TEXT, position_id TEXT)",
            "CREATE TABLE storage_positions (id TEXT PRIMARY KEY, zone_id TEXT, name TEXT, sort_order INTEGER)",
            "CREATE TABLE storage_zones (id TEXT PRIMARY KEY, room_id TEXT, name TEXT, zone_type TEXT, sort_order INTEGER)",
            "CREATE TABLE rooms (id TEXT PRIMARY KEY, name TEXT, color TEXT)",
            "INSERT INTO reagents VALUES ('r1', 'Benzaldehyde', NULL, NULL, NULL, NULL, NULL)",
            "INSERT INTO batches VALUES ('b1', 'r1', 'BZ-1', NULL, 'mL', 'available', 50, 0, NULL, '2026-01-10T00:00:00Z', NULL)",
            "INSERT INTO rooms VALUES ('1a2b3c4d-0000-4000-8000-000000000001', 'Lab 101', '#667eea')",
            "INSERT INTO rooms VALUES ('room2', 'Lab 102', NULL)",
            "INSERT INTO storage_zones VALUES ('z1', '1a2b3c4d-0000-4000-8000-000000000001', 'Cabinet A', 'cabinet', 1)",
            "INSERT INTO storage_zones VALUES ('z2', '1a2b3c4d-0000-4000-8000-000000000001', 'Fridge', 'refrigerator', 2)",
            "INSERT INTO storage_zones VALUES ('z3', 'room2', 'Cabinet B', 'cabinet', 1)",
            "INSERT INTO storage_positions VALUES ('p1', 'z1', 'Shelf 1', 1)",
            "INSERT INTO storage_positions VALUES ('p2', 'z1', 'Shelf 2', 2)",
            "INSERT INTO storage_positions VALUES ('p3', 'z2', 'Door', 1)",
            "INSERT INTO storage_positions VALUES ('p4', 'z3', 'Shelf 1', 1)",
        ] {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }
        // c1, c2 on Shelf 1, c3 on Shelf 2, c4 on Door; c5 empty and c6 disposed on Shelf 1; c7 in Lab 102
        for (id, status, pos) in [
            ("c1", "full", "p1"), ("c2", "partial", "p1"), ("c3", "full", "p2"), ("c4", "full", "p3"),
            ("c5", "empty", "p1"), ("c6", "disposed", "p1"), ("c7", "full", "p4"),
        ] {
            sqlx::query("INSERT INTO batch_containers VALUES (?, 'b1', 1, 10, 12.5, ?, 0)")
                .bind(id).bind(status).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO batch_placements VALUES (?, ?, ?)")
                .bind(format!("pl-{}", id)).bind(id).bind(pos).execute(&pool).await.unwrap();
        }
        pool
    }

    fn query(kind: LocationKind, id: &str) -> LocationQuery {
        LocationQuery { kind, target: LocationRef::Id(id.to_string()) }
    }

    #[tokio::test]
    async fn shelf_scan_lists_active_bottles_and_path() {
        let pool = setup().await;
        let d = resolve_location(&pool, &query(LocationKind::Position, "p1")).await.unwrap();
        assert_eq!(d.label.row.path, "Lab 101 / Cabinet A / Shelf 1");
        assert_eq!(d.label.row.room_color.as_deref(), Some("#667eea"));
        assert_eq!(d.bottle_count, 2); // empty and disposed are not counted
        assert_eq!(d.bottles.len(), 2);
        assert!(d.children.is_empty());
        assert!(d.label.payload.starts_with("CHL:L:P:"));
    }

    #[tokio::test]
    async fn cabinet_and_room_scans_count_per_child() {
        let pool = setup().await;
        let zone = resolve_location(&pool, &query(LocationKind::Zone, "z1")).await.unwrap();
        assert_eq!(zone.bottle_count, 3);
        let shelves: Vec<(String, i64)> = zone.children.iter().map(|c| (c.name.clone(), c.bottle_count)).collect();
        assert_eq!(shelves, vec![("Shelf 1".to_string(), 2), ("Shelf 2".to_string(), 1)]);
        assert!(zone.bottles.is_empty());

        let room = resolve_location(&pool, &query(LocationKind::Room, "1a2b3c4d-0000-4000-8000-000000000001")).await.unwrap();
        assert_eq!(room.bottle_count, 4);
        let zones: Vec<(String, i64)> = room.children.iter().map(|c| (c.name.clone(), c.bottle_count)).collect();
        assert_eq!(zones, vec![("Cabinet A".to_string(), 3), ("Fridge".to_string(), 1)]);
    }

    #[tokio::test]
    async fn typed_short_code_finds_the_room() {
        let pool = setup().await;
        let q = parse_location_code("R1A2B3C4D").unwrap();
        let d = resolve_location(&pool, &q).await.unwrap();
        assert_eq!(d.label.row.name, "Lab 101");
        assert!(resolve_location(&pool, &query(LocationKind::Position, "nope")).await.is_err());
        // A shelf id is not a room id
        assert!(resolve_location(&pool, &query(LocationKind::Room, "p1")).await.is_err());
    }

    #[tokio::test]
    async fn label_selection_follows_the_tree() {
        let pool = setup().await;
        let names = |v: Vec<LocationLabel>| v.into_iter().map(|l| format!("{}:{}", l.kind.letter(), l.row.name)).collect::<Vec<_>>();

        let one = select_location_labels(&pool, &LocationLabelsRequest { position_id: Some("p2".into()), ..Default::default() }).await.unwrap();
        assert_eq!(names(one), vec!["P:Shelf 2"]);

        let zone = select_location_labels(&pool, &LocationLabelsRequest {
            zone_id: Some("z1".into()), include_children: Some(true), ..Default::default()
        }).await.unwrap();
        assert_eq!(names(zone), vec!["Z:Cabinet A", "P:Shelf 1", "P:Shelf 2"]);

        let room = select_location_labels(&pool, &LocationLabelsRequest {
            room_id: Some("1a2b3c4d-0000-4000-8000-000000000001".into()), include_children: Some(true), ..Default::default()
        }).await.unwrap();
        assert_eq!(names(room), vec!["R:Lab 101", "Z:Cabinet A", "P:Shelf 1", "P:Shelf 2", "Z:Fridge", "P:Door"]);

        assert!(select_location_labels(&pool, &LocationLabelsRequest::default()).await.is_err());
        assert!(select_location_labels(&pool, &LocationLabelsRequest { zone_id: Some("none".into()), ..Default::default() }).await.is_err());
    }
}
