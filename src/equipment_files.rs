// src/equipment_files.rs
//! Where equipment files live on disk, and keeping the disk in step with
//! `equipment_files`.
//!
//! ```text
//! uploads/equipment/                       (EQUIPMENT_FILES_DIR)
//!   balance_ax_224_8121d530/               equipment: name + first 8 hex of its id
//!     photos/
//!     documents/                           manuals, certificates, other
//!     parts/
//!       pan_2_e2764c78/                    part: name + first 8 hex of its id
//!         photos/
//!         documents/
//!       _deleted/                          photos of parts deleted before this layout
//! ```
//!
//! The id suffix keeps two instruments with the same name (two "Pipette
//! 100 µL") apart and makes a folder safe to remove with its equipment.
//! Renaming equipment or a part moves its files; `reorganize_all` (startup)
//! moves files stored in any earlier layout.
//!
//! Deleting a part deletes its files. Before, the foreign key
//! `part_id ... ON DELETE SET NULL` turned a deleted part's photo into an
//! equipment photo, which then showed in place of the equipment's own.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use sqlx::SqlitePool;

use crate::error::{ApiError, ApiResult};

const PHOTOS: &str = "photos";
const DOCUMENTS: &str = "documents";
const PARTS: &str = "parts";
const DELETED_PARTS: &str = "_deleted";
const MAX_NAME_CHARS: usize = 48;

/// Root folder of all equipment files
pub fn files_root() -> PathBuf {
    std::env::var("EQUIPMENT_FILES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(".").join("uploads").join("equipment"))
}

/// Readable, filesystem-safe folder name: `balance_ax_224_8121d530`.
/// Letters (any script) and digits are kept, everything else becomes `_`,
/// so the result never holds a path separator, `..` or a drive prefix.
pub fn folder_name(name: &str, id: &str, fallback: &str) -> String {
    let mut base = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        let c = if c.is_alphanumeric() || c == '-' { c } else { '_' };
        if c == '_' && (base.is_empty() || base.ends_with('_')) {
            continue;
        }
        base.push(c);
        if base.chars().count() >= MAX_NAME_CHARS {
            break;
        }
    }
    let base = base.trim_matches(|c| c == '_' || c == '-');
    let base = if base.is_empty() { fallback } else { base };
    let short: String = id.chars().filter(|c| c.is_ascii_hexdigit()).take(8).collect::<String>().to_lowercase();
    if short.is_empty() {
        base.to_string()
    } else {
        format!("{}_{}", base, short)
    }
}

/// `photos` for photos, `documents` for every other file type
pub fn kind_folder(file_type: &str) -> &'static str {
    if file_type == "photo" { PHOTOS } else { DOCUMENTS }
}

/// Folder of one equipment
pub fn equipment_dir(root: &Path, equipment_name: &str, equipment_id: &str) -> PathBuf {
    root.join(folder_name(equipment_name, equipment_id, "equipment"))
}

/// Folder a file of this kind belongs in
pub fn file_dir(
    root: &Path,
    equipment_name: &str,
    equipment_id: &str,
    part: Option<(&str, &str)>, // (name, id)
    file_type: &str,
) -> PathBuf {
    let mut dir = equipment_dir(root, equipment_name, equipment_id);
    if let Some((part_name, part_id)) = part {
        dir = dir.join(PARTS).join(folder_name(part_name, part_id, "part"));
    }
    dir.join(kind_folder(file_type))
}

/// File extension from a client-supplied name: ASCII letters/digits only,
/// at most 10, else `bin`. (The name itself is never used on disk.)
pub fn safe_extension(original: &str) -> String {
    let ext = original.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    if !ext.is_empty() && ext.len() <= 10 && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        ext.to_ascii_lowercase()
    } else {
        "bin".to_string()
    }
}

/// Stored name for an upload: `<uuid>.<ext>`
pub fn stored_file_name(original: &str) -> String {
    format!("{}.{}", uuid::Uuid::new_v4(), safe_extension(original))
}

/// Same file, whatever the separators or a leading `./`
fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| -> Vec<String> {
        p.components()
            .filter(|c| !matches!(c, Component::CurDir))
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect()
    };
    norm(a) == norm(b)
}

/// Is `path` inside `root` (lexically; both as stored, no `..`)?
fn is_inside(root: &Path, path: &Path) -> bool {
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return false;
    }
    let strip = |p: &Path| -> Vec<String> {
        p.components()
            .filter(|c| !matches!(c, Component::CurDir))
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect()
    };
    let (r, p) = (strip(root), strip(path));
    p.len() > r.len() && p[..r.len()] == r[..]
}

/// Move a file, creating the target folder. `rename`, or copy + remove when
/// the folders are on different devices.
fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir)?;
    }
    match std::fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            std::fs::copy(from, to)?;
            std::fs::remove_file(from)
        }
    }
}

/// A free name in `dir` for `file_name` (adds `-1`, `-2`... if taken)
fn free_target(dir: &Path, file_name: &str) -> PathBuf {
    let first = dir.join(file_name);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = match file_name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{}", e)),
        None => (file_name.to_string(), String::new()),
    };
    (1..).map(|n| dir.join(format!("{}-{}{}", stem, n, ext))).find(|p| !p.exists()).unwrap()
}

/// Remove empty folders from `dir` up to (not including) `root`
fn prune_empty_dirs(root: &Path, dir: &Path) {
    let mut cur = dir.to_path_buf();
    while is_inside(root, &cur) {
        if std::fs::remove_dir(&cur).is_err() {
            break; // not empty, or gone
        }
        match cur.parent() {
            Some(p) => cur = p.to_path_buf(),
            None => break,
        }
    }
}

/// Delete a stored file and the folders it leaves empty. Paths outside the
/// files root are left alone (logged).
pub fn remove_stored_file(path: &str) {
    let root = files_root();
    let p = Path::new(path);
    if !is_inside(&root, p) {
        log::warn!("Equipment file outside {}: {} (not removed)", root.display(), path);
        return;
    }
    match std::fs::remove_file(p) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => log::warn!("Failed to remove {}: {}", path, e),
    }
    if let Some(dir) = p.parent() {
        prune_empty_dirs(&root, dir);
    }
}

// ==================== RELOCATION ====================

#[derive(Debug, Default, Clone, Copy)]
pub struct RelocateReport {
    pub moved: u64,
    pub missing: u64,
    pub orphaned: u64,
}

#[derive(sqlx::FromRow)]
struct FileRow {
    id: String,
    part_id: Option<String>,
    file_type: String,
    file_path: String,
    description: Option<String>,
}

/// Record a file's new place
async fn set_path(pool: &SqlitePool, row_id: &str, target: &Path) -> Result<(), sqlx::Error> {
    let name = target.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    sqlx::query("UPDATE equipment_files SET file_path = ?, stored_filename = ? WHERE id = ?")
        .bind(target.to_string_lossy().to_string()).bind(name).bind(row_id)
        .execute(pool).await.map(|_| ())
}

/// Move one file to `target` (a free name next to it if taken) and record it
async fn relocate_one(pool: &SqlitePool, root: &Path, row_id: &str, current: &Path, target: PathBuf) -> ApiResult<()> {
    let target = match (target.exists(), target.parent(), target.file_name()) {
        (true, Some(dir), Some(name)) => free_target(dir, &name.to_string_lossy()),
        _ => target,
    };
    move_file(current, &target)
        .map_err(|e| ApiError::InternalServerError(format!("Failed to move {}: {}", current.display(), e)))?;
    if let Err(e) = set_path(pool, row_id, &target).await {
        let _ = move_file(&target, current); // keep disk and DB in step
        return Err(e.into());
    }
    if let Some(dir) = current.parent() {
        prune_empty_dirs(root, dir);
    }
    Ok(())
}

/// Stored under `<root>/<equipment>/parts/<part>/...` (any earlier layout)
fn under_parts_folder(root: &Path, path: &Path) -> bool {
    let comps = |p: &Path| -> Vec<String> {
        p.components()
            .filter(|c| !matches!(c, Component::CurDir))
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect()
    };
    let (r, p) = (comps(root), comps(path));
    p.len() >= r.len() + 4 && p[..r.len()] == r[..] && p[r.len() + 1] == PARTS
}

/// Put every file of one equipment where the layout says (after a rename,
/// or files from an earlier layout). Files whose source is missing are left
/// as they are and counted.
pub async fn relocate_equipment(pool: &SqlitePool, equipment_id: &str) -> ApiResult<RelocateReport> {
    let root = files_root();
    let mut report = RelocateReport::default();
    let Some((eq_name,)) = sqlx::query_as::<_, (String,)>("SELECT name FROM equipment WHERE id = ?")
        .bind(equipment_id).fetch_optional(pool).await?
    else {
        return Ok(report);
    };
    let parts: HashMap<String, String> = sqlx::query_as::<_, (String, String)>(
        "SELECT id, name FROM equipment_parts WHERE equipment_id = ?",
    ).bind(equipment_id).fetch_all(pool).await?.into_iter().collect();
    let rows: Vec<FileRow> = sqlx::query_as(
        "SELECT id, part_id, file_type, file_path, description FROM equipment_files WHERE equipment_id = ?",
    ).bind(equipment_id).fetch_all(pool).await?;

    for row in rows {
        let current = PathBuf::from(&row.file_path);
        let Some(file_name) = current.file_name().map(|n| n.to_os_string()) else { continue };

        // A part photo whose part is gone: out of the equipment photos, file kept
        let part = row.part_id.as_ref().and_then(|pid| parts.get(pid).map(|n| (n.as_str(), pid.as_str())));
        let was_part_file = row.part_id.is_some()
            || row.description.as_deref().map_or(false, |d| d.starts_with("Part: "))
            || under_parts_folder(&root, &current);
        if part.is_none() && was_part_file {
            let target = equipment_dir(&root, &eq_name, equipment_id).join(PARTS).join(DELETED_PARTS).join(&file_name);
            if current.exists() && !same_path(&current, &target) {
                let target = free_target(target.parent().unwrap_or(&root), &file_name.to_string_lossy());
                if let Err(e) = move_file(&current, &target) {
                    log::warn!("Could not move orphaned part file {}: {}", current.display(), e);
                    continue;
                }
                if let Some(dir) = current.parent() {
                    prune_empty_dirs(&root, dir);
                }
            }
            sqlx::query("DELETE FROM equipment_files WHERE id = ?").bind(&row.id).execute(pool).await?;
            report.orphaned += 1;
            continue;
        }

        let target = file_dir(&root, &eq_name, equipment_id, part, &row.file_type).join(&file_name);
        if same_path(&current, &target) {
            continue;
        }
        if !current.exists() {
            if target.exists() {
                // moved earlier, DB not updated
                set_path(pool, &row.id, &target).await?;
                report.moved += 1;
            } else {
                report.missing += 1;
            }
            continue;
        }
        match relocate_one(pool, &root, &row.id, &current, target).await {
            Ok(()) => report.moved += 1,
            Err(e) => log::warn!("{}", e),
        }
    }
    Ok(report)
}

/// Startup: bring every equipment's files into the layout
pub async fn reorganize_all(pool: &SqlitePool) -> ApiResult<RelocateReport> {
    let ids: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT equipment_id FROM equipment_files",
    ).fetch_all(pool).await?;
    let mut total = RelocateReport::default();
    for (id,) in ids {
        let r = relocate_equipment(pool, &id).await?;
        total.moved += r.moved;
        total.missing += r.missing;
        total.orphaned += r.orphaned;
    }
    // Folders emptied before (deleting a file used to leave its folder)
    prune_empty_tree(&files_root());
    Ok(total)
}

/// Remove every empty folder below `root` (deepest first); never `root`
fn prune_empty_tree(root: &Path) {
    fn walk(dir: &Path, root: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let p = entry.path();
            // only real folders, never through a symlink
            if entry.file_type().map_or(false, |t| t.is_dir()) {
                walk(&p, root);
            }
        }
        if dir != root {
            let _ = std::fs::remove_dir(dir); // fails unless empty
        }
    }
    if root.is_dir() {
        walk(root, root);
    }
}

// ==================== DELETION ====================

/// Delete a part and its files (records in one transaction, then the disk)
pub async fn delete_part(pool: &SqlitePool, equipment_id: &str, part_id: &str) -> ApiResult<u64> {
    let mut tx = pool.begin().await?;
    let paths: Vec<(String,)> = sqlx::query_as(
        "SELECT file_path FROM equipment_files WHERE equipment_id = ? AND part_id = ?",
    ).bind(equipment_id).bind(part_id).fetch_all(&mut *tx).await?;
    sqlx::query("DELETE FROM equipment_files WHERE equipment_id = ? AND part_id = ?")
        .bind(equipment_id).bind(part_id).execute(&mut *tx).await?;
    let rows = sqlx::query("DELETE FROM equipment_parts WHERE id = ? AND equipment_id = ?")
        .bind(part_id).bind(equipment_id).execute(&mut *tx).await?.rows_affected();
    if rows == 0 {
        tx.rollback().await?;
        return Ok(0);
    }
    tx.commit().await?;
    for (p,) in paths {
        remove_stored_file(&p);
    }
    Ok(rows)
}

/// Permanently delete archived equipment (soft-deleted only) and its folder.
/// Returns 0, touching nothing, when the equipment is not in the archive.
pub async fn purge_equipment(pool: &SqlitePool, equipment_id: &str) -> ApiResult<u64> {
    let mut tx = pool.begin().await?;
    let Some((name,)) = sqlx::query_as::<_, (String,)>(
        "SELECT name FROM equipment WHERE id = ? AND deleted_at IS NOT NULL",
    ).bind(equipment_id).fetch_optional(&mut *tx).await?
    else {
        tx.rollback().await?;
        return Ok(0);
    };
    let paths: Vec<(String,)> = sqlx::query_as("SELECT file_path FROM equipment_files WHERE equipment_id = ?")
        .bind(equipment_id).fetch_all(&mut *tx).await?;
    sqlx::query("DELETE FROM equipment_files WHERE equipment_id = ?").bind(equipment_id).execute(&mut *tx).await?;
    let rows = sqlx::query("DELETE FROM equipment WHERE id = ? AND deleted_at IS NOT NULL")
        .bind(equipment_id).execute(&mut *tx).await?.rows_affected();
    tx.commit().await?;

    for (p,) in paths {
        remove_stored_file(&p);
    }
    // the equipment's own folder (unique by id), with anything left in it
    let root = files_root();
    let dir = equipment_dir(&root, &name, equipment_id);
    if is_inside(&root, &dir) && dir.is_dir() {
        if let Err(e) = std::fs::remove_dir_all(&dir) {
            log::warn!("Failed to remove {}: {}", dir.display(), e);
        }
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names() {
        let id = "8121d530-efb4-446f-bb32-bbc10dd21e34";
        assert_eq!(folder_name("Balance AX/224", id, "equipment"), "balance_ax_224_8121d530");
        assert_eq!(folder_name("  Pipette 100 µL ", id, "x"), "pipette_100_µl_8121d530");
        assert_eq!(folder_name("Спектрофотометр УФ-1800", id, "x"), "спектрофотометр_уф-1800_8121d530");
        assert_eq!(folder_name("Quyruq qızdırıcı", id, "x"), "quyruq_qızdırıcı_8121d530");
        // nothing that could leave the folder
        for evil in ["../../etc", "..", "C:\\Windows", "/abs/path", "a\0b", "CON", "a/../b"] {
            let f = folder_name(evil, id, "equipment");
            assert!(!f.contains('/') && !f.contains('\\') && !f.contains("..") && !f.contains(':') && !f.contains('\0'), "{} -> {}", evil, f);
            assert!(f.ends_with("_8121d530"), "{} -> {}", evil, f);
        }
        assert_eq!(folder_name("!!!", id, "equipment"), "equipment_8121d530");
        assert_eq!(folder_name("", "", "part"), "part");
        assert!(folder_name(&"x".repeat(500), id, "e").chars().count() <= MAX_NAME_CHARS + 9);
    }

    #[test]
    fn extensions() {
        assert_eq!(safe_extension("photo.JPG"), "jpg");
        assert_eq!(safe_extension("manual.pdf"), "pdf");
        assert_eq!(safe_extension("noext"), "bin");
        assert_eq!(safe_extension("x.\\..\\..\\evil"), "bin");
        assert_eq!(safe_extension("x./../evil"), "bin");
        assert_eq!(safe_extension("a.tar.gz"), "gz");
        assert_eq!(safe_extension("x.verylongextension"), "bin");
        assert!(stored_file_name("a.png").ends_with(".png"));
    }

    #[test]
    fn layout() {
        let root = Path::new("./uploads/equipment");
        let id = "8121d530-efb4-446f-bb32-bbc10dd21e34";
        let pid = "e2764c78-32e6-4d72-b667-1bb5a95ee301";
        assert_eq!(file_dir(root, "Balance", id, None, "photo"), root.join("balance_8121d530/photos"));
        assert_eq!(file_dir(root, "Balance", id, None, "manual"), root.join("balance_8121d530/documents"));
        assert_eq!(file_dir(root, "Balance", id, Some(("Pan 2", pid)), "photo"), root.join("balance_8121d530/parts/pan_2_e2764c78/photos"));
    }

    #[test]
    fn paths() {
        let root = Path::new("./uploads/equipment");
        assert!(is_inside(root, Path::new("./uploads/equipment/a/photos/x.png")));
        assert!(is_inside(root, Path::new("uploads/equipment/a")));
        assert!(!is_inside(root, Path::new("./uploads/equipment")));
        assert!(!is_inside(root, Path::new("./uploads/equipment/../secret")));
        assert!(!is_inside(root, Path::new("/etc/passwd")));
        assert!(same_path(Path::new("./uploads/a/b.png"), Path::new("uploads/a/b.png")));
        assert!(!same_path(Path::new("uploads/a/b.png"), Path::new("uploads/a/c.png")));
        // part folders of any layout; equipment that is itself named "parts" is not one
        assert!(under_parts_folder(root, Path::new("./uploads/equipment/bal/parts/pan/images/x.png")));
        assert!(under_parts_folder(root, Path::new("./uploads/equipment/bal_1/parts/pan_2/photos/x.png")));
        assert!(!under_parts_folder(root, Path::new("./uploads/equipment/parts/images/x.png")));
        assert!(!under_parts_folder(root, Path::new("./uploads/equipment/bal/images/x.png")));
    }
}
