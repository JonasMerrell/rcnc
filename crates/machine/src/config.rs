//! Machine configuration: axes, inputs, homing, spindle.

use gcode::Axes;
use serde::{Deserialize, Serialize};

/// A board input pin and the level that means "active".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    pub pin: u8,
    pub active_low: bool,
}

impl Input {
    pub const fn new(pin: u8, active_low: bool) -> Self {
        Input { pin, active_low }
    }

    /// Whether this input is active given the board's input bitmap (bit n = pin n).
    pub fn is_active(&self, inputs: u32) -> bool {
        let high = inputs & (1 << self.pin) != 0;
        high != self.active_low
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxisConfig {
    pub enabled: bool,
    pub steps_per_unit: f64,
    /// mm/s (deg/s for rotary axes).
    pub max_velocity: f64,
    /// mm/s².
    pub acceleration: f64,
    pub soft_min: f64,
    pub soft_max: f64,
    pub home_input: Option<Input>,
    /// Home switch is in the positive direction.
    pub home_positive: bool,
    /// Homing seek speed as a fraction of `max_velocity`.
    pub home_speed: f64,
    /// Machine position set after homing, in mm.
    pub home_position: f64,
}

impl AxisConfig {
    pub fn disabled() -> Self {
        AxisConfig {
            enabled: false,
            steps_per_unit: 1.0,
            max_velocity: 1.0,
            acceleration: 1.0,
            soft_min: -1e9,
            soft_max: 1e9,
            home_input: None,
            home_positive: false,
            home_speed: 0.2,
            home_position: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MachineConfig {
    pub name: String,
    pub axes: [AxisConfig; 6],
    pub probe: Option<Input>,
    pub estop: Option<Input>,
    /// Axis indexes in homing order.
    pub home_order: Vec<usize>,
    /// Distance moved off the switch after it releases, in mm.
    pub home_clearance: f64,
    pub soft_limits: bool,
    /// Constant velocity: blend corners. Off means exact stop at each G-code move (Mach3 "CV off").
    pub constant_velocity: bool,
    /// Allowed path deviation at blended corners, mm.
    pub junction_deviation: f64,
    /// Arc linearisation tolerance, mm.
    pub arc_tolerance: f64,
    pub jog_speed: f64,
    pub spindle_max_rpm: f64,
    pub spindle_spin_up: f64,
    pub spindle_spin_down: f64,
    /// The spindle can run counter-clockwise (M4, G84 tapping). The DMC2's
    /// VFD has one run output, so M4 would silently run it clockwise.
    pub spindle_reversible: bool,
    pub probing: crate::probing::ProbeSettings,
    /// At a tool change (M6), measure the new tool on the tool setter.
    pub tool_change_touch_off: bool,
    pub tool_change: ToolChangeSettings,
}

/// Guided tool change at M6.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolChangeSettings {
    /// Stop the spindle, move to the change position, wait for the new tool,
    /// measure it and go back. Off: just pause at M6, like Mach3.
    pub guided: bool,
    /// Machine X/Y to change tools at.
    pub x: f64,
    pub y: f64,
    /// Lowest machine Z to travel and change tools at; Z is raised to it, never lowered.
    pub z: f64,
    /// Go back to where the tool change started, with the spindle and coolant as they were.
    pub return_to_start: bool,
}

impl Default for ToolChangeSettings {
    fn default() -> Self {
        // Next to the tool setter, at the top of Z.
        let setter = crate::probing::ToolSetter::default();
        ToolChangeSettings { guided: true, x: setter.x, y: setter.y, z: 0.0, return_to_start: true }
    }
}

impl Default for MachineConfig {
    fn default() -> Self {
        MachineConfig::dmc2()
    }
}

impl MachineConfig {
    /// Shariff DMC2, as configured in Jonas's Mach3 profile (DMC2.xml) and
    /// confirmed by USB capture.
    pub fn dmc2() -> Self {
        let linear = |velocity: f64,
                      home: Input,
                      home_positive: bool,
                      home_position: f64,
                      min: f64,
                      max: f64| AxisConfig {
            enabled: true,
            steps_per_unit: 200.0,
            max_velocity: velocity,
            acceleration: 2000.0,
            soft_min: min,
            soft_max: max,
            home_input: Some(home),
            home_positive,
            home_speed: 0.2,
            home_position,
        };
        let rotary_unused = || AxisConfig {
            steps_per_unit: 640.0,
            max_velocity: 25.0,
            acceleration: 120.0,
            soft_min: -100.0,
            soft_max: 100.0,
            ..AxisConfig::disabled()
        };
        MachineConfig {
            name: "Shariff DMC2".into(),
            axes: [
                linear(133.3, Input::new(12, false), false, -8.0, -300.0, 200.0),
                // Home position from the capture (102400 steps); check against the Mach3 DRO.
                // Mach3's soft limits (Y 0..195, Z -130..0) leave out its own home
                // positions and the tool setter. They're kept as Mach3 sends them to
                // the board, but rcnc won't turn soft limits on until they're fixed
                // (see `validate`): measure the real travel first.
                linear(133.3, Input::new(13, true), true, 512.0, 0.0, 195.0),
                linear(100.0, Input::new(15, true), true, 8.0, -130.0, 0.0),
                // A and B are disabled, but the profile keeps settings for them.
                rotary_unused(),
                rotary_unused(),
                AxisConfig::disabled(),
            ],
            probe: Some(Input::new(11, true)),
            estop: Some(Input::new(10, true)),
            home_order: vec![2, 1, 0],
            home_clearance: 1.0,
            soft_limits: false,
            constant_velocity: false,
            junction_deviation: 0.01,
            arc_tolerance: 0.002,
            jog_speed: 0.35,
            spindle_max_rpm: 25000.0,
            spindle_spin_up: 12.0,
            spindle_spin_down: 1.0,
            spindle_reversible: false,
            probing: Default::default(),
            tool_change_touch_off: true,
            tool_change: Default::default(),
        }
    }

    pub fn to_steps(&self, pos: &Axes) -> [i64; 6] {
        std::array::from_fn(|i| (pos[i] * self.axes[i].steps_per_unit).round() as i64)
    }

    pub fn from_steps(&self, steps: &[i64; 6]) -> Axes {
        std::array::from_fn(|i| steps[i] as f64 / self.axes[i].steps_per_unit)
    }

    /// Check settings that would otherwise divide by zero, give NaN positions
    /// or make soft limits block homing and tool changes.
    pub fn validate(&self) -> Result<(), String> {
        let positive = |name: &str, v: f64| {
            if v.is_finite() && v > 0.0 {
                Ok(())
            } else {
                Err(format!("{name} must be more than 0"))
            }
        };
        for (i, a) in self.axes.iter().enumerate().filter(|(_, a)| a.enabled) {
            let l = gcode::AXIS_LETTERS[i];
            positive(&format!("{l} steps/mm"), a.steps_per_unit)?;
            positive(&format!("{l} max speed"), a.max_velocity)?;
            positive(&format!("{l} acceleration"), a.acceleration)?;
            positive(&format!("{l} home speed"), a.home_speed)?;
            if !(a.soft_min < a.soft_max) {
                return Err(format!("{l} soft min must be below its max"));
            }
            if self.soft_limits && a.home_input.is_some() && !(a.soft_min - 1e-6..=a.soft_max + 1e-6).contains(&a.home_position) {
                return Err(format!(
                    "{l} home position {} is outside its soft limits {}..{}",
                    a.home_position, a.soft_min, a.soft_max
                ));
            }
        }
        positive("arc tolerance", self.arc_tolerance)?;
        positive("max spindle speed", self.spindle_max_rpm)?;
        if !(self.junction_deviation >= 0.0) {
            return Err("corner tolerance can't be negative".into());
        }
        if !(self.spindle_spin_up >= 0.0 && self.spindle_spin_down >= 0.0) {
            return Err("spindle waits can't be negative".into());
        }
        if !(self.jog_speed > 0.0 && self.jog_speed <= 1.0) {
            return Err("jog speed must be between 0 and 100%".into());
        }
        let p = &self.probing;
        for (name, v) in [("probe search feed", p.fast_feed), ("probe latch feed", p.slow_feed), ("probe travel feed", p.travel_feed)] {
            positive(name, v)?;
        }
        if self.soft_limits {
            let inside = |name: &str, pos: [f64; 3]| {
                match (0..3).find(|&i| {
                    let a = &self.axes[i];
                    a.enabled && (pos[i] < a.soft_min - 1e-6 || pos[i] > a.soft_max + 1e-6)
                }) {
                    Some(i) => Err(format!("the {name} ({} {}) is outside the soft limits", gcode::AXIS_LETTERS[i], pos[i])),
                    None => Ok(()),
                }
            };
            let t = &self.tool_change;
            if t.guided {
                let z = t.z.max(self.axes[2].soft_min);
                inside("tool change position", [t.x, t.y, z])?;
            }
            let st = &p.setter;
            inside("tool setter", [st.x, st.y, st.travel_z.min(self.axes[2].soft_max)])?;
        }
        Ok(())
    }

    /// First soft-limit violation for a machine position, if limits are on.
    pub fn soft_limit_violation(&self, pos: &Axes) -> Option<usize> {
        if !self.soft_limits {
            return None;
        }
        (0..6).find(|&i| {
            let a = &self.axes[i];
            a.enabled && (pos[i] < a.soft_min - 1e-6 || pos[i] > a.soft_max + 1e-6)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_levels() {
        let low = Input::new(11, true);
        assert!(low.is_active(0));
        assert!(!low.is_active(1 << 11));
        let high = Input::new(12, false);
        assert!(high.is_active(1 << 12));
    }

    #[test]
    fn settings_are_checked() {
        let mut c = MachineConfig::dmc2();
        c.validate().unwrap();
        c.arc_tolerance = 0.0;
        assert!(c.validate().is_err());
        // Mach3's DMC2 limits leave out the home positions: soft limits can't go on like that.
        let mut c = MachineConfig::dmc2();
        c.soft_limits = true;
        assert!(c.validate().unwrap_err().contains("home position"));
        c.axes[1].soft_max = 512.0;
        c.axes[2].soft_max = 8.0;
        c.validate().unwrap();
    }

    #[test]
    fn step_conversion_round_trips() {
        let c = MachineConfig::dmc2();
        let steps = c.to_steps(&[1.0, -2.5, 0.005, 0.0, 0.0, 0.0]);
        assert_eq!(&steps[..3], &[200, -500, 1]);
        assert_eq!(c.from_steps(&steps)[1], -2.5);
    }
}
