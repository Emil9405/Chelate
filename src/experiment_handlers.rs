// src/experiment_handlers.rs
//! Обработчики для экспериментов (v2.1)

use actix_web::{web, HttpResponse};
use actix_files::NamedFile;
use std::sync::Arc;
use std::path::PathBuf;
use crate::AppState;
use crate::models::*;
use crate::error::{ApiError, ApiResult};
use crate::handlers::{ApiResponse, PaginatedResponse};
use chrono::Utc;
use uuid::Uuid;
use validator::Validate;
use log::info;
use serde::{Deserialize, Serialize};

// ==================== QUERY STRUCTS ====================

/// Query parameters for listing experiments
#[derive(Debug, Deserialize)]
pub struct ExperimentQuery {
    pub search: Option<String>,
    pub status: Option<String>,
    pub experiment_type: Option<String>,
    pub location: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub sort_order: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl ExperimentQuery {
    /// Normalize pagination parameters and return (page, per_page, offset)
    pub fn normalize(&self) -> (i64, i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(20).clamp(1, 100);
        let offset = (page - 1) * per_page;
        (page, per_page, offset)
    }
}

// ==================== EXPERIMENT STATS ====================

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExperimentStats {
    pub total: i64,
    pub planned: i64,
    pub in_progress: i64,
    pub completed: i64,
    pub cancelled: i64,
    pub educational: i64,
    pub research: i64,
}

// ==================== EXPERIMENT CRUD ====================

pub async fn get_all_experiments(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<ExperimentQuery>,
) -> ApiResult<HttpResponse> {
    let (page, per_page, offset) = query.normalize();
    
    let mut conditions: Vec<String> = vec!["1=1".to_string()];
    let mut params: Vec<String> = Vec::new();

    // Поиск
    if let Some(ref search) = query.search {
        if !search.trim().is_empty() {
            let pattern = format!("%{}%", search.trim());
            conditions.push("(title LIKE ? OR description LIKE ? OR instructor LIKE ? OR student_group LIKE ?)".to_string());
            params.push(pattern.clone());
            params.push(pattern.clone());
            params.push(pattern.clone());
            params.push(pattern);
        }
    }
    
    // Фильтры
    if let Some(ref status) = query.status {
        conditions.push("status = ?".to_string());
        params.push(status.clone());
    }
    if let Some(ref exp_type) = query.experiment_type {
        conditions.push("experiment_type = ?".to_string());
        params.push(exp_type.clone());
    }
    if let Some(ref location) = query.location {
        conditions.push("location = ?".to_string());
        params.push(location.clone());
    }
    if let Some(ref date_from) = query.date_from {
        conditions.push("experiment_date >= ?".to_string());
        params.push(date_from.clone());
    }
    if let Some(ref date_to) = query.date_to {
        conditions.push("experiment_date <= ?".to_string());
        params.push(date_to.clone());
    }

    let where_clause = conditions.join(" AND ");
    let sort_order = query.sort_order.as_deref().unwrap_or("DESC");

    // Подсчёт
    let count_sql = format!("SELECT COUNT(*) as count FROM experiments WHERE {}", where_clause);
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    for p in &params {
        count_query = count_query.bind(p);
    }
    let total: i64 = count_query.fetch_one(&app_state.db_pool).await?;

    // Выборка данных
    let sql = format!(
        "SELECT * FROM experiments WHERE {} ORDER BY experiment_date {} LIMIT ? OFFSET ?",
        where_clause, sort_order
    );
    let mut select_query = sqlx::query_as::<_, Experiment>(&sql);
    for p in &params {
        select_query = select_query.bind(p);
    }
    select_query = select_query.bind(per_page).bind(offset);
    let experiments: Vec<Experiment> = select_query.fetch_all(&app_state.db_pool).await?;

    let total_pages = (total + per_page - 1) / per_page;
    Ok(HttpResponse::Ok().json(ApiResponse::success(PaginatedResponse { 
        data: experiments, total, page, per_page, total_pages 
    })))
}

pub async fn get_experiment(
    app_state: web::Data<Arc<AppState>>, 
    path: web::Path<String>
) -> ApiResult<HttpResponse> {
    let experiment_id = path.into_inner();
    let experiment: Option<Experiment> = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_optional(&app_state.db_pool)
        .await?;
    match experiment {
        Some(exp) => Ok(HttpResponse::Ok().json(ApiResponse::success(exp))),
        None => Err(ApiError::not_found("Experiment")),
    }
}

pub async fn create_experiment(
    app_state: web::Data<Arc<AppState>>, 
    experiment: web::Json<CreateExperimentRequest>, 
    user_id: String
) -> ApiResult<HttpResponse> {
    experiment.validate()?;
    experiment.validate_educational().map_err(|e| ApiError::bad_request(&e))?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let exp_date = experiment.experiment_date.unwrap_or(now);
    let start_date = experiment.start_date.unwrap_or(exp_date);

    sqlx::query(r#"
        INSERT INTO experiments 
        (id, title, description, experiment_date, experiment_type, 
         instructor, student_group, location, protocol, start_date, end_date, notes,
         status, created_by, updated_by, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'planned', ?, ?, ?, ?)
    "#)
        .bind(&id)
        .bind(&experiment.title)
        .bind(&experiment.description)
        .bind(&exp_date)
        .bind(&experiment.experiment_type)
        .bind(&experiment.instructor)
        .bind(&experiment.student_group)
        .bind(&experiment.location)
        .bind(&experiment.protocol)
        .bind(&start_date)
        .bind(&experiment.end_date)
        .bind(&experiment.notes)
        .bind(&user_id)
        .bind(&user_id)
        .bind(&now)
        .bind(&now)
        .execute(&app_state.db_pool)
        .await?;

    let created: Experiment = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&id)
        .fetch_one(&app_state.db_pool)
        .await?;

    info!("User {} created experiment: {}", user_id, id);
    Ok(HttpResponse::Created().json(ApiResponse::success(created)))
}

pub async fn update_experiment(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    update: web::Json<UpdateExperimentRequest>,
    user_id: String,
    http_request: &actix_web::HttpRequest,
) -> ApiResult<HttpResponse> {
    update.validate()?;
    let experiment_id = path.into_inner();

    let existing: Option<Experiment> = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_optional(&app_state.db_pool)
        .await?;

    if existing.is_none() {
        return Err(ApiError::not_found("Experiment"));
    }
    let existing = existing.unwrap();
    let now = Utc::now();
    
    // Подготовка данных
    let title = update.title.as_ref().unwrap_or(&existing.title);
    let description = update.description.clone().or(existing.description.clone());
    let experiment_date = update.experiment_date.unwrap_or(existing.experiment_date);
    let experiment_type = update.experiment_type.clone().or(existing.experiment_type.clone());
    let instructor = update.instructor.clone().or(existing.instructor.clone());
    let student_group = update.student_group.clone().or(existing.student_group.clone());
    let status = update.status.as_ref().unwrap_or(&existing.status);
    let location = update.location.clone().or(existing.location.clone());
    let room_id = update.room_id.clone().or(existing.room_id.clone());
    let protocol = update.protocol.clone().or(existing.protocol.clone());
    let results = update.results.clone().or(existing.results.clone());
    let notes = update.notes.clone().or(existing.notes.clone());
    let start_date = update.start_date.unwrap_or(existing.start_date);
    let end_date = update.end_date.or(existing.end_date);

    // === ЖЕЛЕЗОБЕТОННОЕ АВТО-СПИСАНИЕ (в единой транзакции с обновлением) ===
    let mut tx = app_state.db_pool.begin().await?;

    let mut consumed: Vec<crate::consumption::ConsumeResult> = Vec::new();
    if status == "completed" && existing.status != "completed" {
        // Write off all pending reagents through the shared consumption path.
        // Any failure (e.g. not enough stock) aborts the whole update.
        consumed = crate::consumption::consume_experiment(
            &mut *tx, &experiment_id, title, Some(&user_id),
        ).await?;
    } else if status == "cancelled" && existing.status != "cancelled" {
        let reagents: Vec<ExperimentReagent> = sqlx::query_as(r#"
            SELECT id, experiment_id, batch_id, planned_quantity, is_consumed, notes, created_at
            FROM experiment_reagents 
            WHERE experiment_id = ?
        "#)
            .bind(&experiment_id)
            .fetch_all(&mut *tx)
            .await?;

        for reagent in reagents {
            if !reagent.is_consumed {
                let qty = reagent.planned_quantity.unwrap_or(0.0);
                if qty > 0.0 {
                    sqlx::query(r#"
                        UPDATE batches 
                        SET reserved_quantity = MAX(0, reserved_quantity - ?)
                        WHERE id = ?
                    "#)
                        .bind(qty)
                        .bind(&reagent.batch_id)
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }
    }

    sqlx::query(r#"
        UPDATE experiments SET 
        title = ?, description = ?, experiment_date = ?, experiment_type = ?, 
        instructor = ?, student_group = ?, status = ?, location = ?, room_id = ?,
        protocol = ?, start_date = ?, end_date = ?, results = ?, notes = ?,
        updated_by = ?, updated_at = ?
        WHERE id = ?
    "#)
        .bind(title)
        .bind(&description)
        .bind(&experiment_date)
        .bind(&experiment_type)
        .bind(&instructor)
        .bind(&student_group)
        .bind(status)
        .bind(&location)
        .bind(&room_id)
        .bind(&protocol)
        .bind(&start_date)
        .bind(&end_date)
        .bind(&results)
        .bind(&notes)
        .bind(&user_id)
        .bind(&now)
        .bind(&experiment_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    audit_experiment_consumption(&app_state.db_pool, &user_id, &consumed, title, http_request).await;

    let updated: Experiment = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    info!("User {} updated experiment: {}", user_id, experiment_id);
    Ok(HttpResponse::Ok().json(ApiResponse::success(updated)))
}

pub async fn delete_experiment(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let experiment_id = path.into_inner();

    let reagents: Vec<ExperimentReagent> = sqlx::query_as(r#"
        SELECT id, experiment_id, batch_id, planned_quantity, is_consumed, notes, created_at
        FROM experiment_reagents 
        WHERE experiment_id = ? AND is_consumed = 0
    "#)
        .bind(&experiment_id)
        .fetch_all(&app_state.db_pool)
        .await?;

    let mut tx = app_state.db_pool.begin().await?;

    for reagent in &reagents {
        let qty = reagent.planned_quantity.unwrap_or(0.0);
        if qty > 0.0 {
            sqlx::query("UPDATE batches SET reserved_quantity = MAX(0, reserved_quantity - ?) WHERE id = ?")
                .bind(qty)
                .bind(&reagent.batch_id)
                .execute(&mut *tx)
                .await?;
        }
    }

    sqlx::query("DELETE FROM experiment_reagents WHERE experiment_id = ?")
        .bind(&experiment_id)
        .execute(&mut *tx)
        .await?;

    let result = sqlx::query("DELETE FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .execute(&mut *tx)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Experiment"));
    }

    tx.commit().await?;

    info!("User {} deleted experiment: {}", user_id, experiment_id);
    Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "message": "Experiment deleted successfully"
    }))))
}

// ==================== EXPERIMENT STATUS ====================

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

pub async fn update_experiment_status(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    body: web::Json<UpdateStatusRequest>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let experiment_id = path.into_inner();
    let now = Utc::now();

    let valid_statuses = ["planned", "in_progress", "completed", "cancelled"];
    if !valid_statuses.contains(&body.status.as_str()) {
        return Err(ApiError::bad_request(&format!(
            "Invalid status. Must be one of: {}", valid_statuses.join(", ")
        )));
    }

    let result = sqlx::query(
        "UPDATE experiments SET status = ?, updated_by = ?, updated_at = ? WHERE id = ?"
    )
        .bind(&body.status)
        .bind(&user_id)
        .bind(&now)
        .bind(&experiment_id)
        .execute(&app_state.db_pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("Experiment"));
    }

    let updated: Experiment = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    info!("User {} changed experiment {} status to {}", user_id, experiment_id, body.status);
    Ok(HttpResponse::Ok().json(ApiResponse::success(updated)))
}

// ==================== EXPERIMENT STATISTICS ====================

pub async fn get_experiment_stats(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let stats: ExperimentStats = sqlx::query_as(r#"
        SELECT 
            COUNT(*) as total,
            SUM(CASE WHEN status = 'planned' THEN 1 ELSE 0 END) as planned,
            SUM(CASE WHEN status = 'in_progress' THEN 1 ELSE 0 END) as in_progress,
            SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
            SUM(CASE WHEN status = 'cancelled' THEN 1 ELSE 0 END) as cancelled,
            SUM(CASE WHEN experiment_type = 'educational' THEN 1 ELSE 0 END) as educational,
            SUM(CASE WHEN experiment_type = 'research' THEN 1 ELSE 0 END) as research
        FROM experiments
    "#)
        .fetch_one(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(stats)))
}

// ==================== EXPERIMENT REAGENTS ====================

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExperimentReagent {
    pub id: String,
    pub experiment_id: String,
    pub batch_id: String,
    pub planned_quantity: Option<f64>,
    pub is_consumed: bool,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExperimentReagentWithDetails {
    pub id: String,
    pub experiment_id: String,
    pub batch_id: String,
    pub quantity_used: Option<f64>,
    pub is_consumed: bool,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    // Batch details
    pub batch_number: String,
    pub unit: String,
    pub available_quantity: f64,
    // Reagent details
    pub reagent_id: String,
    pub reagent_name: String,
}

pub async fn get_experiment_reagents(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let experiment_id = path.into_inner();

    // Check experiment exists
    let _: Experiment = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Experiment"))?;

    let reagents: Vec<ExperimentReagentWithDetails> = sqlx::query_as(r#"
        SELECT 
            er.id, er.experiment_id, er.batch_id, 
            er.planned_quantity as quantity_used, er.is_consumed, er.notes, er.created_at,
            b.batch_number, b.unit, b.quantity as available_quantity,
            b.reagent_id, r.name as reagent_name
        FROM experiment_reagents er
        JOIN batches b ON er.batch_id = b.id
        JOIN reagents r ON b.reagent_id = r.id
        WHERE er.experiment_id = ?
        ORDER BY er.created_at DESC
    "#)
        .bind(&experiment_id)
        .fetch_all(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(reagents)))
}

#[derive(Debug, Deserialize, Validate)]
pub struct AddReagentToExperimentRequest {
    pub batch_id: String,
    #[validate(range(min = 0.001, message = "Quantity must be positive"))]
    pub quantity_used: f64,
    pub notes: Option<String>,
}

pub async fn add_reagent_to_experiment(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    body: web::Json<AddReagentToExperimentRequest>,
    _user_id: String,
) -> ApiResult<HttpResponse> {
    body.validate()?;
    let experiment_id = path.into_inner();

    // Check experiment exists and is modifiable
    let experiment: Experiment = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Experiment"))?;

    if !["planned", "in_progress"].contains(&experiment.status.as_str()) {
        return Err(ApiError::bad_request("Cannot add reagents to completed or cancelled experiment"));
    }

    #[derive(sqlx::FromRow)]
    struct BatchInfo {
        reagent_id: String,
        unit: String,
        quantity: f64,
        reserved_quantity: f64,
    }

    // Check batch exists and has enough quantity
    let batch: BatchInfo = sqlx::query_as("SELECT reagent_id, unit, quantity, reserved_quantity FROM batches WHERE id = ?")
        .bind(&body.batch_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Batch"))?;

    let available = batch.quantity - batch.reserved_quantity;
    if body.quantity_used > available {
        return Err(ApiError::insufficient_quantity(available, body.quantity_used));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let mut tx = app_state.db_pool.begin().await?;

    // Add reagent to experiment
sqlx::query(r#"
        INSERT INTO experiment_reagents (
            id, experiment_id, reagent_id, batch_id, 
            planned_quantity, unit, notes, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    "#)
        .bind(&id)
        .bind(&experiment_id)
        .bind(&batch.reagent_id)
        .bind(&body.batch_id)
        .bind(body.quantity_used)
        .bind(&batch.unit)
        .bind(&body.notes)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

    // Reserve quantity in batch
    sqlx::query("UPDATE batches SET reserved_quantity = reserved_quantity + ? WHERE id = ?")
        .bind(body.quantity_used)
        .bind(&body.batch_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::Created().json(ApiResponse::success(serde_json::json!({
        "id": id,
        "message": "Reagent added to experiment"
    }))))
}

pub async fn remove_reagent_from_experiment(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    _user_id: String,
) -> ApiResult<HttpResponse> {
    let (experiment_id, reagent_link_id) = path.into_inner();

    #[derive(sqlx::FromRow)]
    struct ReagentLink {
        batch_id: String,
        planned_quantity: Option<f64>,
        is_consumed: bool,
    }

    let link: ReagentLink = sqlx::query_as(
        "SELECT batch_id, planned_quantity, is_consumed FROM experiment_reagents WHERE id = ? AND experiment_id = ?"
    )
        .bind(&reagent_link_id)
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Experiment reagent link"))?;

    if link.is_consumed {
        return Err(ApiError::bad_request("Cannot remove already consumed reagent"));
    }

    let mut tx = app_state.db_pool.begin().await?;

    // Remove link
    sqlx::query("DELETE FROM experiment_reagents WHERE id = ?")
        .bind(&reagent_link_id)
        .execute(&mut *tx)
        .await?;

    // Unreserve quantity
    let qty = link.planned_quantity.unwrap_or(0.0);
    sqlx::query("UPDATE batches SET reserved_quantity = MAX(0, reserved_quantity - ?) WHERE id = ?")
        .bind(qty)
        .bind(&link.batch_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "message": "Reagent removed from experiment"
    }))))
}

// ==================== START/COMPLETE/CANCEL EXPERIMENT ====================

/// Запустить эксперимент (planned -> in_progress)
pub async fn start_experiment(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let experiment_id = path.into_inner();
    let now = Utc::now();

    let existing: Experiment = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Experiment"))?;

    if existing.status != "planned" {
        return Err(ApiError::bad_request(&format!(
            "Cannot start experiment with status '{}'. Only 'planned' experiments can be started.",
            existing.status
        )));
    }

    sqlx::query(r#"
        UPDATE experiments 
        SET status = 'in_progress', start_date = COALESCE(start_date, ?), updated_by = ?, updated_at = ?
        WHERE id = ?
    "#)
        .bind(&now)
        .bind(&user_id)
        .bind(&now)
        .bind(&experiment_id)
        .execute(&app_state.db_pool)
        .await?;

    let updated: Experiment = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    info!("User {} started experiment: {}", user_id, experiment_id);
    Ok(HttpResponse::Ok().json(ApiResponse::success(updated)))
}


/// Завершить эксперимент (in_progress -> completed) и израсходовать реагенты
pub async fn complete_experiment(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    user_id: String,
    http_request: &actix_web::HttpRequest,
) -> ApiResult<HttpResponse> {
    let experiment_id = path.into_inner();
    let now = Utc::now();

    let existing: Experiment = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Experiment"))?;

    if existing.status != "in_progress" {
        return Err(ApiError::bad_request(&format!(
            "Cannot complete experiment with status '{}'. Only 'in_progress' experiments can be completed.",
            existing.status
        )));
    }

    let mut tx = app_state.db_pool.begin().await?;

    // Status change first: it is the first write (takes the lock) and the
    // condition makes a concurrent second "complete" fail instead of double-consuming.
    let changed = sqlx::query(r#"
        UPDATE experiments 
        SET status = 'completed', end_date = COALESCE(end_date, ?), updated_by = ?, updated_at = ?
        WHERE id = ? AND status = 'in_progress'
    "#)
        .bind(&now)
        .bind(&user_id)
        .bind(&now)
        .bind(&experiment_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if changed == 0 {
        return Err(ApiError::bad_request("Experiment is no longer in progress"));
    }

    let consumed = crate::consumption::consume_experiment(
        &mut *tx, &experiment_id, &existing.title, Some(&user_id),
    ).await?;

    tx.commit().await?;

    audit_experiment_consumption(&app_state.db_pool, &user_id, &consumed, &existing.title, http_request).await;

    let updated: Experiment = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    info!("User {} completed experiment: {} (consumed {} reagents)", 
          user_id, experiment_id, consumed.len());
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "experiment": updated,
        "reagents_consumed": consumed.len()
    }))))
}

/// Audit entries for experiment write-offs (after commit)
async fn audit_experiment_consumption(
    pool: &sqlx::SqlitePool,
    user_id: &str,
    consumed: &[crate::consumption::ConsumeResult],
    experiment_title: &str,
    http_request: &actix_web::HttpRequest,
) {
    let context = format!("in experiment \"{}\"", experiment_title);
    for r in consumed {
        crate::audit::audit_with_changes(
            pool, user_id, "use_reagent", "batch", &r.batch_id,
            &r.describe(Some(&context)), &r.audit_changes(), http_request,
        ).await;
    }
}

/// Отменить эксперимент (planned|in_progress -> cancelled) и вернуть реагенты
pub async fn cancel_experiment(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    user_id: String,
) -> ApiResult<HttpResponse> {
    let experiment_id = path.into_inner();
    let now = Utc::now();

    let existing: Experiment = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Experiment"))?;

    if !["planned", "in_progress"].contains(&existing.status.as_str()) {
        return Err(ApiError::bad_request(&format!(
            "Cannot cancel experiment with status '{}'. Only 'planned' or 'in_progress' experiments can be cancelled.",
            existing.status
        )));
    }

    let mut tx = app_state.db_pool.begin().await?;

    let reagents: Vec<ExperimentReagent> = sqlx::query_as(r#"
        SELECT id, experiment_id, batch_id, planned_quantity, is_consumed, notes, created_at
        FROM experiment_reagents 
        WHERE experiment_id = ?
    "#)
        .bind(&experiment_id)
        .fetch_all(&mut *tx)
        .await?;

    let mut returned_count = 0;

    // Возвращаем зарезервированное количество в батчи
    for reagent in reagents {
        if !reagent.is_consumed {
            let qty = reagent.planned_quantity.unwrap_or(0.0);
            if qty > 0.0 {
                sqlx::query(r#"
                    UPDATE batches 
                    SET reserved_quantity = MAX(0, reserved_quantity - ?)
                    WHERE id = ?
                "#)
                    .bind(qty)
                    .bind(&reagent.batch_id)
                    .execute(&mut *tx)
                    .await?;
            }
            
            returned_count += 1;
        }
    }

    // Обновляем статус эксперимента
    sqlx::query(r#"
        UPDATE experiments 
        SET status = 'cancelled', updated_by = ?, updated_at = ?
        WHERE id = ?
    "#)
        .bind(&user_id)
        .bind(&now)
        .bind(&experiment_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let updated: Experiment = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await?;

    info!("User {} cancelled experiment: {} (returned {} reagents)", 
          user_id, experiment_id, returned_count);
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "experiment": updated,
        "reagents_returned": returned_count
    }))))
}


/// Израсходовать конкретный реагент эксперимента
pub async fn consume_experiment_reagent(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    user_id: String,
    http_request: &actix_web::HttpRequest,
) -> ApiResult<HttpResponse> {
    let (experiment_id, reagent_link_id) = path.into_inner();

    let experiment: Experiment = sqlx::query_as("SELECT * FROM experiments WHERE id = ?")
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Experiment"))?;

    if experiment.status != "in_progress" {
        return Err(ApiError::bad_request(
            "Can only consume reagents from 'in_progress' experiments"
        ));
    }

    let link: ExperimentReagent = sqlx::query_as(r#"
        SELECT id, experiment_id, batch_id, planned_quantity, is_consumed, notes, created_at
        FROM experiment_reagents 
        WHERE id = ? AND experiment_id = ?
    "#)
        .bind(&reagent_link_id)
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Experiment reagent"))?;

    if link.planned_quantity.unwrap_or(0.0) <= 0.0 {
        return Err(ApiError::bad_request("Reagent has no quantity to consume"));
    }

    let mut tx = app_state.db_pool.begin().await?;
    let result = crate::consumption::consume_experiment_link(
        &mut *tx, &link.id, &link.batch_id, link.planned_quantity,
        &experiment_id, &experiment.title, Some(&user_id),
    ).await?;
    let result = match result {
        Some(r) => r,
        None => return Err(ApiError::bad_request("Reagent is already consumed")),
    };
    tx.commit().await?;

    audit_experiment_consumption(
        &app_state.db_pool, &user_id, std::slice::from_ref(&result), &experiment.title, http_request,
    ).await;

    Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "message": "Reagent consumed successfully",
        "reagent_id": reagent_link_id,
        "quantity_consumed": result.quantity_used,
        "remaining_quantity": result.batch_quantity_after,
        "batch_status": result.status_after,
        "containers": result.containers,
    }))))
}

// ==================== AUTO UPDATE STATUSES ====================

#[derive(Debug, Serialize, Clone)]
pub struct AutoUpdateResult {
    pub started: i32,
    pub completed: i32,
    pub total_updated: i32,
}

/// Сколько секунд до ближайшего события (для smart sleep в фоновой задаче).
/// Возвращает None если нет pending экспериментов.
pub async fn seconds_until_next_transition(pool: &sqlx::SqlitePool) -> Result<Option<i64>, sqlx::Error> {
    // Один лёгкий запрос: MIN из ближайшего start и ближайшего end.
    // datetime() нормализует любой формат даты перед сравнением.
    let row: Option<i64> = sqlx::query_scalar(r#"
        SELECT MIN(seconds) FROM (
            SELECT CAST((julianday(datetime(start_date)) - julianday(datetime('now'))) * 86400 AS INTEGER) as seconds
            FROM experiments
            WHERE status = 'planned' AND start_date IS NOT NULL
            UNION ALL
            SELECT CAST((julianday(datetime(end_date)) - julianday(datetime('now'))) * 86400 AS INTEGER) as seconds
            FROM experiments
            WHERE status = 'in_progress' AND end_date IS NOT NULL
        )
    "#)
        .fetch_one(pool)
        .await?;

    Ok(row)
}

/// Standalone логика авто-обновления.
/// Вызывается и из HTTP-хендлера, и из фоновой задачи.
/// КЛЮЧЕВОЙ ФИX: datetime() нормализует формат дат перед сравнением.
/// Без этого SQLite сравнивает даты как текст и "2025-01-01T09:00:00Z" > "2025-01-01 12:00:00+00:00"
/// потому что 'T' (0x54) > ' ' (0x20) в ASCII.
pub async fn run_auto_update_statuses(pool: &sqlx::SqlitePool) -> Result<AutoUpdateResult, sqlx::Error> {
    let now = Utc::now();
    let mut tx = pool.begin().await?;

    // 1. planned → in_progress (пришло время start_date)
    // datetime() нормализует оба операнда в "YYYY-MM-DD HH:MM:SS"
    let started_result = sqlx::query(r#"
        UPDATE experiments
        SET status = 'in_progress', updated_at = ?
        WHERE status = 'planned'
          AND start_date IS NOT NULL
          AND datetime(start_date) <= datetime(?)
    "#)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

    let started = started_result.rows_affected() as i32;

    // 2. in_progress → completed (пришло время end_date)
    let to_complete: Vec<String> = sqlx::query_scalar(r#"
        SELECT id FROM experiments
        WHERE status = 'in_progress'
          AND end_date IS NOT NULL
          AND datetime(end_date) <= datetime(?)
    "#)
        .bind(&now)
        .fetch_all(&mut *tx)
        .await?;

    let completed = to_complete.len() as i32;

    // Audit lines written after commit: Ok = write-off, Err = skipped write-off
    let mut outcomes: Vec<Result<String, String>> = Vec::new();

    // 3. Для каждого завершаемого — списываем реагенты через общий путь списания.
    //    Background completion must never get stuck, so each reagent runs in its
    //    own savepoint: a failed write-off is rolled back, its reservation is
    //    released, it stays unconsumed and the failure is audited.
    for exp_id in &to_complete {
        let title: String = sqlx::query_scalar("SELECT title FROM experiments WHERE id = ?")
            .bind(exp_id)
            .fetch_one(&mut *tx)
            .await?;
        let context = format!("in experiment \"{}\" (auto-consumed on completion)", title);

        let links = crate::consumption::pending_experiment_links(&mut *tx, exp_id)
            .await
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

        for link in links {
            let mut sp = sqlx::Connection::begin(&mut *tx).await?;
            match crate::consumption::consume_experiment_link(
                &mut *sp, &link.id, &link.batch_id, link.planned_quantity, exp_id, &title, None,
            ).await {
                Ok(Some(r)) => {
                    sp.commit().await?;
                    outcomes.push(Ok(format!("{}|{}", r.batch_id, r.describe(Some(&context)))));
                }
                Ok(None) => sp.commit().await?,
                Err(e) => {
                    sp.rollback().await?;
                    let qty = link.planned_quantity.unwrap_or(0.0);
                    sqlx::query(
                        "UPDATE batches SET reserved_quantity = MAX(0, COALESCE(reserved_quantity, 0) - ?) WHERE id = ?"
                    )
                        .bind(qty)
                        .bind(&link.batch_id)
                        .execute(&mut *tx)
                        .await?;
                    log::warn!("Auto-consumption skipped for experiment {}: {}", exp_id, e);
                    outcomes.push(Err(format!(
                        "{}|Auto-consumption of {} from batch skipped for experiment \"{}\": {}. Reservation released.",
                        link.batch_id, qty, title, e
                    )));
                }
            }
        }

        sqlx::query(r#"
            UPDATE experiments
            SET status = 'completed', updated_at = ?
            WHERE id = ?
        "#)
            .bind(&now)
            .bind(exp_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    // Audit automatic write-offs (system action: no user, no request)
    for outcome in &outcomes {
        let (action, line) = match outcome {
            Ok(l) => ("use_reagent", l),
            Err(l) => ("use_reagent_failed", l),
        };
        let mut parts = line.splitn(2, '|');
        let batch_id = parts.next().unwrap_or_default();
        let description = parts.next().unwrap_or_default();
        if let Err(e) = crate::audit::log_system_activity(pool, action, "batch", batch_id, description).await {
            log::error!("Failed to write audit log for auto-consumption: {}", e);
        }
    }

    let total_updated = started + completed;
    if total_updated > 0 {
        info!("Auto-updated: {} started, {} completed (reagents consumed)", started, completed);
    }

    Ok(AutoUpdateResult { started, completed, total_updated })
}

/// HTTP-хендлер (обёртка)
pub async fn auto_update_experiment_statuses(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let result = run_auto_update_statuses(&app_state.db_pool)
        .await
        .map_err(|e| ApiError::InternalServerError(format!("Auto-update failed: {}", e)))?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(result)))
}

/// Диагностика дат — показывает как хранятся даты и почему сравнение может не работать
pub async fn diagnose_experiment_dates(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    #[derive(Debug, Serialize, sqlx::FromRow)]
    struct DateDiag {
        id: String,
        title: String,
        status: String,
        start_date_raw: Option<String>,
        end_date_raw: Option<String>,
        start_date_normalized: Option<String>,
        end_date_normalized: Option<String>,
        now_normalized: Option<String>,
        start_overdue: Option<i32>,
        end_overdue: Option<i32>,
    }

    let diags: Vec<DateDiag> = sqlx::query_as(r#"
        SELECT
            id,
            title,
            status,
            start_date as start_date_raw,
            end_date as end_date_raw,
            datetime(start_date) as start_date_normalized,
            datetime(end_date) as end_date_normalized,
            datetime('now') as now_normalized,
            CASE WHEN datetime(start_date) <= datetime('now') THEN 1 ELSE 0 END as start_overdue,
            CASE WHEN end_date IS NOT NULL AND datetime(end_date) <= datetime('now') THEN 1 ELSE 0 END as end_overdue
        FROM experiments
        WHERE status IN ('planned', 'in_progress')
        ORDER BY start_date ASC
    "#)
        .fetch_all(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(diags)))
}

// ==================== CALENDAR ====================

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub start: chrono::DateTime<Utc>,
    pub status: String,
    pub experiment_type: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CalendarQuery {
    pub start: Option<String>,
    pub end: Option<String>,
}

pub async fn get_experiments_calendar(
    app_state: web::Data<Arc<AppState>>,
    query: web::Query<CalendarQuery>,
) -> ApiResult<HttpResponse> {
    let start = query.start.as_deref().unwrap_or("1970-01-01");
    let end = query.end.as_deref().unwrap_or("2100-12-31");

    let events: Vec<CalendarEvent> = sqlx::query_as(r#"
        SELECT id, title, experiment_date as start, status, experiment_type, location
        FROM experiments
        WHERE experiment_date >= ? AND experiment_date <= ?
        ORDER BY experiment_date ASC
    "#)
        .bind(start)
        .bind(end)
        .fetch_all(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(events)))
}

// ==================== DOCUMENTS ====================

pub async fn get_experiment_documents(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let experiment_id = path.into_inner();

    #[derive(Debug, Serialize, sqlx::FromRow)]
    struct ExperimentDocument {
        id: String,
        experiment_id: String,
        filename: String,
        original_name: String,
        mime_type: String,
        size: i64,
        uploaded_by: Option<String>,
        created_at: chrono::DateTime<Utc>,
    }

    let docs: Vec<ExperimentDocument> = sqlx::query_as(
        "SELECT * FROM experiment_documents WHERE experiment_id = ? ORDER BY created_at DESC"
    )
        .bind(&experiment_id)
        .fetch_all(&app_state.db_pool)
        .await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(docs)))
}

pub async fn download_experiment_document(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
) -> Result<NamedFile, ApiError> {
    let (experiment_id, doc_id) = path.into_inner();

    #[derive(sqlx::FromRow)]
    struct DocInfo {
        filename: String,
        #[allow(dead_code)]
        original_name: String,
    }

    let doc: DocInfo = sqlx::query_as(
        "SELECT filename, original_name FROM experiment_documents WHERE id = ? AND experiment_id = ?"
    )
        .bind(&doc_id)
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Document"))?;

    let file_path = PathBuf::from("./uploads/experiments").join(&doc.filename);
    
    NamedFile::open(&file_path)
        .map_err(|_| ApiError::not_found("Document file"))
}
