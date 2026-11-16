# Operations (from rCAD: times, peak load from the stock simulation)

Load = worst share of the tool's limits at the op's heaviest cut; spindle % of the power available at that rpm.

## Base op1 bottom: 5.3 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Probe stock top | T99 |  |  | 2 s | – |
| 2 | Probe stock centre | T99 |  |  | 19 s | – |
| 3 | Face the bar top (the bases' bottoms) 0.5 | T24 | 22500 | 2350 | 39 s | 0.22 / 10% (113 W, 3.19 wide × 0.50 deep) |
| 4 | Rough round both bases and the gap to Z6.5 (0.6 mm stepover) | T24 | 22500 | 2350 | 180 s | 0.64 / 64% (703 W, 3.50 wide × 2.85 deep) |
| 5 | Finish the walls, one full-height pass (op 2 grips these) | T24 | 22500 | 2350 | 22 s | 0.22 / 22% (237 W, 0.47 wide × 5.70 deep) |
| 6 | Rough the sawn ends off to Z2.5 | T24 | 22500 | 2350 | 16 s | 0.27 / 27% (291 W, 0.89 wide × 4.00 deep) |
| 7 | Finish the ends 98 apart, full height (op 2 finds Y on them) | T24 | 22500 | 2350 | 12 s | 0.11 / 6% (68 W, 0.07 wide × 9.70 deep) |
| 8 | Chamfer 1.5 the bottom outline | T1 | 22500 | 1850 | 13 s | – |

Simulation: clean; chatter: ok

## Base op2 top: 14.2 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Probe stock top | T99 |  |  | 2 s | – |
| 2 | Probe stock centre | T99 |  |  | 19 s | – |
| 3 | Face to 9 (3.2) | T24 | 22500 | 2350 | 80 s | 0.33 / 33% (366 W, 3.22 wide × 1.60 deep) |
| 4 | Rough the channel and outside to Z6 (parts still joined below) | T24 | 22500 | 2350 | 402 s | 0.42 / 42% (462 W, 2.16 wide × 2.90 deep) |
| 5 | Finish the channel floor (Z6) | T24 | 22500 | 2350 | 74 s | 0.22 / 2% (24 W, 3.32 wide × 0.10 deep) |
| 6 | Finish the rails, both sides, Z9 to Z6 | T24 | 22500 | 2350 | 47 s | 0.11 / 0% (4 W, 0.43 wide × 0.10 deep) |
| 7 | Magnet pockets 8.05 x 3.15 (helical, finish floor) | T24 | 22500 | 2350 | 23 s | 0.22 / 2% (24 W, 6.35 wide × 0.05 deep) |
| 8 | Rest rough below Z6: separates the two bases (light) | T24 | 22500 | 2350 | 75 s | 0.23 / 3% (38 W, 1.36 wide × 0.30 deep) |
| 9 | Finish the outline 9 to 5, one full-height pass | T24 | 22500 | 2350 | 57 s | 0.11 / 1% (11 W, 0.41 wide × 0.30 deep) |
| 10 | Chamfer 1.5 the rail tops outside | T1 | 22500 | 1850 | 16 s | – |
| 11 | Edge break 0.3: channel edges, floor ends, pocket rims | T1 | 22500 | 1850 | 39 s | – |

Simulation: clean; chatter: ok

## Slider op1 bottom: 11.9 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Probe stock top | T99 |  |  | 2 s | – |
| 2 | Probe stock centre | T99 |  |  | 19 s | – |
| 3 | Face the bar top (the sliders' bottoms) 0.2 | T24 | 18000 | 1350 | 67 s | 0.16 / 3% (24 W, 3.19 wide × 0.20 deep) |
| 4 | Rough round both sliders and the gap to Z4 (0.5 mm stepover) | T24 | 18000 | 1350 | 447 s | 0.48 / 48% (420 W, 1.80 wide × 3.15 deep) |
| 5 | Finish the walls, one full-height pass (op 2 grips these) | T24 | 18000 | 1350 | 66 s | 0.09 / 6% (56 W, 0.61 wide × 2.15 deep) |
| 6 | Magnet pockets 8.05 x 3.15 (helical, finish floor) | T24 | 18000 | 1350 | 24 s | 0.14 / 0% (3 W, 1.54 wide × 0.06 deep) |
| 7 | Rough the sawn ends off to Z1 | T24 | 18000 | 1350 | 22 s | 0.12 / 12% (109 W, 0.89 wide × 3.00 deep) |
| 8 | Finish the ends 98 apart, full height (op 2 finds Y on them) | T24 | 18000 | 1350 | 15 s | 0.05 / 2% (18 W, 0.13 wide × 3.00 deep) |
| 9 | Edge break 0.3: bottom outline and pocket rims | T1 | 18000 | 1000 | 34 s | – |

Simulation: clean; chatter: Rough round both sliders and the gap to Z4 (0.5 mm stepover): caution (margin 0.9348)

## Slider op2 top: 6.6 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Probe stock top | T99 |  |  | 2 s | – |
| 2 | Probe stock centre | T99 |  |  | 19 s | – |
| 3 | Face to 8 (1.3) | T24 | 18000 | 1350 | 136 s | 0.16 / 9% (77 W, 3.19 wide × 0.65 deep) |
| 4 | Rough the sides and the gap to Z5 (separates the sliders) | T24 | 18000 | 1350 | 173 s | 0.43 / 43% (376 W, 1.88 wide × 2.70 deep) |
| 5 | Finish the outline 8 to 5, one full-height pass | T24 | 18000 | 1350 | 33 s | 0.09 / 8% (70 W, 0.61 wide × 2.70 deep) |
| 6 | Ingot chamfer 2 x 45 round the top | T1 | 18000 | 1000 | 19 s | – |

Simulation: clean; chatter: Rough the sides and the gap to Z5 (separates the sliders): caution (margin 0.8013)

## Slider op2 engrave: 2.5 min

| # | Operation | Tool | rpm | Feed | Time | Peak load / spindle |
|---|---|---|---|---|---|---|
| 1 | Engrave the crosshatch 0.4 deep (chamfer mill tip) | T1 | 18000 | 1000 | 143 s | – |

Simulation: clean; chatter: ok

