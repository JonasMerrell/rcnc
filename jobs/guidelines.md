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
| Spindle | 2.2 kW 220 V spindle (2-pole, 400 Hz at 24k) on an H100 VFD that outputs only 110 V. **The VFD's F004 (reference frequency) is 150**: full torque up to 9000 rpm, weaker above (the 110 V ceiling). **9k is the strong speed** (2026-11-18): see the table below. At 24k it carries only about 450 W. Max 24 000 rpm. |
| Coolant | Flood, low pressure. Throws chips poorly out of deep pockets, and thin light chips worst of all **(Jonas)**. |
| Controller | rcnc (the rCAD post is `rcnc`). Mach3 macros don't apply. |
| Board | BlueSkyT USB. The probe and the tool setter share input 11, so rcnc refuses M3 with the probe loaded. |

The rCAD machine block is the `M` dict in a job's `cam.py`:

```json
{"name": "DMC2", "model": "DMC2", "travel": [343, 234, 150], "max_feed": 3000, "rapid_z": 3000,
 "tool_changer": false, "spin_up": 4, "spindle_power": 2600, "spindle_efficiency": 0.43, "spindle_rated_rpm": 9000,
 "spindle_above_rated": 0.76, "spindle_never_exceed": 0.85, "spindle_caution": 0.76}
```

Use `"post": "rcnc"` and `"probe": {"z_reference": "Vise"}` with `create_cam`, and `"stay_down": true` (rCAD
`tighten_links`) on the setup before posting. With rCAD PR #152/#154 the power the spindle has falls above 9000
rpm as (9000 / rpm)^0.76: never-exceed **950 W at 9k, about 765 at 12k, 450 at 24k**; caution **850 / 684 / 403 W**
(`loads[].power_rating`).

### What the spindle can take (measured 2026-11-17/18) **(Jonas)**

**Since 2026-11-18 the VFD is at F004 = 150** (it was 400: the spindle got half its flux at every speed). The
1/4" 3-flute (T24, 24 out), 12 deep, F004 = 150, sag over the no-load speed:

| rpm | Model W -> sag |
|---|---|
| **9000** | 484 3.2%, 589 3.6%, 676 4.2%, 759 5.0%, 850 6.5%, 949 6.6%, 1049 9.0%: **all steady** |
| 12000 | 590 5.2%, 676 5.7%, 759 7%, 839 9% (no runaway) |
| 24000 | 446 4.3-4.7% settled; 494-499 crept: the edge |

- **Rough at S9000** with the 1/4", up to about **650-700 W** (caution 850, never-exceed 950). 24k is for light
  finishing only (≤ 400 W).
- **Top-surface roughing at S12000** where the load fits (≤ 684 W): the chips fling better than at 9k **(Jonas)**.
- After the first long 9k job, feel the spindle body: F004 = 150 over-fluxes it a little under 9k (no-load 2.7 A).

The older table below is for F004 = 400 (before 2026-11-18): 24k only.

The 1/4" 3-flute DLC end mill (T24) at 24 000 rpm, 24 mm stick-out, in 6061. The speed is heard from the
transducer on the spindle head.

| Cut | MRR cm³/min | Model W | Result |
|---|---|---|---|
| 0.6 × 6 at 2400 | 8.6 | 351 | 1.8% sag, steady |
| 1.0 × 6 at 2400 | 14.4 | 531 | 3.2% sag, steady |
| 0.6 × 10 at 2500 | 15.0 | about 550 | comfortable (Jonas) |
| 0.6 × 12 at 2500 | 18.0 | 656 | **on the edge** (Jonas; it was written "comfortable" before, wrongly) |
| 1.4 × 6 at 2400 | 20.2 | 647 | 7.5% sag and never settled: **pull-out** |
| **1.8 × 6 at 2400** | 25.9 | 872 | **bogged and broke T24** (stick-out gauge op 1a, 32 tall bar on the 4.7 mm grip) |

The planning rules that follow:
- **Stepover:** about **1 mm or less (≤ 15% of D)** when roughing with the 1/4". Removal comes from depth and
  feed, not width **(Jonas)**.
- **Metal removal rate:** aim at **14–15 cm³/min** (about 3% sag). **Never plan over 18**, nor over **550 W**
  by rCAD; the bulk of a cut sits at 450–500 W.
- **Beyond a proven cut:** any cut past one that has already run fine needs rcnc's **bog guard** armed. Put
  `(RCNC REQUIRE_GUARD)` in the program: it holds the feed when the heard speed sags, and Cycle Start refuses
  without it.
- **Older points disagree:** the test part's 4.65 × 3 and the phone stand's 1 × 12.5 ran although the model
  puts them above the measured pull-out. Don't plan on them until that's understood.

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
  Taking Z from a probed top only works for a face that is already machined or that op 1 faces off; a raw
  face breaks the Z rule below. Otherwise add the shim's thickness to the Vise reference.
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
- Roughing peak spindle power: **under the caution at its speed** (F004 = 150: 850 W at 9k, 684 at 12k, 403 at 24k),
  never over the never-exceed. Read it from `loads[].power_w` / `power_rating`.
- **Depth at 9k: moderate, 5-7 mm**, with a wider step over for the same watts **(Jonas)**: 11-13 deep strips at
  9k packed chips on top of the bar, and at 9k the chips "don't really get flung at all". Prefer thick short chips.
- Every S change is followed by **3 s in air** (G4 P3) before the tool goes near the work: the bog guard learns
  that speed's no-load speed there (`add_dwell.py` in jobs/stickout-gauge).
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
- **A chip-clearing M0 pause (with a MSG) before any finishing pass that follows 9k roughing** **(Jonas)**.
- Check the simulation's `chips[]` (`chip_fill`). Where the chip model cautions, add a **chip pause** after
  the rough: G53 Z0, M5, M9, an `(MSG,…)` saying what to blow out, M0, restart the spindle and coolant, and
  dwell (see `jobs/phone-stand/add_chip_pause.py`). Re-run the pause script after every re-post.

### Chatter

- Check `chatter[].margin`: under 1 is stable, and lower is better.
- It isn't calibrated to the mill yet, so use it to compare toolpaths. The advice it gives (best rpm,
  shorter stick-out) is worth following.

## More rules from the real runs

- **At most 3 cutters per operation** **(Jonas)**: every change is by hand.
- **Chamfer tip offset:** about 1 mm below the chamfer's lower edge. The first part's chamfer, cut near the
  tip, had a poor finish. Check the clearance to floors and fixtures.
- **Z never comes from a raw stock face** **(Jonas)**. The first part came out 16.13 thick against 16 that way.
- **Nothing on the vise may stand above the jaw top** **(Jonas)**: it gets in the way of the tool and holder.
- **Batches use both stations of the vise** **(Jonas)**.
- **Flips:** say on the setup sheet which way the part is flipped, and mirror the probed feature
  coordinates to match (the phone stand's bores were at Y+19.25 in op 1 and Y−19.25 in op 2).
- **Program start:** `G21 G17 G40 G49 G80 G90 G91.1 G94 G64`, then `G54`. All retracts are `G53 G0 Z0`, so
  the machine must be homed; rcnc refuses Cycle Start otherwise. Spindle and coolant off before `T99 M6`.
- **Restart programs:** G54 survives Stop, Open and a restart. A restart program starts at the step that
  failed and must not probe partly cut stock again (see `jobs/phone-stand/op2-finish.nc`).
- **When Jonas is waiting at the machine:** make the quick `.nc` edit and push it first, and tell him the
  moment the file is on the mill PC. Re-post and re-simulate properly afterwards.
- **Say what was only simulated and what was really cut.** Unproven so far: the PR 146 contour seam and
  the finer ball stepover.
- **Vertical edges:** give outside vertical corners (and slot mouths at an end face) a small radius or chamfer in the CAD so the wall contour breaks them; a chamfer mill can't reach a vertical edge (stick-out gauge, 2026-11-17).
- **Measure the stock and confirm it with Jonas before any CAM.** Nominal sizes are not measurements: the stick-out gauge was planned for a "1x1" bar, and the bar was 32 × 32. **(Jonas)**
- **Keep this file current:** when a cut bogs, chatters, packs chips or leaves a mark, add the lesson here.

### Travel between cuts **(Jonas)**

- Don't waste time in air: op1-top (2026-11-17) spent about 10 of its 40 min feeding down through air from Z34
  and returning over the full 134 mm between one-way strips.
- Set `"stay_down": true` on the setup (rCAD `tighten_links`): with the stock simulated, the tool travels between
  cuts only as high as the stock still there needs (+1 mm), stays down at depth through cleared space or past
  the stock's end, and rapids down to 1 mm over the stock before an entry, feeding only the rest. Tool changes
  and speed changes keep the full retract.
- Cut roughing strips **both ways** (every other one conventional is fine for roughing) so each links to the
  next at depth off the stock's end. Finishing stays climb only.
- Report the air time per operation in the review (rapids and feed moves not in material) and check the
  simulation shows no rapid into stock (`problems`).
- **Finish one region before moving to the next (Jonas, 2026-11-18).** Within an operation, group the cuts by
  region and finish one area (all its depth levels) before travelling to the next, taking the nearest area
  next. Don't bounce around the part: on op1-top the adaptive steps hopped end to end and nearly all the time
  was air. rCAD's `stay_down` now orders the cuts this way (`order_by_region`): a cut waits only for earlier
  cuts that overlap its area (so depth levels and rough-before-finish hold), and keeps its direction (climb
  finishes stay climb). Report the long links (moves over ~20 mm between cuts) per operation alongside the
  air time, before and after.

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
- **Several setups in one program (two vise stations):** the op 1 setup leads an rCAD *job*.
  - `create_cam` takes `with: ["<op 2 setup>"]`. Each setup keeps its own `on_table`, stock, fixtures and
    `work_offset` (1 = G54, 2 = G55, …).
  - `export_gcode`, `simulate_cam`, `export_setup_sheet` and `export_cam_simulation` on the lead do the whole job
    in one file: tools grouped across stations, and one simulation with both stations on the machine.
    `stations: [...]` keeps only some (first and last cycles).
  - It needs rcnc with per-work-offset Z references (PR 14).
  - Use a machine model with both vise stations' jaws closed (see `jobs/ingot-slider/vise_machines.py`).
  - Set each setup's `on_table` z so that its stock box bottom (which for op 2 includes op 1's facing) is where it
    sits on the vise.
  - Example: `jobs/ingot-slider/cam.py`.
- Run it with `~/src/cad-rust-app/target/release/rcad run cam.json > out 2> err`. Every tool call prints a
  JSON line; summarise them with a small script (see `jobs/phone-stand/adaptive-handoff/variants/rep.py`).
- Try settings as variants of the same JSON, one simulation at a time. Keep the ones that win and say why.
- Examples:
  - `jobs/phone-stand/` (with `adaptive-handoff/` for the scripts);
  - `jobs/ingot-slider/` on `mill/ingot-slider` (~/dmc2/slider);
  - `jobs/dmc2-testpart/`.
