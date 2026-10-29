# CNC Controller

A modern replacement for Mach3, written in Rust. The first target machine is the Shariff DMC2, running on its stock USB motion board.

## Status

The app drives the DMC2's stock BlueSkyT USB board directly, using a protocol mapped from the Mach3 plugin and from USB captures (see `docs/dmc2-protocol`). The driver matches Mach3's frames byte for byte in tests and has jogged the real machine. A built-in simulator is available with `--sim`.

| Part | State |
| --- | --- |
| G-code interpreter (Mach3 dialect) | Done: G0–G3, R and IJK arcs, helix, G4, G10 L1/L2/L10/L20, G17–19, G20/21, G28/G30, G31/G38.2, G40/G41/G42 cutter compensation, G43/G49, G53, G54–G59, G73/G81/G82/G83/G84/G85/G89, G90/91, G90.1/91.1, G92, G93/94, G98/99, M0–M9, M30, M98/M99 subprograms, user M-code macros, `#` parameters, `[ ]` expressions |
| Motion planner | Done: 4 ms step segments, per-axis velocity and acceleration limits, exact stop (Mach3 "CV off") or blended corners, feed hold, feed and rapid override |
| Controller | Done: homing (seek, back off, clear), jog, MDI, program run, G31 probing, tool change with optional touch-off, spindle and coolant, E-stop input |
| Probing | Done: tool touch-off on the tool setter, Z surface, X/Y edges, outside corners, bore and boss centres (two-pass, guarded moves, auto zero) |
| Settings | Done: tool table, work offsets and settings saved between runs; Mach3 profile import |
| CAM link | Done: CAD/CAM apps can send programs and tools to rcnc on `127.0.0.1:7350`; opened files reload when they change |
| Simulator | Done: behaves like the stock board (4 ms queue, single-axis moves that stop on an input, home switches, probe plate and block) |
| Desktop UI (egui) | Done: DRO, jog, toolpath preview, program listing, cycle controls, overrides, MDI, tool/offset/probe/settings tabs |
| DMC2 stock board driver | Done: tested against captured frames; jogs the real machine |

## Files

Settings (`machine.json`), the tool table and work offsets (`state.json`) live in `%APPDATA%\rcnc` on Windows (`~/.config/rcnc` elsewhere) and are saved as they change. Put macros there as `macros/M<n>.nc`: plain G-code run in place of `M<n>`, with P and Q in `#1` and `#2`.

## CAM link

rcnc listens on `127.0.0.1:7350` for programs from a CAD/CAM app:

- `POST /program` with JSON `{"name": "part.nc", "gcode": "...", "tools": [{"number": 1, "diameter": 6, "description": "6 mm flat"}]}` loads the program and sets those tools' diameters and descriptions (measured lengths are kept). It's refused while the machine runs.
- `GET /status` returns `{"app": "rcnc", "mode": "Idle", "program": "part.nc"}`.

cad-rust-app's **Send to rcnc** button uses this, with its rcnc (Mach3-dialect) post.

## Layout

- `crates/gcode`: parser and interpreter. Turns G-code into machine operations in machine coordinates.
- `crates/machine`: machine config, trajectory planner, `Backend` trait, simulator and controller.
- `crates/blueskyt`: driver for the DMC2's stock BlueSkyT BL-UsbMach USB board.
- `crates/app`: desktop app (eframe/egui).

## Build and run

```sh
cargo test
# Drive the DMC2 (board plugged in, Mach3 closed):
cargo run --release -p cnc-app -- examples/pocket-and-holes.nc
# Or try it without a machine:
cargo run --release -p cnc-app -- --sim examples/pocket-and-holes.nc
```

Close Mach3 first: only one program can talk to the board at a time. Keep a hand on the E-stop for first runs.

On Linux you also need the usual X11/Wayland and OpenGL libraries, such as `libxkbcommon-x11-0` and `libgl1`, plus `libudev-dev` for USB HID access.

## Keyboard

| Key | Action |
| --- | --- |
| Arrow keys | Jog X/Y |
| Page Up / Page Down | Jog Z |
| Space | Feed hold |
| Esc | Stop |
| Alt+R | Cycle start |
