#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_ops;
mod cache_ops;
mod config;
mod logging;
mod memory_ops;
mod models;
mod mouse_hook;
mod startup_ops;
mod update_ops;
mod window_ops;

use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    thread,
    time::Duration,
};

use parking_lot::Mutex;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, State, WebviewWindow, Wry,
};
use tauri_plugin_global_shortcut::{
    Builder as GlobalShortcutBuilder, GlobalShortcutExt, Shortcut, ShortcutState,
};
use tracing::{error, info};

use crate::{
    config::{load_config, save_config},
    models::{
        AppConfig, CacheCleanupOptions, CacheCleanupReport, ConfigUpdate, LogEntry,
        MirrorDownloadInfo,
        MemoryCleanupReport, UpdateCheckInfo, UpdateDownloadResult, WindowInfo,
    },
};

const LOG_EVENT: &str = "skihide://log";
#[allow(dead_code)]
const REFRESH_EVENT: &str = "skihide://refresh-requested";
const OPEN_SETTINGS_EVENT: &str = "skihide://open-settings";

struct AppState {
    app: AppHandle,
    config: Mutex<AppConfig>,
    hidden_windows: Mutex<HashMap<u64, WindowInfo>>,
    hidden_handles: Mutex<HashSet<u64>>,
    mute_applied_by_app: Mutex<bool>,
    mute_tracked_windows: Mutex<HashSet<u64>>,
    hotkey_enabled: Mutex<bool>,
    tray: Mutex<Option<TrayIcon>>,
}

impl AppState {
    fn new(app: AppHandle) -> Result<Self, String> {
        let config = load_config()?;

        Ok(Self {
            app,
            config: Mutex::new(config),
            hidden_windows: Mutex::new(HashMap::new()),
            hidden_handles: Mutex::new(HashSet::new()),
            mute_applied_by_app: Mutex::new(false),
            mute_tracked_windows: Mutex::new(HashSet::new()),
            hotkey_enabled: Mutex::new(false),
            tray: Mutex::new(None),
        })
    }

    fn log(&self, level: &str, message: impl Into<String>) {
        let message = message.into();
        let timestamp = timestamp_string();

        match level {
            "ERROR" => error!("{message}"),
            _ => info!("{message}"),
        }

        let payload = LogEntry {
            level: level.to_string(),
            message,
            timestamp,
        };

        let _ = self.app.emit(LOG_EVENT, payload);
    }

    fn current_config(&self) -> AppConfig {
        self.config.lock().clone()
    }

    fn update_config(&self, patch: ConfigUpdate) -> Result<AppConfig, String> {
        let mut config = self.config.lock();

        if let Some(hotkey) = patch.hotkey {
            config.hotkey = hotkey;
        }

        if let Some(language) = patch.language {
            config.language = language;
        }

        config.last_selected_hwnd = patch.last_selected_hwnd;

        if let Some(theme) = patch.theme {
            config.theme = theme;
        }

        if let Some(font_size) = patch.font_size {
            config.font_size = font_size;
        }

        if let Some(auto_start) = patch.auto_start {
            config.auto_start = auto_start;
        }

        if let Some(silent_start) = patch.silent_start {
            config.silent_start = silent_start;
        }

        if let Some(mute_on_hide) = patch.mute_on_hide {
            config.mute_on_hide = mute_on_hide;
        }

        if let Some(update_source) = patch.update_source {
            config.update_source = update_source;
        }

        if let Some(download_source) = patch.download_source {
            config.download_source = download_source;
        }

        if let Some(mirror_chan_sdk) = patch.mirror_chan_sdk {
            config.mirror_chan_sdk = mirror_chan_sdk;
        }

        if let Some(mouse_side_button_listener) = patch.mouse_side_button_listener {
            config.mouse_side_button_listener = mouse_side_button_listener;
        }
        if let Some(privacy_consent) = patch.privacy_consent {
            config.privacy_consent = privacy_consent;
        }

        save_config(&config)?;
        Ok(config.clone())
    }
}

#[tauri::command]
fn greet(name: String) -> String {
    format!("Hello, {name}!")
}

#[tauri::command]
fn list_windows(state: State<'_, AppState>) -> Result<Vec<WindowInfo>, String> {
    let hidden_windows = state.hidden_windows.lock().clone();
    let windows = window_ops::list_windows(&hidden_windows);
    state.log("INFO", format!("listed {} windows", windows.len()));
    Ok(windows)
}

#[tauri::command]
fn hide_window(hwnd: u64, state: State<'_, AppState>) -> Result<(), String> {
    let snapshot = window_ops::get_window_snapshot(hwnd)?;
    window_ops::hide_window(hwnd)?;

    {
        let mut hidden_windows = state.hidden_windows.lock();
        let mut hidden_handles = state.hidden_handles.lock();
        let mut hidden_snapshot = snapshot.clone();
        hidden_snapshot.is_hidden = true;
        hidden_windows.insert(hwnd, hidden_snapshot);
        hidden_handles.insert(hwnd);
    }

    let _ = state.update_config(ConfigUpdate {
        hotkey: None,
        language: None,
        last_selected_hwnd: Some(hwnd),
        ..Default::default()
    });

    let config = state.current_config();
    if config.mute_on_hide {
        apply_mute_on_hide(hwnd, state.inner());
    }

    state.log("INFO", format!("hid window {hwnd} ({})", snapshot.title));
    Ok(())
}

#[tauri::command]
fn show_window(hwnd: u64, state: State<'_, AppState>) -> Result<(), String> {
    window_ops::show_window(hwnd)?;

    {
        let mut hidden_windows = state.hidden_windows.lock();
        let mut hidden_handles = state.hidden_handles.lock();
        hidden_windows.remove(&hwnd);
        hidden_handles.remove(&hwnd);
    }

    let _ = state.update_config(ConfigUpdate {
        hotkey: None,
        language: None,
        last_selected_hwnd: Some(hwnd),
        ..Default::default()
    });

    restore_mute_on_show(hwnd, state.inner());

    state.log("INFO", format!("restored window {hwnd}"));
    Ok(())
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.current_config()
}

#[tauri::command]
fn update_config(patch: ConfigUpdate, state: State<'_, AppState>) -> Result<AppConfig, String> {
    let previous = state.current_config();
    let previous_hotkey = previous.hotkey.clone();
    let next = state.update_config(patch)?;
    let hotkey_enabled = *state.hotkey_enabled.lock();

    if previous_hotkey != next.hotkey && hotkey_enabled {
        register_hotkey(&state.app, &next.hotkey, true, state.inner())?;
    }

    if previous.auto_start != next.auto_start || previous.silent_start != next.silent_start {
        startup_ops::sync_startup_registration(next.auto_start, next.silent_start)?;
    }

    state.log(
        "INFO",
        format!(
            "config updated: hotkey={}, language={}, last_selected={:?}",
            next.hotkey, next.language, next.last_selected_hwnd
        ),
    );
    Ok(next)
}

#[tauri::command]
fn set_hotkey_enabled(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    if enabled {
        let hotkey = state.current_config().hotkey;
        register_hotkey(&state.app, &hotkey, true, state.inner())?;
        *state.hotkey_enabled.lock() = true;
        state.log("INFO", "hotkey listener enabled");
        return Ok(());
    }

    state
        .app
        .global_shortcut()
        .unregister_all()
        .map_err(|error| format!("failed to unregister hotkeys: {error}"))?;
    *state.hotkey_enabled.lock() = false;
    state.log("INFO", "hotkey listener disabled");
    Ok(())
}

#[tauri::command]
fn open_external_url(url: String, state: State<'_, AppState>) -> Result<(), String> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "", &url])
        .spawn()
        .map_err(|error| format!("failed to open external url: {error}"))?;

    state.log("INFO", format!("opened external url: {url}"));
    Ok(())
}

#[tauri::command]
fn exit_app(state: State<'_, AppState>) {
    state.log("INFO", "app exit requested by frontend");
    state.app.exit(0);
}

#[tauri::command]
fn apply_downloaded_update(file_path: String, state: State<'_, AppState>) -> Result<(), String> {
    let path = std::path::PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("update file does not exist: {file_path}"));
    }

    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable: {error}"))?;
    let pid = std::process::id();

    let source = path
        .canonicalize()
        .map_err(|error| format!("failed to resolve update package path: {error}"))?;
    let target = current_exe
        .canonicalize()
        .map_err(|error| format!("failed to resolve executable path: {error}"))?;

    if source == target {
        return Err("update package path cannot be the same as current executable".to_string());
    }

    let source_escaped = source.to_string_lossy().replace('\'', "''");
    let target_escaped = target.to_string_lossy().replace('\'', "''");
    let backup_escaped = format!("{target_escaped}.old");

    let script = format!(
        "Start-Sleep -Milliseconds 300; \
while (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ Start-Sleep -Milliseconds 300 }}; \
if (Test-Path '{backup_escaped}') {{ Remove-Item '{backup_escaped}' -Force -ErrorAction SilentlyContinue }}; \
if (Test-Path '{target_escaped}') {{ Move-Item '{target_escaped}' '{backup_escaped}' -Force }}; \
Move-Item '{source_escaped}' '{target_escaped}' -Force; \
Start-Process '{target_escaped}'; \
Start-Sleep -Milliseconds 1500; \
if (Test-Path '{backup_escaped}') {{ Remove-Item '{backup_escaped}' -Force -ErrorAction SilentlyContinue }}"
    );

    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .spawn()
        .map_err(|error| format!("failed to start updater helper: {error}"))?;

    state.log(
        "INFO",
        format!("scheduled in-place update replacement using package: {file_path}"),
    );
    state.app.exit(0);
    Ok(())
}

#[tauri::command]
fn cleanup_memory(state: State<'_, AppState>) -> Result<MemoryCleanupReport, String> {
    let report = memory_ops::cleanup_system_memory()?;
    state.log(
        "INFO",
        format!(
            "memory cleanup completed: scanned={}, cleaned={}, failed={}, reclaimed_bytes={}",
            report.scanned, report.cleaned, report.failed, report.reclaimed_bytes
        ),
    );
    Ok(report)
}

#[tauri::command]
fn cleanup_cache(
    options: CacheCleanupOptions,
    state: State<'_, AppState>,
) -> Result<CacheCleanupReport, String> {
    let report = cache_ops::cleanup_cache(&options);
    state.log(
        "INFO",
        format!(
            "cache cleanup completed: selected={}, cleaned={}, failed={}, reclaimed_bytes={}",
            report.selected, report.cleaned, report.failed, report.reclaimed_bytes
        ),
    );
    Ok(report)
}

#[tauri::command]
async fn check_for_updates(state: State<'_, AppState>) -> Result<UpdateCheckInfo, String> {
    let config = state.current_config();
    let current_version = state.app.package_info().version.to_string();

    let result = update_ops::check_for_updates(&current_version, &config).await?;
    state.log(
        "INFO",
        format!(
            "update check completed: source={}, current={}, latest={}, has_update={}",
            result.source, result.current_version, result.latest_version, result.has_update
        ),
    );
    Ok(result)
}

#[tauri::command]
async fn download_update_package(
    urls: Vec<String>,
    expected_sha256: Option<String>,
    version: String,
    state: State<'_, AppState>,
) -> Result<UpdateDownloadResult, String> {
    let result = update_ops::download_update_with_fallback(
        &state.app,
        &urls,
        expected_sha256.as_deref(),
        &version,
    )
    .await?;

    state.log(
        "INFO",
        format!("update package downloaded to {}", result.file_path),
    );
    Ok(result)
}

#[tauri::command]
async fn resolve_mirror_download_url(state: State<'_, AppState>) -> Result<MirrorDownloadInfo, String> {
    let config = state.current_config();
    let current_version = state.app.package_info().version.to_string();
    let result = update_ops::resolve_mirror_download_with_cdk(&current_version, &config.mirror_chan_sdk).await?;
    Ok(result)
}

fn register_hotkey(
    app: &AppHandle,
    hotkey: &str,
    replace: bool,
    state: &AppState,
) -> Result<(), String> {
    let manager = app.global_shortcut();

    if replace {
        manager
            .unregister_all()
            .map_err(|error| format!("failed to unregister hotkeys: {error}"))?;
    }

    let hotkey = hotkey.trim();
    if hotkey.is_empty() {
        state.log("INFO", "hotkey cleared");
        return Ok(());
    }

    let shortcut =
        Shortcut::from_str(hotkey).map_err(|error| format!("invalid hotkey `{hotkey}`: {error}"))?;

    manager
        .register(shortcut)
        .map_err(|error| format!("failed to register hotkey {hotkey}: {error}"))?;

    state.log("INFO", format!("registered hotkey {hotkey}"));
    Ok(())
}

fn apply_mute_on_hide(hwnd: u64, state: &AppState) {
    let mut mute_applied = state.mute_applied_by_app.lock();
    let mut tracked = state.mute_tracked_windows.lock();

    if *mute_applied {
        tracked.insert(hwnd);
        return;
    }

    match audio_ops::is_system_muted() {
        Ok(true) => {
            state.log(
                "INFO",
                "mute-on-hide skipped because system is already muted",
            );
        }
        Ok(false) => match audio_ops::set_system_mute(true) {
            Ok(()) => {
                *mute_applied = true;
                tracked.insert(hwnd);
                state.log("INFO", format!("mute-on-hide applied for window {hwnd}"));
            }
            Err(error) => {
                state.log("ERROR", format!("mute-on-hide failed: {error}"));
            }
        },
        Err(error) => {
            state.log("ERROR", format!("mute-on-hide state check failed: {error}"));
        }
    }
}

fn restore_mute_on_show(hwnd: u64, state: &AppState) {
    let mut mute_applied = state.mute_applied_by_app.lock();
    let mut tracked = state.mute_tracked_windows.lock();
    tracked.remove(&hwnd);

    if !*mute_applied || !tracked.is_empty() {
        return;
    }

    match audio_ops::set_system_mute(false) {
        Ok(()) => {
            *mute_applied = false;
            state.log("INFO", format!("mute-on-hide restored for window {hwnd}"));
        }
        Err(error) => {
            state.log("ERROR", format!("failed to restore mute-on-hide: {error}"));
        }
    }
}

fn handle_hotkey(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !*state.hotkey_enabled.lock() {
        return;
    }
    toggle_selected_window(app, "hotkey");
}

pub(crate) fn handle_mouse_side_button_global(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !*state.hotkey_enabled.lock() {
        return;
    }

    let config = state.current_config();
    if !config.mouse_side_button_listener {
        return;
    }

    toggle_selected_window(app, "mouse side button");
}

fn toggle_selected_window(app: &AppHandle, source: &str) {
    let state = app.state::<AppState>();
    let config = state.current_config();
    let Some(hwnd) = config.last_selected_hwnd else {
        state.log(
            "ERROR",
            format!("{source} triggered, but no selected window is available"),
        );
        return;
    };

    let is_hidden = state.hidden_handles.lock().contains(&hwnd);
    let result = if is_hidden {
        show_window(hwnd, state)
    } else {
        hide_window(hwnd, state)
    };

    if let Err(error) = result {
        let state = app.state::<AppState>();
        state.log("ERROR", format!("{source} action failed: {error}"));
    }
}

fn focus_main_window(window: &WebviewWindow) {
    let _ = window.set_skip_taskbar(false);
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn start_minimize_to_tray_watcher(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(220));

        let Some(window) = app.get_webview_window("main") else {
            break;
        };

        let Ok(minimized) = window.is_minimized() else {
            continue;
        };

        if minimized {
            let _ = window.set_skip_taskbar(true);
            let _ = window.hide();
            let _ = window.unminimize();
        }
    });
}

#[allow(dead_code)]
fn setup_tray(app: &tauri::App<Wry>) -> Result<(), String> {
    let show = MenuItemBuilder::with_id("show_main", "显示主窗口")
        .build(app)
        .map_err(|error| format!("failed to build show menu: {error}"))?;
    let refresh = MenuItemBuilder::with_id("refresh_list", "刷新窗口列表")
        .build(app)
        .map_err(|error| format!("failed to build refresh menu: {error}"))?;
    let quit = MenuItemBuilder::with_id("quit_app", "退出")
        .build(app)
        .map_err(|error| format!("failed to build quit menu: {error}"))?;

    let menu = MenuBuilder::new(app)
        .items(&[&show, &refresh, &quit])
        .build()
        .map_err(|error| format!("failed to build tray menu: {error}"))?;

    let icon = app
        .default_window_icon()
        .ok_or_else(|| "default window icon is not available".to_string())?;

    let tray = TrayIconBuilder::new()
        .menu(&menu)
        .icon(icon.clone())
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show_main" => {
                if let Some(window) = app.get_webview_window("main") {
                    focus_main_window(&window);
                }
            }
            "refresh_list" => {
                let state = app.state::<AppState>();
                state.log("INFO", "tray requested a window list refresh");
                let _ = app.emit(REFRESH_EVENT, ());
            }
            "quit_app" => {
                let state = app.state::<AppState>();
                state.log("INFO", "tray requested app exit");
                app.exit(0);
            }
            _ => {}
        })
        .build(app)
        .map_err(|error| format!("failed to build tray icon: {error}"))?;

    app.state::<AppState>().tray.lock().replace(tray);
    Ok(())
}

#[allow(dead_code)]
fn setup_tray_v2(app: &tauri::App<Wry>) -> Result<(), String> {
    let show = MenuItemBuilder::with_id("show_main", "打开主界面")
        .build(app)
        .map_err(|error| format!("failed to build show menu: {error}"))?;
    let settings = MenuItemBuilder::with_id("open_settings", "设置")
        .build(app)
        .map_err(|error| format!("failed to build settings menu: {error}"))?;
    let quit = MenuItemBuilder::with_id("quit_app", "退出")
        .build(app)
        .map_err(|error| format!("failed to build quit menu: {error}"))?;

    let menu = MenuBuilder::new(app)
        .items(&[&show, &settings, &quit])
        .build()
        .map_err(|error| format!("failed to build tray menu: {error}"))?;

    let icon = app
        .default_window_icon()
        .ok_or_else(|| "default window icon is not available".to_string())?;

    let tray = TrayIconBuilder::new()
        .menu(&menu)
        .icon(icon.clone())
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show_main" => {
                if let Some(window) = app.get_webview_window("main") {
                    focus_main_window(&window);
                }
            }
            "open_settings" => {
                if let Some(window) = app.get_webview_window("main") {
                    focus_main_window(&window);
                }
                let state = app.state::<AppState>();
                state.log("INFO", "tray requested settings page");
                let _ = app.emit(OPEN_SETTINGS_EVENT, ());
            }
            "quit_app" => {
                let state = app.state::<AppState>();
                state.log("INFO", "tray requested app exit");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { button, .. } = event {
                if button == MouseButton::Left {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        focus_main_window(&window);
                    }
                }
            }
        })
        .build(app)
        .map_err(|error| format!("failed to build tray icon: {error}"))?;

    app.state::<AppState>().tray.lock().replace(tray);
    Ok(())
}

fn setup_tray_v3(app: &tauri::App<Wry>) -> Result<(), String> {
    let show = MenuItemBuilder::with_id("show_main", "打开主界面")
        .build(app)
        .map_err(|error| format!("failed to build show menu: {error}"))?;
    let settings = MenuItemBuilder::with_id("open_settings", "设置")
        .build(app)
        .map_err(|error| format!("failed to build settings menu: {error}"))?;
    let quit = MenuItemBuilder::with_id("quit_app", "退出")
        .build(app)
        .map_err(|error| format!("failed to build quit menu: {error}"))?;

    let menu = MenuBuilder::new(app)
        .items(&[&show, &settings, &quit])
        .build()
        .map_err(|error| format!("failed to build tray menu: {error}"))?;

    let icon = app
        .default_window_icon()
        .ok_or_else(|| "default window icon is not available".to_string())?;

    let tray = TrayIconBuilder::new()
        .menu(&menu)
        .icon(icon.clone())
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show_main" => {
                if let Some(window) = app.get_webview_window("main") {
                    focus_main_window(&window);
                }
            }
            "open_settings" => {
                if let Some(window) = app.get_webview_window("main") {
                    focus_main_window(&window);
                }
                let state = app.state::<AppState>();
                state.log("INFO", "tray requested settings page");
                let _ = app.emit(OPEN_SETTINGS_EVENT, ());
            }
            "quit_app" => {
                let state = app.state::<AppState>();
                state.log("INFO", "tray requested app exit");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { button, .. } = event {
                if button == MouseButton::Left {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        focus_main_window(&window);
                    }
                }
            }
        })
        .build(app)
        .map_err(|error| format!("failed to build tray icon: {error}"))?;

    app.state::<AppState>().tray.lock().replace(tray);
    Ok(())
}

fn timestamp_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    format!("{secs}.{millis:03}")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut logging_context = logging::init_logging().unwrap_or_else(|error| {
        panic!("failed to initialize logging system: {error}");
    });

    let app = tauri::Builder::default()
        .plugin(
            GlobalShortcutBuilder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        handle_hotkey(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let state = AppState::new(app.handle().clone())?;
            app.manage(state);
            mouse_hook::start_global_mouse_side_button_hook(app.handle().clone())?;

            {
            let state = app.state::<AppState>();
            state.log("INFO", "application setup completed");
            }

            setup_tray_v3(app)?;
            start_minimize_to_tray_watcher(app.handle().clone());

            {
                let state = app.state::<AppState>();
                let cfg = state.current_config();
                if let Err(error) =
                    startup_ops::sync_startup_registration(cfg.auto_start, cfg.silent_start)
                {
                    state.log("ERROR", format!("failed to sync startup registration: {error}"));
                }
            }

            if startup_ops::launched_in_silent_mode() {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_skip_taskbar(true);
                    let _ = window.hide();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            list_windows,
            hide_window,
            show_window,
            get_config,
            update_config,
            set_hotkey_enabled,
            cleanup_memory,
            cleanup_cache,
            open_external_url,
            exit_app,
            apply_downloaded_update,
            check_for_updates,
            download_update_package,
            resolve_mirror_download_url
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(move |_app_handle, event| {
        if let RunEvent::Exit = event {
            if let Some(guard) = logging_context.guard.take() {
                drop(guard);
            }

            let _ = logging::archive_latest_log(
                &logging_context.latest_log_path,
                &logging_context.logs_dir,
            );
        }
    });
}

fn main() {
    run();
}
