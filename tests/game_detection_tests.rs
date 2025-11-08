//! Integration tests for game detection functionality
//!
//! These tests verify:
//! - Game detection from xEdit executable filename
//! - Game detection from load order file contents
//! - Fallback detection mechanisms
//! - Integration with ConfigManager

use autoqac::services::{detect_game_from_load_order, detect_xedit_game};
use camino::Utf8Path;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_detect_fo4_from_executable() {
    assert_eq!(
        detect_xedit_game("FO4Edit.exe", None),
        Some("FO4".to_string())
    );
    assert_eq!(
        detect_xedit_game("FO4Edit64.exe", None),
        Some("FO4".to_string())
    );
    assert_eq!(
        detect_xedit_game("C:\\Tools\\FO4Edit.exe", None),
        Some("FO4".to_string())
    );
}

#[test]
fn test_detect_sse_from_executable() {
    assert_eq!(
        detect_xedit_game("SSEEdit.exe", None),
        Some("SSE".to_string())
    );
    assert_eq!(
        detect_xedit_game("SSEEdit64.exe", None),
        Some("SSE".to_string())
    );
    assert_eq!(
        detect_xedit_game("TES5Edit.exe", None),
        Some("SSE".to_string())
    );
}

#[test]
fn test_detect_fnv_from_executable() {
    assert_eq!(
        detect_xedit_game("FNVEdit.exe", None),
        Some("FNV".to_string())
    );
    assert_eq!(
        detect_xedit_game("FNVEdit64.exe", None),
        Some("FNV".to_string())
    );
}

#[test]
fn test_detect_fo3_from_executable() {
    assert_eq!(
        detect_xedit_game("FO3Edit.exe", None),
        Some("FO3".to_string())
    );
    assert_eq!(
        detect_xedit_game("FO3Edit64.exe", None),
        Some("FO3".to_string())
    );
}

#[test]
fn test_detect_ttw_from_executable() {
    assert_eq!(
        detect_xedit_game("TTWEdit.exe", None),
        Some("TTW".to_string())
    );
}

#[test]
fn test_detect_fo4vr_from_executable() {
    assert_eq!(
        detect_xedit_game("FO4VREdit.exe", None),
        Some("FO4".to_string())
    );
    assert_eq!(
        detect_xedit_game("FO4VREdit64.exe", None),
        Some("FO4".to_string())
    );
}

#[test]
fn test_detect_skyrimvr_from_executable() {
    assert_eq!(
        detect_xedit_game("SkyrimVREdit.exe", None),
        Some("SSE".to_string())
    );
}

#[test]
fn test_universal_xedit_returns_none_without_load_order() {
    assert_eq!(detect_xedit_game("xEdit.exe", None), None);
    assert_eq!(detect_xedit_game("xEdit64.exe", None), None);
    assert_eq!(detect_xedit_game("xfoedit.exe", None), None);
}

#[test]
fn test_case_insensitive_detection() {
    assert_eq!(
        detect_xedit_game("fo4edit.exe", None),
        Some("FO4".to_string())
    );
    assert_eq!(
        detect_xedit_game("SSEEDIT.EXE", None),
        Some("SSE".to_string())
    );
    assert_eq!(
        detect_xedit_game("FnvEdit.EXE", None),
        Some("FNV".to_string())
    );
}

#[test]
fn test_detect_skyrim_from_load_order() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "*Skyrim.esm").unwrap();
    writeln!(temp_file, "*Update.esm").unwrap();
    writeln!(temp_file, "*Dawnguard.esm").unwrap();

    let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
    let result = detect_game_from_load_order(temp_path).unwrap();

    assert_eq!(result, Some("SSE".to_string()));
}

#[test]
fn test_detect_fallout4_from_load_order() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "*Fallout4.esm").unwrap();
    writeln!(temp_file, "*DLCRobot.esm").unwrap();
    writeln!(temp_file, "*DLCCoast.esm").unwrap();

    let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
    let result = detect_game_from_load_order(temp_path).unwrap();

    assert_eq!(result, Some("FO4".to_string()));
}

#[test]
fn test_detect_fnv_from_load_order() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "*FalloutNV.esm").unwrap();
    writeln!(temp_file, "*DeadMoney.esm").unwrap();
    writeln!(temp_file, "*HonestHearts.esm").unwrap();

    let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
    let result = detect_game_from_load_order(temp_path).unwrap();

    assert_eq!(result, Some("FNV".to_string()));
}

#[test]
fn test_detect_fo3_from_load_order() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "*Fallout3.esm").unwrap();
    writeln!(temp_file, "*Anchorage.esm").unwrap();
    writeln!(temp_file, "*ThePitt.esm").unwrap();

    let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
    let result = detect_game_from_load_order(temp_path).unwrap();

    assert_eq!(result, Some("FO3".to_string()));
}

#[test]
fn test_load_order_with_comments_and_whitespace() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "# This is a comment").unwrap();
    writeln!(temp_file, "").unwrap();
    writeln!(temp_file, "  ").unwrap();
    writeln!(temp_file, "*Skyrim.esm").unwrap();
    writeln!(temp_file, "# Another comment").unwrap();

    let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
    let result = detect_game_from_load_order(temp_path).unwrap();

    assert_eq!(result, Some("SSE".to_string()));
}

#[test]
fn test_load_order_with_prefix_characters() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "+Fallout4.esm").unwrap(); // + prefix
    writeln!(temp_file, "-DLCRobot.esm").unwrap(); // - prefix
    writeln!(temp_file, "*DLCCoast.esm").unwrap(); // * prefix

    let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
    let result = detect_game_from_load_order(temp_path).unwrap();

    assert_eq!(result, Some("FO4".to_string()));
}

#[test]
fn test_load_order_without_prefix() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "Skyrim.esm").unwrap(); // No prefix
    writeln!(temp_file, "Update.esm").unwrap();

    let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
    let result = detect_game_from_load_order(temp_path).unwrap();

    assert_eq!(result, Some("SSE".to_string()));
}

#[test]
fn test_load_order_returns_none_for_unknown_game() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "*UnknownGame.esm").unwrap();
    writeln!(temp_file, "*SomeOtherMod.esm").unwrap();

    let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
    let result = detect_game_from_load_order(temp_path).unwrap();

    assert_eq!(result, None);
}

#[test]
fn test_empty_load_order_returns_none() {
    let temp_file = NamedTempFile::new().unwrap();

    let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
    let result = detect_game_from_load_order(temp_path).unwrap();

    assert_eq!(result, None);
}

#[test]
fn test_fallback_to_load_order_for_universal_xedit() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "*Fallout4.esm").unwrap();

    let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();

    // Universal xEdit should fallback to load order detection
    let result = detect_xedit_game("xEdit.exe", Some(temp_path));

    assert_eq!(result, Some("FO4".to_string()));
}

#[test]
fn test_xedit_detection_takes_precedence_over_load_order() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "*Fallout4.esm").unwrap(); // Load order says FO4

    let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();

    // But xEdit executable says SSE - should use xEdit detection
    let result = detect_xedit_game("SSEEdit.exe", Some(temp_path));

    assert_eq!(result, Some("SSE".to_string()));
}

#[test]
fn test_load_order_case_sensitivity() {
    // NOTE: Current implementation is case-sensitive for master ESM names
    // This matches typical load order files which preserve correct casing
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "*Skyrim.esm").unwrap(); // Correct casing
    writeln!(temp_file, "*Update.esm").unwrap();

    let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
    let result = detect_game_from_load_order(temp_path).unwrap();

    assert_eq!(result, Some("SSE".to_string()));

    // Lowercase version will not be detected (current limitation)
    let mut temp_file2 = NamedTempFile::new().unwrap();
    writeln!(temp_file2, "*skyrim.esm").unwrap(); // Lowercase

    let temp_path2 = Utf8Path::from_path(temp_file2.path()).unwrap();
    let result2 = detect_game_from_load_order(temp_path2).unwrap();

    // This is expected behavior - case-sensitive matching
    assert_eq!(result2, None);
}

#[test]
fn test_integration_with_config() {
    use autoqac::ConfigManager;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = camino::Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap();

    let manager = ConfigManager::new(&config_path).unwrap();
    let main_config = manager.load_main_config().unwrap();

    // Test that xEdit lists contain the expected executables
    let fo4_list = main_config.get_xedit_list("FO4").unwrap();
    assert!(fo4_list.iter().any(|exe| exe.contains("FO4Edit")));

    let sse_list = main_config.get_xedit_list("SSE").unwrap();
    assert!(sse_list.iter().any(|exe| exe.contains("SSEEdit")));
}

#[test]
fn test_game_detection_workflow() {
    // Simulate the full workflow:
    // 1. User selects xEdit executable
    // 2. Application detects game type
    // 3. Application loads appropriate skip list

    use autoqac::ConfigManager;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let config_path = camino::Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap();

    let manager = ConfigManager::new(&config_path).unwrap();
    let main_config = manager.load_main_config().unwrap();

    // Step 1: User selects FO4Edit.exe
    let xedit_path = "C:\\Tools\\FO4Edit.exe";

    // Step 2: Detect game type
    let game_type = detect_xedit_game(xedit_path, None);
    assert_eq!(game_type, Some("FO4".to_string()));

    // Step 3: Get skip list for detected game
    let skip_list = main_config.get_skip_list("FO4").unwrap();

    // Verify skip list contains base game files
    assert!(skip_list.contains(&"Fallout4.esm".to_string()));
    assert!(skip_list.contains(&"DLCCoast.esm".to_string()));

    // Verify user plugins are not in skip list
    assert!(!skip_list.contains(&"MyMod.esp".to_string()));
}

#[test]
fn test_all_supported_xedit_variants() {
    let test_cases = vec![
        ("FO3Edit.exe", "FO3"),
        ("FO3Edit64.exe", "FO3"),
        ("FNVEdit.exe", "FNV"),
        ("FNVEdit64.exe", "FNV"),
        ("FO4Edit.exe", "FO4"),
        ("FO4Edit64.exe", "FO4"),
        ("FO4VREdit.exe", "FO4"),
        ("SSEEdit.exe", "SSE"),
        ("SSEEdit64.exe", "SSE"),
        ("TES5Edit.exe", "SSE"),
        ("SkyrimVREdit.exe", "SSE"),
        ("TTWEdit.exe", "TTW"),
    ];

    for (executable, expected_game) in test_cases {
        let result = detect_xedit_game(executable, None);
        assert_eq!(
            result,
            Some(expected_game.to_string()),
            "Failed for {}",
            executable
        );
    }
}
