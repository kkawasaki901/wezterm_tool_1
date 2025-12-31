use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub root_dir: PathBuf,
    pub soon_days: i64,
    pub editor: String,
    pub archive: bool,
    pub auto_archive: bool,
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            root_dir: home.join("todo"),
            soon_days: 7,
            editor: std::env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string()),
            archive: true,
            auto_archive: false,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut cfg = Config::default();
        if let Some(mut path) = dirs::config_dir() {
            path.push("todo");
            path.push("config.toml");
            if let Ok(s) = std::fs::read_to_string(&path) {
                if let Ok(user_cfg) = toml::from_str::<Config>(&s) {
                    cfg = user_cfg;
                }
            }
        }
        cfg
    }

    pub fn active_dir(&self) -> PathBuf { self.root_dir.join("active") }
    pub fn done_dir(&self) -> PathBuf { self.root_dir.join("done") }
    pub fn canceled_dir(&self) -> PathBuf { self.root_dir.join("canceled") }
    pub fn templates_dir(&self) -> PathBuf { self.root_dir.join("templates") }
    pub fn template_path(&self) -> PathBuf { self.templates_dir().join("todo.md") }
}
