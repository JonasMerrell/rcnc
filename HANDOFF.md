# DMC2 project handoff (to the T3-code coordinator)

Written 2026-11-23, about 5 PM Mountain, by the previous coordinator's handoff thread.
Owner: **Jonas** (GitHub `JonasMerrell`). Jonas's time zone is America/Denver.

This folder: `~/t3/DMC2` on netdash.

| Path | What it is |
|---|---|
| `HANDOFF.md` | This file. Start here. |
| `memory/` | Full copy of the old project memory. `MEMORY.md` is the index; each `[[name]]` link is `memory/<name>.md`. It is the authoritative fact list as of today. |
| `reference/guidelines.md` | **Rules for writing CAM for the DMC2. Read before any CAM work.** (Its "netdash has limited memory / -j2" lines are stale: netdash now has 64 GB.) |
| `reference/dmc2-tools.csv` | Master tool library (T numbers, geometry only). `fusion-geometry-review.csv` is the old Fusion dump, reference only. |
| `reference/vfd-rs485-SETUP.md` | Wiring/setup for reading the VFD over RS-485. |
| `reference/fourth-axis-cycloidal-MAKING.md` | Pre-build machine checks and making plan for the 4th axis. |
| `jobs/ingot-slider-HANDOFF.md` | Detailed handoff for the gift slider + Eclipse car job (programs, hashes, open items). |
| `jobs/eclipse-car-SPEC.md` | Approved Eclipse car design spec. |
| `jobs/fourth-axis-HANDOFF.md` | Detailed handoff for the zero-backlash 4th axis design (written by the 4th axis thread). |

---

## 1. What the project is

Jonas's home-built CNC mill, the **DMC2**, and everything around it:

1. **rcnc**: a Rust CNC controller (replaces Mach3) driving his BlueSkyT step/dir board. Repo `JonasMerrell/rcnc` (default branch `main`). Can be rewritten freely.
2. **rCAD**: Jonas's Rust CAD/CAM app (`JonasMerrell/cad-rust-app`, default `master`). Used for part design, CAM, a physical cutting simulation (power, deflection, chips, chatter), FEA and posting. Agents may fix rCAD toolpaths themselves and open PRs.
3. **Real parts** cut on the mill from rCAD programs: test part, phone stand, stick-out gauge (all done), and the **gift batch** (ingot magnetic slider, Eclipse 1G car slider) in production now.
4. **Machine upgrades** (design stage): standalone arm-and-carousel toolchanger, ISO20 ATC spindle, zero-backlash 4th axis (cycloidal), VFD RS-485, coolant wash-down.

Jonas's overall wishes: simulation as physically accurate as possible; parts cut right first time; minimal air time; mass-produce gifts with minimal hassle.

## 2. Where things live

| Place | What |
|---|---|
| **netdash** (Linux VM, 64 GB RAM, `/home/jonas`) | Where all agent work runs. `~/dmc2` = job scripts, CAD, CAM sources, toolchanger/4th-axis models, `~/dmc2/tools/dmc2-tools.csv`. `~/src/cad-rust-app` (rCAD checkout; **on branch `gauge-build` with uncommitted changes**, so its `rcad` binary is older than PR 169/170: stash/commit and rebuild master before relying on it). `~/src/rcnc-eclipse` (Eclipse job sources). |
| **Mill PC** "DMC2" (Windows, 1920x1080 @150%) | rcnc checkout `C:\src\rcnc`, build output `C:\src\rcnc-test-target\release\cnc-controller.exe` (desktop shortcut). Settings `%APPDATA%\rcnc` (`machine.json`, `state.json`, `log.txt`). Jobs in `C:\src\rcnc\jobs\<job>\`. Currently built from rcnc **main 9bca2b1** (2026-11-21). Reached only through a Remote Control session when Jonas has the Claude app open there; it was **offline** on 2026-11-22. Git push from it works (saved sign-in). |
| **Jonas's desktop** (Windows, `C:\Users\legoj\dmc2`) | Approved folder; occasionally used. |
| **Old project files** (claude.ai project "DMC2", `/mnt/project-files/`) | Renders, STEP files, reports, job copies: `fourth-axis/` (REPORT.md, cycloidal/), `ingot-slider/`, `gift-ideas/`, `stickout-gauge/`, `phone-stand/`, `guidelines/`, `tools/`, `vfd-rs485/`, toolchanger zips/HTML (`DMC2 toolchanger*.zip`, latest `...20261118T223931-64fd.zip`). |
| rcnc job branches | `mill/ingot-slider-vise` (current gift programs, fca5cd0), `design/eclipse-car-surface` (approved car surface), `logs/*` (real-run cut logs), older `mill/*`. |

## 3. Machine facts (essentials; details in `memory/`)

- **Travel** X 343 (homes at -8), Y 234 (homes at +512, +Y moves the bed back), Z 150. X/Y axis max 8000 mm/min; CAM `max_feed` 3000 cutting, links/rapids 5000.
- **Controller board**: BlueSkyT (USB HID 3698:0107), step/dir only; closed-loop steppers but rcnc cannot read encoders. Outputs tight: spindle 11, flood 13, enable 14, only 12 free. Y home = inductive (5 um std dev); X home = mechanical limit switch (8.7 um, once shifted 0.1 mm).
- **Spindle**: 24k rpm, ER20 collet (max 13 mm shank; memory/guidelines may still say ER16 in old places), 2-pole, a ~220 V winding on an **H100-2.2C2-1B VFD that outputs only 110 V**. VFD F004 = 150 so full torque to 9k.
  - **Rough at 9k**: plan up to ~850 W (model watts), never exceed ~950 W; default roughing ~650 W. 12k carries ~590 W. 24k only ~450-500 W: finishing, balls, chamfers. 1/4 in roughing stepover <= ~1-1.15 mm.
  - rcnc bog guard (rpm-slip hold: slip 1400 rpm, cap 7%, 0.3 s) must be armed. The transducer's chatter flag false-alarms.
- **Vibration transducer** on the mill PC mic jack, on the spindle head: gives heard rpm, droop, cut logs (`<program>.cutlog.json`).
- **Probe**: YIDOFENG NPN-NO 3D probe (T99, 2 mm stylus) on input 11, shared with the BTER tool setter (setter G53 X-5 Y289). Keep side touches <= 10 mm deep.
- **Vise**: two-station vise, fixed centre block, jaws 95.5 long, 1.9 mm ledges as built-in parallels, grip ~4.7 mm above the ledges. Z0 = vise floor (Mach3 "vise" offset 40.610 below setter top). Fixed jaw on the LEFT. Find vise location has been run on the mill (saved in `machine.json`).
- **Stock**: Jonas's "1x1" bar is really 32 x 32. Brass 10.0 x 25.0, aluminium 12.34 x 26.6. Always confirm measured stock before posting.
- **Tools**: all carbide; 1/4 in flats DLC. T numbers in `reference/dmc2-tools.csv` (T1 chamfer, T24 1/4 3F DLC, T11-14 balls, T41-44 SpeTool tapered balls, T23 dovetail, T99 probe...). Stick-out is set per job.
- Coolant: low-pressure flood.

## 4. Standing rules and known traps (Jonas's; do not trade off)

1. **Never move the machine yourself.** Nothing runs on the mill unless Jonas asks for that exact thing. Rebuild the mill PC only when he says "rebuild"/"load it".
2. **NEVER launch rcnc (`cnc-controller.exe`) from a Remote Control shell.** It inherits the Claude app's MSIX package and reads/writes stale settings copies (vise location "lost" 2026-11-20). After a rebuild, leave rcnc closed and ask Jonas to start it from Explorer/shortcut. To see real settings files from RC, use WMI (CIM_DataFile). Don't send synthetic key presses on the mill PC while drives are enabled.
3. **Posting rule**: rCAD must refuse to post a program whose sim shows gouges or rapids into stock; never hand over an .nc not verified against the clean sim of the same run (SIM OK gate, `verify_gcode`, rcnc sim). Verify sha256 of files on the mill PC.
4. **Don't change VFD parameters without Jonas.** (Current: F004=150, F005=400, F012=1 vector, F141=110 V, F142=11 A, F143=2, F144=24k. Don't take F004 lower than 150.)
5. CAM preferences: climb everywhere, finishing always climb, full laps round the part; every surface gets a finishing pass; every edge deburred (chamfer mill with ~1 mm tip offset, or ~0.2 mm ball edge break next to shallow faces where the chamfer cone gouges); finish walls in one full-height pass with arc leads + overlap (no witness mark); cut the whole side profile in op 1 (no blending between ops); size cuts from the power budget; minimise air time; op 2 starts with Find stock then Find bores, referencing a through feature or a notch in the carrier; batch jobs use both vise stations; nothing above the jaws, no new holes in the vise.
6. A small CAM change (like a shim) must re-plan in minutes: job scripts pass `cam_op build:false`; no `*-sim.html` playbacks.
7. DFM for machine parts: 3D printed (no supports) or machined with a single 1/4 in endmill; real vendor CAD for bought parts; model every bolt.
8. When Jonas is waiting at the machine: quick .nc fix and push first, full re-post after; tell him the moment the file is on the mill PC.

## 5. Open pull requests (checked 2026-11-23)

**rcnc**: none open. Last merged: #25 probe tool-change checks (main 9bca2b1, built on the mill PC), #24 heard rpm, #23 wash-down, #22/#21 Find vise location.

**cad-rust-app**:
- **#175** `waterline-chords` (base `cam-job-order`, not master): waterline chords on meshes, arcs on 3D finishing, tapered-ball flank height fix, magnet pocket bottom-lap fix. **The current gift programs were posted with this build.** Needs Jonas's review/merge (check its base branch is merged first).
- **#161** `link-feed` (base master): air G1 moves at `machine.link_feed` 5000. Old (2026-11-18), may need a rebase; ask Jonas whether to merge or close.
- Note: the slider handoff lists #163/#165/#166/#167 as open; they were **merged** 2026-11-22. Many other rCAD PRs merged 2026-11-23 (Android app, Gemini/Codex chat, OrcaSlicer link, photogrammetry) came from outside this project.

## 6. Open work, state and next step

### Active / waiting on Jonas

| Work | State | Next step |
|---|---|---|
| **Gift sliders: ingot + Eclipse car** (`jobs/ingot-slider-HANDOFF.md`, memory `dmc2-ingot-slider-job`, `dmc2-eclipse-slider`) | Normal slider + base in production (bases perfect; brass magnet pockets re-posted at 8.07). Eclipse car + thick-rail base designed, approved, programmed, **not cut**. All on rcnc `mill/ingot-slider-vise` fca5cd0. | (1) When the mill PC is online, copy job files only (no rebuild, no launch): replace `slider/` (old to `old\`), add `eclipse-car/`, `eclipse-base-thick/`; verify the 9 hashes; check T44 exists in rcnc's tool table. **Until verified, assume the mill PC has the OLD programs.** (2) Jonas to confirm T44 feed (1500 mm/min assumed). (3) Merge rCAD #175. (4) First car cut: watch the joined rough on the 3.0 mm shim grip; check 8.07 pocket fit. |
| **Mill PC access** (old thread "Test X and Y homing switch repeatability") | That thread held the mill PC Remote Control session; homing test itself is done (Y 5 um, X 8.7 um; good enough for op-1 offsets). Mill PC offline since 2026-11-22. | Ask Jonas to open the Claude app on the mill PC when a mill-PC step is needed. Optional later: find the X 0.1 mm home shift (switch mount, coupling set screws). |
| **Zero-backlash 4th axis** (memory `dmc2-fourth-axis`; project files `fourth-axis/`, `fourth-axis/cycloidal/`; cad-rust-app branch `claude/project-thread-33amtt`, unmerged, no PR) | Design done (strain-wave iter 1-6, then Jonas chose a **home-machined cycloidal** c4: 23:1 + 3:1 belt = 69:1, flexure preload, RU66, air brake, NEMA 23, fits the table, tool setter reachable). Nothing bought, no CAM. Last: Jonas sent an AliExpress harmonic unit link (~138 USD) the agent could not read. | Waiting on Jonas: paste that listing's specs; metrology he owns (mics, gauge pins, DTI, ring gauge); shop air OK for brake; cycloid vs cheap harmonic. Then the five machine checks in `reference/fourth-axis-cycloidal-MAKING.md`, then CAM for the plates. Full detail: `jobs/fourth-axis-HANDOFF.md`. |
| **Detect the new transducer** | Stale: transducer was detected and works (2026-11-15). | Confirm with Jonas and close. |

### Review ready / likely finished

| Work | State | Next step |
|---|---|---|
| Launch rcnc with the test part job | Done long ago (first part cut 2026-11-14). | Close. |
| Pick a drop-in mill controller board | Answered: keep the BlueSkyT (5 axes, ~10 free inputs) but only one free output (pin 12); Flexi-HAL only if more outputs/isolated inputs needed. rcnc runs on Linux too (build a98dd0b). | Close unless Jonas revisits for the toolchanger. |

### Recently completed, loose ends worth knowing

| Work | Loose ends |
|---|---|
| Tool library (`reference/dmc2-tools.csv`) | Unconfirmed geometry: 2 mm ball flute 4, 3 mm ball flute 8, 6x2 T-slot neck 2.8 / shank 6; dovetail flank height 1 mm assumed. |
| Coolant wash-down (rcnc PR 23, on the mill PC) | Flood wiring to rcnc's flood output and nozzle position unconfirmed; never run on the real pump. |
| VFD RS-485 (rcnc PR 19, merged) | Hz registers confirmed; amps/volts register map assumed until checked against the drive display. Waveshare adapter arrived ~2026-11-19; not confirmed wired. |
| Heard rpm (PR 24) | Open question rotor vs field comb at 12k; log VFD Hz on a real run. |
| Arm-and-carousel toolchanger | Design passes in project files (`DMC2 toolchanger*.zip/html`, `DMC2-changer-design.zip`, renders). Standalone unit on the same table, its own BTT board talking to the mill board. Not built. Spindle side: ISO20 ATC (needs 14.1 mm adapter plate) or G-Penny GDL80 2.2 kW (Ø80 drop-in; needs 110 V winding, check 15° pull stud). Alternative assessed: RapidChange-style ER20 nut spinner (workable, ~half ER20 torque spec). |
| rCAD toolpaths from literature | Feed scheduling built (small gain). Next lever: wider stepover by measuring engagement angle; chatter calibration with a tap test. |
| Cutting simulation | Model may over-predict power at 9k; chatter uncalibrated. Keep using real cut logs (`logs/*` branches) to calibrate. |
| Find Mach3 Z offsets, Take over rcnc, Make real parts | Done; facts are in memory. |

## 7. How Jonas works

- Drops requests and comes back later; wants results, not process. Says "go", "rebuild", "load it", "merge" to approve.
- Approves designs from renders before programs are posted; likes options with a recommendation.
- Measures and reports real results; trust his measurements over CAD guesses (but the vise STEP is exact).
- Buys Prime-eligible stock; brass and aluminium both welcome.
