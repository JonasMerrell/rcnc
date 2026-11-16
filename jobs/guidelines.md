# Writing CAM for the DMC2: guidelines for agents

This is for an agent making rCAD CAM and rcnc programs for Jonas's DMC2 mill. The rules marked **(Jonas)** are
his own and are not up for trade-offs. Everything else comes from parts already run on this machine (the test
part, the phone stand, the ingot slider). Where a rule here and a job's own RUN.md disagree, the job wins.
Then say so.

## Safety and how to work

- **Never move the machine yourself.** Nothing gets sent to the mill and nothing gets run on it unless
  Jonas asks for that exact thing. Programs, setup sheets and simulations are yours to make; running them
  is his.
- **Don't change a program Jonas may already be running** unless the simulation shows a real problem.
  If it does, tell him first.
- **netdash has limited memory.**
  - Run one simulation or one cargo build at a time, and pass `-j2` to cargo.
  - Close GUI containers when you're done with them.
  - Commit work in progress often.
  - A new rCAD worktree rebuilds OpenCascade (about 33 min) unless `target/OCCT` is reflinked in.
- **The mill PC builds rcnc from `mill/2026-11-14`**, or from a job branch such as `mill/<job>`. Each job's
  files go in `jobs/<job>/` with a RUN.md beside them. That PC is only rebuilt when Jonas says so.

## The machine

| Item | Value |
|---|---|
| Travel | X 343, Y 234, Z 150 mm. +Y moves the bed back. |
| Feeds | Max feed 3000 mm/min, Z rapid 3000. |
| Tool changes | Manual, ER16. No tool changer yet (the ATC design is in ~/dmc2). |
| Spindle | VFD, rated 1.5 kW input × 0.78 efficiency ≈ 1.17 kW at the tool, constant torque below its rated 24 000 rpm. Max 24 000 rpm. |
| Coolant | Flood, low pressure. Throws chips poorly out of deep pockets, and thin light chips worst of all **(Jonas)**. |
| Controller | rcnc (the rCAD post is `rcnc`). Mach3 macros don't apply. |
| Board | BlueSkyT USB. The probe and the tool setter share input 11, so rcnc refuses M3 with the probe loaded. |

The rCAD machine block is the `M` dict in a job's `cam.py`:

```json
{"name": "DMC2", "model": "DMC2", "travel": [343, 234, 150], "max_feed": 3000, "rapid_z": 3000,
 "tool_changer": false, "spin_up": 4, "spindle_power": 1500, "spindle_efficiency": 0.78, "spindle_rated_rpm": 24000}
```

Use `"post": "rcnc"` and `"probe": {"z_reference": "Vise"}` with `create_cam`.

### What the spindle can take

These points were measured in 6061 with the 1/4" 3-flute DLC end mill. Calibrate against them:

| Cut | Spindle load | Result |
|---|---|---|
| 0.6 mm stepover × 12 mm deep, 24 000 rpm, 2500 mm/min | about 56% | fine |
| 4.65 mm × 3 mm | about 85% | started to bog |
| Corner, 5 mm wide × 4 mm deep (contour cutting corners) | about 122% | audibly bogged |

From this, keep roughing **at or under about 70%** of the spindle. Jonas wants high-speed-machining style:
deep and narrow at high feed rather than wide and shallow.

## Tools (all carbide) **(Jonas)**

| T | Tool | Stick-out | Notes |
|---|---|---|---|
| T24 | 3 FLUTE DLC 1/4 (6.35, 19.05 flute) | 24 | The roughing/finishing end mill. DLC coated, so good in aluminium. |
| T11 | 6 mm 2-flute ball | 21 | Always run at **24 000 rpm / 3000 mm/min**: it barely cuts. |
| T1 | ALUMINUM chamfer 90 (6.35) | 22 | Chamfers and deburring. Use a tip offset so it cuts with the cone, not the weak tip. |
| T99 | YIDOFENG 3D touch probe | 40 | Ø2 ball, 12 mm reachable stylus. NPN-NO. |

- **Library:** import the tool library with `import_tool_library` and `"material": "carbide"`; the CSV says
  HSS, which is wrong. The library is `~/dmc2/tools/fusion-library.csv`.
- **Tool ids:** `cam_op` matches tool **ids** before numbers, so add tools in a fixed order. In the slider
  job the chamfer mill had to be added first to get id 1.
- **Stick-out:** put it on every tool. The program header writes it, rcnc's tool change window shows it,
  and the deflection and chatter models depend on it. Use the shortest stick-out that clears the part and
  the jaws, and check the ER16 nut against the highest feature when the tool is at its lowest.
- **Alarms:** 24 000 rpm in aluminium flagged chatter on the slider, so 22 500 was used. Follow the
  simulation's `best_rpm` advice. Treat the "479 m/min, burnt tool" warning as known for these tools.

## Workholding **(Jonas)**

- **Vise:**
  - Clamped on the fixture plate, **fixed jaw on the LEFT (−X)**.
  - The parts sit on the vise's **built-in parallels**, and Z0 is the part's bottom on them. That is rcnc's
    "Vise" Z reference, 40.610 below the tool setter's top.
  - The jaws grip the bottom **4.7 mm**. Beside the jaws, keep the tool at least **Z5.0** (0.3 mm over them).
  - Jaw length is 95.5 mm.
- **Never put new holes in the vise.**
- **Fixture plate:** M8 holes in two 25 mm grids, the second grid offset by half a pitch (12.5 mm in X and Y)
  so its holes sit in the middle of the first grid's squares. Move the moving jaw's base by whole 25 mm
  holes on the same grid, and say which hole in RUN.md.
- **No vise stops**, and nothing that adds height that gets in the way.
- **Shims:** a thin shim under the stock is fine when the jaw band is too short. The slider's brass sat on
  a 1 mm shim, with Z taken from the probed top. Make it shorter than the jaws so trimming the ends misses it.
- **Fixtures in rCAD:** put the jaws in `create_cam` as `fixtures` boxes so the simulation checks them.
  Give the vise its `on_table` position.

## Locating each setup

- **Op 1:** `T99 M6`, then `(RCNC Z_REFERENCE Vise)`, then
  `(RCNC FIND_STOCK SIZE=x,y,z AT=x,y,z TOL=… DEPTH=… KEEPZ=1)`.
  - Find stock touches the top and both sides at DEPTH below the top. DEPTH must be within what the stylus
    can reach: 10 mm, not 20.
  - `JAWX=-1 JAWZ=` takes X from the fixed jaw, but only when the stock is shorter than the jaws.
- **Op 2: locate from a feature op 1 cut, before roughing.**
  - **(Jonas)** Prefer a small **notch op 1 cuts into the carrier** (stock op 2 removes) over bores or other
    features in the finished part.
  - Probe it before any roughing, so nothing has to be re-located later.
  - Use `(RCNC FIND_BORES A= B= D= TOP= DEPTH= TOL= ROT=)` for two bores, and `(RCNC PROBE_Z …)` for a Z
    face. There is no slot/notch probe yet; ask for one in rcnc when a job needs it.
- **If you must re-locate after roughing** (e.g. Find stock on raw stock, then Find bores later), the
  radial allowance left must be bigger than the possible shift. On the phone stand the re-locate moved
  0.38 mm against 0.3 mm left, so one side didn't clean up. Leave **≥ 0.6 mm** radial stock for the
  passes after it.
- **Blow chips out first:** put a chip pause before any probing of op 1 features, with an MSG saying to
  blow out the bores or notch. A burr or chip in a bore made Find bores fail. rcnc now offers Probe again,
  but don't rely on that.

## Toolpaths

### Roughing

- Use **adaptive** (constant engagement) for pockets, around plinths and at corners. Don't let a contour
  plough into corners: that bogged the spindle.
- Leave **0.3 mm** on walls and **0.2 mm** on floors, more if a re-locate follows (see above).
- Roughing peak spindle load **≤ ~70%**, by the simulation's `loads[].spindle_power_pct`.
- Keep air cutting low (`air[].air_pct`), but not at the cost of overload spikes.
- rCAD's adaptive still has open bugs: links over "cleared" bands that aren't cleared, and wall bands wider
  than the stepover. Check the worst-load location (`loads[].at`, `width`) every time.

### Finishing **(Jonas)**

- **Every surface gets a finishing pass.** Floors too, e.g. where the phone sits.
- Walls: **one full-height contour pass** where the flute reaches. Use leads, not ramps.
  - Since PR 146, contours default to arcs onto and off the wall (half the tool's diameter), a 1.5 mm
    overlap past the seam, a lift as the tool leaves, and a light 0.03 mm last lap when cutting to size.
  - Put the seam on a straight, out of sight (the `seam` point); the default is the longest straight side.
- **Ball finishing:** stepover **about 0.03–0.04 × D** (0.2 mm on the 6 mm ball); 0.06 × D left big cusps.
  - Use scallop (constant cusp), or parallel only on the shallow areas plus waterline on the steep ones at
    the same spacing.
  - Semi-finish slopes first, so the ball never takes a deep cut.
- **Deburr every edge.** Use the chamfer mill (deburr or chamfer ops, with a tip offset), or a ~0.2 mm edge
  break with the ball where the chamfer mill can't reach. This includes bore and pocket mouths and the edges
  op 2 creates.

### Chips

- Deep, narrow pockets and grooves pack chips (low-pressure flood, thin chips).
- Check the simulation's `chips[]` (`chip_fill`). Where the chip model cautions, add a **chip pause** after
  the rough: G53 Z0, M5, M9, an `(MSG,…)` saying what to blow out, M0, restart the spindle and coolant, and
  dwell (see `jobs/phone-stand/add_chip_pause.py`). Re-run the pause script after every re-post.

### Chatter

- Check `chatter[].margin`: under 1 is stable, and lower is better.
- It isn't calibrated to the mill yet, so use it to compare toolpaths. The advice it gives (best rpm,
  shorter stick-out) is worth following.

## Checks before handing over

1. `simulate_cam` for every setup reports no problems. Read the following for each op, and quote them in
   RUN.md:
   - time;
   - air %;
   - peak spindle load %;
   - worst load and where;
   - chatter;
   - chip fill;
   - anything the finish passes leave rough (`unfinished`).
2. Fixtures and jaws aren't hit. Nothing goes below Z5.0 beside the jaws.
3. Each program runs to the end in the rcnc simulator:
   - `RCNC_JOB=file.nc cargo test --release -p cnc-machine runs_a_job_file -- --ignored`;
   - with `RCNC_STOCK`/`RCNC_BORES` set as the job needs.
   This catches probing, tool change and soft-limit problems.
4. The program header lists every tool with its stick-out. The setup sheet (`export_setup_sheet`) is beside
   the program (`<name>.html`).
5. RUN.md says:
   - how to set the vise (which hole) and what to clamp;
   - the tools and their stick-outs;
   - each op's steps, times and spindle peaks;
   - where the chip pauses are and what to do at them;
   - how each op is located;
   - the checks above.
6. Show the result:
   - renders or simulation pictures of the toolpaths and the finished stock;
   - an HTML playback of op 2 if it's 3D (`~/dmc2/stand/websim`).

## rCAD workflow that has worked

- Write a Python `cam.py` that builds the `rcad run` JSON (`cam.json`): open the part document, import the
  library, `create_cam` per setup, `cam_tool`s, `cam_op`s, `simulate_cam`, `render_cam`, `export_gcode`,
  `export_setup_sheet`.
- A later setup inherits the earlier setups' stock automatically, so op 2 sees what op 1 cut.
- Run it with `~/src/cad-rust-app/target/release/rcad run cam.json > out 2> err`. Every tool call prints a
  JSON line; summarise them with a small script (see `jobs/phone-stand/adaptive-handoff/variants/rep.py`).
- Try settings as variants of the same JSON, one simulation at a time. Keep the ones that win and say why.
- Examples:
  - `jobs/phone-stand/` (with `adaptive-handoff/` for the scripts);
  - `jobs/ingot-slider/` on `mill/ingot-slider` (~/dmc2/slider);
  - `jobs/dmc2-testpart/`.
