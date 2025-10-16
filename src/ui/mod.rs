// UI module - GUI logic and event loop bridge
//
// This module contains:
// - EventLoopBridge: Coordinates between tokio async runtime and Slint event loop
// - GuiController: Main controller that wires up the UI with state management

pub mod bridge;
pub mod controller;

pub use bridge::{EventLoopBridge, EventLoopBridgeHandle};
pub use controller::GuiController;
