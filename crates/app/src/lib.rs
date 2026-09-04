pub mod storage_manager;
pub use storage_manager::*;

use eframe::egui;
use shared::{
    export_to_compressed_bson, export_to_csv, export_to_json, import_from_compressed_bson,
    import_from_csv, import_from_json, AppState, ItemCollection, Priority, ThemeMode,
};
#[allow(unused_imports)]
use tracing::{error, info, warn};

#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use wasm_bindgen::JsCast;

pub struct ScreenConstraints {
    pub is_mobile: bool,
    pub is_mobile_portrait: bool,
    pub is_tight_height: bool,
    pub is_ultra_tight: bool,
}

impl ScreenConstraints {
    pub fn compute(ui: &egui::Ui) -> Self {
        let avail_w = ui.available_width();
        let avail_h = ui.available_height();

        Self {
            is_mobile: avail_w < 800.0,
            is_mobile_portrait: avail_w < 650.0,
            is_tight_height: avail_h < 530.0 || avail_w < 350.0,
            is_ultra_tight: avail_w < 330.0 || avail_h < 490.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ExportFormat {
    #[default]
    Json,
    Csv,
    Bson,
}

impl ExportFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Json => "JSON File",
            Self::Csv => "CSV File",
            Self::Bson => "Compressed BSON (.bson)",
        }
    }
}

pub struct TemplateApp {
    pub state: AppState,
    pub current_theme: Option<ThemeMode>,
    pub show_reset_dialog: bool,
    pub show_help_dialog: bool,
    pub show_import_dialog: bool,
    pub import_text_buffer: String,
    pub import_result_message: Option<Result<String, String>>,
    pub show_export_dialog: Option<ExportFormat>,
    pub export_text_buffer: String,
    pub export_copied_notification: Option<f64>,
    pub selected_export_format: ExportFormat,
    pub new_item_title: String,
    pub new_item_description: String,
    pub new_item_priority: Priority,
    pub filter_priority: Option<Priority>,
    pub search_query: String,
    pub storage_diag: StorageDiagnostics,
    pub show_storage_modal: bool,
    pub dismissed_ephemeral_warning: bool,
    pub dismissed_quota_warning: bool,
    pub dismissed_combined_warning: bool,
    pub last_diag_poll_time: f64,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            state: AppState::default(),
            current_theme: None,
            show_reset_dialog: false,
            show_help_dialog: false,
            show_import_dialog: false,
            import_text_buffer: String::new(),
            import_result_message: None,
            show_export_dialog: None,
            export_text_buffer: String::new(),
            export_copied_notification: None,
            selected_export_format: ExportFormat::default(),
            new_item_title: String::new(),
            new_item_description: String::new(),
            new_item_priority: Priority::Medium,
            filter_priority: None,
            search_query: String::new(),
            storage_diag: query_storage_diagnostics(),
            show_storage_modal: false,
            dismissed_ephemeral_warning: false,
            dismissed_quota_warning: false,
            dismissed_combined_warning: false,
            last_diag_poll_time: 0.0,
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        info!("Initializing Serverless & Desktop Template App...");

        let state = load_state_multi_tier(cc.storage).unwrap_or_else(|| {
            warn!("No saved state found in storage, initializing fresh defaults.");
            AppState::default()
        });

        Self {
            state,
            ..Default::default()
        }
    }

    /// Immediately persist current state to multi-tier storage (active persistence)
    pub fn persist_state(&mut self) {
        if let Ok(json_str) = serde_json::to_string(&self.state) {
            match save_state_multi_tier(DEDICATED_STORAGE_KEY, &json_str) {
                Ok(backend) => {
                    self.storage_diag.backend = backend;
                    if backend == StorageBackend::IndexedDb {
                        self.storage_diag.quota_exceeded = true;
                        self.storage_diag.idb_active = true;
                    } else {
                        self.storage_diag.quota_exceeded = false;
                    }
                }
                Err(_) => {
                    self.storage_diag.quota_exceeded = true;
                }
            }
        }
    }

    pub fn open_export_dialog(&mut self, format: ExportFormat) {
        if format == ExportFormat::Bson {
            if let Ok(bytes) = export_to_compressed_bson(&self.state.collection) {
                use base64::{engine::general_purpose, Engine as _};
                self.export_text_buffer = general_purpose::STANDARD.encode(&bytes);
                trigger_binary_download("data_backup.bson", &bytes, "application/octet-stream");
            }
        } else {
            self.export_text_buffer = match format {
                ExportFormat::Json => export_to_json(&self.state.collection).unwrap_or_default(),
                ExportFormat::Csv => export_to_csv(&self.state.collection),
                ExportFormat::Bson => unreachable!(),
            };
        }
        self.show_export_dialog = Some(format);
        self.export_copied_notification = None;
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        if self.current_theme == Some(self.state.config.theme) {
            return;
        }
        self.current_theme = Some(self.state.config.theme);

        let visuals = match self.state.config.theme {
            ThemeMode::Light => {
                let mut light = egui::Visuals::light();

                // Soothing neutral/warm light backgrounds
                light.panel_fill = egui::Color32::from_rgb(245, 244, 241);
                light.window_fill = egui::Color32::from_rgb(252, 250, 246);
                light.extreme_bg_color = egui::Color32::from_rgb(238, 236, 231);

                // Soft charcoal for high-contrast, comfortable reading
                light.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(45, 44, 42);
                light.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(55, 54, 52);
                light.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(20, 20, 18);
                light.widgets.active.fg_stroke.color = egui::Color32::from_rgb(0, 0, 0);

                // Muted border strokes
                light.widgets.noninteractive.bg_stroke.color = egui::Color32::from_rgb(222, 220, 215);
                light.widgets.inactive.bg_stroke.color = egui::Color32::from_rgb(212, 210, 205);

                // Buttons background
                light.widgets.inactive.bg_fill = egui::Color32::from_rgb(252, 251, 248);
                light.widgets.hovered.bg_fill = egui::Color32::from_rgb(236, 234, 229);
                light.widgets.active.bg_fill = egui::Color32::from_rgb(220, 218, 212);

                light
            }
            ThemeMode::Dark => egui::Visuals::dark(),
        };
        ctx.set_visuals(visuals);
    }

    fn render_top_bar(&mut self, ui: &mut egui::Ui, constraints: &ScreenConstraints) {
        egui::Panel::top("top_panel").show(ui, |ui| {
            ui.add_space(4.0);
            let title_text = if constraints.is_mobile { "Serverless & Desktop" } else { "Serverless & Desktop Template" };
            let header_row_height = if constraints.is_mobile { 44.0 } else { 32.0 };

            ui.horizontal(|ui| {
                ui.set_height(header_row_height);

                if constraints.is_mobile {
                    ui.label(egui::RichText::new(title_text).size(18.0).strong());
                } else {
                    ui.heading(title_text);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let theme_icon = match self.state.config.theme {
                        ThemeMode::Light => if constraints.is_mobile { "Dark" } else { "Dark Mode" },
                        ThemeMode::Dark => if constraints.is_mobile { "Light" } else { "Light Mode" },
                    };

                    if ui.button(theme_icon).on_hover_text("Toggle dark / light theme").clicked() {
                        self.state.config.theme = match self.state.config.theme {
                            ThemeMode::Light => ThemeMode::Dark,
                            ThemeMode::Dark => ThemeMode::Light,
                        };
                        self.persist_state();
                    }

                    let help_text = if constraints.is_mobile { "Help" } else { "Help" };
                    if ui.button(help_text).on_hover_text("Help, architecture & shortcuts").clicked() {
                        self.show_help_dialog = true;
                    }

                    // Storage diagnostics button
                    let storage_text = match self.storage_diag.is_persisted {
                        Some(true) => if constraints.is_mobile { "Storage (P)" } else { "Storage: Persistent" },
                        Some(false) => if constraints.is_mobile { "Storage (E)" } else { "Storage: Ephemeral" },
                        None => "Storage",
                    };
                    if ui.button(storage_text).on_hover_text("Storage persistence & backups").clicked() {
                        self.show_storage_modal = true;
                    }

                    // PWA Install Button
                    if self.storage_diag.pwa_install_available && !self.storage_diag.is_pwa_installed {
                        if ui.button("Install App").on_hover_text("Install application for permanent offline storage").clicked() {
                            trigger_pwa_install();
                        }
                    }

                    if ui.button("Import").on_hover_text("Import JSON, CSV or BSON data").clicked() {
                        self.show_import_dialog = true;
                        self.import_text_buffer.clear();
                        self.import_result_message = None;
                    }

                    if ui.button("Reset").on_hover_text("Reset to sample items").clicked() {
                        self.show_reset_dialog = true;
                    }

                    if ui.button("Export").on_hover_text("Export data to JSON / CSV / BSON").clicked() {
                        self.open_export_dialog(self.selected_export_format);
                    }
                });
            });
            ui.add_space(4.0);
        });
    }

    fn render_summary_cards(&mut self, ui: &mut egui::Ui, constraints: &ScreenConstraints) {
        let total = self.state.collection.total_count();
        let completed = self.state.collection.completed_count();
        let ratio = self.state.collection.completion_ratio();

        egui::Frame::group(ui.style())
            .inner_margin(if constraints.is_mobile { 10.0 } else { 16.0 })
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Progress Overview").strong().size(16.0));
                        ui.label(format!("Completed {} of {} items ({:.0}%)", completed, total, ratio * 100.0));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let bar_width = (ui.available_width() - 8.0).clamp(100.0, 300.0);
                        ui.add(
                            egui::ProgressBar::new(ratio)
                                .text(format!("{:.0}%", ratio * 100.0))
                                .desired_width(bar_width),
                        );
                    });
                });
            });
    }

    fn render_new_item_form(&mut self, ui: &mut egui::Ui, constraints: &ScreenConstraints) {
        egui::Frame::group(ui.style())
            .inner_margin(if constraints.is_mobile { 10.0 } else { 16.0 })
            .corner_radius(8.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Add New Item").strong().size(15.0));
                ui.add_space(6.0);

                let mut submit_new_item = false;

                if constraints.is_mobile {
                    ui.vertical(|ui| {
                        let title_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.new_item_title)
                                .hint_text("Item title...")
                                .desired_width(f32::INFINITY),
                        );
                        if title_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            submit_new_item = true;
                        }
                        ui.add_space(4.0);
                        let desc_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.new_item_description)
                                .hint_text("Item description (optional)...")
                                .desired_width(f32::INFINITY),
                        );
                        if desc_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            submit_new_item = true;
                        }
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("mobile_priority_dropdown")
                                .selected_text(format!("Priority: {}", self.new_item_priority.label()))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.new_item_priority, Priority::Low, "Low");
                                    ui.selectable_value(&mut self.new_item_priority, Priority::Medium, "Medium");
                                    ui.selectable_value(&mut self.new_item_priority, Priority::High, "High");
                                });

                            if ui.button("➕ Add Item").clicked() {
                                submit_new_item = true;
                            }
                        });
                    });
                } else {
                    ui.horizontal(|ui| {
                        let title_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.new_item_title)
                                .hint_text("Item title...")
                                .desired_width(220.0),
                        );
                        if title_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            submit_new_item = true;
                        }
                        let desc_resp = ui.add(
                            egui::TextEdit::singleline(&mut self.new_item_description)
                                .hint_text("Description (optional)...")
                                .desired_width(320.0),
                        );
                        if desc_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            submit_new_item = true;
                        }
                        egui::ComboBox::from_id_salt("desktop_priority_dropdown")
                            .selected_text(self.new_item_priority.label())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.new_item_priority, Priority::Low, "Low");
                                ui.selectable_value(&mut self.new_item_priority, Priority::Medium, "Medium");
                                ui.selectable_value(&mut self.new_item_priority, Priority::High, "High");
                            });

                        if ui.button("➕ Add Item").clicked() {
                            submit_new_item = true;
                        }
                    });
                }

                if submit_new_item && !self.new_item_title.trim().is_empty() {
                    self.state.collection.add(
                        self.new_item_title.trim(),
                        self.new_item_description.trim(),
                        self.new_item_priority,
                    );
                    self.new_item_title.clear();
                    self.new_item_description.clear();
                    self.persist_state();
                }
            });
    }

    fn render_item_list(&mut self, ui: &mut egui::Ui, constraints: &ScreenConstraints) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Items").heading());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear Completed").on_hover_text("Remove all finished items").clicked() {
                    self.state.collection.clear_completed();
                    self.persist_state();
                }

                egui::ComboBox::from_id_salt("filter_priority_dropdown")
                    .selected_text(match self.filter_priority {
                        None => "All Priorities",
                        Some(p) => p.label(),
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.filter_priority, None, "All Priorities");
                        ui.selectable_value(&mut self.filter_priority, Some(Priority::Low), "Low");
                        ui.selectable_value(&mut self.filter_priority, Some(Priority::Medium), "Medium");
                        ui.selectable_value(&mut self.filter_priority, Some(Priority::High), "High");
                    });

                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Search items...")
                        .desired_width(if constraints.is_mobile { 130.0 } else { 180.0 }),
                );
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        let query = self.search_query.trim().to_lowercase();
        let priority_filter = self.filter_priority;

        let mut item_ids_to_toggle = Vec::new();
        let mut item_ids_to_remove = Vec::new();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let matching_items: Vec<_> = self
                    .state
                    .collection
                    .items
                    .iter()
                    .filter(|item| {
                        if let Some(pf) = priority_filter
                            && item.priority != pf
                        {
                            return false;
                        }
                        if !query.is_empty() {
                            let match_title = item.title.to_lowercase().contains(&query);
                            let match_desc = item.description.to_lowercase().contains(&query);
                            if !match_title && !match_desc {
                                return false;
                            }
                        }
                        true
                    })
                    .cloned()
                    .collect();

                if matching_items.is_empty() {
                    ui.add_space(20.0);
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new("No items match your query or filter.").weak());
                    });
                    return;
                }

                for item in matching_items {
                    egui::Frame::group(ui.style())
                        .inner_margin(10.0)
                        .corner_radius(6.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let mut checked = item.completed;
                                if ui.checkbox(&mut checked, "").clicked() {
                                    item_ids_to_toggle.push(item.id);
                                }

                                ui.vertical(|ui| {
                                    let mut title_text = egui::RichText::new(&item.title).strong().size(14.0);
                                    if item.completed {
                                        title_text = title_text.strikethrough().weak();
                                    }
                                    ui.label(title_text);

                                    if !item.description.is_empty() {
                                        let mut desc_text = egui::RichText::new(&item.description).small();
                                        if item.completed {
                                            desc_text = desc_text.weak();
                                        }
                                        ui.label(desc_text);
                                    }
                                });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("Delete").on_hover_text("Delete item").clicked() {
                                        item_ids_to_remove.push(item.id);
                                    }

                                    let badge_color = match item.priority {
                                        Priority::High => egui::Color32::from_rgb(220, 70, 70),
                                        Priority::Medium => egui::Color32::from_rgb(230, 140, 50),
                                        Priority::Low => egui::Color32::from_rgb(100, 140, 180),
                                    };
                                    ui.colored_label(badge_color, egui::RichText::new(item.priority.label()).strong().small());
                                });
                            });
                        });
                    ui.add_space(4.0);
                }
            });

        let mut collection_changed = false;
        for id in item_ids_to_toggle {
            self.state.collection.toggle(id);
            collection_changed = true;
        }
        for id in item_ids_to_remove {
            self.state.collection.remove(id);
            collection_changed = true;
        }
        if collection_changed {
            self.persist_state();
        }
    }

    fn render_dialogs(&mut self, ui: &mut egui::Ui) {
        if self.show_help_dialog {
            let mut open = true;
            let win_w = (ui.available_width() - 24.0).clamp(340.0, 600.0);
            let win_h = (ui.available_height() - 32.0).clamp(440.0, 680.0);

            egui::Window::new("Help & Information")
                .open(&mut open)
                .resizable(true)
                .collapsible(true)
                .default_size(egui::vec2(win_w, win_h))
                .min_width(320.0)
                .min_height(380.0)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ui.ctx(), |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.heading("Serverless & Desktop Template");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                                        .strong()
                                        .color(ui.visuals().hyperlink_color),
                                );
                            });
                        });
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.heading("Serverless & Desktop Architecture");
                        ui.add_space(4.0);
                        ui.label("This template demonstrates a production-grade, offline-first application architecture compiling to both WebAssembly (via eframe / Trunk / Cloudflare Pages) and Native Desktop (via eframe / Winit).");

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.heading("Multi-Tier Storage Engine");
                        ui.add_space(4.0);
                        ui.label("• Tier 1: Fast synchronous local storage.");
                        ui.label("• Tier 2: Asynchronous IndexedDB extended quota fallback.");
                        ui.label("• Persistence: StorageManager persistence bridge prevents browser data eviction.");

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.heading("Local Privacy & Security");
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("100% Client-Side: No telemetry or server database calls.")
                                .color(egui::Color32::from_rgb(80, 160, 90))
                                .strong(),
                        );
                        ui.label("Your data is stored strictly in your local browser or desktop application storage.");
                    });
                });
            if !open {
                self.show_help_dialog = false;
            }
        }

        if self.show_reset_dialog {
            egui::Window::new("Reset Data?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ui.ctx(), |ui| {
                    ui.label("Are you sure you want to reset all items to default sample data?");
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Yes, Reset").clicked() {
                            self.state.collection = ItemCollection::default();
                            self.show_reset_dialog = false;
                            self.persist_state();
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_reset_dialog = false;
                        }
                    });
                });
        }

        if let Some(format) = self.show_export_dialog {
            let mut open = true;
            egui::Window::new(format!("Export {}", format.label()))
                .open(&mut open)
                .default_size(egui::vec2(540.0, 380.0))
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Copy to Clipboard").clicked() {
                            ui.ctx().copy_text(self.export_text_buffer.clone());
                            self.export_copied_notification = Some(ui.input(|i| i.time));
                        }
                        if let Some(t) = self.export_copied_notification
                            && ui.input(|i| i.time) - t < 3.0
                        {
                            ui.label(egui::RichText::new("Copied to clipboard!").color(egui::Color32::GREEN));
                        }

                        ui.separator();

                        if ui.button("Download File").clicked() {
                            match format {
                                ExportFormat::Json => {
                                    trigger_text_download("data_export.json", &self.export_text_buffer, "application/json;charset=utf-8");
                                }
                                ExportFormat::Csv => {
                                    trigger_text_download("data_export.csv", &self.export_text_buffer, "text/csv;charset=utf-8");
                                }
                                ExportFormat::Bson => {
                                    if let Ok(bytes) = export_to_compressed_bson(&self.state.collection) {
                                        trigger_binary_download("data_backup.bson", &bytes, "application/octet-stream");
                                    }
                                }
                            }
                        }
                    });

                    ui.add_space(8.0);
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.export_text_buffer)
                                .font(egui::TextStyle::Monospace)
                                .code_editor()
                                .lock_focus(true)
                                .desired_width(f32::INFINITY),
                        );
                    });
                });

            if !open {
                self.show_export_dialog = None;
                self.export_text_buffer.clear();
            }
        }

        if self.show_import_dialog {
            let mut open = true;
            egui::Window::new("Import Data")
                .open(&mut open)
                .default_size(egui::vec2(500.0, 360.0))
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ui.ctx(), |ui| {
                    ui.label("Paste JSON, CSV or Base64 BSON data below to import into your collection:");
                    ui.add_space(8.0);

                    egui::ScrollArea::both()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.import_text_buffer)
                                    .font(egui::TextStyle::Monospace)
                                    .hint_text("Paste JSON, CSV, or Base64 BSON content here...")
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(8),
                            );
                        });

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Apply Import").clicked() {
                            let input = self.import_text_buffer.trim();
                            if input.is_empty() {
                                self.import_result_message = Some(Err("Input is empty.".to_string()));
                            } else if input.starts_with('{') {
                                match import_from_json(input) {
                                    Ok(col) => {
                                        let count = col.total_count();
                                        self.state.collection = col;
                                        self.import_result_message = Some(Ok(format!("Successfully imported {} items from JSON!", count)));
                                        self.persist_state();
                                    }
                                    Err(e) => {
                                        self.import_result_message = Some(Err(e.to_string()));
                                    }
                                }
                            } else if let Ok(decoded_bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, input) {
                                match import_from_compressed_bson(&decoded_bytes) {
                                    Ok(col) => {
                                        let count = col.total_count();
                                        self.state.collection = col;
                                        self.import_result_message = Some(Ok(format!("Successfully imported {} items from compressed BSON!", count)));
                                        self.persist_state();
                                    }
                                    Err(e) => {
                                        self.import_result_message = Some(Err(format!("BSON import failed: {}", e)));
                                    }
                                }
                            } else {
                                match import_from_csv(input) {
                                    Ok(col) => {
                                        let count = col.total_count();
                                        self.state.collection = col;
                                        self.import_result_message = Some(Ok(format!("Successfully imported {} items from CSV!", count)));
                                        self.persist_state();
                                    }
                                    Err(e) => {
                                        self.import_result_message = Some(Err(e.to_string()));
                                    }
                                }
                            }
                        }

                        if ui.button("Cancel").clicked() {
                            self.show_import_dialog = false;
                            self.import_text_buffer.clear();
                            self.import_result_message = None;
                        }
                    });

                    if let Some(ref result) = self.import_result_message {
                        ui.add_space(6.0);
                        match result {
                            Ok(msg) => {
                                ui.label(egui::RichText::new(msg).color(egui::Color32::from_rgb(80, 180, 90)).strong());
                            }
                            Err(msg) => {
                                ui.label(egui::RichText::new(msg).color(egui::Color32::from_rgb(220, 70, 70)).strong());
                            }
                        }
                    }
                });

            if !open {
                self.show_import_dialog = false;
                self.import_text_buffer.clear();
                self.import_result_message = None;
            }
        }

        if self.show_storage_modal {
            self.render_storage_modal(ui);
        }
    }

    fn render_warning_banners(&mut self, ui: &mut egui::Ui) {
        let is_ephemeral = self.storage_diag.is_persisted == Some(false);
        let is_quota = self.storage_diag.quota_exceeded;

        if is_ephemeral && is_quota && !self.dismissed_combined_warning {
            egui::Frame::group(ui.style())
                .fill(egui::Color32::from_rgb(60, 20, 20))
                .inner_margin(8.0)
                .corner_radius(6.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Storage Alert: Storage is Ephemeral AND Quota Limit Exceeded!").color(egui::Color32::from_rgb(255, 120, 120)).strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Dismiss").clicked() {
                                self.dismissed_combined_warning = true;
                            }
                            if ui.button("Save .bson Backup").clicked() {
                                if let Ok(bytes) = export_to_compressed_bson(&self.state.collection) {
                                    trigger_binary_download("emergency_backup.bson", &bytes, "application/octet-stream");
                                }
                            }
                            if ui.button("Request Permission").clicked() {
                                request_persistent_storage();
                            }
                        });
                    });
                });
            ui.add_space(4.0);
        } else {
            if is_ephemeral && !self.dismissed_ephemeral_warning {
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(50, 35, 10))
                    .inner_margin(8.0)
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Ephemeral Storage: Browser may clear local data under storage pressure.").color(egui::Color32::from_rgb(250, 190, 80)).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Dismiss").clicked() {
                                    self.dismissed_ephemeral_warning = true;
                                }
                                if ui.button("Backup .bson").clicked() {
                                    if let Ok(bytes) = export_to_compressed_bson(&self.state.collection) {
                                        trigger_binary_download("data_backup.bson", &bytes, "application/octet-stream");
                                    }
                                }
                                if ui.button("Request Persistence").clicked() {
                                    request_persistent_storage();
                                }
                            });
                        });
                    });
                ui.add_space(4.0);
            }

            if is_quota && !self.dismissed_quota_warning {
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgb(55, 25, 15))
                    .inner_margin(8.0)
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Storage Quota Exceeded: State migrated to IndexedDB fallback tier.").color(egui::Color32::from_rgb(250, 140, 80)).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Dismiss").clicked() {
                                    self.dismissed_quota_warning = true;
                                }
                                if ui.button("Save .bson Backup").clicked() {
                                    if let Ok(bytes) = export_to_compressed_bson(&self.state.collection) {
                                        trigger_binary_download("data_backup.bson", &bytes, "application/octet-stream");
                                    }
                                }
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        }
    }

    fn render_storage_modal(&mut self, ui: &mut egui::Ui) {
        let mut open = true;
        egui::Window::new("Storage & Data Management")
            .open(&mut open)
            .default_size(egui::vec2(480.0, 380.0))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ui.ctx(), |ui| {
                ui.heading("Storage Diagnostics");
                ui.add_space(4.0);

                let status_color = match self.storage_diag.is_persisted {
                    Some(true) => egui::Color32::from_rgb(80, 200, 100),
                    Some(false) => egui::Color32::from_rgb(240, 160, 50),
                    None => egui::Color32::GRAY,
                };
                let status_label = match self.storage_diag.is_persisted {
                    Some(true) => "Persistent (Immune to browser eviction)",
                    Some(false) => "Ephemeral (May be cleared if disk is low)",
                    None => "Unknown / Querying...",
                };

                ui.horizontal(|ui| {
                    ui.label("Persistence Status:");
                    ui.colored_label(status_color, egui::RichText::new(status_label).strong());
                });

                ui.horizontal(|ui| {
                    ui.label("Active Storage Tier:");
                    ui.label(egui::RichText::new(self.storage_diag.backend.label()).strong());
                });

                ui.horizontal(|ui| {
                    ui.label("PWA Installation:");
                    if self.storage_diag.is_pwa_installed {
                        ui.colored_label(egui::Color32::from_rgb(80, 200, 100), "Installed (Permanent App)");
                    } else if self.storage_diag.pwa_install_available {
                        ui.label("Available to Install");
                    } else {
                        ui.label("Not Available in Browser Tab");
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                ui.heading("Actions");
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if self.storage_diag.is_persisted != Some(true) {
                        if ui.button("Request Persistent Storage").on_hover_text("Ask browser permission to protect state from auto-eviction").clicked() {
                            request_persistent_storage();
                        }
                    }

                    if self.storage_diag.pwa_install_available && !self.storage_diag.is_pwa_installed {
                        if ui.button("Install Web App").on_hover_text("Install to homescreen/desktop for highest durability").clicked() {
                            trigger_pwa_install();
                        }
                    }
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Export Compressed BSON Backup").on_hover_text("Download compact, offline state backup (.bson)").clicked() {
                        if let Ok(bytes) = export_to_compressed_bson(&self.state.collection) {
                            trigger_binary_download("data_backup.bson", &bytes, "application/octet-stream");
                        }
                    }

                    if ui.button("Import Backup").clicked() {
                        self.show_storage_modal = false;
                        self.show_import_dialog = true;
                    }
                });
            });

        if !open {
            self.show_storage_modal = false;
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.show_help_dialog {
                self.show_help_dialog = false;
            } else if self.show_reset_dialog {
                self.show_reset_dialog = false;
            } else if self.show_storage_modal {
                self.show_storage_modal = false;
            } else if self.show_export_dialog.is_some() {
                self.show_export_dialog = None;
                self.export_text_buffer.clear();
            } else if self.show_import_dialog {
                self.show_import_dialog = false;
                self.import_text_buffer.clear();
                self.import_result_message = None;
            }
        }
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // 1. Write standard RON state to eframe::APP_KEY
        eframe::set_value(storage, eframe::APP_KEY, &self.state);

        // 2. Write JSON to dedicated key in storage
        if let Ok(json_str) = serde_json::to_string(&self.state) {
            storage.set_string(DEDICATED_STORAGE_KEY, json_str);
        }
        storage.flush();

        // 3. Persist to multi-tier engine
        self.persist_state();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.apply_theme(ui.ctx());
        self.handle_keyboard_shortcuts(ui.ctx());

        // Periodic diagnostics poll (every 2 seconds)
        let cur_time = ui.input(|i| i.time);
        if cur_time - self.last_diag_poll_time > 2.0 {
            self.last_diag_poll_time = cur_time;
            let queried = query_storage_diagnostics();
            self.storage_diag.is_persisted = queried.is_persisted;
            self.storage_diag.pwa_install_available = queried.pwa_install_available;
            self.storage_diag.is_pwa_installed = queried.is_pwa_installed;
        }

        let constraints = ScreenConstraints::compute(ui);
        self.render_top_bar(ui, &constraints);

        egui::CentralPanel::default().show(ui, |ui| {
            self.render_warning_banners(ui);
            ui.add_space(8.0);
            self.render_summary_cards(ui, &constraints);
            ui.add_space(10.0);
            self.render_new_item_form(ui, &constraints);
            ui.add_space(10.0);
            self.render_item_list(ui, &constraints);
        });

        self.render_dialogs(ui);
    }
}
