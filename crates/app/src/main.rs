//! CNC controller desktop app: a modern replacement for Mach3.

mod app;
mod preview;

use std::sync::Arc;

use blueskyt::protocol::BoardSetup;
use blueskyt::BlueSkyBoard;
use machine::backend::Backend;
use machine::sim::{RealClock, Simulator};
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
    let controller = Controller::new(MachineConfig::dmc2(), backend, clock);
    let handle = ControllerHandle::spawn(controller);
    if let Some(path) = program {
        match std::fs::read(&path) {
            Ok(bytes) => handle.send(machine::Command::Load {
                name: std::path::Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or(path.clone()),
                text: String::from_utf8_lossy(&bytes).into_owned(),
            }),
            Err(e) => eprintln!("could not read {path}: {e}"),
        }
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("CNC Controller")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1024.0, 680.0]),
        ..Default::default()
    };
    eframe::run_native(
        "CNC Controller",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc, handle)))),
    )
}
