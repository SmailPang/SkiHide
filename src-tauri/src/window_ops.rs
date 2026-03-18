use std::{collections::HashMap, mem::size_of, path::PathBuf, thread, time::Duration};

use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, MAX_PATH},
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
            VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F2, VK_F3,
            VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_F10, VK_F11, VK_F12, VK_F13, VK_F14,
            VK_F15, VK_F16, VK_F17, VK_F18, VK_F19, VK_F20, VK_F21, VK_F22, VK_F23, VK_F24,
            VK_HOME, VK_INSERT, VK_LEFT, VK_MENU, VK_NEXT, VK_PRIOR, VK_RETURN, VK_RIGHT,
            VK_SHIFT, VK_SPACE, VK_TAB, VK_UP, VK_LWIN,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
            IsWindow, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_HIDE, SW_SHOW,
        },
    },
};

use crate::models::WindowInfo;

struct EnumContext {
    windows: Vec<WindowInfo>,
}

pub fn list_windows(hidden_windows: &HashMap<u64, WindowInfo>) -> Vec<WindowInfo> {
    let mut context = EnumContext { windows: Vec::new() };

    unsafe {
        let context_ptr = &mut context as *mut EnumContext;
        let _ = EnumWindows(
            Some(enum_windows_proc),
            LPARAM(context_ptr.cast::<()>() as isize),
        );
    }

    for hidden in hidden_windows.values() {
        if !context.windows.iter().any(|item| item.hwnd == hidden.hwnd) {
            context.windows.push(hidden.clone());
        }
    }

    context
        .windows
        .sort_by(|left, right| left.title.to_lowercase().cmp(&right.title.to_lowercase()));
    context.windows
}

pub fn get_window_snapshot(hwnd_value: u64) -> Result<WindowInfo, String> {
    let hwnd = to_hwnd(hwnd_value)?;

    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err(format!("window {hwnd_value} is not valid"));
        }
    }

    let title = read_window_title(hwnd)?;
    if title.trim().is_empty() {
        return Err(format!("window {hwnd_value} has no title"));
    }

    Ok(WindowInfo {
        hwnd: hwnd_value,
        title,
        process_name: get_process_name(hwnd),
        is_hidden: false,
    })
}

pub fn hide_window(hwnd_value: u64) -> Result<(), String> {
    let hwnd = to_hwnd(hwnd_value)?;

    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }

    Ok(())
}

pub fn simulate_hotkey(hwnd_value: u64, hotkey: &str) -> Result<(), String> {
    let hwnd = to_hwnd(hwnd_value)?;

    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err(format!("window {hwnd_value} is not valid"));
        }

        let _ = SetForegroundWindow(hwnd);
    }

    thread::sleep(Duration::from_millis(20));

    let sequence = parse_hotkey_sequence(hotkey)?;
    if sequence.is_empty() {
        return Err("pause hotkey is empty".to_string());
    }

    let sent = unsafe { SendInput(&sequence, size_of::<INPUT>() as i32) } as usize;
    if sent != sequence.len() {
        return Err(format!("failed to send complete input sequence: {sent}/{}", sequence.len()));
    }

    Ok(())
}

pub fn show_window(hwnd_value: u64) -> Result<(), String> {
    let hwnd = to_hwnd(hwnd_value)?;

    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
    }

    Ok(())
}

fn to_hwnd(hwnd_value: u64) -> Result<HWND, String> {
    let raw = usize::try_from(hwnd_value).map_err(|_| "hwnd out of range".to_string())?;
    Ok(HWND(raw as *mut std::ffi::c_void))
}

fn parse_hotkey_sequence(hotkey: &str) -> Result<Vec<INPUT>, String> {
    let parts: Vec<&str> = hotkey
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();

    if parts.is_empty() {
        return Err("pause hotkey is empty".to_string());
    }

    let mut modifiers = Vec::new();
    let mut main_key = None;

    for part in parts {
        let vk = parse_virtual_key(part)?;
        if is_modifier_key(vk) {
            modifiers.push(vk);
        } else if main_key.is_some() {
            return Err(format!("pause hotkey supports only one main key: {hotkey}"));
        } else {
            main_key = Some(vk);
        }
    }

    let main_key = main_key.or_else(|| modifiers.pop()).ok_or_else(|| {
        format!("pause hotkey is invalid: {hotkey}")
    })?;

    let mut inputs = Vec::with_capacity(modifiers.len() * 2 + 2);
    for modifier in &modifiers {
        inputs.push(key_input(*modifier, false));
    }
    inputs.push(key_input(main_key, false));
    inputs.push(key_input(main_key, true));
    for modifier in modifiers.into_iter().rev() {
        inputs.push(key_input(modifier, true));
    }

    Ok(inputs)
}

fn key_input(virtual_key: VIRTUAL_KEY, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: virtual_key,
                wScan: 0,
                dwFlags: if key_up { KEYEVENTF_KEYUP } else { Default::default() },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn is_modifier_key(virtual_key: VIRTUAL_KEY) -> bool {
    virtual_key == VK_CONTROL
        || virtual_key == VK_MENU
        || virtual_key == VK_SHIFT
        || virtual_key == VK_LWIN
}

fn parse_virtual_key(key: &str) -> Result<VIRTUAL_KEY, String> {
    let normalized = key.trim().to_ascii_uppercase();

    let virtual_key = match normalized.as_str() {
        "CTRL" | "CONTROL" => VK_CONTROL,
        "ALT" => VK_MENU,
        "SHIFT" => VK_SHIFT,
        "WIN" | "META" => VK_LWIN,
        "ENTER" | "RETURN" => VK_RETURN,
        "SPACE" => VK_SPACE,
        "TAB" => VK_TAB,
        "ESC" | "ESCAPE" => VK_ESCAPE,
        "BACKSPACE" => VK_BACK,
        "INSERT" => VK_INSERT,
        "DELETE" | "DEL" => VK_DELETE,
        "HOME" => VK_HOME,
        "END" => VK_END,
        "PAGEUP" | "PGUP" => VK_PRIOR,
        "PAGEDOWN" | "PGDN" => VK_NEXT,
        "LEFT" => VK_LEFT,
        "RIGHT" => VK_RIGHT,
        "UP" => VK_UP,
        "DOWN" => VK_DOWN,
        "F1" => VK_F1,
        "F2" => VK_F2,
        "F3" => VK_F3,
        "F4" => VK_F4,
        "F5" => VK_F5,
        "F6" => VK_F6,
        "F7" => VK_F7,
        "F8" => VK_F8,
        "F9" => VK_F9,
        "F10" => VK_F10,
        "F11" => VK_F11,
        "F12" => VK_F12,
        "F13" => VK_F13,
        "F14" => VK_F14,
        "F15" => VK_F15,
        "F16" => VK_F16,
        "F17" => VK_F17,
        "F18" => VK_F18,
        "F19" => VK_F19,
        "F20" => VK_F20,
        "F21" => VK_F21,
        "F22" => VK_F22,
        "F23" => VK_F23,
        "F24" => VK_F24,
        _ if normalized.len() == 1 => {
            let ch = normalized.chars().next().unwrap_or_default();
            match ch {
                'A'..='Z' | '0'..='9' => VIRTUAL_KEY(ch as u16),
                _ => return Err(format!("unsupported pause hotkey key: {key}")),
            }
        }
        _ => return Err(format!("unsupported pause hotkey key: {key}")),
    };

    Ok(virtual_key)
}

unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    let context = &mut *(lparam.0 as *mut EnumContext);

    if !IsWindowVisible(hwnd).as_bool() {
        return true.into();
    }

    let title = match read_window_title(hwnd) {
        Ok(title) if !title.trim().is_empty() => title,
        _ => return true.into(),
    };

    let hwnd_u64 = hwnd.0 as usize as u64;
    let process_name = get_process_name(hwnd);
    if should_skip_window(&title, process_name.as_deref()) {
        return true.into();
    }

    context.windows.push(WindowInfo {
        hwnd: hwnd_u64,
        title,
        process_name,
        is_hidden: false,
    });

    true.into()
}

fn read_window_title(hwnd: HWND) -> Result<String, String> {
    let length = unsafe { GetWindowTextLengthW(hwnd) }.try_into().unwrap_or(0usize);

    if length == 0 {
        return Ok(String::new());
    }

    let mut buffer = vec![0u16; length + 1];
    let read_len = unsafe { GetWindowTextW(hwnd, &mut buffer) };

    if read_len == 0 {
        return Ok(String::new());
    }

    let used = usize::try_from(read_len).map_err(|_| "invalid title length".to_string())?;
    Ok(String::from_utf16_lossy(&buffer[..used]))
}

fn get_process_name(hwnd: HWND) -> Option<String> {
    let mut process_id = 0u32;
    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }

    if process_id == 0 {
        return None;
    }

    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()? };

    let mut buffer = vec![0u16; MAX_PATH as usize];
    let mut size = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
    };

    let _ = unsafe { windows::Win32::Foundation::CloseHandle(process) };

    if result.is_err() {
        return None;
    }

    let used = usize::try_from(size).ok()?;
    let full = String::from_utf16_lossy(&buffer[..used]);
    let path = PathBuf::from(full);

    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string())
}

fn should_skip_window(title: &str, process_name: Option<&str>) -> bool {
    let title_lc = title.to_lowercase();
    let process_lc = process_name.unwrap_or_default().to_lowercase();

    let blocked_processes = [
        "textinputhost.exe",
        "systemsettings.exe",
        "powershell.exe",
        "pwsh.exe",
        "openconsole.exe",
        "windowsterminal.exe",
    ];

    if blocked_processes.iter().any(|name| *name == process_lc) {
        return true;
    }

    if process_lc == "skihide.exe" {
        return true;
    }

    if title_lc.contains("program manager")
        || title_lc.contains("nvidia geforce overlay")
        || title_lc.contains("skihide")
    {
        return true;
    }

    let blocked_title_keywords = [
        "windows 输入体验",
        "windows input experience",
        "windows powershell",
        "powershell",
        "设置",
        "settings",
    ];

    blocked_title_keywords
        .iter()
        .any(|keyword| title_lc.contains(keyword))
}
