use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use chrono::Local;
use crossbeam_channel::{unbounded, Receiver, Sender};
use egui::Context;

use crate::db::{file_exists, insert_video, store_thumb, Database};
use crate::media::is_video_file;
use crate::media::probe::probe_video_file;
use crate::media::thumbnailer::extract_thumbnail;

/// Legacy progress struct for compatibility
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

/// Legacy summary struct for compatibility
#[derive(Debug, Clone, Default)]
pub struct ImportSummary {
    pub total: usize,
    pub imported: usize,
    pub duplicates: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

/// Legacy message enum for backward compatibility
pub enum ImportMessage {
    Progress(ImportProgress),
    Complete(ImportSummary),
    Cancelled(ImportSummary),
    Error(String),
}

/// A queued import job consisting of one or more folders or files
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportBatchJob {
    pub id: u64,
    pub label: String,
    pub paths: Vec<PathBuf>,
}

/// Recorded issue during importing (e.g. corrupt file, probe error, DB error, empty folder)
#[derive(Debug, Clone)]
pub struct ImportProblem {
    pub batch_id: u64,
    pub batch_label: String,
    pub file_name: String,
    pub reason: String,
    pub timestamp: String,
}

/// Live status of the currently executing batch
#[derive(Debug, Clone)]
pub struct ActiveImportStatus {
    pub batch_id: u64,
    pub label: String,
    pub current: usize,
    pub total: usize,
    pub current_file: String,
    pub imported: usize,
    pub duplicates: usize,
    pub failed: usize,
}

/// Aggregated state of the import queue system for the UI
#[derive(Debug, Clone, Default)]
pub struct ImportQueueState {
    pub active_job: Option<ActiveImportStatus>,
    pub queued_jobs: Vec<ImportBatchJob>,
    pub completed_batches: usize,
    pub total_imported: usize,
    pub total_duplicates: usize,
    pub problems: Vec<ImportProblem>,
    pub has_unseen_problems: bool,
}

/// Commands sent from the UI to the background import worker
pub enum ImportWorkerCommand {
    Enqueue(ImportBatchJob),
    CancelCurrent,
    RemoveQueued(u64),
    CancelAll,
}

/// Events emitted by the background worker to the UI
#[derive(Debug, Clone)]
pub enum ImportEvent {
    BatchStarted {
        batch_id: u64,
        label: String,
        total: usize,
    },
    Progress {
        batch_id: u64,
        current: usize,
        total: usize,
        current_file: String,
        imported: usize,
        duplicates: usize,
        failed: usize,
    },
    ClipImported {
        batch_id: u64,
        file_name: String,
    },
    ProblemLogged(ImportProblem),
    BatchCompleted {
        batch_id: u64,
        imported: usize,
        duplicates: usize,
        failed: usize,
    },
    BatchCancelled {
        batch_id: u64,
    },
    QueueIdle,
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

    /// Legacy start method spawning a one-shot thread
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

                if file_exists(&conn, &path_str) {
                    duplicates += 1;
                    continue;
                }

                match probe_video_file(&file_path) {
                    Ok(mut record) => {
                        match insert_video(&conn, &record) {
                            Ok(new_id) => {
                                record.id = new_id;
                                imported += 1;

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

/// Multi-batch, non-blocking background import queue manager
pub struct ImportQueueManager {
    tx_cmd: Sender<ImportWorkerCommand>,
    rx_event: Receiver<ImportEvent>,
    cancel_current_flag: Arc<AtomicBool>,
    state: ImportQueueState,
    next_job_id: u64,
}

impl ImportQueueManager {
    pub fn new(db_path: PathBuf, ctx: Context) -> Self {
        let (tx_cmd, rx_cmd) = unbounded::<ImportWorkerCommand>();
        let (tx_event, rx_event) = unbounded::<ImportEvent>();
        let cancel_current_flag = Arc::new(AtomicBool::new(false));
        let cancel_flag_clone = cancel_current_flag.clone();

        std::thread::spawn(move || {
            let mut internal_queue: VecDeque<ImportBatchJob> = VecDeque::new();

            loop {
                // If queue is empty, block until a command arrives
                if internal_queue.is_empty() {
                    match rx_cmd.recv() {
                        Ok(cmd) => match cmd {
                            ImportWorkerCommand::Enqueue(job) => {
                                internal_queue.push_back(job);
                            }
                            ImportWorkerCommand::CancelCurrent => {}
                            ImportWorkerCommand::RemoveQueued(_) => {}
                            ImportWorkerCommand::CancelAll => {
                                internal_queue.clear();
                                let _ = tx_event.send(ImportEvent::QueueIdle);
                                ctx.request_repaint();
                            }
                        },
                        Err(_) => break, // Channel disconnected
                    }
                }

                // Drain any additional pending commands without blocking
                while let Ok(cmd) = rx_cmd.try_recv() {
                    match cmd {
                        ImportWorkerCommand::Enqueue(job) => {
                            internal_queue.push_back(job);
                        }
                        ImportWorkerCommand::CancelCurrent => {}
                        ImportWorkerCommand::RemoveQueued(id) => {
                            internal_queue.retain(|j| j.id != id);
                        }
                        ImportWorkerCommand::CancelAll => {
                            internal_queue.clear();
                        }
                    }
                }

                let current_batch = match internal_queue.pop_front() {
                    Some(b) => b,
                    None => {
                        let _ = tx_event.send(ImportEvent::QueueIdle);
                        ctx.request_repaint();
                        continue;
                    }
                };

                // Reset cancel flag for the starting batch
                cancel_flag_clone.store(false, Ordering::Relaxed);

                // Collect files for the batch
                let files = Importer::collect_video_files(current_batch.paths.clone());
                let total = files.len();

                if total == 0 {
                    let now_str = Local::now().format("%H:%M:%S").to_string();
                    let problem = ImportProblem {
                        batch_id: current_batch.id,
                        batch_label: current_batch.label.clone(),
                        file_name: "No Video Files".to_string(),
                        reason: "No supported video files discovered in folder(s)".to_string(),
                        timestamp: now_str,
                    };
                    let _ = tx_event.send(ImportEvent::ProblemLogged(problem));
                    let _ = tx_event.send(ImportEvent::BatchCompleted {
                        batch_id: current_batch.id,
                        imported: 0,
                        duplicates: 0,
                        failed: 0,
                    });
                    ctx.request_repaint();
                    continue;
                }

                let _ = tx_event.send(ImportEvent::BatchStarted {
                    batch_id: current_batch.id,
                    label: current_batch.label.clone(),
                    total,
                });
                ctx.request_repaint();

                // Connect to SQLite catalog
                let db = match Database::open(&db_path) {
                    Ok(d) => d,
                    Err(e) => {
                        let now_str = Local::now().format("%H:%M:%S").to_string();
                        let problem = ImportProblem {
                            batch_id: current_batch.id,
                            batch_label: current_batch.label.clone(),
                            file_name: "Database Error".to_string(),
                            reason: format!("Failed to open catalog database: {}", e),
                            timestamp: now_str,
                        };
                        let _ = tx_event.send(ImportEvent::ProblemLogged(problem));
                        let _ = tx_event.send(ImportEvent::BatchCompleted {
                            batch_id: current_batch.id,
                            imported: 0,
                            duplicates: 0,
                            failed: total,
                        });
                        ctx.request_repaint();
                        continue;
                    }
                };

                let conn = match db.conn() {
                    Ok(c) => {
                        let _ = c.busy_timeout(std::time::Duration::from_secs(5));
                        c
                    }
                    Err(e) => {
                        let now_str = Local::now().format("%H:%M:%S").to_string();
                        let problem = ImportProblem {
                            batch_id: current_batch.id,
                            batch_label: current_batch.label.clone(),
                            file_name: "Database Connection Error".to_string(),
                            reason: format!("Failed to connect to catalog: {}", e),
                            timestamp: now_str,
                        };
                        let _ = tx_event.send(ImportEvent::ProblemLogged(problem));
                        let _ = tx_event.send(ImportEvent::BatchCompleted {
                            batch_id: current_batch.id,
                            imported: 0,
                            duplicates: 0,
                            failed: total,
                        });
                        ctx.request_repaint();
                        continue;
                    }
                };

                let mut imported = 0;
                let mut duplicates = 0;
                let mut failed = 0;
                let mut was_cancelled = false;

                for (idx, file_path) in files.into_iter().enumerate() {
                    // Check for cancel commands or queue modifications while iterating
                    while let Ok(cmd) = rx_cmd.try_recv() {
                        match cmd {
                            ImportWorkerCommand::Enqueue(job) => {
                                internal_queue.push_back(job);
                            }
                            ImportWorkerCommand::CancelCurrent => {
                                cancel_flag_clone.store(true, Ordering::Relaxed);
                            }
                            ImportWorkerCommand::RemoveQueued(id) => {
                                internal_queue.retain(|j| j.id != id);
                            }
                            ImportWorkerCommand::CancelAll => {
                                internal_queue.clear();
                                cancel_flag_clone.store(true, Ordering::Relaxed);
                            }
                        }
                    }

                    if cancel_flag_clone.load(Ordering::Relaxed) {
                        was_cancelled = true;
                        break;
                    }

                    let file_name = file_path
                        .file_name()
                        .map_or("".to_string(), |s| s.to_string_lossy().to_string());

                    let _ = tx_event.send(ImportEvent::Progress {
                        batch_id: current_batch.id,
                        current: idx + 1,
                        total,
                        current_file: file_name.clone(),
                        imported,
                        duplicates,
                        failed,
                    });
                    ctx.request_repaint();

                    let path_str = file_path.to_string_lossy().to_string();

                    // Check duplicate
                    if file_exists(&conn, &path_str) {
                        duplicates += 1;
                        continue;
                    }

                    // Probe and insert
                    match probe_video_file(&file_path) {
                        Ok(mut record) => {
                            match insert_video(&conn, &record) {
                                Ok(new_id) => {
                                    record.id = new_id;
                                    imported += 1;

                                    if let Ok(thumb_bytes) = extract_thumbnail(&file_path, record.duration_seconds) {
                                        let _ = store_thumb(&conn, new_id, &thumb_bytes);
                                    }

                                    let _ = tx_event.send(ImportEvent::ClipImported {
                                        batch_id: current_batch.id,
                                        file_name: file_name.clone(),
                                    });
                                }
                                Err(db_err) => {
                                    failed += 1;
                                    let now_str = Local::now().format("%H:%M:%S").to_string();
                                    let problem = ImportProblem {
                                        batch_id: current_batch.id,
                                        batch_label: current_batch.label.clone(),
                                        file_name: file_name.clone(),
                                        reason: format!("Database error: {}", db_err),
                                        timestamp: now_str,
                                    };
                                    let _ = tx_event.send(ImportEvent::ProblemLogged(problem));
                                }
                            }
                        }
                        Err(probe_err) => {
                            failed += 1;
                            let now_str = Local::now().format("%H:%M:%S").to_string();
                            let problem = ImportProblem {
                                batch_id: current_batch.id,
                                batch_label: current_batch.label.clone(),
                                file_name: file_name.clone(),
                                reason: probe_err,
                                timestamp: now_str,
                            };
                            let _ = tx_event.send(ImportEvent::ProblemLogged(problem));
                        }
                    }
                }

                if was_cancelled {
                    let _ = tx_event.send(ImportEvent::BatchCancelled {
                        batch_id: current_batch.id,
                    });
                } else {
                    let _ = tx_event.send(ImportEvent::BatchCompleted {
                        batch_id: current_batch.id,
                        imported,
                        duplicates,
                        failed,
                    });
                }
                ctx.request_repaint();
            }
        });

        Self {
            tx_cmd,
            rx_event,
            cancel_current_flag,
            state: ImportQueueState::default(),
            next_job_id: 1,
        }
    }

    /// Add a batch of files/folders to the import queue
    pub fn enqueue(&mut self, paths: Vec<PathBuf>, label_override: Option<String>) -> u64 {
        if paths.is_empty() {
            return 0;
        }

        let id = self.next_job_id;
        self.next_job_id += 1;

        let label = if let Some(l) = label_override {
            l
        } else if paths.len() == 1 {
            paths[0]
                .file_name()
                .map_or("Folder".to_string(), |s| s.to_string_lossy().to_string())
        } else {
            format!("{} items", paths.len())
        };

        let job = ImportBatchJob {
            id,
            label,
            paths,
        };

        self.state.queued_jobs.push(job.clone());
        let _ = self.tx_cmd.send(ImportWorkerCommand::Enqueue(job));
        id
    }

    /// Request cancellation of the currently processing batch
    pub fn cancel_current(&mut self) {
        self.cancel_current_flag.store(true, Ordering::Relaxed);
        let _ = self.tx_cmd.send(ImportWorkerCommand::CancelCurrent);
    }

    /// Remove a job from the pending queue
    pub fn remove_queued(&mut self, id: u64) {
        self.state.queued_jobs.retain(|j| j.id != id);
        let _ = self.tx_cmd.send(ImportWorkerCommand::RemoveQueued(id));
    }

    /// Cancel current job and clear all queued jobs
    pub fn cancel_all(&mut self) {
        self.state.queued_jobs.clear();
        self.cancel_current_flag.store(true, Ordering::Relaxed);
        let _ = self.tx_cmd.send(ImportWorkerCommand::CancelAll);
    }

    /// Clear recorded problems list
    pub fn clear_problems(&mut self) {
        self.state.problems.clear();
        self.state.has_unseen_problems = false;
    }

    /// Mark problems as seen (clears warning badge)
    pub fn mark_problems_seen(&mut self) {
        self.state.has_unseen_problems = false;
    }

    /// Poll incoming events from worker thread.
    /// Returns (clips_added, batch_finished) so caller can refresh UI / catalog.
    pub fn poll_events(&mut self) -> (bool, bool) {
        let mut clips_added = false;
        let mut batch_finished = false;

        while let Ok(event) = self.rx_event.try_recv() {
            match event {
                ImportEvent::BatchStarted { batch_id, label, total } => {
                    self.state.queued_jobs.retain(|j| j.id != batch_id);
                    self.state.active_job = Some(ActiveImportStatus {
                        batch_id,
                        label,
                        current: 0,
                        total,
                        current_file: "Preparing...".to_string(),
                        imported: 0,
                        duplicates: 0,
                        failed: 0,
                    });
                }
                ImportEvent::Progress {
                    batch_id,
                    current,
                    total,
                    current_file,
                    imported,
                    duplicates,
                    failed,
                } => {
                    if let Some(ref mut active) = self.state.active_job {
                        if active.batch_id == batch_id {
                            active.current = current;
                            active.total = total;
                            active.current_file = current_file;
                            active.imported = imported;
                            active.duplicates = duplicates;
                            active.failed = failed;
                        }
                    }
                }
                ImportEvent::ClipImported { .. } => {
                    clips_added = true;
                }
                ImportEvent::ProblemLogged(prob) => {
                    self.state.problems.push(prob);
                    self.state.has_unseen_problems = true;
                }
                ImportEvent::BatchCompleted { imported, duplicates, .. } => {
                    self.state.completed_batches += 1;
                    self.state.total_imported += imported;
                    self.state.total_duplicates += duplicates;
                    self.state.active_job = None;
                    batch_finished = true;
                }
                ImportEvent::BatchCancelled { .. } => {
                    self.state.active_job = None;
                    batch_finished = true;
                }
                ImportEvent::QueueIdle => {
                    self.state.active_job = None;
                }
            }
        }

        (clips_added, batch_finished)
    }

    pub fn is_active(&self) -> bool {
        self.state.active_job.is_some()
    }

    pub fn is_busy(&self) -> bool {
        self.state.active_job.is_some() || !self.state.queued_jobs.is_empty()
    }

    pub fn state(&self) -> &ImportQueueState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut ImportQueueState {
        &mut self.state
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

    #[test]
    fn test_import_batch_queue_management() {
        let mut state = ImportQueueState::default();
        let job1 = ImportBatchJob {
            id: 1,
            label: "Card_01".to_string(),
            paths: vec![PathBuf::from("/card1")],
        };
        let job2 = ImportBatchJob {
            id: 2,
            label: "Card_02".to_string(),
            paths: vec![PathBuf::from("/card2")],
        };

        state.queued_jobs.push(job1.clone());
        state.queued_jobs.push(job2.clone());
        assert_eq!(state.queued_jobs.len(), 2);

        // Remove job 1 from queue
        state.queued_jobs.retain(|j| j.id != 1);
        assert_eq!(state.queued_jobs.len(), 1);
        assert_eq!(state.queued_jobs[0].id, 2);

        // Log a problem
        let prob = ImportProblem {
            batch_id: 2,
            batch_label: "Card_02".to_string(),
            file_name: "corrupt.mov".to_string(),
            reason: "Invalid atom length".to_string(),
            timestamp: "12:00:00".to_string(),
        };
        state.problems.push(prob);
        state.has_unseen_problems = true;
        assert_eq!(state.problems.len(), 1);
        assert!(state.has_unseen_problems);
    }
}
