use bcrypt::{hash, verify};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;
use actix_web::web;
use actix_web::HttpMessage;
use validator::Validate;
use actix_web::{HttpRequest, dev::ServiceRequest};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use crate::error::{ApiError, ApiResult};

// ======== USER MODEL ========

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub name: Option<String>,
    pub role: String,
    pub is_active: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub failed_login_attempts: u32,
    pub locked_until: Option<DateTime<Utc>>,
}

// ======== USER ROLE ========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    Researcher,
    Viewer,
}

impl UserRole {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "admin" => Some(UserRole::Admin),
            "researcher" => Some(UserRole::Researcher),
            "viewer" => Some(UserRole::Viewer),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Admin => "admin",
            UserRole::Researcher => "researcher",
            UserRole::Viewer => "viewer",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            UserRole::Admin => "Administrator",
            UserRole::Researcher => "Researcher",
            UserRole::Viewer => "Viewer",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            UserRole::Admin => "Full access to all system features including user management",
            UserRole::Researcher => "Can create and edit data, limited delete permissions",
            UserRole::Viewer => "Read-only access with ability to use batches",
        }
    }

    // ======== USER MANAGEMENT ========
    pub fn can_manage_users(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    pub fn can_view_users(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    // ======== REAGENT PERMISSIONS ========
    pub fn can_create_reagents(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    pub fn can_edit_reagents(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    pub fn can_delete_reagents(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    pub fn can_view_reagents(&self) -> bool {
        true // All roles can view
    }

    // ======== BATCH PERMISSIONS ========
    pub fn can_create_batches(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    pub fn can_edit_batches(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    pub fn can_delete_batches(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    pub fn can_view_batches(&self) -> bool {
        true // All roles can view
    }

    pub fn can_use_batches(&self) -> bool {
        true // All roles can use/consume batches
    }

    // ======== EQUIPMENT PERMISSIONS ========
    pub fn can_manage_equipment(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    pub fn can_create_equipment(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    pub fn can_edit_equipment(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    pub fn can_delete_equipment(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    pub fn can_view_equipment(&self) -> bool {
        true // All roles can view
    }

    pub fn can_manage_equipment_maintenance(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    // ======== EXPERIMENT PERMISSIONS ========
    pub fn can_create_experiments(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    pub fn can_edit_experiments(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    pub fn can_delete_experiments(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    pub fn can_view_experiments(&self) -> bool {
        true // All roles can view
    }

    // ======== ROOM PERMISSIONS ========
    pub fn can_create_rooms(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    pub fn can_edit_rooms(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    pub fn can_delete_rooms(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    pub fn can_view_rooms(&self) -> bool {
        true // All roles can view
    }

    // ======== REPORT PERMISSIONS ========
    pub fn can_view_reports(&self) -> bool {
        true // All roles can view reports
    }

    pub fn can_export_reports(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    // ======== IMPORT/EXPORT PERMISSIONS ========
    pub fn can_import_data(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    pub fn can_export_data(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Researcher)
    }

    // ======== SYSTEM PERMISSIONS ========
    pub fn can_view_audit_log(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    pub fn can_manage_system(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    /// Get all available roles
    pub fn all_roles() -> Vec<Self> {
        vec![UserRole::Admin, UserRole::Researcher, UserRole::Viewer]
    }

    /// Get all valid role strings
    pub fn all_role_strings() -> Vec<&'static str> {
        vec!["admin", "researcher", "viewer"]
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ======== REQUEST/RESPONSE STRUCTS ========

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, message = "Username is required"))]
    pub username: String,
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50, message = "Username must be 3-50 characters"))]
    pub username: String,
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1, message = "Current password is required"))]
    pub current_password: String,
    #[validate(length(min = 8, message = "New password must be at least 8 characters"))]
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: i64,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub name: Option<String>,
    pub role: UserRole,
    pub is_active: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            name: user.name,
            role: UserRole::from_str(&user.role).unwrap_or(UserRole::Viewer),
            is_active: user.is_active,
            last_login: user.last_login,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub exp: i64,
    pub iat: i64,
}

// ======== AUTH SERVICE ========

pub struct AuthService {
    jwt_secret: String,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthService {
    pub fn new(jwt_secret: &str) -> Self {
        Self {
            jwt_secret: jwt_secret.to_string(),
            encoding_key: EncodingKey::from_secret(jwt_secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(jwt_secret.as_bytes()),
        }
    }

    pub fn hash_password(&self, password: &str) -> Result<String, bcrypt::BcryptError> {
        match validate_password_strength(password) {
            Ok(_) => hash(password, 12),
            Err(e) => Err(bcrypt::BcryptError::InvalidHash(e.to_string())),
        }
    }

    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
        verify(password, hash)
    }

    pub fn generate_token(&self, user: &User) -> ApiResult<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(24);

        let claims = Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            role: UserRole::from_str(&user.role).unwrap_or(UserRole::Viewer),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|_| ApiError::AuthError("Failed to generate token".to_string()))
    }

    pub fn verify_token(&self, token: &str) -> ApiResult<Claims> {
        let validation = Validation::default();
        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|err| {
                match err.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature =>
                        ApiError::AuthError("Token expired".to_string()),
                    jsonwebtoken::errors::ErrorKind::InvalidToken =>
                        ApiError::AuthError("Invalid token".to_string()),
                    _ =>
                        ApiError::AuthError("Token verification failed".to_string()),
                }
            })
    }
}

// ======== PASSWORD VALIDATION ========

fn validate_password_strength(password: &str) -> Result<(), ApiError> {
    if password.len() < 8 {
        return Err(ApiError::ValidationError("Password must be at least 8 characters".to_string()));
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(ApiError::ValidationError("Password must contain at least one uppercase letter".to_string()));
    }
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err(ApiError::ValidationError("Password must contain at least one lowercase letter".to_string()));
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(ApiError::ValidationError("Password must contain at least one digit".to_string()));
    }
    Ok(())
}

// ======== USER METHODS ========

impl User {
    pub async fn find_by_username(pool: &SqlitePool, username: &str) -> ApiResult<User> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::NotFound("User not found".to_string()))
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> ApiResult<User> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::NotFound("User not found".to_string()))
    }

    pub async fn find_by_email(pool: &SqlitePool, email: &str) -> ApiResult<User> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
            .bind(email)
            .fetch_one(pool)
            .await
            .map_err(|_| ApiError::NotFound("User not found".to_string()))
    }

    pub async fn create(
        pool: &SqlitePool,
        request: RegisterRequest,
        role: UserRole,
        auth_service: &AuthService,
    ) -> ApiResult<User> {
        // Validate password strength
        validate_password_strength(&request.password)?;

        // Only Viewer role is available for self-registration
        if role != UserRole::Viewer {
            return Err(ApiError::Forbidden("Cannot register with this role".to_string()));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let password_hash = auth_service.hash_password(&request.password)
            .map_err(|_| ApiError::InternalServerError("Failed to hash password".to_string()))?;

        let user = User {
            id: id.clone(),
            username: request.username,
            email: request.email,
            password_hash,
            name: None,
            role: role.as_str().to_string(),
            is_active: true,
            last_login: None,
            created_at: now,
            updated_at: now,
            failed_login_attempts: 0,
            locked_until: None,
        };

        sqlx::query(
            r#"INSERT INTO users (
                id, username, email, password_hash, role, is_active,
                created_at, updated_at, failed_login_attempts, locked_until
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
            .bind(&user.id)
            .bind(&user.username)
            .bind(&user.email)
            .bind(&user.password_hash)
            .bind(&user.role)
            .bind(user.is_active as i32)
            .bind(&user.created_at)
            .bind(&user.updated_at)
            .bind(user.failed_login_attempts)
            .bind(&user.locked_until)
            .execute(pool)
            .await?;

        Ok(user)
    }

    pub async fn update_last_login(&self, pool: &SqlitePool) -> ApiResult<()> {
        sqlx::query("UPDATE users SET last_login = datetime('now') WHERE id = ?")
            .bind(&self.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn change_password(
        &self,
        pool: &SqlitePool,
        current_password: &str,
        new_password: &str,
        auth_service: &AuthService
    ) -> ApiResult<()> {
        // Verify current password
        if !auth_service.verify_password(current_password, &self.password_hash)
            .map_err(|_| ApiError::InternalServerError("Password verification failed".to_string()))?
        {
            return Err(ApiError::AuthError("Current password is incorrect".to_string()));
        }

        // Validate new password strength
        validate_password_strength(new_password)?;

        // Hash and save new password
        let new_hash = auth_service.hash_password(new_password)
            .map_err(|_| ApiError::InternalServerError("Failed to hash password".to_string()))?;

        sqlx::query(
            "UPDATE users SET password_hash = ?, updated_at = datetime('now') WHERE id = ?"
        )
            .bind(&new_hash)
            .bind(&self.id)
            .execute(pool)
            .await?;

        Ok(())
    }

    // Methods for lock management
    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            Utc::now() < locked_until
        } else {
            false
        }
    }

    pub async fn increment_failed_attempts(&mut self, pool: &SqlitePool) -> ApiResult<()> {
        self.failed_login_attempts += 1;
        sqlx::query("UPDATE users SET failed_login_attempts = ? WHERE id = ?")
            .bind(self.failed_login_attempts)
            .bind(&self.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn lock_for_duration(&mut self, pool: &SqlitePool, duration: Duration) -> ApiResult<()> {
        self.locked_until = Some(Utc::now() + duration);
        sqlx::query("UPDATE users SET locked_until = ? WHERE id = ?")
            .bind(self.locked_until)
            .bind(&self.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn reset_failed_attempts(&mut self, pool: &SqlitePool) -> ApiResult<()> {
        self.failed_login_attempts = 0;
        self.locked_until = None;
        sqlx::query(
            "UPDATE users SET failed_login_attempts = 0, locked_until = NULL WHERE id = ?"
        )
            .bind(&self.id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Get the UserRole enum from the role string
    pub fn get_role(&self) -> UserRole {
        UserRole::from_str(&self.role).unwrap_or(UserRole::Viewer)
    }
}

// ======== HELPER FUNCTIONS ========

pub fn get_current_user(req: &HttpRequest) -> ApiResult<Claims> {
    req.extensions()
        .get::<Claims>().cloned()
        .ok_or_else(|| ApiError::Unauthorized("No user information found".to_string()))
}

pub fn check_permission<F>(claims: &Claims, check: F) -> ApiResult<()>
where
    F: Fn(&UserRole) -> bool,
{
    if check(&claims.role) {
        Ok(())
    } else {
        Err(ApiError::Forbidden("Insufficient permissions".to_string()))
    }
}

/// Check if the current user has a specific permission
pub fn require_permission(req: &HttpRequest, permission_check: fn(&UserRole) -> bool) -> ApiResult<Claims> {
    let claims = get_current_user(req)?;
    check_permission(&claims, permission_check)?;
    Ok(claims)
}

// ======== JWT MIDDLEWARE ========

pub async fn jwt_middleware(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (actix_web::Error, ServiceRequest)> {
    let token = credentials.token();

    let auth_service = match req.app_data::<web::Data<std::sync::Arc<AuthService>>>() {
        Some(svc) => svc,
        None => {
            log::error!("AuthService not found in app data");
            return Err((
                ApiError::InternalServerError("Auth service not available".to_string()).into(),
                req,
            ));
        }
    };

    match auth_service.verify_token(token) {
        Ok(claims) => {
            req.extensions_mut().insert(claims);
            Ok(req)
        }
        Err(err) => {
            log::warn!("JWT verification failed: {}", err);
            Err((err.into(), req))
        }
    }
}