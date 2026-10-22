// src/inventory_handlers.rs
//! Inventory by scanning: walk a room, cabinet or shelf, scan every bottle,
//! and compare what is physically there with what the database says.
//!
//! A session covers one scope (room / zone / position). Each scan is made
//! "on" a shelf of that scope and is classified:
//!   found      — recorded on this shelf
//!   misplaced  — recorded on another shelf (offer: move here)
//!   unplaced   — recorded nowhere (offer: place here)
//!   inactive   — recorded as empty or disposed
//!   unknown    — not a Chelate label / no such bottle
//! Closing the session lists every bottle recorded in the scope that was not
//! scanned as "missing". Results are kept as a dated record; every sighting
//! and every correction is also written to the bottle history.

use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{SqliteConnection, SqlitePool};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::get_current_user;
use crate::container_history::{self as history, event, source, NewEvent};
use crate::error::{ApiError, ApiResult};
use crate::handlers::ApiResponse;
use crate::label_handlers::{self, ContainerLabel, ContainerRow, ScanQuery};
use crate::AppState;

// ==================== SCOPE ====================

fn scope_filter(scope_type: &str) -> ApiResult<&'static str> {
    // Aliases follow CONTAINER_ROW_SELECT and the positions query
    match scope_type {
        "room" => Ok("sz.room_id = ?"),
        "zone" => Ok("sp.zone_id = ?"),
        "position" => Ok("sp.id = ?"),
        _ => Err(ApiError::bad_request("scope_type must be room, zone or position")),
    }
}

async fn scope_name(pool: &SqlitePool, scope_type: &str, scope_id: &str) -> ApiResult<String> {
    let sql = match scope_type {
        "room" => "SELECT name FROM rooms WHERE id = ?".to_string(),
        "zone" => "SELECT rm.name || ' / ' || sz.name FROM storage_zones sz \
                   JOIN rooms rm ON rm.id = sz.room_id WHERE sz.id = ?".to_string(),
        "position" => format!("SELECT {}", history::POSITION_PATH_SQL),
        _ => return Err(ApiError::bad_request("scope_type must be room, zone or position")),
    };
    let name: Option<Option<String>> = sqlx::query_scalar(&sql).bind(scope_id).fetch_optional(pool).await?;
    name.flatten().ok_or_else(|| ApiError::not_found("Storage location"))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ScopePosition {
    pub id: String,
    pub name: String,
    pub zone_id: String,
    pub zone_name: String,
    pub room_name: String,
}

async fn scope_positions(pool: &SqlitePool, scope_type: &str, scope_id: &str) -> ApiResult<Vec<ScopePosition>> {
    let sql = format!(
        "SELECT sp.id, sp.name, sz.id AS zone_id, sz.name AS zone_name, rm.name AS room_name \
         FROM storage_positions sp \
         JOIN storage_zones sz ON sz.id = sp.zone_id \
         JOIN rooms rm ON rm.id = sz.room_id \
         WHERE {} \
         ORDER BY rm.name, sz.sort_order, sz.name, sp.sort_order, sp.name",
        scope_filter(scope_type)?
    );
    Ok(sqlx::query_as(&sql).bind(scope_id).fetch_all(pool).await?)
}

/// Bottles the database places inside the scope (active ones only)
async fn expected_in_scope(pool: &SqlitePool, scope_type: &str, scope_id: &str) -> ApiResult<Vec<ContainerLabel>> {
    let sql = format!(
        "{} WHERE bp.position_id IS NOT NULL AND {} AND bc.status NOT IN ('empty', 'disposed') {}",
        label_handlers::CONTAINER_ROW_SELECT,
        scope_filter(scope_type)?,
        label_handlers::LABEL_ORDER,
    );
    let rows: Vec<ContainerRow> = sqlx::query_as(&sql).bind(scope_id).fetch_all(pool).await?;
    Ok(rows.into_iter().map(ContainerLabel::from).collect())
}

async fn position_path(conn: &mut SqliteConnection, position_id: Option<&str>) -> ApiResult<Option<String>> {
    match position_id {
        None => Ok(None),
        Some(id) => Ok(sqlx::query_scalar::<_, Option<String>>(&format!("SELECT {}", history::POSITION_PATH_SQL))
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?
            .flatten()),
    }
}

// ==================== ROWS ====================

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SessionRow {
    pub id: String,
    pub scope_type: String,
    pub scope_id: String,
    pub scope_name: String,
    pub status: String,
    pub notes: Option<String>,
    pub started_by: Option<String>,
    pub started_by_name: Option<String>,
    pub started_at: DateTime<Utc>,
    pub closed_by: Option<String>,
    pub closed_by_name: Option<String>,
    pub closed_at: Option<DateTime<Utc>>,
    pub expected_count: Option<i64>,
    pub found_count: Option<i64>,
    pub misplaced_count: Option<i64>,
    pub unplaced_count: Option<i64>,
    pub inactive_count: Option<i64>,
    pub unknown_count: Option<i64>,
    pub missing_count: Option<i64>,
    pub scanned_count: i64,
}

const SESSION_SELECT: &str = "SELECT s.id, s.scope_type, s.scope_id, s.scope_name, s.status, s.notes, \
        s.started_by, u1.username AS started_by_name, s.started_at, \
        s.closed_by, u2.username AS closed_by_name, s.closed_at, \
        s.expected_count, s.found_count, s.misplaced_count, s.unplaced_count, \
        s.inactive_count, s.unknown_count, s.missing_count, \
        (SELECT COUNT(*) FROM inventory_items i WHERE i.session_id = s.id AND i.result != 'missing') AS scanned_count \
     FROM inventory_sessions s \
     LEFT JOIN users u1 ON u1.id = s.started_by \
     LEFT JOIN users u2 ON u2.id = s.closed_by";

async fn get_session(pool: &SqlitePool, id: &str) -> ApiResult<SessionRow> {
    let sql = format!("{} WHERE s.id = ?", SESSION_SELECT);
    sqlx::query_as(&sql).bind(id).fetch_optional(pool).await?
        .ok_or_else(|| ApiError::not_found("Inventory session"))
}

async fn get_open_session(pool: &SqlitePool, id: &str) -> ApiResult<SessionRow> {
    let s = get_session(pool, id).await?;
    if s.status != "open" {
        return Err(ApiError::bad_request("This inventory is already finished"));
    }
    Ok(s)
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ItemRow {
    pub id: String,
    pub session_id: String,
    pub container_id: Option<String>,
    pub raw_code: Option<String>,
    pub position_id: Option<String>,
    pub location: Option<String>,
    pub result: String,
    pub expected_position_id: Option<String>,
    pub expected_location: Option<String>,
    pub resolution: Option<String>,
    pub scanned_by: Option<String>,
    pub scanned_by_name: Option<String>,
    pub scanned_at: DateTime<Utc>,
    pub reagent_name: Option<String>,
    pub batch_number: Option<String>,
    pub sequence_number: Option<i64>,
    pub quantity: Option<f64>,
    pub unit: Option<String>,
}

const ITEM_SELECT: &str = "SELECT i.id, i.session_id, i.container_id, i.raw_code, i.position_id, i.location, \
        i.result, i.expected_position_id, i.expected_location, i.resolution, i.scanned_by, \
        u.username AS scanned_by_name, i.scanned_at, \
        r.name AS reagent_name, b.batch_number, bc.sequence_number, bc.quantity, b.unit \
     FROM inventory_items i \
     LEFT JOIN users u ON u.id = i.scanned_by \
     LEFT JOIN batch_containers bc ON bc.id = i.container_id \
     LEFT JOIN batches b ON b.id = bc.batch_id \
     LEFT JOIN reagents r ON r.id = b.reagent_id";

async fn get_item(pool: &SqlitePool, session_id: &str, item_id: &str) -> ApiResult<ItemRow> {
    let sql = format!("{} WHERE i.session_id = ? AND i.id = ?", ITEM_SELECT);
    sqlx::query_as(&sql).bind(session_id).bind(item_id).fetch_optional(pool).await?
        .ok_or_else(|| ApiError::not_found("Inventory item"))
}

async fn session_items(pool: &SqlitePool, session_id: &str) -> ApiResult<Vec<ItemRow>> {
    let sql = format!("{} WHERE i.session_id = ? ORDER BY i.scanned_at DESC, i.rowid DESC", ITEM_SELECT);
    Ok(sqlx::query_as(&sql).bind(session_id).fetch_all(pool).await?)
}

#[derive(Debug, Serialize, Default)]
pub struct Counts {
    pub expected: i64,
    /// Expected bottles not scanned yet (missing once the session is closed)
    pub remaining: i64,
    pub found: i64,
    pub misplaced: i64,
    pub unplaced: i64,
    pub inactive: i64,
    pub unknown: i64,
    pub missing: i64,
}

async fn live_counts(pool: &SqlitePool, s: &SessionRow) -> ApiResult<Counts> {
    if s.status != "open" {
        return Ok(Counts {
            expected: s.expected_count.unwrap_or(0),
            remaining: 0,
            found: s.found_count.unwrap_or(0),
            misplaced: s.misplaced_count.unwrap_or(0),
            unplaced: s.unplaced_count.unwrap_or(0),
            inactive: s.inactive_count.unwrap_or(0),
            unknown: s.unknown_count.unwrap_or(0),
            missing: s.missing_count.unwrap_or(0),
        });
    }
    let expected = expected_in_scope(pool, &s.scope_type, &s.scope_id).await?;
    let scanned: HashSet<String> = sqlx::query_scalar::<_, String>(
        "SELECT container_id FROM inventory_items WHERE session_id = ? AND container_id IS NOT NULL",
    )
    .bind(&s.id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect();
    let by_result: Vec<(String, i64)> = sqlx::query_as(
        "SELECT result, COUNT(*) FROM inventory_items WHERE session_id = ? GROUP BY result",
    )
    .bind(&s.id)
    .fetch_all(pool)
    .await?;
    let get = |k: &str| by_result.iter().find(|(r, _)| r == k).map(|(_, n)| *n).unwrap_or(0);
    Ok(Counts {
        expected: expected.len() as i64,
        remaining: expected.iter().filter(|e| !scanned.contains(&e.row.container_id)).count() as i64,
        found: get("found"),
        misplaced: get("misplaced"),
        unplaced: get("unplaced"),
        inactive: get("inactive"),
        unknown: get("unknown"),
        missing: get("missing"),
    })
}

// ==================== START / LIST / DETAIL ====================

#[derive(Debug, Deserialize)]
pub struct StartRequest {
    pub scope_type: String,
    pub scope_id: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StartResponse {
    pub session: SessionRow,
    /// An open inventory of the same place already existed and is continued
    pub resumed: bool,
}

/// POST /api/v1/inventory/sessions
pub async fn start_session(
    app_state: web::Data<Arc<AppState>>,
    body: web::Json<StartRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;
    let pool = &app_state.db_pool;
    scope_filter(&body.scope_type)?;
    let name = scope_name(pool, &body.scope_type, &body.scope_id).await?;

    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM inventory_sessions WHERE scope_type = ? AND scope_id = ? AND status = 'open' \
         ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&body.scope_type)
    .bind(&body.scope_id)
    .fetch_optional(pool)
    .await?;
    if let Some(id) = existing {
        let session = get_session(pool, &id).await?;
        return Ok(HttpResponse::Ok().json(ApiResponse::success(StartResponse { session, resumed: true })));
    }

    let id = Uuid::new_v4().to_string();
    let notes: Option<String> = body.notes.as_deref().map(|n| n.chars().take(1000).collect());
    sqlx::query(
        "INSERT INTO inventory_sessions (id, scope_type, scope_id, scope_name, status, notes, started_by, started_at) \
         VALUES (?, ?, ?, ?, 'open', ?, ?, ?)",
    )
    .bind(&id)
    .bind(&body.scope_type)
    .bind(&body.scope_id)
    .bind(&name)
    .bind(&notes)
    .bind(&claims.sub)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    crate::audit::audit(pool, &claims.sub, "inventory_start", "inventory", &id,
        &format!("Inventory started: {}", name), &http_request).await;

    let session = get_session(pool, &id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(StartResponse { session, resumed: false })))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
}

/// GET /api/v1/inventory/sessions?status=open|closed|all
pub async fn list_sessions(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<ListQuery>,
) -> ApiResult<HttpResponse> {
    let status = match query.status.as_deref() {
        Some("open") => "open",
        Some("closed") => "closed",
        Some("cancelled") => "cancelled",
        _ => "all",
    };
    let sql = format!(
        "{} WHERE (? = 'all' OR s.status = ?) \
         ORDER BY (s.status = 'open') DESC, s.started_at DESC LIMIT 200",
        SESSION_SELECT
    );
    let rows: Vec<SessionRow> = sqlx::query_as(&sql).bind(status).bind(status).fetch_all(&app_state.db_pool).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(rows)))
}

#[derive(Debug, Serialize)]
pub struct ExpectedBottle {
    #[serde(flatten)]
    pub label: ContainerLabel,
    pub scanned: bool,
}

#[derive(Debug, Serialize)]
pub struct SessionDetail {
    pub session: SessionRow,
    pub positions: Vec<ScopePosition>,
    pub counts: Counts,
    pub items: Vec<ItemRow>,
    /// Open sessions only: bottles recorded in the scope, with scanned flag
    pub expected: Vec<ExpectedBottle>,
}

async fn session_detail(pool: &SqlitePool, id: &str) -> ApiResult<SessionDetail> {
    let session = get_session(pool, id).await?;
    let positions = scope_positions(pool, &session.scope_type, &session.scope_id).await?;
    let counts = live_counts(pool, &session).await?;
    let items = session_items(pool, id).await?;
    let expected = if session.status == "open" {
        let scanned: HashSet<&str> = items.iter().filter_map(|i| i.container_id.as_deref()).collect();
        expected_in_scope(pool, &session.scope_type, &session.scope_id).await?
            .into_iter()
            .map(|label| {
                let scanned = scanned.contains(label.row.container_id.as_str());
                ExpectedBottle { label, scanned }
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(SessionDetail { session, positions, counts, items, expected })
}

/// GET /api/v1/inventory/sessions/{id}
pub async fn get_session_detail(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let detail = session_detail(&app_state.db_pool, &path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(detail)))
}

// ==================== SCAN ====================

#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    pub code: String,
    /// The shelf being checked (must belong to the session's scope)
    pub position_id: String,
}

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub result: String,
    pub already_scanned: bool,
    pub item: ItemRow,
    pub container: Option<ContainerLabel>,
    pub counts: Counts,
}

fn classify(c: &ContainerRow, shelf: &str) -> &'static str {
    if c.container_status == "empty" || c.container_status == "disposed" {
        "inactive"
    } else if c.position_id.as_deref() == Some(shelf) {
        "found"
    } else if c.position_id.is_some() {
        "misplaced"
    } else {
        "unplaced"
    }
}

/// POST /api/v1/inventory/sessions/{id}/scan
pub async fn scan(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    body: web::Json<ScanRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;
    let pool = &app_state.db_pool;
    let session_id = path.into_inner();
    let session = get_open_session(pool, &session_id).await?;

    let in_scope = scope_positions(pool, &session.scope_type, &session.scope_id).await?
        .iter().any(|p| p.id == body.position_id);
    if !in_scope {
        return Err(ApiError::bad_request("This shelf is not part of the inventory"));
    }

    let found = match label_handlers::parse_scan_code(&body.code) {
        None => Vec::new(),
        Some(q) => {
            let m = label_handlers::find_containers(pool, &q).await?;
            if m.len() > 1 && matches!(q, ScanQuery::Prefix(_)) {
                return Err(ApiError::bad_request(
                    "Several bottles share this short code. Scan the symbol instead.",
                ));
            }
            m
        }
    };
    let now = Utc::now();
    let mut tx = pool.begin().await?;
    let shelf_path = position_path(&mut tx, Some(&body.position_id)).await?;

    let (item_id, result, already, container) = match found.into_iter().next() {
        None => {
            // Not ours / no such bottle: keep the raw text for the report
            let raw: String = body.code.chars().filter(|c| !c.is_control()).take(200).collect();
            let prev: Option<String> = sqlx::query_scalar(
                "SELECT id FROM inventory_items WHERE session_id = ? AND container_id IS NULL AND raw_code = ?",
            )
            .bind(&session_id).bind(&raw).fetch_optional(&mut *tx).await?;
            let id = match prev {
                Some(id) => {
                    sqlx::query("UPDATE inventory_items SET position_id = ?, location = ?, scanned_by = ?, scanned_at = ? WHERE id = ?")
                        .bind(&body.position_id).bind(&shelf_path).bind(&claims.sub).bind(now).bind(&id)
                        .execute(&mut *tx).await?;
                    id
                }
                None => {
                    let id = Uuid::new_v4().to_string();
                    sqlx::query(
                        "INSERT INTO inventory_items (id, session_id, container_id, raw_code, position_id, location, \
                         result, scanned_by, scanned_at) VALUES (?, ?, NULL, ?, ?, ?, 'unknown', ?, ?)",
                    )
                    .bind(&id).bind(&session_id).bind(&raw).bind(&body.position_id).bind(&shelf_path)
                    .bind(&claims.sub).bind(now)
                    .execute(&mut *tx).await?;
                    id
                }
            };
            (id, "unknown", false, None)
        }
        Some(c) => {
            let result = classify(&c.row, &body.position_id);
            let prev: Option<(String, Option<String>)> = sqlx::query_as(
                "SELECT id, position_id FROM inventory_items WHERE session_id = ? AND container_id = ?",
            )
            .bind(&session_id).bind(&c.row.container_id).fetch_optional(&mut *tx).await?;
            let expected_path = position_path(&mut tx, c.row.position_id.as_deref()).await?;
            let already = prev.is_some();
            let item_id = match &prev {
                Some((id, _)) => {
                    sqlx::query(
                        "UPDATE inventory_items SET position_id = ?, location = ?, result = ?, \
                         expected_position_id = ?, expected_location = ?, \
                         resolution = CASE WHEN ? = 'found' THEN resolution ELSE NULL END, \
                         scanned_by = ?, scanned_at = ? WHERE id = ?",
                    )
                    .bind(&body.position_id).bind(&shelf_path).bind(result)
                    .bind(&c.row.position_id).bind(&expected_path).bind(result)
                    .bind(&claims.sub).bind(now).bind(id)
                    .execute(&mut *tx).await?;
                    id.clone()
                }
                None => {
                    let id = Uuid::new_v4().to_string();
                    sqlx::query(
                        "INSERT INTO inventory_items (id, session_id, container_id, raw_code, position_id, location, \
                         result, expected_position_id, expected_location, scanned_by, scanned_at) \
                         VALUES (?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind(&id).bind(&session_id).bind(&c.row.container_id)
                    .bind(&body.position_id).bind(&shelf_path).bind(result)
                    .bind(&c.row.position_id).bind(&expected_path)
                    .bind(&claims.sub).bind(now)
                    .execute(&mut *tx).await?;
                    id
                }
            };
            // Bottle history: one "seen" per session and shelf
            let seen_here_before = matches!(&prev, Some((_, Some(p))) if *p == body.position_id);
            if !seen_here_before {
                let note = match result {
                    "misplaced" => format!("Inventory {}; recorded at {}", session.scope_name,
                        expected_path.as_deref().unwrap_or("another shelf")),
                    "unplaced" => format!("Inventory {}; recorded as not placed", session.scope_name),
                    "inactive" => format!("Inventory {}; recorded as {}", session.scope_name, c.row.container_status),
                    _ => format!("Inventory {}", session.scope_name),
                };
                let mut ev = NewEvent::new(&c.row.container_id, event::SEEN, source::INVENTORY, Some(&claims.sub));
                ev.to_position = Some(&body.position_id);
                ev.session_id = Some(&session_id);
                ev.note = Some(&note);
                history::record_event(&mut tx, &ev).await?;
            }
            (item_id, result, already, Some(c))
        }
    };
    tx.commit().await?;

    let item = get_item(pool, &session_id, &item_id).await?;
    let counts = live_counts(pool, &session).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(ScanResult {
        result: result.to_string(),
        already_scanned: already,
        item,
        container,
        counts,
    })))
}

// ==================== RESOLVE ====================

#[derive(Debug, Deserialize)]
pub struct ResolveRequest {
    /// move_here (misplaced, open session) | place_here (unplaced, open)
    /// | unplace (missing, closed session)
    pub action: String,
}

/// POST /api/v1/inventory/sessions/{id}/items/{item_id}/resolve
pub async fn resolve_item(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    body: web::Json<ResolveRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;
    let pool = &app_state.db_pool;
    let (session_id, item_id) = path.into_inner();
    let session = get_session(pool, &session_id).await?;
    let item = get_item(pool, &session_id, &item_id).await?;
    let container_id = item.container_id.clone()
        .ok_or_else(|| ApiError::bad_request("This scan is not a known bottle"))?;

    let (target, resolution, new_result): (Option<&str>, &str, &str) = match body.action.as_str() {
        "move_here" | "place_here" => {
            if session.status != "open" {
                return Err(ApiError::bad_request("This inventory is already finished"));
            }
            let expected = if body.action == "move_here" { "misplaced" } else { "unplaced" };
            if item.result != expected {
                return Err(ApiError::BadRequest(format!("Only {} bottles can be resolved this way", expected)));
            }
            let pos = item.position_id.as_deref().ok_or_else(|| ApiError::bad_request("No shelf recorded for this scan"))?;
            (Some(pos), if body.action == "move_here" { "moved_here" } else { "placed_here" }, "found")
        }
        "unplace" => {
            if session.status != "closed" || item.result != "missing" {
                return Err(ApiError::bad_request("Only missing bottles of a finished inventory can be marked as not placed"));
            }
            (None, "unplaced", "missing")
        }
        _ => return Err(ApiError::bad_request("action must be move_here, place_here or unplace")),
    };

    let mut tx = pool.begin().await?;
    let change = history::change_placement(
        &mut tx, &container_id, target, Some(&claims.sub), source::INVENTORY, Some(&session_id), None,
    ).await?;
    sqlx::query("UPDATE inventory_items SET result = ?, resolution = ? WHERE id = ?")
        .bind(new_result).bind(resolution).bind(&item_id)
        .execute(&mut *tx).await?;
    tx.commit().await?;

    let batch_id: Option<String> = sqlx::query_scalar("SELECT batch_id FROM batch_containers WHERE id = ?")
        .bind(&container_id).fetch_optional(pool).await?;
    if let Some(b) = batch_id {
        let _ = crate::container_handlers::sync_batch_location(pool, &b).await;
    }
    if change.changed() {
        let what = match resolution {
            "moved_here" => format!("moved to {}", item.location.as_deref().unwrap_or("the scanned shelf")),
            "placed_here" => format!("placed at {}", item.location.as_deref().unwrap_or("the scanned shelf")),
            _ => "marked as not placed (missing in inventory)".to_string(),
        };
        crate::audit::audit(pool, &claims.sub, "inventory_resolve", "container", &container_id,
            &format!("Inventory {}: {} #{} {}", session.scope_name,
                item.reagent_name.as_deref().unwrap_or("bottle"), item.sequence_number.unwrap_or(0), what),
            &http_request).await;
    }

    let item = get_item(pool, &session_id, &item_id).await?;
    let counts = live_counts(pool, &session).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({ "item": item, "counts": counts }))))
}

// ==================== CLOSE / CANCEL ====================

#[derive(Debug, Deserialize, Default)]
pub struct CloseRequest {
    pub notes: Option<String>,
}

/// POST /api/v1/inventory/sessions/{id}/close
pub async fn close_session(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    body: Option<web::Json<CloseRequest>>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;
    let pool = &app_state.db_pool;
    let session_id = path.into_inner();
    let session = get_open_session(pool, &session_id).await?;
    let expected = expected_in_scope(pool, &session.scope_type, &session.scope_id).await?;
    let now = Utc::now();

    let mut tx = pool.begin().await?;
    // Re-check under the write lock: two people pressing "finish" at once
    let still_open: Option<String> = sqlx::query_scalar(
        "UPDATE inventory_sessions SET status = 'closed' WHERE id = ? AND status = 'open' RETURNING id",
    )
    .bind(&session_id)
    .fetch_optional(&mut *tx)
    .await?;
    if still_open.is_none() {
        return Err(ApiError::bad_request("This inventory is already finished"));
    }

    let scanned: HashSet<String> = sqlx::query_scalar::<_, String>(
        "SELECT container_id FROM inventory_items WHERE session_id = ? AND container_id IS NOT NULL",
    )
    .bind(&session_id)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect();

    let note = format!("Not found during inventory {}", session.scope_name);
    for e in expected.iter().filter(|e| !scanned.contains(&e.row.container_id)) {
        let path = position_path(&mut tx, e.row.position_id.as_deref()).await?;
        sqlx::query(
            "INSERT INTO inventory_items (id, session_id, container_id, raw_code, position_id, location, \
             result, expected_position_id, expected_location, scanned_by, scanned_at) \
             VALUES (?, ?, ?, NULL, NULL, NULL, 'missing', ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string()).bind(&session_id).bind(&e.row.container_id)
        .bind(&e.row.position_id).bind(&path).bind(&claims.sub).bind(now)
        .execute(&mut *tx).await?;

        let mut ev = NewEvent::new(&e.row.container_id, event::MISSING, source::INVENTORY, Some(&claims.sub));
        ev.from_position = e.row.position_id.as_deref();
        ev.session_id = Some(&session_id);
        ev.note = Some(&note);
        history::record_event(&mut tx, &ev).await?;
    }

    let by_result: Vec<(String, i64)> = sqlx::query_as(
        "SELECT result, COUNT(*) FROM inventory_items WHERE session_id = ? GROUP BY result",
    )
    .bind(&session_id)
    .fetch_all(&mut *tx)
    .await?;
    let get = |k: &str| by_result.iter().find(|(r, _)| r == k).map(|(_, n)| *n).unwrap_or(0);

    let notes = match (session.notes.as_deref(), body.as_ref().and_then(|b| b.notes.as_deref())) {
        (Some(a), Some(b)) if !b.trim().is_empty() => Some(format!("{}\n{}", a, b)),
        (None, Some(b)) if !b.trim().is_empty() => Some(b.to_string()),
        (a, _) => a.map(str::to_string),
    }
    .map(|n| n.chars().take(2000).collect::<String>());

    sqlx::query(
        "UPDATE inventory_sessions SET closed_by = ?, closed_at = ?, notes = ?, \
         expected_count = ?, found_count = ?, misplaced_count = ?, unplaced_count = ?, \
         inactive_count = ?, unknown_count = ?, missing_count = ? WHERE id = ?",
    )
    .bind(&claims.sub).bind(now).bind(&notes)
    .bind(expected.len() as i64).bind(get("found")).bind(get("misplaced")).bind(get("unplaced"))
    .bind(get("inactive")).bind(get("unknown")).bind(get("missing"))
    .bind(&session_id)
    .execute(&mut *tx).await?;
    tx.commit().await?;

    crate::audit::audit(pool, &claims.sub, "inventory_close", "inventory", &session_id,
        &format!("Inventory finished: {} — {} expected, {} found, {} missing, {} misplaced",
            session.scope_name, expected.len(), get("found"), get("missing"), get("misplaced")),
        &http_request).await;

    let detail = session_detail(pool, &session_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(detail)))
}

/// POST /api/v1/inventory/sessions/{id}/cancel — discard an inventory started by mistake
pub async fn cancel_session(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;
    let pool = &app_state.db_pool;
    let session_id = path.into_inner();
    let session = get_open_session(pool, &session_id).await?;
    sqlx::query("UPDATE inventory_sessions SET status = 'cancelled', closed_by = ?, closed_at = ? WHERE id = ? AND status = 'open'")
        .bind(&claims.sub).bind(Utc::now()).bind(&session_id)
        .execute(pool).await?;
    crate::audit::audit(pool, &claims.sub, "inventory_cancel", "inventory", &session_id,
        &format!("Inventory cancelled: {}", session.scope_name), &http_request).await;
    Ok(HttpResponse::Ok().json(ApiResponse::success(get_session(pool, &session_id).await?)))
}

// ==================== TESTS ====================

#[cfg(test)]
mod tests {
    use super::*;

    fn row(status: &str, position: Option<&str>) -> ContainerRow {
        ContainerRow {
            container_id: "c".into(), sequence_number: 1, batch_container_count: 1,
            quantity: 1.0, original_quantity: 1.0, container_status: status.into(), is_opened: false,
            batch_id: "b".into(), batch_number: "B".into(), lot_number: None, unit: "g".into(),
            batch_status: "available".into(), batch_quantity: 1.0, reserved_quantity: 0.0,
            expiry_date: None, received_date: Utc::now(), reagent_id: "r".into(), reagent_name: "R".into(),
            formula: None, cas_number: None, hazard_pictograms: None, storage_conditions: None,
            placement_id: position.map(|_| "pl".into()), position_id: position.map(str::to_string),
            position_name: None, zone_id: None, zone_name: None, room_name: None, room_color: None,
        }
    }

    #[test]
    fn scans_are_classified_against_the_shelf() {
        assert_eq!(classify(&row("full", Some("p1")), "p1"), "found");
        assert_eq!(classify(&row("partial", Some("p2")), "p1"), "misplaced");
        assert_eq!(classify(&row("full", None), "p1"), "unplaced");
        assert_eq!(classify(&row("empty", Some("p1")), "p1"), "inactive");
        assert_eq!(classify(&row("disposed", None), "p1"), "inactive");
    }

    #[test]
    fn scope_types_are_whitelisted() {
        assert!(scope_filter("zone").is_ok());
        assert!(scope_filter("room").is_ok());
        assert!(scope_filter("position").is_ok());
        assert!(scope_filter("1=1; DROP TABLE x").is_err());
    }

    #[tokio::test]
    async fn expected_bottles_follow_the_scope() {
        let pool = history::tests::setup().await;
        for (c, p) in [("c1", "p1"), ("c2", "p2"), ("c3", "p3"), ("c4", "p1")] {
            sqlx::query("INSERT INTO batch_placements VALUES (?, ?, ?, 'u1', '2026-01-01T00:00:00Z', NULL)")
                .bind(format!("pl-{}", c)).bind(c).bind(p).execute(&pool).await.unwrap();
        }
        let ids = |v: Vec<ContainerLabel>| v.into_iter().map(|l| l.row.container_id).collect::<Vec<_>>();
        // c4 is empty: not expected anywhere
        assert_eq!(ids(expected_in_scope(&pool, "position", "p1").await.unwrap()), vec!["c1"]);
        assert_eq!(ids(expected_in_scope(&pool, "zone", "z1").await.unwrap()), vec!["c1", "c2"]);
        assert_eq!(ids(expected_in_scope(&pool, "room", "room2").await.unwrap()), vec!["c3"]);
        let shelves: Vec<String> = scope_positions(&pool, "zone", "z1").await.unwrap().into_iter().map(|p| p.name).collect();
        assert_eq!(shelves, vec!["Shelf 1", "Shelf 2"]);
        assert_eq!(scope_name(&pool, "zone", "z1").await.unwrap(), "Lab 101 / Cabinet A");
        assert!(scope_name(&pool, "zone", "nope").await.is_err());
    }
}
