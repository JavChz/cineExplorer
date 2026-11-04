use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use crate::db::models::{PickFlag, VideoRecord};
use crate::media::color_lut::detect_log_profile;
use crate::media::container_meta::{extract_container_metadata, parse_panasonic_xml_metadata, parse_sony_xml_metadata};
use super::apply_no_window;

#[derive(Debug, Deserialize)]
struct FFProbeOutput {
    streams: Option<Vec<FFStream>>,
    format: Option<FFFormat>,
}

#[derive(Debug, Deserialize)]
struct FFStream {
    #[serde(default)]
    #[allow(dead_code)]
    index: Option<usize>,
    codec_type: Option<String>,
    codec_name: Option<String>,
    codec_long_name: Option<String>,
    codec_tag_string: Option<String>,
    profile: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    pix_fmt: Option<String>,
    bits_per_raw_sample: Option<String>,
    color_space: Option<String>,
    color_primaries: Option<String>,
    color_transfer: Option<String>,
    duration: Option<String>,
    nb_frames: Option<String>,
    bit_rate: Option<String>,
    sample_rate: Option<String>,
    #[serde(default)]
    channels: Option<u16>,
    #[serde(default)]
    tags: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct FFFormat {
    format_name: Option<String>,
    duration: Option<String>,
    size: Option<String>,
    bit_rate: Option<String>,
    #[serde(default)]
    tags: Option<HashMap<String, serde_json::Value>>,
}

fn json_val_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => v.to_string(),
    }
}

fn parse_fraction(s: &str) -> Option<f32> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 2 {
        let num: f32 = parts[0].trim().parse().ok()?;
        let den: f32 = parts[1].trim().parse().ok()?;
        if den > 0.0 {
            return Some(num / den);
        }
    } else if let Ok(val) = s.trim().parse::<f32>() {
        return Some(val);
    }
    None
}

fn detect_bit_depth(stream: &FFStream) -> u16 {
    // 1. Direct bits_per_raw_sample
    if let Some(ref b) = stream.bits_per_raw_sample {
        if let Ok(val) = b.parse::<u16>() {
            if val > 0 {
                return val;
            }
        }
    }

    // 2. Pixel format check
    if let Some(ref pf) = stream.pix_fmt {
        let p_lower = pf.to_lowercase();
        if p_lower.contains("12le") || p_lower.contains("12be") || p_lower.ends_with("12") {
            return 12;
        }
        if p_lower.contains("10le") || p_lower.contains("10be") || p_lower.ends_with("10") {
            return 10;
        }
        if p_lower.contains("14le") || p_lower.contains("14be") {
            return 14;
        }
        if p_lower.contains("16le") || p_lower.contains("16be") || p_lower.ends_with("16") {
            return 16;
        }
    }

    // 3. Profile check (e.g. "Main 10", "High 10", ProRes HQ/4444)
    if let Some(ref prof) = stream.profile {
        let prof_lower = prof.to_lowercase();
        if prof_lower.contains("10") || prof_lower.contains("prores") {
            return 10;
        }
        if prof_lower.contains("12") || prof_lower.contains("4444 xq") {
            return 12;
        }
    }

    // 4. Codec check (BRAW is 12-bit)
    if let Some(ref c) = stream.codec_name {
        if c.to_lowercase().contains("braw") {
            return 12;
        }
    }
    if let Some(ref tag) = stream.codec_tag_string {
        if tag.to_lowercase().starts_with("br") {
            return 12;
        }
    }

    8
}

pub fn probe_video_file<P: AsRef<Path>>(file_path: P) -> Result<VideoRecord, String> {
    let path = file_path.as_ref();
    let file_path_str = path.to_string_lossy().to_string();
    let file_name = path.file_name().map_or("".to_string(), |s| s.to_string_lossy().to_string());
    let extension = path.extension().map_or("".to_string(), |s| s.to_string_lossy().to_lowercase());

    let mut cmd = Command::new("ffprobe");
    cmd.arg("-v")
        .arg("error")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(path);

    apply_no_window(&mut cmd);

    let output = cmd.output().map_err(|e| format!("Failed to execute ffprobe: {}. Make sure ffmpeg/ffprobe is installed.", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let err_desc = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            format!("ffprobe exited with exit code {:?}", output.status.code())
        };
        return Err(format!("ffprobe failed on '{}': {}", file_name, err_desc));
    }

    let parsed: FFProbeOutput = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse ffprobe json for '{}': {}", file_name, e))?;

    let streams = parsed.streams.unwrap_or_default();
    let format = parsed.format;

    // Find first video stream
    let video_stream = streams.iter().find(|s| s.codec_type.as_deref() == Some("video"));
    // Find first audio stream
    let audio_stream = streams.iter().find(|s| s.codec_type.as_deref() == Some("audio"));

    let file_size = format.as_ref()
        .and_then(|f| f.size.as_ref())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| std::fs::metadata(path).map(|m| m.len()).unwrap_or(0));

    let duration_seconds = format.as_ref()
        .and_then(|f| f.duration.as_ref())
        .and_then(|d| d.parse::<f32>().ok())
        .or_else(|| video_stream.and_then(|vs| vs.duration.as_ref()).and_then(|d| d.parse::<f32>().ok()))
        .unwrap_or(0.0);

    let format_bitrate = format.as_ref()
        .and_then(|f| f.bit_rate.as_ref())
        .and_then(|b| b.parse::<u64>().ok())
        .unwrap_or(0);

    let container = format.as_ref()
        .and_then(|f| f.format_name.as_ref())
        .map(|s| {
            let first = s.split(',').next().unwrap_or(s);
            if first.eq_ignore_ascii_case("mov") || first.eq_ignore_ascii_case("quicktime") {
                if extension == "braw" { "braw".to_string() } else { "mov".to_string() }
            } else if first.eq_ignore_ascii_case("matroska") {
                "mkv".to_string()
            } else {
                first.to_string()
            }
        })
        .unwrap_or_else(|| extension.clone());

    let mut codec = String::new();
    let mut codec_long_name = String::new();
    let mut profile = String::new();
    let mut width = 0;
    let mut height = 0;
    let mut fps = 0.0;
    let mut frame_count = 0;
    let mut bit_depth = 8;
    let mut pixel_format = String::new();
    let mut color_space = String::new();
    let mut color_primaries = String::new();
    let mut color_transfer = String::new();
    let mut video_bitrate = 0;

    let mut camera_make = String::new();
    let mut camera_model = String::new();
    let mut lens_model = String::new();
    let mut focal_length = String::new();
    let mut aperture = String::new();
    let mut iso = 0;
    let mut shutter_speed = String::new();
    let mut white_balance = String::new();
    let mut date_recorded = None;

    let mut scene = String::new();
    let mut shot = String::new();
    let mut take = String::new();
    let mut reel = String::new();
    let mut location = String::new();
    let camera_unit = String::new();

    if let Some(vs) = video_stream {
        codec = vs.codec_name.clone().unwrap_or_default();
        codec_long_name = vs.codec_long_name.clone().unwrap_or_default();
        profile = vs.profile.clone().unwrap_or_default();
        width = vs.width.unwrap_or(0);
        height = vs.height.unwrap_or(0);

        if let Some(ref r_fps) = vs.r_frame_rate {
            fps = parse_fraction(r_fps).unwrap_or(0.0);
        }
        if fps <= 0.0 {
            if let Some(ref avg_fps) = vs.avg_frame_rate {
                fps = parse_fraction(avg_fps).unwrap_or(0.0);
            }
        }

        if let Some(ref nb) = vs.nb_frames {
            frame_count = nb.parse::<u32>().unwrap_or(0);
        }
        if frame_count == 0 && duration_seconds > 0.0 && fps > 0.0 {
            frame_count = (duration_seconds * fps).round() as u32;
        }

        bit_depth = detect_bit_depth(vs);
        pixel_format = vs.pix_fmt.clone().unwrap_or_default();
        color_space = vs.color_space.clone().unwrap_or_default();
        color_primaries = vs.color_primaries.clone().unwrap_or_default();
        color_transfer = vs.color_transfer.clone().unwrap_or_default();

        if let Some(ref br) = vs.bit_rate {
            video_bitrate = br.parse::<u64>().unwrap_or(0);
        }
    }

    let mut raw_log_profile = None;

    // Extract metadata tags from format and video stream
    let mut combined_tags: HashMap<String, String> = HashMap::new();
    if let Some(ref f) = format {
        if let Some(ref tags) = f.tags {
            for (k, v) in tags {
                let val_str = json_val_to_string(v);
                if !val_str.trim().is_empty() {
                    combined_tags.insert(k.to_lowercase(), val_str);
                }
            }
        }
    }
    if let Some(vs) = video_stream {
        if let Some(ref tags) = vs.tags {
            for (k, v) in tags {
                let val_str = json_val_to_string(v);
                if !val_str.trim().is_empty() {
                    combined_tags.insert(k.to_lowercase(), val_str);
                }
            }
        }
    }

    // Check for embedded Panasonic Semi-Pro XML tag in ffprobe
    for (k, v) in &combined_tags {
        if k.contains("semi-pro.metadata") || k.contains("clipmain") {
            let mut xml_meta = crate::media::container_meta::ContainerMetadata::default();
            parse_panasonic_xml_metadata(v, &mut xml_meta);
            if let Some(m) = xml_meta.camera_make { if camera_make.is_empty() { camera_make = m; } }
            if let Some(m) = xml_meta.camera_model { if camera_model.is_empty() { camera_model = m; } }
            if let Some(i) = xml_meta.iso { if iso == 0 { iso = i; } }
            if let Some(lp) = xml_meta.log_profile { raw_log_profile = Some(lp); }
        }
        if k.contains("nonrealtimemeta") {
            let mut xml_meta = crate::media::container_meta::ContainerMetadata::default();
            parse_sony_xml_metadata(v, &mut xml_meta);
            if let Some(m) = xml_meta.camera_make { if camera_make.is_empty() { camera_make = m; } }
            if let Some(m) = xml_meta.camera_model { if camera_model.is_empty() { camera_model = m; } }
            if let Some(l) = xml_meta.lens_model { if lens_model.is_empty() { lens_model = l; } }
            if let Some(lp) = xml_meta.log_profile { raw_log_profile = Some(lp); }
        }
    }

    // Parse camera tags from ffprobe
    for (k, v) in &combined_tags {
        if k == "make" || k == "manufacturer" || k.ends_with(".make") || k.contains("com.apple.quicktime.make") {
            if camera_make.is_empty() && !v.is_empty() { camera_make = v.clone(); }
        } else if k == "model" || k == "camera_type" || k.ends_with(".model") || k.contains("com.apple.quicktime.model") {
            if camera_model.is_empty() && !v.is_empty() { camera_model = v.clone(); }
        } else if k.contains("lens") || k == "lens_type" {
            if lens_model.is_empty() && !v.is_empty() { lens_model = v.clone(); }
        } else if k.contains("focal") {
            if focal_length.is_empty() && !v.is_empty() { focal_length = v.clone(); }
        } else if k.contains("aperture") || k.contains("fnumber") {
            if aperture.is_empty() && !v.is_empty() { aperture = v.clone(); }
        } else if k == "iso" || k.ends_with(".iso") {
            if iso == 0 { iso = v.parse::<u32>().unwrap_or(0); }
        } else if (k == "shutter_speed" || k == "exposure_time") && !k.contains("type") {
            if shutter_speed.is_empty() && !v.is_empty() { shutter_speed = v.clone(); }
        } else if k.contains("scene") {
            if scene.is_empty() && !v.is_empty() { scene = v.clone(); }
        } else if k == "shot" || k.ends_with(".shot") {
            if shot.is_empty() && !v.is_empty() { shot = v.clone(); }
        } else if k.contains("take") && !k.contains("type") {
            if take.is_empty() && !v.is_empty() { take = v.clone(); }
        } else if k.contains("reel") {
            if reel.is_empty() && !v.is_empty() { reel = v.clone(); }
        } else if k.contains("location") {
            if location.is_empty() && !v.is_empty() { location = v.clone(); }
        } else if k.contains("gamma") {
            if raw_log_profile.is_none() && !v.is_empty() { raw_log_profile = Some(v.clone()); }
        } else if (k.contains("creation_time") || k.contains("date")) && date_recorded.is_none() {
            date_recorded = Some(v.clone());
        }
    }

    // Direct container metadata extraction (Canon uuid/CNTH/CNDA, Panasonic PANA/udta, Sony meta/xml, BRAW bmdf)
    let container_info = extract_container_metadata(path);
    if let Some(m) = container_info.camera_make {
        if camera_make.is_empty() || camera_make.to_lowercase() == "blackmagic camera" {
            camera_make = m;
        }
    }
    if let Some(m) = container_info.camera_model {
        if camera_model.is_empty() || camera_model.to_lowercase() == "blackmagic camera" {
            camera_model = m;
        }
    }
    if let Some(l) = container_info.lens_model {
        if lens_model.is_empty() { lens_model = l; }
    }
    if let Some(fl) = container_info.focal_length {
        if focal_length.is_empty() { focal_length = fl; }
    }
    if let Some(ap) = container_info.aperture {
        if aperture.is_empty() { aperture = ap; }
    }
    if let Some(i) = container_info.iso {
        if iso == 0 { iso = i; }
    }
    if let Some(ss) = container_info.shutter_speed {
        if shutter_speed.is_empty() { shutter_speed = ss; }
    }
    if let Some(wb) = container_info.white_balance {
        if white_balance.is_empty() { white_balance = wb; }
    }
    if let Some(dr) = container_info.date_recorded {
        if date_recorded.is_none() { date_recorded = Some(dr); }
    }
    if let Some(lp) = container_info.log_profile {
        if raw_log_profile.is_none() { raw_log_profile = Some(lp); }
    }

    // Fallback camera name and metadata for BRAW
    let is_braw = extension == "braw"
        || container.eq_ignore_ascii_case("braw")
        || video_stream
            .and_then(|vs| vs.codec_tag_string.as_deref())
            .is_some_and(|tag| tag.to_lowercase().starts_with("br"));

    if is_braw {
        if codec.is_empty() {
            codec = "braw".to_string();
        }
        if codec_long_name.is_empty() {
            codec_long_name = "Blackmagic RAW".to_string();
        }
        bit_depth = 12;
        if pixel_format.is_empty() {
            pixel_format = "12-bit RAW".to_string();
        }
        if profile.is_empty() {
            if let Some(ratio) = combined_tags.get("braw_compression_ratio") {
                profile = ratio.clone();
            } else {
                profile = "12-bit RAW".to_string();
            }
        }
        if color_space.is_empty() {
            color_space = "Blackmagic Wide Gamut Gen 5".to_string();
        }
        if camera_make.is_empty() {
            camera_make = "Blackmagic Design".to_string();
        }
        if camera_model.is_empty() {
            camera_model = "Blackmagic Camera".to_string();
        }
    }

    // Auto-detect Log Profile
    let detected_profile = detect_log_profile(
        &camera_make,
        &camera_model,
        raw_log_profile.as_deref(),
        &color_transfer,
        &codec,
        &profile,
        bit_depth,
    );
    let log_profile_str = if detected_profile.is_log() {
        detected_profile.display_name().to_string()
    } else {
        String::new()
    };

    // Audio specs
    let has_audio = audio_stream.is_some();
    let mut audio_codec = String::new();
    let mut audio_channels = 0;
    let mut audio_sample_rate = 0;

    if let Some(as_) = audio_stream {
        audio_codec = as_.codec_name.clone().unwrap_or_default();
        audio_channels = as_.channels.unwrap_or(0);
        audio_sample_rate = as_.sample_rate.as_ref().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
    }

    let bitrate = if video_bitrate > 0 { video_bitrate } else { format_bitrate };
    let imported_at = chrono::Utc::now().to_rfc3339();

    Ok(VideoRecord {
        id: 0,
        file_path: file_path_str,
        file_name,
        file_size,
        container,
        codec,
        codec_long_name,
        profile,
        width,
        height,
        fps,
        frame_count,
        duration_seconds,
        bit_depth,
        pixel_format,
        color_space,
        color_primaries,
        color_transfer,
        bitrate,
        has_audio,
        audio_codec,
        audio_channels,
        audio_sample_rate,
        camera_make,
        camera_model,
        lens_model,
        focal_length,
        aperture,
        iso,
        shutter_speed,
        white_balance,
        date_recorded,
        imported_at,
        rating: 0,
        flag: PickFlag::Unflagged,
        notes: String::new(),
        color_label: String::new(),
        scene,
        shot,
        take,
        reel,
        location,
        camera_unit,
        log_profile: log_profile_str,
        tags: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_probe_braw_file() {
        let braw_path = Path::new("P:\\VideoRepo\\BMPCC\\2026-01-21\\A009_03112214_C001.braw");
        if braw_path.exists() {
            let res = probe_video_file(braw_path);
            println!("Probe BRAW result: {:?}", res);
            if let Ok(rec) = res {
                println!("Record: codec='{}', width={}, height={}, fps={}, bit_depth={}, make='{}', model='{}', container='{}'",
                    rec.codec, rec.width, rec.height, rec.fps, rec.bit_depth, rec.camera_make, rec.camera_model, rec.container);
            }
        }
    }

    #[test]
    fn test_probe_real_footage() {
        if let Ok(db) = rusqlite::Connection::open("D:\\Dev\\githubprojects\\cineExplorer\\MyVideoCatalog.cinecat") {
            if let Ok(mut stmt) = db.prepare("SELECT id, file_name, file_path FROM videos LIMIT 3") {
                if let Ok(rows) = stmt.query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
                }) {
                    for r in rows.flatten() {
                        let p = Path::new(&r.2);
                        if p.exists() {
                            let res = probe_video_file(p);
                            assert!(res.is_ok(), "Probe should succeed for existing file");
                        }
                    }
                }
            }
        }
    }
}
