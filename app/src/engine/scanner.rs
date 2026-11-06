use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use super::types::SearchResultEntry;

#[derive(Debug, Clone)]
pub struct NetworkDriveInfo {
    pub letter: char,
    pub path: String,       // e.g. "Y:\"
    pub remote_unc: String, // e.g. "\\192.168.9.6\nvme11-..."
}

pub fn get_remote_drive_target(drive_letter: char) -> Option<String> {
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        let local_name = format!("{}:", drive_letter);
        let wide_local: Vec<u16> = OsStr::new(&local_name).encode_wide().chain(std::iter::once(0)).collect();
        let mut buffer = [0u16; 512];
        let mut buf_len = buffer.len() as u32;

        extern "system" {
            fn LoadLibraryA(lpLibFileName: *const u8) -> *mut core::ffi::c_void;
            fn GetProcAddress(
                hModule: *mut core::ffi::c_void,
                lpProcName: *const u8,
            ) -> *mut core::ffi::c_void;
        }

        unsafe {
            let mpr = LoadLibraryA(b"mpr.dll\0".as_ptr());
            if !mpr.is_null() {
                let proc = GetProcAddress(mpr, b"WNetGetConnectionW\0".as_ptr());
                if !proc.is_null() {
                    type WNetFn = unsafe extern "system" fn(*const u16, *mut u16, *mut u32) -> u32;
                    let f: WNetFn = std::mem::transmute(proc);
                    if f(wide_local.as_ptr(), buffer.as_mut_ptr(), &mut buf_len) == 0 {
                        let len = buffer.iter().position(|&c| c == 0).unwrap_or(0);
                        if len > 0 {
                            return Some(String::from_utf16_lossy(&buffer[..len]));
                        }
                    }
                }
            }
        }
    }
    let _ = drive_letter;
    None
}

pub fn list_network_drives() -> Vec<NetworkDriveInfo> {
    let mut network_drives = Vec::new();
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        extern "system" {
            fn GetLogicalDrives() -> u32;
            fn GetDriveTypeW(lprootpathname: *const u16) -> u32;
        }
        let mask = unsafe { GetLogicalDrives() };
        for i in 0..26 {
            if (mask & (1 << i)) != 0 {
                let letter = (b'A' + i) as char;
                let path_str = format!("{}:\\", letter);
                let wide: Vec<u16> = OsStr::new(&path_str).encode_wide().chain(std::iter::once(0)).collect();
                let dtype = unsafe { GetDriveTypeW(wide.as_ptr()) };
                // 4 = DRIVE_REMOTE
                if dtype == 4 {
                    let remote_unc = get_remote_drive_target(letter).unwrap_or_default();
                    network_drives.push(NetworkDriveInfo {
                        letter,
                        path: path_str,
                        remote_unc,
                    });
                }
            }
        }
    }
    network_drives
}

pub fn list_windows_drives() -> Vec<PathBuf> {
    let mut drives = Vec::new();
    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        extern "system" {
            fn GetLogicalDrives() -> u32;
            fn GetDriveTypeW(lprootpathname: *const u16) -> u32;
            fn GetVolumeInformationW(
                lprootpathname: *const u16,
                lpvolumenamebuffer: *mut u16,
                nvolumenamesize: u32,
                lpvolumeserialnumber: *mut u32,
                lpmaximumcomponentlength: *mut u32,
                lpfilesystemflags: *mut u32,
                lpfilesystemnamebuffer: *mut u16,
                nfilesystemnamesize: u32,
            ) -> i32;
        }
        let mask = unsafe { GetLogicalDrives() };
        for i in 0..26 {
            if (mask & (1 << i)) != 0 {
                let letter = (b'A' + i) as char;
                let path_str = format!("{}:\\", letter);
                let wide: Vec<u16> = OsStr::new(&path_str).encode_wide().chain(std::iter::once(0)).collect();
                let dtype = unsafe { GetDriveTypeW(wide.as_ptr()) };
                // 3 = DRIVE_FIXED, 2 = DRIVE_REMOVABLE
                if dtype == 3 || dtype == 2 {
                    let mut flags = 0u32;
                    let ok = unsafe {
                        GetVolumeInformationW(
                            wide.as_ptr(),
                            std::ptr::null_mut(),
                            0,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            &mut flags,
                            std::ptr::null_mut(),
                            0,
                        )
                    };
                    if ok != 0 {
                        drives.push(PathBuf::from(path_str));
                    }
                }
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

#[cfg(windows)]
pub fn scan_single_root_batched<F>(
    root: &Path,
    cancel_flag: Arc<AtomicBool>,
    id_counter: Arc<AtomicU64>,
    batch_size: usize,
    mut on_batch: F,
) where
    F: FnMut(Vec<SearchResultEntry>),
{
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Storage::FileSystem::{
        FindClose, FindFirstFileW, FindNextFileW, FILE_ATTRIBUTE_DIRECTORY, WIN32_FIND_DATAW,
    };

    let mut stack = vec![root.to_path_buf()];
    let mut batch = Vec::with_capacity(batch_size.min(10_000));

    let mut find_data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };

    let ft_to_unix = |high: u32, low: u32| -> i64 {
        let intervals = ((high as u64) << 32) | (low as u64);
        if intervals > 116444736000000000 {
            ((intervals - 116444736000000000) / 10_000_000) as i64
        } else {
            0
        }
    };

    while let Some(current_dir) = stack.pop() {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        let pattern = current_dir.join("*");
        let wide_pattern: Vec<u16> = pattern.as_os_str().encode_wide().chain(std::iter::once(0)).collect();

        let handle = unsafe { FindFirstFileW(wide_pattern.as_ptr(), &mut find_data) };
        if handle == INVALID_HANDLE_VALUE {
            continue;
        }

        let parent_str = current_dir.to_string_lossy().to_string();

        loop {
            let len = find_data.cFileName.iter().position(|&c| c == 0).unwrap_or(0);
            if len > 0 {
                let name = String::from_utf16_lossy(&find_data.cFileName[..len]);
                if name != "." && name != ".." {
                    let is_dir = (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;

                    if is_dir {
                        if !is_ignored_directory_name(&name) {
                            stack.push(current_dir.join(&name));
                        }
                    }

                    let full_path = if parent_str.ends_with('\\') {
                        format!("{}{}", parent_str, name)
                    } else {
                        format!("{}\\{}", parent_str, name)
                    };

                    let ext = if is_dir {
                        String::new()
                    } else {
                        std::path::Path::new(&name)
                            .extension()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_lowercase()
                    };

                    let size = if is_dir {
                        0
                    } else {
                        ((find_data.nFileSizeHigh as u64) << 32) | (find_data.nFileSizeLow as u64)
                    };

                    let modified = ft_to_unix(
                        find_data.ftLastWriteTime.dwHighDateTime,
                        find_data.ftLastWriteTime.dwLowDateTime,
                    );
                    let created = ft_to_unix(
                        find_data.ftCreationTime.dwHighDateTime,
                        find_data.ftCreationTime.dwLowDateTime,
                    );

                    let id = id_counter.fetch_add(1, Ordering::Relaxed);

                    batch.push(SearchResultEntry {
                        id,
                        name,
                        path: parent_str.clone(),
                        full_path,
                        is_dir,
                        size,
                        modified,
                        created,
                        extension: ext,
                        attributes: find_data.dwFileAttributes,
                    });

                    if batch.len() >= batch_size {
                        on_batch(std::mem::take(&mut batch));
                    }
                }
            }

            if unsafe { FindNextFileW(handle, &mut find_data) } == 0 {
                break;
            }
        }

        unsafe {
            FindClose(handle);
        }
    }

    if !batch.is_empty() {
        on_batch(batch);
    }
}

#[cfg(not(windows))]
pub fn scan_single_root_batched<F>(
    root: &Path,
    cancel_flag: Arc<AtomicBool>,
    id_counter: Arc<AtomicU64>,
    batch_size: usize,
    mut on_batch: F,
) where
    F: FnMut(Vec<SearchResultEntry>),
{
    use std::time::SystemTime;
    use walkdir::WalkDir;

    let mut batch = Vec::with_capacity(batch_size.min(10_000));
    let mut total_scanned = 0;
    const MAX_FALLBACK_FILES_PER_DRIVE: usize = 100_000;

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

