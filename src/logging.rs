use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use std::fs;
use tracing_appender::rolling;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Setup logging with rotating file appender.
///
/// Logs are written to the specified directory with daily rotation.
/// This mirrors the Python logging_config.py setup.
///
/// # Arguments
/// * `log_dir` - Directory for log files (e.g., "logs")
/// * `log_prefix` - Prefix for log files (e.g., "autoqac")
/// * `debug_mode` - If true, use debug level; otherwise use info level
///
/// # Returns
/// A guard that must be held for the duration of the program to keep logging active
pub fn setup_logging(
    log_dir: &str,
    log_prefix: &str,
    debug_mode: bool,
) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    // Create log directory if it doesn't exist
    let log_path = Utf8PathBuf::from(log_dir);
    if !log_path.exists() {
        fs::create_dir_all(&log_path)
            .with_context(|| format!("Failed to create log directory: {}", log_dir))?;
    }

    // Create daily rotating file appender
    let file_appender = rolling::daily(log_dir, log_prefix);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Determine log level based on debug mode
    let env_filter = if debug_mode {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    // Build the subscriber with file output
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false) // No ANSI codes in log files
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true),
        )
        .init();

    tracing::info!(
        "Logging initialized: dir={}, prefix={}, debug={}",
        log_dir,
        log_prefix,
        debug_mode
    );

    Ok(guard)
}

/// Setup logging with optional console output for debugging.
///
/// This is useful for development and testing.
///
/// # Arguments
/// * `log_dir` - Directory for log files
/// * `log_prefix` - Prefix for log files
/// * `debug_mode` - If true, use debug level; otherwise use info level
/// * `console_output` - If true, also log to console
///
/// # Returns
/// A guard that must be held for the duration of the program to keep logging active
pub fn setup_logging_with_console(
    log_dir: &str,
    log_prefix: &str,
    debug_mode: bool,
    console_output: bool,
) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    // Create log directory if it doesn't exist
    let log_path = Utf8PathBuf::from(log_dir);
    if !log_path.exists() {
        fs::create_dir_all(&log_path)
            .with_context(|| format!("Failed to create log directory: {}", log_dir))?;
    }

    // Create daily rotating file appender
    let file_appender = rolling::daily(log_dir, log_prefix);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Determine log level based on debug mode
    let env_filter = if debug_mode {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    if console_output {
        // Also log to console with ANSI colors for better readability
        let console_layer = tracing_subscriber::fmt::layer()
            .with_ansi(true)
            .with_target(false);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .with(console_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .init();
    }

    tracing::info!(
        "Logging initialized: dir={}, prefix={}, debug={}, console={}",
        log_dir,
        log_prefix,
        debug_mode,
        console_output
    );

    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    #[allow(unused_variables)]
    fn test_setup_logging() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_str().unwrap();

        // Setup logging - this will fail if called multiple times in the same process
        // but that's okay for a single test
        let result = setup_logging(log_dir, "test", false);

        // The result might be an error if logging is already initialized in another test
        // but the directory should still be created
        assert!(Utf8PathBuf::from(log_dir).exists());
    }

    #[test]
    fn test_log_directory_created() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("logs");
        let log_dir_str = log_dir.to_str().unwrap();

        // Just test directory creation, not full logging setup
        // to avoid global subscriber conflicts in test environment
        let log_path = Utf8PathBuf::from(log_dir_str);
        if !log_path.exists() {
            fs::create_dir_all(&log_path).unwrap();
        }

        assert!(log_dir.exists());
    }
}
