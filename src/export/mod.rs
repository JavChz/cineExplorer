pub mod launcher;
pub mod metadata_csv;
pub mod premiere_xml;
pub mod resolve_fcpxml;

use std::fs;
use std::path::Path;
use crate::db::models::VideoRecord;
use launcher::open_file_in_nle;
use metadata_csv::generate_resolve_csv;
use premiere_xml::{generate_premiere_xml, PremiereXmlExportOptions};
use resolve_fcpxml::{generate_resolve_fcpxml, FcpXmlExportOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NleTarget {
    DaVinciResolveFcpxml,
    AdobePremiereXml,
    DaVinciResolveCsv,
}

impl NleTarget {
    pub fn display_name(&self) -> &'static str {
        match self {
            NleTarget::DaVinciResolveFcpxml => "DaVinci Resolve (.fcpxml)",
            NleTarget::AdobePremiereXml => "Adobe Premiere Pro (.xml)",
            NleTarget::DaVinciResolveCsv => "DaVinci Resolve Metadata (.csv)",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            NleTarget::DaVinciResolveFcpxml => "fcpxml",
            NleTarget::AdobePremiereXml => "xml",
            NleTarget::DaVinciResolveCsv => "csv",
        }
    }
}

pub struct NleExportOptions {
    pub target: NleTarget,
    pub project_name: String,
    pub include_user_tags: bool,
    pub include_tech_keywords: bool,
    pub include_slate: bool,
    pub launch_after_export: bool,
}

impl Default for NleExportOptions {
    fn default() -> Self {
        Self {
            target: NleTarget::DaVinciResolveFcpxml,
            project_name: "cineExplorer Export".to_string(),
            include_user_tags: true,
            include_tech_keywords: true,
            include_slate: true,
            launch_after_export: true,
        }
    }
}

pub fn export_and_launch<P: AsRef<Path>>(
    videos: &[VideoRecord],
    output_path: P,
    options: &NleExportOptions,
) -> Result<(), String> {
    let path = output_path.as_ref();

    let content = match options.target {
        NleTarget::DaVinciResolveFcpxml => {
            let fcpxml_opts = FcpXmlExportOptions {
                project_name: options.project_name.clone(),
                include_user_tags: options.include_user_tags,
                include_tech_keywords: options.include_tech_keywords,
                include_slate: options.include_slate,
            };
            generate_resolve_fcpxml(videos, &fcpxml_opts)
        }
        NleTarget::AdobePremiereXml => {
            let prem_opts = PremiereXmlExportOptions {
                project_name: options.project_name.clone(),
                include_keywords: options.include_user_tags || options.include_tech_keywords,
                include_slate: options.include_slate,
            };
            generate_premiere_xml(videos, &prem_opts)
        }
        NleTarget::DaVinciResolveCsv => {
            generate_resolve_csv(videos)
        }
    };

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    fs::write(path, content.as_bytes())
        .map_err(|e| format!("Failed to write export file: {}", e))?;

    if options.launch_after_export {
        let nle_name = match options.target {
            NleTarget::DaVinciResolveFcpxml | NleTarget::DaVinciResolveCsv => "resolve",
            NleTarget::AdobePremiereXml => "premiere",
        };
        let _ = open_file_in_nle(nle_name, path);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{PickFlag, VideoRecord};

    fn make_test_video() -> VideoRecord {
        VideoRecord {
            id: 1,
            file_path: "D:\\Footage\\A001_C001.mov".to_string(),
            file_name: "A001_C001.mov".to_string(),
            file_size: 1024 * 1024 * 500,
            container: "mov".to_string(),
            codec: "prores".to_string(),
            codec_long_name: "Apple ProRes 422 HQ".to_string(),
            profile: "ProRes 422 HQ".to_string(),
            width: 3840,
            height: 2160,
            fps: 23.976,
            frame_count: 240,
            duration_seconds: 10.01,
            bit_depth: 10,
            pixel_format: "yuv422p10le".to_string(),
            color_space: "bt709".to_string(),
            color_primaries: "bt709".to_string(),
            color_transfer: "bt709".to_string(),
            bitrate: 220000000,
            has_audio: true,
            audio_codec: "pcm_s24le".to_string(),
            audio_channels: 2,
            audio_sample_rate: 48000,
            camera_make: "Sony".to_string(),
            camera_model: "Sony FX3".to_string(),
            lens_model: "FE 24-70mm F2.8 GM II".to_string(),
            focal_length: "50mm".to_string(),
            aperture: "f/2.8".to_string(),
            iso: 800,
            shutter_speed: "1/50".to_string(),
            white_balance: "5600K".to_string(),
            log_profile: "sony_slog3".to_string(),
            date_recorded: Some("2026-05-10T14:30:00Z".to_string()),
            imported_at: "2026-05-10T16:00:00Z".to_string(),
            rating: 5,
            flag: PickFlag::Picked,
            notes: "Hero close-up take".to_string(),
            color_label: "orange".to_string(),
            scene: "4B".to_string(),
            shot: "1".to_string(),
            take: "3".to_string(),
            reel: "A001".to_string(),
            location: "Studio Stage 2".to_string(),
            camera_unit: "A-Cam".to_string(),
            tags: vec!["Exterior".to_string(), "Day".to_string()],
        }
    }

    #[test]
    fn test_resolve_fcpxml_generation() {
        let videos = vec![make_test_video()];
        let opts = FcpXmlExportOptions::default();
        let xml = generate_resolve_fcpxml(&videos, &opts);

        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<fcpxml version=\"1.9\">"));
        assert!(xml.contains("file:///D:/Footage/A001_C001.mov"));
        assert!(xml.contains("<keyword start=\"0s\" duration=\"10.0100s\" value=\"Exterior\"/>"));
        assert!(xml.contains("<keyword start=\"0s\" duration=\"10.0100s\" value=\"10-bit\"/>"));
        assert!(xml.contains("<keyword start=\"0s\" duration=\"10.0100s\" value=\"ProRes HQ\"/>"));
        assert!(xml.contains("<keyword start=\"0s\" duration=\"10.0100s\" value=\"Sony FX3\"/>"));
        assert!(xml.contains("<keyword start=\"0s\" duration=\"10.0100s\" value=\"Sony S-Log3\"/>"));
        assert!(xml.contains("<keyword start=\"0s\" duration=\"10.0100s\" value=\"Pick\"/>"));
        assert!(xml.contains("<md key=\"com.apple.proapps.studio.scene\" value=\"4B\"/>"));
        assert!(xml.contains("<md key=\"com.apple.proapps.studio.take\" value=\"3\"/>"));
    }

    #[test]
    fn test_premiere_xml_generation() {
        let videos = vec![make_test_video()];
        let opts = PremiereXmlExportOptions::default();
        let xml = generate_premiere_xml(&videos, &opts);

        assert!(xml.contains("<xmeml version=\"5\">"));
        assert!(xml.contains("<pathurl>file://localhost/D:/Footage/A001_C001.mov</pathurl>"));
        assert!(xml.contains("<scene>4B</scene>"));
        assert!(xml.contains("<good>TRUE</good>"));
        assert!(xml.contains("<lognote>Keywords: Exterior, Day, 10-bit, 4K UHD, ProRes HQ, Sony FX3, Sony S-Log3</lognote>"));
    }

    #[test]
    fn test_resolve_csv_generation() {
        let videos = vec![make_test_video()];
        let csv = generate_resolve_csv(&videos);

        assert!(csv.starts_with("File Name,File Path,Reel Name,Scene,Shot,Take,Good Take,Clip Color,Keywords"));
        assert!(csv.contains("A001_C001.mov,D:\\Footage\\A001_C001.mov,A001,4B,1,3,Yes,orange"));
        assert!(csv.contains("\"Exterior, Day, 10-bit, 4K UHD, ProRes HQ, Sony FX3, Sony S-Log3\""));
    }
}
