use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize console-based logging.
pub fn init() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,pisum_transcript_lib=debug"));

    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .init();

    tracing::info!("Logging initialized");
}
