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

/// The message log kept beside state.json, one timestamped line each (UTC), so what a probe found
/// can be read after it scrolls away. Over 2 MB it's moved to `log.old.txt` and started again.
pub const LOG_FILE: &str = "log.txt";

pub fn append_log(dir: &Path, msg: &str) {
    use std::io::Write;
    let path = dir.join(LOG_FILE);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 2_000_000) {
        let _ = std::fs::rename(&path, dir.join("log.old.txt"));
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(f, "{} {msg}", utc_now());
    }
}

/// `2026-11-15 19:54:51Z`
fn utc_now() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // civil from days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}Z", rem / 3600, rem / 60 % 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_file_gets_timestamped_lines() {
        let dir = std::env::temp_dir().join(format!("rcnc-log-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let _ = std::fs::remove_file(dir.join(LOG_FILE));
        append_log(&dir, "Bores found: here");
        append_log(&dir, "Alarm: there");
        let text = std::fs::read_to_string(dir.join(LOG_FILE)).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        // 2026-11-15 19:54:51Z Bores found: here
        assert!(lines[0].len() == 21 + "Bores found: here".len() && lines[0].ends_with("Z Bores found: here"), "{}", lines[0]);
        assert!(lines[0].starts_with("20") && &lines[0][4..5] == "-" && &lines[0][10..11] == " ", "{}", lines[0]);
    }

    #[test]
    fn round_trips_state() {
        let dir = std::env::temp_dir().join(format!("rcnc-store-{}", std::process::id()));
        let path = dir.join("state.json");
        let mut s = SavedState::default();
        s.tools.insert(3, Tool { length: 41.5, diameter: 6.0, description: "6 mm flat".into(), ..Default::default() });
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
