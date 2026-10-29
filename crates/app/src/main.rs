//! CNC controller desktop app: a modern replacement for Mach3.

mod app;
mod link;
mod preview;

use std::sync::Arc;

use blueskyt::protocol::BoardSetup;
use blueskyt::BlueSkyBoard;
use machine::backend::Backend;
use machine::sim::{RealClock, Simulator};
use machine::store;
use machine::{Controller, ControllerHandle, MachineConfig};

const USAGE: &str = "usage: cnc-controller [--sim] [program.nc]

  --sim    run against the built-in simulator instead of the DMC2's USB board";

fn main() -> eframe::Result<()> {
    let mut simulate = false;
    let mut program = None;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--sim" => simulate = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            _ if arg.starts_with('-') => {
                eprintln!("unknown option {arg}\n{USAGE}");
                std::process::exit(2);
            }
            _ => program = Some(arg),
        }
    }

    let clock = Arc::new(RealClock::new());
    let backend: Box<dyn Backend> = if simulate {
        Box::new(Simulator::new(clock.clone()))
    } else {
        Box::new(BlueSkyBoard::new(BoardSetup::dmc2()))
    };
    // Settings, tool table and work offsets live in the data folder and are
    // saved as they change; the DMC2 defaults apply until something is saved.
    let data = store::data_dir();
    let config = match store::load::<MachineConfig>(&data.join("machine.json")) {
        Ok(Some(c)) => c,
        Ok(None) => MachineConfig::dmc2(),
        Err(e) => {
            eprintln!("ignoring unreadable settings ({e}); using DMC2 defaults");
            MachineConfig::dmc2()
        }
    };
    let mut controller = Controller::new(config, backend, clock);
    controller.persist_to(data);
    let handle = ControllerHandle::spawn(controller);
    link::spawn(handle.clone());
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("CNC Controller")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([800.0, 480.0])
            .with_maximized(true),
        ..Default::default()
    };
    eframe::run_native(
        "CNC Controller",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, handle).with_program(program.map(Into::into))))),
    )
}
