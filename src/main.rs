// AutoQAC - Automatic Quick Auto Clean for Bethesda Game Plugins
//
// Main entry point for the GUI application.
// Phase 4: Complete Fluent Design UI with tokio/Slint event loop coordination

use anyhow::Result;
use autoqac::ui::GuiController;
use autoqac::{ConfigManager, StateManager, APP_NAME, VERSION};
use std::sync::Arc;

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

    // Clean up
    tracing::info!("GUI closed, shutting down");

    // Shutdown the tokio runtime gracefully
    runtime.shutdown_timeout(std::time::Duration::from_secs(5));

    tracing::info!("Application shutdown complete");

    result.map_err(|e| {
        tracing::error!("GUI error: {}", e);
        anyhow::anyhow!("GUI error: {}", e)
    })
}
