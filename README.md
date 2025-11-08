# AutoQAC - Rust Implementation

**Automatic Quick Auto Clean for Bethesda Game Plugins**

A modern Rust application with Slint GUI for batch cleaning game plugins using xEdit's Quick Auto Clean (-QAC) functionality.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Architecture](#architecture)
  - [Core Components](#core-components)
  - [Threading Model](#threading-model)
  - [Module Organization](#module-organization)
- [Technology Stack](#technology-stack)
- [Building & Running](#building--running)
- [Usage](#usage)
  - [GUI Application](#gui-application)
  - [Library Usage](#library-usage)
- [Configuration](#configuration)
- [Development](#development)
  - [Project Structure](#project-structure)
  - [Testing](#testing)
  - [Documentation](#documentation)
- [Supported Games](#supported-games)
- [License](#license)

## Overview

AutoQAC automates the process of cleaning Bethesda game plugins (ESP/ESM/ESL files) using xEdit's Quick Auto Clean mode. It removes:

- **ITMs (Identical To Master)**: Records that are identical to the master file
- **UDRs (Undisabled References)**: References that should be disabled but aren't
- **Deleted Navmeshes**: Navigation meshes that cause crashes

This Rust implementation provides a modern, thread-safe alternative to the original Python/PySide6 version, with:
- **Modern UI**: Microsoft Fluent Design System via Slint
- **Async Processing**: Tokio runtime for non-blocking operations
- **Type Safety**: Rust's strong type system prevents common errors
- **Performance**: Native code execution with zero-cost abstractions

## Features

### Core Functionality
- ✅ Batch cleaning of multiple plugins
- ✅ Skip list integration (don't clean base game files)
- ✅ Auto-detection of game type from xEdit executable or load order
- ✅ MO2 (Mod Organizer 2) integration support
- ✅ Configurable timeout per plugin (default: 300s)
- ✅ Real-time progress tracking with record-level statistics
- ✅ Cancellation support (stop cleaning mid-operation)
- ✅ Comprehensive error handling and logging

### UI Features
- ✅ Modern Fluent Design interface
- ✅ File browser dialogs for configuration
- ✅ Real-time progress updates with percentage display
- ✅ Color-coded statistics badges (UDRs, ITMs, Navmeshes, Partial Forms)
- ✅ Contextual status messages
- ✅ Configuration validation with visual feedback
- ✅ About dialog with version information
- ✅ Close confirmation when cleaning is active

### Advanced Features
- ✅ Partial Forms experimental support (opt-in via `-iknowwhatimdoing -allowmakepartial`)
- ✅ Record-level statistics parsing from xEdit output
- ✅ Aggregate statistics across all cleaned plugins
- ✅ Game-specific configuration management
- ✅ Legacy config file migration (PACT Settings.yaml → AutoQAC Config.yaml)

## Architecture

### Core Components

The application follows a layered architecture with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────┐
│                    Slint UI Layer                       │
│  (main.slint - Declarative UI with Fluent Design)      │
└─────────────────────────────────────────────────────────┘
                          ↕
┌─────────────────────────────────────────────────────────┐
│                   GuiController                         │
│     (Bridges UI events with business logic)             │
└─────────────────────────────────────────────────────────┘
                          ↕
┌──────────────────┬──────────────────┬──────────────────┐
│  StateManager    │  ConfigManager   │ CleaningService  │
│ (Thread-safe     │ (YAML I/O for    │ (Business logic  │
│  state with      │  settings)       │  for cleaning)   │
│  reactive events)│                  │                  │
└──────────────────┴──────────────────┴──────────────────┘
                          ↕
┌─────────────────────────────────────────────────────────┐
│              Models (AppState, Configs)                 │
│  (Core data structures and domain types)                │
└─────────────────────────────────────────────────────────┘
```

#### 1. **StateManager** ([src/state/mod.rs](src/state/mod.rs))

Centralized, thread-safe state management with reactive change events.

- Uses `Arc<RwLock<AppState>>` for safe concurrent access
- Emits `StateChange` events via tokio broadcast channel
- Provides atomic state mutations with automatic event emission
- Supports multiple subscribers (GUI, logging, testing)

**Key Methods**:
```rust
// Read state snapshot
let state = state_manager.snapshot();

// Update with closure (emits events)
state_manager.update(|state| {
    state.is_cleaning = true;
});

// Subscribe to changes
let mut rx = state_manager.subscribe();
while let Ok(change) = rx.recv().await {
    match change {
        StateChange::ProgressUpdated { current, total, .. } => { /* ... */ }
        _ => {}
    }
}
```

#### 2. **ConfigManager** ([src/config/mod.rs](src/config/mod.rs))

YAML configuration file management with validation.

- Manages three config files:
  - `AutoQAC Main.yaml`: Game configurations, skip lists, xEdit paths
  - `AutoQAC Config.yaml`: User settings, file paths, timeouts
  - `PACT Ignore.yaml`: Additional plugin ignore list (optional)
- Supports legacy config migration (PACT Settings.yaml)
- Creates default configurations when files are missing

**Usage**:
```rust
let config_manager = ConfigManager::new("AutoQAC Data")?;
let main_config = config_manager.load_main_config()?;
let user_config = config_manager.load_user_config()?;
```

#### 3. **CleaningService** ([src/services/cleaning.rs](src/services/cleaning.rs))

Pure business logic for plugin cleaning (framework-agnostic, no Qt/Slint dependencies).

- Builds xEdit command-line arguments
- Executes subprocess with timeout
- Parses xEdit log output for statistics (regex-based)
- Supports MO2 integration mode
- Handles experimental Partial Forms feature

**Workflow**:
```rust
let service = CleaningService::new();

// Build command
let command = service.build_cleaning_command(
    xedit_path,
    plugin_name,
    Some("FO4"),  // Game mode for universal xEdit
    mo2_path,     // Optional MO2 integration
    partial_forms // Experimental feature flag
);

// Execute with timeout
let exit_code = service.execute_cleaning_command(
    &command,
    Duration::from_secs(300)
).await?;

// Parse log file
let stats = service.parse_log_file(log_path)?;
println!("Removed {} ITMs, undeleted {} UDRs", stats.removed, stats.undeleted);
```

#### 4. **GuiController** ([src/ui/controller.rs](src/ui/controller.rs))

Orchestrates the complete GUI workflow, bridging Slint UI with business logic.

- Creates and configures Slint UI
- Wires UI callbacks to async operations
- Manages file dialogs (rfd)
- Coordinates state updates with UI rendering
- Handles cleaning workflow with cancellation support

**Initialization**:
```rust
let controller = GuiController::new(
    state_manager,
    config_manager,
    main_config,
    tokio_runtime_handle
)?;

controller.run()?; // Blocks until window closes
```

#### 5. **EventLoopBridge** ([src/ui/bridge.rs](src/ui/bridge.rs))

Coordinates between tokio async runtime and Slint's synchronous event loop.

- Uses channels to communicate between execution contexts
- Provides `spawn_async()` to run async tasks from UI callbacks
- Provides `update_ui()` to update UI from async tasks
- Uses `Weak<AppWindow>` to prevent memory leaks

**Pattern**:
```rust
// From UI callback (sync) → async task
ui.on_start_cleaning(move || {
    bridge.spawn_async(|| async move {
        // Run async cleaning workflow
        controller.run_cleaning_workflow().await;
    });
});

// From async task → UI update
bridge_handle.update_ui(move |ui| {
    ui.set_progress_current(current as i32);
});
```

### Threading Model

The application uses a **hybrid threading model**:

```
┌──────────────────────────────────────────────────────────────┐
│                    Main Thread                               │
│  - Runs Slint event loop (blocking, synchronous)             │
│  - Handles UI rendering and user input                       │
│  - Processes callbacks from UI interactions                  │
└──────────────────────────────────────────────────────────────┘
                          ↓ EventLoopBridge
┌──────────────────────────────────────────────────────────────┐
│             Tokio Runtime (4 Worker Threads)                 │
│  - Handles async operations (xEdit subprocess execution)     │
│  - File I/O (reading load order, parsing logs)               │
│  - State updates and event broadcasting                      │
└──────────────────────────────────────────────────────────────┘
                          ↓ Broadcast Channel
┌──────────────────────────────────────────────────────────────┐
│              State Subscription Thread                       │
│  - Listens for StateChange events                            │
│  - Triggers UI updates via EventLoopBridge                   │
│  - Logs state transitions                                    │
└──────────────────────────────────────────────────────────────┘
```

**Key Constraints**:
- **Single xEdit Process**: Only 1 concurrent xEdit subprocess allowed (enforced by tokio Semaphore)
- **Main Thread Blocking**: Slint's `run()` blocks until window closes
- **Async Operations**: All I/O and subprocess execution runs on tokio workers
- **UI Updates**: Must happen on main thread via `EventLoopBridge::update_ui()`

### Module Organization

```
src/
├── main.rs                  # Application entry point (84 lines)
│                            # - Logging setup
│                            # - Tokio runtime creation
│                            # - StateManager and ConfigManager initialization
│                            # - GuiController launch
│
├── lib.rs                   # Library root with re-exports
│
├── models/                  # Core data structures
│   ├── mod.rs              # Module exports
│   ├── app_state.rs        # AppState (central application state)
│   └── config.rs           # MainConfig, UserConfig, IgnoreConfig
│
├── state/                   # State management
│   └── mod.rs              # StateManager (thread-safe with events)
│
├── config/                  # Configuration management
│   └── mod.rs              # ConfigManager (YAML I/O)
│
├── services/                # Business logic (framework-agnostic)
│   ├── mod.rs              # Module exports
│   ├── cleaning.rs         # CleaningService (xEdit subprocess management)
│   └── game_detection.rs   # Game type detection from executable/load order
│
├── ui/                      # GUI layer
│   ├── mod.rs              # UI module exports
│   ├── bridge.rs           # EventLoopBridge (async ↔ UI coordination)
│   └── controller.rs       # GuiController (main UI orchestrator)
│
└── logging.rs              # Logging infrastructure (tracing + file rotation)

ui/                          # Slint UI definitions
└── main.slint              # Main window UI (Fluent Design)
    # - Configuration panel (file pickers)
    # - Progress display (with statistics badges)
    # - Status bar (contextual messages)
    # - Dialogs (About, Error, Partial Forms warning)
```

## Technology Stack

### Core Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| **slint** | 1.13 | GUI framework with Microsoft Fluent Design |
| **tokio** | 1.41 | Async runtime for subprocess execution |
| **serde** / **serde_yaml_ng** | 1.0 / 0.10 | Configuration serialization (YAML) |
| **tracing** / **tracing-subscriber** | 0.1 / 0.3 | Structured logging with file rotation |
| **anyhow** | 1.0 | Application-level error handling |
| **thiserror** | 2.0 | Library-level error types |
| **camino** | 1.1 | UTF-8 path handling (Windows-safe) |
| **indexmap** | 2.0 | Order-preserving maps for configs |
| **rfd** | 0.15 | Native file dialogs |
| **regex** | 1.10 | xEdit log parsing |

### Development Dependencies

| Crate | Purpose |
|-------|---------|
| **tokio-test** | Async test utilities |
| **tempfile** | Temporary files for tests |
| **proptest** | Property-based testing |
| **criterion** | Benchmarking |
| **mockall** | Mocking for unit tests |

## Building & Running

### Prerequisites

- **Rust**: 1.70+ (edition 2021)
  ```bash
  rustup update
  ```

- **Platform**: Windows 10/11 (primary), Linux/macOS (secondary)

### Build Commands

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run application
cargo run

# Run with logging output
RUST_LOG=autoqac=debug cargo run

# Generate documentation
cargo doc --open

# Run tests
cargo test

# Run benchmarks
cargo bench

# Check code (no build)
cargo check
```

### Build Configuration

The release profile is optimized for size and performance:

```toml
[profile.release]
opt-level = 3           # Maximum optimization
lto = true              # Link-Time Optimization
codegen-units = 1       # Single codegen unit for better optimization
strip = true            # Strip debug symbols
```

## Usage

### GUI Application

1. **Launch** the application:
   ```bash
   cargo run --release
   ```

2. **Configure paths**:
   - Click **Browse** next to "Load Order" → select your `plugins.txt` or `loadorder.txt`
   - Click **Browse** next to "xEdit" → select your xEdit executable (FO4Edit.exe, SSEEdit.exe, etc.)
   - (Optional) Click **Browse** next to "MO2" → select ModOrganizer.exe for MO2 integration

3. **Configure settings**:
   - **Partial Forms**: Enable experimental partial forms cleaning (⚠ USE WITH CAUTION)
   - **Timeout**: Adjust per-plugin timeout (default: 300s)

4. **Start cleaning**:
   - Click **Start Cleaning** button
   - Monitor progress in real-time with statistics badges
   - Cancel anytime with **Cancel** button

5. **Review results**:
   - Check aggregate statistics (total UDRs, ITMs, navmeshes, partial forms)
   - View individual plugin results in the summary

### Library Usage

The `autoqac` library can be used programmatically:

```rust
use autoqac::{StateManager, ConfigManager, services::CleaningService};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize state
    let state = Arc::new(StateManager::new());

    // Load configuration
    let config_manager = ConfigManager::new("AutoQAC Data")?;
    let main_config = config_manager.load_main_config()?;
    let user_config = config_manager.load_user_config()?;

    // Load user config into state
    state.load_from_user_config(&user_config);

    // Create cleaning service
    let service = CleaningService::new();

    // Get xEdit path and plugin to clean
    let xedit_path = user_config.pact_settings.xedit_exe;
    let plugin_name = "MyPlugin.esp";

    // Check skip list
    if let Some(skip_list) = main_config.get_skip_list("FO4") {
        if skip_list.iter().any(|s| s.eq_ignore_ascii_case(plugin_name)) {
            println!("Plugin {} is in skip list, skipping", plugin_name);
            return Ok(());
        }
    }

    // Build cleaning command
    let command = service.build_cleaning_command(
        &xedit_path,
        plugin_name,
        Some("FO4"),  // Game mode for universal xEdit
        None,         // No MO2 integration
        false,        // No partial forms
    );

    println!("Executing: {:?}", command);

    // Execute cleaning
    let exit_code = service.execute_cleaning_command(
        &command,
        Duration::from_secs(300),
    ).await?;

    if exit_code == 0 {
        println!("Cleaning completed successfully");

        // Parse log file for statistics
        let log_path = service.get_log_path(&xedit_path, plugin_name)?;
        if let Ok(stats) = service.parse_log_file(&log_path) {
            println!("Statistics: {} UDRs undeleted, {} ITMs removed, {} navmeshes deleted",
                     stats.undeleted, stats.removed, stats.skipped);
        }
    } else {
        println!("Cleaning failed with exit code: {}", exit_code);
    }

    Ok(())
}
```

## Configuration

Configuration files are stored in `AutoQAC Data/` directory:

### 1. AutoQAC Main.yaml

Game configurations, xEdit executable lists, and skip lists.

```yaml
PACT_Data:
  version: "3.0.0"
  version_date: "25.01.14"

  XEdit_Lists:
    FO4:
      - FO4Edit.exe
      - FO4Edit64.exe
    SSE:
      - SSEEdit.exe
      - SSEEdit64.exe
    # ... other games

  Skip_Lists:
    FO4:
      - Fallout4.esm
      - DLCCoast.esm
      - DLCNukaWorld.esm
      # ... base game files
    SSE:
      - Skyrim.esm
      - Update.esm
      # ... base game files

  Errors:
    missing_masters: "Plugin has missing master files"
    # ... error messages

  Warnings:
    partial_forms: "Partial forms may cause instability"
    # ... warning messages
```

### 2. AutoQAC Config.yaml

User settings and file paths.

```yaml
PACT_Settings:
  Update Check: true
  Stat Logging: true
  Cleaning Timeout: 300        # Seconds per plugin
  Journal Expiration: 7        # Days to keep log files
  LoadOrder TXT: "C:\\Games\\Fallout 4\\Data\\plugins.txt"
  XEDIT EXE: "C:\\Tools\\FO4Edit.exe"
  MO2 EXE: ""                  # Optional MO2 path
  Partial Forms: false         # Experimental feature
  Debug Mode: false
```

### 3. PACT Ignore.yaml

Additional plugins to ignore during cleaning.

```yaml
PACT_Ignore_FO4:
  - MyModInDevelopment.esp
  - ExperimentalPlugin.esp

PACT_Ignore_SSE:
  - TestMod.esp
```

## Development

### Project Structure

See [Module Organization](#module-organization) for detailed breakdown.

### Testing

The codebase includes comprehensive test coverage:

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_state_manager

# Run tests for specific module
cargo test state::

# Run with coverage (requires tarpaulin)
cargo tarpaulin --out Html
```

**Test Organization**:
- **Unit tests**: Embedded in source files (`#[cfg(test)] mod tests`)
- **Integration tests**: `tests/` directory (not yet implemented)
- **Property-based tests**: Using `proptest` for fuzzing
- **Benchmarks**: `benches/` directory (not yet implemented)

**Coverage Goals**:
- Unit tests: 60 passing tests across all modules
- Critical paths: State management, configuration loading, subprocess execution
- Edge cases: Timeout handling, cancellation, malformed configs

### Documentation

Generate and view documentation:

```bash
# Generate docs for this crate only
cargo doc --no-deps --open

# Generate docs for all dependencies
cargo doc --open

# Check for broken links in docs
cargo doc --no-deps 2>&1 | grep -i "warning"
```

**Documentation Standards**:
- All public APIs have doc comments with examples
- Module-level documentation explains purpose and architecture
- Complex algorithms have inline comments
- Examples use `ignore` attribute (since they require runtime setup)

### Code Quality

```bash
# Format code
cargo fmt

# Lint with Clippy
cargo clippy -- -D warnings

# Check for unused dependencies
cargo machete  # Requires cargo-machete
```

## Supported Games

| Game | Short Code | xEdit Executables | Master ESM |
|------|------------|-------------------|------------|
| **Fallout 3** | FO3 | FO3Edit.exe, FO3Edit64.exe | Fallout3.esm |
| **Fallout New Vegas** | FNV | FNVEdit.exe, FNVEdit64.exe | FalloutNV.esm |
| **Fallout 4** | FO4 | FO4Edit.exe, FO4Edit64.exe | Fallout4.esm |
| **Skyrim Special Edition** | SSE | SSEEdit.exe, SSEEdit64.exe, TES5Edit.exe | Skyrim.esm |
| **Fallout 4 VR** | FO4VR | FO4VREdit.exe, FO4VREdit64.exe | Fallout4.esm |
| **Skyrim VR** | SkyrimVR | TES5VREdit.exe | Skyrim.esm |
| **Tale of Two Wastelands** | TTW | TTWEdit.exe | TaleOfTwoWastelands.esm |

**Universal xEdit**: The application also supports universal xEdit executables (`xEdit.exe`, `xEdit64.exe`) with game mode auto-detection from load order files.

## License

MIT License - See [LICENSE](../LICENSE) for details.

---

**Part of the XEdit-PACT Project**

For more information about the overall project, see the [main README](../README.md).
