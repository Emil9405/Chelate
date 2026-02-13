// src/db.rs - Database migrations and setup
// Optimized for 270,000+ records with hybrid pagination

use sqlx::SqlitePool;
use anyhow::Result;
use log::info;

pub async fn ensure_performance_indexes(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    info!("Checking and applying performance indexes...");

    let queries = [
        // Batches indexes
        r#"CREATE INDEX IF NOT EXISTS idx_batches_status_expiry ON batches(status, expiry_date);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_batches_reagent_status ON batches(reagent_id, status);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_batches_status_quantities ON batches(status, quantity, original_quantity);"#,

        // Reagents indexes for pagination
        r#"CREATE INDEX IF NOT EXISTS idx_reagents_status_name ON reagents(status, name);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_reagents_name ON reagents(name);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_reagents_created_at ON reagents(created_at DESC, id ASC);"#,

        // Critical index for keyset pagination by total_quantity
        r#"CREATE INDEX IF NOT EXISTS idx_reagents_total_qty ON reagents(total_quantity DESC, id ASC);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_reagents_total_qty_asc ON reagents(total_quantity ASC, id DESC);"#,

        // Audit logs
        r#"CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_audit_logs_entity_type ON audit_logs(entity_type);"#,

        // User permissions
        r#"CREATE INDEX IF NOT EXISTS idx_user_permissions_user_id ON user_permissions(user_id);"#,
    ];

    for query in queries {
        sqlx::query(query).execute(pool).await?;
    }

    sqlx::query("ANALYZE;").execute(pool).await?;

    info!("Performance indexes applied successfully.");
    Ok(())
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    // Enable foreign keys and WAL mode
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await?;

    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(pool)
        .await?;

    // Create users table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE CHECK(length(username) >= 3 AND length(username) <= 50),
            email TEXT NOT NULL UNIQUE CHECK(length(email) >= 5 AND length(email) <= 255),
            password_hash TEXT NOT NULL,
            name TEXT CHECK(name IS NULL OR length(name) <= 100),
            role TEXT NOT NULL DEFAULT 'viewer' CHECK(
                role IN ('admin', 'researcher', 'viewer')
            ),
            is_active INTEGER NOT NULL DEFAULT 1 CHECK(is_active IN (0, 1)),
            last_login DATETIME,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            failed_login_attempts INTEGER NOT NULL DEFAULT 0,
            locked_until DATETIME
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Create reagents table with cached fields
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS reagents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE CHECK(length(name) > 0 AND length(name) <= 255),
            formula TEXT CHECK(formula IS NULL OR length(formula) <= 500),
            molecular_weight REAL CHECK(molecular_weight IS NULL OR molecular_weight >= 0),
            physical_state TEXT CHECK(physical_state IS NULL OR length(physical_state) <= 255),
            cas_number TEXT CHECK(cas_number IS NULL OR length(cas_number) <= 50),
            manufacturer TEXT CHECK(manufacturer IS NULL OR length(manufacturer) <= 255),
            description TEXT CHECK(description IS NULL OR length(description) <= 1000),
            storage_conditions TEXT CHECK(storage_conditions IS NULL OR length(storage_conditions) <= 255),
            appearance TEXT CHECK(appearance IS NULL OR length(appearance) <= 255),
            hazard_pictograms TEXT CHECK(hazard_pictograms IS NULL OR length(hazard_pictograms) <= 100),
            status TEXT NOT NULL DEFAULT 'active' CHECK(
                status IN ('active', 'inactive', 'discontinued')
            ),
            -- Cached aggregation fields (updated by triggers)
            total_quantity REAL NOT NULL DEFAULT 0.0,
            batches_count INTEGER NOT NULL DEFAULT 0,
            primary_unit TEXT,
            -- Audit fields
            created_by TEXT,
            updated_by TEXT,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            FOREIGN KEY (created_by) REFERENCES users (id),
            FOREIGN KEY (updated_by) REFERENCES users (id)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Create batches table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS batches (
            id TEXT PRIMARY KEY,
            reagent_id TEXT NOT NULL,
            lot_number TEXT CHECK(lot_number IS NULL OR length(lot_number) <= 100),
            batch_number TEXT NOT NULL,
            quantity REAL NOT NULL CHECK(quantity >= 0),
            cat_number TEXT CHECK(cat_number IS NULL OR length(cat_number) <= 100),
            original_quantity REAL NOT NULL CHECK(original_quantity >= 0),
            reserved_quantity REAL NOT NULL DEFAULT 0.0 CHECK(reserved_quantity >= 0),
            unit TEXT NOT NULL CHECK(length(unit) > 0 AND length(unit) <= 20),
            pack_size REAL CHECK(pack_size IS NULL OR pack_size > 0),
            expiry_date DATETIME,
            supplier TEXT CHECK(supplier IS NULL OR length(supplier) <= 255),
            manufacturer TEXT CHECK(manufacturer IS NULL OR length(manufacturer) <= 255),
            received_date DATETIME NOT NULL,
            status TEXT NOT NULL DEFAULT 'available' CHECK(
                status IN ('available', 'in_use', 'expired', 'depleted')
            ),
            location TEXT CHECK(location IS NULL OR length(location) <= 255),
            notes TEXT CHECK(notes IS NULL OR length(notes) <= 1000),
            created_by TEXT,
            updated_by TEXT,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            deleted_at DATETIME,
            FOREIGN KEY (reagent_id) REFERENCES reagents (id) ON DELETE CASCADE,
            FOREIGN KEY (created_by) REFERENCES users (id),
            FOREIGN KEY (updated_by) REFERENCES users (id),
            UNIQUE(reagent_id, batch_number)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // ==================== EQUIPMENT TABLE ====================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS equipment (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL CHECK(length(name) > 0 AND length(name) <= 255),
            type_ TEXT NOT NULL CHECK(type_ IN (
                'equipment', 'labware', 'instrument', 'glassware',
                'safety', 'storage', 'consumable', 'other'
            )),
            quantity INTEGER NOT NULL DEFAULT 1 CHECK(quantity >= 1),
            unit TEXT CHECK(unit IS NULL OR length(unit) <= 20),
            status TEXT NOT NULL DEFAULT 'available' CHECK(
                status IN ('available', 'in_use', 'maintenance', 'damaged', 'calibration', 'retired')
            ),
            location TEXT CHECK(location IS NULL OR length(location) <= 255),
            description TEXT CHECK(description IS NULL OR length(description) <= 1000),
            serial_number TEXT CHECK(serial_number IS NULL OR length(serial_number) <= 100),
            manufacturer TEXT CHECK(manufacturer IS NULL OR length(manufacturer) <= 255),
            model TEXT CHECK(model IS NULL OR length(model) <= 255),
            purchase_date TEXT,
            warranty_until TEXT,
            last_maintenance TEXT,
            next_maintenance TEXT,
            maintenance_interval_days INTEGER DEFAULT 90,
            created_by TEXT,
            updated_by TEXT,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            FOREIGN KEY (created_by) REFERENCES users (id),
            FOREIGN KEY (updated_by) REFERENCES users (id)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // ==================== ROOMS TABLE ====================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS rooms (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE CHECK(length(name) > 0 AND length(name) <= 100),
            description TEXT CHECK(description IS NULL OR length(description) <= 500),
            capacity INTEGER CHECK(capacity IS NULL OR capacity > 0),
            status TEXT NOT NULL DEFAULT 'available' CHECK(
                status IN ('available', 'occupied', 'maintenance', 'unavailable')
            ),
            equipment_list TEXT,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL
        )
        "#,
    )
        .execute(pool)
        .await?;

    // ==================== EXPERIMENTS TABLE ====================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS experiments (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL CHECK(length(title) > 0 AND length(title) <= 255),
            description TEXT CHECK(description IS NULL OR length(description) <= 2000),
            status TEXT NOT NULL DEFAULT 'draft' CHECK(
                status IN ('draft', 'planned', 'in_progress', 'completed', 'cancelled', 'on_hold')
            ),
            experiment_type TEXT NOT NULL DEFAULT 'research' CHECK(
                experiment_type IN ('educational', 'research')
            ),
            start_date DATETIME,
            end_date DATETIME,
            location TEXT CHECK(location IS NULL OR length(location) <= 255),
            room_id TEXT,
            researcher_id TEXT,
            created_by TEXT,
            updated_by TEXT,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            FOREIGN KEY (researcher_id) REFERENCES users (id),
            FOREIGN KEY (room_id) REFERENCES rooms (id),
            FOREIGN KEY (created_by) REFERENCES users (id),
            FOREIGN KEY (updated_by) REFERENCES users (id)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // ==================== EXPERIMENT_REAGENTS TABLE ====================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS experiment_reagents (
            id TEXT PRIMARY KEY,
            experiment_id TEXT NOT NULL,
            reagent_id TEXT NOT NULL,
            batch_id TEXT,
            planned_quantity REAL NOT NULL CHECK(planned_quantity > 0),
            actual_quantity REAL,
            unit TEXT NOT NULL CHECK(length(unit) > 0 AND length(unit) <= 20),
            is_consumed INTEGER NOT NULL DEFAULT 0 CHECK(is_consumed IN (0, 1)),
            notes TEXT CHECK(notes IS NULL OR length(notes) <= 500),
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            FOREIGN KEY (experiment_id) REFERENCES experiments (id) ON DELETE CASCADE,
            FOREIGN KEY (reagent_id) REFERENCES reagents (id),
            FOREIGN KEY (batch_id) REFERENCES batches (id),
            UNIQUE(experiment_id, reagent_id, batch_id)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // ==================== USAGE_LOGS TABLE ====================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS usage_logs (
            id TEXT PRIMARY KEY,
            reagent_id TEXT NOT NULL,
            batch_id TEXT NOT NULL,
            user_id TEXT,
            experiment_id TEXT,
            quantity_used REAL NOT NULL CHECK(quantity_used > 0),
            unit TEXT NOT NULL,
            purpose TEXT CHECK(purpose IS NULL OR length(purpose) <= 500),
            notes TEXT CHECK(notes IS NULL OR length(notes) <= 1000),
            created_at DATETIME NOT NULL,
            FOREIGN KEY (reagent_id) REFERENCES reagents (id),
            FOREIGN KEY (batch_id) REFERENCES batches (id),
            FOREIGN KEY (user_id) REFERENCES users (id),
            FOREIGN KEY (experiment_id) REFERENCES experiments (id)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // ==================== AUDIT_LOGS TABLE ====================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS audit_logs (
            id TEXT PRIMARY KEY,
            user_id TEXT,
            action TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT,
            description TEXT,
            old_value TEXT,
            new_value TEXT,
            changes TEXT,
            ip_address TEXT,
            user_agent TEXT,
            created_at DATETIME NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE SET NULL
        )
        "#,
    )
        .execute(pool)
        .await?;

    // ==================== USER_PERMISSIONS TABLE ====================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_permissions (
            user_id TEXT PRIMARY KEY,
            permissions TEXT NOT NULL DEFAULT '{}',
            created_at DATETIME NOT NULL DEFAULT (datetime('now')),
            updated_at DATETIME NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
        )
        "#,
    )
        .execute(pool)
        .await?;

    // ==================== EQUIPMENT PARTS TABLE ====================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS equipment_parts (
            id TEXT PRIMARY KEY,
            equipment_id TEXT NOT NULL,
            name TEXT NOT NULL CHECK(length(name) > 0 AND length(name) <= 255),
            part_number TEXT CHECK(part_number IS NULL OR length(part_number) <= 100),
            manufacturer TEXT CHECK(manufacturer IS NULL OR length(manufacturer) <= 255),
            quantity INTEGER NOT NULL DEFAULT 1 CHECK(quantity >= 0),
            min_quantity INTEGER NOT NULL DEFAULT 0 CHECK(min_quantity >= 0),
            status TEXT NOT NULL DEFAULT 'good' CHECK(
                status IN ('good', 'needs_attention', 'needs_replacement', 'replaced', 'missing')
            ),
            last_replaced TEXT,
            next_replacement TEXT,
            notes TEXT CHECK(notes IS NULL OR length(notes) <= 1000),
            created_by TEXT,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            FOREIGN KEY (equipment_id) REFERENCES equipment (id) ON DELETE CASCADE,
            FOREIGN KEY (created_by) REFERENCES users (id)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // ==================== EQUIPMENT MAINTENANCE TABLE ====================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS equipment_maintenance (
            id TEXT PRIMARY KEY,
            equipment_id TEXT NOT NULL,
            maintenance_type TEXT NOT NULL CHECK(
                maintenance_type IN ('calibration', 'repair', 'inspection', 'cleaning', 'replacement', 'other')
            ),
            status TEXT NOT NULL DEFAULT 'scheduled' CHECK(
                status IN ('scheduled', 'in_progress', 'completed', 'cancelled')
            ),
            scheduled_date TEXT NOT NULL,
            completed_date TEXT,
            performed_by TEXT,
            description TEXT CHECK(description IS NULL OR length(description) <= 2000),
            cost REAL CHECK(cost IS NULL OR cost >= 0),
            parts_replaced TEXT CHECK(parts_replaced IS NULL OR length(parts_replaced) <= 1000),
            notes TEXT CHECK(notes IS NULL OR length(notes) <= 1000),
            created_by TEXT,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            FOREIGN KEY (equipment_id) REFERENCES equipment (id) ON DELETE CASCADE,
            FOREIGN KEY (created_by) REFERENCES users (id)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // ==================== EQUIPMENT FILES TABLE ====================
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS equipment_files (
            id TEXT PRIMARY KEY,
            equipment_id TEXT NOT NULL,
            part_id TEXT,
            file_type TEXT NOT NULL DEFAULT 'other' CHECK(
                file_type IN ('manual', 'certificate', 'photo', 'other')
            ),
            original_filename TEXT NOT NULL,
            stored_filename TEXT NOT NULL,
            file_path TEXT NOT NULL,
            file_size INTEGER NOT NULL CHECK(file_size > 0),
            mime_type TEXT NOT NULL,
            description TEXT CHECK(description IS NULL OR length(description) <= 500),
            uploaded_by TEXT,
            created_at DATETIME NOT NULL,
            FOREIGN KEY (equipment_id) REFERENCES equipment (id) ON DELETE CASCADE,
            FOREIGN KEY (part_id) REFERENCES equipment_parts (id) ON DELETE SET NULL,
            FOREIGN KEY (uploaded_by) REFERENCES users (id)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // ==================== RUN ADDITIONAL MIGRATIONS ====================
    run_additional_migrations(pool).await?;

    // ==================== CREATE BATCH TRIGGERS ====================
    create_batch_triggers(pool).await?;

    // ==================== CREATE FTS TABLES ====================
    create_fts_tables(pool).await?;

    // ==================== INITIALIZE CACHED FIELDS ====================
    initialize_reagent_cache(pool).await?;

    // ==================== PERFORMANCE INDEXES ====================
    ensure_performance_indexes(pool).await?;

    Ok(())
}

// ==================== BATCH TRIGGERS ====================
// Automatically update total_quantity and batches_count in reagents

async fn create_batch_triggers(pool: &SqlitePool) -> Result<()> {
    info!("Creating batch triggers for reagent cache...");

    // Drop old triggers if exist
    let drop_triggers = [
        "DROP TRIGGER IF EXISTS trg_batches_insert",
        "DROP TRIGGER IF EXISTS trg_batches_update",
        "DROP TRIGGER IF EXISTS trg_batches_delete",
    ];

    for query in drop_triggers {
        let _ = sqlx::query(query).execute(pool).await;
    }

    // INSERT trigger - when adding a batch with status='available' and not deleted
    sqlx::query(r#"
        CREATE TRIGGER IF NOT EXISTS trg_batches_insert
        AFTER INSERT ON batches
        WHEN NEW.status = 'available' AND NEW.deleted_at IS NULL
        BEGIN
            UPDATE reagents SET
                total_quantity = total_quantity + NEW.quantity,
                batches_count = batches_count + 1,
                primary_unit = COALESCE(primary_unit, NEW.unit),
                updated_at = datetime('now')
            WHERE id = NEW.reagent_id;
        END
    "#)
        .execute(pool)
        .await?;

    // DELETE trigger - when hard deleting a batch with status='available' (not soft-deleted)
    sqlx::query(r#"
        CREATE TRIGGER IF NOT EXISTS trg_batches_delete
        AFTER DELETE ON batches
        WHEN OLD.status = 'available' AND OLD.deleted_at IS NULL
        BEGIN
            UPDATE reagents SET
                total_quantity = MAX(0, total_quantity - OLD.quantity),
                batches_count = MAX(0, batches_count - 1),
                updated_at = datetime('now')
            WHERE id = OLD.reagent_id;
        END
    "#)
        .execute(pool)
        .await?;

    // UPDATE trigger - full recalculation on change (including soft delete)
    sqlx::query(r#"
        CREATE TRIGGER IF NOT EXISTS trg_batches_update
        AFTER UPDATE ON batches
        BEGIN
            UPDATE reagents SET
                total_quantity = (
                    SELECT COALESCE(SUM(quantity), 0)
                    FROM batches
                    WHERE reagent_id = NEW.reagent_id AND status = 'available' AND deleted_at IS NULL
                ),
                batches_count = (
                    SELECT COUNT(*)
                    FROM batches
                    WHERE reagent_id = NEW.reagent_id AND status = 'available' AND deleted_at IS NULL
                ),
                primary_unit = (
                    SELECT unit
                    FROM batches
                    WHERE reagent_id = NEW.reagent_id AND status = 'available' AND deleted_at IS NULL
                    LIMIT 1
                ),
                updated_at = datetime('now')
            WHERE id = NEW.reagent_id;

            -- If reagent_id changed, update the old reagent as well
            UPDATE reagents SET
                total_quantity = (
                    SELECT COALESCE(SUM(quantity), 0)
                    FROM batches
                    WHERE reagent_id = OLD.reagent_id AND status = 'available' AND deleted_at IS NULL
                ),
                batches_count = (
                    SELECT COUNT(*)
                    FROM batches
                    WHERE reagent_id = OLD.reagent_id AND status = 'available' AND deleted_at IS NULL
                ),
                primary_unit = (
                    SELECT unit
                    FROM batches
                    WHERE reagent_id = OLD.reagent_id AND status = 'available' AND deleted_at IS NULL
                    LIMIT 1
                ),
                updated_at = datetime('now')
            WHERE id = OLD.reagent_id AND OLD.reagent_id != NEW.reagent_id;
        END
    "#)
        .execute(pool)
        .await?;

    info!("Batch triggers created successfully.");
    Ok(())
}

// ==================== FTS TABLES ====================
// Full-text search for fast searching across 100k+ records
// Search fields: name, cas_number, formula

async fn create_fts_tables(pool: &SqlitePool) -> Result<()> {
    info!("Creating FTS5 tables for full-text search...");

    // Check if FTS table already exists
    let exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='reagents_fts'"
    ).fetch_one(pool).await?;

    if exists.0 > 0 {
        info!("FTS table already exists, skipping creation.");
        return Ok(());
    }

    // Create FTS5 virtual table for reagents
    sqlx::query(r#"
        CREATE VIRTUAL TABLE reagents_fts USING fts5(
            name,
            cas_number,
            formula,
            content='reagents',
            content_rowid='rowid',
            tokenize='unicode61 remove_diacritics 1'
        )
    "#).execute(pool).await?;

    // INSERT trigger - sync new reagents to FTS
    sqlx::query(r#"
        CREATE TRIGGER reagents_fts_insert AFTER INSERT ON reagents BEGIN
            INSERT INTO reagents_fts(rowid, name, cas_number, formula)
            VALUES (NEW.rowid, NEW.name, NEW.cas_number, NEW.formula);
        END
    "#).execute(pool).await?;

    // DELETE trigger - remove from FTS
    sqlx::query(r#"
        CREATE TRIGGER reagents_fts_delete AFTER DELETE ON reagents BEGIN
            INSERT INTO reagents_fts(reagents_fts, rowid, name, cas_number, formula)
            VALUES ('delete', OLD.rowid, OLD.name, OLD.cas_number, OLD.formula);
        END
    "#).execute(pool).await?;

    // UPDATE trigger - update FTS index
    sqlx::query(r#"
        CREATE TRIGGER reagents_fts_update AFTER UPDATE ON reagents BEGIN
            INSERT INTO reagents_fts(reagents_fts, rowid, name, cas_number, formula)
            VALUES ('delete', OLD.rowid, OLD.name, OLD.cas_number, OLD.formula);
            INSERT INTO reagents_fts(rowid, name, cas_number, formula)
            VALUES (NEW.rowid, NEW.name, NEW.cas_number, NEW.formula);
        END
    "#).execute(pool).await?;

    // Populate FTS with existing data
    sqlx::query(r#"
        INSERT INTO reagents_fts(rowid, name, cas_number, formula)
        SELECT rowid, name, cas_number, formula FROM reagents
    "#).execute(pool).await?;

    // Optimize the FTS index
    let _ = sqlx::query("INSERT INTO reagents_fts(reagents_fts) VALUES('optimize')").execute(pool).await;

    info!("FTS5 table created and populated.");
    Ok(())
}

// ==================== INITIALIZE CACHE ====================
// Populate cached fields for existing data

async fn initialize_reagent_cache(pool: &SqlitePool) -> Result<()> {
    info!("Initializing reagent cache fields...");

    // Recalculate total_quantity and batches_count from batches (excluding soft-deleted)
    let result = sqlx::query(r#"
        UPDATE reagents SET
            total_quantity = (
                SELECT COALESCE(SUM(quantity), 0)
                FROM batches
                WHERE reagent_id = reagents.id AND status = 'available' AND deleted_at IS NULL
            ),
            batches_count = (
                SELECT COUNT(*)
                FROM batches
                WHERE reagent_id = reagents.id AND status = 'available' AND deleted_at IS NULL
            ),
            primary_unit = (
                SELECT unit
                FROM batches
                WHERE reagent_id = reagents.id AND status = 'available' AND deleted_at IS NULL
                LIMIT 1
            )
        WHERE EXISTS (SELECT 1 FROM batches WHERE reagent_id = reagents.id AND deleted_at IS NULL)
           OR total_quantity != 0
           OR batches_count != 0
    "#)
        .execute(pool)
        .await?;

    info!("Reagent cache initialized: {} rows updated", result.rows_affected());
    Ok(())
}

// ==================== ADDITIONAL MIGRATIONS ====================

async fn run_additional_migrations(pool: &SqlitePool) -> Result<()> {
    info!("Running additional migrations...");

    let migration_queries = [
        // ==================== REAGENTS ====================
        "ALTER TABLE reagents ADD COLUMN total_quantity REAL NOT NULL DEFAULT 0.0",
        "ALTER TABLE reagents ADD COLUMN batches_count INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE reagents ADD COLUMN primary_unit TEXT",

        // ==================== EQUIPMENT ====================
        "ALTER TABLE equipment ADD COLUMN serial_number TEXT CHECK(serial_number IS NULL OR length(serial_number) <= 100)",
        "ALTER TABLE equipment ADD COLUMN manufacturer TEXT CHECK(manufacturer IS NULL OR length(manufacturer) <= 255)",
        "ALTER TABLE equipment ADD COLUMN model TEXT CHECK(model IS NULL OR length(model) <= 255)",
        "ALTER TABLE equipment ADD COLUMN purchase_date TEXT",
        "ALTER TABLE equipment ADD COLUMN warranty_until TEXT",
        "ALTER TABLE equipment ADD COLUMN last_maintenance TEXT",
        "ALTER TABLE equipment ADD COLUMN next_maintenance TEXT",
        "ALTER TABLE equipment ADD COLUMN maintenance_interval_days INTEGER DEFAULT 90",

        // ==================== USERS ====================
        "ALTER TABLE users ADD COLUMN failed_login_attempts INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE users ADD COLUMN locked_until DATETIME",
        "ALTER TABLE users ADD COLUMN name TEXT CHECK(name IS NULL OR length(name) <= 100)",

        // ==================== BATCHES ====================
        "ALTER TABLE batches ADD COLUMN lot_number TEXT CHECK(lot_number IS NULL OR length(lot_number) <= 100)",
        "ALTER TABLE batches ADD COLUMN cat_number TEXT CHECK(cat_number IS NULL OR length(cat_number) <= 100)",
        "ALTER TABLE batches ADD COLUMN original_quantity REAL",
        "ALTER TABLE batches ADD COLUMN manufacturer TEXT CHECK(manufacturer IS NULL OR length(manufacturer) <= 255)",
        "ALTER TABLE batches ADD COLUMN supplier TEXT CHECK(supplier IS NULL OR length(supplier) <= 255)",
        "ALTER TABLE batches ADD COLUMN location TEXT CHECK(location IS NULL OR length(location) <= 255)",
        "ALTER TABLE batches ADD COLUMN notes TEXT CHECK(notes IS NULL OR length(notes) <= 1000)",
        "ALTER TABLE batches ADD COLUMN created_by TEXT",
        "ALTER TABLE batches ADD COLUMN updated_by TEXT",
        "ALTER TABLE batches ADD COLUMN reserved_quantity REAL NOT NULL DEFAULT 0.0 CHECK(reserved_quantity >= 0)",
        "ALTER TABLE batches ADD COLUMN pack_size REAL CHECK(pack_size IS NULL OR pack_size > 0)",
        "ALTER TABLE batches ADD COLUMN deleted_at DATETIME",

        // ==================== EXPERIMENTS ====================
        "ALTER TABLE experiment_reagents ADD COLUMN is_consumed INTEGER NOT NULL DEFAULT 0 CHECK(is_consumed IN (0, 1))",
        "ALTER TABLE experiments ADD COLUMN location TEXT CHECK(location IS NULL OR length(location) <= 255)",
        "ALTER TABLE experiments ADD COLUMN room_id TEXT REFERENCES rooms(id)",
        "ALTER TABLE experiments ADD COLUMN experiment_type TEXT NOT NULL DEFAULT 'research' CHECK(experiment_type IN ('educational', 'research'))",

        // ==================== AUDIT_LOGS ====================
        "ALTER TABLE audit_logs ADD COLUMN description TEXT",
        "ALTER TABLE audit_logs ADD COLUMN changes TEXT",

        // ==================== ROOMS ====================
        "ALTER TABLE rooms ADD COLUMN color TEXT CHECK(color IS NULL OR length(color) <= 20)",
        "ALTER TABLE rooms ADD COLUMN created_by TEXT REFERENCES users(id)",
        "ALTER TABLE rooms ADD COLUMN updated_by TEXT REFERENCES users(id)",
    ];

    for query in migration_queries.iter() {
        // Ignore errors for existing columns
        let _ = sqlx::query(query).execute(pool).await;
    }

    // Update original_quantity for existing batches
    let _ = sqlx::query("UPDATE batches SET original_quantity = quantity WHERE original_quantity IS NULL")
        .execute(pool)
        .await;

    // ==================== CLEANUP OLD CACHE TABLES ====================
    let _ = sqlx::query("DROP TABLE IF EXISTS reagent_stock_cache").execute(pool).await;
    let _ = sqlx::query("DROP TABLE IF EXISTS reagent_count_cache").execute(pool).await;

    info!("Additional migrations completed.");
    Ok(())
}

// ==================== DATABASE RESET (DEVELOPMENT ONLY) ====================

pub async fn reset_database(pool: &SqlitePool) -> Result<()> {
    log::warn!("Resetting database - all data will be lost!");

    let drop_queries = [
        "DROP TRIGGER IF EXISTS reagents_fts_insert",
        "DROP TRIGGER IF EXISTS reagents_fts_update",
        "DROP TRIGGER IF EXISTS reagents_fts_delete",
        "DROP TABLE IF EXISTS equipment_fts",
        "DROP TABLE IF EXISTS reagents_fts",
        "DROP TABLE IF EXISTS equipment_files",
        "DROP TABLE IF EXISTS equipment_maintenance",
        "DROP TABLE IF EXISTS equipment_parts",
        "DROP TABLE IF EXISTS experiment_equipment",
        "DROP TABLE IF EXISTS experiment_reagents",
        "DROP TABLE IF EXISTS experiment_documents",
        "DROP TABLE IF EXISTS experiments",
        "DROP TABLE IF EXISTS rooms",
        "DROP TABLE IF EXISTS equipment",
        "DROP TABLE IF EXISTS audit_logs",
        "DROP TABLE IF EXISTS usage_logs",
        "DROP TABLE IF EXISTS user_permissions",
        "DROP TABLE IF EXISTS batches",
        "DROP TABLE IF EXISTS reagents",
        "DROP TABLE IF EXISTS users",
        "DROP TABLE IF EXISTS reagent_stock_cache",
        "DROP TABLE IF EXISTS reagent_count_cache",
    ];

    for query in drop_queries.iter() {
        let _ = sqlx::query(query).execute(pool).await;
    }

    // Recreate tables
    run_migrations(pool).await?;

    Ok(())
}

// ==================== UTILITY FUNCTIONS ====================

/// Check if a column exists in a table
#[allow(dead_code)]
pub async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> Result<bool> {
    let query = format!("SELECT COUNT(*) as count FROM pragma_table_info('{}') WHERE name = ?", table);
    let result: (i32,) = sqlx::query_as(&query)
        .bind(column)
        .fetch_one(pool)
        .await?;
    Ok(result.0 > 0)
}

/// Get table info for debugging
#[allow(dead_code)]
pub async fn get_table_columns(pool: &SqlitePool, table: &str) -> Result<Vec<String>> {
    let query = format!("SELECT name FROM pragma_table_info('{}')", table);
    let rows: Vec<(String,)> = sqlx::query_as(&query)
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Rebuild reagent cache manually (for maintenance)
pub async fn rebuild_reagent_cache(pool: &SqlitePool) -> Result<u64> {
    let result = sqlx::query(r#"
        UPDATE reagents SET
            total_quantity = (
                SELECT COALESCE(SUM(quantity), 0)
                FROM batches
                WHERE reagent_id = reagents.id AND status = 'available' AND deleted_at IS NULL
            ),
            batches_count = (
                SELECT COUNT(*)
                FROM batches
                WHERE reagent_id = reagents.id AND status = 'available' AND deleted_at IS NULL
            ),
            primary_unit = (
                SELECT unit
                FROM batches
                WHERE reagent_id = reagents.id AND status = 'available' AND deleted_at IS NULL
                LIMIT 1
            ),
            updated_at = datetime('now')
    "#)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

/// Rebuild FTS index (for maintenance after bulk imports)
pub async fn rebuild_fts_index(pool: &SqlitePool) -> Result<u64> {
    info!("Rebuilding FTS index...");

    // Clear and repopulate
    let _ = sqlx::query("DELETE FROM reagents_fts").execute(pool).await;

    let result = sqlx::query(r#"
        INSERT INTO reagents_fts(rowid, name, cas_number, formula)
        SELECT rowid, name, cas_number, formula FROM reagents
    "#).execute(pool).await?;

    // Optimize
    let _ = sqlx::query("INSERT INTO reagents_fts(reagents_fts) VALUES('optimize')").execute(pool).await;

    info!("FTS index rebuilt: {} rows", result.rows_affected());
    Ok(result.rows_affected())
}