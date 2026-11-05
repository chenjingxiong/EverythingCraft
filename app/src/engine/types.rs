use serde::{Deserialize, Serialize};

pub const SEARCH_MAX_RESULTS: usize = 2000;
pub const SEARCH_BATCH_SIZE: usize = 500;
pub const USN_BUFFER_SIZE: usize = 65536;
pub const DEFAULT_WINDOW_WIDTH: f32 = 1022.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 713.0;
pub const FILTERS_DROPDOWN_WIDTH: f32 = 128.0;
pub const THUMBNAIL_SIZE: u32 = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultEntry {
    pub id: u64,
    pub name: String,
    pub path: String,       // Directory path without trailing slash
    pub full_path: String,  // Complete path including file name
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,      // Unix timestamp in seconds
    pub created: i64,       // Unix timestamp in seconds
    pub extension: String,  // Lowercase extension without dot
    pub attributes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortColumn {
    Name,
    Path,
    Size,
    DateModified,
    Extension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub match_case: bool,
    pub match_whole_word: bool,
    pub match_path: bool,
    pub match_regex: bool,
    pub folders_first: bool,
    pub sort_column: SortColumn,
    pub sort_direction: SortDirection,
    pub max_results: Option<usize>,
    pub filter_category: FilterCategory,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            match_case: false,
            match_whole_word: false,
            match_path: false,
            match_regex: false,
            folders_first: true,
            sort_column: SortColumn::Name,
            sort_direction: SortDirection::Ascending,
            max_results: Some(SEARCH_MAX_RESULTS),
            filter_category: FilterCategory::Everything,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilterCategory {
    Everything,
    Audio,
    Compressed,
    Document,
    Executable,
    Folder,
    Picture,
    Video,
}

impl FilterCategory {
    pub fn name(&self) -> &'static str {
        match self {
            FilterCategory::Everything => "Everything",
            FilterCategory::Audio => "Audio",
            FilterCategory::Compressed => "Compressed",
            FilterCategory::Document => "Document",
            FilterCategory::Executable => "Executable",
            FilterCategory::Folder => "Folder",
            FilterCategory::Picture => "Picture",
            FilterCategory::Video => "Video",
        }
    }

    pub fn all() -> &'static [FilterCategory] {
        &[
            FilterCategory::Everything,
            FilterCategory::Audio,
            FilterCategory::Compressed,
            FilterCategory::Document,
            FilterCategory::Executable,
            FilterCategory::Folder,
            FilterCategory::Picture,
            FilterCategory::Video,
        ]
    }

    pub fn matches_extension(&self, ext: &str, is_dir: bool) -> bool {
        match self {
            FilterCategory::Everything => true,
            FilterCategory::Folder => is_dir,
            FilterCategory::Audio => matches!(
                ext,
                "aac" | "ac3" | "aif" | "aifc" | "aiff" | "au" | "cda" | "dts" | "fla" | "flac"
                    | "it" | "m1a" | "m2a" | "m3u" | "m4a" | "mid" | "midi" | "mka" | "mod"
                    | "mp2" | "mp3" | "mpa" | "ogg" | "ra" | "rmi" | "spx" | "umx" | "voc"
                    | "wav" | "wma" | "xm"
            ),
            FilterCategory::Compressed => matches!(
                ext,
                "7z" | "ace" | "alz" | "apk" | "ar" | "arc" | "arj" | "bz2" | "cab" | "cpio"
                    | "deb" | "dmg" | "ear" | "egg" | "epub" | "fat" | "gz" | "gzip" | "hfs"
                    | "iso" | "jar" | "lha" | "lzh" | "lzma" | "pak" | "rar" | "rpm" | "s7z"
                    | "tar" | "tbz2" | "tgz" | "vhd" | "vmdk" | "war" | "wim" | "xar" | "xz"
                    | "z" | "zip"
            ),
            FilterCategory::Document => matches!(
                ext,
                "c" | "chm" | "cpp" | "csv" | "cxx" | "doc" | "docm" | "docx" | "dot" | "dotm"
                    | "dotx" | "h" | "hpp" | "htm" | "html" | "hxx" | "ini" | "jav" | "java"
                    | "js" | "json" | "log" | "m" | "mak" | "md" | "pdf" | "php" | "pl"
                    | "pot" | "potm" | "potx" | "ppam" | "pps" | "ppsm" | "ppsx" | "ppt"
                    | "pptm" | "pptx" | "ps1" | "py" | "rtf" | "sql" | "text" | "tsv" | "txt"
                    | "vbs" | "wks" | "wps" | "wri" | "xhtml" | "xlam" | "xls" | "xlsb"
                    | "xlsm" | "xlsx" | "xlt" | "xltm" | "xltx" | "xml" | "yaml" | "yml"
            ),
            FilterCategory::Executable => matches!(
                ext,
                "bat" | "cmd" | "exe" | "msi" | "msp" | "scr" | "com" | "vbs" | "ps1"
            ),
            FilterCategory::Picture => matches!(
                ext,
                "ani" | "bmp" | "gif" | "ico" | "jpe" | "jpeg" | "jpg" | "pcx" | "png" | "psd"
                    | "svg" | "tga" | "tif" | "tiff" | "webp" | "wmf"
            ),
            FilterCategory::Video => matches!(
                ext,
                "3g2" | "3gp" | "3gp2" | "3gpp" | "amv" | "asf" | "asx" | "avi" | "divx"
                    | "f4v" | "flv" | "m2ts" | "m4v" | "mkv" | "mov" | "mp4" | "mpeg" | "mpg"
                    | "mpe" | "ogm" | "ogv" | "qt" | "rm" | "rmvb" | "swf" | "ts" | "vob"
                    | "webm" | "wmv"
            ),
        }
    }
}
