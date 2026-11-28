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
    /// After the fast seek finds the switch, back off this far (mm) and
    /// come back slowly to find it again precisely.
    pub home_pulloff: f64,
    /// Speed of that slow second approach, as a fraction of the seek speed.
    pub home_latch_speed: f64,
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
    pub wash_down: WashDownSettings,
}

/// Coolant wash-down: with the spindle stopped, turn the flood coolant on
/// and sweep the nozzle over the vise and table so the chips are washed off,
/// then turn it off again. The nozzle rides on the spindle head, so the
/// spindle sweeps the area at a safe height, in machine coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WashDownSettings {
    /// Sweep the saved vise location's outline (`vise_size` about the fixed
    /// centre block) plus `margin`. Off: sweep `x` and `y`.
    pub around_vise: bool,
    /// The whole vise's X and Y size, centred on its fixed centre block.
    pub vise_size: [f64; 2],
    /// Sweep this far past the vise's outline.
    pub margin: f64,
    /// The area to sweep when not around the vise: machine X and Y, low to high.
    pub x: [f64; 2],
    pub y: [f64; 2],
    /// Lowest machine Z to sweep at: Z is raised to it, never lowered.
    pub z: f64,
    /// Between the sweep's rows.
    pub spacing: f64,
    /// Sweep feed, mm/min: slower soaks every spot for longer.
    pub feed: f64,
    /// How many times the area is swept; every other pass runs crosswise.
    pub passes: u32,
    /// Seconds of coolant before the first move, so it is flowing.
    pub soak: f64,
}

impl Default for WashDownSettings {
    fn default() -> Self {
        // The two-station vise from vise.step: 317.7 x 144.5, centred on its centre block.
        WashDownSettings {
            around_vise: true,
            vise_size: [317.7, 144.5],
            margin: 15.0,
            x: [0.0, 0.0],
            y: [0.0, 0.0],
            z: 0.0,
            spacing: 30.0,
            feed: 3000.0,
            passes: 2,
            soak: 2.0,
        }
    }
}

/// Guided tool change at M6.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolChangeSettings {
    /// Stop the spindle, raise Z, move the bed back, wait for the new tool,
    /// measure it and go back. Off: just pause at M6, like Mach3.
    pub guided: bool,
    /// Machine X to change tools at, used only with `move_x`.
    pub x: f64,
    /// Machine Y to change tools at, so the spindle is easy to reach. None:
    /// the tool setter's Y (see `change_y`).
    pub y: Option<f64>,
    /// Also move X to `x`. Off: X stays where it is and only the bed moves.
    pub move_x: bool,
    /// Move X to the middle of its travel (`x_travel` from its home end);
    /// takes the place of `x` and `move_x`.
    pub centre_x: bool,
    /// How far X travels from its home switch, mm.
    pub x_travel: f64,
    /// Lowest machine Z to travel and change tools at; Z is raised to it, never lowered.
    pub z: f64,
    /// Go back to where the tool change started, with the spindle as it was. The coolant stays off: the
    /// program's next M7/M8 turns it on again.
    pub return_to_start: bool,
    /// Run the flood coolant over the tool setter for the fast search down onto it, to wash chips off it;
    /// off for the two slow touches. Never with a touch probe in the spindle.
    pub setter_coolant: bool,
    /// Seconds to wait after the coolant goes off before the slow touches, so the spray stops and the setter
    /// drains.
    pub setter_drain: f64,
    /// Machine Z of the collet nut's face when a tool's tip touches the setter, less the tool's stick-out:
    /// with it, measuring a tool gives its stick-out. None until calibrated with a tool whose stick-out
    /// is known (Tools page).
    pub nut_face_z: Option<f64>,
    /// How far the measured stick-out may be from the program's before the tool change stops, mm.
    pub stickout_tolerance: f64,
}

impl Default for ToolChangeSettings {
    fn default() -> Self {
        // Top of Z, the bed at the tool setter's Y.
        let setter = crate::probing::ToolSetter::default();
        // X travel 343 mm: the DMC2's from the CAD (spindle x -123.7 to 219.3). Its
        // configured soft limits (Mach3's) don't contain the X home position, so
        // they can't give the middle.
        ToolChangeSettings {
            guided: true,
            x: setter.x,
            y: None,
            move_x: false,
            centre_x: true,
            x_travel: 343.0,
            z: 0.0,
            return_to_start: true,
            setter_coolant: true,
            setter_drain: 1.0,
            nut_face_z: None,
            stickout_tolerance: 1.5,
        }
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
            home_pulloff: 2.0,
            home_latch_speed: 0.1,
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
            wash_down: Default::default(),
        }
    }

    /// The machine X and Y area the wash-down sweeps (low to high), kept
    /// inside the travel: never past a home switch, X within its travel from
    /// home, and inside the soft limits when they're on.
    pub fn wash_area(&self) -> Result<([f64; 2], [f64; 2]), String> {
        let w = &self.wash_down;
        let (mut x, mut y) = if w.around_vise {
            let Some(v) = &self.probing.vise.location else {
                return Err("it sweeps around the saved vise location, and there is none: run Find vise location (Probe page), or set an area by hand (Tools page)".into());
            };
            let cx = (v.faces[0] + v.faces[1]) / 2.0;
            let (hx, hy) = (w.vise_size[0].abs() / 2.0 + w.margin, w.vise_size[1].abs() / 2.0 + w.margin);
            ([cx - hx, cx + hx], [v.y - hy, v.y + hy])
        } else {
            ([w.x[0].min(w.x[1]), w.x[0].max(w.x[1])], [w.y[0].min(w.y[1]), w.y[0].max(w.y[1])])
        };
        for (i, r) in [&mut x, &mut y].into_iter().enumerate() {
            let a = &self.axes[i];
            let (mut lo, mut hi) = if self.soft_limits { (a.soft_min, a.soft_max) } else { (f64::NEG_INFINITY, f64::INFINITY) };
            if a.home_positive {
                hi = hi.min(a.home_position);
            } else {
                lo = lo.max(a.home_position);
            }
            if i == 0 && self.tool_change.x_travel.abs() > 0.0 {
                let t = self.tool_change.x_travel.abs();
                if a.home_positive {
                    lo = lo.max(a.home_position - t);
                } else {
                    hi = hi.min(a.home_position + t);
                }
            }
            *r = [r[0].clamp(lo, hi), r[1].clamp(lo, hi)];
        }
        if !(x[1] - x[0] > 1.0 && y[1] - y[0] > 1.0) || ![x[0], x[1], y[0], y[1]].iter().all(|v| v.is_finite()) {
            return Err(format!(
                "the area to sweep, X{:.1} to X{:.1} and Y{:.1} to Y{:.1}, is empty once kept inside the machine's travel (Tools page)",
                x[0], x[1], y[0], y[1]
            ));
        }
        Ok((x, y))
    }

    /// Machine X the tool change moves to, if it moves X: by default the
    /// middle of X's travel, so the spindle is easy to reach.
    pub fn change_x(&self) -> Option<f64> {
        let t = &self.tool_change;
        let x = &self.axes[0];
        let want = if t.centre_x {
            let half = t.x_travel.abs() / 2.0;
            if x.home_positive { x.home_position - half } else { x.home_position + half }
        } else if t.move_x {
            t.x
        } else {
            return None;
        };
        // Never past the configured soft limits, nor beyond the home switch: a
        // wrong travel or sign must not drive X into its end.
        let (mut lo, mut hi) = (x.soft_min.min(x.soft_max), x.soft_max.max(x.soft_min));
        if x.home_positive {
            hi = hi.min(x.home_position);
        } else {
            lo = lo.max(x.home_position);
        }
        Some(if lo <= hi { want.clamp(lo, hi) } else { x.home_position })
    }

    /// Machine Y the tool change moves the bed to: by default the tool
    /// setter's Y (Jonas: that puts the bed where the spindle is easy to reach).
    pub fn change_y(&self) -> f64 {
        self.tool_change.y.unwrap_or(self.probing.setter.y)
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
        positive("home pull-off", self.home_pulloff)?;
        if !(self.home_latch_speed > 0.0 && self.home_latch_speed <= 1.0) {
            return Err("slow homing speed must be between 0 and 100% of the seek speed".into());
        }
        positive("max spindle speed", self.spindle_max_rpm)?;
        positive("wash-down row spacing", self.wash_down.spacing)?;
        positive("wash-down feed", self.wash_down.feed)?;
        if !(self.wash_down.soak >= 0.0 && self.wash_down.margin.is_finite()) {
            return Err("the wash-down soak can't be negative".into());
        }
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
        for (name, v) in [
            ("probe search feed", p.fast_feed),
            ("probe latch feed", p.slow_feed),
            ("probe travel feed", p.travel_feed),
            ("tool setter repeat tolerance", p.setter.repeat_tolerance),
        ] {
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
                // Without move_x, X stays wherever it is.
                let x = self.change_x().unwrap_or(self.axes[0].soft_min);
                inside("tool change position", [x, self.change_y(), z])?;
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
