use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "zh-CN")]
    Chinese,
    #[serde(rename = "en")]
    English,
}

impl Language {
    pub fn from_os() -> Self {
        #[cfg(windows)]
        {
            #[link(name = "kernel32")]
            extern "system" {
                fn GetUserDefaultUILanguage() -> u16;
            }
            let lang_id = unsafe { GetUserDefaultUILanguage() };
            // Primary language 0x04 is Chinese (0x0804 = zh-CN, 0x0404 = zh-TW, 0x0c04 = zh-HK)
            if (lang_id & 0x3ff) == 0x04 {
                return Language::Chinese;
            }
        }
        let env_lang = std::env::var("LANG").unwrap_or_default().to_lowercase();
        if env_lang.contains("zh") || env_lang.contains("chinese") {
            Language::Chinese
        } else {
            Language::English
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Language::Chinese => "中文 (简体)",
            Language::English => "English",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[serde(rename = "dark")]
    Dark,
    #[serde(rename = "light")]
    Light,
}

impl Theme {
    pub fn name(&self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShortcutId {
    NewWindow,
    Open,
    OpenPath,
    CopyPath,
    CopyName,
    SelectAll,
    AddBookmark,
    ClearSearch,
    RefreshIndex,
    ToggleMatchCase,
    ToggleMatchPath,
    ToggleMatchWord,
    ToggleRegex,
    FocusSearch,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Options,
    Exit,
}

impl ShortcutId {
    pub fn all() -> &'static [ShortcutId] {
        &[
            ShortcutId::NewWindow,
            ShortcutId::Open,
            ShortcutId::OpenPath,
            ShortcutId::CopyPath,
            ShortcutId::CopyName,
            ShortcutId::SelectAll,
            ShortcutId::AddBookmark,
            ShortcutId::ClearSearch,
            ShortcutId::RefreshIndex,
            ShortcutId::ToggleMatchCase,
            ShortcutId::ToggleMatchPath,
            ShortcutId::ToggleMatchWord,
            ShortcutId::ToggleRegex,
            ShortcutId::FocusSearch,
            ShortcutId::ZoomIn,
            ShortcutId::ZoomOut,
            ShortcutId::ZoomReset,
            ShortcutId::Options,
            ShortcutId::Exit,
        ]
    }

    pub fn key(&self) -> &'static str {
        match self {
            ShortcutId::NewWindow => "new_window",
            ShortcutId::Open => "open",
            ShortcutId::OpenPath => "open_path",
            ShortcutId::CopyPath => "copy_path",
            ShortcutId::CopyName => "copy_name",
            ShortcutId::SelectAll => "select_all",
            ShortcutId::AddBookmark => "add_bookmark",
            ShortcutId::ClearSearch => "clear_search",
            ShortcutId::RefreshIndex => "refresh_index",
            ShortcutId::ToggleMatchCase => "toggle_match_case",
            ShortcutId::ToggleMatchPath => "toggle_match_path",
            ShortcutId::ToggleMatchWord => "toggle_match_word",
            ShortcutId::ToggleRegex => "toggle_regex",
            ShortcutId::FocusSearch => "focus_search",
            ShortcutId::ZoomIn => "zoom_in",
            ShortcutId::ZoomOut => "zoom_out",
            ShortcutId::ZoomReset => "zoom_reset",
            ShortcutId::Options => "options",
            ShortcutId::Exit => "exit",
        }
    }
}

pub fn default_shortcut(id: ShortcutId) -> String {
    match id {
        ShortcutId::NewWindow => "Ctrl+N",
        ShortcutId::Open => "Enter",
        ShortcutId::OpenPath => "Ctrl+Enter",
        ShortcutId::CopyPath => "Ctrl+Shift+C",
        ShortcutId::CopyName => "Ctrl+C",
        ShortcutId::SelectAll => "Ctrl+A",
        ShortcutId::AddBookmark => "Ctrl+D",
        ShortcutId::ClearSearch => "Esc",
        ShortcutId::RefreshIndex => "F5",
        ShortcutId::ToggleMatchCase => "Ctrl+I",
        ShortcutId::ToggleMatchPath => "Ctrl+U",
        ShortcutId::ToggleMatchWord => "Ctrl+W",
        ShortcutId::ToggleRegex => "Ctrl+R",
        ShortcutId::FocusSearch => "Ctrl+F",
        ShortcutId::ZoomIn => "Ctrl+=",
        ShortcutId::ZoomOut => "Ctrl+-",
        ShortcutId::ZoomReset => "Ctrl+0",
        ShortcutId::Options => "Ctrl+P",
        ShortcutId::Exit => "Alt+F4",
    }
    .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeFormat {
    Human,
    Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ColumnSettings {
    pub show_name: bool,
    pub show_path: bool,
    pub show_size: bool,
    pub show_modified: bool,
    pub show_created: bool,
    pub show_extension: bool,
    pub show_attributes: bool,
    pub width_name: f32,
    pub width_path: f32,
    pub width_size: f32,
    pub width_modified: f32,
    pub width_created: f32,
    pub width_extension: f32,
    pub width_attributes: f32,
}

impl Default for ColumnSettings {
    fn default() -> Self {
        Self {
            show_name: true,
            show_path: true,
            show_size: true,
            show_modified: true,
            show_created: false,
            show_extension: false,
            show_attributes: false,
            width_name: 300.0,
            width_path: 380.0,
            width_size: 110.0,
            width_modified: 150.0,
            width_created: 150.0,
            width_extension: 80.0,
            width_attributes: 80.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub language: Option<Language>, // None = follow OS
    pub theme: Theme,
    pub zoom_percent: f32,
    pub shortcuts: BTreeMap<String, String>,
    // Index options
    pub index_ntfs: bool,
    pub index_hidden: bool,
    pub index_system: bool,
    pub index_network_drives: bool,
    pub selected_network_drives: Vec<String>,
    pub selected_local_drives: Vec<String>,
    pub exclude_folders: Vec<String>,
    // Search options
    pub max_results: usize,
    pub search_as_you_type: bool,
    // View options
    pub show_status_bar: bool,
    pub show_header: bool,
    pub alternate_row_color: bool,
    pub show_icons: bool,
    pub show_path_in_title: bool,
    pub date_format: String,
    pub size_format: SizeFormat,
    pub columns: ColumnSettings,
    // Window
    pub window_width: f32,
    pub window_height: f32,
    pub window_maximized: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: None,
            theme: Theme::Dark,
            zoom_percent: 100.0,
            shortcuts: BTreeMap::new(),
            index_ntfs: true,
            index_hidden: true,
            index_system: true,
            index_network_drives: true,
            selected_network_drives: vec!["Y:".to_string(), "Z:".to_string()],
            selected_local_drives: vec!["C:".to_string(), "D:".to_string(), "F:".to_string()],
            exclude_folders: vec![
                "$RECYCLE.BIN".to_string(),
                "System Volume Information".to_string(),
            ],
            max_results: 100_000,
            search_as_you_type: true,
            show_status_bar: true,
            show_header: true,
            alternate_row_color: true,
            show_icons: true,
            show_path_in_title: true,
            date_format: "%Y/%m/%d %H:%M".to_string(),
            size_format: SizeFormat::Human,
            columns: ColumnSettings::default(),
            window_width: 1022.0,
            window_height: 713.0,
            window_maximized: false,
        }
    }
}

impl AppSettings {
    pub fn shortcut(&self, id: ShortcutId) -> String {
        self.shortcuts
            .get(id.key())
            .cloned()
            .unwrap_or_else(|| default_shortcut(id))
    }

    pub fn set_shortcut(&mut self, id: ShortcutId, combo: String) {
        self.shortcuts.insert(id.key().to_string(), combo);
    }

    pub fn reset_shortcuts(&mut self) {
        self.shortcuts.clear();
    }

    pub fn effective_language(&self) -> Language {
        self.language.unwrap_or_else(Language::from_os)
    }

    pub fn load() -> Self {
        for candidate in Self::config_paths() {
            if let Ok(text) = fs::read_to_string(&candidate) {
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&text) {
                    return Self::sanitize(settings);
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::primary_config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, text).map_err(|e| e.to_string())
    }

    fn sanitize(mut s: Self) -> Self {
        if !(50.0..=500.0).contains(&s.zoom_percent) || !s.zoom_percent.is_finite() {
            s.zoom_percent = 100.0;
        }
        if s.max_results == 0 {
            s.max_results = 100_000;
        }
        s
    }

    fn exe_dir() -> Option<PathBuf> {
        std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()))
    }

    fn primary_config_path() -> PathBuf {
        if let Some(dir) = Self::exe_dir() {
            let portable = dir.join("Everything-Settings.json");
            if portable.exists() {
                return portable;
            }
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("EverythingCraft").join("settings.json");
        }
        PathBuf::from("Everything-Settings.json")
    }

    fn config_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Some(dir) = Self::exe_dir() {
            paths.push(dir.join("Everything-Settings.json"));
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            paths.push(PathBuf::from(appdata).join("EverythingCraft").join("settings.json"));
        }
        paths
    }
}

pub struct I18n;

impl I18n {
    pub fn t(lang: Language, key: &str) -> String {
        match lang {
            Language::English => I18n::en(key),
            Language::Chinese => I18n::zh(key),
        }
    }

    fn en(key: &str) -> String {
        match key {
            "menu.file" => "File".into(),
            "menu.edit" => "Edit".into(),
            "menu.view" => "View".into(),
            "menu.search" => "Search".into(),
            "menu.bookmarks" => "Bookmarks".into(),
            "menu.tools" => "Tools".into(),
            "menu.help" => "Help".into(),
            "menu.options" => "Options...".into(),
            "menu.exit" => "Exit".into(),
            "file.new_window" => "New Window".into(),
            "file.open" => "Open".into(),
            "file.open_path" => "Open Path".into(),
            "file.export" => "Export...".into(),
            "file.export_csv" => "Export to CSV...".into(),
            "file.export_tsv" => "Export to TSV...".into(),
            "file.export_efu" => "Export to EFU...".into(),
            "file.export_txt" => "Export to TXT...".into(),
            "file.export_json" => "Export to JSON...".into(),
            "edit.copy_path" => "Copy Full Path".into(),
            "edit.copy_name" => "Copy File Name".into(),
            "edit.select_all" => "Select All".into(),
            "view.folders_first" => "Folders First".into(),
            "view.refresh" => "Refresh".into(),
            "view.status_bar" => "Status Bar".into(),
            "view.header" => "Column Header".into(),
            "view.alternate_row" => "Alternate Row Colors".into(),
            "view.zoom_in" => "Zoom In".into(),
            "view.zoom_out" => "Zoom Out".into(),
            "view.zoom_reset" => "Zoom 100%".into(),
            "search.match_case" => "Match Case".into(),
            "search.match_whole_word" => "Match Whole Word".into(),
            "search.match_path" => "Match Path".into(),
            "search.regex" => "Enable Regex".into(),
            "search.clear" => "Clear Search".into(),
            "search.hint" => "Type to search (wildcards *, ?, filters: size:>10MB dm:today ext:exe path:...)".into(),
            "bookmarks.add" => "Add to Bookmarks...".into(),
            "bookmarks.manage" => "Manage Bookmarks".into(),
            "tools.rebuild" => "Force Rebuild Index".into(),
            "tools.indexing" => "Indexing in progress...".into(),
            "help.about" => "About Everything".into(),
            "col.name" => "Name".into(),
            "col.path" => "Path".into(),
            "col.size" => "Size".into(),
            "col.modified" => "Date Modified".into(),
            "col.created" => "Date Created".into(),
            "col.extension" => "Extension".into(),
            "col.attributes" => "Attributes".into(),
            "view.columns" => "Columns".into(),
            "view.show_icons" => "Show Icons".into(),
            "opt.show_icons" => "Show icons next to files and folders".into(),
            "opt.show_path_in_title" => "Show full path in window title".into(),
            "opt.columns" => "Columns".into(),
            "opt.reset_columns" => "Reset Column Widths".into(),
            "tools.run_as_admin" => "Run as administrator".into(),
            "tools.is_admin" => "Running as administrator".into(),
            "opt.run_as_admin_desc" => "NTFS USN Journal indexing requires administrator privileges.".into(),
            "opt.restart_as_admin" => "Restart as Administrator".into(),
            "opt.title" => "Options".into(),
            "opt.general" => "General".into(),
            "opt.language" => "Language".into(),
            "opt.language_auto" => "Auto (follow OS)".into(),
            "opt.theme" => "Theme".into(),
            "opt.theme_dark" => "Dark".into(),
            "opt.theme_light" => "Light".into(),
            "opt.scale" => "Scale / Zoom".into(),
            "opt.scale_presets" => "Presets".into(),
            "opt.shortcuts" => "Keyboard".into(),
            "opt.index" => "Indexes".into(),
            "opt.index_ntfs" => "Enable NTFS USN Journal (fast MFT scan)".into(),
            "opt.index_hidden" => "Index hidden files and folders".into(),
            "opt.index_system" => "Index system files and folders".into(),
            "opt.index_network" => "Index Network Drives".into(),
            "opt.index_network_desc" => "Select network drives to index:".into(),
            "opt.index_local_desc" => "Local Fixed Drives:".into(),
            "opt.rebuild_now" => "Rebuild Index Now".into(),
            "opt.no_network_drives" => "No mapped network drives detected (Map network drives in Windows)".into(),
            "opt.exclude" => "Exclude folders (semicolon separated)".into(),
            "opt.view" => "View".into(),
            "opt.status_bar" => "Show status bar".into(),
            "opt.header" => "Show column header".into(),
            "opt.alternate_row" => "Alternate row colors".into(),
            "opt.size_format" => "Size format".into(),
            "opt.size_human" => "Human readable (KB/MB/GB)".into(),
            "opt.size_bytes" => "Bytes".into(),
            "opt.date_format" => "Date format".into(),
            "opt.search" => "Search".into(),
            "opt.max_results" => "Result limit".into(),
            "opt.as_you_type" => "Search as you type".into(),
            "opt.focus_search" => "Focus Search Box".into(),
            "opt.shortcut_action" => "Command / Action".into(),
            "opt.shortcut_keys" => "Shortcut Key".into(),
            "opt.reset_shortcuts" => "Reset All Shortcuts to Default".into(),
            "opt.press_keys" => "Press key combination... (Esc to cancel)".into(),
            "opt.ok" => "OK".into(),
            "opt.cancel" => "Cancel".into(),
            "opt.apply" => "Apply".into(),
            "opt.restart_hint" => "Changes take effect immediately".into(),
            "status.objects" => "objects".into(),
            "status.selected" => "of".into(),
            "status.indexing" => "Indexing...".into(),
            "status.db" => "DB".into(),
            "status.filter" => "Filter".into(),
            "dlg.bookmark_title" => "Add Bookmark".into(),
            "dlg.bookmark_query" => "Query".into(),
            "dlg.bookmark_save" => "Save".into(),
            "dlg.bookmark_cancel" => "Cancel".into(),
            "dlg.about_body" => "Everything Fast Search Engine\nBuilt with Rust + egui.\nSupports NTFS USN Journal & parallel instant query.".into(),
            "res.none" => "No files match the search query.".into(),
            "common.ready" => "Ready".into(),
            _ => key.to_string(),
        }
    }

    fn zh(key: &str) -> String {
        match key {
            "menu.file" => "文件(F)".into(),
            "menu.edit" => "编辑(E)".into(),
            "menu.view" => "视图(V)".into(),
            "menu.search" => "搜索(S)".into(),
            "menu.bookmarks" => "书签(B)".into(),
            "menu.tools" => "工具(T)".into(),
            "menu.help" => "帮助(H)".into(),
            "menu.options" => "选项(O)...".into(),
            "menu.exit" => "退出(X)".into(),
            "file.new_window" => "新建窗口".into(),
            "file.open" => "打开".into(),
            "file.open_path" => "打开所在路径".into(),
            "file.export" => "导出...".into(),
            "file.export_csv" => "导出为 CSV...".into(),
            "file.export_tsv" => "导出为 TSV...".into(),
            "file.export_efu" => "导出为 EFU (Everything文件列表)...".into(),
            "file.export_txt" => "导出为 TXT...".into(),
            "file.export_json" => "导出为 JSON...".into(),
            "edit.copy_path" => "复制完整路径".into(),
            "edit.copy_name" => "复制文件名".into(),
            "edit.select_all" => "全选".into(),
            "view.folders_first" => "文件夹置顶".into(),
            "view.refresh" => "刷新".into(),
            "view.status_bar" => "状态栏".into(),
            "view.header" => "列标题".into(),
            "view.alternate_row" => "交替行背景色".into(),
            "view.zoom_in" => "放大".into(),
            "view.zoom_out" => "缩小".into(),
            "view.zoom_reset" => "默认大小 100%".into(),
            "search.match_case" => "区分大小写".into(),
            "search.match_whole_word" => "全字匹配".into(),
            "search.match_path" => "匹配路径".into(),
            "search.regex" => "使用正则表达式".into(),
            "search.clear" => "清空搜索".into(),
            "search.hint" => "输入关键字搜索 (通配符 *, ?, 语法: size:>10MB dm:today ext:exe path:...)".into(),
            "bookmarks.add" => "添加到书签...".into(),
            "bookmarks.manage" => "管理书签".into(),
            "tools.rebuild" => "强制重建索引".into(),
            "tools.indexing" => "正在建立索引...".into(),
            "help.about" => "关于 Everything".into(),
            "col.name" => "名称".into(),
            "col.path" => "路径".into(),
            "col.size" => "大小".into(),
            "col.modified" => "修改时间".into(),
            "col.created" => "创建时间".into(),
            "col.extension" => "扩展名".into(),
            "col.attributes" => "属性".into(),
            "view.columns" => "选取列".into(),
            "view.show_icons" => "显示图标".into(),
            "opt.show_icons" => "显示文件和文件夹图标".into(),
            "opt.show_path_in_title" => "在窗口标题栏显示完整路径".into(),
            "opt.columns" => "显示的列".into(),
            "opt.reset_columns" => "恢复默认列宽".into(),
            "tools.run_as_admin" => "以管理员身份运行".into(),
            "tools.is_admin" => "已以管理员身份运行".into(),
            "opt.run_as_admin_desc" => "NTFS USN 日志直接读取 MFT 需要管理员权限。".into(),
            "opt.restart_as_admin" => "以管理员身份重启".into(),
            "opt.title" => "选项".into(),
            "opt.general" => "常规".into(),
            "opt.language" => "语言 (Language)".into(),
            "opt.language_auto" => "自动 (跟随系统语言)".into(),
            "opt.theme" => "主题外观".into(),
            "opt.theme_dark" => "深色主题".into(),
            "opt.theme_light" => "浅色主题".into(),
            "opt.scale" => "界面缩放 (Scale / Zoom)".into(),
            "opt.scale_presets" => "预设缩放".into(),
            "opt.shortcuts" => "快捷键".into(),
            "opt.index" => "索引".into(),
            "opt.index_ntfs" => "启用 NTFS USN 日志 (高速直接扫描 MFT)".into(),
            "opt.index_hidden" => "索引隐藏文件和文件夹".into(),
            "opt.index_system" => "索引系统文件和文件夹".into(),
            "opt.index_network" => "索引网络驱动器".into(),
            "opt.index_network_desc" => "勾选需要索引的网络驱动器:".into(),
            "opt.index_local_desc" => "本地固定驱动器:".into(),
            "opt.rebuild_now" => "立即重建索引".into(),
            "opt.no_network_drives" => "未检测到已映射的网络驱动器 (可在 Windows 资源管理器中映射)".into(),
            "opt.exclude" => "排除文件夹 (分号分隔)".into(),
            "opt.view" => "视图".into(),
            "opt.status_bar" => "显示状态栏".into(),
            "opt.header" => "显示列标题".into(),
            "opt.alternate_row" => "交替行背景颜色".into(),
            "opt.size_format" => "文件大小显示格式".into(),
            "opt.size_human" => "人类可读单位 (KB/MB/GB)".into(),
            "opt.size_bytes" => "精确字节数 (Bytes)".into(),
            "opt.date_format" => "日期显示格式".into(),
            "opt.search" => "搜索".into(),
            "opt.max_results" => "最大返回结果数".into(),
            "opt.as_you_type" => "即输即搜 (实时搜索)".into(),
            "opt.focus_search" => "定位到搜索栏".into(),
            "opt.shortcut_action" => "功能 / 操作".into(),
            "opt.shortcut_keys" => "快捷键组合".into(),
            "opt.reset_shortcuts" => "恢复全部默认快捷键".into(),
            "opt.press_keys" => "请按下新的快捷键组合... (按 Esc 取消)".into(),
            "opt.ok" => "确定".into(),
            "opt.cancel" => "取消".into(),
            "opt.apply" => "应用".into(),
            "opt.restart_hint" => "设置立即生效并自动保存".into(),
            "status.objects" => "个对象".into(),
            "status.selected" => "选中 / 共".into(),
            "status.indexing" => "正在索引...".into(),
            "status.db" => "数据库".into(),
            "status.filter" => "过滤器".into(),
            "dlg.bookmark_title" => "添加书签".into(),
            "dlg.bookmark_query" => "查询语句".into(),
            "dlg.bookmark_save" => "保存".into(),
            "dlg.bookmark_cancel" => "取消".into(),
            "dlg.about_body" => "Everything 超快文件搜索工具\n基于 Rust + egui 构建\n支持 NTFS USN Journal 原生日志扫描及并行实时搜索。".into(),
            "res.none" => "没有找到匹配的文件。".into(),
            "common.ready" => "就绪".into(),
            _ => key.to_string(),
        }
    }
}
