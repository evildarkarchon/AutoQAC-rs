// State management module
//
// This module provides the StateManager which wraps AppState with thread-safe access
// using Arc<RwLock<T>> and emits change events for GUI updates.

use crate::models::AppState;
use camino::Utf8PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

/// Change events emitted when state is modified
///
/// These events are emitted to notify interested parties (primarily the GUI)
/// about state changes without requiring them to poll the state.
#[derive(Clone, Debug, PartialEq)]
pub enum StateChange {
    /// Configuration has been updated
    ConfigurationChanged {
        is_fully_configured: bool,
    },

    /// Progress has been updated during cleaning
    ProgressUpdated {
        current: usize,
        total: usize,
        current_plugin: Option<String>,
    },

    /// Cleaning process has started
    CleaningStarted {
        total_plugins: usize,
    },

    /// Cleaning process has finished
    CleaningFinished {
        cleaned: usize,
        failed: usize,
        skipped: usize,
    },

    /// A plugin has been processed
    PluginProcessed {
        plugin: String,
        status: String,
        message: String,
    },

    /// Current operation has changed
    OperationChanged {
        operation: String,
    },

    /// Settings have been updated
    SettingsChanged,

    /// State has been reset
    StateReset,
}

/// Thread-safe state manager with event emission
///
/// This is the central state management component that:
/// - Provides thread-safe access to [`AppState`] via `Arc<RwLock<T>>`
/// - Detects state changes and emits [`StateChange`] events
/// - Validates state transitions
/// - Supports subscribing to state changes via tokio broadcast channels
///
/// # Usage
///
/// Always use `StateManager` instead of accessing [`AppState`] directly:
/// - [`read()`](Self::read) for reading state without locking
/// - [`update()`](Self::update) for mutations with automatic event emission
/// - [`subscribe()`](Self::subscribe) for listening to state changes
///
/// # Related Types
///
/// - [`crate::models::AppState`]: The underlying state structure
/// - [`StateChange`]: Event types emitted on state mutations
/// - [`crate::config::ConfigManager`]: Loads configurations into state
/// - [`crate::ui::controller::GuiController`]: Primary consumer of state events
pub struct StateManager {
    /// The application state protected by RwLock for thread-safe access
    state: Arc<RwLock<AppState>>,

    /// Broadcast channel for emitting state change events
    /// Multiple subscribers can listen for state changes
    state_tx: broadcast::Sender<StateChange>,
}

impl StateManager {
    /// Create a new StateManager with default state
    ///
    /// # Returns
    /// A new StateManager with a broadcast channel buffer of 100 events
    pub fn new() -> Self {
        let (state_tx, _) = broadcast::channel(100);
        Self {
            state: Arc::new(RwLock::new(AppState::default())),
            state_tx,
        }
    }

    /// Get a read-only snapshot of the current state
    ///
    /// This clones the entire state, so it's safe to use without holding locks.
    /// For checking individual fields, consider using `read()` with a closure.
    pub fn snapshot(&self) -> AppState {
        self.state.read().unwrap().clone()
    }

    /// Execute a function with read access to the state
    ///
    /// # Example
    /// ```ignore
    /// let is_configured = state_manager.read(|state| state.is_fully_configured());
    /// ```
    pub fn read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&AppState) -> R,
    {
        let state = self.state.read().unwrap();
        f(&state)
    }

    /// Update the state and emit change events
    ///
    /// This is the primary way to modify state. It:
    /// 1. Captures the old state
    /// 2. Applies the update function
    /// 3. Detects what changed
    /// 4. Emits appropriate events
    ///
    /// # Arguments
    /// * `update_fn` - A function that mutates the state
    ///
    /// # Returns
    /// A vector of StateChange events that were emitted
    ///
    /// # Example
    /// ```ignore
    /// state_manager.update(|state| {
    ///     state.is_cleaning = true;
    ///     state.progress = 0;
    /// });
    /// ```
    pub fn update<F>(&self, update_fn: F) -> Vec<StateChange>
    where
        F: FnOnce(&mut AppState),
    {
        let mut state = self.state.write().unwrap();
        let old_state = state.clone();

        // Apply the update
        update_fn(&mut state);

        // Detect changes and emit events
        let changes = self.detect_changes(&old_state, &state);

        for change in &changes {
            // Ignore send errors - it's OK if no one is listening
            let _ = self.state_tx.send(change.clone());
        }

        changes
    }

    /// Subscribe to state change events
    ///
    /// Returns a receiver that will get notified of all future state changes.
    /// Multiple subscribers can listen simultaneously.
    pub fn subscribe(&self) -> broadcast::Receiver<StateChange> {
        self.state_tx.subscribe()
    }

    /// Detect what changed between two states and generate events
    ///
    /// This is called internally by `update()` to determine which events to emit.
    fn detect_changes(&self, old: &AppState, new: &AppState) -> Vec<StateChange> {
        let mut changes = Vec::new();

        // Configuration changes
        if old.is_load_order_configured != new.is_load_order_configured
            || old.is_mo2_configured != new.is_mo2_configured
            || old.is_xedit_configured != new.is_xedit_configured
        {
            changes.push(StateChange::ConfigurationChanged {
                is_fully_configured: new.is_fully_configured(),
            });
        }

        // Cleaning state changes
        if old.is_cleaning != new.is_cleaning {
            if new.is_cleaning {
                changes.push(StateChange::CleaningStarted {
                    total_plugins: new.total_plugins,
                });
            } else {
                changes.push(StateChange::CleaningFinished {
                    cleaned: new.cleaned_plugins.len(),
                    failed: new.failed_plugins.len(),
                    skipped: new.skipped_plugins.len(),
                });
            }
        }

        // Progress changes
        if old.progress != new.progress
            || old.total_plugins != new.total_plugins
            || old.current_plugin != new.current_plugin
        {
            changes.push(StateChange::ProgressUpdated {
                current: new.progress,
                total: new.total_plugins,
                current_plugin: new.current_plugin.clone(),
            });
        }

        // Operation changes
        if old.current_operation != new.current_operation {
            changes.push(StateChange::OperationChanged {
                operation: new.current_operation.clone(),
            });
        }

        // Settings changes (checking all settings fields)
        if old.journal_expiration != new.journal_expiration
            || old.cleaning_timeout != new.cleaning_timeout
            || old.cpu_threshold != new.cpu_threshold
            || old.mo2_mode != new.mo2_mode
            || old.partial_forms_enabled != new.partial_forms_enabled
        {
            changes.push(StateChange::SettingsChanged);
        }

        changes
    }

    // Convenience methods for common state updates

    /// Set the load order path and update configuration status
    pub fn set_load_order_path(&self, path: Option<Utf8PathBuf>) -> Vec<StateChange> {
        self.update(|state| {
            state.load_order_path = path.clone();
            state.is_load_order_configured = path.is_some();
        })
    }

    /// Set the xEdit executable path and update configuration status
    pub fn set_xedit_exe_path(&self, path: Option<Utf8PathBuf>) -> Vec<StateChange> {
        self.update(|state| {
            state.xedit_exe_path = path.clone();
            state.is_xedit_configured = path.is_some();
        })
    }

    /// Set the MO2 executable path and update configuration status
    pub fn set_mo2_exe_path(&self, path: Option<Utf8PathBuf>) -> Vec<StateChange> {
        self.update(|state| {
            state.mo2_exe_path = path.clone();
            state.is_mo2_configured = path.is_some();
        })
    }

    /// Start a cleaning operation
    pub fn start_cleaning(&self, plugins: Vec<String>) -> Vec<StateChange> {
        self.update(|state| {
            state.is_cleaning = true;
            state.progress = 0;
            state.total_plugins = plugins.len();
            state.plugins_to_clean = plugins;
            state.current_plugin = None;
            state.current_operation = "Starting cleaning...".to_string();
            state.cleaned_plugins.clear();
            state.failed_plugins.clear();
            state.skipped_plugins.clear();
        })
    }

    /// Stop the cleaning operation
    pub fn stop_cleaning(&self) -> Vec<StateChange> {
        self.update(|state| {
            state.is_cleaning = false;
            state.current_plugin = None;
            state.current_operation.clear();
        })
    }

    /// Update progress for the current plugin
    pub fn update_progress(&self, plugin: String, operation: String) -> Vec<StateChange> {
        self.update(|state| {
            state.current_plugin = Some(plugin);
            state.current_operation = operation;
        })
    }

    /// Record the result of processing a plugin
    ///
    /// # Arguments
    /// * `plugin` - Name of the plugin that was processed
    /// * `status` - Status of the operation ("cleaned", "failed", or "skipped")
    /// * `message` - Human-readable message about the result
    /// * `stats` - Optional cleaning statistics (ITMs, UDRs, etc.)
    pub fn add_plugin_result(
        &self,
        plugin: String,
        status: &str,
        message: String,
        stats: Option<crate::services::cleaning::CleaningStats>,
    ) -> Vec<StateChange> {
        let mut changes = self.update(|state| {
            state.add_result(plugin.clone(), status);

            // Update statistics if provided
            if let Some(ref cleaning_stats) = stats {
                // Update current statistics from CleaningStats
                state.current_undeleted = cleaning_stats.undeleted;
                state.current_removed = cleaning_stats.removed;
                state.current_skipped = cleaning_stats.skipped;
                state.current_partial_forms = cleaning_stats.partial_forms;
                state.current_total_processed = cleaning_stats.undeleted
                    + cleaning_stats.removed
                    + cleaning_stats.skipped
                    + cleaning_stats.partial_forms;

                // Aggregate into totals
                state.aggregate_current_stats();
            }
        });

        // Emit a plugin processed event
        let plugin_event = StateChange::PluginProcessed {
            plugin,
            status: status.to_string(),
            message,
        };

        let _ = self.state_tx.send(plugin_event.clone());
        changes.push(plugin_event);

        changes
    }

    /// Reset all cleaning-related state
    pub fn reset_cleaning_state(&self) -> Vec<StateChange> {
        let mut changes = self.update(|state| {
            state.reset_cleaning_state();
        });

        // Emit a reset event
        let reset_event = StateChange::StateReset;
        let _ = self.state_tx.send(reset_event.clone());
        changes.push(reset_event);

        changes
    }

    /// Update settings
    pub fn update_settings<F>(&self, settings_fn: F) -> Vec<StateChange>
    where
        F: FnOnce(&mut AppState),
    {
        self.update(settings_fn)
    }

    /// Load configuration from UserConfig
    ///
    /// This populates AppState fields from the user configuration file,
    /// setting paths, timeouts, and user preferences.
    ///
    /// # Arguments
    /// * `user_config` - The loaded user configuration
    ///
    /// # Returns
    /// A vector of StateChange events that were emitted
    pub fn load_from_user_config(&self, user_config: &crate::models::UserConfig) -> Vec<StateChange> {
        use std::time::Duration;

        self.update(|state| {
            let settings = &user_config.pact_settings;

            // Load path configurations
            if !settings.loadorder_txt.is_empty() {
                state.load_order_path = Some(Utf8PathBuf::from(&settings.loadorder_txt));
                state.is_load_order_configured = true;
            }

            if !settings.xedit_exe.is_empty() {
                state.xedit_exe_path = Some(Utf8PathBuf::from(&settings.xedit_exe));
                state.is_xedit_configured = true;
            }

            if !settings.mo2_exe.is_empty() {
                state.mo2_exe_path = Some(Utf8PathBuf::from(&settings.mo2_exe));
                state.is_mo2_configured = true;
            }

            // Load settings
            state.partial_forms_enabled = settings.partial_forms;
            state.cleaning_timeout = Duration::from_secs(settings.cleaning_timeout as u64);
            state.journal_expiration = settings.journal_expiration;

            tracing::info!(
                "Loaded user config: load_order={}, xedit={}, mo2={}, partial_forms={}, timeout={}s",
                state.is_load_order_configured,
                state.is_xedit_configured,
                state.is_mo2_configured,
                state.partial_forms_enabled,
                settings.cleaning_timeout
            );
        })
    }

    /// Get an Arc reference to the state for use in worker threads
    ///
    /// Use this when you need to share state across threads but want
    /// to minimize cloning. Remember to use read/write locks appropriately.
    pub fn state_arc(&self) -> Arc<RwLock<AppState>> {
        Arc::clone(&self.state)
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

// Make StateManager cloneable for sharing across threads
impl Clone for StateManager {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            state_tx: self.state_tx.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_new_state_manager() {
        let manager = StateManager::new();
        let state = manager.snapshot();

        assert!(!state.is_cleaning);
        assert!(!state.is_fully_configured());
        assert_eq!(state.progress, 0);
    }

    #[test]
    fn test_update_with_change_detection() {
        let manager = StateManager::new();

        let changes = manager.update(|state| {
            state.is_cleaning = true;
            state.total_plugins = 10;
        });

        assert_eq!(changes.len(), 2);
        assert!(matches!(changes[0], StateChange::CleaningStarted { .. }));
        assert!(matches!(changes[1], StateChange::ProgressUpdated { .. }));
    }

    #[test]
    fn test_configuration_changes() {
        let manager = StateManager::new();

        let changes = manager.set_load_order_path(Some(Utf8PathBuf::from("/path/to/plugins.txt")));

        assert_eq!(changes.len(), 1);
        assert!(matches!(
            changes[0],
            StateChange::ConfigurationChanged { is_fully_configured: false }
        ));

        let state = manager.snapshot();
        assert!(state.is_load_order_configured);
        assert!(!state.is_fully_configured()); // Still need MO2 and xEdit
    }

    #[test]
    fn test_full_configuration_detection() {
        let manager = StateManager::new();

        manager.set_load_order_path(Some(Utf8PathBuf::from("/plugins.txt")));
        manager.set_xedit_exe_path(Some(Utf8PathBuf::from("/xedit.exe")));
        let changes = manager.set_mo2_exe_path(Some(Utf8PathBuf::from("/mo2.exe")));

        assert!(matches!(
            changes[0],
            StateChange::ConfigurationChanged { is_fully_configured: true }
        ));

        let state = manager.snapshot();
        assert!(state.is_fully_configured());
    }

    #[test]
    fn test_start_cleaning() {
        let manager = StateManager::new();
        let plugins = vec!["plugin1.esp".to_string(), "plugin2.esp".to_string()];

        let changes = manager.start_cleaning(plugins.clone());

        assert!(matches!(changes[0], StateChange::CleaningStarted { total_plugins: 2 }));

        let state = manager.snapshot();
        assert!(state.is_cleaning);
        assert_eq!(state.total_plugins, 2);
        assert_eq!(state.plugins_to_clean, plugins);
    }

    #[test]
    fn test_stop_cleaning() {
        let manager = StateManager::new();
        manager.start_cleaning(vec!["test.esp".to_string()]);

        let changes = manager.stop_cleaning();

        assert!(matches!(
            changes[0],
            StateChange::CleaningFinished { .. }
        ));

        let state = manager.snapshot();
        assert!(!state.is_cleaning);
    }

    #[test]
    fn test_update_progress() {
        let manager = StateManager::new();

        let changes = manager.update_progress(
            "plugin1.esp".to_string(),
            "Cleaning ITMs...".to_string(),
        );

        assert!(matches!(changes[0], StateChange::ProgressUpdated { .. }));
        assert!(matches!(changes[1], StateChange::OperationChanged { .. }));

        let state = manager.snapshot();
        assert_eq!(state.current_plugin, Some("plugin1.esp".to_string()));
        assert_eq!(state.current_operation, "Cleaning ITMs...");
    }

    #[test]
    fn test_add_plugin_result() {
        let manager = StateManager::new();
        manager.start_cleaning(vec!["plugin1.esp".to_string()]);

        let changes = manager.add_plugin_result(
            "plugin1.esp".to_string(),
            "cleaned",
            "Removed 5 ITMs".to_string(),
            None,
        );

        // Should have progress update and plugin processed event
        assert!(changes.iter().any(|c| matches!(c, StateChange::PluginProcessed { .. })));

        let state = manager.snapshot();
        assert_eq!(state.cleaned_plugins.len(), 1);
        assert_eq!(state.progress, 1);
    }

    #[test]
    fn test_add_plugin_result_with_stats() {
        let manager = StateManager::new();
        manager.start_cleaning(vec!["plugin1.esp".to_string(), "plugin2.esp".to_string()]);

        // Create mock cleaning stats
        let stats1 = crate::services::cleaning::CleaningStats {
            undeleted: 3,
            removed: 5,
            skipped: 1,
            partial_forms: 0,
        };

        let changes = manager.add_plugin_result(
            "plugin1.esp".to_string(),
            "cleaned",
            "Removed 5 ITMs, undeleted 3 UDRs".to_string(),
            Some(stats1),
        );

        assert!(changes.iter().any(|c| matches!(c, StateChange::PluginProcessed { .. })));

        let state = manager.snapshot();
        assert_eq!(state.cleaned_plugins.len(), 1);
        assert_eq!(state.progress, 1);

        // Check current statistics
        assert_eq!(state.current_undeleted, 3);
        assert_eq!(state.current_removed, 5);
        assert_eq!(state.current_skipped, 1);
        assert_eq!(state.current_total_processed, 9);

        // Check aggregate statistics
        assert_eq!(state.total_undeleted, 3);
        assert_eq!(state.total_removed, 5);
        assert_eq!(state.total_skipped, 1);
        assert_eq!(state.total_records_processed, 9);

        // Process second plugin
        let stats2 = crate::services::cleaning::CleaningStats {
            undeleted: 2,
            removed: 7,
            skipped: 0,
            partial_forms: 1,
        };

        manager.add_plugin_result(
            "plugin2.esp".to_string(),
            "cleaned",
            "Removed 7 ITMs, undeleted 2 UDRs, 1 partial form".to_string(),
            Some(stats2),
        );

        let state = manager.snapshot();

        // Current stats should be from the last plugin
        assert_eq!(state.current_undeleted, 2);
        assert_eq!(state.current_removed, 7);
        assert_eq!(state.current_partial_forms, 1);
        assert_eq!(state.current_total_processed, 10);

        // Aggregate stats should be sum of both plugins
        assert_eq!(state.total_undeleted, 5);
        assert_eq!(state.total_removed, 12);
        assert_eq!(state.total_skipped, 1);
        assert_eq!(state.total_partial_forms, 1);
        assert_eq!(state.total_records_processed, 19);
    }

    #[test]
    fn test_reset_cleaning_state() {
        let manager = StateManager::new();
        manager.start_cleaning(vec!["test.esp".to_string()]);
        manager.add_plugin_result("test.esp".to_string(), "cleaned", "Done".to_string(), None);

        let changes = manager.reset_cleaning_state();

        assert!(changes.iter().any(|c| matches!(c, StateChange::StateReset)));

        let state = manager.snapshot();
        assert!(!state.is_cleaning);
        assert_eq!(state.progress, 0);
        assert_eq!(state.total_plugins, 0);
        assert!(state.cleaned_plugins.is_empty());
    }

    #[test]
    fn test_settings_change_detection() {
        let manager = StateManager::new();

        let changes = manager.update_settings(|state| {
            state.cleaning_timeout = Duration::from_secs(600);
            state.cpu_threshold = 10;
        });

        assert!(matches!(changes[0], StateChange::SettingsChanged));

        let state = manager.snapshot();
        assert_eq!(state.cleaning_timeout, Duration::from_secs(600));
        assert_eq!(state.cpu_threshold, 10);
    }

    #[test]
    fn test_subscribe_to_changes() {
        let manager = StateManager::new();
        let mut rx = manager.subscribe();

        // Make a change
        manager.update(|state| {
            state.is_cleaning = true;
        });

        // Should receive the event
        let event = rx.try_recv();
        assert!(event.is_ok());
        assert!(matches!(event.unwrap(), StateChange::CleaningStarted { .. }));
    }

    #[test]
    fn test_multiple_subscribers() {
        let manager = StateManager::new();
        let mut rx1 = manager.subscribe();
        let mut rx2 = manager.subscribe();

        manager.start_cleaning(vec!["test.esp".to_string()]);

        // Both subscribers should receive the event
        assert!(rx1.try_recv().is_ok());
        assert!(rx2.try_recv().is_ok());
    }

    #[test]
    fn test_read_with_closure() {
        let manager = StateManager::new();
        manager.update(|state| {
            state.progress = 42;
        });

        let progress = manager.read(|state| state.progress);
        assert_eq!(progress, 42);
    }

    #[test]
    fn test_clone_state_manager() {
        let manager1 = StateManager::new();
        let manager2 = manager1.clone();

        // Update through one manager
        manager1.update(|state| {
            state.progress = 10;
        });

        // Changes should be visible through the clone
        let state = manager2.snapshot();
        assert_eq!(state.progress, 10);
    }

    #[test]
    fn test_state_arc() {
        let manager = StateManager::new();
        let state_arc = manager.state_arc();

        // Modify through the Arc
        {
            let mut state = state_arc.write().unwrap();
            state.progress = 99;
        }

        // Changes should be visible through manager
        let state = manager.snapshot();
        assert_eq!(state.progress, 99);
    }
}
