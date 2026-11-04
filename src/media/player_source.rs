use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use egui::ColorImage;
use crate::media::color_lut::{build_vf_filter, LogProfile};
use super::apply_no_window;

/// Calculate optimal viewport rendering dimensions, preserving aspect ratio
/// and guaranteeing even numbers for video hardware decoders and filters.
pub fn calculate_player_dimensions(orig_w: u32, orig_h: u32) -> (usize, usize) {
    let max_w = 1280.0f32;
    let max_h = 720.0f32;
    if orig_w == 0 || orig_h == 0 {
        return (1280, 720);
    }
    let aspect = orig_w as f32 / orig_h as f32;
    let (w, h) = if aspect >= (max_w / max_h) {
        (max_w, max_w / aspect)
    } else {
        (max_h * aspect, max_h)
    };
    let mut iw = (w.round() as usize).max(2);
    let mut ih = (h.round() as usize).max(2);
    if !iw.is_multiple_of(2) {
        iw += 1;
    }
    if !ih.is_multiple_of(2) {
        ih += 1;
    }
    (iw, ih)
}

#[derive(Clone)]
pub struct VideoPlayerSource {
    pub file_path: PathBuf,
    pub duration_seconds: f32,
    pub fps: f32,
    pub width: u32,
    pub height: u32,
    pub render_width: usize,
    pub render_height: usize,
    pub codec: String,
    pub bit_depth: u16,
    pub pixel_format: String,
    pub log_profile: LogProfile,
    pub apply_rec709: bool,
}

impl VideoPlayerSource {
    #[allow(clippy::too_many_arguments)]
    pub fn new<P: AsRef<Path>>(
        file_path: P,
        duration_seconds: f32,
        fps: f32,
        width: u32,
        height: u32,
        codec: &str,
        bit_depth: u16,
        pixel_format: &str,
        log_profile: LogProfile,
        apply_rec709: bool,
    ) -> Self {
        let (render_width, render_height) = calculate_player_dimensions(width, height);
        Self {
            file_path: file_path.as_ref().to_path_buf(),
            duration_seconds,
            fps: if fps > 0.0 { fps } else { 24.0 },
            width,
            height,
            render_width,
            render_height,
            codec: codec.to_lowercase(),
            bit_depth,
            pixel_format: pixel_format.to_lowercase(),
            log_profile,
            apply_rec709,
        }
    }

    pub fn set_log_config(&mut self, profile: LogProfile, apply_rec709: bool) {
        self.log_profile = profile;
        self.apply_rec709 = apply_rec709;
    }

    /// Check if hardware decoding is realistically supported for this codec/profile
    pub fn is_hw_accel_supported(&self) -> bool {
        let c = self.codec.as_str();
        let pf = self.pixel_format.as_str();

        // H.264 High 4:2:2 or 10-bit is NOT supported by NVDEC/Intel/AMD/Apple silicon hardware decoders.
        // Attempting -hwaccel wastes 600ms+ initializing GPU device only to fail or fall back.
        if (c.contains("h264") || c.contains("avc")) && (self.bit_depth > 8 || pf.contains("422") || pf.contains("444")) {
            return false;
        }

        // BRAW and ProRes are typically software or hybrid CPU/GPU formats in ffmpeg
        if c.contains("braw") || c.contains("prores") {
            return false;
        }

        // HEVC/H.265 (8/10/12-bit 4:2:0 & 4:2:2), AV1, VP9, standard H.264 8-bit are well supported by D3D11VA/CUDA
        true
    }

    pub fn is_braw(&self) -> bool {
        let ext = self.file_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
        ext == "braw" || self.codec.contains("braw")
    }

    /// Single-frame extraction with fast slice threading, low probe size, and automatic fallback.
    pub fn extract_frame_at(&self, seconds: f32) -> Result<ColorImage, String> {
        if self.is_braw() {
            #[cfg(windows)]
            {
                if let Ok((w, h, rgba)) = crate::media::os_thumb::extract_os_frame_rgba(
                    &self.file_path,
                    self.render_width as u32,
                    self.render_height as u32,
                ) {
                    return Ok(ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba));
                }
            }
        }

        let t = seconds.clamp(0.0, self.duration_seconds.max(0.0));
        let expected_bytes = self.render_width * self.render_height * 4;
        let vf = build_vf_filter(self.render_width, self.render_height, self.log_profile, self.apply_rec709);

        if self.is_hw_accel_supported() {
            let mut cmd = Command::new("ffmpeg");
            cmd.arg("-nostdin")
                .arg("-hide_banner")
                .arg("-loglevel")
                .arg("error")
                .arg("-probesize")
                .arg("2M")
                .arg("-analyzeduration")
                .arg("100000")
                .arg("-ss")
                .arg(format!("{:.4}", t))
                .arg("-hwaccel")
                .cfg_target_hwaccel()
                .arg("-i")
                .arg(&self.file_path)
                .arg("-an")
                .arg("-sn")
                .arg("-dn")
                .arg("-vf")
                .arg(&vf)
                .arg("-vframes")
                .arg("1")
                .arg("-f")
                .arg("rawvideo")
                .arg("-pix_fmt")
                .arg("rgba")
                .arg("pipe:1");

            apply_no_window(&mut cmd);
            cmd.stdin(Stdio::null());

            if let Ok(output) = cmd.output() {
                if output.status.success() && output.stdout.len() >= expected_bytes {
                    return Ok(ColorImage::from_rgba_unmultiplied(
                        [self.render_width, self.render_height],
                        &output.stdout[..expected_bytes],
                    ));
                }
            }
        }

        // Software fallback / direct fast decoding
        self.extract_frame_software(t, expected_bytes, &vf)
    }

    fn extract_frame_software(&self, t: f32, expected_bytes: usize, vf: &str) -> Result<ColorImage, String> {
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-nostdin")
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-probesize")
            .arg("2M")
            .arg("-analyzeduration")
            .arg("100000")
            .arg("-ss")
            .arg(format!("{:.4}", t))
            .arg("-threads")
            .arg("4")
            .arg("-thread_type")
            .arg("slice")
            .arg("-flags2")
            .arg("fast")
            .arg("-i")
            .arg(&self.file_path)
            .arg("-an")
            .arg("-sn")
            .arg("-dn")
            .arg("-vf")
            .arg(vf)
            .arg("-vframes")
            .arg("1")
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgba")
            .arg("pipe:1");

        apply_no_window(&mut cmd);
        cmd.stdin(Stdio::null());

        let output = cmd.output().map_err(|e| format!("FFmpeg execution failed: {}", e))?;
        if output.status.success() && output.stdout.len() >= expected_bytes {
            Ok(ColorImage::from_rgba_unmultiplied(
                [self.render_width, self.render_height],
                &output.stdout[..expected_bytes],
            ))
        } else {
            #[cfg(windows)]
            {
                if let Ok((w, h, rgba)) = crate::media::os_thumb::extract_os_frame_rgba(
                    &self.file_path,
                    self.render_width as u32,
                    self.render_height as u32,
                ) {
                    return Ok(ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba));
                }
            }
            Err("Failed to decode video frame".to_string())
        }
    }

    /// Spawn a streaming FFmpeg process with hardware acceleration if supported.
    pub fn spawn_stream_hw(&self, start_seconds: f32) -> Result<Child, String> {
        let t = start_seconds.clamp(0.0, self.duration_seconds.max(0.0));
        let vf_base = build_vf_filter(self.render_width, self.render_height, self.log_profile, self.apply_rec709);
        let vf = format!("{}:threads=0", vf_base);

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-nostdin")
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-probesize")
            .arg("2M")
            .arg("-analyzeduration")
            .arg("100000")
            .arg("-ss")
            .arg(format!("{:.4}", t))
            .arg("-hwaccel")
            .cfg_target_hwaccel()
            .arg("-threads")
            .arg("0")
            .arg("-flags2")
            .arg("fast")
            .arg("-i")
            .arg(&self.file_path)
            .arg("-an")
            .arg("-sn")
            .arg("-dn")
            .arg("-vf")
            .arg(&vf)
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgba")
            .arg("pipe:1");

        apply_no_window(&mut cmd);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        cmd.spawn().map_err(|e| format!("Failed to spawn hardware streaming decoder: {}", e))
    }

    /// Spawn a streaming FFmpeg process with multi-threaded CPU decoding.
    pub fn spawn_stream_sw(&self, start_seconds: f32) -> Result<Child, String> {
        let t = start_seconds.clamp(0.0, self.duration_seconds.max(0.0));
        let vf_base = build_vf_filter(self.render_width, self.render_height, self.log_profile, self.apply_rec709);
        let vf = format!("{}:threads=0", vf_base);

        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-nostdin")
            .arg("-hide_banner")
            .arg("-loglevel")
            .arg("error")
            .arg("-probesize")
            .arg("2M")
            .arg("-analyzeduration")
            .arg("100000")
            .arg("-ss")
            .arg(format!("{:.4}", t))
            .arg("-threads")
            .arg("0")
            .arg("-filter_threads")
            .arg("0")
            .arg("-flags2")
            .arg("fast")
            .arg("-i")
            .arg(&self.file_path)
            .arg("-an")
            .arg("-sn")
            .arg("-dn")
            .arg("-vf")
            .arg(&vf)
            .arg("-f")
            .arg("rawvideo")
            .arg("-pix_fmt")
            .arg("rgba")
            .arg("pipe:1");

        apply_no_window(&mut cmd);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        cmd.spawn().map_err(|e| format!("Failed to spawn software streaming decoder: {}", e))
    }

    /// Spawns stream decoder based on codec capabilities
    pub fn spawn_stream(&self, start_seconds: f32) -> (Result<Child, String>, bool) {
        if self.is_braw() {
            return (
                Err("Blackmagic RAW uses a proprietary 12-bit RAW stream requiring Blackmagic RAW Player or DaVinci Resolve".to_string()),
                false,
            );
        }
        if self.is_hw_accel_supported() {
            if let Ok(child) = self.spawn_stream_hw(start_seconds) {
                return (Ok(child), true);
            }
        }
        (self.spawn_stream_sw(start_seconds), false)
    }
}

trait CommandHwAccelExt {
    fn cfg_target_hwaccel(&mut self) -> &mut Self;
}

impl CommandHwAccelExt for Command {
    fn cfg_target_hwaccel(&mut self) -> &mut Self {
        #[cfg(target_os = "windows")]
        {
            self.arg("d3d11va")
        }
        #[cfg(target_os = "macos")]
        {
            self.arg("videotoolbox")
        }
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        {
            self.arg("auto")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_hw_accel_supported() {
        let p = VideoPlayerSource::new("test.mov", 10.0, 24.0, 1920, 1080, "h264", 10, "yuv422p10le", LogProfile::None, false);
        assert!(!p.is_hw_accel_supported());

        let p2 = VideoPlayerSource::new("test.mp4", 10.0, 24.0, 1920, 1080, "h264", 8, "yuv420p", LogProfile::None, false);
        assert!(p2.is_hw_accel_supported());

        let p3 = VideoPlayerSource::new("test.mov", 10.0, 24.0, 1920, 1080, "hevc", 10, "yuv420p10le", LogProfile::None, false);
        assert!(p3.is_hw_accel_supported());

        let p4 = VideoPlayerSource::new("test.braw", 10.0, 24.0, 1920, 1080, "braw", 12, "braw", LogProfile::None, false);
        assert!(!p4.is_hw_accel_supported());
    }

    #[test]
    fn test_calculate_player_dimensions() {
        // Standard 16:9 1080p -> 1280x720
        let (w, h) = calculate_player_dimensions(1920, 1080);
        assert_eq!(w, 1280);
        assert_eq!(h, 720);

        // Standard 16:9 4K -> 1280x720
        let (w, h) = calculate_player_dimensions(3840, 2160);
        assert_eq!(w, 1280);
        assert_eq!(h, 720);

        // Cinemascope 2.40:1 -> 1280 x 534
        let (w, h) = calculate_player_dimensions(3840, 1600);
        assert_eq!(w, 1280);
        assert_eq!(h % 2, 0);

        // Vertical 9:16 -> 406 x 720
        let (w, h) = calculate_player_dimensions(1080, 1920);
        assert_eq!(h, 720);
        assert_eq!(w % 2, 0);

        // 0 dimensions
        let (w, h) = calculate_player_dimensions(0, 0);
        assert_eq!(w, 1280);
        assert_eq!(h, 720);
    }
}
