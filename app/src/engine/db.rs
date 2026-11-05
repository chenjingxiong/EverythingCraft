use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use super::query::{compare_ignore_case, QueryEvaluator};
use super::types::{SearchOptions, SearchResultEntry, SortColumn, SortDirection};

#[derive(Debug, Clone, Default)]
pub struct PathPool {
    dirs: Vec<String>,
    lookup: HashMap<String, u32>,
}

impl PathPool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, dir: &str) -> u32 {
        if let Some(&id) = self.lookup.get(dir) {
            return id;
        }
        let id = self.dirs.len() as u32;
        let s = dir.to_string();
        self.dirs.push(s.clone());
        self.lookup.insert(s, id);
        id
    }

    pub fn get(&self, id: u32) -> &str {
        self.dirs.get(id as usize).map(|s| s.as_str()).unwrap_or("")
    }

    pub fn clear(&mut self) {
        self.dirs.clear();
        self.lookup.clear();
    }
}

#[derive(Debug, Clone)]
pub struct CompactEntry {
    pub id: u64,
    pub dir_id: u32,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
    pub created: i64,
    pub attributes: u32,
    pub extension: String,
}

impl CompactEntry {
    pub fn to_search_result(&self, pool: &PathPool) -> SearchResultEntry {
        let dir = pool.get(self.dir_id);
        let full_path = if dir.is_empty() {
            self.name.clone()
        } else if dir.ends_with('\\') || dir.ends_with('/') {
            format!("{}{}", dir, self.name)
        } else {
            format!("{}\\{}", dir, self.name)
        };

        SearchResultEntry {
            id: self.id,
            name: self.name.clone(),
            path: dir.to_string(),
            full_path,
            is_dir: self.is_dir,
            size: self.size,
            modified: self.modified,
            created: self.created,
            extension: self.extension.clone(),
            attributes: self.attributes,
        }
    }
}

pub struct Database {
    entries: RwLock<Vec<CompactEntry>>,
    path_pool: RwLock<PathPool>,
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

impl Database {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            path_pool: RwLock::new(PathPool::new()),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.read().map(|e| e.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        if let Ok(mut e) = self.entries.write() {
            e.clear();
        }
        if let Ok(mut p) = self.path_pool.write() {
            p.clear();
        }
    }

    pub fn set_entries(&self, new_entries: Vec<SearchResultEntry>) {
        let mut pool = match self.path_pool.write() {
            Ok(p) => p,
            Err(_) => return,
        };
        let mut entries = match self.entries.write() {
            Ok(e) => e,
            Err(_) => return,
        };

        pool.clear();
        entries.clear();
        entries.reserve(new_entries.len());

        for item in new_entries {
            let dir_id = pool.intern(&item.path);
            entries.push(CompactEntry {
                id: item.id,
                dir_id,
                name: item.name,
                is_dir: item.is_dir,
                size: item.size,
                modified: item.modified,
                created: item.created,
                attributes: item.attributes,
                extension: item.extension,
            });
        }
    }

    pub fn add_entries(&self, more_entries: Vec<SearchResultEntry>) {
        if more_entries.is_empty() {
            return;
        }
        let mut pool = match self.path_pool.write() {
            Ok(p) => p,
            Err(_) => return,
        };
        let mut entries = match self.entries.write() {
            Ok(e) => e,
            Err(_) => return,
        };

        entries.reserve(more_entries.len());
        for item in more_entries {
            let dir_id = pool.intern(&item.path);
            entries.push(CompactEntry {
                id: item.id,
                dir_id,
                name: item.name,
                is_dir: item.is_dir,
                size: item.size,
                modified: item.modified,
                created: item.created,
                attributes: item.attributes,
                extension: item.extension,
            });
        }
    }

    pub fn search(
        &self,
        query: &str,
        options: &SearchOptions,
        cancel_flag: Option<Arc<AtomicBool>>,
    ) -> Vec<SearchResultEntry> {
        let evaluator = QueryEvaluator::parse(query, options);

        let entries_guard = match self.entries.read() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };
        let pool_guard = match self.path_pool.read() {
            Ok(g) => g,
            Err(_) => return Vec::new(),
        };

        if entries_guard.is_empty() {
            return Vec::new();
        }

        // Parallel filter returning matching indices only
        let mut matched_indices: Vec<usize> = entries_guard
            .par_iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                if let Some(ref cf) = cancel_flag {
                    if cf.load(Ordering::Relaxed) {
                        return None;
                    }
                }
                if evaluator.matches_compact(entry, &pool_guard) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();

        // Sort indices based on options without allocating strings
        Self::sort_indices(&mut matched_indices, &entries_guard, &pool_guard, options);

        if let Some(limit) = options.max_results {
            matched_indices.truncate(limit);
        }

        // Only materialize SearchResultEntry for the top results
        matched_indices
            .into_iter()
            .map(|idx| entries_guard[idx].to_search_result(&pool_guard))
            .collect()
    }

    fn sort_indices(
        indices: &mut [usize],
        entries: &[CompactEntry],
        pool: &PathPool,
        options: &SearchOptions,
    ) {
        let sort_col = options.sort_column;
        let sort_dir = options.sort_direction;
        let folders_first = options.folders_first;

        indices.par_sort_unstable_by(|&idx_a, &idx_b| {
            let a = &entries[idx_a];
            let b = &entries[idx_b];

            if folders_first && a.is_dir != b.is_dir {
                return if a.is_dir {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }

            let cmp = match sort_col {
                SortColumn::Name => compare_ignore_case(&a.name, &b.name),
                SortColumn::Path => {
                    let dir_a = pool.get(a.dir_id);
                    let dir_b = pool.get(b.dir_id);
                    compare_ignore_case(dir_a, dir_b)
                }
                SortColumn::Size => a.size.cmp(&b.size),
                SortColumn::DateModified => a.modified.cmp(&b.modified),
                SortColumn::DateCreated => a.created.cmp(&b.created),
                SortColumn::Extension => compare_ignore_case(&a.extension, &b.extension),
                SortColumn::Attributes => a.attributes.cmp(&b.attributes),
            };

            match sort_dir {
                SortDirection::Ascending => cmp,
                SortDirection::Descending => cmp.reverse(),
            }
        });
    }

    pub fn get_statistics(&self) -> (usize, usize, u64) {
        let guard = match self.entries.read() {
            Ok(g) => g,
            Err(_) => return (0, 0, 0),
        };

        let mut total_files = 0;
        let mut total_folders = 0;
        let mut total_size = 0;

        for entry in guard.iter() {
            if entry.is_dir {
                total_folders += 1;
            } else {
                total_files += 1;
                total_size += entry.size;
            }
        }

        (total_files, total_folders, total_size)
    }
}
