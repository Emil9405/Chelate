// src/routes/experiments.rs
use actix_web::{web, HttpRequest, HttpResponse};
use std::sync::Arc;
use crate::{AppState, auth, auth_handlers, audit, experiment_handlers, filter_handlers};
use crate::audit::ChangeSet;
use crate::error::ApiResult;

// ==================== PROTECTED WRAPPERS ====================

async fn create_experiment_protected(
    app_state: web::Data<Arc<AppState>>,
    experiment: web::Json<crate::models::experiment::CreateExperimentRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_experiment_permission(&http_request, auth_handlers::ExperimentAction::Create, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let user_id = claims.sub.clone();

    let mut cs = ChangeSet::new();
    cs.created("title", &experiment.title);
    if let Some(ref desc) = experiment.description { cs.created("description", desc); }
    if let Some(ref et) = experiment.experiment_type { cs.created("experiment_type", et); }
    if let Some(ref loc) = experiment.location { cs.created("location", loc); }

    let response = experiment_handlers::create_experiment(app_state.clone(), experiment, claims.sub).await?;
    audit::audit_with_changes(&app_state.db_pool, &user_id, "create", "experiment", "", &format!("Created experiment: {}", cs.to_description()), &cs, &http_request).await;
    Ok(response)
}

async fn update_experiment_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    update_data: web::Json<crate::models::experiment::UpdateExperimentRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_experiment_permission(&http_request, auth_handlers::ExperimentAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let user_id = claims.sub.clone();
    let experiment_id = path.into_inner();

    let mut cs = ChangeSet::new();
    if let Ok(old) = sqlx::query_as::<_, (String, Option<String>, String, Option<String>)>(
        "SELECT title, description, status, location FROM experiments WHERE id = ?"
    ).bind(&experiment_id).fetch_one(&app_state.db_pool).await {
        if let Some(ref new_title) = update_data.title { cs.add("title", &old.0, new_title); }
        if let Some(ref new_desc) = update_data.description { cs.add_opt("description", &old.1, &Some(new_desc.clone())); }
        if let Some(ref new_loc) = update_data.location { cs.add_opt("location", &old.3, &Some(new_loc.clone())); }
    }

    let desc = if cs.has_changes() {
        format!("Experiment {} updated: {}", experiment_id, cs.to_description())
    } else {
        format!("Experiment {} updated", experiment_id)
    };

    let response = experiment_handlers::update_experiment(app_state.clone(), web::Path::from(experiment_id.clone()), update_data, claims.sub).await?;
    audit::audit_with_changes(&app_state.db_pool, &user_id, "edit", "experiment", &experiment_id, &desc, &cs, &http_request).await;
    Ok(response)
}

async fn delete_experiment_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_experiment_permission(&http_request, auth_handlers::ExperimentAction::Delete, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    let user_id = claims.sub.clone();
    let experiment_id = path.into_inner();

    let mut cs = ChangeSet::new();
    if let Ok(old) = sqlx::query_as::<_, (String, String)>(
        "SELECT title, status FROM experiments WHERE id = ?"
    ).bind(&experiment_id).fetch_one(&app_state.db_pool).await {
        cs.deleted("title", &old.0);
        cs.deleted("status", &old.1);
    }

    let response = experiment_handlers::delete_experiment(app_state.clone(), web::Path::from(experiment_id.clone()), claims.sub).await?;
    audit::audit_with_changes(&app_state.db_pool, &user_id, "delete", "experiment", &experiment_id, &format!("Deleted experiment: {}", cs.to_description()), &cs, &http_request).await;
    Ok(response)
}

async fn add_experiment_reagent_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    reagent: web::Json<experiment_handlers::AddReagentToExperimentRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_experiment_permission(&http_request, auth_handlers::ExperimentAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    experiment_handlers::add_reagent_to_experiment(app_state, path, reagent, claims.sub).await
}

async fn remove_experiment_reagent_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_experiment_permission(&http_request, auth_handlers::ExperimentAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    experiment_handlers::remove_reagent_from_experiment(app_state, path, claims.sub).await
}

async fn start_experiment_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_experiment_permission(&http_request, auth_handlers::ExperimentAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    experiment_handlers::start_experiment(app_state, path, claims.sub).await
}

async fn complete_experiment_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_experiment_permission(&http_request, auth_handlers::ExperimentAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    experiment_handlers::complete_experiment(app_state, path, claims.sub).await
}

async fn cancel_experiment_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_experiment_permission(&http_request, auth_handlers::ExperimentAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    experiment_handlers::cancel_experiment(app_state, path, claims.sub).await
}

async fn consume_experiment_reagent_protected(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    auth_handlers::check_experiment_permission(&http_request, auth_handlers::ExperimentAction::Edit, &app_state.db_pool).await?;
    let claims = auth::get_current_user(&http_request)?;
    experiment_handlers::consume_experiment_reagent(app_state, path, claims.sub).await
}

// ==================== ROUTES ====================

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/experiments")
            .route("", web::post().to(create_experiment_protected))
            .route("", web::get().to(experiment_handlers::get_all_experiments))
            .route("/stats", web::get().to(experiment_handlers::get_experiment_stats))
            .route("/filter", web::post().to(filter_handlers::get_experiments_filtered))
            .route("/auto-update-statuses", web::post().to(experiment_handlers::auto_update_experiment_statuses))
            .route("/diagnose-dates", web::get().to(experiment_handlers::diagnose_experiment_dates))
            .route("/{id}", web::get().to(experiment_handlers::get_experiment))
            .route("/{id}", web::put().to(update_experiment_protected))
            .route("/{id}", web::delete().to(delete_experiment_protected))
            .route("/{id}/start", web::post().to(start_experiment_protected))
            .route("/{id}/complete", web::post().to(complete_experiment_protected))
            .route("/{id}/cancel", web::post().to(cancel_experiment_protected))
            .route("/{id}/reagents", web::get().to(experiment_handlers::get_experiment_reagents))
            .route("/{id}/reagents", web::post().to(add_experiment_reagent_protected))
            .route("/{id}/reagents/{reagent_id}", web::delete().to(remove_experiment_reagent_protected))
            .route("/{id}/reagents/{reagent_id}/consume", web::post().to(consume_experiment_reagent_protected))
    );
}
