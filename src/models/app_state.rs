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

        state.reset_cleaning_state();

        assert!(!state.is_cleaning);
        assert!(state.current_plugin.is_none());
        assert_eq!(state.progress, 0);
        assert_eq!(state.total_plugins, 0);
        assert!(state.cleaned_plugins.is_empty());
    }
}
