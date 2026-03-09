// src/routes/equipment.rs
use actix_web::{web, HttpRequest, HttpResponse};
use actix_multipart::Multipart;
use std::sync::Arc;
use crate::{AppState, auth, auth_handlers, audit, equipment_handlers, import_export};
use crate::models::{CreateEquipmentRequest, UpdateEquipmentRequest, CreateEquipmentPartRequest, UpdateEquipmentPartRequest, CreateMaintenanceRequest, UpdateMaintenanceRequest, CompleteMaintenanceRequest};
use crate::audit::ChangeSet;
use crate::error::ApiResult;

// ==================== PROTECTED WRAPPERS ====================

async fn create_equipment_protected(
    app_state: web::Data<Arc<AppState>>,
    equipment: web::Json<CreateEquipmentRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_equipment_permission(&http_request, auth_handlers::EquipmentAction::Create, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let user_id = claims.sub.clone();

    let mut cs = ChangeSet::new();
    cs.created("name", &equipment.name);
    cs.created("type", &equipment.type_);
    cs.created("quantity", &format!("{}", equipment.quantity));
    if let Some(ref v) = equipment.location { cs.created("location", v); }
    if let Some(ref v) = equipment.serial_number { cs.created("serial_number", v); }
    if let Some(ref v) = equipment.manufacturer { cs.created("manufacturer", v); }
    if let Some(ref v) = equipment.model { cs.created("model", v); }

    let response = equipment_handlers::create_equipment(app_state.clone(), equipment, claims.sub).await?;
    audit::audit_with_changes(&app_state.db_pool, &user_id, "create", "equipment", "", &format!("Created equipment: {}", cs.to_description()), &cs, &http_request).await;
    Ok(response)
}

async fn update_equipment_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    update_data: web::Json<UpdateEquipmentRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_equipment_permission(&http_request, auth_handlers::EquipmentAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let user_id = claims.sub.clone();
    let equipment_id = path.into_inner();

    let mut cs = ChangeSet::new();
    let mut equip_name = equipment_id.clone();

    if let Ok(old) = sqlx::query_as::<_, (
        String, i64, String, Option<String>, Option<String>,
        Option<String>, Option<String>, Option<String>
    )>(
        "SELECT name, quantity, status, location, serial_number, manufacturer, model, description FROM equipment WHERE id = ?"
    ).bind(&equipment_id).fetch_one(&app_state.db_pool).await {
        equip_name = old.0.clone();
        if let Some(ref new_val) = update_data.name { cs.add("name", &old.0, new_val); }
        if let Some(new_val) = update_data.quantity { cs.add_i64("quantity", old.1, new_val as i64); }
        if let Some(ref new_val) = update_data.status { cs.add("status", &old.2, new_val); }
        if let Some(ref new_val) = update_data.location { cs.add_opt("location", &old.3, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.serial_number { cs.add_opt("serial_number", &old.4, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.manufacturer { cs.add_opt("manufacturer", &old.5, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.model { cs.add_opt("model", &old.6, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.description { cs.add_opt("description", &old.7, &Some(new_val.clone())); }
    }

    let desc = if cs.has_changes() {
        format!("Equipment '{}' updated: {}", equip_name, cs.to_description())
    } else {
        format!("Equipment '{}' updated", equip_name)
    };

    let response = equipment_handlers::update_equipment(app_state.clone(), web::Path::from(equipment_id.clone()), update_data, claims.sub).await?;
    audit::audit_with_changes(&app_state.db_pool, &user_id, "edit", "equipment", &equipment_id, &desc, &cs, &http_request).await;
    Ok(response)
}

async fn delete_equipment_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_equipment_permission(&http_request, auth_handlers::EquipmentAction::Delete, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let equipment_id = path.into_inner();

    let mut cs = ChangeSet::new();
    if let Ok(old) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT name, type_, status FROM equipment WHERE id = ?"
    ).bind(&equipment_id).fetch_one(&app_state.db_pool).await {
        cs.deleted("name", &old.0);
        cs.deleted("type", &old.1);
        cs.deleted("status", &old.2);
    }

    let response = equipment_handlers::delete_equipment(app_state.clone(), web::Path::from(equipment_id.clone())).await?;
    audit::audit_with_changes(&app_state.db_pool, &claims.sub, "delete", "equipment", &equipment_id, &format!("Deleted equipment: {}", cs.to_description()), &cs, &http_request).await;
    Ok(response)
}

// Parts
async fn add_equipment_part_protected(app_state: web::Data<Arc<AppState>>, path: web::Path<String>, part: web::Json<CreateEquipmentPartRequest>, http_request: HttpRequest) -> ApiResult<HttpResponse> {
    let claims = auth_handlers::get_claims_from_request(&http_request)?;
    auth_handlers::check_equipment_permission(&http_request, auth_handlers::EquipmentAction::Edit, &app_state.db_pool).await?;
    equipment_handlers::add_equipment_part(app_state, path, part, claims.sub).await
}
async fn update_equipment_part_protected(app_state: web::Data<Arc<AppState>>, path: web::Path<(String, String)>, update: web::Json<UpdateEquipmentPartRequest>, http_request: HttpRequest) -> ApiResult<HttpResponse> {
    let claims = auth_handlers::get_claims_from_request(&http_request)?;
    auth_handlers::check_equipment_permission(&http_request, auth_handlers::EquipmentAction::Edit, &app_state.db_pool).await?;
    equipment_handlers::update_equipment_part(app_state, path, update, claims.sub).await
}
async fn delete_equipment_part_protected(app_state: web::Data<Arc<AppState>>, path: web::Path<(String, String)>, http_request: HttpRequest) -> ApiResult<HttpResponse> {
    auth_handlers::check_equipment_permission(&http_request, auth_handlers::EquipmentAction::Delete, &app_state.db_pool).await?;
    equipment_handlers::delete_equipment_part(app_state, path).await
}

// Maintenance
async fn create_maintenance_protected(app_state: web::Data<Arc<AppState>>, path: web::Path<String>, maintenance: web::Json<CreateMaintenanceRequest>, http_request: HttpRequest) -> ApiResult<HttpResponse> {
    let claims = auth_handlers::get_claims_from_request(&http_request)?;
    auth_handlers::check_equipment_permission(&http_request, auth_handlers::EquipmentAction::Edit, &app_state.db_pool).await?;
    equipment_handlers::create_maintenance(app_state, path, maintenance, claims.sub).await
}
async fn update_maintenance_protected(app_state: web::Data<Arc<AppState>>, path: web::Path<(String, String)>, update: web::Json<UpdateMaintenanceRequest>, http_request: HttpRequest) -> ApiResult<HttpResponse> {
    let claims = auth_handlers::get_claims_from_request(&http_request)?;
    auth_handlers::check_equipment_permission(&http_request, auth_handlers::EquipmentAction::Edit, &app_state.db_pool).await?;
    equipment_handlers::update_maintenance(app_state, path, update, claims.sub).await
}
async fn complete_maintenance_protected(app_state: web::Data<Arc<AppState>>, path: web::Path<(String, String)>, body: web::Json<CompleteMaintenanceRequest>, http_request: HttpRequest) -> ApiResult<HttpResponse> {
    let claims = auth_handlers::get_claims_from_request(&http_request)?;
    auth_handlers::check_equipment_permission(&http_request, auth_handlers::EquipmentAction::Edit, &app_state.db_pool).await?;
    equipment_handlers::complete_maintenance(app_state, path, body, claims.sub).await
}
async fn delete_maintenance_protected(app_state: web::Data<Arc<AppState>>, path: web::Path<(String, String)>, http_request: HttpRequest) -> ApiResult<HttpResponse> {
    auth_handlers::check_equipment_permission(&http_request, auth_handlers::EquipmentAction::Delete, &app_state.db_pool).await?;
    equipment_handlers::delete_maintenance(app_state, path).await
}

// Files
async fn upload_equipment_file_protected(app_state: web::Data<Arc<AppState>>, path: web::Path<String>, payload: Multipart, http_request: HttpRequest) -> ApiResult<HttpResponse> {
    let claims = auth_handlers::get_claims_from_request(&http_request)?;
    auth_handlers::check_equipment_permission(&http_request, auth_handlers::EquipmentAction::Edit, &app_state.db_pool).await?;
    equipment_handlers::upload_equipment_file(app_state, path, payload, claims.sub).await
}
async fn delete_equipment_file_protected(app_state: web::Data<Arc<AppState>>, path: web::Path<(String, String)>, http_request: HttpRequest) -> ApiResult<HttpResponse> {
    auth_handlers::check_equipment_permission(&http_request, auth_handlers::EquipmentAction::Delete, &app_state.db_pool).await?;
    equipment_handlers::delete_equipment_file(app_state, path).await
}

// ==================== ROUTES ====================

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/equipment")
            .route("", web::post().to(create_equipment_protected))
            .route("", web::get().to(equipment_handlers::get_equipment))
            .route("/search", web::get().to(equipment_handlers::search_equipment))
            .route("/export", web::get().to(import_export::export_equipment))
            .route("/import", web::post().to(import_export::import_equipment))
            .route("/import/json", web::post().to(import_export::import_equipment_json))
            .route("/import/excel", web::post().to(import_export::import_equipment_excel))
            .route("/{id}", web::get().to(equipment_handlers::get_equipment_by_id))
            .route("/{id}", web::put().to(update_equipment_protected))
            .route("/{id}", web::delete().to(delete_equipment_protected))
            .route("/{id}/parts", web::get().to(equipment_handlers::get_equipment_parts))
            .route("/{id}/parts", web::post().to(add_equipment_part_protected))
            .route("/{id}/parts/{part_id}", web::put().to(update_equipment_part_protected))
            .route("/{id}/parts/{part_id}", web::delete().to(delete_equipment_part_protected))
            .route("/{id}/parts/{part_id}/files", web::get().to(equipment_handlers::get_part_files))
            .route("/{id}/maintenance", web::get().to(equipment_handlers::get_equipment_maintenance))
            .route("/{id}/maintenance", web::post().to(create_maintenance_protected))
            .route("/{id}/maintenance/{maintenance_id}", web::put().to(update_maintenance_protected))
            .route("/{id}/maintenance/{maintenance_id}/complete", web::post().to(complete_maintenance_protected))
            .route("/{id}/maintenance/{maintenance_id}", web::delete().to(delete_maintenance_protected))
            .route("/{id}/files", web::get().to(equipment_handlers::get_equipment_files))
            .route("/{id}/files", web::post().to(upload_equipment_file_protected))
            .route("/{id}/files/{file_id}", web::get().to(equipment_handlers::download_equipment_file))
            .route("/{id}/files/{file_id}", web::delete().to(delete_equipment_file_protected))
    );
}
