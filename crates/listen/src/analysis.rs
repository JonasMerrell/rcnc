//! The rolling spectrum and what it says about the cut.

use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

/// What the machine is doing while the samples come in.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Context {
    /// Program line (from 1), operation (the program's last "(Name - Kind)" comment), tool.
    pub line: usize,
    pub op: String,
    pub tool: u32,
    pub teeth: u32,
    /// Programmed spindle speed (0: off) and feed, mm/min.
    pub rpm: f64,
    pub feed: f64,
    /// A feed move is running with the spindle on (the tool may be in the work).
    pub cutting: bool,
    /// Where the tool is (work X, Y, Z) and how fast it really moves, mm/min: the programmed feed is what the
    /// interpreter last read, which runs ahead of the motion.
    #[serde(default)]
    pub pos: [f64; 3],
    #[serde(default)]
    pub velocity: f64,
}

/// One analysed window of sound.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct Frame {
    pub ctx: Context,
    /// The spindle's speed as heard, and how far under the programmed speed it is (0.03 = 3% slow).
    pub rpm: Option<f64>,
    pub droop: Option<f64>,
    /// The strongest peak that isn't a harmonic of the spindle's rotation, if strong enough to be chatter:
    /// its frequency and how far above the background it stands, dB.
    pub chatter: Option<(f64, f64)>,
    /// Level of the window, dB re full scale.
    pub loud_db: f64,
    /// Level of the teeth's beat (the tooth-passing frequency at the heard speed), dB on the spectrum's scale:
    /// in a cut it rises with the load, so it's a cleaner load measure than the whole window's level.
    pub tooth_db: Option<f64>,
}

pub struct Analyzer {
    rate: f32,
    n: usize,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buf: Vec<f32>,
    /// Samples between windows.
    hop: usize,
    /// The spindle's own lines per speed (rpm / 250, rounded), heard with it on and not cutting: frequency
    /// and loudest level, dB. A pickup on the spindle head hears its bearings, the VFD's carrier and the
    /// motor's slot lines too; in a cut a peak there is chatter only if it stands well over its air level.
    air: std::collections::HashMap<i64, Vec<(f64, f64)>>,
}

/// In a cut, a peak at one of the spindle's own (air) lines counts as chatter only this far over its air level, dB.
pub const CHATTER_OVER_AIR_DB: f64 = 10.0;

fn air_key(rpm: f64) -> i64 {
    (rpm / 250.0).round() as i64
}

/// A peak must stand this far above the median of the spectrum to count as chatter, dB, and be no more
/// than this far under the strongest spindle harmonic.
pub const CHATTER_PROMINENCE_DB: f64 = 20.0;
pub const CHATTER_UNDER_HARMONIC_DB: f64 = 10.0;
/// How far the spindle may run under its programmed speed and still be found, share.
pub const MAX_DROOP: f64 = 0.15;

impl Analyzer {
    /// About 0.17 s windows (8192 samples at 48 kHz), half overlapping.
    pub fn new(rate: u32) -> Analyzer {
        let n = if rate >= 32_000 { 8192 } else { 4096 };
        let fft = FftPlanner::new().plan_fft_forward(n);
        let window = (0..n).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos()).collect();
        Analyzer { rate: rate as f32, n, fft, window, buf: vec![], hop: n / 2, air: Default::default() }
    }

    /// Add samples; a frame for each full window, with `ctx` as what was going on.
    pub fn push(&mut self, samples: &[f32], ctx: &Context) -> Vec<Frame> {
        self.buf.extend_from_slice(samples);
        let mut out = vec![];
        while self.buf.len() >= self.n {
            let x = self.buf[..self.n].to_vec();
            let air = self.air.get(&air_key(ctx.rpm)).cloned().unwrap_or_default();
            let (f, peaks) = self.analyse_with(&x, ctx, &air);
            // spindle on, not cutting: learn its own lines at this speed
            if ctx.rpm > 0.0 && !ctx.cutting {
                let known = self.air.entry(air_key(ctx.rpm)).or_default();
                for (hz, db) in peaks {
                    match known.iter_mut().find(|k| (k.0 - hz).abs() <= (0.01 * hz).max(3.0 * self.rate as f64 / self.n as f64)) {
                        Some(k) => k.1 = k.1.max(db),
                        None => known.push((hz, db)),
                    }
                }
            }
            out.push(f);
            self.buf.drain(..self.hop);
        }
        out
    }

    /// How many of the spindle's own lines it has heard in air at this speed.
    pub fn air_lines(&self, rpm: f64) -> usize {
        self.air.get(&air_key(rpm)).map(|v| v.len()).unwrap_or(0)
    }

    /// The window's magnitude spectrum, dB.
    fn spectrum(&self, x: &[f32]) -> Vec<f64> {
        let mut c: Vec<Complex<f32>> = x.iter().zip(&self.window).map(|(s, w)| Complex::new(s * w, 0.0)).collect();
        self.fft.process(&mut c);
        c[..self.n / 2].iter().map(|v| 20.0 * ((v.norm() as f64) / self.n as f64 + 1e-12).log10()).collect()
    }

    pub fn analyse(&self, x: &[f32], ctx: &Context) -> Frame {
        self.analyse_with(x, ctx, &[]).0
    }

    /// `analyse` with the spindle's air lines at this speed (peaks there must stand CHATTER_OVER_AIR_DB over
    /// them to count as chatter); also returns the prominent peaks that aren't rotation harmonics (hz, dB).
    fn analyse_with(&self, x: &[f32], ctx: &Context, air: &[(f64, f64)]) -> (Frame, Vec<(f64, f64)>) {
        let rms = (x.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>() / x.len().max(1) as f64).sqrt();
        let loud_db = 20.0 * (rms + 1e-12).log10();
        let mut f = Frame { ctx: ctx.clone(), loud_db, ..Default::default() };
        let mut peaks = vec![];
        if ctx.rpm <= 0.0 {
            return (f, peaks);
        }
        let db = self.spectrum(x);
        let bin = self.rate as f64 / self.n as f64;
        // the spindle's rotation, or in a cut the teeth passing (a tooth's beat is louder)
        let teeth = ctx.teeth.max(1) as f64;
        let rot = ctx.rpm / 60.0;
        let base = if ctx.cutting && teeth > 1.0 { rot * teeth } else { rot };
        // the rotation speed that best explains the peaks: the sum of the spectrum at its first harmonics (the
        // rotation, the teeth's beat and theirs), searched from MAX_DROOP slow to 2% fast; so a wrong tooth
        // count in the tool table doesn't matter
        let _ = (teeth, base);
        let lin: Vec<f64> = db.iter().map(|d| 10f64.powf(d / 20.0)).collect();
        let at = |hz: f64| {
            let i = (hz / bin).round() as usize;
            if i + 1 >= lin.len() || i == 0 {
                0.0
            } else {
                lin[i - 1].max(lin[i]).max(lin[i + 1])
            }
        };
        let harmonics = 12usize.min(((self.rate as f64 / 2.0 * 0.9) / rot) as usize).max(1);
        let score = |f: f64| (1..=harmonics).map(|k| at(k as f64 * f)).sum::<f64>();
        let (lo_f, hi_f) = (rot * (1.0 - MAX_DROOP), rot * 1.02);
        let step = bin / (2.0 * harmonics as f64);
        let mut best = (f64::NEG_INFINITY, rot);
        let mut f0 = lo_f;
        while f0 <= hi_f {
            let sc = score(f0);
            if sc > best.0 {
                best = (sc, f0);
            }
            f0 += step;
        }
        // it must stand out from the harmonics of a frequency well off it
        let off = score(best.1 * 1.37).max(score(best.1 * 0.71));
        if best.0 > 2.0 * off {
            // finer: the strongest of those harmonics, its peak interpolated between bins, over its number
            let k = (1..=harmonics).max_by(|a, b| at(*a as f64 * best.1).total_cmp(&at(*b as f64 * best.1))).unwrap_or(1);
            let i = (k as f64 * best.1 / bin).round() as usize;
            let i = (i.saturating_sub(1)..=(i + 1).min(db.len() - 2)).max_by(|a, b| db[*a].total_cmp(&db[*b])).unwrap_or(i);
            let fine = self.interp(&db, i) / k as f64;
            let rpm = fine / rot * ctx.rpm;
            f.rpm = Some(rpm);
            f.droop = Some(1.0 - rpm / ctx.rpm);
        }
        // chatter: the strongest peak between 300 Hz and the top that isn't a rotation harmonic (at the heard speed)
        let heard_rot = f.rpm.unwrap_or(ctx.rpm) / 60.0;
        let tooth = heard_rot * teeth;
        let ti = (tooth / bin).round() as usize;
        if ti >= 1 && ti + 1 < db.len() {
            f.tooth_db = Some(db[ti - 1].max(db[ti]).max(db[ti + 1]));
        }
        let (lo, hi) = ((300.0 / bin) as usize, ((self.rate as f64 / 2.0 * 0.9) / bin) as usize);
        if hi > lo + 8 {
            let mut band: Vec<f64> = db[lo..hi].to_vec();
            band.sort_by(|a, b| a.total_cmp(b));
            let floor = band[band.len() / 2];
            let harmonic = |hz: f64| {
                let k = (hz / heard_rot).round();
                k >= 1.0 && (hz - k * heard_rot).abs() <= (0.004 * hz).max(2.5 * bin)
            };
            let mut top_h = f64::NEG_INFINITY;
            let mut best: Option<(f64, f64)> = None;
            for i in lo + 1..hi - 1 {
                if db[i] < db[i - 1] || db[i] < db[i + 1] {
                    continue;
                }
                let hz = self.interp(&db, i) ;
                if harmonic(hz) {
                    top_h = top_h.max(db[i]);
                } else if db[i] - floor > CHATTER_PROMINENCE_DB {
                    peaks.push((hz, db[i]));
                    // one of the spindle's own lines: only as chatter if well over its air level
                    let own = air.iter().find(|a| (a.0 - hz).abs() <= (0.01 * hz).max(3.0 * bin));
                    if own.is_some_and(|a| db[i] < a.1 + CHATTER_OVER_AIR_DB) {
                        continue;
                    }
                    if best.is_none_or(|b| db[i] > b.1 + floor) {
                        best = Some((hz, db[i] - floor));
                    }
                }
            }
            if let Some((hz, above)) = best {
                if above + floor >= top_h - CHATTER_UNDER_HARMONIC_DB {
                    f.chatter = Some((hz, above));
                }
            }
        }
        (f, peaks)
    }

    /// The strongest peak between `lo` and `hi` Hz, interpolated between bins.
    fn peak_near(&self, db: &[f64], lo: f64, hi: f64) -> Option<f64> {
        let bin = self.rate as f64 / self.n as f64;
        let (a, b) = ((lo / bin).floor().max(1.0) as usize, ((hi / bin).ceil() as usize).min(db.len() - 2));
        if b <= a {
            return None;
        }
        let i = (a..=b).max_by(|x, y| db[*x].total_cmp(&db[*y]))?;
        // must stand out from what's around it
        let around: f64 = (a..=b).map(|k| db[k]).sum::<f64>() / (b - a + 1) as f64;
        (db[i] - around > 6.0).then(|| self.interp(db, i))
    }

    fn interp(&self, db: &[f64], i: usize) -> f64 {
        let bin = self.rate as f64 / self.n as f64;
        if i == 0 || i + 1 >= db.len() {
            return i as f64 * bin;
        }
        let (a, b, c) = (db[i - 1], db[i], db[i + 1]);
        let d = a - 2.0 * b + c;
        let off = if d.abs() > 1e-12 { 0.5 * (a - c) / d } else { 0.0 };
        (i as f64 + off.clamp(-0.5, 0.5)) * bin
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A spindle at `rpm` with `teeth` teeth cutting: the tooth-passing tone and harmonics, the rotation, some
    /// noise; plus an optional chatter tone.
    pub fn cut(rpm: f64, teeth: u32, chatter: Option<f64>, secs: f64, rate: u32) -> Vec<f32> {
        let mut x = 12345u64;
        let mut noise = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x % 10_000) as f32 / 10_000.0 - 0.5
        };
        let rot = rpm / 60.0;
        let ft = rot * teeth as f64;
        (0..(secs * rate as f64) as usize)
            .map(|i| {
                let t = i as f64 / rate as f64;
                let mut s = 0.25 * (2.0 * std::f64::consts::PI * ft * t).sin() + 0.1 * (2.0 * std::f64::consts::PI * 2.0 * ft * t).sin() + 0.05 * (2.0 * std::f64::consts::PI * rot * t).sin();
                if let Some(c) = chatter {
                    s += 0.15 * (2.0 * std::f64::consts::PI * c * t).sin();
                }
                (s as f32) + 0.02 * noise()
            })
            .collect()
    }

    #[test]
    fn hears_the_spindle_slow_down_under_load() {
        let mut a = Analyzer::new(48_000);
        let ctx = Context { rpm: 24_000.0, teeth: 3, cutting: true, ..Default::default() };
        // really turning 3% slow
        let frames = a.push(&cut(24_000.0 * 0.97, 3, None, 1.0, 48_000), &ctx);
        assert!(frames.len() >= 4);
        for f in &frames {
            let d = f.droop.expect("found the spindle");
            assert!((d - 0.03).abs() < 0.004, "{f:?}");
            assert!(f.chatter.is_none(), "{f:?}");
        }
    }

    #[test]
    fn hears_chatter_between_the_harmonics() {
        let mut a = Analyzer::new(48_000);
        let ctx = Context { rpm: 24_000.0, teeth: 3, cutting: true, ..Default::default() };
        let frames = a.push(&cut(24_000.0, 3, Some(3730.0), 0.5, 48_000), &ctx);
        let (hz, above) = frames[0].chatter.expect("chatter");
        assert!((hz - 3730.0).abs() < 10.0 && above > 20.0, "{:?}", frames[0]);
    }

    /// The tool table says 2 teeth, the tool has 3: the speed comes from the rotation, and the teeth's beat
    /// (a rotation harmonic) isn't taken for chatter.
    #[test]
    fn wrong_tooth_count_still_finds_the_speed() {
        let mut a = Analyzer::new(48_000);
        let ctx = Context { rpm: 24_000.0, teeth: 2, cutting: true, ..Default::default() };
        let f = &a.push(&cut(24_000.0 * 0.97, 3, None, 0.5, 48_000), &ctx)[0];
        assert!((f.droop.unwrap() - 0.03).abs() < 0.004 && f.chatter.is_none(), "{f:?}");
    }

    #[test]
    fn the_teeth_beat_louder_in_a_heavier_cut() {
        let a = Analyzer::new(48_000);
        let ctx = Context { rpm: 24_000.0, teeth: 3, cutting: true, ..Default::default() };
        let light: Vec<f32> = cut(24_000.0, 3, None, 0.5, 48_000).iter().map(|s| s * 0.3).collect();
        let heavy = cut(24_000.0, 3, None, 0.5, 48_000);
        let (l, h) = (a.analyse(&light[..a.n], &ctx).tooth_db.unwrap(), a.analyse(&heavy[..a.n], &ctx).tooth_db.unwrap());
        // 0.3 of the amplitude: about 10.5 dB lower
        assert!((h - l - 10.46).abs() < 1.0, "{l} {h}");
    }

    /// The spindle's own line (a pickup on the head hears e.g. the VFD at 8.6 kHz) is learned in air and not
    /// taken for chatter in the cut; real chatter elsewhere still is, and so is the same line grown much louder.
    #[test]
    fn the_spindles_own_lines_are_not_chatter() {
        let mut a = Analyzer::new(48_000);
        let air_ctx = Context { rpm: 24_000.0, teeth: 3, cutting: false, ..Default::default() };
        let cut_ctx = Context { cutting: true, ..air_ctx.clone() };
        // in air: the rotation plus a steady 8617 Hz line, as loud as chatter would be
        let air: Vec<f32> = cut(24_000.0, 3, Some(8617.0), 1.0, 48_000);
        let fresh = Analyzer::new(48_000);
        assert!(fresh.analyse(&air[..fresh.n], &cut_ctx).chatter.is_some(), "without a baseline it looks like chatter");
        let frames = a.push(&air, &air_ctx);
        assert!(!frames.is_empty() && a.air_lines(24_000.0) > 0);
        // cutting with the same line: not chatter
        let same = a.push(&air, &cut_ctx);
        assert!(same.iter().all(|f| f.chatter.is_none()), "{:?}", same.iter().map(|f| f.chatter).collect::<Vec<_>>());
        // real chatter at the tool's mode, 7000 Hz, beside it: chatter
        let mut both = cut(24_000.0, 3, Some(8617.0), 1.0, 48_000);
        for (i, s) in both.iter_mut().enumerate() {
            *s += 0.15 * (2.0 * std::f64::consts::PI * 7000.0 * i as f64 / 48_000.0).sin() as f32;
        }
        let f = a.push(&both, &cut_ctx);
        assert!(f.iter().any(|f| f.chatter.is_some_and(|c| (c.0 - 7000.0).abs() < 15.0)), "{:?}", f.iter().map(|f| f.chatter).collect::<Vec<_>>());
    }

    #[test]
    fn quiet_without_a_spindle() {
        let mut a = Analyzer::new(48_000);
        let f = &a.push(&vec![0.0; 9000], &Context::default())[0];
        assert!(f.rpm.is_none() && f.chatter.is_none() && f.loud_db < -100.0);
    }
}
