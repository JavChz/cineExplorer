use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use crossbeam_channel::{unbounded, Receiver, Sender};
use egui::{ColorImage, Context, TextureHandle, TextureOptions};

use crate::db::{load_thumb, store_thumb, Database};
use crate::media::thumbnailer::extract_thumbnail;

struct ThumbRequest {
    video_id: i64,
    file_path: String,
    duration_seconds: f32,
    log_profile: Option<String>,
}

struct ThumbResult {
    video_id: i64,
    image: ColorImage,
}

pub struct ThumbnailManager {
    textures: HashMap<i64, TextureHandle>,
    in_flight: HashSet<i64>,
    tx_req: Sender<ThumbRequest>,
    rx_res: Receiver<ThumbResult>,
    db_path: PathBuf,
}

impl ThumbnailManager {
    pub fn new(db_path: PathBuf, ctx: Context) -> Self {
        let (tx_req, rx_req) = unbounded::<ThumbRequest>();
        let (tx_res, rx_res) = unbounded::<ThumbResult>();

        // Background thumbnail generation worker
        let worker_db_path = db_path.clone();
        std::thread::spawn(move || {
            let db = Database::open(&worker_db_path).ok();

            while let Ok(req) = rx_req.recv() {
                // 1. Check SQLite cache first
                let cached_jpeg = if let Some(ref d) = db {
                    if let Ok(conn) = d.conn() {
                        load_thumb(&conn, req.video_id)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let jpeg_bytes = match cached_jpeg {
                    Some(bytes) => bytes,
                    None => {
                        // 2. Extract with ffmpeg
                        match extract_thumbnail(&req.file_path, req.duration_seconds, req.log_profile.as_deref()) {
                            Ok(bytes) => {
                                if let Some(ref d) = db {
                                    if let Ok(conn) = d.conn() {
                                        let _ = store_thumb(&conn, req.video_id, &bytes);
                                    }
                                }
                                bytes
                            }
                            Err(_) => continue,
                        }
                    }
                };

                // Decode JPEG to RGBA ColorImage
                if let Ok(img) = image::load_from_memory(&jpeg_bytes) {
                    let rgba = img.to_rgba8();
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let color_img = ColorImage::from_rgba_unmultiplied(size, &rgba.into_raw());

                    let _ = tx_res.send(ThumbResult {
                        video_id: req.video_id,
                        image: color_img,
                    });
                    ctx.request_repaint();
                }
            }
        });

        Self {
            textures: HashMap::new(),
            in_flight: HashSet::new(),
            tx_req,
            rx_res,
            db_path,
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        while let Ok(res) = self.rx_res.try_recv() {
            self.in_flight.remove(&res.video_id);
            let handle = ctx.load_texture(
                format!("thumb_{}", res.video_id),
                res.image,
                TextureOptions::LINEAR,
            );
            self.textures.insert(res.video_id, handle);
        }
    }

    pub fn get_texture(&self, video_id: i64) -> Option<&TextureHandle> {
        self.textures.get(&video_id)
    }

    pub fn request(&mut self, video_id: i64, file_path: &str, duration_seconds: f32, log_profile: Option<&str>) {
        if self.textures.contains_key(&video_id) || self.in_flight.contains(&video_id) {
            return;
        }
        self.in_flight.insert(video_id);
        let _ = self.tx_req.send(ThumbRequest {
            video_id,
            file_path: file_path.to_string(),
            duration_seconds,
            log_profile: log_profile.map(|s| s.to_string()),
        });
    }

    pub fn invalidate(&mut self, video_id: i64) {
        self.textures.remove(&video_id);
        self.in_flight.remove(&video_id);
        if let Ok(conn) = rusqlite::Connection::open(&self.db_path) {
            let _ = conn.execute("DELETE FROM thumbs WHERE video_id = ?1", rusqlite::params![video_id]);
        }
    }

    pub fn clear(&mut self) {
        self.textures.clear();
        self.in_flight.clear();
    }
}
