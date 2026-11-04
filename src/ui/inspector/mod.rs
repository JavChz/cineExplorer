pub mod sections;

use egui::{RichText, ScrollArea, Ui, Vec2};
use crate::db::models::VideoRecord;
use crate::thumbnails::ThumbnailManager;
use crate::ui::theme::*;
use sections::{render_inspector_content, InspectorAction};

#[allow(clippy::too_many_arguments)]
pub fn render_inspector(
    ui: &mut Ui,
    selected_video: Option<&VideoRecord>,
    is_player_mode: bool,
    thumb_mgr: &mut ThumbnailManager,
    edit_scene: &mut String,
    edit_shot: &mut String,
    edit_take: &mut String,
    edit_reel: &mut String,
    edit_location: &mut String,
    edit_unit: &mut String,
    edit_notes: &mut String,
    new_clip_tag: &mut String,
) -> Option<InspectorAction> {
    let mut header_action = None;

    // Docked Panel Header
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("METADATA INSPECTOR")
                .size(11.0)
                .strong()
                .color(TEXT_MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close_btn = egui::Button::new(
                RichText::new(egui_phosphor::regular::CARET_DOUBLE_RIGHT)
                    .size(11.5)
                    .color(TEXT_MUTED),
            )
            .frame(false)
            .min_size(Vec2::new(18.0, 18.0));
            if ui
                .add(close_btn)
                .on_hover_text("Collapse Inspector (Ctrl+J)")
                .clicked()
            {
                header_action = Some(InspectorAction::CloseInspector);
            }
        });
    });
    ui.separator();

    if header_action.is_some() {
        return header_action;
    }

    match selected_video {
        Some(video) => {
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    render_inspector_content(
                        ui,
                        video,
                        is_player_mode,
                        thumb_mgr,
                        edit_scene,
                        edit_shot,
                        edit_take,
                        edit_reel,
                        edit_location,
                        edit_unit,
                        edit_notes,
                        new_clip_tag,
                    )
                })
                .inner
        }
        None => {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.label(
                        RichText::new(egui_phosphor::regular::SLIDERS)
                            .size(32.0)
                            .color(TEXT_MUTED),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("No Clip Selected")
                            .strong()
                            .size(13.0)
                            .color(TEXT_SECONDARY),
                    );
                    ui.label(
                        RichText::new("Select a video clip to inspect and log metadata")
                            .size(11.5)
                            .color(TEXT_MUTED),
                    );
                });
            });
            None
        }
    }
}
