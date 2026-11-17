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
}

impl Default for GuardSettings {
    fn default() -> Self {
        GuardSettings { on: true, limit: 0.06, time: 0.3, settle: 2.0 }
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
        }
        if rpm <= 0.0 || t < self.settle_until {
            return None;
        }
        let key = (rpm / 250.0).round() as i64;
        let Some(d) = f.droop else { return None };
        if !f.ctx.cutting {
            // no load: follow its own droop (slowly, so a stray window doesn't move it)
            let a = self.air.entry(key).or_insert(d);
            *a += 0.2 * (d - *a);
            self.over_since = None;
            self.tripped = false;
            return None;
        }
        let excess = d - self.air.get(&key).copied().unwrap_or(0.0);
        if excess <= self.settings.limit {
            self.over_since = None;
            self.tripped = false;
            return None;
        }
        let since = *self.over_since.get_or_insert(t);
        if self.settings.on && !self.tripped && t - since >= self.settings.time - 1e-9 {
            self.tripped = true;
            return Some(format!(
                "Feed hold: the spindle is {:.1}% slow under load (over {:.1}% for {:.1} s) at line {}: it is bogging. Lighten the cut or Cycle Start to carry on",
                excess * 100.0,
                self.settings.limit * 100.0,
                t - since,
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
