use rusqlite::{Connection, Result};

pub const SCHEMA_VERSION: i64 = 2;

/// Initialize database tables for a cineExplorer catalog
pub fn init(conn: &Connection) -> Result<bool> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS videos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path TEXT NOT NULL UNIQUE,
            file_name TEXT NOT NULL,
            file_size INTEGER NOT NULL DEFAULT 0,
            container TEXT NOT NULL DEFAULT '',
            codec TEXT NOT NULL DEFAULT '',
            codec_long_name TEXT NOT NULL DEFAULT '',
            profile TEXT NOT NULL DEFAULT '',
            width INTEGER NOT NULL DEFAULT 0,
            height INTEGER NOT NULL DEFAULT 0,
            fps REAL NOT NULL DEFAULT 0,
            frame_count INTEGER NOT NULL DEFAULT 0,
            duration_seconds REAL NOT NULL DEFAULT 0,
            bit_depth INTEGER NOT NULL DEFAULT 8,
            pixel_format TEXT NOT NULL DEFAULT '',
            color_space TEXT NOT NULL DEFAULT '',
            color_primaries TEXT NOT NULL DEFAULT '',
            color_transfer TEXT NOT NULL DEFAULT '',
            bitrate INTEGER NOT NULL DEFAULT 0,
            has_audio INTEGER NOT NULL DEFAULT 0,
            audio_codec TEXT NOT NULL DEFAULT '',
            audio_channels INTEGER NOT NULL DEFAULT 0,
            audio_sample_rate INTEGER NOT NULL DEFAULT 0,
            camera_make TEXT NOT NULL DEFAULT '',
            camera_model TEXT NOT NULL DEFAULT '',
            lens_model TEXT NOT NULL DEFAULT '',
            focal_length TEXT NOT NULL DEFAULT '',
            aperture TEXT NOT NULL DEFAULT '',
            iso INTEGER NOT NULL DEFAULT 0,
            shutter_speed TEXT NOT NULL DEFAULT '',
            white_balance TEXT NOT NULL DEFAULT '',
            date_recorded TEXT,
            imported_at TEXT NOT NULL DEFAULT '',
            rating INTEGER NOT NULL DEFAULT 0,
            flag INTEGER NOT NULL DEFAULT 0,
            notes TEXT NOT NULL DEFAULT '',
            color_label TEXT NOT NULL DEFAULT '',
            scene TEXT NOT NULL DEFAULT '',
            shot TEXT NOT NULL DEFAULT '',
            take TEXT NOT NULL DEFAULT '',
            reel TEXT NOT NULL DEFAULT '',
            location TEXT NOT NULL DEFAULT '',
            camera_unit TEXT NOT NULL DEFAULT '',
            log_profile TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL DEFAULT '#5B8DEF'
        );

        CREATE TABLE IF NOT EXISTS video_tags (
            video_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            PRIMARY KEY (video_id, tag_id),
            FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS thumbs (
            video_id INTEGER PRIMARY KEY,
            jpeg BLOB NOT NULL,
            FOREIGN KEY (video_id) REFERENCES videos(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_videos_codec ON videos(codec);
        CREATE INDEX IF NOT EXISTS idx_videos_container ON videos(container);
        CREATE INDEX IF NOT EXISTS idx_videos_bit_depth ON videos(bit_depth);
        CREATE INDEX IF NOT EXISTS idx_videos_camera ON videos(camera_model);
        CREATE INDEX IF NOT EXISTS idx_videos_rating ON videos(rating);
        CREATE INDEX IF NOT EXISTS idx_videos_flag ON videos(flag);
        CREATE INDEX IF NOT EXISTS idx_videos_date ON videos(date_recorded);
        "#,
    )?;

    let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap_or(0);
    if version < 2 {
        // Safe additive migration for log_profile
        let _ = conn.execute("ALTER TABLE videos ADD COLUMN log_profile TEXT NOT NULL DEFAULT ''", []);
        let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_videos_log ON videos(log_profile)", []);
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }

    Ok(true)
}
