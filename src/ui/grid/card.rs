use egui::{Color32, Rect, RichText, Rounding, Stroke, Ui, Vec2};
use crate::db::models::{PickFlag, VideoRecord};
use crate::thumbnails::ThumbnailManager;
use crate::ui::theme::*;
use crate::ui::widgets::*;

pub enum CardAction {
    Select(bool), // multi_select
    OpenPlayer,
    PlayExternal,
    RevealInExplorer,
    UpdateRating(u8),
    UpdateFlag(PickFlag),
    UpdateColor(String),
    DeleteVideo,
}

pub fn render_video_card(
    ui: &mut Ui,
    video: &VideoRecord,
    is_selected: bool,
    card_width: f32,
    thumb_mgr: &mut ThumbnailManager,
) -> Option<CardAction> {
    let mut action = None;
    let card_height = card_width * 0.78 + 72.0;
    let thumb_height = card_width * (9.0 / 16.0);

    let bg_color = if is_selected {
        BG_CARD_SELECTED
    } else {
        BG_CARD
    };

    let border_stroke = if is_selected {
        Stroke::new(1.5_f32, BORDER_CARD_SELECTED)
    } else {
        Stroke::new(1.0_f32, Color32::from_rgb(45, 48, 58))
    };

    let frame = egui::Frame::none()
        .fill(bg_color)
        .stroke(border_stroke)
        .rounding(Rounding::same(6.0))
        .inner_margin(egui::Margin::same(6.0));

    frame.show(ui, |ui| {
        ui.set_width(card_width);
        ui.set_height(card_height);

        // Thumbnail area
        let (thumb_rect, thumb_resp) = ui.allocate_exact_size(
            Vec2::new(card_width, thumb_height),
            egui::Sense::click_and_drag(),
        );

        // Request thumbnail if not loaded yet
        thumb_mgr.request(video.id, &video.file_path, video.duration_seconds);

        // Draw thumbnail or placeholder
        if let Some(tex) = thumb_mgr.get_texture(video.id) {
            ui.painter().image(
                tex.id(),
                thumb_rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            ui.painter().rect_filled(thumb_rect, Rounding::same(4.0), Color32::from_rgb(20, 22, 28));
            ui.painter().text(
                thumb_rect.center(),
                egui::Align2::CENTER_CENTER,
                "🎞️ Loading...",
                egui::FontId::proportional(11.0),
                TEXT_MUTED,
            );
        }

        // Overlay Badges on Thumbnail
        // 1. Resolution badge (Top-Left)
        let res_text = video.resolution_label();
        let res_rect = Rect::from_min_size(
            thumb_rect.min + Vec2::new(4.0, 4.0),
            Vec2::new(42.0, 16.0),
        );
        ui.painter().rect_filled(res_rect, Rounding::same(3.0), BADGE_4K_BG);
        ui.painter().text(
            res_rect.center(),
            egui::Align2::CENTER_CENTER,
            res_text,
            egui::FontId::proportional(10.0),
            BADGE_4K_FG,
        );

        // 2. Bit depth badge (Top-Right)
        if video.bit_depth >= 10 {
            let bd_text = format!("{}-bit", video.bit_depth);
            let bd_rect = Rect::from_min_size(
                egui::pos2(thumb_rect.max.x - 44.0, thumb_rect.min.y + 4.0),
                Vec2::new(40.0, 16.0),
            );
            ui.painter().rect_filled(bd_rect, Rounding::same(3.0), BADGE_10BIT_BG);
            ui.painter().text(
                bd_rect.center(),
                egui::Align2::CENTER_CENTER,
                bd_text,
                egui::FontId::proportional(10.0),
                BADGE_10BIT_FG,
            );
        }

        // 3. Duration badge (Bottom-Right)
        let dur_text = video.duration_tc();
        let dur_rect = Rect::from_min_size(
            egui::pos2(thumb_rect.max.x - 68.0, thumb_rect.max.y - 20.0),
            Vec2::new(64.0, 16.0),
        );
        ui.painter().rect_filled(dur_rect, Rounding::same(3.0), Color32::from_black_alpha(180));
        ui.painter().text(
            dur_rect.center(),
            egui::Align2::CENTER_CENTER,
            dur_text,
            egui::FontId::proportional(10.5),
            Color32::WHITE,
        );

        // Hover scrubber ratio preview
        if thumb_resp.hovered() {
            if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                let x_rel = (mouse_pos.x - thumb_rect.min.x).clamp(0.0, thumb_rect.width());
                let ratio = x_rel / thumb_rect.width();
                let hover_sec = ratio * video.duration_seconds;

                // Draw scrub indicator line
                let scrub_x = thumb_rect.min.x + x_rel;
                ui.painter().line_segment(
                    [egui::pos2(scrub_x, thumb_rect.min.y), egui::pos2(scrub_x, thumb_rect.max.y)],
                    Stroke::new(2.0_f32, ACCENT_BLUE),
                );

                // Show hover time tooltip
                thumb_resp.show_tooltip_text(format!("{:.1}s / {:.1}s", hover_sec, video.duration_seconds));
            }
        }

        // Card Click Interactions
        if thumb_resp.double_clicked() {
            action = Some(CardAction::OpenPlayer);
        } else if thumb_resp.clicked() {
            let multi = ui.input(|i| i.modifiers.ctrl || i.modifiers.command || i.modifiers.shift);
            action = Some(CardAction::Select(multi));
        }

        ui.add_space(4.0);

        // Text & Metadata details
        ui.horizontal(|ui| {
            if !video.color_label.is_empty() {
                color_dot(ui, &video.color_label);
            }
            ui.label(
                RichText::new(&video.file_name)
                    .strong()
                    .size(12.5)
                    .color(TEXT_PRIMARY),
            );
        });

        // Slate or Camera / Codec info line
        ui.horizontal(|ui| {
            if let Some(slate) = video.slate_label() {
                ui.label(RichText::new(slate).size(11.0).color(ACCENT_ORANGE));
            } else {
                let codec_str = video.codec_display_name();
                ui.label(RichText::new(codec_str).size(11.0).color(TEXT_MUTED));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(video.fps_label()).size(10.5).color(TEXT_MUTED));
            });
        });

        // Bottom row: rating stars + flag buttons
        ui.horizontal(|ui| {
            let mut rating = video.rating;
            if rating_stars(ui, &mut rating) {
                action = Some(CardAction::UpdateRating(rating));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut flag = video.flag;
                if flag_picker(ui, &mut flag) {
                    action = Some(CardAction::UpdateFlag(flag));
                }
            });
        });

        // Context Menu
        thumb_resp.context_menu(|ui| {
            if ui.button("▶ Open in Player").clicked() {
                action = Some(CardAction::OpenPlayer);
                ui.close_menu();
            }
            if ui.button("🚀 Export to DaVinci Resolve...").clicked() {
                action = Some(CardAction::Select(false));
                ui.close_menu();
            }
            if ui.button("🎬 Play in External Player").clicked() {
                action = Some(CardAction::PlayExternal);
                ui.close_menu();
            }
            if ui.button("📁 Show in File Explorer").clicked() {
                action = Some(CardAction::RevealInExplorer);
                ui.close_menu();
            }
            ui.separator();
            ui.menu_button("⭐ Rating", |ui| {
                for r in 0..=5 {
                    if ui.button(format!("{} Stars", r)).clicked() {
                        action = Some(CardAction::UpdateRating(r));
                        ui.close_menu();
                    }
                }
            });
            ui.menu_button("🎨 Color Label", |ui| {
                for c in ["None", "Red", "Orange", "Yellow", "Green", "Blue", "Purple"] {
                    let val = if c == "None" { String::new() } else { c.to_lowercase() };
                    if ui.button(c).clicked() {
                        action = Some(CardAction::UpdateColor(val));
                        ui.close_menu();
                    }
                }
            });
            ui.separator();
            if ui.button("🗑 Remove from Catalog").clicked() {
                action = Some(CardAction::DeleteVideo);
                ui.close_menu();
            }
        });
    });

    action
}
