use std::collections::HashSet;
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use crossbeam_channel::Sender;
use egui::Context;

use crate::db::{file_exists, insert_video, store_thumb, Database};
use crate::importer::Importer;
use crate::media::probe::probe_video_file;
use crate::media::thumbnailer::extract_thumbnail;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestPhase {
    Scanning,
    Copying,
    Indexing,
}

#[derive(Debug, Clone)]
pub struct IngestProgress {
    pub phase: IngestPhase,
    pub current_file: String,
    pub current_file_index: usize,
    pub total_files: usize,
    pub current_file_bytes: u64,
    pub current_file_total_bytes: u64,
    pub total_bytes_copied: u64,
    pub total_bytes: u64,
    pub speed_mbps: f64,
    pub eta_seconds: Option<u64>,
    pub copied: usize,
    pub duplicates: usize,
    pub failed: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IngestSummary {
    pub total_files: usize,
    pub copied: usize,
    pub duplicates: usize,
    pub failed: usize,
    pub total_bytes: u64,
    pub errors: Vec<String>,
}

pub enum IngestMessage {
    Progress(IngestProgress),
    Complete(IngestSummary),
    Cancelled(IngestSummary),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct IngestItem {
    pub source_path: PathBuf,
    pub dest_path: PathBuf,
    pub size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct IngestConfig {
    pub source_paths: Vec<PathBuf>,
    pub destination_dir: PathBuf,
    pub subfolder: Option<String>,
    pub preserve_structure: bool,
}

pub struct Ingester;

impl Ingester {
    /// Prepare the list of items to ingest, computing destination paths and file sizes
    pub fn prepare_items(config: &IngestConfig) -> Vec<IngestItem> {
        let mut items = Vec::new();
        let mut target_base = config.destination_dir.clone();
        if let Some(ref sub) = config.subfolder {
            let sub_trimmed = sub.trim();
            if !sub_trimmed.is_empty() {
                target_base.push(sub_trimmed);
            }
        }

        let mut seen_destinations = HashSet::new();

        for source in &config.source_paths {
            if source.is_file() {
                let file_name = match source.file_name() {
                    Some(n) => n,
                    None => continue,
                };
                let dest_path = target_base.join(file_name);
                let size_bytes = std::fs::metadata(source).map(|m| m.len()).unwrap_or(0);
                if seen_destinations.insert(dest_path.clone()) {
                    items.push(IngestItem {
                        source_path: source.clone(),
                        dest_path,
                        size_bytes,
                    });
                }
            } else if source.is_dir() {
                let video_files = Importer::collect_video_files(vec![source.clone()]);
                for vf in video_files {
                    let dest_path = if config.preserve_structure {
                        if let Ok(rel) = vf.strip_prefix(source) {
                            target_base.join(rel)
                        } else {
                            target_base.join(vf.file_name().unwrap_or_default())
                        }
                    } else {
                        target_base.join(vf.file_name().unwrap_or_default())
                    };

                    let size_bytes = std::fs::metadata(&vf).map(|m| m.len()).unwrap_or(0);
                    if seen_destinations.insert(dest_path.clone()) {
                        items.push(IngestItem {
                            source_path: vf,
                            dest_path,
                            size_bytes,
                        });
                    }
                }
            }
        }

        items
    }

    /// Calculate total bytes for a list of items
    pub fn calculate_total_bytes(items: &[IngestItem]) -> u64 {
        items.iter().map(|item| item.size_bytes).sum()
    }

    /// Start background ingestion process (Copy -> Verify -> Index)
    pub fn start(
        config: IngestConfig,
        db_path: PathBuf,
        tx: Sender<IngestMessage>,
        cancel_flag: Arc<AtomicBool>,
        ctx: Context,
    ) {
        std::thread::spawn(move || {
            let mut errors = Vec::new();

            let _ = tx.send(IngestMessage::Progress(IngestProgress {
                phase: IngestPhase::Scanning,
                current_file: "Preparing files...".to_string(),
                current_file_index: 0,
                total_files: 0,
                current_file_bytes: 0,
                current_file_total_bytes: 0,
                total_bytes_copied: 0,
                total_bytes: 0,
                speed_mbps: 0.0,
                eta_seconds: None,
                copied: 0,
                duplicates: 0,
                failed: 0,
                last_error: None,
            }));
            ctx.request_repaint();

            let items = Self::prepare_items(&config);
            let total_files = items.len();
            let total_bytes = Self::calculate_total_bytes(&items);

            if total_files == 0 {
                let err_msg = "No video clips found in selected source path(s).".to_string();
                errors.push(err_msg.clone());
                let _ = tx.send(IngestMessage::Complete(IngestSummary {
                    total_files: 0,
                    copied: 0,
                    duplicates: 0,
                    failed: 0,
                    total_bytes: 0,
                    errors,
                }));
                ctx.request_repaint();
                return;
            }

            let db = match Database::open(&db_path) {
                Ok(d) => d,
                Err(e) => {
                    let err_msg = format!("Failed to open database: {}", e);
                    let _ = tx.send(IngestMessage::Error(err_msg.clone()));
                    let _ = tx.send(IngestMessage::Complete(IngestSummary {
                        total_files,
                        copied: 0,
                        duplicates: 0,
                        failed: total_files,
                        total_bytes: 0,
                        errors: vec![err_msg],
                    }));
                    ctx.request_repaint();
                    return;
                }
            };

            let conn = match db.conn() {
                Ok(c) => {
                    let _ = c.busy_timeout(std::time::Duration::from_secs(5));
                    c
                }
                Err(e) => {
                    let err_msg = format!("Failed to connect to database: {}", e);
                    let _ = tx.send(IngestMessage::Error(err_msg.clone()));
                    let _ = tx.send(IngestMessage::Complete(IngestSummary {
                        total_files,
                        copied: 0,
                        duplicates: 0,
                        failed: total_files,
                        total_bytes: 0,
                        errors: vec![err_msg],
                    }));
                    ctx.request_repaint();
                    return;
                }
            };

            let mut copied = 0;
            let mut duplicates = 0;
            let mut failed = 0;
            let mut total_bytes_transferred: u64 = 0;

            let start_time = Instant::now();
            let mut last_progress_time = Instant::now();

            for (idx, item) in items.into_iter().enumerate() {
                if cancel_flag.load(Ordering::Relaxed) {
                    let _ = tx.send(IngestMessage::Cancelled(IngestSummary {
                        total_files,
                        copied,
                        duplicates,
                        failed,
                        total_bytes: total_bytes_transferred,
                        errors,
                    }));
                    ctx.request_repaint();
                    return;
                }

                let file_name = item
                    .source_path
                    .file_name()
                    .map_or("".to_string(), |s| s.to_string_lossy().to_string());

                // Check if destination path exists and is already registered in DB
                let dest_path_str = item.dest_path.to_string_lossy().to_string();
                if file_exists(&conn, &dest_path_str) {
                    duplicates += 1;
                    total_bytes_transferred += item.size_bytes;
                    continue;
                }

                // Ensure destination directory exists
                if let Some(parent) = item.dest_path.parent() {
                    if let Err(e) = create_dir_all(parent) {
                        let err_msg = format!("{}: Failed to create destination folder: {}", file_name, e);
                        eprintln!("[cineExplorer ingest] {}", err_msg);
                        if errors.len() < 100 { errors.push(err_msg.clone()); }
                        let _ = tx.send(IngestMessage::Error(err_msg));
                        failed += 1;
                        continue;
                    }
                }

                // Check if destination file already exists on disk with identical size
                let already_copied = if item.dest_path.exists() {
                    match std::fs::metadata(&item.dest_path) {
                        Ok(meta) => meta.len() == item.size_bytes,
                        Err(_) => false,
                    }
                } else {
                    false
                };

                let mut current_file_bytes: u64 = 0;

                if !already_copied {
                    // Open source file
                    let mut src_file = match File::open(&item.source_path) {
                        Ok(f) => f,
                        Err(e) => {
                            let msg = format!("{}: Failed to open source file: {}", file_name, e);
                            eprintln!("[cineExplorer ingest] {}", msg);
                            if errors.len() < 100 { errors.push(msg); }
                            failed += 1;
                            continue;
                        }
                    };

                    // Create destination file
                    let mut dst_file = match File::create(&item.dest_path) {
                        Ok(f) => f,
                        Err(e) => {
                            let msg = format!("{}: Failed to create destination file: {}", file_name, e);
                            eprintln!("[cineExplorer ingest] {}", msg);
                            if errors.len() < 100 { errors.push(msg); }
                            failed += 1;
                            continue;
                        }
                    };

                    let mut buffer = vec![0u8; 1024 * 1024]; // 1MB streaming buffer
                    let mut copy_failed = false;
                    let mut copy_err_desc = String::new();

                    loop {
                        if cancel_flag.load(Ordering::Relaxed) {
                            drop(dst_file);
                            let _ = std::fs::remove_file(&item.dest_path);
                            let _ = tx.send(IngestMessage::Cancelled(IngestSummary {
                                total_files,
                                copied,
                                duplicates,
                                failed,
                                total_bytes: total_bytes_transferred,
                                errors,
                            }));
                            ctx.request_repaint();
                            return;
                        }

                        match src_file.read(&mut buffer) {
                            Ok(0) => break, // EOF
                            Ok(n) => {
                                if let Err(e) = dst_file.write_all(&buffer[..n]) {
                                    copy_failed = true;
                                    copy_err_desc = format!("Write error: {}", e);
                                    break;
                                }
                                current_file_bytes += n as u64;
                                total_bytes_transferred += n as u64;

                                if last_progress_time.elapsed().as_millis() >= 120 {
                                    last_progress_time = Instant::now();
                                    let elapsed = start_time.elapsed().as_secs_f64();
                                    let speed_mbps = if elapsed > 0.05 {
                                        (total_bytes_transferred as f64) / (elapsed * 1024.0 * 1024.0)
                                    } else {
                                        0.0
                                    };

                                    let remaining_bytes = total_bytes.saturating_sub(total_bytes_transferred);
                                    let eta_seconds = if speed_mbps > 0.1 {
                                        Some(((remaining_bytes as f64) / (speed_mbps * 1024.0 * 1024.0)) as u64)
                                    } else {
                                        None
                                    };

                                    let _ = tx.send(IngestMessage::Progress(IngestProgress {
                                        phase: IngestPhase::Copying,
                                        current_file: file_name.clone(),
                                        current_file_index: idx + 1,
                                        total_files,
                                        current_file_bytes,
                                        current_file_total_bytes: item.size_bytes,
                                        total_bytes_copied: total_bytes_transferred,
                                        total_bytes,
                                        speed_mbps,
                                        eta_seconds,
                                        copied,
                                        duplicates,
                                        failed,
                                        last_error: errors.last().cloned(),
                                    }));
                                    ctx.request_repaint();
                                }
                            }
                            Err(e) => {
                                copy_failed = true;
                                copy_err_desc = format!("Read error: {}", e);
                                break;
                            }
                        }
                    }

                    if copy_failed {
                        drop(dst_file);
                        let _ = std::fs::remove_file(&item.dest_path);
                        let msg = format!("{}: Copy failed ({})", file_name, copy_err_desc);
                        eprintln!("[cineExplorer ingest] {}", msg);
                        if errors.len() < 100 { errors.push(msg); }
                        failed += 1;
                        continue;
                    }

                    let _ = dst_file.flush();
                } else {
                    total_bytes_transferred += item.size_bytes;
                }

                // Indexing phase: Probe destination video file and register into Catalog DB
                let _ = tx.send(IngestMessage::Progress(IngestProgress {
                    phase: IngestPhase::Indexing,
                    current_file: file_name.clone(),
                    current_file_index: idx + 1,
                    total_files,
                    current_file_bytes: item.size_bytes,
                    current_file_total_bytes: item.size_bytes,
                    total_bytes_copied: total_bytes_transferred,
                    total_bytes,
                    speed_mbps: 0.0,
                    eta_seconds: None,
                    copied,
                    duplicates,
                    failed,
                    last_error: errors.last().cloned(),
                }));
                ctx.request_repaint();

                // Check again in case it was added
                if file_exists(&conn, &dest_path_str) {
                    duplicates += 1;
                    continue;
                }

                // Probe metadata from newly copied destination file
                match probe_video_file(&item.dest_path) {
                    Ok(mut record) => {
                        match insert_video(&conn, &record) {
                            Ok(new_id) => {
                                record.id = new_id;
                                copied += 1;

                                // Generate thumbnail and save into DB
                                if let Ok(thumb_bytes) = extract_thumbnail(&item.dest_path, record.duration_seconds) {
                                    let _ = store_thumb(&conn, new_id, &thumb_bytes);
                                }
                            }
                            Err(e) => {
                                let msg = format!("{}: Database error: {}", file_name, e);
                                eprintln!("[cineExplorer ingest] {}", msg);
                                if errors.len() < 100 { errors.push(msg); }
                                failed += 1;
                            }
                        }
                    }
                    Err(e) => {
                        let msg = format!("{}: {}", file_name, e);
                        eprintln!("[cineExplorer ingest] {}", msg);
                        if errors.len() < 100 { errors.push(msg); }
                        failed += 1;
                    }
                }
            }

            let _ = tx.send(IngestMessage::Complete(IngestSummary {
                total_files,
                copied,
                duplicates,
                failed,
                total_bytes: total_bytes_transferred,
                errors,
            }));
            ctx.request_repaint();
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, File};
    use std::io::Write;

    #[test]
    fn test_prepare_items_single_file() {
        let temp_dir = std::env::temp_dir().join("cine_test_ingest_single");
        let _ = create_dir_all(&temp_dir);
        let src_file = temp_dir.join("test_clip.mp4");
        {
            let mut f = File::create(&src_file).unwrap();
            f.write_all(b"dummy video content 123456").unwrap();
        }

        let dest_dir = temp_dir.join("dest");
        let config = IngestConfig {
            source_paths: vec![src_file.clone()],
            destination_dir: dest_dir.clone(),
            subfolder: Some("Card_01".to_string()),
            preserve_structure: false,
        };

        let items = Ingester::prepare_items(&config);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source_path, src_file);
        assert_eq!(items[0].dest_path, dest_dir.join("Card_01").join("test_clip.mp4"));
        assert_eq!(items[0].size_bytes, 26);

        let total_bytes = Ingester::calculate_total_bytes(&items);
        assert_eq!(total_bytes, 26);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_prepare_items_directory_structure() {
        let temp_dir = std::env::temp_dir().join("cine_test_ingest_dir");
        let card_root = temp_dir.join("SD_CARD");
        let dcim_folder = card_root.join("DCIM").join("100CANON");
        create_dir_all(&dcim_folder).unwrap();

        let clip1 = dcim_folder.join("MVI_0001.MP4");
        let clip2 = dcim_folder.join("MVI_0002.MOV");
        let ignored_txt = dcim_folder.join("INFO.TXT");
        File::create(&clip1).unwrap().write_all(b"video1").unwrap();
        File::create(&clip2).unwrap().write_all(b"video22").unwrap();
        File::create(&ignored_txt).unwrap().write_all(b"ignore me").unwrap();

        let dest_dir = temp_dir.join("HDD_STORAGE");

        // Case 1: preserve_structure = true
        let config_preserve = IngestConfig {
            source_paths: vec![card_root.clone()],
            destination_dir: dest_dir.clone(),
            subfolder: None,
            preserve_structure: true,
        };
        let items_preserve = Ingester::prepare_items(&config_preserve);
        assert_eq!(items_preserve.len(), 2);
        for item in &items_preserve {
            assert!(item.dest_path.to_string_lossy().contains("DCIM"));
        }

        // Case 2: preserve_structure = false (flat copy into subfolder)
        let config_flat = IngestConfig {
            source_paths: vec![card_root],
            destination_dir: dest_dir.clone(),
            subfolder: Some("Day1".to_string()),
            preserve_structure: false,
        };
        let items_flat = Ingester::prepare_items(&config_flat);
        assert_eq!(items_flat.len(), 2);
        for item in &items_flat {
            assert_eq!(item.dest_path.parent().unwrap(), &dest_dir.join("Day1"));
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
