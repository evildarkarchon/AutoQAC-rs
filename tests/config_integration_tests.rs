//! Integration tests for ConfigManager and configuration file handling
//!
//! These tests verify:
//! - Configuration loading and saving
//! - Default configuration generation
//! - Legacy configuration migration
//! - Configuration validation
//! - Integration with StateManager

use autoqac::ConfigManager;
use camino::Utf8PathBuf;
use std::fs;
use tempfile::TempDir;

fn create_test_config_dir() -> (TempDir, Utf8PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap();
    (temp_dir, config_path)
}

#[test]
fn test_create_config_manager() {
    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = ConfigManager::new(&config_path).unwrap();

    assert_eq!(manager.config_dir(), &config_path);
}

#[test]
fn test_load_default_main_config() {
    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = ConfigManager::new(&config_path).unwrap();

    // Main config file doesn't exist, should return defaults
    let main_config = manager.load_main_config().unwrap();

    // Verify default structure
    assert_eq!(main_config.pact_data.version, "3.0.0");
    assert!(main_config.pact_data.xedit_lists.contains_key("FO4"));
    assert!(main_config.pact_data.xedit_lists.contains_key("SSE"));
    assert!(main_config.pact_data.skip_lists.contains_key("FO4"));
    assert!(main_config.pact_data.skip_lists.contains_key("SSE"));
}

#[test]
fn test_load_default_user_config() {
    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = ConfigManager::new(&config_path).unwrap();

    // User config file doesn't exist, should return defaults
    let user_config = manager.load_user_config().unwrap();

    // Verify default values
    assert_eq!(user_config.pact_settings.cleaning_timeout, 300);
    assert_eq!(user_config.pact_settings.journal_expiration, 7);
    assert!(user_config.pact_settings.update_check);
    assert!(!user_config.pact_settings.partial_forms);
}

#[test]
fn test_save_and_load_main_config() {
    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = ConfigManager::new(&config_path).unwrap();

    // Load default config
    let mut main_config = manager.load_main_config().unwrap();

    // Modify it
    main_config.pact_data.version = "3.1.0".to_string();

    // Save it
    manager.save_main_config(&main_config).unwrap();

    // Load it again
    let loaded_config = manager.load_main_config().unwrap();

    assert_eq!(loaded_config.pact_data.version, "3.1.0");
}

#[test]
fn test_save_and_load_user_config() {
    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = ConfigManager::new(&config_path).unwrap();

    // Create custom user config
    let mut user_config = autoqac::UserConfig::default();
    user_config.pact_settings.cleaning_timeout = 600;
    user_config.pact_settings.partial_forms = true;
    user_config.pact_settings.xedit_exe = "C:\\Tools\\FO4Edit.exe".to_string();

    // Save it
    manager.save_user_config(&user_config).unwrap();

    // Load it again
    let loaded_config = manager.load_user_config().unwrap();

    assert_eq!(loaded_config.pact_settings.cleaning_timeout, 600);
    assert!(loaded_config.pact_settings.partial_forms);
    assert_eq!(
        loaded_config.pact_settings.xedit_exe,
        "C:\\Tools\\FO4Edit.exe"
    );
}

#[test]
fn test_legacy_config_migration() {
    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = ConfigManager::new(&config_path).unwrap();

    // Create legacy "PACT Settings.yaml" file
    let legacy_path = config_path.join("PACT Settings.yaml");
    let legacy_content = r#"
PACT_Settings:
  Update Check: false
  Stat Logging: true
  Cleaning Timeout: 450
  Journal Expiration: 14
  LoadOrder TXT: "C:\\Games\\Fallout4\\loadorder.txt"
  XEDIT EXE: "C:\\Tools\\FO4Edit.exe"
  MO2 EXE: ""
  Partial Forms: true
  Debug Mode: false
"#;
    fs::write(&legacy_path, legacy_content).unwrap();

    // Load config (should find legacy file)
    let user_config = manager.load_user_config().unwrap();

    // Verify legacy values were loaded
    assert_eq!(user_config.pact_settings.cleaning_timeout, 450);
    assert_eq!(user_config.pact_settings.journal_expiration, 14);
    assert!(!user_config.pact_settings.update_check);
    assert!(user_config.pact_settings.stat_logging);
    assert!(user_config.pact_settings.partial_forms);
    assert_eq!(
        user_config.pact_settings.loadorder_txt,
        "C:\\Games\\Fallout4\\loadorder.txt"
    );
}

#[test]
fn test_ignore_config_load_and_save() {
    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = ConfigManager::new(&config_path).unwrap();

    // Load default ignore config
    let mut ignore_config = manager.load_ignore_config().unwrap();

    // Modify it
    ignore_config.fo4 = vec!["MyMod.esp".to_string(), "AnotherMod.esp".to_string()];

    // Save it
    manager.save_ignore_config(&ignore_config).unwrap();

    // Load it again
    let loaded_config = manager.load_ignore_config().unwrap();

    assert_eq!(loaded_config.fo4.len(), 2);
    assert!(loaded_config.fo4.contains(&"MyMod.esp".to_string()));
    assert!(loaded_config.fo4.contains(&"AnotherMod.esp".to_string()));
}

#[test]
fn test_skip_list_functionality() {
    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = ConfigManager::new(&config_path).unwrap();
    let main_config = manager.load_main_config().unwrap();

    // Test FO4 skip list
    let fo4_skip_list = main_config.get_skip_list("FO4").unwrap();
    assert!(fo4_skip_list.contains(&"Fallout4.esm".to_string()));
    assert!(fo4_skip_list.contains(&"DLCCoast.esm".to_string()));

    // Test plugin should be skipped
    assert!(main_config.should_skip_plugin("FO4", "Fallout4.esm"));
    assert!(main_config.should_skip_plugin("FO4", "DLCCoast.esm"));

    // Case insensitive check
    assert!(main_config.should_skip_plugin("FO4", "fallout4.esm"));

    // User plugin should not be skipped
    assert!(!main_config.should_skip_plugin("FO4", "MyMod.esp"));
}

#[test]
fn test_xedit_list_functionality() {
    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = ConfigManager::new(&config_path).unwrap();
    let main_config = manager.load_main_config().unwrap();

    // Test FO4 xEdit list
    let fo4_xedit_list = main_config.get_xedit_list("FO4").unwrap();
    assert!(fo4_xedit_list.contains(&"FO4Edit.exe".to_string()));
    assert!(fo4_xedit_list.contains(&"FO4Edit64.exe".to_string()));

    // Test SSE xEdit list
    let sse_xedit_list = main_config.get_xedit_list("SSE").unwrap();
    assert!(sse_xedit_list.contains(&"SSEEdit.exe".to_string()));
    assert!(sse_xedit_list.contains(&"SSEEdit64.exe".to_string()));
}

#[test]
fn test_config_integration_with_state() {
    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = ConfigManager::new(&config_path).unwrap();

    // Create and save user config
    let mut user_config = autoqac::UserConfig::default();
    user_config.pact_settings.loadorder_txt = "C:\\Games\\loadorder.txt".to_string();
    user_config.pact_settings.xedit_exe = "C:\\Tools\\xEdit.exe".to_string();
    user_config.pact_settings.mo2_exe = "C:\\MO2\\ModOrganizer.exe".to_string();
    user_config.pact_settings.partial_forms = true;
    user_config.pact_settings.cleaning_timeout = 600;

    manager.save_user_config(&user_config).unwrap();

    // Load into StateManager
    use autoqac::StateManager;
    use std::sync::Arc;

    let state = Arc::new(StateManager::new());
    let loaded_config = manager.load_user_config().unwrap();
    state.load_from_user_config(&loaded_config);

    // Verify state was populated correctly
    let snapshot = state.snapshot();
    assert!(snapshot.is_load_order_configured);
    assert!(snapshot.is_xedit_configured);
    assert!(snapshot.is_mo2_configured);
    assert!(snapshot.is_fully_configured());
    assert!(snapshot.partial_forms_enabled);
    assert_eq!(
        snapshot.cleaning_timeout,
        std::time::Duration::from_secs(600)
    );
}

#[test]
fn test_all_supported_games_have_configs() {
    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = ConfigManager::new(&config_path).unwrap();
    let main_config = manager.load_main_config().unwrap();

    // All supported games should have entries
    let games = vec!["FO3", "FNV", "FO4", "SSE"];

    for game in games {
        assert!(
            main_config.pact_data.xedit_lists.contains_key(game),
            "Game {} should have xEdit list",
            game
        );
        assert!(
            main_config.pact_data.skip_lists.contains_key(game),
            "Game {} should have skip list",
            game
        );

        // Verify skip lists contain base game files
        let skip_list = main_config.get_skip_list(game).unwrap();
        assert!(
            !skip_list.is_empty(),
            "Game {} should have non-empty skip list",
            game
        );
    }
}

#[test]
fn test_config_directory_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf())
        .unwrap()
        .join("nonexistent_dir");

    // Directory doesn't exist yet
    assert!(!config_path.exists());

    // Creating ConfigManager should create the directory
    let _manager = ConfigManager::new(&config_path).unwrap();

    // Directory should now exist
    assert!(config_path.exists());
}

#[test]
fn test_invalid_yaml_handling() {
    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = ConfigManager::new(&config_path).unwrap();

    // Create invalid YAML file
    let main_config_path = config_path.join("AutoQAC Main.yaml");
    fs::write(&main_config_path, "invalid: yaml: content: {{").unwrap();

    // Loading should return error
    let result = manager.load_main_config();
    assert!(result.is_err(), "Should fail to parse invalid YAML");
}

#[test]
fn test_concurrent_config_access() {
    use std::sync::Arc;

    let (_temp_dir, config_path) = create_test_config_dir();
    let manager = Arc::new(ConfigManager::new(&config_path).unwrap());

    // Spawn multiple threads reading config concurrently
    let mut handles = vec![];

    for _ in 0..10 {
        let manager_clone = manager.clone();
        let handle = std::thread::spawn(move || {
            let _config = manager_clone.load_main_config().unwrap();
        });
        handles.push(handle);
    }

    // All threads should complete successfully
    for handle in handles {
        handle.join().unwrap();
    }
}
