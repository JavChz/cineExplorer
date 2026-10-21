#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
pub mod config;
pub mod db;
pub mod export;
pub mod importer;
pub mod ingester;
pub mod media;
pub mod playback;
pub mod thumbnails;
pub mod ui;

use app::CineExplorerApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("cineExplorer · Professional Video Catalog & Browser")
            .with_inner_size([1400.0, 860.0])
            .with_min_inner_size([980.0, 620.0])
            .with_drag_and_drop(true),
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration {
            power_preference: eframe::wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        },
        ..Default::default()
    };

    eframe::run_native(
        "cineExplorer",
        native_options,
        Box::new(|cc| Ok(Box::new(CineExplorerApp::new(cc)))),
    )
}
