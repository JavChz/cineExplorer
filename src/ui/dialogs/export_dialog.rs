use std::path::PathBuf;
use egui::{Color32, RichText, Window};
use crate::export::{NleExportOptions, NleTarget};
use crate::ui::theme::*;

pub enum ExportDialogAction {
    ExecuteExport {
        target: NleTarget,
        options: NleExportOptions,
        output_path: PathBuf,
        use_selected_only: bool,
    },
    Cancel,
}

pub struct ExportDialogState {
    pub is_open: bool,
    pub target: NleTarget,
    pub project_name: String,
    pub include_user_tags: bool,
    pub include_tech_keywords: bool,
    pub include_slate: bool,
    pub launch_after_export: bool,
    pub use_selected_only: bool,
    pub selected_count: usize,
    pub total_count: usize,
    pub custom_output_path: Option<PathBuf>,
}

impl Default for ExportDialogState {
    fn default() -> Self {
        Self {
            is_open: false,
            target: NleTarget::DaVinciResolveFcpxml,
            project_name: "cineExplorer Export".to_string(),
            include_user_tags: true,
            include_tech_keywords: true,
            include_slate: true,
            launch_after_export: true,
            use_selected_only: false,
            selected_count: 0,
            total_count: 0,
            custom_output_path: None,
        }
    }
}

pub fn render_export_dialog(
    ctx: &egui::Context,
    state: &mut ExportDialogState,
) -> Option<ExportDialogAction> {
    if !state.is_open {
        return None;
    }

    let mut action = None;

    Window::new("Export to Video Editor (NLE)")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_width(460.0);
            ui.spacing_mut().item_spacing.y = 10.0;

            ui.label(RichText::new("Export catalog clips and metadata to your editing software:").strong());

            // Target Selector
            ui.horizontal(|ui| {
                ui.label(RichText::new("Target NLE:").strong());
                egui::ComboBox::from_id_salt("nle_target_combo")
                    .selected_text(state.target.display_name())
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut state.target,
                            NleTarget::DaVinciResolveFcpxml,
                            "DaVinci Resolve (.fcpxml) - Recommended",
                        );
                        ui.selectable_value(
                            &mut state.target,
                            NleTarget::AdobePremiereXml,
                            "Adobe Premiere Pro (.xml)",
                        );
                        ui.selectable_value(
                            &mut state.target,
                            NleTarget::DaVinciResolveCsv,
                            "DaVinci Resolve Metadata (.csv)",
                        );
                    });
            });

            // Project / Bin Name
            ui.horizontal(|ui| {
                ui.label("Project Name:");
                ui.add(egui::TextEdit::singleline(&mut state.project_name).desired_width(260.0));
            });

            ui.separator();

            // Scope Selector
            ui.label(RichText::new("Clips to Export:").strong());
            ui.horizontal(|ui| {
                if state.selected_count > 0 {
                    ui.radio_value(
                        &mut state.use_selected_only,
                        true,
                        format!("Selected clips ({} items)", state.selected_count),
                    );
                }
                ui.radio_value(
                    &mut state.use_selected_only,
                    false,
                    format!("All clips in current view ({} items)", state.total_count),
                );
            });

            ui.separator();

            // Keyword & Metadata Options
            ui.label(RichText::new("Metadata & Keywords:").strong());
            ui.checkbox(&mut state.include_user_tags, "Include user tags as Keywords");
            ui.checkbox(
                &mut state.include_tech_keywords,
                "Include tech badges as Keywords (10-bit, 4K, ProRes, BRAW, Camera)",
            );
            ui.checkbox(&mut state.include_slate, "Include Production Slate (Scene, Shot, Take, Reel)");
            ui.checkbox(&mut state.launch_after_export, "Launch in Editor immediately after export");

            ui.separator();

            // Output Path
            ui.horizontal(|ui| {
                ui.label("Save to:");
                let current_ext = state.target.extension();
                let default_name = format!("{}.{}", state.project_name.replace(' ', "_"), current_ext);

                if let Some(ref p) = state.custom_output_path {
                    ui.label(RichText::new(p.file_name().unwrap_or_default().to_string_lossy().to_string()).small().strong());
                } else {
                    ui.label(RichText::new(format!("Documents/{}", default_name)).small().color(TEXT_MUTED));
                }

                if ui.button("Browse...").clicked() {
                    if let Some(file) = rfd::FileDialog::new()
                        .set_file_name(&default_name)
                        .add_filter(state.target.display_name(), &[current_ext])
                        .save_file()
                    {
                        state.custom_output_path = Some(file);
                    }
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                let btn_text = if state.target == NleTarget::DaVinciResolveFcpxml && state.launch_after_export {
                    "🎬 Export & Open in DaVinci Resolve"
                } else {
                    "Export File"
                };

                let export_btn = egui::Button::new(
                    RichText::new(btn_text).strong().color(Color32::WHITE),
                )
                .fill(ACCENT_BLUE);

                if ui.add(export_btn).clicked() {
                    let current_ext = state.target.extension();
                    let default_name = format!("{}.{}", state.project_name.replace(' ', "_"), current_ext);
                    let final_path = state.custom_output_path.clone().unwrap_or_else(|| {
                        if let Some(doc_dir) = std::env::var_os("USERPROFILE") {
                            PathBuf::from(doc_dir).join("Documents").join(default_name)
                        } else {
                            PathBuf::from(default_name)
                        }
                    });

                    let options = NleExportOptions {
                        target: state.target,
                        project_name: state.project_name.clone(),
                        include_user_tags: state.include_user_tags,
                        include_tech_keywords: state.include_tech_keywords,
                        include_slate: state.include_slate,
                        launch_after_export: state.launch_after_export,
                    };

                    action = Some(ExportDialogAction::ExecuteExport {
                        target: state.target,
                        options,
                        output_path: final_path,
                        use_selected_only: state.use_selected_only,
                    });
                    state.is_open = false;
                }

                if ui.button("Cancel").clicked() {
                    state.is_open = false;
                    action = Some(ExportDialogAction::Cancel);
                }
            });
        });

    action
}
