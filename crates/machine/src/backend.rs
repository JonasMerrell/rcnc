//! The interface every motion controller board implements.
//!
//! It mirrors what the DMC2's stock board offers: a status poll that also
//! carries the outputs, a queue of fixed-period step segments, a single-axis
//! "move until an input changes" command for jog, homing and probing, and
//! position counters the host can set.

use crate::config::{Input, MachineConfig};
use crate::planner::Segment;

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("board not found")]
    NotFound,
    #[error("not connected")]
    NotConnected,
    #[error("communication error: {0}")]
    Io(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, BackendError>;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoardInfo {
    pub name: String,
    pub firmware: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    pub position: [i64; 6],
    /// Bit n = input pin n, physical level.
    pub inputs: u32,
    /// Free segment slots in the board's queue.
    pub queue_free: usize,
    /// No motion running and nothing queued.
    pub idle: bool,
    /// Source line of the segment being executed.
    pub line: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Outputs {
    /// Drive enable (on while out of reset).
    pub enable: bool,
    pub spindle_on: bool,
    pub spindle_ccw: bool,
    /// Spindle speed as a fraction of maximum, 0..=1.
    pub spindle_duty: f64,
    pub spindle_rpm: f64,
    pub flood: bool,
    pub mist: bool,
}

/// Single-axis move that stops after `steps` or when `stop_on` becomes active.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisMove {
    pub axis: usize,
    pub positive: bool,
    pub steps: u32,
    /// steps/s
    pub velocity: f64,
    /// steps/s²
    pub acceleration: f64,
    /// Stop when this input reaches the given state (true = active).
    pub stop_on: Option<(Input, bool)>,
}

pub trait Backend: Send {
    fn name(&self) -> &str;
    fn connect(&mut self, config: &MachineConfig) -> Result<BoardInfo>;
    fn disconnect(&mut self);
    /// Duration of one queued segment, in seconds.
    fn segment_period(&self) -> f64;
    fn queue_capacity(&self) -> usize;
    fn poll(&mut self, outputs: &Outputs) -> Result<Status>;
    /// Queue segments; `last` marks the end of a continuous move.
    fn push_segments(&mut self, segments: &[Segment], last: bool) -> Result<()>;
    fn axis_move(&mut self, m: &AxisMove) -> Result<()>;
    /// Stop an axis move with a controlled deceleration (jog release).
    fn stop_axis_move(&mut self) -> Result<()>;
    /// Stop everything now and drop the queue.
    fn abort(&mut self) -> Result<()>;
    fn set_position(&mut self, steps: [i64; 6]) -> Result<()>;
}
