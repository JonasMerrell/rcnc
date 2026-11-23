# Handoff: Eclipse car slider, thick-rail base, and the brass magnet pockets (2026-11-24)

These notes are for whoever picks up the ingot-slider job next: Claude or Jonas. The branch is
`mill/ingot-slider-vise` at fca5cd0. Nothing here was sent to the mill PC by an agent, because netdash has no route to
it; the mill PC has to pull the branch itself.

## What to run

| Part | Material, per blank | Programs, in batch order | Folder |
|---|---|---|---|
| Eclipse car | C360 brass 10 × 25 × 101, 2 cars | `eclipse-car-first.nc` → `eclipse-car-cycle.nc` per blank → `eclipse-car-last.nc` | `eclipse-car/` |
| Thick-rail car base | 6061 12.34 × 26.6 × 101, 2 bases | `eclipse-base-thick-first.nc` → `-cycle.nc` → `-last.nc` | `eclipse-base-thick/` |
| Normal ingot slider | C360 brass, 2 sliders | `slider-first.nc` → `slider-cycle.nc` → `slider-last.nc` | `slider/` |
| Normal ingot base | 6061, 2 bases | unchanged, `base/` | `base/` |

**Do not use:** `eclipse/` and `eclipse-base/`. They are the superseded v3 car and its centre-rail base.

**Blank length:** the programs are posted for 101 mm blanks. Find stock accepts 100.0–102.0 on op 1 (TOL 1). Two
parts span 98 mm, so anything shorter than 100 needs a re-post with the measured length.

**Mixed blanks:** none. Jonas decided brass blanks stay unmixed (two cars, or two sliders) and the aluminium
blanks are two car bases. One car with one normal slider in a brass blank would need packers in op 2: the car is
17.3 wide and the slider 19.6.

## Hashes and times (sha256 prefix; rCAD time / rcnc simulator)

| Program | sha256 | rCAD time | rcnc simulator |
|---|---|---|---|
| eclipse-car-first / -op1 | 2737b8b84d98 | 5:46 | 544 s |
| eclipse-car-cycle | 54e5a9a4e38e | 26:13 | 2023 s |
| eclipse-car-last | 6ffb8aab62d0 | 20:28 | 1622 s (earlier run; moves unchanged) |
| eclipse-car-op2 | 8b76f97356e5 | 20:44 | 1622 s |
| eclipse-base-thick-first / -op1 | a6462995895f | 2:36 | 359 s |
| eclipse-base-thick-cycle | 3a2e5390eaff | 7:11 | 750 s |
| eclipse-base-thick-last | da709e0a995e | 4:35 | 529 s |
| eclipse-base-thick-op2 | aa8297c9f3bf | 4:56 | 529 s |
| slider-first / -op1 | c761fc5b4397 | 4:57 | 491 s |
| slider-cycle | 3dbdf5ac75c1 | 9:16 | 835 s |
| slider-last | f28d459fc8a0 | 4:18 | 488 s |

All of these carry rCAD's SIM OK stamp. The checks behind it ran at the fine setting: 1000 cells along the
stock, in both the export gate and `verify_gcode`.
- **`verify_gcode`:** ran on every final file, after the post-processing scripts. All ok, 0 problems.
- **rcnc simulator:** each program ran to the end. That is rcnc 2a7bb0d with the local `RCNC_VISE_OFF` harness
  (`RCNC_VISE_ADJ=-1`) and `RCNC_BORES="-5,0;5,0"` for op 2. The cycles ran on a single block, 11.47 high.
- **In every program:** `(RCNC REQUIRE_GUARD)`, 3 s dwells after each speed change, Find stock from the saved vise
  location, and a blow-out pause before Find bores.
- **Z references:** only op 1 of a cycle takes Z from Find stock; op 2 uses the Vise reference.

## The Eclipse car: what was decided

**Model.** The car is the approved solid `eclipse-design/eclipse-car-solid-smooth.stl` (design commit c0ad1f0,
cherry-picked here), imported into rCAD as-is. Its deviation from the approved height map is at most 0.087 mm.
The nose (R4.8) and tail (R2.4) corner walls are smooth. The first version used `eclipse-car-top-0.15mm.stl`,
which had stair-stepped corners; Jonas flagged both corners.

**Op 1:** as the ingot slider. The bar sits on the 1.67 mm shims, Z comes from the probed top, and the walls are
cut full height in one pass.

**Op 2: workholding and roughing.**
- Op 2 also stands on 1.67 mm shims (Jonas confirmed they fit). Only 3.0 mm of the car is in the jaws.
- The shim strips under the car must be narrower than the car (17.3); the model has 16 × 39.
- Every T24 cut in op 2 targets 100 W and is capped at 120 W (Jonas: op 2 cuts must be very light on the shims).
  Peaks are 36 W on the face, 110 W on the rough while the cars are joined, and 100 W on the separating rough.

**Op 2: finishing with T44.**
- T44 is a SpeTool W01004 tapered ball: R1 tip, 2.24° per side, 18 mm stick-out (Jonas: "can be whatever").
- **The feed, F1500 at 24k, is still assumed and not confirmed by Jonas.** At F1000 the cycle would take
  31.9 min; at F2000, 23.3 min.
- The passes are a semi-finish, a parallel finish at 0.07 mm stepover, a waterline at 0.069 mm stepdown, and a
  pencil pass. The ball never goes below car z 3.85.

**T1 edge breaks.**
- T1 breaks the tail lip, and the straight part of the nose top edge with a trace from x −3.6 to +3.6.
- **No T1 break** on the nose corners (rCAD's deburr ran onto the fenders), on the tail-end edge (the bumper slope
  and lamp panel are in the way), or on the side edges (they aren't sharp). The ball passes run over all of them.

**Left unfinished.** Only what an R1 ball can't reach:
- The mirror bases: up to 0.44 mm on 20 mm².
- The hood shut line and body creases, which stay soft (they are not engraved).
- The steps at the glass edges.
- The foot of the lamp panel: at most 0.11 mm.

See `eclipse-car-op2-ballsim.png` and `eclipse-car-op2-cut-vs-approved.png`.

**Nose top-edge facet lines.** These are ripples in the surface data itself: about 0.02 mm along the edge, and
0.04–0.05 mm where the corner meets the side. They are too fine for the R1 ball to follow. Whether the surface
data needs another pass is Jonas's call; nobody has decided.

**Brass over the nose pocket:** 1.53 mm at its front rim.

## The brass magnet pockets (slider and car), 2026-11-22

**Problem.** The fit was great on the first two sliders and too tight on the rest. The cause: the cutter is
re-seated in the collet at every tool change (Jonas). The old pocket was a single helix plus a bottom lap that
restarted at 0°, which left a wedge up to 0.17 mm high at the floor edge.

**Fix (brass only).** Pockets are now **8.07** × 3.15, cut as:
- a helix 0.1 mm inside the wall;
- a full lap at size at full depth;
- a spring lap at the same size.

The aluminium bases stay 8.05 and were not re-posted. A magnet that comes out loose gets a drop of glue.

**Not yet measured:** nobody has checked the real spread between seatings. Measure a pocket from the first blank
and one from a later blank with a pin gauge.

## rCAD

**PR #175** (https://github.com/JonasMerrell/cad-rust-app/pull/175): branch `waterline-chords`, on
`cam-job-order` d6902ae. **Open, not merged.** Every program above was posted with a local build of that branch,
`~/dmc2/eclipse-car/bin/rcad` (063b2ff). Re-posting with a binary that lacks these fixes would bring the bugs back.

| Fix | Commit |
|---|---|
| Waterline loops took chords across narrow features on meshes (gouged up to 0.76 mm). | 1709c13, 159de47, 37939a1 |
| The post fits arcs to 3D finishing and deburr passes only where the points lie on them. | 20ead45 |
| The simulation read a tapered ball's flank up to 0.05 mm low, which flagged false gouges. | a5db36c |
| Bore laps continue from the helix; `spring_pass` adds the finish lap and the spring lap. | 063b2ff |

## Sources on netdash

**Car and thick-rail base:** `~/dmc2/eclipse-car/` (copies in `eclipse-car/src/`).
- The pipeline is `mkstl.py` → `build.py` → `cam.py` (`./tsrun.sh cam.json cam`) → `post.py` → `verify.json`.
- `cam-car.json` re-plans the car only. Verify it against `eclipse-car-cam-caronly.json` with `verify-car.json`.
- Checks: `ballsim.py`, `balllimit.py`, `cornercheck.py`, `deviation3d.py`, `contourcheck.py`, `feedtimes.py`.

**Normal slider and base:** `~/dmc2/slider/`. Re-plan with `RCAD=~/dmc2/eclipse-car/bin/rcad ./replan.sh slider`
(or `base`). `build.py` now has `SLIDER_POCKET_D = 8.07`.

**rCAD build:** `~/src/cad-rust-app-slider`. That worktree's own `target/release/rcad` is the older d6902ae; the
fixed binary is the copy in `~/dmc2/eclipse-car/bin/rcad`.

**rcnc simulator:** run from `~/src/rcnc-vise`, which holds the uncommitted `RCNC_VISE_OFF` harness.

**Gotcha:** don't `pkill -f` or `pgrep -f` a pattern that also appears in your own command line. It matches and
kills the calling shell.

## Open items

1. Confirm the T44 feed (F1500 posted).
2. Measure the 8.07 pockets (pin gauge, first blank against a later one).
3. Review and merge rCAD PR #175.
4. Decide whether the nose top-edge facet lines (0.02–0.05 mm) need another pass of the surface data.
5. After the first car op 2, report how the 120 W roughing held on the 3.0 mm grip, and whether the parts moved.
