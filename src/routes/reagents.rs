// src/routes/reagents.rs
use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;
use crate::{AppState, auth, auth_handlers, audit, reagent_handlers, handlers, import_export};
use crate::audit::ChangeSet;
use crate::error::ApiResult;

// ==================== PROTECTED WRAPPERS ====================

async fn create_reagent_protected(
    app_state: web::Data<Arc<AppState>>,
    reagent: web::Json<crate::models::reagent::CreateReagentRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_reagent_permission_async(&http_request, auth_handlers::ReagentAction::Create, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let user_id = claims.sub.clone();

    let mut cs = ChangeSet::new();
    cs.created("name", &reagent.name);
    if let Some(ref v) = reagent.formula { cs.created("formula", v); }
    if let Some(ref v) = reagent.cas_number { cs.created("cas_number", v); }
    if let Some(ref v) = reagent.manufacturer { cs.created("manufacturer", v); }
    if let Some(ref v) = reagent.physical_state { cs.created("physical_state", v); }
    if let Some(ref v) = reagent.hazard_pictograms { cs.created("hazard_pictograms", v); }
    if let Some(ref v) = reagent.storage_conditions { cs.created("storage_conditions", v); }

    let response = reagent_handlers::create_reagent(app_state.clone(), reagent, claims.sub).await?;
    audit::audit_with_changes(&app_state.db_pool, &user_id, "create", "reagent", "", &format!("Created reagent: {}", cs.to_description()), &cs, &http_request).await;
    Ok(response)
}

async fn update_reagent_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    update_data: web::Json<crate::models::reagent::UpdateReagentRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_reagent_permission_async(&http_request, auth_handlers::ReagentAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let user_id = claims.sub.clone();
    let reagent_id = path.into_inner();

    let mut cs = ChangeSet::new();
    let mut reagent_name = reagent_id.clone();

    if let Ok(old) = sqlx::query_as::<_, (
        String, Option<String>, Option<f64>, Option<String>, Option<String>,
        Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, String
    )>(
        "SELECT name, formula, molecular_weight, physical_state, cas_number, \
         manufacturer, description, storage_conditions, appearance, hazard_pictograms, status \
         FROM reagents WHERE id = ?"
    ).bind(&reagent_id).fetch_one(&app_state.db_pool).await {
        reagent_name = old.0.clone();
        if let Some(ref new_val) = update_data.name { cs.add("name", &old.0, new_val); }
        if let Some(ref new_val) = update_data.formula { cs.add_opt("formula", &old.1, &Some(new_val.clone())); }
        if let Some(new_val) = update_data.molecular_weight { cs.add_opt_f64("molecular_weight", old.2, Some(new_val)); }
        if let Some(ref new_val) = update_data.physical_state { cs.add_opt("physical_state", &old.3, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.cas_number { cs.add_opt("cas_number", &old.4, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.manufacturer { cs.add_opt("manufacturer", &old.5, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.description { cs.add_opt("description", &old.6, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.storage_conditions { cs.add_opt("storage_conditions", &old.7, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.appearance { cs.add_opt("appearance", &old.8, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.hazard_pictograms { cs.add_opt("hazard_pictograms", &old.9, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.status { cs.add("status", &old.10, new_val); }
    }

    let desc = if cs.has_changes() {
        format!("Reagent '{}' updated: {}", reagent_name, cs.to_description())
    } else {
        format!("Reagent '{}' updated", reagent_name)
    };

    let response = reagent_handlers::update_reagent(app_state.clone(), web::Path::from(reagent_id.clone()), update_data, claims.sub).await?;
    audit::audit_with_changes(&app_state.db_pool, &user_id, "edit", "reagent", &reagent_id, &desc, &cs, &http_request).await;
    Ok(response)
}

async fn delete_reagent_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_reagent_permission_async(&http_request, auth_handlers::ReagentAction::Delete, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let reagent_id = path.into_inner();

    let mut cs = ChangeSet::new();
    if let Ok(old) = sqlx::query_as::<_, (String, Option<String>, Option<String>, String)>(
        "SELECT name, cas_number, manufacturer, status FROM reagents WHERE id = ? AND deleted_at IS NULL"
    ).bind(&reagent_id).fetch_one(&app_state.db_pool).await {
        cs.deleted("name", &old.0);
        if let Some(ref cas) = old.1 { cs.deleted("cas_number", cas); }
        if let Some(ref mfr) = old.2 { cs.deleted("manufacturer", mfr); }
        cs.deleted("status", &old.3);
    }

    let response = reagent_handlers::delete_reagent(app_state.clone(), web::Path::from(reagent_id.clone()), claims.sub.clone()).await?;
    audit::audit_with_changes(&app_state.db_pool, &claims.sub, "delete", "reagent", &reagent_id, &format!("Deleted reagent: {}", cs.to_description()), &cs, &http_request).await;
    Ok(response)
}

// ==================== ROUTES ====================

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/reagents")
            .route("", web::post().to(create_reagent_protected))
            .route("", web::get().to(reagent_handlers::get_reagents))
            .route("/search", web::get().to(reagent_handlers::search_reagents))
            .route("/export", web::get().to(import_export::export_reagents))
            .route("/import", web::post().to(import_export::import_reagents))
            .route("/import/json", web::post().to(import_export::import_reagents_json))
            .route("/import/excel", web::post().to(import_export::import_reagents_excel))
            .route("/{id}", web::get().to(reagent_handlers::get_reagent_by_id))
            .route("/{id}", web::put().to(update_reagent_protected))
            .route("/{id}", web::delete().to(delete_reagent_protected))
            .route("/{id}/details", web::get().to(handlers::get_reagent_with_batches))
            .route("/{id}/batches", web::get().to(crate::batch_handlers::get_batches_for_reagent))
            .route("/{id}/batches", web::post().to(super::batches::create_batch_protected))
            .route("/{reagent_id}/batches/{batch_id}", web::get().to(crate::batch_handlers::get_batch))
            .route("/{reagent_id}/batches/{batch_id}", web::put().to(super::batches::update_batch_protected))
            .route("/{reagent_id}/batches/{batch_id}", web::delete().to(super::batches::delete_batch_protected))
            .route("/{reagent_id}/batches/{batch_id}/use", web::post().to(handlers::use_reagent))
            .route("/{reagent_id}/batches/{batch_id}/usage", web::get().to(handlers::get_usage_history))
            .route("/{reagent_id}/batches/{batch_id}/dispense-units", web::post().to(crate::batch_handlers::dispense_units))
            .route("/{reagent_id}/batches/{batch_id}/units-info", web::get().to(crate::batch_handlers::get_batch_units_info))
    );
}
