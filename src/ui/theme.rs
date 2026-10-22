use egui::{Color32, Rounding, Stroke, Visuals};

pub const BG_APP: Color32 = Color32::from_rgb(18, 19, 23);
pub const BG_PANEL: Color32 = Color32::from_rgb(24, 26, 32);
pub const BG_CARD: Color32 = Color32::from_rgb(32, 35, 43);
pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(42, 46, 56);
pub const BG_CARD_SELECTED: Color32 = Color32::from_rgb(46, 54, 72);
pub const BORDER_CARD_SELECTED: Color32 = Color32::from_rgb(90, 140, 255);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 242, 245);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(155, 162, 175);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(105, 112, 125);

pub const ACCENT_BLUE: Color32 = Color32::from_rgb(66, 133, 244);
pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(245, 130, 32);
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(52, 168, 83);
pub const ACCENT_RED: Color32 = Color32::from_rgb(234, 67, 53);
pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(168, 85, 247);
pub const ACCENT_GOLD: Color32 = Color32::from_rgb(251, 188, 5);

// Badge Colors
pub const BADGE_10BIT_BG: Color32 = Color32::from_rgb(40, 30, 80);
pub const BADGE_10BIT_FG: Color32 = Color32::from_rgb(180, 150, 255);
pub const BADGE_4K_BG: Color32 = Color32::from_rgb(20, 50, 60);
pub const BADGE_4K_FG: Color32 = Color32::from_rgb(70, 210, 230);
pub const BADGE_PRORES_BG: Color32 = Color32::from_rgb(50, 35, 20);
pub const BADGE_PRORES_FG: Color32 = Color32::from_rgb(245, 160, 60);
pub const BADGE_BRAW_BG: Color32 = Color32::from_rgb(55, 20, 25);
pub const BADGE_BRAW_FG: Color32 = Color32::from_rgb(245, 80, 100);

pub fn apply_theme(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Fill);
    ctx.set_fonts(fonts);

    let mut visuals = Visuals::dark();
    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_PANEL;
    visuals.faint_bg_color = BG_CARD;
    visuals.extreme_bg_color = BG_APP;
    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.inactive.bg_fill = BG_CARD;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_SECONDARY);
    visuals.widgets.hovered.bg_fill = BG_CARD_HOVER;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.active.bg_fill = ACCENT_BLUE;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.window_rounding = Rounding::same(8.0);
    visuals.widgets.noninteractive.rounding = Rounding::same(6.0);
    visuals.widgets.inactive.rounding = Rounding::same(6.0);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);
    visuals.widgets.active.rounding = Rounding::same(6.0);

    ctx.set_visuals(visuals);
}
