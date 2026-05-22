//! 应用日志与 tracing 输出的中文文案。

use crate::models::{AppConfig, CacheCleanupOptions, MirrorCdkValidationInfo};

pub fn application_shutdown_started() -> &'static str {
    "应用开始退出"
}

pub fn tray_requested_settings() -> &'static str {
    "托盘：打开设置页"
}

pub fn tray_requested_exit() -> &'static str {
    "托盘：请求退出应用"
}

pub fn tray_requested_window_refresh() -> &'static str {
    "托盘：请求刷新窗口列表"
}

pub fn listed_windows(count: usize) -> String {
    format!("已列出 {count} 个窗口")
}

pub fn got_foreground_window(title: &str, hwnd: u64) -> String {
    format!("已获取前台窗口：{title}（句柄 {hwnd}）")
}

pub fn pause_on_hide_failed(hwnd: u64, error: &str) -> String {
    format!("隐藏时暂停失败（窗口 {hwnd}）：{error}")
}

pub fn hid_window(hwnd: u64, title: &str) -> String {
    format!("已隐藏窗口 {hwnd}（{title}）")
}

pub fn restored_window(hwnd: u64) -> String {
    format!("已恢复窗口 {hwnd}")
}

/// 对比更新前后配置，生成逐项「字段：旧值 → 新值」的中文变更说明。
pub fn describe_config_changes(before: &AppConfig, after: &AppConfig) -> String {
    let mut changes: Vec<String> = Vec::new();

    push_change(
        &mut changes,
        "快捷键",
        &format_hotkey(&before.hotkey),
        &format_hotkey(&after.hotkey),
        before.hotkey != after.hotkey,
    );
    push_change(
        &mut changes,
        "语言",
        &format_language(&before.language),
        &format_language(&after.language),
        before.language != after.language,
    );
    push_change(
        &mut changes,
        "选中窗口",
        &format_selected_hwnd(before.last_selected_hwnd),
        &format_selected_hwnd(after.last_selected_hwnd),
        before.last_selected_hwnd != after.last_selected_hwnd,
    );
    push_change(
        &mut changes,
        "主题",
        &format_theme(&before.theme),
        &format_theme(&after.theme),
        before.theme != after.theme,
    );
    push_change(
        &mut changes,
        "字体",
        &format_font_family(&before.font_family),
        &format_font_family(&after.font_family),
        before.font_family != after.font_family,
    );
    push_change(
        &mut changes,
        "字体大小",
        &format_font_size(&before.font_size),
        &format_font_size(&after.font_size),
        before.font_size != after.font_size,
    );
    push_change(
        &mut changes,
        "开机自启动",
        &format_bool(before.auto_start),
        &format_bool(after.auto_start),
        before.auto_start != after.auto_start,
    );
    push_change(
        &mut changes,
        "静默启动",
        &format_bool(before.silent_start),
        &format_bool(after.silent_start),
        before.silent_start != after.silent_start,
    );
    push_change(
        &mut changes,
        "隐藏时静音",
        &format_bool(before.mute_on_hide),
        &format_bool(after.mute_on_hide),
        before.mute_on_hide != after.mute_on_hide,
    );
    push_change(
        &mut changes,
        "隐藏时暂停",
        &format_bool(before.pause_on_hide),
        &format_bool(after.pause_on_hide),
        before.pause_on_hide != after.pause_on_hide,
    );
    push_change(
        &mut changes,
        "暂停快捷键",
        &format_hotkey(&before.pause_hotkey),
        &format_hotkey(&after.pause_hotkey),
        before.pause_hotkey != after.pause_hotkey,
    );
    push_change(
        &mut changes,
        "更新源",
        &format_update_source(&before.update_source),
        &format_update_source(&after.update_source),
        before.update_source != after.update_source,
    );
    push_change(
        &mut changes,
        "更新通道",
        &format_update_channel(&before.update_channel),
        &format_update_channel(&after.update_channel),
        before.update_channel != after.update_channel,
    );
    push_change(
        &mut changes,
        "下载源",
        &format_download_source(&before.download_source),
        &format_download_source(&after.download_source),
        before.download_source != after.download_source,
    );
    push_change(
        &mut changes,
        "Mirror 酱 CDK",
        &format_mirror_sdk(&before.mirror_chan_sdk),
        &format_mirror_sdk(&after.mirror_chan_sdk),
        before.mirror_chan_sdk != after.mirror_chan_sdk,
    );
    push_change(
        &mut changes,
        "自动检查更新",
        &format_bool(before.auto_check_updates),
        &format_bool(after.auto_check_updates),
        before.auto_check_updates != after.auto_check_updates,
    );
    push_change(
        &mut changes,
        "监听鼠标侧键",
        &format_bool(before.mouse_side_button_listener),
        &format_bool(after.mouse_side_button_listener),
        before.mouse_side_button_listener != after.mouse_side_button_listener,
    );
    push_change(
        &mut changes,
        "隐私政策同意",
        &format_bool(before.privacy_consent),
        &format_bool(after.privacy_consent),
        before.privacy_consent != after.privacy_consent,
    );
    push_change(
        &mut changes,
        "启动时自动监听",
        &format_bool(before.auto_listen_on_startup),
        &format_bool(after.auto_listen_on_startup),
        before.auto_listen_on_startup != after.auto_listen_on_startup,
    );

    if changes.is_empty() {
        "配置已保存（未检测到字段变化）".to_string()
    } else {
        format!("配置已变更：{}", changes.join("；"))
    }
}

fn push_change(changes: &mut Vec<String>, label: &str, before: &str, after: &str, changed: bool) {
    if changed {
        changes.push(format!("{label}：{before} → {after}"));
    }
}

fn format_bool(value: bool) -> String {
    if value {
        "开启".to_string()
    } else {
        "关闭".to_string()
    }
}

fn format_hotkey(value: &str) -> String {
    if value.trim().is_empty() {
        "（未设置）".to_string()
    } else {
        value.to_string()
    }
}

fn format_selected_hwnd(value: Option<u64>) -> String {
    match value {
        None => "（未选择）".to_string(),
        Some(0) => "当前前台窗口".to_string(),
        Some(hwnd) => format!("句柄 {hwnd}"),
    }
}

fn format_font_family(value: &str) -> String {
    if value.trim().is_empty() {
        "默认（内置）".to_string()
    } else {
        value.to_string()
    }
}

fn format_mirror_sdk(value: &str) -> String {
    if value.trim().is_empty() {
        "（未填写）".to_string()
    } else {
        format!("（已填写，{} 字符）", value.chars().count())
    }
}

fn format_theme(value: &str) -> String {
    match value {
        "system" => "跟随系统".to_string(),
        "light" => "浅色".to_string(),
        "dark" => "深色".to_string(),
        other => other.to_string(),
    }
}

fn format_font_size(value: &str) -> String {
    match value {
        "small" => "小".to_string(),
        "medium" => "标准".to_string(),
        "large" => "大".to_string(),
        "xlarge" => "超大".to_string(),
        other => other.to_string(),
    }
}

fn format_language(value: &str) -> String {
    match value {
        "zh_CN" => "简体中文".to_string(),
        "zh_TW" => "繁体中文".to_string(),
        "en" => "English".to_string(),
        "ja" => "日本語".to_string(),
        other => other.to_string(),
    }
}

fn format_update_source(value: &str) -> String {
    match value {
        "mirror_chan" => "Mirror酱".to_string(),
        "skihide" => "SkiHide".to_string(),
        other => other.to_string(),
    }
}

fn format_update_channel(value: &str) -> String {
    match value {
        "stable" => "稳定通道".to_string(),
        "beta" => "测试通道".to_string(),
        other => other.to_string(),
    }
}

fn format_download_source(value: &str) -> String {
    match value {
        "mirror_chan" => "Mirror酱".to_string(),
        "github" => "GitHub".to_string(),
        "rainyun_cdn" => "雨云 CDN".to_string(),
        other => other.to_string(),
    }
}

pub fn hotkey_listener_enabled() -> &'static str {
    "热键监听已启用"
}

pub fn hotkey_listener_disabled() -> &'static str {
    "热键监听已禁用"
}

pub fn opened_external_url(url: &str) -> String {
    format!("已打开外部链接：{url}")
}

pub fn app_exit_requested_by_frontend() -> &'static str {
    "前端请求退出应用"
}

pub fn scheduled_in_place_update(file_path: &str) -> String {
    format!("已安排就地更新，安装包：{file_path}")
}

fn format_memory_interval(value: u32, unit: &str) -> String {
    match unit {
        "seconds" => format!("{value} 秒"),
        "minutes" => format!("{value} 分钟"),
        "hours" => format!("{value} 小时"),
        other => format!("{value} {other}"),
    }
}

pub fn memory_auto_cleanup_schedule(
    enabled: bool,
    interval_value: u32,
    interval_unit: &str,
    scheduler_active: bool,
) -> String {
    if !enabled {
        return "自动清理内存：已关闭".to_string();
    }

    let interval = format_memory_interval(interval_value, interval_unit);
    if scheduler_active {
        format!("自动清理内存：已开启，每隔 {interval} 执行一次")
    } else {
        format!("自动清理内存：已开启，但间隔无效（{interval}），定时清理未启动")
    }
}

pub fn memory_manual_cleanup_completed(
    scanned: u32,
    cleaned: u32,
    failed: u32,
    reclaimed_bytes: u64,
) -> String {
    format!(
        "手动清理内存完成：扫描={scanned}，成功={cleaned}，失败={failed}，释放字节={reclaimed_bytes}"
    )
}

pub fn memory_auto_cleanup_triggered(
    interval_value: u32,
    interval_unit: &str,
    scanned: u32,
    cleaned: u32,
    failed: u32,
    reclaimed_bytes: u64,
) -> String {
    let interval = format_memory_interval(interval_value, interval_unit);
    format!(
        "定时自动清理内存（间隔 {interval}）：扫描={scanned}，成功={cleaned}，失败={failed}，释放字节={reclaimed_bytes}"
    )
}

fn format_cache_selections(options: &CacheCleanupOptions) -> String {
    let mut items = Vec::new();
    if options.system_cache {
        items.push("系统缓存");
    }
    if options.temp_files {
        items.push("临时文件");
    }
    if options.thumbnail_cache {
        items.push("缩略图缓存");
    }
    if options.app_cache {
        items.push("应用缓存");
    }
    if options.recycle_bin {
        items.push("回收站残留");
    }

    if items.is_empty() {
        "（未选择）".to_string()
    } else {
        items.join("、")
    }
}

pub fn cache_cleanup_completed(
    options: &CacheCleanupOptions,
    cleaned: u32,
    failed: u32,
    reclaimed_bytes: u64,
) -> String {
    let selections = format_cache_selections(options);
    format!(
        "缓存清理完成：清理项={selections}，成功={cleaned}，失败={failed}，释放字节={reclaimed_bytes}"
    )
}

pub fn update_check_completed(
    source: &str,
    current: &str,
    latest: &str,
    has_update: bool,
) -> String {
    format!(
        "更新检查完成：来源={source}，当前版本={current}，最新版本={latest}，有更新={has_update}"
    )
}

pub fn update_package_downloaded(path: &str) -> String {
    format!("更新包已下载至 {path}")
}

fn format_mirror_cdk_error_code(code: i32) -> String {
    match code {
        7001 => "CDK 已过期".to_string(),
        7002 => "CDK 错误".to_string(),
        7003 => "CDK 今日下载次数已达上限".to_string(),
        7004 => "CDK 与当前资源不匹配".to_string(),
        7005 => "CDK 已被封禁".to_string(),
        other => format!("未知错误（错误码 {other}）"),
    }
}

/// 记录 Mirror 酱 CDK 校验结果（不输出 CDK 明文）。
pub fn mirror_cdk_validation_result(result: &MirrorCdkValidationInfo, context: &str) -> String {
    let cdk_hint = "（CDK 已脱敏，仅记录校验结果）";

    if result.valid && result.mirror_code.is_none() {
        return format!("{context}：Mirror 酱 CDK 校验通过 {cdk_hint}");
    }

    let reason = result
        .mirror_code
        .map(format_mirror_cdk_error_code)
        .unwrap_or_else(|| "校验未通过".to_string());
    let detail = result
        .mirror_message
        .as_deref()
        .filter(|message| !message.trim().is_empty())
        .map(|message| format!("，接口返回：{message}"))
        .unwrap_or_default();

    format!("{context}：Mirror 酱 CDK 校验未通过，{reason}{detail} {cdk_hint}")
}

pub fn mirror_cdk_validation_request_failed(context: &str, error: &str) -> String {
    format!("{context}：Mirror 酱 CDK 校验请求失败：{error}")
}

pub fn hotkey_cleared() -> &'static str {
    "热键已清空"
}

pub fn registered_hotkey(hotkey: &str) -> String {
    format!("已注册热键 {hotkey}")
}

pub fn mute_on_hide_skipped_already_muted() -> &'static str {
    "隐藏时静音已跳过：系统当前已静音"
}

pub fn mute_on_hide_applied(hwnd: u64) -> String {
    format!("隐藏时静音已生效（窗口 {hwnd}）")
}

pub fn mute_on_hide_failed(error: &str) -> String {
    format!("隐藏时静音失败：{error}")
}

pub fn mute_on_hide_state_check_failed(error: &str) -> String {
    format!("隐藏时静音状态检查失败：{error}")
}

pub fn mute_on_hide_restored(hwnd: u64) -> String {
    format!("隐藏时静音已恢复（窗口 {hwnd}）")
}

pub fn mute_on_hide_restore_failed(error: &str) -> String {
    format!("恢复隐藏时静音失败：{error}")
}

pub fn trigger_no_selected_window(source: &str) -> String {
    format!("{source} 已触发，但没有可用的选中窗口")
}

pub fn trigger_using_foreground_window(source: &str, title: &str, hwnd: u64) -> String {
    format!("{source} 使用前台窗口：{title}（句柄 {hwnd}）")
}

pub fn trigger_foreground_failed(source: &str, error: &str) -> String {
    format!("{source} 获取前台窗口失败：{error}")
}

pub fn trigger_action_failed(source: &str, error: &str) -> String {
    format!("{source} 操作失败：{error}")
}

pub fn failed_relaunch_as_admin(error: &str) -> String {
    format!("以管理员身份重新启动失败：{error}")
}

pub fn application_setup_completed() -> &'static str {
    "应用初始化完成"
}

pub fn failed_sync_startup_registration(error: &str) -> String {
    format!("同步开机启动注册失败：{error}")
}

pub fn failed_auto_enable_hotkey_on_startup(error: &str) -> String {
    format!("启动时自动启用热键监听失败：{error}")
}

pub fn auto_enabled_hotkey_on_startup() -> &'static str {
    "已在启动时自动启用热键监听"
}

pub fn mouse_hook_install_failed(error: &impl std::fmt::Display) -> String {
    format!("安装全局鼠标侧键钩子失败：{error}")
}

pub fn mouse_hook_installed() -> &'static str {
    "全局鼠标侧键钩子已安装"
}

pub fn mouse_hook_removed() -> &'static str {
    "全局鼠标侧键钩子已移除"
}
