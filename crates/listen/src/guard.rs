//! Feed hold on a bogging spindle: when the heard speed falls more than a limit under its no-load speed, for
//! longer than a time, while cutting, say so (the app holds the feed; the spindle stays on).
//!
//! The limit is the slip in rpm: an induction spindle slips about the same rpm for the same torque at any speed
//! (the DMC2, the same cut at 24k-15k: 650-720 rpm), so a percent is lenient at 24k and strict at 9-12k. A percent
//! cap stays behind it. Measured on the DMC2 (T24, 2026-11-17/18): steady cuts settle at 330-1070 rpm of slip;
//! 24k cuts at its edge creep at 1230-1630 (the broken-tool run's C3 at about 1560, then C4 ran away and broke the
//! tool); at 12k with the VFD's F004 = 200, 840 W by rCAD's model slipped 960-1130 rpm without running away.

use crate::Frame;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GuardSettings {
    pub on: bool,
    /// Hold when the slip under the no-load speed is over this many rpm...
    #[serde(default = "slip_rpm")]
    pub slip_rpm: f64,
    /// ...or over this share of the speed (7%: at 12k with the VFD at half flux the DMC2 pulled out at about 1000 rpm
    /// of slip, 8%, under any rpm limit that lets the same spindle carry its 24k cuts; at full flux 7% at 12k is
    /// 759 W by rCAD's model, past the planning limit),
    #[serde(default = "cap")]
    pub cap: f64,
    /// for at least this long, s.
    pub time: f64,
    /// After the spindle starts or changes speed, wait this long before judging, s.
    pub settle: f64,
    /// Also hold when the slip is over `rising_rpm` and still growing (at least `rising_rate` rpm a second, never
    /// dropping back) over `rising_time` s: a cut the spindle can't carry keeps slowing down. Judged only from
    /// `entry` s after a cut starts (and again after it starts again from a hold), past the entry ramp: a cut that
    /// settles takes up to about 0.8 s at 24k to get there.
    #[serde(default = "rising_rpm")]
    pub rising_rpm: f64,
    #[serde(default = "rising_rate")]
    pub rising_rate: f64,
    #[serde(default = "rising_time")]
    pub rising_time: f64,
    #[serde(default = "entry")]
    pub entry: f64,
}

fn slip_rpm() -> f64 {
    1400.0
}
fn cap() -> f64 {
    0.07
}
fn rising_rpm() -> f64 {
    800.0
}
fn rising_rate() -> f64 {
    100.0
}
fn rising_time() -> f64 {
    0.5
}
fn entry() -> f64 {
    1.0
}

impl Default for GuardSettings {
    fn default() -> Self {
        GuardSettings {
            on: true,
            slip_rpm: slip_rpm(),
            cap: cap(),
            time: 0.3,
            settle: 2.0,
            rising_rpm: rising_rpm(),
            rising_rate: rising_rate(),
            rising_time: rising_time(),
            entry: entry(),
        }
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
    /// When the cut in hand started to load the spindle (None between cuts and while held).
    engaged_at: Option<f64>,
    /// Recent (time, slip rpm) past the entry, for the rising trigger; the last heard excess (share).
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

    /// A window's sag over the no-load sag learned at its speed: the one scale the hold and the adaptive feed
    /// both work in. None without a heard speed or before any no-load sag is learned there.
    pub fn excess(&self, f: &Frame) -> Option<f64> {
        Some(f.droop? - self.air_droop(f.ctx.rpm)?)
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
            self.engaged_at = None;
            self.recent.clear();
        }
        if rpm <= 0.0 || t < self.settle_until {
            return None;
        }
        let key = (rpm / 250.0).round() as i64;
        // standing: its own no-load sag, the fastest it runs here, rising only slowly (a slow end of spin-up or a
        // stray window can't make the reference slow). The long window's reading (`air_droop`) counts too: in air
        // the short window can mistake the machine's fixed lines for the rotation (v2 run 1: 2.9% for a real 1.2%)
        if !f.ctx.cutting {
            let reading = match (f.air_droop, f.droop) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
            if let Some(d) = reading {
                let a = self.air.entry(key).or_insert(d);
                *a = if d < *a { d } else { *a + 0.02 * (d - *a) };
            }
            self.over_since = None;
            self.tripped = false;
            self.engaged_at = None;
            self.recent.clear();
            self.last_excess = 0.0;
            return None;
        }
        let st = self.settings;
        let d = match f.droop {
            Some(d) => d,
            // speed lost while cutting right after a big slip: it ran away past what the analysis searches
            None if f.ctx.cutting && self.last_excess * rpm > st.rising_rpm => self.last_excess + self.air.get(&key).copied().unwrap_or(0.0),
            None => return None,
        };
        let excess = d - self.air.get(&key).copied().unwrap_or(0.0);
        let slip = excess * rpm;
        self.last_excess = excess;
        // a cut starts when it loads the spindle, and starts again after a hold (the feed stopped: not cutting)
        if slip < 100.0 {
            self.engaged_at = None;
        } else if self.engaged_at.is_none() && slip > 150.0 {
            self.engaged_at = Some(t);
        }
        let past_entry = self.engaged_at.is_some_and(|t0| t - t0 >= st.entry - 1e-9);
        if past_entry {
            self.recent.push_back((t, slip));
        } else {
            self.recent.clear();
        }
        while self.recent.front().is_some_and(|(t0, _)| t - t0 > st.rising_time + 0.15) {
            self.recent.pop_front();
        }
        // still slowing down: over rising_rpm the whole time and gaining at least rising_rate, never dropping back
        let rising = self.recent.front().is_some_and(|(t0, s0)| t - t0 >= st.rising_time - 1e-9 && slip - s0 >= st.rising_rate * st.rising_time)
            && self.recent.iter().all(|(_, s)| *s > st.rising_rpm)
            && self.recent.iter().zip(self.recent.iter().skip(1)).all(|(a, b)| b.1 >= a.1 - 15.0);
        let over = slip > st.slip_rpm || excess > st.cap;
        if !over && !rising {
            self.over_since = None;
            self.tripped = false;
            return None;
        }
        let since = if over { *self.over_since.get_or_insert(t) } else { t };
        let steady_over = over && t - since >= st.time - 1e-9;
        if st.on && !self.tripped && (steady_over || rising) {
            self.tripped = true;
            let why = if steady_over {
                if slip > st.slip_rpm {
                    format!("over {:.0} rpm of slip for {:.1} s", st.slip_rpm, t - since)
                } else {
                    format!("over {:.0}% for {:.1} s", st.cap * 100.0, t - since)
                }
            } else {
                format!("still slowing down after {:.1} s in the cut", t - self.engaged_at.unwrap_or(t))
            };
            return Some(format!(
                "Feed hold: the spindle is {:.0} rpm ({:.1}%) slow under load ({why}) at line {}: it is bogging. Lighten the cut, or Cycle Start to carry on",
                slip,
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
        assert!(h[0].1.contains("(7.5%)") && h[0].1.contains("1800 rpm") && h[0].1.contains("line 7"), "{}", h[0].1);
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
        // (judged from 1 s into the cut, over 800 rpm then and growing 480 rpm/s; the 1400 rpm level from 1.8 s)
        assert!(m.contains("still slowing down") && after < 1.7, "{after} {m}");
        // a cut that settles at 4.5% (1080 rpm: over rising_rpm, under the limit) after its entry ramp: no hold
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

    /// v2 run 1's 12k pass with the VFD at half flux (F004 = 400): the slip ran from 5.7% to 8.3% in a second and
    /// kept growing (the old guard held at 1.1 s; the recording stops there). Its pull-out is at about 1000 rpm
    /// of slip, under the rpm limit: the 7% cap holds it as early.
    #[test]
    fn holds_the_half_flux_12k_stall() {
        let mut g = Guard::new(GuardSettings::default());
        let mut t = 0.0;
        while t < 3.0 {
            g.update(t, &frame(12_000.0, false, Some(0.03)));
            t += 0.085;
        }
        let start = t;
        let mut held = None;
        while t < start + 3.0 && held.is_none() {
            // over its no-load sag: 0 -> 5.7% in 0.25 s, then on up 2.6% a second
            let e = (t - start) / 0.25 * 0.057;
            let e = if t - start < 0.25 { e } else { 0.057 + 0.026 * (t - start - 0.25) / 1.0 * 1.0 };
            held = g.update(t, &frame(12_000.0, true, Some(0.03 + e))).map(|m| (t - start, m));
            t += 0.085;
        }
        let (after, m) = held.expect("held");
        assert!(after < 1.2, "{after} {m}");
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
