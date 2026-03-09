// src/routes/dashboard.rs
use actix_web::web;
use crate::handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/dashboard")
            .route("/stats", web::get().to(handlers::get_dashboard_stats))
            .route("/recent-activity", web::get().to(handlers::get_recent_activity))
            .route("/trends", web::get().to(handlers::get_dashboard_trends))
    );
}
