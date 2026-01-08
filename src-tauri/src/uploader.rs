use crate::scanner::DetectedGame;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use zip::write::FileOptions;
use zip::ZipWriter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResult {
    pub game_name: String,
    pub success: bool,
    pub message: String,
    pub upload_id: Option<String>,
    pub version_number: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UploadResponse {
    success: bool,
    save_version: Option<SaveVersionResponse>,
    upload_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SaveVersionResponse {
    id: String,
    version_number: i32,
}

pub struct Uploader {
    api_url: String,
    device_token: String,
}

impl Uploader {
    pub fn new(api_url: &str, device_token: &str) -> Self {
        Self {
            api_url: api_url.to_string(),
            device_token: device_token.to_string(),
        }
    }

    pub async fn upload_game(
        &self,
        game: &DetectedGame,
        game_profile_id: &str,
    ) -> Result<UploadResult, Box<dyn std::error::Error + Send + Sync>> {
        let temp_dir = std::env::temp_dir();
        let zip_path = temp_dir.join(format!("{}.zip", sanitize_filename(&game.name)));

        self.create_save_zip(game, &zip_path)?;

        let checksum = self.calculate_checksum(&zip_path)?;

        let file_content = fs::read(&zip_path)?;
        let file_size = file_content.len();

        let form = Form::new()
            .text("slotName", format!("{} Auto-Backup", game.name))
            .text("localPath", game.paths.first().map(|p| p.resolved_path.clone()).unwrap_or_default())
            .text("checksum", checksum)
            .part(
                "saveFile",
                Part::bytes(file_content)
                    .file_name(format!("{}.zip", sanitize_filename(&game.name)))
                    .mime_str("application/zip")?,
            );

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/devices/upload/{}", self.api_url, game_profile_id))
            .header("Authorization", format!("Bearer {}", self.device_token))
            .multipart(form)
            .send()
            .await?;

        fs::remove_file(&zip_path).ok();

        if response.status().is_success() {
            let result: UploadResponse = response.json().await?;
            Ok(UploadResult {
                game_name: game.name.clone(),
                success: true,
                message: format!(
                    "Uploaded {} bytes successfully",
                    file_size
                ),
                upload_id: result.upload_id,
                version_number: result.save_version.map(|v| v.version_number),
            })
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Ok(UploadResult {
                game_name: game.name.clone(),
                success: false,
                message: error_text,
                upload_id: None,
                version_number: None,
            })
        }
    }

    fn create_save_zip(
        &self,
        game: &DetectedGame,
        output_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let file = File::create(output_path)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        for detected_path in &game.paths {
            if !detected_path.exists {
                continue;
            }

            for entry in glob::glob(&detected_path.resolved_path).into_iter().flatten().flatten() {
                if entry.is_file() {
                    let relative_name = entry
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "file".to_string());
                    
                    zip.start_file(&relative_name, options)?;
                    let mut file = File::open(&entry)?;
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer)?;
                    zip.write_all(&buffer)?;
                } else if entry.is_dir() {
                    self.add_dir_to_zip(&mut zip, &entry, &entry, options)?;
                }
            }
        }

        zip.finish()?;
        Ok(())
    }

    fn add_dir_to_zip<W: Write + std::io::Seek>(
        &self,
        zip: &mut ZipWriter<W>,
        base_path: &Path,
        current_path: &Path,
        options: FileOptions,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for entry in fs::read_dir(current_path)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path.strip_prefix(base_path).unwrap_or(&path);
            let name = relative.to_string_lossy().replace('\\', "/");

            if path.is_file() {
                zip.start_file(&name, options)?;
                let mut file = File::open(&path)?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                zip.write_all(&buffer)?;
            } else if path.is_dir() {
                zip.add_directory(&name, options)?;
                self.add_dir_to_zip(zip, base_path, &path, options)?;
            }
        }
        Ok(())
    }

    fn calculate_checksum(&self, path: &Path) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(hex::encode(hasher.finalize()))
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}
