use std::{
    backtrace::Backtrace,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    panic::PanicHookInfo,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Local;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, registry};

pub struct LoggingContext {
    pub logs_dir: PathBuf,
    pub latest_log_path: PathBuf,
    pub guard: Option<WorkerGuard>,
}

pub fn init_logging() -> Result<LoggingContext, String> {
    let logs_dir = resolve_logs_dir()?;
    fs::create_dir_all(&logs_dir)
        .map_err(|error| format!("创建日志目录失败：{error}"))?;

    let latest_log_path = logs_dir.join("latest.log");
    let latest_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&latest_log_path)
        .map_err(|error| format!("打开 latest.log 失败：{error}"))?;

    let (file_writer, guard) = tracing_appender::non_blocking(latest_file);

    let stdout_layer = fmt::layer().with_target(false).with_level(true).compact();
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_level(true)
        .with_writer(file_writer)
        .compact();

    let subscriber = registry().with(stdout_layer).with(file_layer);
    tracing::subscriber::set_global_default(subscriber)
        .map_err(|error| format!("初始化 tracing 订阅器失败：{error}"))?;

    install_panic_hook(logs_dir.clone(), latest_log_path.clone());

    Ok(LoggingContext {
        logs_dir,
        latest_log_path,
        guard: Some(guard),
    })
}

pub fn archive_latest_log(latest_log_path: &Path, logs_dir: &Path) -> Result<PathBuf, String> {
    if !latest_log_path.exists() {
        return Err("latest.log 不存在".to_string());
    }

    let filename = format!("{}.log", timestamp_for_file_name());
    let archive_path = logs_dir.join(filename);

    fs::copy(latest_log_path, &archive_path)
        .map_err(|error| format!("归档 latest.log 失败：{error}"))?;

    Ok(archive_path)
}

fn resolve_logs_dir() -> Result<PathBuf, String> {
    let current_dir =
        std::env::current_dir().map_err(|error| format!("获取当前目录失败：{error}"))?;
    Ok(current_dir.join("logs"))
}

fn install_panic_hook(logs_dir: PathBuf, latest_log_path: PathBuf) {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = write_error_log(&logs_dir, &latest_log_path, panic_info);
        default_hook(panic_info);
    }));
}

fn write_error_log(
    logs_dir: &Path,
    latest_log_path: &Path,
    panic_info: &PanicHookInfo<'_>,
) -> Result<(), String> {
    fs::create_dir_all(logs_dir).map_err(|error| format!("确保日志目录存在失败：{error}"))?;
    let error_log_path = logs_dir.join("error.log");

    let mut file = File::create(&error_log_path)
        .map_err(|error| format!("创建 error.log 失败：{error}"))?;

    let timestamp = timestamp_seconds();
    writeln!(file, "SkiHide 崩溃报告")
        .map_err(|error| format!("写入崩溃报告标题失败：{error}"))?;
    writeln!(file, "时间戳_unix: {timestamp}")
        .map_err(|error| format!("写入崩溃时间戳失败：{error}"))?;
    writeln!(file, "操作系统: {}", std::env::consts::OS)
        .map_err(|error| format!("写入操作系统信息失败：{error}"))?;
    writeln!(file, "架构: {}", std::env::consts::ARCH)
        .map_err(|error| format!("写入架构信息失败：{error}"))?;
    writeln!(file, "系统族: {}", std::env::consts::FAMILY)
        .map_err(|error| format!("写入系统族信息失败：{error}"))?;
    writeln!(
        file,
        "当前目录: {}",
        std::env::current_dir()
            .ok()
            .map(|v| v.display().to_string())
            .unwrap_or_else(|| "未知".to_string())
    )
    .map_err(|error| format!("写入当前目录失败：{error}"))?;
    writeln!(
        file,
        "可执行文件路径: {}",
        std::env::current_exe()
            .ok()
            .map(|v| v.display().to_string())
            .unwrap_or_else(|| "未知".to_string())
    )
    .map_err(|error| format!("写入可执行文件路径失败：{error}"))?;
    writeln!(
        file,
        "用户名: {}",
        std::env::var("USERNAME").unwrap_or_else(|_| "未知".to_string())
    )
    .map_err(|error| format!("写入用户名失败：{error}"))?;
    writeln!(
        file,
        "计算机名: {}",
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "未知".to_string())
    )
    .map_err(|error| format!("写入计算机名失败：{error}"))?;
    writeln!(file).map_err(|error| format!("写入空行失败：{error}"))?;

    writeln!(file, "崩溃信息: {}", panic_payload(panic_info))
        .map_err(|error| format!("写入崩溃信息失败：{error}"))?;
    if let Some(location) = panic_info.location() {
        writeln!(
            file,
            "崩溃位置: {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        )
        .map_err(|error| format!("写入崩溃位置失败：{error}"))?;
    } else {
        writeln!(file, "崩溃位置: 未知")
            .map_err(|error| format!("写入崩溃位置失败：{error}"))?;
    }
    writeln!(file).map_err(|error| format!("写入空行失败：{error}"))?;

    let backtrace = Backtrace::force_capture();
    writeln!(file, "调用栈:\n{backtrace}")
        .map_err(|error| format!("写入调用栈失败：{error}"))?;
    writeln!(file).map_err(|error| format!("写入空行失败：{error}"))?;

    writeln!(file, "latest.log 末尾片段:")
        .map_err(|error| format!("写入日志末尾标题失败：{error}"))?;
    for line in tail_lines(latest_log_path, 300) {
        writeln!(file, "{line}")
            .map_err(|error| format!("写入日志末尾行失败：{error}"))?;
    }

    Ok(())
}

fn panic_payload(panic_info: &PanicHookInfo<'_>) -> String {
    if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
        return (*message).to_string();
    }

    if let Some(message) = panic_info.payload().downcast_ref::<String>() {
        return message.clone();
    }

    "非字符串类型的崩溃载荷".to_string()
}

fn tail_lines(path: &Path, max_lines: usize) -> Vec<String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return vec!["<无法读取 latest.log>".to_string()],
    };

    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader
        .lines()
        .map_while(Result::ok)
        .collect();

    if lines.len() > max_lines {
        let start = lines.len() - max_lines;
        lines = lines.split_off(start);
    }

    lines
}

fn timestamp_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

fn timestamp_for_file_name() -> String {
    Local::now().format("%Y-%m-%d_%H-%M-%S").to_string()
}
