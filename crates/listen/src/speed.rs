//! The spindle's speed as heard, steadied for a readout: of the windows heard over the last `WINDOW_S`, the
//! speed most of them agree on (within `AGREE`), so the windows that snap a harmonic onto a neighbouring line
//! don't make the number jump. On the DMC2's transducer at 24k about one window in eight reads 380-420 rpm slow,
//! two or three of them close together at times, so a plain median over half a second still flipped. It keeps
//! showing the last speed for `HOLD_S` after the spindle stops being heard, then says it can't hear it.

use crate::Frame;
use std::collections::VecDeque;

/// The spindle's measured speed now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Speed {
    /// Heard, rpm.
    pub rpm: f64,
    /// Programmed (the S word), rpm.
    pub programmed: f64,
    /// How far under the programmed speed it runs (0.03 = 3% slow; negative: fast).
    pub droop: f64,
    /// How many windows agree on it, of how many heard; and how far apart the heard ones are (max - min), rpm.
    pub agree: usize,
    pub windows: usize,
    pub spread: f64,
}

impl Speed {
    /// The slip under the speed the VFD drives the motor at (its output frequency's synchronous speed), rpm:
    /// the rotor's lag behind the field, which grows with the torque.
    pub fn slip_from(&self, vfd_rpm: f64) -> f64 {
        vfd_rpm - self.rpm
    }
}

#[derive(Debug, Default)]
pub struct SpeedMeter {
    /// (time, heard rpm) over the last WINDOW_S.
    recent: VecDeque<(f64, f64)>,
    programmed: f64,
    /// When the speed was last heard.
    heard_at: Option<f64>,
}

impl SpeedMeter {
    /// The windows heard in this long have their say, s (8 windows of the analysis at 48 or 24 kHz).
    pub const WINDOW_S: f64 = 0.7;
    /// Windows within this share of each other agree (0.5%: 120 rpm at 24k, 45 at 9k).
    pub const AGREE: f64 = 0.005;
    /// After the last heard window the readout stays this long, s.
    pub const HOLD_S: f64 = 1.0;

    pub fn new() -> SpeedMeter {
        SpeedMeter::default()
    }

    /// A window heard at `t` s.
    pub fn update(&mut self, t: f64, f: &Frame) {
        let prog = f.ctx.rpm;
        if (prog - self.programmed).abs() > 1.0 {
            // started, stopped or a new S: the old speeds say nothing about this one
            self.recent.clear();
            self.heard_at = None;
            self.programmed = prog;
        }
        if prog <= 0.0 {
            return;
        }
        if let Some(rpm) = f.rpm {
            self.recent.push_back((t, rpm));
            self.heard_at = Some(t);
            while self.recent.front().is_some_and(|(at, _)| *at < t - Self::WINDOW_S) {
                self.recent.pop_front();
            }
        }
    }

    /// The speed at `now` s (the same clock as `update`); None with the spindle off or not heard lately.
    pub fn get(&self, now: f64) -> Option<Speed> {
        if self.programmed <= 0.0 || self.heard_at.is_none_or(|t| now - t > Self::HOLD_S) {
            return None;
        }
        // the windows heard over WINDOW_S up to the last one heard: the one the most others agree with (on a tie
        // the latest, so a real change shows once half the windows have it), and the mean of those that agree
        let v: Vec<f64> = self.recent.iter().map(|r| r.1).collect();
        if v.is_empty() {
            return None;
        }
        let near = |a: f64, b: f64| (a - b).abs() <= Self::AGREE * a.max(b);
        let support = |a: f64| v.iter().filter(|b| near(a, **b)).count();
        let best = (0..v.len()).max_by_key(|i| (support(v[*i]), *i)).unwrap_or(0);
        let agreeing: Vec<f64> = v.iter().copied().filter(|b| near(v[best], *b)).collect();
        let rpm = agreeing.iter().sum::<f64>() / agreeing.len() as f64;
        let (lo, hi) = v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), x| (l.min(*x), h.max(*x)));
        Some(Speed { rpm, programmed: self.programmed, droop: 1.0 - rpm / self.programmed, agree: agreeing.len(), windows: v.len(), spread: hi - lo })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Context;

    fn frame(prog: f64, heard: Option<f64>) -> Frame {
        Frame { ctx: Context { rpm: prog, ..Default::default() }, rpm: heard, ..Default::default() }
    }

    #[test]
    fn one_window_off_does_not_move_the_readout() {
        let mut m = SpeedMeter::new();
        // as the DMC2 heard it at 24k in air: a few windows snap about 400 rpm slow, two of them together
        let heard = [23_820.0, 23_826.0, 23_410.0, 23_824.0, 23_822.0, 23_405.0, 23_398.0, 23_825.0, 23_823.0];
        for (i, h) in heard.iter().enumerate() {
            m.update(i as f64 * 0.085, &frame(24_000.0, Some(*h)));
            let s = m.get(i as f64 * 0.085).unwrap();
            if i >= 2 {
                assert!((s.rpm - 23_822.0).abs() < 5.0, "{i}: {s:?}");
            }
        }
        let s = m.get(0.8).unwrap();
        assert!((s.droop - (1.0 - 23_823.0 / 24_000.0)).abs() < 0.0003, "{s:?}");
        assert!((s.slip_from(24_000.0) - 176.0).abs() < 3.0);
    }

    #[test]
    fn it_follows_a_real_change_within_the_window() {
        let mut m = SpeedMeter::new();
        let mut t = 0.0;
        for _ in 0..10 {
            m.update(t, &frame(9_000.0, Some(8_950.0)));
            t += 0.085;
        }
        // the cut loads it: 300 rpm slower from here; it shows once half the windows have it
        for _ in 0..5 {
            m.update(t, &frame(9_000.0, Some(8_650.0)));
            t += 0.085;
        }
        assert!((m.get(t).unwrap().rpm - 8_650.0).abs() < 1.0);
    }

    #[test]
    fn unheard_or_off_says_so() {
        let mut m = SpeedMeter::new();
        assert!(m.get(0.0).is_none());
        m.update(0.0, &frame(12_000.0, Some(11_800.0)));
        // windows that don't hear it: the last speed stands for HOLD_S, then none
        m.update(0.5, &frame(12_000.0, None));
        assert!(m.get(0.5).is_some());
        m.update(1.2, &frame(12_000.0, None));
        assert!(m.get(1.2).is_none());
        // a new S forgets the old speed
        m.update(1.3, &frame(12_000.0, Some(11_810.0)));
        m.update(1.4, &frame(18_000.0, None));
        assert!(m.get(1.4).is_none());
        // spindle off
        m.update(1.5, &frame(18_000.0, Some(17_700.0)));
        m.update(1.6, &frame(0.0, None));
        assert!(m.get(1.6).is_none());
    }
}
