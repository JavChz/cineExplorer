use std::path::PathBuf;
use egui::{Color32, ProgressBar, RichText, Window};
use chrono::Local;
use crate::ingester::{IngestConfig, IngestItem, IngestPhase, IngestProgress, IngestSummary, Ingester};
use crate::ui::theme::*;

pub enum IngestDialogAction {
    StartIngest(IngestConfig),
    Cancel,
    Done,
}

pub struct IngestDialogState {
    pub is_open: bool,
    pub source_paths: Vec<PathBuf>,
    pub destination_dir: Option<PathBuf>,
    pub subfolder: String,
    pub preserve_structure: bool,
    pub is_running: bool,
    pub progress: Option<IngestProgress>,
    pub summary: Option<IngestSummary>,
    pub cached_items: Vec<IngestItem>,
    pub cached_total_bytes: u64,
}

impl Default for IngestDialogState {
    fn default() -> Self {
        Self {
            is_open: false,
            source_paths: Vec::new(),
            destination_dir: None,
            subfolder: String::new(),
            preserve_structure: true,
            is_running: false,
            progress: None,
            summary: None,
            cached_items: Vec::new(),
            cached_total_bytes: 0,
        }
    }
}

impl IngestDialogState {
    pub fn reset(&mut self) {
        self.source_paths.clear();
        self.subfolder.clear();
        self.is_running = false;
        self.progress = None;
        self.summary = None;
        self.cached_items.clear();
        self.cached_total_bytes = 0;
    }

    pub fn refresh_source_cache(&mut self) {
        if self.source_paths.is_empty() {
            self.cached_items.clear();
            self.cached_total_bytes = 0;
            return;
        }

        let dummy_dest = self.destination_dir.clone().unwrap_or_else(|| PathBuf::from("."));
        let config = IngestConfig {
            source_paths: self.source_paths.clone(),
            destination_dir: dummy_dest,
            subfolder: if self.subfolder.trim().is_empty() { None } else { Some(self.subfolder.clone()) },
            preserve_structure: self.preserve_structure,
        };

        self.cached_items = Ingester::prepare_items(&config);
        self.cached_total_bytes = Ingester::calculate_total_bytes(&self.cached_items);
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_eta(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {:02}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {:02}m", seconds / 3600, (seconds % 3600) / 60)
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

    Window::new("Ingest Footage (Offload from SD Card / Media)")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_width(540.0);
            ui.spacing_mut().item_spacing.y = 10.0;

            // 1. RUNNING PHASE
            if state.is_running {
                let phase_label = match state.progress.as_ref().map(|p| &p.phase) {
                    Some(IngestPhase::Scanning) => "Scanning source media...",
                    Some(IngestPhase::Copying) => "Offloading & copying video files...",
                    Some(IngestPhase::Indexing) => "Analyzing metadata & extracting thumbnails...",
                    None => "Initializing ingest...",
                };

                ui.label(RichText::new(phase_label).strong().size(15.0));

                if let Some(ref prog) = state.progress {
                    let fraction = if prog.total_bytes > 0 {
                        (prog.total_bytes_copied as f32) / (prog.total_bytes as f32)
                    } else if prog.total_files > 0 {
                        (prog.current_file_index as f32) / (prog.total_files as f32)
                    } else {
                        0.0
                    };

                    ui.add(ProgressBar::new(fraction).show_percentage());

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("File {}/{}:", prog.current_file_index, prog.total_files)).strong());
                        ui.label(RichText::new(&prog.current_file).monospace().color(TEXT_PRIMARY));
                    });

                    ui.horizontal(|ui| {
                        let transferred_str = format!(
                            "Transferred: {} / {}",
                            format_bytes(prog.total_bytes_copied),
                            format_bytes(prog.total_bytes)
                        );
                        ui.label(RichText::new(transferred_str).small().color(TEXT_MUTED));

                        if prog.speed_mbps > 0.05 {
                            ui.separator();
                            ui.label(RichText::new(format!("Speed: {:.1} MB/s", prog.speed_mbps)).small().color(ACCENT_BLUE));
                        }

                        if let Some(eta) = prog.eta_seconds {
                            ui.separator();
                            ui.label(RichText::new(format!("ETA: {}", format_eta(eta))).small().color(TEXT_MUTED));
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(format!("Copied & Cataloged: {}", prog.copied));
                        if prog.duplicates > 0 {
                            ui.label(format!("Duplicates: {}", prog.duplicates));
                        }
                        if prog.failed > 0 {
                            ui.label(RichText::new(format!("Failed: {}", prog.failed)).color(ACCENT_RED));
                        }
                    });

                    if let Some(ref err) = prog.last_error {
                        ui.label(RichText::new(format!("⚠️ Last Issue: {}", err)).small().color(ACCENT_ORANGE));
                    }
                } else {
                    ui.add(ProgressBar::new(0.0).text("Preparing transfer..."));
                }

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Hide / Run in Background").on_hover_text("Keep offloading in background and return to catalog").clicked() {
                        state.is_open = false;
                    }

                    if ui.button(RichText::new("Cancel Ingest").color(ACCENT_RED)).clicked() {
                        action = Some(IngestDialogAction::Cancel);
                    }
                });
            }
            // 2. SUMMARY PHASE
            else if let Some(ref summary) = state.summary {
                if summary.total_files == 0 {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚠️").size(24.0));
                        ui.vertical(|ui| {
                            ui.label(RichText::new("No Video Clips Found").strong().size(16.0).color(ACCENT_ORANGE));
                            ui.label(RichText::new("No supported video files were discovered in the selected source.").small());
                        });
                    });

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
                            ui.label(RichText::new("Ingest Completed Successfully!").strong().size(16.0).color(ACCENT_GREEN));
                            ui.label(RichText::new(format!("Copied & cataloged {} video clips ({})", summary.copied, format_bytes(summary.total_bytes))).small());
                        });
                    });

                    if summary.duplicates > 0 {
                        ui.label(RichText::new(format!("ℹ️ {} duplicate files already existed at destination.", summary.duplicates)).small().color(TEXT_MUTED));
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚠️").size(24.0));
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Ingest Finished with Errors").strong().size(16.0).color(ACCENT_RED));
                            ui.label(RichText::new(format!("Copied: {}  |  Duplicates: {}  |  Failed: {}", summary.copied, summary.duplicates, summary.failed)).small());
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
                        action = Some(IngestDialogAction::Done);
                    }

                    if ui.button("Ingest More Media...").clicked() {
                        state.reset();
                    }
                });
            }
            // 3. SELECTION PHASE
            else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📥").size(24.0));
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Offload from SD Card / External Media").strong().size(15.0));
                        ui.label(
                            RichText::new("Recursively copies clips from your card to local HDD/SSD, and automatically indexes them into this catalog.")
                                .small()
                                .color(TEXT_MUTED)
                        );
                    });
                });

                ui.separator();

                // 1. SOURCE SELECTION
                ui.label(RichText::new("1. Source (SD Card / Camera Media)").strong());

                ui.horizontal(|ui| {
                    if ui.button("📁 Choose SD Card / Folder...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            state.source_paths.push(folder);
                            state.refresh_source_cache();
                        }
                    }

                    if ui.button("📄 Choose Video Files...").clicked() {
                        if let Some(files) = rfd::FileDialog::new()
                            .add_filter("Video Files", crate::media::SUPPORTED_EXTENSIONS)
                            .pick_files()
                        {
                            state.source_paths.extend(files);
                            state.refresh_source_cache();
                        }
                    }

                    if !state.source_paths.is_empty() && ui.button("Clear").clicked() {
                        state.source_paths.clear();
                        state.refresh_source_cache();
                    }
                });

                if !state.source_paths.is_empty() {
                    egui::ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
                        for p in &state.source_paths {
                            ui.label(RichText::new(p.to_string_lossy().to_string()).small().monospace());
                        }
                    });

                    let summary_text = format!(
                        "⚡ Found {} video clip(s) to offload ({})",
                        state.cached_items.len(),
                        format_bytes(state.cached_total_bytes)
                    );
                    ui.label(RichText::new(summary_text).small().color(ACCENT_GREEN));
                } else {
                    ui.label(RichText::new("No source selected. Choose your SD card or camera card folder.").small().color(TEXT_MUTED));
                }

                ui.separator();

                // 2. DESTINATION SELECTION
                ui.label(RichText::new("2. Destination (Local HDD / SSD Storage)").strong());

                ui.horizontal(|ui| {
                    if ui.button("📁 Choose Destination Folder...").clicked() {
                        if let Some(dest) = rfd::FileDialog::new().pick_folder() {
                            state.destination_dir = Some(dest);
                            state.refresh_source_cache();
                        }
                    }

                    if let Some(ref dest) = state.destination_dir {
                        ui.label(RichText::new(dest.to_string_lossy().to_string()).monospace().small());
                    } else {
                        ui.label(RichText::new("Select a target folder on your HDD/SSD").small().color(ACCENT_ORANGE));
                    }
                });

                // Optional Subfolder & Structure options
                ui.horizontal(|ui| {
                    ui.label("Subfolder:");
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut state.subfolder)
                            .hint_text("e.g. Card_01 or Shoot_Day1")
                            .desired_width(180.0)
                    );
                    if edit.changed() {
                        state.refresh_source_cache();
                    }

                    if ui.button("📅 Today's Date").clicked() {
                        let today = Local::now().format("%Y-%m-%d").to_string();
                        state.subfolder = today;
                        state.refresh_source_cache();
                    }
                });

                ui.checkbox(&mut state.preserve_structure, "Preserve card folder structure");

                ui.separator();

                // ACTION BUTTONS
                ui.horizontal(|ui| {
                    let can_start = !state.cached_items.is_empty() && state.destination_dir.is_some();

                    let start_btn = egui::Button::new(
                        RichText::new("Start Ingest (Copy & Catalog)")
                            .strong()
                            .color(if can_start { Color32::WHITE } else { TEXT_MUTED })
                    )
                    .fill(if can_start { ACCENT_BLUE } else { BG_CARD });

                    if ui.add_enabled(can_start, start_btn).clicked() {
                        if let Some(ref dest) = state.destination_dir {
                            let config = IngestConfig {
                                source_paths: state.source_paths.clone(),
                                destination_dir: dest.clone(),
                                subfolder: if state.subfolder.trim().is_empty() { None } else { Some(state.subfolder.clone()) },
                                preserve_structure: state.preserve_structure,
                            };
                            action = Some(IngestDialogAction::StartIngest(config));
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        state.is_open = false;
                        state.reset();
                    }
                });
            }
        });

    action
}
