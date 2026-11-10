// src/error.rs
use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    InternalServerError(String),
    ValidationError(String),
    DatabaseError(sqlx::Error),
    AuthError(String),
}

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    message: String,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            ApiError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            ApiError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            ApiError::InternalServerError(msg) => write!(f, "Internal Server Error: {}", msg),
            ApiError::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            ApiError::DatabaseError(err) => write!(f, "Database Error: {}", err),
            ApiError::AuthError(msg) => write!(f, "Auth Error: {}", msg),
        }
    }
}

impl ApiError {
    /// How a database error is reported. Constraint violations are caused by
    /// the submitted data, so they are client errors and their message (which
    /// only names the column or rule) is useful to the user. Everything else
    /// is a server fault whose details (SQL text, file paths) stay in the log.
    fn database_error_parts(err: &sqlx::Error) -> (StatusCode, String) {
        use sqlx::error::ErrorKind;
        match err.as_database_error().map(|d| (d.kind(), d.message().to_string())) {
            Some((ErrorKind::UniqueViolation, msg)) | Some((ErrorKind::ForeignKeyViolation, msg)) => {
                (StatusCode::CONFLICT, format!("Conflict: {}", msg))
            }
            Some((ErrorKind::NotNullViolation, msg)) | Some((ErrorKind::CheckViolation, msg)) => {
                (StatusCode::BAD_REQUEST, format!("Bad Request: {}", msg))
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Database Error".to_string()),
        }
    }
}

impl ResponseError for ApiError {
    // Without this actix reports 500 for every variant to anything that asks
    // the error (not the response) for its status, e.g. middleware and tests.
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Unauthorized(_) | ApiError::AuthError(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::DatabaseError(err) => Self::database_error_parts(err).0,
            ApiError::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let message = match self {
            ApiError::DatabaseError(err) => {
                let (status, message) = Self::database_error_parts(err);
                if status.is_server_error() {
                    log::error!("Database error: {}", err);
                }
                message
            }
            other => other.to_string(),
        };

        HttpResponse::build(self.status_code()).json(ErrorResponse { success: false, message })
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::DatabaseError(err)
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(err: validator::ValidationErrors) -> Self {
        ApiError::ValidationError(err.to_string())
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::InternalServerError(format!("IO Error: {}", err))
    }
}

impl From<actix_multipart::MultipartError> for ApiError {
    fn from(err: actix_multipart::MultipartError) -> Self {
        ApiError::BadRequest(format!("Multipart Error: {}", err))
    }
}

// ✅ NEW: From<String> для использования с ?
impl From<String> for ApiError {
    fn from(err: String) -> Self {
        ApiError::BadRequest(err)
    }
}

// ✅ NEW: From<&str> для удобства
impl From<&str> for ApiError {
    fn from(err: &str) -> Self {
        ApiError::BadRequest(err.to_string())
    }
}

// Специфичные ошибки для LIMS
impl ApiError {
    pub fn not_found(resource: &str) -> Self {
        ApiError::NotFound(format!("{} not found", resource))
    }

    pub fn bad_request(message: &str) -> Self {
        ApiError::BadRequest(message.to_string())
    }

    // ✅ NEW: internal server error helper
    pub fn internal(message: &str) -> Self {
        ApiError::InternalServerError(message.to_string())
    }

    // ✅ NEW: internal с format
    pub fn internal_error(message: impl Into<String>) -> Self {
        ApiError::InternalServerError(message.into())
    }

    pub fn reagent_not_found(id: &str) -> Self {
        ApiError::NotFound(format!("Reagent with ID '{}' not found", id))
    }

    pub fn batch_not_found(id: &str) -> Self {
        ApiError::NotFound(format!("Batch with ID '{}' not found", id))
    }

    pub fn reagent_already_exists(name: &str) -> Self {
        ApiError::BadRequest(format!("Reagent '{}' already exists", name))
    }

    pub fn batch_already_exists(reagent_name: &str, batch_number: &str) -> Self {
        ApiError::BadRequest(format!("Batch '{}' already exists for reagent '{}'", batch_number, reagent_name))
    }

    pub fn invalid_reagent_id() -> Self {
        ApiError::BadRequest("Invalid reagent ID format".to_string())
    }

    pub fn invalid_batch_id() -> Self {
        ApiError::BadRequest("Invalid batch ID format".to_string())
    }

    pub fn insufficient_quantity(available: f64, requested: f64) -> Self {
        ApiError::BadRequest(format!("Insufficient quantity. Available: {}, Requested: {}", available, requested))
    }

    pub fn batch_expiry_date_invalid() -> Self {
        ApiError::BadRequest("Expiry date cannot be in the past".to_string())
    }

    pub fn cannot_modify_depleted_batch() -> Self {
        ApiError::BadRequest("Cannot modify depleted batch".to_string())
    }

    pub fn validation_failed(field: &str) -> Self {
        ApiError::ValidationError(format!("Validation failed for field: {}", field))
    }

    pub fn equipment_not_found(id: &str) -> Self {
        ApiError::NotFound(format!("Equipment with ID '{}' not found", id))
    }

    pub fn file_not_found(id: &str) -> Self {
        ApiError::NotFound(format!("File with ID '{}' not found", id))
    }

    pub fn maintenance_not_found(id: &str) -> Self {
        ApiError::NotFound(format!("Maintenance record with ID '{}' not found", id))
    }

    pub fn part_not_found(id: &str) -> Self {
        ApiError::NotFound(format!("Part with ID '{}' not found", id))
    }
}

// Функции валидации
pub fn validate_cas_number(cas: &str) -> Result<(), ApiError> {
    if cas.is_empty() {
        return Ok(());
    }

    let parts: Vec<&str> = cas.split('-').collect();
    if parts.len() != 3 {
        return Err(ApiError::ValidationError("Invalid CAS number format".to_string()));
    }

    Ok(())
}

pub fn validate_quantity(quantity: f64) -> Result<(), ApiError> {
    if quantity < 0.0 {
        return Err(ApiError::ValidationError("Quantity cannot be negative".to_string()));
    }
    if quantity > 1e9 {
        return Err(ApiError::ValidationError("Quantity too large".to_string()));
    }
    Ok(())
}

pub fn validate_unit(unit: &str) -> Result<(), ApiError> {
    let valid_units = ["g", "kg", "mg", "μg", "L", "mL", "μL", "mol", "mmol", "μmol", "pieces", "pcs"];
    if !valid_units.contains(&unit) {
        return Err(ApiError::ValidationError(format!(
            "Invalid unit '{}'. Valid units: {}",
            unit,
            valid_units.join(", ")
        )));
    }
    Ok(())
}
