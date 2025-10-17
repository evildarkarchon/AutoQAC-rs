//! Data models for the AutoQAC application.
//!
//! This module contains all the core data structures used throughout the application:
//! - [`AppState`]: The central state container holding runtime data, settings, and cleaning results
//! - [`MainConfig`]: Game configurations, xEdit executables, and skip lists loaded from `AutoQAC Main.yaml`
//! - [`UserConfig`]: User preferences and paths loaded from `AutoQAC Config.yaml` or `PACT Settings.yaml`
//! - [`IgnoreConfig`]: Additional plugin ignore list from `PACT Ignore.yaml`
//! - [`MAX_CONCURRENT_XEDIT_PROCESSES`]: Critical concurrency limit constant (always 1 due to xEdit file locking)
//!
//! # Architecture Note
//!
//! The models are designed to be:
//! - **Serializable**: All config structs derive `Serialize`/`Deserialize` for YAML persistence
//! - **Cloneable**: AppState is wrapped in `Arc<RwLock<>>` by [`StateManager`](crate::state::StateManager) for thread-safe access
//! - **Immutable**: State updates go through StateManager's `update()` method to ensure consistency

pub mod app_state;
pub mod config;

pub use app_state::{AppState, MAX_CONCURRENT_XEDIT_PROCESSES};
pub use config::{IgnoreConfig, MainConfig, PactData, UserConfig};
