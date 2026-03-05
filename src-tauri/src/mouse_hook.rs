use std::{sync::OnceLock, thread};

use tauri::AppHandle;
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        UnhookWindowsHookEx, MSG, WH_MOUSE_LL, WM_XBUTTONUP,
    },
};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn start_global_mouse_side_button_hook(app: AppHandle) -> Result<(), String> {
    let _ = APP_HANDLE.set(app);

    thread::Builder::new()
        .name("mouse-side-button-hook".to_string())
        .spawn(move || unsafe {
            let hook = match SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), None, 0) {
                Ok(hook) => hook,
                Err(error) => {
                    tracing::error!(
                        "failed to install global mouse side button hook: {error}"
                    );
                    return;
                }
            };

            tracing::info!("global mouse side button hook installed");

            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).into() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }

            let _ = UnhookWindowsHookEx(hook);
            tracing::info!("global mouse side button hook removed");
        })
        .map_err(|error| format!("failed to spawn mouse side button hook thread: {error}"))?;

    Ok(())
}

unsafe extern "system" fn low_level_mouse_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 && wparam.0 as u32 == WM_XBUTTONUP {
        if let Some(app) = APP_HANDLE.get() {
            crate::handle_mouse_side_button_global(app);
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}
