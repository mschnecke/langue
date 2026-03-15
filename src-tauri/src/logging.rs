use std::path::PathBuf;
use tracing_appender::rolling;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Returns the log directory path: `~/.pisum-transcript/logs/`
fn log_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Failed to determine home directory");
    home.join(".pisum-transcript").join("logs")
}

/// Initialize file-based logging with daily rotation.
///
/// - Log directory: `~/.pisum-transcript/logs/`
/// - Daily rotation, kept for 7 days (tracing-appender handles rotation;
///   cleanup of old files is best-effort via a simple sweep on startup)
/// - In debug builds, also logs to stdout
pub fn init() {
    let dir = log_dir();
    std::fs::create_dir_all(&dir).expect("Failed to create log directory");

    // Clean up log files older than 7 days
    cleanup_old_logs(&dir, 7);

    let file_appender = rolling::daily(&dir, "pisum-transcript.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Leak the guard so the appender lives for the entire process
    std::mem::forget(_guard);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,pisum_transcript_lib=debug"));

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true);

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer);

    // In debug mode, also log to console
    #[cfg(debug_assertions)]
    {
        let console_layer = fmt::layer()
            .with_target(true)
            .with_thread_ids(false);
        registry.with(console_layer).init();
    }

    #[cfg(not(debug_assertions))]
    {
        registry.init();
    }

    tracing::info!("Logging initialized, log directory: {}", dir.display());
}

/// Remove log files older than `max_age_days` days.
fn cleanup_old_logs(dir: &std::path::Path, max_age_days: u64) {
    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(max_age_days * 24 * 60 * 60);

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff {
                        let _ = std::fs::remove_file(entry.path());
                        tracing::debug!("Removed old log file: {:?}", entry.path());
                    }
                }
            }
        }
    }
}
