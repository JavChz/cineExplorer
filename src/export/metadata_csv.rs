use crate::db::models::{PickFlag, VideoRecord};

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn generate_resolve_csv(videos: &[VideoRecord]) -> String {
    let mut out = String::new();
    out.push_str("File Name,File Path,Reel Name,Scene,Shot,Take,Good Take,Clip Color,Keywords,Comments,Description,Camera Type,FPS,Resolution,Bit Depth\n");

    for v in videos {
        let mut kw_list = v.tags.clone();
        kw_list.push(format!("{}-bit", v.bit_depth));
        kw_list.push(v.resolution_label());
        kw_list.push(v.codec_display_name());
        if !v.camera_model.is_empty() {
            kw_list.push(v.camera_model.clone());
        }
        if !v.log_profile.is_empty() {
            let prof = crate::media::color_lut::LogProfile::from_str(&v.log_profile);
            if prof.is_log() {
                kw_list.push(prof.display_name().to_string());
            }
        }

        let good_take = match v.flag {
            PickFlag::Picked => "Yes",
            PickFlag::Rejected => "No",
            PickFlag::Unflagged => "",
        };

        let row = vec![
            escape_csv(&v.file_name),
            escape_csv(&v.file_path),
            escape_csv(&v.reel),
            escape_csv(&v.scene),
            escape_csv(&v.shot),
            escape_csv(&v.take),
            escape_csv(good_take),
            escape_csv(&v.color_label),
            escape_csv(&kw_list.join(", ")),
            escape_csv(&v.notes),
            escape_csv(&v.slate_label().unwrap_or_default()),
            escape_csv(&v.camera_model),
            escape_csv(&v.fps_label()),
            escape_csv(&format!("{}x{}", v.width, v.height)),
            escape_csv(&format!("{}-bit", v.bit_depth)),
        ];

        out.push_str(&row.join(","));
        out.push('\n');
    }

    out
}
