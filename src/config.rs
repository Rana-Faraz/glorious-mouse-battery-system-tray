use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

const DEFAULT_REFRESH_INTERVAL_SECONDS: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub selected_device_key: Option<String>,
    pub refresh_interval_seconds: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            selected_device_key: None,
            refresh_interval_seconds: DEFAULT_REFRESH_INTERVAL_SECONDS,
        }
    }
}

impl AppConfig {
    pub fn normalized(mut self) -> Self {
        if self.refresh_interval_seconds == 0 {
            self.refresh_interval_seconds = DEFAULT_REFRESH_INTERVAL_SECONDS;
        }
        self
    }
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn new() -> io::Result<Self> {
        let path = app_data_dir()?.join("config.toml");
        Ok(Self { path })
    }

    #[cfg(test)]
    fn from_path(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn load(&self) -> AppConfig {
        let Ok(contents) = fs::read_to_string(&self.path) else {
            return AppConfig::default();
        };

        match toml::from_str::<AppConfig>(&contents) {
            Ok(config) => config.normalized(),
            Err(error) => {
                tracing::warn!(%error, path = %self.path.display(), "invalid config; using defaults");
                AppConfig::default()
            }
        }
    }

    pub fn save(&self, config: &AppConfig) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(&config.clone().normalized())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        fs::write(&self.path, contents)
    }
}

pub fn app_data_dir() -> io::Result<PathBuf> {
    ProjectDirs::from("", "", "ModelD2ProBattery")
        .map(|dirs| dirs.data_local_dir().to_path_buf())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "local app data directory not found",
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_store(name: &str) -> ConfigStore {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("model-d2-config-{name}-{unique}"));
        ConfigStore::from_path(dir.join("config.toml"))
    }

    #[test]
    fn missing_config_returns_defaults() {
        let store = temp_config_store("missing");
        assert_eq!(store.load(), AppConfig::default());
    }

    #[test]
    fn invalid_config_returns_defaults() {
        let store = temp_config_store("invalid");
        fs::create_dir_all(store.path().parent().unwrap()).unwrap();
        fs::write(store.path(), "this is not toml = = =").unwrap();

        assert_eq!(store.load(), AppConfig::default());
    }

    #[test]
    fn config_round_trips() {
        let store = temp_config_store("roundtrip");
        let config = AppConfig {
            selected_device_key: Some("device-key".to_string()),
            refresh_interval_seconds: 15,
        };

        store.save(&config).unwrap();
        assert_eq!(store.load(), config);
    }
}
