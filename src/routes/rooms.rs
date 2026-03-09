// src/routes/rooms.rs
use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;
use crate::{AppState, auth, auth_handlers, audit, room_handlers, container_handlers, placement_handlers};
use crate::audit::ChangeSet;
use crate::error::ApiResult;

// ==================== PROTECTED WRAPPERS ====================

async fn create_room_protected(
    app_state: web::Data<Arc<AppState>>,
    room: web::Json<crate::models::room::CreateRoomRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_room_permission(&http_request, auth_handlers::RoomAction::Create, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let user_id = claims.sub.clone();

    let mut cs = ChangeSet::new();
    cs.created("name", &room.name);
    if let Some(ref v) = room.description { cs.created("description", v); }
    if let Some(v) = room.capacity { cs.created("capacity", &format!("{}", v)); }

    let response = room_handlers::create_room(app_state.clone(), room, claims.sub).await?;
    audit::audit_with_changes(&app_state.db_pool, &user_id, "create", "room", "", &format!("Created room: {}", cs.to_description()), &cs, &http_request).await;
    Ok(response)
}

async fn update_room_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    update_data: web::Json<crate::models::room::UpdateRoomRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_room_permission(&http_request, auth_handlers::RoomAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let user_id = claims.sub.clone();
    let room_id = path.into_inner();

    let mut cs = ChangeSet::new();
    let mut room_name = room_id.clone();

    if let Ok(old) = sqlx::query_as::<_, (String, Option<String>, Option<i64>, String)>(
        "SELECT name, description, capacity, status FROM rooms WHERE id = ?"
    ).bind(&room_id).fetch_one(&app_state.db_pool).await {
        room_name = old.0.clone();
        if let Some(ref new_val) = update_data.name { cs.add("name", &old.0, new_val); }
        if let Some(ref new_val) = update_data.description { cs.add_opt("description", &old.1, &Some(new_val.clone())); }
        if let Some(new_val) = update_data.capacity { cs.add_i64("capacity", old.2.unwrap_or(0), new_val as i64); }
    }

    let desc = if cs.has_changes() {
        format!("Room '{}' updated: {}", room_name, cs.to_description())
    } else {
        format!("Room '{}' updated", room_name)
    };

    let response = room_handlers::update_room(app_state.clone(), web::Path::from(room_id.clone()), update_data, claims.sub).await?;
    audit::audit_with_changes(&app_state.db_pool, &user_id, "edit", "room", &room_id, &desc, &cs, &http_request).await;
    Ok(response)
}

async fn delete_room_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_room_permission(&http_request, auth_handlers::RoomAction::Delete, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let room_id = path.into_inner();

    let mut cs = ChangeSet::new();
    if let Ok(old) = sqlx::query_as::<_, (String, String)>(
        "SELECT name, status FROM rooms WHERE id = ?"
    ).bind(&room_id).fetch_one(&app_state.db_pool).await {
        cs.deleted("name", &old.0);
        cs.deleted("status", &old.1);
    }

    let response = room_handlers::delete_room(app_state.clone(), web::Path::from(room_id.clone())).await?;
    audit::audit_with_changes(&app_state.db_pool, &claims.sub, "delete", "room", &room_id, &format!("Deleted room: {}", cs.to_description()), &cs, &http_request).await;
    Ok(response)
}

// ==================== ROUTES ====================

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/rooms")
            .route("", web::get().to(room_handlers::get_all_rooms))
            .route("", web::post().to(create_room_protected))
            .route("/available", web::get().to(room_handlers::get_available_rooms))
            .route("/{id}", web::get().to(room_handlers::get_room))
            .route("/{id}", web::put().to(update_room_protected))
            .route("/{id}", web::delete().to(delete_room_protected))
            .route("/{id}/inventory", web::get().to(container_handlers::get_room_inventory))
            .route("/{id}/placements", web::get().to(placement_handlers::get_room_placements))
    );
}
