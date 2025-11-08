//! AutoQAC - Automatic Quick Auto Clean for Bethesda Game Plugins
//!
//! This is the library crate containing the core business logic and data structures.
//! The binary crate ([`main.rs`]) provides the Slint GUI entry point.
//!
//! # Overview
//!
//! AutoQAC is a Rust application for batch cleaning Bethesda game plugins using xEdit's
//! Quick Auto Clean (QAC) mode. It provides a modern Fluent Design GUI built with Slint
//! and uses tokio for async subprocess management.
//!
//! # Architecture
//!
//! The library is organized into several key modules:
//!
//! - [`models`]: Core data structures ([`AppState`], [`MainConfig`], [`UserConfig`], [`IgnoreConfig`])
//! - [`state`]: Thread-safe state management via [`StateManager`] with reactive change events
//! - [`config`]: YAML configuration file loading/saving via [`ConfigManager`]
//! - [`services`]: Pure business logic for plugin cleaning (framework-agnostic)
//! - [`ui`]: Slint GUI integration and event loop coordination
//! - [`logging`]: Structured logging setup with file rotation
//!
//! # Threading Model
//!
//! - **Main thread**: Slint event loop (synchronous, blocking)
//! - **Tokio runtime**: 4 worker threads for async operations (subprocess execution, file I/O)
//! - **State subscription**: Background std::thread for listening to state changes
//! - **Serial execution**: Only 1 xEdit subprocess runs at a time (enforced by semaphore)
//!
//! # Quick Start
//!
//! ## Basic Usage (Library)
//!
//! ```ignore
//! use autoqac::{StateManager, ConfigManager, services::CleaningService};
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize state management
//!     let state = Arc::new(StateManager::new());
//!
//!     // Load configurations
//!     let config_manager = ConfigManager::new("AutoQAC Data".into());
//!     let main_config = config_manager.load_main_config()?;
//!     let user_config = config_manager.load_user_config()?;
//!
//!     // Create cleaning service
//!     let service = CleaningService::new();
//!
//!     // Build and execute cleaning command
//!     let command = service.build_cleaning_command(
//!         &user_config.xedit_exe_path,
//!         "MyPlugin.esp",
//!         Some("FO4"),  // Universal xEdit game mode
//!         None,         // No MO2
//!         false,        // No partial forms
//!     );
//!
//!     let exit_code = service.execute_cleaning_command(
//!         &command,
//!         std::time::Duration::from_secs(300),
//!     ).await?;
//!
//!     println!("xEdit exited with code: {}", exit_code);
//!     Ok(())
//! }
//! ```
//!
//! ## State Management
//!
//! ```ignore
//! use autoqac::{StateManager, StateChange};
//! use std::sync::Arc;
//!
//! let state = Arc::new(StateManager::new());
//!
//! // Subscribe to state changes
//! let mut rx = state.subscribe();
//!
//! // Spawn a listener
//! tokio::spawn(async move {
//!     while let Ok(change) = rx.recv().await {
//!         match change {
//!             StateChange::ProgressUpdated { current, total, .. } => {
//!                 println!("Progress: {}/{}", current, total);
//!             }
//!             StateChange::CleaningFinished { cleaned, failed, skipped } => {
//!                 println!("Done! Cleaned: {}, Failed: {}, Skipped: {}",
//!                          cleaned, failed, skipped);
//!             }
//!             _ => {}
//!         }
//!     }
//! });
//!
//! // Update state (triggers events)
//! state.update(|s| {
//!     s.progress = 5;
//!     s.total_plugins = 10;
//! });
//! ```
//!
//! ## Configuration Management
//!
//! ```ignore
//! use autoqac::ConfigManager;
//! use camino::Utf8PathBuf;
//!
//! let config_dir = Utf8PathBuf::from("AutoQAC Data");
//! let manager = ConfigManager::new(config_dir);
//!
//! // Load main configuration (games, skip lists)
//! let main_config = manager.load_main_config()?;
//! println!("Loaded {} game configurations", main_config.games.len());
//!
//! // Load user configuration (paths, settings)
//! let user_config = manager.load_user_config()?;
//! println!("xEdit path: {:?}", user_config.xedit_exe_path);
//!
//! // Load ignore list
//! let ignore = manager.load_ignore_config()?;
//! println!("Ignored plugins: {:?}", ignore.ignore_list);
//! ```
//!
//! # Supported Games
//!
//! - Fallout 3 (FO3)
//! - Fallout New Vegas (FNV)
//! - Fallout 4 (FO4)
//! - Skyrim Special Edition (SSE)
//! - Fallout 4 VR (FO4VR)
//! - Skyrim VR (SkyrimVR)
//!
//! # xEdit Integration
//!
//! The library integrates with xEdit by:
//! 1. Building command-line arguments with `-QAC -autoexit -autoload` flags
//! 2. Executing xEdit as a subprocess (with timeout support)
//! 3. Parsing log files to extract cleaning statistics:
//!    - **UDRs**: Undisabled References (undeleting)
//!    - **ITMs**: Identical To Master records (removing)
//!    - **Navmeshes**: Deleted navmeshes (skipping)
//!    - **Partial Forms**: Experimental partial form handling
//!
//! # Concurrency Constraints
//!
//! **CRITICAL**: Only **1 concurrent xEdit process** is allowed due to file locking issues.
//! This is hardcoded via [`models::MAX_CONCURRENT_XEDIT_PROCESSES`] and enforced using
//! a tokio semaphore.
//!
//! # Feature Flags
//!
//! Currently, the library has no optional features. All functionality is enabled by default.
//!
//! # Platform Support
//!
//! - **Primary**: Windows 10/11 (x86_64)
//! - **Secondary**: Cross-platform support via tokio and Slint (Linux, macOS)
//!
//! # Error Handling
//!
//! - **Application errors**: [`anyhow::Result`] for context propagation
//! - **Library errors**: [`thiserror::Error`] for structured error types
//! - **Logging**: [`tracing`] with file rotation and JSON support

pub mod config;
pub mod logging;
pub mod metrics;
pub mod models;
pub mod services;
pub mod state;
pub mod ui;

// Re-export commonly used types for convenience
pub use config::ConfigManager;
pub use metrics::Metrics;
pub use models::{AppState, IgnoreConfig, MainConfig, UserConfig};
pub use state::{StateChange, StateManager};

/// Application version from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name from Cargo.toml
pub const APP_NAME: &str = env!("CARGO_PKG_NAME");
