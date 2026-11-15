# Vise stop: how to make and use it

Two work stops from one 6061 block (39 × 20 × 77): each sits on top of the vise's **fixed jaw** and
gives a repeatable Y position for the part, to go with X from the fixed jaw. Its foot touches the part's
end **below the jaw top**, where no tool ever goes, so it stays in place while cutting.

Files: `op1-top.nc` / `op2-bottom.nc`, setup sheets `op1-top.html` / `op2-bottom.html` beside them,
renders `stop-cad-iso.png`, `op1-sim.png`, `op2-sim.png`.

## Hardware

Per stop: **1 × M5 × 20 socket head cap screw** (ISO 4762 / DIN 912). It replaces one of the fixed jaw's
M5 × 8 insert-bar screws and goes into the same tapped hole: under the head 4.25 mm of the stop, then the
bar (8.7 mm), then **7.05 mm of thread**, 2.95 mm short of the bottom of the 10 mm deep tapped hole.
The head sits flush in the stop's 5 mm counterbore. No tapping, no new holes in the vise.

## Make them

Same workflow as the test part: Z0 is the Vise reference (the top of the built-in parallels, the part's
bottom), the fixed jaw on the left, both ops start with the probe.

**Op 1** (stock on the built-in parallels, ~5 min): Open `op1-top.nc`, Cycle Start.
1. T99 + Find stock (jog over the middle, Continue).
2. T24: face to the stops' top, rough the outlines and finish them in one full-height pass,
   counterbores, one Ø8 locating bore per stop.
3. T20: the M5 slots. T1: chamfers.

**Op 2** (flip the block: the stops' tops down on the parallels, their finished walls in the jaws, fixed
jaw left; ~3 min): Open `op2-bottom.nc`, Cycle Start.
1. T99 + Find stock.
2. T24: face the band off, clear the underside around the feet. This separates the two stops; each stays
   clamped by its own walls.
3. T99: Find bores (one bore on each stop) sets X/Y.
4. T1: chamfers. Take both stops out.

## Use one

1. Take one M5 × 8 out of the top of the fixed jaw's insert bar, at the end where the stop should be.
2. Lay the stop on the bar, its foot toward the part side and hanging into the station.
3. Put the M5 × 20 through the stop's slot into that hole; slide the stop to where the part's end should
   be (the slot gives 20 mm, the bar's holes are 25.4 apart) and tighten.
4. Push each part's end against the foot and its side against the fixed jaw, clamp. X from the jaw,
   Y from the stop.
5. Put the M5 × 8 back when the stop comes off.
