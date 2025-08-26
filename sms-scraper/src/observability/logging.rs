use std::fs;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initializes the logging system with both console and file output.
pub fn init_logging() {
    // Ensure logs directory exists
    let _ = fs::create_dir_all("logs");

    // Create a non-blocking file appender for daily log rotation
    let file_appender = tracing_appender::rolling::daily("logs", "scraper.log");
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);

    // Create a JSON layer for file logging
    let file_layer = fmt::layer().json().with_writer(non_blocking_writer);

    // Create a formatted layer for console logging
    let console_layer = fmt::layer()
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stdout);

    // Determine filter: respect RUST_LOG if set; otherwise default to verbose for our crate
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("sms_scraper=debug,info"));

    // Set the global default subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(console_layer)
        .init();

    // We need to keep the guard in scope to ensure logs are flushed on exit
    std::mem::forget(_guard);
}
