use std::ffi::c_void;
use std::mem::size_of;

use tracing::{info, warn};
use windows::core::{w, PCWSTR};
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE, LUID},
    Security::{
        AdjustTokenPrivileges, LookupPrivilegeValueW,
        LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
        TOKEN_QUERY,
    },
    System::{
        SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
        Threading::GetCurrentProcess,
    },
};

use crate::models::{MemoryCleanupReport, MemoryStatusInfo};

// OpenProcessToken 在 windows-rs 中的模块路径因版本而异，直接从 advapi32 链接更可靠
#[link(name = "advapi32")]
extern "system" {
    fn OpenProcessToken(
        process_handle: HANDLE,
        desired_access: u32,
        token_handle: *mut HANDLE,
    ) -> i32; // BOOL：非零为成功
}

// ---------------------------------------------------------------------------
// NtSetSystemInformation（通过 ntdll.lib 链接，ntdll 在所有 Windows 进程中常驻）
// ---------------------------------------------------------------------------

#[link(name = "ntdll")]
extern "system" {
    fn NtSetSystemInformation(
        system_information_class: i32,
        system_information: *mut c_void,
        system_information_length: u32,
    ) -> i32; // NTSTATUS：≥ 0 成功，< 0 失败
}

/// SystemMemoryListInformation（未文档化，值 = 80）
const SYSTEM_MEMORY_LIST_INFORMATION: i32 = 80;
/// SystemFileCacheInformation（值 = 21）
const SYSTEM_FILE_CACHE_INFORMATION: i32 = 21;

/// 系统级压缩所有进程工作集
const MEMORY_EMPTY_WORKING_SETS: i32 = 2;
/// 将已修改（脏）页刷写到换页文件，变为 Standby
const MEMORY_FLUSH_MODIFIED_LIST: i32 = 3;
/// 清除 Standby 列表，实际释放物理内存
const MEMORY_PURGE_STANDBY_LIST: i32 = 4;
/// 同上，针对低优先级 Standby 页
const MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST: i32 = 5;

/// 与 SYSTEM_FILECACHE_INFORMATION 布局对应（所有字段均为指针宽度）
#[repr(C)]
struct SystemFileCacheInformation {
    current_size: usize,
    peak_size: usize,
    page_fault_count: usize,
    minimum_working_set: usize,
    maximum_working_set: usize,
    current_size_including_transition_in_pages: usize,
    peak_size_including_transition_in_pages: usize,
    transition_re_purpose_count: usize,
    flags: usize,
}

// ---------------------------------------------------------------------------
// 公开接口
// ---------------------------------------------------------------------------

pub fn get_memory_status() -> Result<MemoryStatusInfo, String> {
    let mut status = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };

    unsafe { GlobalMemoryStatusEx(&mut status) }
        .map_err(|error| format!("获取内存状态失败：{error}"))?;

    let total_bytes = status.ullTotalPhys;
    let available_bytes = status.ullAvailPhys;
    let used_bytes = total_bytes.saturating_sub(available_bytes);

    Ok(MemoryStatusInfo {
        total_bytes,
        used_bytes,
        usage_percent: status.dwMemoryLoad,
    })
}

pub fn cleanup_system_memory() -> Result<MemoryCleanupReport, String> {
    // ── 1. 记录处理前内存量（参考 PCL_CE：处理前内存量 X）──────────────────
    let before = read_full_memory_status();
    info!(
        "内存优化开始：可用内存 {} / {}（已用 {}%）",
        format_bytes(before.available),
        format_bytes(before.total),
        before.load,
    );

    // ── 2. 申请权限（参考 PCL_CE：获取内存管理权限……）──────────────────────
    info!("正在申请内存管理权限（SeProfileSingleProcessPrivilege, SeIncreaseQuotaPrivilege）…");
    unsafe {
        let ok1 = acquire_privilege(w!("SeProfileSingleProcessPrivilege"));
        let ok2 = acquire_privilege(w!("SeIncreaseQuotaPrivilege"));
        if ok1 && ok2 {
            info!("内存管理权限申请成功");
        } else {
            warn!(
                "内存管理权限申请部分失败（SeProfileSingleProcess={}，SeIncreaseQuota={}），部分操作可能受限",
                if ok1 { "成功" } else { "失败" },
                if ok2 { "成功" } else { "失败" },
            );
        }
    }

    // ── 3. 逐步骤执行（参考 PCL_CE：开始处理，区域请求：All）──────────────
    info!("开始执行内存优化步骤（共 5 步）…");

    let mut succeeded = 0u32;
    let mut failed = 0u32;

    run_step(
        1, 5,
        "压缩工作集（EmptyWorkingSets）",
        execute_memory_list_op(MEMORY_EMPTY_WORKING_SETS),
        &mut succeeded, &mut failed,
    );

    run_step(
        2, 5,
        "刷写脏页（FlushModifiedList）",
        execute_memory_list_op(MEMORY_FLUSH_MODIFIED_LIST),
        &mut succeeded, &mut failed,
    );

    run_step(
        3, 5,
        "清除备用列表（PurgeStandbyList）",
        execute_memory_list_op(MEMORY_PURGE_STANDBY_LIST),
        &mut succeeded, &mut failed,
    );

    run_step(
        4, 5,
        "清除低优先级备用列表（PurgeLowPriorityStandbyList）",
        execute_memory_list_op(MEMORY_PURGE_LOW_PRIORITY_STANDBY_LIST),
        &mut succeeded, &mut failed,
    );

    run_step(
        5, 5,
        "释放文件缓存（FlushFileCache）",
        execute_flush_file_cache(),
        &mut succeeded, &mut failed,
    );

    // ── 4. 记录处理后内存量（参考 PCL_CE：处理后内存量 X，总共处理 Y）──────
    let after = read_full_memory_status();
    let reclaimed_bytes = after.available.saturating_sub(before.available);

    info!(
        "处理后内存量：可用 {} / {}（已用 {}%）",
        format_bytes(after.available),
        format_bytes(after.total),
        after.load,
    );
    info!(
        "内存优化结束：{succeeded}/{} 步骤成功，共释放 {}",
        succeeded + failed,
        format_bytes(reclaimed_bytes),
    );

    Ok(MemoryCleanupReport {
        scanned: succeeded + failed,
        cleaned: succeeded,
        failed,
        reclaimed_bytes,
    })
}

// ---------------------------------------------------------------------------
// 内部实现
// ---------------------------------------------------------------------------

struct FullMemoryStatus {
    total: u64,
    available: u64,
    load: u32,
}

fn read_full_memory_status() -> FullMemoryStatus {
    let mut s = MEMORYSTATUSEX {
        dwLength: size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    if unsafe { GlobalMemoryStatusEx(&mut s) }.is_ok() {
        FullMemoryStatus {
            total: s.ullTotalPhys,
            available: s.ullAvailPhys,
            load: s.dwMemoryLoad,
        }
    } else {
        FullMemoryStatus { total: 0, available: 0, load: 0 }
    }
}

/// 将字节数格式化为人类可读的 MB / GB 字符串。
fn format_bytes(bytes: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    let f = bytes as f64;
    if f >= GB {
        format!("{:.2} GB", f / GB)
    } else {
        format!("{:.1} MB", f / MB)
    }
}

/// 记录单步结果（成功/失败 + NTSTATUS 错误码）。
fn run_step(
    index: u32,
    total: u32,
    name: &str,
    ntstatus: i32,
    succeeded: &mut u32,
    failed: &mut u32,
) {
    if ntstatus >= 0 {
        info!("[{index}/{total}] {name}… 成功");
        *succeeded += 1;
    } else {
        // NTSTATUS 以十六进制表示更易读
        warn!("[{index}/{total}] {name}… 失败（NTSTATUS = {ntstatus:#010X}）");
        *failed += 1;
    }
}

/// 执行一次 SystemMemoryListInformation 操作，返回 NTSTATUS。
fn execute_memory_list_op(command: i32) -> i32 {
    let mut value = command;
    unsafe {
        NtSetSystemInformation(
            SYSTEM_MEMORY_LIST_INFORMATION,
            std::ptr::addr_of_mut!(value).cast::<c_void>(),
            size_of::<i32>() as u32,
        )
    }
}

/// 将系统文件缓存工作集设为 usize::MAX 以触发完全刷新，返回 NTSTATUS。
fn execute_flush_file_cache() -> i32 {
    let mut info = SystemFileCacheInformation {
        current_size: 0,
        peak_size: 0,
        page_fault_count: 0,
        minimum_working_set: usize::MAX,
        maximum_working_set: usize::MAX,
        current_size_including_transition_in_pages: 0,
        peak_size_including_transition_in_pages: 0,
        transition_re_purpose_count: 0,
        flags: 0,
    };

    unsafe {
        NtSetSystemInformation(
            SYSTEM_FILE_CACHE_INFORMATION,
            std::ptr::addr_of_mut!(info).cast::<c_void>(),
            size_of::<SystemFileCacheInformation>() as u32,
        )
    }
}

/// 申请指定特权，返回是否成功。
unsafe fn acquire_privilege(privilege_name: PCWSTR) -> bool {
    let mut token = HANDLE::default();
    let access = (TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY).0;
    if OpenProcessToken(GetCurrentProcess(), access, &mut token) == 0 {
        return false;
    }

    let mut luid = LUID::default();
    if LookupPrivilegeValueW(PCWSTR::null(), privilege_name, &mut luid).is_err() {
        let _ = CloseHandle(token);
        return false;
    }

    let tp = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };

    let ok = AdjustTokenPrivileges(token, false, Some(&tp), 0, None, None).is_ok();
    let _ = CloseHandle(token);
    ok
}
