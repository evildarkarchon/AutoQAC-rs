use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Main configuration from AutoQAC Main.yaml
///
/// Contains game configurations, skip lists, and XEdit executable lists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainConfig {
    #[serde(rename = "PACT_Data")]
    pub pact_data: PactData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PactData {
    pub version: String,
    pub version_date: String,

    pub default_settings: String,
    pub default_ignorefile: String,

    #[serde(rename = "XEdit_Lists")]
    pub xedit_lists: IndexMap<String, Vec<String>>,

    #[serde(rename = "Skip_Lists")]
    pub skip_lists: IndexMap<String, Vec<String>>,

    #[serde(rename = "Errors")]
    pub errors: IndexMap<String, String>,

    #[serde(rename = "Warnings")]
    pub warnings: IndexMap<String, String>,
}

/// User configuration from AutoQAC Config.yaml or PACT Settings.yaml
///
/// Contains user-specific settings and file paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(rename = "PACT_Settings")]
    pub pact_settings: PactSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PactSettings {
    #[serde(rename = "Update Check", default)]
    pub update_check: bool,

    #[serde(rename = "Stat Logging", default)]
    pub stat_logging: bool,

    #[serde(rename = "Cleaning Timeout", default = "default_cleaning_timeout")]
    pub cleaning_timeout: u32,

    #[serde(rename = "Journal Expiration", default = "default_journal_expiration")]
    pub journal_expiration: u32,

    #[serde(rename = "LoadOrder TXT", default)]
    pub loadorder_txt: String,

    #[serde(rename = "XEDIT EXE", default)]
    pub xedit_exe: String,

    #[serde(rename = "MO2 EXE", default)]
    pub mo2_exe: String,

    #[serde(rename = "Partial Forms", default)]
    pub partial_forms: bool,

    #[serde(rename = "Debug Mode", default)]
    pub debug_mode: bool,
}

impl Default for PactSettings {
    fn default() -> Self {
        Self {
            update_check: true,
            stat_logging: true,
            cleaning_timeout: 300,
            journal_expiration: 7,
            loadorder_txt: String::new(),
            xedit_exe: String::new(),
            mo2_exe: String::new(),
            partial_forms: false,
            debug_mode: false,
        }
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            pact_settings: PactSettings::default(),
        }
    }
}

fn default_cleaning_timeout() -> u32 {
    300
}

fn default_journal_expiration() -> u32 {
    7
}

/// Additional ignore file structure for PACT Ignore.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreConfig {
    #[serde(rename = "PACT_Ignore_FO3", default)]
    pub fo3: Vec<String>,

    #[serde(rename = "PACT_Ignore_FNV", default)]
    pub fnv: Vec<String>,

    #[serde(rename = "PACT_Ignore_FO4", default)]
    pub fo4: Vec<String>,

    #[serde(rename = "PACT_Ignore_SSE", default)]
    pub sse: Vec<String>,
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        Self {
            fo3: vec!["Example Plugin.esp".to_string()],
            fnv: vec!["Example Plugin.esp".to_string()],
            fo4: vec!["Example Plugin.esp".to_string()],
            sse: vec!["Example Plugin.esp".to_string()],
        }
    }
}

impl MainConfig {
    /// Get skip list for a specific game type
    pub fn get_skip_list(&self, game_type: &str) -> Option<&Vec<String>> {
        self.pact_data.skip_lists.get(game_type)
    }

    /// Get XEdit executable list for a specific game type
    pub fn get_xedit_list(&self, game_type: &str) -> Option<&Vec<String>> {
        self.pact_data.xedit_lists.get(game_type)
    }

    /// Check if a plugin should be skipped for a given game
    pub fn should_skip_plugin(&self, game_type: &str, plugin: &str) -> bool {
        if let Some(skip_list) = self.get_skip_list(game_type) {
            skip_list.iter().any(|s| s.eq_ignore_ascii_case(plugin))
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pact_settings_defaults() {
        let settings = PactSettings::default();
        assert_eq!(settings.cleaning_timeout, 300);
        assert_eq!(settings.journal_expiration, 7);
        assert!(settings.update_check);
        assert!(!settings.partial_forms);
    }

    #[test]
    fn test_user_config_default() {
        let config = UserConfig::default();
        assert_eq!(config.pact_settings.cleaning_timeout, 300);
    }

    #[test]
    fn test_ignore_config_default() {
        let config = IgnoreConfig::default();
        assert_eq!(config.fo3.len(), 1);
        assert_eq!(config.fo3[0], "Example Plugin.esp");
    }
}
