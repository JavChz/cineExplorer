use std::path::PathBuf;
use egui::{Color32, ProgressBar, RichText, Window};
use crate::importer::{ImportProgress, ImportSummary};
use crate::ui::theme::*;

pub enum ImportDialogAction {
    StartImport(Vec<PathBuf>),
    Cancel,
    Done,
}

#[derive(Default)]
pub struct ImportDialogState {
    pub is_open: bool,
    pub selected_paths: Vec<PathBuf>,
    pub is_running: bool,
    pub progress: Option<ImportProgress>,
    pub summary: Option<ImportSummary>,
    pub error_message: Option<String>,
}

impl ImportDialogState {
    pub fn reset(&mut self) {
        self.selected_paths.clear();
        self.is_running = false;
        self.progress = None;
        self.summary = None;
        self.error_message = None;
    }
}

pub fn render_import_dialog(
    ctx: &egui::Context,
    state: &mut ImportDialogState,
) -> Option<ImportDialogAction> {
    if !state.is_open {
        return None;
    }

    let mut action = None;

    Window::new("Import Footage (In-Place Indexing)")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_width(520.0);
            ui.spacing_mut().item_spacing.y = 10.0;

            // 1. RUNNING PHASE
            if state.is_running {
                ui.label(RichText::new("Indexing and analyzing video metadata in place...").strong().size(15.0));

                if let Some(ref prog) = state.progress {
                    let fraction = if prog.total > 0 {
                        (prog.current as f32) / (prog.total as f32)
                    } else {
                        0.0
                    };

                    ui.add(ProgressBar::new(fraction).show_percentage());

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("File {} of {}:", prog.current, prog.total)).strong());
                        ui.label(RichText::new(&prog.current_file).monospace().color(TEXT_PRIMARY));
                    });

                    ui.horizontal(|ui| {
                        ui.label(format!("Imported: {}", prog.imported));
                        ui.separator();
                        ui.label(format!("Duplicates: {}", prog.duplicates));
                        if prog.failed > 0 {
                            ui.separator();
                            ui.label(RichText::new(format!("Failed: {}", prog.failed)).color(ACCENT_RED));
                        }
                    });

                    if let Some(ref err) = prog.last_error {
                        ui.label(RichText::new(format!("⚠️ Last Issue: {}", err)).small().color(ACCENT_ORANGE));
                    }
                } else {
                    ui.add(ProgressBar::new(0.0).text("Scanning directory tree and subfolders..."));
                }

                ui.separator();

                if ui.button("Cancel Import").clicked() {
                    action = Some(ImportDialogAction::Cancel);
                }
            }
            // 2. SUMMARY / RESULT PHASE
            else if let Some(ref summary) = state.summary {
                if summary.total == 0 {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚠️").size(24.0));
                        ui.vertical(|ui| {
                            ui.label(RichText::new("No Video Files Found").strong().size(16.0).color(ACCENT_ORANGE));
                            ui.label(RichText::new("No supported video files were discovered in the selected folder(s) or subfolders.").small());
                        });
                    });

                    ui.separator();
                    ui.label(RichText::new("Supported Formats:").strong().small());
                    ui.label(RichText::new("MP4, MOV, BRAW, MKV, MXF, AVI, WebM, ProRes, H.264, H.265 (8/10/12-bit)").small().color(TEXT_MUTED));

                    if !summary.errors.is_empty() {
                        ui.separator();
                        ui.label(RichText::new("Details:").strong().small());
                        for err in &summary.errors {
                            ui.label(RichText::new(err).small().color(ACCENT_RED));
                        }
                    }
                } else if summary.failed == 0 {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("✅").size(24.0));
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Import Completed Successfully!").strong().size(16.0).color(ACCENT_GREEN));
                            ui.label(RichText::new(format!("Successfully indexed {} video clip(s) into this catalog.", summary.imported)).small());
                        });
                    });

                    if summary.duplicates > 0 {
                        ui.label(RichText::new(format!("ℹ️ {} duplicate file(s) were already in the catalog and were skipped.", summary.duplicates)).small().color(TEXT_MUTED));
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚠️").size(24.0));
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Import Completed with Errors").strong().size(16.0).color(ACCENT_RED));
                            ui.label(RichText::new(format!("Imported: {}  |  Duplicates: {}  |  Failed: {}", summary.imported, summary.duplicates, summary.failed)).small());
                        });
                    });

                    ui.separator();
                    ui.label(RichText::new(format!("Error Reasons ({} failed item(s)):", summary.errors.len())).strong().small().color(ACCENT_RED));

                    egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                        for err in &summary.errors {
                            ui.label(RichText::new(err).small().monospace().color(ACCENT_RED));
                        }
                    });
                }

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Done").strong()).clicked() {
                        state.is_open = false;
                        state.reset();
                        action = Some(ImportDialogAction::Done);
                    }

                    if ui.button("Import More Footage...").clicked() {
                        state.reset();
                    }
                });
            }
            // 3. FILE SELECTION PHASE
            else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📁").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(RichText::new("In-Place Media Import").strong().size(15.0));
                        ui.label(
                            RichText::new("Scans folders recursively (including all subfolders). Files remain in place on your drive without copying.")
                                .small()
                                .color(TEXT_MUTED)
                        );
                    });
                });

                if let Some(ref err) = state.error_message {
                    ui.label(RichText::new(format!("⚠️ {}", err)).color(ACCENT_RED).small());
                }

                ui.add_space(4.0);
                ui.label(RichText::new("Supports MP4, MOV, BRAW, MKV, MXF, ProRes, H.264, H.265 (8/10/12-bit)").small().color(TEXT_MUTED));

                ui.horizontal(|ui| {
                    if ui.button("📁 Choose Folder...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            state.selected_paths.push(folder);
                            state.error_message = None;
                        }
                    }

                    if ui.button("📄 Choose Files...").clicked() {
                        if let Some(files) = rfd::FileDialog::new()
                            .add_filter("Video Files", crate::media::SUPPORTED_EXTENSIONS)
                            .pick_files()
                        {
                            state.selected_paths.extend(files);
                            state.error_message = None;
                        }
                    }

                    if !state.selected_paths.is_empty() && ui.button("Clear List").clicked() {
                        state.selected_paths.clear();
                        state.error_message = None;
                    }
                });

                if !state.selected_paths.is_empty() {
                    ui.separator();
                    ui.label(RichText::new(format!("Selected Source Paths ({} item(s)):", state.selected_paths.len())).small());

                    egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                        for p in &state.selected_paths {
                            ui.label(RichText::new(p.to_string_lossy().to_string()).small().monospace());
                        }
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        let start_btn = egui::Button::new(RichText::new("Start In-Place Import").strong().color(Color32::WHITE))
                            .fill(ACCENT_BLUE);

                        if ui.add(start_btn).clicked() {
                            action = Some(ImportDialogAction::StartImport(state.selected_paths.clone()));
                        }

                        if ui.button("Cancel").clicked() {
                            state.is_open = false;
                            state.reset();
                        }
                    });
                } else {
                    ui.add_space(8.0);
                    if ui.button("Close").clicked() {
                        state.is_open = false;
                        state.reset();
                    }
                }
            }
        });

    action
}
