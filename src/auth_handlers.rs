// src/auth_handlers.rs - Authentication route handlers with enhanced permissions

use actix_web::{web, HttpRequest, HttpResponse};
use validator::Validate;
use std::sync::Arc;
use chrono::{Duration, Utc};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::handlers::ApiResponse;
use crate::auth::{
    AuthService, User, LoginRequest, RegisterRequest, ChangePasswordRequest,
    LoginResponse, UserInfo, UserRole, get_current_user, check_permission
};
use crate::error::{ApiError, ApiResult};
use crate::AppState;

// Re-export get_current_user as get_claims_from_request for backward compatibility
pub use crate::auth::get_current_user as get_claims_from_request;

// ======== REQUEST STRUCTS ========

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    pub role: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ChangeUserPasswordRequest {
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub new_password: String,
}

/// Request for admin to create a new user
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 50, message = "Username must be 3-50 characters"))]
    pub username: String,
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
    #[validate(length(min = 1, message = "Role is required"))]
    pub role: String,
}
// ✅ ADDED MISSING LOGOUT FUNCTION
pub async fn logout() -> ApiResult<HttpResponse> {
    // In stateless JWT auth, the server doesn't need to do much.
    // The client should delete the token. 
    // If you use cookies, you would clear the cookie here.
    Ok(HttpResponse::Ok().json(ApiResponse::<()>::success_with_message((), "Logged out successfully".to_string())))
}
/// Response with user info and optional generated password
#[derive(Debug, Serialize)]
pub struct CreateUserResponse {
    pub user: UserInfo,
    pub generated_password: Option<String>,
}

// ======== PERMISSION DEFINITIONS ========

/// Available system permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    // User management
    ManageUsers,
    ViewUsers,
    
    // Reagent permissions
    CreateReagent,
    EditReagent,
    DeleteReagent,
    ViewReagent,
    
    // Batch permissions
    CreateBatch,
    EditBatch,
    DeleteBatch,
    ViewBatch,
    UseBatch,
    
    // Equipment permissions
    CreateEquipment,
    EditEquipment,
    DeleteEquipment,
    ViewEquipment,
    ManageEquipmentMaintenance,
    
    // Experiment permissions
    CreateExperiment,
    EditExperiment,
    DeleteExperiment,
    ViewExperiment,
    
    // Room permissions
    CreateRoom,
    EditRoom,
    DeleteRoom,
    ViewRoom,
    
    // Report permissions
    ViewReports,
    ExportReports,
    
    // Import/Export permissions
    ImportData,
    ExportData,
    
    // System permissions
    ViewAuditLog,
    ManageSystem,
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::ManageUsers => "manage_users",
            Permission::ViewUsers => "view_users",
            Permission::CreateReagent => "create_reagent",
            Permission::EditReagent => "edit_reagent",
            Permission::DeleteReagent => "delete_reagent",
            Permission::ViewReagent => "view_reagent",
            Permission::CreateBatch => "create_batch",
            Permission::EditBatch => "edit_batch",
            Permission::DeleteBatch => "delete_batch",
            Permission::ViewBatch => "view_batch",
            Permission::UseBatch => "use_batch",
            Permission::CreateEquipment => "create_equipment",
            Permission::EditEquipment => "edit_equipment",
            Permission::DeleteEquipment => "delete_equipment",
            Permission::ViewEquipment => "view_equipment",
            Permission::ManageEquipmentMaintenance => "manage_equipment_maintenance",
            Permission::CreateExperiment => "create_experiment",
            Permission::EditExperiment => "edit_experiment",
            Permission::DeleteExperiment => "delete_experiment",
            Permission::ViewExperiment => "view_experiment",
            Permission::CreateRoom => "create_room",
            Permission::EditRoom => "edit_room",
            Permission::DeleteRoom => "delete_room",
            Permission::ViewRoom => "view_room",
            Permission::ViewReports => "view_reports",
            Permission::ExportReports => "export_reports",
            Permission::ImportData => "import_data",
            Permission::ExportData => "export_data",
            Permission::ViewAuditLog => "view_audit_log",
            Permission::ManageSystem => "manage_system",
        }
    }
}

/// Helper function to get permissions list for a role
pub fn get_role_permissions(role: &UserRole) -> Vec<Permission> {
    match role {
        UserRole::Admin => vec![
            // All permissions
            Permission::ManageUsers,
            Permission::ViewUsers,
            Permission::CreateReagent,
            Permission::EditReagent,
            Permission::DeleteReagent,
            Permission::ViewReagent,
            Permission::CreateBatch,
            Permission::EditBatch,
            Permission::DeleteBatch,
            Permission::ViewBatch,
            Permission::UseBatch,
            Permission::CreateEquipment,
            Permission::EditEquipment,
            Permission::DeleteEquipment,
            Permission::ViewEquipment,
            Permission::ManageEquipmentMaintenance,
            Permission::CreateExperiment,
            Permission::EditExperiment,
            Permission::DeleteExperiment,
            Permission::ViewExperiment,
            Permission::CreateRoom,
            Permission::EditRoom,
            Permission::DeleteRoom,
            Permission::ViewRoom,
            Permission::ViewReports,
            Permission::ExportReports,
            Permission::ImportData,
            Permission::ExportData,
            Permission::ViewAuditLog,
            Permission::ManageSystem,
        ],
        UserRole::Researcher => vec![
            // Create, edit, view but limited delete
            Permission::ViewUsers,
            Permission::CreateReagent,
            Permission::EditReagent,
            Permission::ViewReagent,
            Permission::CreateBatch,
            Permission::EditBatch,
            Permission::ViewBatch,
            Permission::UseBatch,
            Permission::CreateEquipment,
            Permission::EditEquipment,
            Permission::ViewEquipment,
            Permission::ManageEquipmentMaintenance,
            Permission::CreateExperiment,
            Permission::EditExperiment,
            Permission::ViewExperiment,
            Permission::CreateRoom,
            Permission::EditRoom,
            Permission::ViewRoom,
            Permission::ViewReports,
            Permission::ExportReports,
            Permission::ExportData,
        ],
        UserRole::Viewer => vec![
            // View only + use batch
            Permission::ViewReagent,
            Permission::ViewBatch,
            Permission::UseBatch,
            Permission::ViewEquipment,
            Permission::ViewExperiment,
            Permission::ViewRoom,
            Permission::ViewReports,
        ],
    }
}

// ======== AUTH HANDLERS ========

pub async fn login(
    app_state: web::Data<Arc<AppState>>,
    auth_service: web::Data<Arc<AuthService>>,
    request: web::Json<LoginRequest>,
) -> ApiResult<HttpResponse> {
    request.validate()?;

    // Find user by username
    let mut user = User::find_by_username(&app_state.db_pool, &request.username).await
        .map_err(|_| ApiError::BadRequest("Invalid username or password".to_string()))?;

    // Check if user is locked
    if user.is_locked() {
        return Err(ApiError::AuthError("Account is temporarily locked. Try again later.".to_string()));
    }

    // Verify password
    if !auth_service.verify_password(&request.password, &user.password_hash)
        .map_err(|_| ApiError::InternalServerError("Password verification failed".to_string()))? {

        // Increment failed attempts
        user.increment_failed_attempts(&app_state.db_pool).await?;

        // Lock for 15 minutes after 5 failed attempts
        if user.failed_login_attempts >= 5 {
            user.lock_for_duration(&app_state.db_pool, Duration::minutes(15)).await?;
            return Err(ApiError::AuthError(
                "Account locked due to too many failed attempts. Try again in 15 minutes.".to_string()
            ));
        }

        return Err(ApiError::BadRequest("Invalid username or password".to_string()));
    }

    // Check if lock has expired and reset
    if let Some(locked_until) = user.locked_until {
        if Utc::now() > locked_until {
            user.reset_failed_attempts(&app_state.db_pool).await?;
        }
    }

    // Reset failed attempts on successful login
    user.reset_failed_attempts(&app_state.db_pool).await?;

    // Update last login
    user.update_last_login(&app_state.db_pool).await?;

    // Generate token
    let token = auth_service.generate_token(&user)?;

    let response = LoginResponse {
        token,
        expires_in: 24 * 3600, // 24 hours in seconds
        user: user.clone().into(),
    };

    log::info!("User {} logged in successfully", user.username);

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        response,
        "Login successful".to_string(),
    )))
}

// FIXED: Register handler with transaction to prevent race condition
pub async fn register(
    app_state: web::Data<Arc<AppState>>,
    auth_service: web::Data<Arc<AuthService>>,
    request: web::Json<RegisterRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    request.validate()?;

    // Determine user role with transaction to prevent race condition
    let role = if let Ok(claims) = get_current_user(&http_request) {
        // Admin is creating a new user
        check_permission(&claims, |role| role.can_manage_users())?;

        // Admin can specify role or default to Viewer
        if let Some(role_str) = &request.role {
            UserRole::from_str(role_str)
                .ok_or_else(|| ApiError::BadRequest("Invalid role specified".to_string()))?
        } else {
            UserRole::Viewer
        }
    } else {
        // FIXED: Use transaction to prevent race condition on first user
        let mut tx = app_state.db_pool.begin().await?;
        
        // Lock users table and count within transaction
        let user_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users"
        )
        .fetch_one(&mut *tx)
        .await?;

        let role = if user_count.0 == 0 {
            UserRole::Admin // First user becomes admin
        } else {
            UserRole::Viewer // Self-registration only allows Viewer
        };
        
        // Commit transaction to release lock
        tx.commit().await?;
        
        role
    };

    // Create user (this will start its own transaction internally)
    let user = User::create(&app_state.db_pool, request.into_inner(), role, &auth_service).await?;

    // Generate token
    let token = auth_service.generate_token(&user)?;

    let response = LoginResponse {
        token,
        expires_in: 24 * 3600,
        user: user.into(),
    };

    log::info!("New user registered: {} with role {:?}", response.user.username, response.user.role);

    Ok(HttpResponse::Created().json(ApiResponse::success_with_message(
        response,
        "User registered successfully".to_string(),
    )))
}

pub async fn get_profile(http_request: HttpRequest) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;

    // Get role permissions
    let permissions: Vec<String> = get_role_permissions(&claims.role)
        .iter()
        .map(|p| p.as_str().to_string())
        .collect();

    #[derive(Serialize)]
    struct ProfileResponse {
        #[serde(flatten)]
        user: UserInfo,
        permissions: Vec<String>,
    }

    let user_info = UserInfo {
        id: claims.sub,
        username: claims.username,
        email: claims.email,
        role: claims.role.clone(),
        is_active: true,
        last_login: None,
    };

    let response = ProfileResponse {
        user: user_info,
        permissions,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(response)))
}

pub async fn change_password(
    app_state: web::Data<Arc<AppState>>,
    auth_service: web::Data<Arc<AuthService>>,
    request: web::Json<ChangePasswordRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    request.validate()?;
    let claims = get_current_user(&http_request)?;

    let user = User::find_by_id(&app_state.db_pool, &claims.sub).await?;

    // Use the change_password method from User
    user.change_password(
        &app_state.db_pool,
        &request.current_password,
        &request.new_password,
        &auth_service
    ).await?;

    log::info!("User {} changed password", user.username);

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        (),
        "Password changed successfully".to_string(),
    )))
}

// ======== USER MANAGEMENT (ADMIN) ========

pub async fn get_users(
    app_state: web::Data<Arc<AppState>>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;
    check_permission(&claims, |role| role.can_manage_users())?;

    let users: Vec<User> = sqlx::query_as("SELECT * FROM users ORDER BY created_at DESC")
        .fetch_all(&app_state.db_pool)
        .await?;

    let user_infos: Vec<UserInfo> = users.into_iter().map(|u| u.into()).collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(user_infos)))
}

pub async fn get_user(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let user_id = path.into_inner();
    let claims = get_current_user(&http_request)?;
    check_permission(&claims, |role| role.can_manage_users())?;

    let user = User::find_by_id(&app_state.db_pool, &user_id).await?;
    let user_info: UserInfo = user.into();

    Ok(HttpResponse::Ok().json(ApiResponse::success(user_info)))
}

/// Create a new user (admin only)
pub async fn create_user(
    app_state: web::Data<Arc<AppState>>,
    auth_service: web::Data<Arc<AuthService>>,
    request: web::Json<CreateUserRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;
    check_permission(&claims, |role| role.can_manage_users())?;

    request.validate()?;

    // Validate role
    let role = UserRole::from_str(&request.role)
        .ok_or_else(|| ApiError::BadRequest(format!(
            "Invalid role '{}'. Valid roles: admin, researcher, viewer", 
            request.role
        )))?;

    // Check if username already exists
    let existing_username: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM users WHERE username = ?"
    )
    .bind(&request.username)
    .fetch_optional(&app_state.db_pool)
    .await?;

    if existing_username.is_some() {
        return Err(ApiError::BadRequest(format!(
            "Username '{}' already exists", 
            request.username
        )));
    }

    // Check if email already exists
    let existing_email: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM users WHERE email = ?"
    )
    .bind(&request.email)
    .fetch_optional(&app_state.db_pool)
    .await?;

    if existing_email.is_some() {
        return Err(ApiError::BadRequest(format!(
            "Email '{}' already exists", 
            request.email
        )));
    }

    // Hash password
    let password_hash = auth_service.hash_password(&request.password)
        .map_err(|e| ApiError::InternalServerError(format!("Failed to hash password: {}", e)))?;

    let now = Utc::now();
    let id = Uuid::new_v4().to_string();

    // Create user
    sqlx::query(
        r#"INSERT INTO users (
            id, username, email, password_hash, role, is_active,
            created_at, updated_at, failed_login_attempts, locked_until
        ) VALUES (?, ?, ?, ?, ?, 1, ?, ?, 0, NULL)"#
    )
    .bind(&id)
    .bind(&request.username)
    .bind(&request.email)
    .bind(&password_hash)
    .bind(role.as_str())
    .bind(&now)
    .bind(&now)
    .execute(&app_state.db_pool)
    .await?;

    // Fetch created user
    let user = User::find_by_id(&app_state.db_pool, &id).await?;
    let user_info: UserInfo = user.into();

    log::info!(
        "Admin {} created user {} with role {:?}", 
        claims.username, request.username, role
    );

    Ok(HttpResponse::Created().json(ApiResponse::success_with_message(
        CreateUserResponse {
            user: user_info,
            generated_password: None, // Password was provided by admin
        },
        "User created successfully".to_string(),
    )))
}

pub async fn update_user(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<UpdateUserRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let user_id = path.into_inner();
    let claims = get_current_user(&http_request)?;
    check_permission(&claims, |role| role.can_manage_users())?;

    request.validate()?;

    let now = Utc::now();

    // Validate role if provided
    if let Some(ref role_str) = request.role {
        if UserRole::from_str(role_str).is_none() {
            return Err(ApiError::BadRequest(format!(
                "Invalid role '{}'. Valid roles: admin, researcher, viewer",
                role_str
            )));
        }
    }

    // Prevent admin from demoting themselves
    if user_id == claims.sub {
        if let Some(ref role_str) = request.role {
            if role_str != claims.role.as_str() {
                return Err(ApiError::BadRequest(
                    "Cannot change your own role".to_string()
                ));
            }
        }
        if let Some(is_active) = request.is_active {
            if !is_active {
                return Err(ApiError::BadRequest(
                    "Cannot deactivate your own account".to_string()
                ));
            }
        }
    }

    // Build update query based on provided fields
    if let Some(ref role) = request.role {
        if let Some(is_active) = request.is_active {
            // Update both role and status
            let result = sqlx::query(
                "UPDATE users SET updated_at = ?, role = ?, is_active = ? WHERE id = ?"
            )
                .bind(now)
                .bind(role)
                .bind(is_active)
                .bind(&user_id)
                .execute(&app_state.db_pool)
                .await?;

            if result.rows_affected() > 0 {
                log::info!("Admin {} updated user {} (role and status)", claims.username, user_id);
                return Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
                    (),
                    "User updated successfully".to_string(),
                )));
            }
        } else {
            // Update only role
            let result = sqlx::query(
                "UPDATE users SET updated_at = ?, role = ? WHERE id = ?"
            )
                .bind(now)
                .bind(role)
                .bind(&user_id)
                .execute(&app_state.db_pool)
                .await?;

            if result.rows_affected() > 0 {
                log::info!("Admin {} updated user {} (role only)", claims.username, user_id);
                return Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
                    (),
                    "User updated successfully".to_string(),
                )));
            }
        }
    } else if let Some(is_active) = request.is_active {
        // Update only status
        let result = sqlx::query(
            "UPDATE users SET updated_at = ?, is_active = ? WHERE id = ?"
        )
            .bind(now)
            .bind(is_active)
            .bind(&user_id)
            .execute(&app_state.db_pool)
            .await?;

        if result.rows_affected() > 0 {
            log::info!("Admin {} updated user {} (status only)", claims.username, user_id);
            return Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
                (),
                "User updated successfully".to_string(),
            )));
        }
    } else {
        // Update only timestamp
        let result = sqlx::query(
            "UPDATE users SET updated_at = ? WHERE id = ?"
        )
            .bind(now)
            .bind(&user_id)
            .execute(&app_state.db_pool)
            .await?;

        if result.rows_affected() > 0 {
            log::info!("Admin {} updated user {} (timestamp only)", claims.username, user_id);
            return Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
                (),
                "User updated successfully".to_string(),
            )));
        }
    }

    Err(ApiError::NotFound("User not found".to_string()))
}

pub async fn change_user_password(
    app_state: web::Data<Arc<AppState>>,
    auth_service: web::Data<Arc<AuthService>>,
    path: web::Path<String>,
    request: web::Json<ChangeUserPasswordRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let user_id = path.into_inner();
    let claims = get_current_user(&http_request)?;
    check_permission(&claims, |role| role.can_manage_users())?;

    request.validate()?;

    // Hash new password
    let new_password_hash = auth_service.hash_password(&request.new_password)
        .map_err(|_| ApiError::InternalServerError("Password hashing failed".to_string()))?;

    // Update password and reset lock
    let result = sqlx::query(
        "UPDATE users SET password_hash = ?, updated_at = datetime('now'), failed_login_attempts = 0, locked_until = NULL WHERE id = ?"
    )
        .bind(&new_password_hash)
        .bind(&user_id)
        .execute(&app_state.db_pool)
        .await?;

    if result.rows_affected() > 0 {
        log::info!("Admin {} changed password for user {}", claims.username, user_id);
        Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
            (),
            "Password changed successfully".to_string(),
        )))
    } else {
        Err(ApiError::NotFound("User not found".to_string()))
    }
}

pub async fn delete_user(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let user_id = path.into_inner();
    let claims = get_current_user(&http_request)?;
    check_permission(&claims, |role| role.can_manage_users())?;

    // Prevent admin from deleting their own account
    if user_id == claims.sub {
        return Err(ApiError::BadRequest("Cannot delete your own account".to_string()));
    }

    // Check if this is the last admin
    let admin_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM users WHERE role = 'admin' AND is_active = 1"
    )
    .fetch_one(&app_state.db_pool)
    .await?;

    let target_user = User::find_by_id(&app_state.db_pool, &user_id).await?;
    
    if target_user.role == "admin" && admin_count.0 <= 1 {
        return Err(ApiError::BadRequest(
            "Cannot delete the last admin user".to_string()
        ));
    }

    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&user_id)
        .execute(&app_state.db_pool)
        .await?;

    if result.rows_affected() > 0 {
        log::info!("Admin {} deleted user {}", claims.username, user_id);
        Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
            (),
            "User deleted successfully".to_string(),
        )))
    } else {
        Err(ApiError::NotFound("User not found".to_string()))
    }
}

/// Get available roles
pub async fn get_roles(
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;
    check_permission(&claims, |role| role.can_manage_users())?;

    #[derive(Serialize)]
    struct RoleInfo {
        id: String,
        name: String,
        description: String,
        permissions: Vec<String>,
    }

    let roles = vec![
        RoleInfo {
            id: "admin".to_string(),
            name: "Administrator".to_string(),
            description: "Full access to all system features".to_string(),
            permissions: get_role_permissions(&UserRole::Admin).iter().map(|p| p.as_str().to_string()).collect(),
        },
        RoleInfo {
            id: "researcher".to_string(),
            name: "Researcher".to_string(),
            description: "Can create and edit data, limited delete permissions".to_string(),
            permissions: get_role_permissions(&UserRole::Researcher).iter().map(|p| p.as_str().to_string()).collect(),
        },
        RoleInfo {
            id: "viewer".to_string(),
            name: "Viewer".to_string(),
            description: "Read-only access with ability to use batches".to_string(),
            permissions: get_role_permissions(&UserRole::Viewer).iter().map(|p| p.as_str().to_string()).collect(),
        },
    ];

    Ok(HttpResponse::Ok().json(ApiResponse::success(roles)))
}

// ======== PERMISSION CHECK FUNCTIONS ========

pub fn check_reagent_permission(
    http_request: &HttpRequest,
    action: ReagentAction,
) -> ApiResult<()> {
    let claims = get_current_user(http_request)?;

    match action {
        ReagentAction::Create => check_permission(&claims, |role| role.can_create_reagents()),
        ReagentAction::Edit => check_permission(&claims, |role| role.can_edit_reagents()),
        ReagentAction::Delete => check_permission(&claims, |role| role.can_delete_reagents()),
        ReagentAction::View => Ok(()), // All can view
    }
}

pub fn check_batch_permission(
    http_request: &HttpRequest,
    action: BatchAction,
) -> ApiResult<()> {
    let claims = get_current_user(http_request)?;

    match action {
        BatchAction::Create => check_permission(&claims, |role| role.can_create_batches()),
        BatchAction::Edit => check_permission(&claims, |role| role.can_edit_batches()),
        BatchAction::Delete => check_permission(&claims, |role| role.can_delete_batches()),
        BatchAction::View => Ok(()), // All can view
    }
}

pub fn check_equipment_permission(
    http_request: &HttpRequest,
    action: EquipmentAction,
) -> ApiResult<()> {
    let claims = get_current_user(http_request)?;

    match action {
        EquipmentAction::Create | EquipmentAction::Edit | EquipmentAction::Delete =>
            check_permission(&claims, |role| role.can_manage_equipment()),
        EquipmentAction::View => Ok(()), // All can view
    }
}

pub fn check_experiment_permission(
    http_request: &HttpRequest,
    action: ExperimentAction,
) -> ApiResult<()> {
    let claims = get_current_user(http_request)?;

    match action {
        ExperimentAction::Create => check_permission(&claims, |role| role.can_create_experiments()),
        ExperimentAction::Edit => check_permission(&claims, |role| role.can_edit_experiments()),
        ExperimentAction::Delete => check_permission(&claims, |role| role.can_delete_experiments()),
        ExperimentAction::View => Ok(()), // All can view
    }
}

pub fn check_room_permission(
    http_request: &HttpRequest,
    action: RoomAction,
) -> ApiResult<()> {
    let claims = get_current_user(http_request)?;

    match action {
        RoomAction::Create => check_permission(&claims, |role| role.can_create_rooms()),
        RoomAction::Edit => check_permission(&claims, |role| role.can_edit_rooms()),
        RoomAction::Delete => check_permission(&claims, |role| role.can_delete_rooms()),
        RoomAction::View => Ok(()), // All can view
    }
}

// ======== ACTION ENUMS ========

pub enum ReagentAction {
    Create,
    Edit,
    Delete,
    View,
}

pub enum BatchAction {
    Create,
    Edit,
    Delete,
    View,
}

pub enum EquipmentAction {
    Create,
    Edit,
    Delete,
    View,
}

pub enum ExperimentAction {
    Create,
    Edit,
    Delete,
    View,
}

pub enum RoomAction {
    Create,
    Edit,
    Delete,
    View,
}