//! Game detection utilities for identifying game type from xEdit executables or load order files.
//!
//! This module provides functions to auto-detect the game type (FO3, FNV, FO4, SSE, etc.)
//! based on:
//! - xEdit executable filename (FO4Edit.exe → FO4, SSEEdit.exe → SSE, etc.)
//! - Load order file contents (looking for master ESM files like Skyrim.esm, Fallout4.esm, etc.)
//!
//! # Examples
//!
//! ```ignore
//! use autoqac::services::game_detection::{detect_xedit_game, detect_game_from_load_order};
//! use camino::Utf8Path;
//!
//! // Detect from xEdit executable
//! let game = detect_xedit_game("C:/xEdit/SSEEdit.exe", None);
//! assert_eq!(game, Some("SSE".to_string()));
//!
//! // Detect from load order file
//! let load_order = Utf8Path::new("loadorder.txt");
//! let game = detect_game_from_load_order(load_order)?;
//! ```

use anyhow::{Context, Result};
use camino::Utf8Path;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Detects game type from the xEdit executable filename.
///
/// This function examines the filename of the xEdit executable to determine
/// which game it's configured for. It handles both specific xEdit versions
/// (FO4Edit.exe, SSEEdit.exe) and can optionally fall back to load order detection
/// for universal xEdit executables (xEdit.exe, xEdit64.exe).
///
/// # Arguments
///
/// * `xedit_path` - Path to the xEdit executable
/// * `load_order_path` - Optional path to load order file for fallback detection
///
/// # Returns
///
/// Game type abbreviation (FO3, FNV, FO4, SSE, TTW) if detected, None otherwise
///
/// # Examples
///
/// ```ignore
/// let game = detect_xedit_game("SSEEdit.exe", None);
/// assert_eq!(game, Some("SSE".to_string()));
/// ```
pub fn detect_xedit_game(xedit_path: &str, load_order_path: Option<&Utf8Path>) -> Option<String> {
    let filename = Utf8Path::new(xedit_path).file_stem()?.to_lowercase();

    // Map of executable name patterns to game types
    let game_map: Vec<(&str, &str)> = vec![
        ("fo3edit", "FO3"),
        ("fnvedit", "FNV"),
        ("ttwedit", "TTW"),
        ("fo4edit", "FO4"),
        ("fo4vredit", "FO4"),
        ("sseedit", "SSE"),
        ("tes5edit", "SSE"),
        ("skyrimvredit", "SSE"),
    ];

    // Try to detect from xEdit executable name
    for (pattern, game) in game_map {
        if filename.contains(pattern) {
            tracing::info!("Detected game type from xEdit: {}", game);
            return Some(game.to_string());
        }
    }

    // If xEdit detection failed and load order path is provided, try load order detection
    if let Some(lo_path) = load_order_path {
        if lo_path.exists() {
            match detect_game_from_load_order(lo_path) {
                Ok(Some(game)) => {
                    tracing::info!("Detected game type from load order: {}", game);
                    return Some(game);
                }
                Ok(None) => {
                    tracing::debug!("Could not detect game type from load order");
                }
                Err(e) => {
                    tracing::warn!("Error detecting game type from load order: {}", e);
                }
            }
        }
    }

    tracing::debug!("Could not detect game type from xEdit executable or load order");
    None
}

/// Detects game type by reading the load order file and looking for specific master ESM files.
///
/// This function reads the load order file line by line, looking for game-specific
/// master files (Skyrim.esm, Fallout4.esm, etc.) in the first few entries.
///
/// # Arguments
///
/// * `load_order_path` - Path to the load order file (plugins.txt or loadorder.txt)
///
/// # Returns
///
/// Game type abbreviation (FO3, FNV, FO4, SSE) if detected, None otherwise
///
/// # Errors
///
/// Returns an error if the file cannot be read or decoded
///
/// # Examples
///
/// ```ignore
/// use camino::Utf8Path;
/// let game = detect_game_from_load_order(Utf8Path::new("loadorder.txt"))?;
/// assert_eq!(game, Some("SSE".to_string()));
/// ```
pub fn detect_game_from_load_order(load_order_path: &Utf8Path) -> Result<Option<String>> {
    let file = File::open(load_order_path)
        .with_context(|| format!("Failed to open load order file: {}", load_order_path))?;

    let reader = BufReader::new(file);

    for line_result in reader.lines() {
        let line = line_result.context("Failed to read line from load order file")?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Remove prefix characters (*, +, -)
        let plugin_name = if let Some(first_char) = line.chars().next() {
            if first_char == '*' || first_char == '+' || first_char == '-' {
                line[1..].trim()
            } else {
                line
            }
        } else {
            line
        };

        // Check for game-specific master files
        if plugin_name.contains("Skyrim.esm") {
            return Ok(Some("SSE".to_string()));
        }
        if plugin_name.contains("Fallout3.esm") {
            return Ok(Some("FO3".to_string()));
        }
        if plugin_name.contains("FalloutNV.esm") {
            return Ok(Some("FNV".to_string()));
        }
        if plugin_name.contains("Fallout4.esm") {
            return Ok(Some("FO4".to_string()));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_detect_from_fo4edit() {
        assert_eq!(
            detect_xedit_game("FO4Edit.exe", None),
            Some("FO4".to_string())
        );
        assert_eq!(
            detect_xedit_game("FO4Edit64.exe", None),
            Some("FO4".to_string())
        );
    }

    #[test]
    fn test_detect_from_sseedit() {
        assert_eq!(
            detect_xedit_game("SSEEdit.exe", None),
            Some("SSE".to_string())
        );
        assert_eq!(
            detect_xedit_game("SSEEdit64.exe", None),
            Some("SSE".to_string())
        );
    }

    #[test]
    fn test_detect_from_fnvedit() {
        assert_eq!(
            detect_xedit_game("FNVEdit.exe", None),
            Some("FNV".to_string())
        );
    }

    #[test]
    fn test_detect_from_fo3edit() {
        assert_eq!(
            detect_xedit_game("FO3Edit.exe", None),
            Some("FO3".to_string())
        );
    }

    #[test]
    fn test_detect_from_ttwedit() {
        assert_eq!(
            detect_xedit_game("TTWEdit.exe", None),
            Some("TTW".to_string())
        );
    }

    #[test]
    fn test_universal_xedit_returns_none() {
        assert_eq!(detect_xedit_game("xEdit.exe", None), None);
        assert_eq!(detect_xedit_game("xEdit64.exe", None), None);
    }

    #[test]
    fn test_detect_skyrim_from_load_order() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "*Skyrim.esm").unwrap();
        writeln!(temp_file, "*Update.esm").unwrap();

        let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
        let result = detect_game_from_load_order(temp_path).unwrap();
        assert_eq!(result, Some("SSE".to_string()));
    }

    #[test]
    fn test_detect_fallout4_from_load_order() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "*Fallout4.esm").unwrap();
        writeln!(temp_file, "*DLCRobot.esm").unwrap();

        let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
        let result = detect_game_from_load_order(temp_path).unwrap();
        assert_eq!(result, Some("FO4".to_string()));
    }

    #[test]
    fn test_detect_fnv_from_load_order() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "*FalloutNV.esm").unwrap();

        let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
        let result = detect_game_from_load_order(temp_path).unwrap();
        assert_eq!(result, Some("FNV".to_string()));
    }

    #[test]
    fn test_detect_fo3_from_load_order() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "*Fallout3.esm").unwrap();

        let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
        let result = detect_game_from_load_order(temp_path).unwrap();
        assert_eq!(result, Some("FO3".to_string()));
    }

    #[test]
    fn test_load_order_with_comments() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "# This is a comment").unwrap();
        writeln!(temp_file, "*Skyrim.esm").unwrap();

        let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
        let result = detect_game_from_load_order(temp_path).unwrap();
        assert_eq!(result, Some("SSE".to_string()));
    }

    #[test]
    fn test_fallback_to_load_order() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "*Fallout4.esm").unwrap();

        let temp_path = Utf8Path::from_path(temp_file.path()).unwrap();
        let result = detect_xedit_game("xEdit.exe", Some(temp_path));
        assert_eq!(result, Some("FO4".to_string()));
    }
}
