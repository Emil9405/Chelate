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
    #[validate(length(min = 3, max = 50, message = "Username must be 3-50 characters"))]
    pub username: Option<String>,
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
    #[validate(length(max = 100, message = "Name cannot exceed 100 characters"))]
    pub name: Option<String>,
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
    #[validate(length(max = 100, message = "Name cannot exceed 100 characters"))]
    pub name: Option<String>,
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
    http_request: HttpRequest,
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
    crate::audit::audit(
    &app_state.db_pool, &user.id, "login", "user", &user.id,
    &format!("User {} logged in", user.username), &http_request).await;

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

    // Save before into() consumes user
    let user_id = user.id.clone();
    let user_name = user.username.clone();

    // Generate token
    let token = auth_service.generate_token(&user)?;

    let response = LoginResponse {
        token,
        expires_in: 24 * 3600,
        user: user.into(),
    };

    crate::audit::audit(
        &app_state.db_pool, &user_id, "register", "user", &user_id,
        &format!("New user registered: {}", user_name), &http_request,
    ).await;

    log::info!("New user registered: {} with role {:?}", response.user.username, response.user.role);

    Ok(HttpResponse::Created().json(ApiResponse::success_with_message(
        response,
        "User registered successfully".to_string(),
    )))
}

pub async fn get_profile(
    app_state: web::Data<Arc<AppState>>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let claims = get_current_user(&http_request)?;

    // Fetch full user from database
    let user = User::find_by_id(&app_state.db_pool, &claims.sub).await?;
    let user_info: UserInfo = user.into();

    // Get role-based permissions
    let mut permissions: std::collections::HashSet<String> = get_role_permissions(&claims.role)
        .iter()
        .map(|p| p.as_str().to_string())
        .collect();

    // Load custom permissions from user_permissions table
    let custom_perms: Option<(String,)> = sqlx::query_as(
        "SELECT permissions FROM user_permissions WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_optional(&app_state.db_pool)
    .await
    .unwrap_or(None);

    if let Some((perms_json,)) = custom_perms {
        if let Ok(perms) = serde_json::from_str::<std::collections::HashMap<String, bool>>(&perms_json) {
            for (key, value) in perms {
                if value {
                    permissions.insert(key);
                }
            }
        }
    }

    let permissions_vec: Vec<String> = permissions.into_iter().collect();

    #[derive(Serialize)]
    struct ProfileResponse {
        #[serde(flatten)]
        user: UserInfo,
        permissions: Vec<String>,
    }

    let response = ProfileResponse {
        user: user_info,
        permissions: permissions_vec,
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

    crate::audit::audit(
        &app_state.db_pool, &claims.sub, "change_password", "user", &claims.sub,
        &format!("User {} changed password", user.username), &http_request,
    ).await;

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
            id, username, email, password_hash, name, role, is_active,
            created_at, updated_at, failed_login_attempts, locked_until
        ) VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?, 0, NULL)"#
    )
    .bind(&id)
    .bind(&request.username)
    .bind(&request.email)
    .bind(&password_hash)
    .bind(&request.name)
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

    crate::audit::audit(
        &app_state.db_pool, &claims.sub, "create_user", "user", &id,
        &format!("Admin created user: {}", request.username), &http_request,
    ).await;

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
            // Case-insensitive comparison
            if role_str.to_lowercase() != claims.role.as_str().to_lowercase() {
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

    // Fetch existing user to check for conflicts
    let existing_user = User::find_by_id(&app_state.db_pool, &user_id).await?;

    // Check username uniqueness if changing
    if let Some(ref new_username) = request.username {
        if new_username != &existing_user.username {
            let conflict: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM users WHERE username = ? AND id != ?"
            )
            .bind(new_username)
            .bind(&user_id)
            .fetch_optional(&app_state.db_pool)
            .await?;

            if conflict.is_some() {
                return Err(ApiError::BadRequest(format!(
                    "Username '{}' already exists", new_username
                )));
            }
        }
    }

    // Check email uniqueness if changing
    if let Some(ref new_email) = request.email {
        if new_email != &existing_user.email {
            let conflict: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM users WHERE email = ? AND id != ?"
            )
            .bind(new_email)
            .bind(&user_id)
            .fetch_optional(&app_state.db_pool)
            .await?;

            if conflict.is_some() {
                return Err(ApiError::BadRequest(format!(
                    "Email '{}' already exists", new_email
                )));
            }
        }
    }

    // Build dynamic update query
    let mut updates = vec!["updated_at = ?".to_string()];
    let mut has_changes = false;

    if request.username.is_some() { updates.push("username = ?".to_string()); has_changes = true; }
    if request.email.is_some() { updates.push("email = ?".to_string()); has_changes = true; }
    if request.name.is_some() { updates.push("name = ?".to_string()); has_changes = true; }
    if request.role.is_some() { updates.push("role = ?".to_string()); has_changes = true; }
    if request.is_active.is_some() { updates.push("is_active = ?".to_string()); has_changes = true; }

    let sql = format!("UPDATE users SET {} WHERE id = ?", updates.join(", "));

    // Build query with dynamic bindings
    let mut query = sqlx::query(&sql).bind(now);

    if let Some(ref username) = request.username { query = query.bind(username); }
    if let Some(ref email) = request.email { query = query.bind(email); }
    if let Some(ref name) = request.name { query = query.bind(name); }
    if let Some(ref role) = request.role { 
        // Normalize role to lowercase for storage
        query = query.bind(role.to_lowercase()); 
    }
    if let Some(is_active) = request.is_active { query = query.bind(is_active); }

    query = query.bind(&user_id);

    let result = query.execute(&app_state.db_pool).await?;

    if result.rows_affected() > 0 {
        log::info!("Admin {} updated user {}", claims.username, user_id);
        crate::audit::audit(
            &app_state.db_pool, &claims.sub, "update_user", "user", &user_id,
            &format!("Updated user {}", user_id), &http_request,
        ).await;
        return Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
            (),
            "User updated successfully".to_string(),
        )));
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
        crate::audit::audit(
            &app_state.db_pool, &claims.sub, "change_user_password", "user", &user_id,
            &format!("Admin reset password for user {}", user_id), &http_request,
        ).await;
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
        crate::audit::audit(
            &app_state.db_pool, &claims.sub, "delete_user", "user", &user_id,
            &format!("Admin deleted user {}", target_user.username), &http_request,
        ).await;
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

/// Async version that checks custom permissions from user_permissions table
pub async fn check_batch_permission_async(
    http_request: &HttpRequest,
    action: BatchAction,
    pool: &sqlx::SqlitePool,
) -> ApiResult<()> {
    let claims = get_current_user(http_request)?;

    // First check role-based permissions
    let role_allowed = match action {
        BatchAction::Create => claims.role.can_create_batches(),
        BatchAction::Edit => claims.role.can_edit_batches(),
        BatchAction::Delete => claims.role.can_delete_batches(),
        BatchAction::View => true,
    };

    if role_allowed {
        log::debug!("User {} allowed {:?} via role {:?}", claims.username, action, claims.role);
        return Ok(());
    }

    // If role doesn't allow, check custom permissions from user_permissions table
    let permission_key = match action {
        BatchAction::Create => "create_batch",
        BatchAction::Edit => "edit_batch",
        BatchAction::Delete => "delete_batch",
        BatchAction::View => return Ok(()),
    };

    log::info!("Checking custom permissions for user {} ({}), action: {}", claims.username, claims.sub, permission_key);

    let result: Option<(String,)> = sqlx::query_as(
        "SELECT permissions FROM user_permissions WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        log::error!("DB error checking permissions: {:?}", e);
        ApiError::InternalServerError("Database error".to_string())
    })?;

    if let Some((perms_json,)) = result {
        log::info!("Found custom permissions for user {}: {}", claims.username, perms_json);
        match serde_json::from_str::<std::collections::HashMap<String, bool>>(&perms_json) {
            Ok(perms) => {
                log::info!("Parsed permissions: {:?}", perms);
                if perms.get(permission_key).copied().unwrap_or(false) {
                    log::info!("User {} granted {} via custom permissions", claims.username, permission_key);
                    return Ok(());
                } else {
                    log::info!("Permission {} not granted for user {} (value: {:?})", 
                        permission_key, claims.username, perms.get(permission_key));
                }
            }
            Err(e) => {
                log::error!("Failed to parse permissions JSON for user {}: {:?}", claims.username, e);
            }
        }
    } else {
        log::info!("No custom permissions found for user {} ({})", claims.username, claims.sub);
    }

    Err(ApiError::Forbidden("Insufficient permissions".to_string()))
}

/// Async version for reagent permissions
pub async fn check_reagent_permission_async(
    http_request: &HttpRequest,
    action: ReagentAction,
    pool: &sqlx::SqlitePool,
) -> ApiResult<()> {
    let claims = get_current_user(http_request)?;

    let role_allowed = match action {
        ReagentAction::Create => claims.role.can_create_reagents(),
        ReagentAction::Edit => claims.role.can_edit_reagents(),
        ReagentAction::Delete => claims.role.can_delete_reagents(),
        ReagentAction::View => true,
    };

    if role_allowed {
        log::debug!("User {} allowed {:?} via role {:?}", claims.username, action, claims.role);
        return Ok(());
    }

    let permission_key = match action {
        ReagentAction::Create => "create_reagent",
        ReagentAction::Edit => "edit_reagent",
        ReagentAction::Delete => "delete_reagent",
        ReagentAction::View => return Ok(()),
    };

    log::info!("Checking custom permissions for user {} ({}), action: {}", claims.username, claims.sub, permission_key);

    let result: Option<(String,)> = sqlx::query_as(
        "SELECT permissions FROM user_permissions WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        log::error!("DB error checking permissions: {:?}", e);
        ApiError::InternalServerError("Database error".to_string())
    })?;

    if let Some((perms_json,)) = result {
        log::info!("Found custom permissions for user {}: {}", claims.username, perms_json);
        match serde_json::from_str::<std::collections::HashMap<String, bool>>(&perms_json) {
            Ok(perms) => {
                log::info!("Parsed permissions: {:?}", perms);
                if perms.get(permission_key).copied().unwrap_or(false) {
                    log::info!("User {} granted {} via custom permissions", claims.username, permission_key);
                    return Ok(());
                } else {
                    log::info!("Permission {} not granted for user {} (value: {:?})", 
                        permission_key, claims.username, perms.get(permission_key));
                }
            }
            Err(e) => {
                log::error!("Failed to parse permissions JSON for user {}: {:?}", claims.username, e);
            }
        }
    } else {
        log::info!("No custom permissions found for user {} ({})", claims.username, claims.sub);
    }

    Err(ApiError::Forbidden("Insufficient permissions".to_string()))
}

pub async fn check_equipment_permission(
    http_request: &HttpRequest,
    action: EquipmentAction,
    pool: &sqlx::SqlitePool,
) -> ApiResult<()> {
    let claims = get_current_user(http_request)?;

    let role_allowed = match action {
        EquipmentAction::Create | EquipmentAction::Edit | EquipmentAction::Delete =>
            claims.role.can_manage_equipment(),
        EquipmentAction::View => true,
    };

    if role_allowed {
        return Ok(());
    }

    let permission_key = match action {
        EquipmentAction::Create => "create_equipment",
        EquipmentAction::Edit => "edit_equipment",
        EquipmentAction::Delete => "delete_equipment",
        EquipmentAction::View => return Ok(()),
    };

    let result: Option<(String,)> = sqlx::query_as(
        "SELECT permissions FROM user_permissions WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::InternalServerError("Database error".to_string()))?;

    if let Some((perms_json,)) = result {
        if let Ok(perms) = serde_json::from_str::<std::collections::HashMap<String, bool>>(&perms_json) {
            if perms.get(permission_key).copied().unwrap_or(false) {
                return Ok(());
            }
        }
    }

    Err(ApiError::Forbidden("Insufficient permissions".to_string()))
}

pub async fn check_experiment_permission(
    http_request: &HttpRequest,
    action: ExperimentAction,
    pool: &sqlx::SqlitePool,
) -> ApiResult<()> {
    let claims = get_current_user(http_request)?;

    let role_allowed = match action {
        ExperimentAction::Create => claims.role.can_create_experiments(),
        ExperimentAction::Edit => claims.role.can_edit_experiments(),
        ExperimentAction::Delete => claims.role.can_delete_experiments(),
        ExperimentAction::View => true,
    };

    if role_allowed {
        return Ok(());
    }

    let permission_key = match action {
        ExperimentAction::Create => "create_experiment",
        ExperimentAction::Edit => "edit_experiment",
        ExperimentAction::Delete => "delete_experiment",
        ExperimentAction::View => return Ok(()),
    };

    let result: Option<(String,)> = sqlx::query_as(
        "SELECT permissions FROM user_permissions WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::InternalServerError("Database error".to_string()))?;

    if let Some((perms_json,)) = result {
        if let Ok(perms) = serde_json::from_str::<std::collections::HashMap<String, bool>>(&perms_json) {
            if perms.get(permission_key).copied().unwrap_or(false) {
                return Ok(());
            }
        }
    }

    Err(ApiError::Forbidden("Insufficient permissions".to_string()))
}

pub async fn check_room_permission(
    http_request: &HttpRequest,
    action: RoomAction,
    pool: &sqlx::SqlitePool,
) -> ApiResult<()> {
    let claims = get_current_user(http_request)?;

    let role_allowed = match action {
        RoomAction::Create => claims.role.can_create_rooms(),
        RoomAction::Edit => claims.role.can_edit_rooms(),
        RoomAction::Delete => claims.role.can_delete_rooms(),
        RoomAction::View => true,
    };

    if role_allowed {
        return Ok(());
    }

    let permission_key = match action {
        RoomAction::Create => "create_room",
        RoomAction::Edit => "edit_room",
        RoomAction::Delete => "delete_room",
        RoomAction::View => return Ok(()),
    };

    let result: Option<(String,)> = sqlx::query_as(
        "SELECT permissions FROM user_permissions WHERE user_id = ?"
    )
    .bind(&claims.sub)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::InternalServerError("Database error".to_string()))?;

    if let Some((perms_json,)) = result {
        if let Ok(perms) = serde_json::from_str::<std::collections::HashMap<String, bool>>(&perms_json) {
            if perms.get(permission_key).copied().unwrap_or(false) {
                return Ok(());
            }
        }
    }

    Err(ApiError::Forbidden("Insufficient permissions".to_string()))
}

// ======== ACTION ENUMS ========

#[derive(Debug)]
pub enum ReagentAction {
    Create,
    Edit,
    Delete,
    View,
}

#[derive(Debug)]
pub enum BatchAction {
    Create,
    Edit,
    Delete,
    View,
}

#[derive(Debug)]
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

// ======== USER PERMISSIONS HANDLERS ========

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserPermissionsResponse {
    pub user_id: String,
    pub permissions: std::collections::HashMap<String, bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePermissionsRequest {
    pub permissions: std::collections::HashMap<String, bool>,
}

/// Get user permissions
pub async fn get_user_permissions(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let user_id = path.into_inner();
    let claims = get_current_user(&http_request)?;
    check_permission(&claims, |role| role.can_manage_users())?;

    // Try to get custom permissions from database
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT permissions FROM user_permissions WHERE user_id = ?"
    )
    .bind(&user_id)
    .fetch_optional(&app_state.db_pool)
    .await?;

    let permissions: std::collections::HashMap<String, bool> = if let Some((perms_json,)) = result {
        serde_json::from_str(&perms_json).unwrap_or_default()
    } else {
        // Return default permissions based on user role
        let user = User::find_by_id(&app_state.db_pool, &user_id).await?;
        get_default_permissions_for_role(&user.role)
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(UserPermissionsResponse {
        user_id,
        permissions,
    })))
}

/// Update user permissions
pub async fn update_user_permissions(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    request: web::Json<UpdatePermissionsRequest>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let user_id = path.into_inner();
    let claims = get_current_user(&http_request)?;
    check_permission(&claims, |role| role.can_manage_users())?;

    // Verify user exists
    let target_user = User::find_by_id(&app_state.db_pool, &user_id).await?;

    let permissions_json = serde_json::to_string(&request.permissions)
        .map_err(|_| ApiError::InternalServerError("Failed to serialize permissions".to_string()))?;

    log::info!("Admin {} saving permissions for user {} ({}): {}", 
        claims.username, target_user.username, user_id, permissions_json);

    // Upsert permissions
    sqlx::query(
        r#"
        INSERT INTO user_permissions (user_id, permissions, created_at, updated_at)
        VALUES (?, ?, datetime('now'), datetime('now'))
        ON CONFLICT(user_id) DO UPDATE SET
            permissions = excluded.permissions,
            updated_at = datetime('now')
        "#
    )
    .bind(&user_id)
    .bind(&permissions_json)
    .execute(&app_state.db_pool)
    .await?;

    log::info!("Admin {} updated permissions for user {}", claims.username, user_id);

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        (),
        "Permissions updated successfully".to_string(),
    )))
}

/// Get default permissions based on role
fn get_default_permissions_for_role(role: &str) -> std::collections::HashMap<String, bool> {
    let mut perms = std::collections::HashMap::new();
    
    match role.to_lowercase().as_str() {
        "admin" => {
            for perm in &[
                "create_reagent", "edit_reagent", "delete_reagent", "view_reagent",
                "create_batch", "edit_batch", "delete_batch", "view_batch", "use_batch",
                "create_equipment", "edit_equipment", "delete_equipment", "view_equipment", "manage_maintenance",
                "create_experiment", "edit_experiment", "delete_experiment", "view_experiment",
                "create_room", "edit_room", "delete_room", "view_room",
                "view_reports", "export_reports", "import_data", "export_data",
                "view_audit_log", "manage_users", "manage_system",
            ] {
                perms.insert(perm.to_string(), true);
            }
        }
        "researcher" => {
            for perm in &[
                "create_reagent", "edit_reagent", "view_reagent",
                "create_batch", "edit_batch", "view_batch", "use_batch",
                "create_equipment", "edit_equipment", "view_equipment", "manage_maintenance",
                "create_experiment", "edit_experiment", "view_experiment",
                "create_room", "edit_room", "view_room",
                "view_reports", "export_reports", "export_data",
            ] {
                perms.insert(perm.to_string(), true);
            }
        }
        _ => {
            // viewer and others
            for perm in &[
                "view_reagent", "view_batch", "use_batch",
                "view_equipment", "view_experiment", "view_room",
                "view_reports",
            ] {
                perms.insert(perm.to_string(), true);
            }
        }
    }
    
    perms
}

// ======== USER ACTIVITY HISTORY ========

#[derive(Debug, Serialize)]
pub struct ActivityRecord {
    pub id: String,
    pub user_id: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub description: Option<String>,
    pub changes: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ActivityQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub action_type: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

/// Get user activity history
pub async fn get_user_activity(
    app_state: web::Data<Arc<AppState>>,
    path: web::Path<String>,
    query: web::Query<ActivityQuery>,
    http_request: HttpRequest,
) -> ApiResult<HttpResponse> {
    let user_id = path.into_inner();
    let claims = get_current_user(&http_request)?;
    check_permission(&claims, |role| role.can_manage_users())?;

    let limit = query.limit.unwrap_or(100).min(500);
    let offset = query.offset.unwrap_or(0);

    // Build query with filters
    let mut sql = String::from(
        r#"
        SELECT id, user_id, action, entity_type, entity_id, 
               description, changes, ip_address, created_at
        FROM audit_logs
        WHERE user_id = ?
        "#
    );

    let mut conditions = Vec::new();
    
    if let Some(ref action_type) = query.action_type {
        if action_type != "all" {
            conditions.push(format!("LOWER(action) LIKE '%{}%'", action_type.to_lowercase().replace('\'', "''")));
        }
    }
    
    if let Some(ref date_from) = query.date_from {
        conditions.push(format!("created_at >= '{}'", date_from.replace('\'', "''")));
    }
    
    if let Some(ref date_to) = query.date_to {
        conditions.push(format!("created_at <= '{}T23:59:59'", date_to.replace('\'', "''")));
    }

    for cond in conditions {
        sql.push_str(&format!(" AND {}", cond));
    }

    sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

    let rows: Vec<(String, Option<String>, String, String, Option<String>, Option<String>, Option<String>, Option<String>, String)> = 
        sqlx::query_as(&sql)
            .bind(&user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&app_state.db_pool)
            .await?;

    let activities: Vec<ActivityRecord> = rows.into_iter()
        .map(|(id, user_id, action, entity_type, entity_id, description, changes, ip_address, created_at)| {
            ActivityRecord {
                id,
                user_id,
                action,
                entity_type,
                entity_id,
                description,
                changes,
                ip_address,
                created_at,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(activities)))
}