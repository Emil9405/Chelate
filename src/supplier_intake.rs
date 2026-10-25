// src/supplier_intake.rs
//! Receiving goods by scanning the supplier's own code.
//!
//! A scanned supplier code (see `supplier_codes`) names an article, a batch and
//! sometimes an expiry date. Two small tables remember what Chelate has learned
//! about articles, so that each article needs a person only once:
//!
//! - `supplier_products`: (series, article) -> reagent, plus the base unit of
//!   that article's pack codes (Merck packs are a code like `1000`, not a quantity);
//! - `supplier_gtins`: GTIN -> (series, article), learnt from GS1 scans, so that the
//!   EAN-13 on the same bottle finds the article too.
//!
//! `receive` turns one scanned batch into a batch with N bottles in a single
//! transaction: the batch (or more bottles added to an existing batch), the
//! containers, optionally their shelf (journaled as scan placements) and the
//! article mapping.
//!
//! Batch numbers typed by hand differ from the printed ones (`l1322907` for
//! `I1322907`, a cut-off digit, the pack size typed after the number), so a new
//! batch number is checked against the existing ones first (`check_batch`) and
//! a similar one has to be chosen or confirmed as new.
//!
//! `backfill` fills `supplier_products` from the catalog numbers already stored
//! in batches, so articles that are in Chelate are known at the first scan.

use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use uuid::Uuid;

use crate::audit::{self, ChangeSet};
use crate::auth;
use crate::auth_handlers::{self, BatchAction};
use crate::container_history;
use crate::error::{ApiError, ApiResult};
use crate::handlers::ApiResponse;
use crate::supplier_codes::{article_key, typed_merck_pack_code, ArticleKey, Series, SupplierCode};
use crate::AppState;

pub const MAX_BOTTLES: i64 = 500;

pub const MIGRATIONS: &[&str] = &[
    r#"CREATE TABLE IF NOT EXISTS supplier_products (
        id TEXT PRIMARY KEY,
        series TEXT NOT NULL CHECK(series IN ('merck','sigma','other')),
        catalog TEXT NOT NULL,
        reagent_id TEXT NOT NULL REFERENCES reagents(id) ON DELETE CASCADE,
        pack_unit TEXT CHECK(pack_unit IS NULL OR pack_unit IN ('g','mL')),
        created_by TEXT,
        created_at DATETIME NOT NULL,
        updated_at DATETIME NOT NULL,
        UNIQUE(series, catalog)
    )"#,
    "CREATE INDEX IF NOT EXISTS idx_supplier_products_reagent ON supplier_products(reagent_id)",
    r#"CREATE TABLE IF NOT EXISTS supplier_gtins (
        gtin TEXT PRIMARY KEY,
        series TEXT NOT NULL,
        catalog TEXT NOT NULL
    )"#,
    // What a Merck pack code holds for one article: `2511` of 1.05432 is 2.5 L
    r#"CREATE TABLE IF NOT EXISTS supplier_packs (
        series TEXT NOT NULL,
        catalog TEXT NOT NULL,
        pack_code TEXT NOT NULL,
        value REAL NOT NULL CHECK(value > 0),
        unit TEXT NOT NULL,
        updated_at DATETIME NOT NULL,
        PRIMARY KEY (series, catalog, pack_code)
    )"#,
];

pub async fn run_migrations(pool: &SqlitePool) -> ApiResult<()> {
    for ddl in MIGRATIONS {
        sqlx::query(ddl).execute(pool).await?;
    }
    Ok(())
}

// ==================== SCAN LOOKUP ====================

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MappedReagent {
    pub id: String,
    pub name: String,
    pub cas_number: Option<String>,
    pub formula: Option<String>,
    pub storage_conditions: Option<String>,
    /// Base unit of this article's Merck pack codes (`g` or `mL`), if known
    pub pack_unit: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExistingBatch {
    pub batch_id: String,
    pub batch_number: String,
    pub pack_size: Option<f64>,
    pub unit: String,
    pub quantity: f64,
    pub expiry_date: Option<DateTime<Utc>>,
    /// Bottles of this batch already in Chelate (not disposed)
    pub bottles: i64,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct SuggestedPack {
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Serialize)]
pub struct SupplierScan {
    pub code: SupplierCode,
    /// The reagent this article was linked to before; None for a new article
    pub reagent: Option<MappedReagent>,
    /// The same batch number already received for that reagent
    pub batch: Option<ExistingBatch>,
    /// Batches of that reagent whose number is probably this one, typed differently
    pub similar_batches: Vec<SimilarBatch>,
    /// This batch number recorded under another reagent (with no reagent known yet:
    /// every reagent), a hint that the article belongs there
    pub batch_elsewhere: Vec<SimilarBatch>,
    pub suggested_pack: Option<SuggestedPack>,
    pub expired: bool,
}

/// What the code alone says about the pack, or what a stored base unit adds.
/// A Merck pack code is only turned into a quantity for round codes: `2511` is a
/// special pack, not 2511 g, so it is left for a person to enter (and remembered
/// afterwards, see `pack_suggestion`).
pub fn suggest_pack(code: &SupplierCode, base_unit: Option<&str>) -> Option<SuggestedPack> {
    let pack = code.pack.as_ref()?;
    if let (Some(value), Some(unit)) = (pack.value, pack.unit.as_deref()) {
        return Some(SuggestedPack { value, unit: unit.to_string() });
    }
    if code.series != Series::Merck {
        return None;
    }
    merck_round_pack(&pack.raw, base_unit?)
}

/// `1000` with base unit mL is 1 L, `0500` with g is 500 g; `2511` is no quantity
fn merck_round_pack(pack_code: &str, base_unit: &str) -> Option<SuggestedPack> {
    if pack_code.is_empty() || !pack_code.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let n: f64 = pack_code.parse().ok()?;
    if n <= 0.0 || (n as i64) % 10 != 0 || !matches!(base_unit, "g" | "mL") {
        return None;
    }
    Some(readable_pack(n, base_unit))
}

/// 2500 mL reads as 2.5 L, 1000 g as 1 kg
fn readable_pack(value: f64, unit: &str) -> SuggestedPack {
    match canonical_unit(unit) {
        Some("mL") if value >= 1000.0 => SuggestedPack { value: round9(value / 1000.0), unit: "L".to_string() },
        Some("g") if value >= 1000.0 => SuggestedPack { value: round9(value / 1000.0), unit: "kg".to_string() },
        Some(c) => SuggestedPack { value, unit: c.to_string() },
        None => SuggestedPack { value, unit: unit.to_string() },
    }
}

/// `g` or `mL` when all batches of the reagent are kept in units of one kind
async fn reagent_base_unit(conn: &mut sqlx::SqliteConnection, reagent_id: &str) -> ApiResult<Option<&'static str>> {
    let units: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT unit FROM batches WHERE reagent_id = ? AND deleted_at IS NULL",
    )
    .bind(reagent_id)
    .fetch_all(&mut *conn)
    .await?;
    let kinds: BTreeSet<u8> = units
        .iter()
        .filter_map(|u| canonical_unit(u).and_then(unit_scale).map(|(k, _)| k))
        .collect();
    Ok(match kinds.iter().collect::<Vec<_>>().as_slice() {
        [0] => Some("g"),
        [1] => Some("mL"),
        _ => None,
    })
}

/// Quantity in one bottle of an article with this pack code: what was received
/// before, else a round Merck code read with the article's (or the reagent's) unit.
pub async fn pack_suggestion(
    conn: &mut sqlx::SqliteConnection,
    series: &str,
    catalog: &str,
    pack_code: &str,
    reagent_id: Option<&str>,
) -> ApiResult<Option<SuggestedPack>> {
    let catalog = catalog.trim().to_uppercase();
    let pack_code = pack_code.trim();
    if catalog.is_empty() || pack_code.is_empty() {
        return Ok(None);
    }
    let known: Option<(f64, String)> = sqlx::query_as(
        "SELECT value, unit FROM supplier_packs WHERE series = ? AND catalog = ? AND pack_code = ?",
    )
    .bind(series)
    .bind(&catalog)
    .bind(pack_code)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some((value, unit)) = known {
        return Ok(Some(readable_pack(value, &unit)));
    }
    if series != Series::Merck.as_str() {
        return Ok(None);
    }
    let stored: Option<Option<String>> = sqlx::query_scalar(
        "SELECT pack_unit FROM supplier_products WHERE series = ? AND catalog = ?",
    )
    .bind(series)
    .bind(&catalog)
    .fetch_optional(&mut *conn)
    .await?;
    let base = match (stored.flatten(), reagent_id) {
        (Some(u), _) => Some(if u == "g" { "g" } else { "mL" }),
        (None, Some(r)) => reagent_base_unit(conn, r).await?,
        (None, None) => None,
    };
    Ok(base.and_then(|b| merck_round_pack(pack_code, b)))
}

// ==================== UNITS ====================

/// One spelling per unit. Batches typed by hand carry `ML`, `ml`, `G`, `KG`, `PIECE`…
pub fn canonical_unit(unit: &str) -> Option<&'static str> {
    Some(match unit.trim().to_lowercase().as_str() {
        "g" | "gr" => "g",
        "kg" => "kg",
        "mg" => "mg",
        "µg" | "μg" | "ug" => "μg",
        "ml" => "mL",
        "l" => "L",
        "µl" | "μl" | "ul" => "μL",
        "pcs" | "pc" | "piece" | "pieces" | "шт" => "pcs",
        _ => return None,
    })
}

fn unit_scale(canonical: &str) -> Option<(u8, f64)> {
    Some(match canonical {
        "μg" => (0, 1e-6),
        "mg" => (0, 1e-3),
        "g" => (0, 1.0),
        "kg" => (0, 1e3),
        "μL" => (1, 1e-3),
        "mL" => (1, 1.0),
        "L" => (1, 1e3),
        "pcs" => (2, 1.0),
        _ => return None,
    })
}

/// Factor that turns a quantity in `from` into one in `to`; None when they do not convert
pub fn unit_factor(from: &str, to: &str) -> Option<f64> {
    if from.trim() == to.trim() {
        return Some(1.0);
    }
    let (kf, sf) = unit_scale(canonical_unit(from)?)?;
    let (kt, st) = unit_scale(canonical_unit(to)?)?;
    (kf == kt).then(|| sf / st)
}

fn round9(v: f64) -> f64 {
    (v * 1e9).round() / 1e9
}

fn fmt_qty(v: f64) -> String {
    let s = format!("{:.6}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

// ==================== SIMILAR BATCH NUMBERS ====================

lazy_static::lazy_static! {
    /// What is left of a batch number after the real one: a pack size typed after it
    /// (`SHBP6353` + `5G`, `BCCH8042` + `0500`, `MKCT9494` + `-2.5`), compared on keys
    static ref PACK_TAIL_RE: regex::Regex =
        regex::Regex::new(r"^[-./]?\d+(?:[.,]\d+)?(?:KG|MG|G|M1|1)?$").unwrap();
}

/// Batch number with case, spaces and look-alike characters (O/0, I/l/1) folded
pub fn batch_key(s: &str) -> String {
    s.trim()
        .to_uppercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        })
        .collect()
}

/// Why `other` is probably the same batch as `typed`, or None. Equal numbers give `same`.
pub fn batch_similarity(typed: &str, other: &str) -> Option<&'static str> {
    let (a, b) = (typed.trim(), other.trim());
    if a.is_empty() || b.is_empty() {
        return None;
    }
    if a == b {
        return Some("same");
    }
    if a.eq_ignore_ascii_case(b) {
        return Some("case");
    }
    let (ka, kb) = (batch_key(a), batch_key(b));
    if ka == kb {
        return Some("lookalike");
    }
    let (short, long) = if ka.len() < kb.len() { (&ka, &kb) } else { (&kb, &ka) };
    if short.chars().count() < 6 {
        return None;
    }
    let tail = long.strip_prefix(short.as_str())?;
    if tail.len() <= 2 && tail.chars().all(|c| c.is_ascii_digit()) {
        Some("cut")
    } else if tail.len() <= 7 && PACK_TAIL_RE.is_match(tail) {
        Some("pack")
    } else {
        None
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct SimilarBatch {
    pub batch_id: String,
    pub batch_number: String,
    pub reagent_id: String,
    pub reagent_name: String,
    pub unit: String,
    pub pack_size: Option<f64>,
    pub bottles: i64,
    /// `same`, `case`, `lookalike` (O/0, I/l/1), `cut` (one or two characters missing
    /// at the end), `pack` (a pack size typed after the number)
    pub reason: &'static str,
}

#[derive(Debug, Serialize, Default)]
pub struct BatchCheck {
    /// The same batch number under this reagent: bottles are added to it
    pub exact: Option<ExistingBatch>,
    /// A batch of this reagent whose number is probably the same, typed differently
    pub similar: Vec<SimilarBatch>,
    /// The same or a similar number recorded under another reagent
    pub elsewhere: Vec<SimilarBatch>,
    /// Quantity in one bottle, when the article and pack code were given and tell it
    pub suggested_pack: Option<SuggestedPack>,
}

impl BatchCheck {
    pub fn needs_confirmation(&self) -> bool {
        self.exact.is_none() && (!self.similar.is_empty() || !self.elsewhere.is_empty())
    }
}

#[derive(sqlx::FromRow)]
struct BatchRow {
    batch_id: String,
    batch_number: String,
    reagent_id: String,
    reagent_name: String,
    pack_size: Option<f64>,
    unit: String,
    quantity: f64,
    expiry_date: Option<DateTime<Utc>>,
    bottles: i64,
}

const MAX_SIMILAR: usize = 5;

/// Looks for `number` among the batches in Chelate: under `reagent_id` (exact and
/// similar numbers) and under other reagents.
pub async fn check_batch(
    conn: &mut sqlx::SqliteConnection,
    reagent_id: Option<&str>,
    number: &str,
) -> ApiResult<BatchCheck> {
    let number = number.trim();
    let mut check = BatchCheck::default();
    if number.is_empty() {
        return Ok(check);
    }
    let len = number.chars().count() as i64;
    let rows: Vec<BatchRow> = sqlx::query_as(
        "SELECT b.id AS batch_id, b.batch_number, b.reagent_id, r.name AS reagent_name, b.pack_size, b.unit, \
           b.quantity, b.expiry_date, \
           (SELECT COUNT(*) FROM batch_containers bc WHERE bc.batch_id = b.id AND bc.status != 'disposed') AS bottles \
         FROM batches b JOIN reagents r ON r.id = b.reagent_id \
         WHERE b.deleted_at IS NULL AND r.deleted_at IS NULL \
           AND length(b.batch_number) BETWEEN ? AND ? \
         ORDER BY b.received_date DESC",
    )
    .bind(len - 8)
    .bind(len + 8)
    .fetch_all(&mut *conn)
    .await?;

    for row in rows {
        let Some(reason) = batch_similarity(number, &row.batch_number) else { continue };
        let mine = reagent_id == Some(row.reagent_id.as_str());
        if mine && reason == "same" {
            check.exact = Some(ExistingBatch {
                batch_id: row.batch_id,
                batch_number: row.batch_number,
                pack_size: row.pack_size,
                unit: row.unit,
                quantity: row.quantity,
                expiry_date: row.expiry_date,
                bottles: row.bottles,
            });
            continue;
        }
        let list = if mine { &mut check.similar } else { &mut check.elsewhere };
        if list.len() < MAX_SIMILAR {
            list.push(SimilarBatch {
                batch_id: row.batch_id,
                batch_number: row.batch_number,
                reagent_id: row.reagent_id,
                reagent_name: row.reagent_name,
                unit: row.unit,
                pack_size: row.pack_size,
                bottles: row.bottles,
                reason,
            });
        }
    }
    Ok(check)
}

async fn find_reagent(pool: &SqlitePool, series: &str, catalog: &str) -> ApiResult<Option<MappedReagent>> {
    Ok(sqlx::query_as::<_, MappedReagent>(
        "SELECT r.id, r.name, r.cas_number, r.formula, r.storage_conditions, sp.pack_unit \
         FROM supplier_products sp JOIN reagents r ON r.id = sp.reagent_id \
         WHERE sp.series = ? AND sp.catalog = ? AND r.deleted_at IS NULL",
    )
    .bind(series)
    .bind(catalog)
    .fetch_optional(pool)
    .await?)
}

pub async fn scan_info(pool: &SqlitePool, code: SupplierCode) -> ApiResult<SupplierScan> {
    let mut reagent = match code.catalog.as_deref() {
        Some(cat) => find_reagent(pool, code.series.as_str(), cat).await?,
        None => None,
    };
    if reagent.is_none() {
        if let Some(gtin) = code.gtin.as_deref() {
            let known: Option<(String, String)> =
                sqlx::query_as("SELECT series, catalog FROM supplier_gtins WHERE gtin = ?")
                    .bind(gtin)
                    .fetch_optional(pool)
                    .await?;
            if let Some((series, catalog)) = known {
                reagent = find_reagent(pool, &series, &catalog).await?;
            }
        }
    }

    let check = match code.batch.as_deref() {
        Some(number) => {
            let mut conn = pool.acquire().await?;
            check_batch(&mut conn, reagent.as_ref().map(|r| r.id.as_str()), number).await?
        }
        None => BatchCheck::default(),
    };

    let base_unit = reagent.as_ref().and_then(|r| r.pack_unit.as_deref());
    let mut suggested_pack = suggest_pack(&code, base_unit);
    if suggested_pack.is_none() {
        if let (Some(cat), Some(pack)) = (code.catalog.as_deref(), code.pack.as_ref()) {
            let mut conn = pool.acquire().await?;
            suggested_pack = pack_suggestion(
                &mut conn,
                code.series.as_str(),
                cat,
                &pack.raw,
                reagent.as_ref().map(|r| r.id.as_str()),
            )
            .await?;
        }
    }
    let expired = code.expiry.map(|d| d < Utc::now().date_naive()).unwrap_or(false);
    Ok(SupplierScan {
        code,
        reagent,
        batch: check.exact,
        similar_batches: check.similar,
        batch_elsewhere: check.elsewhere,
        suggested_pack,
        expired,
    })
}

// ==================== RECEIVE ====================

#[derive(Debug, Deserialize)]
pub struct RememberProduct {
    pub series: String,
    pub catalog: String,
    pub gtin: Option<String>,
    /// `g` or `mL`: what a Merck pack code counts
    pub pack_unit: Option<String>,
    /// The Merck pack code of these bottles (`2511`): the quantity received is
    /// remembered for it and offered next time
    #[serde(default)]
    pub pack_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReceiveRequest {
    pub reagent_id: String,
    pub batch_number: String,
    pub bottles: i64,
    /// Quantity in one bottle, in `unit`
    pub pack_size: f64,
    pub unit: String,
    pub expiry_date: Option<NaiveDate>,
    pub cat_number: Option<String>,
    pub supplier: Option<String>,
    pub manufacturer: Option<String>,
    /// Sigma's Pcode, kept in the batch notes
    pub pcode: Option<String>,
    /// Shelf for all new bottles
    pub position_id: Option<String>,
    pub source: Option<String>,
    pub remember: Option<RememberProduct>,
    /// A batch with a similar number exists (see `BatchCheck`) and the person
    /// confirmed this one is new
    #[serde(default)]
    pub confirm_new_batch: bool,
}

#[derive(Debug, Serialize)]
pub struct ReceiveResult {
    pub batch_id: String,
    pub created_batch: bool,
    /// New bottles in sequence order, for printing their labels
    pub container_ids: Vec<String>,
    pub placed: usize,
    /// Bottles of the batch after this receipt
    pub batch_bottles: i64,
    pub warnings: Vec<String>,
}

fn clean(s: &Option<String>) -> Option<String> {
    s.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

fn validate(req: &ReceiveRequest) -> ApiResult<()> {
    let bad = |m: &str| Err(ApiError::BadRequest(m.to_string()));
    let batch = req.batch_number.trim();
    if batch.is_empty() || batch.chars().count() > 100 {
        return bad("Batch number must be 1 to 100 characters");
    }
    if req.bottles < 1 || req.bottles > MAX_BOTTLES {
        return bad(&format!("Bottles must be between 1 and {}", MAX_BOTTLES));
    }
    if !req.pack_size.is_finite() || req.pack_size <= 0.0 {
        return bad("Quantity per bottle must be positive");
    }
    let unit = req.unit.trim();
    if unit.is_empty() || unit.chars().count() > 20 {
        return bad("Unit must be 1 to 20 characters");
    }
    for (name, v, max) in [
        ("Catalog number", &req.cat_number, 100),
        ("Supplier", &req.supplier, 255),
        ("Manufacturer", &req.manufacturer, 255),
        ("Pcode", &req.pcode, 100),
    ] {
        if v.as_deref().map(|s| s.chars().count() > max).unwrap_or(false) {
            return bad(&format!("{} is too long", name));
        }
    }
    if let Some(r) = &req.remember {
        if !["merck", "sigma", "other"].contains(&r.series.as_str()) {
            return bad("Unknown article series");
        }
        if r.catalog.trim().is_empty() || r.catalog.chars().count() > 100 {
            return bad("Article number must be 1 to 100 characters");
        }
        if let Some(u) = r.pack_unit.as_deref() {
            if u != "g" && u != "mL" {
                return bad("Pack unit must be g or mL");
            }
        }
        if let Some(g) = r.gtin.as_deref() {
            if g.len() != 14 || !g.chars().all(|c| c.is_ascii_digit()) {
                return bad("GTIN must be 14 digits");
            }
        }
        if let Some(p) = r.pack_code.as_deref() {
            if p.is_empty() || p.len() > 10 || !p.chars().all(|c| c.is_ascii_digit()) {
                return bad("Pack code must be 1 to 10 digits");
            }
        }
    }
    Ok(())
}

pub async fn receive(pool: &SqlitePool, user_id: &str, req: &ReceiveRequest) -> ApiResult<ReceiveResult> {
    validate(req)?;
    let batch_number = req.batch_number.trim().to_string();
    let unit = canonical_unit(&req.unit).map(str::to_string).unwrap_or_else(|| req.unit.trim().to_string());
    let source = container_history::client_source(req.source.as_deref());
    let now = Utc::now();
    let mut warnings = Vec::new();

    let mut tx = pool.begin().await?;

    let reagent: Option<String> =
        sqlx::query_scalar("SELECT id FROM reagents WHERE id = ? AND deleted_at IS NULL")
            .bind(&req.reagent_id)
            .fetch_optional(&mut *tx)
            .await?;
    if reagent.is_none() {
        return Err(ApiError::NotFound("Reagent not found".to_string()));
    }
    if let Some(pos) = &req.position_id {
        let ok: Option<String> = sqlx::query_scalar("SELECT id FROM storage_positions WHERE id = ?")
            .bind(pos)
            .fetch_optional(&mut *tx)
            .await?;
        if ok.is_none() {
            return Err(ApiError::NotFound("Shelf not found".to_string()));
        }
    }

    let existing: Option<(String, String, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, unit, expiry_date FROM batches \
         WHERE reagent_id = ? AND batch_number = ? AND deleted_at IS NULL",
    )
    .bind(&req.reagent_id)
    .bind(&batch_number)
    .fetch_optional(&mut *tx)
    .await?;

    if existing.is_none() && !req.confirm_new_batch {
        let check = check_batch(&mut *tx, Some(&req.reagent_id), &batch_number).await?;
        if check.needs_confirmation() {
            let mut seen: Vec<String> = check
                .similar
                .iter()
                .map(|b| b.batch_number.clone())
                .chain(check.elsewhere.iter().map(|b| format!("{} ({})", b.batch_number, b.reagent_name)))
                .collect();
            seen.dedup();
            return Err(ApiError::BadRequest(format!(
                "A similar batch is already in Chelate: {}. Add the bottles to it, or confirm that {} is a new batch.",
                seen.join(", "),
                batch_number
            )));
        }
    }

    // Bottles are counted in the batch's own unit; 2.5 L bottles of a batch kept in mL are 2500 mL
    let pack_size = match &existing {
        Some((_, existing_unit, _)) => {
            let factor = unit_factor(&unit, existing_unit).ok_or_else(|| {
                ApiError::BadRequest(format!(
                    "This batch is recorded in {}, which {} does not convert to. Use {} or change the batch first.",
                    existing_unit, unit, existing_unit
                ))
            })?;
            let converted = round9(req.pack_size * factor);
            if canonical_unit(&unit) != canonical_unit(existing_unit) {
                let shown = canonical_unit(existing_unit).unwrap_or(existing_unit);
                warnings.push(format!(
                    "The batch is kept in {}: each bottle of {} {} was recorded as {} {}.",
                    shown, fmt_qty(req.pack_size), unit, fmt_qty(converted), shown
                ));
            }
            converted
        }
        None => req.pack_size,
    };
    let added = round9(pack_size * req.bottles as f64);

    let (batch_id, created_batch) = match existing {
        Some((id, _existing_unit, existing_expiry)) => {
            if let (Some(have), Some(new)) = (existing_expiry, req.expiry_date) {
                if have.date_naive() != new {
                    warnings.push(format!(
                        "The batch already has expiry {}; the scanned {} was not applied.",
                        have.date_naive(), new
                    ));
                }
            }
            let expiry = req
                .expiry_date
                .filter(|_| existing_expiry.is_none())
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc());
            sqlx::query(
                "UPDATE batches SET quantity = quantity + ?, original_quantity = original_quantity + ?, \
                   pack_size = COALESCE(pack_size, ?), expiry_date = COALESCE(expiry_date, ?), \
                   status = CASE WHEN status = 'depleted' THEN 'available' ELSE status END, \
                   updated_by = ?, updated_at = ? WHERE id = ?",
            )
            .bind(added)
            .bind(added)
            .bind(pack_size)
            .bind(expiry)
            .bind(user_id)
            .bind(now)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
            (id, false)
        }
        None => {
            // A soft-deleted batch with this number would collide with UNIQUE(reagent_id, batch_number)
            sqlx::query(
                "UPDATE batches SET batch_number = batch_number || '_deleted_' || id \
                 WHERE reagent_id = ? AND batch_number = ? AND deleted_at IS NOT NULL",
            )
            .bind(&req.reagent_id)
            .bind(&batch_number)
            .execute(&mut *tx)
            .await?;

            let id = Uuid::new_v4().to_string();
            let notes = clean(&req.pcode).map(|p| format!("Supplier Pcode: {}", p));
            let expiry = req.expiry_date.map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc());
            sqlx::query(
                "INSERT INTO batches (id, reagent_id, lot_number, batch_number, cat_number, \
                   quantity, original_quantity, reserved_quantity, unit, pack_size, expiry_date, \
                   supplier, manufacturer, received_date, status, location, notes, \
                   created_by, updated_by, created_at, updated_at) \
                 VALUES (?, ?, NULL, ?, ?, ?, ?, 0.0, ?, ?, ?, ?, ?, ?, 'available', NULL, ?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&req.reagent_id)
            .bind(&batch_number)
            .bind(clean(&req.cat_number))
            .bind(added)
            .bind(added)
            .bind(&unit)
            .bind(pack_size)
            .bind(expiry)
            .bind(clean(&req.supplier))
            .bind(clean(&req.manufacturer))
            .bind(now)
            .bind(notes)
            .bind(user_id)
            .bind(user_id)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;
            (id, true)
        }
    };

    let mut seq: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(sequence_number), 0) FROM batch_containers WHERE batch_id = ?",
    )
    .bind(&batch_id)
    .fetch_one(&mut *tx)
    .await?;

    let mut container_ids = Vec::with_capacity(req.bottles as usize);
    let mut placed = 0;
    for _ in 0..req.bottles {
        seq += 1;
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO batch_containers (id, batch_id, sequence_number, quantity, original_quantity, \
               is_opened, status, notes, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, 0, 'full', NULL, ?, ?)",
        )
        .bind(&id)
        .bind(&batch_id)
        .bind(seq)
        .bind(pack_size)
        .bind(pack_size)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        if let Some(pos) = &req.position_id {
            container_history::change_placement(&mut tx, &id, Some(pos), Some(user_id), source, None, None).await?;
            placed += 1;
        }
        container_ids.push(id);
    }

    if let Some(r) = &req.remember {
        remember_product(&mut tx, r, &req.reagent_id, user_id, Some((req.pack_size, unit.as_str()))).await?;
    }

    let batch_bottles: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM batch_containers WHERE batch_id = ? AND status != 'disposed'",
    )
    .bind(&batch_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(ReceiveResult { batch_id, created_batch, container_ids, placed, batch_bottles, warnings })
}

async fn remember_product(
    conn: &mut sqlx::SqliteConnection,
    r: &RememberProduct,
    reagent_id: &str,
    user_id: &str,
    received: Option<(f64, &str)>,
) -> ApiResult<()> {
    let catalog = r.catalog.trim().to_uppercase();
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO supplier_products (id, series, catalog, reagent_id, pack_unit, created_by, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(series, catalog) DO UPDATE SET reagent_id = excluded.reagent_id, \
           pack_unit = COALESCE(excluded.pack_unit, supplier_products.pack_unit), updated_at = excluded.updated_at",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&r.series)
    .bind(&catalog)
    .bind(reagent_id)
    .bind(r.pack_unit.as_deref())
    .bind(user_id)
    .bind(now)
    .bind(now)
    .execute(&mut *conn)
    .await?;
    if let Some(g) = r.gtin.as_deref() {
        sqlx::query(
            "INSERT INTO supplier_gtins (gtin, series, catalog) VALUES (?, ?, ?) \
             ON CONFLICT(gtin) DO UPDATE SET series = excluded.series, catalog = excluded.catalog",
        )
        .bind(g)
        .bind(&r.series)
        .bind(&catalog)
        .execute(&mut *conn)
        .await?;
    }
    if let (Some(code), Some((value, unit)), "merck") = (r.pack_code.as_deref(), received, r.series.as_str()) {
        sqlx::query(
            "INSERT INTO supplier_packs (series, catalog, pack_code, value, unit, updated_at) VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(series, catalog, pack_code) DO UPDATE SET value = excluded.value, unit = excluded.unit, \
               updated_at = excluded.updated_at",
        )
        .bind(&r.series)
        .bind(&catalog)
        .bind(code)
        .bind(value)
        .bind(canonical_unit(unit).unwrap_or(unit))
        .bind(Utc::now())
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

// ==================== BACKFILL ====================

#[derive(Debug, Serialize, Clone)]
pub struct BackfillReagent {
    pub reagent_id: String,
    pub reagent_name: String,
    pub batches: i64,
}

#[derive(Debug, Serialize)]
pub struct BackfillLink {
    pub series: String,
    pub catalog: String,
    pub reagent_id: String,
    pub reagent_name: String,
    pub batches: i64,
    pub pack_unit: Option<String>,
}

/// An article that cannot be linked without a person
#[derive(Debug, Serialize)]
pub struct BackfillConflict {
    pub series: String,
    pub catalog: String,
    /// Reagents whose batches carry this article
    pub reagents: Vec<BackfillReagent>,
    /// The reagent the article is already linked to, when that differs
    pub linked_to: Option<BackfillReagent>,
}

/// A catalog number that was not read as an article
#[derive(Debug, Serialize)]
pub struct BackfillSkip {
    pub cat_number: String,
    pub batch_number: String,
    pub reagent_id: String,
    pub reagent_name: String,
    /// For six digits that fit Merck and Sigma alike
    pub catalog: Option<String>,
}

/// A Merck pack code whose quantity the stored batches agree on
#[derive(Debug, Serialize)]
pub struct BackfillPack {
    pub series: String,
    pub catalog: String,
    pub pack_code: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Serialize, Default)]
pub struct BackfillReport {
    pub applied: bool,
    pub new_links: Vec<BackfillLink>,
    /// Pack quantities learnt for Merck pack codes not known before
    pub new_packs: Vec<BackfillPack>,
    pub already_linked: usize,
    pub conflicts: Vec<BackfillConflict>,
    pub ambiguous: Vec<BackfillSkip>,
    pub unrecognised: Vec<BackfillSkip>,
    pub batches_without_article: i64,
}

#[derive(sqlx::FromRow)]
struct CatRow {
    cat_number: String,
    batch_number: String,
    unit: String,
    pack_size: Option<f64>,
    reagent_id: String,
    reagent_name: String,
}

/// Reads the catalog numbers stored in batches and links each article to its
/// reagent. An article found under several reagents, or already linked to another
/// one, is reported and left alone. With `apply` false nothing is written.
pub async fn backfill(pool: &SqlitePool, user_id: &str, apply: bool) -> ApiResult<BackfillReport> {
    let mut report = BackfillReport::default();
    report.batches_without_article = sqlx::query_scalar(
        "SELECT COUNT(*) FROM batches b JOIN reagents r ON r.id = b.reagent_id \
         WHERE b.deleted_at IS NULL AND r.deleted_at IS NULL AND trim(COALESCE(b.cat_number, '')) = ''",
    )
    .fetch_one(pool)
    .await?;
    let rows: Vec<CatRow> = sqlx::query_as(
        "SELECT trim(b.cat_number) AS cat_number, b.batch_number, b.unit, b.pack_size, b.reagent_id, r.name AS reagent_name \
         FROM batches b JOIN reagents r ON r.id = b.reagent_id \
         WHERE b.deleted_at IS NULL AND r.deleted_at IS NULL AND trim(COALESCE(b.cat_number, '')) != '' \
         ORDER BY r.name, b.cat_number",
    )
    .fetch_all(pool)
    .await?;

    // (series, catalog) -> reagent_id -> (name, batches, units)
    let mut found: BTreeMap<(String, String), BTreeMap<String, (String, i64, BTreeSet<u8>)>> = BTreeMap::new();
    let mut skipped: BTreeSet<(String, String)> = BTreeSet::new();
    // (catalog, pack code) -> (value, unit) -> batches
    let mut packs: BTreeMap<(String, String), BTreeMap<(String, String), i64>> = BTreeMap::new();
    for row in rows {
        match article_key(&row.cat_number, Some(&row.batch_number)) {
            ArticleKey::Known { series, catalog } => {
                if series == Series::Merck {
                    if let (Some(code), Some(size), Some(unit)) =
                        (typed_merck_pack_code(&row.cat_number), row.pack_size, canonical_unit(&row.unit))
                    {
                        if size > 0.0 {
                            *packs
                                .entry((catalog.clone(), code))
                                .or_default()
                                .entry((fmt_qty(size), unit.to_string()))
                                .or_default() += 1;
                        }
                    }
                }
                let e = found
                    .entry((series.as_str().to_string(), catalog))
                    .or_default()
                    .entry(row.reagent_id.clone())
                    .or_insert_with(|| (row.reagent_name.clone(), 0, BTreeSet::new()));
                e.1 += 1;
                if let Some((kind, _)) = canonical_unit(&row.unit).and_then(unit_scale) {
                    e.2.insert(kind);
                }
            }
            key => {
                // one line per catalog number and reagent
                if !skipped.insert((row.cat_number.to_uppercase(), row.reagent_id.clone())) {
                    continue;
                }
                let skip = BackfillSkip {
                    cat_number: row.cat_number,
                    batch_number: row.batch_number,
                    reagent_id: row.reagent_id,
                    reagent_name: row.reagent_name,
                    catalog: None,
                };
                match key {
                    ArticleKey::Ambiguous { catalog } => report.ambiguous.push(BackfillSkip { catalog: Some(catalog), ..skip }),
                    _ => report.unrecognised.push(skip),
                }
            }
        }
    }

    let linked: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT sp.series, sp.catalog, sp.reagent_id, COALESCE(r.name, sp.reagent_id) \
         FROM supplier_products sp LEFT JOIN reagents r ON r.id = sp.reagent_id",
    )
    .fetch_all(pool)
    .await?;
    let linked: BTreeMap<(String, String), (String, String)> =
        linked.into_iter().map(|(s, c, id, name)| ((s, c), (id, name))).collect();

    for ((series, catalog), reagents) in found {
        let list: Vec<BackfillReagent> = reagents
            .iter()
            .map(|(id, (name, n, _))| BackfillReagent { reagent_id: id.clone(), reagent_name: name.clone(), batches: *n })
            .collect();
        let current = linked.get(&(series.clone(), catalog.clone()));
        if list.len() > 1 {
            report.conflicts.push(BackfillConflict {
                series,
                catalog,
                reagents: list,
                linked_to: current.map(|(id, name)| BackfillReagent { reagent_id: id.clone(), reagent_name: name.clone(), batches: 0 }),
            });
            continue;
        }
        let only = list.into_iter().next().expect("one reagent");
        match current {
            Some((id, _)) if *id == only.reagent_id => report.already_linked += 1,
            Some((id, name)) => report.conflicts.push(BackfillConflict {
                series,
                catalog,
                reagents: vec![only],
                linked_to: Some(BackfillReagent { reagent_id: id.clone(), reagent_name: name.clone(), batches: 0 }),
            }),
            None => {
                let kinds = &reagents[&only.reagent_id].2;
                let pack_unit = if series != Series::Merck.as_str() || kinds.len() != 1 {
                    None
                } else {
                    match kinds.iter().next() {
                        Some(0) => Some("g".to_string()),
                        Some(1) => Some("mL".to_string()),
                        _ => None,
                    }
                };
                report.new_links.push(BackfillLink {
                    series,
                    catalog,
                    reagent_id: only.reagent_id,
                    reagent_name: only.reagent_name,
                    batches: only.batches,
                    pack_unit,
                });
            }
        }
    }

    // A pack code is learnt when its batches agree on one quantity (or most do)
    let known_packs: BTreeSet<(String, String)> =
        sqlx::query_as::<_, (String, String)>("SELECT catalog, pack_code FROM supplier_packs WHERE series = 'merck'")
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect();
    for ((catalog, code), sizes) in packs {
        if known_packs.contains(&(catalog.clone(), code.clone())) {
            continue;
        }
        let mut by_count: Vec<((String, String), i64)> = sizes.into_iter().collect();
        by_count.sort_by(|a, b| b.1.cmp(&a.1));
        if by_count.len() > 1 && by_count[0].1 == by_count[1].1 {
            continue;
        }
        let ((value, unit), _) = by_count.swap_remove(0);
        report.new_packs.push(BackfillPack {
            series: Series::Merck.as_str().to_string(),
            catalog,
            pack_code: code,
            value: value.parse().unwrap_or(0.0),
            unit,
        });
    }

    if apply && (!report.new_links.is_empty() || !report.new_packs.is_empty()) {
        let now = Utc::now();
        let mut tx = pool.begin().await?;
        for p in &report.new_packs {
            sqlx::query(
                "INSERT INTO supplier_packs (series, catalog, pack_code, value, unit, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(series, catalog, pack_code) DO NOTHING",
            )
            .bind(&p.series)
            .bind(&p.catalog)
            .bind(&p.pack_code)
            .bind(p.value)
            .bind(&p.unit)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
        for l in &report.new_links {
            sqlx::query(
                "INSERT INTO supplier_products (id, series, catalog, reagent_id, pack_unit, created_by, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(series, catalog) DO NOTHING",
            )
            .bind(Uuid::new_v4().to_string())
            .bind(&l.series)
            .bind(&l.catalog)
            .bind(&l.reagent_id)
            .bind(l.pack_unit.as_deref())
            .bind(user_id)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
    }
    report.applied = apply;
    Ok(report)
}

// ==================== HTTP ====================

pub async fn receive_handler(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<ReceiveRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, BatchAction::Create, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;

    let result = receive(&app_state.db_pool, &claims.sub, &body).await?;

    let reagent_name: String = sqlx::query_scalar("SELECT name FROM reagents WHERE id = ?")
        .bind(&body.reagent_id)
        .fetch_optional(&app_state.db_pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| body.reagent_id.clone());
    let mut cs = ChangeSet::new();
    cs.created("reagent", &reagent_name);
    cs.created("batch_number", body.batch_number.trim());
    cs.created("bottles", &format!("{} x {} {}", body.bottles, body.pack_size, body.unit.trim()));
    let action = if result.created_batch { "create" } else { "update" };
    audit::audit_with_changes(
        &app_state.db_pool,
        &claims.sub,
        action,
        "batch",
        &result.batch_id,
        &format!("Received {} bottle(s) of '{}', batch {}", body.bottles, reagent_name, body.batch_number.trim()),
        &cs,
        &http_request,
    )
    .await;

    Ok(HttpResponse::Created().json(ApiResponse::success(result)))
}

#[derive(Debug, Deserialize)]
pub struct LinkProductRequest {
    pub series: String,
    pub catalog: String,
    pub reagent_id: String,
    pub gtin: Option<String>,
    pub pack_unit: Option<String>,
}

/// Links an article to a reagent without receiving anything
pub async fn link_product_handler(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<LinkProductRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, BatchAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;

    let exists: Option<String> = sqlx::query_scalar("SELECT id FROM reagents WHERE id = ? AND deleted_at IS NULL")
        .bind(&body.reagent_id)
        .fetch_optional(&app_state.db_pool)
        .await?;
    if exists.is_none() {
        return Err(ApiError::NotFound("Reagent not found".to_string()));
    }
    let remember = RememberProduct {
        series: body.series.clone(),
        catalog: body.catalog.clone(),
        gtin: body.gtin.clone(),
        pack_unit: body.pack_unit.clone(),
        pack_code: None,
    };
    // Reuse receive()'s checks on the article part
    let probe = ReceiveRequest {
        reagent_id: body.reagent_id.clone(),
        batch_number: "-".to_string(),
        bottles: 1,
        pack_size: 1.0,
        unit: "g".to_string(),
        expiry_date: None,
        cat_number: None,
        supplier: None,
        manufacturer: None,
        pcode: None,
        position_id: None,
        source: None,
        remember: Some(RememberProduct {
            series: remember.series.clone(),
            catalog: remember.catalog.clone(),
            gtin: remember.gtin.clone(),
            pack_unit: remember.pack_unit.clone(),
            pack_code: None,
        }),
        confirm_new_batch: false,
    };
    validate(&probe)?;

    let mut tx = app_state.db_pool.begin().await?;
    remember_product(&mut tx, &remember, &body.reagent_id, &claims.sub, None).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({ "linked": true }))))
}

#[derive(Debug, Deserialize)]
pub struct BatchCheckQuery {
    pub reagent_id: Option<String>,
    pub batch: String,
    /// Article and pack code of the scanned bottle, for `suggested_pack`
    pub series: Option<String>,
    pub catalog: Option<String>,
    pub pack: Option<String>,
}

/// GET /intake/batch-check?reagent_id=&batch= : the same or a similar batch number in Chelate
pub async fn batch_check_handler(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<BatchCheckQuery>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, BatchAction::View, &app_state.db_pool).await?;
    if query.batch.chars().count() > 100 {
        return Err(ApiError::BadRequest("Batch number must be at most 100 characters".to_string()));
    }
    let reagent = query.reagent_id.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let mut conn = app_state.db_pool.acquire().await?;
    let mut check = check_batch(&mut conn, reagent, &query.batch).await?;
    if let (Some(series), Some(catalog), Some(pack)) = (&query.series, &query.catalog, &query.pack) {
        if catalog.chars().count() <= 100 && pack.chars().count() <= 20 {
            check.suggested_pack = pack_suggestion(&mut conn, series, catalog, pack, reagent).await?;
        }
    }
    Ok(HttpResponse::Ok().json(ApiResponse::success(check)))
}

#[derive(Debug, Deserialize)]
pub struct ArticlesQuery {
    /// Comma-separated reagent ids
    pub reagent_ids: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LinkedArticle {
    pub reagent_id: String,
    pub series: String,
    pub catalog: String,
}

/// GET /intake/products?reagent_ids=a,b : articles linked to these reagents, so a
/// person choosing a reagent for a new article sees which products it already holds
pub async fn articles_handler(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<ArticlesQuery>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, BatchAction::View, &app_state.db_pool).await?;
    let ids: Vec<&str> = query.reagent_ids.split(',').map(str::trim).filter(|s| !s.is_empty()).take(50).collect();
    let mut out: BTreeMap<String, Vec<LinkedArticle>> = BTreeMap::new();
    if !ids.is_empty() {
        let sql = format!(
            "SELECT reagent_id, series, catalog FROM supplier_products WHERE reagent_id IN ({}) ORDER BY series, catalog",
            vec!["?"; ids.len()].join(",")
        );
        let mut q = sqlx::query_as::<_, LinkedArticle>(&sql);
        for id in &ids {
            q = q.bind(*id);
        }
        for a in q.fetch_all(&app_state.db_pool).await? {
            out.entry(a.reagent_id.clone()).or_default().push(a);
        }
    }
    Ok(HttpResponse::Ok().json(ApiResponse::success(out)))
}

/// GET /intake/products/backfill : what linking the stored catalog numbers would do
pub async fn backfill_preview_handler(
    app_state: web::Data<Arc<AppState>>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, BatchAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let report = backfill(&app_state.db_pool, &claims.sub, false).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(report)))
}

/// POST /intake/products/backfill : link every article whose reagent is unambiguous
pub async fn backfill_apply_handler(
    app_state: web::Data<Arc<AppState>>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, BatchAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let report = backfill(&app_state.db_pool, &claims.sub, true).await?;

    let mut cs = ChangeSet::new();
    cs.created("articles_linked", &report.new_links.len().to_string());
    cs.created("conflicts_left", &report.conflicts.len().to_string());
    cs.created("pack_sizes_learnt", &report.new_packs.len().to_string());
    audit::audit_with_changes(
        &app_state.db_pool,
        &claims.sub,
        "create",
        "supplier_products",
        "backfill",
        &format!("Linked {} article(s) to reagents from stored catalog numbers", report.new_links.len()),
        &cs,
        &http_request,
    )
    .await;
    Ok(HttpResponse::Ok().json(ApiResponse::success(report)))
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supplier_codes::{parse_supplier_code, GS};

    fn gs(s: &str) -> String {
        s.replace('␝', &GS.to_string())
    }

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        for ddl in [
            "CREATE TABLE reagents (id TEXT PRIMARY KEY, name TEXT, formula TEXT, cas_number TEXT, \
             storage_conditions TEXT, deleted_at TEXT)",
            "CREATE TABLE batches (id TEXT PRIMARY KEY, reagent_id TEXT NOT NULL, lot_number TEXT, \
             batch_number TEXT NOT NULL, cat_number TEXT, quantity REAL NOT NULL, original_quantity REAL NOT NULL, \
             reserved_quantity REAL NOT NULL DEFAULT 0, unit TEXT NOT NULL, pack_size REAL, expiry_date TEXT, \
             supplier TEXT, manufacturer TEXT, received_date TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'available', \
             location TEXT, notes TEXT, created_by TEXT, updated_by TEXT, created_at TEXT, updated_at TEXT, \
             deleted_at TEXT, UNIQUE(reagent_id, batch_number))",
            "CREATE TABLE batch_containers (id TEXT PRIMARY KEY, batch_id TEXT NOT NULL, sequence_number INTEGER NOT NULL, \
             quantity REAL, original_quantity REAL, is_opened INTEGER, status TEXT, notes TEXT, \
             created_at TEXT, updated_at TEXT, UNIQUE(batch_id, sequence_number))",
            "CREATE TABLE batch_placements (id TEXT PRIMARY KEY, container_id TEXT NOT NULL UNIQUE, \
             position_id TEXT NOT NULL, placed_by TEXT, placed_at TEXT, notes TEXT)",
            "CREATE TABLE storage_positions (id TEXT PRIMARY KEY, zone_id TEXT, name TEXT, sort_order INTEGER)",
            "CREATE TABLE storage_zones (id TEXT PRIMARY KEY, room_id TEXT, name TEXT, zone_type TEXT, sort_order INTEGER)",
            "CREATE TABLE rooms (id TEXT PRIMARY KEY, name TEXT, color TEXT)",
            "INSERT INTO reagents VALUES ('r1', 'trans-Cinnamaldehyde', 'C9H8O', '14371-10-9', NULL, NULL)",
            "INSERT INTO reagents VALUES ('r2', 'Acetonitrile', NULL, '75-05-8', NULL, NULL)",
            "INSERT INTO rooms VALUES ('room1', 'Lab 101', NULL)",
            "INSERT INTO storage_zones VALUES ('z1', 'room1', 'Cabinet A', 'cabinet', 1)",
            "INSERT INTO storage_positions VALUES ('p1', 'z1', 'Shelf 1', 1)",
        ] {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }
        for ddl in container_history::MIGRATIONS {
            sqlx::query(ddl).execute(&pool).await.unwrap();
        }
        run_migrations(&pool).await.unwrap();
        pool
    }

    fn request(reagent: &str, batch: &str, bottles: i64) -> ReceiveRequest {
        ReceiveRequest {
            reagent_id: reagent.to_string(),
            batch_number: batch.to_string(),
            bottles,
            pack_size: 2.5,
            unit: "L".to_string(),
            expiry_date: NaiveDate::from_ymd_opt(2028, 3, 31),
            cat_number: Some("105426".to_string()),
            supplier: Some("Merck".to_string()),
            manufacturer: None,
            pcode: None,
            position_id: None,
            source: Some("scan".to_string()),
            remember: None,
            confirm_new_batch: false,
        }
    }

    #[tokio::test]
    async fn receives_a_new_batch_with_bottles_on_a_shelf() {
        let pool = setup().await;
        let mut req = request("r2", "K55640426", 26);
        req.position_id = Some("p1".to_string());
        let res = receive(&pool, "u1", &req).await.unwrap();

        assert!(res.created_batch);
        assert_eq!(res.container_ids.len(), 26);
        assert_eq!(res.placed, 26);
        assert_eq!(res.batch_bottles, 26);

        let (qty, orig, unit, pack): (f64, f64, String, Option<f64>) = sqlx::query_as(
            "SELECT quantity, original_quantity, unit, pack_size FROM batches WHERE id = ?",
        )
        .bind(&res.batch_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((qty, orig, unit.as_str(), pack), (65.0, 65.0, "L", Some(2.5)));

        let seqs: Vec<i64> = sqlx::query_scalar("SELECT sequence_number FROM batch_containers WHERE batch_id = ? ORDER BY 1")
            .bind(&res.batch_id)
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(seqs, (1..=26).collect::<Vec<_>>());

        // every bottle has a journaled placement from a scan
        let events: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM container_events WHERE event_type = 'placed' AND source = 'scan' AND to_position_id = 'p1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(events, 26);
    }

    #[tokio::test]
    async fn more_bottles_join_an_existing_batch() {
        let pool = setup().await;
        let first = receive(&pool, "u1", &request("r2", "K55640426", 3)).await.unwrap();
        let second = receive(&pool, "u1", &request("r2", "K55640426", 2)).await.unwrap();

        assert!(first.created_batch);
        assert!(!second.created_batch);
        assert_eq!(first.batch_id, second.batch_id);
        assert_eq!(second.batch_bottles, 5);

        let (qty, orig): (f64, f64) = sqlx::query_as("SELECT quantity, original_quantity FROM batches WHERE id = ?")
            .bind(&first.batch_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!((qty, orig), (12.5, 12.5));
        let max_seq: i64 = sqlx::query_scalar("SELECT MAX(sequence_number) FROM batch_containers WHERE batch_id = ?")
            .bind(&first.batch_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(max_seq, 5);
    }

    #[tokio::test]
    async fn existing_batch_keeps_its_expiry_and_rejects_another_unit() {
        let pool = setup().await;
        receive(&pool, "u1", &request("r2", "B1", 1)).await.unwrap();

        let mut later = request("r2", "B1", 1);
        later.expiry_date = NaiveDate::from_ymd_opt(2030, 1, 1);
        let res = receive(&pool, "u1", &later).await.unwrap();
        assert_eq!(res.warnings.len(), 1, "{:?}", res.warnings);
        let expiry: String = sqlx::query_scalar("SELECT expiry_date FROM batches WHERE id = ?")
            .bind(&res.batch_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(expiry.starts_with("2028-03-31"), "{expiry}");

        // grams do not go into a batch kept in litres
        let mut other = request("r2", "B1", 1);
        other.unit = "g".to_string();
        assert!(matches!(receive(&pool, "u1", &other).await, Err(ApiError::BadRequest(_))));
    }

    #[tokio::test]
    async fn nothing_is_written_when_the_shelf_is_unknown() {
        let pool = setup().await;
        let mut req = request("r2", "B1", 3);
        req.position_id = Some("nope".to_string());
        assert!(matches!(receive(&pool, "u1", &req).await, Err(ApiError::NotFound(_))));
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM batches").fetch_one(&pool).await.unwrap();
        assert_eq!(n, 0);
        assert!(matches!(receive(&pool, "u1", &request("missing", "B1", 1)).await, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn rejects_bad_numbers() {
        let pool = setup().await;
        for (bottles, pack, batch) in [(0, 1.0, "B"), (501, 1.0, "B"), (1, 0.0, "B"), (1, -1.0, "B"), (1, f64::NAN, "B"), (1, 1.0, "  ")] {
            let mut req = request("r2", batch, bottles);
            req.pack_size = pack;
            assert!(receive(&pool, "u1", &req).await.is_err(), "{bottles} {pack} {batch:?}");
        }
    }

    #[tokio::test]
    async fn a_deleted_batch_with_the_same_number_does_not_block_receiving() {
        let pool = setup().await;
        let first = receive(&pool, "u1", &request("r2", "B1", 1)).await.unwrap();
        sqlx::query("UPDATE batches SET deleted_at = '2026-01-01' WHERE id = ?")
            .bind(&first.batch_id)
            .execute(&pool)
            .await
            .unwrap();
        let again = receive(&pool, "u1", &request("r2", "B1", 1)).await.unwrap();
        assert!(again.created_batch);
        assert_ne!(first.batch_id, again.batch_id);
    }

    #[tokio::test]
    async fn pcode_goes_to_notes() {
        let pool = setup().await;
        let mut req = request("r1", "MKCH5601", 1);
        req.pcode = Some("102136798".to_string());
        let res = receive(&pool, "u1", &req).await.unwrap();
        let notes: Option<String> = sqlx::query_scalar("SELECT notes FROM batches WHERE id = ?")
            .bind(&res.batch_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(notes.as_deref(), Some("Supplier Pcode: 102136798"));
    }

    #[tokio::test]
    async fn an_article_is_learnt_once_and_found_by_gtin_later() {
        let pool = setup().await;
        let dm = parse_supplier_code(&gs("0104065268083710422276␝10K55640426␝2401054262511")).unwrap();

        // First scan: unknown article
        let scan = scan_info(&pool, dm.clone()).await.unwrap();
        assert!(scan.reagent.is_none());
        assert!(scan.suggested_pack.is_none());

        // The person picks the reagent; the article, GTIN and pack unit are remembered
        let mut req = request("r2", "K55640426", 2);
        req.remember = Some(RememberProduct {
            series: "merck".to_string(),
            catalog: "105426".to_string(),
            gtin: dm.gtin.clone(),
            pack_unit: Some("mL".to_string()),
            pack_code: None,
        });
        receive(&pool, "u1", &req).await.unwrap();

        // Next scan of any bottle of the article: reagent known, existing batch seen
        let scan = scan_info(&pool, dm.clone()).await.unwrap();
        assert_eq!(scan.reagent.as_ref().map(|r| r.name.as_str()), Some("Acetonitrile"));
        assert_eq!(scan.batch.as_ref().map(|b| b.bottles), Some(2));
        // 2511 is a special pack: no quantity is guessed from it
        assert_eq!(scan.suggested_pack, None);

        // The EAN-13 on the same bottle finds the article through the GTIN
        let ean = parse_supplier_code("4065268083710").unwrap();
        assert_eq!(ean.gtin, dm.gtin);
        let by_ean = scan_info(&pool, ean).await.unwrap();
        assert_eq!(by_ean.reagent.as_ref().map(|r| r.id.as_str()), Some("r2"));
    }

    #[tokio::test]
    async fn merck_round_pack_codes_become_quantities_only_with_a_known_unit() {
        let pool = setup().await;
        let code = parse_supplier_code(&gs("010402253602860615230831422276␝10A1335066␝2401020661000")).unwrap();
        assert_eq!(suggest_pack(&code, None), None);
        assert_eq!(
            suggest_pack(&code, Some("g")),
            Some(SuggestedPack { value: 1.0, unit: "kg".to_string() })
        );
        assert_eq!(
            suggest_pack(&code, Some("mL")),
            Some(SuggestedPack { value: 1.0, unit: "L".to_string() })
        );
        let small = parse_supplier_code(&gs("0104022536384597422156␝10S7346911␝2408043110100")).unwrap();
        assert_eq!(
            suggest_pack(&small, Some("g")),
            Some(SuggestedPack { value: 100.0, unit: "g".to_string() })
        );
        let special = parse_supplier_code(&gs("0104065268083710422276␝10K55640426␝2401054262511")).unwrap();
        assert_eq!(suggest_pack(&special, Some("mL")), None);

        // Sigma packs state their unit
        let sigma = parse_supplier_code("W228605-1KG-K;MKCH5601;102136798").unwrap();
        assert_eq!(suggest_pack(&sigma, None), Some(SuggestedPack { value: 1.0, unit: "kg".to_string() }));
        let _ = pool;
    }

    #[tokio::test]
    async fn expired_stock_is_flagged() {
        let pool = setup().await;
        let old = parse_supplier_code(&gs("010402253602860615230831422276␝10A1335066␝2401020661000")).unwrap();
        assert!(scan_info(&pool, old).await.unwrap().expired);
        let fresh = parse_supplier_code("(01)04022536028606(17)990101(10)A1(240)1020661000").unwrap();
        assert!(!scan_info(&pool, fresh).await.unwrap().expired);
    }

    #[tokio::test]
    async fn a_series_keeps_equal_digits_apart() {
        let pool = setup().await;
        // Merck 452882 and Sigma 452882 are different articles
        let mut req = request("r1", "X1", 1);
        req.remember = Some(RememberProduct {
            series: "sigma".to_string(),
            catalog: "452882".to_string(),
            gtin: None,
            pack_unit: None,
            pack_code: None,
        });
        receive(&pool, "u1", &req).await.unwrap();
        let sigma = parse_supplier_code(&gs("010406183555518510STBL0017␝240452882-500G␝422840")).unwrap();
        assert!(scan_info(&pool, sigma).await.unwrap().reagent.is_some());
        let merck = parse_supplier_code("4528821000").unwrap();
        assert!(scan_info(&pool, merck).await.unwrap().reagent.is_none());
    }

    /// A batch as the import or older versions left it: any unit spelling, no bottles
    async fn old_batch(pool: &SqlitePool, id: &str, reagent: &str, number: &str, unit: &str, cat: Option<&str>) {
        sqlx::query(
            "INSERT INTO batches (id, reagent_id, batch_number, cat_number, quantity, original_quantity, unit, \
             pack_size, received_date) VALUES (?, ?, ?, ?, 5000, 5000, ?, 2500, '2026-01-01T00:00:00Z')",
        )
        .bind(id)
        .bind(reagent)
        .bind(number)
        .bind(cat)
        .bind(unit)
        .execute(pool)
        .await
        .unwrap();
    }

    #[test]
    fn units_in_any_spelling() {
        for (u, want) in [("ML", "mL"), ("ml", "mL"), ("Ml", "mL"), ("G", "g"), ("KG", "kg"), ("l", "L"), ("PIECE", "pcs"), (" pcs ", "pcs")] {
            assert_eq!(canonical_unit(u), Some(want), "{u}");
        }
        assert_eq!(canonical_unit("bottle"), None);
        assert_eq!(unit_factor("ML", "mL"), Some(1.0));
        assert_eq!(unit_factor("L", "ML"), Some(1000.0));
        assert_eq!(unit_factor("g", "KG"), Some(0.001));
        assert_eq!(unit_factor("g", "mL"), None);
        assert_eq!(unit_factor("PIECE", "pcs"), Some(1.0));
        assert_eq!(unit_factor("bottle", "bottle"), Some(1.0));
    }

    #[tokio::test]
    async fn an_old_batch_in_upper_case_units_takes_more_bottles() {
        let pool = setup().await;
        old_batch(&pool, "b1", "r2", "K55353850", "ML", None).await;

        // same unit, other spelling: no conversion, no warning
        let mut req = request("r2", "K55353850", 2);
        req.unit = "mL".to_string();
        req.pack_size = 2500.0;
        let res = receive(&pool, "u1", &req).await.unwrap();
        assert!(!res.created_batch);
        assert!(res.warnings.is_empty(), "{:?}", res.warnings);

        // litres into a batch kept in millilitres
        let res = receive(&pool, "u1", &request("r2", "K55353850", 2)).await.unwrap();
        assert_eq!(res.warnings.len(), 1, "{:?}", res.warnings);
        assert!(res.warnings[0].contains("recorded as 2500 mL"), "{}", res.warnings[0]);

        let (qty, unit): (f64, String) = sqlx::query_as("SELECT quantity, unit FROM batches WHERE id = 'b1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!((qty, unit.as_str()), (5000.0 + 5000.0 + 5000.0, "ML"));
        let bottle_qty: Vec<f64> = sqlx::query_scalar("SELECT quantity FROM batch_containers WHERE batch_id = 'b1'")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(bottle_qty, vec![2500.0; 4]);
    }

    #[tokio::test]
    async fn new_batches_get_one_unit_spelling() {
        let pool = setup().await;
        let mut req = request("r2", "Z0839541", 1);
        req.unit = "ml".to_string();
        let res = receive(&pool, "u1", &req).await.unwrap();
        let unit: String = sqlx::query_scalar("SELECT unit FROM batches WHERE id = ?")
            .bind(&res.batch_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(unit, "mL");
    }

    #[test]
    fn similar_batch_numbers() {
        // cases from the lab's batch list
        for (typed, stored, reason) in [
            ("I1322907", "l1322907", Some("lookalike")),
            ("K51185817", "K5118581", Some("cut")),
            ("K5118581", "K51185817", Some("cut")),
            ("SHBP6353", "SHBP63535G", Some("pack")),
            ("BCCH8042", "BCCH80420500", Some("pack")),
            ("MKCT9494", "MKCT9494-2.5", Some("pack")),
            ("mkcc5117", "MKCC5117", Some("case")),
            ("K47917114", "K47917114", Some("same")),
            // neighbouring batches are different batches
            ("MKCC5117", "MKCC5118", None),
            ("I1303207", "I1003207", None),
            ("H0680", "H0608", None),
            // too short to call a cut-off
            ("A123", "A1234", None),
        ] {
            assert_eq!(batch_similarity(typed, stored), reason, "{typed} vs {stored}");
        }
    }

    #[tokio::test]
    async fn a_similar_batch_must_be_chosen_or_confirmed() {
        let pool = setup().await;
        old_batch(&pool, "b1", "r2", "l1322907", "L", None).await;

        let check = check_batch(&mut pool.acquire().await.unwrap(), Some("r2"), "I1322907").await.unwrap();
        assert!(check.exact.is_none());
        assert_eq!(check.similar.len(), 1);
        assert_eq!(check.similar[0].reason, "lookalike");

        let req = request("r2", "I1322907", 1);
        match receive(&pool, "u1", &req).await {
            Err(ApiError::BadRequest(m)) => assert!(m.contains("l1322907"), "{m}"),
            other => panic!("{other:?}"),
        }
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM batches").fetch_one(&pool).await.unwrap();
        assert_eq!(n, 1);

        // choosing the stored number adds to it
        let res = receive(&pool, "u1", &request("r2", "l1322907", 1)).await.unwrap();
        assert!(!res.created_batch);

        // or the person says it is new
        let mut new = request("r2", "I1322907", 1);
        new.confirm_new_batch = true;
        assert!(receive(&pool, "u1", &new).await.unwrap().created_batch);
    }

    #[tokio::test]
    async fn the_same_batch_under_another_reagent_is_pointed_out() {
        let pool = setup().await;
        old_batch(&pool, "b1", "r1", "K5484222", "mL", None).await;

        // No reagent chosen yet: the batch tells where the article probably belongs
        let mut conn = pool.acquire().await.unwrap();
        let check = check_batch(&mut conn, None, "K5484222").await.unwrap();
        assert_eq!(check.elsewhere.len(), 1);
        assert_eq!(check.elsewhere[0].reagent_name, "trans-Cinnamaldehyde");
        assert_eq!(check.elsewhere[0].reason, "same");
        drop(conn);

        // Receiving it under another reagent needs a confirmation
        assert!(matches!(receive(&pool, "u1", &request("r2", "K5484222", 1)).await, Err(ApiError::BadRequest(_))));
        let mut req = request("r2", "K5484222", 1);
        req.confirm_new_batch = true;
        assert!(receive(&pool, "u1", &req).await.is_ok());

        // A scan shows it too
        let dm = parse_supplier_code(&gs("0104065268083710422276␝10K5484222␝2401054232511")).unwrap();
        let scan = scan_info(&pool, dm).await.unwrap();
        assert_eq!(scan.batch_elsewhere.len(), 2);
    }

    #[tokio::test]
    async fn backfill_links_articles_from_stored_catalog_numbers() {
        let pool = setup().await;
        sqlx::query("INSERT INTO reagents VALUES ('r3', 'Ammonia solution 25%', NULL, '1336-21-6', NULL, NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO reagents VALUES ('r4', 'Ammonia solution 30%', NULL, '1336-21-6', NULL, NULL)")
            .execute(&pool)
            .await
            .unwrap();
        // acetonitrile: two Merck forms of one article and a Sigma one
        old_batch(&pool, "b1", "r2", "I1278029", "ML", Some("1000292500")).await;
        old_batch(&pool, "b2", "r2", "I1264830", "L", Some("100029-2.5L")).await;
        old_batch(&pool, "b3", "r1", "MKCH5601", "g", Some("W228605-1KG-K")).await;
        // one article under two reagents: left for a person
        old_batch(&pool, "b4", "r3", "Z0891532", "mL", Some("105423-2511L")).await;
        old_batch(&pool, "b5", "r4", "K5484222", "mL", Some("105423-2511L")).await;
        // not readable as an article, and six digits with an unhelpful batch
        old_batch(&pool, "b6", "r1", "17I204006", "L", Some("21390.293")).await;
        old_batch(&pool, "b7", "r1", "1004123AZ", "g", Some("185361-500G")).await;
        old_batch(&pool, "b8", "r1", "X1", "g", None).await;

        let preview = backfill(&pool, "u1", false).await.unwrap();
        assert!(!preview.applied);
        let links: Vec<(&str, &str, &str, Option<&str>)> = preview
            .new_links
            .iter()
            .map(|l| (l.series.as_str(), l.catalog.as_str(), l.reagent_id.as_str(), l.pack_unit.as_deref()))
            .collect();
        assert_eq!(links, vec![("merck", "100029", "r2", Some("mL")), ("sigma", "W228605", "r1", None)]);
        assert_eq!(preview.new_links[0].batches, 2);
        assert_eq!(preview.conflicts.len(), 1);
        assert_eq!(preview.conflicts[0].catalog, "105423");
        assert_eq!(preview.conflicts[0].reagents.len(), 2);
        assert_eq!(preview.unrecognised.len(), 1);
        assert_eq!(preview.ambiguous.len(), 1);
        assert_eq!(preview.ambiguous[0].catalog.as_deref(), Some("185361"));
        assert_eq!(preview.batches_without_article, 1);
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM supplier_products").fetch_one(&pool).await.unwrap();
        assert_eq!(n, 0, "a preview writes nothing");

        let applied = backfill(&pool, "u1", true).await.unwrap();
        assert_eq!(applied.new_links.len(), 2);

        // a bottle of that article is now known at the first scan
        let dm = parse_supplier_code(&gs("0104065268083710422276␝10I1999999␝2401000292500")).unwrap();
        let scan = scan_info(&pool, dm).await.unwrap();
        assert_eq!(scan.reagent.as_ref().map(|r| r.id.as_str()), Some("r2"));

        // running it again changes nothing
        let again = backfill(&pool, "u1", true).await.unwrap();
        assert!(again.new_links.is_empty());
        assert_eq!(again.already_linked, 2);

        // an article already linked elsewhere is reported, not moved
        sqlx::query("UPDATE supplier_products SET reagent_id = 'r1' WHERE catalog = '100029'")
            .execute(&pool)
            .await
            .unwrap();
        let moved = backfill(&pool, "u1", true).await.unwrap();
        assert_eq!(moved.conflicts.len(), 2);
        let c = moved.conflicts.iter().find(|c| c.catalog == "100029").unwrap();
        assert_eq!(c.linked_to.as_ref().map(|r| r.reagent_id.as_str()), Some("r1"));
        let still: String = sqlx::query_scalar("SELECT reagent_id FROM supplier_products WHERE catalog = '100029'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(still, "r1");
    }

    #[tokio::test]
    async fn a_special_merck_pack_is_remembered_after_the_first_receipt() {
        let pool = setup().await;
        let dm = parse_supplier_code(&gs("0104065268083710422276␝10K55640426␝2401054262511")).unwrap();
        assert_eq!(scan_info(&pool, dm.clone()).await.unwrap().suggested_pack, None);

        let mut req = request("r2", "K55640426", 1);
        req.remember = Some(RememberProduct {
            series: "merck".to_string(),
            catalog: "105426".to_string(),
            gtin: None,
            pack_unit: Some("mL".to_string()),
            pack_code: Some("2511".to_string()),
        });
        receive(&pool, "u1", &req).await.unwrap();

        // next batch of the same article and pack: 2.5 L offered
        let next = parse_supplier_code(&gs("0104065268083710422276␝10K60000001␝2401054262511")).unwrap();
        assert_eq!(
            scan_info(&pool, next).await.unwrap().suggested_pack,
            Some(SuggestedPack { value: 2.5, unit: "L".to_string() })
        );
        // a bad pack code is refused
        let mut bad = request("r2", "K1", 1);
        bad.remember = Some(RememberProduct {
            series: "merck".to_string(),
            catalog: "105426".to_string(),
            gtin: None,
            pack_unit: None,
            pack_code: Some("25x".to_string()),
        });
        assert!(receive(&pool, "u1", &bad).await.is_err());
    }

    #[tokio::test]
    async fn a_new_merck_article_takes_its_unit_from_the_chosen_reagent() {
        let pool = setup().await;
        old_batch(&pool, "b1", "r2", "K1", "L", None).await;
        old_batch(&pool, "b2", "r1", "K2", "G", None).await;
        let mut conn = pool.acquire().await.unwrap();
        // not linked yet: nothing without a reagent
        assert_eq!(pack_suggestion(&mut conn, "merck", "109999", "0500", None).await.unwrap(), None);
        assert_eq!(
            pack_suggestion(&mut conn, "merck", "109999", "0500", Some("r2")).await.unwrap(),
            Some(SuggestedPack { value: 500.0, unit: "mL".to_string() })
        );
        assert_eq!(
            pack_suggestion(&mut conn, "merck", "109999", "1000", Some("r1")).await.unwrap(),
            Some(SuggestedPack { value: 1.0, unit: "kg".to_string() })
        );
        // a special code needs a person once
        assert_eq!(pack_suggestion(&mut conn, "merck", "109999", "2511", Some("r2")).await.unwrap(), None);
        // Sigma sizes come from the code itself, not from here
        assert_eq!(pack_suggestion(&mut conn, "sigma", "34850", "0500", Some("r2")).await.unwrap(), None);
    }

    #[tokio::test]
    async fn backfill_learns_pack_quantities_from_stored_batches() {
        let pool = setup().await;
        old_batch(&pool, "b1", "r2", "Z0891532", "ml", Some("105432-2511L")).await;
        old_batch(&pool, "b2", "r2", "Z0891533", "ML", Some("1054322511")).await;
        // round codes and Sigma sizes are not stored
        old_batch(&pool, "b3", "r1", "MKCH5601", "g", Some("W228605-1KG-K")).await;
        let report = backfill(&pool, "u1", true).await.unwrap();
        let packs: Vec<(&str, &str, f64, &str)> = report
            .new_packs
            .iter()
            .map(|p| (p.catalog.as_str(), p.pack_code.as_str(), p.value, p.unit.as_str()))
            .collect();
        assert_eq!(packs, vec![("105432", "2511", 2500.0, "mL")]);

        let dm = parse_supplier_code(&gs("0104065268083710422276␝10Z0999999␝2401054322511")).unwrap();
        assert_eq!(
            scan_info(&pool, dm).await.unwrap().suggested_pack,
            Some(SuggestedPack { value: 2.5, unit: "L".to_string() })
        );
        assert!(backfill(&pool, "u1", true).await.unwrap().new_packs.is_empty());
    }
}
