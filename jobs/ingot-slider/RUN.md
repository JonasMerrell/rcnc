# Ingot slider: how to run it

> **PROVISIONAL (2026-11-18).** Planned for the bars as bought, at their nominal sizes: 6061 1/2 × 1 in
> (12.7 × 25.4) and C360 3/8 × 1 in (9.5 × 25.4). The bars haven't arrived. When they do, measure thickness and
> width, put the numbers in `BAR_AL` / `BAR_BR` at the top of `cam.py`, then re-run `cam.py`, `rcad run cam.json`,
> `post.py` and `rcad run verify.json`. The asserts in `cam.py` re-check that every seam is still inside its chamfer.
> **Simulated only: nothing here has been cut.**

Two 6061 bases per 1/2 × 1 in bar blank and two C360 brass sliders per 3/8 × 1 in bar blank. Each part gets two
vise setups, and the sliders get a separate engraving program. Part sizes, the chamfers and the seam arithmetic
are in `DESIGN.md`. Per-operation tools, feeds, times and loads are in `OPS.md`. The job follows
`jobs/guidelines.md` (rcnc main, 2026-11-18); where it differs, it says so below.

## What changed in this re-plan

- **No vertical wall is cut in two operations.** Op 1 finishes every outside wall full height, down to the jaw
  limit Z5.0. Op 2 cuts nothing on those walls below the top chamfer. The chamfer's lower edge sits 0.2 under
  the top of op 1's wall, so the op 1 / op 2 seam is inside the chamfer and is cut away with it:

  | | Op 1 wall top (op 2 Z) | Chamfer | Chamfer lower edge | Margin | Chamfer tip |
  |---|---|---|---|---|---|
  | Base | Z7.5 | 1.7 on the rail tops (was 1.5) | Z7.3 | 0.2 | Z6.3 |
  | Slider | Z5.8 | 2.4 ingot chamfer (was 2.0) | Z5.6 | 0.2 | Z5.02 |

  The same holds at the part ends and round the corners (R2.5 base, R3 slider): op 1's contour runs the whole
  outline, and the chamfer runs round the corners. Base rail top: 0.8 of its 2.5 width is left flat, 0.5 after
  the 0.3 inner edge break.
- **Op 2's band rough** takes the stock above op 1's walls off to 0.2 under their top (Z7.3 / Z5.6), staying
  0.3 off the walls. That leaves no web between the two parts. The chamfer's cone then takes the 0.3 strip
  and the raw skin together.
- **Brass shims are 1.5 mm (were 1.0).** With 1.5 the slider's op 1 walls reach Z5.8 in op 2, which puts the
  2.4 chamfer's lower edge 0.2 under them. In op 1 the jaws then grip only **3.2 mm** of the brass bar. The cuts
  are light: the roughing peak is 378 W, and the rough runs in three levels.
- **Aluminium op 1 faces 0.2 (was 0.5).** Its walls go to Z5.0 (was 6.5).
- **Spindle at F004 = 150:** aluminium is roughed at **S9000** (target 650 W, under the 850 W caution), with top
  facing and finishing at S12000 and the chamfer mill at S24000 (light). Every S change is followed by
  `G4 P3` in air.
- **Chip pauses before the finishes** that follow the 9k roughing (aluminium, both ops).
- **Op 2 locates from the carrier bores:** Find stock (size from op 1's simulation), the BORES PAUSE to blow them
  out, then Find bores, which also measures the part's rotation (rcnc PR #18).
- **Every program has `(RCNC REQUIRE_GUARD)`.** These cuts go beyond the proven ones (9k at 650–740 W, the new
  brass cuts), so arm rcnc's bog guard first (Settings → Listening, feed hold on). rcnc refuses Cycle Start
  without it.
- **Every program has a SIM OK stamp** from rCAD's export gate. `verify_gcode` re-read every final file (after
  pauses, dwells and the guard line) and found 0 problems. For the two-station cycles, verify now reads each
  station's stretch with its own setup (rCAD branch `slider-1118`).

The programs were made with rCAD branch `slider-1118`: master + `link_feed` (PR #161) + three commits from this
re-plan. Those commits are the deburr tip fix, the deburr corner fix and job-aware `verify_gcode`; see the end
of this file. Run them on rcnc **main** (branch `mill/ingot-slider` is main plus this folder). Main has the bog
guard, Find bores with rotation, and per-work-offset Z references.

| Program | Station | Time (rcnc sim) | Before | Peak spindle vs its caution | Sheet |
|---|---|---|---|---|---|
| `base-op1.nc` | left | 6.1 min | 7.1 | 734 W at 9k / 850 (rough) | `base-op1.html` |
| `base-op2.nc` | right | 9.4 min | 12.9 | 673 W at 9k / 850 (channel rough); face 411 W at 12k / 684 | `base-op2.html` |
| `slider-op1.nc` | left (bar on two 1.5 mm shims) | 8.9 min | 13.4 | 378 W at 18k / 501 (rough) | `slider-op1.html` |
| `slider-op2.nc` | right | 6.0 min | 9.6 | 359 W at 18k / 501 (band rough) | `slider-op2.html` |
| `slider-engrave.nc` | right, straight after `slider-op2` | 1.5 min (rCAD) | about 2.5 | – | `slider-engrave.html` |
| `base-cycle.nc` | both (G54 left + G55 right) | **13.2 min** | 17.7 | as the singles | `base-cycle.html` |
| `slider-cycle.nc` | both, with the engraving | **14.2 min** | 23.2 | as the singles | `slider-cycle.html` |

Playbacks: `*-sim.html` (rCAD simulation with the machine and the vise closed on the work). Pictures:
`*-vise-iso.png`, `*-vise-front.png`, `*-cycle-vise-top.png`.

## Blanks, shims and tools

- Saw blanks to **100 ± 1 mm**. Find stock accepts ± 1.5. The parts take 98.0 (two 45 mm parts and an 8 mm gap).
  Op 1 finishes the outer ends full height and op 2's band rough takes the stub beyond them.
- Aluminium: 6061 1/2 × 1 flat bar (12.7 × 25.4). Brass: C360 3/8 × 1 (9.5 × 25.4).
- **Shims for brass op 1:** two flat **1.5 mm** strips about **25 × 39**, one under each slider. Put them 12 mm
  apart, centred on the bar's middle, so the two carrier bores at the middle break through between them. Both
  strips stay inside the jaws' length. 1.5 is the least that keeps the seam 0.2 inside the 2.4 chamfer. Thicker
  is fine but costs grip in op 1.
- Tools, with stick-outs from the ER16 nut:

  | T | Tool | Stick-out |
  |---|---|---|
  | T24 | 3 FLUTE DLC 1/4 | 24 |
  | T1 | ALUMINUM chamfer 90 | 22 |
  | T99 | YIDOFENG probe | 40 |

  The deepest T24 cut is the aluminium carrier bores: 12.8 below the faced bar top. The nut clears it by 11 mm.
  Flood coolant.
- Speeds:

  | Material | Cutter | Use | rpm | Feed (mm/min) | Power | Why |
  |---|---|---|---|---|---|---|
  | Aluminium | T24 | roughing | **9000** | 3000, feed-scheduled (down to 1185 in the gap) | target 650 W, cap 850 (caution) | 9k is the strong speed (guidelines, F004 = 150) |
  | Aluminium | T24 | top facing, finishing | 12 000 | 3000 face, 2000 finish | under 684 W | chips fling better at 12k |
  | Aluminium | T1 | chamfers, edge breaks | 24 000 | 1850 | light | as guidelines (the "479 m/min" warning is known) |
  | Brass | T24, T1 | everything | 18 000 | 1350 rough/face, 1000 finish/T1, feed-scheduled | target 300 W, cap 501 (the 18k caution) | Mild on purpose: no C360 measurements yet, and the high-helix DLC cutter can pull into C360. The op 1 rough is in three levels (1.9 deep), which takes its peak from 527 W to 378 W. |

  Links run at 5000 mm/min (`link_feed`), cutting at most 3000, rapids 5000. All roughing is climb, in laps
  (adaptive), with `stay_down`.

## Setting the vise

The centre block is the fixed jaw of both stations.

- **Left station (op 1):** the 25.4 bar is clamped between the centre block (on the part's **+X / right**) and the
  moving jaw.
  - Guidelines deviation: the guidelines put the fixed jaw on the left. Op 1 takes X from the bar's own two
    sides, so it doesn't matter which jaw is fixed. Using this station lets a batch run op 1 here and op 2 in
    the right station at the same time.
  - In the simulation the moving jaw's base is 1 hole (25 mm) further in than in the vise CAD, and the jaw
    screw closes it the rest of the way to 25.4.
- **Right station (op 2):** the fixed jaw is on the part's **−X / left**, as in the earlier jobs. The moving jaw
  closes on the finished op 1 walls: 25.0 for the base, 19.6 for the slider.
  - In the simulation the base is moved 4 holes (100 mm) in from the vise CAD's position. The CAD's 118 mm
    station isn't how your vise is set now.
  - If the right station is still set for the 39 mm test-part block, keep that hole and close the screw about
    14 mm more (base) or 20 mm more (slider).
  - **Open point:** the jaws close on finished walls that show on the part, under the chamfer. Use protected or
    soft jaw faces (or a strip of paper/aluminium tape) if bare jaws mark 6061 or brass.
- Clamp on the bottom 4.7 mm, as usual. Nothing stands above the jaw tops: no stops, and the shims are under
  the bar.

## Op 1 (bottom side up), left station

Lay the bar on the ledges, centred along the jaws. Both ends stick out about 2 mm past the jaws. **Brass: the two
1.5 mm shim strips go under it.** The jaws grip 4.7 of the aluminium bar and 3.2 of the brass.

1. Open `base-op1.nc` / `slider-op1.nc`, arm the bog guard, Cycle Start.
2. T99 + Find stock (TOL 1.5, DEPTH 3): jog over the middle, then Continue.
   - **Aluminium:** Z comes from the Vise reference.
   - **Brass:** Z comes from the bar's probed top, so Z0 is the bar's bottom on the shims. The guidelines allow
     a probed Z here because op 1 faces that top off.
3. T24 cuts, in this order:
   - Faces the top 0.2 at 12k (brass: 18k, with the passes along the slide (Y), since this is the sliding face).
   - Roughs round both parts, the 8 mm gap and the ends to Z5 (aluminium, two levels of 3.75 at 9k) or 3.5 above
     the blank's bottom (brass, three levels at 18k). Where it enters the gap it is close to slotting (5.95 wide
     in aluminium). The feed schedule drops to 1185 mm/min there, and that's the 734 W peak.
   - **Aluminium: CHIP PAUSE.** Spindle and coolant stop. Blow the chips off the bar and out of the gap, then
     Cycle Start.
   - One full-height finish pass on all the walls, down to the jaw limit. These walls are final: op 2 grips
     them and never cuts them again.
   - Brass only: the 6 magnet pockets.
   - **Two Ø7.5 locating bores through the carrier in the gap, at X ±5 Y0.** Op 2 locates from these, and op 2
     cuts them away.
4. T1 cuts a 0.3 edge break on the bottom outline (brass: and the pocket rims). It's kept at 0.3 because op 2 stands
   the parts on the 1.9 mm ledges: that leaves **1.6 mm of land on each ledge**, and 4.4 of the 4.7 mm grip.

## Op 2 (top side up), right station

Turn the blank over, either way (the carrier bores are symmetric about the middle). Wipe the ledges clean first:
the parts sit on 1.6 mm of land on each 1.9 ledge. The finished bottom goes down on the ledges, with op 1's
finished walls in the jaws. Those walls reach 7.5 mm (aluminium) / 5.8 mm (brass) above the ledge, and the jaws
grip 4.7. The raw band above the walls is wider (25.4): make sure the jaws close on the finished walls, not on it.
Push the bar against the fixed jaw and clamp.

1. Open `base-op2.nc` / `slider-op2.nc`, arm the bog guard, Cycle Start.
2. T99 + Find stock: a rough X/Y (it expects the size op 1's simulation left). Z comes from the Vise reference.
3. **BORES PAUSE:** the program stops with a message. Blow the chips out of the two carrier bores, check their
   mouths for burrs, then Cycle Start. A chip or burr in a bore can fail Find bores.
4. T99 + **Find bores**: X/Y and the turn from the two carrier bores (Ø7.5, 10 apart, TOL 0.1, turn ≤ 0.5°).
   This happens before any cutting. The carrier is located from these bores, not from a raw face.
5. Base:
   - Face to Z9 at 12k, two levels of 1.75.
   - Rough the channel to Z6 at 9k (walls leave 0.3, floor 0.2).
   - **CHIP PAUSE:** blow the chips out of the two channels and off the top.
   - Finish the channel floor at 12k, passes along the slide (Y) so the tool marks run the way the slider moves.
   - The magnet pockets.
   - Rough the band above op 1's walls off to Z7.3. This **separates the two bases**. It's light (198 W).
   - Finish the channels' walls, Z9 to Z6, in one pass. It runs straight down both channels and through the cleared gap.
   - T1 at 24k: the **1.7 rail-top chamfers**, outside, round the R2.5 corners to the channel. The tip is at Z6.3.
     Then the 0.3 edge breaks (channel edges, floor ends, pocket rims).
6. Slider:
   - Face to Z8 (1.3 off).
   - Rough the band above op 1's walls off to Z5.6. The two sliders **separate during this rough**.
   - T1: the **2.4 ingot chamfer** round the top. The tip is at Z5.02, just over the jaws' Z5.0.
7. Sliders: without unclamping, open `slider-engrave.nc` and run it. It uses T1 again, the same work offset and
   no probing. To swap the pattern for a custom one or a name:
   - put a sketch on the top field (14.8 × 40.2 flat; keep 1 mm in from the chamfer: 12.8 × 38.2);
   - add a `trace` op (lines) or a `vcarve` op (text) in the "Slider op2 engrave" tab of `slider-cam.json`.

**Separation risk:** after the separating cut, each part is held only by its own 45 mm of 4.7 mm grip. After that
come only the base's wall finish (18 W) and the chamfers. Both parts of a blank were finished in the same op 1
pass, so they're the same width and the jaws hold both. Check the jaw is tight before Cycle Start.

## Production: the two-station cycle (`base-cycle.nc`, `slider-cycle.nc`)

Each cycle runs one blank's op 1 in the **left station (G54)** and the previous blank's op 2 in the **right station
(G55)** in a single program, ordered by tool, so each cutter goes in once:

| Tool | Left station (G54) | Right station (G55) |
|---|---|---|
| T99 | Find stock (raw bar) | Find stock, **BORES PAUSE**, Find bores |
| T24 | all of op 1 (aluminium: CHIP PAUSE before the wall finish) | all of op 2 (aluminium: CHIP PAUSE before the floor finish) |
| T1 | op 1 edge breaks | op 2 chamfers, then (sliders) the crosshatch engraving |

**The programs come straight from rCAD.** In `slider-cam.json` the op 1 setup leads a *job*: `with: ["Base op2 top"]`
for the bases, or the op 2 and engraving setups for the sliders. Each setup keeps its own station (`on_table`),
work offset, stock and fixtures. `export_gcode` on the op 1 setup posts the whole job, and
`stations: ["Base op1 bottom"]` posts op 1 alone.

| | Separate programs (rcnc sim) | Cycle (rcnc sim) | Before | Tool changes |
|---|---|---|---|---|
| Bases | 6.1 + 9.4 = 15.5 min | **13.2 min** | 17.7 | 6 → **3** |
| Sliders | 8.9 + 6.0 + 1.5 = 16.4 min | **14.2 min** | 23.2 | 7 → **3** |

The times come from the rcnc simulator. They include its tool-change and probing moves, but not the time you
spend at the pauses (three in the base cycle, one in the slider cycle) or on the tool swaps.

### Running batches

1. **First cycle** (right station empty): clamp a raw blank in the left station and run `base-op1.nc` /
   `slider-op1.nc`. Each is posted from the job with only op 1 and is G54 only.
2. **Every cycle after that:**
   - take the finished parts out of the right station;
   - flip the left station's blank into the right station, onto its finished bottom;
   - clamp a new raw blank in the left station (brass: on the two shim strips);
   - run `base-cycle.nc` / `slider-cycle.nc`.

   Find stock asks you to jog over each station in turn: the left first, then the right.
3. **Last cycle** (left station empty): run `base-op2.nc` / `slider-op2.nc` and then `slider-engrave.nc` (G55 only).

Within a tool, the program goes up to `G53 G0 Z0` between stations, switches the work offset, travels over at the
top and comes down over the other station.

## Magnets

N52 (or N35/N42) Ø8 × 3, glued 0.15 below flush. For detents, every base magnet goes N up and every slider magnet
S down. Try it dry first.

## Checks

Quoted from rCAD's simulation (details in `OPS.md`). Every setup has the jaws closed on the work as machine parts
and as fixture boxes, and the toolpaths stay 0.3 over them.

| Program | Problems | Peak spindle (where) | cm³/min there | Chatter | Chips | Unfinished |
|---|---|---|---|---|---|---|
| base-op1 | none | 734 W at 9k (caution 850): rough entering the gap, X−7.6 Y0.5, 5.95 × 3.75 at F1185 | 26.4 | rough: "expected" (1.62) | ok | none |
| base-op2 | none | 673 W at 9k (850): channel rough, X−1.7 Y41.6, 3.58 × 2.80 at F3000; face 411 W at 12k (684) | 30.1; face 17.0 | ok | channel rough: caution (0.56) | none |
| slider-op1 | none | 378 W at 18k (501): rough, X5.7 Y−0.5, 3.26 × 3.87 at F1050 | 13.2 | rough: "expected" (1.54) | ok | none |
| slider-op2 | none | 359 W at 18k (501): band rough, X−3.2 Y0.4, 5.55 × 2.20 at F1050 | 12.8 | band rough: "expected" (1.34) | ok | none |
| slider-engrave | none | – | – | ok | ok | none |
| base-cycle | none | as the singles | | | | |
| slider-cycle | none | as the singles | | | | |

Notes on the checks:
- **Chatter "expected" on the roughs (margins 1.3–1.6, under 1 is stable).** rCAD's model isn't calibrated to the
  mill (guidelines), and its advice is 22 500 rpm, which the spindle can't power for these cuts (403 W caution at
  24k). The guidelines say to rough at 9k. I kept 9k (aluminium) and the mild 18k (brass), and the bog guard is
  required. Listen on the first part.
- **Depth at 9k:** the base op 1 rough is two levels of 3.75 (7.5 in all). The guidelines prefer 5–7 per level,
  and one 7.5 level would be over 7.
- **verify_gcode on every final file** (`verify.json`, after pauses, dwells and the guard line): all 7 ok, 0
  problems, stamps match the setups. The cycles were read station by station and simulated together on the
  machine. As a check, a cycle with one Z changed to dig into station 2 was caught as a gouge.
- **rcnc simulator** (`RCNC_JOB=… cargo test --release -p cnc-machine runs_a_job_file -- --ignored`, with
  `RCNC_BORES="-5,0;5,0"` for op 2): all six programs that probe ran to the end with the guard line, Find stock,
  the pauses, Find bores (bores 10.000 apart, 0.000° turn) and three tool changes each. `slider-engrave` alone
  can't run there, because G55 isn't set without op 2 before it. It ran inside `slider-cycle`.
- **Find stock size in op 2:** the single op 2 programs expect 25.6 × 100.2 (op 1's simulated leftover, on its
  0.6 mm grid). The cycles expect the setup's 25.4 × 100. Both are inside TOL 1.5.

## rCAD changes made for this re-plan (branch `slider-1118`)

- **Deburr tip fix:** the deburr operation put the chamfer mill's tip a whole chamfer depth deeper than
  `tip_offset` said. For the 1.7 rail chamfer the tip went to Z4.6, clamped at the jaws to 5.0, and the shank cut
  a step above a 0.5 chamfer. Now the tip is `tip_offset` under the chamfer's lower edge (Z6.3), as in the
  chamfer operation. A chamfer the cone can't reach at its top is refused.
- **Deburr corner fix:** a picked loop that started mid-edge lost its rounded corner at the start. Two of the four
  rail chamfers missed a corner.
- **verify_gcode for jobs:** a two-station program is read back station by station.
