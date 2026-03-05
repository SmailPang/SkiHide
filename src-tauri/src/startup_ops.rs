use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, WIN32_ERROR},
        System::Registry::{
            RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW,
            HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPEN_CREATE_OPTIONS,
            REG_OPTION_NON_VOLATILE, REG_SZ, REG_VALUE_TYPE,
        },
    },
};

const RUN_KEY_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_VALUE_NAME: &str = "SkiHide";

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

pub fn sync_startup_registration(auto_start: bool, silent_start: bool) -> Result<(), String> {
    let key = open_run_key()?;

    if !auto_start {
        delete_run_value(key.raw(), RUN_VALUE_NAME)?;
        return Ok(());
    }

    let exe_path = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current exe path: {error}"))?;
    let exe_text = exe_path
        .to_str()
        .ok_or_else(|| "failed to convert exe path to utf-8".to_string())?;

    let mut command = format!("\"{exe_text}\"");
    if silent_start {
        command.push_str(" --silent");
    }

    write_string_value(key.raw(), RUN_VALUE_NAME, &command)
}

pub fn launched_in_silent_mode() -> bool {
    std::env::args().any(|arg| arg == "--silent")
}

fn open_run_key() -> Result<RegistryKey, String> {
    let mut result = HKEY::default();
    let subkey = to_wide(RUN_KEY_PATH);
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
            "failed to open run key {}: {}",
            RUN_KEY_PATH, status.0
        ));
    }

    Ok(RegistryKey(result))
}

fn delete_run_value(key: HKEY, name: &str) -> Result<(), String> {
    let value_name = to_wide(name);
    let status = unsafe { RegDeleteValueW(key, PCWSTR(value_name.as_ptr())) };
    if status == ERROR_FILE_NOT_FOUND || status == WIN32_ERROR(0) {
        return Ok(());
    }

    Err(format!(
        "failed to delete run value {}: {}",
        name, status.0
    ))
}

fn write_string_value(key: HKEY, name: &str, value: &str) -> Result<(), String> {
    let value_name = to_wide(name);
    let data_wide = to_wide(value);
    let data = unsafe {
        std::slice::from_raw_parts(
            data_wide.as_ptr() as *const u8,
            data_wide.len() * std::mem::size_of::<u16>(),
        )
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
            "failed to write run value {}: {}",
            name, status.0
        ));
    }

    Ok(())
}

#[allow(dead_code)]
fn _read_string_value(key: HKEY, name: &str) -> Result<Option<String>, String> {
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
            "failed querying run value {} size: {}",
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
            "failed reading run value {}: {}",
            name, query_status.0
        ));
    }

    let utf16_len = (size as usize) / 2;
    let utf16_slice =
        unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u16, utf16_len) };
    let content_len = utf16_slice
        .iter()
        .position(|ch| *ch == 0)
        .unwrap_or(utf16_slice.len());
    Ok(Some(String::from_utf16_lossy(&utf16_slice[..content_len])))
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
