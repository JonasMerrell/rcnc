//! Driver for the BlueSkyT "BL-UsbMach" USB motion board that ships with the
//! Shariff DMC2. The protocol was reverse-engineered from USB captures of
//! Mach3 and its plugin; see `docs/dmc2-protocol`.

pub mod board;
pub mod protocol;

pub use board::BlueSkyBoard;
