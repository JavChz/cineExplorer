use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::Sender;
use egui::Context;

use crate::db::{file_exists, insert_video, store_thumb, Database};
use crate::media::is_video_file;
use crate::media::probe::probe_video_file;
use crate::media::thumbnailer::extract_thumbnail;

#[derive(Debug, Clone)]
pub struct ImportProgress {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
    pub imported: usize,
    pub duplicates: usize,
    pub failed: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ImportSummary {
    pub total: usize,
    pub imported: usize,
    pub duplicates: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

pub enum ImportMessage {
    Progress(ImportProgress),
    Complete(ImportSummary),
    Cancelled(ImportSummary),
    Error(String),
}

pub struct Importer;

impl Importer {
    /// Collect all supported video files recursively from paths
    pub fn collect_video_files(paths: Vec<PathBuf>) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for path in paths {
            if path.is_file() {
                if is_video_file(&path) {
                    files.push(path);
                }
            } else if path.is_dir() {
                for entry in walkdir::WalkDir::new(&path)
                    .follow_links(true)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let p = entry.into_path();
                    if p.is_file() && is_video_file(&p) {
                        files.push(p);
                    }
                }
            }
        }

        let mut seen = HashSet::new();
        files.retain(|p| seen.insert(p.clone()));
        files
    }

    /// Spawn background import thread
    pub fn start(
        paths: Vec<PathBuf>,
        db_path: PathBuf,
        tx: Sender<ImportMessage>,
        cancel_flag: Arc<AtomicBool>,
        ctx: Context,
    ) {
        std::thread::spawn(move || {
            let files = Self::collect_video_files(paths);
            let total = files.len();

            let mut errors = Vec::new();

            if total == 0 {
                errors.push("No supported video files (.mp4, .mov, .braw, .mxf, .mkv, .prores, etc.) found in the selected folder(s) or subfolders.".to_string());
                let _ = tx.send(ImportMessage::Complete(ImportSummary {
                    total: 0,
                    imported: 0,
                    duplicates: 0,
                    failed: 0,
                    errors,
                }));
                ctx.request_repaint();
                return;
            }

            let db = match Database::open(&db_path) {
                Ok(d) => d,
                Err(e) => {
                    let err_msg = format!("Failed to open catalog database: {}", e);
                    let _ = tx.send(ImportMessage::Error(err_msg.clone()));
                    let _ = tx.send(ImportMessage::Complete(ImportSummary {
                        total,
                        imported: 0,
                        duplicates: 0,
                        failed: total,
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
                    let err_msg = format!("Failed to create database connection: {}", e);
                    let _ = tx.send(ImportMessage::Error(err_msg.clone()));
                    let _ = tx.send(ImportMessage::Complete(ImportSummary {
                        total,
                        imported: 0,
                        duplicates: 0,
                        failed: total,
                        errors: vec![err_msg],
                    }));
                    ctx.request_repaint();
                    return;
                }
            };

            let mut imported = 0;
            let mut duplicates = 0;
            let mut failed = 0;

            for (idx, file_path) in files.into_iter().enumerate() {
                if cancel_flag.load(Ordering::Relaxed) {
                    let _ = tx.send(ImportMessage::Cancelled(ImportSummary {
                        total,
                        imported,
                        duplicates,
                        failed,
                        errors,
                    }));
                    ctx.request_repaint();
                    return;
                }

                let file_name = file_path
                    .file_name()
                    .map_or("".to_string(), |s| s.to_string_lossy().to_string());

                let _ = tx.send(ImportMessage::Progress(ImportProgress {
                    current: idx + 1,
                    total,
                    current_file: file_name.clone(),
                    imported,
                    duplicates,
                    failed,
                    last_error: errors.last().cloned(),
                }));
                ctx.request_repaint();

                let path_str = file_path.to_string_lossy().to_string();

                // Check duplicate
                if file_exists(&conn, &path_str) {
                    duplicates += 1;
                    continue;
                }

                // Probe video file with ffprobe
                match probe_video_file(&file_path) {
                    Ok(mut record) => {
                        match insert_video(&conn, &record) {
                            Ok(new_id) => {
                                record.id = new_id;
                                imported += 1;

                                // Generate thumbnail and save into DB
                                if let Ok(thumb_bytes) = extract_thumbnail(&file_path, record.duration_seconds) {
                                    let _ = store_thumb(&conn, new_id, &thumb_bytes);
                                }
                            }
                            Err(db_err) => {
                                failed += 1;
                                let msg = format!("{}: Database error: {}", file_name, db_err);
                                eprintln!("[cineExplorer import error] {}", msg);
                                if errors.len() < 100 {
                                    errors.push(msg);
                                }
                            }
                        }
                    }
                    Err(probe_err) => {
                        failed += 1;
                        let msg = format!("{}: {}", file_name, probe_err);
                        eprintln!("[cineExplorer import error] {}", msg);
                        if errors.len() < 100 {
                            errors.push(msg);
                        }
                    }
                }
            }

            let _ = tx.send(ImportMessage::Complete(ImportSummary {
                total,
                imported,
                duplicates,
                failed,
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
    fn test_collect_subfolders() {
        let temp = std::env::temp_dir().join("cine_test_subfolders");
        let sub = temp.join("sub1").join("sub2");
        create_dir_all(&sub).unwrap();
        File::create(sub.join("test.mov")).unwrap().write_all(b"123").unwrap();
        File::create(sub.join("test.MOV")).unwrap().write_all(b"123").unwrap();
        File::create(sub.join("._test.mov")).unwrap().write_all(b"123").unwrap(); // AppleDouble

        let collected = Importer::collect_video_files(vec![temp.clone()]);
        assert!(collected.iter().any(|p| p.file_name().unwrap() == "test.mov"));
        assert!(!collected.iter().any(|p| p.file_name().unwrap().to_str().unwrap().starts_with("._")));

        let _ = std::fs::remove_dir_all(&temp);
    }
}
