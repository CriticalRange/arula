use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ai: AiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    pub api_url: String,
    pub api_key: String,
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn get_config_path() -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.arula/config.yaml", home)
    }

    pub fn load_or_default() -> Result<Self> {
        let config_path = Self::get_config_path();
        let config_file = Path::new(&config_path);

        // Try to load existing config
        if config_file.exists() {
            if let Ok(config) = Self::load_from_file(config_file) {
                return Ok(config);
            }
        }

        // Return default config if loading fails
        Ok(Self::default())
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path();
        self.save_to_file(config_path)
    }

    pub fn default() -> Self {
        Self {
            ai: AiConfig {
                provider: "openai".to_string(),
                model: "gpt-3.5-turbo".to_string(),
                api_url: "https://api.openai.com".to_string(),
                api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            },
        }
    }
}
