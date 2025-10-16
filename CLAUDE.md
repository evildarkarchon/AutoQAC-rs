# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

AutoQAC is a Rust rewrite of the Python PySide6 application for batch cleaning Bethesda game plugins using xEdit's quickautoclean. This is Phase 1-4 of the Rust migration, featuring a Slint-based GUI with Fluent Design, tokio async runtime, and complete plugin cleaning functionality.

## Essential Commands

```bash
# Build and run the application
cargo run --release

# Development build and run
cargo run

# Run tests
cargo test                          # All tests
cargo test --lib                    # Library tests only
cargo test --test '*'               # Integration tests only

# Run tests with output
cargo test -- --nocapture           # Show println! output
cargo test -- --show-output         # Show all output

# Build only
cargo build                         # Debug build
cargo build --release               # Release build with optimizations

# Check code without building
cargo check                         # Quick check
cargo clippy                        # Linting with Clippy
cargo clippy -- -W clippy::all      # All Clippy warnings

# Format code
cargo fmt                           # Format all code
cargo fmt -- --check                # Check formatting without modifying

# Build the Slint UI (normally automatic)
cargo build                         # Triggers slint-build in build.rs

# Clean build artifacts
cargo clean
```

## Architecture Overview

### Core Design Philosophy

This is a **Rust port** of the Python AutoQAC application with these key differences:
- **Slint** instead of PySide6 for GUI framework
- **Tokio** async runtime for subprocess execution and I/O operations
- **Thread-safe state management** using `Arc<RwLock<AppState>>` with broadcast channels for change events
- **Strict concurrency limit**: Only **1 concurrent xEdit process** allowed (hardcoded due to xEdit file locking issues)

### Key Components

1. **StateManager** (`src/state/mod.rs`): Thread-safe state container wrapping `AppState` with `Arc<RwLock<T>>`. Emits `StateChange` events via tokio broadcast channels for reactive GUI updates.

2. **AppState** (`src/models/app_state.rs`): Single source of truth for all application state. Includes configuration paths, runtime flags, progress tracking, results, and settings.

3. **ConfigManager** (`src/config/mod.rs`): Manages YAML configuration files:
   - `AutoQAC Data/AutoQAC Main.yaml`: Game configurations, skip lists
   - `AutoQAC Data/AutoQAC Config.yaml` or `PACT Settings.yaml`: User settings, paths
   - `AutoQAC Data/PACT Ignore.yaml`: Additional ignore list

4. **CleaningService** (`src/services/cleaning.rs`): Pure business logic for plugin cleaning. No GUI dependencies. Handles subprocess execution, log parsing, and result extraction.

5. **GuiController** (`src/ui/controller.rs`): Mediator between Slint UI and business logic. Coordinates:
   - Slint UI callbacks → tokio async tasks
   - StateManager events → UI updates
   - File browser dialogs
   - Cleaning workflow orchestration with cancellation support

6. **EventLoopBridge** (`src/ui/bridge.rs`): Coordinates between tokio async runtime and Slint's synchronous event loop using channels and weak UI handles.

7. **main.rs**: Slim entry point (~70 lines) that:
   - Creates tokio runtime (4 worker threads)
   - Initializes StateManager and ConfigManager
   - Loads configurations
   - Creates and runs GuiController
   - Handles graceful shutdown

### Threading Model

**CRITICAL**: This application uses **tokio exclusively** for concurrency:
- Main thread: Slint event loop (blocking)
- Tokio runtime: 4 worker threads for async operations
- State subscription: Background std::thread for listening to state changes
- **Serial execution**: Only 1 xEdit subprocess runs at a time (enforced by `tokio::sync::Semaphore`)

**NEVER**:
- Mix tokio with std threading beyond the state subscription thread
- Use async/await in Slint callbacks directly (use EventLoopBridge instead)
- Allow concurrent xEdit processes (causes file corruption)

### Cancellation Architecture

The application supports **immediate cancellation** of cleaning operations:
- **Watch channel** (`tokio::sync::watch`): Primary cancellation mechanism
- **State flag** (`is_cleaning`): Secondary coordination mechanism
- `tokio::select!` races cleaning operations against cancellation for instant responsiveness
- No polling loops - cancellation is event-driven

### State Change Events

All state mutations emit events through broadcast channels:
```rust
pub enum StateChange {
    ConfigurationChanged { is_fully_configured: bool },
    ProgressUpdated { current, total, current_plugin },
    CleaningStarted { total_plugins },
    CleaningFinished { cleaned, failed, skipped },
    PluginProcessed { plugin, status, message },
    OperationChanged { operation },
    SettingsChanged,
    StateReset,
}
```

### Slint UI Integration

- UI files: `ui/main.slint`, `ui/components/`, `ui/dialogs/`
- Compiled at build time by `slint-build` (see `build.rs`)
- Native file dialogs via `rfd` crate
- Fluent Design styling with acrylic effects and modern Windows 11 aesthetics

### Workflow: Plugin Cleaning

1. Load plugins from load order file (`plugins.txt` or `loadorder.txt`)
2. Filter using skip lists from main config (TODO: fully integrated)
3. Create `CleaningService` and `Semaphore(1)` for serial execution
4. For each plugin (with cancellation support):
   - Acquire semaphore permit (blocks if another plugin is cleaning)
   - Clear old xEdit log files
   - Build xEdit QAC command with appropriate flags
   - Execute subprocess with timeout (default 300s)
   - Check exception log for errors (missing requirements, empty plugin)
   - Parse main log for cleaning stats (ITMs, UDRs, navmeshes, partial forms)
   - Update state with results
   - Release permit (next plugin starts)
5. Emit completion events and update UI

## Critical Constraints

### xEdit Concurrency Limit
```rust
pub const MAX_CONCURRENT_XEDIT_PROCESSES: usize = 1;
```
This is **hardcoded** and **non-negotiable**. xEdit has file locking issues that prevent multiple instances from running simultaneously. The semaphore in `GuiController::run_cleaning_workflow` enforces this.

### Async/Sync Boundary

**Slint callbacks are synchronous** but need to trigger **async operations**. Use `EventLoopBridge::spawn_async`:
```rust
ui.on_start_cleaning(move || {
    let bridge = bridge_handle.clone();
    bridge.spawn_async(move || async move {
        // Your async code here
    });
});
```

### UTF-8 Path Handling

All paths use `camino::Utf8PathBuf` for Windows-safe UTF-8 paths. This avoids encoding issues with Windows path APIs.

## Error Handling

- **anyhow::Result** for application-level errors (propagates context)
- **thiserror::Error** for library-level errors (CleaningError)
- All errors logged via `tracing` with appropriate levels (error, warn, info, debug, trace)

## Logging

- File logs: `logs/autoqac_<timestamp>.log` (rotating)
- Console logs: Enabled in main.rs for development
- Configured via `tracing-subscriber` with JSON formatting support
- Setup: `autoqac::logging::setup_logging_with_console()`

## Testing

- Unit tests in each module (`#[cfg(test)] mod tests`)
- Integration tests in `tests/` directory (TODO: add more)
- Uses: `tokio-test`, `tempfile`, `proptest`, `mockall`
- Property-based testing for edge cases (via `proptest`)

## Dependencies

### Core Runtime
- **slint**: GUI framework with Fluent Design
- **tokio**: Async runtime for subprocess management

### Configuration & Serialization
- **serde**: Serialization framework
- **serde_yaml_ng**: YAML config files (maintained fork)
- **config**: Structured configuration (not heavily used yet)

### Error Handling & Logging
- **anyhow**: Application errors with context
- **thiserror**: Library errors with derive macros
- **tracing**: Structured logging
- **tracing-subscriber**: Log formatting and filtering
- **tracing-appender**: Rotating file logs

### Utilities
- **camino**: UTF-8 paths (Windows-safe)
- **regex**: Log file parsing
- **indexmap**: Order-preserving hashmaps for configs
- **rfd**: Native file dialogs

## Project Structure

```
autoqac-rust/
├── src/
│   ├── main.rs                    # Entry point (~70 lines)
│   ├── lib.rs                     # Library root with re-exports
│   ├── models/
│   │   ├── mod.rs                # Model re-exports
│   │   ├── app_state.rs          # AppState and MAX_CONCURRENT_XEDIT_PROCESSES
│   │   └── config.rs             # YAML config structures
│   ├── state/
│   │   └── mod.rs                # StateManager and StateChange events
│   ├── config/
│   │   └── mod.rs                # ConfigManager for YAML I/O
│   ├── services/
│   │   ├── mod.rs
│   │   └── cleaning.rs           # CleaningService (pure business logic)
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── controller.rs         # GuiController (GUI-business mediator)
│   │   └── bridge.rs             # EventLoopBridge (tokio/Slint coordination)
│   └── logging.rs                # Logging setup
├── ui/
│   ├── main.slint                # Main window UI definition
│   ├── components/               # Reusable UI components
│   ├── dialogs/                  # Dialog components
│   └── fluent/                   # Fluent Design styles
├── build.rs                      # Slint build script
├── Cargo.toml                    # Dependencies and metadata
└── AutoQAC Data/                 # Configuration files (runtime)
```

## Supported Games

- Fallout 3 (FO3)
- Fallout New Vegas (FNV)
- Fallout 4 (FO4)
- Skyrim Special Edition (SSE)
- Fallout 4 VR (FO4VR)
- Skyrim VR (SkyrimVR)

Each game has xEdit executables and skip lists configured in `AutoQAC Main.yaml`.

## xEdit Integration

Commands built with `-QAC` flag for Quick Auto Clean:
- Direct execution: `"C:/xEdit.exe" -QAC -autoexit -autoload "Plugin.esp"`
- MO2 mode: `"C:/MO2.exe" run "C:/xEdit.exe" -a "-QAC -autoexit -autoload \"Plugin.esp\""`
- Universal xEdit: `"C:/xEdit.exe" -FO4 -QAC -autoexit -autoload "Plugin.esp"`
- Partial Forms (experimental): Add `-iknowwhatimdoing -allowmakepartial` flags

Log parsing extracts:
- **UDRs** (Undisabled References): `Undeleting: [FormID]`
- **ITMs** (Identical To Master): `Removing: [FormID]`
- **Navmeshes**: `Skipping: [NavMesh]`
- **Partial Forms**: `Making Partial Form: [FormID]`

## Development Notes

- Rust 1.70+ required (specified in Cargo.toml)
- Windows primary platform (cross-platform support via tokio/Slint)
- Release builds use LTO and strip symbols for smaller binaries
- Slint UI hot-reload: Not supported (requires full rebuild)
- State manager cloning is cheap (Arc-based)
