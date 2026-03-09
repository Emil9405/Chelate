// src/routes/batches.rs
use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;
use crate::{AppState, auth, auth_handlers, audit, batch_handlers, import_export, filter_handlers, container_handlers, placement_handlers};
use crate::audit::ChangeSet;
use crate::error::ApiResult;

// ==================== PROTECTED WRAPPERS (pub for cross-module use) ====================

pub async fn create_batch_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    batch: web::Json<crate::models::batch::CreateBatchRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, auth_handlers::BatchAction::Create, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let user_id = claims.sub.clone();
    let reagent_id = path.into_inner();

    let reagent_name = sqlx::query_as::<_, (String,)>("SELECT name FROM reagents WHERE id = ?")
        .bind(&reagent_id).fetch_optional(&app_state.db_pool).await
        .ok().flatten().map(|r| r.0).unwrap_or_else(|| reagent_id.clone());

    let mut cs = ChangeSet::new();
    cs.created("reagent", &reagent_name);
    cs.created("batch_number", &batch.batch_number);
    cs.created("quantity", &format!("{} {}", batch.quantity, batch.unit));
    if let Some(ref v) = batch.lot_number { cs.created("lot_number", v); }
    if let Some(ref v) = batch.supplier { cs.created("supplier", v); }
    if let Some(ref v) = batch.location { cs.created("location", v); }
    if let Some(ref v) = batch.cat_number { cs.created("cat_number", v); }
    if let Some(ref v) = batch.expiry_date { cs.created("expiry_date", &v.to_string()); }

    let response = batch_handlers::create_batch(app_state.clone(), web::Path::from(reagent_id.clone()), batch, claims.sub).await?;
    audit::audit_with_changes(&app_state.db_pool, &user_id, "create", "batch", "", &format!("Created batch for '{}': {}", reagent_name, cs.to_description()), &cs, &http_request).await;
    Ok(response)
}

pub async fn update_batch_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    update_data: web::Json<crate::models::batch::UpdateBatchRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, auth_handlers::BatchAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let user_id = claims.sub.clone();
    let (reagent_id, batch_id) = path.into_inner();

    let mut cs = ChangeSet::new();
    let mut batch_label = batch_id.clone();

    if let Ok(old) = sqlx::query_as::<_, (
        String, f64, String, String, Option<String>, Option<String>,
        Option<String>, Option<String>, Option<String>, Option<String>, Option<String>
    )>(
        "SELECT batch_number, quantity, unit, status, \
         lot_number, location, supplier, manufacturer, expiry_date, notes, cat_number \
         FROM batches WHERE id = ? AND reagent_id = ?"
    ).bind(&batch_id).bind(&reagent_id).fetch_one(&app_state.db_pool).await {
        batch_label = old.0.clone();
        if let Some(ref new_val) = update_data.batch_number { cs.add("batch_number", &old.0, new_val); }
        if let Some(new_val) = update_data.quantity { cs.add_f64("quantity", old.1, new_val); }
        if let Some(ref new_val) = update_data.unit { cs.add("unit", &old.2, new_val); }
        if let Some(ref new_val) = update_data.status { cs.add("status", &old.3, new_val); }
        if let Some(ref new_val) = update_data.lot_number { cs.add_opt("lot_number", &old.4, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.location { cs.add_opt("location", &old.5, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.supplier { cs.add_opt("supplier", &old.6, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.manufacturer { cs.add_opt("manufacturer", &old.7, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.expiry_date { cs.add_opt("expiry_date", &old.8, &Some(new_val.to_string())); }
        if let Some(ref new_val) = update_data.notes { cs.add_opt("notes", &old.9, &Some(new_val.clone())); }
        if let Some(ref new_val) = update_data.cat_number { cs.add_opt("cat_number", &old.10, &Some(new_val.clone())); }
    }

    let reagent_name = sqlx::query_as::<_, (String,)>("SELECT name FROM reagents WHERE id = ?")
        .bind(&reagent_id).fetch_optional(&app_state.db_pool).await
        .ok().flatten().map(|r| r.0).unwrap_or_else(|| reagent_id.clone());

    let desc = if cs.has_changes() {
        format!("Batch {} of reagent '{}' updated: {}", batch_label, reagent_name, cs.to_description())
    } else {
        format!("Batch {} of reagent '{}' updated", batch_label, reagent_name)
    };

    let response = batch_handlers::update_batch(app_state.clone(), web::Path::from((reagent_id.clone(), batch_id.clone())), update_data, claims.sub).await?;
    audit::audit_with_changes(&app_state.db_pool, &user_id, "edit", "batch", &batch_id, &desc, &cs, &http_request).await;
    Ok(response)
}

pub async fn delete_batch_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, auth_handlers::BatchAction::Delete, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let user_id = claims.sub.clone();
    let (reagent_id, batch_id) = path.into_inner();

    let mut cs = ChangeSet::new();
    let reagent_name = sqlx::query_as::<_, (String,)>("SELECT name FROM reagents WHERE id = ?")
        .bind(&reagent_id).fetch_optional(&app_state.db_pool).await
        .ok().flatten().map(|r| r.0).unwrap_or_else(|| reagent_id.clone());

    if let Ok(old) = sqlx::query_as::<_, (String, f64, String, String, Option<String>)>(
        "SELECT batch_number, quantity, unit, status, lot_number FROM batches WHERE id = ? AND reagent_id = ?"
    ).bind(&batch_id).bind(&reagent_id).fetch_one(&app_state.db_pool).await {
        cs.deleted("batch_number", &old.0);
        cs.deleted("quantity", &format!("{} {}", old.1, old.2));
        cs.deleted("status", &old.3);
        if let Some(ref lot) = old.4 { cs.deleted("lot_number", lot); }
    }

    let response = batch_handlers::delete_batch(app_state.clone(), web::Path::from((reagent_id.clone(), batch_id.clone())), claims.sub).await?;
    audit::audit_with_changes(&app_state.db_pool, &user_id, "delete", "batch", &batch_id, &format!("Deleted batch of reagent '{}': {}", reagent_name, cs.to_description()), &cs, &http_request).await;
    Ok(response)
}

async fn split_batch_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<crate::models::batch_container::SplitBatchRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, auth_handlers::BatchAction::Edit, &app_state.db_pool).await?;
    container_handlers::split_batch_into_containers(app_state, path, request, http_request).await
}

async fn create_container_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<crate::models::batch_container::CreateContainerRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_batch_permission_async(&http_request, auth_handlers::BatchAction::Edit, &app_state.db_pool).await?;
    container_handlers::create_container(app_state, path, request, http_request).await
}

// ==================== ROUTES ====================

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/batches")
            .route("/filter", web::post().to(filter_handlers::get_batches_filtered))
            .route("/preset/{preset}", web::get().to(filter_handlers::get_batches_by_preset))
            .route("", web::get().to(batch_handlers::get_all_batches))
            .route("/low-stock", web::get().to(batch_handlers::get_low_stock_batches))
            .route("/expiring", web::get().to(batch_handlers::get_expiring_batches))
            .route("/export", web::get().to(import_export::export_batches))
            .route("/import", web::post().to(import_export::import_batches))
            .route("/import/json", web::post().to(import_export::import_batches_json))
            .route("/import/excel", web::post().to(import_export::import_batches_excel))
            .route("/{batch_id}/containers", web::get().to(container_handlers::get_batch_containers))
            .route("/{batch_id}/containers", web::post().to(create_container_protected))
            .route("/{batch_id}/containers/split", web::post().to(split_batch_protected))
            .route("/{batch_id}/placements", web::get().to(placement_handlers::get_batch_placements))
    );
}
