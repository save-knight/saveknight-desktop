use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const LUDUSAVI_MANIFEST_URL: &str = "https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/master/data/manifest.yaml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEntry {
    pub name: String,
    pub paths: Vec<SavePath>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePath {
    pub path: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestGame {
    #[serde(default)]
    pub files: HashMap<String, ManifestFile>,
    #[serde(default)]
    pub registry: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ManifestFile {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub when: Vec<serde_json::Value>,
}

pub struct LudusaviManifest {
    games: HashMap<String, ManifestGame>,
}

impl LudusaviManifest {
    pub fn cache_path() -> PathBuf {
        let mut path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("SaveKnight");
        fs::create_dir_all(&path).ok();
        path.push("manifest.yaml");
        path
    }

    pub async fn fetch_or_load() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cache_path = Self::cache_path();
        
        let should_update = if cache_path.exists() {
            if let Ok(metadata) = fs::metadata(&cache_path) {
                if let Ok(modified) = metadata.modified() {
                    let age = std::time::SystemTime::now()
                        .duration_since(modified)
                        .unwrap_or_default();
                    age.as_secs() > 7 * 24 * 60 * 60
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };

        if should_update {
            match Self::fetch_manifest().await {
                Ok(content) => {
                    fs::write(&cache_path, &content).ok();
                    return Self::parse_manifest(&content);
                }
                Err(e) => {
                    log::warn!("Failed to fetch manifest: {}", e);
                }
            }
        }

        if cache_path.exists() {
            let content = fs::read_to_string(&cache_path)?;
            Self::parse_manifest(&content)
        } else {
            Ok(Self {
                games: HashMap::new(),
            })
        }
    }

    async fn fetch_manifest() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let response = client.get(LUDUSAVI_MANIFEST_URL).send().await?;
        let content = response.text().await?;
        Ok(content)
    }

    fn parse_manifest(content: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let games: HashMap<String, ManifestGame> = serde_yaml::from_str(content)
            .unwrap_or_else(|_| HashMap::new());
        Ok(Self { games })
    }

    pub fn get_game_paths(&self, game_name: &str) -> Vec<SavePath> {
        let mut paths = Vec::new();
        
        if let Some(game) = self.games.get(game_name) {
            for (path_pattern, file_info) in &game.files {
                let expanded = Self::expand_path(path_pattern);
                paths.push(SavePath {
                    path: expanded,
                    tags: file_info.tags.clone(),
                });
            }
        }
        
        paths
    }

    pub fn list_games(&self) -> Vec<String> {
        self.games.keys().cloned().collect()
    }

    pub fn search_games(&self, query: &str) -> Vec<String> {
        let query_lower = query.to_lowercase();
        self.games
            .keys()
            .filter(|name| name.to_lowercase().contains(&query_lower))
            .cloned()
            .collect()
    }

    fn expand_path(path: &str) -> String {
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

        path.replace("<home>", &home)
            .replace("<documents>", &documents)
            .replace("<appData>", &appdata)
            .replace("<localAppData>", &local_appdata)
            .replace("<storeUserId>", "*")
            .replace("<osUserName>", &whoami::username())
    }
}

fn whoami_username() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "user".to_string())
}

mod whoami {
    pub fn username() -> String {
        std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "user".to_string())
    }
}
