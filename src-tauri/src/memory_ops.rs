use std::mem::size_of;

use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        ProcessStatus::{EmptyWorkingSet, GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA, PROCESS_VM_READ},
    },
};

use crate::models::MemoryCleanupReport;

pub fn cleanup_system_memory() -> Result<MemoryCleanupReport, String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
        .map_err(|error| format!("failed to create process snapshot: {error}"))?;

    let mut entry = PROCESSENTRY32W::default();
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

    let mut scanned = 0u32;
    let mut cleaned = 0u32;
    let mut failed = 0u32;
    let mut reclaimed_bytes = 0u64;

    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();

    while has_entry {
        let pid = entry.th32ProcessID;
        if pid > 4 {
            scanned += 1;
            match cleanup_process_working_set(pid) {
                Ok(reclaimed) => {
                    cleaned += 1;
                    reclaimed_bytes = reclaimed_bytes.saturating_add(reclaimed);
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }

        has_entry = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }

    let _ = unsafe { CloseHandle(snapshot) };

    Ok(MemoryCleanupReport {
        scanned,
        cleaned,
        failed,
        reclaimed_bytes,
    })
}

fn cleanup_process_working_set(pid: u32) -> Result<u64, String> {
    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA | PROCESS_VM_READ,
            false,
            pid,
        )
    }
    .map_err(|error| format!("failed to open process {pid}: {error}"))?;

    let before = read_working_set(handle).unwrap_or(0);
    let empty_ok = unsafe { EmptyWorkingSet(handle) }.is_ok();
    let after = read_working_set(handle).unwrap_or(before);

    let _ = unsafe { CloseHandle(handle) };

    if !empty_ok {
        return Err(format!("failed to empty working set for process {pid}"));
    }

    Ok(before.saturating_sub(after))
}

fn read_working_set(handle: HANDLE) -> Result<u64, String> {
    let mut counters = PROCESS_MEMORY_COUNTERS::default();
    let ok = unsafe {
        GetProcessMemoryInfo(
            handle,
            &mut counters,
            size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
    }
    .is_ok();

    if !ok {
        return Err("failed to read process memory counters".to_string());
    }

    Ok(counters.WorkingSetSize as u64)
}
