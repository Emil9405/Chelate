// src/routes/labels.rs
//! Bottle and storage location labels: label data for printing and scan lookup (read-only).

use actix_web::web;
use crate::{label_handlers, location_labels};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/labels")
            .route("/resolve", web::get().to(label_handlers::resolve_scan))
            .route("/containers", web::post().to(label_handlers::get_container_labels))
            .route("/locations", web::post().to(location_labels::get_location_labels))
    );
}
