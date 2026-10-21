use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use crossbeam_channel::{unbounded, Receiver, Sender};
use egui::{ColorImage, Context, TextureHandle, TextureOptions};

use crate::db::models::VideoRecord;
use crate::media::player_source::VideoPlayerSource;

struct FrameRequest {
    seconds: f32,
    generation: usize,
}

struct FrameResponse {
    #[allow(dead_code)]
    seconds: f32,
    generation: usize,
    image: ColorImage,
}

pub struct VideoPlayer {
    pub clip: VideoRecord,
    pub playhead: f32,
    pub is_playing: bool,
    pub looping: bool,
    pub playback_rate: f32,
    pub in_point: Option<f32>,
    pub out_point: Option<f32>,
    pub current_texture: Option<TextureHandle>,
    last_tick: Instant,
    req_tx: Sender<FrameRequest>,
    res_rx: Receiver<FrameResponse>,
    generation: usize,
    pending_request: bool,
    last_requested_seconds: f32,
    #[allow(dead_code)]
    is_fetching: Arc<AtomicBool>,
}

impl VideoPlayer {
    pub fn new(clip: VideoRecord, ctx: Context) -> Self {
        let (req_tx, req_rx) = unbounded::<FrameRequest>();
        let (res_tx, res_rx) = unbounded::<FrameResponse>();

        let source = VideoPlayerSource::new(
            &clip.file_path,
            clip.duration_seconds,
            clip.fps,
            clip.width,
            clip.height,
        );

        let is_fetching = Arc::new(AtomicBool::new(false));
        let is_fetching_clone = is_fetching.clone();

        std::thread::spawn(move || {
            while let Ok(mut req) = req_rx.recv() {
                // Drain any newer requests in queue
                while let Ok(newer) = req_rx.try_recv() {
                    req = newer;
                }

                is_fetching_clone.store(true, Ordering::Relaxed);
                if let Ok(img) = source.extract_frame_at(req.seconds) {
                    let _ = res_tx.send(FrameResponse {
                        seconds: req.seconds,
                        generation: req.generation,
                        image: img,
                    });
                    ctx.request_repaint();
                }
                is_fetching_clone.store(false, Ordering::Relaxed);
            }
        });

        let mut player = Self {
            clip,
            playhead: 0.0,
            is_playing: false,
            looping: true,
            playback_rate: 1.0,
            in_point: None,
            out_point: None,
            current_texture: None,
            last_tick: Instant::now(),
            req_tx,
            res_rx,
            generation: 1,
            pending_request: false,
            last_requested_seconds: -1.0,
            is_fetching,
        };

        player.request_frame(0.0);
        player
    }

    pub fn play(&mut self) {
        self.is_playing = true;
        self.last_tick = Instant::now();
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    pub fn toggle_play(&mut self) {
        if self.is_playing {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn seek(&mut self, seconds: f32) {
        let max_sec = self.clip.duration_seconds.max(0.0);
        self.playhead = seconds.clamp(0.0, max_sec);
        self.request_frame(self.playhead);
    }

    pub fn step_forward(&mut self) {
        let dt = 1.0 / self.clip.fps.max(1.0);
        self.seek(self.playhead + dt);
    }

    pub fn step_backward(&mut self) {
        let dt = 1.0 / self.clip.fps.max(1.0);
        self.seek(self.playhead - dt);
    }

    pub fn step_10_forward(&mut self) {
        let dt = 10.0 / self.clip.fps.max(1.0);
        self.seek(self.playhead + dt);
    }

    pub fn step_10_backward(&mut self) {
        let dt = 10.0 / self.clip.fps.max(1.0);
        self.seek(self.playhead - dt);
    }

    fn request_frame(&mut self, seconds: f32) {
        if (self.last_requested_seconds - seconds).abs() < 0.01 && self.pending_request {
            return;
        }
        self.last_requested_seconds = seconds;
        self.generation += 1;
        self.pending_request = true;
        let _ = self.req_tx.send(FrameRequest {
            seconds,
            generation: self.generation,
        });
    }

    pub fn update(&mut self, ctx: &Context) {
        // Receive decoded frame from worker
        while let Ok(res) = self.res_rx.try_recv() {
            if res.generation >= self.generation.saturating_sub(1) {
                let handle = ctx.load_texture(
                    "player_current_frame",
                    res.image,
                    TextureOptions::LINEAR,
                );
                self.current_texture = Some(handle);
                self.pending_request = false;
            }
        }

        // Advance playhead if playing
        if self.is_playing {
            let now = Instant::now();
            let dt = now.duration_since(self.last_tick).as_secs_f32();
            self.last_tick = now;

            let end_sec = self.out_point.unwrap_or(self.clip.duration_seconds).max(0.1);
            let start_sec = self.in_point.unwrap_or(0.0);

            self.playhead += dt * self.playback_rate;

            if self.playhead >= end_sec {
                if self.looping {
                    self.playhead = start_sec;
                } else {
                    self.playhead = end_sec;
                    self.is_playing = false;
                }
            }

            self.request_frame(self.playhead);
            ctx.request_repaint();
        }
    }

    pub fn current_timecode(&self) -> String {
        let t = self.playhead.max(0.0);
        let h = (t / 3600.0) as u32;
        let m = ((t % 3600.0) / 60.0) as u32;
        let s = (t % 60.0) as u32;
        let f = ((t.fract()) * self.clip.fps.max(1.0)) as u32;

        if h > 0 {
            format!("{:02}:{:02}:{:02}:{:02}", h, m, s, f)
        } else {
            format!("{:02}:{:02}:{:02}", m, s, f)
        }
    }
}
