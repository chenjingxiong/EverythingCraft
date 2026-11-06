use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use crate::engine::{export_to_csv, export_to_efu, export_to_json, export_to_tsv, export_to_txt, SearchResultEntry};
use crate::model::AppModel;

pub enum ControllerEvent {
    IndexingFinished(usize),
    IndexingFailed(String),
}

pub struct AppController {
    event_tx: Sender<ControllerEvent>,
    event_rx: Receiver<ControllerEvent>,
    screenshot_requested: bool,
}

impl Default for AppController {
    fn default() -> Self {
        Self::new()
    }
}

impl AppController {
    pub fn new() -> Self {
        let (event_tx, event_rx) = channel();
        Self {
            event_tx,
            event_rx,
            screenshot_requested: false,
        }
    }

    pub fn update(&mut self, model: &mut AppModel, ctx: &egui::Context) {
        // Process background events
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                ControllerEvent::IndexingFinished(count) => {
                    model.is_indexing = false;
                    model.status_message = format!("Indexed {} objects", count);
                    model.execute_search();
                }
                ControllerEvent::IndexingFailed(err) => {
                    model.is_indexing = false;
                    model.status_message = format!("Indexing warning: {}", err);
                    model.execute_search();
                }
            }
        }

        if model.is_indexing {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        }
        if model.tick_search_debounce() {
            ctx.request_repaint();
        } else if model.search_pending.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }

        // Handle screenshot mode if active
        if let Some(ref target_path) = model.screenshot_path {
            model.frame_counter += 1;
            // Redraw continuously until target frame
            ctx.request_repaint();

            if model.frame_counter >= model.target_frames && !self.screenshot_requested {
                self.screenshot_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }

            // Check if screenshot event arrived
            ctx.input(|i| {
                for event in &i.raw.events {
                    if let egui::Event::Screenshot { image, .. } = event {
                        let path = target_path.clone();
                        let img = image.clone();
                        Self::save_screenshot_and_exit(&path, &img);
                    }
                }
            });
        }
    }

    fn save_screenshot_and_exit(path: &Path, image: &Arc<egui::ColorImage>) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let width = image.width() as u32;
        let height = image.height() as u32;

        let mut raw_pixels = Vec::with_capacity((width * height * 4) as usize);
        for pixel in &image.pixels {
            raw_pixels.push(pixel.r());
            raw_pixels.push(pixel.g());
            raw_pixels.push(pixel.b());
            raw_pixels.push(pixel.a());
        }

        if let Some(img_buffer) = image::RgbaImage::from_raw(width, height, raw_pixels) {
            if let Err(e) = img_buffer.save(path) {
                eprintln!("Failed to save screenshot: {}", e);
            } else {
                println!("Screenshot successfully saved to {:?}", path);
            }
        }

        std::process::exit(0);
    }

    pub fn start_background_indexing(&mut self, model: &mut AppModel) {
        if model.is_indexing {
            return;
        }
        model.is_indexing = true;
        let engine = model.engine.clone();
        let tx = self.event_tx.clone();
        let local_drives = model.settings.selected_local_drives.clone();
        let index_network = model.settings.index_network_drives;
        let network_drives = model.settings.selected_network_drives.clone();
        let use_usn = model.settings.index_ntfs;

        std::thread::spawn(move || {
            match engine.index_drives(&local_drives, index_network, &network_drives, use_usn) {
                Ok(count) => {
                    let _ = tx.send(ControllerEvent::IndexingFinished(count));
                }
                Err(e) => {
                    let _ = tx.send(ControllerEvent::IndexingFailed(e));
                }
            }
        });
    }

    pub fn open_entry(&self, entry: &SearchResultEntry) {
        #[cfg(windows)]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            extern "system" {
                fn ShellExecuteW(
                    hwnd: isize,
                    lpoperation: *const u16,
                    lpfile: *const u16,
                    lpparameters: *const u16,
                    lpdirectory: *const u16,
                    nshowcmd: i32,
                ) -> isize;
            }
            let op: Vec<u16> = OsStr::new("open").encode_wide().chain(std::iter::once(0)).collect();
            let file: Vec<u16> = OsStr::new(&entry.full_path).encode_wide().chain(std::iter::once(0)).collect();
            unsafe {
                ShellExecuteW(0, op.as_ptr(), file.as_ptr(), std::ptr::null(), std::ptr::null(), 1);
            }
        }
        #[cfg(not(windows))]
        {
            let _ = Command::new("xdg-open").arg(&entry.full_path).spawn();
        }
    }

    pub fn open_containing_folder(&self, full_path: &str) {
        #[cfg(windows)]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            extern "system" {
                fn ShellExecuteW(
                    hwnd: isize,
                    lpoperation: *const u16,
                    lpfile: *const u16,
                    lpparameters: *const u16,
                    lpdirectory: *const u16,
                    nshowcmd: i32,
                ) -> isize;
            }
            let op: Vec<u16> = OsStr::new("open").encode_wide().chain(std::iter::once(0)).collect();
            let explorer: Vec<u16> = OsStr::new("explorer.exe").encode_wide().chain(std::iter::once(0)).collect();
            let param_str = format!("/select,\"{}\"", full_path);
            let params: Vec<u16> = OsStr::new(&param_str).encode_wide().chain(std::iter::once(0)).collect();
            unsafe {
                ShellExecuteW(0, op.as_ptr(), explorer.as_ptr(), params.as_ptr(), std::ptr::null(), 1);
            }
        }
        #[cfg(not(windows))]
        {
            if let Some(parent) = Path::new(full_path).parent() {
                let _ = Command::new("xdg-open").arg(parent).spawn();
            }
        }
    }

    pub fn copy_to_clipboard(&self, text: &str) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.to_string());
        }
    }

    pub fn export_results(&self, model: &mut AppModel, format: &str) {
        let export_dir = dirs_or_fallback();
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("Everything_Export_{}.{}", timestamp, format);
        let path = export_dir.join(&filename);

        let res = match format {
            "csv" => export_to_csv(&model.search_results, &path),
            "tsv" => export_to_tsv(&model.search_results, &path),
            "efu" => export_to_efu(&model.search_results, &path),
            "txt" => export_to_txt(&model.search_results, &path),
            "json" => export_to_json(&model.search_results, &path),
            _ => return,
        };

        match res {
            Ok(_) => {
                model.export_status = Some(format!("Exported to {}", filename));
            }
            Err(e) => {
                model.export_status = Some(format!("Export failed: {}", e));
            }
        }
    }

    #[cfg(windows)]
    pub fn is_elevated() -> bool {
        #[link(name = "shell32")]
        extern "system" {
            fn IsUserAnAdmin() -> i32;
        }
        unsafe { IsUserAnAdmin() != 0 }
    }

    #[cfg(not(windows))]
    pub fn is_elevated() -> bool {
        false
    }

    pub fn restart_as_admin() {
        #[cfg(windows)]
        {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            #[link(name = "shell32")]
            extern "system" {
                fn ShellExecuteW(
                    hwnd: isize,
                    lpoperation: *const u16,
                    lpfile: *const u16,
                    lpparameters: *const u16,
                    lpdirectory: *const u16,
                    nshowcmd: i32,
                ) -> isize;
            }
            if let Ok(exe) = std::env::current_exe() {
                let exe_wide: Vec<u16> = exe.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
                let op_wide: Vec<u16> = OsStr::new("runas").encode_wide().chain(std::iter::once(0)).collect();
                unsafe {
                    ShellExecuteW(0, op_wide.as_ptr(), exe_wide.as_ptr(), std::ptr::null(), std::ptr::null(), 1);
                }
                std::process::exit(0);
            }
        }
    }
}

fn dirs_or_fallback() -> PathBuf {
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let desktop = PathBuf::from(userprofile).join("Desktop");
        if desktop.exists() {
            return desktop;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
