// src/config.rs - Configuration management with hot reload support
use serde::Deserialize;
use std::env;
use std::sync::{Arc, RwLock};
use anyhow::{Context, Result};
use rand::{thread_rng, Rng, distributions::Alphanumeric};
use std::path::Path;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
    pub hot_reload: HotReloadConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HotReloadConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub watch_paths: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
    pub keep_alive: u64,
    pub client_timeout: u64,
    pub client_shutdown: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
    pub idle_timeout: u64,
    pub backup_enabled: bool,
    pub backup_interval_hours: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_expiration_hours: i64,
    pub bcrypt_cost: u32,
    pub max_login_attempts: u32,
    pub lockout_duration_minutes: u64,
    pub allow_self_registration: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    pub allowed_origins: Vec<String>,
    pub rate_limit_requests: u32,
    pub rate_limit_window_seconds: u64,
    pub max_request_size: usize,
    pub require_https: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub file_enabled: bool,
    pub file_path: Option<String>,
    pub console_enabled: bool,
}

// Dummy defaults for tests (no ENV read here)
impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            // Auto-generate a secure secret if none provided via .env
            jwt_secret: generate_jwt_secret(),
            token_expiration_hours: 24,
            bcrypt_cost: 10,
            max_login_attempts: 5,
            lockout_duration_minutes: 15,
            allow_self_registration: false,
        }
    }
}

// Defaults for other structs (no ENV)
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            workers: None,
            keep_alive: 30,
            client_timeout: 30,
            client_shutdown: 5,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "lims.db".to_string(),
            max_connections: 10,
            min_connections: 1,
            connect_timeout: 30,
            idle_timeout: 600,
            backup_enabled: true,
            backup_interval_hours: 24,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "http://127.0.0.1:8080".to_string(),
                "http://localhost:8080".to_string(),
                
            ],
            rate_limit_requests: 100,
            rate_limit_window_seconds: 60,
            max_request_size: 1024 * 1024,
            require_https: false,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
            file_enabled: true,
            file_path: Some("lims.log".to_string()),
            console_enabled: true,
        }
    }
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: 10,
            watch_paths: vec![".env".to_string(), "config.toml".to_string()],
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            auth: AuthConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
            hot_reload: HotReloadConfig::default(),
        }
    }
}

// Генерация безопасного JWT секрета
pub fn generate_jwt_secret() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

pub fn generate_and_save_jwt_secret() -> Result<String> {
    let secret = generate_jwt_secret();

    if let Ok(env_path) = env::var("ENV_FILE") {
        let path = Path::new(&env_path);
        let mut content = fs::read_to_string(path).unwrap_or_default();
        if !content.contains("JWT_SECRET=") {
            content.push_str(&format!("\nJWT_SECRET={}\n", secret));
            fs::write(path, content)?;
        }
    } else if Path::new(".env").exists() {
        let mut content = fs::read_to_string(".env").unwrap_or_default();
        if !content.contains("JWT_SECRET=") {
            content.push_str(&format!("\nJWT_SECRET={}\n", secret));
            fs::write(".env", content)?;
        }
    }

    Ok(secret)
}

#[derive(Clone)]
pub struct ReloadableConfig(Arc<RwLock<Config>>);

impl ReloadableConfig {
    pub fn new(config: Config) -> Self {
        Self(Arc::new(RwLock::new(config)))
    }

    pub fn get(&self) -> Config {
        self.0.read().unwrap().clone()
    }

    pub fn reload(&self) -> Result<()> {
        let config = load_config()?;
        *self.0.write().unwrap() = config;
        Ok(())
    }
}

pub fn load_config() -> Result<Config> {
    load_env_file()?;

    let mut config = if let Ok(config_file) = env::var("CONFIG_FILE") {
        let path = Path::new(&config_file);
        let config_str = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", config_file))?;
        toml::from_str(&config_str)
            .with_context(|| format!("Failed to parse config file: {}", config_file))?
    } else {
        Config::default()
    };

    override_with_env(&mut config)?;

    // If JWT secret is still too short (no .env, no env var), auto-generate and persist
    if config.auth.jwt_secret.len() < 32 {
        log::warn!("JWT_SECRET too short ({}), auto-generating secure secret...", config.auth.jwt_secret.len());
        let new_secret = generate_jwt_secret();
        
        // Try to persist to .env so next restart uses the same secret
        if let Err(e) = persist_jwt_secret(&new_secret) {
            log::warn!("Could not persist JWT_SECRET to .env: {}. Secret will be regenerated on restart.", e);
        } else {
            log::info!("✓ Generated and saved new JWT_SECRET to .env");
        }
        
        config.auth.jwt_secret = new_secret;
    }

    config.validate()
        .context("Configuration validation failed")?;

    Ok(config)
}

/// Persists JWT secret to the .env file
fn persist_jwt_secret(secret: &str) -> Result<()> {
    let env_path = env::var("ENV_FILE").unwrap_or_else(|_| {
        // Find existing .env or use CWD
        let candidates = get_env_candidate_paths();
        for candidate in &candidates {
            if Path::new(candidate).exists() {
                return candidate.clone();
            }
        }
        ".env".to_string()
    });

    let path = Path::new(&env_path);
    let mut content = fs::read_to_string(path).unwrap_or_default();

    if content.contains("JWT_SECRET=") {
        // Replace existing
        let lines: Vec<String> = content.lines().map(|line| {
            if line.trim().starts_with("JWT_SECRET=") {
                format!("JWT_SECRET={}", secret)
            } else {
                line.to_string()
            }
        }).collect();
        content = lines.join("\n") + "\n";
    } else {
        // Append
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("JWT_SECRET={}\n", secret));
    }

    fs::write(path, content)?;
    Ok(())
}

fn override_with_env(config: &mut Config) -> Result<()> {
    if let Ok(host) = env::var("BIND_ADDRESS") {
        config.server.host = host;
    }
    if let Ok(port_str) = env::var("LIMS_PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            config.server.port = port;
        }
    }
    if let Ok(workers_str) = env::var("LIMS_WORKERS") {
        if let Ok(workers) = workers_str.parse::<usize>() {
            config.server.workers = Some(workers);
        }
    }
    if let Ok(jwt_secret) = env::var("JWT_SECRET") {
        config.auth.jwt_secret = jwt_secret;
    }
    if let Ok(expiration_str) = env::var("AUTH_TOKEN_EXPIRATION_HOURS") {
        if let Ok(expiration) = expiration_str.parse::<i64>() {
            config.auth.token_expiration_hours = expiration;
        }
    }
    if let Ok(bcrypt_str) = env::var("AUTH_BCRYPT_COST") {
        if let Ok(bcrypt) = bcrypt_str.parse::<u32>() {
            config.auth.bcrypt_cost = bcrypt;
        }
    }
    if let Ok(max_str) = env::var("AUTH_MAX_LOGIN_ATTEMPTS") {
        if let Ok(max) = max_str.parse::<u32>() {
            config.auth.max_login_attempts = max;
        }
    }
    if let Ok(lockout_str) = env::var("AUTH_LOCKOUT_DURATION_MINUTES") {
        if let Ok(lockout) = lockout_str.parse::<u64>() {
            config.auth.lockout_duration_minutes = lockout;
        }
    }
    if let Ok(url) = env::var("DATABASE_URL") {
        config.database.url = url;
		
    }
    if let Ok(max_conn_str) = env::var("DATABASE_MAX_CONNECTIONS") {
        if let Ok(max_conn) = max_conn_str.parse::<u32>() {
            config.database.max_connections = max_conn;
        }
    }
    if let Ok(min_conn_str) = env::var("DATABASE_MIN_CONNECTIONS") {
        if let Ok(min_conn) = min_conn_str.parse::<u32>() {
            config.database.min_connections = min_conn;
        }
    }
    if let Ok(origins_str) = env::var("ALLOWED_ORIGINS") {
        config.security.allowed_origins = origins_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    if let Ok(level) = env::var("RUST_LOG") {
        config.logging.level = level;
    }

    Ok(())
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.auth.jwt_secret.len() < 32 {
            return Err(anyhow::anyhow!(
                "JWT_SECRET must be at least 32 characters long (current: {})",
                self.auth.jwt_secret.len()
            ));
        }

        if self.database.max_connections < self.database.min_connections {
            return Err(anyhow::anyhow!(
                "max_connections ({}) must be >= min_connections ({})",
                self.database.max_connections,
                self.database.min_connections
            ));
        }

        Ok(())
    }

    pub fn is_production(&self) -> bool {
        env::var("LIMS_ENV").map(|v| v == "production").unwrap_or(false)
    }

    pub fn print_startup_info(&self) {
        log::info!("🧪 LIMS Starting up...");
        log::info!("🌐 Server: {}:{}", self.server.host, self.server.port);
        log::info!("💾 Database: {}",
            if self.database.url.contains("sqlite") { "SQLite" }
            else if self.database.url.contains("postgres") { "PostgreSQL" }
            else { "Unknown" });
        log::info!("🔒 Auth: JWT ({}h expiration)", self.auth.token_expiration_hours);
        log::info!("📊 Logging: {} level", self.logging.level);
        log::info!("🔄 Hot Reload: {}", if self.hot_reload.enabled { "Enabled" } else { "Disabled" });

        if !self.is_production() {
            log::warn!("🚧 Running in development mode");
        }

        if self.security.require_https {
            log::info!("🔒 HTTPS enforcement enabled");
        } else if self.is_production() {
            log::warn!("⚠️  HTTPS not required in production mode");
        }
    }
}

fn is_root_user() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::getuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

pub fn load_env_file() -> Result<()> {
    // 1. Explicit ENV_FILE variable — highest priority
    if let Ok(env_file) = env::var("ENV_FILE") {
        let path = Path::new(&env_file);
        if path.exists() {
            dotenvy::from_filename(&env_file)
                .with_context(|| format!("Failed to load environment file: {}", env_file))?;
            log::info!("✓ Loaded env from ENV_FILE: {}", env_file);
            return Ok(());
        } else {
            log::warn!("ENV_FILE set to '{}' but file not found", env_file);
        }
    }

    // 2. Try multiple common locations for .env
    let candidate_paths = get_env_candidate_paths();
    
    for candidate in &candidate_paths {
        let path = Path::new(candidate);
        if path.exists() {
            match dotenvy::from_filename(candidate) {
                Ok(_) => {
                    log::info!("✓ Loaded env from: {}", candidate);
                    return Ok(());
                }
                Err(e) => {
                    log::warn!("Found .env at '{}' but failed to parse: {}", candidate, e);
                }
            }
        }
    }

    // 3. No .env found — not an error, we'll use defaults + auto-generated secrets
    log::warn!("No .env file found (searched: {:?}). Using defaults with auto-generated JWT secret.", candidate_paths);
    Ok(())
}

/// Returns a list of candidate paths to search for .env file
fn get_env_candidate_paths() -> Vec<String> {
    let mut paths = vec![".env".to_string()];

    // Next to the executable
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            paths.push(exe_dir.join(".env").to_string_lossy().to_string());
        }
    }

    // CARGO_MANIFEST_DIR (available during `cargo run`)
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        paths.push(Path::new(&manifest_dir).join(".env").to_string_lossy().to_string());
    }

    // Parent of CWD (useful when running from target/debug/)
    if let Ok(cwd) = env::current_dir() {
        if let Some(parent) = cwd.parent() {
            paths.push(parent.join(".env").to_string_lossy().to_string());
        }
        // Also try two levels up (target/debug/ → project root)
        if let Some(grandparent) = cwd.parent().and_then(|p| p.parent()) {
            paths.push(grandparent.join(".env").to_string_lossy().to_string());
        }
    }

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| {
        let canonical = fs::canonicalize(p)
            .map(|c| c.to_string_lossy().to_string())
            .unwrap_or_else(|_| p.clone());
        seen.insert(canonical)
    });

    paths
}







