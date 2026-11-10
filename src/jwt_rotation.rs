// src/jwt_rotation.rs - Automatic JWT secret rotation
//!
//! Every `ROTATION_INTERVAL_DAYS` a new signing secret is generated and
//!   1. written to the env file the server was started from, together with the
//!      secret it replaces (`JWT_SECRET`, `JWT_SECRET_PREVIOUS`),
//!   2. put to use by the running server at once (`AuthService::rotate`).
//!
//! Tokens signed with the replaced secret stay valid until they expire, so
//! neither the rotation nor a later restart signs anyone out. Before, the new
//! secret only went to the file: the running server kept signing with the old
//! one indefinitely, and the next restart invalidated every session.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokio::time;

use crate::auth::AuthService;

const JWT_SECRET_LENGTH: usize = 64;
const ROTATION_INTERVAL_DAYS: i64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct JwtRotationRecord {
    pub id: i32,
    /// Fingerprint of the secret, never the secret itself
    pub secret_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_active: bool,
}

fn generate_jwt_secret() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(JWT_SECRET_LENGTH)
        .map(char::from)
        .collect()
}

/// Short fingerprint for the rotation log (to tell secrets apart, not a credential)
fn hash_secret(secret: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    secret.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Sets `KEY=value` lines in env-file text: an existing line is replaced in
/// place, a missing one is appended. Other lines are left untouched.
fn set_env_values(content: &str, values: &[(&str, &str)]) -> String {
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    for (key, value) in values {
        let prefix = format!("{}=", key);
        let new_line = format!("{}={}", key, value);
        match lines.iter_mut().find(|l| l.trim_start().starts_with(&prefix)) {
            Some(line) => *line = new_line,
            None => lines.push(new_line),
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Writes the new secret (and the one it replaces) to the env file
pub fn update_env_file(env_path: &str, new_secret: &str, previous_secret: Option<&str>) -> Result<()> {
    let path = Path::new(env_path);
    let content = if path.exists() { fs::read_to_string(path)? } else { String::new() };

    let mut values = vec![("JWT_SECRET", new_secret)];
    if let Some(previous) = previous_secret {
        values.push(("JWT_SECRET_PREVIOUS", previous));
    }
    let new_content = set_env_values(&content, &values);

    // No backup copies: each one kept a full copy of .env (old JWT secret,
    // admin password) next to the project and they ended up in the repository.
    // Write to a temp file and rename, so a crash never leaves a truncated .env.
    let tmp_path = format!("{}.tmp", env_path);
    write_private(&tmp_path, &new_content).context("Failed to write temporary .env file")?;
    fs::rename(&tmp_path, path).context("Failed to update .env file")?;

    log::info!("✓ JWT secret written to {}", env_path);
    Ok(())
}

/// Write a file readable only by the owner (0600 on Unix)
fn write_private(path: &str, content: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(content.as_bytes())?;
    f.sync_all()
}

pub async fn init_rotation_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS jwt_rotation_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            secret_hash TEXT NOT NULL,
            created_at DATETIME NOT NULL,
            expires_at DATETIME NOT NULL,
            is_active BOOLEAN NOT NULL DEFAULT 1
        )
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create jwt_rotation_log table")?;
    Ok(())
}

async fn get_active_rotation_record(pool: &SqlitePool) -> Result<Option<JwtRotationRecord>> {
    let record = sqlx::query_as(
        "SELECT * FROM jwt_rotation_log WHERE is_active = 1 ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(record)
}

/// Replaces the active record of the rotation log with one for `secret`
async fn record_secret(pool: &SqlitePool, secret: &str) -> Result<DateTime<Utc>> {
    let now = Utc::now();
    let expires_at = now + ChronoDuration::days(ROTATION_INTERVAL_DAYS);
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE jwt_rotation_log SET is_active = 0 WHERE is_active = 1")
        .execute(&mut *tx)
        .await
        .context("Failed to deactivate old keys")?;
    sqlx::query(
        "INSERT INTO jwt_rotation_log (secret_hash, created_at, expires_at, is_active) VALUES (?, ?, ?, 1)",
    )
    .bind(hash_secret(secret))
    .bind(now)
    .bind(expires_at)
    .execute(&mut *tx)
    .await
    .context("Failed to insert new rotation record")?;
    tx.commit().await?;
    Ok(expires_at)
}

/// What the rotation task has to do now
#[derive(Debug, PartialEq, Eq)]
enum Due {
    /// The secret in use is the recorded one and has not expired
    Nothing,
    /// No record yet, or the secret was replaced by hand in the env file:
    /// start counting from now, nothing to rotate
    Record,
    Rotate,
}

async fn what_is_due(pool: &SqlitePool, auth: &AuthService) -> Result<Due> {
    Ok(match get_active_rotation_record(pool).await? {
        None => Due::Record,
        Some(r) if r.secret_hash != hash_secret(&auth.current_secret()) => Due::Record,
        Some(r) if Utc::now() >= r.expires_at => Due::Rotate,
        Some(_) => Due::Nothing,
    })
}

/// Rotates the secret: env file first, then the running server. If the file
/// cannot be written nothing changes, so the file and the server never disagree.
pub async fn rotate_jwt_secret(pool: &SqlitePool, env_path: &str, auth: &AuthService) -> Result<DateTime<Utc>> {
    let old_secret = auth.current_secret();
    let new_secret = generate_jwt_secret();

    update_env_file(env_path, &new_secret, Some(&old_secret)).context("Failed to update .env file")?;
    auth.rotate(&new_secret);
    let expires_at = record_secret(pool, &new_secret).await?;

    log::info!("✓ JWT secret rotated and in use; next rotation at {}", expires_at);
    Ok(expires_at)
}

async fn run_check(pool: &SqlitePool, env_path: &str, auth: &AuthService) -> Result<()> {
    match what_is_due(pool, auth).await? {
        Due::Nothing => {}
        Due::Record => {
            let expires_at = record_secret(pool, &auth.current_secret()).await?;
            log::info!("JWT secret registered; first rotation at {}", expires_at);
        }
        Due::Rotate => {
            rotate_jwt_secret(pool, env_path, auth).await?;
            log_rotation(pool).await;
        }
    }
    Ok(())
}

/// Background task: checks hourly whether the secret is due for rotation
pub async fn start_rotation_task(pool: SqlitePool, env_path: String, auth: std::sync::Arc<AuthService>) {
    log::info!(
        "🔐 JWT rotation task started (every {} days, env file: {})",
        ROTATION_INTERVAL_DAYS, env_path
    );
    if let Err(e) = init_rotation_table(&pool).await {
        log::error!("Failed to initialize rotation table: {}", e);
        return;
    }

    let mut interval = time::interval(Duration::from_secs(3600));
    loop {
        // the first tick completes immediately
        interval.tick().await;
        if let Err(e) = run_check(&pool, &env_path, &auth).await {
            log::error!("JWT rotation check failed: {:#}", e);
        }
    }
}

/// System entry in the audit log
async fn log_rotation(pool: &SqlitePool) {
    let result = sqlx::query(
        r#"INSERT INTO audit_logs (id, user_id, action, entity_type, entity_id, description, created_at)
           VALUES (?, NULL, 'jwt_rotation', 'system', 'jwt', 'JWT secret rotated automatically', datetime('now'))"#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .execute(pool)
    .await;
    if let Err(e) = result {
        log::warn!("Failed to log rotation in audit_logs: {}", e);
    }
}

pub async fn get_rotation_stats(pool: &SqlitePool) -> Result<RotationStats> {
    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM jwt_rotation_log")
        .fetch_one(pool)
        .await?;
    let active_record = get_active_rotation_record(pool).await?;

    Ok(RotationStats {
        total_rotations: total.0,
        last_rotation: active_record.as_ref().map(|r| r.created_at).unwrap_or_else(Utc::now),
        next_rotation: active_record.as_ref().map(|r| r.expires_at).unwrap_or_else(Utc::now),
        is_active: active_record.is_some(),
    })
}

#[derive(Debug, Serialize)]
pub struct RotationStats {
    pub total_rotations: i64,
    pub last_rotation: DateTime<Utc>,
    pub next_rotation: DateTime<Utc>,
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_jwt_secret() {
        let secret = generate_jwt_secret();
        assert_eq!(secret.len(), JWT_SECRET_LENGTH);
        assert!(secret.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_hash_secret() {
        assert_eq!(hash_secret("test_secret_123"), hash_secret("test_secret_123"));
        assert_ne!(hash_secret("test_secret_123"), hash_secret("different_secret"));
    }

    #[test]
    fn env_values_are_replaced_in_place_and_other_lines_kept() {
        let before = "DATABASE_URL=lims.db\nJWT_SECRET=old\n# comment\nLIMS_PORT=8080";
        let after = set_env_values(before, &[("JWT_SECRET", "new"), ("JWT_SECRET_PREVIOUS", "old")]);
        assert_eq!(
            after,
            "DATABASE_URL=lims.db\nJWT_SECRET=new\n# comment\nLIMS_PORT=8080\nJWT_SECRET_PREVIOUS=old\n"
        );
        // a second rotation replaces both lines and does not confuse the two keys
        let again = set_env_values(&after, &[("JWT_SECRET", "newer"), ("JWT_SECRET_PREVIOUS", "new")]);
        assert_eq!(
            again,
            "DATABASE_URL=lims.db\nJWT_SECRET=newer\n# comment\nLIMS_PORT=8080\nJWT_SECRET_PREVIOUS=new\n"
        );
        assert_eq!(set_env_values("", &[("JWT_SECRET", "x")]), "JWT_SECRET=x\n");
    }
}
