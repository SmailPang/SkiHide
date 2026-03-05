use std::{
    backtrace::Backtrace,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    panic::PanicHookInfo,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

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
        .map_err(|error| format!("failed to create logs directory: {error}"))?;

    let latest_log_path = logs_dir.join("latest.log");
    let latest_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&latest_log_path)
        .map_err(|error| format!("failed to open latest log file: {error}"))?;

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
        .map_err(|error| format!("failed to initialize tracing subscriber: {error}"))?;

    install_panic_hook(logs_dir.clone(), latest_log_path.clone());

    Ok(LoggingContext {
        logs_dir,
        latest_log_path,
        guard: Some(guard),
    })
}

pub fn archive_latest_log(latest_log_path: &Path, logs_dir: &Path) -> Result<PathBuf, String> {
    if !latest_log_path.exists() {
        return Err("latest.log does not exist".to_string());
    }

    let filename = format!("{}.log", timestamp_for_file_name());
    let archive_path = logs_dir.join(filename);

    fs::copy(latest_log_path, &archive_path)
        .map_err(|error| format!("failed to archive latest log: {error}"))?;

    Ok(archive_path)
}

fn resolve_logs_dir() -> Result<PathBuf, String> {
    let current_dir =
        std::env::current_dir().map_err(|error| format!("failed to get current dir: {error}"))?;
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
    fs::create_dir_all(logs_dir).map_err(|error| format!("failed to ensure logs dir: {error}"))?;
    let error_log_path = logs_dir.join("error.log");

    let mut file = File::create(&error_log_path)
        .map_err(|error| format!("failed to create error.log: {error}"))?;

    let timestamp = timestamp_seconds();
    writeln!(file, "SkiHide Crash Report")
        .map_err(|error| format!("failed to write crash report header: {error}"))?;
    writeln!(file, "timestamp_unix: {timestamp}")
        .map_err(|error| format!("failed to write crash timestamp: {error}"))?;
    writeln!(file, "os: {}", std::env::consts::OS)
        .map_err(|error| format!("failed to write os: {error}"))?;
    writeln!(file, "arch: {}", std::env::consts::ARCH)
        .map_err(|error| format!("failed to write arch: {error}"))?;
    writeln!(file, "family: {}", std::env::consts::FAMILY)
        .map_err(|error| format!("failed to write family: {error}"))?;
    writeln!(
        file,
        "current_dir: {}",
        std::env::current_dir()
            .ok()
            .map(|v| v.display().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    )
    .map_err(|error| format!("failed to write current_dir: {error}"))?;
    writeln!(
        file,
        "exe_path: {}",
        std::env::current_exe()
            .ok()
            .map(|v| v.display().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    )
    .map_err(|error| format!("failed to write exe_path: {error}"))?;
    writeln!(
        file,
        "username: {}",
        std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string())
    )
    .map_err(|error| format!("failed to write username: {error}"))?;
    writeln!(
        file,
        "computer_name: {}",
        std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".to_string())
    )
    .map_err(|error| format!("failed to write computer_name: {error}"))?;
    writeln!(file).map_err(|error| format!("failed to write spacer: {error}"))?;

    writeln!(file, "panic_message: {}", panic_payload(panic_info))
        .map_err(|error| format!("failed to write panic_message: {error}"))?;
    if let Some(location) = panic_info.location() {
        writeln!(
            file,
            "panic_location: {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        )
        .map_err(|error| format!("failed to write panic_location: {error}"))?;
    } else {
        writeln!(file, "panic_location: unknown")
            .map_err(|error| format!("failed to write panic_location: {error}"))?;
    }
    writeln!(file).map_err(|error| format!("failed to write spacer: {error}"))?;

    let backtrace = Backtrace::force_capture();
    writeln!(file, "backtrace:\n{backtrace}")
        .map_err(|error| format!("failed to write backtrace: {error}"))?;
    writeln!(file).map_err(|error| format!("failed to write spacer: {error}"))?;

    writeln!(file, "latest_log_tail:")
        .map_err(|error| format!("failed to write latest_log_tail header: {error}"))?;
    for line in tail_lines(latest_log_path, 300) {
        writeln!(file, "{line}")
            .map_err(|error| format!("failed to write latest_log_tail line: {error}"))?;
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

    "non-string panic payload".to_string()
}

fn tail_lines(path: &Path, max_lines: usize) -> Vec<String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return vec!["<unable to read latest.log>".to_string()],
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
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("run-{}-{:03}", now.as_secs(), now.subsec_millis())
}
