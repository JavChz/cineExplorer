use egui::{Color32, Rect, RichText, Rounding, Stroke, Ui, Vec2};
use crate::db::models::PickFlag;
use crate::media::color_lut::LogProfile;
use crate::playback::VideoPlayer;
use crate::ui::theme::*;
use crate::ui::widgets::*;

pub enum TransportAction {
    TogglePlay,
    StepBackward,
    StepForward,
    Step10Backward,
    Step10Forward,
    Seek(f32),
    SetInPoint,
    SetOutPoint,
    ClearInOut,
    GotoIn,
    GotoOut,
    GotoStart,
    GotoEnd,
    UpdateRating(u8),
    UpdateFlag(PickFlag),
    PrevClip,
    NextClip,
    ClosePlayer,
    PlayExternal,
    ToggleRec709Lut,
    SetLogProfile(LogProfile),
    ToggleFullscreen,
}

pub fn render_transport_bar(
    ui: &mut Ui,
    player: &mut VideoPlayer,
    clip_index: usize,
    total_clips: usize,
    fullscreen: bool,
) -> Option<TransportAction> {
    let mut action = None;
    let max_sec = player.clip.duration_seconds.max(0.1);

    ui.vertical(|ui| {
        // ROW 1: FULL-WIDTH INTERACTIVE TIMELINE SCRUBBER
        let track_height = 20.0;
        let track_size = Vec2::new(ui.available_width(), track_height);
        let (track_rect, track_resp) =
            ui.allocate_exact_size(track_size, egui::Sense::click_and_drag());

        // Background track bar
        let bar_h = 6.0;
        let bar_rect = Rect::from_center_size(
            track_rect.center(),
            Vec2::new(track_rect.width(), bar_h),
        );
        ui.painter().rect_filled(
            bar_rect,
            Rounding::same(3.0),
            Color32::from_rgb(28, 31, 40),
        );

        // In / Out shaded region
        let in_sec = player.in_point.unwrap_or(0.0).clamp(0.0, max_sec);
        let out_sec = player.out_point.unwrap_or(max_sec).clamp(0.0, max_sec);

        let in_x = bar_rect.min.x + (in_sec / max_sec) * bar_rect.width();
        let out_x = bar_rect.min.x + (out_sec / max_sec) * bar_rect.width();

        if in_x < out_x && (player.in_point.is_some() || player.out_point.is_some()) {
            let io_rect = Rect::from_min_max(
                egui::pos2(in_x, bar_rect.min.y - 1.0),
                egui::pos2(out_x, bar_rect.max.y + 1.0),
            );
            ui.painter()
                .rect_filled(io_rect, Rounding::same(2.0), Color32::from_rgb(42, 68, 110));

            if player.in_point.is_some() {
                ui.painter().line_segment(
                    [
                        egui::pos2(in_x, track_rect.min.y + 1.0),
                        egui::pos2(in_x, track_rect.max.y - 1.0),
                    ],
                    Stroke::new(2.0_f32, ACCENT_ORANGE),
                );
            }
            if player.out_point.is_some() {
                ui.painter().line_segment(
                    [
                        egui::pos2(out_x, track_rect.min.y + 1.0),
                        egui::pos2(out_x, track_rect.max.y - 1.0),
                    ],
                    Stroke::new(2.0_f32, ACCENT_ORANGE),
                );
            }
        }

        // Played progress bar up to playhead
        let playhead_sec = player.playhead.clamp(0.0, max_sec);
        let playhead_x = bar_rect.min.x + (playhead_sec / max_sec) * bar_rect.width();
        let progress_rect =
            Rect::from_min_max(bar_rect.min, egui::pos2(playhead_x, bar_rect.max.y));
        ui.painter()
            .rect_filled(progress_rect, Rounding::same(3.0), ACCENT_BLUE);

        // Playhead scrubber needle & handle
        ui.painter().line_segment(
            [
                egui::pos2(playhead_x, track_rect.min.y),
                egui::pos2(playhead_x, track_rect.max.y),
            ],
            Stroke::new(2.0_f32, Color32::WHITE),
        );
        ui.painter().circle_filled(
            egui::pos2(playhead_x, track_rect.center().y),
            5.5,
            Color32::WHITE,
        );

        // Hover scrubber & click/drag seeking
        if track_resp.hovered() || track_resp.dragged() {
            if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                let x_rel = (mouse_pos.x - bar_rect.min.x).clamp(0.0, bar_rect.width());
                let hover_sec = (x_rel / bar_rect.width().max(1.0)) * max_sec;

                let ghost_x = bar_rect.min.x + x_rel;
                ui.painter().line_segment(
                    [
                        egui::pos2(ghost_x, track_rect.min.y + 3.0),
                        egui::pos2(ghost_x, track_rect.max.y - 3.0),
                    ],
                    Stroke::new(1.0_f32, Color32::from_white_alpha(130)),
                );

                let h = (hover_sec / 3600.0) as u32;
                let m = ((hover_sec % 3600.0) / 60.0) as u32;
                let s = (hover_sec % 60.0) as u32;
                let f = ((hover_sec.fract()) * player.clip.fps.max(1.0)) as u32;
                let tc = format!("{:02}:{:02}:{:02}:{:02}", h, m, s, f);
                track_resp.show_tooltip_text(tc);

                if track_resp.clicked() || track_resp.dragged() {
                    action = Some(TransportAction::Seek(hover_sec));
                }
            }
        }

        ui.add_space(4.0);

        // ROW 2: TRUE 3-ZONE NON-OVERLAPPING TRANSPORT DECK
        let deck_h = 30.0;
        let total_w = ui.available_width();
        let (deck_rect, _) =
            ui.allocate_exact_size(Vec2::new(total_w, deck_h), egui::Sense::hover());

        let center_w = 290.0_f32.min(total_w * 0.42);
        let side_w = ((total_w - center_w) * 0.5).max(120.0);

        let left_rect = Rect::from_min_size(deck_rect.min, Vec2::new(side_w, deck_h));
        let center_rect = Rect::from_center_size(deck_rect.center(), Vec2::new(center_w, deck_h));
        let right_rect = Rect::from_min_max(
            egui::pos2(deck_rect.max.x - side_w, deck_rect.min.y),
            deck_rect.max,
        );

        // ZONE 1 (LEFT): Timecode, Frame Counter & Shuttle Speed HUD
        let mut left_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(left_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        left_ui.set_clip_rect(left_rect);
        left_ui.spacing_mut().item_spacing.x = 8.0;

        left_ui.label(
            RichText::new(player.current_timecode())
                .monospace()
                .strong()
                .size(13.5)
                .color(Color32::WHITE),
        );
        left_ui.label(
            RichText::new(format!("/ {}", player.clip.duration_tc()))
                .monospace()
                .size(12.0)
                .color(TEXT_MUTED),
        );

        let current_frame = (player.playhead * player.clip.fps.max(1.0)) as u32;
        let total_frames = player.clip.frame_count;
        if total_frames > 0 {
            left_ui.label(
                RichText::new(format!("F {}/{}", current_frame, total_frames))
                    .monospace()
                    .size(11.0)
                    .color(TEXT_SECONDARY),
            );
        }

        if (player.playback_rate - 1.0).abs() > 0.05 {
            badge(
                &mut left_ui,
                &format!("{:.1}x", player.playback_rate),
                Color32::from_rgb(60, 38, 16),
                ACCENT_ORANGE,
            );
        }

        if player.in_point.is_some() || player.out_point.is_some() {
            let io_dur = (out_sec - in_sec).max(0.0);
            left_ui.label(
                RichText::new(format!("I/O {:.1}s", io_dur))
                    .monospace()
                    .size(10.5)
                    .color(ACCENT_ORANGE),
            );
            if left_ui
                .small_button(RichText::new("×").color(TEXT_MUTED))
                .on_hover_text("Clear In/Out Points (Alt+X)")
                .clicked()
            {
                action = Some(TransportAction::ClearInOut);
            }
        }

        // ZONE 2 (CENTER): Centered Transport Deck Under Monitor
        let mut center_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(center_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        center_ui.spacing_mut().item_spacing.x = 5.0;

        // Center content horizontally inside center_rect
        let controls_est_w = 248.0;
        let pad_left = ((center_rect.width() - controls_est_w) * 0.5).max(0.0);
        if pad_left > 0.0 {
            center_ui.add_space(pad_left);
        }

        // Mark In Button
        let in_active = player.in_point.is_some();
        let in_btn = egui::Button::new(
            RichText::new("[ In")
                .monospace()
                .size(11.0)
                .strong()
                .color(if in_active { ACCENT_ORANGE } else { TEXT_SECONDARY }),
        )
        .min_size(Vec2::new(30.0, 24.0));
        if center_ui
            .add(in_btn)
            .on_hover_text("Mark In Point (I) · Shift+I to Go to In")
            .clicked()
        {
            action = Some(TransportAction::SetInPoint);
        }

        // Step -10 frames
        if center_ui
            .add(
                egui::Button::new(
                    RichText::new(egui_phosphor::regular::CARET_DOUBLE_LEFT).size(13.5),
                )
                .min_size(Vec2::new(26.0, 24.0)),
            )
            .on_hover_text("Step 10 Frames Backward (Shift+Left)")
            .clicked()
        {
            action = Some(TransportAction::Step10Backward);
        }

        // Step -1 frame
        if center_ui
            .add(
                egui::Button::new(RichText::new(egui_phosphor::regular::CARET_LEFT).size(13.5))
                    .min_size(Vec2::new(26.0, 24.0)),
            )
            .on_hover_text("Step 1 Frame Backward (Left)")
            .clicked()
        {
            action = Some(TransportAction::StepBackward);
        }

        // Primary Play / Pause Button
        let play_icon = if player.is_playing {
            egui_phosphor::fill::PAUSE
        } else {
            egui_phosphor::fill::PLAY
        };
        let play_btn = egui::Button::new(fill_icon_text(play_icon, 16.0, Color32::WHITE))
            .fill(ACCENT_BLUE)
            .min_size(Vec2::new(44.0, 26.0));

        if center_ui
            .add(play_btn)
            .on_hover_text("Play / Pause (Space / K) · Shuttle (J / K / L)")
            .clicked()
        {
            action = Some(TransportAction::TogglePlay);
        }

        // Step +1 frame
        if center_ui
            .add(
                egui::Button::new(RichText::new(egui_phosphor::regular::CARET_RIGHT).size(13.5))
                    .min_size(Vec2::new(26.0, 24.0)),
            )
            .on_hover_text("Step 1 Frame Forward (Right)")
            .clicked()
        {
            action = Some(TransportAction::StepForward);
        }

        // Step +10 frames
        if center_ui
            .add(
                egui::Button::new(
                    RichText::new(egui_phosphor::regular::CARET_DOUBLE_RIGHT).size(13.5),
                )
                .min_size(Vec2::new(26.0, 24.0)),
            )
            .on_hover_text("Step 10 Frames Forward (Shift+Right)")
            .clicked()
        {
            action = Some(TransportAction::Step10Forward);
        }

        // Mark Out Button
        let out_active = player.out_point.is_some();
        let out_btn = egui::Button::new(
            RichText::new("Out ]")
                .monospace()
                .size(11.0)
                .strong()
                .color(if out_active { ACCENT_ORANGE } else { TEXT_SECONDARY }),
        )
        .min_size(Vec2::new(34.0, 24.0));
        if center_ui
            .add(out_btn)
            .on_hover_text("Mark Out Point (O) · Shift+O to Go to Out")
            .clicked()
        {
            action = Some(TransportAction::SetOutPoint);
        }

        // Loop Toggle
        let loop_fg = if player.looping { ACCENT_BLUE } else { TEXT_MUTED };
        let loop_btn = egui::Button::new(
            RichText::new(egui_phosphor::regular::REPEAT)
                .color(loop_fg)
                .size(13.5),
        )
        .min_size(Vec2::new(26.0, 24.0));
        if center_ui
            .add(loop_btn)
            .on_hover_text("Toggle Loop Playback (Ctrl+L)")
            .clicked()
        {
            player.looping = !player.looping;
        }

        // ZONE 3 (RIGHT): Culling & Bin Stepper (laid out cleanly without reversing stars/flags)
        let mut right_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(right_rect)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        right_ui.set_clip_rect(right_rect);
        right_ui.spacing_mut().item_spacing.x = 6.0;

        // External OS Player
        let ext_tooltip = if player.clip.is_braw() {
            "Open in Blackmagic RAW Player / DaVinci Resolve"
        } else {
            "Open in External OS Video Player"
        };
        if right_ui
            .add(
                egui::Button::new(
                    RichText::new(egui_phosphor::regular::ARROW_SQUARE_OUT).size(13.5),
                )
                .min_size(Vec2::new(26.0, 24.0)),
            )
            .on_hover_text(ext_tooltip)
            .clicked()
        {
            action = Some(TransportAction::PlayExternal);
        }

        // Fullscreen Toggle Button (F)
        let (fs_icon, fs_tooltip) = if fullscreen {
            (egui_phosphor::regular::CORNERS_IN, "Exit Fullscreen (F / Esc)")
        } else {
            (egui_phosphor::regular::CORNERS_OUT, "Enter Fullscreen (F)")
        };
        if right_ui
            .add(
                egui::Button::new(RichText::new(fs_icon).size(13.5))
                    .min_size(Vec2::new(26.0, 24.0)),
            )
            .on_hover_text(fs_tooltip)
            .clicked()
        {
            action = Some(TransportAction::ToggleFullscreen);
        }

        right_ui.add(egui::Separator::default().vertical());

        // Next Clip Button
        if right_ui
            .add(
                egui::Button::new(RichText::new(egui_phosphor::regular::SKIP_FORWARD).size(13.0))
                    .min_size(Vec2::new(24.0, 24.0)),
            )
            .on_hover_text("Next Clip in Bin (])")
            .clicked()
        {
            action = Some(TransportAction::NextClip);
        }

        if total_clips > 0 {
            right_ui.label(
                RichText::new(format!("{}/{}", clip_index + 1, total_clips))
                    .monospace()
                    .size(11.0)
                    .color(TEXT_SECONDARY),
            );
        }

        // Prev Clip Button
        if right_ui
            .add(
                egui::Button::new(RichText::new(egui_phosphor::regular::SKIP_BACK).size(13.0))
                    .min_size(Vec2::new(24.0, 24.0)),
            )
            .on_hover_text("Previous Clip in Bin ([)")
            .clicked()
        {
            action = Some(TransportAction::PrevClip);
        }

        right_ui.add(egui::Separator::default().vertical());

        // Inline Flag picker (Pick / Reject)
        let mut flag = player.clip.flag;
        if flag_picker(&mut right_ui, &mut flag) {
            action = Some(TransportAction::UpdateFlag(flag));
        }

        // Inline Rating stars
        let mut rating = player.clip.rating;
        if rating_stars(&mut right_ui, &mut rating) {
            action = Some(TransportAction::UpdateRating(rating));
        }
    });

    action
}
