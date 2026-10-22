// src/label_handlers.rs
//! Bottle labels (Data Matrix) and scan lookup.
//!
//! One label = one physical bottle, i.e. one `batch_containers` row.
//! The symbol encodes `CHL:C:<container id as 32 uppercase hex>`:
//!   - the `CHL:` prefix tells our labels apart from manufacturer codes
//!     (suppliers print GS1 Data Matrix on their bottles);
//!   - compact uppercase hex keeps the symbol at 22×22 modules, against
//!     24×24 for the hyphenated lowercase UUID, which matters on vial labels.
//!
//! Each label also prints a short code (the first 8 hex digits), so a bottle
//! can still be found by typing when the camera cannot read the symbol.

use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::ApiResponse;
use crate::AppState;

pub const PAYLOAD_PREFIX: &str = "CHL:C:";
pub const SHORT_CODE_LEN: usize = 8;
/// Labels per request (about 30 A4 sheets of the smallest layout)
const MAX_LABELS: i64 = 2000;
/// Candidates shown for an ambiguous typed short code
const MAX_MATCHES: i64 = 10;
/// Ids accepted per selector in one labels request
const MAX_IDS: usize = 1000;

// ==================== PAYLOAD FORMAT ====================

/// Container id as printed in the symbol: 32 uppercase hex digits for UUIDs,
/// the id unchanged otherwise (ids created outside the app).
pub fn compact_id(id: &str) -> String {
    match Uuid::parse_str(id) {
        Ok(u) => u.simple().to_string().to_uppercase(),
        Err(_) => id.to_string(),
    }
}

pub fn container_payload(id: &str) -> String {
    format!("{}{}", PAYLOAD_PREFIX, compact_id(id))
}

pub fn short_code(id: &str) -> String {
    compact_id(id).chars().take(SHORT_CODE_LEN).collect()
}

#[derive(Debug, PartialEq)]
pub enum ScanQuery {
    /// Exact container id (hyphenated lowercase for UUIDs)
    Id(String),
    /// Hex prefix typed by hand (short code), lowercase, no dashes
    Prefix(String),
}

/// Turns whatever came from the camera, a USB scanner or the keyboard into
/// a lookup. Returns None for codes that are not ours.
pub fn parse_scan_code(raw: &str) -> Option<ScanQuery> {
    // Scanners may append CR/LF or GS; nothing printable is lost by dropping them
    let s: String = raw.chars().filter(|c| !c.is_control()).collect();
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    if s.len() > PAYLOAD_PREFIX.len()
        && s.is_char_boundary(PAYLOAD_PREFIX.len())
        && s[..PAYLOAD_PREFIX.len()].eq_ignore_ascii_case(PAYLOAD_PREFIX)
    {
        let token = s[PAYLOAD_PREFIX.len()..].trim();
        return Some(ScanQuery::Id(match Uuid::parse_str(token) {
            Ok(u) => u.hyphenated().to_string(),
            Err(_) => token.to_string(),
        }));
    }

    // A bare UUID (hyphenated or 32 hex), e.g. copied from the app
    if let Ok(u) = Uuid::parse_str(s) {
        return Some(ScanQuery::Id(u.hyphenated().to_string()));
    }

    // Typed short code: "24870DF5", "c 24870df5", "C-2487 0DF5"…
    // A leading "C" is a type marker only when a separator follows it,
    // since a hex code can itself start with C.
    let mut t = s.to_ascii_lowercase();
    for sep in ["c ", "c-", "c:", "c·"] {
        if let Some(rest) = t.strip_prefix(sep) {
            t = rest.to_string();
            break;
        }
    }
    let t: String = t.chars().filter(|c| *c != ' ' && *c != '-').collect();
    // Short codes are 8 characters; longer digit runs are other codes (GS1)
    if (6..=12).contains(&t.len()) && t.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(ScanQuery::Prefix(t));
    }
    None
}

// ==================== DATA ====================

/// Everything a label or a scan result shows about one bottle
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ContainerRow {
    pub container_id: String,
    pub sequence_number: i64,
    /// Containers ever created for the batch ("#3 of 8")
    pub batch_container_count: i64,
    pub quantity: f64,
    pub original_quantity: f64,
    pub container_status: String,
    pub is_opened: bool,

    pub batch_id: String,
    pub batch_number: String,
    pub lot_number: Option<String>,
    pub unit: String,
    pub batch_status: String,
    pub batch_quantity: f64,
    pub reserved_quantity: f64,
    pub expiry_date: Option<DateTime<Utc>>,
    pub received_date: DateTime<Utc>,

    pub reagent_id: String,
    pub reagent_name: String,
    pub formula: Option<String>,
    pub cas_number: Option<String>,
    pub hazard_pictograms: Option<String>,
    pub storage_conditions: Option<String>,

    pub placement_id: Option<String>,
    pub position_id: Option<String>,
    pub position_name: Option<String>,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub room_name: Option<String>,
    pub room_color: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ContainerLabel {
    #[serde(flatten)]
    pub row: ContainerRow,
    /// Text to encode in the Data Matrix symbol
    pub payload: String,
    /// Human-readable code printed under the symbol
    pub short_code: String,
}

impl From<ContainerRow> for ContainerLabel {
    fn from(row: ContainerRow) -> Self {
        let payload = container_payload(&row.container_id);
        let short_code = short_code(&row.container_id);
        Self { row, payload, short_code }
    }
}

pub(crate) const CONTAINER_ROW_SELECT: &str = r#"
    SELECT
        bc.id AS container_id,
        bc.sequence_number,
        (SELECT COUNT(*) FROM batch_containers x WHERE x.batch_id = bc.batch_id) AS batch_container_count,
        bc.quantity,
        bc.original_quantity,
        bc.status AS container_status,
        bc.is_opened,
        b.id AS batch_id,
        b.batch_number,
        b.lot_number,
        b.unit,
        b.status AS batch_status,
        b.quantity AS batch_quantity,
        b.reserved_quantity,
        b.expiry_date,
        b.received_date,
        r.id AS reagent_id,
        r.name AS reagent_name,
        r.formula,
        r.cas_number,
        r.hazard_pictograms,
        r.storage_conditions,
        bp.id AS placement_id,
        bp.position_id,
        sp.name AS position_name,
        sz.id AS zone_id,
        sz.name AS zone_name,
        rm.name AS room_name,
        rm.color AS room_color
    FROM batch_containers bc
    JOIN batches b ON b.id = bc.batch_id AND b.deleted_at IS NULL
    JOIN reagents r ON r.id = b.reagent_id AND r.deleted_at IS NULL
    LEFT JOIN batch_placements bp ON bp.container_id = bc.id
    LEFT JOIN storage_positions sp ON sp.id = bp.position_id
    LEFT JOIN storage_zones sz ON sz.id = sp.zone_id
    LEFT JOIN rooms rm ON rm.id = sz.room_id
"#;

pub(crate) const LABEL_ORDER: &str =
    "ORDER BY r.name COLLATE NOCASE, b.batch_number COLLATE NOCASE, bc.sequence_number";

pub async fn find_containers(pool: &SqlitePool, query: &ScanQuery) -> ApiResult<Vec<ContainerLabel>> {
    let rows: Vec<ContainerRow> = match query {
        ScanQuery::Id(id) => {
            let sql = format!("{} WHERE lower(bc.id) = lower(?)", CONTAINER_ROW_SELECT);
            sqlx::query_as(&sql).bind(id).fetch_all(pool).await?
        }
        ScanQuery::Prefix(prefix) => {
            // prefix is validated hex, so it holds no LIKE wildcards
            let sql = format!(
                "{} WHERE lower(replace(bc.id, '-', '')) LIKE ? {} LIMIT ?",
                CONTAINER_ROW_SELECT, LABEL_ORDER
            );
            sqlx::query_as(&sql)
                .bind(format!("{}%", prefix))
                .bind(MAX_MATCHES)
                .fetch_all(pool)
                .await?
        }
    };
    Ok(rows.into_iter().map(ContainerLabel::from).collect())
}

// ==================== HANDLERS ====================

#[derive(Debug, Deserialize)]
pub struct ResolveQuery {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct ScanResponse {
    /// One match for a scanned label; several only for an ambiguous typed code
    pub matches: Vec<ContainerLabel>,
}

/// GET /api/v1/labels/resolve?code=...
pub async fn resolve_scan(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<ResolveQuery>,
) -> ApiResult<HttpResponse> {
    let raw = query.code.as_str();
    let parsed = parse_scan_code(raw).ok_or_else(|| {
        let shown: String = raw.chars().filter(|c| !c.is_control()).take(60).collect();
        ApiError::BadRequest(format!(
            "Not a Chelate label: \"{}\". Scan the code printed by Chelate, or type its 8-character short code.",
            shown
        ))
    })?;

    let matches = find_containers(&app_state.db_pool, &parsed).await?;
    if matches.is_empty() {
        return Err(ApiError::NotFound(
            "No bottle with this label. Its batch may have been deleted.".to_string(),
        ));
    }
    Ok(HttpResponse::Ok().json(ApiResponse::success(ScanResponse { matches })))
}

#[derive(Debug, Deserialize, Default)]
pub struct LabelsRequest {
    pub batch_ids: Option<Vec<String>>,
    pub container_ids: Option<Vec<String>>,
    pub zone_id: Option<String>,
    /// Also label empty containers (disposed ones are never labelled)
    pub include_empty: Option<bool>,
}

pub async fn select_labels(pool: &SqlitePool, req: &LabelsRequest) -> ApiResult<Vec<ContainerLabel>> {
    let batch_ids = req.batch_ids.as_deref().unwrap_or(&[]);
    let container_ids = req.container_ids.as_deref().unwrap_or(&[]);
    let zone_id = req.zone_id.as_deref().filter(|z| !z.is_empty());

    if batch_ids.is_empty() && container_ids.is_empty() && zone_id.is_none() {
        return Err(ApiError::bad_request("Select batches, containers or a storage zone"));
    }
    if batch_ids.len() > MAX_IDS || container_ids.len() > MAX_IDS {
        return Err(ApiError::BadRequest(format!("At most {} ids per request", MAX_IDS)));
    }

    // Union of the selectors
    let mut any_of: Vec<String> = Vec::new();
    let mut params: Vec<&str> = Vec::new();
    let placeholders = |n: usize| vec!["?"; n].join(", ");
    if !batch_ids.is_empty() {
        any_of.push(format!("bc.batch_id IN ({})", placeholders(batch_ids.len())));
        params.extend(batch_ids.iter().map(String::as_str));
    }
    if !container_ids.is_empty() {
        any_of.push(format!("bc.id IN ({})", placeholders(container_ids.len())));
        params.extend(container_ids.iter().map(String::as_str));
    }
    if let Some(z) = zone_id {
        any_of.push("sp.zone_id = ?".to_string());
        params.push(z);
    }

    let status_filter = if req.include_empty.unwrap_or(false) {
        "bc.status != 'disposed'"
    } else {
        "bc.status NOT IN ('disposed', 'empty')"
    };

    let sql = format!(
        "{} WHERE ({}) AND {} {} LIMIT ?",
        CONTAINER_ROW_SELECT,
        any_of.join(" OR "),
        status_filter,
        LABEL_ORDER
    );
    let mut q = sqlx::query_as::<_, ContainerRow>(&sql);
    for p in params {
        q = q.bind(p);
    }
    let rows = q.bind(MAX_LABELS).fetch_all(pool).await?;
    Ok(rows.into_iter().map(ContainerLabel::from).collect())
}

/// POST /api/v1/labels/containers
pub async fn get_container_labels(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<LabelsRequest>,
) -> ApiResult<HttpResponse> {
    let labels = select_labels(&app_state.db_pool, &body).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(labels)))
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "24870df5-f571-4dc7-a4c4-dc02b225c49a";

    #[test]
    fn payload_is_prefixed_compact_hex() {
        assert_eq!(container_payload(ID), "CHL:C:24870DF5F5714DC7A4C4DC02B225C49A");
        assert_eq!(short_code(ID), "24870DF5");
        // Non-UUID ids pass through unchanged
        assert_eq!(container_payload("legacy-7"), "CHL:C:legacy-7");
    }

    #[test]
    fn parses_scanned_and_typed_codes() {
        let id = || Some(ScanQuery::Id(ID.to_string()));
        assert_eq!(parse_scan_code("CHL:C:24870DF5F5714DC7A4C4DC02B225C49A"), id());
        assert_eq!(parse_scan_code("chl:c:24870df5f5714dc7a4c4dc02b225c49a\r\n"), id());
        assert_eq!(parse_scan_code(&format!("CHL:C:{}", ID)), id());
        assert_eq!(parse_scan_code(ID), id());
        assert_eq!(parse_scan_code("CHL:C:legacy-7"), Some(ScanQuery::Id("legacy-7".into())));

        let prefix = |p: &str| Some(ScanQuery::Prefix(p.to_string()));
        assert_eq!(parse_scan_code("24870DF5"), prefix("24870df5"));
        assert_eq!(parse_scan_code(" c 2487-0df5 "), prefix("24870df5"));
        // Hex that starts with C is not mistaken for the type marker
        assert_eq!(parse_scan_code("C3F9A1C2"), prefix("c3f9a1c2"));

        // Not ours: GS1 manufacturer code, words, too short
        assert_eq!(parse_scan_code("010040123456789017270131101234"), None);
        assert_eq!(parse_scan_code("acetone"), None);
        assert_eq!(parse_scan_code("24870"), None);
        assert_eq!(parse_scan_code("   "), None);
        assert_eq!(parse_scan_code("CHL:C:"), None);
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
            "CREATE TABLE storage_positions (id TEXT PRIMARY KEY, zone_id TEXT, name TEXT)",
            "CREATE TABLE storage_zones (id TEXT PRIMARY KEY, room_id TEXT, name TEXT)",
            "CREATE TABLE rooms (id TEXT PRIMARY KEY, name TEXT, color TEXT)",
            "INSERT INTO reagents VALUES ('r1', 'Benzaldehyde', 'C7H6O', '100-52-7', 'GHS07', NULL, NULL)",
            "INSERT INTO reagents VALUES ('r2', 'Acetone', NULL, NULL, NULL, NULL, NULL)",
            "INSERT INTO batches VALUES ('b1', 'r1', 'BZ-1', NULL, 'mL', 'available', 50, 0, \
             '2027-05-01T00:00:00Z', '2026-01-10T00:00:00Z', NULL)",
            "INSERT INTO batches VALUES ('b2', 'r2', 'AC-1', NULL, 'L', 'available', 1, 0, NULL, \
             '2026-01-10T00:00:00Z', NULL)",
            "INSERT INTO batches VALUES ('b3', 'r2', 'AC-OLD', NULL, 'L', 'available', 1, 0, NULL, \
             '2026-01-10T00:00:00Z', '2026-02-01T00:00:00Z')",
            "INSERT INTO rooms VALUES ('room', 'Lab 101', '#667eea')",
            "INSERT INTO storage_zones VALUES ('z1', 'room', 'Cabinet A')",
            "INSERT INTO storage_positions VALUES ('p1', 'z1', 'Shelf 1')",
        ] {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }
        for (id, batch, seq, status) in [
            (ID, "b1", 1, "partial"),
            ("24870df5-0000-4000-8000-000000000002", "b1", 2, "full"), // same short code
            ("aaaa0000-0000-4000-8000-000000000003", "b1", 3, "empty"),
            ("bbbb0000-0000-4000-8000-000000000004", "b2", 1, "full"),
            ("cccc0000-0000-4000-8000-000000000005", "b3", 1, "full"), // deleted batch
            ("dddd0000-0000-4000-8000-000000000006", "b2", 2, "disposed"),
        ] {
            sqlx::query("INSERT INTO batch_containers VALUES (?, ?, ?, 10, 12.5, ?, 0)")
                .bind(id).bind(batch).bind(seq).bind(status)
                .execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO batch_placements VALUES ('pl1', ?, 'p1')")
            .bind(ID).execute(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn scan_finds_bottle_with_location() {
        let pool = setup().await;
        let q = parse_scan_code(&container_payload(ID)).unwrap();
        let found = find_containers(&pool, &q).await.unwrap();
        assert_eq!(found.len(), 1);
        let c = &found[0];
        assert_eq!(c.row.reagent_name, "Benzaldehyde");
        assert_eq!(c.row.batch_container_count, 3);
        assert_eq!(c.row.position_name.as_deref(), Some("Shelf 1"));
        assert_eq!(c.row.room_name.as_deref(), Some("Lab 101"));
        assert_eq!(c.short_code, "24870DF5");
    }

    #[tokio::test]
    async fn typed_short_code_can_be_ambiguous_and_skips_deleted_batches() {
        let pool = setup().await;
        let found = find_containers(&pool, &ScanQuery::Prefix("24870df5".into())).await.unwrap();
        assert_eq!(found.len(), 2);
        let gone = find_containers(&pool, &ScanQuery::Prefix("cccc0000".into())).await.unwrap();
        assert!(gone.is_empty());
    }

    #[tokio::test]
    async fn labels_select_union_and_skip_empty_and_disposed() {
        let pool = setup().await;
        let by_batch = select_labels(&pool, &LabelsRequest {
            batch_ids: Some(vec!["b1".into(), "b2".into()]), ..Default::default()
        }).await.unwrap();
        // Acetone before Benzaldehyde; empty and disposed containers left out
        let seqs: Vec<(String, i64)> = by_batch.iter()
            .map(|l| (l.row.batch_number.clone(), l.row.sequence_number)).collect();
        assert_eq!(seqs, vec![("AC-1".into(), 1), ("BZ-1".into(), 1), ("BZ-1".into(), 2)]);

        let with_empty = select_labels(&pool, &LabelsRequest {
            batch_ids: Some(vec!["b1".into()]), include_empty: Some(true), ..Default::default()
        }).await.unwrap();
        assert_eq!(with_empty.len(), 3);

        let by_zone = select_labels(&pool, &LabelsRequest {
            zone_id: Some("z1".into()), ..Default::default()
        }).await.unwrap();
        assert_eq!(by_zone.len(), 1);
        assert_eq!(by_zone[0].row.container_id, ID);

        assert!(select_labels(&pool, &LabelsRequest::default()).await.is_err());
    }
}
