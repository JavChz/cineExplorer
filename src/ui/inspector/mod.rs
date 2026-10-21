pub mod sections;

use egui::{ScrollArea, Ui};
use crate::db::models::VideoRecord;
use crate::thumbnails::ThumbnailManager;
use sections::{render_inspector_content, InspectorAction};

pub fn render_inspector(
    ui: &mut Ui,
    selected_video: Option<&VideoRecord>,
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
    match selected_video {
        Some(video) => {
            ScrollArea::vertical().show(ui, |ui| {
                render_inspector_content(
                    ui,
                    video,
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
            }).inner
        }
        None => {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Select a video clip to inspect metadata")
                        .color(egui::Color32::from_rgb(120, 120, 120)),
                );
            });
            None
        }
    }
}
