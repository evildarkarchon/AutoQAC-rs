// Performance metrics module
//
// Provides lightweight metrics tracking for monitoring application performance

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Global performance metrics
///
/// Uses atomic operations for thread-safe metric tracking without locks.
/// Metrics are collected throughout the application lifecycle and can be
/// logged periodically or on shutdown for performance analysis.
#[derive(Debug)]
pub struct Metrics {
    /// Total number of plugins successfully cleaned
    pub plugins_cleaned: AtomicUsize,

    /// Total number of plugins that failed to clean
    pub plugins_failed: AtomicUsize,

    /// Total number of plugins skipped
    pub plugins_skipped: AtomicUsize,

    /// Total cleaning time in milliseconds
    pub total_cleaning_time_ms: AtomicU64,

    /// Number of state updates performed
    pub state_updates: AtomicU64,

    /// Number of UI updates sent
    pub ui_updates: AtomicU64,

    /// Number of state broadcasts sent
    pub state_broadcasts: AtomicU64,

    /// Number of state broadcast errors (channel full or closed)
    pub state_broadcast_errors: AtomicU64,

    /// Number of UI update channel full errors
    pub ui_update_channel_full: AtomicU64,

    /// Application start time
    start_time: Instant,
}

impl Metrics {
    /// Create a new Metrics instance
    pub fn new() -> Self {
        Self {
            plugins_cleaned: AtomicUsize::new(0),
            plugins_failed: AtomicUsize::new(0),
            plugins_skipped: AtomicUsize::new(0),
            total_cleaning_time_ms: AtomicU64::new(0),
            state_updates: AtomicU64::new(0),
            ui_updates: AtomicU64::new(0),
            state_broadcasts: AtomicU64::new(0),
            state_broadcast_errors: AtomicU64::new(0),
            ui_update_channel_full: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Record a plugin cleaning operation
    pub fn record_plugin_cleaned(&self) {
        self.plugins_cleaned.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a plugin failure
    pub fn record_plugin_failed(&self) {
        self.plugins_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a plugin skip
    pub fn record_plugin_skipped(&self) {
        self.plugins_skipped.fetch_add(1, Ordering::Relaxed);
    }

    /// Record cleaning time for a plugin
    pub fn record_cleaning_time(&self, duration: Duration) {
        self.total_cleaning_time_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
    }

    /// Record a state update
    pub fn record_state_update(&self) {
        self.state_updates.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a UI update
    pub fn record_ui_update(&self) {
        self.ui_updates.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a state broadcast
    pub fn record_state_broadcast(&self) {
        self.state_broadcasts.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a state broadcast error
    pub fn record_state_broadcast_error(&self) {
        self.state_broadcast_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a UI update channel full error
    pub fn record_ui_channel_full(&self) {
        self.ui_update_channel_full.fetch_add(1, Ordering::Relaxed);
    }

    /// Get total uptime
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get average cleaning time per plugin in milliseconds
    pub fn avg_cleaning_time_ms(&self) -> f64 {
        let total = self.total_cleaning_time_ms.load(Ordering::Relaxed);
        let count = self.plugins_cleaned.load(Ordering::Relaxed);
        if count > 0 {
            total as f64 / count as f64
        } else {
            0.0
        }
    }

    /// Log metrics summary
    pub fn log_summary(&self) {
        let uptime = self.uptime();
        tracing::info!("=== Performance Metrics Summary ===");
        tracing::info!("Uptime: {:.2}s", uptime.as_secs_f64());
        tracing::info!(
            "Plugins: {} cleaned, {} failed, {} skipped",
            self.plugins_cleaned.load(Ordering::Relaxed),
            self.plugins_failed.load(Ordering::Relaxed),
            self.plugins_skipped.load(Ordering::Relaxed)
        );
        tracing::info!(
            "Total cleaning time: {:.2}s (avg: {:.2}ms per plugin)",
            self.total_cleaning_time_ms.load(Ordering::Relaxed) as f64 / 1000.0,
            self.avg_cleaning_time_ms()
        );
        tracing::info!(
            "State updates: {}, broadcasts: {}, errors: {}",
            self.state_updates.load(Ordering::Relaxed),
            self.state_broadcasts.load(Ordering::Relaxed),
            self.state_broadcast_errors.load(Ordering::Relaxed)
        );
        tracing::info!(
            "UI updates: {}, channel full errors: {}",
            self.ui_updates.load(Ordering::Relaxed),
            self.ui_update_channel_full.load(Ordering::Relaxed)
        );
    }

    /// Log periodic metrics (for long-running operations)
    pub fn log_periodic(&self) {
        tracing::info!(
            "Metrics: {} plugins processed, {} state updates, {} UI updates, uptime {:.0}s",
            self.plugins_cleaned.load(Ordering::Relaxed)
                + self.plugins_failed.load(Ordering::Relaxed)
                + self.plugins_skipped.load(Ordering::Relaxed),
            self.state_updates.load(Ordering::Relaxed),
            self.ui_updates.load(Ordering::Relaxed),
            self.uptime().as_secs_f64()
        );
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_metrics_creation() {
        let metrics = Metrics::new();
        assert_eq!(metrics.plugins_cleaned.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.plugins_failed.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_plugin_operations() {
        let metrics = Metrics::new();

        metrics.record_plugin_cleaned();
        metrics.record_plugin_cleaned();
        metrics.record_plugin_failed();
        metrics.record_plugin_skipped();

        assert_eq!(metrics.plugins_cleaned.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.plugins_failed.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.plugins_skipped.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_record_cleaning_time() {
        let metrics = Metrics::new();

        metrics.record_plugin_cleaned();
        metrics.record_cleaning_time(Duration::from_millis(100));
        metrics.record_plugin_cleaned();
        metrics.record_cleaning_time(Duration::from_millis(200));

        assert_eq!(metrics.total_cleaning_time_ms.load(Ordering::Relaxed), 300);
        assert_eq!(metrics.avg_cleaning_time_ms(), 150.0);
    }

    #[test]
    fn test_avg_cleaning_time_no_plugins() {
        let metrics = Metrics::new();
        assert_eq!(metrics.avg_cleaning_time_ms(), 0.0);
    }

    #[test]
    fn test_uptime() {
        let metrics = Metrics::new();
        thread::sleep(Duration::from_millis(10));
        assert!(metrics.uptime().as_millis() >= 10);
    }

    #[test]
    fn test_state_and_ui_counters() {
        let metrics = Metrics::new();

        metrics.record_state_update();
        metrics.record_ui_update();
        metrics.record_state_broadcast();
        metrics.record_state_broadcast_error();
        metrics.record_ui_channel_full();

        assert_eq!(metrics.state_updates.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.ui_updates.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.state_broadcasts.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.state_broadcast_errors.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.ui_update_channel_full.load(Ordering::Relaxed), 1);
    }
}
