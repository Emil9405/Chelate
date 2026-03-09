// src/main.rs — Bootstrap only
// Protected wrappers and routes are in src/routes/

use actix_web::{
    middleware::{Logger, DefaultHeaders, Compress},
    web, App, HttpResponse, HttpServer, HttpRequest, Result,
};
use actix_web_httpauth::middleware::HttpAuthentication;
use actix_web::http::header;
use actix_cors::Cors;
use actix_files::{NamedFile, Files};
use std::env;
use std::path::PathBuf;
use crate::config::load_config;
use crate::auth::get_current_user;

use rand::{thread_rng, Rng, distributions::Alphanumeric};
use rand::distributions::Distribution;
use rand::seq::SliceRandom;
use anyhow::Context;
use sqlx::{sqlite::SqliteConnectOptions, migrate::MigrateDatabase, Sqlite, SqlitePool};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Module declarations
mod auth;
mod audit;
mod auth_handlers;
mod filter_handlers;
mod config;
mod db;
mod error;
mod handlers;
mod experiment_handlers;
mod report_handlers;
mod models;
mod monitoring;
mod jwt_rotation;
mod storage_handlers;
pub mod validator;
mod placement_handlers;
mod container_handlers;
pub mod repositories;
pub mod query_builders;
mod reagent_handlers;
pub mod room_handlers;
mod batch_handlers;
mod equipment_handlers;
mod import_export;
mod pagination;
mod routes;

use config::Config;
use auth::{AuthService, jwt_middleware};
use auth_handlers::*;
use monitoring::{Metrics, RequestLogger, start_maintenance_tasks};
use error::ApiResult;
use experiment_handlers::{run_auto_update_statuses, seconds_until_next_transition};

pub struct AppState {
    pub db_pool: SqlitePool,
    pub config: Config,
}

// ==================== ADMIN PROTECTED ====================

async fn rebuild_cache_protected(
    app_state: web::Data<Arc<AppState>>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = auth::get_current_user(&http_request)?;
    if claims.role != crate::auth::UserRole::Admin {
        return Err(crate::error::ApiError::Forbidden("Admin access required".to_string()));
    }
    reagent_handlers::rebuild_cache(app_state).await
}

// ==================== MAIN ====================

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let config = load_config()?;
    setup_logging(&config)?;

    if env::var("LIMS_ENV").as_deref() == Ok("production") {
        validate_production_config(&config)?;
    }

    setup_database(&config.database.url).await?;
    let pool = create_database_pool(&config.database).await?;
    db::run_migrations(&pool).await?;
    jwt_rotation::init_rotation_table(&pool).await?;

    let auth_service = Arc::new(AuthService::new(&config.auth.jwt_secret));
    create_default_admin_if_needed(&pool, &auth_service).await?;

    let app_state = Arc::new(AppState {
        db_pool: pool.clone(),
        config: config.clone(),
    });

    // Background tasks
    let pool_clone = pool.clone();
    tokio::spawn(async move { start_maintenance_tasks(pool_clone).await; });

    // Experiment auto-update (event-driven)
    let experiment_pool = pool.clone();
    tokio::spawn(async move {
        use tokio::time::{sleep, Duration};
        const MAX_IDLE_SECS: u64 = 300;
        const MIN_PAUSE_SECS: u64 = 2;

        sleep(Duration::from_secs(5)).await;
        log::info!("Experiment auto-update task started (event-driven, idle check: {}s)", MAX_IDLE_SECS);

        loop {
            let sleep_secs = match seconds_until_next_transition(&experiment_pool).await {
                Ok(Some(secs)) if secs <= 0 => {
                    match run_auto_update_statuses(&experiment_pool).await {
                        Ok(r) if r.total_updated > 0 => {
                            log::info!("BG auto-update: {} started, {} completed", r.started, r.completed);
                        }
                        Err(e) => log::error!("BG auto-update error: {}", e),
                        _ => {}
                    }
                    MIN_PAUSE_SECS
                }
                Ok(Some(secs)) => {
                    let wait = (secs as u64).min(MAX_IDLE_SECS) + 1;
                    log::debug!("Next experiment transition in ~{}s, sleeping {}s", secs, wait);
                    wait
                }
                Ok(None) => MAX_IDLE_SECS,
                Err(e) => { log::error!("BG next-transition query error: {}", e); MAX_IDLE_SECS }
            };
            sleep(Duration::from_secs(sleep_secs)).await;
        }
    });

    // JWT rotation
    let rotation_pool = pool.clone();
    let env_file = env::var("ENV_FILE").unwrap_or_else(|_| ".env".to_string());
    tokio::spawn(async move { jwt_rotation::start_rotation_task(rotation_pool, env_file).await; });

    let bind_address = format!("{}:{}", config.server.host, config.server.port);
    log::info!("Starting server at http://{}", bind_address);

    let metrics_arc = Arc::new(Metrics::new());
    let metrics = web::Data::from(metrics_arc.clone());

    HttpServer::new(move || {
        let cors = setup_improved_cors(&config.security.allowed_origins);
        let security_headers = setup_security_headers(&config.security);

        let app = App::new()
            .wrap(cors)
            .wrap(security_headers)
            .wrap(Logger::default())
            .wrap(Compress::default())
            .wrap(RequestLogger::new(metrics_arc.clone()))
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(auth_service.clone()))
            .app_data(metrics.clone())

            // Health (no auth)
            .service(
                web::scope("/health")
                    .route("", web::get().to(|| async { HttpResponse::Ok().body("OK") }))
                    .route("/metrics", web::get().to(monitoring::metrics_endpoint))
            )

            // Auth (no auth)
            .service(
                web::scope("/auth")
                    .route("/login", web::post().to(login))
                    .route("/register", web::post().to(register))
            )

            // Public file access
            .service(
                web::scope("/api/v1/public")
                    .route("/equipment/{id}/files/{file_id}", web::get().to(equipment_handlers::download_equipment_file))
            )

            // All protected API routes
            .configure(routes::configure_api);

        // Static files (production)
        if env::var("LIMS_ENV").as_deref() == Ok("production") {
            let build_dir = env::var("FRONTEND_BUILD_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("..").join("lims-frontend").join("build"));
            let build_dir_str = build_dir.to_string_lossy().to_string();
            app.service(Files::new("/static", format!("{}/static", build_dir_str)))
                .service(Files::new("/assets", format!("{}/assets", build_dir_str)))
                .default_service(web::route().to(serve_index))
        } else {
            app.route("/", web::get().to(serve_index))
        }
    })
        .bind(&bind_address)?
        .run()
        .await
        .context("Server failed to run")?;

    Ok(())
}

// ==================== HELPERS ====================

pub fn setup_improved_cors(allowed_origins: &[String]) -> Cors {
    let mut cors = Cors::default()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allowed_headers(vec![
            header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT,
            header::USER_AGENT, header::REFERER,
        ])
        .expose_headers(vec![header::CONTENT_LENGTH])
        .max_age(3600);

    let is_production = std::env::var("LIMS_ENV").as_deref() == Ok("production");

    if allowed_origins.contains(&"*".to_string()) {
        if is_production {
            panic!("Cannot start server with wildcard CORS in production");
        } else {
            cors = cors.allow_any_origin().allow_any_header().allow_any_method();
        }
    } else if !is_production {
        for origin in allowed_origins { cors = cors.allowed_origin(origin); }
    } else {
        for origin in allowed_origins {
            if !origin.is_empty() { cors = cors.allowed_origin(origin); }
        }
    }
    cors
}

#[deprecated(note = "Use setup_improved_cors instead")]
pub fn setup_cors(allowed_origins: &[String]) -> Cors { setup_improved_cors(allowed_origins) }

fn setup_logging(config: &Config) -> anyhow::Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(config.logging.level.as_str()));
    tracing_subscriber::registry().with(filter).with(tracing_subscriber::fmt::layer()).init();
    Ok(())
}

fn validate_production_config(config: &Config) -> anyhow::Result<()> {
    if config.auth.jwt_secret == "your-secret-key-here" || config.auth.jwt_secret.len() < 32 {
        anyhow::bail!("Insecure JWT secret in production!");
    }
    if config.security.allowed_origins.contains(&"*".to_string()) {
        anyhow::bail!("Wildcard CORS origins not allowed in production!");
    }
    Ok(())
}

async fn setup_database(database_url: &str) -> anyhow::Result<()> {
    if !Sqlite::database_exists(database_url).await.unwrap_or(false) {
        log::info!("Creating database: {}", database_url);
        Sqlite::create_database(database_url).await?;
    }
    Ok(())
}

async fn create_database_pool(db_config: &crate::config::DatabaseConfig) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::new().filename(&db_config.url).create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    Ok(pool)
}

fn setup_security_headers(config: &crate::config::SecurityConfig) -> DefaultHeaders {
    let mut headers = DefaultHeaders::new()
        .add(("X-Content-Type-Options", "nosniff"))
        .add(("X-Frame-Options", "DENY"))
        .add(("X-XSS-Protection", "1; mode=block"))
        .add(("Referrer-Policy", "strict-origin-when-cross-origin"));
    if config.require_https {
        headers = headers.add(("Strict-Transport-Security", "max-age=31536000; includeSubDomains; preload"));
    }
    headers
}

async fn create_default_admin_if_needed(pool: &SqlitePool, auth_service: &AuthService) -> anyhow::Result<()> {
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(pool).await?;
    if user_count.0 == 0 {
        use crate::auth::{RegisterRequest, UserRole};

        let password = env::var("DEFAULT_ADMIN_PASSWORD").unwrap_or_else(|_| {
            let mut rng = thread_rng();
            let digits: Vec<char> = "0123456789".chars().collect();
            let specials: Vec<char> = "!@#$%^&*()_+-=[]{}|;:,.<>?".chars().collect();
            let uppercase: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect();
            let lowercase: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
            let alphanumeric = Alphanumeric;
            let mut pwd_chars: Vec<char> = Vec::new();
            pwd_chars.push(*digits.choose(&mut rng).unwrap());
            pwd_chars.push(*specials.choose(&mut rng).unwrap());
            pwd_chars.push(*uppercase.choose(&mut rng).unwrap());
            pwd_chars.push(*lowercase.choose(&mut rng).unwrap());
            for _ in 0..8 {
                if rng.gen_bool(0.5) {
                    if rng.gen_bool(0.5) {
                        let sample_u8 = alphanumeric.sample(&mut rng);
                        pwd_chars.push(char::from_u32(sample_u8 as u32).unwrap());
                    } else { pwd_chars.push(*digits.choose(&mut rng).unwrap()); }
                } else { pwd_chars.push(*specials.choose(&mut rng).unwrap()); }
            }
            pwd_chars.shuffle(&mut rng);
            let pwd: String = pwd_chars.into_iter().collect();
            log::warn!("Generated admin password: {}", pwd);
            pwd
        });

        let admin_request = RegisterRequest {
            username: "admin".to_string(),
            email: "admin@lims.local".to_string(),
            password: password.clone(),
            role: None,
        };

        let mut user = crate::auth::User::create(pool, admin_request, UserRole::Viewer, auth_service)
            .await.map_err(|e| anyhow::anyhow!("Failed to create default admin: {}", e))?;

        sqlx::query("UPDATE users SET role = ?, updated_at = datetime('now') WHERE id = ?")
            .bind("admin").bind(&user.id).execute(pool).await?;
        user.role = "admin".to_string();

        log::warn!("Default admin created. Username: admin, Password: {}", password);
    }
    Ok(())
}

async fn serve_index() -> Result<NamedFile> {
    let path: PathBuf = match env::var("LIMS_ENV").as_deref() {
        Ok("production") => {
            env::var("FRONTEND_BUILD_DIR").map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("..").join("lims-frontend").join("build"))
                .join("index.html")
        }
        _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web_interface.html"),
    };
    Ok(NamedFile::open(path)?)
}
