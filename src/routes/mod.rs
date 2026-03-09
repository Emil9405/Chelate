// src/routes/mod.rs
//! Маршрутизация разбита по доменам.
//! Каждый суб-модуль содержит protected wrappers + configure функцию.

pub mod experiments;
pub mod reagents;
pub mod batches;
pub mod containers;
pub mod equipment;
pub mod rooms;
pub mod storage;
pub mod reports;
pub mod auth_routes;
pub mod dashboard;

use actix_web::web;
use actix_web_httpauth::middleware::HttpAuthentication;
use crate::auth::jwt_middleware;
use crate::import_export;
use crate::filter_handlers;

/// Настраивает все API маршруты
pub fn configure_api(cfg: &mut web::ServiceConfig) {
    let auth_middleware = HttpAuthentication::bearer(jwt_middleware);

    cfg.service(
        web::scope("/api/v1")
            .wrap(auth_middleware)
            .configure(dashboard::configure)
            .configure(auth_routes::configure)
            .configure(reagents::configure)
            .configure(batches::configure)
            .configure(containers::configure)
            .configure(equipment::configure)
            .configure(rooms::configure)
            .configure(storage::configure)
            .configure(experiments::configure)
            .configure(reports::configure)
            // Unit conversion
            .service(
                web::scope("/units")
                    .route("/convert", web::post().to(crate::batch_handlers::convert_units))
            )
            // Admin
            .service(
                web::scope("/admin")
                    .route("/cache/rebuild", web::post().to(super::rebuild_cache_protected))
            )
    );
}
