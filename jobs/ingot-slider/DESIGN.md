# Ingot slider: design notes

Magnetic fidget slider: a C360 brass "ingot" slides in a 6061 channel base and clicks between three positions
15 mm apart on 8 × 3 mm N52 magnets. Two machined parts, six magnets, glue. Everything here is built by
`build.py` (CAD, `slider.json`) and `cam.py` (CAM, `slider-cam.json` + programs).

## Parts

| | Base (6061-T6) | Slider (C360 brass) |
|---|---|---|
| Size | 25.0 × 45.0 × 9.0 | 19.6 × 45.0 × 8.0 |
| Outline corners | R2.5 (ends on the channel wall) | R3 |
| Feature | channel 20.0 wide × 3.0 deep, full length (2.5 mm rails) | ingot chamfer 2.4 × 45° round the top (top flat 14.8 × 40.2; engraving field 12.8 × 38.2) |
| Magnet pockets | 3 × Ø8.05 × 3.15 in the channel floor, y −15 / 0 / +15 | 3 × Ø8.05 × 3.15 in the bottom, same places |
| Edges | 1.7 chamfer on the rail tops outside (0.8 of the 2.5 rail top left flat, 0.5 after the 0.3 inner break); 0.3 on the bottom outline, channel edges, floor ends, pocket rims | 0.3 bottom outline and pocket rims |
| Finish | every face machined | every face machined; crosshatch engraved 0.4 deep on the top field (swappable) |
| Mass (approx.) | 18 g | 53 g |

Fit: 0.2 mm side clearance in the channel; the slider stands 5 mm above the rails. Under each base magnet
2.85 mm of floor, over each slider magnet 4.85 mm of brass. With both sets 0.15 below flush the faces are
0.3 apart when the slider sits on the floor.

## Changes from the proposal

- **Ingot chamfer 2.4 mm (2026-11-18; was 2.0, proposal 3).** It hides the op 1 / op 2 seam (below). The 90°
  chamfer mill (Ø6.35, cone 3.175 wide) cuts 2.4 with a 0.58 tip offset (cone used 2.98 wide), and the tip stays
  at Z5.02, just over the jaw limit Z5.0. 3 mm would need a deeper tip than the jaws allow.
- **Rail-top chamfer 1.7 (was 1.5)**, also to hide the seam. Tip offset 1.0: tip at Z6.3, cone used 2.7 wide.
- **Base outline corners R2.5** (= the rail width), so the corner arc ends on the channel wall instead of
  leaving a sliver of rail end.
- **Outer vertical edges are radiused (R2.5 / R3), not chamfered**: a vertical edge can't be chamfered with a
  vertical chamfer mill. The 1.7 chamfers go round the rails' outer top edges only (round the R2.5 corners to the channel).
- **Base bottom outline: 0.3 edge break, not a 1.5 chamfer** (Jonas): op 2 stands the base on the vise's 1.9 mm
  ledges; a 1.5 chamfer would leave 0.4 mm of land on each and take 1.5 off the gripped walls.
- Magnet pitch kept at 15 (3.5 mm wall from the end pockets to the part ends).
- Blanks: two parts end to end 8 mm apart, 98.0 overall: the parts' outer ends are the faced blank ends.
- Op 2 locating (guidelines): op 1 bores two Ø7.5 holes through the carrier in the gap (X ±5); op 2 finds them
  (rcnc Find bores) before roughing. X can't come from the fixed jaw: the 98 blank is longer than the 95.5 jaws.

## Seating in op 2 (checked)

The flipped parts stand on the vise's built-in ledges, 1.9 mm wide along each jaw, and the jaws close on the walls
op 1 finished.

| | Width in the jaws | Bottom edge break | Land on each ledge | Gripped wall |
|---|---|---|---|---|
| Base | 25.0 | 0.3 | 1.6 mm, along 43.75 of each part's 45 mm (the outer 1.25 overhangs the jaw ends) | 4.4 of 4.7 |
| Slider | 19.6 | 0.3 | 1.6 mm, as for the base | 4.4 of 4.7 |

Nothing else takes support away. The slider's magnet pockets reach x ±4.0 and the ledges start at ±7.9, so they
are well clear. The base has no bottom pockets. The carrier bores and the 8 mm gap are in the carrier, between the
parts, so the parts' own seats aren't touched.

## Stock and the jaws, and where the op 1 / op 2 seam goes (checked 2026-11-18)

Vise: jaws grip 4.7 above the ledges; beside the jaws the tool stays at Z ≥ 5.0 (0.3 over). Op 1 can therefore cut
the walls only down to Z5 (absolute), and op 2 grips those walls after the flip. **No vertical wall is cut in two
operations:** op 1 finishes every outside wall full height down to the jaw limit; op 2 cuts nothing on those walls
below the top chamfer, and the chamfer's lower edge sits 0.2 under the top of op 1's wall, so the seam between
them is inside the chamfer, where the chamfer removes it.

The bars are bought: **6061 1/2 × 1 (12.7 × 25.4)** and **C360 3/8 × 1 (9.5 × 25.4)**, nominal until measured
(`BAR_AL`, `BAR_BR`, `SHIM_BR` in `cam.py`; the asserts there re-check the seam if they change).

| | Base (6061, 12.7 bar) | Slider (C360, 9.5 bar) |
|---|---|---|
| Op 1 face | 0.2 → 12.5 | 0.2 → 9.3 |
| Under the bar in op 1 | nothing | two 1.5 mm shim strips (25 × 39, 12 mm apart) |
| Op 1 walls down to (op 1 Z) | Z5.0 | Z5.0 (3.5 above the blank's bottom) |
| Op 1 grip on the raw bar | 4.7 | **3.2** (light cuts: roughing peak 378 W at 18k) |
| Op 1 walls in op 2 reach | Z7.5 | Z5.8 |
| Op 2 face to | Z9 (3.5 off, two levels) | Z8 (1.3 off) |
| Top chamfer | 1.7 on the rails | 2.4 round the ingot |
| Chamfer lower edge | **Z7.3** (0.2 under 7.5) | **Z5.6** (0.2 under 5.8) |
| Chamfer tip (≥ 5.0 beside the jaws) | Z6.3 | Z5.02 |
| Op 2 grip on op 1's finished walls | 4.7 of 7.5 | 4.7 of 5.8 |

Op 2's band rough takes the stock above op 1's walls down to exactly their top, leaving 0.3 off the walls; the
chamfer's cone removes that 0.3 strip and the raw skin together. The same holds at the part ends and round the
corners (R2.5 base, R3 slider): op 1's contour runs the whole outline.

- **No 0 mm seam left.** The smallest margin is the 0.2 between each chamfer's lower edge and op 1's wall top.
  If the measured bars differ, `cam.py` re-derives the wall tops; the asserts fail if a margin goes under 0.2.
- **Base gap web:** between the two bases the 8 mm gap is cut by op 1 down to Z5 (op 1), so in op 2 the carrier is
  only the band above Z7.5. The band rough separates the bases; after it each base is held by its own 45 mm of
  4.7 mm grip, and only finishing and T1 follow.
- Brass op 1 takes Z from the probed bar top (Z0 = the blank's bottom = the shim top), so the shim's thickness
  doesn't change the cut; a thinner shim lowers the wall top in op 2 by the same amount and the assert in
  `cam.py` catches it (1.5 is the least that keeps 0.2 under the 2.4 chamfer).
