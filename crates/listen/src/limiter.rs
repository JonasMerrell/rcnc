//! Adaptive feed: a feed limiter on the spindle's sag, and (stage 2, when a program sets `max` over 1) a capped
//! speed-up in air. While a program cuts with the bog
//! guard armed, it trims the feed (a share of the programmed feed, `min` to 1) to hold the heard sag over the
//! no-load sag near a target. It never asks for more than the programmed feed; the bog guard's feed hold stays
//! behind it for what it can't catch (a cut too heavy even at `min`, or too sudden for its reaction time).
//!
//! Reaction time: a window is 0.17 s of sound (heard about 0.1 s after its middle, in the app's batches), and
//! the board has 0.245 s of motion queued at the old feed, so a change shows in the sag about 0.4 s after the
//! cut changed. The loop is slow on purpose: a proportional step on the sag's rise and an integral on its
//! distance from the target, both on the feed's logarithm, slew-limited.
//!
//! Fail-safe: no new window, no heard speed, no learned no-load sag, the spindle off or not moving: the feed
//! stays where it is (never raised on a guess); a heard speed lost right after a high sag counts as high.
//!
//! In air (stage 2): a window is air when the teeth's beat is under the level heard with the spindle on and the
//! machine standing plus `air_db`, and the sag is under `air_sag` (the broken-tool run: air -64 dB standing,
//! -70 to -89 moving; C1 0.6 x 6 -46 to -55 dB and 1.4% sag). After `air_confirm` s of air the feed rises to
//! `max` at `slew_air` a second; the first window that isn't air takes it straight back to the programmed feed
//! (or under). That first window comes about 0.4 s after the tool meets the work, so the cut's first 0.4 s runs
//! at up to `max`: keep it small (1.2: a 430 W pass enters at about 490 W). The controller never applies more
//! than the programmed feed to a move going down.

use crate::Frame;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LimiterSettings {
    /// Sag over the no-load sag to hold, share (0.03 = 3%), on the bog guard's scale (`Guard::excess`; the
    /// broken-tool run's C1 0.6 x 6 read 1.4%, C2 1.0 x 6 (rCAD 499 W) 3.0%, C3 6.5% and rising; the guard
    /// holds at 6%).
    pub target: f64,
    /// The lowest share of the programmed feed it goes to.
    pub min: f64,
    /// Proportional gain: change of ln(feed) per unit rise of the sag (share).
    pub kp: f64,
    /// Integral gain: change of ln(feed) per second per unit of sag over the target (share).
    pub ki: f64,
    /// Fastest it lowers / raises the feed, share of the programmed feed per second.
    pub slew_down: f64,
    pub slew_up: f64,
    /// After a cut starts (the sag first over half the target), don't raise the feed for this long, s.
    pub hold_off: f64,
    /// In air: the highest share of the programmed feed (1 = no speed-up), how fast it gets there (share a
    /// second), how long it must be air first (s), and what counts as air (the teeth's beat over its standing
    /// level, dB; the sag over no-load, share).
    #[serde(default = "one")]
    pub max: f64,
    #[serde(default = "slew_air")]
    pub slew_air: f64,
    #[serde(default = "air_confirm")]
    pub air_confirm: f64,
    #[serde(default = "air_db")]
    pub air_db: f64,
    #[serde(default = "air_sag")]
    pub air_sag: f64,
}

fn one() -> f64 {
    1.0
}
fn slew_air() -> f64 {
    0.5
}
fn air_confirm() -> f64 {
    0.3
}
fn air_db() -> f64 {
    6.0
}
fn air_sag() -> f64 {
    0.007
}

impl Default for LimiterSettings {
    fn default() -> Self {
        LimiterSettings {
            target: 0.03,
            min: 0.6,
            kp: 4.0,
            ki: 10.0,
            slew_down: 2.0,
            slew_up: 0.15,
            hold_off: 0.5,
            max: one(),
            slew_air: slew_air(),
            air_confirm: air_confirm(),
            air_db: air_db(),
            air_sag: air_sag(),
        }
    }
}

#[derive(Debug)]
pub struct Limiter {
    pub settings: LimiterSettings,
    /// The feed now asked for, share of the programmed feed.
    pub feed: f64,
    last_t: Option<f64>,
    last_excess: Option<f64>,
    /// When the sag last came up from under half the target (a cut starting).
    engaged_at: Option<f64>,
    /// The teeth's beat heard with the spindle on and the machine standing, dB; since when it's been air.
    air_tooth: Option<f64>,
    air_since: Option<f64>,
}

impl Limiter {
    pub fn new(settings: LimiterSettings) -> Limiter {
        Limiter { settings, feed: 1.0, last_t: None, last_excess: None, engaged_at: None, air_tooth: None, air_since: None }
    }

    /// Back to the programmed feed (a program starts or ends, an operation where it's off).
    pub fn reset(&mut self) {
        self.feed = 1.0;
        self.last_t = None;
        self.last_excess = None;
        self.engaged_at = None;
        self.air_since = None;
    }

    /// A window heard at `t` s, with its sag over the no-load sag as the bog guard measures it
    /// (`Guard::excess`: one baseline for both): the feed to ask for.
    pub fn update(&mut self, t: f64, f: &Frame, over_air: Option<f64>) -> f64 {
        let s = self.settings;
        let dt = self.last_t.map(|l| (t - l).clamp(0.0, 0.5)).unwrap_or(0.0);
        self.last_t = Some(t);
        let moving = f.ctx.rpm > 0.0 && f.ctx.cutting;
        // standing with the spindle on: the teeth's beat in air (the loudest lately, so air is judged strictly)
        if f.ctx.rpm > 0.0 && !f.ctx.cutting {
            if let Some(db) = f.tooth_db {
                let a = self.air_tooth.get_or_insert(db);
                *a = if db > *a { db } else { *a + 0.05 * (db - *a) };
            }
        }
        // in air: speed up after a while; anything else takes it back to the programmed feed at once
        let air = moving
            && s.max > 1.0
            && over_air.is_some_and(|e| e < s.air_sag)
            && f.tooth_db.zip(self.air_tooth).is_some_and(|(db, a)| db < a + s.air_db);
        if air {
            let since = *self.air_since.get_or_insert(t);
            self.last_excess = over_air;
            self.engaged_at = None;
            if t - since >= s.air_confirm - 1e-9 {
                self.feed = (self.feed + s.slew_air * dt).min(s.max);
            }
            return self.feed;
        }
        self.air_since = None;
        if self.feed > 1.0 {
            self.feed = 1.0;
        }
        let excess = match over_air {
            Some(e) if moving => Some(e),
            // the speed lost in a cut right after a high sag: it ran away past what the analysis searches
            None if moving && f.droop.is_none() && self.last_excess.is_some_and(|e| e > 1.5 * s.target) => self.last_excess.map(|e| e.max(2.0 * s.target)),
            _ => None,
        };
        let Some(e) = excess else {
            // nothing to go on: keep the feed
            self.last_excess = None;
            return self.feed;
        };
        if e < 0.5 * s.target {
            self.engaged_at = None;
        } else if self.engaged_at.is_none() {
            self.engaged_at = Some(t);
        }
        // a cut's first window rises from nothing
        let rise = e - self.last_excess.unwrap_or(0.0);
        self.last_excess = Some(e);
        // the proportional part only over the target (noise under it doesn't move the feed)
        let p = if e > s.target { s.kp * rise } else { 0.0 };
        let mut step = -(p + s.ki * (e - s.target) * dt);
        let raising = step > 0.0;
        if raising && self.engaged_at.is_some_and(|t0| t - t0 < s.hold_off) {
            step = 0.0;
        }
        let next = (self.feed.ln() + step).exp();
        let next = next.clamp(self.feed - s.slew_down * dt.max(0.05), self.feed + s.slew_up * dt);
        self.feed = next.clamp(s.min, 1.0);
        self.feed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Context;

    fn frame(cutting: bool, droop: Option<f64>) -> Frame {
        Frame { ctx: Context { rpm: 24_000.0, cutting, teeth: 3, ..Default::default() }, droop, ..Default::default() }
    }

    /// A window as the app passes it: its sag over `air`, the no-load sag the guard learned.
    fn step(l: &mut Limiter, t: f64, cutting: bool, droop: Option<f64>, air: Option<f64>) -> f64 {
        let f = frame(cutting, droop);
        let over = droop.zip(air).map(|(d, a)| d - a);
        l.update(t, &f, over)
    }

    #[test]
    fn a_light_cut_keeps_the_programmed_feed() {
        let mut l = Limiter::new(LimiterSettings::default());
        for k in 0..100 {
            assert_eq!(step(&mut l, k as f64 * 0.085, true, Some(0.0245 + 0.012), Some(0.0245)), 1.0);
        }
    }

    #[test]
    fn a_heavy_cut_is_slowed_down_but_not_under_the_floor() {
        let mut l = Limiter::new(LimiterSettings::default());
        let mut f = 1.0;
        for k in 0..100 {
            f = step(&mut l, k as f64 * 0.085, true, Some(0.0245 + 0.06), Some(0.0245));
        }
        assert_eq!(f, 0.6);
    }

    #[test]
    fn no_signal_keeps_the_feed_and_never_raises_it() {
        let mut l = Limiter::new(LimiterSettings::default());
        for k in 0..10 {
            step(&mut l, k as f64 * 0.085, true, Some(0.0245 + 0.04), Some(0.0245));
        }
        let low = l.feed;
        assert!(low < 1.0);
        // no learned no-load sag, a lost speed after a modest sag, the spindle stopped: it stays
        for k in 10..60 {
            let t = k as f64 * 0.085;
            assert!(step(&mut l, t, true, Some(0.0), None) <= low + 1e-12);
            assert!(step(&mut l, t, false, Some(0.0), Some(0.0245)) <= low + 1e-12);
        }
    }

    #[test]
    fn a_lost_speed_after_a_high_sag_slows_it_more() {
        let mut l = Limiter::new(LimiterSettings::default());
        for k in 0..4 {
            step(&mut l, k as f64 * 0.085, true, Some(0.0245 + 0.05), Some(0.0245));
        }
        let before = l.feed;
        step(&mut l, 0.4, true, None, Some(0.0245));
        assert!(l.feed < before, "{before} {}", l.feed);
    }

    fn heard(cutting: bool, droop: Option<f64>, tooth: f64) -> Frame {
        Frame { tooth_db: Some(tooth), ..frame(cutting, droop) }
    }

    #[test]
    fn in_air_it_speeds_up_to_the_cap_and_drops_back_at_the_first_cut() {
        let mut l = Limiter::new(LimiterSettings { max: 1.2, ..Default::default() });
        let air = Some(0.008);
        let mut t = 0.0;
        // standing: the beat in air
        for _ in 0..10 {
            l.update(t, &heard(false, Some(0.008), -64.0), Some(0.0));
            t += 0.085;
        }
        // moving in air: nothing for 0.3 s, then up at 0.5 a second to 1.2
        let mut fs = vec![];
        for _ in 0..12 {
            let f = heard(true, Some(0.009), -78.0);
            fs.push(l.update(t, &f, f.droop.zip(air).map(|(d, a)| d - a)));
            t += 0.085;
        }
        assert_eq!(fs[2], 1.0);
        assert!(fs[11] > 1.15 && fs[11] <= 1.2 + 1e-12, "{fs:?}");
        // the tool meets the work: the first window back to the programmed feed
        let f = heard(true, Some(0.008 + 0.014), -48.0);
        assert!(l.update(t, &f, Some(0.014)) <= 1.0);
    }

    #[test]
    fn a_light_cut_is_not_air() {
        let mut l = Limiter::new(LimiterSettings { max: 1.2, ..Default::default() });
        for k in 0..10 {
            l.update(k as f64 * 0.085, &heard(false, Some(0.008), -64.0), Some(0.0));
        }
        // C1's beat with almost no sag; and C1's sag with an air-quiet beat: neither speeds up
        for k in 10..40 {
            assert_eq!(l.update(k as f64 * 0.085, &heard(true, Some(0.010), -50.0), Some(0.002)), 1.0);
            assert_eq!(l.update(k as f64 * 0.085, &heard(true, Some(0.022), -75.0), Some(0.014)), 1.0);
        }
        // no beat heard standing yet, or max 1: no speed-up either
        let mut fresh = Limiter::new(LimiterSettings { max: 1.2, ..Default::default() });
        let mut off = Limiter::new(LimiterSettings::default());
        for _ in 0..2 {
            off.update(0.0, &heard(false, Some(0.008), -64.0), Some(0.0));
        }
        for k in 0..30 {
            assert_eq!(fresh.update(k as f64 * 0.085, &heard(true, Some(0.009), -78.0), Some(0.001)), 1.0);
            assert_eq!(off.update(k as f64 * 0.085, &heard(true, Some(0.009), -78.0), Some(0.001)), 1.0);
        }
    }

    #[test]
    fn it_comes_back_up_slowly_once_the_cut_lightens() {
        let mut l = Limiter::new(LimiterSettings::default());
        for k in 0..30 {
            step(&mut l, k as f64 * 0.085, true, Some(0.0245 + 0.05), Some(0.0245));
        }
        let low = l.feed;
        let mut t = 30.0 * 0.085;
        let mut f = low;
        while t < 30.0 * 0.085 + 1.0 {
            f = step(&mut l, t, true, Some(0.0245 + 0.005), Some(0.0245));
            t += 0.085;
        }
        // at most slew_up a second (12 windows: 1.02 s)
        assert!(f > low && f <= low + 0.15 * 12.0 * 0.085 + 1e-9, "{low} {f}");
    }
}
