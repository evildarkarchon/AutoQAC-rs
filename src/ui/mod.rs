//! User interface module - Slint GUI integration and event loop coordination.
//!
//! This module provides the complete UI layer for the AutoQAC application, handling the integration
//! between Slint's synchronous GUI framework and the tokio async runtime used for business logic.
//!
//! # Components
//!
//! - [`EventLoopBridge`]: Coordinates between tokio async tasks and Slint's event loop using channels
//!   and weak UI handles. Allows async operations to update the UI without blocking.
//!
//! - [`GuiController`]: Main orchestrator that wires together the Slint UI, [`StateManager`](crate::state::StateManager),
//!   file dialogs, and the [`CleaningService`](crate::services::cleaning::CleaningService). Manages the
//!   complete cleaning workflow with cancellation support.
//!
//! # Threading Architecture
//!
//! The UI module bridges two execution models:
//! - **Slint event loop** (main thread, synchronous): Handles UI rendering and user interactions
//! - **Tokio runtime** (worker threads, async): Executes business logic, subprocess management, file I/O
//!
//! The [`EventLoopBridge`] uses channels to safely communicate between these contexts, ensuring
//! UI updates happen on the main thread while expensive operations run asynchronously.
//!
//! # Cancellation
//!
//! The UI supports immediate cancellation of long-running cleaning operations via a watch channel.
//! See [`GuiController::request_cancel`] and the implementation in `clean_plugin()` for details.
//!
//! # Example Flow
//!
//! ```ignore
//! // User clicks "Start Cleaning" button
//! ui.on_start_cleaning(|| {
//!     // Spawn async task via EventLoopBridge
//!     bridge.spawn_async(|| async {
//!         // Run cleaning workflow (async operations)
//!         controller.run_cleaning_workflow().await;
//!     });
//! });
//! ```

pub mod bridge;
pub mod controller;

pub use bridge::{EventLoopBridge, EventLoopBridgeHandle};
pub use controller::GuiController;
