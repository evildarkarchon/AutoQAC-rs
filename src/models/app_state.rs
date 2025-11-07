use camino::Utf8PathBuf;
use std::collections::HashSet;
use std::time::Duration;

/// Maximum number of concurrent xEdit subprocesses.
///
/// **IMPORTANT:** This is hardcoded to 1 because xEdit has file locking issues
/// that prevent multiple instances from running simultaneously. Running multiple
/// instances will not crash, but will result in undefined behavior due to
/// concurrent file access.
///
/// This constraint is enforced in the cleaning workflow (see [`crate::ui::GuiController`])
/// using a `tokio::sync::Semaphore` to serialize execution.
///
/// # See Also
///
/// - [`crate::services::cleaning::CleaningService`] - The service that executes xEdit commands
/// - [`crate::ui::GuiController`] - Orchestrates the cleaning workflow with semaphore enforcement
pub const MAX_CONCURRENT_XEDIT_PROCESSES: usize = 1;

/// Single source of truth for all application state.
///
/// This struct mirrors the Python AppState dataclass and contains all
/// configuration, runtime state, progress tracking, and results.
///
/// # Thread Safety
///
/// `AppState` is wrapped in `Arc<RwLock<AppState>>` by [`crate::state::StateManager`]
/// to provide thread-safe access across the application. Never access `AppState`
/// directly - always use [`StateManager`](crate::state::StateManager) methods:
/// - [`read()`](crate::state::StateManager::read) for read-only access
/// - [`update()`](crate::state::StateManager::update) for mutations with automatic change events
///
/// # Related Types
///
/// - [`crate::state::StateManager`]: Thread-safe wrapper with event emission
/// - [`crate::state::StateChange`]: Event types for state mutations
/// - [`crate::models::UserConfig`]: User configuration loaded from YAML
/// - [`crate::models::MainConfig`]: Game and xEdit configurations
#[derive(Clone, Debug)]
pub struct AppState {
    // Configuration paths
    pub load_order_path: Option<Utf8PathBuf>,
    pub mo2_exe_path: Option<Utf8PathBuf>,
    pub mo2_install_path: Option<Utf8PathBuf>,
    pub xedit_exe_path: Option<Utf8PathBuf>,
    pub xedit_install_path: Option<Utf8PathBuf>,

    // Configuration validity flags
    pub is_load_order_configured: bool,
    pub is_mo2_configured: bool,
    pub is_xedit_configured: bool,

    // Runtime state
    pub is_cleaning: bool,
    pub current_plugin: Option<String>,
    pub current_operation: String,

    // Progress state
    pub progress: usize,
    pub total_plugins: usize,
    pub plugins_to_clean: Vec<String>,

    // Results
    pub cleaned_plugins: HashSet<String>,
    pub failed_plugins: HashSet<String>,
    pub skipped_plugins: HashSet<String>,

    // Per-plugin record statistics (reset for each plugin)
    pub current_undeleted: usize,    // UDRs (Undeleted References)
    pub current_removed: usize,      // ITMs (Identical To Master)
    pub current_skipped: usize,      // Skipped records
    pub current_partial_forms: usize,
    pub current_total_processed: usize,

    // Aggregate statistics across all plugins
    pub total_undeleted: usize,
    pub total_removed: usize,
    pub total_skipped: usize,
    pub total_partial_forms: usize,
    pub total_records_processed: usize,

    // Settings
    pub journal_expiration: u32,
    pub cleaning_timeout: Duration,
    pub cpu_threshold: u32,
    pub mo2_mode: bool,
    pub partial_forms_enabled: bool,
    pub game_type: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            // Configuration paths
            load_order_path: None,
            mo2_exe_path: None,
            mo2_install_path: None,
            xedit_exe_path: None,
            xedit_install_path: None,

            // Configuration validity
            is_load_order_configured: false,
            is_mo2_configured: false,
            is_xedit_configured: false,

            // Runtime state
            is_cleaning: false,
            current_plugin: None,
            current_operation: String::new(),

            // Progress state
            progress: 0,
            total_plugins: 0,
            plugins_to_clean: Vec::new(),

            // Results
            cleaned_plugins: HashSet::new(),
            failed_plugins: HashSet::new(),
            skipped_plugins: HashSet::new(),

            // Per-plugin record statistics
            current_undeleted: 0,
            current_removed: 0,
            current_skipped: 0,
            current_partial_forms: 0,
            current_total_processed: 0,

            // Aggregate statistics
            total_undeleted: 0,
            total_removed: 0,
            total_skipped: 0,
            total_partial_forms: 0,
            total_records_processed: 0,

            // Settings with defaults matching Python version
            journal_expiration: 7,
            cleaning_timeout: Duration::from_secs(300),
            cpu_threshold: 5,
            mo2_mode: false,
            partial_forms_enabled: false,
            game_type: None,
        }
    }
}

impl AppState {
    /// Check if all required configuration is present.
    ///
    /// This mirrors the Python property `is_fully_configured`.
    pub fn is_fully_configured(&self) -> bool {
        self.is_load_order_configured
            && self.is_mo2_configured
            && self.is_xedit_configured
    }

    /// Get current cleaning statistics.
    ///
    /// Returns a tuple of (cleaned, failed, skipped, total).
    pub fn cleaning_stats(&self) -> (usize, usize, usize, usize) {
        (
            self.cleaned_plugins.len(),
            self.failed_plugins.len(),
            self.skipped_plugins.len(),
            self.total_plugins,
        )
    }

    /// Reset all cleaning-related state to initial values.
    ///
    /// This mirrors the Python method `reset_cleaning_state`.
    pub fn reset_cleaning_state(&mut self) {
        self.is_cleaning = false;
        self.current_plugin = None;
        self.current_operation.clear();
        self.progress = 0;
        self.total_plugins = 0;
        self.plugins_to_clean.clear();
        self.cleaned_plugins.clear();
        self.failed_plugins.clear();
        self.skipped_plugins.clear();

        // Reset statistics
        self.reset_current_stats();
        self.total_undeleted = 0;
        self.total_removed = 0;
        self.total_skipped = 0;
        self.total_partial_forms = 0;
        self.total_records_processed = 0;
    }

    /// Reset per-plugin statistics before processing a new plugin.
    pub fn reset_current_stats(&mut self) {
        self.current_undeleted = 0;
        self.current_removed = 0;
        self.current_skipped = 0;
        self.current_partial_forms = 0;
        self.current_total_processed = 0;
    }

    /// Aggregate current plugin statistics into totals.
    ///
    /// Call this after completing processing of a plugin to add its
    /// statistics to the running totals.
    pub fn aggregate_current_stats(&mut self) {
        self.total_undeleted += self.current_undeleted;
        self.total_removed += self.current_removed;
        self.total_skipped += self.current_skipped;
        self.total_partial_forms += self.current_partial_forms;
        self.total_records_processed += self.current_total_processed;
    }

    /// Increment a specific statistic counter.
    ///
    /// This is used during xEdit output parsing to track individual operations.
    pub fn increment_stat(&mut self, stat_type: &str) {
        match stat_type {
            "undeleted" => {
                self.current_undeleted += 1;
                self.current_total_processed += 1;
            }
            "removed" => {
                self.current_removed += 1;
                self.current_total_processed += 1;
            }
            "skipped" => {
                self.current_skipped += 1;
                self.current_total_processed += 1;
            }
            "partial_forms" => {
                self.current_partial_forms += 1;
                self.current_total_processed += 1;
            }
            _ => {
                // Unknown stat type - ignore
            }
        }
    }

    /// Get a formatted string summarizing current plugin statistics.
    ///
    /// Returns an empty string if no records were processed.
    pub fn current_stats_summary(&self) -> String {
        if self.current_total_processed == 0 {
            return String::new();
        }

        let mut parts = Vec::new();

        if self.current_undeleted > 0 {
            parts.push(format!("{} undeleted", self.current_undeleted));
        }
        if self.current_removed > 0 {
            parts.push(format!("{} removed", self.current_removed));
        }
        if self.current_skipped > 0 {
            parts.push(format!("{} skipped", self.current_skipped));
        }
        if self.current_partial_forms > 0 {
            parts.push(format!("{} partial forms", self.current_partial_forms));
        }

        if parts.is_empty() {
            format!(" ({} items processed)", self.current_total_processed)
        } else {
            format!(" ({})", parts.join(", "))
        }
    }

    /// Get a formatted string summarizing total statistics across all plugins.
    pub fn total_stats_summary(&self) -> String {
        if self.total_records_processed == 0 {
            return String::new();
        }

        let mut parts = Vec::new();

        if self.total_undeleted > 0 {
            parts.push(format!("{} undeleted", self.total_undeleted));
        }
        if self.total_removed > 0 {
            parts.push(format!("{} removed", self.total_removed));
        }
        if self.total_skipped > 0 {
            parts.push(format!("{} skipped", self.total_skipped));
        }
        if self.total_partial_forms > 0 {
            parts.push(format!("{} partial forms", self.total_partial_forms));
        }

        if parts.is_empty() {
            format!("Total: {} items processed", self.total_records_processed)
        } else {
            format!("Total: {}", parts.join(", "))
        }
    }

    /// Add a plugin processing result.
    ///
    /// This mirrors the Python method `add_result`.
    pub fn add_result(&mut self, plugin: String, status: &str) {
        match status {
            "cleaned" => {
                self.cleaned_plugins.insert(plugin);
            }
            "failed" => {
                self.failed_plugins.insert(plugin);
            }
            "skipped" => {
                self.skipped_plugins.insert(plugin);
            }
            _ => {
                // Unknown status - could log a warning here
            }
        }
        self.progress += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = AppState::default();
        assert!(!state.is_fully_configured());
        assert_eq!(state.cleaning_timeout, Duration::from_secs(300));
        // MAX_CONCURRENT_XEDIT_PROCESSES is a module-level constant, not in AppState
        assert_eq!(MAX_CONCURRENT_XEDIT_PROCESSES, 1);
    }

    #[test]
    fn test_is_fully_configured() {
        let mut state = AppState::default();
        assert!(!state.is_fully_configured());

        state.is_load_order_configured = true;
        state.is_mo2_configured = true;
        state.is_xedit_configured = true;
        assert!(state.is_fully_configured());
    }

    #[test]
    fn test_cleaning_stats() {
        let mut state = AppState::default();
        state.total_plugins = 10;
        state.cleaned_plugins.insert("plugin1.esp".to_string());
        state.failed_plugins.insert("plugin2.esp".to_string());
        state.skipped_plugins.insert("plugin3.esp".to_string());

        let (cleaned, failed, skipped, total) = state.cleaning_stats();
        assert_eq!(cleaned, 1);
        assert_eq!(failed, 1);
        assert_eq!(skipped, 1);
        assert_eq!(total, 10);
    }

    #[test]
    fn test_add_result() {
        let mut state = AppState::default();
        state.add_result("plugin1.esp".to_string(), "cleaned");
        state.add_result("plugin2.esp".to_string(), "failed");
        state.add_result("plugin3.esp".to_string(), "skipped");

        assert_eq!(state.cleaned_plugins.len(), 1);
        assert_eq!(state.failed_plugins.len(), 1);
        assert_eq!(state.skipped_plugins.len(), 1);
        assert_eq!(state.progress, 3);
    }

    #[test]
    fn test_reset_cleaning_state() {
        let mut state = AppState::default();
        state.is_cleaning = true;
        state.current_plugin = Some("test.esp".to_string());
        state.progress = 5;
        state.total_plugins = 10;
        state.cleaned_plugins.insert("plugin1.esp".to_string());

        // Add some statistics
        state.current_removed = 10;
        state.total_removed = 20;

        state.reset_cleaning_state();

        assert!(!state.is_cleaning);
        assert!(state.current_plugin.is_none());
        assert_eq!(state.progress, 0);
        assert_eq!(state.total_plugins, 0);
        assert!(state.cleaned_plugins.is_empty());

        // Verify statistics are reset
        assert_eq!(state.current_removed, 0);
        assert_eq!(state.total_removed, 0);
    }

    #[test]
    fn test_reset_current_stats() {
        let mut state = AppState::default();
        state.current_undeleted = 5;
        state.current_removed = 10;
        state.current_skipped = 2;
        state.current_partial_forms = 1;
        state.current_total_processed = 18;

        state.reset_current_stats();

        assert_eq!(state.current_undeleted, 0);
        assert_eq!(state.current_removed, 0);
        assert_eq!(state.current_skipped, 0);
        assert_eq!(state.current_partial_forms, 0);
        assert_eq!(state.current_total_processed, 0);
    }

    #[test]
    fn test_aggregate_current_stats() {
        let mut state = AppState::default();

        // First plugin
        state.current_undeleted = 5;
        state.current_removed = 10;
        state.current_skipped = 2;
        state.current_partial_forms = 1;
        state.current_total_processed = 18;
        state.aggregate_current_stats();

        assert_eq!(state.total_undeleted, 5);
        assert_eq!(state.total_removed, 10);
        assert_eq!(state.total_skipped, 2);
        assert_eq!(state.total_partial_forms, 1);
        assert_eq!(state.total_records_processed, 18);

        // Second plugin
        state.reset_current_stats();
        state.current_undeleted = 3;
        state.current_removed = 7;
        state.current_total_processed = 10;
        state.aggregate_current_stats();

        assert_eq!(state.total_undeleted, 8);
        assert_eq!(state.total_removed, 17);
        assert_eq!(state.total_skipped, 2);
        assert_eq!(state.total_partial_forms, 1);
        assert_eq!(state.total_records_processed, 28);
    }

    #[test]
    fn test_increment_stat() {
        let mut state = AppState::default();

        state.increment_stat("undeleted");
        assert_eq!(state.current_undeleted, 1);
        assert_eq!(state.current_total_processed, 1);

        state.increment_stat("removed");
        state.increment_stat("removed");
        assert_eq!(state.current_removed, 2);
        assert_eq!(state.current_total_processed, 3);

        state.increment_stat("skipped");
        assert_eq!(state.current_skipped, 1);
        assert_eq!(state.current_total_processed, 4);

        state.increment_stat("partial_forms");
        assert_eq!(state.current_partial_forms, 1);
        assert_eq!(state.current_total_processed, 5);

        // Unknown stat type should be ignored
        state.increment_stat("unknown");
        assert_eq!(state.current_total_processed, 5);
    }

    #[test]
    fn test_current_stats_summary() {
        let mut state = AppState::default();

        // No stats
        assert_eq!(state.current_stats_summary(), "");

        // Only removed
        state.current_removed = 5;
        state.current_total_processed = 5;
        assert_eq!(state.current_stats_summary(), " (5 removed)");

        // Multiple stats
        state.current_undeleted = 3;
        state.current_skipped = 1;
        state.current_total_processed = 9;
        assert_eq!(
            state.current_stats_summary(),
            " (3 undeleted, 5 removed, 1 skipped)"
        );

        // With partial forms
        state.current_partial_forms = 2;
        state.current_total_processed = 11;
        assert_eq!(
            state.current_stats_summary(),
            " (3 undeleted, 5 removed, 1 skipped, 2 partial forms)"
        );
    }

    #[test]
    fn test_total_stats_summary() {
        let mut state = AppState::default();

        // No stats
        assert_eq!(state.total_stats_summary(), "");

        // Only removed
        state.total_removed = 15;
        state.total_records_processed = 15;
        assert_eq!(state.total_stats_summary(), "Total: 15 removed");

        // Multiple stats
        state.total_undeleted = 8;
        state.total_skipped = 3;
        state.total_records_processed = 26;
        assert_eq!(
            state.total_stats_summary(),
            "Total: 8 undeleted, 15 removed, 3 skipped"
        );

        // With partial forms
        state.total_partial_forms = 4;
        state.total_records_processed = 30;
        assert_eq!(
            state.total_stats_summary(),
            "Total: 8 undeleted, 15 removed, 3 skipped, 4 partial forms"
        );
    }
}
