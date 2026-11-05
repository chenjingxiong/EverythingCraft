pub mod db;
pub mod export;
pub mod ntfs;
pub mod query;
pub mod scanner;
pub mod types;

pub use db::Database;
pub use export::*;
pub use types::*;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct EverythingEngine {
    db: Arc<Database>,
    is_indexing: Arc<AtomicBool>,
    cancel_indexing: Arc<AtomicBool>,
    cancel_search: Arc<AtomicBool>,
}

impl Default for EverythingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl EverythingEngine {
    pub fn new() -> Self {
        Self {
            db: Arc::new(Database::new()),
            is_indexing: Arc::new(AtomicBool::new(false)),
            cancel_indexing: Arc::new(AtomicBool::new(false)),
            cancel_search: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_database(db: Arc<Database>) -> Self {
        Self {
            db,
            is_indexing: Arc::new(AtomicBool::new(false)),
            cancel_indexing: Arc::new(AtomicBool::new(false)),
            cancel_search: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn database(&self) -> Arc<Database> {
        self.db.clone()
    }

    pub fn is_indexing(&self) -> bool {
        self.is_indexing.load(Ordering::Relaxed)
    }

    pub fn cancel_indexing(&self) {
        self.cancel_indexing.store(true, Ordering::Relaxed);
    }

    pub fn cancel_search(&self) {
        self.cancel_search.store(true, Ordering::Relaxed);
    }

    pub fn get_indexed_count(&self) -> usize {
        self.db.len()
    }

    pub fn get_statistics(&self) -> (usize, usize, u64) {
        self.db.get_statistics()
    }

    /// Index all available local drives
    pub fn index_all_drives(&self) -> Result<usize, String> {
        if self.is_indexing.swap(true, Ordering::SeqCst) {
            return Err("Indexing already in progress".to_string());
        }
        self.cancel_indexing.store(false, Ordering::Relaxed);

        let drives = scanner::list_windows_drives();
        let mut total_indexed = 0;

        for drive in &drives {
            if self.cancel_indexing.load(Ordering::Relaxed) {
                break;
            }

            // Attempt NTFS USN Journal first if drive is lettered (like C:\)
            let mut scanned = false;
            let drive_str = drive.to_string_lossy();
            if let Some(letter) = drive_str.chars().next() {
                if letter.is_ascii_alphabetic() {
                    match ntfs::scan_volume_usn(letter) {
                        Ok(usn_entries) => {
                            let count = usn_entries.len();
                            self.db.add_entries(usn_entries);
                            total_indexed += count;
                            scanned = true;
                        }
                        Err(_) => {
                            // Non-admin or non-NTFS volume, will fallback below
                        }
                    }
                }
            }

            // Fallback to batched filesystem scan
            if !scanned {
                let db = self.db.clone();
                let counter = Arc::new(std::sync::atomic::AtomicU64::new(total_indexed as u64 + 1));
                scanner::scan_single_root_batched(
                    drive,
                    self.cancel_indexing.clone(),
                    counter,
                    5000,
                    |batch| {
                        total_indexed += batch.len();
                        db.add_entries(batch);
                    },
                );
            }
        }

        self.is_indexing.store(false, Ordering::SeqCst);
        Ok(total_indexed)
    }

    /// Index a specific directory path
    pub fn index_path(&self, path: &Path) -> usize {
        self.cancel_indexing.store(false, Ordering::Relaxed);
        let entries = scanner::scan_single_root(
            path,
            self.cancel_indexing.clone(),
            Arc::new(std::sync::atomic::AtomicU64::new(self.db.len() as u64 + 1)),
        );
        let count = entries.len();
        self.db.add_entries(entries);
        count
    }

    /// Load entries directly (e.g. for seed data or testing)
    pub fn load_entries(&self, entries: Vec<SearchResultEntry>) {
        self.db.set_entries(entries);
    }

    /// Perform a synchronous search
    pub fn search(&self, query: &str, options: &SearchOptions) -> (Vec<SearchResultEntry>, std::time::Duration) {
        self.cancel_search.store(false, Ordering::Relaxed);
        let start = Instant::now();
        let results = self.db.search(query, options, Some(self.cancel_search.clone()));
        let duration = start.elapsed();
        (results, duration)
    }

    /// Stream search results in batches
    pub fn search_stream<F>(
        &self,
        query: &str,
        options: &SearchOptions,
        batch_size: usize,
        mut on_batch: F,
    ) where
        F: FnMut(Vec<SearchResultEntry>, usize, bool), // batch, total so far, is_done
    {
        let (all, _) = self.search(query, options);
        let total = all.len();
        if all.is_empty() {
            on_batch(Vec::new(), 0, true);
            return;
        }

        let mut offset = 0;
        while offset < total {
            if self.cancel_search.load(Ordering::Relaxed) {
                break;
            }
            let end = (offset + batch_size).min(total);
            let batch = all[offset..end].to_vec();
            let is_done = end == total;
            on_batch(batch, end, is_done);
            offset = end;
        }
    }
}
