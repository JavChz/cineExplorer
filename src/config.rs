use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub last_catalog_path: Option<PathBuf>,
    pub recent_catalogs: Vec<PathBuf>,
    pub card_width: f32,
    pub volume: f32,
    pub external_player_cmd: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            last_catalog_path: None,
            recent_catalogs: Vec::new(),
            card_width: 260.0,
            volume: 1.0,
            external_player_cmd: None,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Some(app_data) = std::env::var_os("APPDATA") {
            let p = PathBuf::from(app_data).join("cineExplorer");
            let _ = fs::create_dir_all(&p);
            p.join("config.json")
        } else {
            let p = PathBuf::from(".cineExplorer");
            let _ = fs::create_dir_all(&p);
            p.join("config.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(bytes) = fs::read(&path) {
            if let Ok(cfg) = serde_json::from_slice::<AppConfig>(&bytes) {
                return cfg;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    pub fn add_recent_catalog(&mut self, path: PathBuf) {
        self.recent_catalogs.retain(|p| p != &path);
        self.recent_catalogs.insert(0, path.clone());
        if self.recent_catalogs.len() > 10 {
            self.recent_catalogs.truncate(10);
        }
        self.last_catalog_path = Some(path);
        self.save();
    }
}
