// src/monitoring.rs
use actix_web::{HttpResponse, web};
use serde::Serialize;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use tokio::time::{interval, sleep, Duration};

#[derive(Debug, Clone)]
pub struct Metrics {
    pub request_count: Arc<AtomicU64>,
    pub error_count: Arc<AtomicU64>,
    pub response_times: Arc<std::sync::Mutex<Vec<u64>>>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            request_count: Arc::new(AtomicU64::new(0)),
            error_count: Arc::new(AtomicU64::new(0)),
            response_times: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn increment_requests(&self) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_response_time(&self, time_ms: u64) {
        if let Ok(mut times) = self.response_times.lock() {
            times.push(time_ms);
            if times.len() > 1000 {
                times.remove(0);
            }
        }
    }
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Serialize)]
pub struct MetricsResponse {
    pub requests_total: u64,
    pub errors_total: u64,
    pub avg_response_time_ms: f64,
    pub database_connections: i32,
    pub memory_usage_mb: f64,
}

pub async fn health_check() -> HttpResponse {
    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, 
    };

    HttpResponse::Ok().json(response)
}

pub async fn readiness_check(pool: web::Data<SqlitePool>) -> HttpResponse {
    match sqlx::query("SELECT 1").fetch_one(pool.get_ref()).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "status": "ready",
            "database": "connected"
        })),
        Err(_) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "status": "not ready",
            "database": "disconnected"
        })),
    }
}

pub async fn liveness_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "alive",
        "timestamp": Utc::now()
    }))
}

pub async fn metrics_endpoint(metrics: web::Data<Arc<Metrics>>) -> HttpResponse {
    let request_count = metrics.request_count.load(Ordering::Relaxed);
    let error_count = metrics.error_count.load(Ordering::Relaxed);

    let avg_response_time = if let Ok(times) = metrics.response_times.lock() {
        if times.is_empty() { 0.0 } else { times.iter().sum::<u64>() as f64 / times.len() as f64 }
    } else { 0.0 };

    let response = MetricsResponse {
        requests_total: request_count,
        errors_total: error_count,
        avg_response_time_ms: avg_response_time,
        database_connections: 0,
        memory_usage_mb: 0.0,
    };

    HttpResponse::Ok().json(response)
}

pub struct RequestLogger {
    metrics: Arc<Metrics>,
}

impl RequestLogger {
    pub fn new(metrics: Arc<Metrics>) -> Self {
        Self { metrics }
    }
}

impl<S, B> actix_web::dev::Transform<S, actix_web::dev::ServiceRequest> for RequestLogger
where
    S: actix_web::dev::Service<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<B>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = RequestLoggerMiddleware<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(RequestLoggerMiddleware {
            service,
            metrics: self.metrics.clone(),
        }))
    }
}

pub struct RequestLoggerMiddleware<S> {
    service: S,
    metrics: Arc<Metrics>,
}

impl<S, B> actix_web::dev::Service<actix_web::dev::ServiceRequest> for RequestLoggerMiddleware<S>
where
    S: actix_web::dev::Service<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<B>,
        Error = actix_web::Error,
    >,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, ctx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: actix_web::dev::ServiceRequest) -> Self::Future {
        let start_time = std::time::Instant::now();
        let metrics = self.metrics.clone();
        let fut = self.service.call(req);

        Box::pin(async move {
            metrics.increment_requests();
            let res = fut.await;
            let elapsed = start_time.elapsed().as_millis() as u64;
            metrics.record_response_time(elapsed);

            if let Ok(ref response) = res {
                if response.status().is_client_error() || response.status().is_server_error() {
                    metrics.increment_errors();
                }
            }
            res
        })
    }
}

pub async fn start_maintenance_tasks(pool: SqlitePool) {
    let pool_clone1 = pool.clone();
    let pool_clone2 = pool.clone();
    
    tokio::spawn(async move {
        cleanup_old_audit_logs(pool_clone1).await;
    });
    
    tokio::spawn(async move {
        update_batch_statuses(pool_clone2).await;
    });
}

async fn cleanup_old_audit_logs(pool: SqlitePool) {
    let mut interval = interval(Duration::from_secs(24 * 3600)); // Раз в день

    loop {
        interval.tick().await;
        log::info!("Starting daily cleanup of audit logs...");
        let mut total_deleted = 0;

        loop {
            // Удаляем пачками по 1000
            let result = sqlx::query(
                "DELETE FROM audit_logs 
                 WHERE id IN (
                     SELECT id FROM audit_logs 
                     WHERE created_at < datetime('now', '-90 days') 
                     LIMIT 1000
                 )"
            )
            .execute(&pool)
            .await;

            match result {
                Ok(res) => {
                    let count = res.rows_affected();
                    total_deleted += count;
                    if count < 1000 { break; }
                    sleep(Duration::from_millis(50)).await;
                },
                Err(e) => {
                    log::error!("Failed to cleanup audit logs chunk: {}", e);
                    break;
                }
            }
        }
        if total_deleted > 0 {
            log::info!("Cleaned up {} old audit log entries in chunks", total_deleted);
        }
    }
}

async fn update_batch_statuses(pool: SqlitePool) {
    let mut interval = interval(Duration::from_secs(3600)); // Раз в час

    loop {
        interval.tick().await;
        log::info!("Starting hourly batch status update...");
        let mut total_updated = 0;

        loop {
            // 1. Ищем ID просроченных (по 1000)
            let batch_ids: Vec<String> = match sqlx::query_scalar(
                r#"SELECT id FROM batches 
                   WHERE expiry_date < datetime('now') 
                   AND status = 'available' 
                   LIMIT 1000"#
            )
            .fetch_all(&pool)
            .await 
            {
                Ok(ids) => ids,
                Err(e) => {
                    log::error!("Failed to fetch expiring batches: {}", e);
                    break;
                }
            };

            if batch_ids.is_empty() { break; }

            // 2. Обновляем пачку
            let query = format!(
                "UPDATE batches SET status = 'expired', updated_at = datetime('now') WHERE id IN ({})",
                batch_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",")
            );
            
            let mut q = sqlx::query(&query);
            for id in &batch_ids {
                q = q.bind(id);
            }

            match q.execute(&pool).await {
                Ok(_) => { total_updated += batch_ids.len(); },
                Err(e) => { log::error!("Failed to update batch chunk: {}", e); }
            }

            sleep(Duration::from_millis(50)).await;
        }

        if total_updated > 0 {
            log::info!("Updated {} expired batches in chunks", total_updated);
        }
    }
}