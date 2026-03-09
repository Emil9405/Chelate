// src/routes/auth_routes.rs
use actix_web::{web, HttpRequest, HttpResponse};
use crate::auth_handlers;
use crate::handlers;
use crate::error::ApiResult;

// Logout stub
async fn logout(_http_request: HttpRequest) -> ApiResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(handlers::ApiResponse::<()>::success_with_message(
        (),
        "Logged out successfully".to_string(),
    )))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .route("/profile", web::get().to(auth_handlers::get_profile))
            .route("/change-password", web::post().to(auth_handlers::change_password))
            .route("/logout", web::post().to(logout))
            .route("/roles", web::get().to(auth_handlers::get_roles))
            .route("/users", web::get().to(auth_handlers::get_users))
            .route("/users", web::post().to(auth_handlers::create_user))
            .route("/users/{id}", web::get().to(auth_handlers::get_user))
            .route("/users/{id}", web::put().to(auth_handlers::update_user))
            .route("/users/{id}", web::delete().to(auth_handlers::delete_user))
            .route("/users/{id}/reset-password", web::put().to(auth_handlers::change_user_password))
            .route("/users/{id}/permissions", web::get().to(auth_handlers::get_user_permissions))
            .route("/users/{id}/permissions", web::put().to(auth_handlers::update_user_permissions))
            .route("/users/{id}/activity", web::get().to(auth_handlers::get_user_activity))
            .route("/jwt/status", web::get().to(handlers::get_jwt_rotation_status))
            .route("/jwt/rotate", web::post().to(handlers::force_jwt_rotation))
    );
}
