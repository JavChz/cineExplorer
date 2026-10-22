use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct ContainerMetadata {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub focal_length: Option<String>,
    pub aperture: Option<String>,
    pub iso: Option<u32>,
    pub shutter_speed: Option<String>,
    pub white_balance: Option<String>,
    pub date_recorded: Option<String>,
    pub log_profile: Option<String>,
    pub color_gamut: Option<String>,
}

/// Helper function to parse big-endian or little-endian unsigned 16-bit integers
#[inline]
fn read_u16(buf: &[u8], offset: usize, le: bool) -> Option<u16> {
    if offset + 2 > buf.len() {
        return None;
    }
    if le {
        Some(u16::from_le_bytes([buf[offset], buf[offset + 1]]))
    } else {
        Some(u16::from_be_bytes([buf[offset], buf[offset + 1]]))
    }
}

/// Helper function to parse big-endian or little-endian unsigned 32-bit integers
#[inline]
fn read_u32(buf: &[u8], offset: usize, le: bool) -> Option<u32> {
    if offset + 4 > buf.len() {
        return None;
    }
    if le {
        Some(u32::from_le_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]))
    } else {
        Some(u32::from_be_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]))
    }
}

/// Parse a rational number (numerator, denominator) into a clean string representation
fn format_rational(num: u32, den: u32) -> Option<String> {
    if den == 0 {
        return None;
    }
    if num == 0 {
        return Some("0".to_string());
    }
    if num % den == 0 {
        Some((num / den).to_string())
    } else {
        Some(format!("{:.1}", num as f64 / den as f64))
    }
}

/// Parse exposure time into standard shutter speed representation (e.g., 1/50s)
fn format_shutter_speed(num: u32, den: u32) -> Option<String> {
    if num == 0 || den == 0 {
        return None;
    }
    if num == 1 {
        Some(format!("1/{}s", den))
    } else if den % num == 0 {
        Some(format!("1/{}s", den / num))
    } else {
        let sec = num as f64 / den as f64;
        if sec < 1.0 {
            let inv = (1.0 / sec).round() as u64;
            Some(format!("1/{}s", inv))
        } else {
            Some(format!("{:.1}s", sec))
        }
    }
}

/// Parse F-number into standard aperture format (e.g., f/2.8)
fn format_aperture(num: u32, den: u32) -> Option<String> {
    if num == 0 || den == 0 {
        return None;
    }
    let f = num as f64 / den as f64;
    Some(format!("f/{:.1}", f))
}

/// Parse focal length into standard format (e.g., 50mm)
fn format_focal_length(num: u32, den: u32) -> Option<String> {
    if num == 0 || den == 0 {
        return None;
    }
    let fl = num as f64 / den as f64;
    if (fl.fract()).abs() < 0.05 {
        Some(format!("{:.0}mm", fl))
    } else {
        Some(format!("{:.1}mm", fl))
    }
}

/// Parse an ASCII string from a TIFF entry
fn read_tiff_ascii(buf: &[u8], tiff_start: usize, count: u32, val_offset: u32) -> Option<String> {
    if count == 0 {
        return None;
    }
    let max_len = (count as usize).min(256);
    let slice = if count <= 4 {
        // Value is stored in-line in val_offset field
        &buf[val_offset as usize..val_offset as usize + max_len]
    } else {
        let start = tiff_start.checked_add(val_offset as usize)?;
        if start + max_len > buf.len() {
            return None;
        }
        &buf[start..start + max_len]
    };
    let s = String::from_utf8_lossy(slice);
    let trimmed = s.trim_matches(char::from(0)).trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Parse TIFF IFDs (IFD0, ExifIFD, MakerNotes) from a buffer starting at `tiff_start`
fn parse_tiff_block(buf: &[u8], tiff_start: usize, meta: &mut ContainerMetadata) {
    if tiff_start + 8 > buf.len() {
        return;
    }
    let le = if buf[tiff_start] == 0x49 && buf[tiff_start + 1] == 0x49 && buf[tiff_start + 2] == 0x2A && buf[tiff_start + 3] == 0x00 {
        true
    } else if buf[tiff_start] == 0x4D && buf[tiff_start + 1] == 0x4D && buf[tiff_start + 2] == 0x00 && buf[tiff_start + 3] == 0x2A {
        false
    } else {
        return;
    };

    let ifd0_offset = match read_u32(buf, tiff_start + 4, le) {
        Some(off) => off as usize,
        None => return,
    };

    let mut exif_ifd_offset = None;
    let mut makernote_offset = None;
    let mut makernote_size = 0;

    // 1. Parse IFD0
    let ifd0_pos = tiff_start + ifd0_offset;
    if ifd0_pos + 2 <= buf.len() {
        let entry_count = read_u16(buf, ifd0_pos, le).unwrap_or(0) as usize;
        for i in 0..entry_count {
            let entry_pos = ifd0_pos + 2 + i * 12;
            if entry_pos + 12 > buf.len() {
                break;
            }
            let tag = read_u16(buf, entry_pos, le).unwrap_or(0);
            let tag_type = read_u16(buf, entry_pos + 2, le).unwrap_or(0);
            let count = read_u32(buf, entry_pos + 4, le).unwrap_or(0);
            let val_or_offset = read_u32(buf, entry_pos + 8, le).unwrap_or(0);

            match tag {
                0x010F => {
                    // Make
                    if meta.camera_make.is_none() && tag_type == 2 {
                        if let Some(s) = read_tiff_ascii(buf, tiff_start, count, val_or_offset) {
                            meta.camera_make = Some(s);
                        }
                    }
                }
                0x0110 => {
                    // Model
                    if meta.camera_model.is_none() && tag_type == 2 {
                        if let Some(s) = read_tiff_ascii(buf, tiff_start, count, val_or_offset) {
                            meta.camera_model = Some(s);
                        }
                    }
                }
                0x0132 => {
                    // DateTime
                    if meta.date_recorded.is_none() && tag_type == 2 {
                        if let Some(s) = read_tiff_ascii(buf, tiff_start, count, val_or_offset) {
                            meta.date_recorded = Some(s);
                        }
                    }
                }
                0x8769 => {
                    // ExifIFD pointer
                    exif_ifd_offset = Some(val_or_offset as usize);
                }
                _ => {}
            }
        }
    }

    // 2. Parse ExifIFD
    if let Some(exif_offset) = exif_ifd_offset {
        let exif_pos = tiff_start + exif_offset;
        if exif_pos + 2 <= buf.len() {
            let entry_count = read_u16(buf, exif_pos, le).unwrap_or(0) as usize;
            for i in 0..entry_count {
                let entry_pos = exif_pos + 2 + i * 12;
                if entry_pos + 12 > buf.len() {
                    break;
                }
                let tag = read_u16(buf, entry_pos, le).unwrap_or(0);
                let tag_type = read_u16(buf, entry_pos + 2, le).unwrap_or(0);
                let count = read_u32(buf, entry_pos + 4, le).unwrap_or(0);
                let val_or_offset = read_u32(buf, entry_pos + 8, le).unwrap_or(0);

                match tag {
                    0x829A => {
                        // ExposureTime (Rational)
                        if meta.shutter_speed.is_none() && (tag_type == 5 || tag_type == 10) {
                            let r_pos = tiff_start + val_or_offset as usize;
                            if let (Some(num), Some(den)) = (read_u32(buf, r_pos, le), read_u32(buf, r_pos + 4, le)) {
                                meta.shutter_speed = format_shutter_speed(num, den);
                            }
                        }
                    }
                    0x829D => {
                        // FNumber (Rational)
                        if meta.aperture.is_none() && (tag_type == 5 || tag_type == 10) {
                            let r_pos = tiff_start + val_or_offset as usize;
                            if let (Some(num), Some(den)) = (read_u32(buf, r_pos, le), read_u32(buf, r_pos + 4, le)) {
                                meta.aperture = format_aperture(num, den);
                            }
                        }
                    }
                    0x8827 | 0x8832 => {
                        // ISOSpeedRatings or RecommendedExposureIndex
                        if meta.iso.is_none() || meta.iso == Some(0) {
                            if tag_type == 3 {
                                let iso_val = read_u16(buf, entry_pos + 8, le).unwrap_or(0) as u32;
                                if iso_val > 0 {
                                    meta.iso = Some(iso_val);
                                }
                            } else if tag_type == 4 {
                                if val_or_offset > 0 {
                                    meta.iso = Some(val_or_offset);
                                }
                            }
                        }
                    }
                    0x9003 => {
                        // DateTimeOriginal
                        if tag_type == 2 {
                            if let Some(s) = read_tiff_ascii(buf, tiff_start, count, val_or_offset) {
                                meta.date_recorded = Some(s);
                            }
                        }
                    }
                    0x920A => {
                        // FocalLength (Rational)
                        if meta.focal_length.is_none() && (tag_type == 5 || tag_type == 10) {
                            let r_pos = tiff_start + val_or_offset as usize;
                            if let (Some(num), Some(den)) = (read_u32(buf, r_pos, le), read_u32(buf, r_pos + 4, le)) {
                                meta.focal_length = format_focal_length(num, den);
                            }
                        }
                    }
                    0xA434 => {
                        // LensModel (ASCII)
                        if meta.lens_model.is_none() && tag_type == 2 {
                            if let Some(s) = read_tiff_ascii(buf, tiff_start, count, val_or_offset) {
                                meta.lens_model = Some(clean_lens_model(&s));
                            }
                        }
                    }
                    0x927C => {
                        // MakerNote pointer
                        makernote_offset = Some(val_or_offset as usize);
                        makernote_size = count as usize;
                    }
                    _ => {}
                }
            }
        }
    }

    // 3. Parse MakerNote
    if let Some(mn_offset) = makernote_offset {
        let mn_pos = tiff_start + mn_offset;
        if mn_pos + 4 <= buf.len() {
            // Check if Panasonic header ("Panasonic\0\0\0" 12 bytes)
            let is_panasonic = mn_pos + 12 <= buf.len() && &buf[mn_pos..mn_pos + 9] == b"Panasonic";
            let actual_mn_pos = if is_panasonic { mn_pos + 12 } else { mn_pos };

            if actual_mn_pos + 2 <= buf.len() {
                let entry_count = read_u16(buf, actual_mn_pos, le).unwrap_or(0) as usize;
                // Guard against unreasonable counts in corrupt or encrypted notes
                if entry_count < 300 {
                    for i in 0..entry_count {
                        let entry_pos = actual_mn_pos + 2 + i * 12;
                        if entry_pos + 12 > buf.len() {
                            break;
                        }
                        let tag = read_u16(buf, entry_pos, le).unwrap_or(0);
                        let tag_type = read_u16(buf, entry_pos + 2, le).unwrap_or(0);
                        let count = read_u32(buf, entry_pos + 4, le).unwrap_or(0);
                        let val_or_offset = read_u32(buf, entry_pos + 8, le).unwrap_or(0);

                        if is_panasonic {
                            // Panasonic MakerNote tags
                            if tag == 0x0051 && tag_type == 2 && meta.lens_model.is_none() {
                                if let Some(s) = read_tiff_ascii(buf, tiff_start, count, val_or_offset) {
                                    meta.lens_model = Some(clean_lens_model(&s));
                                }
                            }
                        } else {
                            // Canon MakerNote tags
                            match tag {
                                0x0095 => {
                                    // Canon LensModel
                                    if meta.lens_model.is_none() && tag_type == 2 {
                                        if let Some(s) = read_tiff_ascii(buf, tiff_start, count, val_or_offset) {
                                            meta.lens_model = Some(clean_lens_model(&s));
                                        }
                                    }
                                }
                                0x00A0 => {
                                    // ProcessingInfo (White balance color temperature at index 9)
                                    if meta.white_balance.is_none() && tag_type == 3 && count >= 10 {
                                        let p = tiff_start + val_or_offset as usize + 9 * 2;
                                        if let Some(kelvin) = read_u16(buf, p, le) {
                                            if kelvin >= 2000 && kelvin <= 15000 {
                                                meta.white_balance = Some(format!("{}K", kelvin));
                                            }
                                        }
                                    }
                                }
                                0x4026 => {
                                    // CanonLogInfo (Array of U32)
                                    // Index 3: CanonLog Mode (1 = ON, 0 = OFF)
                                    // Index 11: CanonLog Curve (1 = C-Log, 2 = C-Log2, 3 = C-Log3)
                                    // Index 9: Color Space / Gamut (1 = BT.709, 2 = BT.2020, 3 = Cinema Gamut)
                                    if tag_type == 4 && count >= 12 {
                                        let p = tiff_start + val_or_offset as usize;
                                        let is_log_on = read_u32(buf, p + 3 * 4, le).unwrap_or(0);
                                        let curve_type = read_u32(buf, p + 11 * 4, le).unwrap_or(0);
                                        let gamut_type = read_u32(buf, p + 9 * 4, le).unwrap_or(0);

                                        if is_log_on == 1 {
                                            let profile_name = match curve_type {
                                                2 => "Canon C-Log2",
                                                3 => "Canon C-Log3",
                                                _ => "Canon C-Log",
                                            };
                                            meta.log_profile = Some(profile_name.to_string());

                                            let gamut_name = match gamut_type {
                                                2 => "BT.2020",
                                                3 => "Cinema Gamut",
                                                _ => "BT.709",
                                            };
                                            meta.color_gamut = Some(gamut_name.to_string());
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Strip non-ASCII trailing artifacts from lens model strings
fn clean_lens_model(raw: &str) -> String {
    let mut clean = String::new();
    for c in raw.chars() {
        if c.is_ascii_graphic() || c == ' ' {
            clean.push(c);
        } else {
            break;
        }
    }
    clean.trim().to_string()
}

/// Parse Sony Non-Real-Time XML metadata string
pub fn parse_sony_xml_metadata(xml_text: &str, meta: &mut ContainerMetadata) {
    // 1. Device Manufacturer & Model
    // Example: <Device manufacturer="Sony" modelName="ILME-FX3" serialNo="..."/>
    if meta.camera_make.is_none() {
        if let Some(pos) = xml_text.find("manufacturer=\"") {
            let start = pos + 14;
            if let Some(end) = xml_text[start..].find('"') {
                let m = xml_text[start..start + end].trim();
                if !m.is_empty() {
                    meta.camera_make = Some(m.to_string());
                }
            }
        }
    }
    if meta.camera_model.is_none() {
        if let Some(pos) = xml_text.find("modelName=\"") {
            let start = pos + 11;
            if let Some(end) = xml_text[start..].find('"') {
                let m = xml_text[start..start + end].trim();
                if !m.is_empty() {
                    meta.camera_model = Some(m.to_string());
                }
            }
        }
    }

    // 2. Lens Model
    // Example: <Lens modelName="FE 24-70mm F2.8 GM II"/>
    if meta.lens_model.is_none() {
        if let Some(pos) = xml_text.find("<Lens ") {
            if let Some(sub) = xml_text[pos..].split('>').next() {
                if let Some(m_pos) = sub.find("modelName=\"") {
                    let start = m_pos + 11;
                    if let Some(end) = sub[start..].find('"') {
                        let l = sub[start..start + end].trim();
                        if !l.is_empty() {
                            meta.lens_model = Some(clean_lens_model(l));
                        }
                    }
                }
            }
        }
    }

    // 3. Log Gamma Equation & Color Primaries
    // Example: <Item name="CaptureGammaEquation" value="s-log3"/>
    if meta.log_profile.is_none() {
        if let Some(pos) = xml_text.find("CaptureGammaEquation\" value=\"") {
            let start = pos + 29;
            if let Some(end) = xml_text[start..].find('"') {
                let gamma = xml_text[start..start + end].trim().to_lowercase();
                if gamma.contains("s-log3") || gamma.contains("slog3") {
                    meta.log_profile = Some("Sony S-Log3".to_string());
                } else if gamma.contains("s-log2") || gamma.contains("slog2") {
                    meta.log_profile = Some("Sony S-Log2".to_string());
                } else if gamma.contains("s-cinetone") {
                    meta.log_profile = Some("Sony S-Cinetone".to_string());
                } else if gamma.contains("hlg") {
                    meta.log_profile = Some("HLG".to_string());
                }
            }
        }
    }

    if meta.color_gamut.is_none() {
        if let Some(pos) = xml_text.find("CaptureColorPrimaries\" value=\"") {
            let start = pos + 30;
            if let Some(end) = xml_text[start..].find('"') {
                let gamut = xml_text[start..start + end].trim();
                if !gamut.is_empty() {
                    meta.color_gamut = Some(gamut.to_string());
                }
            }
        }
    }
}

/// Parse Panasonic Semi-Pro XML metadata string
pub fn parse_panasonic_xml_metadata(xml_text: &str, meta: &mut ContainerMetadata) {
    // 1. Manufacturer & Model
    // <Manufacturer>Panasonic</Manufacturer>
    // <ModelName>DC-S5</ModelName>
    if meta.camera_make.is_none() {
        if let Some(start_tag) = xml_text.find("<Manufacturer>") {
            let start = start_tag + 14;
            if let Some(end_tag) = xml_text[start..].find("</Manufacturer>") {
                meta.camera_make = Some(xml_text[start..start + end_tag].trim().to_string());
            }
        }
    }
    if meta.camera_model.is_none() {
        if let Some(start_tag) = xml_text.find("<ModelName>") {
            let start = start_tag + 11;
            if let Some(end_tag) = xml_text[start..].find("</ModelName>") {
                meta.camera_model = Some(xml_text[start..start + end_tag].trim().to_string());
            }
        }
    }

    // 2. ISO Sensitivity
    // <ISOSensitivity>800</ISOSensitivity>
    if meta.iso.is_none() || meta.iso == Some(0) {
        if let Some(start_tag) = xml_text.find("<ISOSensitivity>") {
            let start = start_tag + 16;
            if let Some(end_tag) = xml_text[start..].find("</ISOSensitivity>") {
                if let Ok(v) = xml_text[start..start + end_tag].trim().parse::<u32>() {
                    meta.iso = Some(v);
                }
            }
        }
    }

    // 3. Log Gamma & Gamut
    // <CaptureGamma>V-Log</CaptureGamma>
    // <CaptureGamut>V-Gamut</CaptureGamut>
    if meta.log_profile.is_none() {
        if let Some(start_tag) = xml_text.find("<CaptureGamma>") {
            let start = start_tag + 14;
            if let Some(end_tag) = xml_text[start..].find("</CaptureGamma>") {
                let gamma = xml_text[start..start + end_tag].trim();
                if gamma.eq_ignore_ascii_case("V-Log") || gamma.eq_ignore_ascii_case("V-Log L") {
                    meta.log_profile = Some("Panasonic V-Log".to_string());
                } else if gamma.eq_ignore_ascii_case("HLG") {
                    meta.log_profile = Some("HLG".to_string());
                } else if !gamma.is_empty() {
                    meta.log_profile = Some(gamma.to_string());
                }
            }
        }
    }

    if meta.color_gamut.is_none() {
        if let Some(start_tag) = xml_text.find("<CaptureGamut>") {
            let start = start_tag + 14;
            if let Some(end_tag) = xml_text[start..].find("</CaptureGamut>") {
                let gamut = xml_text[start..start + end_tag].trim();
                if !gamut.is_empty() {
                    meta.color_gamut = Some(gamut.to_string());
                }
            }
        }
    }
}

/// Parse Blackmagic RAW bmdf frame header inside mdat (usually within first 16KB)
fn parse_braw_bmdf_packet(buf: &[u8], meta: &mut ContainerMetadata) {
    if let Some(pos) = buf.windows(4).position(|w| w == b"bmdf") {
        let chunk = &buf[pos..buf.len().min(pos + 1024)];
        let chunk_str = String::from_utf8_lossy(chunk);

        // Aperture e.g. " aptrf1.25"
        if meta.aperture.is_none() {
            if let Some(idx) = chunk_str.find("aptr") {
                let val_str = &chunk_str[idx + 4..];
                let word: String = val_str.chars().skip_while(|c| c.is_whitespace()).take_while(|c| !c.is_whitespace() && c.is_ascii_graphic()).collect();
                if !word.is_empty() {
                    meta.aperture = Some(word);
                }
            }
        }

        // Focal length e.g. " fcln13mm"
        if meta.focal_length.is_none() {
            if let Some(idx) = chunk_str.find("fcln") {
                let val_str = &chunk_str[idx + 4..];
                let word: String = val_str.chars().skip_while(|c| c.is_whitespace()).take_while(|c| !c.is_whitespace() && c.is_ascii_graphic()).collect();
                if !word.is_empty() {
                    meta.focal_length = Some(word);
                }
            }
        }

        // Shutter speed / angle e.g. " shtv75°"
        if meta.shutter_speed.is_none() {
            if let Some(idx) = chunk_str.find("shtv") {
                let val_str = &chunk_str[idx + 4..];
                let word: String = val_str.chars().skip_while(|c| c.is_whitespace()).take_while(|c| !c.is_whitespace() && c.is_ascii_graphic()).collect();
                if !word.is_empty() {
                    meta.shutter_speed = Some(word);
                }
            }
        }

        // ISO e.g. "isoe" followed by 32-bit int
        if meta.iso.is_none() || meta.iso == Some(0) {
            if let Some(idx) = chunk.windows(4).position(|w| w == b"isoe") {
                if idx + 8 <= chunk.len() {
                    let v = u32::from_be_bytes([chunk[idx + 4], chunk[idx + 5], chunk[idx + 6], chunk[idx + 7]]);
                    if v > 0 && v < 1000000 {
                        meta.iso = Some(v);
                    }
                }
            }
        }

        // White balance Kelvin e.g. "wkel" followed by 32-bit int
        if meta.white_balance.is_none() {
            if let Some(idx) = chunk.windows(4).position(|w| w == b"wkel") {
                if idx + 8 <= chunk.len() {
                    let kelvin = u32::from_be_bytes([chunk[idx + 4], chunk[idx + 5], chunk[idx + 6], chunk[idx + 7]]);
                    if kelvin >= 2000 && kelvin <= 20000 {
                        meta.white_balance = Some(format!("{}K", kelvin));
                    }
                }
            }
        }
    }
}

/// Scan a buffer for any embedded TIFF headers (`II*\0` or `MM\0*`) and parse them
fn scan_buffer_for_tiff(buf: &[u8], meta: &mut ContainerMetadata) {
    if buf.len() < 16 {
        return;
    }
    // Scan every 2 or 4 bytes for TIFF signatures
    let max_scan = buf.len() - 16;
    for i in 0..max_scan {
        if (buf[i] == 0x49 && buf[i + 1] == 0x49 && buf[i + 2] == 0x2A && buf[i + 3] == 0x00)
            || (buf[i] == 0x4D && buf[i + 1] == 0x4D && buf[i + 2] == 0x00 && buf[i + 3] == 0x2A)
        {
            parse_tiff_block(buf, i, meta);
        }
    }
}

/// Recursively walk MP4/MOV container boxes without loading video sample data into memory
fn walk_container_boxes(
    file: &mut File,
    start: u64,
    end: u64,
    depth: usize,
    meta: &mut ContainerMetadata,
) -> std::io::Result<()> {
    if depth > 10 {
        return Ok(());
    }
    let mut pos = start;
    let mut header_buf = [0u8; 16];

    while pos + 8 <= end {
        file.seek(SeekFrom::Start(pos))?;
        if file.read_exact(&mut header_buf).is_err() {
            break;
        }

        let sz32 = u32::from_be_bytes([header_buf[0], header_buf[1], header_buf[2], header_buf[3]]) as u64;
        let box_type = &header_buf[4..8];

        let (box_size, header_size) = if sz32 == 1 {
            let sz64 = u64::from_be_bytes([
                header_buf[8],
                header_buf[9],
                header_buf[10],
                header_buf[11],
                header_buf[12],
                header_buf[13],
                header_buf[14],
                header_buf[15],
            ]);
            (sz64, 16u64)
        } else if sz32 == 0 {
            (end - pos, 8u64)
        } else {
            (sz32, 8u64)
        };

        if box_size < header_size || pos + box_size > end {
            break;
        }

        match box_type {
            b"moov" | b"trak" | b"mdia" | b"minf" | b"udta" | b"CNTH" => {
                walk_container_boxes(file, pos + header_size, pos + box_size, depth + 1, meta)?;
            }
            b"meta" => {
                // meta can be a FullBox (header_size = 12) or normal box
                let mut peek = [0u8; 8];
                let _ = file.read_exact(&mut peek);
                let sub_type = &peek[4..8];
                let inner_start = if sub_type == b"hdlr" || sub_type == b"keys" || sub_type == b"ilst" || sub_type == b"xml " {
                    pos + header_size
                } else {
                    pos + header_size + 4
                };
                walk_container_boxes(file, inner_start, pos + box_size, depth + 1, meta)?;
            }
            b"uuid" => {
                // Read UUID payload (max 256KB)
                let payload_len = ((box_size - header_size) as usize).min(262144);
                if payload_len > 16 {
                    let mut uuid_buf = vec![0u8; payload_len];
                    file.seek(SeekFrom::Start(pos + header_size))?;
                    if file.read_exact(&mut uuid_buf).is_ok() {
                        scan_buffer_for_tiff(&uuid_buf, meta);

                        // Also check if text XML is inside the UUID payload
                        let s = String::from_utf8_lossy(&uuid_buf);
                        if s.contains("nonRealTimeMeta") || s.contains("CaptureGammaEquation") {
                            parse_sony_xml_metadata(&s, meta);
                        }
                    }
                }
            }
            b"PANA" => {
                // Panasonic MakerNote & thumbnail container box
                let payload_len = ((box_size - header_size) as usize).min(131072);
                let mut pana_buf = vec![0u8; payload_len];
                file.seek(SeekFrom::Start(pos + header_size))?;
                if file.read_exact(&mut pana_buf).is_ok() {
                    scan_buffer_for_tiff(&pana_buf, meta);
                }
            }
            b"xml " => {
                // Sony NRT XML atom in meta box
                let payload_len = ((box_size - header_size) as usize).min(131072);
                let mut xml_buf = vec![0u8; payload_len];
                file.seek(SeekFrom::Start(pos + header_size))?;
                if file.read_exact(&mut xml_buf).is_ok() {
                    let s = String::from_utf8_lossy(&xml_buf);
                    parse_sony_xml_metadata(&s, meta);
                }
            }
            b"ilst" | b"keys" => {
                // Apple QuickTime / MP4 metadata list
                let payload_len = ((box_size - header_size) as usize).min(131072);
                let mut ilst_buf = vec![0u8; payload_len];
                file.seek(SeekFrom::Start(pos + header_size))?;
                if file.read_exact(&mut ilst_buf).is_ok() {
                    let s = String::from_utf8_lossy(&ilst_buf);
                    if s.contains("com.panasonic") || s.contains("<ClipMain") {
                        parse_panasonic_xml_metadata(&s, meta);
                    }
                    if s.contains("nonRealTimeMeta") || s.contains("CaptureGammaEquation") {
                        parse_sony_xml_metadata(&s, meta);
                    }
                    scan_buffer_for_tiff(&ilst_buf, meta);
                }
            }
            b"mdat" => {
                // In BRAW files, the bmdf packet is in the first 16KB of mdat
                let peek_len = ((box_size - header_size) as usize).min(16384);
                let mut mdat_peek = vec![0u8; peek_len];
                file.seek(SeekFrom::Start(pos + header_size))?;
                if file.read_exact(&mut mdat_peek).is_ok() {
                    parse_braw_bmdf_packet(&mdat_peek, meta);
                }
            }
            _ => {}
        }

        pos += box_size;
    }

    Ok(())
}

/// Check for adjacent Sony sidecar XML files (e.g. `C0001M01.XML` for `C0001.MP4`)
fn check_sony_sidecar_xml(path: &Path, meta: &mut ContainerMetadata) {
    let parent = match path.parent() {
        Some(p) => p,
        None => return,
    };
    let file_stem = match path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s,
        None => return,
    };

    // Sony naming conventions: <stem>M01.XML or <stem>.XML
    let candidates = [
        parent.join(format!("{}M01.XML", file_stem)),
        parent.join(format!("{}M01.xml", file_stem)),
        parent.join(format!("{}.XML", file_stem)),
        parent.join(format!("{}.xml", file_stem)),
    ];

    for candidate in &candidates {
        if candidate.is_file() {
            if let Ok(content) = std::fs::read_to_string(candidate) {
                parse_sony_xml_metadata(&content, meta);
                break;
            }
        }
    }
}

/// Extract comprehensive camera, lens, exposure, and Log metadata directly from video containers
pub fn extract_container_metadata<P: AsRef<Path>>(file_path: P) -> ContainerMetadata {
    let path = file_path.as_ref();
    let mut meta = ContainerMetadata::default();

    // Check for Sony sidecar XML first (if present)
    check_sony_sidecar_xml(path, &mut meta);

    // Open file and walk MP4/MOV/BRAW box hierarchy
    if let Ok(mut file) = File::open(path) {
        if let Ok(len) = file.metadata().map(|m| m.len()) {
            let _ = walk_container_boxes(&mut file, 0, len, 0, &mut meta);
        }
    }

    meta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_lens_model() {
        let dirty = "TAMRON SP 150-600mm F/5-6.3 Di VC USD G2 A022 \u{0}\u{1}?p\u{0}";
        assert_eq!(
            clean_lens_model(dirty),
            "TAMRON SP 150-600mm F/5-6.3 Di VC USD G2 A022"
        );
    }

    #[test]
    fn test_parse_panasonic_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ClipMain>
  <ClipMetadata>
    <Device>
      <Manufacturer>Panasonic</Manufacturer>
      <ModelName>DC-S5</ModelName>
    </Device>
  </ClipMetadata>
  <UserArea>
    <CameraUnitMetadata>
      <ISOSensitivity>800</ISOSensitivity>
      <Gamma>
        <CaptureGamma>V-Log</CaptureGamma>
      </Gamma>
      <Gamut>
        <CaptureGamut>V-Gamut</CaptureGamut>
      </Gamut>
    </CameraUnitMetadata>
  </UserArea>
</ClipMain>"#;
        let mut meta = ContainerMetadata::default();
        parse_panasonic_xml_metadata(xml, &mut meta);
        assert_eq!(meta.camera_make.as_deref(), Some("Panasonic"));
        assert_eq!(meta.camera_model.as_deref(), Some("DC-S5"));
        assert_eq!(meta.iso, Some(800));
        assert_eq!(meta.log_profile.as_deref(), Some("Panasonic V-Log"));
        assert_eq!(meta.color_gamut.as_deref(), Some("V-Gamut"));
    }

    #[test]
    fn test_parse_sony_xml() {
        let xml = r#"<NonRealTimeMeta>
  <Device manufacturer="Sony" modelName="ILME-FX3"/>
  <Lens modelName="FE 24-70mm F2.8 GM II"/>
  <Item name="CaptureGammaEquation" value="s-log3"/>
  <Item name="CaptureColorPrimaries" value="s-gamut3.cine"/>
</NonRealTimeMeta>"#;
        let mut meta = ContainerMetadata::default();
        parse_sony_xml_metadata(xml, &mut meta);
        assert_eq!(meta.camera_make.as_deref(), Some("Sony"));
        assert_eq!(meta.camera_model.as_deref(), Some("ILME-FX3"));
        assert_eq!(meta.lens_model.as_deref(), Some("FE 24-70mm F2.8 GM II"));
        assert_eq!(meta.log_profile.as_deref(), Some("Sony S-Log3"));
        assert_eq!(meta.color_gamut.as_deref(), Some("s-gamut3.cine"));
    }
}
