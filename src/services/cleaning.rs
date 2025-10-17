use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use regex::Regex;
use std::fs;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::process::Command;
use tokio::time::timeout;

/// Result of a plugin cleaning operation
#[derive(Debug, Clone)]
pub struct CleanResult {
    pub success: bool,
    pub message: String,
    pub status: CleanStatus,
    pub duration: Duration,
    pub stats: CleaningStats,
}

/// Status of a cleaning operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanStatus {
    Cleaned,
    Failed,
    Skipped,
}

/// Statistics from a cleaning operation
#[derive(Debug, Clone, Default)]
pub struct CleaningStats {
    pub undeleted: usize,         // Undisabled References (UDR)
    pub removed: usize,           // Identical To Master (ITM)
    pub skipped: usize,           // Deleted Navmeshes
    pub partial_forms: usize,     // Partial Forms (experimental)
}

impl CleaningStats {
    /// Check if anything was actually cleaned
    pub fn has_changes(&self) -> bool {
        self.undeleted > 0 || self.removed > 0 || self.skipped > 0 || self.partial_forms > 0
    }

    /// Get a summary string of what was cleaned
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        if self.undeleted > 0 {
            parts.push(format!("{} UDRs", self.undeleted));
        }
        if self.removed > 0 {
            parts.push(format!("{} ITMs", self.removed));
        }
        if self.skipped > 0 {
            parts.push(format!("{} deleted navmeshes", self.skipped));
        }
        if self.partial_forms > 0 {
            parts.push(format!("{} partial forms", self.partial_forms));
        }

        if parts.is_empty() {
            "Nothing to clean".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Errors that can occur during cleaning
#[derive(Error, Debug)]
pub enum CleaningError {
    #[error("Plugin {0} not found")]
    PluginNotFound(String),

    #[error("xEdit executable not configured")]
    XEditNotConfigured,

    #[error("Game type not configured")]
    GameTypeNotConfigured,

    #[error("Timeout after {0:?}")]
    Timeout(Duration),

    #[error("Process error: {0}")]
    ProcessError(#[from] std::io::Error),

    #[error("Plugin has missing requirements or is empty: {0}")]
    MissingRequirements(String),

    #[error("Log file not found: {0}")]
    LogFileNotFound(String),

    #[error("Failed to parse log file: {0}")]
    LogParseError(String),
}

/// Service for cleaning plugins with xEdit
///
/// This service handles all aspects of executing xEdit's Quick Auto Clean (QAC) mode,
/// including command construction, subprocess execution, and log parsing.
///
/// # Fields
///
/// The service pre-compiles regex patterns at construction time for performance:
///
/// - `udr_pattern`: Matches "Undeleting: ..." lines to count Undisabled References (UDRs)
///   - Pattern: `Undeleting:\s*(.*)`
///   - Example match: "Undeleting: \[00000D62\] <Skyrim.esm>"
///
/// - `itm_pattern`: Matches "Removing: ..." lines to count Identical To Master records (ITMs)
///   - Pattern: `Removing:\s*(.*)`
///   - Example match: "Removing: \[FormID\] <Plugin.esp>"
///
/// - `nvm_pattern`: Matches "Skipping: ..." lines to count deleted navmeshes
///   - Pattern: `Skipping:\s*(.*)`
///   - Example match: "Skipping: \[NavMesh\] <Plugin.esp>"
///
/// - `partial_form_pattern`: Matches "Making Partial Form: ..." for experimental partial forms
///   - Pattern: `Making Partial Form:\s*(.*)`
///   - Example match: "Making Partial Form: \[00000001\]"
///
/// # Design Philosophy
///
/// - **Stateless**: All operations take explicit parameters; no hidden state
/// - **Framework-agnostic**: No GUI dependencies, works with any UI or CLI
/// - **Testable**: Pure functions with predictable I/O
/// - **Async**: Uses tokio for non-blocking subprocess execution and file I/O
pub struct CleaningService {
    /// Regex for detecting "Undeleting: ..." lines (UDRs) in xEdit logs
    udr_pattern: Regex,

    /// Regex for detecting "Removing: ..." lines (ITMs) in xEdit logs
    itm_pattern: Regex,

    /// Regex for detecting "Skipping: ..." lines (deleted navmeshes) in xEdit logs
    nvm_pattern: Regex,

    /// Regex for detecting "Making Partial Form: ..." lines in xEdit logs
    partial_form_pattern: Regex,
}

impl CleaningService {
    /// Create a new CleaningService with compiled regex patterns
    pub fn new() -> Self {
        Self {
            udr_pattern: Regex::new(r"Undeleting:\s*(.*)").expect("Invalid UDR regex"),
            itm_pattern: Regex::new(r"Removing:\s*(.*)").expect("Invalid ITM regex"),
            nvm_pattern: Regex::new(r"Skipping:\s*(.*)").expect("Invalid NVM regex"),
            partial_form_pattern: Regex::new(r"Making Partial Form:\s*(.*)").expect("Invalid partial form regex"),
        }
    }

    /// Get the log file paths for xEdit
    ///
    /// xEdit creates log files in the same directory as the executable:
    /// - Main log: `<GameMode>Edit_log.txt` or `<XEDIT>_log.txt`
    /// - Exception log: `<GameMode>EditException.log` or `<XEDIT>Exception.log`
    ///
    /// # Arguments
    /// * `xedit_exe_path` - Path to the xEdit executable
    /// * `game_type` - Optional game type for universal xEdit (e.g., "FO4", "SSE")
    pub fn get_log_paths(
        &self,
        xedit_exe_path: &Utf8Path,
        game_type: Option<&str>,
    ) -> (Utf8PathBuf, Utf8PathBuf) {
        let xedit_dir = xedit_exe_path.parent().expect("xEdit path must have parent");
        let xedit_stem = xedit_exe_path.file_stem().expect("xEdit must have stem");

        // Determine the base name for log files
        let log_base = if let Some(game) = game_type {
            // Universal xEdit with game mode: FO4Edit, SSEEdit, etc.
            format!("{}Edit", game.to_uppercase())
        } else {
            // Specific xEdit: Use the exe stem (e.g., "SSEEdit" from "SSEEdit.exe")
            xedit_stem.to_uppercase()
        };

        let main_log = xedit_dir.join(format!("{}_log.txt", log_base));
        let exception_log = xedit_dir.join(format!("{}Exception.log", log_base));

        (main_log, exception_log)
    }

    /// Clear xEdit log files before cleaning
    ///
    /// This ensures we're reading fresh logs for each plugin.
    /// Python equivalent: `clear_xedit_logs()`
    pub fn clear_logs(&self, main_log: &Utf8Path, exception_log: &Utf8Path) -> Result<()> {
        if main_log.exists() {
            fs::remove_file(main_log)
                .with_context(|| format!("Failed to remove main log: {}", main_log))?;
            tracing::debug!("Cleared main log: {}", main_log);
        }

        if exception_log.exists() {
            fs::remove_file(exception_log)
                .with_context(|| format!("Failed to remove exception log: {}", exception_log))?;
            tracing::debug!("Cleared exception log: {}", exception_log);
        }

        Ok(())
    }

    /// Check the exception log for errors
    ///
    /// Returns true if the plugin has missing requirements or is empty.
    /// Python equivalent: `check_process_exceptions()`
    pub fn check_exception_log(&self, exception_log: &Utf8Path) -> Result<bool> {
        if !exception_log.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(exception_log)
            .with_context(|| format!("Failed to read exception log: {}", exception_log))?;

        // Check for known error patterns from Python version
        let has_error = content.contains("which can not be found")
            || content.contains("which it does not have");

        if has_error {
            tracing::warn!("Exception log indicates missing requirements or empty plugin");
        }

        Ok(has_error)
    }

    /// Parse the main log file to get cleaning statistics
    ///
    /// Python equivalent: `check_cleaning_results()`
    ///
    /// # Returns
    /// CleaningStats with counts of UDRs, ITMs, navmeshes, and partial forms
    pub fn parse_log_file(&self, main_log: &Utf8Path) -> Result<CleaningStats> {
        if !main_log.exists() {
            return Err(CleaningError::LogFileNotFound(main_log.to_string()).into());
        }

        let content = fs::read_to_string(main_log)
            .with_context(|| format!("Failed to read main log: {}", main_log))?;

        let mut stats = CleaningStats::default();

        // Parse each line for cleaning patterns
        for line in content.lines() {
            if self.udr_pattern.is_match(line) {
                stats.undeleted += 1;
            } else if self.itm_pattern.is_match(line) {
                stats.removed += 1;
            } else if self.nvm_pattern.is_match(line) {
                stats.skipped += 1;
            } else if self.partial_form_pattern.is_match(line) {
                stats.partial_forms += 1;
            }
        }

        tracing::debug!(
            "Parsed log - UDRs: {}, ITMs: {}, Navmeshes: {}, Partial Forms: {}",
            stats.undeleted,
            stats.removed,
            stats.skipped,
            stats.partial_forms
        );

        Ok(stats)
    }

    /// Build the xEdit cleaning command
    ///
    /// Python equivalent: `create_bat_command()` and `_build_cleaning_command()`
    ///
    /// # Arguments
    /// * `xedit_exe_path` - Path to xEdit executable
    /// * `plugin_name` - Name of the plugin to clean
    /// * `game_type` - Optional game type for universal xEdit
    /// * `mo2_exe_path` - Optional MO2 executable path for MO2 mode
    /// * `partial_forms_enabled` - Enable partial forms cleaning
    ///
    /// # Returns
    /// The complete command string ready to execute
    pub fn build_cleaning_command(
        &self,
        xedit_exe_path: &Utf8Path,
        plugin_name: &str,
        game_type: Option<&str>,
        mo2_exe_path: Option<&Utf8Path>,
        partial_forms_enabled: bool,
    ) -> String {
        let cleaning_flag = "-QAC";

        // Add partial forms flags if enabled (experimental)
        let partial_forms_options = if partial_forms_enabled {
            " -iknowwhatimdoing -allowmakepartial"
        } else {
            ""
        };

        // Build the command based on MO2 mode and game type
        if let Some(mo2_path) = mo2_exe_path {
            // MO2 mode
            if let Some(game) = game_type {
                // Universal xEdit with game mode
                let args = format!(
                    "{} -autoexit -autoload{} \"{}\"",
                    cleaning_flag,
                    partial_forms_options,
                    plugin_name
                );
                format!(
                    "\"{}\" run \"{} -{}\" {}",
                    mo2_path,
                    xedit_exe_path,
                    game,
                    args
                )
            } else {
                // Specific xEdit
                let args = format!(
                    "{} -autoexit -autoload{} \"{}\"",
                    cleaning_flag,
                    partial_forms_options,
                    plugin_name
                );
                format!(
                    "\"{}\" run \"{}\" {}",
                    mo2_path,
                    xedit_exe_path,
                    args
                )
            }
        } else {
            // Direct mode (no MO2)
            if let Some(game) = game_type {
                // Universal xEdit with game mode
                format!(
                    "\"{}\" -{} {} -autoexit -autoload{} \"{}\"",
                    xedit_exe_path,
                    game,
                    cleaning_flag,
                    partial_forms_options,
                    plugin_name
                )
            } else {
                // Specific xEdit
                format!(
                    "\"{}\" {} -autoexit -autoload{} \"{}\"",
                    xedit_exe_path,
                    cleaning_flag,
                    partial_forms_options,
                    plugin_name
                )
            }
        }
    }

    /// Execute xEdit QAC (Quick Auto Clean) for a plugin
    ///
    /// Python equivalent: `run_process()` and subprocess execution
    ///
    /// # Arguments
    /// * `command` - The full command to execute
    /// * `timeout_duration` - Maximum time to wait for the process
    ///
    /// # Returns
    /// The process exit code (0 = success)
    pub async fn execute_cleaning_command(
        &self,
        command: &str,
        timeout_duration: Duration,
    ) -> Result<i32> {
        tracing::info!("Executing: {}", command);

        let start = Instant::now();

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", command]);
            c
        };

        // Spawn the process
        let child = cmd.spawn().context("Failed to spawn xEdit process")?;

        // Execute with timeout
        let output = timeout(timeout_duration, child.wait_with_output())
            .await
            .map_err(|_| {
                tracing::warn!("xEdit process timed out after {:?}", timeout_duration);
                CleaningError::Timeout(timeout_duration)
            })?
            .context("Failed to wait for xEdit process")?;

        let duration = start.elapsed();
        let exit_code = output.status.code().unwrap_or(-1);

        tracing::info!(
            "xEdit process completed in {:.2}s with exit code {}",
            duration.as_secs_f32(),
            exit_code
        );

        Ok(exit_code)
    }
}

impl Default for CleaningService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_paths_specific_xedit() {
        let service = CleaningService::new();
        let xedit_path = Utf8PathBuf::from("C:/Games/SSEEdit.exe");
        let (main_log, exc_log) = service.get_log_paths(&xedit_path, None);

        assert_eq!(main_log, Utf8PathBuf::from("C:/Games/SSEEDIT_log.txt"));
        assert_eq!(exc_log, Utf8PathBuf::from("C:/Games/SSEEDITException.log"));
    }

    #[test]
    fn test_log_paths_universal_xedit() {
        let service = CleaningService::new();
        let xedit_path = Utf8PathBuf::from("C:/Games/xEdit.exe");
        let (main_log, exc_log) = service.get_log_paths(&xedit_path, Some("FO4"));

        // Note: path separator might be \ or / depending on OS
        assert!(main_log.as_str().ends_with("FO4Edit_log.txt"));
        assert!(exc_log.as_str().ends_with("FO4EditException.log"));
    }

    #[test]
    fn test_parse_log_file() {
        let service = CleaningService::new();

        // Create temporary log file
        use tempfile::NamedTempFile;
        let mut temp_file = NamedTempFile::new().unwrap();
        use std::io::Write;
        writeln!(temp_file, "Undeleting: [00000001] <Example.esp>").unwrap();
        writeln!(temp_file, "Removing: [00000002] <Example.esp>").unwrap();
        writeln!(temp_file, "Removing: [00000003] <Example.esp>").unwrap();
        writeln!(temp_file, "Skipping: [00000004] <Example.esp>").unwrap();
        writeln!(temp_file, "Making Partial Form: [00000005] <Example.esp>").unwrap();
        temp_file.flush().unwrap();

        let path = Utf8PathBuf::try_from(temp_file.path().to_path_buf()).unwrap();
        let stats = service.parse_log_file(&path).unwrap();

        assert_eq!(stats.undeleted, 1);
        assert_eq!(stats.removed, 2);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.partial_forms, 1);
        assert!(stats.has_changes());
    }

    #[test]
    fn test_cleaning_stats_summary() {
        let stats = CleaningStats {
            undeleted: 2,
            removed: 5,
            skipped: 1,
            partial_forms: 0,
        };

        let summary = stats.summary();
        assert!(summary.contains("2 UDRs"));
        assert!(summary.contains("5 ITMs"));
        assert!(summary.contains("1 deleted navmeshes"));
    }

    #[test]
    fn test_build_cleaning_command_direct() {
        let service = CleaningService::new();
        let xedit = Utf8PathBuf::from("C:/Games/SSEEdit.exe");

        let cmd = service.build_cleaning_command(&xedit, "Test.esp", None, None, false);
        assert!(cmd.contains("SSEEdit.exe"));
        assert!(cmd.contains("-QAC"));
        assert!(cmd.contains("-autoexit"));
        assert!(cmd.contains("Test.esp"));
    }

    #[test]
    fn test_build_cleaning_command_with_partial_forms() {
        let service = CleaningService::new();
        let xedit = Utf8PathBuf::from("C:/Games/SSEEdit.exe");

        let cmd = service.build_cleaning_command(&xedit, "Test.esp", None, None, true);
        assert!(cmd.contains("-iknowwhatimdoing"));
        assert!(cmd.contains("-allowmakepartial"));
    }

    #[test]
    fn test_build_cleaning_command_mo2_mode() {
        let service = CleaningService::new();
        let xedit = Utf8PathBuf::from("C:/Games/SSEEdit.exe");
        let mo2 = Utf8PathBuf::from("C:/MO2/ModOrganizer.exe");

        let cmd = service.build_cleaning_command(&xedit, "Test.esp", None, Some(&mo2), false);
        assert!(cmd.contains("ModOrganizer.exe"));
        assert!(cmd.contains("run"));
    }

    #[test]
    fn test_regex_patterns() {
        let service = CleaningService::new();

        assert!(service.udr_pattern.is_match("Undeleting: [00000D62] <Skyrim.esm>"));
        assert!(service.itm_pattern.is_match("Removing: [FormID] <Plugin.esp>"));
        assert!(service.nvm_pattern.is_match("Skipping: [NavMesh] <Plugin.esp>"));
        assert!(service.partial_form_pattern.is_match("Making Partial Form: [00000001]"));

        assert!(!service.udr_pattern.is_match("Removing: test"));
        assert!(!service.itm_pattern.is_match("Undeleting: test"));
    }
}
