# Handoff: ingot slider gift job and the Eclipse 1G car variant

Written 2026-11-23, 4:55 PM Mountain. Everything below is the state at that time.

## 1. Where things stand

| Item | State |
|---|---|
| Normal ingot slider (brass) and base (aluminium) | In production. Several blanks cut. Bases fit perfectly; brass magnet pockets were tight after the first blank, now re-posted at 8.07 (section 4). |
| Eclipse car (brass) and thick-rail car base (aluminium) | Designed, approved by Jonas, programmed and checked. NOT cut yet. |
| Mill PC upload | Jonas said "put on mill" on 2026-11-22 3:30 PM. The request went to the mill PC thread, but the mill PC was offline and **no confirmation has come back. Treat the mill PC as still holding the OLD programs until the nine hashes in section 3 are verified there.** |

All current programs are on rcnc branch `mill/ingot-slider-vise`, commit **fca5cd0**, under `jobs/ingot-slider/`.

## 2. The designs

**Normal ingot slider set**
- Base, 6061: 25 x 45 x 8.6, channel 20.0 wide x 3.0 deep, rails 2.5, R2.5 corners.
- Slider, C360 brass: 19.6 x 45 x 8, R3 corners, engraved top.
- Three 8 x 3 mm N35 magnets in each part at -15 / 0 / +15 along the slide.

**Eclipse 1G car set** (approved 2026-11-21, "yes I like the thicker rails")
- Car, brass: 45 x 17.3 x 9.5, true width, nose corners R4.8, tail corners R2.4. Bottom 3.0 mm rides in the channel; nothing is shaped below 3.9 mm.
- Car base, 6061: the normal base with ONE change, channel 17.7 wide, so rails are 3.65. Folder `eclipse-base-thick`.
- Profile rules from Jonas: hood nearly flat, long flat roof, hatch glass curving to the deck, deck rising slightly at the tail, NO spoiler line, no logos or badges. Squashed proportions rejected.
- Source of truth for the shape: `jobs/ingot-slider/eclipse-design/` on rcnc branch `design/eclipse-car-surface` (c0ad1f0): `SPEC.md` and `eclipse-car-solid-smooth.stl` (closed solid, exact corner walls). Generator script and renders: project files `gift-ideas/eclipse/` (`eclipse.py`, reference pictures in `ref/`).

## 3. Programs (commit fca5cd0) and hashes

Batch order for every part: `-first` on the first raw blank (op 1, left station), `-cycle` for each blank after it (op 1 left plus op 2 right), `-last` for the last blank (op 2 only, right station).

| Folder | first | cycle | last |
|---|---|---|---|
| `slider/` (brass, pockets 8.07) | 4:57 `c761fc5b4397` | 9:16 `3dbdf5ac75c1` | 4:18 `f28d459fc8a0` |
| `eclipse-car/` (brass, pockets 8.07) | 5:46 `2737b8b84d98` | 26:13 `54e5a9a4e38e` | 20:28 `6ffb8aab62d0` |
| `eclipse-base-thick/` (aluminium) | 2:36 `a6462995895f` | 7:11 `3a2e5390eaff` | 4:35 `da709e0a995e` |
| `base/` (aluminium, normal) | unchanged, already on the mill PC | | |

Hashes are the first 12 hex of sha256. All pass rCAD's fine simulation check, `verify_gcode` and the rcnc simulator. `base-first` and `eclipse-base-thick-first` are the identical op 1.

**Setup facts**
- Stock: brass 10.0 x 25.0, aluminium 12.34 x 26.6. Blanks are posted for 101 mm and Find stock accepts 100 to 102. The two parts span 98 mm (8 mm carrier between them); anything under 100 needs a re-post.
- Brass op 1 and op 2 stand on 1.67 mm shims, grip 3.0 mm. Jonas confirmed the shims fit under the 17.3 mm car. Aluminium uses no shim (grip 4.7).
- Car op 2 is deliberately light for that grip: face 36 W, joined rough 110 W, separating rough 100 W, ball passes 1 to 3 W.
- Car cutters: T99 probe, T24 1/4 in 3 flute, T44 SpeTool W01004 tapered ball (R1.0, 2.24 deg per side, 1/8 in shank) at **18 mm stick-out**, T1 chamfer mill.
- T44 feed is 1500 mm/min at 24k. This is the one value Jonas has not confirmed. Cycle would be 31.9 min at 1000 or 23.3 min at 2000.
- Bog guard must be armed. Find vise location must have been run (it has, for the normal base).
- Blanks are never mixed: brass is two cars or two normal sliders (car 17.3 and slider 19.6 cannot share a clamp); Jonas wants two cars and two car bases.

## 4. Magnet pocket fit (2026-11-22)

- Jonas: first two brass sliders pressed in great, the rest too tight, bases perfect throughout.
- The toolpath was identical for all blanks. Cause per Jonas: the cutter is re-seated in the collet at every tool change, so runout and hole size change between blanks. Brass does not forgive 0.02 mm; aluminium does.
- Also found a bug: the old bottom lap left a wedge up to 0.17 mm high on about a third of the floor edge.
- Jonas chose: brass pockets 8.07, cut as a spiral 0.1 inside the wall, a full lap at size at depth, and a spring lap. A loose magnet gets a drop of glue. Bases stay at 8.05 and were not re-posted (they still have the small wedge, and fit fine).

## 5. Known limits of the car as programmed

- The R1 ball leaves material in tight creases: up to 0.44 mm at the mirror bases, about 0.1 mm at the tail corners and the foot of the lamp panel. Hood shut line and glass edges come out soft.
- Tiny ripples (0.02 to 0.05 mm) remain in the model at the nose corner top edge; the ball smooths them.
- T1 breaks only the tail lip and the straight part of the nose edge. The rounded nose corners, the tail-end edge at 3.9 and the side top edges get no chamfer; the ball passes run over them.
- Only 1.53 mm of brass remains over the rim of the nose magnet pocket.

## 6. Open items

1. Verify the nine hashes on the mill PC (section 3), move the old slider programs to `jobs\ingot-slider\old`, and check that the mill PC's rcnc tool table has T44. Never launch rcnc from a remote shell; Jonas starts it from Explorer.
2. Jonas to confirm or change the T44 feed.
3. rCAD PR #175 (github.com/JonasMerrell/cad-rust-app/pull/175) is not merged. It holds the waterline fix, the tapered-ball flank check fix and the pocket bottom-lap fix. The programs above were posted with that build. PRs #163, #165, #166 and #167 are also open.
4. First car cut: watch the joined rough on the shim grip, and check the pocket fit at 8.07 on the first blank.
5. Older folders `eclipse/`, `eclipse-base/` and `engraved/` on the branch are superseded; do not run them.

## 7. Where to look

- Programs, READMEs and pictures: rcnc `mill/ingot-slider-vise`, `jobs/ingot-slider/<folder>/`.
- CAM sources: netdash `~/dmc2` (slider job) and `~/src/rcnc-eclipse/jobs/ingot-slider/`.
- Pictures of the programmed car: project files `ingot-slider/eclipse-car/` (files starting `smooth-` are the current model).
- Companion file with the netdash script locations and how to re-plan: `jobs/ingot-slider/HANDOFF-eclipse.md` on `mill/ingot-slider-vise` (commit 7490909, which only adds that file; the programs are still those of fca5cd0).
- Rules for any CAM on this machine: `jobs/guidelines.md`.
