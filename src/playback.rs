use std::io::Read;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use egui::{ColorImage, Context, TextureHandle, TextureOptions};

use crate::db::models::VideoRecord;
use crate::media::color_lut::LogProfile;
use crate::media::player_source::VideoPlayerSource;

pub struct StreamFrame {
    pub pts: f32,
    pub image: ColorImage,
}

struct StreamController {
    stop_flag: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
    frame_rx: Receiver<StreamFrame>,
    pending_frame: Option<StreamFrame>,
    stream_started: bool,
    eof_reached: Arc<AtomicBool>,
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
    pub is_gpu_accelerated: bool,
    pub log_profile: LogProfile,
    pub apply_rec709_lut: bool,
    source: VideoPlayerSource,
    ctx: Context,
    last_tick: Instant,
    stream: Option<StreamController>,
    seek_tx: Sender<(f32, LogProfile, bool)>,
    seek_rx: Receiver<(f32, ColorImage)>,
    last_seek_requested: f32,
}

impl VideoPlayer {
    pub fn new(clip: VideoRecord, ctx: Context) -> Self {
        let (seek_tx, seek_req_rx) = unbounded::<(f32, LogProfile, bool)>();
        let (seek_res_tx, seek_rx) = unbounded::<(f32, ColorImage)>();

        let log_profile = LogProfile::from_str(&clip.log_profile);
        let apply_rec709_lut = log_profile.is_log();

        let source = VideoPlayerSource::new(
            &clip.file_path,
            clip.duration_seconds,
            clip.fps,
            clip.width,
            clip.height,
            &clip.codec,
            clip.bit_depth,
            &clip.pixel_format,
            log_profile,
            apply_rec709_lut,
        );

        let is_hw = source.is_hw_accel_supported();
        let mut source_seek = source.clone();
        let ctx_seek = ctx.clone();

        // Worker thread for responsive single-frame scrubbing and seeking
        std::thread::spawn(move || {
            while let Ok(mut req) = seek_req_rx.recv() {
                // Drain any newer requests to only decode the latest scrub target
                while let Ok(newer) = seek_req_rx.try_recv() {
                    req = newer;
                }

                let (target, prof, apply_lut) = req;
                source_seek.set_log_config(prof, apply_lut);

                if let Ok(img) = source_seek.extract_frame_at(target) {
                    let _ = seek_res_tx.send((target, img));
                    ctx_seek.request_repaint();
                }
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
            is_gpu_accelerated: is_hw,
            log_profile,
            apply_rec709_lut,
            source,
            ctx,
            last_tick: Instant::now(),
            stream: None,
            seek_tx,
            seek_rx,
            last_seek_requested: -1.0,
        };

        // For BRAW files, immediately load the native high-res OS frame
        if player.clip.is_braw() {
            player.request_single_frame(0.0);
        } else {
            // Immediately start playback of the clip
            player.play();
        }
        player
    }

    pub fn toggle_rec709_lut(&mut self) {
        self.apply_rec709_lut = !self.apply_rec709_lut;
        self.source.set_log_config(self.log_profile, self.apply_rec709_lut);
        if self.is_playing {
            self.start_stream(self.playhead);
        } else {
            self.last_seek_requested = -1.0;
            self.request_single_frame(self.playhead);
        }
    }

    pub fn set_log_profile(&mut self, profile: LogProfile) {
        self.log_profile = profile;
        if profile.is_log() {
            self.apply_rec709_lut = true;
        }
        self.source.set_log_config(self.log_profile, self.apply_rec709_lut);
        if self.is_playing {
            self.start_stream(self.playhead);
        } else {
            self.last_seek_requested = -1.0;
            self.request_single_frame(self.playhead);
        }
    }

    pub fn play(&mut self) {
        if self.clip.is_braw() {
            // Launch in the official Blackmagic RAW Player or external player
            let _ = open::that(&self.clip.file_path);
            return;
        }

        let end_sec = self.out_point.unwrap_or(self.clip.duration_seconds).max(0.1);
        let start_sec = self.in_point.unwrap_or(0.0);

        if self.playhead >= end_sec - 0.05 {
            self.playhead = start_sec;
        }

        self.is_playing = true;
        self.last_tick = Instant::now();
        let start = self.playhead;
        self.start_stream(start);
    }

    pub fn pause(&mut self) {
        if self.is_playing {
            self.is_playing = false;
            self.stop_stream();
            self.request_single_frame(self.playhead);
        }
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

        if self.is_playing {
            // Reposition streaming decoder smoothly
            self.start_stream(self.playhead);
        } else {
            self.request_single_frame(self.playhead);
        }
    }

    pub fn shuttle_forward(&mut self) {
        if !self.is_playing {
            self.playback_rate = 1.0;
            self.play();
        } else if self.playback_rate < 0.0 {
            self.playback_rate = 1.0;
        } else if (self.playback_rate - 1.0).abs() < 0.1 {
            self.playback_rate = 2.0;
        } else if (self.playback_rate - 2.0).abs() < 0.1 {
            self.playback_rate = 4.0;
        } else if (self.playback_rate - 4.0).abs() < 0.1 {
            self.playback_rate = 8.0;
        }
    }

    pub fn shuttle_reverse(&mut self) {
        if !self.is_playing {
            self.playback_rate = -1.0;
            self.play();
        } else if self.playback_rate > 0.0 {
            self.playback_rate = -1.0;
        } else if (self.playback_rate - (-1.0)).abs() < 0.1 {
            self.playback_rate = -2.0;
        } else if (self.playback_rate - (-2.0)).abs() < 0.1 {
            self.playback_rate = -4.0;
        }
    }

    pub fn shuttle_stop(&mut self) {
        self.pause();
        self.playback_rate = 1.0;
    }

    pub fn goto_start(&mut self) {
        let start = self.in_point.unwrap_or(0.0);
        self.seek(start);
    }

    pub fn goto_end(&mut self) {
        let end = self.out_point.unwrap_or(self.clip.duration_seconds);
        self.seek(end);
    }

    pub fn goto_in(&mut self) {
        if let Some(inp) = self.in_point {
            self.seek(inp);
        }
    }

    pub fn goto_out(&mut self) {
        if let Some(outp) = self.out_point {
            self.seek(outp);
        }
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

    fn request_single_frame(&mut self, seconds: f32) {
        if (self.last_seek_requested - seconds).abs() > 0.005 {
            self.last_seek_requested = seconds;
            let _ = self.seek_tx.send((seconds, self.log_profile, self.apply_rec709_lut));
        }
    }

    fn stop_stream(&mut self) {
        if let Some(ctrl) = self.stream.take() {
            ctrl.stop_flag.store(true, Ordering::Relaxed);
            if let Ok(mut lock) = ctrl.child.lock() {
                if let Some(mut child) = lock.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
    }

    fn start_stream(&mut self, start_time: f32) {
        self.stop_stream();

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_thread = stop_flag.clone();
        let eof_reached = Arc::new(AtomicBool::new(false));
        let eof_reached_thread = eof_reached.clone();

        let child_cell = Arc::new(Mutex::new(None));
        let child_cell_thread = child_cell.clone();

        // 24-frame ring buffer (~0.8 sec read-ahead). Provides backpressure to FFmpeg.
        let (frame_tx, frame_rx) = bounded::<StreamFrame>(24);

        let source = self.source.clone();
        let ctx = self.ctx.clone();
        let fps = source.fps;
        let rw = source.render_width;
        let rh = source.render_height;

        std::thread::spawn(move || {
            let (child_res, is_hw) = source.spawn_stream(start_time);
            let mut child = match child_res {
                Ok(c) => c,
                Err(_) => return,
            };

            let mut stdout = match child.stdout.take() {
                Some(s) => s,
                None => {
                    let _ = child.kill();
                    return;
                }
            };

            if let Ok(mut lock) = child_cell_thread.lock() {
                *lock = Some(child);
            }

            let frame_bytes = rw * rh * 4;
            let mut frame_buf = vec![0u8; frame_bytes];
            let mut frame_idx: usize = 0;
            let dt = 1.0 / fps.max(1.0);

            while !stop_flag_thread.load(Ordering::Relaxed) {
                match stdout.read_exact(&mut frame_buf) {
                    Ok(_) => {
                        let pts = start_time + (frame_idx as f32 * dt);
                        frame_idx += 1;
                        let img = ColorImage::from_rgba_unmultiplied([rw, rh], &frame_buf);

                        // If queue is full, this blocks naturally, pausing ffmpeg decoding
                        if frame_tx.send(StreamFrame { pts, image: img }).is_err() {
                            break;
                        }
                        ctx.request_repaint();
                    }
                    Err(_) => {
                        // If hardware stream failed on frame 0, fall back to software
                        if frame_idx == 0 && is_hw && !stop_flag_thread.load(Ordering::Relaxed) {
                            if let Ok(mut lock) = child_cell_thread.lock() {
                                if let Some(mut old_child) = lock.take() {
                                    let _ = old_child.kill();
                                    let _ = old_child.wait();
                                }
                            }
                            if let Ok(mut sw_child) = source.spawn_stream_sw(start_time) {
                                if let Some(sw_stdout) = sw_child.stdout.take() {
                                    stdout = sw_stdout;
                                    if let Ok(mut lock) = child_cell_thread.lock() {
                                        *lock = Some(sw_child);
                                    }
                                    continue;
                                }
                            }
                        }
                        // EOF reached
                        eof_reached_thread.store(true, Ordering::Relaxed);
                        ctx.request_repaint();
                        break;
                    }
                }
            }

            if let Ok(mut lock) = child_cell_thread.lock() {
                if let Some(mut child) = lock.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        });

        self.stream = Some(StreamController {
            stop_flag,
            child: child_cell,
            frame_rx,
            pending_frame: None,
            stream_started: false,
            eof_reached,
        });
    }

    pub fn update(&mut self, ctx: &Context) {
        // 1. Check for single-frame seek results when paused
        if !self.is_playing {
            while let Ok((_t, img)) = self.seek_rx.try_recv() {
                if let Some(ref mut tex) = self.current_texture {
                    tex.set(img, TextureOptions::LINEAR);
                } else {
                    let handle = ctx.load_texture("player_current_frame", img, TextureOptions::LINEAR);
                    self.current_texture = Some(handle);
                }
            }
            return;
        }

        // 2. Process decoded frames with accurate PTS sync and backpressure
        let dt_frame = 1.0 / self.clip.fps.max(1.0);
        let mut new_texture_image = None;

        if let Some(ref mut ctrl) = self.stream {
            loop {
                // Peek at next frame (either cached in pending_frame or received from channel)
                if ctrl.pending_frame.is_none() {
                    if let Ok(f) = ctrl.frame_rx.try_recv() {
                        ctrl.pending_frame = Some(f);
                    }
                }

                match ctrl.pending_frame.take() {
                    Some(frame) => {
                        // First frame of stream: present immediately and anchor the clock
                        if !ctrl.stream_started {
                            ctrl.stream_started = true;
                            self.playhead = frame.pts;
                            self.last_tick = Instant::now();
                            new_texture_image = Some(frame.image);
                            continue;
                        }

                        // Frame matches or is behind the current playhead
                        if frame.pts <= self.playhead + (dt_frame * 0.4) {
                            new_texture_image = Some(frame.image);
                            continue;
                        } else {
                            // Frame belongs in future: preserve in pending_frame for backpressure
                            ctrl.pending_frame = Some(frame);
                            break;
                        }
                    }
                    None => break,
                }
            }
        }

        if let Some(img) = new_texture_image {
            if let Some(ref mut tex) = self.current_texture {
                tex.set(img, TextureOptions::LINEAR);
            } else {
                let handle = ctx.load_texture("player_current_frame", img, TextureOptions::LINEAR);
                self.current_texture = Some(handle);
            }
        }

        // 3. Continuous playback clock
        let is_stream_ready = self.stream.as_ref().is_some_and(|s| s.stream_started);
        if is_stream_ready {
            let now = Instant::now();
            let dt = now.duration_since(self.last_tick).as_secs_f32();
            self.last_tick = now;

            let end_sec = self.out_point.unwrap_or(self.clip.duration_seconds).max(0.1);
            let start_sec = self.in_point.unwrap_or(0.0);

            self.playhead += dt * self.playback_rate;

            let is_eof = self.stream.as_ref().is_some_and(|s| {
                s.eof_reached.load(Ordering::Relaxed) && s.pending_frame.is_none() && s.frame_rx.is_empty()
            });

            if self.playhead >= end_sec || is_eof {
                if self.looping {
                    self.playhead = start_sec;
                    self.start_stream(start_sec);
                } else {
                    self.playhead = end_sec;
                    self.pause();
                    return;
                }
            }
        }

        // Keep UI animating smoothly at native monitor refresh rate
        ctx.request_repaint();
    }

    pub fn current_timecode(&self) -> String {
        let t = self.playhead.max(0.0);
        let h = (t / 3600.0) as u32;
        let m = ((t % 3600.0) / 60.0) as u32;
        let s = (t % 60.0) as u32;
        let f = ((t.fract()) * self.clip.fps.max(1.0)) as u32;
        format!("{:02}:{:02}:{:02}:{:02}", h, m, s, f)
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.stop_stream();
    }
}
