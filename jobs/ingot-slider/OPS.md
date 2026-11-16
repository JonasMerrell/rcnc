# Operations (from rCAD: times, peak load from the stock simulation)

Load = worst share of the tool's limits at the op's heaviest cut; spindle % of the power available at that rpm.

## Base op1 bottom: 5.3 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Probe stock top | T99 |  |  | 2 s | – |
| 2 | Probe stock centre | T99 |  |  | 19 s | – |
| 3 | Face the bar top (the bases' bottoms) 0.5 | T24 | 22500 | 2350 | 39 s | 0.22 / 10% (113 W, 3.19 wide × 0.50 deep) |
| 4 | Rough round both bases and the gap to Z6.5 (0.6 mm stepover) | T24 | 22500 | 2350 | 164 s | 0.60 / 60% (656 W, 3.25 wide × 2.85 deep) |
| 5 | Finish the walls, one full-height pass (op 2 grips these) | T24 | 22500 | 2350 | 22 s | 0.38 / 38% (416 W, 0.89 wide × 5.70 deep) |
| 6 | Two locating bores 7.5 through the carrier between the bases (op 2 finds them) | T24 | 22500 | 2350 | 14 s | 0.22 / 2% (24 W, 6.35 wide × 0.05 deep) |
| 7 | Rough the sawn ends off to Z2.5 | T24 | 22500 | 2350 | 16 s | 0.27 / 27% (291 W, 0.89 wide × 4.00 deep) |
| 8 | Finish the ends 98 apart, full height (op 2 finds Y on them) | T24 | 22500 | 2350 | 12 s | 0.11 / 6% (68 W, 0.07 wide × 9.70 deep) |
| 9 | Edge break 0.3 the bottom outline (op 2 sits it on the 1.9 ledges) | T1 | 22500 | 1850 | 12 s | – |

- Simulation: no problems
- Peak spindle: 60% in "Rough round both bases and the gap to Z6.5 (0.6 mm stepover)" at X-9.3 Y0.1 Z9.3 (3.25 wide × 2.85 deep, F2350)
- Air cutting: most in "Rough the sawn ends off to Z2.5", 88% of its feed time
- Chatter: ok (all margins under 0.7)
- Chips: Two locating bores 7.5 through the carrier between the bases (op 2 finds them): caution (fill 0.6404)
- Unfinished faces: none

## Base op2 top: 13.4 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Probe stock top | T99 |  |  | 2 s | – |
| 2 | Probe stock centre | T99 |  |  | 19 s | – |
| 3 | Probe carrier bore A | T99 |  |  | 5 s | – |
| 4 | Probe carrier bore B | T99 |  |  | 5 s | – |
| 5 | Face to 9 (3.2) | T24 | 22500 | 2350 | 80 s | 0.33 / 33% (366 W, 3.22 wide × 1.60 deep) |
| 6 | Rough the channel and outside to Z6 (parts still joined below) | T24 | 22500 | 2350 | 393 s | 0.40 / 40% (441 W, 2.13 wide × 2.80 deep) |
| 7 | Finish the channel floor (Z6) | T24 | 22500 | 2350 | 57 s | 0.22 / 5% (57 W, 4.08 wide × 0.20 deep) |
| 8 | Finish the rails, both sides, Z9 to Z6 | T24 | 22500 | 2350 | 47 s | 0.11 / 10% (112 W, 0.42 wide × 3.00 deep) |
| 9 | Magnet pockets 8.05 x 3.15 (helical, finish floor) | T24 | 22500 | 2350 | 23 s | 0.22 / 2% (24 W, 6.35 wide × 0.05 deep) |
| 10 | Rest rough below Z6: separates the two bases (light) | T24 | 22500 | 2350 | 45 s | 0.22 / 6% (66 W, 3.09 wide × 0.30 deep) |
| 11 | Finish the outline 9 to 5, one full-height pass | T24 | 22500 | 2350 | 57 s | 0.13 / 1% (16 W, 0.62 wide × 0.30 deep) |
| 12 | Chamfer 1.5 the rail tops outside | T1 | 22500 | 1850 | 16 s | – |
| 13 | Edge break 0.3: channel edges, floor ends, pocket rims | T1 | 22500 | 1850 | 39 s | – |

- Simulation: no problems
- Peak spindle: 40% in "Rough the channel and outside to Z6 (parts still joined below)" at X7.7 Y-0.3 Z6.2 (2.13 wide × 2.80 deep, F2350)
- Air cutting: most in "Finish the outline 9 to 5, one full-height pass", 81% of its feed time
- Chatter: ok (all margins under 0.7)
- Chips: ok (fill under 0.5 everywhere)
- Unfinished faces: none

## Slider op1 bottom: 12.2 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Probe stock top | T99 |  |  | 2 s | – |
| 2 | Probe stock centre | T99 |  |  | 19 s | – |
| 3 | Face the bar top (the sliders' bottoms) 0.2 | T24 | 18000 | 1350 | 67 s | 0.16 / 3% (24 W, 3.19 wide × 0.20 deep) |
| 4 | Rough round both sliders and the gap to Z4 (0.5 mm stepover) | T24 | 18000 | 1350 | 447 s | 0.36 / 36% (319 W, 2.91 wide × 2.95 deep) |
| 5 | Finish the walls, one full-height pass (op 2 grips these) | T24 | 18000 | 1350 | 66 s | 0.13 / 13% (111 W, 0.86 wide × 3.15 deep) |
| 6 | Magnet pockets 8.05 x 3.15 (helical, finish floor) | T24 | 18000 | 1350 | 24 s | 0.16 / 1% (8 W, 4.00 wide × 0.05 deep) |
| 7 | Two locating bores 7.5 through the carrier between the sliders (op 2 finds them) | T24 | 18000 | 1350 | 19 s | 0.16 / 1% (12 W, 6.35 wide × 0.05 deep) |
| 8 | Rough the sawn ends off to Z1 | T24 | 18000 | 1350 | 22 s | 0.12 / 12% (109 W, 0.89 wide × 3.00 deep) |
| 9 | Finish the ends 98 apart, full height (op 2 finds Y on them) | T24 | 18000 | 1350 | 15 s | 0.05 / 2% (18 W, 0.13 wide × 3.00 deep) |
| 10 | Edge break 0.3: bottom outline and pocket rims | T1 | 18000 | 1000 | 34 s | – |

- Simulation: no problems
- Peak spindle: 36% in "Rough round both sliders and the gap to Z4 (0.5 mm stepover)" at X7.9 Y0.1 Z6.3 (2.91 wide × 2.95 deep, F1350)
- Air cutting: most in "Rough the sawn ends off to Z1", 87% of its feed time
- Chatter: Rough round both sliders and the gap to Z4 (0.5 mm stepover): caution (margin 0.9762)
- Chips: ok (fill under 0.5 everywhere)
- Unfinished faces: none

## Slider op2 top: 6.7 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Probe stock top | T99 |  |  | 2 s | – |
| 2 | Probe stock centre | T99 |  |  | 19 s | – |
| 3 | Probe carrier bore A | T99 |  |  | 5 s | – |
| 4 | Probe carrier bore B | T99 |  |  | 5 s | – |
| 5 | Face to 8 (1.3) | T24 | 18000 | 1350 | 136 s | 0.16 / 9% (77 W, 3.19 wide × 0.65 deep) |
| 6 | Rough the sides and the gap to Z5 (separates the sliders) | T24 | 18000 | 1350 | 167 s | 0.30 / 30% (267 W, 2.64 wide × 2.70 deep) |
| 7 | Finish the outline 8 to 5, one full-height pass | T24 | 18000 | 1350 | 33 s | 0.11 / 11% (95 W, 0.86 wide × 2.70 deep) |
| 8 | Ingot chamfer 2 x 45 round the top | T1 | 18000 | 1000 | 19 s | – |

- Simulation: no problems
- Peak spindle: 30% in "Rough the sides and the gap to Z5 (separates the sliders)" at X-3.4 Y0.1 Z5.0 (2.64 wide × 2.70 deep, F1350)
- Air cutting: most in "Finish the outline 8 to 5, one full-height pass", 49% of its feed time
- Chatter: ok (all margins under 0.7)
- Chips: ok (fill under 0.5 everywhere)
- Unfinished faces: none

## Slider op2 engrave: 2.5 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Engrave the crosshatch 0.4 deep (chamfer mill tip) | T1 | 18000 | 1000 | 143 s | – |

- Simulation: no problems
- Air cutting: most in "Engrave the crosshatch 0.4 deep (chamfer mill tip)", 9% of its feed time
- Chatter: ok (all margins under 0.7)
- Chips: ok (fill under 0.5 everywhere)
- Unfinished faces: none

