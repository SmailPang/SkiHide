use std::{ffi::OsStr, mem::size_of, os::windows::ffi::OsStrExt, path::Path};

use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{CloseHandle, HANDLE, HWND},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    },
};

pub fn ensure_elevated_startup() -> Result<bool, String> {
    if is_process_elevated()? {
        return Ok(false);
    }

    let exe = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current executable path: {error}"))?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let params = build_command_line(&args);

    relaunch_as_admin(&exe, &params)?;
    Ok(true)
}

fn is_process_elevated() -> Result<bool, String> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
        .map_err(|error| format!("failed to open process token: {error}"))?;

    let mut elevation = TOKEN_ELEVATION::default();
    let mut return_len = 0u32;
    let result = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_len,
        )
    };

    let _ = unsafe { CloseHandle(token) };
    result.map_err(|error| format!("failed to query token elevation: {error}"))?;

    Ok(elevation.TokenIsElevated != 0)
}

fn relaunch_as_admin(exe: &Path, params: &str) -> Result<(), String> {
    let operation = to_wide("runas");
    let exe_str = exe.to_string_lossy().to_string();
    let exe_w = to_wide(&exe_str);
    let params_w = to_wide(params);

    let params_ptr = if params.is_empty() {
        PCWSTR::null()
    } else {
        PCWSTR(params_w.as_ptr())
    };

    let result = unsafe {
        ShellExecuteW(
            Some(HWND(0 as _)),
            PCWSTR(operation.as_ptr()),
            PCWSTR(exe_w.as_ptr()),
            params_ptr,
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };

    let code = result.0 as isize;
    if code <= 32 {
        return Err(format!("ShellExecuteW runas failed with code {code}"));
    }

    Ok(())
}

fn build_command_line(args: &[String]) -> String {
    args.iter()
        .map(|arg| quote_arg(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_arg(arg: &str) -> String {
    if arg.is_empty() || arg.chars().any(|ch| ch.is_whitespace() || ch == '"') {
        format!("\"{}\"", arg.replace('"', "\\\""))
    } else {
        arg.to_string()
    }
}

fn to_wide(text: &str) -> Vec<u16> {
    OsStr::new(text)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
