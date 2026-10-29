//! Numbered parameters (`#1`..`#9999`), as used by Mach3 and RS274/NGC.

use std::collections::HashMap;

/// Well-known parameter numbers.
pub mod num {
    /// G31 probe trip position X..C (Mach3 convention), in program coordinates.
    pub const PROBE_X: u32 = 2000;
    /// G28 home position X..C, in machine coordinates.
    pub const G28_HOME: u32 = 5161;
    /// G30 home position X..C, in machine coordinates.
    pub const G30_HOME: u32 = 5181;
    /// G92 offset X..C.
    pub const G92: u32 = 5211;
    /// Active work coordinate system, 1..=6 for G54..G59.
    pub const COORD_SYSTEM: u32 = 5220;
    /// G54 X..C offsets; G55 starts 20 higher, and so on.
    pub const G54: u32 = 5221;
    /// Stride between coordinate systems.
    pub const COORD_STRIDE: u32 = 20;
    /// Parameters kept between sessions: G28/G30 homes, G92, the active
    /// coordinate system and G54..G59 (as in RS274/NGC's parameter file).
    pub const PERSISTENT: std::ops::RangeInclusive<u32> = 5161..=5390;
}

#[derive(Debug, Clone, Default)]
pub struct Params {
    values: HashMap<u32, f64>,
}

impl Params {
    pub fn get(&self, index: u32) -> f64 {
        self.values.get(&index).copied().unwrap_or(0.0)
    }

    pub fn set(&mut self, index: u32, value: f64) {
        self.values.insert(index, value);
    }

    pub fn get_axes(&self, start: u32) -> [f64; 6] {
        std::array::from_fn(|i| self.get(start + i as u32))
    }

    pub fn set_axes(&mut self, start: u32, values: [f64; 6]) {
        for (i, v) in values.into_iter().enumerate() {
            self.set(start + i as u32, v);
        }
    }

    /// Offsets of work coordinate system `index` (0 = G54 .. 5 = G59).
    pub fn coord_offset(&self, index: usize) -> [f64; 6] {
        self.get_axes(num::G54 + num::COORD_STRIDE * index as u32)
    }

    /// Non-zero parameters in the persistent range, for saving.
    pub fn persistent(&self) -> std::collections::BTreeMap<u32, f64> {
        self.values
            .iter()
            .filter(|(k, v)| num::PERSISTENT.contains(k) && **v != 0.0)
            .map(|(k, v)| (*k, *v))
            .collect()
    }

    pub fn set_coord_offset(&mut self, index: usize, values: [f64; 6]) {
        self.set_axes(num::G54 + num::COORD_STRIDE * index as u32, values)
    }
}
