use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum PickFlag {
    Unflagged = 0,
    Picked = 1,
    Rejected = 2,
}

impl From<i64> for PickFlag {
    fn from(val: i64) -> Self {
        match val {
            1 => PickFlag::Picked,
            2 => PickFlag::Rejected,
            _ => PickFlag::Unflagged,
        }
    }
}

impl PickFlag {
    pub fn as_i64(&self) -> i64 {
        match self {
            PickFlag::Unflagged => 0,
            PickFlag::Picked => 1,
            PickFlag::Rejected => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRecord {
    pub id: i64,
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
    pub container: String,
    pub codec: String,
    pub codec_long_name: String,
    pub profile: String,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub frame_count: u32,
    pub duration_seconds: f32,
    pub bit_depth: u16,
    pub pixel_format: String,
    pub color_space: String,
    pub color_primaries: String,
    pub color_transfer: String,
    pub bitrate: u64,
    pub has_audio: bool,
    pub audio_codec: String,
    pub audio_channels: u16,
    pub audio_sample_rate: u32,
    pub camera_make: String,
    pub camera_model: String,
    pub lens_model: String,
    pub focal_length: String,
    pub aperture: String,
    pub iso: u32,
    pub shutter_speed: String,
    pub white_balance: String,
    pub date_recorded: Option<String>,
    pub imported_at: String,
    pub rating: u8,
    pub flag: PickFlag,
    pub notes: String,
    pub color_label: String,
    pub scene: String,
    pub shot: String,
    pub take: String,
    pub reel: String,
    pub location: String,
    pub camera_unit: String,
    pub log_profile: String,
    pub tags: Vec<String>,
}

impl VideoRecord {
    pub fn slate_label(&self) -> Option<String> {
        fn is_valid_slate_val(s: &str) -> bool {
            let t = s.trim();
            !t.is_empty()
                && !t.eq_ignore_ascii_case("none")
                && !t.eq_ignore_ascii_case("null")
                && !t.eq_ignore_ascii_case("n/a")
                && !t.eq_ignore_ascii_case("undefined")
        }

        let mut parts = Vec::new();
        if is_valid_slate_val(&self.scene) {
            parts.push(format!("Sc {}", self.scene.trim()));
        }
        if is_valid_slate_val(&self.shot) {
            parts.push(format!("Sh {}", self.shot.trim()));
        }
        if is_valid_slate_val(&self.take) {
            parts.push(format!("Tk {}", self.take.trim()));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" · "))
        }
    }

    pub fn duration_tc(&self) -> String {
        let t = self.duration_seconds.max(0.0);
        let h = (t / 3600.0) as u32;
        let m = ((t % 3600.0) / 60.0) as u32;
        let s = (t % 60.0) as u32;
        let f = ((t.fract()) * self.fps.max(1.0)) as u32;

        if h > 0 {
            format!("{:02}:{:02}:{:02}:{:02}", h, m, s, f)
        } else {
            format!("{:02}:{:02}:{:02}", m, s, f)
        }
    }

    pub fn resolution_label(&self) -> String {
        if self.width >= 7680 || self.height >= 4320 {
            "8K".to_string()
        } else if self.width >= 5760 || self.height >= 3240 {
            "6K".to_string()
        } else if self.width >= 3840 || self.height >= 2160 {
            "4K UHD".to_string()
        } else if self.width >= 2048 {
            "2K DCI".to_string()
        } else if self.width >= 1920 || self.height >= 1080 {
            "1080p".to_string()
        } else if self.width >= 1280 || self.height >= 720 {
            "720p".to_string()
        } else {
            format!("{}x{}", self.width, self.height)
        }
    }

    pub fn fps_label(&self) -> String {
        if (self.fps - 23.976).abs() < 0.05 {
            "23.98 fps".to_string()
        } else if (self.fps - 24.0).abs() < 0.05 {
            "24 fps".to_string()
        } else if (self.fps - 25.0).abs() < 0.05 {
            "25 fps".to_string()
        } else if (self.fps - 29.97).abs() < 0.05 {
            "29.97 fps".to_string()
        } else if (self.fps - 30.0).abs() < 0.05 {
            "30 fps".to_string()
        } else if (self.fps - 50.0).abs() < 0.05 {
            "50 fps".to_string()
        } else if (self.fps - 59.94).abs() < 0.05 {
            "59.94 fps".to_string()
        } else if (self.fps - 60.0).abs() < 0.05 {
            "60 fps".to_string()
        } else if (self.fps - 119.88).abs() < 0.1 || (self.fps - 120.0).abs() < 0.1 {
            "120 fps".to_string()
        } else if self.fps > 0.0 {
            format!("{:.2} fps", self.fps)
        } else {
            "-".to_string()
        }
    }

    pub fn codec_display_name(&self) -> String {
        let c = self.codec.to_lowercase();
        let p = self.profile.to_lowercase();
        if c.contains("prores") || p.contains("prores") {
            if p.contains("hq") {
                "ProRes HQ".to_string()
            } else if p.contains("4444 xq") {
                "ProRes 4444 XQ".to_string()
            } else if p.contains("4444") {
                "ProRes 4444".to_string()
            } else if p.contains("proxy") {
                "ProRes Proxy".to_string()
            } else if p.contains("lt") {
                "ProRes LT".to_string()
            } else {
                "Apple ProRes".to_string()
            }
        } else if c == "hevc" || c == "h265" {
            "H.265 / HEVC".to_string()
        } else if c == "h264" || c == "avc" {
            "H.264 / AVC".to_string()
        } else if c.contains("braw") || self.container.to_lowercase() == "braw" {
            "Blackmagic RAW".to_string()
        } else if c.contains("dnxhd") || c.contains("dnxhr") {
            "Avid DNxHD/HR".to_string()
        } else if c == "av1" {
            "AV1".to_string()
        } else if c == "vp9" {
            "VP9".to_string()
        } else if !c.is_empty() {
            c.to_uppercase()
        } else {
            self.container.to_uppercase()
        }
    }

    pub fn file_size_formatted(&self) -> String {
        let sz = self.file_size as f64;
        if sz >= 1024.0 * 1024.0 * 1024.0 {
            format!("{:.2} GB", sz / (1024.0 * 1024.0 * 1024.0))
        } else if sz >= 1024.0 * 1024.0 {
            format!("{:.1} MB", sz / (1024.0 * 1024.0))
        } else if sz >= 1024.0 {
            format!("{:.0} KB", sz / 1024.0)
        } else {
            format!("{} B", sz)
        }
    }

    pub fn bitrate_formatted(&self) -> String {
        if self.bitrate == 0 {
            return "-".to_string();
        }
        let mbps = (self.bitrate as f64) / 1_000_000.0;
        if mbps >= 1.0 {
            format!("{:.1} Mbps", mbps)
        } else {
            format!("{:.0} Kbps", (self.bitrate as f64) / 1000.0)
        }
    }

    pub fn is_braw(&self) -> bool {
        self.container.eq_ignore_ascii_case("braw")
            || self.codec.to_lowercase().contains("braw")
            || self.file_name.to_lowercase().ends_with(".braw")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub clip_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CatalogStats {
    pub total_clips: usize,
    pub total_size_bytes: u64,
    pub total_duration_seconds: f64,
    pub picks_clips: usize,
    pub rejected_clips: usize,
    pub unflagged_clips: usize,
    pub favorites_clips: usize,
    pub ten_bit_clips: usize,
    pub four_k_clips: usize,
    pub hfr_clips: usize,
    pub braw_clips: usize,
    pub prores_clips: usize,
    pub h265_clips: usize,
    pub h264_clips: usize,
    pub log_clips: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortField {
    #[default]
    DateRecorded,
    ImportedAt,
    FileName,
    Duration,
    FileSize,
    Rating,
    Codec,
    BitDepth,
    CameraModel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    Ascending,
    #[default]
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SmartCollection {
    #[default]
    All,
    Picks,
    Rejected,
    Unflagged,
    Favorites,   // 4+ or 5 stars
    LogFootage,  // Log profile detected
    TenBitPlus,  // >= 10-bit
    FourKPlus,   // >= 3840 width
    HighFrameRate, // >= 50 fps
    ProRes,
    BRAW,
    H265,
    H264,
    Container(String),
    Codec(String),
    Camera(String),
    Tag(String),
    Folder(String),
}

#[derive(Debug, Clone, Default)]
pub struct FilterCriteria {
    pub search_query: String,
    pub collection: SmartCollection,
    pub min_rating: u8,
    pub flag_filter: Option<PickFlag>,
    pub color_filter: Option<String>,
    pub sort_field: SortField,
    pub sort_dir: SortDirection,
}
