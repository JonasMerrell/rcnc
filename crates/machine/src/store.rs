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
    save_text(path, &text)
}

fn save_text(path: &Path, text: &str) -> Result<(), String> {
    let err = |e: std::io::Error| format!("{}: {e}", path.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    let tmp = path.with_extension("tmp");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp).map_err(err)?;
        f.write_all(text.as_bytes()).map_err(err)?;
        // On disk before the rename, so a power cut can't leave an empty file in its place.
        f.sync_all().map_err(err)?;
    }
    std::fs::rename(&tmp, path).map_err(err)
}

/// Writes files on a thread of its own, so a slow disk (antivirus, a synced
/// folder) never holds up the controller while the machine moves.
pub struct Saver {
    tx: std::sync::mpsc::Sender<(PathBuf, String)>,
    shared: std::sync::Arc<(std::sync::Mutex<(usize, Vec<String>)>, std::sync::Condvar)>,
}

impl Saver {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<(PathBuf, String)>();
        let shared = std::sync::Arc::new((std::sync::Mutex::new((0usize, Vec::new())), std::sync::Condvar::new()));
        let s2 = shared.clone();
        std::thread::Builder::new()
            .name("settings-saver".into())
            .spawn(move || {
                for (path, text) in rx {
                    let result = save_text(&path, &text);
                    let (lock, cv) = &*s2;
                    let mut g = lock.lock().unwrap_or_else(|p| p.into_inner());
                    g.0 -= 1;
                    if let Err(e) = result {
                        g.1.push(e);
                    }
                    cv.notify_all();
                }
            })
            .expect("spawn saver thread");
        Saver { tx, shared }
    }

    pub fn save<T: Serialize>(&self, path: &Path, value: &T) -> Result<(), String> {
        let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
        self.shared.0.lock().unwrap_or_else(|p| p.into_inner()).0 += 1;
        self.tx.send((path.to_path_buf(), text)).map_err(|_| "saver stopped".to_string())
    }

    /// Errors from writes since the last call.
    pub fn errors(&self) -> Vec<String> {
        std::mem::take(&mut self.shared.0.lock().unwrap_or_else(|p| p.into_inner()).1)
    }

    /// Wait until everything handed over is written.
    pub fn wait(&self) {
        let (lock, cv) = &*self.shared;
        let mut g = lock.lock().unwrap_or_else(|p| p.into_inner());
        while g.0 > 0 {
            g = cv.wait(g).unwrap_or_else(|p| p.into_inner());
        }
    }
}

impl Default for Saver {
    fn default() -> Self {
        Self::new()
    }
}

/// Move an unreadable file out of the way (to `<name>.bad-<seconds>`), so
/// saving the defaults doesn't overwrite what was in it. Says where it went.
pub fn set_aside(path: &Path) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let to = path.with_file_name(format!("{name}.bad-{secs}"));
    match std::fs::rename(path, &to) {
        Ok(()) => format!("the file was kept as {}", to.display()),
        Err(e) => format!("and it couldn't be moved aside ({e})"),
    }
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

    #[test]
    fn sets_aside_unreadable_files() {
        let dir = std::env::temp_dir().join(format!("rcnc-store-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert!(load::<SavedState>(&path).is_err());
        let note = set_aside(&path);
        assert!(note.contains("state.json.bad-"), "{note}");
        assert!(!path.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
