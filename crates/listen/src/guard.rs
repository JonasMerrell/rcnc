//! Feed hold on a bogging spindle: when the heard speed droops more than a limit over its no-load droop, for
//! longer than a time, while cutting, say so (the app holds the feed; the spindle stays on).

use crate::Frame;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GuardSettings {
    pub on: bool,
    /// Droop over the no-load droop at that speed, share (0.06 = 6%).
    pub limit: f64,
    /// For at least this long, s.
    pub time: f64,
    /// After the spindle starts or changes speed, wait this long before judging, s.
    pub settle: f64,
    /// Also hold when the droop is over `rising_from` and still growing (not settled) over `rising_time` s:
    /// a cut the spindle can't carry keeps slowing down (the broken-tool run: C3 never settled, C4 ran away).
    #[serde(default = "rising_from")]
    pub rising_from: f64,
    #[serde(default = "rising_time")]
    pub rising_time: f64,
}

fn rising_from() -> f64 {
    0.04
}

fn rising_time() -> f64 {
    0.5
}

impl Default for GuardSettings {
    fn default() -> Self {
        GuardSettings { on: true, limit: 0.06, time: 0.3, settle: 2.0, rising_from: 0.04, rising_time: 0.5 }
    }
}

#[derive(Debug, Default)]
pub struct Guard {
    pub settings: GuardSettings,
    /// No-load droop per speed (rpm / 250): the VFD's own slip and the speed it really holds in air.
    air: HashMap<i64, f64>,
    rpm: f64,
    settle_until: f64,
    over_since: Option<f64>,
    tripped: bool,
    /// Recent (time, excess droop) while cutting, for the rising trigger; the last heard excess.
    recent: std::collections::VecDeque<(f64, f64)>,
    last_excess: f64,
}

impl Guard {
    pub fn new(settings: GuardSettings) -> Guard {
        Guard { settings, ..Default::default() }
    }

    /// The no-load droop learned at this speed.
    pub fn air_droop(&self, rpm: f64) -> Option<f64> {
        self.air.get(&((rpm / 250.0).round() as i64)).copied()
    }

    /// A window heard at `t` s: Some(why) when the feed should be held now (once per bog: it re-arms when the
    /// droop is back under the limit).
    pub fn update(&mut self, t: f64, f: &Frame) -> Option<String> {
        let rpm = f.ctx.rpm;
        if (rpm - self.rpm).abs() > 1.0 {
            // started, stopped or a new S: let the VFD get there
            self.rpm = rpm;
            self.settle_until = t + self.settings.settle;
            self.over_since = None;
            self.recent.clear();
        }
        if rpm <= 0.0 || t < self.settle_until {
            return None;
        }
        let key = (rpm / 250.0).round() as i64;
        let d = match f.droop {
            Some(d) => d,
            // speed lost while cutting right after a big droop: it ran away past what the analysis searches
            None if f.ctx.cutting && self.last_excess > self.settings.limit => self.last_excess + self.air.get(&key).copied().unwrap_or(0.0),
            None => return None,
        };
        if !f.ctx.cutting {
            // no load: its own droop. The fastest it runs here, rising only slowly, so a slow end of spin-up
            // or a stray window can't make the reference slow
            let a = self.air.entry(key).or_insert(d);
            *a = if d < *a { d } else { *a + 0.02 * (d - *a) };
            self.over_since = None;
            self.tripped = false;
            self.recent.clear();
            self.last_excess = 0.0;
            return None;
        }
        let excess = d - self.air.get(&key).copied().unwrap_or(0.0);
        self.last_excess = excess;
        self.recent.push_back((t, excess));
        while self.recent.front().is_some_and(|(t0, _)| t - t0 > self.settings.rising_time + 0.15) {
            self.recent.pop_front();
        }
        // still slowing down: over rising_from and growing across the last rising_time
        // (over rising_from the whole time, gaining at least 1%/s, never dropping back)
        let rising = self.recent.front().is_some_and(|(t0, e0)| t - t0 >= self.settings.rising_time - 1e-9 && excess - e0 > 0.01 * self.settings.rising_time)
            && self.recent.iter().all(|(_, e)| *e > self.settings.rising_from)
            && self.recent.iter().zip(self.recent.iter().skip(1)).all(|(a, b)| b.1 >= a.1 - 0.002);
        let over = excess > self.settings.limit;
        if !over && !rising {
            self.over_since = None;
            self.tripped = false;
            return None;
        }
        let since = if over { *self.over_since.get_or_insert(t) } else { t };
        let steady_over = over && t - since >= self.settings.time - 1e-9;
        if self.settings.on && !self.tripped && (steady_over || rising) {
            self.tripped = true;
            let why = if steady_over {
                format!("over {:.1}% for {:.1} s", self.settings.limit * 100.0, t - since)
            } else {
                format!("still slowing down after {:.1} s", self.settings.rising_time)
            };
            return Some(format!(
                "Feed hold: the spindle is {:.1}% slow under load ({why}) at line {}: it is bogging. Lighten the cut, or Cycle Start to carry on",
                excess * 100.0,
                f.ctx.line
            ));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Context;

    fn frame(rpm: f64, cutting: bool, droop: Option<f64>) -> Frame {
        Frame { ctx: Context { rpm, cutting, teeth: 3, line: 7, ..Default::default() }, droop, ..Default::default() }
    }

    #[test]
    fn holds_on_a_steady_bog_only() {
        let mut g = Guard::new(GuardSettings::default());
        let mut t = 0.0;
        let mut step = |g: &mut Guard, cutting: bool, d: f64, secs: f64| {
            let mut out = vec![];
            let end = t + secs;
            while t < end {
                if let Some(m) = g.update(t, &frame(24_000.0, cutting, Some(d))) {
                    out.push((t, m));
                }
                t += 0.085;
            }
            out
        };
        // spin-up: big droop while it gets there, not cutting: nothing
        assert!(step(&mut g, false, 0.12, 1.5).is_empty());
        // in air at speed: 1.5% of its own
        assert!(step(&mut g, false, 0.015, 2.0).is_empty());
        assert!((g.air_droop(24_000.0).unwrap() - 0.015).abs() < 0.005);
        // a normal cut, 4% (2.5 over air): nothing
        assert!(step(&mut g, true, 0.04, 2.0).is_empty());
        // a short spike over the limit (0.2 s): nothing
        assert!(step(&mut g, true, 0.10, 0.2).is_empty());
        assert!(step(&mut g, true, 0.04, 0.5).is_empty());
        // a steady bog, 9% (7.5 over air): one hold after 0.3 s
        let h = step(&mut g, true, 0.09, 1.0);
        assert_eq!(h.len(), 1, "{h:?}");
        assert!(h[0].1.contains("7.5% slow") && h[0].1.contains("line 7"), "{}", h[0].1);
        // after it recovers it holds again on the next bog
        assert!(step(&mut g, true, 0.03, 0.5).is_empty());
        assert_eq!(step(&mut g, true, 0.09, 1.0).len(), 1);
    }

    #[test]
    fn holds_on_a_droop_that_keeps_growing() {
        let mut g = Guard::new(GuardSettings::default());
        let mut t = 0.0;
        while t < 3.0 {
            g.update(t, &frame(24_000.0, false, Some(0.007)));
            t += 0.085;
        }
        // a cut that never settles: 3% rising 2%/s (under the 6% limit at first)
        let start = t;
        let mut held = None;
        while t < start + 3.0 && held.is_none() {
            let d = 0.03 + 0.02 * (t - start);
            held = g.update(t, &frame(24_000.0, true, Some(d))).map(|m| (t - start, m));
            t += 0.085;
        }
        let (after, m) = held.expect("held");
        // (it is over 4% from 0.5 s, over the 6% limit only from 1.5 s: the level trigger would hold at 1.8 s)
        assert!(m.contains("still slowing down") && after < 1.6, "{after} {m}");
        // a cut that settles at 4.5% (over rising_from, under the limit): no hold
        let mut g = Guard::new(GuardSettings::default());
        let mut t = 0.0;
        while t < 3.0 {
            g.update(t, &frame(24_000.0, false, Some(0.007)));
            t += 0.085;
        }
        let start = t;
        while t < start + 4.0 {
            let d = 0.007 + 0.045 * (1.0 - (-(t - start) / 0.15).exp());
            assert!(g.update(t, &frame(24_000.0, true, Some(d))).is_none(), "held a settled cut at {}", t - start);
            t += 0.085;
        }
    }

    #[test]
    fn a_speed_change_is_not_a_bog() {
        let mut g = Guard::new(GuardSettings::default());
        let mut t = 0.0;
        for _ in 0..40 {
            assert!(g.update(t, &frame(24_000.0, false, Some(0.01))).is_none());
            t += 0.085;
        }
        // S18000 while the feed moves on: the VFD takes a moment, the heard speed lags (looks like droop)
        for _ in 0..20 {
            assert!(g.update(t, &frame(18_000.0, true, Some(0.12))).is_none(), "held during the settle time at {t}");
            t += 0.085;
        }
        // off: nothing
        assert!(g.update(t, &frame(0.0, false, None)).is_none());
    }
}
