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
}

pub struct Analyzer {
    rate: f32,
    n: usize,
    fft: Arc<dyn Fft<f32>>,
    window: Vec<f32>,
    buf: Vec<f32>,
    /// Samples between windows.
    hop: usize,
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
        Analyzer { rate: rate as f32, n, fft, window, buf: vec![], hop: n / 2 }
    }

    /// Add samples; a frame for each full window, with `ctx` as what was going on.
    pub fn push(&mut self, samples: &[f32], ctx: &Context) -> Vec<Frame> {
        self.buf.extend_from_slice(samples);
        let mut out = vec![];
        while self.buf.len() >= self.n {
            out.push(self.analyse(&self.buf[..self.n].to_vec(), ctx));
            self.buf.drain(..self.hop);
        }
        out
    }

    /// The window's magnitude spectrum, dB.
    fn spectrum(&self, x: &[f32]) -> Vec<f64> {
        let mut c: Vec<Complex<f32>> = x.iter().zip(&self.window).map(|(s, w)| Complex::new(s * w, 0.0)).collect();
        self.fft.process(&mut c);
        c[..self.n / 2].iter().map(|v| 20.0 * ((v.norm() as f64) / self.n as f64 + 1e-12).log10()).collect()
    }

    pub fn analyse(&self, x: &[f32], ctx: &Context) -> Frame {
        let rms = (x.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>() / x.len().max(1) as f64).sqrt();
        let loud_db = 20.0 * (rms + 1e-12).log10();
        let mut f = Frame { ctx: ctx.clone(), loud_db, ..Default::default() };
        if ctx.rpm <= 0.0 {
            return f;
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
        let (lo, hi) = ((300.0 / bin) as usize, ((self.rate as f64 / 2.0 * 0.9) / bin) as usize);
        if hi > lo + 8 {
            let mut band: Vec<f64> = db[lo..hi].to_vec();
            band.sort_by(|a, b| a.total_cmp(b));
            let floor = band[band.len() / 2];
            let harmonic = |hz: f64| {
                let k = (hz / heard_rot).round();
                k >= 1.0 && (hz - k * heard_rot).abs() <= (0.015 * hz).max(2.5 * bin)
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
                } else if db[i] - floor > CHATTER_PROMINENCE_DB && best.is_none_or(|b| db[i] > b.1 + floor) {
                    best = Some((hz, db[i] - floor));
                }
            }
            if let Some((hz, above)) = best {
                if above + floor >= top_h - CHATTER_UNDER_HARMONIC_DB {
                    f.chatter = Some((hz, above));
                }
            }
        }
        f
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
    fn quiet_without_a_spindle() {
        let mut a = Analyzer::new(48_000);
        let f = &a.push(&vec![0.0; 9000], &Context::default())[0];
        assert!(f.rpm.is_none() && f.chatter.is_none() && f.loud_db < -100.0);
    }
}
