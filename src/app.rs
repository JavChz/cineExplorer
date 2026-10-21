use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crossbeam_channel::{unbounded, Receiver};
use egui::{CentralPanel, Context, SidePanel, TopBottomPanel};

use crate::config::AppConfig;
use crate::db::models::{CatalogStats, FilterCriteria, PickFlag, Tag, VideoRecord};
use crate::db::{
    add_tag_to_video, create_tag, delete_video, get_catalog_stats, get_distinct_cameras,
    get_distinct_codecs, get_distinct_containers, get_distinct_folders, list_tags, query_videos,
    remove_tag_from_video, update_color_label, update_flag, update_notes, update_rating,
    update_slate, Database,
};
use crate::importer::{ImportMessage, Importer};
use crate::playback::VideoPlayer;
use crate::thumbnails::ThumbnailManager;
use crate::ui::dialogs::catalog_dialog::{render_catalog_dialog, CatalogDialogResult, CatalogDialogState};
use crate::ui::dialogs::delete_dialog::{render_delete_dialog, DeleteChoice, DeleteDialogState};
use crate::ui::dialogs::ingest_dialog::{render_ingest_dialog, IngestDialogAction, IngestDialogState};
use crate::ui::grid::{render_grid, GridAction};
use crate::ui::inspector::render_inspector;
use crate::ui::inspector::sections::InspectorAction;
use crate::ui::navigator::{render_navigator, NavAction};
use crate::ui::player::render_player_view;
use crate::ui::player::transport::TransportAction;
use crate::ui::theme::apply_theme;
use crate::ui::top_bar::{render_top_bar, TopBarAction};

pub struct CineExplorerApp {
    config: AppConfig,
    db: Option<Database>,
    current_catalog_path: Option<PathBuf>,
    videos: Vec<VideoRecord>,
    selected_ids: HashSet<i64>,
    stats: CatalogStats,
    containers: Vec<(String, usize)>,
    codecs: Vec<(String, usize)>,
    cameras: Vec<(String, usize)>,
    folders: Vec<String>,
    tags: Vec<Tag>,
    filter: FilterCriteria,
    card_width: f32,
    show_inspector: bool,
    is_player_mode: bool,
    active_player: Option<VideoPlayer>,
    thumb_mgr: Option<ThumbnailManager>,

    // Ingest state
    import_rx: Option<Receiver<ImportMessage>>,
    import_cancel_flag: Option<Arc<AtomicBool>>,

    // Dialogs
    catalog_dialog: CatalogDialogState,
    ingest_dialog: IngestDialogState,
    delete_dialog: DeleteDialogState,

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
            stats: CatalogStats::default(),
            containers: Vec::new(),
            codecs: Vec::new(),
            cameras: Vec::new(),
            folders: Vec::new(),
            tags: Vec::new(),
            filter: FilterCriteria::default(),
            card_width,
            show_inspector: true,
            is_player_mode: false,
            active_player: None,
            thumb_mgr: None,
            import_rx: None,
            import_cancel_flag: None,
            catalog_dialog: CatalogDialogState::default(),
            ingest_dialog: IngestDialogState::default(),
            delete_dialog: DeleteDialogState::default(),
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
            self.db = Some(db);
            self.current_catalog_path = Some(path.clone());
            self.config.add_recent_catalog(path);
            self.selected_ids.clear();
            self.active_player = None;
            self.refresh_catalog();
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

    fn handle_drag_and_drop(&mut self, ctx: &Context) {
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let paths: Vec<PathBuf> = i.raw.dropped_files.iter()
                    .filter_map(|df| df.path.clone())
                    .collect();
                if !paths.is_empty() {
                    self.ingest_dialog.selected_paths = paths;
                    self.ingest_dialog.is_open = true;
                }
            }
        });
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &Context) {
        ctx.input(|i| {
            // Space: play/pause
            if i.key_pressed(egui::Key::Space) {
                if let Some(ref mut p) = self.active_player {
                    p.toggle_play();
                }
            }
            // Arrow Left / Right: step frames
            if i.key_pressed(egui::Key::ArrowLeft) {
                if let Some(ref mut p) = self.active_player {
                    if i.modifiers.shift { p.step_10_backward(); } else { p.step_backward(); }
                }
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                if let Some(ref mut p) = self.active_player {
                    if i.modifiers.shift { p.step_10_forward(); } else { p.step_forward(); }
                }
            }
            // Escape: close player
            if i.key_pressed(egui::Key::Escape) {
                if self.is_player_mode {
                    self.is_player_mode = false;
                    self.active_player = None;
                }
            }
            // Tab: toggle player view
            if i.key_pressed(egui::Key::Tab) {
                if let Some(v) = self.selected_video().cloned() {
                    if self.is_player_mode {
                        self.is_player_mode = false;
                    } else {
                        self.active_player = Some(VideoPlayer::new(v, ctx.clone()));
                        self.is_player_mode = true;
                    }
                }
            }
            // P: Pick, X: Reject, U: Unflag
            if i.key_pressed(egui::Key::P) && !i.modifiers.command && !i.modifiers.ctrl {
                if let Some(id) = self.selected_ids.iter().next().copied() {
                    self.update_video_flag(id, PickFlag::Picked);
                }
            }
            if i.key_pressed(egui::Key::X) && !i.modifiers.command && !i.modifiers.ctrl {
                if let Some(id) = self.selected_ids.iter().next().copied() {
                    self.update_video_flag(id, PickFlag::Rejected);
                }
            }
            if i.key_pressed(egui::Key::U) && !i.modifiers.command && !i.modifiers.ctrl {
                if let Some(id) = self.selected_ids.iter().next().copied() {
                    self.update_video_flag(id, PickFlag::Unflagged);
                }
            }
            // 1-5: Ratings
            for star in 1..=5 {
                let key = match star {
                    1 => egui::Key::Num1,
                    2 => egui::Key::Num2,
                    3 => egui::Key::Num3,
                    4 => egui::Key::Num4,
                    5 => egui::Key::Num5,
                    _ => continue,
                };
                if i.key_pressed(key) && !i.modifiers.command && !i.modifiers.ctrl {
                    if let Some(id) = self.selected_ids.iter().next().copied() {
                        self.update_video_rating(id, star);
                    }
                }
            }
        });
    }

    fn update_video_rating(&mut self, video_id: i64, rating: u8) {
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.conn() {
                let _ = update_rating(&conn, video_id, rating);
                if let Some(v) = self.videos.iter_mut().find(|v| v.id == video_id) {
                    v.rating = rating;
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

        // Poll importer messages
        let mut import_finished = false;
        if let Some(ref rx) = self.import_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ImportMessage::Progress(prog) => {
                        self.ingest_dialog.progress = Some(prog);
                    }
                    ImportMessage::Complete { .. } | ImportMessage::Cancelled { .. } => {
                        import_finished = true;
                    }
                }
            }
        }
        if import_finished {
            self.ingest_dialog.is_running = false;
            self.ingest_dialog.is_open = false;
            self.import_rx = None;
            self.import_cancel_flag = None;
            self.refresh_catalog();
        }

        self.handle_drag_and_drop(ctx);
        self.handle_keyboard_shortcuts(ctx);

        let catalog_name = self.current_catalog_path.as_ref()
            .and_then(|p| p.file_stem())
            .map_or("No Catalog", |s| s.to_str().unwrap_or(""));

        // TOP PANEL: Header & Search & Controls
        let mut top_action = None;
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            let act = render_top_bar(
                ui,
                catalog_name,
                &self.config.recent_catalogs,
                &mut self.filter,
                &mut self.card_width,
                self.is_player_mode,
                &mut self.show_inspector,
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
                TopBarAction::TogglePlayerView => {
                    if self.is_player_mode {
                        self.is_player_mode = false;
                    } else if let Some(v) = self.selected_video().cloned() {
                        self.active_player = Some(VideoPlayer::new(v, ctx.clone()));
                        self.is_player_mode = true;
                    }
                }
                TopBarAction::ToggleInspector => {
                    self.show_inspector = !self.show_inspector;
                }
            }
        }

        // LEFT PANEL: Navigator & Smart Collections
        let mut nav_action = None;
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

        // RIGHT PANEL: Metadata Inspector (if open and not full player)
        let mut inspector_action = None;
        if self.show_inspector && !self.is_player_mode {
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
                    InspectorAction::DeleteVideo => {
                        if let Some(v) = self.selected_video().cloned() {
                            self.delete_dialog.video_id = v.id;
                            self.delete_dialog.file_path = v.file_path;
                            self.delete_dialog.file_name = v.file_name;
                            self.delete_dialog.is_open = true;
                        }
                    }
                }
            }
        }

        // CENTRAL PANEL: Video Grid or Integrated Player
        let mut grid_action = None;
        let mut player_action = None;

        CentralPanel::default().show(ctx, |ui| {
            if self.is_player_mode {
                if let Some(ref mut player) = self.active_player {
                    if let Some(p_act) = render_player_view(ui, player) {
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
                TransportAction::ClosePlayer => {
                    self.is_player_mode = false;
                    self.active_player = None;
                }
                TransportAction::PlayExternal => {
                    if let Some(ref p) = self.active_player {
                        let _ = open::that(&p.clip.file_path);
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
                    self.sync_inspector_buffers();
                }
                GridAction::ToggleSelect(id) => {
                    if self.selected_ids.contains(&id) {
                        self.selected_ids.remove(&id);
                    } else {
                        self.selected_ids.insert(id);
                    }
                    self.sync_inspector_buffers();
                }
                GridAction::OpenPlayer(record) => {
                    self.active_player = Some(VideoPlayer::new(record, ctx.clone()));
                    self.is_player_mode = true;
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

        if let Some(ing_act) = render_ingest_dialog(ctx, &mut self.ingest_dialog) {
            match ing_act {
                IngestDialogAction::StartIngest(paths) => {
                    if let Some(ref db_path) = self.current_catalog_path {
                        let (tx, rx) = unbounded();
                        let cancel_flag = Arc::new(AtomicBool::new(false));
                        self.import_rx = Some(rx);
                        self.import_cancel_flag = Some(cancel_flag.clone());
                        self.ingest_dialog.is_running = true;

                        Importer::start(
                            paths,
                            db_path.clone(),
                            tx,
                            cancel_flag,
                            ctx.clone(),
                        );
                    }
                }
                IngestDialogAction::Cancel => {
                    if let Some(ref cancel_flag) = self.import_cancel_flag {
                        cancel_flag.store(true, Ordering::Relaxed);
                    }
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
    }
}
