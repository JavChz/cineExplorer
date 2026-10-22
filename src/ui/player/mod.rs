pub mod transport;

use egui::{Color32, Rect, RichText, Rounding, Ui, Vec2};
use crate::playback::VideoPlayer;
use crate::ui::theme::*;
use transport::{render_transport_bar, TransportAction};

pub fn render_player_view(
    ui: &mut Ui,
    player: &mut VideoPlayer,
    clip_index: usize,
    total_clips: usize,
) -> Option<TransportAction> {
    let mut action = None;

    ui.vertical(|ui| {
        // Player Header Info
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&player.clip.file_name)
                    .strong()
                    .size(14.0)
                    .color(TEXT_PRIMARY),
            );

            let spec_summary = format!(
                "· {} · {} · {}",
                player.clip.resolution_label(),
                player.clip.fps_label(),
                player.clip.codec_display_name()
            );
            ui.label(RichText::new(spec_summary).size(11.5).color(TEXT_MUTED));

            if player.clip.bit_depth >= 10 {
                ui.label(
                    RichText::new(format!("{}-bit", player.clip.bit_depth))
                        .small()
                        .color(BADGE_10BIT_FG),
                );
            }

            // Log Profile and Rec.709 LUT Controls
            let is_log = player.log_profile.is_log();
            if is_log {
                let badge_text = format!("{} {}", egui_phosphor::regular::PALETTE, player.log_profile.short_name());
                ui.label(RichText::new(badge_text).small().strong().color(BADGE_10BIT_FG));

                let (lut_icon, lut_color, lut_text) = if player.apply_rec709_lut {
                    (egui_phosphor::fill::CHECK_CIRCLE, ACCENT_GREEN, "Rec.709")
                } else {
                    (egui_phosphor::regular::CIRCLE, ACCENT_ORANGE, "Raw Log")
                };
                let lut_btn = egui::Button::new(
                    RichText::new(format!("{} {}", lut_icon, lut_text)).small().strong().color(lut_color),
                );
                let tooltip = if player.apply_rec709_lut {
                    "Rec.709 3D LUT is Active (Alt+L) — click to view ungraded Log"
                } else {
                    "Raw Log displayed (Alt+L) — click to apply Rec.709 LUT"
                };
                if ui.add(lut_btn).on_hover_text(tooltip).clicked() {
                    action = Some(TransportAction::ToggleRec709Lut);
                }
            }

            // Profile selector dropdown
            let profile_label = if is_log {
                player.log_profile.short_name()
            } else {
                "SDR"
            };
            egui::ComboBox::from_id_salt("player_log_profile_select")
                .selected_text(
                    RichText::new(profile_label)
                        .small()
                        .color(if is_log { ACCENT_BLUE } else { TEXT_MUTED }),
                )
                .show_ui(ui, |ui| {
                    for opt in crate::media::color_lut::LogProfile::all_options() {
                        let is_sel = player.log_profile == *opt;
                        if ui.selectable_label(is_sel, opt.display_name()).clicked() {
                            action = Some(TransportAction::SetLogProfile(*opt));
                        }
                    }
                });

            if player.is_gpu_accelerated {
                ui.label(
                    RichText::new(format!("{} GPU Accelerated", egui_phosphor::regular::LIGHTNING))
                        .small()
                        .color(ACCENT_GREEN),
                );
            } else {
                ui.label(
                    RichText::new(format!("{} Multi-Threaded · wgpu", egui_phosphor::regular::CPU))
                        .small()
                        .color(ACCENT_BLUE),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let back_btn = egui::Button::new(
                    RichText::new(format!("{} Back to Grid", egui_phosphor::regular::ARROW_LEFT))
                        .size(12.0)
                        .color(TEXT_PRIMARY),
                );
                if ui.add(back_btn).on_hover_text("Back to Grid View (Esc / G / Tab)").clicked() {
                    action = Some(TransportAction::ClosePlayer);
                }
            });
        });

        ui.add_space(2.0);

        // Video Viewport Canvas
        let avail_size = Vec2::new(
            ui.available_width(),
            (ui.available_height() - 62.0).max(100.0),
        );

        let (rect, resp) = ui.allocate_exact_size(avail_size, egui::Sense::click());
        ui.painter().rect_filled(rect, Rounding::same(4.0), Color32::from_rgb(10, 11, 14));

        if resp.double_clicked() {
            action = Some(TransportAction::ClosePlayer);
        } else if resp.clicked() {
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
                format!("{} Loading video frame...", egui_phosphor::regular::FILM_STRIP),
                egui::FontId::proportional(13.0),
                TEXT_MUTED,
            );
        }

        ui.add_space(4.0);

        // Transport Controls Bar
        if let Some(t_act) = render_transport_bar(ui, player, clip_index, total_clips) {
            action = Some(t_act);
        }
    });

    action
}