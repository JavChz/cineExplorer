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
}

pub enum ImportMessage {
    Progress(ImportProgress),
    Complete {
        total: usize,
        imported: usize,
        duplicates: usize,
        failed: usize,
    },
    Cancelled {
        total: usize,
        imported: usize,
        duplicates: usize,
        failed: usize,
    },
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
                for entry in walkdir::WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
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
        let files = Self::collect_video_files(paths);
        let total = files.len();

        std::thread::spawn(move || {
            let db = match Database::open(&db_path) {
                Ok(d) => d,
                Err(_) => {
                    let _ = tx.send(ImportMessage::Complete {
                        total: 0,
                        imported: 0,
                        duplicates: 0,
                        failed: 0,
                    });
                    return;
                }
            };

            let mut imported = 0;
            let mut duplicates = 0;
            let mut failed = 0;

            for (idx, file_path) in files.into_iter().enumerate() {
                if cancel_flag.load(Ordering::Relaxed) {
                    let _ = tx.send(ImportMessage::Cancelled {
                        total,
                        imported,
                        duplicates,
                        failed,
                    });
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
                }));
                ctx.request_repaint();

                let path_str = file_path.to_string_lossy().to_string();

                let conn = match db.conn() {
                    Ok(c) => c,
                    Err(_) => {
                        failed += 1;
                        continue;
                    }
                };

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
                            Err(_) => {
                                failed += 1;
                            }
                        }
                    }
                    Err(_) => {
                        failed += 1;
                    }
                }
            }

            let _ = tx.send(ImportMessage::Complete {
                total,
                imported,
                duplicates,
                failed,
            });
            ctx.request_repaint();
        });
    }
}
