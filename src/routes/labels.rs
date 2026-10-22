// src/routes/labels.rs
//! Bottle labels: label data for printing and scan lookup (read-only).

use actix_web::web;
use crate::label_handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/labels")
            .route("/resolve", web::get().to(label_handlers::resolve_scan))
            .route("/containers", web::post().to(label_handlers::get_container_labels))
    );
}
