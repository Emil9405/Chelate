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

use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

use crate::audit::{self, ChangeSet};
use crate::auth;
use crate::auth_handlers::{self, BatchAction};
use crate::container_history;
use crate::error::{ApiError, ApiResult};
use crate::handlers::ApiResponse;
use crate::supplier_codes::{Series, SupplierCode};
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
    pub suggested_pack: Option<SuggestedPack>,
    pub expired: bool,
}

/// What the code alone says about the pack, or what a stored base unit adds.
/// A Merck pack code is only turned into a quantity for round codes: `2511` is a
/// special pack, not 2511 g, so it is left for a person to enter.
pub fn suggest_pack(code: &SupplierCode, base_unit: Option<&str>) -> Option<SuggestedPack> {
    let pack = code.pack.as_ref()?;
    if let (Some(value), Some(unit)) = (pack.value, pack.unit.as_deref()) {
        return Some(SuggestedPack { value, unit: unit.to_string() });
    }
    if code.series != Series::Merck {
        return None;
    }
    let n = pack.value?;
    let unit = base_unit?;
    if n <= 0.0 || n.fract() != 0.0 || (n as i64) % 10 != 0 {
        return None;
    }
    Some(if n >= 1000.0 {
        SuggestedPack {
            value: n / 1000.0,
            unit: if unit == "mL" { "L" } else { "kg" }.to_string(),
        }
    } else {
        SuggestedPack { value: n, unit: unit.to_string() }
    })
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

    let batch = match (&reagent, code.batch.as_deref()) {
        (Some(r), Some(number)) => {
            sqlx::query_as::<_, ExistingBatch>(
                "SELECT b.id AS batch_id, b.batch_number, b.pack_size, b.unit, b.quantity, b.expiry_date, \
                   (SELECT COUNT(*) FROM batch_containers bc \
                     WHERE bc.batch_id = b.id AND bc.status != 'disposed') AS bottles \
                 FROM batches b \
                 WHERE b.reagent_id = ? AND b.batch_number = ? AND b.deleted_at IS NULL",
            )
            .bind(&r.id)
            .bind(number)
            .fetch_optional(pool)
            .await?
        }
        _ => None,
    };

    let base_unit = reagent.as_ref().and_then(|r| r.pack_unit.as_deref());
    let suggested_pack = suggest_pack(&code, base_unit);
    let expired = code.expiry.map(|d| d < Utc::now().date_naive()).unwrap_or(false);
    Ok(SupplierScan { code, reagent, batch, suggested_pack, expired })
}

// ==================== RECEIVE ====================

#[derive(Debug, Deserialize)]
pub struct RememberProduct {
    pub series: String,
    pub catalog: String,
    pub gtin: Option<String>,
    /// `g` or `mL`: what a Merck pack code counts
    pub pack_unit: Option<String>,
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
    }
    Ok(())
}

pub async fn receive(pool: &SqlitePool, user_id: &str, req: &ReceiveRequest) -> ApiResult<ReceiveResult> {
    validate(req)?;
    let batch_number = req.batch_number.trim().to_string();
    let unit = req.unit.trim().to_string();
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

    let added = req.pack_size * req.bottles as f64;
    let existing: Option<(String, String, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, unit, expiry_date FROM batches \
         WHERE reagent_id = ? AND batch_number = ? AND deleted_at IS NULL",
    )
    .bind(&req.reagent_id)
    .bind(&batch_number)
    .fetch_optional(&mut *tx)
    .await?;

    let (batch_id, created_batch) = match existing {
        Some((id, existing_unit, existing_expiry)) => {
            if existing_unit != unit {
                return Err(ApiError::BadRequest(format!(
                    "This batch is recorded in {}, not {}. Use the same unit or change the batch first.",
                    existing_unit, unit
                )));
            }
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
            .bind(req.pack_size)
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
            .bind(req.pack_size)
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
        .bind(req.pack_size)
        .bind(req.pack_size)
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
        remember_product(&mut tx, r, &req.reagent_id, user_id).await?;
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
    Ok(())
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
        }),
    };
    validate(&probe)?;

    let mut tx = app_state.db_pool.begin().await?;
    remember_product(&mut tx, &remember, &body.reagent_id, &claims.sub).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({ "linked": true }))))
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

        let mut other = request("r2", "B1", 1);
        other.unit = "mL".to_string();
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
        });
        receive(&pool, "u1", &req).await.unwrap();
        let sigma = parse_supplier_code(&gs("010406183555518510STBL0017␝240452882-500G␝422840")).unwrap();
        assert!(scan_info(&pool, sigma).await.unwrap().reagent.is_some());
        let merck = parse_supplier_code("4528821000").unwrap();
        assert!(scan_info(&pool, merck).await.unwrap().reagent.is_none());
    }
}
