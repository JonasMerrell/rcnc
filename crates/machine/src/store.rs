//! Settings and machine state kept between runs: machine config, tool table
//! and persistent parameters (work offsets, G28/G30 positions, G92).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use gcode::Tool;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// State the interpreter changes while running and Mach3 would remember.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedState {
    pub tools: BTreeMap<u32, Tool>,
    /// Persistent parameters by number (see `gcode::params::num::PERSISTENT`).
    pub params: BTreeMap<u32, f64>,
}

/// Where the app keeps its files: `%APPDATA%\rcnc` on Windows,
/// `$XDG_CONFIG_HOME/rcnc` or `~/.config/rcnc` elsewhere.
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("rcnc")
}

pub fn load<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Write via a temporary file so a crash never leaves a half-written file.
pub fn save<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let err = |e: std::io::Error| format!("{}: {e}", path.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).map_err(err)?;
    std::fs::rename(&tmp, path).map_err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_state() {
        let dir = std::env::temp_dir().join(format!("rcnc-store-{}", std::process::id()));
        let path = dir.join("state.json");
        let mut s = SavedState::default();
        s.tools.insert(3, Tool { length: 41.5, diameter: 6.0, description: "6 mm flat".into() });
        s.params.insert(5221, -120.25);
        save(&path, &s).unwrap();
        assert_eq!(load::<SavedState>(&path).unwrap(), Some(s));
        assert_eq!(load::<SavedState>(&dir.join("missing.json")).unwrap(), None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
