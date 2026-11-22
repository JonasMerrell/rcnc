# Ingot slider: Eclipse car on the approved surface (brass), rides `../eclipse-base-thick`

This is the car Jonas approved from renders on 2026-11-21 (`../eclipse-design/SPEC.md`, with the addendum for
smooth corners). It is built from the approved solid itself, not re-traced. Nothing was added: no spoiler, wing,
vents, logos or badges.

## Model

- rCAD imports `../eclipse-design/eclipse-car-solid-smooth.stl` (design commit c0ad1f0) as a solid. That file is
  closed as delivered, with exact walls on the outline (45 × 17.3, R4.8 nose, R2.4 tail, in 1.5° facets).
  - `src/mkstl.py` turns it for the vise: length along Y, nose at +Y.
  - `src/build.py` cuts the three magnet pockets (8.05 × 3.15 at −15 / 0 / +15, exact cylinders).
  - No trimming is needed. The nose and tail corner walls are smooth: see `model-nose-corner.png` and
    `model-tail-corner.png`.
- **Deviation from the approved height map:** I measured every map point inside the outline to the nearest point
  of rCAD's own export of the model (`src/deviation3d.py`).
  - Largest: **0.087 mm**, at the tail lamp-panel cliff (x 21.54, y 6.45).
  - Mean 0.002 mm, 99 % within 0.027 mm.
- **Facet lines on the nose top edge** (seen in the close-up): they are ripples in the surface data itself, not in
  the walls.
  - Along the edge, 0.15–0.5 mm in from it: about **0.02 mm** high (0.04 mm per 0.1 mm step at most).
  - Right at the nose's corner joint with the side: up to **0.04–0.05 mm**.
  - They are about 0.1–0.2 mm long, finer than the R1 ball can follow, so the ball smooths them in the cut.
- Pictures:
  - `eclipse-car-model-vs-approved.png`: my model under the approved side view.
  - `model-nose-corner.png`, `model-tail-corner.png`, `car-iso.png`, `car-top.png`.
  - `asm-iso.png`, `asm-front.png`: the car on the thick-rail base.
- **Brass left over the magnets:** 4.88 mm over the middle pocket and 3.44 over the tail one, but only **1.53 mm at
  the nose pocket's front rim** (2.5 over its centre).

## Programs (two cars per 101 mm blank, both vise stations, as the ingot slider)

| Program | What | rCAD time | rcnc simulator | sha256 |
|---|---|---|---|---|
| `eclipse-car-first.nc` | op 1 alone, left station (G54): first blank | 5:42 | 539 s, ran to the end | `0df6ff8d40e5` |
| `eclipse-car-cycle.nc` | op 1 left (G54) + op 2 right (G55): every blank after it | 26:10 | 2017 s, ran to the end | `80f790b67a12` |
| `eclipse-car-last.nc` | op 2 alone, right station (G55): last blank | 20:28 | 1622 s, ran to the end | `2e675bfcada9` |
| `eclipse-car-op1.nc`, `eclipse-car-op2.nc` | the two ops as single programs | 5:42 / 20:44 | 539 / 1622 s | `0df6ff8d40e5` / `8b76f97356e5` |

**Checks:**
- Every program has the SIM OK stamp (0 problems). Since this version they are checked at the fine setting: rCAD's
  stock simulation at 1000 cells along the stock instead of 500.
- Every program carries `(RCNC REQUIRE_GUARD)` and 3 s dwells after each S change.
- Find stock runs from the saved vise location: op 1 `VISE=1`, op 2 `VISE=-1 JAWAT=-8.65`.
- Op 2 pauses before Find bores so the carrier bores can be blown out.
- `verify_gcode` on the final files: **all 5 ok, 0 problems, stamps match the setups** (at the fine setting).

**Z:**
- Op 1 stands on the two 1.67 shim strips under the bar, as the ingot slider, and takes Z from the probed top,
  which it faces.
- Op 2 takes Z from the Vise reference (`Z_REFERENCE Vise`, `KEEPZ=1`), with its shims in the model.
- So **only op 1 of a cycle takes Z from Find stock**; the two stations never both do.

## Tools

| T | Tool | Stick-out | Speeds |
|---|---|---|---|
| T24 | 3 FLUTE DLC 1/4 | 24 | brass as the ingot slider: 18 000 rpm, rough to the power budget (op 1 cap 400 W) |
| T44 | **SpeTool W01004 tapered ball**, R1.0 tip, 2.24° per side, 2 flutes, 15 flute, 1/8 shank, 38.1 long, TiAlN | **18 (ASSUMED)** | 24 000 rpm, **F1500 (ASSUMED)**, plunge F750 |
| T1 | ALUMINUM chamfer 90 | 22 | 18 000 rpm |
| T99 | YIDOFENG touch probe | 40 | |

**T44 assumptions (not confirmed by Jonas):**
- Stick-out 18 = the 15 mm flute + 3 (20 mm in the ER16 collet).
- F1500 at 24k is 0.031 mm/tooth nominal; at the 0.07 finishing stepover the real chip is a few microns.

**Cycle times at other T44 feeds** (feed and plunge scaled together, everything else as planned):

| | T44 F1000 | T44 F1500 (posted) | T44 F2000 |
|---|---|---|---|
| op 2 / last | 26.5 min | 20.7 | 17.8 |
| cycle | 31.9 min | 26.2 | 23.3 |

## Op 1 (bottom up, left station, on the 1.67 shims; T99, T24, T1)

1. Face 0.2.
2. Rough round both cars, the gap and the ends to Z3.33: 4 levels, laps, capped at 400 W.
3. **Walls in one full-height pass** to Z3.33, which is car z 6.47.
4. Magnet pockets.
5. Two Ø7.5 carrier bores in the gap.
6. T1 0.3 edge break of the bottom outline and the pocket rims.

Simulation: 0 problems. Peak 331 W at 18k (the rough, entering the gap; caution 501 W). Chatter "expected" on the
rough (margin 1.28), as on the ingot slider's op 1, which ran. Nothing unfinished.

## Op 2 (top up, right station, on 1.67 shims; T99, T24, T44, T1)

1. Find stock, blow-out pause, Find bores.
2. Face to the roof (car z 9.5).
3. Rough while the band still joins the cars, down to car z 6.52: **light**, 100 W target, capped at 120 W (peak 110 W).
4. Light rough to car z 3.9 that separates the cars: 100 W target, cap 120 W (peak 100 W).

   Every T24 cut in op 2 stays light because op 2 stands on the shims with only 3.0 mm of the car in the jaws
   (Jonas, 2026-11-22). Peaks: face 36 W, joined rough 110 W, separating rough 100 W; T44 passes 1–3 W; all at
   18 000 rpm for T24, climb, in laps.
5. T44 semi-finish: parallel, 0.25 stepover, leaves 0.05.
6. T44 finish along the car on slopes ≤ 45°, **0.07 stepover (0.035 D)**.
7. T44 waterline on the steep faces (≥ 40°), 0.069 apart.
8. T44 pencil along the creases.
9. T1 edge breaks (below).

The ball never goes below car z 3.85 (work Z5.52; the jaw tops are at 4.7). The roof flat (z 9.5) is the T24 face;
the 3.9 nose band is the light rough's floor.

**T1 edge breaks, and which edges don't get one:**
- Op 1: the whole bottom outline and the three pocket rims (0.3).
- Op 2: the tail lip (0.2, deburr), and the **straight part of the nose top edge** at car z 3.9 (0.2: a trace
  with the tip on the edge, from x −3.6 to +3.6).
- **Left without a T1 break:**
  - **The R4.8 nose corners' top edge.** rCAD's deburr ran on round the smooth corners onto the rising fenders and
    cut into them (0.07–0.1), so the T1 pass stops where the corners start.
  - **The whole tail-end top edge at 3.9** (straight and corners). It is 0.2 from the bumper slope and 0.9 from
    the lamp panel, so the cone would hit them.
  - **The side top edges.** They are not sharp: the wall runs up into the rounded shoulder.
  - The ball's passes run over all of these edges.

**Shims for op 2 (check):** the jaws close on the car itself (17.3 wide), so op 2's shim strips must be narrower
than the car. The model has two 16 × 39 strips. The moving jaw closes 2.3 mm further than for the 19.6 ingot
slider.

**What is left unfinished:**
- **rCAD's own check:** 20 mm² carrying up to 0.44 mm, **at the mirror bases** (about 7.3 mm from the car's middle,
  5.5 back from the windscreen base). That is the R1 ball's own limit, not a missed pass: the best an R1 ball can
  do there (`src/balllimit.py`) leaves up to 0.5.
- **My own check** (`src/ballsim.py`): a 0.025 mm z-map of every T44 move in the posted op 2 against the approved
  surface, measured along the surface normal.
  - Nothing is cut into the surface (−0.024 at worst, which is the z-map's own step).
  - Where the ball cuts: mean +0.008 mm, 99 % within 0.15 mm.
  - Off the roof, more than 0.1 mm is left on 11.5 mm²; the R1 ball's own limit is 4.8 mm². Those places are:
    - the hood shut line and the body creases (0.2-deep V grooves in the height map, left soft, not engraved);
    - the steps at the glass edges;
    - the mirror bases.
  - **Tail corners:** at most 0.10 mm left, on 0.4 mm².
  - **Tail-end band:** at most 0.11 mm, on 2.9 mm² over 0.05 mm, in the concave foot of the lamp panel.
- Pictures: `eclipse-car-op2-cut-vs-approved.png` (hill-shaded, approved over simulated) and
  `eclipse-car-op2-ballsim.png` (the difference).

## rCAD changes made for this job (PR #175, branch `waterline-chords`, on `cam-job-order` d6902ae)

- **Waterline chords.** On the mesh, rCAD's waterline loops took chords across proud features narrower than their
  grid and cut up to 0.76 mm into the car. Each chord is now checked along its length: a dip is moved to the nearest
  clear spot, or the loop breaks there. Dips under 3 µm are left.
- **Arcs on 3D finishing.** The post fits arcs to parallel, waterline, scallop, pencil and deburr passes only where
  the points already lie on them (bulge 0.002 mm, not 0.02).
- **Simulation flank height.** The stock simulation read the tool's height at the 0.002 mm table step below each
  cell's distance. On the tapered ball's flank (25.6 mm rise per mm) that is up to 0.05 mm low, so it flagged
  0.03–0.05 "gouges" on op 1's vertical tail-corner walls that the flank only touches. An exact ball-and-flank
  check of the posted path gives 0.002–0.005 mm clearance there. The simulation now interpolates between steps.
  A test runs a tapered ball beside a tall wall: before the fix it flagged a false 0.046 mm gouge; with it, the
  0.0002 clear case is clean and a 0.05 mm-deep case is still flagged.
- The programs were posted with `bin/rcad` built from that branch (a5db36c), which is not merged yet.

## Sources

`src/` holds copies of the scripts. They live in `~/dmc2/eclipse-car` on netdash and use absolute paths.
- Pipeline: `mkstl.py` → `build.py` (rCAD CAD) → `cam.py` (rCAD CAM; run with `tsrun.sh cam.json cam`) →
  `post.py` → `verify.json`.
- Checks: `deviation3d.py`, `cornercheck.py`, `contourcheck.py`, `ballsim.py`, `balllimit.py`, `shade.py`,
  `feedtimes.py`.
- rcnc simulator: rcnc 2a7bb0d with the local `RCNC_VISE_OFF` harness (`RCNC_VISE_ADJ=-1`), and op 2 with
  `RCNC_BORES="-5,0;5,0"`. The cycle ran on one 25 × 101 × 11.47 block, because the harness has one block for
  both stations.

Nothing was sent to the mill PC.
