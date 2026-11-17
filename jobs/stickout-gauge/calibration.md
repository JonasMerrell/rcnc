# Calibration passes (op 1's waste)

T24 (1/4" 3-flute DLC carbide, stick-out 24), climb, straight along +Y over the whole 120 bar (126 mm with the run-in from air), one pass at a time with a rapid back over the top. Chatter margin and tool deflection per pass come from rCAD's simulation once the passes are rCAD operations (see README).

| Pass | What | Width ae (mm) | Depth ap (mm) | Feed (mm/min) | rpm | Predicted spindle | Mean cutting force (N) | Mean chip | Steady cut (s) | Where (op 1 frame) |
|---|---|---|---|---|---|---|---|---|---|---|
| A0 | air baseline 24k | – | – | 2400 | 24000 | 0 (no load) | – | – | 3.1 | above the stock, z 30.0 |
| A1 | air baseline 21k | – | – | 2100 | 21000 | 0 (no load) | – | – | 3.6 | above the stock, z 30.0 |
| A2 | air baseline 18k | – | – | 1800 | 18000 | 0 (no load) | – | – | 4.2 | above the stock, z 30.0 |
| C1 | width ladder | 0.6 | 6 | 2400 | 24000 | **27%** (318 W) | 40 | 10.1 µm | 3.1 | x +12.10 .. +12.70, z 19.4 |
| C2 | width ladder | 1 | 6 | 2400 | 24000 | **43%** (499 W) | 63 | 12.9 µm | 3.1 | x +11.10 .. +12.10, z 19.4 |
| C3 | width ladder | 1.4 | 6 | 2400 | 24000 | **57%** (672 W) | 84 | 15.0 µm | 3.1 | x +9.70 .. +11.10, z 19.4 |
| C4 | width ladder | 1.8 | 6 | 2400 | 24000 | **72%** (840 W) | 105 | 16.8 µm | 3.1 | x +7.90 .. +9.70, z 19.4 |
| C5 | feed ladder | 1.4 | 6 | 1800 | 24000 | **46%** (541 W) | 68 | 11.3 µm | 4.2 | x +6.50 .. +7.90, z 19.4 |
| C6 | feed ladder | 1.4 | 6 | 3000 | 24000 | **68%** (794 W) | 100 | 18.8 µm | 2.5 | x +5.10 .. +6.50, z 19.4 |
| C7 | rpm sweep (same chip) | 1 | 6 | 1800 | 18000 | **43%** (374 W) | 63 | 12.9 µm | 4.2 | x +4.10 .. +5.10, z 19.4 |
| C8 | rpm sweep (same chip) | 1 | 6 | 1950 | 19500 | **43%** (405 W) | 63 | 12.9 µm | 3.9 | x +3.10 .. +4.10, z 19.4 |
| C9 | rpm sweep (same chip) | 1 | 6 | 2100 | 21000 | **43%** (436 W) | 63 | 12.9 µm | 3.6 | x +2.10 .. +3.10, z 19.4 |
| C10 | rpm sweep (same chip) | 1 | 6 | 2250 | 22500 | **43%** (468 W) | 63 | 12.9 µm | 3.4 | x +1.10 .. +2.10, z 19.4 |
| C11 | rpm sweep (same chip) | 1 | 6 | 2400 | 24000 | **43%** (499 W) | 63 | 12.9 µm | 3.1 | x +0.10 .. +1.10, z 19.4 |
| C12 | repeat of C3 (drift, repeatability) | 1.4 | 6 | 2400 | 24000 | **57%** (672 W) | 84 | 15.0 µm | 3.1 | x -1.30 .. +0.10, z 19.4 |
| C13 | depth ladder | 1.4 | 4.5 | 2400 | 24000 | **43%** (504 W) | 63 | 15.0 µm | 3.1 | x -2.70 .. -1.30, z 20.9 |
| C14 | depth ladder | 1.4 | 3 | 2400 | 24000 | **29%** (336 W) | 42 | 15.0 µm | 3.1 | x -4.10 .. -2.70, z 22.4 |
| C15 | side, deep | 0.3 | 12 | 2400 | 24000 | **30%** (346 W) | 43 | 7.2 µm | 3.1 | x +12.40 .. +12.70, z 7.0 |
| C16 | side, deep: Jonas's known 0.6 x 12 at 2500 | 0.6 | 12 | 2500 | 24000 | **56%** (656 W) | 82 | 10.5 µm | 3.0 | x +11.80 .. +12.40, z 7.0 |
| C17 | side, deep | 0.75 | 12 | 2400 | 24000 | **66%** (774 W) | 97 | 11.2 µm | 3.1 | x +11.05 .. +11.80, z 7.0 |

Total steady cutting about 1.1 min (plus rapids back, about the same again).
