# Reading the H100 VFD into rcnc over RS-485

Adapter: Waveshare USB to RS485 (FT232RL + SP485EEN). rcnc only **reads** from the drive. It never writes a parameter and never sends run/stop. The spindle stays controlled by the Mach3 board, exactly as now.

## 1. Wiring (power the VFD off first)

| Adapter terminal | H100 control terminal |
|---|---|
| A+ | 485+ |
| B- | 485- |
| GND | GND (the control-terminal ground, not PE) |

- Use a short twisted pair, shielded if you have it. Ground the shield at one end only (the VFD's GND). Keep it away from the spindle motor cable.
- A run this short (same table, under ~2 m) doesn't need a termination resistor. Leave the VFD's termination jumper as it is.
- If rcnc says "no answer", first swap A+ and B-. A swap does no harm, and the labelling varies between makers.
- Windows normally installs the FTDI driver by itself. The adapter shows up as a COM port in Device Manager under "Ports (COM & LPT)".

## 2. VFD parameters: read them, don't change them yet

On the VFD panel, **read** these and send me the values (no changes needed yet):

- **F163**: communication address (factory 1)
- **F164**: baud rate (0 = 4800, 1 = 9600, 2 = 19200, 3 = 38400)
- **F165**: data format (8N1/8E1/8O1, RTU)
- **F169**: protocol (0 = standard Modbus), if your drive has it

rcnc tries 9600, 19200, 38400 and 4800 with 8N1, 8E1, 8O1 and 8N2 at the address set in rcnc (default 1), so the factory settings will probably just work.

**Leave F001 (run command source) and F002 (frequency source) alone.** Reading works with them on terminal/analog control.

## 3. Check it in rcnc

1. Plug the adapter into the mill PC and start rcnc. Open **Settings → Spindle VFD (RS-485)**. It should say "Connected on COMx at 9600 8N1" (or similar) and list the drive's F parameters.
2. Click **Keep <line>** so it stops scanning on later starts.
3. Run the spindle at S9000 (150 Hz at F004=150). In the live register table, **reg 0** should read 1500 and **reg 1** 1500 (0.1 Hz units). Both registers are confirmed.
4. **Check the registers that aren't confirmed yet.** Press the drive's display key until it shows output current (A), and compare with **reg 2** (expected ×0.1 A, so 27 = 2.7 A, about the no-load current at 150 Hz). Do the same for output voltage (**reg 5**), speed (**reg 3**) and DC bus (**reg 4**). Send me a photo of the table next to the display if anything doesn't match. The register map is in `vfd.json` in rcnc's data folder.
5. The status bar now shows `VFD 150.0 Hz 2.7 A 63 V`.

## 4. What rcnc does with it

- **Status bar:** output frequency, current and voltage. Hover for rpm, set frequency, DC bus and the peak current this run.
- **Log:** every run writes `<program>.vfd.csv` beside the program (air cuts write `.air.vfd.csv`). It has 10 rows a second: time, line, tool, programmed rpm, feed, velocity, Hz, A, V, DC bus, temperature, apparent VA and the raw registers. This lets rCAD calibrate its power model against real current.
- **Feed hold on spindle current (opt-in, off by default):** this holds the feed when the current stays over a limit (default 8.5 A) for 0.3 s while cutting at speed. It ignores acceleration current, because it only judges once the output frequency has reached the set frequency, plus 1 s. Turn it on after step 3.4 confirms the current register. Reference from the 2026-11-18 ladders: about 2.7 A no-load at 9k, and 8.6 A at about 1049 W.
- The microphone's bog guard still matters. The drive reports its *output frequency*, not the real spindle speed: there is no encoder, so slip under load is invisible to the VFD. Current shows the load and the mic shows the sag.

## Standalone check (optional)

From an rcnc checkout: `cargo run -p cnc-vfd --features serial --example vfd_probe` (or `... -- COM5 1`). It finds the drive and prints its parameters, then the raw registers twice a second for 20 s.

## Sources

- grblHAL H100 driver (`Plugins_spindle/vfd/h100.c`): Modbus RTU, F parameters as holding registers (reads F143, F011, F005), live readings with function 0x04 from register 0.
- FluidNC `H100Protocol.md` ("all these things are tested"): `01 04 0000 0002` returns the running frequency, then the set frequency.
- F164 baud codes: Onefinity forum / Industrial Monitor Direct H100 article. F163/F164/F165/F169 names and factory values: V70 manual (a sibling drive).
- Registers 2–6 (current, speed, DC bus, output voltage, temperature) are assumed to follow the drive's display order. They are **not confirmed** until step 3.4.
