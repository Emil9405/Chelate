// ============================================================
// FILE: src/audit.rs — Enhanced audit logging module with change tracking
// ============================================================

use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::Utc;
use actix_web::HttpRequest;
use serde::{Serialize, Deserialize};

// ==================== CHANGE TRACKING ====================

/// Single field change: old value -> new value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// Set of changes to be stored in audit_logs.changes as JSON
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangeSet {
    pub changes: Vec<FieldChange>,
}

impl ChangeSet {
    pub fn new() -> Self {
        Self { changes: Vec::new() }
    }

    /// Add a string field change
    pub fn add(&mut self, field: &str, old_val: &str, new_val: &str) -> &mut Self {
        if old_val != new_val {
            self.changes.push(FieldChange {
                field: field.to_string(),
                old_value: Some(old_val.to_string()),
                new_value: Some(new_val.to_string()),
            });
        }
        self
    }

    /// Add a change for Option<String> fields
    pub fn add_opt(&mut self, field: &str, old_val: &Option<String>, new_val: &Option<String>) -> &mut Self {
        let old_s = old_val.as_deref().unwrap_or("");
        let new_s = new_val.as_deref().unwrap_or("");
        if old_s != new_s {
            self.changes.push(FieldChange {
                field: field.to_string(),
                old_value: Some(old_s.to_string()),
                new_value: Some(new_s.to_string()),
            });
        }
        self
    }

    /// Add a numeric field change (f64)
    pub fn add_f64(&mut self, field: &str, old_val: f64, new_val: f64) -> &mut Self {
        if (old_val - new_val).abs() > f64::EPSILON {
            self.changes.push(FieldChange {
                field: field.to_string(),
                old_value: Some(format!("{}", old_val)),
                new_value: Some(format!("{}", new_val)),
            });
        }
        self
    }

    /// Add a change for Option<f64> fields
    pub fn add_opt_f64(&mut self, field: &str, old_val: Option<f64>, new_val: Option<f64>) -> &mut Self {
        match (old_val, new_val) {
            (Some(o), Some(n)) if (o - n).abs() > f64::EPSILON => {
                self.changes.push(FieldChange {
                    field: field.to_string(),
                    old_value: Some(format!("{}", o)),
                    new_value: Some(format!("{}", n)),
                });
            }
            (None, Some(n)) => {
                self.changes.push(FieldChange {
                    field: field.to_string(),
                    old_value: None,
                    new_value: Some(format!("{}", n)),
                });
            }
            (Some(o), None) => {
                self.changes.push(FieldChange {
                    field: field.to_string(),
                    old_value: Some(format!("{}", o)),
                    new_value: None,
                });
            }
            _ => {}
        }
        self
    }

    /// Add an integer field change (i64)
    pub fn add_i64(&mut self, field: &str, old_val: i64, new_val: i64) -> &mut Self {
        if old_val != new_val {
            self.changes.push(FieldChange {
                field: field.to_string(),
                old_value: Some(format!("{}", old_val)),
                new_value: Some(format!("{}", new_val)),
            });
        }
        self
    }

    /// Add a boolean field change
    pub fn add_bool(&mut self, field: &str, old_val: bool, new_val: bool) -> &mut Self {
        if old_val != new_val {
            self.changes.push(FieldChange {
                field: field.to_string(),
                old_value: Some(format!("{}", old_val)),
                new_value: Some(format!("{}", new_val)),
            });
        }
        self
    }

    /// Record creation of a new entity (new values only, no old values)
    pub fn created(&mut self, field: &str, value: &str) -> &mut Self {
        if !value.is_empty() {
            self.changes.push(FieldChange {
                field: field.to_string(),
                old_value: None,
                new_value: Some(value.to_string()),
            });
        }
        self
    }

    /// Record deleted values (old values only, no new values)
    pub fn deleted(&mut self, field: &str, value: &str) -> &mut Self {
        if !value.is_empty() {
            self.changes.push(FieldChange {
                field: field.to_string(),
                old_value: Some(value.to_string()),
                new_value: None,
            });
        }
        self
    }

    /// Whether any changes were recorded
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    /// Number of changes
    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Option<String> {
        if self.changes.is_empty() {
            return None;
        }
        serde_json::to_string(&self.changes).ok()
    }

    /// Build a human-readable description of all changes
    pub fn to_description(&self) -> String {
        if self.changes.is_empty() {
            return "No changes".to_string();
        }

        self.changes
            .iter()
            .map(|c| {
                match (&c.old_value, &c.new_value) {
                    (None, Some(new)) => format!("{}: set to \"{}\"", c.field, new),
                    (Some(old), None) => format!("{}: \"{}\" (removed)", c.field, old),
                    (Some(old), Some(new)) => format!("{}: \"{}\" -> \"{}\"", c.field, old, new),
                    (None, None) => format!("{}: changed", c.field),
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

// ==================== CORE AUDIT FUNCTIONS ====================

/// Write an event to audit_logs (full version)
pub async fn log_activity(
    pool: &SqlitePool,
    user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    description: Option<&str>,
    changes: Option<&str>,
    request: Option<&HttpRequest>,
) -> Result<(), sqlx::Error> {
    let ip_address = request.and_then(|req| {
        req.connection_info()
            .realip_remote_addr()
            .map(|s| s.to_string())
    });

    let user_agent = request.and_then(|req| {
        req.headers()
            .get("User-Agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    });

    insert_audit_row(
        pool, user_id, action, entity_type, entity_id, description, changes,
        ip_address, user_agent,
    ).await
}

/// Audit entry for background/system actions (no user, no HTTP request).
/// Safe to call from `tokio::spawn`ed tasks: holds no `HttpRequest` across awaits.
pub async fn log_system_activity(
    pool: &SqlitePool,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    description: &str,
) -> Result<(), sqlx::Error> {
    insert_audit_row(
        pool, None, action, entity_type, Some(entity_id), Some(description), None,
        None, None,
    ).await
}

#[allow(clippy::too_many_arguments)]
async fn insert_audit_row(
    pool: &SqlitePool,
    user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    description: Option<&str>,
    changes: Option<&str>,
    ip_address: Option<String>,
    user_agent: Option<String>,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        r#"INSERT INTO audit_logs 
           (id, user_id, action, entity_type, entity_id, description, changes, ip_address, user_agent, created_at)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
    .bind(&id)
    .bind(user_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(description)
    .bind(changes)
    .bind(&ip_address)
    .bind(&user_agent)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Short version for frequent calls (without changes)
pub async fn audit(
    pool: &SqlitePool,
    user_id: &str,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    description: &str,
    request: &HttpRequest,
) {
    if let Err(e) = log_activity(
        pool,
        Some(user_id),
        action,
        entity_type,
        Some(entity_id),
        Some(description),
        None,
        Some(request),
    ).await {
        log::error!("Failed to write audit log: {}", e);
    }
}

/// Extended version with ChangeSet — writes JSON changes to the changes field
pub async fn audit_with_changes(
    pool: &SqlitePool,
    user_id: &str,
    action: &str,
    entity_type: &str,
    entity_id: &str,
    description: &str,
    changeset: &ChangeSet,
    request: &HttpRequest,
) {
    let changes_json = changeset.to_json();

    if let Err(e) = log_activity(
        pool,
        Some(user_id),
        action,
        entity_type,
        Some(entity_id),
        Some(description),
        changes_json.as_deref(),
        Some(request),
    ).await {
        log::error!("Failed to write audit log with changes: {}", e);
    }
}
