# Operations (from rCAD: times, peak load from the stock simulation)

Load = worst share of the tool's limits at the op's heaviest cut; spindle % of the power available at that rpm.

## Base op1 bottom: 2.8 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Probe stock top | T99 |  |  | 2 s | – |
| 2 | Probe stock centre | T99 |  |  | 19 s | – |
| 3 | Face the bar top (the bases' bottoms) 0.2 | T24 | 12000 | 3000 | 29 s | 0.52 / 5% (47 W, 3.19 wide × 0.20 deep) |
| 4 | Rough round both bases, the gap and the ends to Z5 (laps, power budget) | T24 | 9000 | 3000 | 30 s | 0.81 / 66% (734 W, 5.95 wide × 3.75 deep) |
| 5 | Finish the walls in one full-height pass, down to Z5 (op 2 cuts no wall below the chamfer) | T24 | 12000 | 2000 | 31 s | 0.55 / 47% (421 W, 0.91 wide × 7.50 deep) |
| 6 | Two locating bores 7.5 through the carrier between the bases (op 2 finds them) | T24 | 9000 | 1500 | 21 s | 0.35 / 1% (14 W, 6.35 wide × 0.05 deep) |
| 7 | Edge break 0.3 the bottom outline (op 2 sits it on the 1.9 ledges) | T1 | 24000 | 1850 | 11 s | – |

- Simulation: no problems
- Peak spindle: 66% in "Rough round both bases, the gap and the ends to Z5 (laps, power budget)" at X-7.6 Y0.5 Z8.8 (5.95 wide × 3.75 deep, F1185)
- Air cutting: most in "Two locating bores 7.5 through the carrier between the bases (op 2 finds them)", 60% of its feed time
- Chatter: Rough round both bases, the gap and the ends to Z5 (laps, power budget): chatter expected (margin 1.6235)
- Chips: ok (fill under 0.5 everywhere)
- Unfinished faces: none

## Base op2 top: 5.1 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Probe stock top | T99 |  |  | 2 s | – |
| 2 | Probe stock centre | T99 |  |  | 19 s | – |
| 3 | Probe carrier bore A | T99 |  |  | 5 s | – |
| 4 | Probe carrier bore B | T99 |  |  | 5 s | – |
| 5 | Face the top down to Z9 (3.5, two levels) | T24 | 12000 | 3000 | 112 s | 0.54 / 46% (411 W, 3.23 wide × 1.75 deep) |
| 6 | Rough the channel to Z6 | T24 | 9000 | 3000 | 51 s | 0.77 / 60% (673 W, 3.58 wide × 2.80 deep) |
| 7 | Finish the channel floor (Z6), passes along the slide (Y) | T24 | 12000 | 2000 | 16 s | 0.35 / 8% (68 W, 6.34 wide × 0.20 deep) |
| 8 | Magnet pockets 8.05 x 3.15 (helical, finish floor) | T24 | 12000 | 1500 | 19 s | 0.26 / 2% (15 W, 6.35 wide × 0.05 deep) |
| 9 | Rough the band above op 1's walls off to Z7.3, 0.2 under their top Z7.5 (no web left), 0.3 off them (separates the bases) | T24 | 9000 | 3000 | 3 s | 0.61 / 18% (198 W, 1.60 wide × 1.70 deep) |
| 10 | Finish the rails' channel walls, Z9 to Z6, one pass (traces inside the channel sketch, straight through the cleared gap) | T24 | 12000 | 2000 | 8 s | 0.30 / 2% (18 W, 1.59 wide × 0.20 deep) |
| 11 | Chamfer 1.7 the rail tops outside (takes the 0.3 left on the band and hides the op 1 / op 2 seam) | T1 | 24000 | 1850 | 12 s | – |
| 12 | Edge break 0.3: channel edges, floor ends, pocket rims | T1 | 24000 | 1850 | 24 s | – |

- Simulation: no problems
- Peak spindle: 60% in "Rough the channel to Z6" at X-1.7 Y41.6 Z6.2 (3.58 wide × 2.80 deep, F3000)
- Air cutting: most in "Rough the band above op 1's walls off to Z7.3, 0.2 under their top Z7.5 (no web left), 0.3 off them (separates the bases)", 57% of its feed time
- Chatter: ok (all margins under 0.7)
- Chips: Rough the channel to Z6: caution (fill 0.5627)
- Unfinished faces: none

## Slider op1 bottom: 5.8 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Probe stock top | T99 |  |  | 2 s | – |
| 2 | Probe stock centre | T99 |  |  | 19 s | – |
| 3 | Face the bar top (the sliders' bottoms) 0.2, passes along the slide (Y) | T24 | 18000 | 1350 | 51 s | 0.16 / 4% (24 W, 3.19 wide × 0.20 deep) |
| 4 | Rough round both sliders, the gap and the ends to Z3.5 (laps, power budget) | T24 | 18000 | 1350 | 101 s | 0.67 / 57% (378 W, 3.26 wide × 3.87 deep) |
| 5 | Finish the walls in one full-height pass, down to Z3.5 (op 2 cuts no wall below the chamfer) | T24 | 18000 | 1000 | 88 s | 0.16 / 14% (90 W, 0.90 wide × 3.15 deep) |
| 6 | Magnet pockets 8.05 x 3.15 (helical, finish floor) | T24 | 18000 | 1000 | 28 s | 0.12 / 1% (6 W, 4.00 wide × 0.05 deep) |
| 7 | Two locating bores 7.5 through the carrier between the sliders (op 2 finds them) | T24 | 18000 | 1000 | 24 s | 0.12 / 10% (69 W, 2.46 wide × 0.95 deep) |
| 8 | Edge break 0.3: bottom outline and pocket rims | T1 | 18000 | 1000 | 30 s | – |

- Simulation: no problems
- Peak spindle: 57% in "Rough round both sliders, the gap and the ends to Z3.5 (laps, power budget)" at X5.7 Y-0.5 Z3.5 (3.26 wide × 3.87 deep, F1050)
- Air cutting: most in "Two locating bores 7.5 through the carrier between the sliders (op 2 finds them)", 63% of its feed time
- Chatter: Rough round both sliders, the gap and the ends to Z3.5 (laps, power budget): chatter expected (margin 1.5354)
- Chips: ok (fill under 0.5 everywhere)
- Unfinished faces: none

## Slider op2 top: 2.5 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Probe stock top | T99 |  |  | 2 s | – |
| 2 | Probe stock centre | T99 |  |  | 19 s | – |
| 3 | Probe carrier bore A | T99 |  |  | 5 s | – |
| 4 | Probe carrier bore B | T99 |  |  | 5 s | – |
| 5 | Face the top down to Z8 (1.3) | T24 | 18000 | 1350 | 64 s | 0.28 / 23% (155 W, 3.23 wide × 1.30 deep) |
| 6 | Rough the band above op 1's walls off to Z5.6, 0.2 under their top Z5.8 (no web left), 0.3 off them (separates the sliders) | T24 | 18000 | 1350 | 25 s | 0.64 / 54% (359 W, 5.55 wide × 2.20 deep) |
| 7 | Ingot chamfer 2.4 x 45 round the top (takes the 0.3 left on the band, hides the seam; tip at Z5.02) | T1 | 18000 | 1000 | 18 s | – |

- Simulation: no problems
- Peak spindle: 54% in "Rough the band above op 1's walls off to Z5.6, 0.2 under their top Z5.8 (no web left), 0.3 off them (separates the sliders)" at X-3.2 Y0.4 Z5.6 (5.55 wide × 2.20 deep, F1050)
- Air cutting: most in "Rough the band above op 1's walls off to Z5.6, 0.2 under their top Z5.8 (no web left), 0.3 off them (separates the sliders)", 23% of its feed time
- Chatter: Rough the band above op 1's walls off to Z5.6, 0.2 under their top Z5.8 (no web left), 0.3 off them (separates the sliders): chatter expected (margin 1.3449)
- Chips: ok (fill under 0.5 everywhere)
- Unfinished faces: none

## Slider op2 engrave: 1.5 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Engrave the crosshatch 0.4 deep (chamfer mill tip) | T1 | 18000 | 1000 | 88 s | – |

- Simulation: no problems
- Air cutting: most in "Engrave the crosshatch 0.4 deep (chamfer mill tip)", 8% of its feed time
- Chatter: ok (all margins under 0.7)
- Chips: ok (fill under 0.5 everywhere)
- Unfinished faces: none

