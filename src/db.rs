// src/db.rs - Database migrations and setup with FTS5 support

use sqlx::SqlitePool;
use anyhow::Result;

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    // Enable foreign keys and WAL mode
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await?;

    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(pool)
        .await?;

    // Create users table - Fixed boolean handling for SQLite
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE CHECK(length(username) >= 3 AND length(username) <= 50),
            email TEXT NOT NULL UNIQUE CHECK(length(email) >= 5 AND length(email) <= 255),
            password_hash TEXT NOT NULL,
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

    // Create the reagent table
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

    // Create batches table with reserved_quantity field
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
    // ✅ UPDATED: Full schema with all fields including serial_number, manufacturer, model, etc.
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
            -- Additional fields for equipment management
            serial_number TEXT CHECK(serial_number IS NULL OR length(serial_number) <= 100),
            manufacturer TEXT CHECK(manufacturer IS NULL OR length(manufacturer) <= 255),
            model TEXT CHECK(model IS NULL OR length(model) <= 255),
            purchase_date TEXT,
            warranty_until TEXT,
            last_maintenance TEXT,
            next_maintenance TEXT,
            maintenance_interval_days INTEGER DEFAULT 90,
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

    // Create equipment_parts table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS equipment_parts (
            id TEXT PRIMARY KEY NOT NULL,
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
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (equipment_id) REFERENCES equipment(id) ON DELETE CASCADE
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Create usage_logs table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS usage_logs (
            id TEXT PRIMARY KEY,
            batch_id TEXT NOT NULL,
            user_id TEXT,
            quantity_used REAL NOT NULL CHECK(quantity_used > 0),
            purpose TEXT CHECK(purpose IS NULL OR length(purpose) <= 500),
            notes TEXT CHECK(notes IS NULL OR length(notes) <= 1000),
            used_at DATETIME NOT NULL,
            created_at DATETIME NOT NULL,
            FOREIGN KEY (batch_id) REFERENCES batches (id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users (id)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Create equipment_maintenance table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS equipment_maintenance (
            id TEXT PRIMARY KEY NOT NULL,
            equipment_id TEXT NOT NULL,
            maintenance_type TEXT NOT NULL CHECK(
                maintenance_type IN ('scheduled', 'unscheduled', 'calibration', 'repair', 
                                     'inspection', 'cleaning', 'part_replacement')
            ),
            status TEXT NOT NULL DEFAULT 'scheduled' CHECK(
                status IN ('scheduled', 'in_progress', 'completed', 'cancelled', 'overdue')
            ),
            scheduled_date TEXT NOT NULL,
            completed_date TEXT,
            performed_by TEXT CHECK(performed_by IS NULL OR length(performed_by) <= 255),
            description TEXT CHECK(description IS NULL OR length(description) <= 1000),
            cost REAL CHECK(cost IS NULL OR cost >= 0),
            parts_replaced TEXT,
            notes TEXT CHECK(notes IS NULL OR length(notes) <= 1000),
            created_by TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (equipment_id) REFERENCES equipment(id) ON DELETE CASCADE
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Create equipment_files table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS equipment_files (
            id TEXT PRIMARY KEY NOT NULL,
            equipment_id TEXT NOT NULL,
            part_id TEXT,
            file_type TEXT NOT NULL CHECK(
                file_type IN ('manual', 'image', 'certificate', 'specification', 'maintenance_log', 'other')
            ),
            original_filename TEXT NOT NULL CHECK(length(original_filename) > 0),
            stored_filename TEXT NOT NULL,
            file_path TEXT NOT NULL,
            file_size INTEGER NOT NULL CHECK(file_size > 0),
            mime_type TEXT NOT NULL,
            description TEXT CHECK(description IS NULL OR length(description) <= 500),
            uploaded_by TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (equipment_id) REFERENCES equipment(id) ON DELETE CASCADE,
            FOREIGN KEY (part_id) REFERENCES equipment_parts(id) ON DELETE CASCADE
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Create audit_logs table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS audit_logs (
            id TEXT PRIMARY KEY,
            user_id TEXT,
            action TEXT NOT NULL CHECK(
                action IN ('CREATE', 'UPDATE', 'DELETE')
            ),
            table_name TEXT NOT NULL,
            record_id TEXT NOT NULL,
            old_values TEXT,
            new_values TEXT,
            ip_address TEXT,
            user_agent TEXT,
            created_at DATETIME NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users (id)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Create experiments table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS experiments (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL CHECK(length(title) > 0 AND length(title) <= 255),
            description TEXT CHECK(description IS NULL OR length(description) <= 2000),
            experiment_date DATETIME NOT NULL,
            experiment_type TEXT NOT NULL DEFAULT 'research' CHECK(
                experiment_type IN ('educational', 'research')
            ),
            protocol TEXT CHECK(protocol IS NULL OR length(protocol) <= 5000),
            start_date DATETIME NOT NULL,
            end_date DATETIME,
            results TEXT CHECK(results IS NULL OR length(results) <= 5000),
            notes TEXT CHECK(notes IS NULL OR length(notes) <= 1000),
            instructor TEXT CHECK(instructor IS NULL OR length(instructor) <= 255),
            student_group TEXT CHECK(student_group IS NULL OR length(student_group) <= 100),
            location TEXT CHECK(location IS NULL OR length(location) <= 255),
            status TEXT NOT NULL DEFAULT 'planned' CHECK(
                status IN ('planned', 'in_progress', 'completed', 'cancelled')
            ),
            room_id TEXT,
            created_by TEXT NOT NULL,
            updated_by TEXT,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL,
            FOREIGN KEY (created_by) REFERENCES users (id),
            FOREIGN KEY (updated_by) REFERENCES users (id),
            FOREIGN KEY (room_id) REFERENCES rooms (id)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Create experiment_documents table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS experiment_documents (
            id TEXT PRIMARY KEY,
            experiment_id TEXT NOT NULL,
            filename TEXT NOT NULL CHECK(length(filename) > 0 AND length(filename) <= 255),
            original_filename TEXT NOT NULL,
            file_path TEXT NOT NULL,
            file_size INTEGER NOT NULL CHECK(file_size > 0),
            mime_type TEXT NOT NULL,
            uploaded_by TEXT NOT NULL,
            uploaded_at DATETIME NOT NULL,
            FOREIGN KEY (experiment_id) REFERENCES experiments (id) ON DELETE CASCADE,
            FOREIGN KEY (uploaded_by) REFERENCES users (id)
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Create experiment_reagents table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS experiment_reagents (
            id TEXT PRIMARY KEY,
            experiment_id TEXT NOT NULL,
            batch_id TEXT NOT NULL,
            quantity_used REAL NOT NULL CHECK(quantity_used > 0),
            is_consumed INTEGER NOT NULL DEFAULT 0 CHECK(is_consumed IN (0, 1)),
            notes TEXT CHECK(notes IS NULL OR length(notes) <= 500),
            created_at DATETIME NOT NULL,
            FOREIGN KEY (experiment_id) REFERENCES experiments (id) ON DELETE CASCADE,
            FOREIGN KEY (batch_id) REFERENCES batches (id) ON DELETE CASCADE
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Create experiment_equipment table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS experiment_equipment (
            id TEXT PRIMARY KEY,
            experiment_id TEXT NOT NULL,
            equipment_id TEXT NOT NULL,
            quantity_used INTEGER NOT NULL DEFAULT 1 CHECK(quantity_used >= 1),
            notes TEXT CHECK(notes IS NULL OR length(notes) <= 500),
            created_at DATETIME NOT NULL,
            FOREIGN KEY (experiment_id) REFERENCES experiments (id) ON DELETE CASCADE,
            FOREIGN KEY (equipment_id) REFERENCES equipment (id) ON DELETE CASCADE
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Create rooms table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS rooms (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE CHECK(length(name) > 0 AND length(name) <= 100),
            description TEXT CHECK(description IS NULL OR length(description) <= 500),
            capacity INTEGER CHECK(capacity IS NULL OR (capacity >= 1 AND capacity <= 1000)),
            color TEXT CHECK(color IS NULL OR length(color) <= 20),
            status TEXT NOT NULL DEFAULT 'available' CHECK(
                status IN ('available', 'reserved', 'occupied', 'maintenance', 'unavailable')
            ),
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

    // ==================== CREATE INDEXES ====================
    
    // Equipment indexes
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_status ON equipment(status)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_type ON equipment(type_)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_location ON equipment(location)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_serial_number ON equipment(serial_number)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_manufacturer ON equipment(manufacturer)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_next_maintenance ON equipment(next_maintenance)")
        .execute(pool).await;

    // Equipment parts indexes
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_parts_equipment_id ON equipment_parts(equipment_id)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_parts_status ON equipment_parts(status)")
        .execute(pool).await;

    // Equipment maintenance indexes
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_maintenance_equipment_id ON equipment_maintenance(equipment_id)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_maintenance_status ON equipment_maintenance(status)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_maintenance_scheduled_date ON equipment_maintenance(scheduled_date)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_maintenance_type ON equipment_maintenance(maintenance_type)")
        .execute(pool).await;

    // Equipment files indexes
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_files_equipment_id ON equipment_files(equipment_id)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_files_part_id ON equipment_files(part_id)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_equipment_files_file_type ON equipment_files(file_type)")
        .execute(pool).await;

    // Rooms indexes
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_rooms_status ON rooms(status)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_experiments_room_id ON experiments(room_id)")
        .execute(pool).await;

    // Other indexes
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_reagents_status ON reagents(status)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_reagents_cas ON reagents(cas_number)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_batches_reagent ON batches(reagent_id)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_batches_status ON batches(status)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_batches_expiry ON batches(expiry_date)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_usage_batch ON usage_logs(batch_id)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_usage_user ON usage_logs(user_id)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_table ON audit_logs(table_name)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_record ON audit_logs(record_id)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_experiments_status ON experiments(status)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_experiments_date ON experiments(experiment_date)")
        .execute(pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_experiments_type ON experiments(experiment_type)")
        .execute(pool).await;

    // ==================== FTS5 FULL-TEXT SEARCH ====================
    
    // Reagents FTS
    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS reagents_fts USING fts5(
            name,
            formula,
            cas_number,
            manufacturer,
            description,
            storage_conditions,
            appearance,
            content='reagents',
            content_rowid='rowid'
        )
        "#,
    )
        .execute(pool)
        .await?;

    // Equipment FTS
    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS equipment_fts USING fts5(
            name,
            description,
            location,
            manufacturer,
            model,
            serial_number,
            content='equipment',
            content_rowid='rowid',
            tokenize='unicode61'
        )
        "#,
    )
        .execute(pool)
        .await?;

    // ==================== AUDIT TRIGGERS ====================
    
    let trigger_queries = [
        // Reagents audit triggers
        r#"CREATE TRIGGER IF NOT EXISTS reagents_audit_insert
           AFTER INSERT ON reagents
           BEGIN
               INSERT INTO audit_logs (id, user_id, action, table_name, record_id, new_values, created_at)
               VALUES (
                   lower(hex(randomblob(16))),
                   NEW.created_by,
                   'CREATE',
                   'reagents',
                   NEW.id,
                   json_object('name', NEW.name, 'formula', NEW.formula, 'status', NEW.status),
                   NEW.created_at
               );
           END"#,

        r#"CREATE TRIGGER IF NOT EXISTS reagents_audit_update
           AFTER UPDATE ON reagents
           BEGIN
               INSERT INTO audit_logs (id, user_id, action, table_name, record_id, old_values, new_values, created_at)
               VALUES (
                   lower(hex(randomblob(16))),
                   NEW.updated_by,
                   'UPDATE',
                   'reagents',
                   NEW.id,
                   json_object('name', OLD.name, 'formula', OLD.formula, 'status', OLD.status),
                   json_object('name', NEW.name, 'formula', NEW.formula, 'status', NEW.status),
                   datetime('now')
               );
           END"#,

        // Batches audit triggers
        r#"CREATE TRIGGER IF NOT EXISTS batches_audit_insert
           AFTER INSERT ON batches
           BEGIN
               INSERT INTO audit_logs (id, user_id, action, table_name, record_id, new_values, created_at)
               VALUES (
                   lower(hex(randomblob(16))),
                   NEW.created_by,
                   'CREATE',
                   'batches',
                   NEW.id,
                   json_object(
                       'batch_number', NEW.batch_number,
                       'quantity', NEW.quantity,
                       'unit', NEW.unit,
                       'status', NEW.status,
                       'location', NEW.location
                   ),
                   NEW.created_at
               );
           END"#,

        r#"CREATE TRIGGER IF NOT EXISTS batches_audit_update
           AFTER UPDATE ON batches
           BEGIN
               INSERT INTO audit_logs (id, user_id, action, table_name, record_id, old_values, new_values, created_at)
               VALUES (
                   lower(hex(randomblob(16))),
                   NEW.updated_by,
                   'UPDATE',
                   'batches',
                   NEW.id,
                   json_object(
                       'batch_number', OLD.batch_number,
                       'quantity', OLD.quantity,
                       'status', OLD.status
                   ),
                   json_object(
                       'batch_number', NEW.batch_number,
                       'quantity', NEW.quantity,
                       'status', NEW.status
                   ),
                   datetime('now')
               );
           END"#,

        // Equipment audit triggers
        r#"CREATE TRIGGER IF NOT EXISTS equipment_audit_insert
            AFTER INSERT ON equipment
            BEGIN
                INSERT INTO audit_logs (id, user_id, action, table_name, record_id, new_values, created_at)
                VALUES (
                    lower(hex(randomblob(16))),
                    NEW.created_by,
                    'CREATE',
                    'equipment',
                    NEW.id,
                    json_object(
                        'name', NEW.name,
                        'type_', NEW.type_,
                        'quantity', NEW.quantity,
                        'status', NEW.status,
                        'location', NEW.location,
                        'serial_number', NEW.serial_number,
                        'manufacturer', NEW.manufacturer
                    ),
                    NEW.created_at
               );
         END"#,

        r#"CREATE TRIGGER IF NOT EXISTS equipment_audit_update
           AFTER UPDATE ON equipment
           BEGIN
               INSERT INTO audit_logs (id, user_id, action, table_name, record_id, old_values, new_values, created_at)
               VALUES (
                   lower(hex(randomblob(16))),
                   NEW.updated_by,
                   'UPDATE',
                   'equipment',
                   NEW.id,
                   json_object(
                       'name', OLD.name,
                       'quantity', OLD.quantity,
                       'status', OLD.status,
                       'location', OLD.location
                   ),
                   json_object(
                       'name', NEW.name,
                       'quantity', NEW.quantity,
                       'status', NEW.status,
                       'location', NEW.location
                   ),
                   datetime('now')
               );
           END"#,

        r#"CREATE TRIGGER IF NOT EXISTS equipment_audit_delete
            BEFORE DELETE ON equipment
            BEGIN
                INSERT INTO audit_logs (id, action, table_name, record_id, old_values, created_at)
                VALUES (
                    lower(hex(randomblob(16))),
                    'DELETE',
                    'equipment',
                    OLD.id,
                    json_object(
                        'name', OLD.name,
                        'type_', OLD.type_,
                        'status', OLD.status,
                        'serial_number', OLD.serial_number
                    ),
                    datetime('now')
                );
            END"#,

        // Equipment maintenance triggers
        r#"CREATE TRIGGER IF NOT EXISTS equipment_maintenance_audit_insert
            AFTER INSERT ON equipment_maintenance
            BEGIN
                INSERT INTO audit_logs (id, user_id, action, table_name, record_id, new_values, created_at)
                VALUES (
                    lower(hex(randomblob(16))),
                    NEW.created_by,
                    'CREATE',
                    'equipment_maintenance',
                    NEW.id,
                    json_object(
                        'equipment_id', NEW.equipment_id,
                        'maintenance_type', NEW.maintenance_type,
                        'status', NEW.status,
                        'scheduled_date', NEW.scheduled_date
                    ),
                    NEW.created_at
                );
            END"#,

        r#"CREATE TRIGGER IF NOT EXISTS equipment_maintenance_audit_update
            AFTER UPDATE ON equipment_maintenance
            BEGIN
                INSERT INTO audit_logs (id, action, table_name, record_id, old_values, new_values, created_at)
                VALUES (
                    lower(hex(randomblob(16))),
                    'UPDATE',
                    'equipment_maintenance',
                    NEW.id,
                    json_object('status', OLD.status, 'completed_date', OLD.completed_date),
                    json_object('status', NEW.status, 'completed_date', NEW.completed_date),
                    datetime('now')
                );
            END"#,
    ];

    for query in trigger_queries.iter() {
        let _ = sqlx::query(query).execute(pool).await;
    }
    let fts_triggers = [
        // --- Reagents FTS Sync ---
        "CREATE TRIGGER IF NOT EXISTS reagents_ai AFTER INSERT ON reagents BEGIN
           INSERT INTO reagents_fts(rowid, name, formula, cas_number, manufacturer, description, storage_conditions, appearance)
           VALUES (new.rowid, new.name, new.formula, new.cas_number, new.manufacturer, new.description, new.storage_conditions, new.appearance);
         END;",
        "CREATE TRIGGER IF NOT EXISTS reagents_ad AFTER DELETE ON reagents BEGIN
                INSERT INTO reagents_fts(reagents_fts, rowid, name, formula, cas_number, manufacturer, description, storage_conditions, appearance)
                VALUES('delete', old.rowid, old.name, old.formula, old.cas_number, old.manufacturer, old.description, old.storage_conditions, old.appearance);
         END;",
        "CREATE TRIGGER IF NOT EXISTS reagents_au AFTER UPDATE ON reagents BEGIN
           INSERT INTO reagents_fts(reagents_fts, rowid, name, formula, cas_number, manufacturer, description, storage_conditions, appearance)
           VALUES('delete', old.rowid, old.name, old.formula, old.cas_number, old.manufacturer, old.description, old.storage_conditions, old.appearance);
           INSERT INTO reagents_fts(rowid, name, formula, cas_number, manufacturer, description, storage_conditions, appearance)
           VALUES (new.rowid, new.name, new.formula, new.cas_number, new.manufacturer, new.description, new.storage_conditions, new.appearance);
         END;",
        // --- Equipment FTS Sync (ДОБАВИТЬ ЭТО) ---
        "CREATE TRIGGER IF NOT EXISTS equipment_ai AFTER INSERT ON equipment BEGIN
           INSERT INTO equipment_fts(rowid, name, description, location, manufacturer, model, serial_number)
           VALUES (new.rowid, new.name, new.description, new.location, new.manufacturer, new.model, new.serial_number);
         END;",
        "CREATE TRIGGER IF NOT EXISTS equipment_ad AFTER DELETE ON equipment BEGIN
           INSERT INTO equipment_fts(equipment_fts, rowid, name, description, location, manufacturer, model, serial_number)
           VALUES('delete', old.rowid, old.name, old.description, old.location, old.manufacturer, old.model, old.serial_number);
         END;",
        "CREATE TRIGGER IF NOT EXISTS equipment_au AFTER UPDATE ON equipment BEGIN
           INSERT INTO equipment_fts(equipment_fts, rowid, name, description, location, manufacturer, model, serial_number)
           VALUES('delete', old.rowid, old.name, old.description, old.location, old.manufacturer, old.model, old.serial_number);
           INSERT INTO equipment_fts(rowid, name, description, location, manufacturer, model, serial_number)
           VALUES (new.rowid, new.name, new.description, new.location, new.manufacturer, new.model, new.serial_number);
         END;",


    ];

    for query in fts_triggers.iter() {
        let _ = sqlx::query(query).execute(pool).await;
    }

    // Run migrations for existing tables
    migrate_existing_tables(pool).await?;

    Ok(())
}

// ==================== MIGRATION FOR EXISTING DATABASES ====================

pub async fn migrate_existing_tables(pool: &SqlitePool) -> Result<()> {
    // Add new columns to existing tables if they don't exist
    let migration_queries = [
        // ==================== REAGENTS ====================
        "ALTER TABLE reagents ADD COLUMN cas_number TEXT CHECK(cas_number IS NULL OR length(cas_number) <= 50)",
        "ALTER TABLE reagents ADD COLUMN manufacturer TEXT CHECK(manufacturer IS NULL OR length(manufacturer) <= 255)",
        "ALTER TABLE reagents ADD COLUMN created_by TEXT",
        "ALTER TABLE reagents ADD COLUMN updated_by TEXT",
        "ALTER TABLE reagents ADD COLUMN molecular_weight REAL CHECK(molecular_weight IS NULL OR molecular_weight >= 0)",
        "ALTER TABLE reagents ADD COLUMN storage_conditions TEXT CHECK(storage_conditions IS NULL OR length(storage_conditions) <= 255)",
        "ALTER TABLE reagents ADD COLUMN appearance TEXT CHECK(appearance IS NULL OR length(appearance) <= 255)",
        "ALTER TABLE reagents ADD COLUMN hazard_pictograms TEXT CHECK(hazard_pictograms IS NULL OR length(hazard_pictograms) <= 100)",
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
        
        // ==================== EXPERIMENTS ====================
        "ALTER TABLE experiment_reagents ADD COLUMN is_consumed INTEGER NOT NULL DEFAULT 0 CHECK(is_consumed IN (0, 1))",
        "ALTER TABLE experiments ADD COLUMN location TEXT CHECK(location IS NULL OR length(location) <= 255)",
        "ALTER TABLE experiments ADD COLUMN room_id TEXT REFERENCES rooms(id)",
        "ALTER TABLE experiments ADD COLUMN experiment_type TEXT NOT NULL DEFAULT 'research' CHECK(experiment_type IN ('educational', 'research'))",
    ];

    for query in migration_queries.iter() {
        // Ignore errors for existing columns
        let _ = sqlx::query(query).execute(pool).await;
    }

    // Update original_quantity for existing batches
    let _ = sqlx::query("UPDATE batches SET original_quantity = quantity WHERE original_quantity IS NULL")
        .execute(pool)
        .await;

    // ==================== FTS MIGRATIONS ====================
    // Пересоздаём equipment_fts если она имеет старую схему с equipment_id
    // Проверяем есть ли колонка equipment_id в FTS (которой не должно быть)
    let has_old_fts: bool = sqlx::query_scalar(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('equipment_fts') WHERE name = 'equipment_id'"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if has_old_fts {
        log::info!("Recreating equipment_fts with correct schema...");
        
        // Удаляем старые триггеры
        let _ = sqlx::query("DROP TRIGGER IF EXISTS equipment_ai").execute(pool).await;
        let _ = sqlx::query("DROP TRIGGER IF EXISTS equipment_ad").execute(pool).await;
        let _ = sqlx::query("DROP TRIGGER IF EXISTS equipment_au").execute(pool).await;
        
        // Удаляем старую FTS таблицу
        let _ = sqlx::query("DROP TABLE IF EXISTS equipment_fts").execute(pool).await;
        
        // Создаём новую FTS таблицу с правильной схемой
        let _ = sqlx::query(
            r#"CREATE VIRTUAL TABLE IF NOT EXISTS equipment_fts USING fts5(
                name,
                description,
                location,
                manufacturer,
                model,
                serial_number,
                content='equipment',
                content_rowid='rowid',
                tokenize='unicode61'
            )"#
        ).execute(pool).await;
        
        // Пересоздаём триггеры
        let _ = sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS equipment_ai AFTER INSERT ON equipment BEGIN
               INSERT INTO equipment_fts(rowid, name, description, location, manufacturer, model, serial_number)
               VALUES (new.rowid, new.name, new.description, new.location, new.manufacturer, new.model, new.serial_number);
             END"
        ).execute(pool).await;
        
        let _ = sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS equipment_ad AFTER DELETE ON equipment BEGIN
               INSERT INTO equipment_fts(equipment_fts, rowid, name, description, location, manufacturer, model, serial_number)
               VALUES('delete', old.rowid, old.name, old.description, old.location, old.manufacturer, old.model, old.serial_number);
             END"
        ).execute(pool).await;
        
        let _ = sqlx::query(
            "CREATE TRIGGER IF NOT EXISTS equipment_au AFTER UPDATE ON equipment BEGIN
               INSERT INTO equipment_fts(equipment_fts, rowid, name, description, location, manufacturer, model, serial_number)
               VALUES('delete', old.rowid, old.name, old.description, old.location, old.manufacturer, old.model, old.serial_number);
               INSERT INTO equipment_fts(rowid, name, description, location, manufacturer, model, serial_number)
               VALUES (new.rowid, new.name, new.description, new.location, new.manufacturer, new.model, new.serial_number);
             END"
        ).execute(pool).await;
        
        // Перестраиваем FTS индекс из существующих данных
        let _ = sqlx::query("INSERT INTO equipment_fts(equipment_fts) VALUES('rebuild')").execute(pool).await;
        
        log::info!("equipment_fts recreated successfully");
    }

    Ok(())
}

// ==================== DATABASE RESET (DEVELOPMENT ONLY) ====================

pub async fn reset_database(pool: &SqlitePool) -> Result<()> {
    log::warn!("Resetting database - all data will be lost!");

    let drop_queries = [
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
        "DROP TABLE IF EXISTS batches",
        "DROP TABLE IF EXISTS reagents",
        "DROP TABLE IF EXISTS users",
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