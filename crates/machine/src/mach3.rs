//! Import a Mach3 profile (the `.xml` next to Mach3.exe) into a machine config.
//!
//! Mach3 writes every setting as its own element under `<profile><Preferences>`,
//! e.g. `<Steps0>200.</Steps0>`, with C-style floats ("200.", "1.e+002").

use std::collections::HashMap;

use crate::config::{Input, MachineConfig};

/// Mach3 input signal numbers: three per axis (++ limit, -- limit, home).
const HOME_INPUT: [usize; 6] = [2, 5, 8, 11, 14, 17];
const PROBE_INPUT: usize = 22;
const ESTOP_INPUT: usize = 25;
/// Mach3 stores user DRO n (the probing screen's settings) as `OEMDRO2_<n - 1255>`.
const USER_DRO_BASE: i64 = 1255;

pub struct Import {
    pub config: MachineConfig,
    /// What was imported and what was left alone, for the user to check.
    pub notes: Vec<String>,
}

/// The `<Preferences>` settings of a Mach3 profile by element name.
pub fn preferences(xml: &str) -> Result<HashMap<String, String>, String> {
    let start = xml.find("<Preferences>").ok_or("not a Mach3 profile: no <Preferences> section")?;
    let end = xml[start..].find("</Preferences>").map(|e| start + e).ok_or("unterminated <Preferences>")?;
    let mut rest = &xml[start + "<Preferences>".len()..end];
    let mut out = HashMap::new();
    while let Some(open) = rest.find('<') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('>') else { break };
        let tag = &rest[..close];
        rest = &rest[close + 1..];
        if tag.starts_with('/') || tag.ends_with('/') {
            continue;
        }
        let end_tag = format!("</{tag}>");
        let Some(e) = rest.find(&end_tag) else { continue };
        out.insert(tag.to_string(), rest[..e].trim().to_string());
        rest = &rest[e + end_tag.len()..];
    }
    Ok(out)
}

pub fn import(xml: &str, base: &MachineConfig) -> Result<Import, String> {
    let p = preferences(xml)?;
    let num = |k: &str| p.get(k).and_then(|v| v.trim().parse::<f64>().ok());
    let flag = |k: &str| num(k).map(|v| v != 0.0);
    let mut c = base.clone();
    let mut notes = Vec::new();

    let inch = num("Units") == Some(1.0);
    let unit = if inch { 25.4 } else { 1.0 };
    if inch {
        notes.push("Profile is in inches; converted to mm".into());
    }

    let input = |n: usize| -> Option<Input> {
        let active = flag(&format!("Input{n}Active"))?;
        let pin = num(&format!("Input{n}Pin"))? as u8;
        (active && pin > 0).then(|| Input::new(pin, flag(&format!("Input{n}Neg")).unwrap_or(false)))
    };

    for i in 0..6 {
        let letter = gcode::AXIS_LETTERS[i];
        let Some(steps) = num(&format!("Steps{i}")) else { continue };
        let a = &mut c.axes[i];
        a.enabled = flag(&format!("Motor{i}Active")).unwrap_or(a.enabled);
        if !a.enabled {
            continue;
        }
        let rotary = i >= 3;
        let scale = if rotary { 1.0 } else { unit };
        a.steps_per_unit = steps / scale;
        if let Some(v) = num(&format!("Vel{i}")) {
            a.max_velocity = v * scale;
        }
        if let Some(v) = num(&format!("Acc{i}")) {
            a.acceleration = v * scale;
        }
        if let (Some(min), Some(max)) = (num(&format!("M{i}Min")), num(&format!("M{i}Max"))) {
            a.soft_min = min * scale;
            a.soft_max = max * scale;
        }
        // Checked against the board's home-direction bits captured from Mach3.
        if let Some(neg) = flag(&format!("M{i}Neg")) {
            a.home_positive = neg;
        }
        if let Some(v) = num(&format!("RefSpeed{i}")) {
            a.home_speed = (v / 100.0).clamp(0.01, 1.0);
        }
        if let Some(v) = num(&format!("M{i}RefHome")) {
            a.home_position = v / a.steps_per_unit;
        }
        a.home_input = input(HOME_INPUT[i]);
        notes.push(format!(
            "{letter}: {} steps/mm, {} mm/s, {} mm/s², limits {}..{}, homes {} to {}",
            a.steps_per_unit, a.max_velocity, a.acceleration, a.soft_min, a.soft_max,
            if a.home_positive { "+" } else { "-" }, a.home_position
        ));
    }

    c.probe = input(PROBE_INPUT);
    c.estop = input(ESTOP_INPUT);
    let pin = |i: Option<Input>| i.map(|i| format!("pin {}{}", i.pin, if i.active_low { " (active low)" } else { "" }));
    notes.push(format!(
        "Probe {}, E-stop {}",
        pin(c.probe).unwrap_or("not set".into()),
        pin(c.estop).unwrap_or("not set".into())
    ));

    if let Some(v) = flag("SoftLimit") {
        c.soft_limits = v;
    }
    if let Some(v) = num("JogSpeed") {
        c.jog_speed = (v / 100.0).clamp(0.001, 1.0);
    }
    if let Some(v) = flag("CVFeedOn") {
        c.constant_velocity = v;
    }
    if let Some(v) = num("SPEED1").filter(|v| *v > 0.0) {
        c.spindle_max_rpm = v;
    }
    if let Some(v) = num("SpinDelayCW") {
        c.spindle_spin_up = v;
    }
    if let Some(v) = num("SpinDownCW") {
        c.spindle_spin_down = v;
    }
    // CW and CCW on the same output means the spindle can't reverse.
    if let (Some(cw), Some(ccw)) = (num("SpinCW"), num("SpinCCW")) {
        c.spindle_reversible = cw != ccw;
    }
    notes.push(format!(
        "Spindle max {} rpm, spin-up {} s, {}",
        c.spindle_max_rpm,
        c.spindle_spin_up,
        if c.spindle_reversible { "reversible" } else { "forward only" }
    ));

    // Probing screen settings (verser macros), when the profile has them.
    let dro = |n: i64| num(&format!("OEMDRO2_{}", n - USER_DRO_BASE)).filter(|v| *v > 0.0);
    let pr = &mut c.probing;
    let mut probe_found = false;
    for (n, field) in [
        (1821, &mut pr.fast_feed),
        (1822, &mut pr.slow_feed),
        (1823, &mut pr.max_distance),
        (1825, &mut pr.latch),
        (1826, &mut pr.xy_clearance),
        (1829, &mut pr.stylus_diameter),
        (1830, &mut pr.z_depth),
    ] {
        if let Some(v) = dro(n) {
            *field = v * unit;
            probe_found = true;
        }
    }
    if probe_found {
        notes.push(format!(
            "Probing: {} / {} mm/min, search {} mm, latch {} mm, stylus {} mm",
            pr.fast_feed, pr.slow_feed, pr.max_distance, pr.latch, pr.stylus_diameter
        ));
    }
    notes.push("Output pins (spindle, coolant, enable) stay as set for the DMC2 board".into());
    Ok(Import { config: c, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Excerpts of Jonas's DMC2.xml, verbatim, concatenated as in the file.
    const DMC2: &str = "<profile><Preferences><Syskey1>1</Syskey1><Profile>DMC2</Profile><Units>0</Units>\
<Steps0>200.</Steps0><Steps1>200.</Steps1><Steps2>200.</Steps2><Steps3>640.</Steps3>\
<Vel0>133.3</Vel0><Vel1>133.3</Vel1><Vel2>100.</Vel2><Acc0>2000.</Acc0><Acc1>2000.</Acc1><Acc2>2000.</Acc2>\
<M0Neg>0</M0Neg><M1Neg>1</M1Neg><M2Neg>1</M2Neg><RefSpeed0>20</RefSpeed0><RefSpeed1>20</RefSpeed1><RefSpeed2>20</RefSpeed2>\
<M0RefHome>-1600</M0RefHome><M1RefHome>102400</M1RefHome><M2RefHome>1600</M2RefHome>\
<M0Max>200.</M0Max><M0Min>-300.</M0Min><M1Max>195.</M1Max><M1Min>0.</M1Min><M2Max>0.</M2Max><M2Min>-130.</M2Min>\
<SoftLimit>0</SoftLimit>\
<Input2Active>1</Input2Active><Input2Port>1</Input2Port><Input2Pin>12</Input2Pin><Input2Neg>0</Input2Neg><Input2Emulated>0</Input2Emulated><Input2EmuKey>0</Input2EmuKey>\
<Input5Active>1</Input5Active><Input5Port>1</Input5Port><Input5Pin>13</Input5Pin><Input5Neg>1</Input5Neg>\
<Input8Active>1</Input8Active><Input8Port>1</Input8Port><Input8Pin>15</Input8Pin><Input8Neg>1</Input8Neg>\
<Input22Active>1</Input22Active><Input22Port>1</Input22Port><Input22Pin>11</Input22Pin><Input22Neg>1</Input22Neg>\
<Input25Active>1</Input25Active><Input25Port>1</Input25Port><Input25Pin>10</Input25Pin><Input25Neg>1</Input25Neg>\
<SpinCW>7</SpinCW><SpinCCW>7</SpinCCW><SPEED1>25000</SPEED1><SpinDelayCW>12</SpinDelayCW><SpinDownCW>1.</SpinDownCW>\
<JogSpeed>35.</JogSpeed><CVFeedOn>0</CVFeedOn>\
<Motor0Active>1</Motor0Active><Motor1Active>1</Motor1Active><Motor2Active>1</Motor2Active><Motor3Active>0</Motor3Active>\
<OEMDRO2_566>500.</OEMDRO2_566><OEMDRO2_567>10.</OEMDRO2_567><OEMDRO2_568>50.</OEMDRO2_568><OEMDRO2_569>0.</OEMDRO2_569>\
<OEMDRO2_570>1.</OEMDRO2_570><OEMDRO2_571>5.</OEMDRO2_571><OEMDRO2_574>2.</OEMDRO2_574><OEMDRO2_575>5.</OEMDRO2_575>\
<DefDir>C:\\Mach3\r\n</DefDir></Preferences><Control></Control></profile>";

    #[test]
    fn dmc2_profile_matches_built_in_config() {
        // Start from something unlike the DMC2 so every value has to come from the file.
        let mut base = MachineConfig::dmc2();
        for a in &mut base.axes {
            a.steps_per_unit = 1.0;
            a.home_input = None;
        }
        base.probe = None;
        base.estop = None;
        base.spindle_reversible = true;
        let imported = import(DMC2, &base).unwrap().config;
        let dmc2 = MachineConfig::dmc2();
        for i in 0..3 {
            let (a, b) = (&imported.axes[i], &dmc2.axes[i]);
            assert_eq!(a.steps_per_unit, b.steps_per_unit, "axis {i}");
            assert_eq!(a.max_velocity, b.max_velocity, "axis {i}");
            assert_eq!(a.home_input, b.home_input, "axis {i}");
            assert_eq!(a.home_positive, b.home_positive, "axis {i}");
            assert_eq!(a.home_position, b.home_position, "axis {i}");
            assert_eq!((a.soft_min, a.soft_max), (b.soft_min, b.soft_max), "axis {i}");
        }
        assert_eq!(imported.probe, dmc2.probe);
        assert_eq!(imported.estop, dmc2.estop);
        assert!(!imported.spindle_reversible);
        assert_eq!(imported.jog_speed, 0.35);
        assert_eq!(imported.probing, dmc2.probing);
    }

    #[test]
    fn rejects_other_files() {
        assert!(import("<html></html>", &MachineConfig::dmc2()).is_err());
    }
}
