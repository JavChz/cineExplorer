pub mod transport;

use egui::{Color32, Rect, RichText, Rounding, Stroke, Ui, Vec2};
use crate::playback::VideoPlayer;
use crate::ui::theme::*;
use crate::ui::widgets::badge;
use transport::{render_transport_bar, TransportAction};

pub fn render_player_view(
    ui: &mut Ui,
    player: &mut VideoPlayer,
    clip_index: usize,
    total_clips: usize,
    fullscreen: bool,
    controls_visible: bool,
) -> Option<TransportAction> {
    if fullscreen {
        render_fullscreen_player(ui, player, clip_index, total_clips, controls_visible)
    } else {
        render_windowed_player(ui, player, clip_index, total_clips)
    }
}

pub fn render_player_header(
    ui: &mut Ui,
    player: &mut VideoPlayer,
    fullscreen: bool,
) -> Option<TransportAction> {
    let mut action = None;
    let header_h = if fullscreen { 34.0 } else { 28.0 };
    let (header_rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), header_h),
        egui::Sense::hover(),
    );

    // Left side of header: Back to Grid + Fullscreen Toggle + Clip Title + Tech Badges
    let right_reserve = 250.0;
    let left_w = (header_rect.width() - right_reserve).max(200.0);
    let left_rect = Rect::from_min_size(header_rect.min, Vec2::new(left_w, header_h));
    let mut left_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(left_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    left_ui.spacing_mut().item_spacing.x = 6.0;

    let back_btn = egui::Button::new(
        RichText::new(format!("{} Grid", egui_phosphor::regular::ARROW_LEFT))
            .size(12.0)
            .strong()
            .color(TEXT_PRIMARY),
    );
    if left_ui
        .add(back_btn)
        .on_hover_text("Return to Media Grid (Esc / G / Tab)")
        .clicked()
    {
        action = Some(TransportAction::ClosePlayer);
    }

    // Fullscreen toggle button in header
    let (fs_icon, fs_label, fs_tip) = if fullscreen {
        (egui_phosphor::regular::CORNERS_IN, "Exit Fullscreen", "Exit Fullscreen (F / Esc)")
    } else {
        (egui_phosphor::regular::CORNERS_OUT, "Fullscreen", "Fullscreen Mode (F)")
    };
    let fs_btn = egui::Button::new(
        RichText::new(format!("{} {}", fs_icon, fs_label))
            .size(11.5)
            .color(TEXT_SECONDARY),
    );
    if left_ui.add(fs_btn).on_hover_text(fs_tip).clicked() {
        action = Some(TransportAction::ToggleFullscreen);
    }

    left_ui.add(egui::Separator::default().vertical());

    left_ui.add(
        egui::Label::new(
            RichText::new(&player.clip.file_name)
                .strong()
                .size(13.5)
                .color(TEXT_PRIMARY),
        )
        .truncate(),
    );

    badge(
        &mut left_ui,
        &player.clip.resolution_label(),
        BADGE_4K_BG,
        BADGE_4K_FG,
    );

    badge(
        &mut left_ui,
        &player.clip.fps_label(),
        BG_CARD,
        TEXT_SECONDARY,
    );

    badge(
        &mut left_ui,
        &player.clip.codec_display_name(),
        BG_CARD,
        TEXT_SECONDARY,
    );

    if player.clip.bit_depth >= 10 {
        badge(
            &mut left_ui,
            &format!("{}-bit", player.clip.bit_depth),
            BADGE_10BIT_BG,
            BADGE_10BIT_FG,
        );
    }

    if player.clip.is_braw() {
        badge(
            &mut left_ui,
            "Blackmagic RAW",
            BADGE_BRAW_BG,
            BADGE_BRAW_FG,
        );
    } else if player.is_gpu_accelerated {
        left_ui.label(
            RichText::new(format!("{} GPU", egui_phosphor::regular::LIGHTNING))
                .small()
                .color(ACCENT_GREEN),
        );
    }

    // Right side of header: Unified Color / 3D LUT Monitor Controls
    let right_rect = Rect::from_min_max(
        egui::pos2(header_rect.max.x - right_reserve, header_rect.min.y),
        header_rect.max,
    );
    let mut right_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(right_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
    );
    right_ui.spacing_mut().item_spacing.x = 6.0;

    let is_log = player.log_profile.is_log();

    // Log Profile selector dropdown
    let profile_label = if is_log {
        player.log_profile.short_name()
    } else {
        "SDR (Rec.709)"
    };
    egui::ComboBox::from_id_salt(if fullscreen { "fs_player_log_profile_select" } else { "player_log_profile_select" })
        .selected_text(
            RichText::new(profile_label)
                .size(11.5)
                .strong()
                .color(if is_log { BADGE_10BIT_FG } else { TEXT_SECONDARY }),
        )
        .width(110.0)
        .show_ui(&mut right_ui, |ui| {
            for opt in crate::media::color_lut::LogProfile::all_options() {
                let is_sel = player.log_profile == *opt;
                if ui.selectable_label(is_sel, opt.display_name()).clicked() {
                    action = Some(TransportAction::SetLogProfile(*opt));
                }
            }
        });

    // Single authoritative Rec.709 3D LUT toggle button when viewing Log footage
    if is_log {
        let (lut_icon, lut_fg, lut_bg, lut_border, lut_text) = if player.apply_rec709_lut {
            (
                egui_phosphor::regular::EYE,
                ACCENT_GREEN,
                Color32::from_rgb(20, 46, 30),
                ACCENT_GREEN,
                "Rec.709 LUT",
            )
        } else {
            (
                egui_phosphor::regular::EYE_SLASH,
                ACCENT_ORANGE,
                Color32::from_rgb(52, 34, 18),
                ACCENT_ORANGE,
                "Raw Log",
            )
        };
        let lut_btn = egui::Button::new(
            RichText::new(format!("{} {}", lut_icon, lut_text))
                .size(11.5)
                .strong()
                .color(lut_fg),
        )
        .fill(lut_bg)
        .stroke(Stroke::new(1.0_f32, lut_border));

        let tooltip = if player.apply_rec709_lut {
            "Rec.709 3D LUT Active (Alt+L) — Click to view ungraded Log"
        } else {
            "Viewing Raw Ungraded Log (Alt+L) — Click to apply Rec.709 LUT"
        };
        if right_ui.add(lut_btn).on_hover_text(tooltip).clicked() {
            action = Some(TransportAction::ToggleRec709Lut);
        }
    } else {
        right_ui.label(RichText::new("Color:").small().color(TEXT_MUTED));
    }

    action
}

fn render_windowed_player(
    ui: &mut Ui,
    player: &mut VideoPlayer,
    clip_index: usize,
    total_clips: usize,
) -> Option<TransportAction> {
    let mut action = None;

    ui.vertical(|ui| {
        if let Some(act) = render_player_header(ui, player, false) {
            action = Some(act);
        }

        ui.add_space(2.0);

        // VIDEO VIEWPORT CANVAS
        let transport_reserve = 64.0;
        let avail_size = Vec2::new(
            ui.available_width(),
            (ui.available_height() - transport_reserve).max(120.0),
        );

        let (rect, resp) = ui.allocate_exact_size(avail_size, egui::Sense::click());
        ui.painter().rect_filled(rect, Rounding::same(4.0), Color32::from_rgb(9, 10, 13));
        ui.painter().rect_stroke(
            rect,
            Rounding::same(4.0),
            Stroke::new(1.0_f32, Color32::from_rgb(30, 33, 42)),
        );

        if resp.double_clicked() {
            action = Some(TransportAction::ToggleFullscreen);
        } else if resp.clicked() {
            action = Some(TransportAction::TogglePlay);
        }

        // Compute letterboxed video dimensions
        let clip_w = (player.clip.width as f32).max(1.0);
        let clip_h = (player.clip.height as f32).max(1.0);
        let aspect = clip_w / clip_h;

        let inner_canvas = rect.shrink(2.0);
        let target_w = (inner_canvas.height() * aspect).min(inner_canvas.width());
        let target_h = (target_w / aspect).min(inner_canvas.height());
        let video_rect = Rect::from_center_size(inner_canvas.center(), Vec2::new(target_w, target_h));

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

        if player.clip.is_braw() {
            let banner_rect = Rect::from_center_size(
                egui::pos2(video_rect.center().x, video_rect.max.y - 30.0),
                Vec2::new(350.0, 36.0),
            );
            ui.painter().rect_filled(
                banner_rect,
                Rounding::same(6.0),
                Color32::from_black_alpha(215),
            );
            ui.painter().rect_stroke(
                banner_rect,
                Rounding::same(6.0),
                Stroke::new(1.2_f32, BADGE_BRAW_FG),
            );
            let braw_btn = egui::Button::new(
                RichText::new("🎬 Open in Blackmagic RAW Player (Space)")
                    .strong()
                    .size(12.5)
                    .color(BADGE_BRAW_FG),
            )
            .fill(Color32::TRANSPARENT);
            if ui.put(banner_rect, braw_btn).on_hover_text("Open in native Blackmagic RAW Player or DaVinci Resolve (Space)").clicked() {
                action = Some(TransportAction::PlayExternal);
            }
        }

        ui.add_space(4.0);

        // TRANSPORT CONTROLS BAR
        if let Some(t_act) = render_transport_bar(ui, player, clip_index, total_clips, false) {
            action = Some(t_act);
        }
    });

    action
}

fn render_fullscreen_player(
    ui: &mut Ui,
    player: &mut VideoPlayer,
    clip_index: usize,
    total_clips: usize,
    controls_visible: bool,
) -> Option<TransportAction> {
    let mut action = None;

    let avail_rect = ui.available_rect_before_wrap();
    let (canvas_rect, canvas_resp) = ui.allocate_exact_size(avail_rect.size(), egui::Sense::click());

    // 1. Fullscreen letterbox background
    ui.painter().rect_filled(canvas_rect, Rounding::ZERO, Color32::BLACK);

    // 2. Letterboxed video frame
    let clip_w = (player.clip.width as f32).max(1.0);
    let clip_h = (player.clip.height as f32).max(1.0);
    let aspect = clip_w / clip_h;

    let target_w = (canvas_rect.height() * aspect).min(canvas_rect.width());
    let target_h = (target_w / aspect).min(canvas_rect.height());
    let video_rect = Rect::from_center_size(canvas_rect.center(), Vec2::new(target_w, target_h));

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
            egui::FontId::proportional(15.0),
            TEXT_MUTED,
        );
    }

    // Check if pointer is over overlay control zones
    let header_h = 44.0;
    let transport_h = 72.0;
    let top_overlay_rect = Rect::from_min_size(canvas_rect.min, Vec2::new(canvas_rect.width(), header_h));
    let bot_overlay_rect = Rect::from_min_max(
        egui::pos2(canvas_rect.min.x, canvas_rect.max.y - transport_h),
        canvas_rect.max,
    );

    let pointer_pos = ui.ctx().pointer_latest_pos();
    let is_over_controls = controls_visible && pointer_pos.is_some_and(|p| {
        top_overlay_rect.contains(p) || bot_overlay_rect.contains(p)
    });

    if !is_over_controls {
        if canvas_resp.double_clicked() {
            action = Some(TransportAction::ToggleFullscreen);
        } else if canvas_resp.clicked() {
            action = Some(TransportAction::TogglePlay);
        }
    }

    // If controls are visible (mouse not idle), render top and bottom overlays on top of video
    if controls_visible {
        // TOP OVERLAY: Header Bar
        ui.painter().rect_filled(
            top_overlay_rect,
            Rounding::ZERO,
            Color32::from_rgba_unmultiplied(12, 14, 18, 225),
        );
        ui.painter().line_segment(
            [egui::pos2(top_overlay_rect.min.x, top_overlay_rect.max.y), top_overlay_rect.max],
            Stroke::new(1.0_f32, Color32::from_white_alpha(30)),
        );

        let top_inner = top_overlay_rect.shrink2(Vec2::new(14.0, 4.0));
        let mut top_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(top_inner)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        if let Some(act) = render_player_header(&mut top_ui, player, true) {
            action = Some(act);
        }

        // BRAW banner if BRAW
        if player.clip.is_braw() {
            let banner_rect = Rect::from_center_size(
                egui::pos2(video_rect.center().x, bot_overlay_rect.min.y - 28.0),
                Vec2::new(380.0, 36.0),
            );
            ui.painter().rect_filled(banner_rect, Rounding::same(6.0), Color32::from_black_alpha(225));
            ui.painter().rect_stroke(banner_rect, Rounding::same(6.0), Stroke::new(1.2_f32, BADGE_BRAW_FG));
            let braw_btn = egui::Button::new(
                RichText::new("🎬 Open in Blackmagic RAW Player (Space)")
                    .strong()
                    .size(13.0)
                    .color(BADGE_BRAW_FG),
            )
            .fill(Color32::TRANSPARENT);
            if ui.put(banner_rect, braw_btn).on_hover_text("Open in native Blackmagic RAW Player or DaVinci Resolve (Space)").clicked() {
                action = Some(TransportAction::PlayExternal);
            }
        }

        // BOTTOM OVERLAY: Transport Controls Bar
        ui.painter().rect_filled(
            bot_overlay_rect,
            Rounding::ZERO,
            Color32::from_rgba_unmultiplied(12, 14, 18, 230),
        );
        ui.painter().line_segment(
            [bot_overlay_rect.min, egui::pos2(bot_overlay_rect.max.x, bot_overlay_rect.min.y)],
            Stroke::new(1.0_f32, Color32::from_white_alpha(30)),
        );

        let bot_inner = bot_overlay_rect.shrink2(Vec2::new(16.0, 6.0));
        let mut bot_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bot_inner)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        if let Some(act) = render_transport_bar(&mut bot_ui, player, clip_index, total_clips, true) {
            action = Some(act);
        }
    }

    action
}
