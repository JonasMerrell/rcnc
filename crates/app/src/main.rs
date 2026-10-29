//! CNC controller desktop app: a modern replacement for Mach3.

mod app;
mod preview;

use std::sync::Arc;

use machine::sim::{RealClock, Simulator};
use machine::{Controller, ControllerHandle, MachineConfig};

fn main() -> eframe::Result<()> {
    let clock = Arc::new(RealClock::new());
    // Only the simulator exists so far; the DMC2 board driver plugs in here.
    let backend = Box::new(Simulator::new(clock.clone()));
    let controller = Controller::new(MachineConfig::dmc2(), backend, clock);
    let handle = ControllerHandle::spawn(controller);
    if let Some(path) = std::env::args().nth(1) {
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
