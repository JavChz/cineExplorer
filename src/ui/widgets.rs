use egui::{Color32, Rounding, Stroke, Ui, Vec2};
use crate::db::models::PickFlag;
use super::theme::*;

pub fn badge(ui: &mut Ui, text: &str, bg: Color32, fg: Color32) {
    let padding = Vec2::new(5.0, 2.0);
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(10.5),
        fg,
    );
    let size = galley.size() + padding * 2.0;
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

    ui.painter().rect_filled(rect, Rounding::same(3.0), bg);
    let text_pos = rect.min + padding;
    ui.painter().galley(text_pos, galley, fg);
}

pub fn kbd_pill(ui: &mut Ui, text: &str) {
    let padding = Vec2::new(4.0, 1.5);
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::monospace(10.0),
        TEXT_SECONDARY,
    );
    let size = galley.size() + padding * 2.0;
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());

    ui.painter().rect_filled(rect, Rounding::same(3.0), Color32::from_rgb(36, 40, 50));
    ui.painter().rect_stroke(rect, Rounding::same(3.0), Stroke::new(1.0_f32, Color32::from_rgb(55, 60, 75)));
    let text_pos = rect.min + padding;
    ui.painter().galley(text_pos, galley, TEXT_SECONDARY);
}

pub fn rating_stars(ui: &mut Ui, rating: &mut u8) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for star in 1..=5 {
            let filled = *rating >= star;
            let icon = if filled {
                egui_phosphor::fill::STAR
            } else {
                egui_phosphor::regular::STAR
            };
            let color = if filled { ACCENT_GOLD } else { Color32::from_rgb(70, 75, 88) };

            let btn = egui::Button::new(
                egui::RichText::new(icon).color(color).size(14.0)
            )
            .frame(false)
            .min_size(Vec2::new(15.0, 15.0));

            if ui.add(btn).on_hover_text(format!("Rate {} Star{}", star, if star > 1 { "s" } else { "" })).clicked() {
                if *rating == star {
                    *rating = 0;
                } else {
                    *rating = star;
                }
                changed = true;
            }
        }
    });
    changed
}

pub fn flag_picker(ui: &mut Ui, flag: &mut PickFlag) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;

        // Pick button
        let is_pick = *flag == PickFlag::Picked;
        let pick_color = if is_pick { ACCENT_GREEN } else { TEXT_MUTED };
        let pick_bg = if is_pick { Color32::from_rgb(25, 48, 35) } else { Color32::TRANSPARENT };
        let icon = if is_pick { egui_phosphor::fill::FLAG } else { egui_phosphor::regular::FLAG };
        let btn_pick = egui::Button::new(
            egui::RichText::new(icon)
                .color(pick_color)
                .size(13.5),
        )
        .fill(pick_bg)
        .stroke(if is_pick { Stroke::new(1.0_f32, ACCENT_GREEN) } else { Stroke::NONE })
        .min_size(Vec2::new(20.0, 18.0));

        if ui.add(btn_pick).on_hover_text("Pick (P)").clicked() {
            *flag = if is_pick { PickFlag::Unflagged } else { PickFlag::Picked };
            changed = true;
        }

        // Reject button
        let is_rej = *flag == PickFlag::Rejected;
        let rej_color = if is_rej { ACCENT_RED } else { TEXT_MUTED };
        let rej_bg = if is_rej { Color32::from_rgb(50, 25, 28) } else { Color32::TRANSPARENT };
        let btn_rej = egui::Button::new(
            egui::RichText::new(egui_phosphor::regular::X)
                .color(rej_color)
                .size(13.5),
        )
        .fill(rej_bg)
        .stroke(if is_rej { Stroke::new(1.0_f32, ACCENT_RED) } else { Stroke::NONE })
        .min_size(Vec2::new(20.0, 18.0));

        if ui.add(btn_rej).on_hover_text("Reject (X)").clicked() {
            *flag = if is_rej { PickFlag::Unflagged } else { PickFlag::Rejected };
            changed = true;
        }
    });
    changed
}

pub fn color_dot(ui: &mut Ui, color_name: &str) {
    let color = match color_name.to_lowercase().as_str() {
        "red" => ACCENT_RED,
        "orange" => ACCENT_ORANGE,
        "yellow" => ACCENT_GOLD,
        "green" => ACCENT_GREEN,
        "blue" => ACCENT_BLUE,
        "purple" => ACCENT_PURPLE,
        _ => return,
    };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

pub fn color_label_picker(ui: &mut Ui, current_color: &mut String) -> bool {
    let mut changed = false;
    let colors = [
        ("", "None", Color32::from_rgb(60, 64, 76)),
        ("red", "Red", ACCENT_RED),
        ("orange", "Orange", ACCENT_ORANGE),
        ("yellow", "Yellow", ACCENT_GOLD),
        ("green", "Green", ACCENT_GREEN),
        ("blue", "Blue", ACCENT_BLUE),
        ("purple", "Purple", ACCENT_PURPLE),
    ];

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        for (code, name, c) in colors {
            let is_selected = current_color == code;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::click());
            if resp.clicked() {
                *current_color = code.to_string();
                changed = true;
            }
            ui.painter().circle_filled(rect.center(), 6.5, c);
            if is_selected {
                ui.painter().circle_stroke(rect.center(), 8.0, Stroke::new(1.5_f32, Color32::WHITE));
            }
            resp.on_hover_text(name);
        }
    });
    changed
}
