use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub hwnd: u64,
    pub title: String,
    pub process_name: Option<String>,
    pub is_hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub hotkey: String,
    pub language: String,
    pub last_selected_hwnd: Option<u64>,
    pub theme: String,
    pub font_size: String,
    pub auto_start: bool,
    pub silent_start: bool,
    pub mute_on_hide: bool,
    pub pause_on_hide: bool,
    pub pause_hotkey: String,
    pub update_source: String,
    pub download_source: String,
    pub mirror_chan_sdk: String,
    pub auto_check_updates: bool,
    pub mouse_side_button_listener: bool,
    pub privacy_consent: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: String::new(),
            language: "zh_CN".to_string(),
            last_selected_hwnd: None,
            theme: "system".to_string(),
            font_size: "medium".to_string(),
            auto_start: false,
            silent_start: false,
            mute_on_hide: false,
            pause_on_hide: false,
            pause_hotkey: String::new(),
            update_source: "mirror_chan".to_string(),
            download_source: "rainyun_cdn".to_string(),
            mirror_chan_sdk: String::new(),
            auto_check_updates: true,
            mouse_side_button_listener: false,
            privacy_consent: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigUpdate {
    pub hotkey: Option<String>,
    pub language: Option<String>,
    pub last_selected_hwnd: Option<u64>,
    pub theme: Option<String>,
    pub font_size: Option<String>,
    pub auto_start: Option<bool>,
    pub silent_start: Option<bool>,
    pub mute_on_hide: Option<bool>,
    pub pause_on_hide: Option<bool>,
    pub pause_hotkey: Option<String>,
    pub update_source: Option<String>,
    pub download_source: Option<String>,
    pub mirror_chan_sdk: Option<String>,
    pub auto_check_updates: Option<bool>,
    pub mouse_side_button_listener: Option<bool>,
    pub privacy_consent: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryCleanupReport {
    pub scanned: u32,
    pub cleaned: u32,
    pub failed: u32,
    pub reclaimed_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryStatusInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub usage_percent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheCleanupOptions {
    pub system_cache: bool,
    pub temp_files: bool,
    pub thumbnail_cache: bool,
    pub app_cache: bool,
    pub recycle_bin: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CacheCleanupReport {
    pub selected: u32,
    pub cleaned: u32,
    pub failed: u32,
    pub reclaimed_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckInfo {
    pub source: String,
    pub current_version: String,
    pub latest_version: String,
    pub changelog: String,
    pub has_update: bool,
    pub download_url: Option<String>,
    pub download_candidates: Vec<String>,
    pub sha256: Option<String>,
    pub mirror_code: Option<i32>,
    pub mirror_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDownloadResult {
    pub file_path: String,
    pub sha256: Option<String>,
    pub used_url: String,
    pub fallback_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorDownloadInfo {
    pub url: Option<String>,
    pub sha256: Option<String>,
    pub mirror_code: Option<i32>,
    pub mirror_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorCdkValidationInfo {
    pub valid: bool,
    pub mirror_code: Option<i32>,
    pub mirror_message: Option<String>,
}
