use egui::{Color32, RichText, Window};
use crate::ui::theme::*;

pub enum DeleteChoice {
    RemoveFromCatalog,
    MoveToTrash,
    Cancel,
}

pub struct DeleteDialogState {
    pub is_open: bool,
    pub video_id: i64,
    pub file_path: String,
    pub file_name: String,
}

impl Default for DeleteDialogState {
    fn default() -> Self {
        Self {
            is_open: false,
            video_id: 0,
            file_path: String::new(),
            file_name: String::new(),
        }
    }
}

pub fn render_delete_dialog(ctx: &egui::Context, state: &mut DeleteDialogState) -> Option<DeleteChoice> {
    if !state.is_open {
        return None;
    }

    let mut choice = None;

    Window::new("Remove / Delete Video")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.set_width(380.0);
            ui.spacing_mut().item_spacing.y = 12.0;

            ui.label(RichText::new(format!("Clip: {}", state.file_name)).strong());
            ui.label(RichText::new("Choose whether to only remove this entry from your catalog, or permanently send the source video file to the system Recycle Bin:").small());

            ui.separator();

            ui.horizontal(|ui| {
                let remove_btn = egui::Button::new("Remove from Catalog Only")
                    .fill(BG_CARD);

                if ui.add(remove_btn).clicked() {
                    state.is_open = false;
                    choice = Some(DeleteChoice::RemoveFromCatalog);
                }

                let trash_btn = egui::Button::new(RichText::new("🗑 Move to Trash").color(Color32::WHITE))
                    .fill(ACCENT_RED);

                if ui.add(trash_btn).clicked() {
                    state.is_open = false;
                    choice = Some(DeleteChoice::MoveToTrash);
                }

                if ui.button("Cancel").clicked() {
                    state.is_open = false;
                    choice = Some(DeleteChoice::Cancel);
                }
            });
        });

    choice
}
