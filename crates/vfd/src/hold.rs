//! Feed hold on the VFD's output current: when the drive puts out more than a limit, for longer than a time,
//! while cutting at speed, say so (the app holds the feed; the spindle stays on). It sits beside the
//! microphone's bog guard: current rises with the cut's torque before the speed sags, and it needs no sound.
//!
//! The DMC2 (2026-11-18 ladders, F004 = 150): no-load current 2.7 A at 9k; 1049 W (cut model) drew 8.6 A,
//! about the rated current (F142 11 A). The default limit, 8.5 A, sits at the never-exceed ~950 W.

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AmpsHoldSettings {
    /// Off until the current register is checked against the drive's display.
    pub on: bool,
    /// Output current over this, A.
    pub limit: f64,
    /// For at least this long, s.
    pub time: f64,
    /// After the drive reaches its set frequency, wait this long before judging, s.
    pub settle: f64,
}

impl Default for AmpsHoldSettings {
    fn default() -> Self {
        AmpsHoldSettings { on: false, limit: 8.5, time: 0.3, settle: 1.0 }
    }
}

#[derive(Debug, Default)]
pub struct AmpsHold {
    pub settings: AmpsHoldSettings,
    at_speed_since: Option<f64>,
    over_since: Option<f64>,
    tripped: bool,
}

impl AmpsHold {
    pub fn new(settings: AmpsHoldSettings) -> AmpsHold {
        AmpsHold { settings, ..Default::default() }
    }

    /// A reading at `t` s: Some(why) when the feed should be held now (once per overload: it re-arms when
    /// the current is back under the limit). `cutting`: a program is running and the machine moving.
    pub fn update(&mut self, t: f64, r: &crate::Reading, cutting: bool) -> Option<String> {
        if !r.at_speed() {
            // stopped, accelerating or braking: that current isn't the cut's
            self.at_speed_since = None;
            self.over_since = None;
            return None;
        }
        let since = *self.at_speed_since.get_or_insert(t);
        let Some(amps) = r.amps else {
            self.over_since = None;
            return None;
        };
        if amps <= self.settings.limit {
            self.over_since = None;
            self.tripped = false;
            return None;
        }
        if !self.settings.on || !cutting || t - since < self.settings.settle {
            self.over_since = None;
            return None;
        }
        let over = *self.over_since.get_or_insert(t);
        if t - over >= self.settings.time && !self.tripped {
            self.tripped = true;
            return Some(format!(
                "Feed held: the spindle drew {amps:.1} A for {:.1} s (limit {:.1} A, Settings -> Spindle VFD). The spindle is still on; Cycle Start carries on.",
                t - over,
                self.settings.limit
            ));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Reading;

    fn at(amps: f64, hz: f64) -> Reading {
        Reading { out_hz: Some(hz), set_hz: Some(150.0), amps: Some(amps), ..Default::default() }
    }

    #[test]
    fn holds_once_on_a_sustained_overload_while_cutting() {
        let mut h = AmpsHold::new(AmpsHoldSettings { on: true, ..Default::default() });
        // accelerating: 10 A means nothing
        assert!(h.update(0.0, &at(10.0, 80.0), true).is_none());
        // at speed, settling
        assert!(h.update(0.5, &at(9.0, 150.0), true).is_none());
        assert!(h.update(1.2, &at(3.0, 150.0), true).is_none());
        // a short spike: no hold
        assert!(h.update(2.0, &at(9.0, 150.0), true).is_none());
        assert!(h.update(2.1, &at(5.0, 150.0), true).is_none());
        // sustained
        assert!(h.update(3.0, &at(9.0, 150.0), true).is_none());
        assert!(h.update(3.2, &at(9.2, 149.0), true).is_none());
        assert!(h.update(3.31, &at(9.2, 149.0), true).unwrap().contains("9.2 A"));
        // once per overload
        assert!(h.update(3.5, &at(9.4, 149.0), true).is_none());
        // re-armed once back under
        assert!(h.update(4.0, &at(4.0, 150.0), true).is_none());
        assert!(h.update(5.0, &at(9.0, 150.0), true).is_none());
        assert!(h.update(5.4, &at(9.0, 150.0), true).is_some());
    }

    #[test]
    fn never_holds_when_off_not_cutting_or_without_a_current() {
        let mut off = AmpsHold::new(AmpsHoldSettings::default());
        let mut idle = AmpsHold::new(AmpsHoldSettings { on: true, ..Default::default() });
        for i in 0..50 {
            let t = i as f64 * 0.1;
            assert!(off.update(t, &at(10.0, 150.0), true).is_none());
            assert!(idle.update(t, &at(10.0, 150.0), false).is_none());
            let no_amps = Reading { amps: None, ..at(0.0, 150.0) };
            assert!(idle.update(t, &no_amps, true).is_none());
        }
    }
}
