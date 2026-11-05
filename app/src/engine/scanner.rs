use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use walkdir::WalkDir;

use super::types::SearchResultEntry;

pub fn list_windows_drives() -> Vec<PathBuf> {
    let mut drives = Vec::new();
    #[cfg(windows)]
    {
        for letter in b'A'..=b'Z' {
            let path_str = format!("{}:\\", letter as char);
            let path = Path::new(&path_str);
            if path.exists() {
                drives.push(path.to_path_buf());
            }
        }
    }
    #[cfg(not(windows))]
    {
        drives.push(PathBuf::from("/"));
    }
    drives
}

pub fn is_ignored_directory_name(name: &str) -> bool {
    matches!(
        name,
        "$RECYCLE.BIN"
            | "$Recycle.Bin"
            | "System Volume Information"
            | ".git"
            | "node_modules"
            | "target"
            | "dist"
            | ".cache"
            | "__pycache__"
            | ".venv"
            | "venv"
    )
}

pub fn scan_single_root(
    root: &Path,
    cancel_flag: Arc<AtomicBool>,
    id_counter: Arc<AtomicU64>,
) -> Vec<SearchResultEntry> {
    let mut entries = Vec::with_capacity(20_000);
    scan_single_root_batched(root, cancel_flag, id_counter, 50_000, |batch| {
        entries.extend(batch);
    });
    entries
}

pub fn scan_single_root_batched<F>(
    root: &Path,
    cancel_flag: Arc<AtomicBool>,
    id_counter: Arc<AtomicU64>,
    batch_size: usize,
    mut on_batch: F,
) where
    F: FnMut(Vec<SearchResultEntry>),
{
    let mut batch = Vec::with_capacity(batch_size.min(10_000));
    let mut total_scanned = 0;
    const MAX_FALLBACK_FILES_PER_DRIVE: usize = 300_000;

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !is_ignored_directory_name(name.as_ref())
        });

    for entry_res in walker {
        if cancel_flag.load(Ordering::Relaxed) || total_scanned >= MAX_FALLBACK_FILES_PER_DRIVE {
            break;
        }

        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let is_dir = metadata.is_dir();
        let file_name = entry.file_name().to_string_lossy().to_string();

        let parent_str = path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let full_path_str = path.to_string_lossy().to_string();

        let ext = if is_dir {
            String::new()
        } else {
            path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase()
        };

        let size = if is_dir { 0 } else { metadata.len() };

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let created = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(modified);

        let attributes = if is_dir { 0x10 } else { 0x20 };
        let id = id_counter.fetch_add(1, Ordering::Relaxed);

        batch.push(SearchResultEntry {
            id,
            name: file_name,
            path: parent_str,
            full_path: full_path_str,
            is_dir,
            size,
            modified,
            created,
            extension: ext,
            attributes,
        });

        total_scanned += 1;

        if batch.len() >= batch_size {
            on_batch(std::mem::take(&mut batch));
        }
    }

    if !batch.is_empty() {
        on_batch(batch);
    }
}
