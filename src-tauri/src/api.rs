use crate::config::Config;
use crate::scanner::{DetectedGame, Scanner};
use crate::uploader::{UploadResult, Uploader};
use crate::AppState;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use tauri::State;

const KEYRING_SERVICE: &str = "saveknight-desktop";
const KEYRING_USER: &str = "device-token";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub is_authenticated: bool,
    pub device_id: Option<String>,
    pub user_email: Option<String>,
    pub plan_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub device_name: String,
    pub machine_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceResponse {
    device_id: String,
    token: String,
    expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeResponse {
    device: DeviceInfo,
    user: UserInfo,
    subscription: SubscriptionInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceInfo {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserInfo {
    id: String,
    email: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubscriptionInfo {
    plan_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProfile {
    pub id: String,
    pub name: String,
    pub platform: String,
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[tauri::command]
pub async fn save_config(state: State<'_, AppState>, new_config: Config) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    *config = new_config.clone();
    config.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    session_cookie: String,
    device_name: String,
) -> Result<AuthStatus, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let api_url = config.api_url.clone();
    drop(config);

    let machine_id = get_machine_id();

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/devices/register", api_url))
        .header("Cookie", format!("connect.sid={}", session_cookie))
        .json(&serde_json::json!({
            "deviceName": device_name,
            "machineId": machine_id,
            "deviceType": "windows"
        }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let error = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("Registration failed: {}", error));
    }

    let device_response: DeviceResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| format!("Keyring error: {}", e))?;
    entry
        .set_password(&device_response.token)
        .map_err(|e| format!("Failed to store token: {}", e))?;

    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.device_id = Some(device_response.device_id.clone());
    config.save().map_err(|e| e.to_string())?;
    drop(config);

    get_auth_status(state).await
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER);
    if let Ok(entry) = entry {
        entry.delete_password().ok();
    }

    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.device_id = None;
    config.save().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_auth_status(state: State<'_, AppState>) -> Result<AuthStatus, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let api_url = config.api_url.clone();
    let device_id = config.device_id.clone();
    drop(config);

    let token = match get_stored_token() {
        Some(t) => t,
        None => {
            return Ok(AuthStatus {
                is_authenticated: false,
                device_id: None,
                user_email: None,
                plan_name: None,
            });
        }
    };

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/devices/me", api_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            let me: MeResponse = resp.json().await.map_err(|e| e.to_string())?;
            Ok(AuthStatus {
                is_authenticated: true,
                device_id,
                user_email: me.user.email,
                plan_name: Some(me.subscription.plan_name),
            })
        }
        _ => Ok(AuthStatus {
            is_authenticated: false,
            device_id: None,
            user_email: None,
            plan_name: None,
        }),
    }
}

#[tauri::command]
pub async fn scan_games(state: State<'_, AppState>) -> Result<Vec<DetectedGame>, String> {
    {
        let mut is_scanning = state.is_scanning.lock().map_err(|e| e.to_string())?;
        if *is_scanning {
            return Err("Scan already in progress".to_string());
        }
        *is_scanning = true;
    }

    let result = async {
        let scanner = Scanner::new().await.map_err(|e| e.to_string())?;
        Ok(scanner.scan_all_games())
    }
    .await;

    {
        let mut is_scanning = state.is_scanning.lock().map_err(|e| e.to_string())?;
        *is_scanning = false;
    }

    result
}

#[tauri::command]
pub async fn get_detected_games(_state: State<'_, AppState>) -> Result<Vec<DetectedGame>, String> {
    let scanner = Scanner::new().await.map_err(|e| e.to_string())?;
    Ok(scanner.scan_all_games())
}

#[tauri::command]
pub async fn upload_saves(
    state: State<'_, AppState>,
    games: Vec<DetectedGame>,
    game_profile_id: String,
) -> Result<Vec<UploadResult>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let api_url = config.api_url.clone();
    drop(config);

    let token = get_stored_token().ok_or("Not authenticated")?;
    let uploader = Uploader::new(&api_url, &token);

    let mut results = Vec::new();
    for game in games {
        match uploader.upload_game(&game, &game_profile_id).await {
            Ok(result) => results.push(result),
            Err(e) => results.push(UploadResult {
                game_name: game.name,
                success: false,
                message: e.to_string(),
                upload_id: None,
                version_number: None,
            }),
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn get_upload_history(_state: State<'_, AppState>) -> Result<Vec<serde_json::Value>, String> {
    Ok(Vec::new())
}

#[tauri::command]
pub async fn get_game_profiles(state: State<'_, AppState>) -> Result<Vec<GameProfile>, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let api_url = config.api_url.clone();
    drop(config);

    let token = get_stored_token().ok_or("Not authenticated")?;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/devices/game-profiles", api_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let profiles: Vec<GameProfile> = response.json().await.map_err(|e| e.to_string())?;
        Ok(profiles)
    } else {
        Err("Failed to fetch game profiles".to_string())
    }
}

#[tauri::command]
pub async fn create_game_profile(
    state: State<'_, AppState>,
    name: String,
    platform: String,
) -> Result<GameProfile, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let api_url = config.api_url.clone();
    drop(config);

    let token = get_stored_token().ok_or("Not authenticated")?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/devices/game-profiles", api_url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": name,
            "platform": platform,
        }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if response.status().is_success() {
        let profile: GameProfile = response.json().await.map_err(|e| e.to_string())?;
        Ok(profile)
    } else {
        let error = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("Failed to create game profile: {}", error))
    }
}

fn get_stored_token() -> Option<String> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER).ok()?;
    entry.get_password().ok()
}

fn get_machine_id() -> String {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("wmic")
            .args(["csproduct", "get", "uuid"])
            .output()
            .ok()
            .and_then(|output| {
                String::from_utf8(output.stdout)
                    .ok()
                    .and_then(|s| s.lines().nth(1).map(|l| l.trim().to_string()))
            })
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        uuid::Uuid::new_v4().to_string()
    }
}

mod uuid {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    pub struct Uuid;
    
    impl Uuid {
        pub fn new_v4() -> UuidV4 {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            UuidV4(format!("{:032x}", timestamp))
        }
    }
    
    pub struct UuidV4(String);
    
    impl UuidV4 {
        pub fn to_string(&self) -> String {
            self.0.clone()
        }
    }
}
