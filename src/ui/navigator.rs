use egui::{Color32, RichText, ScrollArea, Ui};
use crate::db::models::{CatalogStats, SmartCollection, Tag};
use super::theme::*;

pub enum NavAction {
    SelectCollection(SmartCollection),
    CreateTag(String, String),
    DeleteTag(i64),
}

pub fn render_navigator(
    ui: &mut Ui,
    current_collection: &SmartCollection,
    stats: &CatalogStats,
    containers: &[(String, usize)],
    codecs: &[(String, usize)],
    cameras: &[(String, usize)],
    folders: &[String],
    tags: &[Tag],
    new_tag_name: &mut String,
) -> Option<NavAction> {
    let mut action = None;

    ScrollArea::vertical().show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 4.0;

        // SECTION: Library
        ui.label(RichText::new("LIBRARY").small().strong().color(TEXT_MUTED));

        let is_all = matches!(current_collection, SmartCollection::All);
        if nav_item(ui, "🎬 All Videos", stats.total_clips, is_all) {
            action = Some(NavAction::SelectCollection(SmartCollection::All));
        }

        let is_picks = matches!(current_collection, SmartCollection::Picks);
        if nav_item(ui, "🚩 Picks", 0, is_picks) {
            action = Some(NavAction::SelectCollection(SmartCollection::Picks));
        }

        let is_rej = matches!(current_collection, SmartCollection::Rejected);
        if nav_item(ui, "❌ Rejected", 0, is_rej) {
            action = Some(NavAction::SelectCollection(SmartCollection::Rejected));
        }

        let is_fav = matches!(current_collection, SmartCollection::Favorites);
        if nav_item(ui, "⭐ Favorites (4★+)", 0, is_fav) {
            action = Some(NavAction::SelectCollection(SmartCollection::Favorites));
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // SECTION: Smart Collections
        ui.label(RichText::new("SMART COLLECTIONS").small().strong().color(TEXT_MUTED));

        let is_10bit = matches!(current_collection, SmartCollection::TenBitPlus);
        if nav_item(ui, "🎨 10-bit / Deep Color", stats.ten_bit_clips, is_10bit) {
            action = Some(NavAction::SelectCollection(SmartCollection::TenBitPlus));
        }

        let is_4k = matches!(current_collection, SmartCollection::FourKPlus);
        if nav_item(ui, "📺 4K+ UHD", stats.four_k_clips, is_4k) {
            action = Some(NavAction::SelectCollection(SmartCollection::FourKPlus));
        }

        let is_braw = matches!(current_collection, SmartCollection::BRAW);
        if nav_item(ui, "🎞️ Blackmagic RAW", stats.braw_clips, is_braw) {
            action = Some(NavAction::SelectCollection(SmartCollection::BRAW));
        }

        let is_prores = matches!(current_collection, SmartCollection::ProRes);
        if nav_item(ui, "🍎 Apple ProRes", stats.prores_clips, is_prores) {
            action = Some(NavAction::SelectCollection(SmartCollection::ProRes));
        }

        let is_h265 = matches!(current_collection, SmartCollection::H265);
        if nav_item(ui, "⚡ H.265 / HEVC", stats.h265_clips, is_h265) {
            action = Some(NavAction::SelectCollection(SmartCollection::H265));
        }

        let is_h264 = matches!(current_collection, SmartCollection::H264);
        if nav_item(ui, "📹 H.264 / AVC", stats.h264_clips, is_h264) {
            action = Some(NavAction::SelectCollection(SmartCollection::H264));
        }

        let is_hfr = matches!(current_collection, SmartCollection::HighFrameRate);
        if nav_item(ui, "⏱️ High FPS (≥50fps)", 0, is_hfr) {
            action = Some(NavAction::SelectCollection(SmartCollection::HighFrameRate));
        }

        if !containers.is_empty() {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(RichText::new("CONTAINERS").small().strong().color(TEXT_MUTED));
            for (container_name, count) in containers {
                let is_sel = matches!(current_collection, SmartCollection::Container(c) if c.eq_ignore_ascii_case(container_name));
                if nav_item(ui, &format!("📦 {}", container_name), *count, is_sel) {
                    action = Some(NavAction::SelectCollection(SmartCollection::Container(container_name.clone())));
                }
            }
        }

        if !codecs.is_empty() {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(RichText::new("CODECS").small().strong().color(TEXT_MUTED));
            for (codec_name, count) in codecs {
                let is_sel = matches!(current_collection, SmartCollection::Codec(c) if c.eq_ignore_ascii_case(codec_name));
                if nav_item(ui, &format!("📼 {}", codec_name), *count, is_sel) {
                    action = Some(NavAction::SelectCollection(SmartCollection::Codec(codec_name.clone())));
                }
            }
        }

        if !cameras.is_empty() {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(RichText::new("CAMERAS").small().strong().color(TEXT_MUTED));
            for (cam_name, count) in cameras {
                let is_sel = matches!(current_collection, SmartCollection::Camera(c) if c == cam_name);
                if nav_item(ui, &format!("📷 {}", cam_name), *count, is_sel) {
                    action = Some(NavAction::SelectCollection(SmartCollection::Camera(cam_name.clone())));
                }
            }
        }

        if !folders.is_empty() {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(RichText::new("FOLDERS").small().strong().color(TEXT_MUTED));
            for folder in folders {
                let short_name = std::path::Path::new(folder)
                    .file_name()
                    .map_or(folder.as_str(), |s| s.to_str().unwrap_or(folder.as_str()));
                let is_sel = matches!(current_collection, SmartCollection::Folder(f) if f == folder);
                if nav_item(ui, &format!("📁 {}", short_name), 0, is_sel) {
                    action = Some(NavAction::SelectCollection(SmartCollection::Folder(folder.clone())));
                }
            }
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // SECTION: Tags
        ui.label(RichText::new("TAGS & KEYWORDS").small().strong().color(TEXT_MUTED));

        for tag in tags {
            let is_sel = matches!(current_collection, SmartCollection::Tag(t) if t == &tag.name);
            if nav_item(ui, &format!("🏷️ {}", tag.name), tag.clip_count, is_sel) {
                action = Some(NavAction::SelectCollection(SmartCollection::Tag(tag.name.clone())));
            }
        }

        // Add New Tag Input
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(new_tag_name).hint_text("New tag...").desired_width(120.0));
            if ui.button("+").clicked() && !new_tag_name.trim().is_empty() {
                action = Some(NavAction::CreateTag(new_tag_name.trim().to_string(), "#5B8DEF".to_string()));
                new_tag_name.clear();
            }
        });
    });

    action
}

fn nav_item(ui: &mut Ui, label: &str, count: usize, selected: bool) -> bool {
    let mut clicked = false;
    let bg = if selected { BG_CARD_SELECTED } else { Color32::TRANSPARENT };

    let frame = egui::Frame::none()
        .fill(bg)
        .rounding(4.0)
        .inner_margin(egui::Margin::symmetric(6.0, 4.0));

    frame.show(ui, |ui| {
        let (rect, resp) = ui.allocate_exact_size(
            egui::Vec2::new(ui.available_width(), 20.0),
            egui::Sense::click(),
        );

        if resp.clicked() {
            clicked = true;
        }

        let text_color = if selected { TEXT_PRIMARY } else { TEXT_SECONDARY };
        ui.painter().text(
            rect.left_center() + egui::Vec2::new(4.0, 0.0),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(12.5),
            text_color,
        );

        if count > 0 {
            ui.painter().text(
                rect.right_center() - egui::Vec2::new(4.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                count.to_string(),
                egui::FontId::proportional(11.0),
                TEXT_MUTED,
            );
        }
    });

    clicked
}
