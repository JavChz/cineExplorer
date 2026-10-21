use egui::{Color32, RichText, Ui, Vec2};
use crate::playback::VideoPlayer;
use crate::ui::theme::*;

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
    ClosePlayer,
    PlayExternal,
}

pub fn render_transport_bar(ui: &mut Ui, player: &mut VideoPlayer) -> Option<TransportAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;

        // Play / Pause button
        let play_icon = if player.is_playing { "⏸" } else { "▶" };
        let play_btn = egui::Button::new(RichText::new(play_icon).size(18.0).color(Color32::WHITE))
            .fill(ACCENT_BLUE)
            .min_size(Vec2::new(36.0, 28.0));

        if ui.add(play_btn).on_hover_text("Play/Pause (Space)").clicked() {
            action = Some(TransportAction::TogglePlay);
        }

        // Step buttons
        if ui.button("⏮").on_hover_text("Step 10 Frames Back").clicked() {
            action = Some(TransportAction::Step10Backward);
        }
        if ui.button("◀").on_hover_text("Step 1 Frame Back (Left)").clicked() {
            action = Some(TransportAction::StepBackward);
        }
        if ui.button("▶").on_hover_text("Step 1 Frame Forward (Right)").clicked() {
            action = Some(TransportAction::StepForward);
        }
        if ui.button("⏭").on_hover_text("Step 10 Frames Forward").clicked() {
            action = Some(TransportAction::Step10Forward);
        }

        ui.separator();

        // Timecode Display: Current / Total
        let tc_str = format!("{} / {}", player.current_timecode(), player.clip.duration_tc());
        ui.label(RichText::new(tc_str).monospace().strong().size(13.0).color(TEXT_PRIMARY));

        ui.separator();

        // Scrubber Timeline Slider
        let max_sec = player.clip.duration_seconds.max(0.1);
        let mut seek_pos = player.playhead;
        ui.spacing_mut().slider_width = (ui.available_width() - 220.0).max(100.0);
        let slider_resp = ui.add(
            egui::Slider::new(&mut seek_pos, 0.0..=max_sec).show_value(false)
        );

        if slider_resp.changed() {
            action = Some(TransportAction::Seek(seek_pos));
        }

        // In / Out point controls
        if ui.small_button("[ I ]").on_hover_text("Mark In Point (I)").clicked() {
            action = Some(TransportAction::SetInPoint);
        }
        if ui.small_button("[ O ]").on_hover_text("Mark Out Point (O)").clicked() {
            action = Some(TransportAction::SetOutPoint);
        }

        // Loop button
        let loop_text = if player.looping { "🔁" } else { "➡" };
        if ui.selectable_label(player.looping, loop_text).on_hover_text("Loop Playback").clicked() {
            player.looping = !player.looping;
        }

        // External Player launch
        if ui.button("🎬 External").on_hover_text("Launch in Default / External Player").clicked() {
            action = Some(TransportAction::PlayExternal);
        }

        // Close Player
        if ui.button("✖ Close").on_hover_text("Back to Grid (Esc)").clicked() {
            action = Some(TransportAction::ClosePlayer);
        }
    });

    action
}
