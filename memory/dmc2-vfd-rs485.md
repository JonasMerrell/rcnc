---
name: dmc2-vfd-rs485
description: Reading the H100 VFD into rcnc over RS-485 (rcnc PR 19, branch vfd-rs485): protocol facts, what's verified, next steps
metadata:
  type: project
  modified: 2026-11-18T17:02:40.862Z
---
- Jonas ordered a Waveshare USB-RS485 adapter (FT232RL), arriving ~2026-11-19, to wire to the H100's 485+/485-/GND.
- rcnc PR 19 (branch vfd-rs485, MERGED to main 2026-11-18 as f9c6b90): crate cnc-vfd + app/vfd.rs. Read-only (only Modbus 0x03/0x04). Auto-scans USB ports and line settings, then confirms the drive via F143. Status bar, Settings -> Spindle VFD (raw registers), <program>.vfd.csv per run, opt-in amps feed hold (8.5 A / 0.3 s, at speed only). Not yet on the mill PC; needs Jonas's "rebuild".
- H100 protocol: standard Modbus RTU. Holding reg N = parameter FN. Input reg 0 = output Hz x0.1 and reg 1 = set Hz (confirmed by grblHAL h100.c and FluidNC H100Protocol.md). Regs 2-6 (amps x0.1, rpm, DC bus, out V, temp) are ASSUMED (display order) until checked against the drive display; the map is in vfd.json.
- F164 baud: 0=4800, 1=9600, 2=19200, 3=38400. F165 format codes unverified. Leave F001/F002 alone; reading doesn't need them changed.
- The VFD can't see slip (no encoder), so the mic droop guard is still needed; current is the load signal.
- Setup sheet: project files vfd-rs485/SETUP.md (also rcnc docs/vfd-rs485.md).
- Related: [[dmc2-cutting-simulation]]
