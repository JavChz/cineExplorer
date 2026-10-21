use std::path::Path;
use crate::db::models::{PickFlag, VideoRecord};

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn path_to_file_url(path_str: &str) -> String {
    let p = Path::new(path_str);
    let normalized = p.to_string_lossy().replace('\\', "/");
    let mut url = String::from("file:///");
    if let Some(stripped) = normalized.strip_prefix('/') {
        url.push_str(stripped);
    } else {
        url.push_str(&normalized);
    }
    // Encode spaces
    url.replace(' ', "%20")
}

pub struct FcpXmlExportOptions {
    pub project_name: String,
    pub include_user_tags: bool,
    pub include_tech_keywords: bool,
    pub include_slate: bool,
}

impl Default for FcpXmlExportOptions {
    fn default() -> Self {
        Self {
            project_name: "cineExplorer Export".to_string(),
            include_user_tags: true,
            include_tech_keywords: true,
            include_slate: true,
        }
    }
}

pub fn generate_resolve_fcpxml(videos: &[VideoRecord], options: &FcpXmlExportOptions) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<!DOCTYPE fcpxml>\n");
    out.push_str("<fcpxml version=\"1.9\">\n");

    // 1. Resources
    out.push_str("  <resources>\n");

    // Default project format: 4K UHD or 1080p, 24fps
    let (proj_w, proj_h) = if let Some(v) = videos.first() {
        (v.width.max(1920), v.height.max(1080))
    } else {
        (3840, 2160)
    };
    out.push_str(&format!(
        "    <format id=\"r_proj\" name=\"FFVideoFormatRateUndefined\" width=\"{}\" height=\"{}\" frameDuration=\"100/2400s\"/>\n",
        proj_w, proj_h
    ));

    // Asset resources for each video clip
    for (i, v) in videos.iter().enumerate() {
        let asset_id = format!("r_asset_{}", i + 1);
        let file_url = path_to_file_url(&v.file_path);
        let dur = v.duration_seconds.max(0.1);

        out.push_str(&format!(
            "    <asset id=\"{}\" name=\"{}\" src=\"{}\" start=\"0s\" duration=\"{:.4}s\" hasVideo=\"1\" hasAudio=\"{}\">\n",
            asset_id,
            xml_escape(&v.file_name),
            file_url,
            dur,
            if v.has_audio { "1" } else { "0" },
        ));

        // Metadata block (Resolve reads these directly into Media Pool metadata columns!)
        out.push_str("      <metadata>\n");
        if options.include_slate {
            if !v.scene.is_empty() {
                out.push_str(&format!("        <md key=\"com.apple.proapps.studio.scene\" value=\"{}\"/>\n", xml_escape(&v.scene)));
            }
            if !v.shot.is_empty() {
                out.push_str(&format!("        <md key=\"com.apple.proapps.studio.shot\" value=\"{}\"/>\n", xml_escape(&v.shot)));
            }
            if !v.take.is_empty() {
                out.push_str(&format!("        <md key=\"com.apple.proapps.studio.take\" value=\"{}\"/>\n", xml_escape(&v.take)));
            }
            if !v.reel.is_empty() {
                out.push_str(&format!("        <md key=\"com.apple.proapps.studio.reel\" value=\"{}\"/>\n", xml_escape(&v.reel)));
            }
            if !v.camera_unit.is_empty() {
                out.push_str(&format!("        <md key=\"com.apple.proapps.studio.camera\" value=\"{}\"/>\n", xml_escape(&v.camera_unit)));
            }
        }
        if !v.camera_model.is_empty() {
            out.push_str(&format!("        <md key=\"com.apple.proapps.studio.cameraName\" value=\"{}\"/>\n", xml_escape(&v.camera_model)));
        }
        if !v.notes.is_empty() {
            out.push_str(&format!("        <md key=\"com.apple.proapps.studio.comment\" value=\"{}\"/>\n", xml_escape(&v.notes)));
        }
        out.push_str("      </metadata>\n");
        out.push_str("    </asset>\n");
    }
    out.push_str("  </resources>\n");

    // 2. Library & Event
    out.push_str("  <library>\n");
    out.push_str(&format!("    <event name=\"{}\">\n", xml_escape(&options.project_name)));
    out.push_str(&format!("      <project name=\"{}\">\n", xml_escape(&options.project_name)));

    let total_dur: f32 = videos.iter().map(|v| v.duration_seconds.max(0.1)).sum();
    out.push_str(&format!(
        "        <sequence format=\"r_proj\" duration=\"{:.4}s\" tcStart=\"0s\">\n",
        total_dur.max(1.0)
    ));
    out.push_str("          <spine>\n");

    let mut timeline_offset = 0.0_f32;
    for (i, v) in videos.iter().enumerate() {
        let asset_id = format!("r_asset_{}", i + 1);
        let dur = v.duration_seconds.max(0.1);

        out.push_str(&format!(
            "            <ref-clip name=\"{}\" ref=\"{}\" offset=\"{:.4}s\" duration=\"{:.4}s\" start=\"0s\">\n",
            xml_escape(&v.file_name),
            asset_id,
            timeline_offset,
            dur,
        ));

        // Keywords list for DaVinci Resolve
        let mut keywords = Vec::new();

        // 1. User Tags
        if options.include_user_tags {
            for tag in &v.tags {
                if !tag.trim().is_empty() {
                    keywords.push(tag.trim().to_string());
                }
            }
        }

        // 2. Technical and format keywords (creates Smart Keyword Bins in Resolve!)
        if options.include_tech_keywords {
            // Bit depth
            keywords.push(format!("{}-bit", v.bit_depth));
            // Resolution
            keywords.push(v.resolution_label());
            // Codec display
            keywords.push(v.codec_display_name());
            // Container
            if !v.container.is_empty() {
                keywords.push(v.container.to_uppercase());
            }
            // Camera
            if !v.camera_model.is_empty() {
                keywords.push(v.camera_model.clone());
            }
            // Pick / Reject
            match v.flag {
                PickFlag::Picked => keywords.push("Pick".to_string()),
                PickFlag::Rejected => keywords.push("Reject".to_string()),
                PickFlag::Unflagged => {}
            }
            // Color label
            if !v.color_label.is_empty() {
                keywords.push(format!("Color: {}", v.color_label));
            }
            // Star ratings
            if v.rating > 0 {
                keywords.push(format!("{} Stars", v.rating));
            }
        }

        // Write each keyword element inside ref-clip
        let mut seen = std::collections::HashSet::new();
        for kw in keywords {
            if seen.insert(kw.clone()) {
                out.push_str(&format!(
                    "              <keyword start=\"0s\" duration=\"{:.4}s\" value=\"{}\"/>\n",
                    dur,
                    xml_escape(&kw)
                ));
            }
        }

        out.push_str("            </ref-clip>\n");
        timeline_offset += dur;
    }

    out.push_str("          </spine>\n");
    out.push_str("        </sequence>\n");
    out.push_str("      </project>\n");
    out.push_str("    </event>\n");
    out.push_str("  </library>\n");
    out.push_str("</fcpxml>\n");

    out
}
