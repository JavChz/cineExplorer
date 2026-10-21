use egui::{Color32, RichText, Rounding, Ui, Vec2};
use crate::db::models::{PickFlag, VideoRecord};
use crate::thumbnails::ThumbnailManager;
use crate::ui::theme::*;
use crate::ui::widgets::*;

pub enum InspectorAction {
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
    DeleteVideo,
}

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

    ui.spacing_mut().item_spacing.y = 8.0;

    // Header Title
    ui.horizontal(|ui| {
        ui.label(RichText::new(&video.file_name).strong().size(14.0).color(TEXT_PRIMARY));
    });

    // Action buttons: Open in Player, Reveal in File Explorer
    ui.horizontal(|ui| {
        if ui.button("🎬 Play External").clicked() {
            action = Some(InspectorAction::PlayExternal);
        }
        if ui.button("📁 Reveal in Explorer").clicked() {
            action = Some(InspectorAction::RevealInExplorer);
        }
    });

    // Large Poster Preview
    let poster_w = ui.available_width().max(160.0);
    let poster_h = poster_w * (9.0 / 16.0);
    let (poster_rect, _) = ui.allocate_exact_size(Vec2::new(poster_w, poster_h), egui::Sense::hover());

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

    ui.separator();

    // SECTION 1: Video Tech Specs
    egui::CollapsingHeader::new(RichText::new("📹 Video Tech Specs").strong())
        .default_open(true)
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

    // SECTION 2: Audio Specs
    egui::CollapsingHeader::new(RichText::new("🔊 Audio Specs").strong())
        .default_open(true)
        .show(ui, |ui| {
            if video.has_audio {
                spec_row(ui, "Audio Codec", &video.audio_codec.to_uppercase());
                spec_row(ui, "Channels", &format!("{} ch", video.audio_channels));
                spec_row(ui, "Sample Rate", &format!("{} Hz", video.audio_sample_rate));
            } else {
                ui.label(RichText::new("No Audio Track").color(TEXT_MUTED));
            }
        });

    ui.separator();

    // SECTION 3: Camera & Shooting Info
    egui::CollapsingHeader::new(RichText::new("📷 Camera & Lens").strong())
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
            if let Some(ref d) = video.date_recorded {
                spec_row(ui, "Date Recorded", d);
            }
        });

    ui.separator();

    // SECTION 4: Production Slate (Editable)
    egui::CollapsingHeader::new(RichText::new("🎬 Slate Metadata").strong())
        .default_open(true)
        .show(ui, |ui| {
            let mut slate_changed = false;
            ui.horizontal(|ui| {
                ui.label("Scene:");
                if ui.text_edit_singleline(edit_scene).changed() { slate_changed = true; }
                ui.label("Shot:");
                if ui.text_edit_singleline(edit_shot).changed() { slate_changed = true; }
            });
            ui.horizontal(|ui| {
                ui.label("Take:");
                if ui.text_edit_singleline(edit_take).changed() { slate_changed = true; }
                ui.label("Reel:");
                if ui.text_edit_singleline(edit_reel).changed() { slate_changed = true; }
            });
            ui.horizontal(|ui| {
                ui.label("Location:");
                if ui.text_edit_singleline(edit_location).changed() { slate_changed = true; }
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

    // SECTION 5: Ratings & Flags
    egui::CollapsingHeader::new(RichText::new("⭐ Rating & Status").strong())
        .default_open(true)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Rating:");
                let mut rating = video.rating;
                if rating_stars(ui, &mut rating) {
                    action = Some(InspectorAction::UpdateRating(rating));
                }
            });

            ui.horizontal(|ui| {
                ui.label("Flag:");
                let mut flag = video.flag;
                if flag_picker(ui, &mut flag) {
                    action = Some(InspectorAction::UpdateFlag(flag));
                }
            });

            ui.horizontal(|ui| {
                ui.label("Color:");
                let mut color = video.color_label.clone();
                if color_label_picker(ui, &mut color) {
                    action = Some(InspectorAction::UpdateColor(color));
                }
            });
        });

    ui.separator();

    // SECTION 6: Tags
    egui::CollapsingHeader::new(RichText::new("🏷️ Tags").strong())
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

    // SECTION 7: Notes
    egui::CollapsingHeader::new(RichText::new("📝 Notes").strong())
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

    action
}

fn spec_row(ui: &mut Ui, label: &str, val: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(TEXT_MUTED).size(11.5));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(val).strong().color(TEXT_PRIMARY).size(11.5));
        });
    });
}
