//! What the app remembers between runs, as TOML in the platform's config folder.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::hotkey;
use crate::market::Mode;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Config {
    pub mode: Mode,
    /// The shortcut that switches between selling and buying, in `global-hotkey`'s
    /// notation (`control+KeyM`); empty for none.
    pub hotkey: String,
    /// Overrides the export folder found on its own.
    pub logs_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mode: Mode::Sell,
            hotkey: hotkey::DEFAULT.to_string(),
            logs_dir: None,
        }
    }
}

fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("marketeer").join("config.toml"))
}

impl Config {
    /// The saved settings, or the defaults. A file that does not parse is moved aside
    /// rather than overwritten, and the second value says so.
    pub(crate) fn load() -> (Config, Option<String>) {
        let Some(path) = path() else {
            return (Config::default(), None);
        };
        let Ok(text) = std::fs::read_to_string(&path) else {
            return (Config::default(), None);
        };
        match toml::from_str::<Config>(&text) {
            Ok(config) => (config, None),
            Err(err) => {
                let aside = path.with_extension("toml.broken");
                let _ = std::fs::rename(&path, &aside);
                let problem = format!(
                    "The settings file did not parse and was moved to {}: {}",
                    aside.display(),
                    err.message()
                );
                (Config::default(), Some(problem))
            }
        }
    }

    /// Write the file in one step, so a crash halfway leaves the old one.
    pub(crate) fn save(&self) -> Result<(), String> {
        let path = path().ok_or("no config folder on this system")?;
        let text = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let temp = path.with_extension("toml.tmp");
        std::fs::write(&temp, text).map_err(|e| e.to_string())?;
        std::fs::rename(&temp, &path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_keys_get_their_defaults() {
        let config: Config = toml::from_str("mode = \"buy\"").unwrap();
        assert_eq!(config.mode, Mode::Buy);
        assert_eq!(config.hotkey, hotkey::DEFAULT);
        assert_eq!(config.logs_dir, None);
    }

    #[test]
    fn round_trips() {
        let config = Config {
            mode: Mode::Buy,
            hotkey: String::new(),
            logs_dir: Some(PathBuf::from("/tmp/Marketlogs")),
        };
        let text = toml::to_string_pretty(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
    }
}
