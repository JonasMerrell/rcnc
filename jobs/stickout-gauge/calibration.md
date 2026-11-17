# Calibration passes (op 1's waste)

T24 (1/4" 3-flute DLC carbide, stick-out 24), climb, straight along +Y over the whole 120 bar (126 mm with the run-in from air), one pass at a time with a rapid back over the top. Chatter margin and tool deflection per pass come from rCAD's simulation once the passes are rCAD operations (see README).

| Pass | What | Width ae (mm) | Depth ap (mm) | Feed (mm/min) | rpm | Predicted spindle | Mean cutting force (N) | Mean chip | Steady cut (s) | Where (op 1 frame) |
|---|---|---|---|---|---|---|---|---|---|---|
| C1 | width ladder | 0.6 | 6 | 2400 | 24000 | **27%** (318 W) | 40 | 10.1 µm | 3.4 | x +15.40 .. +16.00, z 26.0 |
| C2 | width ladder | 1 | 6 | 2400 | 24000 | **43%** (499 W) | 63 | 12.9 µm | 3.4 | x +14.40 .. +15.40, z 26.0 |
| C3 | width ladder | 1.4 | 6 | 2400 | 24000 | **57%** (672 W) | 84 | 15.0 µm | 3.4 | x +13.00 .. +14.40, z 26.0 |
| C4 | width ladder | 1.8 | 6 | 2400 | 24000 | **72%** (840 W) | 105 | 16.8 µm | 3.4 | x +11.20 .. +13.00, z 26.0 |
| C5 | feed ladder | 1.4 | 6 | 1800 | 24000 | **46%** (541 W) | 68 | 11.3 µm | 4.5 | x +9.80 .. +11.20, z 26.0 |
| C6 | feed ladder | 1.4 | 6 | 3000 | 24000 | **68%** (794 W) | 100 | 18.8 µm | 2.7 | x +8.40 .. +9.80, z 26.0 |
| C7 | rpm sweep (same chip) | 1 | 6 | 1800 | 18000 | **43%** (374 W) | 63 | 12.9 µm | 4.5 | x +7.40 .. +8.40, z 26.0 |
| C8 | rpm sweep (same chip) | 1 | 6 | 1950 | 19500 | **43%** (405 W) | 63 | 12.9 µm | 4.1 | x +6.40 .. +7.40, z 26.0 |
| C9 | rpm sweep (same chip) | 1 | 6 | 2100 | 21000 | **43%** (436 W) | 63 | 12.9 µm | 3.8 | x +5.40 .. +6.40, z 26.0 |
| C10 | rpm sweep (same chip) | 1 | 6 | 2250 | 22500 | **43%** (468 W) | 63 | 12.9 µm | 3.6 | x +4.40 .. +5.40, z 26.0 |
| C11 | rpm sweep (same chip) | 1 | 6 | 2400 | 24000 | **43%** (499 W) | 63 | 12.9 µm | 3.4 | x +3.40 .. +4.40, z 26.0 |
| C12 | repeat of C3 (drift, repeatability) | 1.4 | 6 | 2400 | 24000 | **57%** (672 W) | 84 | 15.0 µm | 3.4 | x +2.00 .. +3.40, z 26.0 |
| C13 | depth ladder | 1.4 | 4.5 | 2400 | 24000 | **43%** (504 W) | 63 | 15.0 µm | 3.4 | x +0.60 .. +2.00, z 27.5 |
| C14 | depth ladder | 1.4 | 3 | 2400 | 24000 | **29%** (336 W) | 42 | 15.0 µm | 3.4 | x -0.80 .. +0.60, z 29.0 |
| C15 | 12 deep | 0.3 | 12 | 2400 | 24000 | **30%** (346 W) | 43 | 7.2 µm | 3.4 | x -16.00 .. -15.70, z 20.0 |
| C16 | 12 deep: Jonas's known 0.6 x 12 at 2500 | 0.6 | 12 | 2500 | 24000 | **56%** (656 W) | 82 | 10.5 µm | 3.2 | x -15.70 .. -15.10, z 20.0 |
| C17 | 12 deep, wider (F2000 keeps it under 75%) | 0.9 | 12 | 2000 | 24000 | **68%** (793 W) | 99 | 10.2 µm | 4.0 | x -15.10 .. -14.20, z 20.0 |

Total steady cutting about 1.0 min (plus rapids back, about the same again).
