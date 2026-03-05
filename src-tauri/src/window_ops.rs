use std::{collections::HashMap, path::PathBuf};

use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, MAX_PATH},
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
            IsWindow, IsWindowVisible, ShowWindow, SW_HIDE, SW_SHOW,
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
