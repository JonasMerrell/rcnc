//! Machine control: configuration, trajectory planning, motion backends and
//! the controller that ties them to the G-code interpreter.

pub mod backend;
pub mod config;
pub mod controller;
pub mod planner;
pub mod sim;

pub use backend::{Backend, BoardInfo, Outputs, Status};
pub use config::{AxisConfig, Input, MachineConfig};
pub use controller::{Command, Controller, ControllerHandle, Mode, Snapshot};
