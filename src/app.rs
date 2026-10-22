use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::{unbounded, Receiver};
use egui::{CentralPanel, Context, RichText, SidePanel, TopBottomPanel, Vec2};

use crate::config::AppConfig;
use crate::db::models::{CatalogStats, FilterCriteria, PickFlag, Tag, VideoRecord};
use crate::db::{
    add_tag_to_video, create_tag, delete_video, get_catalog_stats, get_distinct_cameras,
    get_distinct_codecs, get_distinct_containers, get_distinct_folders, list_tags, query_videos,
    remove_tag_from_video, update_color_label, update_flag, update_notes, update_rating,
    update_slate, Database,
};
use crate::export::export_and_launch;
use crate::importer::ImportQueueManager;
use crate::ingester::{IngestMessage, Ingester};
use crate::media::color_lut::LogProfile;
use crate::playback::VideoPlayer;
use crate::thumbnails::ThumbnailManager;
use crate::ui::dialogs::catalog_dialog::{render_catalog_dialog, CatalogDialogResult, CatalogDialogState};
use crate::ui::dialogs::delete_dialog::{render_delete_dialog, DeleteChoice, DeleteDialogState};
use crate::ui::dialogs::export_dialog::{render_export_dialog, ExportDialogAction, ExportDialogState};
use crate::ui::dialogs::import_dialog::{
    render_import_dialog, render_import_status_window, ImportDialogAction, ImportDialogState,
    ImportStatusAction,
};
use crate::ui::dialogs::ingest_dialog::{render_ingest_dialog, IngestDialogAction, IngestDialogState};
use crate::ui::grid::{render_grid, GridAction};
use crate::ui::inspector::render_inspector;
use crate::ui::inspector::sections::InspectorAction;
use crate::ui::navigator::{render_navigator, NavAction};
use crate::ui::player::render_player_view;
use crate::ui::player::transport::TransportAction;
use crate::ui::theme::*;
use crate::ui::top_bar::{render_top_bar, TopBarAction};
use crate::ui::widgets::kbd_pill;

pub struct CineExplorerApp {
    config: AppConfig,
    db: Option<Database>,
    current_catalog_path: Option<PathBuf>,
    videos: Vec<VideoRecord>,
    selected_ids: HashSet<i64>,
    last_selected_id: Option<i64>,
    stats: CatalogStats,
    containers: Vec<(String, usize)>,
    codecs: Vec<(String, usize)>,
    cameras: Vec<(String, usize)>,
    folders: Vec<String>,
    tags: Vec<Tag>,
    filter: FilterCriteria,
    card_width: f32,
    show_navigator: bool,
    show_inspector: bool,
    show_shortcuts_modal: bool,
    is_player_mode: bool,
    player_pending_close: bool,
    active_player: Option<VideoPlayer>,
    thumb_mgr: Option<ThumbnailManager>,
    rescan_rx: Option<Receiver<(i64, VideoRecord)>>,

    // Import queue & Ingest state
    import_queue: Option<ImportQueueManager>,
    show_import_status_window: bool,
    last_live_catalog_refresh: std::time::Instant,
    ingest_rx: Option<Receiver<IngestMessage>>,
    ingest_cancel_flag: Option<Arc<AtomicBool>>,

    // Dialogs
    catalog_dialog: CatalogDialogState,
    import_dialog: ImportDialogState,
    ingest_dialog: IngestDialogState,
    delete_dialog: DeleteDialogState,
    export_dialog: ExportDialogState,

    // Inspector edit buffers
    edit_scene: String,
    edit_shot: String,
    edit_take: String,
    edit_reel: String,
    edit_location: String,
    edit_unit: String,
    edit_notes: String,
    new_clip_tag: String,
    new_nav_tag: String,
}

impl CineExplorerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_theme(&cc.egui_ctx);

        let config = AppConfig::load();
        let card_width = config.card_width;

        let mut app = Self {
            config: config.clone(),
            db: None,
            current_catalog_path: None,
            videos: Vec::new(),
            selected_ids: HashSet::new(),
            last_selected_id: None,
            stats: CatalogStats::default(),
            containers: Vec::new(),
            codecs: Vec::new(),
            cameras: Vec::new(),
            folders: Vec::new(),
            tags: Vec::new(),
            filter: FilterCriteria::default(),
            card_width,
            show_navigator: true,
            show_inspector: true,
            show_shortcuts_modal: false,
            is_player_mode: false,
            player_pending_close: false,
            active_player: None,
            thumb_mgr: None,
            rescan_rx: None,
            import_queue: None,
            show_import_status_window: false,
            last_live_catalog_refresh: std::time::Instant::now(),
            ingest_rx: None,
            ingest_cancel_flag: None,
            catalog_dialog: CatalogDialogState::default(),
            import_dialog: ImportDialogState::default(),
            ingest_dialog: IngestDialogState::default(),
            delete_dialog: DeleteDialogState::default(),
            export_dialog: ExportDialogState::default(),
            edit_scene: String::new(),
            edit_shot: String::new(),
            edit_take: String::new(),
            edit_reel: String::new(),
            edit_location: String::new(),
            edit_unit: String::new(),
            edit_notes: String::new(),
            new_clip_tag: String::new(),
            new_nav_tag: String::new(),
        };

        // Load last opened catalog or create a default catalog
        if let Some(ref path) = config.last_catalog_path {
            if path.exists() {
                app.open_catalog(path.clone(), &cc.egui_ctx);
            }
        }

        if app.db.is_none() {
            let default_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let default_cat = default_dir.join("library.cinecat");
            app.open_catalog(default_cat, &cc.egui_ctx);
        }

        app
    }

    pub fn open_catalog(&mut self, path: PathBuf, ctx: &Context) {
        if let Ok(db) = Database::open(&path) {
            self.thumb_mgr = Some(ThumbnailManager::new(path.clone(), ctx.clone()));
            self.import_queue = Some(ImportQueueManager::new(path.clone(), ctx.clone()));
            self.db = Some(db);
            self.current_catalog_path = Some(path.clone());
            self.config.add_recent_catalog(path);
            self.selected_ids.clear();
            self.last_selected_id = None;
            self.active_player = None;
            self.refresh_catalog();

            // Auto-detect and re-scan clips missing camera/lens/log metadata in background
            let missing_items: Vec<(i64, PathBuf)> = self.videos
                .iter()
                .filter(|v| v.camera_model.is_empty() || v.log_profile.is_empty())
                .map(|v| (v.id, PathBuf::from(&v.file_path)))
                .collect();
            if !missing_items.is_empty() {
                self.spawn_rescan(missing_items, ctx);
            }
        }
    }

    pub fn refresh_catalog(&mut self) {
        let db = match self.db.as_ref() {
            Some(d) => d,
            None => return,
        };

        let conn = match db.conn() {
            Ok(c) => c,
            Err(_) => return,
        };

        self.stats = get_catalog_stats(&conn);
        self.containers = get_distinct_containers(&conn);
        self.codecs = get_distinct_codecs(&conn);
        self.cameras = get_distinct_cameras(&conn);
        self.folders = get_distinct_folders(&conn);
        self.tags = list_tags(&conn).unwrap_or_default();
        self.videos = query_videos(&conn, &self.filter).unwrap_or_default();

        self.sync_inspector_buffers();
    }

    fn sync_inspector_buffers(&mut self) {
        let selected_opt = self.selected_video().cloned();
        if let Some(video) = selected_opt {
            self.edit_scene = video.scene;
            self.edit_shot = video.shot;
            self.edit_take = video.take;
            self.edit_reel = video.reel;
            self.edit_location = video.location;
            self.edit_unit = video.camera_unit;
            self.edit_notes = video.notes;
        } else {
            self.edit_scene.clear();
            self.edit_shot.clear();
            self.edit_take.clear();
            self.edit_reel.clear();
            self.edit_location.clear();
            self.edit_unit.clear();
            self.edit_notes.clear();
        }
    }

    fn selected_video(&self) -> Option<&VideoRecord> {
        let first_id = self.selected_ids.iter().next()?;
        self.videos.iter().find(|v| v.id == *first_id)
    }

    fn current_clip_index(&self) -> (usize, usize) {
        let total = self.videos.len();
        if let Some(ref p) = self.active_player {
            if let Some(pos) = self.videos.iter().position(|v| v.id == p.clip.id) {
                return (pos, total);
            }
        }
        if let Some(id) = self.selected_ids.iter().next() {
            if let Some(pos) = self.videos.iter().position(|v| v.id == *id) {
                return (pos, total);
            }
        }
        (0, total)
    }

    fn handle_drag_and_drop(&mut self, ctx: &Context) {
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let paths: Vec<PathBuf> = i.raw.dropped_files.iter()
                    .filter_map(|df| df.path.clone())
                    .collect();
                if !paths.is_empty() {
                    if let Some(ref mut queue) = self.import_queue {
                        queue.enqueue(paths, None);
                        self.show_import_status_window = true;
                    } else {
                        self.import_dialog.selected_paths = paths;
                        self.import_dialog.is_open = true;
                    }
                }
            }
        });
    }

    fn open_player(&mut self, record: VideoRecord, ctx: &Context) {
        self.player_pending_close = false;
        self.selected_ids.clear();
        self.selected_ids.insert(record.id);
        self.last_selected_id = Some(record.id);
        self.active_player = Some(VideoPlayer::new(record, ctx.clone()));
        self.is_player_mode = true;
        self.sync_inspector_buffers();
    }

    fn close_player(&mut self) {
        if let Some(ref mut p) = self.active_player {
            p.pause();
        }
        self.is_player_mode = false;
        self.player_pending_close = true;
    }

    fn next_player_clip(&mut self, ctx: &Context) {
        if self.videos.is_empty() {
            return;
        }
        let cur_id = self.active_player.as_ref().map(|p| p.clip.id).or_else(|| self.selected_ids.iter().next().copied());
        let next_idx = match cur_id {
            Some(id) => {
                if let Some(pos) = self.videos.iter().position(|v| v.id == id) {
                    (pos + 1) % self.videos.len()
                } else {
                    0
                }
            }
            None => 0,
        };
        let next_vid = self.videos[next_idx].clone();
        self.open_player(next_vid, ctx);
    }

    fn prev_player_clip(&mut self, ctx: &Context) {
        if self.videos.is_empty() {
            return;
        }
        let cur_id = self.active_player.as_ref().map(|p| p.clip.id).or_else(|| self.selected_ids.iter().next().copied());
        let prev_idx = match cur_id {
            Some(id) => {
                if let Some(pos) = self.videos.iter().position(|v| v.id == id) {
                    if pos == 0 { self.videos.len() - 1 } else { pos - 1 }
                } else {
                    0
                }
            }
            None => 0,
        };
        let prev_vid = self.videos[prev_idx].clone();
        self.open_player(prev_vid, ctx);
    }

    fn navigate_grid(&mut self, delta: isize, ctx: &Context) {
        if self.videos.is_empty() {
            return;
        }
        let cur_idx = self.selected_ids.iter().next()
            .and_then(|id| self.videos.iter().position(|v| v.id == *id))
            .unwrap_or(0) as isize;

        let next_idx = (cur_idx + delta).clamp(0, (self.videos.len() - 1) as isize) as usize;
        let vid_id = self.videos[next_idx].id;
        self.selected_ids.clear();
        self.selected_ids.insert(vid_id);
        self.last_selected_id = Some(vid_id);
        self.sync_inspector_buffers();

        if self.is_player_mode {
            let vid = self.videos[next_idx].clone();
            self.open_player(vid, ctx);
        }
    }

    fn select_range(&mut self, target_id: i64) {
        let last_id = self.last_selected_id.unwrap_or(target_id);
        let start_pos = self.videos.iter().position(|v| v.id == last_id);
        let end_pos = self.videos.iter().position(|v| v.id == target_id);

        if let (Some(s), Some(e)) = (start_pos, end_pos) {
            let (min_idx, max_idx) = if s < e { (s, e) } else { (e, s) };
            for v in &self.videos[min_idx..=max_idx] {
                self.selected_ids.insert(v.id);
            }
        } else {
            self.selected_ids.insert(target_id);
        }
        self.last_selected_id = Some(target_id);
        self.sync_inspector_buffers();
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &Context) {
        let text_focused = ctx.wants_keyboard_input();

        ctx.input(|i| {
            // Global shortcuts that work even when typing
            if i.key_pressed(egui::Key::Escape) {
                if self.show_shortcuts_modal {
                    self.show_shortcuts_modal = false;
                } else if self.is_player_mode {
                    self.close_player();
                } else if !self.selected_ids.is_empty() {
                    self.selected_ids.clear();
                    self.sync_inspector_buffers();
                }
                return;
            }

            // Command / Ctrl key shortcuts
            let cmd = i.modifiers.command || i.modifiers.ctrl;
            if cmd {
                // Ctrl+A: Select All
                if i.key_pressed(egui::Key::A) && !text_focused {
                    for v in &self.videos {
                        self.selected_ids.insert(v.id);
                    }
                    self.sync_inspector_buffers();
                }
                // Ctrl+D: Deselect All
                if i.key_pressed(egui::Key::D) && !text_focused {
                    self.selected_ids.clear();
                    self.sync_inspector_buffers();
                }
                // Ctrl+O: Open Catalog
                if i.key_pressed(egui::Key::O) {
                    self.catalog_dialog.is_creating_new = false;
                    self.catalog_dialog.is_open = true;
                }
                // Ctrl+N: New Catalog
                if i.key_pressed(egui::Key::N) {
                    self.catalog_dialog.is_creating_new = true;
                    self.catalog_dialog.is_open = true;
                }
                // Ctrl+Shift+I: Ingest
                if i.key_pressed(egui::Key::I) && i.modifiers.shift {
                    self.ingest_dialog.is_open = true;
                }
                // Ctrl+I: Import
                else if i.key_pressed(egui::Key::I) && !i.modifiers.shift {
                    self.import_dialog.is_open = true;
                }
                // Ctrl+E: Export to NLE
                if i.key_pressed(egui::Key::E) {
                    self.export_dialog.selected_count = self.selected_ids.len();
                    self.export_dialog.total_count = self.videos.len();
                    self.export_dialog.use_selected_only = !self.selected_ids.is_empty();
                    self.export_dialog.is_open = true;
                }
                // Ctrl+B: Toggle Navigator
                if i.key_pressed(egui::Key::B) {
                    self.show_navigator = !self.show_navigator;
                }
                // Ctrl+J: Toggle Inspector
                if i.key_pressed(egui::Key::J) {
                    self.show_inspector = !self.show_inspector;
                }
                // Ctrl+L: Toggle Loop in player
                if i.key_pressed(egui::Key::L) {
                    if let Some(ref mut p) = self.active_player {
                        p.looping = !p.looping;
                    }
                }
                return;
            }

            // Guard single-key shortcuts from firing while editing text!
            if text_focused {
                return;
            }

            // Shortcuts Help: ? or F1
            if i.key_pressed(egui::Key::F1) || (i.key_pressed(egui::Key::Slash) && i.modifiers.shift) {
                self.show_shortcuts_modal = !self.show_shortcuts_modal;
                return;
            }

            // View Mode Shortcuts
            // G: Grid view
            if i.key_pressed(egui::Key::G) && self.is_player_mode {
                self.close_player();
            }
            // Tab: Toggle between Grid and Player
            if i.key_pressed(egui::Key::Tab) {
                if self.is_player_mode {
                    self.close_player();
                } else if let Some(v) = self.selected_video().cloned() {
                    self.open_player(v, ctx);
                }
            }
            // Enter: Open in player if in grid, or close if in player
            if i.key_pressed(egui::Key::Enter) {
                if self.is_player_mode {
                    self.close_player();
                } else if let Some(v) = self.selected_video().cloned() {
                    self.open_player(v, ctx);
                }
            }

            // Space: Play/Pause in player, or open in player if in grid
            if i.key_pressed(egui::Key::Space) {
                if let Some(ref mut p) = self.active_player {
                    p.toggle_play();
                } else if let Some(v) = self.selected_video().cloned() {
                    self.open_player(v, ctx);
                }
            }

            // J / K / L Shuttle Controls (NLE Standard)
            if self.is_player_mode {
                if i.key_pressed(egui::Key::J) {
                    if let Some(ref mut p) = self.active_player { p.shuttle_reverse(); }
                }
                if i.key_pressed(egui::Key::K) {
                    if let Some(ref mut p) = self.active_player { p.shuttle_stop(); }
                }
                if i.key_pressed(egui::Key::L) {
                    if let Some(ref mut p) = self.active_player {
                        if i.modifiers.alt {
                            p.toggle_rec709_lut();
                        } else {
                            p.shuttle_forward();
                        }
                    }
                }

                // In / Out Points
                if i.key_pressed(egui::Key::I) {
                    if let Some(ref mut p) = self.active_player {
                        if i.modifiers.shift {
                            p.goto_in();
                        } else {
                            p.in_point = Some(p.playhead);
                        }
                    }
                }
                if i.key_pressed(egui::Key::O) {
                    if let Some(ref mut p) = self.active_player {
                        if i.modifiers.shift {
                            p.goto_out();
                        } else {
                            p.out_point = Some(p.playhead);
                        }
                    }
                }
                if i.key_pressed(egui::Key::X) && i.modifiers.alt {
                    if let Some(ref mut p) = self.active_player {
                        p.in_point = None;
                        p.out_point = None;
                    }
                }
                // Home / End
                if i.key_pressed(egui::Key::Home) {
                    if let Some(ref mut p) = self.active_player { p.goto_start(); }
                }
                if i.key_pressed(egui::Key::End) {
                    if let Some(ref mut p) = self.active_player { p.goto_end(); }
                }
            }

            // Arrow Keys
            if i.key_pressed(egui::Key::ArrowLeft) {
                if self.is_player_mode {
                    if let Some(ref mut p) = self.active_player {
                        if i.modifiers.shift { p.step_10_backward(); } else { p.step_backward(); }
                    }
                } else {
                    self.navigate_grid(-1, ctx);
                }
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                if self.is_player_mode {
                    if let Some(ref mut p) = self.active_player {
                        if i.modifiers.shift { p.step_10_forward(); } else { p.step_forward(); }
                    }
                } else {
                    self.navigate_grid(1, ctx);
                }
            }
            if i.key_pressed(egui::Key::ArrowUp) && !self.is_player_mode {
                self.navigate_grid(-4, ctx);
            }
            if i.key_pressed(egui::Key::ArrowDown) && !self.is_player_mode {
                self.navigate_grid(4, ctx);
            }

            // [ and ] Clip Navigation
            if i.key_pressed(egui::Key::OpenBracket) {
                self.prev_player_clip(ctx);
            }
            if i.key_pressed(egui::Key::CloseBracket) {
                self.next_player_clip(ctx);
            }

            // Culling & Batch Flagging: P (Pick), X (Reject), U (Unflag)
            if i.key_pressed(egui::Key::P) && !i.modifiers.alt {
                self.update_selected_flags(PickFlag::Picked);
            }
            if i.key_pressed(egui::Key::X) && !i.modifiers.alt {
                self.update_selected_flags(PickFlag::Rejected);
            }
            if i.key_pressed(egui::Key::U) {
                self.update_selected_flags(PickFlag::Unflagged);
            }

            // Star Ratings 0–5
            if i.key_pressed(egui::Key::Num0) {
                self.update_selected_ratings(0);
            }
            for star in 1..=5 {
                let key = match star {
                    1 => egui::Key::Num1,
                    2 => egui::Key::Num2,
                    3 => egui::Key::Num3,
                    4 => egui::Key::Num4,
                    5 => egui::Key::Num5,
                    _ => continue,
                };
                if i.key_pressed(key) {
                    self.update_selected_ratings(star);
                }
            }

            // Delete / Backspace: Remove selected video
            if i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace) {
                if let Some(v) = self.selected_video().cloned() {
                    self.delete_dialog.video_id = v.id;
                    self.delete_dialog.file_path = v.file_path;
                    self.delete_dialog.file_name = v.file_name;
                    self.delete_dialog.is_open = true;
                }
            }
        });
    }

    fn update_selected_ratings(&mut self, rating: u8) {
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.conn() {
                for id in &self.selected_ids {
                    let _ = update_rating(&conn, *id, rating);
                    if let Some(v) = self.videos.iter_mut().find(|v| v.id == *id) {
                        v.rating = rating;
                    }
                }
                if let Some(ref mut p) = self.active_player {
                    if self.selected_ids.contains(&p.clip.id) {
                        p.clip.rating = rating;
                    }
                }
            }
        }
    }

    fn update_selected_flags(&mut self, flag: PickFlag) {
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.conn() {
                for id in &self.selected_ids {
                    let _ = update_flag(&conn, *id, flag);
                    if let Some(v) = self.videos.iter_mut().find(|v| v.id == *id) {
                        v.flag = flag;
                    }
                }
                if let Some(ref mut p) = self.active_player {
                    if self.selected_ids.contains(&p.clip.id) {
                        p.clip.flag = flag;
                    }
                }
            }
        }
    }

    fn update_video_rating(&mut self, video_id: i64, rating: u8) {
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.conn() {
                let _ = update_rating(&conn, video_id, rating);
                if let Some(v) = self.videos.iter_mut().find(|v| v.id == video_id) {
                    v.rating = rating;
                }
                if let Some(ref mut p) = self.active_player {
                    if p.clip.id == video_id {
                        p.clip.rating = rating;
                    }
                }
            }
        }
    }

    fn update_video_flag(&mut self, video_id: i64, flag: PickFlag) {
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.conn() {
                let _ = update_flag(&conn, video_id, flag);
                if let Some(v) = self.videos.iter_mut().find(|v| v.id == video_id) {
                    v.flag = flag;
                }
                if let Some(ref mut p) = self.active_player {
                    if p.clip.id == video_id {
                        p.clip.flag = flag;
                    }
                }
            }
        }
    }

    fn update_video_color(&mut self, video_id: i64, color: String) {
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.conn() {
                let _ = update_color_label(&conn, video_id, &color);
                if let Some(v) = self.videos.iter_mut().find(|v| v.id == video_id) {
                    v.color_label = color;
                }
            }
        }
    }

    fn update_video_log_profile(&mut self, video_id: i64, profile: LogProfile) {
        let prof_str = profile.as_str().to_string();
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.conn() {
                let _ = conn.execute(
                    "UPDATE videos SET log_profile = ?1 WHERE id = ?2",
                    rusqlite::params![prof_str, video_id],
                );
            }
        }
        if let Some(v) = self.videos.iter_mut().find(|v| v.id == video_id) {
            v.log_profile = prof_str.clone();
        }
        if let Some(ref mut p) = self.active_player {
            if p.clip.id == video_id {
                p.clip.log_profile = prof_str;
                p.set_log_profile(profile);
            }
        }
        if let Some(ref mut tm) = self.thumb_mgr {
            tm.invalidate(video_id);
        }
        self.refresh_catalog();
    }

    fn spawn_rescan(&mut self, items: Vec<(i64, PathBuf)>, ctx: &Context) {
        if items.is_empty() {
            return;
        }
        let catalog_path = match self.current_catalog_path.clone() {
            Some(p) => p,
            None => return,
        };

        let (tx, rx) = unbounded();
        self.rescan_rx = Some(rx);
        let ctx_clone = ctx.clone();

        std::thread::spawn(move || {
            for (id, path) in items {
                if let Ok(probe) = crate::media::probe::probe_video_file(&path) {
                    if let Ok(conn) = rusqlite::Connection::open(&catalog_path) {
                        let _ = crate::db::queries::update_video_metadata(
                            &conn,
                            id,
                            &probe.camera_make,
                            &probe.camera_model,
                            &probe.lens_model,
                            &probe.focal_length,
                            &probe.aperture,
                            probe.iso,
                            &probe.shutter_speed,
                            &probe.white_balance,
                            &probe.log_profile,
                            probe.date_recorded.as_deref(),
                        );
                    }
                    let _ = tx.send((id, probe));
                    ctx_clone.request_repaint();
                }
            }
        });
    }
}

impl eframe::App for CineExplorerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll thumbnail manager
        if let Some(ref mut tm) = self.thumb_mgr {
            tm.update(ctx);
        }

        // Poll video player
        if let Some(ref mut player) = self.active_player {
            player.update(ctx);
        }

        // Poll metadata rescan messages
        let mut rescanned_items = Vec::new();
        if let Some(ref rx) = self.rescan_rx {
            while let Ok(msg) = rx.try_recv() {
                rescanned_items.push(msg);
            }
        }
        if !rescanned_items.is_empty() {
            for (id, probe) in rescanned_items {
                if let Some(v) = self.videos.iter_mut().find(|v| v.id == id) {
                    v.camera_make = probe.camera_make.clone();
                    v.camera_model = probe.camera_model.clone();
                    v.lens_model = probe.lens_model.clone();
                    v.focal_length = probe.focal_length.clone();
                    v.aperture = probe.aperture.clone();
                    v.iso = probe.iso;
                    v.shutter_speed = probe.shutter_speed.clone();
                    v.white_balance = probe.white_balance.clone();
                    v.log_profile = probe.log_profile.clone();
                    if let Some(ref d) = probe.date_recorded {
                        v.date_recorded = Some(d.clone());
                    }
                }
                if let Some(ref mut p) = self.active_player {
                    if p.clip.id == id {
                        p.clip.camera_make = probe.camera_make.clone();
                        p.clip.camera_model = probe.camera_model.clone();
                        p.clip.lens_model = probe.lens_model.clone();
                        p.clip.focal_length = probe.focal_length.clone();
                        p.clip.aperture = probe.aperture.clone();
                        p.clip.iso = probe.iso;
                        p.clip.shutter_speed = probe.shutter_speed.clone();
                        p.clip.white_balance = probe.white_balance.clone();
                        p.clip.log_profile = probe.log_profile.clone();
                        p.set_log_profile(LogProfile::from_str(&probe.log_profile));
                    }
                }
                if let Some(ref mut tm) = self.thumb_mgr {
                    tm.invalidate(id);
                }
            }
            self.refresh_catalog();
        }

        // Poll importer events (Background Non-Blocking Queue)
        let mut do_catalog_refresh = false;
        if let Some(ref mut q) = self.import_queue {
            let (clips_added, batch_finished) = q.poll_events();
            if batch_finished || (clips_added && self.last_live_catalog_refresh.elapsed() >= std::time::Duration::from_millis(500)) {
                do_catalog_refresh = true;
            }
        }
        if do_catalog_refresh {
            self.last_live_catalog_refresh = std::time::Instant::now();
            self.refresh_catalog();
        }

        // Poll ingester messages (SD Card / Media Offload & Copy)
        let mut ingest_msgs = Vec::new();
        if let Some(ref rx) = self.ingest_rx {
            while let Ok(msg) = rx.try_recv() {
                ingest_msgs.push(msg);
            }
        }
        for msg in ingest_msgs {
            match msg {
                IngestMessage::Progress(prog) => {
                    self.ingest_dialog.progress = Some(prog);
                }
                IngestMessage::Complete(summary) => {
                    self.ingest_dialog.is_running = false;
                    self.ingest_dialog.summary = Some(summary);
                    self.ingest_rx = None;
                    self.ingest_cancel_flag = None;
                    self.refresh_catalog();
                }
                IngestMessage::Cancelled(summary) => {
                    self.ingest_dialog.is_running = false;
                    self.ingest_dialog.summary = Some(summary);
                    self.ingest_rx = None;
                    self.ingest_cancel_flag = None;
                    self.refresh_catalog();
                }
                IngestMessage::Error(_err) => {
                    self.ingest_dialog.is_running = false;
                    self.ingest_rx = None;
                    self.ingest_cancel_flag = None;
                }
            }
        }

        self.handle_drag_and_drop(ctx);
        self.handle_keyboard_shortcuts(ctx);

        let catalog_name = self.current_catalog_path.as_ref()
            .and_then(|p| p.file_stem())
            .map_or("No Catalog", |s| s.to_str().unwrap_or(""));

        // TOP PANEL: Header & Search & Controls
        let mut top_action = None;
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            let default_queue_state = crate::importer::ImportQueueState::default();
            let q_state = self.import_queue.as_ref().map(|q| q.state()).unwrap_or(&default_queue_state);
            let act = render_top_bar(
                ui,
                catalog_name,
                &self.config.recent_catalogs,
                &mut self.filter,
                &mut self.card_width,
                self.is_player_mode,
                self.show_navigator,
                self.show_inspector,
                q_state,
                self.show_import_status_window,
            );
            if act.is_some() {
                top_action = act;
            }
        });

        if let Some(act) = top_action {
            match act {
                TopBarAction::NewCatalog => {
                    self.catalog_dialog.is_creating_new = true;
                    self.catalog_dialog.is_open = true;
                }
                TopBarAction::OpenCatalog => {
                    self.catalog_dialog.is_creating_new = false;
                    self.catalog_dialog.is_open = true;
                }
                TopBarAction::OpenRecent(p) => {
                    self.open_catalog(p, ctx);
                }
                TopBarAction::StartIngest => {
                    self.ingest_dialog.is_open = true;
                }
                TopBarAction::StartImport => {
                    self.import_dialog.is_open = true;
                }
                TopBarAction::OpenExportDialog => {
                    self.export_dialog.selected_count = self.selected_ids.len();
                    self.export_dialog.total_count = self.videos.len();
                    self.export_dialog.use_selected_only = !self.selected_ids.is_empty();
                    self.export_dialog.is_open = true;
                }
                TopBarAction::TogglePlayerView => {
                    if self.is_player_mode {
                        self.close_player();
                    } else if let Some(v) = self.selected_video().cloned() {
                        self.open_player(v, ctx);
                    }
                }
                TopBarAction::ToggleNavigator => {
                    self.show_navigator = !self.show_navigator;
                }
                TopBarAction::ToggleInspector => {
                    self.show_inspector = !self.show_inspector;
                }
                TopBarAction::ToggleImportStatus => {
                    self.show_import_status_window = !self.show_import_status_window;
                }
                TopBarAction::ShowShortcutsHelp => {
                    self.show_shortcuts_modal = true;
                }
                TopBarAction::FilterChanged => {
                    self.refresh_catalog();
                }
                TopBarAction::RescanMetadata => {
                    let items: Vec<(i64, PathBuf)> = if !self.selected_ids.is_empty() {
                        self.videos
                            .iter()
                            .filter(|v| self.selected_ids.contains(&v.id))
                            .map(|v| (v.id, PathBuf::from(&v.file_path)))
                            .collect()
                    } else {
                        self.videos
                            .iter()
                            .map(|v| (v.id, PathBuf::from(&v.file_path)))
                            .collect()
                    };
                    self.spawn_rescan(items, ctx);
                }
            }
        }

        // BOTTOM PANEL: Professional Status Bar
        TopBottomPanel::bottom("bottom_status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;

                // Left: Catalog & selection stats
                let total_vids = self.videos.len();
                let sel_count = self.selected_ids.len();
                let count_str = if sel_count > 0 {
                    format!("{} of {} clips selected", sel_count, total_vids)
                } else {
                    format!("{} clips", total_vids)
                };
                ui.label(RichText::new(count_str).strong().size(11.5).color(TEXT_PRIMARY));

                let total_gb = (self.stats.total_size_bytes as f64) / (1024.0 * 1024.0 * 1024.0);
                let dur_h = (self.stats.total_duration_seconds / 3600.0) as u32;
                let dur_m = ((self.stats.total_duration_seconds % 3600.0) / 60.0) as u32;
                let dur_s = (self.stats.total_duration_seconds % 60.0) as u32;
                ui.label(
                    RichText::new(format!("· {:.2} GB · {:02}h {:02}m {:02}s", total_gb, dur_h, dur_m, dur_s))
                        .size(11.0)
                        .color(TEXT_MUTED),
                );

                // Center: Background operation progress indicator
                if let Some(ref q) = self.import_queue {
                    let st = q.state();
                    if let Some(ref active) = st.active_job {
                        ui.add(egui::Separator::default().vertical());
                        let text = format!("{} Importing {}/{} ({})", egui_phosphor::regular::SPINNER_GAP, active.current, active.total, active.label);
                        if ui.small_button(RichText::new(text).small().color(ACCENT_BLUE)).on_hover_text("Click to open Import Queue & Status").clicked() {
                            self.show_import_status_window = true;
                        }
                    } else if !st.problems.is_empty() {
                        ui.add(egui::Separator::default().vertical());
                        let text = format!("{} {} problem(s)", egui_phosphor::regular::WARNING, st.problems.len());
                        if ui.small_button(RichText::new(text).small().color(ACCENT_RED)).on_hover_text("Click to view import problems").clicked() {
                            self.show_import_status_window = true;
                        }
                    }
                }
                if self.ingest_dialog.is_running {
                    ui.add(egui::Separator::default().vertical());
                    if ui.small_button(RichText::new("Offloading media...").small().color(ACCENT_ORANGE)).on_hover_text("Click to show Ingest progress").clicked() {
                        self.ingest_dialog.is_open = true;
                    }
                }

                // Right: Quick keyboard shortcuts hint pills
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("? Shortcuts").on_hover_text("Open Keyboard Shortcuts Cheat-sheet (F1)").clicked() {
                        self.show_shortcuts_modal = true;
                    }
                    kbd_pill(ui, "1-5 Rate");
                    kbd_pill(ui, "P/X Flag");
                    kbd_pill(ui, "I/O Points");
                    kbd_pill(ui, "J/K/L Shuttle");
                    kbd_pill(ui, "Space/Enter Play");
                });
            });
        });

        // LEFT PANEL: Navigator & Smart Collections (if open)
        let mut nav_action = None;
        if self.show_navigator {
            SidePanel::left("left_navigator")
                .resizable(true)
                .default_width(220.0)
                .width_range(180.0..=360.0)
                .show(ctx, |ui| {
                    let act = render_navigator(
                        ui,
                        &self.filter.collection,
                        &self.stats,
                        &self.containers,
                        &self.codecs,
                        &self.cameras,
                        &self.folders,
                        &self.tags,
                        &mut self.new_nav_tag,
                    );
                    if act.is_some() {
                        nav_action = act;
                    }
                });
        }

        if let Some(act) = nav_action {
            match act {
                NavAction::SelectCollection(col) => {
                    self.filter.collection = col;
                    self.refresh_catalog();
                }
                NavAction::CreateTag(name, color) => {
                    if let Some(ref db) = self.db {
                        if let Ok(conn) = db.conn() {
                            let _ = create_tag(&conn, &name, &color);
                            self.refresh_catalog();
                        }
                    }
                }
                NavAction::DeleteTag(_) => {}
            }
        }

        // RIGHT PANEL: Metadata Inspector (available in BOTH Grid and Player views!)
        let mut inspector_action = None;
        if self.show_inspector {
            SidePanel::right("right_inspector")
                .resizable(true)
                .default_width(280.0)
                .width_range(240.0..=440.0)
                .show(ctx, |ui| {
                    let sel_video = self.selected_video().cloned();
                    if let Some(ref mut tm) = self.thumb_mgr {
                        let act = render_inspector(
                            ui,
                            sel_video.as_ref(),
                            tm,
                            &mut self.edit_scene,
                            &mut self.edit_shot,
                            &mut self.edit_take,
                            &mut self.edit_reel,
                            &mut self.edit_location,
                            &mut self.edit_unit,
                            &mut self.edit_notes,
                            &mut self.new_clip_tag,
                        );
                        if act.is_some() {
                            inspector_action = act;
                        }
                    }
                });
        }

        if let Some(act) = inspector_action {
            if let Some(sel_id) = self.selected_ids.iter().next().copied() {
                match act {
                    InspectorAction::OpenPlayer(record) => {
                        self.open_player(*record, ctx);
                    }
                    InspectorAction::UpdateRating(r) => self.update_video_rating(sel_id, r),
                    InspectorAction::UpdateFlag(f) => self.update_video_flag(sel_id, f),
                    InspectorAction::UpdateColor(c) => self.update_video_color(sel_id, c),
                    InspectorAction::UpdateNotes(notes) => {
                        if let Some(ref db) = self.db {
                            if let Ok(conn) = db.conn() {
                                let _ = update_notes(&conn, sel_id, &notes);
                                if let Some(v) = self.videos.iter_mut().find(|v| v.id == sel_id) {
                                    v.notes = notes;
                                }
                            }
                        }
                    }
                    InspectorAction::UpdateSlate { scene, shot, take, reel, location, camera_unit } => {
                        if let Some(ref db) = self.db {
                            if let Ok(conn) = db.conn() {
                                let _ = update_slate(&conn, sel_id, &scene, &shot, &take, &reel, &location, &camera_unit);
                                if let Some(v) = self.videos.iter_mut().find(|v| v.id == sel_id) {
                                    v.scene = scene;
                                    v.shot = shot;
                                    v.take = take;
                                    v.reel = reel;
                                    v.location = location;
                                    v.camera_unit = camera_unit;
                                }
                            }
                        }
                    }
                    InspectorAction::AddTag(tag) => {
                        if let Some(ref db) = self.db {
                            if let Ok(conn) = db.conn() {
                                let _ = add_tag_to_video(&conn, sel_id, &tag);
                                self.refresh_catalog();
                            }
                        }
                    }
                    InspectorAction::RemoveTag(tag) => {
                        if let Some(ref db) = self.db {
                            if let Ok(conn) = db.conn() {
                                let _ = remove_tag_from_video(&conn, sel_id, &tag);
                                self.refresh_catalog();
                            }
                        }
                    }
                    InspectorAction::PlayExternal => {
                        if let Some(v) = self.selected_video() {
                            let _ = open::that(&v.file_path);
                        }
                    }
                    InspectorAction::RevealInExplorer => {
                        if let Some(v) = self.selected_video() {
                            let path = Path::new(&v.file_path);
                            if let Some(parent) = path.parent() {
                                let _ = open::that(parent);
                            }
                        }
                    }
                    InspectorAction::OpenExportDialog => {
                        self.export_dialog.selected_count = self.selected_ids.len();
                        self.export_dialog.total_count = self.videos.len();
                        self.export_dialog.use_selected_only = !self.selected_ids.is_empty();
                        self.export_dialog.is_open = true;
                    }
                    InspectorAction::DeleteVideo => {
                        if let Some(v) = self.selected_video().cloned() {
                            self.delete_dialog.video_id = v.id;
                            self.delete_dialog.file_path = v.file_path;
                            self.delete_dialog.file_name = v.file_name;
                            self.delete_dialog.is_open = true;
                        }
                    }
                    InspectorAction::UpdateLogProfile(profile) => {
                        self.update_video_log_profile(sel_id, profile);
                    }
                    InspectorAction::RescanMetadata(id) => {
                        if let Some(v) = self.videos.iter().find(|v| v.id == id) {
                            let item = vec![(id, PathBuf::from(&v.file_path))];
                            self.spawn_rescan(item, ctx);
                        }
                    }
                }
            }
        }

        // CENTRAL PANEL: Video Grid or Integrated Player
        let mut grid_action = None;
        let mut player_action = None;

        let (clip_idx, total_clips) = self.current_clip_index();

        CentralPanel::default().show(ctx, |ui| {
            if self.is_player_mode {
                if let Some(ref mut player) = self.active_player {
                    if let Some(p_act) = render_player_view(ui, player, clip_idx, total_clips) {
                        player_action = Some(p_act);
                    }
                } else {
                    self.is_player_mode = false;
                }
            } else if let Some(ref mut tm) = self.thumb_mgr {
                if let Some(g_act) = render_grid(ui, &self.videos, &self.selected_ids, self.card_width, tm) {
                    grid_action = Some(g_act);
                }
            }
        });

        // Handle Player actions
        if let Some(p_act) = player_action {
            match p_act {
                TransportAction::TogglePlay => {
                    if let Some(ref mut p) = self.active_player { p.toggle_play(); }
                }
                TransportAction::StepBackward => {
                    if let Some(ref mut p) = self.active_player { p.step_backward(); }
                }
                TransportAction::StepForward => {
                    if let Some(ref mut p) = self.active_player { p.step_forward(); }
                }
                TransportAction::Step10Backward => {
                    if let Some(ref mut p) = self.active_player { p.step_10_backward(); }
                }
                TransportAction::Step10Forward => {
                    if let Some(ref mut p) = self.active_player { p.step_10_forward(); }
                }
                TransportAction::Seek(s) => {
                    if let Some(ref mut p) = self.active_player { p.seek(s); }
                }
                TransportAction::SetInPoint => {
                    if let Some(ref mut p) = self.active_player { p.in_point = Some(p.playhead); }
                }
                TransportAction::SetOutPoint => {
                    if let Some(ref mut p) = self.active_player { p.out_point = Some(p.playhead); }
                }
                TransportAction::ClearInOut => {
                    if let Some(ref mut p) = self.active_player {
                        p.in_point = None;
                        p.out_point = None;
                    }
                }
                TransportAction::GotoIn => {
                    if let Some(ref mut p) = self.active_player { p.goto_in(); }
                }
                TransportAction::GotoOut => {
                    if let Some(ref mut p) = self.active_player { p.goto_out(); }
                }
                TransportAction::GotoStart => {
                    if let Some(ref mut p) = self.active_player { p.goto_start(); }
                }
                TransportAction::GotoEnd => {
                    if let Some(ref mut p) = self.active_player { p.goto_end(); }
                }
                TransportAction::UpdateRating(r) => {
                    if let Some(ref p) = self.active_player {
                        let id = p.clip.id;
                        self.update_video_rating(id, r);
                    }
                }
                TransportAction::UpdateFlag(f) => {
                    if let Some(ref p) = self.active_player {
                        let id = p.clip.id;
                        self.update_video_flag(id, f);
                    }
                }
                TransportAction::PrevClip => {
                    self.prev_player_clip(ctx);
                }
                TransportAction::NextClip => {
                    self.next_player_clip(ctx);
                }
                TransportAction::ClosePlayer => {
                    self.close_player();
                }
                TransportAction::PlayExternal => {
                    if let Some(ref p) = self.active_player {
                        let _ = open::that(&p.clip.file_path);
                    }
                }
                TransportAction::ToggleRec709Lut => {
                    if let Some(ref mut p) = self.active_player {
                        p.toggle_rec709_lut();
                    }
                }
                TransportAction::SetLogProfile(profile) => {
                    if let Some(ref mut p) = self.active_player {
                        let id = p.clip.id;
                        p.set_log_profile(profile);
                        self.update_video_log_profile(id, profile);
                    }
                }
            }
        }

        // Handle Grid actions
        if let Some(g_act) = grid_action {
            match g_act {
                GridAction::SelectOnly(id) => {
                    self.selected_ids.clear();
                    self.selected_ids.insert(id);
                    self.last_selected_id = Some(id);
                    self.sync_inspector_buffers();
                }
                GridAction::ToggleSelect(id) => {
                    if self.selected_ids.contains(&id) {
                        self.selected_ids.remove(&id);
                    } else {
                        self.selected_ids.insert(id);
                        self.last_selected_id = Some(id);
                    }
                    self.sync_inspector_buffers();
                }
                GridAction::SelectRange(id) => {
                    self.select_range(id);
                }
                GridAction::OpenPlayer(record) => {
                    self.open_player(*record, ctx);
                }
                GridAction::PlayExternal(file_path) => {
                    let _ = open::that(file_path);
                }
                GridAction::RevealInExplorer(file_path) => {
                    let path = Path::new(&file_path);
                    if let Some(parent) = path.parent() {
                        let _ = open::that(parent);
                    }
                }
                GridAction::UpdateRating(id, r) => self.update_video_rating(id, r),
                GridAction::UpdateFlag(id, f) => self.update_video_flag(id, f),
                GridAction::UpdateColor(id, c) => self.update_video_color(id, c),
                GridAction::DeleteVideo(id) => {
                    if let Some(v) = self.videos.iter().find(|v| v.id == id) {
                        self.delete_dialog.video_id = id;
                        self.delete_dialog.file_path = v.file_path.clone();
                        self.delete_dialog.file_name = v.file_name.clone();
                        self.delete_dialog.is_open = true;
                    }
                }
                GridAction::StartIngest => {
                    self.ingest_dialog.is_open = true;
                }
                GridAction::StartImport => {
                    self.import_dialog.is_open = true;
                }
            }
        }

        // Render Dialogs
        if let Some(cat_res) = render_catalog_dialog(ctx, &mut self.catalog_dialog) {
            match cat_res {
                CatalogDialogResult::Open(p) => self.open_catalog(p, ctx),
                CatalogDialogResult::Create(p) => self.open_catalog(p, ctx),
                CatalogDialogResult::Cancel => {}
            }
        }

        // Render Import Dialog (In-Place Indexing file/folder picker)
        let is_busy = self.import_queue.as_ref().is_some_and(|q| q.is_busy());
        if let Some(imp_act) = render_import_dialog(ctx, &mut self.import_dialog, is_busy) {
            match imp_act {
                ImportDialogAction::EnqueueImport(paths) => {
                    if let Some(ref mut q) = self.import_queue {
                        q.enqueue(paths, None);
                        self.show_import_status_window = true;
                    }
                }
                ImportDialogAction::Cancel => {}
            }
        }

        // Render Floating Import Queue & Status Window
        if let Some(ref mut q) = self.import_queue {
            if let Some(st_act) = render_import_status_window(ctx, q, &mut self.show_import_status_window) {
                match st_act {
                    ImportStatusAction::OpenImportDialog => {
                        self.import_dialog.is_open = true;
                    }
                }
            }
        }

        // Render Ingest Dialog (SD Card / Media Offload & Copy)
        if let Some(ing_act) = render_ingest_dialog(ctx, &mut self.ingest_dialog) {
            match ing_act {
                IngestDialogAction::StartIngest(config) => {
                    if let Some(ref db_path) = self.current_catalog_path {
                        let (tx, rx) = unbounded();
                        let cancel_flag = Arc::new(AtomicBool::new(false));
                        self.ingest_rx = Some(rx);
                        self.ingest_cancel_flag = Some(cancel_flag.clone());
                        self.ingest_dialog.is_running = true;
                        self.ingest_dialog.summary = None;

                        Ingester::start(
                            config,
                            db_path.clone(),
                            tx,
                            cancel_flag,
                            ctx.clone(),
                        );
                    }
                }
                IngestDialogAction::Cancel => {
                    if let Some(ref cancel_flag) = self.ingest_cancel_flag {
                        cancel_flag.store(true, Ordering::Relaxed);
                    }
                }
                IngestDialogAction::Done => {
                    self.refresh_catalog();
                }
            }
        }

        if let Some(del_choice) = render_delete_dialog(ctx, &mut self.delete_dialog) {
            let vid_id = self.delete_dialog.video_id;
            let fpath = self.delete_dialog.file_path.clone();

            match del_choice {
                DeleteChoice::RemoveFromCatalog => {
                    if let Some(ref db) = self.db {
                        if let Ok(conn) = db.conn() {
                            let _ = delete_video(&conn, vid_id);
                            self.selected_ids.remove(&vid_id);
                            self.refresh_catalog();
                        }
                    }
                }
                DeleteChoice::MoveToTrash => {
                    let _ = trash::delete(Path::new(&fpath));
                    if let Some(ref db) = self.db {
                        if let Ok(conn) = db.conn() {
                            let _ = delete_video(&conn, vid_id);
                            self.selected_ids.remove(&vid_id);
                            self.refresh_catalog();
                        }
                    }
                }
                DeleteChoice::Cancel => {}
            }
        }

        if let Some(exp_act) = render_export_dialog(ctx, &mut self.export_dialog) {
            match exp_act {
                ExportDialogAction::ExecuteExport {
                    options,
                    output_path,
                    use_selected_only,
                    ..
                } => {
                    let clips_to_export: Vec<VideoRecord> = if use_selected_only && !self.selected_ids.is_empty() {
                        self.videos
                            .iter()
                            .filter(|v| self.selected_ids.contains(&v.id))
                            .cloned()
                            .collect()
                    } else {
                        self.videos.clone()
                    };

                    let _ = export_and_launch(&clips_to_export, output_path, &options);
                }
                ExportDialogAction::Cancel => {}
            }
        }

        // Keyboard Shortcuts Reference Modal
        if self.show_shortcuts_modal {
            let mut close_modal = false;
            egui::Window::new(RichText::new(format!("{} Keyboard Shortcuts", egui_phosphor::regular::KEYBOARD)).strong())
                .collapsible(false)
                .resizable(false)
                .default_width(520.0)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;

                    egui::Grid::new("shortcuts_grid")
                        .striped(true)
                        .spacing(Vec2::new(16.0, 6.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("ACTION").small().strong().color(TEXT_MUTED));
                            ui.label(RichText::new("SHORTCUT").small().strong().color(TEXT_MUTED));
                            ui.label(RichText::new("CONTEXT").small().strong().color(TEXT_MUTED));
                            ui.end_row();

                            shortcut_row(ui, "Play / Pause", "Space / K", "Player / Grid");
                            shortcut_row(ui, "Open in Player", "Enter / Space", "Grid");
                            shortcut_row(ui, "Back to Grid", "Esc / G / Tab", "Player");
                            shortcut_row(ui, "Shuttle Reverse / Stop / Forward", "J / K / L", "Player (NLE 1x/2x/4x)");
                            shortcut_row(ui, "Step ±1 Frame", "← / →", "Player");
                            shortcut_row(ui, "Step ±10 Frames", "Shift + ← / →", "Player");
                            shortcut_row(ui, "Mark In / Out Point", "I / O", "Player");
                            shortcut_row(ui, "Clear In / Out Points", "Alt + X", "Player");
                            shortcut_row(ui, "Toggle Loop", "Ctrl + L", "Player");
                            shortcut_row(ui, "Toggle Rec.709 LUT", "Alt + L", "Player");
                            shortcut_row(ui, "Next / Prev Clip", "[ / ]", "Player");
                            shortcut_row(ui, "Navigate Grid Cards", "Arrow Keys", "Grid");
                            shortcut_row(ui, "Flag as Pick / Reject / Unflag", "P / X / U", "Global (Batch)");
                            shortcut_row(ui, "Star Rating (0-5)", "0, 1, 2, 3, 4, 5", "Global (Batch)");
                            shortcut_row(ui, "Select All / Deselect All", "Ctrl + A / Ctrl + D", "Grid");
                            shortcut_row(ui, "Range Selection", "Shift + Click", "Grid");
                            shortcut_row(ui, "Multi Selection", "Ctrl + Click", "Grid");
                            shortcut_row(ui, "Toggle Left Navigator", "Ctrl + B / [", "Global");
                            shortcut_row(ui, "Toggle Right Inspector", "Ctrl + J / F9", "Global");
                            shortcut_row(ui, "Ingest from Card / Media", "Ctrl + Shift + I", "Global");
                            shortcut_row(ui, "Import Folder In-Place", "Ctrl + I", "Global");
                            shortcut_row(ui, "Export to DaVinci / Premiere", "Ctrl + E", "Global");
                            shortcut_row(ui, "New / Open Catalog", "Ctrl + N / Ctrl + O", "Global");
                            shortcut_row(ui, "Remove from Catalog", "Delete / Backspace", "Global");
                            shortcut_row(ui, "This Shortcuts Help", "? / F1", "Global");
                        });

                    ui.add_space(10.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close_modal = true;
                        }
                    });
                });

            if close_modal {
                self.show_shortcuts_modal = false;
            }
        }
    }
}

fn shortcut_row(ui: &mut egui::Ui, action: &str, shortcut: &str, context: &str) {
    ui.label(RichText::new(action).size(12.0).color(TEXT_PRIMARY));
    ui.label(RichText::new(shortcut).monospace().strong().size(11.5).color(ACCENT_BLUE));
    ui.label(RichText::new(context).small().color(TEXT_SECONDARY));
    ui.end_row();
}