//! Machine control: configuration, trajectory planning, motion backends and
//! the controller that ties them to the G-code interpreter.

pub mod backend;
pub mod config;
pub mod controller;
pub mod mach3;
pub mod planner;
pub mod probing;
pub mod sim;
pub mod store;

pub use backend::{Backend, BoardInfo, Outputs, Status};
pub use config::{AxisConfig, Input, MachineConfig, ToolChangeSettings};
pub use controller::{
    CalibrateStage, CalibrateView, Command, HomeTestView, home_test_stats, Controller, ControllerHandle, FindStockStage, FindStockView, Mode, Snapshot, StockFound, ToolChangeStage,
    ToolChangeView, STOCK_REFERENCE,
};
pub use probing::{FindStockSettings, ProbeJob, ProbeSettings, ToolSetter, ZReference};
