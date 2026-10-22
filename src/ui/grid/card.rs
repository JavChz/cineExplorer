use egui::{Color32, Rect, RichText, Rounding, Stroke, Ui, Vec2};
use crate::db::models::{PickFlag, VideoRecord};
use crate::thumbnails::ThumbnailManager;
use crate::ui::theme::*;
use crate::ui::widgets::*;

pub enum CardAction {
    Select { toggle: bool, range: bool },
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
    let card_height = card_width * 0.78 + 68.0;
    let thumb_height = card_width * (9.0 / 16.0);

    let bg_color = if is_selected {
        BG_CARD_SELECTED
    } else {
        BG_CARD
    };

    let border_stroke = if is_selected {
        Stroke::new(1.5_f32, BORDER_CARD_SELECTED)
    } else if video.flag == PickFlag::Picked {
        Stroke::new(1.0_f32, Color32::from_rgb(40, 110, 60))
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
        thumb_mgr.request(video.id, &video.file_path, video.duration_seconds, Some(&video.log_profile));

        // Draw thumbnail with reject dimming if rejected
        let img_tint = if video.flag == PickFlag::Rejected {
            Color32::from_white_alpha(130)
        } else {
            Color32::WHITE
        };

        if let Some(tex) = thumb_mgr.get_texture(video.id) {
            ui.painter().image(
                tex.id(),
                thumb_rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                img_tint,
            );
        } else {
            ui.painter().rect_filled(thumb_rect, Rounding::same(4.0), Color32::from_rgb(20, 22, 28));
            ui.painter().text(
                thumb_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("{} Loading...", egui_phosphor::regular::FILM_STRIP),
                egui::FontId::proportional(11.0),
                TEXT_MUTED,
            );
        }

        // Overlay Badges on Thumbnail
        // 1. Flag badge or Resolution badge (Top-Left)
        if video.flag == PickFlag::Picked {
            let flag_rect = Rect::from_min_size(
                thumb_rect.min + Vec2::new(4.0, 4.0),
                Vec2::new(20.0, 16.0),
            );
            ui.painter().rect_filled(flag_rect, Rounding::same(3.0), Color32::from_rgb(25, 75, 40));
            ui.painter().text(
                flag_rect.center(),
                egui::Align2::CENTER_CENTER,
                egui_phosphor::fill::FLAG,
                egui::FontId::proportional(11.0),
                ACCENT_GREEN,
            );
        } else if video.flag == PickFlag::Rejected {
            let flag_rect = Rect::from_min_size(
                thumb_rect.min + Vec2::new(4.0, 4.0),
                Vec2::new(20.0, 16.0),
            );
            ui.painter().rect_filled(flag_rect, Rounding::same(3.0), Color32::from_rgb(75, 25, 25));
            ui.painter().text(
                flag_rect.center(),
                egui::Align2::CENTER_CENTER,
                egui_phosphor::regular::X_CIRCLE,
                egui::FontId::proportional(11.0),
                ACCENT_RED,
            );
        }

        // Resolution badge (next to flag or top-left)
        let res_text = video.resolution_label();
        let res_offset_x = if video.flag != PickFlag::Unflagged { 28.0 } else { 4.0 };
        let res_rect = Rect::from_min_size(
            thumb_rect.min + Vec2::new(res_offset_x, 4.0),
            Vec2::new(40.0, 16.0),
        );
        ui.painter().rect_filled(res_rect, Rounding::same(3.0), BADGE_4K_BG);
        ui.painter().text(
            res_rect.center(),
            egui::Align2::CENTER_CENTER,
            res_text,
            egui::FontId::proportional(10.0),
            BADGE_4K_FG,
        );

        // 2. Bit depth & Log profile badges (Top-Right)
        let mut tr_offset_x = 44.0;
        if video.bit_depth >= 10 {
            let bd_text = format!("{}-bit", video.bit_depth);
            let bd_rect = Rect::from_min_size(
                egui::pos2(thumb_rect.max.x - tr_offset_x, thumb_rect.min.y + 4.0),
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
            tr_offset_x += 44.0;
        }

        if !video.log_profile.is_empty() && video.log_profile.to_lowercase() != "none" {
            let short = crate::media::color_lut::LogProfile::from_str(&video.log_profile).short_name();
            let log_text = if !short.is_empty() { short } else { &video.log_profile };
            let width = (log_text.len() as f32 * 6.5 + 10.0).max(40.0);
            let log_rect = Rect::from_min_size(
                egui::pos2(thumb_rect.max.x - tr_offset_x - width + 40.0, thumb_rect.min.y + 4.0),
                Vec2::new(width, 16.0),
            );
            ui.painter().rect_filled(log_rect, Rounding::same(3.0), Color32::from_rgb(45, 30, 75));
            ui.painter().text(
                log_rect.center(),
                egui::Align2::CENTER_CENTER,
                log_text,
                egui::FontId::proportional(10.0),
                Color32::from_rgb(215, 175, 255),
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

                // Show formatted timecode in tooltip
                let h = (hover_sec / 3600.0) as u32;
                let m = ((hover_sec % 3600.0) / 60.0) as u32;
                let s = (hover_sec % 60.0) as u32;
                let f = ((hover_sec.fract()) * video.fps.max(1.0)) as u32;
                let tc = if h > 0 {
                    format!("{:02}:{:02}:{:02}:{:02}", h, m, s, f)
                } else {
                    format!("{:02}:{:02}:{:02}", m, s, f)
                };
                thumb_resp.show_tooltip_text(format!("{} / {}", tc, video.duration_tc()));
            }
        }

        // Card Click Interactions
        if thumb_resp.double_clicked() {
            action = Some(CardAction::OpenPlayer);
        } else if thumb_resp.clicked() {
            let toggle = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
            let range = ui.input(|i| i.modifiers.shift);
            action = Some(CardAction::Select { toggle, range });
        }

        ui.add_space(4.0);

        // Text & Metadata details
        ui.horizontal(|ui| {
            if !video.color_label.is_empty() {
                color_dot(ui, &video.color_label);
            }
            let label = ui.add(
                egui::Label::new(
                    RichText::new(&video.file_name)
                        .strong()
                        .size(12.0)
                        .color(if video.flag == PickFlag::Rejected { TEXT_MUTED } else { TEXT_PRIMARY }),
                )
                .truncate()
            );
            label.on_hover_text(&video.file_name);
        });

        // Slate or Camera / Codec info line
        ui.horizontal(|ui| {
            if let Some(slate) = video.slate_label() {
                ui.label(RichText::new(slate).size(10.5).color(ACCENT_ORANGE));
            } else if !video.camera_model.is_empty() {
                let text = if !video.focal_length.is_empty() {
                    format!("{} · {}", video.camera_model, video.focal_length)
                } else if !video.lens_model.is_empty() {
                    format!("{} · {}", video.camera_model, video.lens_model)
                } else {
                    format!("{} · {}", video.camera_model, video.codec_display_name())
                };
                ui.add(egui::Label::new(RichText::new(text).size(10.5).color(TEXT_MUTED)).truncate());
            } else {
                let codec_str = video.codec_display_name();
                ui.label(RichText::new(codec_str).size(10.5).color(TEXT_MUTED));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(video.fps_label()).size(10.0).color(TEXT_MUTED));
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
            if ui.button(format!("{} Open in Player (Enter)", egui_phosphor::regular::PLAY_CIRCLE)).clicked() {
                action = Some(CardAction::OpenPlayer);
                ui.close_menu();
            }
            if ui.button(format!("{} Export to NLE... (Ctrl+E)", egui_phosphor::regular::EXPORT)).clicked() {
                action = Some(CardAction::Select { toggle: false, range: false });
                ui.close_menu();
            }
            if ui.button(format!("{} Play in External Player", egui_phosphor::regular::ARROW_SQUARE_OUT)).clicked() {
                action = Some(CardAction::PlayExternal);
                ui.close_menu();
            }
            if ui.button(format!("{} Reveal in File Explorer", egui_phosphor::regular::FOLDER_OPEN)).clicked() {
                action = Some(CardAction::RevealInExplorer);
                ui.close_menu();
            }
            ui.separator();
            ui.menu_button(format!("{} Star Rating (1-5, 0)", egui_phosphor::regular::STAR), |ui| {
                for r in 0..=5 {
                    let label = if r == 0 { "Clear Rating (0)".to_string() } else { format!("{} Stars ({})", r, r) };
                    if ui.button(label).clicked() {
                        action = Some(CardAction::UpdateRating(r));
                        ui.close_menu();
                    }
                }
            });
            ui.menu_button(format!("{} Flag Status (P / X / U)", egui_phosphor::regular::FLAG), |ui| {
                if ui.button("Pick (P)").clicked() {
                    action = Some(CardAction::UpdateFlag(PickFlag::Picked));
                    ui.close_menu();
                }
                if ui.button("Reject (X)").clicked() {
                    action = Some(CardAction::UpdateFlag(PickFlag::Rejected));
                    ui.close_menu();
                }
                if ui.button("Unflagged (U)").clicked() {
                    action = Some(CardAction::UpdateFlag(PickFlag::Unflagged));
                    ui.close_menu();
                }
            });
            ui.menu_button(format!("{} Color Label", egui_phosphor::regular::PALETTE), |ui| {
                for c in ["None", "Red", "Orange", "Yellow", "Green", "Blue", "Purple"] {
                    let val = if c == "None" { String::new() } else { c.to_lowercase() };
                    if ui.button(c).clicked() {
                        action = Some(CardAction::UpdateColor(val));
                        ui.close_menu();
                    }
                }
            });
            ui.separator();
            if ui.button(format!("{} Remove from Catalog (Del)", egui_phosphor::regular::TRASH)).clicked() {
                action = Some(CardAction::DeleteVideo);
                ui.close_menu();
            }
        });
    });

    action
}