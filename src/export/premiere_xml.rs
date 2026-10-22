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
    let mut url = String::from("file://localhost/");
    if let Some(stripped) = normalized.strip_prefix('/') {
        url.push_str(stripped);
    } else {
        url.push_str(&normalized);
    }
    url.replace(' ', "%20")
}

pub struct PremiereXmlExportOptions {
    pub project_name: String,
    pub include_keywords: bool,
    pub include_slate: bool,
}

impl Default for PremiereXmlExportOptions {
    fn default() -> Self {
        Self {
            project_name: "cineExplorer Export".to_string(),
            include_keywords: true,
            include_slate: true,
        }
    }
}

pub fn generate_premiere_xml(videos: &[VideoRecord], options: &PremiereXmlExportOptions) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<!DOCTYPE xmeml>\n");
    out.push_str("<xmeml version=\"5\">\n");
    out.push_str("  <project>\n");
    out.push_str(&format!("    <name>{}</name>\n", xml_escape(&options.project_name)));
    out.push_str("    <children>\n");

    // Media Bin
    out.push_str("      <bin>\n");
    out.push_str(&format!("        <name>{}</name>\n", xml_escape(&options.project_name)));
    out.push_str("        <children>\n");

    for (i, v) in videos.iter().enumerate() {
        let clip_id = format!("clip-{}", i + 1);
        let file_id = format!("file-{}", i + 1);
        let fps = if v.fps > 0.0 { v.fps } else { 24.0 };
        let timebase = fps.round() as u32;
        let frame_count = if v.frame_count > 0 {
            v.frame_count
        } else {
            (v.duration_seconds * fps).round() as u32
        }.max(1);

        let file_url = path_to_file_url(&v.file_path);

        out.push_str(&format!("          <clip id=\"{}\">\n", clip_id));
        out.push_str(&format!("            <name>{}</name>\n", xml_escape(&v.file_name)));
        out.push_str(&format!("            <duration>{}</duration>\n", frame_count));
        out.push_str("            <rate>\n");
        out.push_str(&format!("              <timebase>{}</timebase>\n", timebase));
        out.push_str("              <ntsc>FALSE</ntsc>\n");
        out.push_str("            </rate>\n");

        out.push_str("            <media>\n");
        out.push_str("              <video>\n");
        out.push_str("                <track>\n");
        out.push_str(&format!("                  <clipitem id=\"item-{}\">\n", i + 1));
        out.push_str(&format!("                    <name>{}</name>\n", xml_escape(&v.file_name)));
        out.push_str(&format!("                    <file id=\"{}\">\n", file_id));
        out.push_str(&format!("                      <name>{}</name>\n", xml_escape(&v.file_name)));
        out.push_str(&format!("                      <pathurl>{}</pathurl>\n", file_url));
        out.push_str("                      <rate>\n");
        out.push_str(&format!("                        <timebase>{}</timebase>\n", timebase));
        out.push_str("                        <ntsc>FALSE</ntsc>\n");
        out.push_str("                      </rate>\n");
        out.push_str(&format!("                      <duration>{}</duration>\n", frame_count));
        out.push_str("                      <media>\n");
        out.push_str("                        <video>\n");
        out.push_str("                          <samplecharacteristics>\n");
        out.push_str(&format!("                            <width>{}</width>\n", v.width.max(1920)));
        out.push_str(&format!("                            <height>{}</height>\n", v.height.max(1080)));
        out.push_str("                          </samplecharacteristics>\n");
        out.push_str("                        </video>\n");
        out.push_str("                      </media>\n");
        out.push_str("                    </file>\n");

        // Logging info for Premiere Pro
        out.push_str("                    <logginginfo>\n");
        if options.include_slate {
            if !v.scene.is_empty() {
                out.push_str(&format!("                      <scene>{}</scene>\n", xml_escape(&v.scene)));
            }
            if !v.shot.is_empty() || !v.take.is_empty() {
                let shottake = format!("{}/{}", v.shot, v.take);
                out.push_str(&format!("                      <shottake>{}</shottake>\n", xml_escape(&shottake)));
            }
            if !v.reel.is_empty() {
                out.push_str(&format!("                      <reel>{}</reel>\n", xml_escape(&v.reel)));
            }
        }

        // Keywords in Log Note
        if options.include_keywords {
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
            let lognote = format!("Keywords: {}", kw_list.join(", "));
            out.push_str(&format!("                      <lognote>{}</lognote>\n", xml_escape(&lognote)));
        }

        out.push_str(&format!(
            "                      <good>{}</good>\n",
            if v.flag == PickFlag::Picked { "TRUE" } else { "FALSE" }
        ));
        out.push_str("                    </logginginfo>\n");

        // Color label
        if !v.color_label.is_empty() {
            out.push_str("                    <labels>\n");
            out.push_str(&format!("                      <label2>{}</label2>\n", xml_escape(&v.color_label)));
            out.push_str("                    </labels>\n");
        }

        out.push_str("                  </clipitem>\n");
        out.push_str("                </track>\n");
        out.push_str("              </video>\n");
        out.push_str("            </media>\n");
        out.push_str("          </clip>\n");
    }

    out.push_str("        </children>\n");
    out.push_str("      </bin>\n");
    out.push_str("    </children>\n");
    out.push_str("  </project>\n");
    out.push_str("</xmeml>\n");

    out
}
