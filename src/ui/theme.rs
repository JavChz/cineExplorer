use std::sync::Arc;
use egui::{Color32, FontFamily, FontId, RichText, Rounding, Stroke, Visuals};

pub const BG_APP: Color32 = Color32::from_rgb(15, 16, 20);
pub const BG_PANEL: Color32 = Color32::from_rgb(22, 24, 30);
pub const BG_PANEL_HEADER: Color32 = Color32::from_rgb(28, 31, 38);
pub const BG_CARD: Color32 = Color32::from_rgb(30, 33, 41);
pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(38, 42, 52);
pub const BG_CARD_SELECTED: Color32 = Color32::from_rgb(42, 50, 68);
pub const BORDER_PANEL: Color32 = Color32::from_rgb(38, 42, 52);
pub const BORDER_CARD: Color32 = Color32::from_rgb(45, 49, 60);
pub const BORDER_CARD_SELECTED: Color32 = Color32::from_rgb(82, 138, 255);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 242, 246);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 168, 182);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(105, 112, 126);

pub const ACCENT_BLUE: Color32 = Color32::from_rgb(59, 130, 246);
pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(245, 140, 38);
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(46, 175, 90);
pub const ACCENT_RED: Color32 = Color32::from_rgb(235, 68, 58);
pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(168, 85, 247);
pub const ACCENT_GOLD: Color32 = Color32::from_rgb(250, 190, 15);

// Badge Colors
pub const BADGE_10BIT_BG: Color32 = Color32::from_rgb(40, 30, 80);
pub const BADGE_10BIT_FG: Color32 = Color32::from_rgb(190, 160, 255);
pub const BADGE_4K_BG: Color32 = Color32::from_rgb(18, 48, 58);
pub const BADGE_4K_FG: Color32 = Color32::from_rgb(80, 215, 235);
pub const BADGE_PRORES_BG: Color32 = Color32::from_rgb(50, 35, 20);
pub const BADGE_PRORES_FG: Color32 = Color32::from_rgb(245, 160, 60);
pub const BADGE_BRAW_BG: Color32 = Color32::from_rgb(55, 20, 25);
pub const BADGE_BRAW_FG: Color32 = Color32::from_rgb(245, 80, 100);

pub const PHOSPHOR_FILL_FAMILY: &str = "phosphor-fill";

pub fn phosphor_fill_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(Arc::from(PHOSPHOR_FILL_FAMILY)))
}

pub fn fill_icon_text(icon: &str, size: f32, color: Color32) -> RichText {
    RichText::new(icon)
        .font(phosphor_fill_font(size))
        .color(color)
}

pub fn apply_theme(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Register Regular icons as primary icon font in Proportional and Monospace
    fonts.font_data.insert(
        "phosphor-regular".into(),
        egui_phosphor::Variant::Regular.font_data(),
    );
    if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
        prop.insert(1, "phosphor-regular".into());
    }
    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono.push("phosphor-regular".into());
    }

    // Register Fill icons under a separate named font family so it never overwrites Regular
    fonts.font_data.insert(
        PHOSPHOR_FILL_FAMILY.into(),
        egui_phosphor::Variant::Fill.font_data(),
    );
    let mut fill_family = vec![PHOSPHOR_FILL_FAMILY.into()];
    if let Some(prop) = fonts.families.get(&FontFamily::Proportional) {
        for f in prop {
            fill_family.push(f.clone());
        }
    }
    fonts.families.insert(
        FontFamily::Name(Arc::from(PHOSPHOR_FILL_FAMILY)),
        fill_family,
    );

    ctx.set_fonts(fonts);

    let mut visuals = Visuals::dark();
    visuals.panel_fill = BG_PANEL;
    visuals.window_fill = BG_PANEL;
    visuals.faint_bg_color = BG_CARD;
    visuals.extreme_bg_color = BG_APP;
    visuals.widgets.noninteractive.bg_fill = BG_PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_PANEL);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.inactive.bg_fill = BG_CARD;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(44, 48, 60));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_SECONDARY);
    visuals.widgets.hovered.bg_fill = BG_CARD_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(70, 78, 96));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.active.bg_fill = ACCENT_BLUE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT_BLUE);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.window_rounding = Rounding::same(6.0);
    visuals.widgets.noninteractive.rounding = Rounding::same(4.0);
    visuals.widgets.inactive.rounding = Rounding::same(4.0);
    visuals.widgets.hovered.rounding = Rounding::same(4.0);
    visuals.widgets.active.rounding = Rounding::same(4.0);

    ctx.set_visuals(visuals);
}
