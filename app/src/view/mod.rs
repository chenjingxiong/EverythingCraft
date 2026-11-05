use chrono::{Local, TimeZone};
use egui::{
    vec2, Align, Align2, Color32, Context, Key, Layout, Modifiers, Rect, RichText, ScrollArea,
    Sense, Ui,
};

use crate::controller::AppController;
use crate::engine::{FilterCategory, SearchResultEntry, SortColumn, SortDirection, FILTERS_DROPDOWN_WIDTH};
use crate::model::AppModel;
use crate::settings::{Language, ShortcutId, SizeFormat, Theme};

pub struct AppView;

/// Parse a human shortcut string like "Ctrl+Shift+C" / "Enter" / "Alt+F4" into
/// (modifiers, key). Used to match against egui input.
fn parse_shortcut(s: &str) -> Option<(Modifiers, Key)> {
    let mut mods = Modifiers::NONE;
    let mut key: Option<Key> = None;

    for part in s.split('+') {
        let part = part.trim();
        match part.to_lowercase().as_str() {
            "ctrl" => mods.ctrl = true,
            "shift" => mods.shift = true,
            "alt" => mods.alt = true,
            "cmd" | "win" => mods.command = true,
            _ => {
                let k = match part.to_lowercase().as_str() {
                    "enter" | "return" => Key::Enter,
                    "esc" | "escape" => Key::Escape,
                    "tab" => Key::Tab,
                    "space" => Key::Space,
                    "backspace" => Key::Backspace,
                    "f1" => Key::F1,
                    "f2" => Key::F2,
                    "f3" => Key::F3,
                    "f4" => Key::F4,
                    "f5" => Key::F5,
                    "f6" => Key::F6,
                    "f7" => Key::F7,
                    "f8" => Key::F8,
                    "f9" => Key::F9,
                    "f10" => Key::F10,
                    "f11" => Key::F11,
                    "f12" => Key::F12,
                    "arrowup" | "up" => Key::ArrowUp,
                    "arrowdown" | "down" => Key::ArrowDown,
                    "arrowleft" | "left" => Key::ArrowLeft,
                    "arrowright" | "right" => Key::ArrowRight,
                    "delete" | "del" => Key::Delete,
                    "insert" => Key::Insert,
                    "home" => Key::Home,
                    "end" => Key::End,
                    "pageup" => Key::PageUp,
                    "pagedown" => Key::PageDown,
                    single if single.len() == 1 => {
                        let c = single.chars().next().unwrap();
                        if c.is_ascii_alphabetic() {
                            Key::from_name(&c.to_uppercase().to_string())?
                        } else if c.is_ascii_digit() {
                            Key::from_name(&c.to_string())?
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                };
                key = Some(k);
            }
        }
    }
    key.map(|k| (mods, k))
}

impl AppView {
    pub fn render(model: &mut AppModel, controller: &mut AppController, ctx: &Context) {
        Self::handle_shortcuts(model, controller, ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            Self::render_menu_bar(model, controller, ui);
            ui.add_space(2.0);
            Self::render_search_bar(model, controller, ui);
            ui.add_space(3.0);
        });

        if model.settings.show_status_bar {
            egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
                Self::render_status_bar(model, ui);
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            Self::render_results_table(model, controller, ui);
        });

        Self::render_dialogs(model, controller, ctx);
    }

    fn action_shortcut(model: &AppModel, id: ShortcutId, ctx: &Context, check: impl Fn(Modifiers, Key, &egui::InputState) -> bool) -> bool {
        let combo = model.settings.shortcut(id);
        if let Some((m, k)) = parse_shortcut(&combo) {
            if ctx.input(|i| check(m, k, i)) {
                return true;
            }
        }
        false
    }

    fn handle_shortcuts(model: &mut AppModel, controller: &mut AppController, ctx: &Context) {
        // Escape: clear search (always)
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            model.search_query.clear();
            model.force_execute_search();
        }

        let check = |m: Modifiers, k: Key, i: &egui::InputState| i.modifiers.matches_exact(m) && i.key_pressed(k);

        if Self::action_shortcut(model, ShortcutId::FocusSearch, ctx, check) {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("main_search_box")));
        }
        if Self::action_shortcut(model, ShortcutId::RefreshIndex, ctx, check) {
            controller.start_background_indexing(model);
        }
        if Self::action_shortcut(model, ShortcutId::Options, ctx, check) {
            model.show_options_dialog = true;
        }
        if Self::action_shortcut(model, ShortcutId::CopyName, ctx, check) {
            if let Some(entry) = model.selected_entry() {
                controller.copy_to_clipboard(&entry.name);
            }
        }
        if Self::action_shortcut(model, ShortcutId::CopyPath, ctx, check) {
            if let Some(entry) = model.selected_entry() {
                controller.copy_to_clipboard(&entry.full_path);
            }
        }
        if Self::action_shortcut(model, ShortcutId::OpenPath, ctx, check) {
            if let Some(entry) = model.selected_entry() {
                controller.open_containing_folder(&entry.full_path);
            }
        }
        if Self::action_shortcut(model, ShortcutId::ToggleMatchCase, ctx, check) {
            model.search_options.match_case = !model.search_options.match_case;
            model.force_execute_search();
        }
        if Self::action_shortcut(model, ShortcutId::ToggleMatchPath, ctx, check) {
            model.search_options.match_path = !model.search_options.match_path;
            model.force_execute_search();
        }
        if Self::action_shortcut(model, ShortcutId::ToggleMatchWord, ctx, check) {
            model.search_options.match_whole_word = !model.search_options.match_whole_word;
            model.force_execute_search();
        }
        if Self::action_shortcut(model, ShortcutId::ToggleRegex, ctx, check) {
            model.search_options.match_regex = !model.search_options.match_regex;
            model.force_execute_search();
        }
        if Self::action_shortcut(model, ShortcutId::AddBookmark, ctx, check) {
            model.show_bookmarks_dialog = true;
        }
        if Self::action_shortcut(model, ShortcutId::Open, ctx, check) {
            if let Some(entry) = model.selected_entry() {
                controller.open_entry(entry);
            }
        }

        // Zoom shortcuts (Ctrl+= / Ctrl+- / Ctrl+0 / Ctrl+wheel, like Everything)
        if Self::action_shortcut(model, ShortcutId::ZoomIn, ctx, check)
            || ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Equals))
        {
            model.settings.zoom_percent = (model.settings.zoom_percent + 10.0).min(300.0);
            model.settings_dirty = true;
            model.needs_resave = true;
        }
        if Self::action_shortcut(model, ShortcutId::ZoomOut, ctx, check)
            || ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Minus))
        {
            model.settings.zoom_percent = (model.settings.zoom_percent - 10.0).max(50.0);
            model.settings_dirty = true;
            model.needs_resave = true;
        }
        if Self::action_shortcut(model, ShortcutId::ZoomReset, ctx, check)
            || ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Num0))
        {
            model.settings.zoom_percent = 100.0;
            model.settings_dirty = true;
            model.needs_resave = true;
        }
        if ctx.input(|i| i.modifiers.ctrl && i.raw_scroll_delta.y.abs() > 0.0) {
            let scroll = ctx.input(|i| i.raw_scroll_delta.y);
            let delta = if scroll > 0.0 { 5.0 } else { -5.0 };
            model.settings.zoom_percent = (model.settings.zoom_percent + delta).clamp(50.0, 300.0);
            model.settings_dirty = true;
            model.needs_resave = true;
        }

        // Arrow navigation only when the search box is not focused
        let search_focused = ctx.memory(|m| m.has_focus(egui::Id::new("main_search_box")));
        if !search_focused {
            if ctx.input(|i| i.key_pressed(Key::ArrowDown)) && !model.search_results.is_empty() {
                let current = model.selected_index.unwrap_or(0);
                model.selected_index = Some((current + 1).min(model.search_results.len() - 1));
            }
            if ctx.input(|i| i.key_pressed(Key::ArrowUp)) && !model.search_results.is_empty() {
                let current = model.selected_index.unwrap_or(0);
                model.selected_index = Some(current.saturating_sub(1));
            }
        }

        // Search-as-you-type debounce tick
        if model.tick_search_debounce() {
            ctx.request_repaint();
        } else if model.search_pending.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
    }

    fn render_menu_bar(model: &mut AppModel, controller: &mut AppController, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button(model.t("menu.file"), |ui| {
                if ui.button(format!("{}\t{}", model.t("file.new_window"), model.settings.shortcut(ShortcutId::NewWindow))).clicked() {
                    let exe = std::env::current_exe().unwrap_or_default();
                    let _ = std::process::Command::new(exe).spawn();
                    ui.close_menu();
                }
                if ui.button(format!("{}\t{}", model.t("file.open"), model.settings.shortcut(ShortcutId::Open))).clicked() {
                    if let Some(entry) = model.selected_entry() {
                        controller.open_entry(entry);
                    }
                    ui.close_menu();
                }
                if ui.button(format!("{}\t{}", model.t("file.open_path"), model.settings.shortcut(ShortcutId::OpenPath))).clicked() {
                    if let Some(entry) = model.selected_entry() {
                        controller.open_containing_folder(&entry.full_path);
                    }
                    ui.close_menu();
                }
                ui.separator();
                ui.menu_button(format!("{}…", model.t("file.export")), |ui| {
                    for (fmt, label_key) in [
                        ("csv", "file.export_csv"),
                        ("tsv", "file.export_tsv"),
                        ("efu", "file.export_efu"),
                        ("txt", "file.export_txt"),
                        ("json", "file.export_json"),
                    ] {
                        if ui.button(model.t(label_key)).clicked() {
                            controller.export_results(model, fmt);
                            ui.close_menu();
                        }
                    }
                });
                ui.separator();
                if ui.button(format!("{}\t{}", model.t("menu.exit"), model.settings.shortcut(ShortcutId::Exit))).clicked() {
                    ui.close_menu();
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button(model.t("menu.edit"), |ui| {
                if ui.button(format!("{}\t{}", model.t("edit.copy_path"), model.settings.shortcut(ShortcutId::CopyPath))).clicked() {
                    if let Some(entry) = model.selected_entry() {
                        controller.copy_to_clipboard(&entry.full_path);
                    }
                    ui.close_menu();
                }
                if ui.button(format!("{}\t{}", model.t("edit.copy_name"), model.settings.shortcut(ShortcutId::CopyName))).clicked() {
                    if let Some(entry) = model.selected_entry() {
                        controller.copy_to_clipboard(&entry.name);
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button(format!("{}\t{}", model.t("edit.select_all"), model.settings.shortcut(ShortcutId::SelectAll))).clicked() {
                    if !model.search_results.is_empty() {
                        model.selected_index = Some(0);
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button(model.t("menu.view").to_string(), |ui| {
                let t_folders = model.t("view.folders_first").to_string();
                if ui.checkbox(&mut model.search_options.folders_first, &t_folders).changed() {
                    model.force_execute_search();
                }
                let t_status = model.t("view.status_bar").to_string();
                if ui.checkbox(&mut model.settings.show_status_bar, &t_status).changed() {
                    model.needs_resave = true;
                }
                let t_header = model.t("view.header").to_string();
                if ui.checkbox(&mut model.settings.show_header, &t_header).changed() {
                    model.needs_resave = true;
                }
                ui.separator();
                if ui.button(format!("{}  (+10%)", model.t("view.zoom_in"))).clicked() {
                    model.settings.zoom_percent = (model.settings.zoom_percent + 10.0).min(400.0);
                    model.settings_dirty = true;
                    model.needs_resave = true;
                }
                if ui.button(format!("{}  (-10%)", model.t("view.zoom_out"))).clicked() {
                    model.settings.zoom_percent = (model.settings.zoom_percent - 10.0).max(50.0);
                    model.settings_dirty = true;
                    model.needs_resave = true;
                }
                if ui.button(model.t("view.zoom_reset")).clicked() {
                    model.settings.zoom_percent = 100.0;
                    model.settings_dirty = true;
                    model.needs_resave = true;
                }
                ui.separator();
                if ui.button(format!("{}\t{}", model.t("view.refresh"), model.settings.shortcut(ShortcutId::RefreshIndex))).clicked() {
                    controller.start_background_indexing(model);
                    ui.close_menu();
                }
            });

            ui.menu_button(model.t("menu.search").to_string(), |ui| {
                let t_case = model.t("search.match_case").to_string();
                if ui.checkbox(&mut model.search_options.match_case, &t_case).changed() {
                    model.force_execute_search();
                }
                let t_word = model.t("search.match_whole_word").to_string();
                if ui.checkbox(&mut model.search_options.match_whole_word, &t_word).changed() {
                    model.force_execute_search();
                }
                let t_path = model.t("search.match_path").to_string();
                if ui.checkbox(&mut model.search_options.match_path, &t_path).changed() {
                    model.force_execute_search();
                }
                let t_regex = model.t("search.regex").to_string();
                if ui.checkbox(&mut model.search_options.match_regex, &t_regex).changed() {
                    model.force_execute_search();
                }
                ui.separator();
                let t_ayt = model.t("opt.as_you_type").to_string();
                if ui.checkbox(&mut model.settings.search_as_you_type, &t_ayt).changed() {
                    model.needs_resave = true;
                }
                ui.separator();
                if ui.button(format!("{}\t{}", model.t("search.clear"), model.settings.shortcut(ShortcutId::ClearSearch))).clicked() {
                    model.search_query.clear();
                    model.force_execute_search();
                    ui.close_menu();
                }
            });

            ui.menu_button(model.t("menu.bookmarks"), |ui| {
                if ui.button(format!("{}\t{}", model.t("bookmarks.add"), model.settings.shortcut(ShortcutId::AddBookmark))).clicked() {
                    model.show_bookmarks_dialog = true;
                    ui.close_menu();
                }
                ui.separator();
                let bookmarks = model.bookmarks.clone();
                for bookmark in bookmarks {
                    if ui.button(&bookmark.name).clicked() {
                        model.search_query = bookmark.query.clone();
                        model.search_options.filter_category = bookmark.filter;
                        model.force_execute_search();
                        ui.close_menu();
                    }
                }
            });

            ui.menu_button(model.t("menu.tools"), |ui| {
                if ui.button(model.t("tools.rebuild")).clicked() {
                    controller.start_background_indexing(model);
                    ui.close_menu();
                }
                if ui.button("Load Preloaded System Samples").clicked() {
                    model.load_seed_samples();
                    ui.close_menu();
                }
            });

            ui.menu_button(model.t("menu.help"), |ui| {
                if ui.button(model.t("menu.options")).clicked() {
                    model.show_options_dialog = true;
                    ui.close_menu();
                }
                ui.separator();
                if ui.button(model.t("help.about")).clicked() {
                    model.show_about_dialog = true;
                    ui.close_menu();
                }
            });
        });
    }

    fn render_search_bar(model: &mut AppModel, _controller: &mut AppController, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🔍").size(15.0));

            let search_id = egui::Id::new("main_search_box");
            let mut focus_search = false;
            let combo = model.settings.shortcut(ShortcutId::FocusSearch);
            if let Some((m, k)) = parse_shortcut(&combo) {
                if ui.input(|i| i.modifiers == m && i.key_pressed(k)) {
                    focus_search = true;
                }
            }

            let hint = model.t("search.hint");
            let text_edit = egui::TextEdit::singleline(&mut model.search_query)
                .id(search_id)
                .hint_text(hint)
                .desired_width(ui.available_width() - 280.0);

            let res = ui.add(text_edit);
            if res.changed() {
                model.queue_search();
            }
            if focus_search {
                res.request_focus();
            }

            if !model.search_query.is_empty() {
                if ui.button("✕").on_hover_text(model.t("search.clear")).clicked() {
                    model.search_query.clear();
                    model.force_execute_search();
                }
            }

            ui.separator();

            if ui.selectable_label(model.search_options.match_case, "Aa")
                .on_hover_text(format!("{} ({})", model.t("search.match_case"), model.settings.shortcut(ShortcutId::ToggleMatchCase)))
                .clicked()
            {
                model.search_options.match_case = !model.search_options.match_case;
                model.force_execute_search();
            }

            if ui.selectable_label(model.search_options.match_whole_word, "[ \" ]")
                .on_hover_text(format!("{} ({})", model.t("search.match_whole_word"), model.settings.shortcut(ShortcutId::ToggleMatchWord)))
                .clicked()
            {
                model.search_options.match_whole_word = !model.search_options.match_whole_word;
                model.force_execute_search();
            }

            if ui.selectable_label(model.search_options.match_path, "/")
                .on_hover_text(format!("{} ({})", model.t("search.match_path"), model.settings.shortcut(ShortcutId::ToggleMatchPath)))
                .clicked()
            {
                model.search_options.match_path = !model.search_options.match_path;
                model.force_execute_search();
            }

            if ui.selectable_label(model.search_options.match_regex, ".*")
                .on_hover_text(format!("{} ({})", model.t("search.regex"), model.settings.shortcut(ShortcutId::ToggleRegex)))
                .clicked()
            {
                model.search_options.match_regex = !model.search_options.match_regex;
                model.force_execute_search();
            }

            ui.separator();

            egui::ComboBox::from_id_salt("filter_category_dropdown")
                .width(FILTERS_DROPDOWN_WIDTH - 10.0)
                .selected_text(model.search_options.filter_category.name())
                .show_ui(ui, |ui| {
                    for cat in FilterCategory::all() {
                        let selected = model.search_options.filter_category == *cat;
                        if ui.selectable_label(selected, cat.name()).clicked() {
                            model.set_filter(*cat);
                        }
                    }
                });
        });
    }

    fn render_results_table(model: &mut AppModel, controller: &mut AppController, ui: &mut Ui) {
        let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
        let row_height = text_height + 8.0;

        if model.settings.show_header {
            ui.horizontal(|ui| {
                let sort_col = model.search_options.sort_column;
                let sort_dir = model.search_options.sort_direction;

                let make_header = |title: &str, col: SortColumn| -> String {
                    let arrow = if sort_col == col {
                        match sort_dir {
                            SortDirection::Ascending => " ▲",
                            SortDirection::Descending => " ▼",
                        }
                    } else {
                        ""
                    };
                    format!("{}{}", title, arrow)
                };

                if ui.add(egui::Button::new(RichText::new(make_header(&model.t("col.name"), SortColumn::Name)).strong())
                    .min_size(vec2(320.0, 24.0)))
                    .clicked()
                {
                    model.toggle_sort(SortColumn::Name);
                }

                let avail_w = ui.available_width() - 320.0;
                let path_w = (avail_w - 20.0).max(200.0);
                if ui.add(egui::Button::new(RichText::new(make_header(&model.t("col.path"), SortColumn::Path)).strong())
                    .min_size(vec2(path_w, 24.0)))
                    .clicked()
                {
                    model.toggle_sort(SortColumn::Path);
                }

                if ui.add(egui::Button::new(RichText::new(make_header(&model.t("col.size"), SortColumn::Size)).strong())
                    .min_size(vec2(120.0, 24.0)))
                    .clicked()
                {
                    model.toggle_sort(SortColumn::Size);
                }

                if ui.add(egui::Button::new(RichText::new(make_header(&model.t("col.modified"), SortColumn::DateModified)).strong())
                    .min_size(vec2(160.0, 24.0)))
                    .clicked()
                {
                    model.toggle_sort(SortColumn::DateModified);
                }
            });
            ui.separator();
        }

        let total_rows = model.search_results.len();
        if total_rows == 0 {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.label(RichText::new(model.t("res.none")).color(Color32::GRAY));
                if model.engine.get_indexed_count() == 0 {
                    ui.add_space(8.0);
                    if ui.button("Load Windows Directory Samples").clicked() {
                        model.load_seed_samples();
                    }
                }
            });
            return;
        }

        let mut row_to_open: Option<SearchResultEntry> = None;
        let mut row_to_open_folder: Option<String> = None;
        let mut row_to_copy_path: Option<String> = None;
        let mut row_to_copy_name: Option<String> = None;
        let mut row_clicked: Option<usize> = None;

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show_rows(ui, row_height, total_rows, |ui, row_range| {
                for row_idx in row_range {
                    let Some(entry) = model.search_results.get(row_idx) else {
                        continue;
                    };
                    let is_selected = model.selected_index == Some(row_idx);
                    let entry = entry.clone();

                    let (rect, resp) = ui.allocate_exact_size(
                        vec2(ui.available_width(), row_height),
                        Sense::click(),
                    );

                    if is_selected {
                        ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(0, 120, 215));
                    } else if resp.hovered() {
                        ui.painter().rect_filled(
                            rect,
                            2.0,
                            Color32::from_rgba_unmultiplied(200, 220, 245, 60),
                        );
                    }

                    if resp.clicked() {
                        row_clicked = Some(row_idx);
                    }
                    if resp.double_clicked() {
                        row_to_open = Some(entry.clone());
                    }

                    resp.context_menu(|ui| {
                        if ui.button(format!("{}\t{}", model.t("file.open"), model.settings.shortcut(ShortcutId::Open))).clicked() {
                            row_to_open = Some(entry.clone());
                            ui.close_menu();
                        }
                        if ui.button(format!("{}\t{}", model.t("file.open_path"), model.settings.shortcut(ShortcutId::OpenPath))).clicked() {
                            row_to_open_folder = Some(entry.full_path.clone());
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(format!("{}\t{}", model.t("edit.copy_path"), model.settings.shortcut(ShortcutId::CopyPath))).clicked() {
                            row_to_copy_path = Some(entry.full_path.clone());
                            ui.close_menu();
                        }
                        if ui.button(format!("{}\t{}", model.t("edit.copy_name"), model.settings.shortcut(ShortcutId::CopyName))).clicked() {
                            row_to_copy_name = Some(entry.name.clone());
                            ui.close_menu();
                        }
                    });

                    let icon = if entry.is_dir {
                        "📁"
                    } else {
                        match entry.extension.as_str() {
                            "exe" | "msi" | "bat" | "cmd" => "⚙",
                            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" => "🖼",
                            "mp3" | "wav" | "flac" | "ogg" => "🎵",
                            "mp4" | "mkv" | "avi" | "mov" => "🎬",
                            "zip" | "7z" | "rar" | "tar" | "gz" => "📦",
                            "pdf" | "doc" | "docx" => "📄",
                            _ => "🗎",
                        }
                    };

                    let text_color = if is_selected {
                        Color32::WHITE
                    } else {
                        ui.visuals().text_color()
                    };

                    let name_display = format!("{} {}", icon, entry.name);
                    let size_display = if entry.is_dir {
                        String::new()
                    } else {
                        format_size_for(entry.size, model.settings.size_format)
                    };
                    let date_display = format_unix_timestamp(entry.modified, &model.settings.date_format);

                    let font_id = egui::TextStyle::Body.resolve(ui.style());

                    let name_rect = Rect::from_min_size(rect.min + vec2(4.0, 4.0), vec2(316.0, row_height));
                    ui.painter().text(name_rect.min, Align2::LEFT_TOP, name_display, font_id.clone(), text_color);

                    let path_rect = Rect::from_min_size(rect.min + vec2(328.0, 4.0), vec2(ui.available_width() - 610.0, row_height));
                    ui.painter().text(path_rect.min, Align2::LEFT_TOP, &entry.path, font_id.clone(), text_color);

                    let size_rect = Rect::from_min_size(rect.max - vec2(280.0, row_height - 4.0), vec2(110.0, row_height));
                    ui.painter().text(size_rect.max, Align2::RIGHT_BOTTOM, size_display, font_id.clone(), text_color);

                    let date_rect = Rect::from_min_size(rect.max - vec2(160.0, row_height - 4.0), vec2(150.0, row_height));
                    ui.painter().text(date_rect.min, Align2::LEFT_TOP, date_display, font_id.clone(), text_color);
                }
            });

        if let Some(idx) = row_clicked {
            model.selected_index = Some(idx);
        }
        if let Some(entry) = row_to_open {
            controller.open_entry(&entry);
        }
        if let Some(full_path) = row_to_open_folder {
            controller.open_containing_folder(&full_path);
        }
        if let Some(path) = row_to_copy_path {
            controller.copy_to_clipboard(&path);
        }
        if let Some(name) = row_to_copy_name {
            controller.copy_to_clipboard(&name);
        }
    }

    fn render_status_bar(model: &AppModel, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let total = model.search_results.len();
            let selected_text = if model.selected_index.is_some() {
                format!("1 {} {} {}", model.t("status.selected"), total, model.t("status.objects"))
            } else {
                format!("{} {}", total, model.t("status.objects"))
            };

            let duration_text = if model.search_duration.as_millis() > 0 {
                format!("({:.3}s)", model.search_duration.as_secs_f64())
            } else {
                format!("({:.2}ms)", model.search_duration.as_micros() as f64 / 1000.0)
            };

            ui.label(format!("{} {}", selected_text, duration_text));

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(ref status) = model.export_status {
                    ui.label(RichText::new(status).color(Color32::from_rgb(0, 180, 0)));
                }

                if model.is_indexing {
                    ui.label(RichText::new(model.t("status.indexing")).color(Color32::from_rgb(255, 140, 0)));
                } else {
                    let total_db = model.engine.get_indexed_count();
                    ui.label(format!(
                        "{}: {} | {}: {}",
                        model.t("status.db"),
                        total_db,
                        model.t("status.filter"),
                        model.search_options.filter_category.name()
                    ));
                }
            });
        });
    }

    fn render_dialogs(model: &mut AppModel, controller: &mut AppController, ctx: &Context) {
        if model.show_about_dialog {
            egui::Window::new(model.t("help.about"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Everything (Rust)");
                        ui.label("Version 1.5.0a");
                        ui.add_space(8.0);
                        ui.label(model.t("dlg.about_body"));
                        ui.add_space(12.0);
                        if ui.button(model.t("opt.ok")).clicked() {
                            model.show_about_dialog = false;
                        }
                    });
                });
        }

        if model.show_bookmarks_dialog {
            egui::Window::new(model.t("dlg.bookmark_title"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.label(format!("{}: \"{}\"", model.t("dlg.bookmark_query"), model.search_query));
                    ui.label(format!("{}: {}", model.t("status.filter"), model.search_options.filter_category.name()));
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        if ui.button(model.t("dlg.bookmark_save")).clicked() {
                            let name = if model.search_query.is_empty() {
                                format!("Filter: {}", model.search_options.filter_category.name())
                            } else {
                                model.search_query.clone()
                            };
                            model.bookmarks.push(crate::model::Bookmark {
                                name,
                                query: model.search_query.clone(),
                                filter: model.search_options.filter_category,
                            });
                            model.show_bookmarks_dialog = false;
                        }
                        if ui.button(model.t("dlg.bookmark_cancel")).clicked() {
                            model.show_bookmarks_dialog = false;
                        }
                    });
                });
        }

        if model.show_options_dialog {
            Self::render_options_dialog(model, controller, ctx);
        }
    }

    fn render_options_dialog(model: &mut AppModel, _controller: &mut AppController, ctx: &Context) {
        let mut keep_open = true;
        let mut apply = false;

        // Take capture state out to avoid double borrows
        egui::Window::new(model.t("opt.title"))
            .min_width(560.0)
            .default_height(420.0)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                egui::SidePanel::left("opt_nav")
                    .resizable(false)
                    .exact_width(140.0)
                    .show_inside(ui, |ui| {
                        let t0 = model.t("opt.general").to_string();
                        ui.selectable_value(&mut model.options_tab, 0, &t0);
                        let t1 = model.t("opt.shortcuts").to_string();
                        ui.selectable_value(&mut model.options_tab, 1, &t1);
                        let t2 = model.t("opt.index").to_string();
                        ui.selectable_value(&mut model.options_tab, 2, &t2);
                        let t3 = model.t("opt.view").to_string();
                        ui.selectable_value(&mut model.options_tab, 3, &t3);
                        let t4 = model.t("opt.search").to_string();
                        ui.selectable_value(&mut model.options_tab, 4, &t4);
                    });

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    match model.options_tab {
                        0 => Self::options_general(model, ui),
                        1 => Self::options_shortcuts(model, ui),
                        2 => Self::options_index(model, ui),
                        3 => Self::options_view(model, ui),
                        _ => Self::options_search(model, ui),
                    }
                });

                ui.separator();
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(model.t("opt.ok")).clicked() {
                        apply = true;
                        keep_open = false;
                    }
                    if ui.button(model.t("opt.cancel")).clicked() {
                        keep_open = false;
                        let backup = model.settings_backup.clone();
                        model.settings = backup;
                        model.settings_dirty = true;
                    }
                    if ui.button(model.t("opt.apply")).clicked() {
                        apply = true;
                    }
                });
            });
        if !keep_open {
            model.show_options_dialog = false;
        }
        if apply {
            model.search_options.max_results = Some(model.settings.max_results);
            model.settings_backup = model.settings.clone();
            model.settings_dirty = true;
            model.needs_resave = true;
            let _ = model.settings.save();
            model.force_execute_search();
        }
    }

    fn options_general(model: &mut AppModel, ui: &mut Ui) {
        ui.heading(model.t("opt.general"));
        ui.add_space(8.0);

        ui.label(model.t("opt.language"));
        let lang = model.settings.language;
        let auto = lang.is_none();
        let cur = lang.unwrap_or_else(Language::from_os);
        egui::ComboBox::from_id_salt("opt_language")
            .selected_text(if auto {
                format!("{} ({})", model.t("opt.language_auto"), cur.name())
            } else {
                cur.name().to_string()
            })
            .show_ui(ui, |ui| {
                if ui.selectable_label(auto, model.t("opt.language_auto")).clicked() {
                    model.settings.language = None;
                    model.settings_dirty = true;
                }
                for l in [Language::Chinese, Language::English] {
                    let sel = lang == Some(l);
                    if ui.selectable_label(sel, l.name()).clicked() {
                        model.settings.language = Some(l);
                        model.settings_dirty = true;
                    }
                }
            });

        ui.add_space(8.0);
        ui.label(model.t("opt.theme"));
        ui.horizontal(|ui| {
            if ui
                .selectable_label(model.settings.theme == Theme::Dark, model.t("opt.theme_dark"))
                .clicked()
            {
                model.settings.theme = Theme::Dark;
                model.settings_dirty = true;
            }
            if ui
                .selectable_label(model.settings.theme == Theme::Light, model.t("opt.theme_light"))
                .clicked()
            {
                model.settings.theme = Theme::Light;
                model.settings_dirty = true;
            }
        });

        ui.add_space(8.0);
        ui.label(format!("{}: {:.0}%", model.t("opt.scale"), model.settings.zoom_percent));
        let prev_zoom = model.settings.zoom_percent;
        ui.add(
            egui::Slider::new(&mut model.settings.zoom_percent, 50.0..=300.0)
                .step_by(5.0)
                .text("%"),
        );
        ui.horizontal(|ui| {
            ui.label(format!("{}:", model.t("opt.scale_presets")));
            for preset in [75.0, 100.0, 125.0, 150.0, 175.0, 200.0] {
                let text = format!("{:.0}%", preset);
                if ui.selectable_label((model.settings.zoom_percent - preset).abs() < 1.0, text).clicked() {
                    model.settings.zoom_percent = preset;
                    model.settings_dirty = true;
                }
            }
        });
        if (model.settings.zoom_percent - prev_zoom).abs() > 0.01 {
            model.settings_dirty = true;
        }
        ui.add_space(4.0);
        ui.small(model.t("opt.restart_hint"));
    }

    fn options_shortcuts(model: &mut AppModel, ui: &mut Ui) {
        ui.heading(model.t("opt.shortcuts"));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(model.t("opt.shortcut_action"));
            ui.label(model.t("opt.shortcut_keys"));
        });
        ui.separator();

        // Collect key events while capturing, outside the model borrow
        let mut captured_event: Option<Option<String>> = None;
        if let Some(cap_id) = model.shortcut_capture {
            let mut ev_captured: Option<Option<String>> = None;
            ui.input(|i| {
                for ev in &i.events {
                    if let egui::Event::Key { key, modifiers, pressed: true, .. } = ev {
                        if *key == Key::Escape {
                            ev_captured = Some(None); // cancel capture
                        } else {
                            ev_captured = Some(Some(format_combo(*modifiers, *key)));
                        }
                    }
                }
            });
            captured_event = ev_captured;
            let _ = cap_id;
        }

        ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("shortcuts_grid").num_columns(2).show(ui, |ui| {
                for id in ShortcutId::all() {
                    ui.label(shortcut_label(model, *id));
                    let combo = model.settings.shortcut(*id);

                    if model.shortcut_capture == Some(*id) {
                        let btn = egui::Button::new(RichText::new(model.t("opt.press_keys")).color(Color32::YELLOW));
                        if ui.add(btn).clicked() {
                            // keep capturing
                        }
                        if let Some(Some(c)) = &captured_event {
                            model.settings.set_shortcut(*id, c.clone());
                            model.shortcut_capture = None;
                        } else if captured_event.is_some() {
                            model.shortcut_capture = None;
                        }
                    } else {
                        if ui.button(&combo).clicked() {
                            model.shortcut_capture = Some(*id);
                        }
                    }
                    ui.end_row();
                }
            });
        });
        ui.separator();
        if ui.button(model.t("opt.reset_shortcuts")).clicked() {
            model.settings.reset_shortcuts();
        }
    }

    fn options_index(model: &mut AppModel, ui: &mut Ui) {
        ui.heading(model.t("opt.index").to_string());
        ui.add_space(8.0);
        let t1 = model.t("opt.index_ntfs").to_string();
        ui.checkbox(&mut model.settings.index_ntfs, &t1);
        let t2 = model.t("opt.index_hidden").to_string();
        ui.checkbox(&mut model.settings.index_hidden, &t2);
        let t3 = model.t("opt.index_system").to_string();
        ui.checkbox(&mut model.settings.index_system, &t3);
        ui.add_space(8.0);
        ui.label(model.t("opt.exclude"));
        let mut exclude = model.settings.exclude_folders.join(";");
        let resp = ui.add(egui::TextEdit::singleline(&mut exclude).desired_width(f32::INFINITY));
        if resp.changed() {
            model.settings.exclude_folders = exclude
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    fn options_view(model: &mut AppModel, ui: &mut Ui) {
        ui.heading(model.t("opt.view").to_string());
        ui.add_space(8.0);
        let t1 = model.t("opt.status_bar").to_string();
        ui.checkbox(&mut model.settings.show_status_bar, &t1);
        let t2 = model.t("opt.header").to_string();
        ui.checkbox(&mut model.settings.show_header, &t2);
        let t3 = model.t("opt.alternate_row").to_string();
        ui.checkbox(&mut model.settings.alternate_row_color, &t3);
        ui.add_space(8.0);

        ui.label(model.t("opt.size_format"));
        egui::ComboBox::from_id_salt("opt_size_format")
            .selected_text(match model.settings.size_format {
                SizeFormat::Human => model.t("opt.size_human"),
                SizeFormat::Bytes => model.t("opt.size_bytes"),
            })
            .show_ui(ui, |ui| {
                if ui.selectable_label(matches!(model.settings.size_format, SizeFormat::Human), model.t("opt.size_human")).clicked() {
                    model.settings.size_format = SizeFormat::Human;
                }
                if ui.selectable_label(matches!(model.settings.size_format, SizeFormat::Bytes), model.t("opt.size_bytes")).clicked() {
                    model.settings.size_format = SizeFormat::Bytes;
                }
            });

        ui.add_space(8.0);
        ui.label(model.t("opt.date_format"));
        let mut fmt = model.settings.date_format.clone();
        let resp = ui.add(egui::TextEdit::singleline(&mut fmt).desired_width(200.0));
        if resp.changed() {
            model.settings.date_format = fmt;
        }
        ui.small(chrono::Local::now().format(&model.settings.date_format).to_string());
    }

    fn options_search(model: &mut AppModel, ui: &mut Ui) {
        ui.heading(model.t("opt.search").to_string());
        ui.add_space(8.0);
        let t_ayt = model.t("opt.as_you_type").to_string();
        ui.checkbox(&mut model.settings.search_as_you_type, &t_ayt);

        ui.add_space(8.0);
        ui.label(model.t("opt.max_results"));
        let mut max = model.settings.max_results as i64;
        let resp = ui.add(egui::DragValue::new(&mut max).range(100..=10_000_000).speed(1000));
        if resp.changed() {
            model.settings.max_results = max.max(1) as usize;
        }
    }
}

fn shortcut_label(model: &AppModel, id: ShortcutId) -> String {
    match id {
        ShortcutId::NewWindow => model.t("file.new_window"),
        ShortcutId::Open => model.t("file.open"),
        ShortcutId::OpenPath => model.t("file.open_path"),
        ShortcutId::CopyPath => model.t("edit.copy_path"),
        ShortcutId::CopyName => model.t("edit.copy_name"),
        ShortcutId::SelectAll => model.t("edit.select_all"),
        ShortcutId::AddBookmark => model.t("bookmarks.add"),
        ShortcutId::ClearSearch => model.t("search.clear"),
        ShortcutId::RefreshIndex => model.t("view.refresh"),
        ShortcutId::ToggleMatchCase => model.t("search.match_case"),
        ShortcutId::ToggleMatchPath => model.t("search.match_path"),
        ShortcutId::ToggleMatchWord => model.t("search.match_whole_word"),
        ShortcutId::ToggleRegex => model.t("search.regex"),
        ShortcutId::FocusSearch => model.t("opt.focus_search"),
        ShortcutId::ZoomIn => model.t("view.zoom_in"),
        ShortcutId::ZoomOut => model.t("view.zoom_out"),
        ShortcutId::ZoomReset => model.t("view.zoom_reset"),
        ShortcutId::Options => model.t("opt.title"),
        ShortcutId::Exit => model.t("menu.exit"),
    }
}

fn format_combo(mods: Modifiers, key: Key) -> String {
    let mut s = String::new();
    if mods.ctrl {
        s.push_str("Ctrl+");
    }
    if mods.shift {
        s.push_str("Shift+");
    }
    if mods.alt {
        s.push_str("Alt+");
    }
    if mods.command {
        s.push_str("Win+");
    }
    let name = match key {
        Key::Enter => "Enter",
        Key::Escape => "Esc",
        Key::Tab => "Tab",
        Key::Space => "Space",
        Key::ArrowUp => "ArrowUp",
        Key::ArrowDown => "ArrowDown",
        Key::ArrowLeft => "ArrowLeft",
        Key::ArrowRight => "ArrowRight",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        k => return format!("{}{:?}", s, k),
    };
    format!("{}{}", s, name)
}

fn format_size_for(bytes: u64, fmt: crate::settings::SizeFormat) -> String {
    match fmt {
        crate::settings::SizeFormat::Bytes => bytes.to_string(),
        crate::settings::SizeFormat::Human => {
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
    }
}

fn format_unix_timestamp(ts: i64, date_format: &str) -> String {
    if ts <= 0 {
        return String::new();
    }
    if let Some(dt) = Local.timestamp_opt(ts, 0).single() {
        dt.format(date_format).to_string()
    } else {
        String::new()
    }
}
