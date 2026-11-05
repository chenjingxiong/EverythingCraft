use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::engine::{
    EverythingEngine, FilterCategory, SearchOptions, SearchResultEntry, SortColumn, SortDirection,
};
use crate::settings::{AppSettings, Language, SizeFormat};

#[derive(Debug, Clone)]
pub struct Bookmark {
    pub name: String,
    pub query: String,
    pub filter: FilterCategory,
}

pub struct AppModel {
    pub engine: EverythingEngine,
    pub search_query: String,
    pub search_options: SearchOptions,
    pub search_results: Vec<SearchResultEntry>,
    pub selected_index: Option<usize>,
    pub search_duration: Duration,
    pub status_message: String,
    pub bookmarks: Vec<Bookmark>,
    pub search_history: Vec<String>,
    pub is_indexing: bool,
    pub show_about_dialog: bool,
    pub show_bookmarks_dialog: bool,
    pub show_options_dialog: bool,
    pub export_status: Option<String>,
    pub screenshot_path: Option<PathBuf>,
    pub frame_counter: u32,
    pub target_frames: u32,
    // Settings + dirty flags
    pub settings: AppSettings,
    pub settings_backup: AppSettings,
    pub settings_dirty: bool, // needs style/font/theme re-apply
    pub needs_resave: bool,   // needs persistence
    pub last_window_rect: Option<egui::Rect>,
    // Options dialog state
    pub options_tab: usize,
    pub shortcut_capture: Option<crate::settings::ShortcutId>,
    // Debounced search-as-you-type
    pub search_pending: Option<Instant>,
    /// Ignore accent / case folding cache is inside engine; track last query to avoid re-runs
    last_executed_query: String,
}

impl AppModel {
    pub fn new(screenshot_path: Option<PathBuf>, settings: AppSettings) -> Self {
        let default_bookmarks = vec![
            Bookmark {
                name: "All Documents".to_string(),
                query: String::new(),
                filter: FilterCategory::Document,
            },
            Bookmark {
                name: "Executables".to_string(),
                query: String::new(),
                filter: FilterCategory::Executable,
            },
            Bookmark {
                name: "Large Files (>100MB)".to_string(),
                query: "size:>100MB".to_string(),
                filter: FilterCategory::Everything,
            },
            Bookmark {
                name: "Recently Modified (Today)".to_string(),
                query: "dm:today".to_string(),
                filter: FilterCategory::Everything,
            },
        ];

        let engine = EverythingEngine::new();

        let mut search_options = SearchOptions::default();
        search_options.max_results = Some(settings.max_results);

        let lang = settings.effective_language();
        let ready = crate::I18n::t(lang, "common.ready").to_string();
        Self {
            engine,
            search_query: String::new(),
            search_options,
            search_results: Vec::new(),
            selected_index: None,
            search_duration: Duration::ZERO,
            status_message: ready,
            bookmarks: default_bookmarks,
            search_history: Vec::new(),
            is_indexing: false,
            show_about_dialog: false,
            show_bookmarks_dialog: false,
            show_options_dialog: false,
            export_status: None,
            screenshot_path,
            frame_counter: 0,
            target_frames: 120,
            settings_backup: settings.clone(),
            settings,
            settings_dirty: false,
            needs_resave: false,
            last_window_rect: None,
            options_tab: 0,
            shortcut_capture: None,
            search_pending: None,
            last_executed_query: String::new(),
        }
    }

    pub fn lang(&self) -> Language {
        self.settings.effective_language()
    }

    pub fn t(&self, key: &str) -> String {
        crate::I18n::t(self.lang(), key)
    }

    /// Schedule a debounced search (search-as-you-type). 120ms feels instant but
    /// avoids hammering the engine on every keystroke.
    pub fn queue_search(&mut self) {
        if self.settings.search_as_you_type {
            self.search_pending = Some(Instant::now());
        } else {
            self.execute_search();
        }
    }

    pub fn tick_search_debounce(&mut self) -> bool {
        if let Some(started) = self.search_pending {
            if started.elapsed() >= Duration::from_millis(120) {
                self.search_pending = None;
                self.execute_search();
                return true;
            }
        }
        false
    }

    pub fn execute_search(&mut self) {
        if self.search_query == self.last_executed_query && !self.search_results.is_empty() {
            return;
        }
        let (results, duration) = self.engine.search(&self.search_query, &self.search_options);
        self.search_duration = duration;
        self.search_results = results;
        self.last_executed_query = self.search_query.clone();
        if self.selected_index.map_or(false, |idx| idx >= self.search_results.len()) {
            self.selected_index = if self.search_results.is_empty() { None } else { Some(0) };
        }
        if !self.search_query.is_empty() && !self.search_history.contains(&self.search_query) {
            self.search_history.push(self.search_query.clone());
            if self.search_history.len() > 30 {
                self.search_history.remove(0);
            }
        }
    }

    pub fn force_execute_search(&mut self) {
        self.last_executed_query.clear();
        self.execute_search();
    }

    pub fn set_filter(&mut self, cat: FilterCategory) {
        self.search_options.filter_category = cat;
        self.force_execute_search();
    }

    pub fn toggle_sort(&mut self, column: SortColumn) {
        if self.search_options.sort_column == column {
            self.search_options.sort_direction = match self.search_options.sort_direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.search_options.sort_column = column;
            self.search_options.sort_direction = SortDirection::Ascending;
        }
        self.force_execute_search();
    }

    pub fn selected_entry(&self) -> Option<&SearchResultEntry> {
        self.selected_index.and_then(|idx| self.search_results.get(idx))
    }

    pub fn format_size(&self, bytes: u64) -> String {
        match self.settings.size_format {
            SizeFormat::Bytes => bytes.to_string(),
            SizeFormat::Human => format_size_human(bytes),
        }
    }

    pub fn load_seed_samples(&mut self) {
        let win_dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".to_string());
        let mut sample_entries = Vec::new();
        let base_id = 1;

        let samples = [
            ("Everything.exe", r"C:\Program Files\Everything 1.5a", false, 5_042_344, "exe"),
            ("Everything.lng", r"C:\Program Files\Everything 1.5a", false, 1_033_107, "lng"),
            ("Everything-1.5a.ini", r"C:\Program Files\Everything 1.5a", false, 111_596, "ini"),
            ("notepad.exe", &format!(r"{}\System32", win_dir), false, 256_000, "exe"),
            ("explorer.exe", &win_dir, false, 5_120_000, "exe"),
            ("cmd.exe", &format!(r"{}\System32", win_dir), false, 320_000, "exe"),
            ("System32", &win_dir, true, 0, ""),
            ("Program Files", r"C:", true, 0, ""),
            ("Users", r"C:", true, 0, ""),
            ("Windows", r"C:", true, 0, ""),
            ("document.pdf", r"C:\Users\Public\Documents", false, 2_450_000, "pdf"),
            ("report_2026.docx", r"C:\Users\Public\Documents", false, 485_000, "docx"),
            ("setup_log.txt", r"C:\Windows\Logs", false, 45_200, "txt"),
            ("wallpaper.png", r"C:\Windows\Web\Wallpaper", false, 3_820_000, "png"),
            ("music_track.mp3", r"C:\Users\Public\Music", false, 8_940_000, "mp3"),
            ("backup_archive.zip", r"C:\Backups", false, 125_000_000, "zip"),
        ];

        let now_ts = chrono::Local::now().timestamp();

        for (i, (name, path, is_dir, size, ext)) in samples.iter().enumerate() {
            let full_path = format!(r"{}\{}", path.trim_end_matches('\\'), name);
            sample_entries.push(SearchResultEntry {
                id: (base_id + i) as u64,
                name: name.to_string(),
                path: path.to_string(),
                full_path,
                is_dir: *is_dir,
                size: *size,
                modified: now_ts - (i as i64 * 3600),
                created: now_ts - (i as i64 * 86400),
                extension: ext.to_string(),
                attributes: if *is_dir { 0x10 } else { 0x20 },
            });
        }

        self.engine.load_entries(sample_entries);
        self.force_execute_search();
    }
}

fn format_size_human(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bytes as f64;
    if b >= TB {
        format!("{:.2} TB", b / TB)
    } else if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}
