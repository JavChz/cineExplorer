use std::path::PathBuf;
use egui::{Color32, RichText, Window};

pub enum CatalogDialogResult {
    Open(PathBuf),
    Create(PathBuf),
    Cancel,
}

pub struct CatalogDialogState {
    pub is_open: bool,
    pub is_creating_new: bool,
    pub catalog_name_input: String,
    pub target_folder: Option<PathBuf>,
}

impl Default for CatalogDialogState {
    fn default() -> Self {
        Self {
            is_open: false,
            is_creating_new: false,
            catalog_name_input: "MyVideoCatalog".to_string(),
            target_folder: None,
        }
    }
}

pub fn render_catalog_dialog(
    ctx: &egui::Context,
    state: &mut CatalogDialogState,
) -> Option<CatalogDialogResult> {
    if !state.is_open {
        return None;
    }

    let mut result = None;

    Window::new(if state.is_creating_new { "Create New Video Catalog" } else { "Open Video Catalog" })
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_width(380.0);
            ui.spacing_mut().item_spacing.y = 10.0;

            if state.is_creating_new {
                ui.label(RichText::new("Enter a name for your new SQLite video catalog:").strong());
                ui.add(egui::TextEdit::singleline(&mut state.catalog_name_input).desired_width(360.0));

                ui.horizontal(|ui| {
                    ui.label("Location:");
                    if let Some(ref f) = state.target_folder {
                        ui.label(RichText::new(f.to_string_lossy().to_string()).small());
                    } else {
                        ui.label(RichText::new("Default Documents folder").small().color(Color32::from_rgb(140, 140, 140)));
                    }
                    if ui.button("Browse...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            state.target_folder = Some(folder);
                        }
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Create Catalog").clicked() {
                        let folder = state.target_folder.clone().unwrap_or_else(|| {
                            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                        });
                        let file_name = if state.catalog_name_input.trim().ends_with(".cinecat") {
                            state.catalog_name_input.trim().to_string()
                        } else {
                            format!("{}.cinecat", state.catalog_name_input.trim())
                        };
                        let full_path = folder.join(file_name);
                        state.is_open = false;
                        result = Some(CatalogDialogResult::Create(full_path));
                    }

                    if ui.button("Cancel").clicked() {
                        state.is_open = false;
                        result = Some(CatalogDialogResult::Cancel);
                    }
                });
            } else {
                ui.label("Select an existing .cinecat or .db SQLite catalog file:");

                ui.horizontal(|ui| {
                    if ui.button("📂 Browse for Catalog File...").clicked() {
                        if let Some(file) = rfd::FileDialog::new()
                            .add_filter("CineExplorer Catalog", &["cinecat", "db", "sqlite"])
                            .pick_file()
                        {
                            state.is_open = false;
                            result = Some(CatalogDialogResult::Open(file));
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        state.is_open = false;
                        result = Some(CatalogDialogResult::Cancel);
                    }
                });
            }
        });

    result
}
