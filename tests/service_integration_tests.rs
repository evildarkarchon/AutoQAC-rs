//! Integration tests for CleaningService
//!
//! These tests verify:
//! - Command building for various scenarios
//! - Log file path generation
//! - Integration with StateManager
//! - Error handling workflows

use autoqac::services::CleaningService;
use camino::Utf8Path;

#[test]
fn test_build_cleaning_command_basic() {
    let service = CleaningService::new();

    let command = service.build_cleaning_command(
        Utf8Path::new("C:\\Tools\\FO4Edit.exe"),
        "MyPlugin.esp",
        None,  // No game mode (FO4Edit knows it's FO4)
        None,  // No MO2
        false, // No partial forms
    );

    // Verify command string contains expected flags
    assert!(command.contains("-QAC"));
    assert!(command.contains("-autoexit"));
    assert!(command.contains("-autoload"));
    assert!(command.contains("MyPlugin.esp"));
    assert!(command.contains("FO4Edit.exe"));
}

#[test]
fn test_build_cleaning_command_with_game_mode() {
    let service = CleaningService::new();

    let command = service.build_cleaning_command(
        Utf8Path::new("C:\\Tools\\xEdit.exe"), // Universal xEdit
        "MyPlugin.esp",
        Some("FO4"), // Specify game mode
        None,
        false,
    );

    // Should contain game mode flag
    assert!(command.contains("-FO4"));
}

#[test]
fn test_build_cleaning_command_with_mo2() {
    let service = CleaningService::new();

    let command = service.build_cleaning_command(
        Utf8Path::new("C:\\Tools\\FO4Edit.exe"),
        "MyPlugin.esp",
        None,
        Some(Utf8Path::new("C:\\MO2\\ModOrganizer.exe")), // MO2 path
        false,
    );

    // With MO2, the command should contain "run"
    assert!(command.contains("ModOrganizer.exe"));
    assert!(command.contains("run"));
    assert!(command.contains("FO4Edit.exe"));
}

#[test]
fn test_build_cleaning_command_with_partial_forms() {
    let service = CleaningService::new();

    let command = service.build_cleaning_command(
        Utf8Path::new("C:\\Tools\\FO4Edit.exe"),
        "MyPlugin.esp",
        None,
        None,
        true, // Enable partial forms
    );

    // Should contain partial forms flags
    assert!(command.contains("-iknowwhatimdoing"));
    assert!(command.contains("-allowmakepartial"));
}

#[test]
fn test_build_cleaning_command_all_options() {
    let service = CleaningService::new();

    let command = service.build_cleaning_command(
        Utf8Path::new("C:\\Tools\\xEdit.exe"),
        "MyPlugin.esp",
        Some("SSE"),                                      // Game mode
        Some(Utf8Path::new("C:\\MO2\\ModOrganizer.exe")), // MO2
        true,                                             // Partial forms
    );

    // Verify all expected strings present
    assert!(command.contains("run"));
    assert!(command.contains("-SSE"));
    assert!(command.contains("-QAC"));
    assert!(command.contains("-autoexit"));
    assert!(command.contains("-autoload"));
    assert!(command.contains("-iknowwhatimdoing"));
    assert!(command.contains("-allowmakepartial"));
}

#[test]
fn test_get_log_paths() {
    let service = CleaningService::new();

    let (main_log, exception_log) = service
        .get_log_paths(
            Utf8Path::new("C:\\Tools\\FO4Edit.exe"),
            None, // No game type (specific xEdit)
        )
        .unwrap();

    // Main log file should be in same directory as xEdit
    assert!(main_log.starts_with("C:\\Tools\\"));

    // Log file should contain xEdit name
    assert!(main_log.as_str().contains("FO4EDIT"));

    // Exception log should also be in same directory
    assert!(exception_log.starts_with("C:\\Tools\\"));
    assert!(exception_log.as_str().contains("FO4EDIT"));
    assert!(exception_log.as_str().contains("Exception"));
}

#[test]
fn test_get_log_paths_with_game_type() {
    let service = CleaningService::new();

    let (main_log, exception_log) = service
        .get_log_paths(
            Utf8Path::new("C:\\Tools\\xEdit.exe"),
            Some("SSE"), // Universal xEdit with game mode
        )
        .unwrap();

    // Log files should use game-specific names
    // Format: SSEEdit_log.txt (game type uppercase + "Edit")
    assert!(main_log.as_str().contains("SSEEdit_log.txt"));
    assert!(exception_log.as_str().contains("SSEEditException.log"));
}

#[test]
fn test_command_escaping_special_characters() {
    let service = CleaningService::new();

    // Test with plugin name containing spaces
    let command = service.build_cleaning_command(
        Utf8Path::new("C:\\Tools\\FO4Edit.exe"),
        "My Awesome Plugin.esp",
        None,
        None,
        false,
    );

    // Plugin name should be quoted in command
    assert!(command.contains("My Awesome Plugin.esp"));
}

#[test]
fn test_integration_with_state_manager() {
    use autoqac::StateManager;
    use std::sync::Arc;

    let state = Arc::new(StateManager::new());
    let service = CleaningService::new();

    // Start cleaning workflow
    state.start_cleaning(vec!["plugin1.esp".to_string(), "plugin2.esp".to_string()]);

    // Verify state
    let snapshot = state.snapshot();
    assert!(snapshot.is_cleaning);
    assert_eq!(snapshot.total_plugins, 2);

    // Simulate updating progress
    state.update_progress("plugin1.esp".to_string(), "Cleaning...".to_string());

    let snapshot = state.snapshot();
    assert_eq!(snapshot.current_plugin, Some("plugin1.esp".to_string()));

    // Simulate adding result
    state.add_plugin_result(
        "plugin1.esp".to_string(),
        "cleaned",
        "Removed 5 ITMs".to_string(),
        None,
    );

    let snapshot = state.snapshot();
    assert_eq!(snapshot.cleaned_plugins.len(), 1);
    assert!(snapshot.cleaned_plugins.contains("plugin1.esp"));

    // Stop cleaning
    state.stop_cleaning();

    let snapshot = state.snapshot();
    assert!(!snapshot.is_cleaning);
}

#[test]
fn test_cleaning_workflow_state_transitions() {
    use autoqac::StateManager;
    use std::sync::Arc;

    let state = Arc::new(StateManager::new());

    // Initial state
    assert!(!state.read(|s| s.is_cleaning));

    // Start cleaning
    state.start_cleaning(vec![
        "plugin1.esp".to_string(),
        "plugin2.esp".to_string(),
        "plugin3.esp".to_string(),
    ]);

    assert!(state.read(|s| s.is_cleaning));
    assert_eq!(state.read(|s| s.total_plugins), 3);

    // Process plugins
    for (i, plugin) in ["plugin1.esp", "plugin2.esp", "plugin3.esp"]
        .iter()
        .enumerate()
    {
        state.update_progress(plugin.to_string(), format!("Cleaning {}", plugin));
        state.add_plugin_result(
            plugin.to_string(),
            "cleaned",
            format!("Done with {}", plugin),
            None,
        );

        let progress = state.read(|s| s.progress);
        assert_eq!(progress, i + 1);
    }

    // Final state
    assert_eq!(state.read(|s| s.cleaned_plugins.len()), 3);

    // Stop cleaning
    state.stop_cleaning();
    assert!(!state.read(|s| s.is_cleaning));
}

#[test]
fn test_error_handling_workflow() {
    use autoqac::StateManager;
    use std::sync::Arc;

    let state = Arc::new(StateManager::new());

    state.start_cleaning(vec![
        "good_plugin.esp".to_string(),
        "bad_plugin.esp".to_string(),
    ]);

    // First plugin succeeds
    state.add_plugin_result(
        "good_plugin.esp".to_string(),
        "cleaned",
        "Success".to_string(),
        None,
    );

    // Second plugin fails
    state.add_plugin_result(
        "bad_plugin.esp".to_string(),
        "failed",
        "Missing masters".to_string(),
        None,
    );

    let snapshot = state.snapshot();
    assert_eq!(snapshot.cleaned_plugins.len(), 1);
    assert_eq!(snapshot.failed_plugins.len(), 1);
    assert!(snapshot.cleaned_plugins.contains("good_plugin.esp"));
    assert!(snapshot.failed_plugins.contains("bad_plugin.esp"));
}

#[test]
fn test_skip_plugin_workflow() {
    use autoqac::StateManager;
    use std::sync::Arc;

    let state = Arc::new(StateManager::new());

    state.start_cleaning(vec![
        "regular_plugin.esp".to_string(),
        "Fallout4.esm".to_string(), // Base game file - should skip
    ]);

    // Regular plugin cleaned
    state.add_plugin_result(
        "regular_plugin.esp".to_string(),
        "cleaned",
        "Done".to_string(),
        None,
    );

    // Base game file skipped
    state.add_plugin_result(
        "Fallout4.esm".to_string(),
        "skipped",
        "In skip list".to_string(),
        None,
    );

    let snapshot = state.snapshot();
    assert_eq!(snapshot.cleaned_plugins.len(), 1);
    assert_eq!(snapshot.skipped_plugins.len(), 1);
    assert!(snapshot.skipped_plugins.contains("Fallout4.esm"));
}

#[test]
fn test_statistics_tracking_workflow() {
    use autoqac::StateManager;
    use autoqac::services::CleaningStats;
    use std::sync::Arc;

    let state = Arc::new(StateManager::new());

    state.start_cleaning(vec!["plugin1.esp".to_string(), "plugin2.esp".to_string()]);

    // Plugin 1 with stats
    let stats1 = CleaningStats {
        undeleted: 3,
        removed: 5,
        skipped: 1,
        partial_forms: 0,
    };

    state.add_plugin_result(
        "plugin1.esp".to_string(),
        "cleaned",
        "Done".to_string(),
        Some(stats1),
    );

    // Plugin 2 with different stats
    let stats2 = CleaningStats {
        undeleted: 2,
        removed: 8,
        skipped: 0,
        partial_forms: 1,
    };

    state.add_plugin_result(
        "plugin2.esp".to_string(),
        "cleaned",
        "Done".to_string(),
        Some(stats2),
    );

    // Verify aggregate statistics
    let snapshot = state.snapshot();
    assert_eq!(snapshot.total_undeleted, 5); // 3 + 2
    assert_eq!(snapshot.total_removed, 13); // 5 + 8
    assert_eq!(snapshot.total_skipped, 1); // 1 + 0
    assert_eq!(snapshot.total_partial_forms, 1); // 0 + 1
}
