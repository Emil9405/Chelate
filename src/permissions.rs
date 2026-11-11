// src/permissions.rs
//! One place that decides what a signed-in user may do.
//!
//! Rules:
//!   1. Admins have every permission and cannot be restricted. (An admin can
//!      edit anyone's permissions, their own included, so a restriction on an
//!      admin would only be a way to lock the system by mistake.)
//!   2. A user with saved custom permissions (the "Permissions" dialog) has
//!      exactly the keys switched on there. A key that is missing counts as off.
//!   3. Everyone else gets the defaults of their role.
//!   4. `ADMIN_ONLY` keys follow the role and are never granted individually:
//!      whoever manages users can make themselves an admin, so handing that
//!      out through a checkbox would be handing out the admin role.
//!
//! The JWT middleware resolves the set once per request (one query, joined
//! with the account check) and stores it in the request; handlers and the
//! profile endpoint read the same set, so the UI and the server cannot
//! disagree about what is allowed.

use std::collections::{BTreeSet, HashMap};

use actix_web::{http::Method, HttpMessage, HttpRequest};
use sqlx::SqlitePool;

use crate::auth::{get_current_user, Claims, UserRole};
use crate::error::{ApiError, ApiResult};

/// Every permission key, in the order the UI shows them
pub const ALL: &[&str] = &[
    "create_reagent", "edit_reagent", "delete_reagent", "view_reagent",
    "create_batch", "edit_batch", "delete_batch", "view_batch", "use_batch",
    "create_equipment", "edit_equipment", "delete_equipment", "view_equipment", "manage_maintenance",
    "create_experiment", "edit_experiment", "delete_experiment", "view_experiment",
    "create_room", "edit_room", "delete_room", "view_room",
    "view_reports", "export_reports", "import_data", "export_data",
    "view_audit_log", "manage_users", "manage_system",
];

/// Granted by the admin role only (rule 4)
pub const ADMIN_ONLY: &[&str] = &["view_audit_log", "manage_users", "manage_system"];

const RESEARCHER: &[&str] = &[
    "create_reagent", "edit_reagent", "view_reagent",
    "create_batch", "edit_batch", "view_batch", "use_batch",
    "create_equipment", "edit_equipment", "view_equipment", "manage_maintenance",
    "create_experiment", "edit_experiment", "view_experiment",
    "create_room", "edit_room", "view_room",
    "view_reports", "export_reports", "export_data",
];

const VIEWER: &[&str] = &[
    "view_reagent", "view_batch", "use_batch",
    "view_equipment", "view_experiment", "view_room",
    "view_reports",
];

pub fn role_defaults(role: &UserRole) -> &'static [&'static str] {
    match role {
        UserRole::Admin => ALL,
        UserRole::Researcher => RESEARCHER,
        UserRole::Viewer => VIEWER,
    }
}

/// The canonical `&'static str` for a key, None for unknown keys
fn known(key: &str) -> Option<&'static str> {
    ALL.iter().copied().find(|k| *k == key)
}

/// The permissions a user actually has
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectivePermissions(BTreeSet<&'static str>);

impl EffectivePermissions {
    /// `custom_json` is the user's row in `user_permissions`, if any
    pub fn resolve(role: &UserRole, custom_json: Option<&str>) -> Self {
        if *role == UserRole::Admin {
            return Self(ALL.iter().copied().collect());
        }
        let custom = custom_json.and_then(|json| {
            match serde_json::from_str::<HashMap<String, bool>>(json) {
                Ok(map) => Some(map),
                Err(e) => {
                    // Unreadable settings must not widen or wipe access: use the role
                    log::error!("Unreadable custom permissions, using role defaults: {}", e);
                    None
                }
            }
        });
        let set = match custom {
            Some(map) => map
                .iter()
                .filter(|(_, on)| **on)
                .filter_map(|(key, _)| known(key))
                .filter(|key| !ADMIN_ONLY.contains(key))
                .collect(),
            None => role_defaults(role).iter().copied().collect(),
        };
        Self(set)
    }

    pub fn has(&self, key: &str) -> bool {
        self.0.contains(key)
    }

    /// Keys in a stable order, for the profile response
    pub fn keys(&self) -> Vec<&'static str> {
        ALL.iter().copied().filter(|k| self.0.contains(k)).collect()
    }

    /// Full on/off map, the shape the permissions dialog edits
    pub fn as_map(&self) -> HashMap<String, bool> {
        ALL.iter().map(|k| (k.to_string(), self.0.contains(k))).collect()
    }
}

/// Keeps only known, individually grantable keys of a submitted map, so what
/// is stored is exactly what will be enforced
pub fn sanitize(submitted: &HashMap<String, bool>) -> HashMap<String, bool> {
    ALL.iter()
        .filter(|k| !ADMIN_ONLY.contains(k))
        .map(|k| (k.to_string(), submitted.get(*k).copied().unwrap_or(false)))
        .collect()
}

pub async fn load(pool: &SqlitePool, user_id: &str, role: &UserRole) -> ApiResult<EffectivePermissions> {
    let custom: Option<(String,)> = sqlx::query_as("SELECT permissions FROM user_permissions WHERE user_id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    Ok(EffectivePermissions::resolve(role, custom.as_ref().map(|(json,)| json.as_str())))
}

/// The current user's permissions: the set the middleware stored in the
/// request, or (requests built without the middleware, e.g. tests) from the database
pub async fn effective(req: &HttpRequest, pool: &SqlitePool) -> ApiResult<(Claims, EffectivePermissions)> {
    let claims = get_current_user(req)?;
    if let Some(perms) = req.extensions().get::<EffectivePermissions>().cloned() {
        return Ok((claims, perms));
    }
    let perms = load(pool, &claims.sub, &claims.role).await?;
    Ok((claims, perms))
}

/// Fails with 403 unless the current user has `key`
pub async fn require(req: &HttpRequest, pool: &SqlitePool, key: &str) -> ApiResult<Claims> {
    let (claims, perms) = effective(req, pool).await?;
    if perms.has(key) {
        Ok(claims)
    } else {
        log::info!("User {} denied '{}'", claims.username, key);
        Err(ApiError::Forbidden(format!("Insufficient permissions: '{}' is required", key)))
    }
}

/// Permission needed just to reach a route, checked in the JWT middleware.
/// This is what makes the "view" and "export" switches real: reads go
/// straight to their handlers, which have no checks of their own. Writes are
/// checked by their handlers (create/edit/delete/use keys).
pub fn route_permission(method: &Method, path: &str) -> Option<&'static str> {
    let rest = path.strip_prefix("/api/v1/")?;
    let mut segments = rest.split('/').filter(|s| !s.is_empty());
    let area = segments.next()?;
    let tail: Vec<&str> = segments.collect();
    let last = tail.last().copied().unwrap_or("");
    let is_get = method == Method::GET;
    // POST endpoints that only read
    let is_filter = method == Method::POST && last == "filter";

    match area {
        "reports" => Some(if last == "export" { "export_reports" } else { "view_reports" }),
        _ if is_get && tail.len() == 1 && last == "export" => Some("export_data"),
        "reagents" if is_get => Some(if tail.contains(&"batches") { "view_batch" } else { "view_reagent" }),
        "batches" | "containers" if is_get || is_filter => Some("view_batch"),
        "equipment" if is_get => Some("view_equipment"),
        "experiments" if is_get || is_filter => Some("view_experiment"),
        "rooms" | "storage" if is_get => Some("view_room"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_has_everything_and_cannot_be_restricted() {
        let p = EffectivePermissions::resolve(&UserRole::Admin, Some("{}"));
        assert_eq!(p.keys().len(), ALL.len());
        assert!(p.has("manage_users"));
    }

    #[test]
    fn role_defaults_apply_without_custom_permissions() {
        let viewer = EffectivePermissions::resolve(&UserRole::Viewer, None);
        assert!(viewer.has("view_batch") && viewer.has("use_batch"));
        assert!(!viewer.has("edit_batch") && !viewer.has("export_data"));

        let researcher = EffectivePermissions::resolve(&UserRole::Researcher, None);
        assert!(researcher.has("edit_batch") && researcher.has("manage_maintenance"));
        assert!(!researcher.has("delete_batch") && !researcher.has("import_data"));
    }

    #[test]
    fn custom_permissions_replace_the_role_in_both_directions() {
        // a viewer who was given batch editing, and nothing that is not listed
        let p = EffectivePermissions::resolve(&UserRole::Viewer, Some(r#"{"view_batch":true,"edit_batch":true}"#));
        assert!(p.has("edit_batch") && p.has("view_batch"));
        assert!(!p.has("view_reagent"), "a missing key is off");

        // a researcher with a default right switched off
        let p = EffectivePermissions::resolve(&UserRole::Researcher, Some(r#"{"view_batch":true,"edit_batch":false}"#));
        assert!(!p.has("edit_batch"));
    }

    #[test]
    fn admin_only_keys_are_never_granted_individually() {
        let p = EffectivePermissions::resolve(
            &UserRole::Researcher,
            Some(r#"{"manage_users":true,"manage_system":true,"view_audit_log":true,"made_up":true}"#),
        );
        assert!(p.keys().is_empty());
        let stored = sanitize(&HashMap::from([("manage_users".to_string(), true), ("edit_room".to_string(), true)]));
        assert!(!stored.contains_key("manage_users"));
        assert_eq!(stored.get("edit_room"), Some(&true));
        assert_eq!(stored.get("view_room"), Some(&false));
    }

    #[test]
    fn unreadable_custom_permissions_fall_back_to_the_role() {
        let p = EffectivePermissions::resolve(&UserRole::Viewer, Some("not json"));
        assert_eq!(p, EffectivePermissions::resolve(&UserRole::Viewer, None));
    }

    #[test]
    fn routes_map_to_view_and_export_keys() {
        let get = Method::GET;
        let post = Method::POST;
        assert_eq!(route_permission(&get, "/api/v1/reagents"), Some("view_reagent"));
        assert_eq!(route_permission(&get, "/api/v1/reagents/abc/details"), Some("view_reagent"));
        assert_eq!(route_permission(&get, "/api/v1/reagents/abc/batches/b1/usage"), Some("view_batch"));
        assert_eq!(route_permission(&get, "/api/v1/reagents/export"), Some("export_data"));
        assert_eq!(route_permission(&get, "/api/v1/batches/export"), Some("export_data"));
        assert_eq!(route_permission(&get, "/api/v1/equipment/export"), Some("export_data"));
        assert_eq!(route_permission(&get, "/api/v1/batches/low-stock"), Some("view_batch"));
        assert_eq!(route_permission(&post, "/api/v1/batches/filter"), Some("view_batch"));
        assert_eq!(route_permission(&get, "/api/v1/containers/c1/history"), Some("view_batch"));
        assert_eq!(route_permission(&get, "/api/v1/equipment/e1/files/f1"), Some("view_equipment"));
        assert_eq!(route_permission(&post, "/api/v1/experiments/filter"), Some("view_experiment"));
        assert_eq!(route_permission(&get, "/api/v1/storage/hierarchy"), Some("view_room"));
        assert_eq!(route_permission(&get, "/api/v1/rooms/r1/inventory"), Some("view_room"));
        assert_eq!(route_permission(&post, "/api/v1/reports/generate"), Some("view_reports"));
        assert_eq!(route_permission(&post, "/api/v1/reports/export"), Some("export_reports"));
        // writes are checked by their handlers, and these areas are open to every signed-in user
        assert_eq!(route_permission(&post, "/api/v1/reagents"), None);
        assert_eq!(route_permission(&Method::DELETE, "/api/v1/batches/b1"), None);
        assert_eq!(route_permission(&get, "/api/v1/dashboard/stats"), None);
        assert_eq!(route_permission(&get, "/api/v1/auth/profile"), None);
        assert_eq!(route_permission(&get, "/api/v1/labels/resolve"), None);
    }
}
