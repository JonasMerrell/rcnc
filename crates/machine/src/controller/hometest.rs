//! Homing repeatability test: home one axis, move it away, home it again,
//! many times. Just before each home resets the position, the board's step
//! count says where the axis was by the previous home; the difference to the
//! home position is how far this home's switch release landed from the last
//! one (switch scatter plus any lost steps). Also records the switch's
//! hysteresis (trigger to release distance on the slow approach) each time.

use super::{Controller, Mode};
use gcode::AXIS_LETTERS;

/// A homing test in progress or finished, for the UI.
#[derive(Debug, Clone, PartialEq)]
pub struct HomeTestView {
    pub axis: usize,
    /// Homes to record.
    pub cycles: usize,
    /// How far the axis moves away from home between homes, mm.
    pub away: f64,
    /// Each re-home's shift from the previous home, mm (counted position minus
    /// home position just before the reset).
    pub shifts: Vec<f64>,
    /// Trigger to release distance on the slow approach, mm, each home.
    pub hysteresis: Vec<f64>,
    pub running: bool,
    /// Where the results were written, once finished.
    pub saved: Option<String>,
}

/// Mean, standard deviation (sample) and range (max - min) of `v`.
pub fn stats(v: &[f64]) -> Option<(f64, f64, f64)> {
    if v.is_empty() {
        return None;
    }
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    let sd = if v.len() > 1 { (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt() } else { 0.0 };
    let lo = v.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    Some((mean, sd, hi - lo))
}

impl HomeTestView {
    pub fn csv(&self) -> String {
        let mut s = format!("# rcnc homing test, axis {}, away {} mm\ncycle,shift_mm,hysteresis_mm\n", AXIS_LETTERS[self.axis], self.away);
        for (i, d) in self.shifts.iter().enumerate() {
            let h = self.hysteresis.get(i + 1).map(|h| format!("{h:.4}")).unwrap_or_default();
            s += &format!("{},{d:.4},{h}\n", i + 1);
        }
        s
    }

    pub fn summary(&self) -> String {
        let axis = AXIS_LETTERS[self.axis];
        match (stats(&self.shifts), stats(&self.hysteresis)) {
            (Some((m, sd, r)), h) => {
                let mut t = format!(
                    "{axis} homing test, {} re-homes: shift mean {:+.1} µm, std dev {:.1} µm, range {:.1} µm",
                    self.shifts.len(),
                    m * 1000.0,
                    sd * 1000.0,
                    r * 1000.0
                );
                if let Some((hm, hsd, _)) = h {
                    t += &format!("; switch hysteresis {:.3} mm ± {:.1} µm", hm, hsd * 1000.0);
                }
                t
            }
            (None, _) => format!("{axis} homing test: no re-homes recorded"),
        }
    }
}

impl Controller {
    /// Start a homing test on `axis`: `cycles` re-homes, moving `away` mm off
    /// the switch between them. An axis that isn't homed yet is homed once
    /// first (not recorded).
    pub(super) fn start_home_test(&mut self, axis: usize, cycles: usize, away: f64) {
        let Some(a) = self.config.axes.get(axis).filter(|a| a.enabled && a.home_input.is_some()) else {
            return self.info("Homing test: that axis has no home switch");
        };
        let span = super::travel_steps(a) as f64 / a.steps_per_unit;
        if !(away > 0.0) || away > span - 5.0 {
            return self.info(format!("Homing test: move away between 0 and {:.0} mm", span - 5.0));
        }
        if cycles == 0 {
            return self.info("Homing test: at least one cycle");
        }
        self.start_homing(Some(axis));
        if self.mode != Mode::Homing {
            return;
        }
        self.home_test = Some(HomeTestView { axis, cycles, away, shifts: Vec::new(), hysteresis: Vec::new(), running: true, saved: None });
        self.info(format!("Homing test: {} × {}, {} mm away each time", cycles, AXIS_LETTERS[axis], away));
    }

    /// The test's axis finished a home (`shift`: None when it wasn't homed
    /// before). Returns true when the test wants another cycle.
    pub(super) fn home_test_record(&mut self, axis: usize, shift: Option<f64>, hysteresis: f64) -> bool {
        let Some(t) = self.home_test.as_mut().filter(|t| t.running && t.axis == axis) else {
            return false;
        };
        t.hysteresis.push(hysteresis);
        if let Some(d) = shift {
            t.shifts.push(d);
        }
        if t.shifts.len() < t.cycles {
            return true;
        }
        t.running = false;
        let summary = t.summary();
        let csv = t.csv();
        let name = format!("homing-test-{}-{}.csv", AXIS_LETTERS[axis], chrono_stamp());
        if let Some(p) = &self.persist {
            let path = p.dir.join(&name);
            match std::fs::write(&path, csv) {
                Ok(()) => t.saved = Some(path.display().to_string()),
                Err(e) => self.info(format!("Homing test: couldn't save {name}: {e}")),
            }
        }
        self.info(summary);
        false
    }
}

/// Seconds since the Unix epoch: enough to keep test files apart.
fn chrono_stamp() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}
