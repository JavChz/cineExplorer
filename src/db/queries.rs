use rusqlite::{params, Connection, Result};
use std::collections::HashSet;
use std::path::Path;

use super::models::{CatalogStats, FilterCriteria, PickFlag, SmartCollection, SortDirection, SortField, Tag, VideoRecord};

pub fn file_exists(conn: &Connection, file_path: &str) -> bool {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM videos WHERE file_path = ?1",
            params![file_path],
            |row| row.get(0),
        )
        .unwrap_or(0);
    count > 0
}

pub fn insert_video(conn: &Connection, v: &VideoRecord) -> Result<i64> {
    conn.execute(
        r#"
        INSERT INTO videos (
            file_path, file_name, file_size, container, codec, codec_long_name,
            profile, width, height, fps, frame_count, duration_seconds,
            bit_depth, pixel_format, color_space, color_primaries, color_transfer,
            bitrate, has_audio, audio_codec, audio_channels, audio_sample_rate,
            camera_make, camera_model, lens_model, focal_length, aperture,
            iso, shutter_speed, white_balance, date_recorded, imported_at,
            rating, flag, notes, color_label, scene, shot, take, reel, location, camera_unit, log_profile
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6,
            ?7, ?8, ?9, ?10, ?11, ?12,
            ?13, ?14, ?15, ?16, ?17,
            ?18, ?19, ?20, ?21, ?22,
            ?23, ?24, ?25, ?26, ?27,
            ?28, ?29, ?30, ?31, ?32,
            ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43
        )
        "#,
        params![
            v.file_path,
            v.file_name,
            v.file_size,
            v.container,
            v.codec,
            v.codec_long_name,
            v.profile,
            v.width,
            v.height,
            v.fps,
            v.frame_count,
            v.duration_seconds,
            v.bit_depth,
            v.pixel_format,
            v.color_space,
            v.color_primaries,
            v.color_transfer,
            v.bitrate,
            if v.has_audio { 1 } else { 0 },
            v.audio_codec,
            v.audio_channels,
            v.audio_sample_rate,
            v.camera_make,
            v.camera_model,
            v.lens_model,
            v.focal_length,
            v.aperture,
            v.iso,
            v.shutter_speed,
            v.white_balance,
            v.date_recorded,
            v.imported_at,
            v.rating,
            v.flag.as_i64(),
            v.notes,
            v.color_label,
            v.scene,
            v.shot,
            v.take,
            v.reel,
            v.location,
            v.camera_unit,
            v.log_profile,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

pub fn load_thumb(conn: &Connection, video_id: i64) -> Option<Vec<u8>> {
    conn.query_row(
        "SELECT jpeg FROM thumbs WHERE video_id = ?1",
        params![video_id],
        |row| row.get(0),
    )
    .ok()
}

pub fn store_thumb(conn: &Connection, video_id: i64, jpeg_bytes: &[u8]) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO thumbs (video_id, jpeg) VALUES (?1, ?2)",
        params![video_id, jpeg_bytes],
    )?;
    Ok(())
}

pub fn update_rating(conn: &Connection, video_id: i64, rating: u8) -> Result<()> {
    conn.execute(
        "UPDATE videos SET rating = ?1 WHERE id = ?2",
        params![rating, video_id],
    )?;
    Ok(())
}

pub fn update_flag(conn: &Connection, video_id: i64, flag: PickFlag) -> Result<()> {
    conn.execute(
        "UPDATE videos SET flag = ?1 WHERE id = ?2",
        params![flag.as_i64(), video_id],
    )?;
    Ok(())
}

pub fn update_color_label(conn: &Connection, video_id: i64, color: &str) -> Result<()> {
    conn.execute(
        "UPDATE videos SET color_label = ?1 WHERE id = ?2",
        params![color, video_id],
    )?;
    Ok(())
}

pub fn update_notes(conn: &Connection, video_id: i64, notes: &str) -> Result<()> {
    conn.execute(
        "UPDATE videos SET notes = ?1 WHERE id = ?2",
        params![notes, video_id],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update_slate(
    conn: &Connection,
    video_id: i64,
    scene: &str,
    shot: &str,
    take: &str,
    reel: &str,
    location: &str,
    camera_unit: &str,
) -> Result<()> {
    conn.execute(
        r#"
        UPDATE videos SET
            scene = ?1,
            shot = ?2,
            take = ?3,
            reel = ?4,
            location = ?5,
            camera_unit = ?6
        WHERE id = ?7
        "#,
        params![scene, shot, take, reel, location, camera_unit, video_id],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update_video_metadata(
    conn: &Connection,
    video_id: i64,
    camera_make: &str,
    camera_model: &str,
    lens_model: &str,
    focal_length: &str,
    aperture: &str,
    iso: u32,
    shutter_speed: &str,
    white_balance: &str,
    log_profile: &str,
    date_recorded: Option<&str>,
) -> Result<()> {
    conn.execute(
        r#"
        UPDATE videos SET
            camera_make = ?1,
            camera_model = ?2,
            lens_model = ?3,
            focal_length = ?4,
            aperture = ?5,
            iso = ?6,
            shutter_speed = ?7,
            white_balance = ?8,
            log_profile = ?9,
            date_recorded = COALESCE(?10, date_recorded)
        WHERE id = ?11
        "#,
        params![
            camera_make,
            camera_model,
            lens_model,
            focal_length,
            aperture,
            iso,
            shutter_speed,
            white_balance,
            log_profile,
            date_recorded,
            video_id,
        ],
    )?;
    Ok(())
}

pub fn delete_video(conn: &Connection, video_id: i64) -> Result<()> {
    conn.execute("DELETE FROM video_tags WHERE video_id = ?1", params![video_id])?;
    conn.execute("DELETE FROM thumbs WHERE video_id = ?1", params![video_id])?;
    conn.execute("DELETE FROM videos WHERE id = ?1", params![video_id])?;
    Ok(())
}

pub fn list_tags(conn: &Connection) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT t.id, t.name, t.color, COUNT(vt.video_id) as clip_count
        FROM tags t
        LEFT JOIN video_tags vt ON t.id = vt.tag_id
        GROUP BY t.id
        ORDER BY t.name ASC
        "#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            clip_count: row.get::<_, i64>(3)? as usize,
        })
    })?;

    let mut tags = Vec::new();
    for t in rows.flatten() {
        tags.push(t);
    }
    Ok(tags)
}

pub fn create_tag(conn: &Connection, name: &str, color: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO tags (name, color) VALUES (?1, ?2)",
        params![name, color],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn add_tag_to_video(conn: &Connection, video_id: i64, tag_name: &str) -> Result<()> {
    let tag_id: i64 = match conn.query_row("SELECT id FROM tags WHERE name = ?1", params![tag_name], |r| r.get(0)) {
        Ok(id) => id,
        Err(_) => {
            conn.execute("INSERT INTO tags (name) VALUES (?1)", params![tag_name])?;
            conn.last_insert_rowid()
        }
    };
    conn.execute(
        "INSERT OR IGNORE INTO video_tags (video_id, tag_id) VALUES (?1, ?2)",
        params![video_id, tag_id],
    )?;
    Ok(())
}

pub fn remove_tag_from_video(conn: &Connection, video_id: i64, tag_name: &str) -> Result<()> {
    conn.execute(
        r#"
        DELETE FROM video_tags
        WHERE video_id = ?1 AND tag_id = (SELECT id FROM tags WHERE name = ?2)
        "#,
        params![video_id, tag_name],
    )?;
    Ok(())
}

pub fn get_tags_for_video(conn: &Connection, video_id: i64) -> Vec<String> {
    let mut stmt = match conn.prepare(
        r#"
        SELECT t.name FROM tags t
        INNER JOIN video_tags vt ON t.id = vt.tag_id
        WHERE vt.video_id = ?1
        ORDER BY t.name ASC
        "#,
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = stmt.query_map(params![video_id], |row| row.get::<_, String>(0));
    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn query_videos(conn: &Connection, criteria: &FilterCriteria) -> Result<Vec<VideoRecord>> {
    let mut sql = String::from(
        r#"
        SELECT
            v.id, v.file_path, v.file_name, v.file_size, v.container, v.codec, v.codec_long_name,
            v.profile, v.width, v.height, v.fps, v.frame_count, v.duration_seconds,
            v.bit_depth, v.pixel_format, v.color_space, v.color_primaries, v.color_transfer,
            v.bitrate, v.has_audio, v.audio_codec, v.audio_channels, v.audio_sample_rate,
            v.camera_make, v.camera_model, v.lens_model, v.focal_length, v.aperture,
            v.iso, v.shutter_speed, v.white_balance, v.date_recorded, v.imported_at,
            v.rating, v.flag, v.notes, v.color_label, v.scene, v.shot, v.take, v.reel, v.location, v.camera_unit, v.log_profile
        FROM videos v
        "#,
    );

    let mut conditions = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    // 1. Search Query
    if !criteria.search_query.trim().is_empty() {
        let pattern = format!("%{}%", criteria.search_query.trim());
        params_vec.push(Box::new(pattern.clone()));
        let p_idx = params_vec.len();
        conditions.push(format!(
            "(v.file_name LIKE ?{0} OR v.file_path LIKE ?{0} OR v.camera_model LIKE ?{0} OR v.codec LIKE ?{0} OR v.notes LIKE ?{0} OR v.scene LIKE ?{0} OR v.shot LIKE ?{0} OR v.take LIKE ?{0} OR v.reel LIKE ?{0})"
        , p_idx));
    }

    // 2. Minimum Rating
    if criteria.min_rating > 0 {
        params_vec.push(Box::new(criteria.min_rating));
        conditions.push(format!("v.rating >= ?{}", params_vec.len()));
    }

    // 3. Pick Flag
    if let Some(flag) = criteria.flag_filter {
        params_vec.push(Box::new(flag.as_i64()));
        conditions.push(format!("v.flag = ?{}", params_vec.len()));
    }

    // 4. Color Filter
    if let Some(ref color) = criteria.color_filter {
        if !color.is_empty() {
            params_vec.push(Box::new(color.clone()));
            conditions.push(format!("v.color_label = ?{}", params_vec.len()));
        }
    }

    // 5. Smart Collection
    match &criteria.collection {
        SmartCollection::All => {}
        SmartCollection::Picks => {
            conditions.push("v.flag = 1".to_string());
        }
        SmartCollection::Rejected => {
            conditions.push("v.flag = 2".to_string());
        }
        SmartCollection::Unflagged => {
            conditions.push("v.flag = 0".to_string());
        }
        SmartCollection::Favorites => {
            conditions.push("v.rating >= 4".to_string());
        }
        SmartCollection::LogFootage => {
            conditions.push("(v.log_profile != '' AND LOWER(v.log_profile) != 'none')".to_string());
        }
        SmartCollection::TenBitPlus => {
            conditions.push("v.bit_depth >= 10".to_string());
        }
        SmartCollection::FourKPlus => {
            conditions.push("(v.width >= 3840 OR v.height >= 2160)".to_string());
        }
        SmartCollection::HighFrameRate => {
            conditions.push("v.fps >= 49.0".to_string());
        }
        SmartCollection::ProRes => {
            conditions.push("(LOWER(v.codec) LIKE '%prores%' OR LOWER(v.profile) LIKE '%prores%')".to_string());
        }
        SmartCollection::BRAW => {
            conditions.push("(LOWER(v.codec) LIKE '%braw%' OR LOWER(v.container) = 'braw')".to_string());
        }
        SmartCollection::H265 => {
            conditions.push("(LOWER(v.codec) IN ('hevc', 'h265'))".to_string());
        }
        SmartCollection::H264 => {
            conditions.push("(LOWER(v.codec) IN ('h264', 'avc'))".to_string());
        }
        SmartCollection::Container(c) => {
            params_vec.push(Box::new(c.clone()));
            conditions.push(format!("LOWER(v.container) = LOWER(?{})", params_vec.len()));
        }
        SmartCollection::Codec(c) => {
            params_vec.push(Box::new(c.clone()));
            conditions.push(format!("LOWER(v.codec) = LOWER(?{})", params_vec.len()));
        }
        SmartCollection::Camera(cam) => {
            params_vec.push(Box::new(cam.clone()));
            conditions.push(format!("v.camera_model = ?{}", params_vec.len()));
        }
        SmartCollection::Tag(tag) => {
            params_vec.push(Box::new(tag.clone()));
            conditions.push(format!(
                "v.id IN (SELECT vt.video_id FROM video_tags vt JOIN tags t ON vt.tag_id = t.id WHERE t.name = ?{})"
            , params_vec.len()));
        }
        SmartCollection::Folder(folder) => {
            let pattern = format!("{}%", folder);
            params_vec.push(Box::new(pattern));
            conditions.push(format!("v.file_path LIKE ?{}", params_vec.len()));
        }
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }

    // Sort order
    let sort_col = match criteria.sort_field {
        SortField::DateRecorded => "COALESCE(v.date_recorded, v.imported_at)",
        SortField::ImportedAt => "v.imported_at",
        SortField::FileName => "v.file_name",
        SortField::Duration => "v.duration_seconds",
        SortField::FileSize => "v.file_size",
        SortField::Rating => "v.rating",
        SortField::Codec => "v.codec",
        SortField::BitDepth => "v.bit_depth",
        SortField::CameraModel => "v.camera_model",
    };

    let sort_dir_str = match criteria.sort_dir {
        SortDirection::Ascending => "ASC",
        SortDirection::Descending => "DESC",
    };

    sql.push_str(&format!(" ORDER BY {} {}", sort_col, sort_dir_str));

    let mut stmt = conn.prepare(&sql)?;
    let rusqlite_params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let rows = stmt.query_map(rusqlite_params.as_slice(), |row| {
        Ok(VideoRecord {
            id: row.get(0)?,
            file_path: row.get(1)?,
            file_name: row.get(2)?,
            file_size: row.get(3)?,
            container: row.get(4)?,
            codec: row.get(5)?,
            codec_long_name: row.get(6)?,
            profile: row.get(7)?,
            width: row.get(8)?,
            height: row.get(9)?,
            fps: row.get(10)?,
            frame_count: row.get(11)?,
            duration_seconds: row.get(12)?,
            bit_depth: row.get(13)?,
            pixel_format: row.get(14)?,
            color_space: row.get(15)?,
            color_primaries: row.get(16)?,
            color_transfer: row.get(17)?,
            bitrate: row.get(18)?,
            has_audio: row.get::<_, i64>(19)? != 0,
            audio_codec: row.get(20)?,
            audio_channels: row.get(21)?,
            audio_sample_rate: row.get(22)?,
            camera_make: row.get(23)?,
            camera_model: row.get(24)?,
            lens_model: row.get(25)?,
            focal_length: row.get(26)?,
            aperture: row.get(27)?,
            iso: row.get(28)?,
            shutter_speed: row.get(29)?,
            white_balance: row.get(30)?,
            date_recorded: row.get(31)?,
            imported_at: row.get(32)?,
            rating: row.get(33)?,
            flag: PickFlag::from(row.get::<_, i64>(34)?),
            notes: row.get(35)?,
            color_label: row.get(36)?,
            scene: row.get(37)?,
            shot: row.get(38)?,
            take: row.get(39)?,
            reel: row.get(40)?,
            location: row.get(41)?,
            camera_unit: row.get(42)?,
            tags: Vec::new(),
        })
    })?;

    let mut videos = Vec::new();
    for mut v in rows.flatten() {
        v.tags = get_tags_for_video(conn, v.id);
        videos.push(v);
    }

    Ok(videos)
}

pub fn get_catalog_stats(conn: &Connection) -> CatalogStats {
    let mut stats = CatalogStats::default();

    let _ = conn.query_row(
        r#"
        SELECT
            COUNT(*),
            COALESCE(SUM(file_size), 0),
            COALESCE(SUM(duration_seconds), 0.0),
            COALESCE(SUM(CASE WHEN flag = 1 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN flag = 2 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN flag = 0 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN rating >= 4 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN bit_depth >= 10 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN width >= 3840 OR height >= 2160 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN fps >= 49.0 THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN LOWER(codec) LIKE '%braw%' OR LOWER(container) = 'braw' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN LOWER(codec) LIKE '%prores%' OR LOWER(profile) LIKE '%prores%' THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN LOWER(codec) IN ('hevc', 'h265') THEN 1 ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN LOWER(codec) IN ('h264', 'avc') THEN 1 ELSE 0 END), 0)
        FROM videos
        "#,
        [],
        |row| {
            stats.total_clips = row.get::<_, i64>(0)? as usize;
            stats.total_size_bytes = row.get::<_, i64>(1)? as u64;
            stats.total_duration_seconds = row.get::<_, f64>(2)?;
            stats.picks_clips = row.get::<_, i64>(3)? as usize;
            stats.rejected_clips = row.get::<_, i64>(4)? as usize;
            stats.unflagged_clips = row.get::<_, i64>(5)? as usize;
            stats.favorites_clips = row.get::<_, i64>(6)? as usize;
            stats.ten_bit_clips = row.get::<_, i64>(7)? as usize;
            stats.four_k_clips = row.get::<_, i64>(8)? as usize;
            stats.hfr_clips = row.get::<_, i64>(9)? as usize;
            stats.braw_clips = row.get::<_, i64>(10)? as usize;
            stats.prores_clips = row.get::<_, i64>(11)? as usize;
            stats.h265_clips = row.get::<_, i64>(12)? as usize;
            stats.h264_clips = row.get::<_, i64>(13)? as usize;
            Ok(())
        },
    );

    stats
}

pub fn get_distinct_containers(conn: &Connection) -> Vec<(String, usize)> {
    let mut stmt = match conn.prepare(
        "SELECT UPPER(container), COUNT(*) FROM videos WHERE container != '' GROUP BY UPPER(container) ORDER BY COUNT(*) DESC"
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as usize)));
    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn get_distinct_codecs(conn: &Connection) -> Vec<(String, usize)> {
    let mut stmt = match conn.prepare(
        "SELECT codec, COUNT(*) FROM videos WHERE codec != '' GROUP BY codec ORDER BY COUNT(*) DESC"
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as usize)));
    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn get_distinct_cameras(conn: &Connection) -> Vec<(String, usize)> {
    let mut stmt = match conn.prepare(
        "SELECT camera_model, COUNT(*) FROM videos WHERE camera_model != '' GROUP BY camera_model ORDER BY COUNT(*) DESC"
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as usize)));
    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn get_distinct_folders(conn: &Connection) -> Vec<String> {
    let mut stmt = match conn.prepare("SELECT file_path FROM videos") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = stmt.query_map([], |row| row.get::<_, String>(0));
    let mut folder_set = HashSet::new();

    if let Ok(iter) = rows {
        for path_str in iter.flatten() {
            if let Some(parent) = Path::new(&path_str).parent() {
                let p_str = parent.to_string_lossy().to_string();
                if !p_str.is_empty() {
                    folder_set.insert(p_str);
                }
            }
        }
    }

    let mut folders: Vec<String> = folder_set.into_iter().collect();
    folders.sort();
    folders
}
