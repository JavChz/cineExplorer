use egui::{Color32, RichText, Rounding, Ui, Vec2};
use crate::db::models::{PickFlag, VideoRecord};
use crate::thumbnails::ThumbnailManager;
use crate::ui::theme::*;
use crate::ui::widgets::*;

pub enum InspectorAction {
    OpenPlayer(Box<VideoRecord>),
    UpdateRating(u8),
    UpdateFlag(PickFlag),
    UpdateColor(String),
    UpdateNotes(String),
    UpdateSlate {
        scene: String,
        shot: String,
        take: String,
        reel: String,
        location: String,
        camera_unit: String,
    },
    AddTag(String),
    RemoveTag(String),
    PlayExternal,
    RevealInExplorer,
    OpenExportDialog,
    DeleteVideo,
    UpdateLogProfile(crate::media::color_lut::LogProfile),
    RescanMetadata(i64),
}

#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
pub fn render_inspector_content(
    ui: &mut Ui,
    video: &VideoRecord,
    thumb_mgr: &mut ThumbnailManager,
    edit_scene: &mut String,
    edit_shot: &mut String,
    edit_take: &mut String,
    edit_reel: &mut String,
    edit_location: &mut String,
    edit_unit: &mut String,
    edit_notes: &mut String,
    new_clip_tag: &mut String,
) -> Option<InspectorAction> {
    let mut action = None;

    ui.spacing_mut().item_spacing.y = 7.0;

    // Header Title & Action Buttons
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&video.file_name)
                .strong()
                .size(13.5)
                .color(TEXT_PRIMARY),
        );
    });

    ui.horizontal(|ui| {
        let play_btn = egui::Button::new(
            RichText::new(format!("{} Player", egui_phosphor::regular::PLAY))
                .size(11.5)
                .color(Color32::WHITE),
        )
        .fill(ACCENT_BLUE);

        if ui.add(play_btn).on_hover_text("Open in Integrated Player (Enter)").clicked() {
            action = Some(InspectorAction::OpenPlayer(Box::new(video.clone())));
        }

        if ui.button(RichText::new(format!("{} Export", egui_phosphor::regular::EXPORT)).size(11.5)).on_hover_text("Export to NLE").clicked() {
            action = Some(InspectorAction::OpenExportDialog);
        }

        if ui.button(RichText::new(format!("{} Ext", egui_phosphor::regular::ARROW_SQUARE_OUT)).size(11.5)).on_hover_text("Play in Default OS Player").clicked() {
            action = Some(InspectorAction::PlayExternal);
        }

        if ui.button(RichText::new(format!("{} Reveal", egui_phosphor::regular::FOLDER_OPEN)).size(11.5)).on_hover_text("Show in Explorer").clicked() {
            action = Some(InspectorAction::RevealInExplorer);
        }
    });

    // Poster Preview (clickable to open in player)
    let poster_w = ui.available_width().max(160.0);
    let poster_h = poster_w * (9.0 / 16.0);
    let (poster_rect, poster_resp) = ui.allocate_exact_size(Vec2::new(poster_w, poster_h), egui::Sense::click());

    if let Some(tex) = thumb_mgr.get_texture(video.id) {
        ui.painter().image(
            tex.id(),
            poster_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        ui.painter().rect_filled(poster_rect, Rounding::same(4.0), Color32::from_rgb(25, 27, 34));
    }

    if poster_resp.hovered() {
        ui.painter().rect_filled(poster_rect, Rounding::same(4.0), Color32::from_black_alpha(80));
        ui.painter().text(
            poster_rect.center(),
            egui::Align2::CENTER_CENTER,
            egui_phosphor::fill::PLAY,
            egui::FontId::proportional(28.0),
            Color32::WHITE,
        );
        poster_resp.show_tooltip_text("Click to open in player");
    }

    if poster_resp.clicked() {
        action = Some(InspectorAction::OpenPlayer(Box::new(video.clone())));
    }

    ui.separator();

    // SECTION 1: Ratings & Flags (Top priority for editors)
    ui.horizontal(|ui| {
        ui.label(RichText::new("Rating:").small().color(TEXT_MUTED));
        let mut rating = video.rating;
        if rating_stars(ui, &mut rating) {
            action = Some(InspectorAction::UpdateRating(rating));
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut flag = video.flag;
            if flag_picker(ui, &mut flag) {
                action = Some(InspectorAction::UpdateFlag(flag));
            }
            ui.label(RichText::new("Flag:").small().color(TEXT_MUTED));
        });
    });

    ui.horizontal(|ui| {
        ui.label(RichText::new("Color Label:").small().color(TEXT_MUTED));
        let mut color = video.color_label.clone();
        if color_label_picker(ui, &mut color) {
            action = Some(InspectorAction::UpdateColor(color));
        }
    });

    ui.separator();

    // SECTION 2: Production Slate (Editable)
    let slate_header = format!("{} Slate & Production", egui_phosphor::regular::FILM_SLATE);
    egui::CollapsingHeader::new(RichText::new(slate_header).strong().size(12.5))
        .default_open(true)
        .show(ui, |ui| {
            let mut slate_changed = false;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Scene:").size(11.0).color(TEXT_MUTED));
                if ui.add(egui::TextEdit::singleline(edit_scene).desired_width(55.0)).changed() { slate_changed = true; }

                ui.label(RichText::new("Shot:").size(11.0).color(TEXT_MUTED));
                if ui.add(egui::TextEdit::singleline(edit_shot).desired_width(55.0)).changed() { slate_changed = true; }

                ui.label(RichText::new("Take:").size(11.0).color(TEXT_MUTED));
                if ui.add(egui::TextEdit::singleline(edit_take).desired_width(45.0)).changed() { slate_changed = true; }
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new("Reel:").size(11.0).color(TEXT_MUTED));
                if ui.add(egui::TextEdit::singleline(edit_reel).desired_width(60.0)).changed() { slate_changed = true; }

                ui.label(RichText::new("Location:").size(11.0).color(TEXT_MUTED));
                if ui.add(egui::TextEdit::singleline(edit_location).desired_width(ui.available_width())).changed() { slate_changed = true; }
            });

            if slate_changed {
                action = Some(InspectorAction::UpdateSlate {
                    scene: edit_scene.clone(),
                    shot: edit_shot.clone(),
                    take: edit_take.clone(),
                    reel: edit_reel.clone(),
                    location: edit_location.clone(),
                    camera_unit: edit_unit.clone(),
                });
            }
        });

    ui.separator();

    // SECTION 3: Tags & Keywords
    let tags_header = format!("{} Tags & Keywords", egui_phosphor::regular::TAG);
    egui::CollapsingHeader::new(RichText::new(tags_header).strong().size(12.5))
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for tag in &video.tags {
                    ui.horizontal(|ui| {
                        badge(ui, tag, ACCENT_BLUE, Color32::WHITE);
                        if ui.small_button("×").clicked() {
                            action = Some(InspectorAction::RemoveTag(tag.clone()));
                        }
                    });
                }
            });

            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(new_clip_tag).hint_text("Add tag...").desired_width(120.0));
                if ui.button("+").clicked() && !new_clip_tag.trim().is_empty() {
                    action = Some(InspectorAction::AddTag(new_clip_tag.trim().to_string()));
                    new_clip_tag.clear();
                }
            });
        });

    ui.separator();

    // SECTION 4: Notes
    let notes_header = format!("{} Notes", egui_phosphor::regular::NOTE_PENCIL);
    egui::CollapsingHeader::new(RichText::new(notes_header).strong().size(12.5))
        .default_open(true)
        .show(ui, |ui| {
            if ui.add(
                egui::TextEdit::multiline(edit_notes)
                    .desired_rows(3)
                    .desired_width(ui.available_width()),
            ).changed() {
                action = Some(InspectorAction::UpdateNotes(edit_notes.clone()));
            }
        });

    ui.separator();

    // SECTION 5: Video Tech Specs
    let tech_header = format!("{} Video Tech Specs", egui_phosphor::regular::FILM_STRIP);
    egui::CollapsingHeader::new(RichText::new(tech_header).strong().size(12.5))
        .default_open(false)
        .show(ui, |ui| {
            spec_row(ui, "Container", &video.container.to_uppercase());
            spec_row(ui, "Codec", &video.codec_display_name());
            if !video.profile.is_empty() {
                spec_row(ui, "Profile", &video.profile);
            }
            spec_row(ui, "Resolution", &format!("{} x {} ({})", video.width, video.height, video.resolution_label()));
            spec_row(ui, "Frame Rate", &video.fps_label());
            spec_row(ui, "Total Frames", &video.frame_count.to_string());
            spec_row(ui, "Duration", &format!("{} ({:.2}s)", video.duration_tc(), video.duration_seconds));
            spec_row(ui, "Bit Depth", &format!("{}-bit", video.bit_depth));
            if !video.pixel_format.is_empty() {
                spec_row(ui, "Pixel Format", &video.pixel_format);
            }
            if !video.color_space.is_empty() {
                spec_row(ui, "Color Space", &video.color_space);
            }
            if !video.color_transfer.is_empty() {
                spec_row(ui, "Color Transfer", &video.color_transfer);
            }
            spec_row(ui, "Bitrate", &video.bitrate_formatted());
            spec_row(ui, "File Size", &video.file_size_formatted());
        });

    ui.separator();

    // SECTION 6: Camera & Shooting Info
    let cam_header = format!("{} Camera & Lens", egui_phosphor::regular::CAMERA);
    egui::CollapsingHeader::new(RichText::new(cam_header).strong().size(12.5))
        .default_open(true)
        .show(ui, |ui| {
            if !video.camera_make.is_empty() {
                spec_row(ui, "Make", &video.camera_make);
            }
            if !video.camera_model.is_empty() {
                spec_row(ui, "Model", &video.camera_model);
            }
            if !video.lens_model.is_empty() {
                spec_row(ui, "Lens", &video.lens_model);
            }
            if !video.focal_length.is_empty() {
                spec_row(ui, "Focal Length", &video.focal_length);
            }
            if !video.aperture.is_empty() {
                spec_row(ui, "Aperture", &video.aperture);
            }
            if video.iso > 0 {
                spec_row(ui, "ISO", &video.iso.to_string());
            }
            if !video.shutter_speed.is_empty() {
                spec_row(ui, "Shutter", &video.shutter_speed);
            }
            if !video.white_balance.is_empty() {
                spec_row(ui, "White Balance", &video.white_balance);
            }
            if let Some(ref d) = video.date_recorded {
                spec_row(ui, "Date Recorded", d);
            }

            // Log / Color Profile Selector
            let current_prof = crate::media::color_lut::LogProfile::from_str(&video.log_profile);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Color Profile").color(TEXT_MUTED).size(11.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt(format!("inspector_log_prof_{}", video.id))
                        .selected_text(
                            RichText::new(current_prof.display_name())
                                .strong()
                                .color(if current_prof.is_log() { ACCENT_BLUE } else { TEXT_PRIMARY })
                                .size(11.0),
                        )
                        .show_ui(ui, |ui| {
                            for opt in crate::media::color_lut::LogProfile::all_options() {
                                let is_sel = current_prof == *opt;
                                if ui.selectable_label(is_sel, opt.display_name()).clicked() {
                                    action = Some(InspectorAction::UpdateLogProfile(*opt));
                                }
                            }
                        });
                });
            });

            ui.add_space(4.0);
            if ui.button(RichText::new(format!("{} Re-scan Camera Metadata", egui_phosphor::regular::ARROWS_CLOCKWISE)).small())
                .on_hover_text("Re-read camera make/model, lens, white balance, and Log profile from source file")
                .clicked()
            {
                action = Some(InspectorAction::RescanMetadata(video.id));
            }
        });

    ui.separator();

    // SECTION 7: Audio Specs
    let audio_header = format!("{} Audio Specs", egui_phosphor::regular::SPEAKER_HIGH);
    egui::CollapsingHeader::new(RichText::new(audio_header).strong().size(12.5))
        .default_open(false)
        .show(ui, |ui| {
            if video.has_audio {
                spec_row(ui, "Audio Codec", &video.audio_codec.to_uppercase());
                spec_row(ui, "Channels", &format!("{} ch", video.audio_channels));
                spec_row(ui, "Sample Rate", &format!("{} Hz", video.audio_sample_rate));
            } else {
                ui.label(RichText::new("No Audio Track").color(TEXT_MUTED));
            }
        });

    action
}

fn spec_row(ui: &mut Ui, label: &str, val: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT_MUTED).size(11.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(val).strong().color(TEXT_PRIMARY).size(11.0));
        });
    });
}