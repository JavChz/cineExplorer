use std::path::PathBuf;
use egui::{Color32, ProgressBar, RichText, Window};
use crate::importer::ImportProgress;
use crate::ui::theme::*;

pub enum IngestDialogAction {
    StartIngest(Vec<PathBuf>),
    Cancel,
}

pub struct IngestDialogState {
    pub is_open: bool,
    pub selected_paths: Vec<PathBuf>,
    pub is_running: bool,
    pub progress: Option<ImportProgress>,
}

impl Default for IngestDialogState {
    fn default() -> Self {
        Self {
            is_open: false,
            selected_paths: Vec::new(),
            is_running: false,
            progress: None,
        }
    }
}

pub fn render_ingest_dialog(
    ctx: &egui::Context,
    state: &mut IngestDialogState,
) -> Option<IngestDialogAction> {
    if !state.is_open {
        return None;
    }

    let mut action = None;

    Window::new("Ingest Videos into Catalog")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_width(460.0);
            ui.spacing_mut().item_spacing.y = 10.0;

            if state.is_running {
                ui.label(RichText::new("Ingesting and analyzing video metadata...").strong());

                if let Some(ref prog) = state.progress {
                    let fraction = if prog.total > 0 {
                        (prog.current as f32) / (prog.total as f32)
                    } else {
                        0.0
                    };

                    ui.add(ProgressBar::new(fraction).show_percentage());

                    ui.label(RichText::new(format!("Analyzing: {}", prog.current_file)).small().color(TEXT_MUTED));

                    ui.horizontal(|ui| {
                        ui.label(format!("Imported: {}", prog.imported));
                        ui.label(format!("Duplicates skipped: {}", prog.duplicates));
                        if prog.failed > 0 {
                            ui.label(RichText::new(format!("Failed: {}", prog.failed)).color(ACCENT_RED));
                        }
                    });
                } else {
                    ui.add(ProgressBar::new(0.0).text("Scanning directory tree..."));
                }

                ui.separator();

                if ui.button("Cancel Ingest").clicked() {
                    action = Some(IngestDialogAction::Cancel);
                }
            } else {
                ui.label(RichText::new("Select video files or directories to index into this catalog:").strong());
                ui.label(RichText::new("Supports MP4, MOV, BRAW, MKV, MXF, ProRes, H.264, H.265 (8/10/12-bit)").small().color(TEXT_MUTED));

                ui.horizontal(|ui| {
                    if ui.button("📁 Choose Folder...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            state.selected_paths.push(folder);
                        }
                    }

                    if ui.button("📄 Choose Files...").clicked() {
                        if let Some(files) = rfd::FileDialog::new()
                            .add_filter("Video Files", crate::media::SUPPORTED_EXTENSIONS)
                            .pick_files()
                        {
                            state.selected_paths.extend(files);
                        }
                    }

                    if !state.selected_paths.is_empty() && ui.button("Clear List").clicked() {
                        state.selected_paths.clear();
                    }
                });

                if !state.selected_paths.is_empty() {
                    ui.separator();
                    ui.label(RichText::new(format!("Selected Sources ({} items):", state.selected_paths.len())).small());

                    egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                        for p in &state.selected_paths {
                            ui.label(RichText::new(p.to_string_lossy().to_string()).small().monospace());
                        }
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        let start_btn = egui::Button::new(RichText::new("Start Ingest").strong().color(Color32::WHITE))
                            .fill(ACCENT_BLUE);

                        if ui.add(start_btn).clicked() {
                            action = Some(IngestDialogAction::StartIngest(state.selected_paths.clone()));
                        }

                        if ui.button("Cancel").clicked() {
                            state.is_open = false;
                            state.selected_paths.clear();
                        }
                    });
                } else {
                    ui.add_space(8.0);
                    if ui.button("Close").clicked() {
                        state.is_open = false;
                    }
                }
            }
        });

    action
}
