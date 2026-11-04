use egui::{Color32, RichText, ScrollArea, Stroke, Ui, Vec2};
use crate::db::models::{CatalogStats, SmartCollection, Tag};
use super::theme::*;

pub enum NavAction {
    SelectCollection(SmartCollection),
    CreateTag(String, String),
    DeleteTag(i64),
    CloseNavigator,
}

#[allow(clippy::too_many_arguments)]
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

    // Docked Panel Header
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("BINS & COLLECTIONS")
                .size(11.0)
                .strong()
                .color(TEXT_MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close_btn = egui::Button::new(
                RichText::new(egui_phosphor::regular::CARET_DOUBLE_LEFT)
                    .size(11.5)
                    .color(TEXT_MUTED),
            )
            .frame(false)
            .min_size(Vec2::new(18.0, 18.0));
            if ui
                .add(close_btn)
                .on_hover_text("Collapse Navigator (Ctrl+B)")
                .clicked()
            {
                action = Some(NavAction::CloseNavigator);
            }
        });
    });
    ui.separator();

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;

            // SECTION: Library
            ui.add_space(2.0);
            ui.label(RichText::new("LIBRARY").size(10.5).strong().color(TEXT_MUTED));

            let is_all = matches!(current_collection, SmartCollection::All);
            if nav_item(
                ui,
                egui_phosphor::regular::FILM_STRIP,
                ACCENT_BLUE,
                "All Videos",
                stats.total_clips,
                is_all,
            ) {
                action = Some(NavAction::SelectCollection(SmartCollection::All));
            }

            let is_picks = matches!(current_collection, SmartCollection::Picks);
            if nav_item(
                ui,
                egui_phosphor::regular::FLAG,
                ACCENT_GREEN,
                "Picks",
                stats.picks_clips,
                is_picks,
            ) {
                action = Some(NavAction::SelectCollection(SmartCollection::Picks));
            }

            let is_rej = matches!(current_collection, SmartCollection::Rejected);
            if nav_item(
                ui,
                egui_phosphor::regular::X_CIRCLE,
                ACCENT_RED,
                "Rejected",
                stats.rejected_clips,
                is_rej,
            ) {
                action = Some(NavAction::SelectCollection(SmartCollection::Rejected));
            }

            let is_fav = matches!(current_collection, SmartCollection::Favorites);
            if nav_item(
                ui,
                egui_phosphor::regular::STAR,
                ACCENT_GOLD,
                "Favorites (4★+)",
                stats.favorites_clips,
                is_fav,
            ) {
                action = Some(NavAction::SelectCollection(SmartCollection::Favorites));
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            // SECTION: Smart Bins
            ui.label(
                RichText::new("SMART BINS")
                    .size(10.5)
                    .strong()
                    .color(TEXT_MUTED),
            );

            let is_log = matches!(current_collection, SmartCollection::LogFootage);
            if nav_item(
                ui,
                egui_phosphor::regular::PALETTE,
                BADGE_10BIT_FG,
                "Log Footage",
                stats.log_clips,
                is_log,
            ) {
                action = Some(NavAction::SelectCollection(SmartCollection::LogFootage));
            }

            let is_10bit = matches!(current_collection, SmartCollection::TenBitPlus);
            if nav_item(
                ui,
                egui_phosphor::regular::SLIDERS,
                BADGE_10BIT_FG,
                "10-bit / Deep Color",
                stats.ten_bit_clips,
                is_10bit,
            ) {
                action = Some(NavAction::SelectCollection(SmartCollection::TenBitPlus));
            }

            let is_4k = matches!(current_collection, SmartCollection::FourKPlus);
            if nav_item(
                ui,
                egui_phosphor::regular::MONITOR,
                BADGE_4K_FG,
                "4K+ UHD",
                stats.four_k_clips,
                is_4k,
            ) {
                action = Some(NavAction::SelectCollection(SmartCollection::FourKPlus));
            }

            let is_braw = matches!(current_collection, SmartCollection::BRAW);
            if nav_item(
                ui,
                egui_phosphor::regular::APERTURE,
                BADGE_BRAW_FG,
                "Blackmagic RAW",
                stats.braw_clips,
                is_braw,
            ) {
                action = Some(NavAction::SelectCollection(SmartCollection::BRAW));
            }

            let is_prores = matches!(current_collection, SmartCollection::ProRes);
            if nav_item(
                ui,
                egui_phosphor::regular::APPLE_LOGO,
                BADGE_PRORES_FG,
                "Apple ProRes",
                stats.prores_clips,
                is_prores,
            ) {
                action = Some(NavAction::SelectCollection(SmartCollection::ProRes));
            }

            let is_h265 = matches!(current_collection, SmartCollection::H265);
            if nav_item(
                ui,
                egui_phosphor::regular::LIGHTNING,
                ACCENT_ORANGE,
                "H.265 / HEVC",
                stats.h265_clips,
                is_h265,
            ) {
                action = Some(NavAction::SelectCollection(SmartCollection::H265));
            }

            let is_h264 = matches!(current_collection, SmartCollection::H264);
            if nav_item(
                ui,
                egui_phosphor::regular::VIDEO_CAMERA,
                TEXT_SECONDARY,
                "H.264 / AVC",
                stats.h264_clips,
                is_h264,
            ) {
                action = Some(NavAction::SelectCollection(SmartCollection::H264));
            }

            let is_hfr = matches!(current_collection, SmartCollection::HighFrameRate);
            if nav_item(
                ui,
                egui_phosphor::regular::GAUGE,
                ACCENT_BLUE,
                "High FPS (≥50fps)",
                stats.hfr_clips,
                is_hfr,
            ) {
                action = Some(NavAction::SelectCollection(SmartCollection::HighFrameRate));
            }

            if !containers.is_empty() {
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(
                    RichText::new("CONTAINERS")
                        .size(10.5)
                        .strong()
                        .color(TEXT_MUTED),
                );
                for (container_name, count) in containers {
                    let is_sel = matches!(current_collection, SmartCollection::Container(c) if c.eq_ignore_ascii_case(container_name));
                    if nav_item(
                        ui,
                        egui_phosphor::regular::PACKAGE,
                        TEXT_MUTED,
                        container_name,
                        *count,
                        is_sel,
                    ) {
                        action = Some(NavAction::SelectCollection(SmartCollection::Container(
                            container_name.clone(),
                        )));
                    }
                }
            }

            if !codecs.is_empty() {
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(RichText::new("CODECS").size(10.5).strong().color(TEXT_MUTED));
                for (codec_name, count) in codecs {
                    let is_sel = matches!(current_collection, SmartCollection::Codec(c) if c.eq_ignore_ascii_case(codec_name));
                    if nav_item(
                        ui,
                        egui_phosphor::regular::FILE_VIDEO,
                        TEXT_MUTED,
                        codec_name,
                        *count,
                        is_sel,
                    ) {
                        action = Some(NavAction::SelectCollection(SmartCollection::Codec(
                            codec_name.clone(),
                        )));
                    }
                }
            }

            if !cameras.is_empty() {
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(RichText::new("CAMERAS").size(10.5).strong().color(TEXT_MUTED));
                for (cam_name, count) in cameras {
                    let is_sel =
                        matches!(current_collection, SmartCollection::Camera(c) if c == cam_name);
                    if nav_item(
                        ui,
                        egui_phosphor::regular::CAMERA,
                        TEXT_MUTED,
                        cam_name,
                        *count,
                        is_sel,
                    ) {
                        action = Some(NavAction::SelectCollection(SmartCollection::Camera(
                            cam_name.clone(),
                        )));
                    }
                }
            }

            if !folders.is_empty() {
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(RichText::new("FOLDERS").size(10.5).strong().color(TEXT_MUTED));
                for folder in folders {
                    let short_name = std::path::Path::new(folder)
                        .file_name()
                        .map_or(folder.as_str(), |s| s.to_str().unwrap_or(folder.as_str()));
                    let is_sel =
                        matches!(current_collection, SmartCollection::Folder(f) if f == folder);
                    if nav_item(
                        ui,
                        egui_phosphor::regular::FOLDER,
                        TEXT_MUTED,
                        short_name,
                        0,
                        is_sel,
                    ) {
                        action = Some(NavAction::SelectCollection(SmartCollection::Folder(
                            folder.clone(),
                        )));
                    }
                }
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            // SECTION: Tags
            ui.label(
                RichText::new("TAGS & KEYWORDS")
                    .size(10.5)
                    .strong()
                    .color(TEXT_MUTED),
            );

            for tag in tags {
                let is_sel = matches!(current_collection, SmartCollection::Tag(t) if t == &tag.name);
                if nav_item(
                    ui,
                    egui_phosphor::regular::TAG,
                    ACCENT_BLUE,
                    &tag.name,
                    tag.clip_count,
                    is_sel,
                ) {
                    action = Some(NavAction::SelectCollection(SmartCollection::Tag(
                        tag.name.clone(),
                    )));
                }
            }

            // Add New Tag Input
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(new_tag_name)
                        .hint_text("New keyword...")
                        .desired_width((ui.available_width() - 32.0).max(80.0)),
                );
                if ui.button("+").clicked() && !new_tag_name.trim().is_empty() {
                    action = Some(NavAction::CreateTag(
                        new_tag_name.trim().to_string(),
                        "#5B8DEF".to_string(),
                    ));
                    new_tag_name.clear();
                }
            });
            ui.add_space(10.0);
        });

    action
}

fn nav_item(
    ui: &mut Ui,
    icon: &str,
    icon_color: Color32,
    label: &str,
    count: usize,
    selected: bool,
) -> bool {
    let avail_w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail_w, 24.0), egui::Sense::click());

    let hovered = resp.hovered();
    let bg = if selected {
        BG_CARD_SELECTED
    } else if hovered {
        BG_CARD_HOVER
    } else {
        Color32::TRANSPARENT
    };

    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, 4.0, bg);
    }

    // Left accent bar when selected
    if selected {
        ui.painter().line_segment(
            [
                egui::pos2(rect.min.x + 1.5, rect.min.y + 4.0),
                egui::pos2(rect.min.x + 1.5, rect.max.y - 4.0),
            ],
            Stroke::new(2.5_f32, ACCENT_BLUE),
        );
    }

    // Icon
    ui.painter().text(
        rect.left_center() + Vec2::new(8.0, 0.0),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::proportional(13.0),
        if selected { ACCENT_BLUE } else { icon_color },
    );

    // Truncate label so it never overlaps the right-side count badge
    let count_reserve = if count > 0 { 34.0 } else { 8.0 };
    let max_label_w = (rect.width() - 28.0 - count_reserve).max(40.0);
    let text_color = if selected || hovered {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };

    let label_clip = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + 26.0, rect.min.y),
        egui::pos2(rect.min.x + 26.0 + max_label_w, rect.max.y),
    );
    ui.painter().with_clip_rect(label_clip).text(
        egui::pos2(rect.min.x + 26.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        text_color,
    );

    // Count badge
    if count > 0 {
        ui.painter().text(
            rect.right_center() - Vec2::new(6.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            count.to_string(),
            egui::FontId::monospace(10.5),
            if selected { TEXT_SECONDARY } else { TEXT_MUTED },
        );
    }

    resp.clicked()
}
