# Ingot slider: design notes

Magnetic fidget slider: a C360 brass "ingot" slides in a 6061 channel base and clicks between three positions
15 mm apart on 8 × 3 mm N52 magnets. Two machined parts, six magnets, glue. Everything here is built by
`build.py` (CAD, `slider.json`) and `cam.py` (CAM, `slider-cam.json` + programs).

## Parts

| | Base (6061-T6) | Slider (C360 brass) |
|---|---|---|
| Size | 25.0 × 45.0 × 9.0 | 19.6 × 45.0 × 8.0 |
| Outline corners | R2.5 (ends on the channel wall) | R3 |
| Feature | channel 20.0 wide × 3.0 deep, full length (2.5 mm rails) | ingot chamfer 2.0 × 45° round the top (top field 15.6 × 41, R1) |
| Magnet pockets | 3 × Ø8.05 × 3.15 in the channel floor, y −15 / 0 / +15 | 3 × Ø8.05 × 3.15 in the bottom, same places |
| Edges | 1.5 chamfer bottom outline and rail tops outside; 0.3 channel edges, floor ends, pocket rims | 0.3 bottom outline and pocket rims |
| Finish | every face machined | every face machined; crosshatch engraved 0.4 deep on the top field (swappable) |
| Mass (approx.) | 18 g | 53 g |

Fit: 0.2 mm side clearance in the channel; the slider stands 5 mm above the rails. Under each base magnet
2.85 mm of floor, over each slider magnet 4.85 mm of brass. With both sets 0.15 below flush the faces are
0.3 apart when the slider sits on the floor.

## Changes from the proposal

- **Ingot chamfer 2 mm, not 3.** The 90° chamfer mill (Ø6.35, cone 3.175 wide) can't cut 3 mm in one pass with
  a tip offset, and below the chamfer the slider is gripped: in op 2 the cone tip must stay 0.3 mm over the jaw
  tops (Z5.0), so the chamfer's lower edge can't go below Z6 with a 1 mm tip offset. 2 mm meets both (tip at
  Z5.05). A 3 mm look would need a 9 mm slider or a multi-pass chamfer op in rCAD.
- **Base outline corners R2.5** (= the rail width), so the corner arc ends on the channel wall instead of
  leaving a sliver of rail end.
- **Outer vertical edges are radiused (R2.5 / R3), not chamfered**: a vertical edge can't be chamfered with a
  vertical chamfer mill. The 1.5 chamfers go round the base's top and bottom outlines.
- Magnet pitch kept at 15 (3.5 mm wall from the end pockets to the part ends).
- Blanks: two parts end to end 8 mm apart, 98.0 overall: the parts' outer ends are the faced blank ends.
- Op 2 locating (guidelines): op 1 bores two Ø7.5 holes through the carrier in the gap (X ±5); op 2 finds them
  (rcnc Find bores) before roughing. X can't come from the fixed jaw: the 98 blank is longer than the 95.5 jaws.

## Stock and the jaws (checked)

Vise: jaws grip 4.7 above the ledges; beside the jaws the tool stays at Z ≥ 5.0 (0.3 over), because the stock
beside the part (0.2 per side on the base, 2.9 on the slider) is narrower than the tool. Op 1 can therefore
cut the walls only down to Z5 (absolute), and op 2 grips those walls after the flip.

- **Aluminium, 12.7 bar, no shim.** Op 1 faces 0.5 (top 12.2) and finishes the walls to Z6.5. After the flip
  those walls are op 2's Z0…5.7: the jaws grip 4.7 of finished wall, and op 2's own wall pass (down to Z5.0)
  overlaps them by 0.7. (Z6.5, not the deepest allowed Z5: it leaves a 5.7…6 web between the two bases so they
  stay joined until late in op 2.)
- **Brass, 9.5 bar: needs the 1 mm shim (confirmed).** Without it, op 1 faces 0.2 (top 9.3) and can cut walls
  only to Z5 → op 2's band is 9.3 − 5.0 = 4.3, under the 4.7 jaw height, and the raw 25.4 band above it would
  hit the jaws first: 0.4 mm short to grip at all, 0.7 short of the 0.3 overlap. With the blank on a 1 mm shim
  (two 25 × 39 strips, 12 mm apart so the carrier bores break through between them; inside the jaws' length so the end trims miss them) the jaw limit is 4.0 above the blank's bottom: band
  9.3 − 4.0 = 5.3, grip 4.7, overlap 0.3. Op 1 then grips only 3.7 mm of the bar: fine for its light cuts.
  Alternatives considered: shim in op 2 instead (works, but then op 2's Z0 is the shim, and the thickness
  depends on it); 9.5 bar with an 8 mm slider leaves no other room.
- Brass op 1 takes Z from the probed bar top (Z0 = the blank's bottom = shim top), so the shim's thickness
  doesn't matter; a bar that isn't 9.5 shifts op 1's depths only.
