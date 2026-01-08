use crate::ludusavi::{LudusaviManifest, SavePath};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedGame {
    pub name: String,
    pub paths: Vec<DetectedSavePath>,
    pub total_size_bytes: u64,
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedSavePath {
    pub pattern: String,
    pub resolved_path: String,
    pub exists: bool,
    pub file_count: u32,
    pub total_size_bytes: u64,
}

pub struct Scanner {
    manifest: LudusaviManifest,
}

impl Scanner {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let manifest = LudusaviManifest::fetch_or_load().await?;
        Ok(Self { manifest })
    }

    pub fn scan_all_games(&self) -> Vec<DetectedGame> {
        let mut detected_games = Vec::new();
        let game_names = self.manifest.list_games();

        for game_name in game_names {
            if let Some(detected) = self.scan_game(&game_name) {
                if detected.paths.iter().any(|p| p.exists && p.file_count > 0) {
                    detected_games.push(detected);
                }
            }
        }

        detected_games.sort_by(|a, b| b.total_size_bytes.cmp(&a.total_size_bytes));
        detected_games
    }

    pub fn scan_game(&self, game_name: &str) -> Option<DetectedGame> {
        let paths = self.manifest.get_game_paths(game_name);
        if paths.is_empty() {
            return None;
        }

        let mut detected_paths = Vec::new();
        let mut total_size: u64 = 0;
        let mut latest_modified: Option<std::time::SystemTime> = None;

        for save_path in paths {
            let detected = self.scan_path(&save_path);
            total_size += detected.total_size_bytes;

            if detected.exists {
                if let Ok(metadata) = fs::metadata(&detected.resolved_path) {
                    if let Ok(modified) = metadata.modified() {
                        latest_modified = Some(match latest_modified {
                            Some(current) => current.max(modified),
                            None => modified,
                        });
                    }
                }
            }

            detected_paths.push(detected);
        }

        let last_modified = latest_modified.map(|t| {
            chrono::DateTime::<chrono::Utc>::from(t)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        });

        Some(DetectedGame {
            name: game_name.to_string(),
            paths: detected_paths,
            total_size_bytes: total_size,
            last_modified,
        })
    }

    fn scan_path(&self, save_path: &SavePath) -> DetectedSavePath {
        let resolved = self.resolve_glob_path(&save_path.path);
        
        let mut file_count = 0u32;
        let mut total_size = 0u64;
        let mut exists = false;

        for entry in glob::glob(&resolved).into_iter().flatten().flatten() {
            exists = true;
            if entry.is_file() {
                file_count += 1;
                if let Ok(metadata) = fs::metadata(&entry) {
                    total_size += metadata.len();
                }
            } else if entry.is_dir() {
                for file_entry in WalkDir::new(&entry).into_iter().filter_map(|e| e.ok()) {
                    if file_entry.file_type().is_file() {
                        file_count += 1;
                        if let Ok(metadata) = file_entry.metadata() {
                            total_size += metadata.len();
                        }
                    }
                }
            }
        }

        DetectedSavePath {
            pattern: save_path.path.clone(),
            resolved_path: resolved,
            exists,
            file_count,
            total_size_bytes: total_size,
        }
    }

    fn resolve_glob_path(&self, path: &str) -> String {
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let documents = dirs::document_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let appdata = dirs::data_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let local_appdata = dirs::data_local_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let username = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "user".to_string());

        path.replace("<home>", &home)
            .replace("<documents>", &documents)
            .replace("<appData>", &appdata)
            .replace("<localAppData>", &local_appdata)
            .replace("<storeUserId>", "*")
            .replace("<osUserName>", &username)
            .replace('/', std::path::MAIN_SEPARATOR_STR)
    }
}
