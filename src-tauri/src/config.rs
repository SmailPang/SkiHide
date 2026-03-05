use std::{mem::size_of, slice};

use crate::models::AppConfig;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, WIN32_ERROR},
        Globalization::GetUserDefaultLocaleName,
        System::Registry::{
            RegCloseKey, RegCreateKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
            HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPEN_CREATE_OPTIONS,
            REG_DWORD, REG_OPTION_NON_VOLATILE, REG_QWORD, REG_SZ, REG_VALUE_TYPE,
        },
    },
};

const REGISTRY_SUBKEY: &str = "Software\\SkiHide";
const VALUE_HOTKEY: &str = "Hotkey";
const VALUE_LANGUAGE: &str = "Language";
const VALUE_LAST_SELECTED_HWND: &str = "LastSelectedHwnd";
const VALUE_THEME: &str = "Theme";
const VALUE_FONT_SIZE: &str = "FontSize";
const VALUE_AUTO_START: &str = "AutoStart";
const VALUE_SILENT_START: &str = "SilentStart";
const VALUE_MUTE_ON_HIDE: &str = "MuteOnHide";
const VALUE_UPDATE_SOURCE: &str = "UpdateSource";
const VALUE_DOWNLOAD_SOURCE: &str = "DownloadSource";
const VALUE_MIRROR_CHAN_SDK: &str = "MirrorChanSdk";
const VALUE_MOUSE_SIDE_BUTTON_LISTENER: &str = "MouseSideButtonListener";
const VALUE_PRIVACY_CONSENT: &str = "PrivacyConsent";

struct RegistryKey(HKEY);

impl RegistryKey {
    fn raw(&self) -> HKEY {
        self.0
    }
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

pub fn load_config() -> Result<AppConfig, String> {
    let key = open_or_create_key()?;
    let mut config = AppConfig::default();

    if let Some(hotkey) = read_string_value(key.raw(), VALUE_HOTKEY)? {
        config.hotkey = hotkey;
    }

    if let Some(language) = read_string_value(key.raw(), VALUE_LANGUAGE)? {
        config.language = normalize_language(&language);
    } else {
        config.language = detect_system_language();
    }

    config.last_selected_hwnd = read_u64_value(key.raw(), VALUE_LAST_SELECTED_HWND)?;
    if let Some(theme) = read_string_value(key.raw(), VALUE_THEME)? {
        config.theme = theme;
    }
    if let Some(font_size) = read_string_value(key.raw(), VALUE_FONT_SIZE)? {
        config.font_size = font_size;
    }
    if let Some(auto_start) = read_bool_value(key.raw(), VALUE_AUTO_START)? {
        config.auto_start = auto_start;
    }
    if let Some(silent_start) = read_bool_value(key.raw(), VALUE_SILENT_START)? {
        config.silent_start = silent_start;
    }
    if let Some(mute_on_hide) = read_bool_value(key.raw(), VALUE_MUTE_ON_HIDE)? {
        config.mute_on_hide = mute_on_hide;
    }
    if let Some(update_source) = read_string_value(key.raw(), VALUE_UPDATE_SOURCE)? {
        config.update_source = update_source;
    }
    if let Some(download_source) = read_string_value(key.raw(), VALUE_DOWNLOAD_SOURCE)? {
        config.download_source = download_source;
    }
    if let Some(mirror_chan_sdk) = read_string_value(key.raw(), VALUE_MIRROR_CHAN_SDK)? {
        config.mirror_chan_sdk = mirror_chan_sdk;
    }
    if let Some(mouse_side_button_listener) =
        read_bool_value(key.raw(), VALUE_MOUSE_SIDE_BUTTON_LISTENER)?
    {
        config.mouse_side_button_listener = mouse_side_button_listener;
    }
    if let Some(privacy_consent) = read_bool_value(key.raw(), VALUE_PRIVACY_CONSENT)? {
        config.privacy_consent = privacy_consent;
    }

    save_config(&config)?;
    Ok(config)
}

fn detect_system_language() -> String {
    let mut locale_name = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(&mut locale_name) };
    if len <= 1 {
        return "zh_CN".to_string();
    }

    let locale = String::from_utf16_lossy(&locale_name[..(len as usize - 1)]);
    normalize_language(&locale)
}

fn normalize_language(raw: &str) -> String {
    let value = raw.trim();
    if value.eq_ignore_ascii_case("zh_CN") {
        return "zh_CN".to_string();
    }
    if value.eq_ignore_ascii_case("zh_TW") {
        return "zh_TW".to_string();
    }
    if value.eq_ignore_ascii_case("en_US") {
        return "en_US".to_string();
    }
    if value.eq_ignore_ascii_case("ja_JP") {
        return "ja_JP".to_string();
    }

    let lowered = value.replace('-', "_").to_ascii_lowercase();
    if lowered.starts_with("zh_tw")
        || lowered.starts_with("zh_hk")
        || lowered.starts_with("zh_mo")
        || lowered.contains("hant")
    {
        return "zh_TW".to_string();
    }
    if lowered.starts_with("zh") {
        return "zh_CN".to_string();
    }
    if lowered.starts_with("ja") {
        return "ja_JP".to_string();
    }
    if lowered.starts_with("en") {
        return "en_US".to_string();
    }

    "zh_CN".to_string()
}

pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let key = open_or_create_key()?;

    write_string_value(key.raw(), VALUE_HOTKEY, &config.hotkey)?;
    write_string_value(key.raw(), VALUE_LANGUAGE, &config.language)?;

    if let Some(hwnd) = config.last_selected_hwnd {
        write_u64_value(key.raw(), VALUE_LAST_SELECTED_HWND, hwnd)?;
    } else {
        write_u64_value(key.raw(), VALUE_LAST_SELECTED_HWND, 0)?;
    }
    write_string_value(key.raw(), VALUE_THEME, &config.theme)?;
    write_string_value(key.raw(), VALUE_FONT_SIZE, &config.font_size)?;
    write_bool_value(key.raw(), VALUE_AUTO_START, config.auto_start)?;
    write_bool_value(key.raw(), VALUE_SILENT_START, config.silent_start)?;
    write_bool_value(key.raw(), VALUE_MUTE_ON_HIDE, config.mute_on_hide)?;
    write_string_value(key.raw(), VALUE_UPDATE_SOURCE, &config.update_source)?;
    write_string_value(key.raw(), VALUE_DOWNLOAD_SOURCE, &config.download_source)?;
    write_string_value(key.raw(), VALUE_MIRROR_CHAN_SDK, &config.mirror_chan_sdk)?;
    write_bool_value(
        key.raw(),
        VALUE_MOUSE_SIDE_BUTTON_LISTENER,
        config.mouse_side_button_listener,
    )?;
    write_bool_value(key.raw(), VALUE_PRIVACY_CONSENT, config.privacy_consent)?;

    Ok(())
}

fn open_or_create_key() -> Result<RegistryKey, String> {
    let mut result = HKEY::default();
    let subkey = to_wide(REGISTRY_SUBKEY);
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPEN_CREATE_OPTIONS(REG_OPTION_NON_VOLATILE.0),
            KEY_READ | KEY_WRITE,
            None,
            &mut result,
            None,
        )
    };

    if status != WIN32_ERROR(0) {
        return Err(format!(
            "failed to open/create registry key {}: {}",
            REGISTRY_SUBKEY, status.0
        ));
    }

    Ok(RegistryKey(result))
}

fn read_string_value(key: HKEY, name: &str) -> Result<Option<String>, String> {
    let value_name = to_wide(name);
    let mut value_type = REG_VALUE_TYPE(0);
    let mut size = 0u32;
    let query_size_status = unsafe {
        RegQueryValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            Some(&mut value_type),
            None,
            Some(&mut size),
        )
    };

    if query_size_status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }

    if query_size_status != WIN32_ERROR(0) {
        return Err(format!(
            "failed querying registry value {} size: {}",
            name, query_size_status.0
        ));
    }

    if size == 0 {
        return Ok(Some(String::new()));
    }

    let mut data = vec![0u8; size as usize];
    let query_status = unsafe {
        RegQueryValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            Some(&mut value_type),
            Some(data.as_mut_ptr()),
            Some(&mut size),
        )
    };

    if query_status != WIN32_ERROR(0) {
        return Err(format!(
            "failed reading registry value {}: {}",
            name, query_status.0
        ));
    }

    let utf16_len = (size as usize) / 2;
    let utf16_slice = unsafe { slice::from_raw_parts(data.as_ptr() as *const u16, utf16_len) };
    let content_len = utf16_slice
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(utf16_slice.len());
    Ok(Some(String::from_utf16_lossy(&utf16_slice[..content_len])))
}

fn read_u64_value(key: HKEY, name: &str) -> Result<Option<u64>, String> {
    let value_name = to_wide(name);
    let mut value_type = REG_VALUE_TYPE(0);
    let mut size = 0u32;
    let query_size_status = unsafe {
        RegQueryValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            Some(&mut value_type),
            None,
            Some(&mut size),
        )
    };

    if query_size_status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }

    if query_size_status != WIN32_ERROR(0) {
        return Err(format!(
            "failed querying registry value {} size: {}",
            name, query_size_status.0
        ));
    }

    if size < 8 {
        return Ok(None);
    }

    let mut bytes = [0u8; 8];
    let mut read_size = 8u32;
    let query_status = unsafe {
        RegQueryValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            Some(&mut value_type),
            Some(bytes.as_mut_ptr()),
            Some(&mut read_size),
        )
    };

    if query_status != WIN32_ERROR(0) {
        return Err(format!(
            "failed reading registry value {}: {}",
            name, query_status.0
        ));
    }

    let value = u64::from_le_bytes(bytes);
    if value == 0 {
        return Ok(None);
    }
    Ok(Some(value))
}

fn read_bool_value(key: HKEY, name: &str) -> Result<Option<bool>, String> {
    let value_name = to_wide(name);
    let mut value_type = REG_VALUE_TYPE(0);
    let mut size = 0u32;
    let query_size_status = unsafe {
        RegQueryValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            Some(&mut value_type),
            None,
            Some(&mut size),
        )
    };

    if query_size_status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }

    if query_size_status != WIN32_ERROR(0) {
        return Err(format!(
            "failed querying registry value {} size: {}",
            name, query_size_status.0
        ));
    }

    if size < 4 {
        return Ok(None);
    }

    let mut bytes = [0u8; 4];
    let mut read_size = 4u32;
    let query_status = unsafe {
        RegQueryValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            Some(&mut value_type),
            Some(bytes.as_mut_ptr()),
            Some(&mut read_size),
        )
    };

    if query_status != WIN32_ERROR(0) {
        return Err(format!(
            "failed reading registry value {}: {}",
            name, query_status.0
        ));
    }

    Ok(Some(u32::from_le_bytes(bytes) != 0))
}

fn write_string_value(key: HKEY, name: &str, value: &str) -> Result<(), String> {
    let value_name = to_wide(name);
    let data_wide = to_wide(value);
    let data = unsafe {
        slice::from_raw_parts(data_wide.as_ptr() as *const u8, data_wide.len() * size_of::<u16>())
    };

    let status = unsafe {
        RegSetValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            REG_SZ,
            Some(data),
        )
    };

    if status != WIN32_ERROR(0) {
        return Err(format!(
            "failed writing registry string value {}: {}",
            name, status.0
        ));
    }
    Ok(())
}

fn write_u64_value(key: HKEY, name: &str, value: u64) -> Result<(), String> {
    let value_name = to_wide(name);
    let bytes = value.to_le_bytes();
    let status = unsafe {
        RegSetValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            REG_QWORD,
            Some(&bytes),
        )
    };

    if status != WIN32_ERROR(0) {
        return Err(format!(
            "failed writing registry qword value {}: {}",
            name, status.0
        ));
    }
    Ok(())
}

fn write_bool_value(key: HKEY, name: &str, value: bool) -> Result<(), String> {
    let value_name = to_wide(name);
    let bytes = (if value { 1u32 } else { 0u32 }).to_le_bytes();
    let status = unsafe {
        RegSetValueExW(
            key,
            PCWSTR(value_name.as_ptr()),
            None,
            REG_DWORD,
            Some(&bytes),
        )
    };

    if status != WIN32_ERROR(0) {
        return Err(format!(
            "failed writing registry bool value {}: {}",
            name, status.0
        ));
    }
    Ok(())
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
