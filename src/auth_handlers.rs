// src/auth_handlers.rs - Authentication route handlers with enhanced permissions

use actix_web::{web, HttpRequest, HttpResponse};
use validator::Validate;
use std::sync::Arc;
use chrono::{Duration, Utc};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::handlers::ApiResponse;
use crate::audit::ChangeSet;
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
        // 403, not 401: the web client treats 401 as "session expired" and
        // reloads the login page, so the user never saw this message
        return Err(ApiError::Forbidden("Account is temporarily locked. Try again later.".to_string()));
    }

    // Verify password
    if !auth_service.verify_password(&request.password, &user.password_hash)
        .map_err(|_| ApiError::InternalServerError("Password verification failed".to_string()))? {

        // Increment failed attempts
        user.increment_failed_attempts(&app_state.db_pool).await?;

        // Lock after too many failed attempts
        // (AUTH_MAX_LOGIN_ATTEMPTS / AUTH_LOCKOUT_DURATION_MINUTES, default 5 / 15)
        let max_attempts = app_state.config.auth.max_login_attempts.max(1);
        let lockout_minutes = app_state.config.auth.lockout_duration_minutes.clamp(1, 7 * 24 * 60) as i64;
        if user.failed_login_attempts >= max_attempts {
            user.lock_for_duration(&app_state.db_pool, Duration::minutes(lockout_minutes)).await?;
            return Err(ApiError::Forbidden(format!(
                "Account locked due to too many failed attempts. Try again in {} minutes.", lockout_minutes
            )));
        }

        return Err(ApiError::BadRequest("Invalid username or password".to_string()));
    }

    // Deactivated accounts cannot sign in (checked after the password so the
    // response does not reveal which usernames exist)
    if !user.is_active {
        return Err(ApiError::Forbidden("Account is disabled. Contact an administrator.".to_string()));
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
        expires_in: auth_service.token_ttl_seconds(),
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

    // Public endpoint (outside the JWT middleware). Accounts are created by an
    // admin via /api/v1/auth/users; self-registration only when explicitly
    // enabled (AUTH_ALLOW_SELF_REGISTRATION=true), and always as Viewer.
    // The first admin is created at startup (create_default_admin_if_needed).
    if !app_state.config.auth.allow_self_registration {
        return Err(ApiError::Forbidden(
            "Self-registration is disabled. Ask an administrator to create an account.".to_string()
        ));
    }
    let role = UserRole::Viewer;

    // Create user (this will start its own transaction internally)
    let user = User::create(&app_state.db_pool, request.into_inner(), role, &auth_service).await?;

    // Save before into() consumes user
    let user_id = user.id.clone();
    let user_name = user.username.clone();

    // Generate token
    let token = auth_service.generate_token(&user)?;

    let response = LoginResponse {
        token,
        expires_in: auth_service.token_ttl_seconds(),
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

    // Exactly the set the server enforces (see src/permissions.rs), so the
    // UI shows a button if and only if the request behind it will be accepted
    let (_, effective) = crate::permissions::effective(&http_request, &app_state.db_pool).await?;
    let permissions_vec: Vec<String> = effective.keys().into_iter().map(str::to_string).collect();

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

    let mut cs = ChangeSet::new();
    cs.created("username", &request.username);
    cs.created("email", &request.email);
    cs.created("role", role.as_str());
    if let Some(ref name) = request.name { cs.created("name", name); }

    crate::audit::audit_with_changes(
        &app_state.db_pool, &claims.sub, "create_user", "user", &id,
        &format!("Created user: {}", cs.to_description()),
        &cs, &http_request,
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
    if request.username.is_some() { updates.push("username = ?".to_string()); }
    if request.email.is_some() { updates.push("email = ?".to_string()); }
    if request.name.is_some() { updates.push("name = ?".to_string()); }
    if request.role.is_some() { updates.push("role = ?".to_string()); }
    if request.is_active.is_some() { updates.push("is_active = ?".to_string()); }

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

    // Custom permissions replace the role's defaults, so a user who keeps them
    // through a role change would keep the old rights (a researcher demoted to
    // viewer could still edit). A new role starts from its own defaults.
    let role_changed = request.role.as_ref()
        .map(|r| r.to_lowercase() != existing_user.role.to_lowercase())
        .unwrap_or(false);
    let mut custom_permissions_reset = false;
    if result.rows_affected() > 0 && role_changed {
        custom_permissions_reset = sqlx::query("DELETE FROM user_permissions WHERE user_id = ?")
            .bind(&user_id)
            .execute(&app_state.db_pool)
            .await?
            .rows_affected() > 0;
    }

    if result.rows_affected() > 0 {
        // Build detailed change log
        let mut cs = ChangeSet::new();
        if custom_permissions_reset {
            cs.add("custom_permissions", "set", "reset to role defaults");
        }
        if let Some(ref new_username) = request.username {
            cs.add("username", &existing_user.username, new_username);
        }
        if let Some(ref new_email) = request.email {
            cs.add("email", &existing_user.email, new_email);
        }
        if let Some(ref new_name) = request.name {
            cs.add_opt("name", &existing_user.name, &Some(new_name.clone()));
        }
        if let Some(ref new_role) = request.role {
            cs.add("role", &existing_user.role, &new_role.to_lowercase());
        }
        if let Some(new_active) = request.is_active {
            cs.add_bool("is_active", existing_user.is_active, new_active);
        }

        let desc = if cs.has_changes() {
            format!("User {} updated: {}", existing_user.username, cs.to_description())
        } else {
            format!("User {} updated (no significant changes)", existing_user.username)
        };

        log::info!("Admin {} updated user {}: {}", claims.username, user_id, desc);
        crate::audit::audit_with_changes(
            &app_state.db_pool, &claims.sub, "update_user", "user", &user_id,
            &desc, &cs, &http_request,
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
        let mut cs = ChangeSet::new();
        cs.deleted("username", &target_user.username);
        cs.deleted("email", &target_user.email);
        cs.deleted("role", &target_user.role);

        log::info!("Admin {} deleted user {}", claims.username, user_id);
        crate::audit::audit_with_changes(
            &app_state.db_pool, &claims.sub, "delete_user", "user", &user_id,
            &format!("Deleted user: {}", cs.to_description()),
            &cs, &http_request,
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
            permissions: crate::permissions::role_defaults(&UserRole::Admin).iter().map(|k| k.to_string()).collect(),
        },
        RoleInfo {
            id: "researcher".to_string(),
            name: "Researcher".to_string(),
            description: "Can create and edit data, limited delete permissions".to_string(),
            permissions: crate::permissions::role_defaults(&UserRole::Researcher).iter().map(|k| k.to_string()).collect(),
        },
        RoleInfo {
            id: "viewer".to_string(),
            name: "Viewer".to_string(),
            description: "Read-only access with ability to use batches".to_string(),
            permissions: crate::permissions::role_defaults(&UserRole::Viewer).iter().map(|k| k.to_string()).collect(),
        },
    ];

    Ok(HttpResponse::Ok().json(ApiResponse::success(roles)))
}

// ======== PERMISSION CHECK FUNCTIONS ========
//
// Thin wrappers that turn "action on a kind of record" into a permission key.
// The decision itself is made in one place, crate::permissions.

pub async fn check_reagent_permission_async(
    http_request: &HttpRequest,
    action: ReagentAction,
    pool: &sqlx::SqlitePool,
) -> ApiResult<()> {
    let key = match action {
        ReagentAction::Create => "create_reagent",
        ReagentAction::Edit => "edit_reagent",
        ReagentAction::Delete => "delete_reagent",
        ReagentAction::View => "view_reagent",
    };
    crate::permissions::require(http_request, pool, key).await.map(|_| ())
}

pub async fn check_batch_permission_async(
    http_request: &HttpRequest,
    action: BatchAction,
    pool: &sqlx::SqlitePool,
) -> ApiResult<()> {
    let key = match action {
        BatchAction::Create => "create_batch",
        BatchAction::Edit => "edit_batch",
        BatchAction::Delete => "delete_batch",
        BatchAction::View => "view_batch",
    };
    crate::permissions::require(http_request, pool, key).await.map(|_| ())
}

pub async fn check_equipment_permission(
    http_request: &HttpRequest,
    action: EquipmentAction,
    pool: &sqlx::SqlitePool,
) -> ApiResult<()> {
    let key = match action {
        EquipmentAction::Create => "create_equipment",
        EquipmentAction::Edit => "edit_equipment",
        EquipmentAction::Delete => "delete_equipment",
        EquipmentAction::View => "view_equipment",
        EquipmentAction::Maintenance => "manage_maintenance",
    };
    crate::permissions::require(http_request, pool, key).await.map(|_| ())
}

pub async fn check_experiment_permission(
    http_request: &HttpRequest,
    action: ExperimentAction,
    pool: &sqlx::SqlitePool,
) -> ApiResult<()> {
    let key = match action {
        ExperimentAction::Create => "create_experiment",
        ExperimentAction::Edit => "edit_experiment",
        ExperimentAction::Delete => "delete_experiment",
        ExperimentAction::View => "view_experiment",
    };
    crate::permissions::require(http_request, pool, key).await.map(|_| ())
}

pub async fn check_room_permission(
    http_request: &HttpRequest,
    action: RoomAction,
    pool: &sqlx::SqlitePool,
) -> ApiResult<()> {
    let key = match action {
        RoomAction::Create => "create_room",
        RoomAction::Edit => "edit_room",
        RoomAction::Delete => "delete_room",
        RoomAction::View => "view_room",
    };
    crate::permissions::require(http_request, pool, key).await.map(|_| ())
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
    /// Maintenance records (`manage_maintenance`)
    Maintenance,
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

    // What this user effectively has now: saved custom permissions, or the
    // defaults of the role when nothing was customised
    let user = User::find_by_id(&app_state.db_pool, &user_id).await?;
    let permissions = crate::permissions::load(&app_state.db_pool, &user_id, &user.get_role())
        .await?
        .as_map();

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

    // Admins always have everything (they could re-grant it to themselves
    // anyway), so custom permissions for an admin would be a setting that
    // looks active and does nothing
    if target_user.get_role() == UserRole::Admin {
        return Err(ApiError::BadRequest(
            "Administrators always have every permission. Change the role first to restrict this user.".to_string()
        ));
    }

    // Only known keys are stored; user management and system keys come with
    // the admin role and cannot be granted individually
    let stored = crate::permissions::sanitize(&request.permissions);
    let permissions_json = serde_json::to_string(&stored)
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

    let mut cs = ChangeSet::new();
    cs.created("permissions", &permissions_json);
    crate::audit::audit_with_changes(
        &app_state.db_pool, &claims.sub, "update_permissions", "user", &user_id,
        &format!("Updated permissions for user {}", target_user.username),
        &cs, &http_request,
    ).await;

    Ok(HttpResponse::Ok().json(ApiResponse::success_with_message(
        (),
        "Permissions updated successfully".to_string(),
    )))
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
        r#"SELECT id, user_id, action, entity_type, entity_id,
               description, changes, ip_address, created_at
        FROM audit_logs WHERE user_id = ?"#
    );
    let mut params: Vec<String> = vec![user_id.clone()];

    if let Some(ref action_type) = query.action_type {
        if action_type != "all" {
            sql.push_str(" AND ");
            sql.push_str(&crate::query_builders::utils::like_sql("LOWER(action)"));
            params.push(crate::query_builders::utils::like_contains(&action_type.to_lowercase()));
        }
    }
    if let Some(ref date_from) = query.date_from {
        sql.push_str(" AND created_at >= ?");
        params.push(date_from.clone());
    }
    if let Some(ref date_to) = query.date_to {
        sql.push_str(" AND created_at <= ?");
        params.push(format!("{}T23:59:59", date_to));
    }

    sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

    let mut db_query = sqlx::query_as::<_, (
        String, Option<String>, String, String, Option<String>,
        Option<String>, Option<String>, Option<String>, String,
    )>(&sql);

    for p in &params {
        db_query = db_query.bind(p);
    }
    db_query = db_query.bind(limit).bind(offset);

    let rows = db_query.fetch_all(&app_state.db_pool).await?;

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

// ======== PERMISSION BY KEY ========

/// Check a permission key (e.g. "import_data"). `_role_default` is kept for
/// the existing call sites; role defaults now live in crate::permissions.
pub async fn require_permission_key(
    http_request: &HttpRequest,
    pool: &sqlx::SqlitePool,
    key: &str,
    _role_default: fn(&UserRole) -> bool,
) -> ApiResult<crate::auth::Claims> {
    crate::permissions::require(http_request, pool, key).await
}
