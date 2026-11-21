# Ingot slider: Eclipse car on the approved surface (brass) — rides `../eclipse-base-thick`

The car Jonas approved from renders on 2026-11-21 (`../eclipse-design/SPEC.md`, rcnc commit 25ebef5), built from
that surface itself, not re-traced: rCAD imports `eclipse-car-top-0.15mm.stl` as a solid. Nothing was added (no
spoiler, wing or vents, no logos).

- **Model:** `src/clean.py` removes 785 pairs of opposite-facing triangles the STL carries on its bottom (cells
  outside the outline lying at z 0; the STL was not closed without that, volume unchanged 5250.425 mm³),
  `src/mkstl.py` turns it for the vise (length along Y, nose at +Y), `src/build.py` imports it into rCAD, trims
  the plan to the exact outline (45 × 17.3, R4.8 nose, R2.4 tail: the STL's corners are its 0.15 mm grid,
  stair-stepped up to 0.27 mm outside those arcs, never inside) and cuts the three magnet pockets
  (8.05 × 3.15 at −15 / 0 / +15, exact cylinders).
- **Deviation of the model from the approved height map** (every map point inside the outline to the nearest
  point of rCAD's own export of the model, `src/deviation3d.py`): **largest 0.108 mm** (at the tail, x 21.0
  y 8.44 z 6.29, where the lamp panel drops 2.2 mm in 0.1 mm and the 0.1 map and 0.15 STL sample the cliff in
  different places), mean 0.002, 99.9 % within 0.059.
- Pictures: `model-vs-approved.png` (my model under the approved side view), `car-iso.png`, `car-top.png`,
  `asm-iso.png` / `asm-front.png` (on the thick-rail base).
- **Brass left over the magnets:** 4.88 mm over the middle pocket, 3.44 over the tail one, **1.53 mm at the
  nose pocket's front rim** (the nose slopes to 4.68 at x −19.0 over it; 2.5 over its centre).

## Programs (two cars per 101 mm blank, both vise stations, as the ingot slider)

| Program | What | rCAD time | rcnc simulator | sha256 |
|---|---|---|---|---|
| `eclipse-car-first.nc` | op 1 alone, left station (G54): first blank | 5:44 | 539 s, ran to the end | `aede1afb319d` |
| `eclipse-car-cycle.nc` | op 1 left (G54) + op 2 right (G55): every blank after it | 24:23 | 1904 s, ran to the end | `c9d6697a2e5a` |
| `eclipse-car-last.nc` | op 2 alone, right station (G55): last blank | 18:40 | 1508 s, ran to the end | `8af848305303` |
| `eclipse-car-op1.nc`, `eclipse-car-op2.nc` | the two ops as single programs | 5:44 / 18:55 | 539 / 1508 s | `aede1afb319d` / `e326c544d0e9` |

Every program has the SIM OK stamp (0 problems), `(RCNC REQUIRE_GUARD)`, 3 s dwells after each S change, Find
stock from the saved vise location (op 1 `VISE=1`, op 2 `VISE=-1 JAWAT=-8.65`), and before op 2's Find bores a
pause to blow the carrier bores out. `verify_gcode` on the final files: **all 5 ok, 0 problems, stamps match the setups**.

**Z:** op 1 stands on the two 1.67 shim strips under the bar (as the ingot slider) and takes Z from the probed top,
which it faces. Op 2 takes Z from the Vise reference (`Z_REFERENCE Vise`, `KEEPZ=1`) with its shims in the model.
So **only op 1 of a cycle takes Z from Find stock**; the two stations never both do.

## Tools

| T | Tool | Stick-out | Speeds |
|---|---|---|---|
| T24 | 3 FLUTE DLC 1/4 | 24 | brass as the ingot slider: 18 000 rpm, rough to the power budget (op 1 cap 400 W) |
| T44 | **SpeTool W01004 tapered ball**, R1.0 tip, 2.24° per side, 2 flutes, 15 flute, 1/8 shank, 38.1 long, TiAlN | **18 (ASSUMED)** | 24 000 rpm, **F1500 (ASSUMED)**, plunge F750 |
| T1 | ALUMINUM chamfer 90 | 22 | 18 000 rpm |
| T99 | YIDOFENG touch probe | 40 | |

**T44 assumptions (not confirmed by you):** stick-out 18 = the 15 mm flute + 3 (20 mm in the ER16 collet); the
nut clears the jaws and the car with the tip at its lowest (car z 3.85 on the shims). F1500 at 24k is
0.031 mm/tooth nominal; at the 0.07 finishing stepover the real chip is a few microns, so it is light for a TiAlN
2 mm ball in C360. **Cycle times at other feeds** (T44 feed and plunge scaled together, everything else as planned):

| | T44 F1000 | T44 F1500 (posted) | T44 F2000 |
|---|---|---|---|
| op 2 / last | 24.9 min | 18.9 | 15.9 |
| cycle | 30.4 min | 24.4 | 21.4 |

## Op 1 (bottom up, left station, on the 1.67 shims; T99, T24, T1)

Face 0.2 → rough round both cars, the gap and the ends to Z3.33 (4 levels, laps, cap 400 W) → **walls in one
full-height pass** to Z3.33 = car z 6.47 (every vertical wall of the car; the side walls rise to 5.2–6.0, the
tail lamp panel to 6.50) → magnet pockets → two Ø7.5 carrier bores in the gap → 0.3 edge break of the bottom
outline and pocket rims (T1). Walls checked against the exact outline: the tool centre runs 3.175 ± 0.001 off it.

Simulation: 0 problems, peak 331 W at 18k (rough, entering the gap; caution 501), chatter "expected" on the rough
(margin 1.28, the same as the ingot slider's op 1, which ran), chips ok, nothing unfinished.

## Op 2 (top up, right station, on 1.67 shims; T99, T24, T44, T1)

Find stock, blow-out pause, Find bores → face to the roof (car z 9.5) → **heavy rough while the band still joins
the cars** down to car z 6.52 (1.0 levels, 300 W target, peak 314 W) → light rough to car z 3.9 that separates the
cars (120 W) → T44 semi-finish (parallel 0.25, leaves 0.05) → T44 finish along the car, slopes ≤ 45°, **0.07
stepover (0.035 D)** → T44 waterline on the steep faces (≥ 40°: shoulders, glass, nose, tail lamp panel), 0.07 apart
→ T44 pencil along the creases → T1 0.2 edge break of the nose and tail top edges and the tail lip.

The ball never goes below car z 3.85 (work Z5.52, jaw tops at 4.7). The roof flat (z 9.5) is the T24 face, the
3.9 nose band the light rough's floor.

**Shims for op 2 (check):** the jaws close on the car itself (17.3 wide), so op 2's shim strips must be narrower
than the car; the model has two 16 × 39 strips. The old v3 folder drew 25 × 39 strips for op 2, which can't fit
between the jaws. The moving jaw closes 2.3 mm further than for the 19.6 ingot slider.

Simulation: 0 problems, chatter ok, chips ok. **rCAD's "unfinished": 22 mm² carrying up to 0.43 mm, at the
mirror bases** (x 7.3, 5.4 from the car's middle). That is the R1 ball's limit, not a missed pass: the best an
R1 ball can do there (`src/balllimit.py`) leaves up to 0.46.

**My own check of the finish** (`src/ballsim.py`: a 0.025 mm z-map of every T44 move in the posted op 2, against the
approved surface, measured along the surface normal): nothing cut into the surface (−0.021 at worst, the z-map's
own step); where the ball cuts, mean +0.008, 99 % within 0.15 mm. Off the roof, more than 0.1 mm is left on
11 mm² (the R1 ball's own limit is 4.7 mm²): the hood shut line and the body creases (0.2 deep V grooves in the
height map: the ball leaves them soft, as the spec expected; they are not engraved), the glass-edge steps and the
mirror bases. Pictures: `eclipse-car-op2-cut-vs-approved.png` (hill-shaded, approved over simulated) and
`eclipse-car-op2-ballsim.png` (the difference).

## rCAD changes made for this job (branch `waterline-chords`, from `cam-job-order` d6902ae)

rCAD's waterline gouged this mesh at a different level each try (0.05–0.76 mm, beside the pillars: its loops took
chords across proud features narrower than its grid). Without the waterline the parallel finish can't reach the
tail lamp panel (the ball would have to go past the end of the car; 85 mm² left up to 2 mm). Three commits:
look along each chord for its deepest dip and move it to the nearest clear spot on either side, break the loop
where there is none, and leave dips under 3 µm. With it: 0 problems. Built locally (`bin/rcad` of 37939a1), not
merged.

## Sources

`src/` holds copies of the scripts (they live in `~/dmc2/eclipse-car` on netdash, absolute paths): `clean.py` →
`mkstl.py` → `build.py` (rCAD CAD) → `cam.py` (rCAD CAM, `tsrun.sh cam.json cam`) → `post.py` → `verify.json`;
checks `deviation3d.py`, `contourcheck.py`, `ballsim.py`, `balllimit.py`, `shade.py`, `feedtimes.py`.
rcnc simulator: rcnc 2a7bb0d with the local `RCNC_VISE_OFF` harness (`RCNC_VISE_ADJ=-1`), op 2 with
`RCNC_BORES="-5,0;5,0"`; the cycle ran on one 25 × 101 × 11.47 block (the harness has one block for both stations).
Nothing was sent to the mill PC.
