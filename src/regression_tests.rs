// src/regression_tests.rs
//! Regression tests on the real schema (all migrations, in-memory database):
//! permission checks that used to be missing and the experiment reservation
//! bookkeeping.

use std::sync::Arc;

use actix_web::{http::StatusCode, test::TestRequest, web, HttpMessage, HttpRequest, ResponseError};
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::auth::{AuthService, Claims, UserRole};
use crate::auth_handlers::{check_room_permission, require_permission_key, RoomAction};
use crate::config::Config;
use crate::experiment_handlers::{
    add_reagent_to_experiment, remove_reagent_from_experiment, AddReagentToExperimentRequest,
};
use crate::AppState;

/// One connection: every connection of an in-memory SQLite gets its own database
async fn setup() -> web::Data<Arc<AppState>> {
    let options: SqliteConnectOptions = "sqlite::memory:".parse::<SqliteConnectOptions>().unwrap().foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    crate::db::run_migrations(&pool).await.unwrap();

    for (id, role) in [("u-admin", "admin"), ("u-res", "researcher"), ("u-view", "viewer")] {
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role, is_active, created_at, updated_at) \
             VALUES (?, ?, ?, 'x', ?, 1, datetime('now'), datetime('now'))",
        )
        .bind(id)
        .bind(format!("user-{}", role))
        .bind(format!("{}@lims.local", role))
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    }

    web::Data::new(Arc::new(AppState { db_pool: pool, config: Config::default() }))
}

fn request_as(user_id: &str, role: UserRole) -> HttpRequest {
    let req = TestRequest::default().to_http_request();
    let now = Utc::now().timestamp();
    req.extensions_mut().insert(Claims {
        sub: user_id.to_string(),
        username: user_id.to_string(),
        email: format!("{}@lims.local", user_id),
        role,
        exp: now + 3600,
        iat: now,
    });
    req
}

async fn set_permissions(pool: &SqlitePool, user_id: &str, json: &str) {
    sqlx::query("INSERT INTO user_permissions (user_id, permissions) VALUES (?, ?)")
        .bind(user_id)
        .bind(json)
        .execute(pool)
        .await
        .unwrap();
}

async fn add_batch(pool: &SqlitePool, id: &str, quantity: f64) {
    sqlx::query(
        "INSERT OR IGNORE INTO reagents (id, name, status, created_at, updated_at) \
         VALUES ('r1', 'Acetone', 'active', datetime('now'), datetime('now'))",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO batches (id, reagent_id, batch_number, quantity, original_quantity, unit, \
                              received_date, status, created_at, updated_at) \
         VALUES (?, 'r1', ?, ?, ?, 'g', datetime('now'), 'available', datetime('now'), datetime('now'))",
    )
    .bind(id)
    .bind(format!("B-{}", id))
    .bind(quantity)
    .bind(quantity)
    .execute(pool)
    .await
    .unwrap();
}

async fn add_experiment(pool: &SqlitePool, id: &str) {
    sqlx::query(
        "INSERT INTO experiments (id, title, experiment_date, status, start_date, created_at, updated_at) \
         VALUES (?, 'Test', datetime('now'), 'planned', datetime('now'), datetime('now'), datetime('now'))",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}

async fn reserved(pool: &SqlitePool, batch_id: &str) -> f64 {
    sqlx::query_scalar("SELECT reserved_quantity FROM batches WHERE id = ?")
        .bind(batch_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn reserve(
    state: &web::Data<Arc<AppState>>,
    experiment_id: &str,
    batch_id: &str,
    quantity: f64,
) -> StatusCode {
    let body = web::Json(AddReagentToExperimentRequest {
        batch_id: batch_id.to_string(),
        quantity_used: quantity,
        notes: None,
    });
    match add_reagent_to_experiment(
        state.clone(),
        web::Path::from(experiment_id.to_string()),
        body,
        "u-res".to_string(),
    )
    .await
    {
        Ok(resp) => resp.status(),
        Err(e) => e.status_code(),
    }
}

// ==================== PERMISSIONS ====================

#[actix_rt::test]
async fn storage_layout_needs_edit_room_permission() {
    let state = setup().await;
    let pool = &state.db_pool;

    let viewer = request_as("u-view", UserRole::Viewer);
    let err = check_room_permission(&viewer, RoomAction::Edit, pool).await.unwrap_err();
    assert_eq!(err.status_code(), StatusCode::FORBIDDEN);

    let researcher = request_as("u-res", UserRole::Researcher);
    assert!(check_room_permission(&researcher, RoomAction::Edit, pool).await.is_ok());

    // custom permissions override the role in both directions
    set_permissions(pool, "u-view", r#"{"edit_room": true}"#).await;
    assert!(check_room_permission(&viewer, RoomAction::Edit, pool).await.is_ok());
    set_permissions(pool, "u-res", r#"{"edit_room": false}"#).await;
    assert!(check_room_permission(&researcher, RoomAction::Edit, pool).await.is_err());
}

#[actix_rt::test]
async fn use_batch_permission_can_be_revoked() {
    let state = setup().await;
    let pool = &state.db_pool;
    let viewer = request_as("u-view", UserRole::Viewer);

    // default: every role may write off stock
    assert!(require_permission_key(&viewer, pool, "use_batch", UserRole::can_use_batches).await.is_ok());

    set_permissions(pool, "u-view", r#"{"view_batch": true, "use_batch": false}"#).await;
    let err = require_permission_key(&viewer, pool, "use_batch", UserRole::can_use_batches)
        .await
        .unwrap_err();
    assert_eq!(err.status_code(), StatusCode::FORBIDDEN);

    // admins are never locked out
    let admin = request_as("u-admin", UserRole::Admin);
    set_permissions(pool, "u-admin", r#"{"use_batch": false}"#).await;
    assert!(require_permission_key(&admin, pool, "use_batch", UserRole::can_use_batches).await.is_ok());
}

// ==================== EXPERIMENT RESERVATIONS ====================

#[actix_rt::test]
async fn reservation_never_exceeds_free_stock() {
    let state = setup().await;
    let pool = &state.db_pool;
    add_batch(pool, "b1", 10.0).await;
    // a batch can be linked to an experiment once, so each request is its own experiment
    for e in ["e1", "e2", "e3", "e4"] {
        add_experiment(pool, e).await;
    }

    assert_eq!(reserve(&state, "e1", "b1", 6.0).await, StatusCode::CREATED);
    assert_eq!(reserve(&state, "e2", "b1", 6.0).await, StatusCode::BAD_REQUEST);
    assert_eq!(reserved(pool, "b1").await, 6.0);

    // the refused request left no link behind
    let links: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM experiment_reagents WHERE batch_id = 'b1'")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(links, 1);

    // exactly the rest is still reservable
    assert_eq!(reserve(&state, "e3", "b1", 4.0).await, StatusCode::CREATED);
    assert_eq!(reserved(pool, "b1").await, 10.0);

    // linking the same batch to the same experiment again is a conflict (409,
    // it used to surface as a 500) and must not leave a reservation behind
    assert_eq!(reserve(&state, "e3", "b1", 0.0001).await, StatusCode::UNPROCESSABLE_ENTITY);
    sqlx::query("UPDATE batches SET quantity = 20, original_quantity = 20 WHERE id = 'b1'")
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(reserve(&state, "e3", "b1", 1.0).await, StatusCode::CONFLICT);
    assert_eq!(reserved(pool, "b1").await, 10.0);
    assert_eq!(reserve(&state, "e4", "b1", 1.0).await, StatusCode::CREATED);
    assert_eq!(reserved(pool, "b1").await, 11.0);
}

#[actix_rt::test]
async fn reservation_tolerates_float_noise() {
    let state = setup().await;
    let pool = &state.db_pool;
    add_batch(pool, "b1", 0.3).await;
    for e in ["e1", "e2", "e3"] {
        add_experiment(pool, e).await;
    }

    // 0.3 - 0.1 = 0.19999999999999998 in f64; 0.2 is still free
    assert_eq!(reserve(&state, "e1", "b1", 0.1).await, StatusCode::CREATED);
    assert_eq!(reserve(&state, "e2", "b1", 0.2).await, StatusCode::CREATED);
    assert_eq!(reserve(&state, "e3", "b1", 0.01).await, StatusCode::BAD_REQUEST);
}

#[actix_rt::test]
async fn deleted_batch_cannot_be_reserved() {
    let state = setup().await;
    let pool = &state.db_pool;
    add_batch(pool, "b1", 10.0).await;
    add_experiment(pool, "e1").await;
    sqlx::query("UPDATE batches SET deleted_at = datetime('now') WHERE id = 'b1'")
        .execute(pool)
        .await
        .unwrap();

    assert_eq!(reserve(&state, "e1", "b1", 1.0).await, StatusCode::NOT_FOUND);
    assert_eq!(reserved(pool, "b1").await, 0.0);
}

#[actix_rt::test]
async fn removing_a_link_twice_releases_once() {
    let state = setup().await;
    let pool = &state.db_pool;
    add_batch(pool, "b1", 10.0).await;
    add_experiment(pool, "e1").await;
    add_experiment(pool, "e2").await;
    assert_eq!(reserve(&state, "e1", "b1", 3.0).await, StatusCode::CREATED);
    assert_eq!(reserve(&state, "e2", "b1", 4.0).await, StatusCode::CREATED);

    let link: String = sqlx::query_scalar(
        "SELECT id FROM experiment_reagents WHERE experiment_id = 'e1' AND planned_quantity = 3.0",
    )
    .fetch_one(pool)
    .await
    .unwrap();

    let remove = |link: String| {
        let state = state.clone();
        async move {
            remove_reagent_from_experiment(
                state,
                web::Path::from(("e1".to_string(), link)),
                "u-res".to_string(),
            )
            .await
            .map(|r| r.status())
            .map_err(|e| e.status_code())
        }
    };

    assert_eq!(remove(link.clone()).await, Ok(StatusCode::OK));
    assert_eq!(reserved(pool, "b1").await, 4.0);
    assert_eq!(remove(link).await, Err(StatusCode::NOT_FOUND));
    assert_eq!(reserved(pool, "b1").await, 4.0, "the other reservation must stay");
}

// ==================== AUTH SETTINGS ====================

#[test]
fn token_lifetime_follows_configuration() {
    let secret = "0123456789abcdef0123456789abcdef";
    assert_eq!(AuthService::new(secret).token_ttl_seconds(), 24 * 3600);
    assert_eq!(AuthService::new(secret).with_token_expiration_hours(8).token_ttl_seconds(), 8 * 3600);
    // a zero or negative setting must not produce tokens that are born expired
    assert_eq!(AuthService::new(secret).with_token_expiration_hours(0).token_ttl_seconds(), 3600);
}

// ==================== JWT ROTATION ====================

fn test_user() -> crate::auth::User {
    crate::auth::User {
        id: "u-res".to_string(),
        username: "user-researcher".to_string(),
        email: "researcher@lims.local".to_string(),
        password_hash: String::new(),
        name: None,
        role: "researcher".to_string(),
        is_active: true,
        last_login: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        failed_login_attempts: 0,
        locked_until: None,
    }
}

#[test]
fn rotation_keeps_current_sessions_and_drops_older_ones() {
    let auth = AuthService::new("first-secret-0123456789abcdef0123456789");
    let before = auth.generate_token(&test_user()).unwrap();

    auth.rotate("second-secret-0123456789abcdef012345678");
    assert!(auth.verify_token(&before).is_ok(), "a token issued before the rotation still works");
    let after = auth.generate_token(&test_user()).unwrap();
    assert!(auth.verify_token(&after).is_ok());

    // A restart with both secrets from the env file keeps both generations
    let restarted = AuthService::new("second-secret-0123456789abcdef012345678")
        .with_previous_secret(Some("first-secret-0123456789abcdef0123456789"));
    assert!(restarted.verify_token(&before).is_ok());
    assert!(restarted.verify_token(&after).is_ok());

    // Only one previous secret is kept
    auth.rotate("third-secret-0123456789abcdef0123456789");
    assert!(auth.verify_token(&after).is_ok());
    assert!(auth.verify_token(&before).is_err(), "two rotations back is no longer accepted");
    assert!(auth.verify_token("not-a-token").is_err());
}

#[actix_rt::test]
async fn rotation_updates_env_file_and_running_service() {
    let state = setup().await;
    let pool = &state.db_pool;
    crate::jwt_rotation::init_rotation_table(pool).await.unwrap();

    let dir = tempfile::tempdir().unwrap();
    let env_path = dir.path().join(".env");
    std::fs::write(&env_path, "DATABASE_URL=lims.db\nJWT_SECRET=first-secret-0123456789abcdef0123456789\nLIMS_PORT=8080\n").unwrap();
    let env_path = env_path.to_string_lossy().to_string();

    let auth = AuthService::new("first-secret-0123456789abcdef0123456789");
    let before = auth.generate_token(&test_user()).unwrap();

    crate::jwt_rotation::rotate_jwt_secret(pool, &env_path, &auth).await.unwrap();

    let new_secret = auth.current_secret();
    assert_ne!(new_secret, "first-secret-0123456789abcdef0123456789");
    let written = std::fs::read_to_string(&env_path).unwrap();
    assert!(written.contains(&format!("JWT_SECRET={}\n", new_secret)));
    assert!(written.contains("JWT_SECRET_PREVIOUS=first-secret-0123456789abcdef0123456789\n"));
    assert!(written.contains("DATABASE_URL=lims.db\n") && written.contains("LIMS_PORT=8080\n"), "other settings untouched");
    assert!(auth.verify_token(&before).is_ok());

    let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM jwt_rotation_log WHERE is_active = 1")
        .fetch_one(pool).await.unwrap();
    assert_eq!(active, 1);
}

#[actix_rt::test]
async fn failed_env_write_leaves_the_secret_unchanged() {
    let state = setup().await;
    let pool = &state.db_pool;
    crate::jwt_rotation::init_rotation_table(pool).await.unwrap();
    let auth = AuthService::new("first-secret-0123456789abcdef0123456789");

    let result = crate::jwt_rotation::rotate_jwt_secret(pool, "/nonexistent-dir/for-test/.env", &auth).await;
    assert!(result.is_err());
    assert_eq!(auth.current_secret(), "first-secret-0123456789abcdef0123456789");
}

// ==================== FILE LINKS ====================

#[test]
fn file_link_is_bound_to_one_file_and_is_not_a_session() {
    let auth = AuthService::new("first-secret-0123456789abcdef0123456789");
    let link = auth.sign_file_link("file-1", 300).unwrap();

    assert!(auth.verify_file_link(&link, "file-1"));
    assert!(!auth.verify_file_link(&link, "file-2"), "a link opens only its own file");
    assert!(auth.verify_token(&link).is_err(), "a file link is not a session token");

    let session = auth.generate_token(&test_user()).unwrap();
    assert!(!auth.verify_file_link(&session, "file-1"), "a session token is not a file link");

    let expired = auth.sign_file_link("file-1", -10).unwrap();
    assert!(!auth.verify_file_link(&expired, "file-1"));

    // links issued just before a rotation still open
    auth.rotate("second-secret-0123456789abcdef012345678");
    assert!(auth.verify_file_link(&link, "file-1"));
}

// ==================== PERMISSIONS: ONE SOURCE ====================

#[actix_rt::test]
async fn view_and_maintenance_keys_are_enforced() {
    use crate::auth_handlers::{check_batch_permission_async, check_equipment_permission, BatchAction, EquipmentAction};
    let state = setup().await;
    let pool = &state.db_pool;
    let viewer = request_as("u-view", UserRole::Viewer);

    assert!(check_batch_permission_async(&viewer, BatchAction::View, pool).await.is_ok());
    assert!(check_equipment_permission(&viewer, EquipmentAction::Maintenance, pool).await.is_err());

    // viewing switched off, maintenance switched on
    set_permissions(pool, "u-view", r#"{"view_batch": false, "manage_maintenance": true}"#).await;
    assert!(check_batch_permission_async(&viewer, BatchAction::View, pool).await.is_err());
    assert!(check_equipment_permission(&viewer, EquipmentAction::Maintenance, pool).await.is_ok());
    assert!(check_equipment_permission(&viewer, EquipmentAction::Edit, pool).await.is_err());
}

#[actix_rt::test]
async fn storage_delete_needs_delete_room() {
    let state = setup().await;
    let pool = &state.db_pool;
    let researcher = request_as("u-res", UserRole::Researcher);
    assert!(check_room_permission(&researcher, RoomAction::Edit, pool).await.is_ok());
    assert!(check_room_permission(&researcher, RoomAction::Delete, pool).await.is_err());
    assert!(check_room_permission(&request_as("u-admin", UserRole::Admin), RoomAction::Delete, pool).await.is_ok());
}
