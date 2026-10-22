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
}

pub fn render_transport_bar(
    ui: &mut Ui,
    player: &mut VideoPlayer,
    clip_index: usize,
    total_clips: usize,
) -> Option<TransportAction> {
    let mut action = None;
    let max_sec = player.clip.duration_seconds.max(0.1);

    ui.vertical(|ui| {
        // ROW 1: FULL-WIDTH INTERACTIVE TIMELINE SCRUBBER
        let track_height = 20.0;
        let track_size = Vec2::new(ui.available_width(), track_height);
        let (track_rect, track_resp) = ui.allocate_exact_size(track_size, egui::Sense::click_and_drag());

        // Background track bar
        let bar_h = 6.0;
        let bar_rect = Rect::from_center_size(
            track_rect.center(),
            Vec2::new(track_rect.width(), bar_h),
        );
        ui.painter().rect_filled(bar_rect, Rounding::same(3.0), Color32::from_rgb(28, 31, 38));

        // In / Out shaded region
        let in_sec = player.in_point.unwrap_or(0.0).clamp(0.0, max_sec);
        let out_sec = player.out_point.unwrap_or(max_sec).clamp(0.0, max_sec);

        let in_x = bar_rect.min.x + (in_sec / max_sec) * bar_rect.width();
        let out_x = bar_rect.min.x + (out_sec / max_sec) * bar_rect.width();

        if in_x < out_x {
            let io_rect = Rect::from_min_max(
                egui::pos2(in_x, bar_rect.min.y),
                egui::pos2(out_x, bar_rect.max.y),
            );
            ui.painter().rect_filled(io_rect, Rounding::ZERO, Color32::from_rgb(45, 65, 95));

            // In / Out marker ticks
            if player.in_point.is_some() {
                ui.painter().line_segment(
                    [egui::pos2(in_x, track_rect.min.y + 2.0), egui::pos2(in_x, track_rect.max.y - 2.0)],
                    Stroke::new(2.0_f32, ACCENT_BLUE),
                );
            }
            if player.out_point.is_some() {
                ui.painter().line_segment(
                    [egui::pos2(out_x, track_rect.min.y + 2.0), egui::pos2(out_x, track_rect.max.y - 2.0)],
                    Stroke::new(2.0_f32, ACCENT_BLUE),
                );
            }
        }

        // Played progress bar up to playhead
        let playhead_sec = player.playhead.clamp(0.0, max_sec);
        let playhead_x = bar_rect.min.x + (playhead_sec / max_sec) * bar_rect.width();
        let progress_rect = Rect::from_min_max(
            bar_rect.min,
            egui::pos2(playhead_x, bar_rect.max.y),
        );
        ui.painter().rect_filled(progress_rect, Rounding::same(3.0), ACCENT_BLUE);

        // Playhead scrubber needle & handle
        ui.painter().line_segment(
            [egui::pos2(playhead_x, track_rect.min.y), egui::pos2(playhead_x, track_rect.max.y)],
            Stroke::new(2.0_f32, Color32::WHITE),
        );
        ui.painter().circle_filled(
            egui::pos2(playhead_x, track_rect.center().y),
            5.0,
            Color32::WHITE,
        );

        // Hover scrubber & click/drag seeking
        if track_resp.hovered() || track_resp.dragged() {
            if let Some(mouse_pos) = ui.ctx().pointer_latest_pos() {
                let x_rel = (mouse_pos.x - bar_rect.min.x).clamp(0.0, bar_rect.width());
                let hover_sec = (x_rel / bar_rect.width()) * max_sec;

                // Draw hover ghost needle
                let ghost_x = bar_rect.min.x + x_rel;
                ui.painter().line_segment(
                    [egui::pos2(ghost_x, track_rect.min.y + 3.0), egui::pos2(ghost_x, track_rect.max.y - 3.0)],
                    Stroke::new(1.0_f32, Color32::from_white_alpha(120)),
                );

                // Tooltip with hover timecode
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

        // ROW 2: THREE-ZONE BALANCED TRANSPORT DECK
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            // ZONE 1 (LEFT): Timecode & Clip Info HUD
            ui.horizontal(|ui| {
                // Timecode pill
                let tc_text = format!("{} / {}", player.current_timecode(), player.clip.duration_tc());
                ui.label(
                    RichText::new(tc_text)
                        .monospace()
                        .strong()
                        .size(13.0)
                        .color(Color32::WHITE),
                );

                // Frame Index
                let current_frame = (player.playhead * player.clip.fps.max(1.0)) as u32;
                let total_frames = player.clip.frame_count;
                if total_frames > 0 {
                    ui.label(
                        RichText::new(format!("F {}/{}", current_frame, total_frames))
                            .monospace()
                            .small()
                            .color(TEXT_MUTED),
                    );
                }

                // Playback speed pill if non-standard
                if (player.playback_rate - 1.0).abs() > 0.1 {
                    let speed_str = format!("{:.1}x", player.playback_rate);
                    ui.label(RichText::new(speed_str).small().strong().color(ACCENT_ORANGE));
                }

                // Log Profile / Rec.709 LUT quick toggle pill
                if player.log_profile.is_log() {
                    let lut_icon = if player.apply_rec709_lut {
                        egui_phosphor::fill::EYE
                    } else {
                        egui_phosphor::regular::EYE_SLASH
                    };
                    let lut_color = if player.apply_rec709_lut { ACCENT_GREEN } else { ACCENT_ORANGE };
                    let lut_text = if player.apply_rec709_lut { "Rec.709" } else { "Raw Log" };
                    let btn = ui.small_button(RichText::new(format!("{} {}", lut_icon, lut_text)).color(lut_color))
                        .on_hover_text(if player.apply_rec709_lut {
                            "Rec.709 3D LUT Active (Alt+L) — click to view raw Log"
                        } else {
                            "Viewing Raw Ungraded Log (Alt+L) — click to apply Rec.709 LUT"
                        });
                    if btn.clicked() {
                        action = Some(TransportAction::ToggleRec709Lut);
                    }
                }
            });

            // ZONE 2 (CENTER): Centered Transport Controls
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(30.0);

                // Mark In Button
                if ui.small_button("[ In ]").on_hover_text("Mark In Point (I)").clicked() {
                    action = Some(TransportAction::SetInPoint);
                }

                // Step -10 frames
                if ui.button(RichText::new(egui_phosphor::regular::CARET_DOUBLE_LEFT).size(14.0)).on_hover_text("Step 10 Frames Backward (Shift+Left)").clicked() {
                    action = Some(TransportAction::Step10Backward);
                }

                // Step -1 frame
                if ui.button(RichText::new(egui_phosphor::regular::CARET_LEFT).size(14.0)).on_hover_text("Step 1 Frame Backward (Left)").clicked() {
                    action = Some(TransportAction::StepBackward);
                }

                // Primary Play / Pause Button
                let play_icon = if player.is_playing {
                    egui_phosphor::fill::PAUSE
                } else {
                    egui_phosphor::fill::PLAY
                };
                let play_btn = egui::Button::new(
                    RichText::new(play_icon).size(18.0).color(Color32::WHITE)
                )
                .fill(ACCENT_BLUE)
                .min_size(Vec2::new(42.0, 26.0));

                if ui.add(play_btn).on_hover_text("Play / Pause (Space / K)").clicked() {
                    action = Some(TransportAction::TogglePlay);
                }

                // Step +1 frame
                if ui.button(RichText::new(egui_phosphor::regular::CARET_RIGHT).size(14.0)).on_hover_text("Step 1 Frame Forward (Right)").clicked() {
                    action = Some(TransportAction::StepForward);
                }

                // Step +10 frames
                if ui.button(RichText::new(egui_phosphor::regular::CARET_DOUBLE_RIGHT).size(14.0)).on_hover_text("Step 10 Frames Forward (Shift+Right)").clicked() {
                    action = Some(TransportAction::Step10Forward);
                }

                // Mark Out Button
                if ui.small_button("[ Out ]").on_hover_text("Mark Out Point (O)").clicked() {
                    action = Some(TransportAction::SetOutPoint);
                }

                // Clear In/Out if set
                if (player.in_point.is_some() || player.out_point.is_some())
                    && ui.small_button(RichText::new("Clear I/O").small().color(TEXT_MUTED)).on_hover_text("Clear In/Out Points (Alt+X)").clicked() {
                    action = Some(TransportAction::ClearInOut);
                }

                // Loop Toggle
                let loop_icon = egui_phosphor::regular::REPEAT;
                let loop_fg = if player.looping { ACCENT_BLUE } else { TEXT_MUTED };
                if ui.button(RichText::new(loop_icon).color(loop_fg).size(14.0)).on_hover_text("Loop Playback (Ctrl+L)").clicked() {
                    player.looping = !player.looping;
                }
            });

            // ZONE 3 (RIGHT): Quick Culling & Navigation
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Close / Back to Grid button
                let close_btn = egui::Button::new(
                    RichText::new(format!("{} Grid", egui_phosphor::regular::SQUARES_FOUR))
                        .size(12.0)
                        .color(TEXT_PRIMARY),
                );
                if ui.add(close_btn).on_hover_text("Return to Grid View (Esc / G / Tab)").clicked() {
                    action = Some(TransportAction::ClosePlayer);
                }

                // External Player
                if ui.button(RichText::new(egui_phosphor::regular::ARROW_SQUARE_OUT).size(14.0)).on_hover_text("Launch in Default OS Player").clicked() {
                    action = Some(TransportAction::PlayExternal);
                }

                ui.add(egui::Separator::default().vertical());

                // Next Clip Button
                if ui.button(RichText::new(egui_phosphor::regular::SKIP_FORWARD).size(14.0)).on_hover_text("Next Clip in Bin (])").clicked() {
                    action = Some(TransportAction::NextClip);
                }

                if total_clips > 0 {
                    ui.label(RichText::new(format!("{}/{}", clip_index + 1, total_clips)).small().color(TEXT_MUTED));
                }

                // Prev Clip Button
                if ui.button(RichText::new(egui_phosphor::regular::SKIP_BACK).size(14.0)).on_hover_text("Previous Clip in Bin ([)").clicked() {
                    action = Some(TransportAction::PrevClip);
                }

                ui.add(egui::Separator::default().vertical());

                // Inline Flag picker (Pick / Reject)
                let mut flag = player.clip.flag;
                if flag_picker(ui, &mut flag) {
                    action = Some(TransportAction::UpdateFlag(flag));
                }

                // Inline Rating stars
                let mut rating = player.clip.rating;
                if rating_stars(ui, &mut rating) {
                    action = Some(TransportAction::UpdateRating(rating));
                }
            });
        });
    });

    action
}