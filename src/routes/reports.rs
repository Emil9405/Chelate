// src/routes/reports.rs
use actix_web::web;
use crate::report_handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/reports")
            .route("/presets", web::get().to(report_handlers::get_report_presets))
            .route("/fields", web::get().to(report_handlers::get_report_fields))
            .route("/generate", web::post().to(report_handlers::generate_report))
            .route("/export", web::post().to(report_handlers::export_report))
    );
}
