use std::path::{Path, PathBuf};
use std::process::Command;
use egui::ColorImage;
use super::apply_no_window;

pub struct VideoPlayerSource {
    pub file_path: PathBuf,
    pub duration_seconds: f32,
    pub fps: f32,
    pub width: u32,
    pub height: u32,
}

impl VideoPlayerSource {
    pub fn new<P: AsRef<Path>>(file_path: P, duration_seconds: f32, fps: f32, width: u32, height: u32) -> Self {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
            duration_seconds,
            fps: if fps > 0.0 { fps } else { 24.0 },
            width,
            height,
        }
    }

    /// Extract a single frame at seconds timestamp
    pub fn extract_frame_at(&self, seconds: f32) -> Result<ColorImage, String> {
        let t = seconds.clamp(0.0, self.duration_seconds.max(0.0));
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-ss")
            .arg(format!("{:.4}", t))
            .arg("-i")
            .arg(&self.file_path)
            .arg("-vframes")
            .arg("1")
            .arg("-vf")
            .arg("scale=1280:-2")
            .arg("-f")
            .arg("image2")
            .arg("-c:v")
            .arg("mjpeg")
            .arg("-q:v")
            .arg("3")
            .arg("pipe:1");

        apply_no_window(&mut cmd);

        let output = cmd.output().map_err(|e| format!("ffmpeg execution failed: {}", e))?;
        if !output.status.success() || output.stdout.is_empty() {
            return Err("ffmpeg failed to output frame".to_string());
        }

        let img = image::load_from_memory(&output.stdout)
            .map_err(|e| format!("Failed to decode image frame: {}", e))?;
        let rgba = img.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = rgba.into_raw();

        Ok(ColorImage::from_rgba_unmultiplied(size, &pixels))
    }
}
