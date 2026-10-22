use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LogProfile {
    #[default]
    None,
    CanonLog,
    CanonLog2,
    CanonLog3,
    SonySLog2,
    SonySLog3,
    PanasonicVLog,
    BlackmagicFilm,
}

impl LogProfile {
    pub fn display_name(&self) -> &'static str {
        match self {
            LogProfile::None => "None (Rec.709)",
            LogProfile::CanonLog => "Canon C-Log",
            LogProfile::CanonLog2 => "Canon C-Log2",
            LogProfile::CanonLog3 => "Canon C-Log3",
            LogProfile::SonySLog2 => "Sony S-Log2",
            LogProfile::SonySLog3 => "Sony S-Log3",
            LogProfile::PanasonicVLog => "Panasonic V-Log",
            LogProfile::BlackmagicFilm => "Blackmagic Film Gen 5",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            LogProfile::None => "",
            LogProfile::CanonLog => "C-Log",
            LogProfile::CanonLog2 => "C-Log2",
            LogProfile::CanonLog3 => "C-Log3",
            LogProfile::SonySLog2 => "S-Log2",
            LogProfile::SonySLog3 => "S-Log3",
            LogProfile::PanasonicVLog => "V-Log",
            LogProfile::BlackmagicFilm => "BMD Film",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogProfile::None => "",
            LogProfile::CanonLog => "canon_clog",
            LogProfile::CanonLog2 => "canon_clog2",
            LogProfile::CanonLog3 => "canon_clog3",
            LogProfile::SonySLog2 => "sony_slog2",
            LogProfile::SonySLog3 => "sony_slog3",
            LogProfile::PanasonicVLog => "panasonic_vlog",
            LogProfile::BlackmagicFilm => "bmd_film",
        }
    }

    pub fn from_str(s: &str) -> Self {
        let l = s.trim().to_lowercase();
        if l.contains("clog3") || l.contains("c-log3") || l.contains("c-log 3") {
            LogProfile::CanonLog3
        } else if l.contains("clog2") || l.contains("c-log2") || l.contains("c-log 2") {
            LogProfile::CanonLog2
        } else if l.contains("clog") || l.contains("c-log") {
            LogProfile::CanonLog
        } else if l.contains("slog3") || l.contains("s-log3") || l.contains("s-log 3") {
            LogProfile::SonySLog3
        } else if l.contains("slog2") || l.contains("s-log2") || l.contains("s-log 2") {
            LogProfile::SonySLog2
        } else if l.contains("vlog") || l.contains("v-log") {
            LogProfile::PanasonicVLog
        } else if l.contains("bmd") || l.contains("blackmagic") || l.contains("braw") {
            LogProfile::BlackmagicFilm
        } else {
            LogProfile::None
        }
    }

    pub fn is_log(&self) -> bool {
        *self != LogProfile::None
    }

    pub fn all_options() -> &'static [LogProfile] {
        &[
            LogProfile::None,
            LogProfile::CanonLog3,
            LogProfile::CanonLog2,
            LogProfile::CanonLog,
            LogProfile::SonySLog3,
            LogProfile::SonySLog2,
            LogProfile::PanasonicVLog,
            LogProfile::BlackmagicFilm,
        ]
    }
}

/// Detect the Log profile based on camera make, model, container tags, and stream characteristics
pub fn detect_log_profile(
    camera_make: &str,
    camera_model: &str,
    raw_log_tag: Option<&str>,
    color_transfer: &str,
    codec: &str,
    profile: &str,
    bit_depth: u16,
) -> LogProfile {
    // 1. Direct tag if present from container metadata (CanonLogInfo, Panasonic XML, Sony NRT XML)
    if let Some(tag) = raw_log_tag {
        let p = LogProfile::from_str(tag);
        if p != LogProfile::None {
            return p;
        }
    }

    let make_lower = camera_make.to_lowercase();
    let model_lower = camera_model.to_lowercase();
    let transfer_lower = color_transfer.to_lowercase();
    let codec_lower = codec.to_lowercase();
    let prof_lower = profile.to_lowercase();

    // 2. Blackmagic RAW check
    if codec_lower.contains("braw") || make_lower.contains("blackmagic") || model_lower.contains("pocket cinema") {
        return LogProfile::BlackmagicFilm;
    }

    // 3. Panasonic check
    if make_lower.contains("panasonic") || model_lower.contains("lumix") || model_lower.contains("dc-") {
        if transfer_lower.contains("v-log") || transfer_lower.contains("vlog") {
            return LogProfile::PanasonicVLog;
        }
    }

    // 4. Sony check
    if make_lower.contains("sony") || model_lower.contains("ilce") || model_lower.contains("ilme") || model_lower.contains("fx") {
        if transfer_lower.contains("s-log3") || transfer_lower.contains("slog3") {
            return LogProfile::SonySLog3;
        }
        if transfer_lower.contains("s-log2") || transfer_lower.contains("slog2") {
            return LogProfile::SonySLog2;
        }
    }

    // 5. Canon check
    if make_lower.contains("canon") || model_lower.contains("eos") {
        if transfer_lower.contains("c-log3") || transfer_lower.contains("clog3") {
            return LogProfile::CanonLog3;
        }
        if transfer_lower.contains("c-log2") || transfer_lower.contains("clog2") {
            return LogProfile::CanonLog2;
        }
        if transfer_lower.contains("c-log") || transfer_lower.contains("clog") {
            return LogProfile::CanonLog;
        }
        // Canon EOS R-series (R5, R6, R7, C70) 10-bit 4:2:2 HEVC Rext is almost exclusively shot in C-Log3
        if bit_depth >= 10 && prof_lower.contains("rext") && (model_lower.contains("r7") || model_lower.contains("r5") || model_lower.contains("r6") || model_lower.contains("r3")) {
            return LogProfile::CanonLog3;
        }
    }

    LogProfile::None
}

// =========================================================================
// Color Science Transfer Functions & Gamut Matrices (Log -> Rec.709)
// =========================================================================

#[inline]
fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

/// Linear Rec.709 to non-linear Rec.709 OETF display curve
#[inline]
fn linear_to_rec709_oetf(linear: f32) -> f32 {
    let l = linear.max(0.0);
    if l < 0.018 {
        4.5 * l
    } else {
        1.099 * l.powf(0.45) - 0.099
    }
}

/// Filmic S-curve tone mapping to gracefully compress HDR camera sensor highlights into SDR 0..1
#[inline]
fn tone_map_highlight_shoulder(linear: f32, max_white: f32) -> f32 {
    if linear <= 0.0 {
        return 0.0;
    }
    // Reinhard-style shoulder with linear mid-tones and gentle rolloff
    let x = linear / max_white;
    let shoulder = (x * (1.0 + x / 4.0)) / (1.0 + x);
    shoulder.min(1.0)
}

/// Convert a normalized Canon C-Log3 sample (0..1) to linear sensor reflection
fn canon_clog3_to_linear(y: f32) -> f32 {
    if y >= 0.109961 {
        let p = (y - 0.42012) / 0.263319;
        (10.0f32.powf(p) - 1.0) / 14.1599
    } else {
        (y - 0.092864).max(0.0) / 14.9832
    }
}

/// Convert a normalized Canon C-Log2 sample (0..1) to linear sensor reflection
fn canon_clog2_to_linear(y: f32) -> f32 {
    if y >= 0.06731 {
        let p = (y - 0.618353) / 0.281863;
        10.0f32.powf(p) - 0.007812
    } else {
        (y - 0.035388).max(0.0) / 8.729
    }
}

/// Convert a normalized Canon original C-Log sample (0..1) to linear sensor reflection
fn canon_clog_to_linear(y: f32) -> f32 {
    if y > 0.1278 {
        let p = (y - 0.529136) / 0.529136;
        10.0f32.powf(p) - 0.073059
    } else {
        (y - 0.073059).max(0.0) / 10.844
    }
}

/// Convert a normalized Sony S-Log3 sample (0..1) to linear reflection
fn sony_slog3_to_linear(y: f32) -> f32 {
    if y >= 171.210774 / 1023.0 {
        let p = (y - 420.0 / 1023.0) / (261.5 / 1023.0);
        10.0f32.powf(p) * 0.19 - 0.01
    } else {
        (y - 95.0 / 1023.0) * (0.18 / (171.210774 / 1023.0 - 95.0 / 1023.0))
    }
}

/// Convert a normalized Sony S-Log2 sample (0..1) to linear reflection
fn sony_slog2_to_linear(y: f32) -> f32 {
    if y >= 43.70 / 1023.0 {
        let p = (y - 43.70 / 1023.0) / 0.432699;
        p.max(0.0).powf(1.0 / 0.58476) * (155.0 / 219.0)
    } else {
        (y - 0.030001).max(0.0) / 3.5388125
    }
}

/// Convert a normalized Panasonic V-Log sample (0..1) to linear reflection
fn panasonic_vlog_to_linear(y: f32) -> f32 {
    let cut = 0.181;
    if y >= cut {
        let p = (y - 0.423) / 0.263;
        10.0f32.powf(p) - 0.0075
    } else {
        (y - 0.125).max(0.0) / 5.6
    }
}

/// Convert a normalized Blackmagic Design Film Gen 5 sample to linear reflection
fn bmd_film_to_linear(y: f32) -> f32 {
    let a = 0.08;
    let b = 0.18;
    if y >= a {
        10.0f32.powf((y - 0.41) / 0.25) * b
    } else {
        y.max(0.0) * (b / a)
    }
}

/// Apply official color gamut 3x3 matrix to convert camera sensor primaries to Rec.709 primaries
#[inline]
fn apply_matrix3(m: &[f32; 9], r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (
        m[0] * r + m[1] * g + m[2] * b,
        m[3] * r + m[4] * g + m[5] * b,
        m[6] * r + m[7] * g + m[8] * b,
    )
}

/// Transform an RGB triple from the given Log profile to standard Rec.709
pub fn transform_log_to_rec709(profile: LogProfile, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    match profile {
        LogProfile::None => (clamp01(r), clamp01(g), clamp01(b)),

        LogProfile::CanonLog3 => {
            let lr = canon_clog3_to_linear(r);
            let lg = canon_clog3_to_linear(g);
            let lb = canon_clog3_to_linear(b);

            // BT.2020 / Cinema Gamut to Rec.709 Matrix
            const M_CANON: [f32; 9] = [
                1.6605, -0.5876, -0.0728,
                -0.1246, 1.1329, -0.0083,
                -0.0182, -0.1006, 1.1187,
            ];
            let (mr, mg, mb) = apply_matrix3(&M_CANON, lr, lg, lb);
            let tr = tone_map_highlight_shoulder(mr, 2.5);
            let tg = tone_map_highlight_shoulder(mg, 2.5);
            let tb = tone_map_highlight_shoulder(mb, 2.5);

            (
                clamp01(linear_to_rec709_oetf(tr)),
                clamp01(linear_to_rec709_oetf(tg)),
                clamp01(linear_to_rec709_oetf(tb)),
            )
        }

        LogProfile::CanonLog2 => {
            let lr = canon_clog2_to_linear(r);
            let lg = canon_clog2_to_linear(g);
            let lb = canon_clog2_to_linear(b);

            const M_CANON: [f32; 9] = [
                1.6605, -0.5876, -0.0728,
                -0.1246, 1.1329, -0.0083,
                -0.0182, -0.1006, 1.1187,
            ];
            let (mr, mg, mb) = apply_matrix3(&M_CANON, lr, lg, lb);
            let tr = tone_map_highlight_shoulder(mr, 3.0);
            let tg = tone_map_highlight_shoulder(mg, 3.0);
            let tb = tone_map_highlight_shoulder(mb, 3.0);

            (
                clamp01(linear_to_rec709_oetf(tr)),
                clamp01(linear_to_rec709_oetf(tg)),
                clamp01(linear_to_rec709_oetf(tb)),
            )
        }

        LogProfile::CanonLog => {
            let lr = canon_clog_to_linear(r);
            let lg = canon_clog_to_linear(g);
            let lb = canon_clog_to_linear(b);

            const M_CANON: [f32; 9] = [
                1.6605, -0.5876, -0.0728,
                -0.1246, 1.1329, -0.0083,
                -0.0182, -0.1006, 1.1187,
            ];
            let (mr, mg, mb) = apply_matrix3(&M_CANON, lr, lg, lb);
            let tr = tone_map_highlight_shoulder(mr, 2.0);
            let tg = tone_map_highlight_shoulder(mg, 2.0);
            let tb = tone_map_highlight_shoulder(mb, 2.0);

            (
                clamp01(linear_to_rec709_oetf(tr)),
                clamp01(linear_to_rec709_oetf(tg)),
                clamp01(linear_to_rec709_oetf(tb)),
            )
        }

        LogProfile::SonySLog3 => {
            let lr = sony_slog3_to_linear(r);
            let lg = sony_slog3_to_linear(g);
            let lb = sony_slog3_to_linear(b);

            // Sony S-Gamut3.Cine to Rec.709 Matrix
            const M_SONY: [f32; 9] = [
                1.755448, -0.655866, -0.099582,
                -0.075908, 1.157833, -0.081926,
                -0.016259, -0.130635, 1.146894,
            ];
            let (mr, mg, mb) = apply_matrix3(&M_SONY, lr, lg, lb);
            let tr = tone_map_highlight_shoulder(mr, 2.8);
            let tg = tone_map_highlight_shoulder(mg, 2.8);
            let tb = tone_map_highlight_shoulder(mb, 2.8);

            (
                clamp01(linear_to_rec709_oetf(tr)),
                clamp01(linear_to_rec709_oetf(tg)),
                clamp01(linear_to_rec709_oetf(tb)),
            )
        }

        LogProfile::SonySLog2 => {
            let lr = sony_slog2_to_linear(r);
            let lg = sony_slog2_to_linear(g);
            let lb = sony_slog2_to_linear(b);

            const M_SONY: [f32; 9] = [
                1.755448, -0.655866, -0.099582,
                -0.075908, 1.157833, -0.081926,
                -0.016259, -0.130635, 1.146894,
            ];
            let (mr, mg, mb) = apply_matrix3(&M_SONY, lr, lg, lb);
            let tr = tone_map_highlight_shoulder(mr, 2.2);
            let tg = tone_map_highlight_shoulder(mg, 2.2);
            let tb = tone_map_highlight_shoulder(mb, 2.2);

            (
                clamp01(linear_to_rec709_oetf(tr)),
                clamp01(linear_to_rec709_oetf(tg)),
                clamp01(linear_to_rec709_oetf(tb)),
            )
        }

        LogProfile::PanasonicVLog => {
            let lr = panasonic_vlog_to_linear(r);
            let lg = panasonic_vlog_to_linear(g);
            let lb = panasonic_vlog_to_linear(b);

            // Panasonic V-Gamut to Rec.709 Matrix
            const M_PANA: [f32; 9] = [
                1.806576, -0.695697, -0.110879,
                -0.170090, 1.305954, -0.135864,
                -0.025206, -0.154468, 1.179674,
            ];
            let (mr, mg, mb) = apply_matrix3(&M_PANA, lr, lg, lb);
            let tr = tone_map_highlight_shoulder(mr, 2.5);
            let tg = tone_map_highlight_shoulder(mg, 2.5);
            let tb = tone_map_highlight_shoulder(mb, 2.5);

            (
                clamp01(linear_to_rec709_oetf(tr)),
                clamp01(linear_to_rec709_oetf(tg)),
                clamp01(linear_to_rec709_oetf(tb)),
            )
        }

        LogProfile::BlackmagicFilm => {
            let lr = bmd_film_to_linear(r);
            let lg = bmd_film_to_linear(g);
            let lb = bmd_film_to_linear(b);

            // Blackmagic Wide Gamut to Rec.709 Matrix
            const M_BMD: [f32; 9] = [
                1.7024, -0.6272, -0.0752,
                -0.1453, 1.2062, -0.0609,
                -0.0221, -0.1342, 1.1563,
            ];
            let (mr, mg, mb) = apply_matrix3(&M_BMD, lr, lg, lb);
            let tr = tone_map_highlight_shoulder(mr, 2.4);
            let tg = tone_map_highlight_shoulder(mg, 2.4);
            let tb = tone_map_highlight_shoulder(mb, 2.4);

            (
                clamp01(linear_to_rec709_oetf(tr)),
                clamp01(linear_to_rec709_oetf(tg)),
                clamp01(linear_to_rec709_oetf(tb)),
            )
        }
    }
}

// =========================================================================
// 3D LUT Generator (.cube format) & Windows Path Escaping for FFmpeg
// =========================================================================

/// Get the directory where generated 3D LUT files are persisted
pub fn get_lut_storage_dir() -> PathBuf {
    let base = if let Some(app_data) = std::env::var_os("APPDATA") {
        PathBuf::from(app_data).join("cineExplorer").join("luts")
    } else {
        PathBuf::from(".cineExplorer").join("luts")
    };
    let _ = create_dir_all(&base);
    base
}

/// Generates a standard Adobe / DaVinci Resolve 33x33x33 3D `.cube` LUT file
pub fn generate_cube_lut(profile: LogProfile, output_path: &Path) -> std::io::Result<()> {
    if let Some(parent) = output_path.parent() {
        create_dir_all(parent)?;
    }

    let mut file = File::create(output_path)?;
    let lut_size = 33usize;

    writeln!(file, "# cineExplorer Log to Rec.709 3D LUT")?;
    writeln!(file, "# Profile: {}", profile.display_name())?;
    writeln!(file, "LUT_3D_SIZE {}", lut_size)?;
    writeln!(file, "DOMAIN_MIN 0.0 0.0 0.0")?;
    writeln!(file, "DOMAIN_MAX 1.0 1.0 1.0")?;

    let step = 1.0f32 / (lut_size - 1) as f32;

    for b_idx in 0..lut_size {
        let b = b_idx as f32 * step;
        for g_idx in 0..lut_size {
            let g = g_idx as f32 * step;
            for r_idx in 0..lut_size {
                let r = r_idx as f32 * step;
                let (or, og, ob) = transform_log_to_rec709(profile, r, g, b);
                writeln!(file, "{:.6} {:.6} {:.6}", or, og, ob)?;
            }
        }
    }

    file.flush()?;
    Ok(())
}

/// Ensure the 3D LUT file exists on disk, generating it on demand if missing
pub fn get_or_create_lut_file(profile: LogProfile) -> Option<PathBuf> {
    if profile == LogProfile::None {
        return None;
    }
    let filename = format!("{}_to_rec709.cube", profile.as_str());
    let lut_path = get_lut_storage_dir().join(filename);

    if !lut_path.exists() {
        let _ = generate_cube_lut(profile, &lut_path);
    }

    if lut_path.exists() {
        Some(lut_path)
    } else {
        None
    }
}

/// Escape a file path safely for FFmpeg filter arguments on Windows and POSIX
/// FFmpeg's avfilter parser uses ':' and '\' as special control characters.
pub fn escape_ffmpeg_filter_path(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    // Escape colon after drive letter e.g. C:/ -> C\:/
    s.replace(':', "\\:")
}

/// Build an FFmpeg `-vf` filter argument including scale and the Log-to-Rec.709 3D LUT
pub fn build_vf_filter(
    render_width: usize,
    render_height: usize,
    profile: LogProfile,
    apply_rec709: bool,
) -> String {
    let scale_str = format!("scale={}:{}:flags=fast_bilinear", render_width, render_height);
    if apply_rec709 && profile.is_log() {
        if let Some(lut_file) = get_or_create_lut_file(profile) {
            let escaped_path = escape_ffmpeg_filter_path(&lut_file);
            return format!("{},lut3d='{}'", scale_str, escaped_path);
        }
    }
    scale_str
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_profile_detection() {
        let p1 = detect_log_profile("Canon", "Canon EOS R7", Some("Canon C-Log3"), "bt709", "hevc", "Rext", 10);
        assert_eq!(p1, LogProfile::CanonLog3);

        let p2 = detect_log_profile("Panasonic", "DC-S5", Some("Panasonic V-Log"), "bt709", "h264", "High 4:2:2", 10);
        assert_eq!(p2, LogProfile::PanasonicVLog);

        let p3 = detect_log_profile("Sony", "ILME-FX3", Some("Sony S-Log3"), "bt709", "hevc", "Main 10", 10);
        assert_eq!(p3, LogProfile::SonySLog3);

        let p4 = detect_log_profile("Blackmagic Design", "BMPCC 4K", None, "bmd film", "braw", "braw", 12);
        assert_eq!(p4, LogProfile::BlackmagicFilm);
    }

    #[test]
    fn test_transform_bounds() {
        for profile in LogProfile::all_options() {
            let (r, g, b) = transform_log_to_rec709(*profile, 0.0, 0.5, 1.0);
            assert!(r >= 0.0 && r <= 1.0);
            assert!(g >= 0.0 && g <= 1.0);
            assert!(b >= 0.0 && b <= 1.0);
        }
    }

    #[test]
    fn test_escape_ffmpeg_path() {
        let p = Path::new("C:\\Users\\test\\cineExplorer\\luts\\lut.cube");
        let esc = escape_ffmpeg_filter_path(p);
        assert_eq!(esc, "C\\:/Users/test/cineExplorer/luts/lut.cube");
    }
}
