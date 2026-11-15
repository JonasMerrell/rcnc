# Phone stand: how to run it

74 × 74 × 29.5 mm 6061 phone stand from a bandsawn 76 × 76 × 35 slab, in two vise setups. Same workflow as
the test part: Z0 is the Vise reference (the built-in parallels, the part's bottom), fixed jaw on the left,
both ops start with the probe.

Files: `op1-underside.nc`, `op2-top.nc` with their setup sheets `op1-underside.html` / `op2-top.html` beside
them (rcnc shows them on its Setup sheet tab), model `phone-stand.stl` / `phone-stand.step.zip`, renders
`concept-*.png`.

## Set the vise for 76 mm

The 39 mm parts used the vise with the jaws about 40 apart. The slab needs 76 (op 1) and 74 (op 2):

1. Fixed jaw stays where it is (left, −X): rcnc takes nothing from its position, Find stock does.
2. Loosen the moving jaw's base on the fixture plate and move it **one 25 mm hole outward (+X)**, same row,
   so it stays on the same 25 mm grid. (A 12.5 mm step would put it on the other grid, which is also 12.5
   off in Y.)
3. Open the jaw screw about 12 mm more than for the 39 mm block, check the slab drops in with a few mm to
   spare, tighten the base.
4. Clamp on the bottom 4.7 mm as usual (the slab stands about 30 mm above the jaws: firm, not gorilla).

If the screw can't take the slab after one hole, use two holes (50 mm) and close it up instead.

## Tools

T99 probe, T24 3 FLUTE DLC 1/4 (stick-out 24), T1 ALUMINUM chamfer 90 (22), T11 6 mm 2FL ball, HSS
(stick-out 21: the ER16 nut just clears the 29.5 mm crest with the ball on the groove floor; don't let it
stick out less). The program header fills rcnc's tool table.

## Op 1: the underside (about 7 min)

Slab **factory face down** on the built-in parallels, saw cut up, fixed jaw left.

1. Open `op1-underside.nc`, Cycle Start.
2. T99 + Find stock (tap, jog over the middle, Continue). Size 76 × 76 × 35, tolerance 3 mm, Z only warns.
3. T24: face the saw cut, rough + one full-height finish pass round the 74 × 74 plinth (14.5 deep),
   the 2 mm recess, two Ø8 bores up into what will be the groove floor.
4. T1: 0.5 mm chamfers.

## Op 2: the sculpted top (about 70 min)

Flip it: **finished underside down** on the parallels, a finished plinth wall against the fixed jaw.

1. Open `op2-top.nc`, Cycle Start.
2. T99 + Find stock on the raw top band (rough X/Y).
3. T24: face to the crest, 3D adaptive rough (about 30 min). This opens the two bores in the groove floor.
4. T99: Find bores runs by itself, sets X/Y from the bores (spacing 36 ± 0.1, turn ≤ 0.2°).
5. T11: parallel finish front to back, then waterline on the steep faces (about 33 min). Nothing goes below
   Z10 (the groove floor) near the jaws, whose tops are at Z4.7.

## Checked

rCAD stock + machine sim clean for both ops; both posted files run to the end in the rcnc simulator
(7.2 and 70 min, Find stock and Find bores pass). Lean of a phone in it: 12 mm thick 14.6°, 10 mm 21.5°,
8 mm 27.4° from vertical.
