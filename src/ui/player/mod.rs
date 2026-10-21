pub mod transport;

use egui::{Color32, Rect, RichText, Rounding, Ui, Vec2};
use crate::playback::VideoPlayer;
use crate::ui::theme::*;
use transport::{render_transport_bar, TransportAction};

pub fn render_player_view(ui: &mut Ui, player: &mut VideoPlayer) -> Option<TransportAction> {
    let mut action = None;

    ui.vertical(|ui| {
        // Player Header Info
        ui.horizontal(|ui| {
            ui.label(RichText::new(&player.clip.file_name).strong().size(15.0).color(TEXT_PRIMARY));
            ui.label(RichText::new(format!("· {} · {} · {}", player.clip.resolution_label(), player.clip.fps_label(), player.clip.codec_display_name())).size(12.0).color(TEXT_MUTED));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✖ Back to Grid").clicked() {
                    action = Some(TransportAction::ClosePlayer);
                }
            });
        });

        ui.add_space(4.0);

        // Video Viewport Canvas
        let avail_size = Vec2::new(
            ui.available_width(),
            (ui.available_height() - 48.0).max(100.0),
        );

        let (rect, resp) = ui.allocate_exact_size(avail_size, egui::Sense::click());
        ui.painter().rect_filled(rect, Rounding::ZERO, Color32::from_rgb(10, 11, 14));

        if resp.clicked() {
            action = Some(TransportAction::TogglePlay);
        }

        // Compute letterboxed video dimensions
        let clip_w = (player.clip.width as f32).max(1.0);
        let clip_h = (player.clip.height as f32).max(1.0);
        let aspect = clip_w / clip_h;

        let target_w = (rect.height() * aspect).min(rect.width());
        let target_h = (target_w / aspect).min(rect.height());
        let video_rect = Rect::from_center_size(rect.center(), Vec2::new(target_w, target_h));

        if let Some(ref tex) = player.current_texture {
            ui.painter().image(
                tex.id(),
                video_rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            ui.painter().text(
                video_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Loading video frame...",
                egui::FontId::proportional(14.0),
                TEXT_MUTED,
            );
        }

        ui.add_space(6.0);

        // Transport Controls Bar
        if let Some(t_act) = render_transport_bar(ui, player) {
            action = Some(t_act);
        }
    });

    action
}
