# Ingot slider: how to run it

> **On the mill:** the base programs are in `base/`, the slider programs in `slider/` (each with its setup
> sheets, pictures and STEP). Run order for a batch: `*-first.nc` on a raw blank, then `*-cycle.nc` once per
> blank after it, then `*-last.nc`. The scripts, docs and `cam.py` stay at the top; `cam.py` and `post.py` write into `base/` and `slider/`.

> **Both bars measured (2026-11-20).** The C360 is **10.0 × 25.0** and lies on Jonas's **1.67 mm** shims in op 1; the
> 6061 is **12.34 × 26.6**, no shims, and the base is **8.6** thick (was 9.0) so its seam stays inside the 1.7 chamfer. Both are cut into 101 mm blanks. Anything else measured:
> as 101 blanks. When the aluminium is measured, put the numbers in `BAR_AL` at the top of `cam.py`, then re-run
> `cam.py`, `rcad run cam.json`, `post.py` and `rcad run verify.json`. The asserts in `cam.py` re-check that every
> seam is still inside its chamfer.
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
  | Base (8.6) | Z7.14 | 1.7 on the rail tops (was 1.5) | Z6.9 | 0.24 | Z5.9 |
  | Slider | Z6.47 | 2.4 ingot chamfer (was 2.0) | Z5.6 | 0.87 | Z5.02 |

  The same holds at the part ends and round the corners (R2.5 base, R3 slider): op 1's contour runs the whole
  outline, and the chamfer runs round the corners. Base rail top: 0.8 of its 2.5 width is left flat, 0.5 after
  the 0.3 inner edge break.
- **Op 2's band rough** takes the stock above op 1's walls off to 0.2 under their top (Z6.94 / Z6.27), staying
  0.3 off the walls. That leaves no web between the two parts. The chamfer's cone then takes the 0.3 strip
  and the raw skin together.
- **Brass shims: Jonas's 1.67 mm strips.** On the 10.0 bar the slider's op 1 walls then reach Z6.47 in op 2,
  0.87 over the 2.4 chamfer's lower edge. 1.0 mm is the least that keeps the 0.2 margin. In op 1 the jaws grip
  **3.0 mm** of the brass bar (4.7 − 1.67). To be kind to that grip, the op 1 rough runs in four levels with its
  power capped at 400 W. Its peak is 320 W (it was 415 W in three levels).
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

The programs were made with rCAD branch `cam-build-cache` (PR #165, on `slider-1118` = PR #163): master + `link_feed`
(PR #161) + four commits from this re-plan. Those commits are the deburr tip fix, the deburr corner fix and job-aware `verify_gcode`; see the end
of this file. Run them on rcnc **main** (branch `mill/ingot-slider` is main plus this folder). Main has the bog
guard, Find bores with rotation, and per-work-offset Z references.

| Program | Station | Time (rcnc sim) | Before | Peak spindle vs its caution | Sheet |
|---|---|---|---|---|---|
| `base-op1.nc` | left | 6.2 min | 7.1 | 728 W at 9k / 850 (rough) | `base-op1.html` |
| `base-op2.nc` | right | 9.4 min | 12.9 | 662 W at 9k / 850 (channel rough); face 415 W at 12k / 684 | `base-op2.html` |
| `slider-op1.nc` | left (bar on two 1.67 mm shims) | 8.7 min | 13.4 | 320 W at 18k / 400 cap (rough; caution 501) | `slider-op1.html` |
| `base-cycle.nc` | both (G54 left + G55 right) | **13.3 min** | 17.7 | as the singles | `base-cycle.html` |
| `slider-cycle.nc` | both, with the engraving | **13.8 min** | 23.2 | as the singles | `slider-cycle.html` |

HTML playbacks (`*-sim.html`) and the screenshots taken from them are no longer made (Jonas, 2026-11-19): each
was another full simulation. The older ones were removed so they can't be mistaken for these programs.
`export_cam_simulation` makes one when wanted. The stock pictures are `*-sim.png` / `*-sim-top.png`, and the
toolpaths are `*-paths.png`.

## After the first slider run (2026-11-19)

Jonas cut the slider. The finish was very clean and the magnets were a perfect press fit. What changed after it:

- **Find bores probed too deep.** rCAD took the bores' top from the part (Z8), but they sit in the carrier between
  the parts, whose top is the stock's (Z9.8). The probe went 3.3 mm down instead of the planned 1.5. rCAD is fixed
  (PR #166, 49573d5), and the depth is now **1.0 below the carrier top** on both parts. On the slider the web is
  Z6.47–9.8, so the ball's contact is 2.3 mm above its bottom; on the base the web is Z7.14–12.14 (3.1 mm margin).
- **The ingot chamfer was too heavy in one pass.** It now runs in two passes, 1.7 then 2.4, with the same 0.58
  tip offset and climb. Estimated tangential force in C360 (about 0.7 J/mm³, 18k, F1000): one pass ~10 N; now
  ~4 N then ~6 N. The base's 1.7 rail chamfer cuts about half the cross-section of the old 2.4, in aluminium, so it
  stays one pass.
- **One slider moved in Y (its engraving was off).** Two things changed, with no extra tool change:
  - The heavier cut after the sliders come apart was the band rough, not the chamfer: 311 W at the gap, about
    52 N of tangential force, against ~10 N for the old chamfer. It's now targeted at 120 W, with a 225 W peak
    (~38 N).
  - The engraving runs before the chamfer, straight after the band rough.

  Each separated part is held by its own 45 mm × 4.7 mm of finished wall on both jaws. With the vise done up by
  hand (say 2 kN, split between the two parts, steel on brass μ ≈ 0.15 on both faces) that's ~300 N of friction
  per part, well over these forces. So I suspect one slider wasn't really clamped. Before Cycle Start, check that
  both parts are tight (try to slide each one by hand), and clean the jaws and ledges.
  - The chamfer's direction isn't chosen per side: rCAD runs it climb round the outline, so along the long walls
    it pushes one way on one side and the other way on the other. I didn't change that.
- **The op 1 wall finish was two depths.** rCAD stopped the outside wall pass at the magnet pockets' floor. Fixed
  (PR #166, 49573d5): the slider walls now go in one 6.47 mm full-height pass at 89 W, 0.41 wide (T24 at 24
  stick-out; the flutes are 19). The base's is one 7.14 mm pass (515 W at 12k on the 26.6 bar, caution 684).
- **Find stock from the saved vise location** (rcnc PR 22, `FIND_STOCK VISE=`): no jogging to the stock.
  - **The mill PC needs rcnc 2a7bb0d** (branch `claude/project-thread-k4skge`, or PR 22 once merged). It is on
    2eb5ac6, where Find stock with `VISE=` still waits for you to move over the stock. 2a7bb0d still has PR 14
    (per-offset Z), PR 18 (rotation) and the bog guard (`REQUIRE_GUARD`).
  - **Once, with the vise empty:** Probe page → Vise → Find vise location, then Save. Do it again if the vise is
    moved.
  - **Place each blank across the middle of the jaws** (101 in 95.5 jaws: 2.75 out at each end). Find stock
    starts at the vise's middle Y and searches about ±8 mm, so by eye is enough; a pencil mark at the jaw middle
    helps.
  - op 1 (left station): `VISE=1`. op 2 (right station): `VISE=-1 JAWAT=-12.5` (base) / `-9.8` (slider), because
    op 1's finished wall lies against the jaw, under the wider raw band.
- `slider-op2.nc` and `slider-engrave.nc` are gone. `slider-last.nc` (op 2 + engraving + chamfer) and the cycle
  replace them.

## Blanks, shims and tools

- Blanks are sawn to **101 mm** (brass measured). Op 1 Find stock expects 101 ± 1.0. The parts take 98.0 (two
  45 mm parts and an 8 mm gap), so 1.5 is left at each end. Op 1 roughs and finishes the outer ends down to the
  jaw limit, and op 2's band rough takes the stub above. Each end sticks out 2.75 past the 95.5 jaws.
- Aluminium: 6061 flat bar, **12.34 × 26.6 measured** (bought as 1/2 × 1). Brass: C360, **10.0 × 25.0 measured**. That's 2.7 of
  stock each side of the 19.6 slider.
- **Shims for brass op 1:** two flat **1.67 mm** strips about **25 × 39**, one under each slider. Put them 12 mm
  apart, centred on the bar's middle, so the two carrier bores at the middle break through between them. Both
  strips stay inside the jaws' length (Y ±45 against the jaws' ±47.75). Anything from 1.0 up keeps the seam
  inside the chamfer, and thicker costs grip in op 1.
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
  | Brass | T24, T1 | everything | 18 000 | 1350 rough/face, 1000 finish/T1; the feed schedule raises light cuts (up to 2850 on a 0.66-wide wall shave) | target 300 W; cap 400 in op 1 (3.0 mm grip), 501 in op 2 (the 18k caution) | Mild on purpose: no C360 measurements yet, and the high-helix DLC cutter can pull into C360. The op 1 rough is in four levels (1.6 deep), with a 320 W peak. |

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
1.67 mm shim strips go under it.** The jaws grip 4.7 of the aluminium bar and 3.0 of the brass.

1. Open `base-op1.nc` / `slider-op1.nc`, arm the bog guard, Cycle Start.
2. T99 + Find stock from the saved vise location (`VISE=1`): tap the stylus and press Continue. rcnc moves over
   the stock by itself (X from the fixed jaw and the bar width, Y the vise's middle). No jogging.
   - **Aluminium:** Z comes from the Vise reference.
   - **Brass:** Z comes from the bar's probed top, so Z0 is the bar's bottom on the shims. The guidelines allow
     a probed Z here because op 1 faces that top off.
3. T24 cuts, in this order:
   - Faces the top 0.2 at 12k (brass: 18k, with the passes along the slide (Y), since this is the sliding face).
   - Roughs round both parts, the 8 mm gap and the ends to Z5 (aluminium, two levels of 3.75 at 9k) or 3.5 above
     the blank's bottom (brass, four levels at 18k, capped at 400 W). Where it enters the gap it is close to slotting (5.95 wide
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
finished walls in the jaws. Those walls reach 7.14 mm (aluminium) / 6.47 mm (brass) above the ledge, and the jaws
grip 4.7. The raw band above the walls is wider (26.6 / 25.0): make sure the jaws close on the finished walls, not on it.
Push the bar against the fixed jaw and clamp.

1. Open `base-last.nc` / `slider-last.nc` (or the cycle), arm the bog guard, Cycle Start.
2. T99 + Find stock from the saved vise location (`VISE=-1 JAWAT=`: op 1's finished wall lies against the jaw):
   tap the stylus and Continue, no jogging. Z comes from the Vise reference.
3. **BORES PAUSE:** the program stops with a message. Blow the chips out of the two carrier bores, check their
   mouths for burrs, then Cycle Start. A chip or burr in a bore can fail Find bores.
4. T99 + **Find bores**: X/Y and the turn from the two carrier bores (Ø7.5, 10 apart, TOL 0.1, turn ≤ 0.5°).
   This happens before any cutting. The carrier is located from these bores, not from a raw face.
5. Base:
   - Face to Z8.6 at 12k, two levels of 1.77.
   - Rough the channel to Z5.6 at 9k (walls leave 0.3, floor 0.2).
   - **CHIP PAUSE:** blow the chips out of the two channels and off the top.
   - Finish the channel floor at 12k, passes along the slide (Y) so the tool marks run the way the slider moves.
   - The magnet pockets.
   - Rough the band above op 1's walls off to Z6.94. This **separates the two bases**. It's light: target 120 W, peak 110 W.
   - Finish the channels' walls, Z8.6 to Z5.6, in one pass. It runs straight down both channels and through the cleared gap.
   - T1 at 24k: the **1.7 rail-top chamfers**, outside, round the R2.5 corners to the channel. The tip is at Z5.9.
     Then the 0.3 edge breaks (channel edges, floor ends, pocket rims).
6. Slider:
   - Face to Z8 (1.8 off, 217 W).
   - Rough the band above op 1's walls off to Z6.27. The two sliders **separate during this rough**. Since the
     first run (one slider moved in Y) it's light: target 120 W, peak 225 W where it opens the gap (was 311 W).
   - T1, in the same program: **the engraving first** (both sliders still undisturbed), then the **2.4 ingot
     chamfer in two passes**: 1.7, then 2.4 (Jonas: one pass was too heavy). Same tip offset, the tip at Z5.72
     then Z5.02, just over the jaws' Z5.0.
7. To swap the engraving for a custom one or a name:
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
| Bases | 6.2 + 9.4 = 15.6 min | **13.3 min** | 17.7 | 6 → **3** |
| Sliders | 8.7 + 5.9 + 1.5 = 16.1 min | **13.8 min** | 23.2 | 7 → **3** |

The times come from the rcnc simulator. They include its tool-change and probing moves, but not the time you
spend at the pauses (three in the base cycle, one in the slider cycle) or on the tool swaps.

### First and last programs (batch start and end)

A batch runs: **the first program on a raw blank, then the cycle once per blank after it, then the last
program**. The first and last programs come from the same job as the cycle, posted with `stations` so each has
only one station's work and one tool change per tool:

| Program | What | Station | Time (rcnc sim) | Sheet |
|---|---|---|---|---|
| `base-first.nc` | base op 1 only | left, G54 | 6.2 min | `base-first.html` |
| `base-last.nc` | base op 2 only (Find stock, BORES PAUSE, Find bores) | right, G55 | 9.5 min | `base-last.html` |
| `slider-first.nc` | slider op 1 only (bar on the 1.67 mm shims) | left, G54 | 8.1 min | `slider-first.html` |
| `slider-last.nc` | slider op 2, the engraving, then the 2-pass chamfer, in one program (T99, T24, T1 once each) | right, G55 | 8.1 min | `slider-last.html` |

The last programs set G55 themselves (Find stock, then Find bores), so they run on their own. The header of a
last program still lists station 1 (op 1) in its "stations" comment, but it contains no op 1 moves and no G54.
`base-first` / `slider-first` have the same toolpaths as `base-op1` / `slider-op1` (same SIM OK hash), and
`base-last` matches `base-op2`. Each has a SIM OK stamp and passed `verify_gcode` after post-processing.

### Running batches

1. **First** (right station empty): clamp a raw blank in the left station and run `base-first.nc` /
   `slider-first.nc` (op 1 only, G54).
2. **Every cycle after that:**
   - take the finished parts out of the right station;
   - flip the left station's blank into the right station, onto its finished bottom;
   - clamp a new raw blank in the left station (brass: on the two shim strips);
   - run `base-cycle.nc` / `slider-cycle.nc`.

   Find stock goes to each station from the saved vise location by itself: the left first, then the right.
3. **Last** (left station empty): run `base-last.nc` / `slider-last.nc` (op 2 only, G55; the slider's includes the
   engraving).

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
| base-op1 | none | 728 W at 9k (caution 850): rough entering the gap, 5.91 × 3.75 at F1185 | 26.3 | rough: "expected" (1.35) | ok | none |
| base-op2 | none | 662 W at 9k (850): channel rough, 3.52 × 2.80 at F3000; face 415 W at 12k (684) | 29.6; face 17.1 | ok | channel rough: caution (0.56) | none |
| slider-op1 | none | 320 W at 18k (cap 400, caution 501): rough, X4.5 Y0.5, 4.30 × 1.62 at F1800 | 12.5 | rough: "expected" (1.28) | ok | none |
| slider op 2 (in slider-last / cycle) | none | 225 W at 18k (cap 150 target 120, caution 501): band rough opening the gap, 5.0 × 1.53 at F1050 | – | band rough: caution (0.86) | ok | none |
| base-cycle | none | as the singles | | | | |
| slider-cycle | none | as the singles | | | | |

Notes on the checks:
- **Chatter "expected" on the roughs (margins 1.2–1.4, under 1 is stable).** rCAD's model isn't calibrated to the
  mill (guidelines), and its advice is 22 500 rpm, which the spindle can't power for these cuts (403 W caution at
  24k). The guidelines say to rough at 9k. I kept 9k (aluminium) and the mild 18k (brass), and the bog guard is
  required. Listen on the first part.
- **Depth at 9k:** the base op 1 rough is two levels of 3.75 (7.5 in all). The guidelines prefer 5–7 per level,
  and one 7.5 level would be over 7.
- **How long a re-plan takes now:** `./replan.sh slider` (or `base`, `all`) re-plans one part: CAM, the first
  and last programs, post-processing and `verify_gcode` on that part's programs, in two parallel processes. It
  needs rCAD PRs #165 and #166 (build cache, shared simulation). A shim-only change took **226 s end to end**
  (CAM 119 s, verify 89 s), and the programs came out byte-identical. Before those PRs it took about 35 min. The rcnc
  simulator then takes about 2 s per program.
- **verify_gcode on every final file** (`verify.json`, after pauses, dwells and the guard line): all 7 ok, 0
  problems, stamps match the setups. The cycles were read station by station and simulated together on the
  machine. As a check, a cycle with one Z changed to dig into station 2 was caught as a gouge.
- **rcnc simulator** (rcnc 2a7bb0d; `RCNC_JOB=… cargo test --release -p cnc-machine runs_a_job_file -- --ignored`, with
  `RCNC_BORES="-5,0;5,0"` for op 2, plus a local, uncommitted harness setting `RCNC_VISE_OFF` that saves a vise
  location at the simulated block's faces): all 9 programs ran to the end with the guard line, Find stock from
  the vise location (stock 25.0 / 26.6 × 101.000, the face check against the jaw passing), the pauses, Find bores
  (10.000 apart, 0.000°) and three tool changes each. Times: base-first 6.2, base-last 9.5, base-cycle 13.3,
  slider-first 8.1, slider-last 8.1, slider-cycle 13.8 min.
- **Find stock size:** op 1 expects the bar (25.0 × 101 brass, 26.6 × 101 aluminium, TOL 1.0). The single op 2
  programs expect op 1's simulated leftover on its grid (25.2 × 101.2 brass, 25.6 × 101.2 aluminium), and the
  cycles expect the setup's size. All are inside op 2's TOL 1.5. The rcnc simulator found 25.0 × 101.000 for the
  brass and 25.4 × 101.000 for the aluminium.

## rCAD changes made for this re-plan (branches `slider-1118`, `cam-build-cache`)

- **Deburr tip fix:** the deburr operation put the chamfer mill's tip a whole chamfer depth deeper than
  `tip_offset` said. For the 1.7 rail chamfer the tip went to Z4.6, clamped at the jaws to 5.0, and the shank cut
  a step above a 0.5 chamfer. Now the tip is `tip_offset` under the chamfer's lower edge (Z6.3), as in the
  chamfer operation. A chamfer the cone can't reach at its top is refused.
- **Deburr corner fix:** a picked loop that started mid-edge lost its rounded corner at the start. Two of the four
  rail chamfers missed a corner.
- **verify_gcode for jobs:** a two-station program is read back station by station.
- **Build cache:** a setup's toolpaths, and the stock the setups before it leave, are kept between calls. They're
  keyed by everything they depend on: the chain's setups and the part geometry. Before, every info, picture,
  export and sheet call rebuilt them, and every call on an op 2 setup re-made and re-simulated op 1.
