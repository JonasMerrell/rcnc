//! The spindle VFD's own readings over RS-485 (a USB-RS485 adapter on the PC wired to the VFD's 485+/485-):
//! the output frequency the drive is putting out, the frequency it is set to, the output current and voltage.
//! For the DMC2's H100-2.2C2-1B (Modbus RTU; the same protocol grblHAL's and FluidNC's H100 drivers use).
//!
//! Read-only: rcnc keeps running the spindle through the Mach3 board as before (the VFD's run and frequency
//! sources stay where they are). Only the two Modbus read requests are ever sent (see `modbus`).
//!
//! What is known about the H100's registers (and from where):
//! - holding registers (0x03) are the F parameters, register N = FN (grblHAL h100.c reads F143, F011, F005;
//!   FluidNC reads F011, F005);
//! - input register 0 is the output frequency in 0.1 Hz and 1 the set frequency (FluidNC's tested H100 notes:
//!   "[01] [04] [0000] [0002] ... First data seems running freq, second data set freq").
//! - The rest of the input registers (current, speed, DC bus, output voltage, temperature) are assumed to
//!   follow the order of the drive's own display (output frequency, set frequency, output current, speed, DC
//!   voltage, AC voltage, temperature): unverified, so the map is a setting (`RegisterMap`) and the Settings
//!   page shows every raw register next to what it's taken as, to check against the drive's display once.

pub mod hold;
pub mod modbus;
#[cfg(feature = "serial")]
pub mod serial;
pub mod link;

pub use hold::{AmpsHold, AmpsHoldSettings};
pub use link::{Line, Parity, Port, Shared, Status, VfdLink, VfdSettings};

/// One register and its scale (value = raw × scale).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Reg {
    pub index: u16,
    pub scale: f64,
}

const fn reg(index: u16, scale: f64) -> Option<Reg> {
    Some(Reg { index, scale })
}

/// Which input register is which. None: not read from the drive.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RegisterMap {
    /// Output frequency, Hz (confirmed: register 0, 0.1 Hz).
    pub out_hz: Option<Reg>,
    /// Set (target) frequency, Hz (confirmed: register 1, 0.1 Hz).
    pub set_hz: Option<Reg>,
    /// Output current, A (assumed register 2, 0.1 A: check against the display).
    pub amps: Option<Reg>,
    /// Speed as the drive shows it, rpm (assumed register 3; it is the frequency scaled by F144, not a
    /// measured speed: the H100 has no encoder).
    pub rpm: Option<Reg>,
    /// DC bus voltage, V (assumed register 4).
    pub dc_volts: Option<Reg>,
    /// Output (AC) voltage, V (assumed register 5).
    pub ac_volts: Option<Reg>,
    /// Heat sink temperature, °C (assumed register 6, 0.1 °C).
    pub temp_c: Option<Reg>,
}

impl Default for RegisterMap {
    fn default() -> Self {
        RegisterMap {
            out_hz: reg(0, 0.1),
            set_hz: reg(1, 0.1),
            amps: reg(2, 0.1),
            rpm: reg(3, 1.0),
            dc_volts: reg(4, 1.0),
            ac_volts: reg(5, 1.0),
            temp_c: reg(6, 0.1),
        }
    }
}

impl RegisterMap {
    /// How many input registers from 0 to read to cover the map (at least the two confirmed ones).
    pub fn count(&self) -> u16 {
        [self.out_hz, self.set_hz, self.amps, self.rpm, self.dc_volts, self.ac_volts, self.temp_c]
            .iter()
            .flatten()
            .map(|r| r.index + 1)
            .max()
            .unwrap_or(2)
            .clamp(2, 32)
    }

    pub fn decode(&self, regs: &[u16]) -> Reading {
        let get = |r: Option<Reg>| r.and_then(|r| regs.get(r.index as usize).map(|v| *v as f64 * r.scale));
        Reading {
            out_hz: get(self.out_hz),
            set_hz: get(self.set_hz),
            amps: get(self.amps),
            rpm: get(self.rpm),
            dc_volts: get(self.dc_volts),
            ac_volts: get(self.ac_volts),
            temp_c: get(self.temp_c),
        }
    }
}

/// One poll of the drive, decoded.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize)]
pub struct Reading {
    pub out_hz: Option<f64>,
    pub set_hz: Option<f64>,
    pub amps: Option<f64>,
    pub rpm: Option<f64>,
    pub dc_volts: Option<f64>,
    pub ac_volts: Option<f64>,
    pub temp_c: Option<f64>,
}

impl Reading {
    /// Spindle speed from the output frequency and the motor's poles (F143): 120 f / poles. The DMC2's
    /// spindle is 2-pole, so 400 Hz = 24,000 rpm. It is the field's speed: the rotor runs a little under it
    /// (slip), more under load.
    pub fn rpm_from_hz(&self, poles: f64) -> Option<f64> {
        Some(self.out_hz? * 120.0 / poles.max(2.0))
    }

    /// Whether the drive has reached its set frequency (within 2%, and running): its acceleration current
    /// isn't load.
    pub fn at_speed(&self) -> bool {
        match (self.out_hz, self.set_hz) {
            (Some(o), Some(s)) => s > 1.0 && (o - s).abs() <= 0.02 * s,
            _ => false,
        }
    }

    /// Apparent output power, VA (√3 · V · A on the three phases). Not the shaft power: the power factor and
    /// the motor's losses aren't known here; logged for calibrating against the cutting model.
    pub fn apparent_va(&self) -> Option<f64> {
        Some(3f64.sqrt() * self.ac_volts? * self.amps?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_map_decodes_the_display_order() {
        let m = RegisterMap::default();
        assert_eq!(m.count(), 7);
        let r = m.decode(&[1500, 1500, 34, 9000, 155, 63, 312]);
        assert!((r.out_hz.unwrap() - 150.0).abs() < 1e-9 && (r.amps.unwrap() - 3.4).abs() < 1e-9);
        assert_eq!(r.rpm_from_hz(2.0), Some(9000.0));
        assert!(r.at_speed());
        // only the two confirmed registers came back
        let r = m.decode(&[1200, 1500]);
        assert!(r.amps.is_none() && !r.at_speed(), "still accelerating");
        let mut m2 = m;
        m2.amps = None;
        m2.rpm = None;
        m2.dc_volts = None;
        m2.ac_volts = None;
        m2.temp_c = None;
        assert_eq!(m2.count(), 2);
    }
}
