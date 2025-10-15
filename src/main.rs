// AutoQAC - Automatic Quick Auto Clean for Bethesda Game Plugins
//
// Main entry point for the GUI application.
// This is Phase 1: Foundation - we're setting up the basic structure.

use anyhow::Result;
use autoqac::{ConfigManager, APP_NAME, VERSION};

// Include the Slint-generated code
slint::include_modules!();

fn main() -> Result<()> {
    // Setup logging
    autoqac::logging::setup_logging_with_console("logs", "autoqac", false, true)?;

    tracing::info!("Starting {} v{}", APP_NAME, VERSION);

    // Create configuration manager
    let config_manager = ConfigManager::new("AutoQAC Data")?;

    // Load configurations
    let main_config = config_manager.load_main_config()?;
    #[allow(unused_variables)] // Will be used in Phase 2
    let user_config = config_manager.load_user_config()?;

    tracing::info!(
        "Loaded configurations - version: {}",
        main_config.pact_data.version
    );

    // Create and run the Slint UI
    let ui = MainWindow::new()?;

    tracing::info!("Launching GUI");
    ui.run()?;

    tracing::info!("Application shutting down");
    Ok(())
}
