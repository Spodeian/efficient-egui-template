//! Top navigation bar and global header controls.

use crate::{storage_manager::*, ScreenConstraints, TemplateApp};
use eframe::egui;
use shared::ThemeMode;

pub fn render_navbar(app: &mut TemplateApp, ui: &mut egui::Ui, constraints: &ScreenConstraints) {
    egui::Panel::top("top_panel").show(ui, |ui| {
        ui.add_space(4.0);
        let title_text = if constraints.is_mobile {
            "Serverless & Desktop"
        } else {
            "Serverless & Desktop Template"
        };
        let header_row_height = if constraints.is_mobile { 44.0 } else { 32.0 };

        ui.horizontal(|ui| {
            ui.set_height(header_row_height);

            if constraints.is_mobile {
                ui.label(egui::RichText::new(title_text).size(18.0).strong());
            } else {
                ui.heading(title_text);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let theme_icon = match app.state.config.theme {
                    ThemeMode::Light => {
                        if constraints.is_mobile {
                            "Dark"
                        } else {
                            "Dark Mode"
                        }
                    }
                    ThemeMode::Dark => {
                        if constraints.is_mobile {
                            "Light"
                        } else {
                            "Light Mode"
                        }
                    }
                };

                if ui
                    .button(theme_icon)
                    .on_hover_text("Toggle dark / light theme")
                    .clicked()
                {
                    app.state.config.theme = match app.state.config.theme {
                        ThemeMode::Light => ThemeMode::Dark,
                        ThemeMode::Dark => ThemeMode::Light,
                    };
                    app.persist_state();
                }

                let help_text = if constraints.is_mobile { "Help" } else { "Help" };
                if ui
                    .button(help_text)
                    .on_hover_text("Help, architecture & shortcuts")
                    .clicked()
                {
                    app.show_help_dialog = true;
                }

                // Storage diagnostics button
                let storage_text = match app.storage_diag.is_persisted {
                    Some(true) => {
                        if constraints.is_mobile {
                            "Storage (P)"
                        } else {
                            "Storage: Persistent"
                        }
                    }
                    Some(false) => {
                        if constraints.is_mobile {
                            "Storage (E)"
                        } else {
                            "Storage: Ephemeral"
                        }
                    }
                    None => "Storage",
                };
                if ui
                    .button(storage_text)
                    .on_hover_text("Storage persistence & backups")
                    .clicked()
                {
                    app.show_storage_modal = true;
                }

                // PWA Install Button
                if app.storage_diag.pwa_install_available && !app.storage_diag.is_pwa_installed {
                    if ui
                        .button("Install App")
                        .on_hover_text("Install application for permanent offline storage")
                        .clicked()
                    {
                        trigger_pwa_install();
                    }
                }

                if ui
                    .button("Import")
                    .on_hover_text("Import JSON, CSV or BSON data")
                    .clicked()
                {
                    app.show_import_dialog = true;
                    app.import_text_buffer.clear();
                    app.import_result_message = None;
                }

                if ui
                    .button("Reset")
                    .on_hover_text("Reset to sample items")
                    .clicked()
                {
                    app.show_reset_dialog = true;
                }

                if ui
                    .button("Export")
                    .on_hover_text("Export data to JSON / CSV / BSON")
                    .clicked()
                {
                    let format = app.selected_export_format;
                    app.open_export_dialog(format);
                }
            });
        });
        ui.add_space(4.0);
    });
}
