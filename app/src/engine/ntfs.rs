use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
#[cfg(windows)]
use windows_sys::Win32::System::Ioctl::{
    MFT_ENUM_DATA_V0, USN_JOURNAL_DATA_V0,
};
#[cfg(windows)]
use windows_sys::Win32::System::IO::DeviceIoControl;

pub const FSCTL_QUERY_USN_JOURNAL: u32 = 589940; // 0x000900f4
pub const FSCTL_READ_USN_JOURNAL: u32 = 589944;  // 0x000900f8
pub const FSCTL_ENUM_USN_DATA: u32 = 589875;     // 0x000900b3
pub const USN_JOURNAL_DATA_V0_SIZE: usize = 56;  // 0x38 bytes

#[cfg(windows)]
extern "system" {
    fn CreateFileW(
        lpfilename: *const u16,
        dwdesiredaccess: u32,
        dwsharemode: u32,
        lpsecurityattributes: *mut core::ffi::c_void,
        dwcreationdisposition: u32,
        dwflagsandattributes: u32,
        htemplatefile: HANDLE,
    ) -> HANDLE;
}

use super::types::SearchResultEntry;

pub struct RawNtfsRecord {
    pub frn: u64,
    pub parent_frn: u64,
    pub name: String,
    pub is_dir: bool,
    pub attributes: u32,
    pub timestamp: i64,
}

pub fn scan_volume_usn(drive_letter: char) -> Result<Vec<SearchResultEntry>, String> {
    #[cfg(windows)]
    {
        let volume_path = format!(r"\\.\{}:", drive_letter);
        let wide_path: Vec<u16> = OsStr::new(&volume_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let handle: HANDLE = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                0x80000000, // GENERIC_READ
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null_mut(),
                OPEN_EXISTING,
                0,
                0 as HANDLE,
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            let err = unsafe { GetLastError() };
            return Err(format!("Failed to open volume handle {}: error {}", volume_path, err));
        }

        struct HandleGuard(HANDLE);
        impl Drop for HandleGuard {
            fn drop(&mut self) {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
        let _guard = HandleGuard(handle);

        let mut journal_data: USN_JOURNAL_DATA_V0 = unsafe { std::mem::zeroed() };
        let mut bytes_returned: u32 = 0;

        let query_success = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_QUERY_USN_JOURNAL,
                null_mut(),
                0,
                &mut journal_data as *mut _ as *mut _,
                USN_JOURNAL_DATA_V0_SIZE as u32,
                &mut bytes_returned,
                null_mut(),
            )
        };

        if query_success == 0 {
            let err = unsafe { GetLastError() };
            return Err(format!("FSCTL_QUERY_USN_JOURNAL failed on {}: error {}", volume_path, err));
        }

        let mut enum_data = MFT_ENUM_DATA_V0 {
            StartFileReferenceNumber: 0,
            LowUsn: 0,
            HighUsn: journal_data.NextUsn,
        };

        let mut buffer = vec![0u8; 65536];
        let mut raw_records: Vec<RawNtfsRecord> = Vec::with_capacity(100_000);

        loop {
            let enum_success = unsafe {
                DeviceIoControl(
                    handle,
                    FSCTL_ENUM_USN_DATA,
                    &mut enum_data as *mut _ as *mut _,
                    std::mem::size_of::<MFT_ENUM_DATA_V0>() as u32,
                    buffer.as_mut_ptr() as *mut _,
                    buffer.len() as u32,
                    &mut bytes_returned,
                    null_mut(),
                )
            };

            if enum_success == 0 {
                let err = unsafe { GetLastError() };
                if err == 38 {
                    // ERROR_HANDLE_EOF
                    break;
                }
                break;
            }

            if bytes_returned < 8 {
                break;
            }

            let next_usn = unsafe { *(buffer.as_ptr() as *const u64) };
            enum_data.StartFileReferenceNumber = next_usn;

            let mut offset = 8;
            while offset + 8 <= bytes_returned as usize {
                let record_len = unsafe { *(buffer.as_ptr().add(offset) as *const u32) } as usize;
                if record_len == 0 || offset + record_len > bytes_returned as usize {
                    break;
                }

                let major_version = unsafe { *(buffer.as_ptr().add(offset + 4) as *const u16) };

                let (frn, parent_frn, file_attributes, file_name_length, file_name_offset, timestamp) =
                    if major_version == 2 {
                        if record_len < 60 {
                            offset += record_len;
                            continue;
                        }
                        let frn = unsafe { *(buffer.as_ptr().add(offset + 8) as *const u64) };
                        let parent_frn = unsafe { *(buffer.as_ptr().add(offset + 16) as *const u64) };
                        let ts = unsafe { *(buffer.as_ptr().add(offset + 32) as *const u64) };
                        let attrs = unsafe { *(buffer.as_ptr().add(offset + 52) as *const u32) };
                        let fn_len = unsafe { *(buffer.as_ptr().add(offset + 56) as *const u16) } as usize;
                        let fn_off = unsafe { *(buffer.as_ptr().add(offset + 58) as *const u16) } as usize;
                        (frn, parent_frn, attrs, fn_len, fn_off, ts)
                    } else if major_version == 3 {
                        if record_len < 76 {
                            offset += record_len;
                            continue;
                        }
                        let frn = unsafe { *(buffer.as_ptr().add(offset + 8) as *const u64) };
                        let parent_frn = unsafe { *(buffer.as_ptr().add(offset + 24) as *const u64) };
                        let ts = unsafe { *(buffer.as_ptr().add(offset + 48) as *const u64) };
                        let attrs = unsafe { *(buffer.as_ptr().add(offset + 68) as *const u32) };
                        let fn_len = unsafe { *(buffer.as_ptr().add(offset + 72) as *const u16) } as usize;
                        let fn_off = unsafe { *(buffer.as_ptr().add(offset + 74) as *const u16) } as usize;
                        (frn, parent_frn, attrs, fn_len, fn_off, ts)
                    } else {
                        offset += record_len;
                        continue;
                    };

                let name_start = offset + file_name_offset;
                if name_start + file_name_length <= bytes_returned as usize && file_name_length > 0 {
                    let name_slice_u16 = unsafe {
                        std::slice::from_raw_parts(
                            buffer.as_ptr().add(name_start) as *const u16,
                            file_name_length / 2,
                        )
                    };
                    let file_name = String::from_utf16_lossy(name_slice_u16);

                    // Skip internal NTFS metadata files ($MFT, $LogFile, etc.)
                    if !file_name.starts_with('$') {
                        let is_dir = (file_attributes & 0x10) != 0;
                        let unix_ts = if timestamp > 116444736000000000 {
                            ((timestamp - 116444736000000000) / 10_000_000) as i64
                        } else {
                            0
                        };

                        raw_records.push(RawNtfsRecord {
                            frn,
                            parent_frn,
                            name: file_name,
                            is_dir,
                            attributes: file_attributes,
                            timestamp: unix_ts,
                        });
                    }
                }

                offset += record_len;
            }
        }

        const MFT_INDEX_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;
        const ROOT_MFT_INDEX: u64 = 5;

        let mut path_cache: HashMap<u64, String> = HashMap::with_capacity(raw_records.len() / 4);
        let root_drive = format!("{}:\\", drive_letter);

        let mut dir_map: HashMap<u64, (u64, String)> = HashMap::with_capacity(raw_records.len() / 5);
        for rec in &raw_records {
            if rec.is_dir {
                dir_map.insert(
                    rec.frn & MFT_INDEX_MASK,
                    (rec.parent_frn & MFT_INDEX_MASK, rec.name.clone()),
                );
            }
        }

        fn resolve_dir_path(
            frn_masked: u64,
            dir_map: &HashMap<u64, (u64, String)>,
            path_cache: &mut HashMap<u64, String>,
            root_drive: &str,
            depth: usize,
        ) -> String {
            if frn_masked == ROOT_MFT_INDEX || frn_masked == 0 || depth > 64 {
                return root_drive.to_string();
            }
            if let Some(cached) = path_cache.get(&frn_masked) {
                return cached.clone();
            }
            if let Some((parent_masked, name)) = dir_map.get(&frn_masked) {
                let parent_path = if *parent_masked == ROOT_MFT_INDEX || *parent_masked == 0 || *parent_masked == frn_masked {
                    root_drive.to_string()
                } else {
                    resolve_dir_path(*parent_masked, dir_map, path_cache, root_drive, depth + 1)
                };
                let clean_parent = parent_path.trim_end_matches('\\');
                let full = format!("{}\\{}", clean_parent, name);
                path_cache.insert(frn_masked, full.clone());
                return full;
            }
            root_drive.to_string()
        }

        let mut results = Vec::with_capacity(raw_records.len());
        let mut id_counter: u64 = 1;

        for rec in raw_records {
            let parent_path = resolve_dir_path(
                rec.parent_frn & MFT_INDEX_MASK,
                &dir_map,
                &mut path_cache,
                &root_drive,
                0,
            );
            let clean_parent = parent_path.trim_end_matches('\\').to_string();
            let full_path = if clean_parent.is_empty() {
                format!("{}:\\{}", drive_letter, rec.name)
            } else if clean_parent.ends_with(':') {
                format!("{}\\{}", clean_parent, rec.name)
            } else {
                format!("{}\\{}", clean_parent, rec.name)
            };

            let ext = if rec.is_dir {
                String::new()
            } else {
                std::path::Path::new(&rec.name)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase()
            };

            results.push(SearchResultEntry {
                id: id_counter,
                name: rec.name,
                path: clean_parent,
                full_path,
                is_dir: rec.is_dir,
                size: 0,
                modified: rec.timestamp,
                created: rec.timestamp,
                extension: ext,
                attributes: rec.attributes,
            });
            id_counter += 1;
        }

        Ok(results)
    }

    #[cfg(not(windows))]
    {
        Err("NTFS USN journal scanning is only available on Windows".to_string())
    }
}

pub fn read_usn_journal_record_demo() -> u32 {
    FSCTL_READ_USN_JOURNAL
}
