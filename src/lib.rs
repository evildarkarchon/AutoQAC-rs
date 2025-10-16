// AutoQAC - Automatic Quick Auto Clean for Bethesda Game Plugins
//
// This is the library crate containing the core business logic and data structures.
// The binary crate (main.rs) provides the GUI entry point.

pub mod config;
pub mod logging;
pub mod models;
pub mod services;
pub mod state;
pub mod ui;

// Re-export commonly used types for convenience
pub use config::ConfigManager;
pub use models::{AppState, IgnoreConfig, MainConfig, UserConfig};
pub use state::{StateChange, StateManager};

/// Application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
