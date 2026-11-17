//! Adaptive feed, stage 1: a reduce-only feed limiter on the spindle's sag. While a program cuts with the bog
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

use crate::Frame;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LimiterSettings {
    /// Sag over the no-load sag to hold, share (0.02 = 2%).
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
}

impl Default for LimiterSettings {
    fn default() -> Self {
        LimiterSettings { target: 0.02, min: 0.6, kp: 4.0, ki: 10.0, slew_down: 2.0, slew_up: 0.15, hold_off: 0.5 }
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
}

impl Limiter {
    pub fn new(settings: LimiterSettings) -> Limiter {
        Limiter { settings, feed: 1.0, last_t: None, last_excess: None, engaged_at: None }
    }

    /// Back to the programmed feed (a program starts or ends, an operation where it's off).
    pub fn reset(&mut self) {
        self.feed = 1.0;
        self.last_t = None;
        self.last_excess = None;
        self.engaged_at = None;
    }

    /// A window heard at `t` s, with the no-load sag learned at its speed (the guard's): the feed to ask for.
    pub fn update(&mut self, t: f64, f: &Frame, air: Option<f64>) -> f64 {
        let s = self.settings;
        let dt = self.last_t.map(|l| (t - l).clamp(0.0, 0.5)).unwrap_or(0.0);
        self.last_t = Some(t);
        let moving = f.ctx.rpm > 0.0 && f.ctx.cutting;
        let excess = match (f.droop, air) {
            (Some(d), Some(a)) if moving => Some(d - a),
            // the speed lost in a cut right after a high sag: it ran away past what the analysis searches
            (None, Some(_)) if moving && self.last_excess.is_some_and(|e| e > 2.0 * s.target) => self.last_excess.map(|e| e.max(3.0 * s.target)),
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

    #[test]
    fn a_light_cut_keeps_the_programmed_feed() {
        let mut l = Limiter::new(LimiterSettings::default());
        for k in 0..100 {
            assert_eq!(l.update(k as f64 * 0.085, &frame(true, Some(0.0245 + 0.012)), Some(0.0245)), 1.0);
        }
    }

    #[test]
    fn a_heavy_cut_is_slowed_down_but_not_under_the_floor() {
        let mut l = Limiter::new(LimiterSettings::default());
        let mut f = 1.0;
        for k in 0..100 {
            f = l.update(k as f64 * 0.085, &frame(true, Some(0.0245 + 0.06)), Some(0.0245));
        }
        assert_eq!(f, 0.6);
    }

    #[test]
    fn no_signal_keeps_the_feed_and_never_raises_it() {
        let mut l = Limiter::new(LimiterSettings::default());
        for k in 0..10 {
            l.update(k as f64 * 0.085, &frame(true, Some(0.0245 + 0.04)), Some(0.0245));
        }
        let low = l.feed;
        assert!(low < 1.0);
        // no learned no-load sag, a lost speed after a modest sag, the spindle stopped: it stays
        for k in 10..60 {
            let t = k as f64 * 0.085;
            assert!(l.update(t, &frame(true, Some(0.0)), None) <= low + 1e-12);
            assert!(l.update(t, &frame(false, Some(0.0)), Some(0.0245)) <= low + 1e-12);
        }
    }

    #[test]
    fn a_lost_speed_after_a_high_sag_slows_it_more() {
        let mut l = Limiter::new(LimiterSettings::default());
        for k in 0..4 {
            l.update(k as f64 * 0.085, &frame(true, Some(0.0245 + 0.05)), Some(0.0245));
        }
        let before = l.feed;
        l.update(0.4, &frame(true, None), Some(0.0245));
        assert!(l.feed < before, "{before} {}", l.feed);
    }

    #[test]
    fn it_comes_back_up_slowly_once_the_cut_lightens() {
        let mut l = Limiter::new(LimiterSettings::default());
        for k in 0..30 {
            l.update(k as f64 * 0.085, &frame(true, Some(0.0245 + 0.05)), Some(0.0245));
        }
        let low = l.feed;
        let mut t = 30.0 * 0.085;
        let mut f = low;
        while t < 30.0 * 0.085 + 1.0 {
            f = l.update(t, &frame(true, Some(0.0245 + 0.005)), Some(0.0245));
            t += 0.085;
        }
        // at most slew_up a second
        assert!(f > low && f <= low + 0.15 + 1e-9, "{low} {f}");
    }
}
