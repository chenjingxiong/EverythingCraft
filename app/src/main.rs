#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use eframe::egui::{self, vec2, Color32, Stroke, Style, Visuals};

mod controller;
mod engine;
mod model;
mod settings;
mod view;

use controller::AppController;
use model::AppModel;
use settings::{AppSettings, Theme};
use view::AppView;

static MSYH_FONT: &[u8] = include_bytes!("../assets/fonts/msyh.ttc");
static SEGOE_EMOJI_FONT: &[u8] = include_bytes!("../assets/fonts/seguiemj.ttf");

struct EverythingApp {
    model: AppModel,
    controller: AppController,
}

impl EverythingApp {
    fn new(cc: &eframe::CreationContext<'_>, screenshot_path: Option<PathBuf>) -> Self {
        let settings = AppSettings::load();
        apply_theme_and_scale(&cc.egui_ctx, &settings);

        let mut model = AppModel::new(screenshot_path, settings);
        let mut controller = AppController::new();

        model.load_seed_samples();
        controller.start_background_indexing(&mut model);

        Self { model, controller }
    }
}

/// Install CJK + Emoji fonts (fixes mojibake/boxes for Chinese text and icons),
/// apply theme and zoom scale. Everything 1.5a default: 100%.
pub fn apply_theme_and_scale(ctx: &egui::Context, settings: &AppSettings) {
    let mut fonts = egui::FontDefinitions::default();

    // Chinese font (msyh.ttc face 0), fallback emoji font
    fonts
        .font_data
        .insert("msyh".into(), egui::FontData::from_static(MSYH_FONT).tweak(egui::FontTweak {
            scale: 1.0,
            y_offset_factor: 0.0,
            y_offset: 1.0,
            baseline_offset_factor: 0.0,
        }));
    fonts
        .font_data
        .insert("emoji".into(), egui::FontData::from_static(SEGOE_EMOJI_FONT).tweak(egui::FontTweak {
            scale: 1.0,
            y_offset_factor: 0.0,
            y_offset: 0.0,
            baseline_offset_factor: 0.0,
        }));

    // Put CJK font first so Chinese glyphs, punctuation and symbols are crisp and never tofu
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        let list = fonts.families.entry(family).or_default();
        list.insert(0, "msyh".to_owned());
        list.push("emoji".to_owned());
    }

    ctx.set_fonts(fonts);

    let mut visuals = match settings.theme {
        Theme::Dark => {
            let mut v = Visuals::dark();
            v.window_fill = Color32::from_rgb(0, 0, 0); // pure black result table
            v.panel_fill = Color32::from_rgb(29, 29, 29);
            v.widgets.noninteractive.bg_fill = Color32::from_rgb(32, 32, 32);
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(50, 50, 50));
            v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(220, 220, 220));
            v.widgets.inactive.bg_fill = Color32::from_rgb(36, 36, 36);
            v.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(220, 220, 220));
            v.widgets.hovered.bg_fill = Color32::from_rgb(55, 55, 55);
            v.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
            v.widgets.active.bg_fill = Color32::from_rgb(71, 89, 138);
            v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
            v.selection.bg_fill = Color32::from_rgb(71, 89, 138);
            v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
            v
        }
        Theme::Light => Visuals::light(),
    };
    visuals.panel_fill = match settings.theme {
        Theme::Dark => Color32::from_rgb(29, 29, 29),
        Theme::Light => Color32::from_rgb(240, 240, 240),
    };

    let mut style = Style::default();
    style.visuals = visuals;
    ctx.set_style(style);

    let zoom = settings.zoom_percent.max(50.0).min(500.0) / 100.0;
    ctx.set_zoom_factor(zoom);
}

impl eframe::App for EverythingApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.controller.update(&mut self.model, ctx);
        AppView::render(&mut self.model, &mut self.controller, ctx);
        if self.model.settings_dirty {
            apply_theme_and_scale(ctx, &self.model.settings);
            self.model.settings_dirty = false;
        }
        if self.model.needs_resave {
            let _ = self.model.settings.save();
            self.model.needs_resave = false;
        }
        if let Some(rect) = ctx.input(|i| i.viewport().outer_rect) {
            if !ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
                self.model.last_window_rect = Some(rect);
            }
        }
        // Persist window size when the user closes the window
        if ctx.input(|i| i.viewport().close_requested()) {
            if let Some(rect) = self.model.last_window_rect {
                self.model.settings.window_width = rect.width();
                self.model.settings.window_height = rect.height();
                let _ = self.model.settings.save();
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut screenshot_path: Option<PathBuf> = None;
    let mut initial_query = String::new();

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--screenshot" && i + 1 < args.len() {
            screenshot_path = Some(PathBuf::from(&args[i + 1]));
            i += 2;
        } else if (args[i] == "-s" || args[i] == "-search") && i + 1 < args.len() {
            initial_query = args[i + 1].clone();
            i += 2;
        } else if !args[i].starts_with('-') && initial_query.is_empty() {
            initial_query = args[i].clone();
            i += 1;
        } else {
            i += 1;
        }
    }

    let settings = AppSettings::load();
    let width = if settings.window_width > 400.0 { settings.window_width } else { 1022.0 };
    let height = if settings.window_height > 300.0 { settings.window_height } else { 713.0 };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(vec2(width, height))
            .with_min_inner_size(vec2(500.0, 350.0))
            .with_title("Everything")
            .with_active(true)
            .with_visible(true),
        ..Default::default()
    };

    eframe::run_native(
        "Everything",
        native_options,
        Box::new(move |cc| {
            let mut app = EverythingApp::new(cc, screenshot_path);
            if !initial_query.is_empty() {
                app.model.search_query = initial_query;
                app.model.execute_search();
            }
            Ok(Box::new(app))
        }),
    )
}

// Re-export so view can reference Language without extra imports elsewhere
pub use settings::I18n;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_loading() {
        let ctx = egui::Context::default();
        let settings = AppSettings::default();
        apply_theme_and_scale(&ctx, &settings);
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("测试中文字体");
            });
        });
    }

    #[test]
    fn test_engine_search() {
        let engine = engine::EverythingEngine::new();
        let entries = vec![
            engine::SearchResultEntry {
                id: 1,
                name: "test_file.txt".to_string(),
                path: r"C:\Folder".to_string(),
                full_path: r"C:\Folder\test_file.txt".to_string(),
                is_dir: false,
                size: 1024,
                modified: 1000,
                created: 1000,
                extension: "txt".to_string(),
                attributes: 0x20,
            },
            engine::SearchResultEntry {
                id: 2,
                name: "测试文件.doc".to_string(),
                path: r"D:\Documents".to_string(),
                full_path: r"D:\Documents\测试文件.doc".to_string(),
                is_dir: false,
                size: 2048,
                modified: 2000,
                created: 2000,
                extension: "doc".to_string(),
                attributes: 0x20,
            },
        ];
        engine.load_entries(entries);
        let opts = engine::SearchOptions::default();
        let (results, _) = engine.search("测试", &opts);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "测试文件.doc");

        let (results, _) = engine.search("ext:txt", &opts);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test_file.txt");
    }
}
