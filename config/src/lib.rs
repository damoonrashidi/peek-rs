use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PeekConfig {
    pub workspaces: Vec<Workspace>,
    pub ai: AIConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AIConfig {
    pub model: String,
    pub url: String,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            model: "qwen3:8b".to_string(),
            url: "http://localhost:11434".to_string(),
        }
    }
}

impl PeekConfig {
    pub fn get_or_default() -> Self {
        let Ok(home_dir) = std::env::var("HOME") else {
            return PeekConfig::default();
        };

        let Ok(config_file) =
            std::fs::read_to_string(format!("{home_dir}/.config/peek/config.toml"))
        else {
            return PeekConfig::default();
        };
        toml::from_str(&config_file).unwrap_or(PeekConfig::default())
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Workspace {
    pub name: String,
    pub connections: Vec<DatabaseConnection>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DatabaseConnection {
    pub name: String,
    pub color: String,
    pub url: String,
    pub ssh: Option<SSHConfig>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SSHConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub ssh_key: Option<String>,
}
