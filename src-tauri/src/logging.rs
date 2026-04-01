//! Structured file logging with size-based rotation, age-based retention, and dynamic log level

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use rolling_file::{BasicRollingFileAppender, RollingConditionBasic};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::reload;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Registry};

use crate::config::schema::LoggingConfig;
use crate::error::AppError;

/// Type-erased reload handle for dynamic log level changes.
/// We store a closure that captures the concrete handle type.
type LogLevelUpdaterFn = dyn Fn(&str) -> Result<(), String> + Send;
static LOG_LEVEL_UPDATER: OnceLock<Mutex<Box<LogLevelUpdaterFn>>> = OnceLock::new();

/// Non-blocking writer guard — must live for the app's lifetime
static _GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

/// Initialize the logging subsystem.
///
/// - Writes to `~/.pisum-transcript/logs/pisum-transcript.log`
/// - Rotates when file exceeds `config.log_max_file_size_mb`
/// - Deletes log files older than `config.log_retention_days` on startup
/// - Outputs to stdout in debug builds
pub fn init(config: &LoggingConfig) -> Result<(), AppError> {
    let log_dir = log_dir();
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| AppError::Config(format!("Failed to create log directory: {}", e)))?;

    // Clean up old log files
    cleanup_old_logs(&log_dir, config.log_retention_days);

    // Size-based rolling file appender
    let max_bytes = config.log_max_file_size_mb as u64 * 1024 * 1024;
    let condition = RollingConditionBasic::new().max_size(max_bytes);
    let file_appender = BasicRollingFileAppender::new(
        log_dir.join("pisum-transcript.log"),
        condition,
        10, // keep up to 10 rotated files; age-based cleanup handles the rest
    )
    .map_err(|e| AppError::Config(format!("Failed to create log appender: {}", e)))?;

    // Non-blocking writer
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let _ = _GUARD.set(guard);

    // Reloadable filter layer
    let filter = build_filter(&config.log_level);
    let (filter_layer, reload_handle) = reload::Layer::new(filter);

    // Store a type-erased updater closure
    let _ = LOG_LEVEL_UPDATER.set(Mutex::new(Box::new(move |level: &str| {
        let new_filter = build_filter(level);
        reload_handle
            .reload(new_filter)
            .map_err(|e| format!("{}", e))
    })));

    // File fmt layer
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_target(true)
        .with_ansi(false);

    // Optional stdout layer for dev mode
    let stdout_layer = if cfg!(debug_assertions) {
        Some(fmt::layer().with_target(true).with_ansi(true))
    } else {
        None
    };

    Registry::default()
        .with(filter_layer)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    Ok(())
}

/// Change the log level at runtime without restarting the app.
pub fn set_log_level(level: &str) -> Result<(), AppError> {
    let updater = LOG_LEVEL_UPDATER
        .get()
        .ok_or_else(|| AppError::Config("Logging not initialized".to_string()))?;

    let updater = updater.lock().unwrap();
    (updater)(level).map_err(|e| AppError::Config(format!("Failed to reload log level: {}", e)))
}

/// Get the log directory path.
pub fn log_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pisum-transcript")
        .join("logs")
}

/// Build an EnvFilter from a level string, defaulting to "info" on failure.
fn build_filter(level: &str) -> EnvFilter {
    EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"))
}

/// Delete log files older than `retention_days` from the given directory.
fn cleanup_old_logs(dir: &Path, retention_days: u32) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(retention_days as u64 * 24 * 60 * 60);

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Only clean up .log files (includes rotated files like pisum-transcript.log.1)
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.contains(".log") {
            continue;
        }

        if let Ok(metadata) = path.metadata() {
            if let Ok(modified) = metadata.modified() {
                if modified < cutoff {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }
}
