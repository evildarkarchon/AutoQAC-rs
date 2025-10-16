// GUI Controller - Bridges Slint UI with Rust State Management
//
// This module contains the GuiController which coordinates between:
// - Slint UI (MainWindow)
// - StateManager (application state)
// - CleaningService (business logic)
// - EventLoopBridge (async/GUI coordination)
//
// It handles:
// - Setting up UI callbacks → async tasks
// - Subscribing to state changes → UI updates
// - File browser dialogs
// - Cleaning orchestration

use crate::models::MAX_CONCURRENT_XEDIT_PROCESSES;
use crate::services::cleaning::CleaningService;
use crate::state::{StateChange, StateManager};
use crate::ui::bridge::EventLoopBridge;
use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use std::sync::Arc;
use tokio::sync::{watch, Semaphore};

// Include the generated Slint code
slint::include_modules!();

/// GUI Controller that wires up the Slint UI with application state and logic
///
/// This is the main coordinator for the GUI layer. It:
/// - Creates and manages the EventLoopBridge for tokio/Slint coordination
/// - Sets up Slint callbacks to trigger async operations
/// - Subscribes to StateManager events and updates UI accordingly
/// - Handles file browser dialogs using the `rfd` crate
///
/// # Example
/// ```ignore
/// let state_manager = Arc::new(StateManager::new());
/// let runtime = tokio::runtime::Runtime::new()?;
///
/// let controller = GuiController::new(state_manager, runtime.handle().clone())?;
/// controller.run()?;  // Blocks until window is closed
/// ```
pub struct GuiController {
    /// The Slint UI window
    ui: MainWindow,

    /// Event loop bridge for coordinating between tokio and Slint
    _bridge: EventLoopBridge<MainWindow>,

    /// Shared state manager
    state_manager: Arc<StateManager>,

    /// Cancellation sender for graceful shutdown
    /// Send `true` to request cancellation of ongoing operations
    cancel_tx: watch::Sender<bool>,
}

impl GuiController {
    /// Create a new GUI controller
    ///
    /// # Arguments
    /// * `state_manager` - Shared application state manager
    /// * `tokio_handle` - Handle to the tokio runtime for spawning async tasks
    ///
    /// # Returns
    /// A new GuiController ready to run
    pub fn new(
        state_manager: Arc<StateManager>,
        tokio_handle: tokio::runtime::Handle,
    ) -> Result<Self> {
        // Create the Slint UI
        let ui = MainWindow::new().context("Failed to create Slint UI")?;

        // Create the event loop bridge
        let bridge = EventLoopBridge::new(&ui, tokio_handle);

        // Create cancellation channel for graceful shutdown
        let (cancel_tx, cancel_rx) = watch::channel(false);

        // Initialize UI with current state
        Self::sync_ui_with_state(&ui, &state_manager);

        // Set up Slint callbacks with cancellation receiver
        Self::setup_callbacks(&ui, &bridge, &state_manager, cancel_rx);

        // Subscribe to state changes and update UI
        Self::setup_state_subscription(&bridge, &state_manager);

        tracing::info!("GUI controller initialized");

        Ok(Self {
            ui,
            _bridge: bridge,
            state_manager,
            cancel_tx,
        })
    }

    /// Run the GUI (blocks until window is closed)
    ///
    /// This starts the Slint event loop and blocks until the user closes the window.
    pub fn run(self) -> Result<(), slint::PlatformError> {
        tracing::info!("Starting GUI event loop");
        self.ui.run()
    }

    /// Synchronize UI with current state
    ///
    /// This is called once at startup to initialize the UI with the current state.
    fn sync_ui_with_state(ui: &MainWindow, state_manager: &StateManager) {
        let state = state_manager.snapshot();

        // Set configuration paths
        ui.set_load_order_path(
            state
                .load_order_path
                .as_ref()
                .map(|p| p.as_str().to_string())
                .unwrap_or_default()
                .into(),
        );
        ui.set_xedit_exe_path(
            state
                .xedit_exe_path
                .as_ref()
                .map(|p| p.as_str().to_string())
                .unwrap_or_default()
                .into(),
        );
        ui.set_mo2_exe_path(
            state
                .mo2_exe_path
                .as_ref()
                .map(|p| p.as_str().to_string())
                .unwrap_or_default()
                .into(),
        );

        // Set runtime state
        ui.set_is_cleaning(state.is_cleaning);
        ui.set_progress_current(state.progress as i32);
        ui.set_progress_total(state.total_plugins as i32);
        ui.set_current_plugin(state.current_plugin.unwrap_or_default().into());
        ui.set_current_operation(state.current_operation.clone().into());

        // Set settings
        ui.set_mo2_mode(state.mo2_mode);
        ui.set_partial_forms_enabled(state.partial_forms_enabled);

        // Set results
        ui.set_cleaned_count(state.cleaned_plugins.len() as i32);
        ui.set_failed_count(state.failed_plugins.len() as i32);
        ui.set_skipped_count(state.skipped_plugins.len() as i32);

        tracing::debug!("UI synchronized with initial state");
    }

    /// Set up Slint UI callbacks
    ///
    /// This connects Slint UI events (button clicks, etc.) to Rust logic.
    fn setup_callbacks(
        ui: &MainWindow,
        bridge: &EventLoopBridge<MainWindow>,
        state_manager: &Arc<StateManager>,
        cancel_rx: watch::Receiver<bool>,
    ) {
        let bridge_handle = bridge.clone_handle();
        let state_manager_clone = Arc::clone(state_manager);
        let cancel_rx_clone = cancel_rx.clone();

        // Start cleaning callback
        ui.on_start_cleaning(move || {
            tracing::info!("Start cleaning button clicked");

            // Validate configuration
            if !state_manager_clone.read(|s| s.is_fully_configured()) {
                tracing::error!("Cannot start cleaning: configuration incomplete");
                return;
            }

            // Clone for async task
            let bridge = bridge_handle.clone();
            let bridge_clone = bridge.clone();
            let state = Arc::clone(&state_manager_clone);
            let cancel = cancel_rx_clone.clone();

            // Spawn async cleaning workflow with cancellation support
            bridge.spawn_async(move || async move {
                if let Err(e) = Self::run_cleaning_workflow(state, bridge_clone, cancel).await {
                    tracing::error!("Cleaning workflow error: {}", e);
                }
            });
        });

        let _bridge_handle = bridge.clone_handle();
        let state = state_manager.clone();

        // Stop cleaning callback - request cancellation
        ui.on_stop_cleaning(move || {
            tracing::info!("Stop cleaning button clicked - requesting cancellation");

            // Stop cleaning in state - the workflow checks this flag for cancellation
            state.stop_cleaning();

            // Log cancellation request
            tracing::warn!("Cancellation requested - ongoing operations will stop after current plugin");
        });

        let state = state_manager.clone();

        // Browse load order callback
        ui.on_browse_load_order(move || {
            tracing::debug!("Browse load order clicked");

            if let Some(path) = Self::show_file_picker(
                "Select Load Order File",
                vec![("Text files", &["txt"])],
            ) {
                tracing::info!("Load order path selected: {}", path);
                state.set_load_order_path(Some(path));
            }
        });

        let state = state_manager.clone();

        // Browse xEdit callback
        ui.on_browse_xedit(move || {
            tracing::debug!("Browse xEdit clicked");

            if let Some(path) =
                Self::show_file_picker("Select xEdit Executable", vec![("Executables", &["exe"])])
            {
                tracing::info!("xEdit path selected: {}", path);
                state.set_xedit_exe_path(Some(path));
            }
        });

        let state = state_manager.clone();

        // Browse MO2 callback
        ui.on_browse_mo2(move || {
            tracing::debug!("Browse MO2 clicked");

            if let Some(path) = Self::show_file_picker(
                "Select Mod Organizer 2 Executable",
                vec![("Executables", &["exe"])],
            ) {
                tracing::info!("MO2 path selected: {}", path);
                state.set_mo2_exe_path(Some(path));
            }
        });

        let state = state_manager.clone();

        // MO2 mode toggled
        ui.on_mo2_mode_toggled(move || {
            let enabled = state.read(|s| s.mo2_mode);
            tracing::debug!("MO2 mode toggled: {}", enabled);
            state.update_settings(|s| {
                s.mo2_mode = enabled;
            });
        });

        let state = state_manager.clone();

        // Partial forms toggled
        ui.on_partial_forms_toggled(move || {
            let enabled = state.read(|s| s.partial_forms_enabled);
            tracing::debug!("Partial forms toggled: {}", enabled);
            state.update_settings(|s| {
                s.partial_forms_enabled = enabled;
            });
        });

        tracing::debug!("UI callbacks configured");
    }

    /// Subscribe to state changes and update UI accordingly
    ///
    /// This spawns a background thread that listens for state change events
    /// and updates the Slint UI via the EventLoopBridge.
    fn setup_state_subscription(
        bridge: &EventLoopBridge<MainWindow>,
        state_manager: &Arc<StateManager>,
    ) {
        let bridge_handle = bridge.clone_handle();
        let state_manager_clone = Arc::clone(state_manager);
        let mut rx = state_manager.subscribe();

        std::thread::spawn(move || {
            tracing::debug!("State subscription thread started");

            while let Ok(change) = rx.blocking_recv() {
                tracing::trace!("State change received: {:?}", change);

                match change {
                    StateChange::ConfigurationChanged {
                        is_fully_configured,
                    } => {
                        tracing::debug!("Configuration changed: {}", is_fully_configured);
                        // Update UI path fields from state
                        let ui_weak = bridge_handle.ui_weak().clone();
                        if let Some(ui) = ui_weak.upgrade() {
                            // Get the current state snapshot
                            let state_snapshot = state_manager_clone.snapshot();

                            ui.set_load_order_path(
                                state_snapshot
                                    .load_order_path
                                    .as_ref()
                                    .map(|p| p.as_str().to_string())
                                    .unwrap_or_default()
                                    .into(),
                            );
                            ui.set_xedit_exe_path(
                                state_snapshot
                                    .xedit_exe_path
                                    .as_ref()
                                    .map(|p| p.as_str().to_string())
                                    .unwrap_or_default()
                                    .into(),
                            );
                            ui.set_mo2_exe_path(
                                state_snapshot
                                    .mo2_exe_path
                                    .as_ref()
                                    .map(|p| p.as_str().to_string())
                                    .unwrap_or_default()
                                    .into(),
                            );
                        }
                    }

                    StateChange::ProgressUpdated {
                        current,
                        total,
                        current_plugin,
                    } => {
                        bridge_handle.update_ui(move |ui| {
                            ui.set_progress_current(current as i32);
                            ui.set_progress_total(total as i32);
                            if let Some(plugin) = current_plugin {
                                ui.set_current_plugin(plugin.into());
                            }
                        });
                    }

                    StateChange::CleaningStarted { total_plugins } => {
                        tracing::info!("Cleaning started: {} plugins", total_plugins);
                        bridge_handle.update_ui(move |ui| {
                            ui.set_is_cleaning(true);
                            ui.set_progress_current(0);
                            ui.set_progress_total(total_plugins as i32);
                        });
                    }

                    StateChange::CleaningFinished {
                        cleaned,
                        failed,
                        skipped,
                    } => {
                        tracing::info!(
                            "Cleaning finished: cleaned={}, failed={}, skipped={}",
                            cleaned,
                            failed,
                            skipped
                        );
                        bridge_handle.update_ui(move |ui| {
                            ui.set_is_cleaning(false);
                            ui.set_cleaned_count(cleaned as i32);
                            ui.set_failed_count(failed as i32);
                            ui.set_skipped_count(skipped as i32);
                        });
                    }

                    StateChange::PluginProcessed {
                        plugin,
                        status,
                        message,
                    } => {
                        tracing::debug!("Plugin processed: {} - {} ({})", plugin, status, message);
                        // Update results counters (already handled by CleaningFinished)
                    }

                    StateChange::OperationChanged { operation } => {
                        bridge_handle.update_ui(move |ui| {
                            ui.set_current_operation(operation.into());
                        });
                    }

                    StateChange::SettingsChanged => {
                        tracing::debug!("Settings changed");
                        // Settings are updated directly via callbacks
                    }

                    StateChange::StateReset => {
                        tracing::info!("State reset");
                        bridge_handle.update_ui(|ui| {
                            ui.set_is_cleaning(false);
                            ui.set_progress_current(0);
                            ui.set_progress_total(0);
                            ui.set_current_plugin("".into());
                            ui.set_current_operation("".into());
                            ui.set_cleaned_count(0);
                            ui.set_failed_count(0);
                            ui.set_skipped_count(0);
                        });
                    }
                }
            }

            tracing::debug!("State subscription thread terminated");
        });
    }

    /// Show a native file picker dialog
    ///
    /// Uses the `rfd` crate to display a native file dialog on Windows.
    ///
    /// # Arguments
    /// * `title` - Dialog title
    /// * `filters` - File type filters (name, extensions)
    ///
    /// # Returns
    /// The selected file path, or None if cancelled
    fn show_file_picker(title: &str, filters: Vec<(&str, &[&str])>) -> Option<Utf8PathBuf> {
        use rfd::FileDialog;

        let mut dialog = FileDialog::new().set_title(title);

        // Add file filters
        for (name, extensions) in filters {
            dialog = dialog.add_filter(name, extensions);
        }

        // Show dialog and convert result
        dialog.pick_file().and_then(|path| {
            Utf8PathBuf::try_from(path)
                .map_err(|e| {
                    tracing::error!("Failed to convert path to UTF-8: {}", e);
                    e
                })
                .ok()
        })
    }

    // ===== Cleaning Orchestration =====

    /// Run the complete cleaning workflow
    ///
    /// This is the main orchestration method that:
    /// 1. Loads plugins from load order file
    /// 2. Filters plugins using skip lists (TODO: need ConfigManager integration)
    /// 3. Creates CleaningService and Semaphore for serial execution
    /// 4. Cleans each plugin sequentially
    /// 5. Updates UI with progress and results
    /// 6. Supports cancellation via watch channel
    async fn run_cleaning_workflow(
        state: Arc<StateManager>,
        bridge: crate::ui::bridge::EventLoopBridgeHandle<MainWindow>,
        mut cancel_rx: watch::Receiver<bool>,
    ) -> Result<()> {
        tracing::info!("Starting cleaning workflow");

        // Load plugins from load order file
        let load_order_path = state
            .read(|s| s.load_order_path.clone())
            .ok_or_else(|| anyhow!("Load order path not configured"))?;

        let plugins = Self::load_plugins_from_file(&load_order_path)
            .context("Failed to load plugins")?;

        tracing::info!("Loaded {} plugins from load order", plugins.len());

        // TODO: Filter plugins using skip lists from config
        // For now, clean all loaded plugins
        let plugins_to_clean = plugins;

        if plugins_to_clean.is_empty() {
            tracing::warn!("No plugins to clean");
            bridge.update_ui(|ui| {
                ui.set_current_operation("No plugins to clean".into());
            });
            return Ok(());
        }

        // Start cleaning operation in state
        state.start_cleaning(plugins_to_clean.clone());

        // Create CleaningService
        let service = Arc::new(CleaningService::new());

        // Create semaphore with 1 permit to enforce serial execution
        // This ensures only one xEdit instance runs at a time
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_XEDIT_PROCESSES));

        tracing::info!(
            "Starting cleaning of {} plugins (max concurrent: {})",
            plugins_to_clean.len(),
            MAX_CONCURRENT_XEDIT_PROCESSES
        );

        // Clean each plugin sequentially with cancellation support
        let mut tasks = Vec::new();

        for (index, plugin) in plugins_to_clean.iter().enumerate() {
            // Check for cancellation before starting next plugin
            if !state.read(|s| s.is_cleaning) {
                tracing::warn!("Cleaning cancelled by user, stopping after plugin {}", index);
                break;
            }

            let plugin = plugin.clone();
            let state_clone = state.clone();
            let bridge_clone = bridge.clone();
            let service_clone = service.clone();
            let semaphore_clone = semaphore.clone();

            let task = tokio::spawn(async move {
                // Acquire semaphore permit (blocks until available)
                let _permit = semaphore_clone.acquire().await.unwrap();

                // Check for cancellation after acquiring permit
                if !state_clone.read(|s| s.is_cleaning) {
                    tracing::warn!("Cleaning cancelled, skipping plugin: {}", plugin);
                    return;
                }

                tracing::info!("Cleaning plugin {}: {}", index + 1, plugin);

                // Update UI with current plugin
                state_clone.update_progress(
                    plugin.clone(),
                    format!("Cleaning {}...", plugin),
                );

                // Clean the plugin with timeout and cancellation awareness
                match Self::clean_plugin(&plugin, &state_clone, &service_clone).await {
                    Ok((status, message)) => {
                        tracing::info!("Plugin {} completed: {} - {}", plugin, status, message);
                        state_clone.add_plugin_result(plugin.clone(), &status, message.clone());

                        // Update UI
                        bridge_clone.update_ui(move |ui| {
                            ui.set_current_operation(format!("Completed: {}", plugin).into());
                        });
                    }
                    Err(e) => {
                        tracing::error!("Plugin {} failed: {}", plugin, e);
                        state_clone.add_plugin_result(
                            plugin.clone(),
                            "failed",
                            format!("Error: {}", e),
                        );
                    }
                }

                // Permit is automatically released when _permit is dropped
            });

            tasks.push(task);
        }

        // Wait for all spawned tasks to complete
        for task in tasks {
            if let Err(e) = task.await {
                tracing::error!("Task join error: {}", e);
            }
        }

        // Finish cleaning
        state.stop_cleaning();

        tracing::info!("Cleaning workflow completed");

        // Update UI with completion message
        bridge.update_ui(|ui| {
            ui.set_current_operation("Cleaning completed".into());
        });

        Ok(())
    }

    /// Load plugins from a load order file (plugins.txt or loadorder.txt)
    ///
    /// Reads the file and extracts plugin names, filtering out comments and invalid entries.
    fn load_plugins_from_file(path: &Utf8Path) -> Result<Vec<String>> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read load order file: {}", path))?;

        let plugins: Vec<String> = content
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                // Skip empty lines and comments
                if line.is_empty() || line.starts_with('#') {
                    return None;
                }

                // Handle plugins.txt format with asterisks
                let plugin = if line.starts_with('*') {
                    line[1..].trim()
                } else {
                    line
                };

                // Only include .esp, .esm, .esl files
                if plugin.ends_with(".esp")
                    || plugin.ends_with(".esm")
                    || plugin.ends_with(".esl")
                {
                    Some(plugin.to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(plugins)
    }

    /// Clean a single plugin
    ///
    /// This performs the full cleaning cycle for one plugin:
    /// 1. Get log paths
    /// 2. Clear old logs
    /// 3. Build and execute cleaning command
    /// 4. Check for errors
    /// 5. Parse results
    ///
    /// Returns (status, message) tuple
    async fn clean_plugin(
        plugin: &str,
        state: &StateManager,
        service: &CleaningService,
    ) -> Result<(String, String)> {
        // Get configuration from state
        let (xedit_exe, game_type, mo2_exe, partial_forms, timeout, _mo2_install, _xedit_install) = state.read(|s| {
            (
                s.xedit_exe_path.clone(),
                s.game_type.clone(),
                s.mo2_exe_path.clone(),
                s.partial_forms_enabled,
                s.cleaning_timeout,
                s.mo2_install_path.clone(),
                s.xedit_install_path.clone(),
            )
        });

        let xedit_exe = xedit_exe.ok_or_else(|| anyhow!("xEdit exe path not configured"))?;

        // Get log paths
        let (main_log, exception_log) = service.get_log_paths(&xedit_exe, game_type.as_deref());

        // Clear old logs
        service.clear_logs(&main_log, &exception_log)?;

        // Build cleaning command
        let command = service.build_cleaning_command(
            &xedit_exe,
            plugin,
            game_type.as_deref(),
            mo2_exe.as_deref(),
            partial_forms,
        );

        tracing::debug!("Executing command: {}", command);

        // Execute cleaning command
        let exit_code = service.execute_cleaning_command(&command, timeout).await?;

        // Check exception log for errors
        if service.check_exception_log(&exception_log)? {
            return Ok((
                "skipped".to_string(),
                "Missing requirements or empty plugin".to_string(),
            ));
        }

        // Check exit code
        if exit_code != 0 {
            return Ok((
                "failed".to_string(),
                format!("xEdit exited with code {}", exit_code),
            ));
        }

        // Parse log file for cleaning stats
        let stats = service.parse_log_file(&main_log)?;

        if stats.has_changes() {
            Ok(("cleaned".to_string(), stats.summary()))
        } else {
            Ok(("skipped".to_string(), "Nothing to clean".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_creation() {
        // Note: This test is limited because Slint UI requires a display/window system
        // More comprehensive tests will be in integration tests

        let state_manager = Arc::new(StateManager::new());
        let rt = tokio::runtime::Runtime::new().unwrap();

        // We can't actually create the controller in a test environment without a display,
        // but we can test the state manager integration
        let state = state_manager.snapshot();
        assert!(!state.is_cleaning);
        assert!(!state.is_fully_configured());
    }

    #[test]
    fn test_state_synchronization() {
        let state_manager = Arc::new(StateManager::new());

        // Update state
        state_manager.update(|state| {
            state.is_cleaning = true;
            state.progress = 5;
            state.total_plugins = 10;
        });

        // Verify state
        let state = state_manager.snapshot();
        assert!(state.is_cleaning);
        assert_eq!(state.progress, 5);
        assert_eq!(state.total_plugins, 10);
    }
}
