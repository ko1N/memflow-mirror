use ::std::{fs, path::PathBuf};

use ::serde::{Deserialize, Serialize};

fn default_as_false() -> bool {
    false
}

fn default_as_true() -> bool {
    false
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MirrorConfig {
    #[serde(default = "default_as_true")]
    pub multithreading: bool,

    #[serde(default = "default_as_true")]
    pub obs_capture: bool,

    #[serde(default = "default_as_false")]
    pub connect_on_startup: bool,
    pub last_connector: Option<String>,
    pub last_connector_args: Option<String>,
    pub last_os: Option<String>,
    pub last_os_args: Option<String>,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            multithreading: true,

            obs_capture: true,

            connect_on_startup: false,
            last_connector: None,
            last_connector_args: None,
            last_os: None,
            last_os_args: None,
        }
    }
}

impl MirrorConfig {
    pub fn load_or_default() -> Self {
        match Self::load() {
            Ok(s) => s,
            Err(_) => {
                let s = Self::default();
                s.save().ok();
                s
            }
        }
    }

    pub fn load() -> Result<Self, &'static str> {
        let path = Self::config_path();
        if fs::metadata(&path).is_err() {
            return Err("config file not found");
        }

        let contents =
            fs::read_to_string(&path).map_err(|_| "unable to read config file contents")?;
        toml::from_str(&contents).map_err(|_| "unable to parse config file contents as toml")
    }

    pub fn save(&self) -> Result<(), &'static str> {
        let path = Self::config_path();
        if let Some(p) = path.parent() {
            fs::create_dir_all(p).map_err(|_| "unable to create path to store config file")?;
        }

        let contents = toml::to_string(self).map_err(|_| "unable to serialize config to toml")?;
        fs::write(&path, contents.as_bytes()).map_err(|_| "unable to write config file")
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .map(|dir| dir.join("mirror/config.toml"))
            .unwrap_or_else(|| "./config.toml".into())
    }
}
