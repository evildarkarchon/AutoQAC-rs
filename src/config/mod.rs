use crate::models::{IgnoreConfig, MainConfig, UserConfig};
use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;

/// Configuration manager for loading and saving YAML configuration files.
///
/// Manages two primary configuration files:
/// - Main config (`AutoQAC Main.yaml`): Game configurations, skip lists
/// - User config (`AutoQAC Config.yaml` or `PACT Settings.yaml`): User settings, paths
#[derive(Debug, Clone)]
pub struct ConfigManager {
    config_dir: Utf8PathBuf,
    main_config_path: Utf8PathBuf,
    user_config_path: Utf8PathBuf,
    ignore_config_path: Utf8PathBuf,
}

impl ConfigManager {
    /// Create a new ConfigManager with the specified configuration directory.
    ///
    /// # Arguments
    /// * `config_dir` - Directory containing configuration files (e.g., "AutoQAC Data")
    ///
    /// # Returns
    /// A new ConfigManager instance
    pub fn new<P: AsRef<Utf8Path>>(config_dir: P) -> Result<Self> {
        let config_dir = config_dir.as_ref().to_path_buf();

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .with_context(|| format!("Failed to create config directory: {}", config_dir))?;
        }

        Ok(Self {
            main_config_path: config_dir.join("AutoQAC Main.yaml"),
            user_config_path: config_dir.join("AutoQAC Config.yaml"),
            ignore_config_path: config_dir.join("PACT Ignore.yaml"),
            config_dir,
        })
    }

    /// Load the main configuration file.
    ///
    /// # Returns
    /// The loaded MainConfig, or default if file doesn't exist
    pub fn load_main_config(&self) -> Result<MainConfig> {
        if !self.main_config_path.exists() {
            tracing::warn!(
                "Main config file not found at {}, using defaults",
                self.main_config_path
            );
            return Ok(self.create_default_main_config()?);
        }

        let file_contents = fs::read_to_string(&self.main_config_path)
            .with_context(|| format!("Failed to read main config: {}", self.main_config_path))?;

        let config: MainConfig = serde_yaml_ng::from_str(&file_contents)
            .with_context(|| format!("Failed to parse main config: {}", self.main_config_path))?;

        tracing::info!("Loaded main config from {}", self.main_config_path);
        Ok(config)
    }

    /// Save the main configuration file.
    ///
    /// # Arguments
    /// * `config` - The MainConfig to save
    pub fn save_main_config(&self, config: &MainConfig) -> Result<()> {
        let yaml_string =
            serde_yaml_ng::to_string(config).context("Failed to serialize main config to YAML")?;

        fs::write(&self.main_config_path, yaml_string)
            .with_context(|| format!("Failed to write main config: {}", self.main_config_path))?;

        tracing::info!("Saved main config to {}", self.main_config_path);
        Ok(())
    }

    /// Load the user configuration file.
    ///
    /// # Returns
    /// The loaded UserConfig, or default if file doesn't exist
    pub fn load_user_config(&self) -> Result<UserConfig> {
        // Try AutoQAC Config.yaml first, fall back to PACT Settings.yaml
        let legacy_path = self.config_dir.join("PACT Settings.yaml");

        let config_path = if self.user_config_path.exists() {
            &self.user_config_path
        } else if legacy_path.exists() {
            tracing::info!("Using legacy config file: {}", legacy_path);
            &legacy_path
        } else {
            tracing::warn!(
                "User config file not found at {} or {}, using defaults",
                self.user_config_path,
                legacy_path
            );
            return Ok(UserConfig::default());
        };

        let file_contents = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read user config: {}", config_path))?;

        let config: UserConfig = serde_yaml_ng::from_str(&file_contents)
            .with_context(|| format!("Failed to parse user config: {}", config_path))?;

        tracing::info!("Loaded user config from {}", config_path);
        Ok(config)
    }

    /// Save the user configuration file.
    ///
    /// # Arguments
    /// * `config` - The UserConfig to save
    pub fn save_user_config(&self, config: &UserConfig) -> Result<()> {
        let yaml_string =
            serde_yaml_ng::to_string(config).context("Failed to serialize user config to YAML")?;

        fs::write(&self.user_config_path, yaml_string)
            .with_context(|| format!("Failed to write user config: {}", self.user_config_path))?;

        tracing::info!("Saved user config to {}", self.user_config_path);
        Ok(())
    }

    /// Load the ignore configuration file.
    ///
    /// # Returns
    /// The loaded IgnoreConfig, or default if file doesn't exist
    pub fn load_ignore_config(&self) -> Result<IgnoreConfig> {
        if !self.ignore_config_path.exists() {
            tracing::warn!(
                "Ignore config file not found at {}, using defaults",
                self.ignore_config_path
            );
            return Ok(IgnoreConfig::default());
        }

        let file_contents = fs::read_to_string(&self.ignore_config_path).with_context(|| {
            format!("Failed to read ignore config: {}", self.ignore_config_path)
        })?;

        let config: IgnoreConfig = serde_yaml_ng::from_str(&file_contents).with_context(|| {
            format!("Failed to parse ignore config: {}", self.ignore_config_path)
        })?;

        tracing::info!("Loaded ignore config from {}", self.ignore_config_path);
        Ok(config)
    }

    /// Save the ignore configuration file.
    ///
    /// # Arguments
    /// * `config` - The IgnoreConfig to save
    pub fn save_ignore_config(&self, config: &IgnoreConfig) -> Result<()> {
        let yaml_string = serde_yaml_ng::to_string(config)
            .context("Failed to serialize ignore config to YAML")?;

        fs::write(&self.ignore_config_path, yaml_string).with_context(|| {
            format!("Failed to write ignore config: {}", self.ignore_config_path)
        })?;

        tracing::info!("Saved ignore config to {}", self.ignore_config_path);
        Ok(())
    }

    /// Create a default main configuration with full skip lists from the existing config.
    ///
    /// This is used when the main config file doesn't exist.
    fn create_default_main_config(&self) -> Result<MainConfig> {
        use crate::models::PactData;
        use indexmap::IndexMap;

        let mut xedit_lists = IndexMap::new();
        xedit_lists.insert(
            "FO3".to_string(),
            vec!["FO3Edit.exe".to_string(), "FO3Edit64.exe".to_string()],
        );
        xedit_lists.insert(
            "FNV".to_string(),
            vec!["FNVEdit.exe".to_string(), "FNVEdit64.exe".to_string()],
        );
        xedit_lists.insert(
            "FO4".to_string(),
            vec!["FO4Edit.exe".to_string(), "FO4Edit64.exe".to_string()],
        );
        xedit_lists.insert(
            "SSE".to_string(),
            vec!["SSEEdit.exe".to_string(), "SSEEdit64.exe".to_string()],
        );
        xedit_lists.insert(
            "FO4VR".to_string(),
            vec!["FO4VREdit.exe".to_string(), "FO4VREdit64.exe".to_string()],
        );
        xedit_lists.insert("SkyrimVR".to_string(), vec!["TES5VREdit.exe".to_string()]);
        xedit_lists.insert(
            "Universal".to_string(),
            vec![
                "xEdit.exe".to_string(),
                "xEdit64.exe".to_string(),
                "xfoedit.exe".to_string(),
                "xfoedit64.exe".to_string(),
            ],
        );

        let mut skip_lists = IndexMap::new();

        // FO3 skip list
        skip_lists.insert(
            "FO3".to_string(),
            vec![
                "".to_string(),
                "Fallout3.esm".to_string(),
                "Anchorage.esm".to_string(),
                "ThePitt.esm".to_string(),
                "BrokenSteel.esm".to_string(),
                "PointLookout.esm".to_string(),
                "Zeta.esm".to_string(),
                "Unofficial Fallout 3 Patch.esm".to_string(),
            ],
        );

        // FNV skip list
        skip_lists.insert(
            "FNV".to_string(),
            vec![
                "".to_string(),
                "FalloutNV.esm".to_string(),
                "DeadMoney.esm".to_string(),
                "OldWorldBlues.esm".to_string(),
                "HonestHearts.esm".to_string(),
                "LonesomeRoad.esm".to_string(),
                "TribalPack.esm".to_string(),
                "MercenaryPack.esm".to_string(),
                "ClassicPack.esm".to_string(),
                "CaravanPack.esm".to_string(),
                "GunRunnersArsenal.esm".to_string(),
                "Unofficial Patch NVSE Plus.esp".to_string(),
                "TaleOfTwoWastelands.esm".to_string(),
                "TTWInteriors_Core.esm".to_string(),
                "TTWInteriorsProject_Combo.esm".to_string(),
                "TTWInteriorsProject_ComboHotfix.esm".to_string(),
                "TTWInteriorsProject_Merged.esm".to_string(),
                "TTWInteriors_Core_Hotfix.esm".to_string(),
            ],
        );

        // FO4 skip list
        skip_lists.insert(
            "FO4".to_string(),
            vec![
                "".to_string(),
                "Fallout4.esm".to_string(),
                "DLCCoast.esm".to_string(),
                "DLCNukaWorld.esm".to_string(),
                "DLCRobot.esm".to_string(),
                "DLCworkshop01.esm".to_string(),
                "DLCworkshop02.esm".to_string(),
                "DLCworkshop03.esm".to_string(),
                "Unofficial Fallout 4 Patch.esp".to_string(),
                "PPF.esm".to_string(),
                "PRP.esp".to_string(),
                "PRP-Compat".to_string(),
                "SS2.esm".to_string(),
                "SS2_XPAC_Chapter2.esm".to_string(),
                "SS2_XPAC_Chapter3.esm".to_string(),
                "SS2Extended.esp".to_string(),
            ],
        );

        // SSE skip list
        skip_lists.insert(
            "SSE".to_string(),
            vec![
                "".to_string(),
                "Skyrim.esm".to_string(),
                "Update.esm".to_string(),
                "HearthFires.esm".to_string(),
                "Dragonborn.esm".to_string(),
                "Dawnguard.esm".to_string(),
                "Unofficial Skyrim Special Edition Patch.esp".to_string(),
                "_ResourcePack.esl".to_string(),
            ],
        );

        let pact_data = PactData {
            version: "3.0.0".to_string(),
            version_date: "25.01.14".to_string(),
            default_settings: String::new(),
            default_ignorefile: String::new(),
            xedit_lists,
            skip_lists,
            errors: IndexMap::new(),
            warnings: IndexMap::new(),
        };

        Ok(MainConfig { pact_data })
    }

    /// Get the configuration directory path.
    pub fn config_dir(&self) -> &Utf8Path {
        &self.config_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config_manager() -> (ConfigManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config_path = Utf8PathBuf::try_from(temp_dir.path().to_path_buf()).unwrap();
        let manager = ConfigManager::new(&config_path).unwrap();
        (manager, temp_dir)
    }

    #[test]
    fn test_create_config_manager() {
        let (_manager, _temp_dir) = create_test_config_manager();
    }

    #[test]
    fn test_load_save_user_config() {
        let (manager, _temp_dir) = create_test_config_manager();

        let config = UserConfig::default();
        manager.save_user_config(&config).unwrap();

        let loaded = manager.load_user_config().unwrap();
        assert_eq!(loaded.pact_settings.cleaning_timeout, 300);
    }

    #[test]
    fn test_load_save_ignore_config() {
        let (manager, _temp_dir) = create_test_config_manager();

        let config = IgnoreConfig::default();
        manager.save_ignore_config(&config).unwrap();

        let loaded = manager.load_ignore_config().unwrap();
        assert_eq!(loaded.fo3.len(), 1);
    }

    #[test]
    fn test_default_main_config() {
        let (manager, _temp_dir) = create_test_config_manager();
        let config = manager.create_default_main_config().unwrap();

        assert!(config.pact_data.xedit_lists.contains_key("FO4"));
        assert!(config.pact_data.skip_lists.contains_key("SSE"));

        // Verify FO4 skip list contains expected entries
        let fo4_skip = config.pact_data.skip_lists.get("FO4").unwrap();
        assert!(fo4_skip.contains(&"Fallout4.esm".to_string()));
        assert!(fo4_skip.contains(&"DLCCoast.esm".to_string()));
    }
}
