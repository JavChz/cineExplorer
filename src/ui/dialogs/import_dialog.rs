use std::path::PathBuf;
use egui::{Color32, ProgressBar, RichText, Window};
use crate::importer::ImportQueueManager;
use crate::ui::theme::*;

pub enum ImportDialogAction {
    EnqueueImport(Vec<PathBuf>),
    Cancel,
}

pub enum ImportStatusAction {
    OpenImportDialog,
}

#[derive(Default)]
pub struct ImportDialogState {
    pub is_open: bool,
    pub selected_paths: Vec<PathBuf>,
    pub error_message: Option<String>,
}

impl ImportDialogState {
    pub fn reset(&mut self) {
        self.selected_paths.clear();
        self.error_message = None;
    }
}

pub fn render_import_dialog(
    ctx: &egui::Context,
    state: &mut ImportDialogState,
    is_busy: bool,
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
            ui.label(RichText::new("Supports MP4, MOV, BRAW, MKV, MXF, AVI, WebM, ProRes, H.264, H.265 (8/10/12-bit)").small().color(TEXT_MUTED));

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
                    let btn_label = if is_busy {
                        "Add to Import Queue"
                    } else {
                        "Start In-Place Import"
                    };

                    let start_btn = egui::Button::new(RichText::new(btn_label).strong().color(Color32::WHITE))
                        .fill(ACCENT_BLUE);

                    if ui.add(start_btn).clicked() {
                        action = Some(ImportDialogAction::EnqueueImport(state.selected_paths.clone()));
                        state.is_open = false;
                        state.reset();
                    }

                    if ui.button("Cancel").clicked() {
                        state.is_open = false;
                        state.reset();
                        action = Some(ImportDialogAction::Cancel);
                    }
                });
            } else {
                ui.add_space(8.0);
                if ui.button("Close").clicked() {
                    state.is_open = false;
                    state.reset();
                }
            }
        });

    action
}

/// Floating / non-blocking Import Queue & Status Window
pub fn render_import_status_window(
    ctx: &egui::Context,
    queue_mgr: &mut ImportQueueManager,
    is_open: &mut bool,
) -> Option<ImportStatusAction> {
    if !*is_open {
        return None;
    }

    let mut action = None;
    let mut close_requested = false;

    Window::new(RichText::new(format!("{} Import Queue & Status", egui_phosphor::regular::TRAY)).strong())
        .collapsible(true)
        .resizable(true)
        .default_width(480.0)
        .default_height(340.0)
        .anchor(egui::Align2::RIGHT_TOP, [-16.0, 48.0])
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;

            let state = queue_mgr.state().clone();

            // 1. ACTIVE IMPORT JOB
            if let Some(ref active) = state.active_job {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(egui_phosphor::regular::SPINNER_GAP).color(ACCENT_BLUE).size(16.0));
                        ui.label(RichText::new(format!("Currently Importing: {}", active.label)).strong());
                    });

                    let fraction = if active.total > 0 {
                        (active.current as f32) / (active.total as f32)
                    } else {
                        0.0
                    };
                    ui.add(ProgressBar::new(fraction).show_percentage());

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("File {} of {}:", active.current, active.total)).small().strong());
                        ui.label(RichText::new(&active.current_file).small().monospace().color(TEXT_PRIMARY));
                    });

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("✓ Imported: {}", active.imported)).small().color(ACCENT_GREEN));
                        ui.separator();
                        ui.label(RichText::new(format!("ℹ Duplicates: {}", active.duplicates)).small().color(TEXT_MUTED));
                        if active.failed > 0 {
                            ui.separator();
                            ui.label(RichText::new(format!("⚠ Problems: {}", active.failed)).small().color(ACCENT_RED));
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button(RichText::new("Skip / Cancel Batch").color(ACCENT_RED)).clicked() {
                                queue_mgr.cancel_current();
                            }
                        });
                    });
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(egui_phosphor::regular::CHECK_CIRCLE).color(ACCENT_GREEN).size(15.0));
                    ui.label(RichText::new("Worker Idle — No import currently in progress").small().color(TEXT_MUTED));
                });
            }

            // 2. QUEUED JOBS
            if !state.queued_jobs.is_empty() {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Queued Batches ({})", state.queued_jobs.len())).strong().small());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(RichText::new("Clear All Queued").color(ACCENT_ORANGE)).clicked() {
                            queue_mgr.cancel_all();
                        }
                    });
                });

                egui::ScrollArea::vertical().max_height(100.0).id_salt("queued_batches_scroll").show(ui, |ui| {
                    for job in &state.queued_jobs {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(egui_phosphor::regular::HOURGLASS).color(TEXT_MUTED).size(13.0));
                            ui.label(RichText::new(&job.label).monospace().small());
                            ui.label(RichText::new(format!("({} path(s))", job.paths.len())).small().color(TEXT_MUTED));

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("✕").on_hover_text("Remove from queue").clicked() {
                                    queue_mgr.remove_queued(job.id);
                                }
                            });
                        });
                    }
                });
            }

            // 3. PROBLEMS & ERROR LOG
            if !state.problems.is_empty() {
                ui.separator();
                ui.horizontal(|ui| {
                    let prob_count = state.problems.len();
                    ui.label(RichText::new(format!("⚠️ Problems Encountered ({})", prob_count)).strong().small().color(ACCENT_RED));
                    ui.label(RichText::new("· Continued importing").small().color(TEXT_MUTED));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Clear Log").clicked() {
                            queue_mgr.clear_problems();
                        }
                    });
                });

                egui::ScrollArea::vertical().max_height(130.0).id_salt("problems_log_scroll").show(ui, |ui| {
                    for prob in state.problems.iter().rev() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&prob.timestamp).small().color(TEXT_MUTED));
                                ui.label(RichText::new(&prob.file_name).strong().small().color(ACCENT_ORANGE));
                                ui.label(RichText::new(format!("[{}]", prob.batch_label)).small().color(TEXT_MUTED));
                            });
                            ui.label(RichText::new(&prob.reason).small().monospace().color(ACCENT_RED));
                        });
                    }
                });
            }

            // 4. FOOTER & ACTIONS
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button(format!("{} Add Files / Folders to Queue...", egui_phosphor::regular::PLUS)).clicked() {
                    action = Some(ImportStatusAction::OpenImportDialog);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });
            });
        });

    if close_requested {
        *is_open = false;
        queue_mgr.mark_problems_seen();
    }

    action
}
