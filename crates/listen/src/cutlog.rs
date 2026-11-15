//! What was heard, line by line, for rCAD to calibrate from.

use crate::Frame;
use std::collections::BTreeMap;

/// Everything heard while a program ran, by program line.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CutLog {
    pub program: String,
    /// The input it was heard on.
    pub input: String,
    pub lines: BTreeMap<usize, Line>,
}

/// One program line: what it asked for and what was heard while it ran.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Line {
    pub op: String,
    pub tool: u32,
    pub rpm: f64,
    pub feed: f64,
    pub cutting: bool,
    /// Windows heard, and of them: the spindle found, chatter heard.
    pub frames: u32,
    pub heard: u32,
    pub chatter: u32,
    /// Mean and worst speed droop (share), the chatter frequency heard most, loudness (dB, mean and loudest).
    pub droop_mean: f64,
    pub droop_max: f64,
    pub chatter_hz: f64,
    pub loud_mean_db: f64,
    pub loud_max_db: f64,
}

impl CutLog {
    pub fn new(program: &str, input: &str) -> CutLog {
        CutLog { program: program.into(), input: input.into(), lines: BTreeMap::new() }
    }

    pub fn add(&mut self, f: &Frame) {
        if f.ctx.line == 0 {
            return;
        }
        let l = self.lines.entry(f.ctx.line).or_insert_with(|| Line {
            op: f.ctx.op.clone(),
            tool: f.ctx.tool,
            rpm: f.ctx.rpm,
            feed: f.ctx.feed,
            cutting: f.ctx.cutting,
            loud_max_db: f64::NEG_INFINITY,
            ..Default::default()
        });
        l.frames += 1;
        // the line's own feed and speed once it is running (the first window can catch it starting)
        l.cutting |= f.ctx.cutting;
        l.feed = l.feed.max(f.ctx.feed);
        l.rpm = l.rpm.max(f.ctx.rpm);
        l.loud_mean_db += (f.loud_db - l.loud_mean_db) / l.frames as f64;
        l.loud_max_db = l.loud_max_db.max(f.loud_db);
        if let Some(d) = f.droop {
            l.heard += 1;
            l.droop_mean += (d - l.droop_mean) / l.heard as f64;
            l.droop_max = if l.heard == 1 { d } else { l.droop_max.max(d) };
        }
        if let Some((hz, _)) = f.chatter {
            l.chatter += 1;
            // the latest; good enough to tell which mode it was
            l.chatter_hz = hz;
        }
    }

    /// `<program>.cutlog.json` beside the program.
    pub fn path_for(program: &std::path::Path) -> std::path::PathBuf {
        let mut p = program.as_os_str().to_owned();
        p.push(".cutlog.json");
        p.into()
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let s = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn load(path: &std::path::Path) -> Result<CutLog, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&s).map_err(|e| format!("{}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Analyzer, Context};

    #[test]
    fn logs_by_line_and_reads_back() {
        let mut a = Analyzer::new(48_000);
        let mut log = CutLog::new("op2-top.nc", "test");
        let ctx = |line| Context { line, op: "3D rough".into(), tool: 24, teeth: 3, rpm: 24_000.0, feed: 2400.0, cutting: true };
        for f in a.push(&crate::analysis::tests::cut(24_000.0 * 0.98, 3, None, 0.6, 48_000), &ctx(40)) {
            log.add(&f);
        }
        for f in a.push(&crate::analysis::tests::cut(24_000.0, 3, Some(3730.0), 0.6, 48_000), &ctx(41)) {
            log.add(&f);
        }
        let (l40, l41) = (&log.lines[&40], &log.lines[&41]);
        assert!((l40.droop_mean - 0.02).abs() < 0.005 && l40.chatter == 0, "{l40:?}");
        assert!(l41.chatter > 0 && (l41.chatter_hz - 3730.0).abs() < 15.0, "{l41:?}");
        let dir = std::env::temp_dir().join("rcnc-cutlog-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = CutLog::path_for(&dir.join("op2-top.nc"));
        assert!(p.to_string_lossy().ends_with("op2-top.nc.cutlog.json"));
        log.save(&p).unwrap();
        // (serde_json doesn't read back every float to the last bit)
        assert_eq!(serde_json::to_string(&CutLog::load(&p).unwrap()).unwrap().len(), serde_json::to_string(&log).unwrap().len());
        assert_eq!(CutLog::load(&p).unwrap().lines[&41].chatter, log.lines[&41].chatter);
    }

    #[test]
    fn a_wav_plays_back_the_same() {
        let dir = std::env::temp_dir().join("rcnc-wav-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("cut.wav");
        let x = crate::analysis::tests::cut(18_000.0, 2, None, 0.5, 48_000);
        crate::write_wav(&p, &x, 48_000).unwrap();
        let (y, rate) = crate::read_wav(&p).unwrap();
        assert_eq!((rate, y.len()), (48_000, x.len()));
        let ctx = Context { rpm: 18_000.0, teeth: 2, cutting: true, ..Default::default() };
        let f = &Analyzer::new(rate).push(&y, &ctx)[0];
        assert!(f.droop.unwrap().abs() < 0.004, "{f:?}");
    }
}
