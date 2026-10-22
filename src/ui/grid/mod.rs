pub mod card;

use std::collections::HashSet;
use egui::{Color32, RichText, ScrollArea, Ui, Vec2};
use crate::db::models::VideoRecord;
use crate::thumbnails::ThumbnailManager;
use crate::ui::theme::*;
use card::{render_video_card, CardAction};

pub enum GridAction {
    SelectOnly(i64),
    ToggleSelect(i64),
    SelectRange(i64),
    OpenPlayer(Box<VideoRecord>),
    PlayExternal(String),
    RevealInExplorer(String),
    UpdateRating(i64, u8),
    UpdateFlag(i64, crate::db::models::PickFlag),
    UpdateColor(i64, String),
    DeleteVideo(i64),
    StartIngest,
    StartImport,
}

pub fn render_grid(
    ui: &mut Ui,
    videos: &[VideoRecord],
    selected_ids: &HashSet<i64>,
    card_width: f32,
    thumb_mgr: &mut ThumbnailManager,
) -> Option<GridAction> {
    if videos.is_empty() {
        let mut empty_act = None;
        ui.centered_and_justified(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(egui_phosphor::regular::FILM_STRIP)
                        .size(48.0)
                        .color(TEXT_MUTED),
                );
                ui.add_space(8.0);
                ui.label(RichText::new("No video clips in this view").strong().size(17.0));
                ui.label(
                    RichText::new("Drag and drop files/folders here, or choose an ingest or import option below.")
                        .color(TEXT_SECONDARY)
                        .size(13.0),
                );
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    let ingest_btn = egui::Button::new(
                        RichText::new(format!("{} Ingest from SD Card / Media...", egui_phosphor::regular::DOWNLOAD_SIMPLE))
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(ACCENT_BLUE)
                    .min_size(Vec2::new(180.0, 32.0));

                    if ui.add(ingest_btn).clicked() {
                        empty_act = Some(GridAction::StartIngest);
                    }

                    let import_btn = egui::Button::new(
                        RichText::new(format!("{} Import Folder In-Place...", egui_phosphor::regular::FOLDER_PLUS))
                            .color(TEXT_PRIMARY),
                    )
                    .min_size(Vec2::new(160.0, 32.0));

                    if ui.add(import_btn).clicked() {
                        empty_act = Some(GridAction::StartImport);
                    }
                });
            });
        });
        return empty_act;
    }

    let mut action = None;
    let available_w = ui.available_width().max(card_width);
    let spacing = 12.0;
    let col_w = card_width + spacing;
    let num_cols = ((available_w + spacing) / col_w).floor().max(1.0) as usize;

    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing = Vec2::new(spacing, spacing);

        let rows = videos.chunks(num_cols);
        for chunk in rows {
            ui.horizontal(|ui| {
                for video in chunk {
                    let is_sel = selected_ids.contains(&video.id);
                    if let Some(card_act) = render_video_card(ui, video, is_sel, card_width, thumb_mgr) {
                        match card_act {
                            CardAction::Select { toggle, range } => {
                                if range {
                                    action = Some(GridAction::SelectRange(video.id));
                                } else if toggle {
                                    action = Some(GridAction::ToggleSelect(video.id));
                                } else {
                                    action = Some(GridAction::SelectOnly(video.id));
                                }
                            }
                            CardAction::OpenPlayer => {
                                action = Some(GridAction::OpenPlayer(Box::new(video.clone())));
                            }
                            CardAction::PlayExternal => {
                                action = Some(GridAction::PlayExternal(video.file_path.clone()));
                            }
                            CardAction::RevealInExplorer => {
                                action = Some(GridAction::RevealInExplorer(video.file_path.clone()));
                            }
                            CardAction::UpdateRating(r) => {
                                action = Some(GridAction::UpdateRating(video.id, r));
                            }
                            CardAction::UpdateFlag(f) => {
                                action = Some(GridAction::UpdateFlag(video.id, f));
                            }
                            CardAction::UpdateColor(c) => {
                                action = Some(GridAction::UpdateColor(video.id, c));
                            }
                            CardAction::DeleteVideo => {
                                action = Some(GridAction::DeleteVideo(video.id));
                            }
                        }
                    }
                }
            });
        }
    });

    action
}