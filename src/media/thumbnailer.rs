use std::path::Path;
use std::process::Command;
use crate::media::color_lut::{escape_ffmpeg_filter_path, get_or_create_lut_file, LogProfile};
use super::apply_no_window;

/// Build the video filter argument for thumbnail extraction, applying Rec.709 3D LUT if Log
fn build_thumb_vf(log_profile: Option<&str>) -> String {
    let base_scale = "scale=480:-2";
    if let Some(lp_str) = log_profile {
        let profile = LogProfile::from_str(lp_str);
        if profile.is_log() {
            if let Some(lut_path) = get_or_create_lut_file(profile) {
                let esc = escape_ffmpeg_filter_path(&lut_path);
                return format!("{},lut3d='{}'", base_scale, esc);
            }
        }
    }
    base_scale.to_string()
}

/// Extract a JPEG thumbnail from a video file using ffmpeg with optional Log-to-Rec.709 LUT
pub fn extract_thumbnail<P: AsRef<Path>>(
    file_path: P,
    duration_seconds: f32,
    log_profile: Option<&str>,
) -> Result<Vec<u8>, String> {
    let path = file_path.as_ref();

    // Pick timestamp: 1.0s or 15% into the video, avoid black start frames
    let seek_time = if duration_seconds > 2.0 {
        (duration_seconds * 0.15).clamp(1.0, 5.0)
    } else if duration_seconds > 0.2 {
        duration_seconds * 0.5
    } else {
        0.0
    };

    let vf = build_thumb_vf(log_profile);

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-ss")
        .arg(format!("{:.3}", seek_time))
        .arg("-i")
        .arg(path)
        .arg("-vframes")
        .arg("1")
        .arg("-vf")
        .arg(&vf)
        .arg("-f")
        .arg("image2")
        .arg("-c:v")
        .arg("mjpeg")
        .arg("-q:v")
        .arg("3")
        .arg("pipe:1");

    apply_no_window(&mut cmd);

    let output = cmd.output().map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;

    if output.status.success() && !output.stdout.is_empty() {
        return Ok(output.stdout);
    }

    // Fallback: try from frame 0
    let mut fallback_cmd = Command::new("ffmpeg");
    fallback_cmd.arg("-ss")
        .arg("0.0")
        .arg("-i")
        .arg(path)
        .arg("-vframes")
        .arg("1")
        .arg("-vf")
        .arg(&vf)
        .arg("-f")
        .arg("image2")
        .arg("-c:v")
        .arg("mjpeg")
        .arg("-q:v")
        .arg("3")
        .arg("pipe:1");

    apply_no_window(&mut fallback_cmd);

    let fallback_out = fallback_cmd.output().map_err(|e| format!("Fallback ffmpeg failed: {}", e))?;
    if fallback_out.status.success() && !fallback_out.stdout.is_empty() {
        Ok(fallback_out.stdout)
    } else if vf != "scale=480:-2" {
        // Fallback without LUT if filter failed
        let mut simple_cmd = Command::new("ffmpeg");
        simple_cmd.arg("-ss")
            .arg("0.0")
            .arg("-i")
            .arg(path)
            .arg("-vframes")
            .arg("1")
            .arg("-vf")
            .arg("scale=480:-2")
            .arg("-f")
            .arg("image2")
            .arg("-c:v")
            .arg("mjpeg")
            .arg("-q:v")
            .arg("3")
            .arg("pipe:1");

        apply_no_window(&mut simple_cmd);
        if let Ok(simple_out) = simple_cmd.output() {
            if simple_out.status.success() && !simple_out.stdout.is_empty() {
                return Ok(simple_out.stdout);
            }
        }
        Err(format!("Could not extract thumbnail from {:?}", path))
    } else {
        Err(format!("Could not extract thumbnail from {:?}", path))
    }
}
