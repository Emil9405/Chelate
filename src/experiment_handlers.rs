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
    
    // Подготовка данных с учётом существующих значений
    let title = update.title.as_ref().unwrap_or(&existing.title);
    let description = update.description.clone().or(existing.description.clone());
    let experiment_date = update.experiment_date.unwrap_or(existing.experiment_date);
    let experiment_type = update.experiment_type.clone().or(existing.experiment_type.clone());
    let instructor = update.instructor.clone().or(existing.instructor.clone());
    let student_group = update.student_group.clone().or(existing.student_group.clone());
    let status = update.status.as_ref().unwrap_or(&existing.status);
    let location = update.location.clone().or(existing.location.clone());
    let protocol = update.protocol.clone().or(existing.protocol.clone());
    let results = update.results.clone().or(existing.results.clone());
    let notes = update.notes.clone().or(existing.notes.clone());
    let start_date = update.start_date.unwrap_or(existing.start_date);
    let end_date = update.end_date.or(existing.end_date);

    sqlx::query(r#"
        UPDATE experiments SET 
        title = ?, description = ?, experiment_date = ?, experiment_type = ?, 
        instructor = ?, student_group = ?, status = ?, location = ?,
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
        .bind(&protocol)
        .bind(&start_date)
        .bind(&end_date)
        .bind(&results)
        .bind(&notes)
        .bind(&user_id)
        .bind(&now)
        .bind(&experiment_id)
        .execute(&app_state.db_pool)
        .await?;

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

    // Сначала удаляем связанные реагенты и возвращаем резервы
    let reagents: Vec<ExperimentReagent> = sqlx::query_as(
        "SELECT * FROM experiment_reagents WHERE experiment_id = ? AND is_consumed = 0"
    )
        .bind(&experiment_id)
        .fetch_all(&app_state.db_pool)
        .await?;

    let mut tx = app_state.db_pool.begin().await?;

    // Возвращаем зарезервированное количество
    for reagent in &reagents {
        let qty = reagent.quantity_used.unwrap_or(0.0);
        sqlx::query("UPDATE batches SET reserved_quantity = MAX(0, reserved_quantity - ?) WHERE id = ?")
            .bind(qty)
            .bind(&reagent.batch_id)
            .execute(&mut *tx)
            .await?;
    }

    // Удаляем связи реагентов
    sqlx::query("DELETE FROM experiment_reagents WHERE experiment_id = ?")
        .bind(&experiment_id)
        .execute(&mut *tx)
        .await?;

    // Удаляем эксперимент
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
    pub quantity_used: Option<f64>,
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
            er.quantity_used, er.is_consumed, er.notes, er.created_at,
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

    // Check batch exists and has enough quantity
    let batch: Batch = sqlx::query_as("SELECT * FROM batches WHERE id = ?")
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
        INSERT INTO experiment_reagents (id, experiment_id, batch_id, quantity_used, notes, created_at)
        VALUES (?, ?, ?, ?, ?, ?)
    "#)
        .bind(&id)
        .bind(&experiment_id)
        .bind(&body.batch_id)
        .bind(body.quantity_used)
        .bind(&body.notes)
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
        quantity_used: Option<f64>,
        is_consumed: bool,
    }

    let link: ReagentLink = sqlx::query_as(
        "SELECT batch_id, quantity_used, is_consumed FROM experiment_reagents WHERE id = ? AND experiment_id = ?"
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
    let qty = link.quantity_used.unwrap_or(0.0);
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

    // Получаем все нерасходованные реагенты эксперимента
    let reagents: Vec<ExperimentReagent> = sqlx::query_as(
        "SELECT * FROM experiment_reagents WHERE experiment_id = ? AND is_consumed = 0"
    )
        .bind(&experiment_id)
        .fetch_all(&mut *tx)
        .await?;

    // Для каждого реагента: списываем из batch и помечаем consumed
    for reagent in &reagents {
        let qty = reagent.quantity_used.unwrap_or(0.0);
        
        // Списываем количество из батча
        sqlx::query(r#"
            UPDATE batches 
            SET quantity = MAX(0, quantity - ?),
                reserved_quantity = MAX(0, reserved_quantity - ?)
            WHERE id = ?
        "#)
            .bind(qty)
            .bind(qty)
            .bind(&reagent.batch_id)
            .execute(&mut *tx)
            .await?;

        // Помечаем как consumed
        sqlx::query("UPDATE experiment_reagents SET is_consumed = 1 WHERE id = ?")
            .bind(&reagent.id)
            .execute(&mut *tx)
            .await?;
    }

    // Обновляем статус эксперимента
    sqlx::query(r#"
        UPDATE experiments 
        SET status = 'completed', end_date = COALESCE(end_date, ?), updated_by = ?, updated_at = ?
        WHERE id = ?
    "#)
        .bind(&now)
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

    info!("User {} completed experiment: {} (consumed {} reagents)", 
          user_id, experiment_id, reagents.len());
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "experiment": updated,
        "reagents_consumed": reagents.len()
    }))))
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

    // Получаем все нерасходованные реагенты
    let reagents: Vec<ExperimentReagent> = sqlx::query_as(
        "SELECT * FROM experiment_reagents WHERE experiment_id = ? AND is_consumed = 0"
    )
        .bind(&experiment_id)
        .fetch_all(&mut *tx)
        .await?;

    // Возвращаем зарезервированное количество в батчи
    for reagent in &reagents {
        let qty = reagent.quantity_used.unwrap_or(0.0);
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
          user_id, experiment_id, reagents.len());
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "experiment": updated,
        "reagents_returned": reagents.len()
    }))))
}

/// Израсходовать конкретный реагент эксперимента
pub async fn consume_experiment_reagent(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
    _user_id: String,
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

    let reagent: ExperimentReagent = sqlx::query_as(
        "SELECT * FROM experiment_reagents WHERE id = ? AND experiment_id = ?"
    )
        .bind(&reagent_link_id)
        .bind(&experiment_id)
        .fetch_one(&app_state.db_pool)
        .await
        .map_err(|_| ApiError::not_found("Experiment reagent"))?;

    if reagent.is_consumed {
        return Err(ApiError::bad_request("Reagent is already consumed"));
    }

    let qty = reagent.quantity_used.unwrap_or(0.0);
    let mut tx = app_state.db_pool.begin().await?;

    // Списываем из батча
    sqlx::query(r#"
        UPDATE batches 
        SET quantity = MAX(0, quantity - ?),
            reserved_quantity = MAX(0, reserved_quantity - ?)
        WHERE id = ?
    "#)
        .bind(qty)
        .bind(qty)
        .bind(&reagent.batch_id)
        .execute(&mut *tx)
        .await?;

    // Помечаем как consumed
    sqlx::query("UPDATE experiment_reagents SET is_consumed = 1 WHERE id = ?")
        .bind(&reagent_link_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
        "message": "Reagent consumed successfully",
        "reagent_id": reagent_link_id,
        "quantity_consumed": qty
    }))))
}

// ==================== AUTO UPDATE STATUSES ====================

#[derive(Debug, Serialize)]
pub struct AutoUpdateResult {
    pub started: i32,
    pub completed: i32,
    pub total_updated: i32,
}

/// Автоматическое обновление статусов экспериментов на основе времени
pub async fn auto_update_experiment_statuses(
    app_state: web::Data<Arc<AppState>>,
) -> ApiResult<HttpResponse> {
    let now = Utc::now();
    
    let mut tx = app_state.db_pool.begin().await?;
    
    // planned -> in_progress (start_date прошла)
    let started_result = sqlx::query(r#"
        UPDATE experiments 
        SET status = 'in_progress', updated_at = ?
        WHERE status = 'planned' 
          AND start_date IS NOT NULL 
          AND start_date <= ?
    "#)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    
    let started = started_result.rows_affected() as i32;
    
    // in_progress -> completed (end_date прошла)
    let completed_result = sqlx::query(r#"
        UPDATE experiments 
        SET status = 'completed', updated_at = ?
        WHERE status = 'in_progress' 
          AND end_date IS NOT NULL 
          AND end_date <= ?
    "#)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    
    let completed = completed_result.rows_affected() as i32;
    
    tx.commit().await?;
    
    let total_updated = started + completed;
    
    if total_updated > 0 {
        info!("Auto-updated experiment statuses: {} started, {} completed", started, completed);
    }
    
    Ok(HttpResponse::Ok().json(ApiResponse::success(AutoUpdateResult {
        started,
        completed,
        total_updated,
    })))
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
