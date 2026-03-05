use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

use windows::{
    core::PCWSTR,
    Win32::UI::Shell::{SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND},
};

use crate::models::{CacheCleanupOptions, CacheCleanupReport};

pub fn cleanup_cache(options: &CacheCleanupOptions) -> CacheCleanupReport {
    let mut report = CacheCleanupReport {
        selected: 0,
        cleaned: 0,
        failed: 0,
        reclaimed_bytes: 0,
    };

    if options.system_cache {
        report.selected += 1;
        if let Some(windir) = env::var_os("WINDIR") {
            let path = PathBuf::from(windir).join("Temp");
            clear_directory_contents(&path, &mut report);
        } else {
            report.failed += 1;
        }
    }

    if options.temp_files {
        report.selected += 1;
        clear_directory_contents(&env::temp_dir(), &mut report);
    }

    if options.thumbnail_cache {
        report.selected += 1;
        clear_thumbnail_cache(&mut report);
    }

    if options.app_cache {
        report.selected += 1;
        clear_app_cache(&mut report);
    }

    if options.recycle_bin {
        report.selected += 1;
        let flags = SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND;
        let result = unsafe { SHEmptyRecycleBinW(None, PCWSTR::null(), flags) };
        if result.is_ok() {
            report.cleaned += 1;
        } else {
            report.failed += 1;
        }
    }

    report
}

fn clear_thumbnail_cache(report: &mut CacheCleanupReport) {
    let Some(local_app_data) = env::var_os("LOCALAPPDATA") else {
        report.failed += 1;
        return;
    };

    let explorer_dir = PathBuf::from(local_app_data).join("Microsoft\\Windows\\Explorer");
    let Ok(entries) = fs::read_dir(explorer_dir) else {
        report.failed += 1;
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        let lower = name.to_lowercase();
        if lower.starts_with("thumbcache") || lower.starts_with("iconcache") {
            clear_path(&path, report);
        }
    }
}

fn clear_app_cache(report: &mut CacheCleanupReport) {
    if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
        clear_directory_contents(&PathBuf::from(local_app_data).join("SkiHide\\cache"), report);
    } else {
        report.failed += 1;
    }

    if let Some(roaming_app_data) = env::var_os("APPDATA") {
        clear_directory_contents(
            &PathBuf::from(roaming_app_data).join("SkiHide\\cache"),
            report,
        );
    } else {
        report.failed += 1;
    }
}

fn clear_directory_contents(path: &Path, report: &mut CacheCleanupReport) {
    if !path.exists() {
        return;
    }

    let Ok(entries) = fs::read_dir(path) else {
        report.failed += 1;
        return;
    };

    for entry in entries.flatten() {
        clear_path(&entry.path(), report);
    }
}

fn clear_path(path: &Path, report: &mut CacheCleanupReport) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        report.failed += 1;
        return;
    };

    if metadata.is_file() {
        let file_size = metadata.len();
        match fs::remove_file(path) {
            Ok(()) => {
                report.cleaned += 1;
                report.reclaimed_bytes = report.reclaimed_bytes.saturating_add(file_size);
            }
            Err(_) => {
                report.failed += 1;
            }
        }
        return;
    }

    if metadata.is_dir() {
        let Ok(entries) = fs::read_dir(path) else {
            report.failed += 1;
            return;
        };
        for entry in entries.flatten() {
            clear_path(&entry.path(), report);
        }

        match fs::remove_dir(path) {
            Ok(()) => {
                report.cleaned += 1;
            }
            Err(_) => {
                report.failed += 1;
            }
        }
    }
}
