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
    /// Every window in order, with where the tool was: passes can be told apart by position and time even
    /// when the line is wrong (a board that doesn't report it), and a run that ends early keeps its last seconds.
    #[serde(default)]
    pub timeline: Vec<Moment>,
    /// Per operation that chattered: the spindle speeds that should calm it (`suggest`), for rCAD to plan the
    /// next run with; rcnc never changes the speed itself.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<RpmSuggestion>,
}

/// An operation that chattered, and the speed to try instead.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RpmSuggestion {
    pub op: String,
    pub tool: u32,
    pub teeth: u32,
    /// The speed it ran at, and the chatter heard: its frequency (the one heard most) and in how many of the
    /// operation's cutting windows.
    pub rpm: f64,
    pub chatter_hz: f64,
    pub chatter_windows: u32,
    pub windows: u32,
    /// The speed to try: the teeth then pass at the chatter frequency over a whole number (`lobe`), the top of a
    /// stability lobe; and the feed scaled with it to keep the chip.
    pub suggest_rpm: f64,
    pub lobe: u32,
    pub feed_scale: f64,
}

/// The spindle speed nearest `rpm`, at most `max_rpm` and at least 60% of `rpm`, at which `teeth` teeth pass at
/// the chatter frequency over a whole number: the tops of the stability lobes, where a cut is stablest. None if
/// there is none in range. The frequency moves a little with the speed, so it's a speed to try, not a promise.
pub fn lobe_rpm(chatter_hz: f64, rpm: f64, teeth: u32, max_rpm: f64) -> Option<(f64, u32)> {
    let z = teeth.max(1) as f64;
    (1..=60u32)
        .map(|n| (60.0 * chatter_hz / (z * n as f64), n))
        .filter(|(r, _)| *r <= max_rpm + 1e-9 && *r >= 0.6 * rpm)
        .min_by(|a, b| (a.0 - rpm).abs().total_cmp(&(b.0 - rpm).abs()))
}

/// One analysed window (about 0.09 s apart).
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Moment {
    /// Seconds since the log started.
    pub t: f64,
    pub line: usize,
    pub op: String,
    pub cutting: bool,
    /// Work X, Y, Z and the real speed along the path, mm/min.
    pub pos: [f64; 3],
    pub velocity: f64,
    /// Programmed and heard spindle speed, droop under the programmed speed (share).
    pub rpm: f64,
    pub heard_rpm: Option<f64>,
    pub droop: Option<f64>,
    pub loud_db: f64,
    pub tooth_db: Option<f64>,
    /// Chatter heard: frequency, dB over the background.
    pub chatter: Option<(f64, f64)>,
    /// The adaptive feed asked for after this window, share of the programmed feed (None: not in use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adaptive_feed: Option<f64>,
    /// The sag the bog guard judges: under the no-load speed it learned at this speed (`droop` is under the
    /// programmed speed, so it also carries the VFD's own no-load slip, about 1.2% at 24k on the DMC2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub over_no_load: Option<f64>,
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
    /// The teeth's beat (tooth-passing frequency), dB: mean and highest over the windows it was found in.
    #[serde(default)]
    pub tooth_mean_db: f64,
    #[serde(default)]
    pub tooth_max_db: f64,
    #[serde(default)]
    pub tooth_frames: u32,
    #[serde(default)]
    pub teeth: u32,
}

impl CutLog {
    pub fn new(program: &str, input: &str) -> CutLog {
        CutLog { program: program.into(), input: input.into(), lines: BTreeMap::new(), timeline: vec![], suggestions: vec![] }
    }

    /// Add a window heard `t` seconds into the run.
    pub fn add_at(&mut self, t: f64, f: &Frame) {
        self.timeline.push(Moment {
            t,
            line: f.ctx.line,
            op: f.ctx.op.clone(),
            cutting: f.ctx.cutting,
            pos: f.ctx.pos,
            velocity: f.ctx.velocity,
            rpm: f.ctx.rpm,
            heard_rpm: f.rpm,
            droop: f.droop,
            loud_db: f.loud_db,
            tooth_db: f.tooth_db,
            chatter: f.chatter,
            adaptive_feed: None,
            over_no_load: None,
        });
        self.add(f);
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
            teeth: f.ctx.teeth,
            loud_max_db: f64::NEG_INFINITY,
            ..Default::default()
        });
        l.frames += 1;
        // the line's own feed and speed once it is running (the first window can catch it starting)
        l.cutting |= f.ctx.cutting;
        // the real speed: the programmed F the interpreter last read can be the next line's (it reads ahead)
        l.feed = l.feed.max(if f.ctx.velocity > 0.0 { f.ctx.velocity } else { f.ctx.feed });
        l.rpm = l.rpm.max(f.ctx.rpm);
        l.loud_mean_db += (f.loud_db - l.loud_mean_db) / l.frames as f64;
        l.loud_max_db = l.loud_max_db.max(f.loud_db);
        if let Some(t) = f.tooth_db {
            l.tooth_frames += 1;
            l.tooth_mean_db += (t - l.tooth_mean_db) / l.tooth_frames as f64;
            l.tooth_max_db = if l.tooth_frames == 1 { t } else { l.tooth_max_db.max(t) };
        }
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

    /// Work out `suggestions` (call before saving): each operation that chattered in at least a fifth of its
    /// cutting windows (and at least 4), the chatter frequency heard most (to 25 Hz), and the speed to try.
    pub fn suggest(&mut self, max_rpm: f64) {
        let mut by_op: Vec<(String, u32, u32, f64, u32, u32, Vec<f64>)> = vec![];
        for l in self.lines.values().filter(|l| l.cutting && l.rpm > 0.0) {
            let i = match by_op.iter().position(|o| o.0 == l.op && o.3 == l.rpm) {
                Some(i) => i,
                None => {
                    by_op.push((l.op.clone(), l.tool, l.teeth, l.rpm, 0, 0, vec![]));
                    by_op.len() - 1
                }
            };
            by_op[i].4 += l.chatter;
            by_op[i].5 += l.frames;
        }
        // the frequencies, from the timeline (each window's)
        for m in self.timeline.iter().filter(|m| m.cutting) {
            if let (Some((hz, _)), Some(o)) = (m.chatter, by_op.iter_mut().find(|o| o.0 == m.op && o.3 == m.rpm)) {
                o.6.push(hz);
            }
        }
        self.suggestions = by_op
            .into_iter()
            .filter(|o| o.4 >= 4 && o.4 as f64 >= 0.2 * o.5 as f64)
            .filter_map(|(op, tool, teeth, rpm, chatter, windows, hzs)| {
                // the most heard, in 25 Hz bins (the line's last one if the timeline has none)
                let hz = if hzs.is_empty() {
                    self.lines.values().find(|l| l.op == op && l.chatter > 0).map(|l| l.chatter_hz)?
                } else {
                    let mut bins: Vec<(i64, Vec<f64>)> = vec![];
                    for h in hzs {
                        let k = (h / 25.0).round() as i64;
                        match bins.iter_mut().find(|b| b.0 == k) {
                            Some(b) => b.1.push(h),
                            None => bins.push((k, vec![h])),
                        }
                    }
                    let best = bins.into_iter().max_by_key(|b| b.1.len())?;
                    best.1.iter().sum::<f64>() / best.1.len() as f64
                };
                let (suggest_rpm, lobe) = lobe_rpm(hz, rpm, teeth, max_rpm)?;
                Some(RpmSuggestion { op, tool, teeth, rpm, chatter_hz: hz, chatter_windows: chatter, windows, suggest_rpm: suggest_rpm.round(), lobe, feed_scale: suggest_rpm / rpm })
            })
            .collect();
    }

    /// `<program>.cutlog.json` beside the program.
    pub fn path_for(program: &std::path::Path) -> std::path::PathBuf {
        Self::beside(program, ".cutlog.json")
    }

    /// Where a run's log goes: `<program>.air.cutlog.json` for an air cut (so it doesn't replace the real
    /// run's), else `<program>.cutlog.json`.
    pub fn path_for_run(program: &std::path::Path, air: bool) -> std::path::PathBuf {
        Self::beside(program, if air { ".air.cutlog.json" } else { ".cutlog.json" })
    }

    /// The recording of a run (`(RCNC RECORD_AUDIO)` in the program): `<program>.wav` / `<program>.air.wav`.
    pub fn wav_for_run(program: &std::path::Path, air: bool) -> std::path::PathBuf {
        Self::beside(program, if air { ".air.wav" } else { ".wav" })
    }

    fn beside(program: &std::path::Path, ext: &str) -> std::path::PathBuf {
        let mut p = program.as_os_str().to_owned();
        p.push(ext);
        p.into()
    }

    /// Write the log to `path`, through a temporary file beside it, so a crash mid-write leaves the last
    /// complete one.
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let s = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".part");
        let tmp = std::path::PathBuf::from(tmp);
        std::fs::write(&tmp, s).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
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
    fn the_lobe_speed_puts_the_teeth_on_the_chatter_over_a_whole_number() {
        // 3 teeth at 24k chattering at 7500 Hz: 7500 / 1200 = 6.25 teeth passes a wave; 6 or 7 a wave
        // would be 25000 or 21429 rpm: under the 24000 top, 21429 (lobe 7)
        let (r, n) = lobe_rpm(7500.0, 24_000.0, 3, 24_000.0).unwrap();
        assert_eq!(n, 7);
        assert!((r - 21428.6).abs() < 0.1, "{r}");
        // nothing in range
        assert!(lobe_rpm(100.0, 24_000.0, 3, 24_000.0).is_none());
    }

    #[test]
    fn a_chattering_operation_gets_a_speed_to_try() {
        let mut log = CutLog::new("t.nc", "test");
        let f = |line, op: &str, chatter: Option<(f64, f64)>| Frame {
            ctx: Context { line, op: op.into(), tool: 24, teeth: 3, rpm: 24_000.0, feed: 2400.0, cutting: true, ..Default::default() },
            droop: Some(0.01),
            chatter,
            ..Default::default()
        };
        for k in 0..20 {
            log.add_at(k as f64 * 0.085, &f(5, "Rough walls", (k % 2 == 0).then_some((7490.0 + k as f64, 25.0))));
            log.add_at(2.0 + k as f64 * 0.085, &f(9, "Finish", None));
        }
        log.suggest(24_000.0);
        assert_eq!(log.suggestions.len(), 1, "{:?}", log.suggestions);
        let s = &log.suggestions[0];
        assert_eq!((s.op.as_str(), s.chatter_windows, s.windows, s.lobe), ("Rough walls", 10, 20, 7));
        assert!((s.suggest_rpm - 21_436.0).abs() < 30.0 && (s.feed_scale - s.suggest_rpm / 24_000.0).abs() < 1e-4, "{s:?}");
        // it goes into the log rCAD reads
        let back: CutLog = serde_json::from_str(&serde_json::to_string(&log).unwrap()).unwrap();
        assert_eq!(back.suggestions, log.suggestions);
    }

    #[test]
    fn logs_by_line_and_reads_back() {
        let mut a = Analyzer::new(48_000);
        let mut log = CutLog::new("op2-top.nc", "test");
        let ctx = |line| Context { line, op: "3D rough".into(), tool: 24, teeth: 3, rpm: 24_000.0, feed: 2400.0, cutting: true, ..Default::default() };
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
        assert_eq!(CutLog::path_for_run(&dir.join("op2-top.nc"), false), p);
        assert!(CutLog::path_for_run(&dir.join("op2-top.nc"), true).to_string_lossy().ends_with("op2-top.nc.air.cutlog.json"));
        assert!(CutLog::wav_for_run(&dir.join("op2-top.nc"), true).to_string_lossy().ends_with("op2-top.nc.air.wav"));
        assert!(log.lines[&40].tooth_frames > 0 && log.lines[&40].tooth_mean_db > log.lines[&40].loud_mean_db - 200.0);
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
