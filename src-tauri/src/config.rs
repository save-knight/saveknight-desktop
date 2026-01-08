use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_url: String,
    pub device_id: Option<String>,
    pub auto_scan: bool,
    pub scan_interval_minutes: u32,
    pub enabled_games: Vec<String>,
    pub custom_paths: Vec<CustomPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPath {
    pub game_name: String,
    pub path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: "https://saveknight.com".to_string(),
            device_id: None,
            auto_scan: true,
            scan_interval_minutes: 60,
            enabled_games: Vec::new(),
            custom_paths: Vec::new(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("SaveKnight");
        fs::create_dir_all(&path).ok();
        path.push("config.toml");
        path
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}
