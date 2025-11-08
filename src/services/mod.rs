//! Services module - Pure business logic for plugin cleaning operations.
//!
//! This module contains all the core business logic for cleaning Bethesda game plugins using
//! xEdit's Quick Auto Clean (QAC) mode. The services are **framework-agnostic** and have no
//! dependencies on the UI layer, making them testable and reusable.
//!
//! # Components
//!
//! - [`CleaningService`]: The main service for executing xEdit cleaning operations. Handles:
//!   - Building xEdit command lines (direct execution, MO2 mode, universal xEdit, partial forms)
//!   - Executing subprocesses with timeout support
//!   - Parsing xEdit log files to extract cleaning statistics
//!   - Error detection from exception logs
//!
//! - [`CleanResult`]: Complete result of a single plugin cleaning operation, including:
//!   - [`CleanStatus`]: Success, skipped, or failure state
//!   - [`CleaningStats`]: ITMs, UDRs, navmeshes, partial forms removed
//!   - Error messages and contextual information
//!
//! # Design Philosophy
//!
//! The services layer is designed to be:
//! - **Pure**: No side effects beyond file I/O and subprocess execution
//! - **Async**: All operations use tokio for non-blocking I/O
//! - **Testable**: No hidden dependencies, all inputs are explicit parameters
//! - **Framework-agnostic**: No Slint, no GUI code, only business logic
//!
//! # Usage Example
//!
//! ```ignore
//! use autoqac::services::CleaningService;
//!
//! let service = CleaningService::new();
//!
//! // Build command for xEdit
//! let command = service.build_cleaning_command(
//!     &game_config,
//!     &user_config,
//!     "MyPlugin.esp",
//!     false, // no partial forms
//! )?;
//!
//! // Execute cleaning
//! let result = service.execute_cleaning_command(
//!     &command,
//!     &xedit_exe_path,
//!     &data_folder_path,
//!     300, // timeout in seconds
//! ).await?;
//! ```
//!
//! # xEdit Integration
//!
//! The service integrates with xEdit by:
//! 1. Clearing old log files before execution
//! 2. Running xEdit with `-QAC -autoexit -autoload` flags
//! 3. Monitoring exception logs for errors (missing masters, empty plugins)
//! 4. Parsing main log files using regex to extract statistics
//!
//! See the [xEdit documentation](https://tes5edit.github.io/) for details on QAC mode.

pub mod cleaning;
pub mod game_detection;

pub use cleaning::{CleanResult, CleanStatus, CleaningError, CleaningService, CleaningStats};
pub use game_detection::{detect_game_from_load_order, detect_xedit_game};
