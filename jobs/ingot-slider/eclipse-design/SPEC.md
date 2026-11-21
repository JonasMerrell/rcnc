# Eclipse car slider on a thick-rail base: build spec (approved by Jonas 2026-11-21)

Jonas approved this shape from renders. Build exactly this surface; do not re-trace the blueprint or invent details.
He rejected: a spoiler/wing line at the rear, a squashed profile, and the faceted body of the earlier programs.

## Car (brass C360, from the 10.0 x 25.0 bar, blanks 101, two per blank)
- Envelope 45.0 long x 17.3 wide x 9.5 tall. x = -22.5 nose, +22.5 tail; y = 0 centre; z = 0 slider bottom.
- Plan outline: rectangle 45 x 17.3 with corner radius 4.8 at the nose and 2.4 at the tail (true-width car).
- Top surface: `eclipse-car-heightmap-0.1mm.csv` (first row y values, first column x values, z in mm; 0 = outside
  the outline). Same surface as a closed solid: `eclipse-car-top-0.15mm.stl`. Section tables: `eclipse-car-profiles.csv`.
- Nothing is shaped below z 3.9: the surface is clipped there, so with the 1.67 mm shims under the parts in op 2 the
  ball stays above the jaw line. Straight vertical walls from z 0 up to the surface edge (op 1, one full-height pass).
- The bottom 3.0 mm rides in the base channel; rail tops are at z 3.0 on the car.
- Key heights on the centreline: nose face 3.9, hood 5.7 to 6.1 (nearly flat), cowl 6.4 at x -8.5, roof 9.5 from x 4 to 6,
  hatch glass curving down to the deck at x 16.8 (z 6.55), deck rising slightly to 6.92 at the tail lip (x 21.5),
  then the lamp panel drops to 4.8 and the bumper step to 3.9. No spoiler.
- Surface detail in the height map (all cuttable with an R1 ball): recessed glass (0.2 deep) with the A, B pillars and
  roof standing proud, raised pop-up headlight lids (0.13), hood bulge on the left, mirrors, fender bulges.
- Engraved lines (chamfer mill tip, 0.2 deep, follow the surface): hood shut line. They are already in the height map as
  V grooves; either engrave them or leave them to the ball (the ball cannot make them sharp).
- Underside as the normal ingot slider: three magnet pockets 8.05 dia x 3.15 deep at x -15, 0, +15, 0.3 bottom break.
  Check brass left above each pocket (hood is about 5.7 at x -15, so about 2.5 mm).
- No logos or badges anywhere.

## Base (6061, 12.34 x 26.6 bar): the normal ingot base with ONE change
- 25 x 45 x 8.6 as now, channel 3.0 deep, but channel width 17.7 (was 20.0), so each rail is 3.65 wide (was 2.5).
- Everything else unchanged: R2.5 corners, rail chamfer 1.7, 0.3 bottom break, three magnet pockets at -15/0/+15.
- Put it in its own folder so it cannot be mixed up with the normal base.

## Machining
- Car op 1: as the ingot slider (bottom, walls full height in one pass, magnet pockets, carrier bores), outline 45 x 17.3.
- Car op 2: on the 1.67 mm shims (grip 3.0). Heavy roughing while the two cars are still joined, then separate, then ball.
- Ball: T44 = SpeTool W01004 tapered ball, R1.0 tip, 2.24 deg per side, 2 flutes, 15 mm flute, 1/8 in shank, 38.1 overall,
  TiAlN carbide (library ~/dmc2/tools/dmc2-tools.csv if present). Stick-out not confirmed by Jonas: assume and state it.
  24k rpm. Feed not confirmed: plan at a feed you can justify and say what the cycle would be at 1000 and 2000 mm/min.
- rcnc trap: only ONE "Stock" Z reference exists; never let both stations take Z from Find stock in one cycle.
- Follow jobs/guidelines.md, bog guard required, Find stock from the vise location, blow-out pause before Find bores.
- Gate: SIM OK, verify_gcode, rcnc simulator. Report anything rCAD skips or leaves unfinished (it silently dropped a
  chamfer once on this job).
- Do NOT send anything to the mill PC. Push to mill/ingot-slider-vise in new folders and report hashes and times.
