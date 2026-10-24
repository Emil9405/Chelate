// src/routes/intake.rs
//! Receiving goods by scanning the supplier's code.

use actix_web::web;
use crate::supplier_intake;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/intake")
            .route("/receive", web::post().to(supplier_intake::receive_handler))
            .route("/batch-check", web::get().to(supplier_intake::batch_check_handler))
            .route("/products", web::put().to(supplier_intake::link_product_handler))
            .route("/products", web::get().to(supplier_intake::articles_handler))
            .route("/products/backfill", web::get().to(supplier_intake::backfill_preview_handler))
            .route("/products/backfill", web::post().to(supplier_intake::backfill_apply_handler))
    );
}
