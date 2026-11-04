use egui::{Color32, Pos2, Rect, RichText, Rounding, Stroke, Ui, Vec2};
use crate::db::models::{PickFlag, VideoRecord};
use crate::thumbnails::ThumbnailManager;
use crate::ui::theme::*;
use crate::ui::widgets::*;

pub enum CardAction {
    Select { toggle: bool, range: bool },
    OpenPlayer,
    ExportSelected,
    PlayExternal,
    RevealInExplorer,
    RegenerateThumbnail,
    UpdateRating(u8),
    UpdateFlag(PickFlag),
    UpdateColor(String),
    DeleteVideo,
}

fn draw_overlay_pill(
    painter: &egui::Painter,
    pos: Pos2,
    align_right: bool,
    text: &str,
    font_id: egui::FontId,
    bg: Color32,
    fg: Color32,
) -> f32 {
    let pad = Vec2::new(5.0, 2.0);
    let galley = painter.layout_no_wrap(text.to_string(), font_id, fg);
    let pill_w = galley.size().x + pad.x * 2.0;
    let pill_h = 16.0;
    let min_x = if align_right { pos.x - pill_w } else { pos.x };
    let rect = Rect::from_min_size(egui::pos2(min_x, pos.y), Vec2::new(pill_w, pill_h));
    painter.rect_filled(rect, Rounding::same(3.0), bg);
    let text_y = rect.center().y - galley.size().y * 0.5;
    painter.galley(egui::pos2(rect.min.x + pad.x, text_y), galley, fg);
    pill_w
}

pub fn render_video_card(
    ui: &mut Ui,
    video: &VideoRecord,
    is_selected: bool,
    card_width: f32,
    thumb_mgr: &mut ThumbnailManager,
) -> Option<CardAction> {
    let mut action = None;

    let pad = 7.0;
    let inner_w = (card_width - pad * 2.0).max(136.0);
    let thumb_h = (inner_w * (9.0 / 16.0)).round();
    let footer_h = 64.0;
    let card_height = pad * 2.0 + thumb_h + footer_h;

    // Allocate exact outer card rectangle so every card in the grid has identical width & height
    let (card_rect, card_resp) = ui.allocate_exact_size(
        Vec2::new(card_width, card_height),
        egui::Sense::click(),
    );

    let is_hovered = ui.rect_contains_pointer(card_rect);

    let bg_color = if is_selected {
        BG_CARD_SELECTED
    } else if is_hovered {
        BG_CARD_HOVER
    } else {
        BG_CARD
    };

    let border_stroke = if is_selected {
        Stroke::new(1.5_f32, BORDER_CARD_SELECTED)
    } else if video.flag == PickFlag::Picked {
        Stroke::new(1.2_f32, Color32::from_rgb(42, 125, 68))
    } else if video.flag == PickFlag::Rejected {
        Stroke::new(1.0_f32, Color32::from_rgb(95, 38, 38))
    } else if is_hovered {
        Stroke::new(1.0_f32, Color32::from_rgb(65, 72, 88))
    } else {
        Stroke::new(1.0_f32, BORDER_CARD)
    };

    ui.painter().rect_filled(card_rect, Rounding::same(6.0), bg_color);
    ui.painter().rect_stroke(card_rect, Rounding::same(6.0), border_stroke);

    let inner_rect = card_rect.shrink(pad);
    let mut child_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child_ui.set_clip_rect(card_rect.shrink(2.0));

    // 1. 16:9 THUMBNAIL VIEWPORT
    let (thumb_rect, thumb_resp) = child_ui.allocate_exact_size(
        Vec2::new(inner_w, thumb_h),
        egui::Sense::click_and_drag(),
    );

    thumb_mgr.request(
        video.id,
        &video.file_path,
        video.duration_seconds,
        Some(&video.log_profile),
    );

    // Dark studio backdrop for thumbnail
    child_ui.painter().rect_filled(
        thumb_rect,
        Rounding::same(4.0),
        Color32::from_rgb(12, 14, 18),
    );

    let img_tint = if video.flag == PickFlag::Rejected {
        Color32::from_white_alpha(115)
    } else {
        Color32::WHITE
    };

    if let Some(tex) = thumb_mgr.get_texture(video.id) {
        child_ui.painter().image(
            tex.id(),
            thumb_rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            img_tint,
        );
    } else {
        child_ui.painter().text(
            thumb_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{} Loading...", egui_phosphor::regular::FILM_STRIP),
            egui::FontId::proportional(11.0),
            TEXT_MUTED,
        );
    }

    // Subtle inner border around thumbnail
    child_ui.painter().rect_stroke(
        thumb_rect,
        Rounding::same(4.0),
        Stroke::new(1.0_f32, Color32::from_black_alpha(110)),
    );

    // OVERLAY BADGES ON THUMBNAIL
    let painter = child_ui.painter();
    let mut tl_x = thumb_rect.min.x + 5.0;
    let top_y = thumb_rect.min.y + 5.0;

    // Top-Left: Flag badge (Pick / Reject)
    if video.flag == PickFlag::Picked {
        let flag_rect = Rect::from_min_size(egui::pos2(tl_x, top_y), Vec2::new(20.0, 16.0));
        painter.rect_filled(flag_rect, Rounding::same(3.0), Color32::from_rgba_unmultiplied(20, 75, 38, 230));
        painter.text(
            flag_rect.center(),
            egui::Align2::CENTER_CENTER,
            egui_phosphor::fill::FLAG,
            phosphor_fill_font(10.5),
            ACCENT_GREEN,
        );
        tl_x += 23.0;
    } else if video.flag == PickFlag::Rejected {
        let flag_rect = Rect::from_min_size(egui::pos2(tl_x, top_y), Vec2::new(20.0, 16.0));
        painter.rect_filled(flag_rect, Rounding::same(3.0), Color32::from_rgba_unmultiplied(85, 22, 24, 230));
        painter.text(
            flag_rect.center(),
            egui::Align2::CENTER_CENTER,
            egui_phosphor::regular::X,
            egui::FontId::proportional(11.0),
            ACCENT_RED,
        );
        tl_x += 23.0;
    }

    // Top-Left: Resolution badge
    let res_text = video.resolution_label();
    let _ = draw_overlay_pill(
        painter,
        egui::pos2(tl_x, top_y),
        false,
        &res_text,
        egui::FontId::proportional(10.0),
        Color32::from_rgba_unmultiplied(18, 48, 58, 225),
        BADGE_4K_FG,
    );

    // Top-Right: Bit Depth & Log Profile badges
    let mut tr_x = thumb_rect.max.x - 5.0;
    if video.bit_depth >= 10 {
        let bd_text = format!("{}-bit", video.bit_depth);
        let w = draw_overlay_pill(
            painter,
            egui::pos2(tr_x, top_y),
            true,
            &bd_text,
            egui::FontId::proportional(10.0),
            Color32::from_rgba_unmultiplied(40, 30, 80, 225),
            BADGE_10BIT_FG,
        );
        tr_x -= w + 4.0;
    }

    if !video.log_profile.is_empty() && !video.log_profile.eq_ignore_ascii_case("none") {
        let short = crate::media::color_lut::LogProfile::from_str(&video.log_profile).short_name();
        let log_text = if !short.is_empty() { short } else { &video.log_profile };
        let _ = draw_overlay_pill(
            painter,
            egui::pos2(tr_x, top_y),
            true,
            log_text,
            egui::FontId::proportional(10.0),
            Color32::from_rgba_unmultiplied(48, 30, 78, 225),
            Color32::from_rgb(220, 180, 255),
        );
    }

    // Bottom-Left: FPS pill
    let bot_y = thumb_rect.max.y - 21.0;
    if video.fps > 0.0 {
        let _ = draw_overlay_pill(
            painter,
            egui::pos2(thumb_rect.min.x + 5.0, bot_y),
            false,
            &video.fps_label(),
            egui::FontId::proportional(9.5),
            Color32::from_black_alpha(175),
            TEXT_SECONDARY,
        );
    }

    // Bottom-Right: Timecode duration pill
    let dur_text = video.duration_tc();
    let _ = draw_overlay_pill(
        painter,
        egui::pos2(thumb_rect.max.x - 5.0, bot_y),
        true,
        &dur_text,
        egui::FontId::monospace(10.0),
        Color32::from_black_alpha(195),
        Color32::WHITE,
    );

    // Hover Scrubber Preview
    if thumb_resp.hovered() {
        if let Some(mouse_pos) = child_ui.ctx().pointer_latest_pos() {
            let x_rel = (mouse_pos.x - thumb_rect.min.x).clamp(0.0, thumb_rect.width());
            let ratio = x_rel / thumb_rect.width().max(1.0);
            let hover_sec = ratio * video.duration_seconds;

            // Vertical scrub line
            let scrub_x = thumb_rect.min.x + x_rel;
            child_ui.painter().line_segment(
                [egui::pos2(scrub_x, thumb_rect.min.y), egui::pos2(scrub_x, thumb_rect.max.y)],
                Stroke::new(1.8_f32, ACCENT_BLUE),
            );

            // Bottom scrub bar
            let prog_rect = Rect::from_min_max(
                egui::pos2(thumb_rect.min.x, thumb_rect.max.y - 3.0),
                egui::pos2(scrub_x, thumb_rect.max.y),
            );
            child_ui.painter().rect_filled(prog_rect, Rounding::ZERO, ACCENT_BLUE);

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

    // 2. FOOTER ROW 1: COLOR DOT + FILENAME
    child_ui.add_space(5.0);
    child_ui.allocate_ui_with_layout(
        Vec2::new(inner_w, 17.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_max_width(inner_w);
            ui.spacing_mut().item_spacing.x = 5.0;
            if !video.color_label.is_empty() {
                color_dot(ui, &video.color_label);
            }
            let name_color = if video.flag == PickFlag::Rejected {
                TEXT_MUTED
            } else {
                TEXT_PRIMARY
            };
            let lbl = ui.add(
                egui::Label::new(
                    RichText::new(&video.file_name)
                        .strong()
                        .size(12.0)
                        .color(name_color),
                )
                .truncate(),
            );
            lbl.on_hover_text(&video.file_name);
        },
    );

    // 3. FOOTER ROW 2: SLATE / CAMERA / CODEC SUBTITLE
    child_ui.add_space(1.0);
    child_ui.allocate_ui_with_layout(
        Vec2::new(inner_w, 15.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_max_width(inner_w);
            ui.spacing_mut().item_spacing.x = 4.0;

            if let Some(slate) = video.slate_label() {
                let mut sub = slate;
                if !video.camera_model.is_empty() {
                    sub = format!("{} · {}", sub, video.camera_model);
                }
                ui.add(
                    egui::Label::new(RichText::new(sub).size(10.5).color(ACCENT_ORANGE)).truncate(),
                );
            } else {
                let info_text = if !video.camera_model.is_empty() {
                    if !video.focal_length.is_empty() {
                        format!("{} · {} · {}", video.camera_model, video.focal_length, video.codec_display_name())
                    } else if !video.lens_model.is_empty() {
                        format!("{} · {}", video.camera_model, video.lens_model)
                    } else {
                        format!("{} · {}", video.camera_model, video.codec_display_name())
                    }
                } else {
                    format!("{} · {}", video.codec_display_name(), video.file_size_formatted())
                };
                ui.add(
                    egui::Label::new(RichText::new(info_text).size(10.5).color(TEXT_MUTED)).truncate(),
                );
            }
        },
    );

    // 4. FOOTER ROW 3: DEDICATED CULLING BAR (STARS LEFT, FLAGS RIGHT)
    child_ui.add_space(3.0);
    let (cull_rect, _) = child_ui.allocate_exact_size(Vec2::new(inner_w, 20.0), egui::Sense::hover());

    // Left: 5-Star Rating
    let stars_rect = Rect::from_min_size(cull_rect.min, Vec2::new((inner_w - 48.0).max(74.0), 20.0));
    let mut stars_ui = child_ui.new_child(
        egui::UiBuilder::new()
            .max_rect(stars_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let mut rating = video.rating;
    if rating_stars(&mut stars_ui, &mut rating) {
        action = Some(CardAction::UpdateRating(rating));
    }

    // Right: Pick / Reject Flags
    let flags_rect = Rect::from_min_max(
        egui::pos2(cull_rect.max.x - 44.0, cull_rect.min.y),
        cull_rect.max,
    );
    let mut flags_ui = child_ui.new_child(
        egui::UiBuilder::new()
            .max_rect(flags_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let mut flag = video.flag;
    if flag_picker(&mut flags_ui, &mut flag) {
        action = Some(CardAction::UpdateFlag(flag));
    }

    // Card Click Interactions (if no inner button consumed the click)
    if action.is_none() {
        if thumb_resp.double_clicked() || card_resp.double_clicked() {
            action = Some(CardAction::OpenPlayer);
        } else if thumb_resp.clicked() || card_resp.clicked() {
            let toggle = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
            let range = ui.input(|i| i.modifiers.shift);
            action = Some(CardAction::Select { toggle, range });
        }
    }

    // Context Menu on both thumbnail and card body
    let menu_builder = |ui: &mut Ui, act: &mut Option<CardAction>| {
        if ui.button(format!("{} Open in Viewer (Enter)", egui_phosphor::regular::PLAY_CIRCLE)).clicked() {
            *act = Some(CardAction::OpenPlayer);
            ui.close_menu();
        }
        if ui.button(format!("{} Export to NLE... (Ctrl+E)", egui_phosphor::regular::EXPORT)).clicked() {
            *act = Some(CardAction::ExportSelected);
            ui.close_menu();
        }
        if ui.button(format!("{} Play in External Player", egui_phosphor::regular::ARROW_SQUARE_OUT)).clicked() {
            *act = Some(CardAction::PlayExternal);
            ui.close_menu();
        }
        if ui.button(format!("{} Reveal in File Explorer", egui_phosphor::regular::FOLDER_OPEN)).clicked() {
            *act = Some(CardAction::RevealInExplorer);
            ui.close_menu();
        }
        if ui.button(format!("{} Regenerate Thumbnail", egui_phosphor::regular::ARROWS_CLOCKWISE)).clicked() {
            *act = Some(CardAction::RegenerateThumbnail);
            ui.close_menu();
        }
        ui.separator();
        ui.menu_button(format!("{} Star Rating (1-5, 0)", egui_phosphor::regular::STAR), |ui| {
            for r in 0..=5 {
                let label = if r == 0 { "Clear Rating (0)".to_string() } else { format!("{} Stars ({})", r, r) };
                if ui.button(label).clicked() {
                    *act = Some(CardAction::UpdateRating(r));
                    ui.close_menu();
                }
            }
        });
        ui.menu_button(format!("{} Flag Status (P / X / U)", egui_phosphor::regular::FLAG), |ui| {
            if ui.button("Pick (P)").clicked() {
                *act = Some(CardAction::UpdateFlag(PickFlag::Picked));
                ui.close_menu();
            }
            if ui.button("Reject (X)").clicked() {
                *act = Some(CardAction::UpdateFlag(PickFlag::Rejected));
                ui.close_menu();
            }
            if ui.button("Unflagged (U)").clicked() {
                *act = Some(CardAction::UpdateFlag(PickFlag::Unflagged));
                ui.close_menu();
            }
        });
        ui.menu_button(format!("{} Color Label", egui_phosphor::regular::PALETTE), |ui| {
            for c in ["None", "Red", "Orange", "Yellow", "Green", "Blue", "Purple"] {
                let val = if c == "None" { String::new() } else { c.to_lowercase() };
                if ui.button(c).clicked() {
                    *act = Some(CardAction::UpdateColor(val));
                    ui.close_menu();
                }
            }
        });
        ui.separator();
        if ui.button(format!("{} Remove from Catalog (Del)", egui_phosphor::regular::TRASH)).clicked() {
            *act = Some(CardAction::DeleteVideo);
            ui.close_menu();
        }
    };

    thumb_resp.context_menu(|ui| menu_builder(ui, &mut action));
    card_resp.context_menu(|ui| menu_builder(ui, &mut action));

    action
}
