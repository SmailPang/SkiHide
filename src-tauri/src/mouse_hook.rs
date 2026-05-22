use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, OnceLock,
    },
    thread,
};

use tauri::AppHandle;

use crate::log_messages;
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
        TranslateMessage, UnhookWindowsHookEx, MSG, WH_MOUSE_LL, WM_QUIT, WM_XBUTTONUP,
    },
};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static IS_EXITING: OnceLock<Arc<AtomicBool>> = OnceLock::new();
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

pub fn start_global_mouse_side_button_hook(
    app: AppHandle,
    is_exiting: Arc<AtomicBool>,
) -> Result<(), String> {
    let _ = APP_HANDLE.set(app);
    let _ = IS_EXITING.set(is_exiting);

    thread::Builder::new()
        .name("mouse-side-button-hook".to_string())
        .spawn(move || unsafe {
            HOOK_THREAD_ID.store(GetCurrentThreadId(), Ordering::SeqCst);

            let hook = match SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), None, 0) {
                Ok(hook) => hook,
                Err(error) => {
                    tracing::error!("{}", log_messages::mouse_hook_install_failed(&error));
                    HOOK_THREAD_ID.store(0, Ordering::SeqCst);
                    return;
                }
            };

            tracing::info!("{}", log_messages::mouse_hook_installed());

            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).into() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }

            let _ = UnhookWindowsHookEx(hook);
            HOOK_THREAD_ID.store(0, Ordering::SeqCst);
            tracing::info!("{}", log_messages::mouse_hook_removed());
        })
        .map_err(|error| format!("failed to spawn mouse side button hook thread: {error}"))?;

    Ok(())
}

/// Posts WM_QUIT to the hook thread so it unhooks and exits its message loop.
pub fn shutdown_global_mouse_side_button_hook() {
    let tid = HOOK_THREAD_ID.load(Ordering::SeqCst);
    if tid == 0 {
        return;
    }

    unsafe {
        let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
    }
}

unsafe extern "system" fn low_level_mouse_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code >= 0 && wparam.0 as u32 == WM_XBUTTONUP {
        if IS_EXITING
            .get()
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
        {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        if let Some(app) = APP_HANDLE.get() {
            crate::handle_mouse_side_button_global(app);
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}
