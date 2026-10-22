use std::path::Path;
use std::process::Command;
use super::apply_no_window;

/// Extract a JPEG thumbnail from a video file using ffmpeg
pub fn extract_thumbnail<P: AsRef<Path>>(file_path: P, duration_seconds: f32) -> Result<Vec<u8>, String> {
    let path = file_path.as_ref();

    // Pick timestamp: 1.0s or 15% into the video, avoid black start frames
    let seek_time = if duration_seconds > 2.0 {
        (duration_seconds * 0.15).clamp(1.0, 5.0)
    } else if duration_seconds > 0.2 {
        duration_seconds * 0.5
    } else {
        0.0
    };

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-ss")
        .arg(format!("{:.3}", seek_time))
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
        .arg("scale=480:-2")
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
    } else {
        Err(format!("Could not extract thumbnail from {:?}", path))
    }
}
