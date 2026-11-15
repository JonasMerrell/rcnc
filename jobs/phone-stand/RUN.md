# Phone stand: how to run it

74 × 74 × 29.5 mm 6061 phone stand from a bandsawn 76 × 76 slab (the saw cut wandered: up to 48 thick,
op 1 faces it down), in two vise setups. Same workflow as
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

T99 probe, T24 3 FLUTE DLC 1/4 (stick-out 24), T1 ALUMINUM chamfer 90 (22), T11 6 mm 2FL ball, carbide
(stick-out 21: the ER16 nut just clears the 29.5 mm crest with the ball on the groove floor; don't let it
stick out less). The program header fills rcnc's tool table.

## Op 1: the underside (about 18 min)

Slab **factory face down** on the built-in parallels, sloping saw cut up, fixed jaw left. It needs to be
at least about 35 thick everywhere (the finished underside is at Z34); up to 48 is fine.

1. Open `op1-underside.nc`, Cycle Start.
2. T99 + Find stock: tap, jog over the middle a few mm above the sloping top, Continue. It touches the
   top there, then the four sides 10 mm below that point (on real material as long as the slab is
   within about 8 mm of level, and well above the jaws). Size 76 × 76, tolerance 3 mm; the top height only warns (it
   expects 48, a sloped top reads lower: fine).
3. T24: face the saw cut from Z48 down to Z34 in 1.5 mm passes (about 12 min; where the slab is thinner it
   cuts air), rough + one full-height finish pass round the 74 × 74 plinth (14.5 deep), the 2 mm recess,
   two Ø8 bores up into what will be the groove floor.
4. T1: 0.5 mm chamfers.

Z room at Z top (from the machine model: the ER16 nut 150 above the table, the Vise reference 23.2 above
the table): the probe's tip (stick-out 40) is about 87 above the parallels, 39 above a 48 slab; T24 about
55 above it, T1 57, T11 58. Before the first run, jog the probe over the slab at Z top and check there is
at least 25 mm under it.

## Op 2: the sculpted top (about 67 min)

Flip it: **finished underside down** on the parallels, a finished plinth wall against the fixed jaw.

1. Open `op2-top.nc`, Cycle Start.
2. T99 + Find stock on the raw top band (rough X/Y).
3. T24: face to the crest, 3D adaptive rough in 3.3 mm steps at 2400 mm/min (about 40 min, 72% of the spindle), then two quick parallel
   semi-rough passes (to 1.7, then to 0.3 over the part, about 3 min) that take the rough's steps off the
   slopes so the ball never takes a deep cut, then a light floor finish at Z10 into the corners. This opens
   the two bores in the groove floor.
4. T99: Find bores runs by itself, sets X/Y from the bores (spacing 36 ± 0.1, turn ≤ 0.2°).
5. T11 at 24 000 rpm / 3000 mm/min (Jonas's usual for the ball: it barely cuts): parallel finish front to
   back, then waterline on the steep faces (about 15 min, under 10% of the spindle). Nothing goes below
   Z10 (the groove floor) near the jaws, whose tops are at Z4.7.

## Checked

rCAD stock + machine sim clean for both ops; both posted files run to the end in the rcnc simulator
(18 and 70 min, Find stock and Find bores pass). Lean of a phone in it: 12 mm thick 14.6°, 10 mm 21.5°,
8 mm 27.4° from vertical.

## Every surface gets a finishing pass

| Op | Surface | Finished by |
|---|---|---|
| 1 | Underside (faced saw cut) | Face, its last 1.5 mm pass at Z34 over the whole underside (T24) |
| 1 | Plinth walls 74 × 74, 14.5 high | One full-height finish pass after the adaptive rough (T24) |
| 1 | Recess floor and walls | The pocket now finishes it (rest machining off). The first part was cut with the adaptive's 0.2 mm left on the recess walls, floor exact |
| 1 | Ø8 bores | Helical bore to size (T24) |
| 1 | Edges | 0.5 chamfers (T1) |
| 2 | Groove floor Z10 | T24 floor finish into the corners (new) |
| 2 | Sculpted top, lip, roll-over | T11 parallel (slopes up to 65°) |
| 2 | Lip face, backrest, steep roll-over | T11 waterline (slopes from 40°) |

rCAD's sim now reports any machined surface left more than 0.1 mm proud ('left rough'): op 2 shows 8 mm²
at the inside corner where the backrest meets the floor at the sides, the 6 mm ball's 3 mm radius. Every
cut is under the tool-load limit (op 2: rough 72%, semi-rough 86% on short ramps down the steps, the
rest 50% or less).

`op2-top-sim.html`: the posted op 2 played back in a browser (orbit, scrub, live stock).
