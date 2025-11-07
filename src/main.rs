//! AutoQAC - Automatic Quick Auto Clean for Bethesda Game Plugins
//!
//! Main entry point for the GUI application.
//!
//! # Overview
//!
//! This binary crate provides the Slint GUI frontend for AutoQAC. It initializes:
//! - Logging infrastructure (file rotation + console output)
//! - Tokio async runtime (4 worker threads for subprocess execution)
//! - State management ([`StateManager`])
//! - Configuration loading ([`ConfigManager`])
//! - GUI controller ([`GuiController`] - bridges Slint UI with business logic)
//!
//! The application uses a hybrid threading model:
//! - **Main thread**: Runs the Slint event loop (blocking, synchronous)
//! - **Tokio workers**: Handle async operations (xEdit subprocess execution, file I/O)
//! - **State listener**: Background std::thread for reactive UI updates
//!
//! # Execution Flow
//!
//! 1. Initialize logging → logs/autoqac_<timestamp>.log
//! 2. Create tokio runtime with 4 worker threads
//! 3. Create StateManager (Arc<RwLock<AppState>>)
//! 4. Load YAML configurations from AutoQAC Data/
//!    - AutoQAC Main.yaml → Game configs, skip lists
//!    - AutoQAC Config.yaml or PACT Settings.yaml → User settings
//! 5. Create GuiController (wires Slint UI to state and runtime)
//! 6. Run Slint event loop (blocks until window closed)
//! 7. Shutdown tokio runtime with 5s timeout
//!
//! # Configuration Files
//!
//! Expected in `AutoQAC Data/` directory:
//! - `AutoQAC Main.yaml`: Game configurations, xEdit paths, skip lists
//! - `AutoQAC Config.yaml` or `PACT Settings.yaml`: User preferences
//! - `PACT Ignore.yaml`: Additional plugin ignore list (optional)
//!
//! # Platform
//!
//! Primary platform: Windows 10/11 (x86_64)
//! Secondary: Cross-platform via Slint and tokio

use anyhow::Result;
use autoqac::ui::GuiController;
use autoqac::{ConfigManager, StateManager, APP_NAME, VERSION};
use std::sync::Arc;

/// Main entry point for the AutoQAC GUI application
///
/// This function orchestrates the complete application lifecycle:
/// 1. Logging setup
/// 2. Tokio runtime initialization
/// 3. State and configuration management
/// 4. GUI launch and execution
/// 5. Graceful shutdown
///
/// # Returns
///
/// - `Ok(())` if the application ran and exited normally
/// - `Err(_)` if initialization or GUI execution failed
///
/// # Errors
///
/// This function can fail if:
/// - Logging initialization fails (disk space, permissions)
/// - Tokio runtime creation fails (system resources)
/// - Configuration files are missing or invalid YAML
/// - Slint UI initialization fails (graphics drivers, display)
/// - GUI encounters a fatal error during execution
fn main() -> Result<()> {
    // Setup logging with both file and console output
    autoqac::logging::setup_logging_with_console("logs", "autoqac", false, true)?;

    tracing::info!("Starting {} v{}", APP_NAME, VERSION);

    // Create tokio runtime for async operations
    // This will handle subprocess execution and other I/O operations
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .thread_name("autoqac-worker")
        .build()?;

    tracing::info!("Tokio runtime initialized with {} worker threads", 4);

    // Create state manager for application state
    let state_manager = Arc::new(StateManager::new());
    tracing::info!("State manager initialized");

    // Create configuration manager
    let config_manager = ConfigManager::new("AutoQAC Data")?;

    // Load configurations
    let main_config = config_manager.load_main_config()?;
    let user_config = config_manager.load_user_config()?;

    tracing::info!(
        "Loaded configurations - version: {}, xedit_lists: {}",
        main_config.pact_data.version,
        main_config.pact_data.xedit_lists.len()
    );

    // Load user config into state manager
    state_manager.load_from_user_config(&user_config);
    tracing::info!("User configuration loaded into state manager");

    // Create GUI controller
    // This wires up the Slint UI with state management and the tokio runtime
    let gui_controller = GuiController::new(state_manager.clone(), runtime.handle().clone())?;

    tracing::info!("GUI controller initialized, launching window");

    // Run the GUI (blocks until window is closed)
    // The tokio runtime stays alive in the background to handle async tasks
    let result = gui_controller.run();

    // Clean up after window closes
    tracing::info!("GUI closed, shutting down");

    // Check if cleaning was in progress and cancel it
    if state_manager.read(|s| s.is_cleaning) {
        tracing::warn!("Window closed during cleaning operation - cancelling...");
        state_manager.stop_cleaning();

        // Give operations a moment to cancel gracefully
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    // Shutdown the tokio runtime gracefully
    runtime.shutdown_timeout(std::time::Duration::from_secs(5));

    tracing::info!("Application shutdown complete");

    result.map_err(|e| {
        tracing::error!("GUI error: {}", e);
        anyhow::anyhow!("GUI error: {}", e)
    })
}
