# Calibration passes (op 1's waste)

T24 (1/4" 3-flute DLC carbide, stick-out 24), climb, straight along +Y over the whole 120 bar (126 mm with the run-in from air), one pass at a time with a rapid back over the top. Chatter margin and tool deflection per pass come from rCAD's simulation once the passes are rCAD operations (see README).

| Pass | What | Step over (mm) | Depth (mm) | Feed (mm/min) | rpm | MRR (cm³/min) | Model | Mean chip | Steady cut (s) | Proven? | Wall W (fill in) | Where (op 1 frame) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| C1 | clean the torn end of the old C4: 0.8 strip | 0.8 | 2 | 2400 | 24000 | 3.8 | **137 W** (21% of pull-out) | 11.6 µm | 3.4 |  | | x +12.20 .. +13.00, z 30.0 |
| C2 | clean the torn end of the old C4: 0.8 strip | 0.8 | 3.5 | 2400 | 24000 | 6.7 | **239 W** (37% of pull-out) | 11.6 µm | 3.4 |  | | x +12.20 .. +13.00, z 28.5 |
| C3 | clean the torn end of the old C4: 0.8 strip | 0.8 | 5 | 2400 | 24000 | 9.6 | **341 W** (53% of pull-out) | 11.6 µm | 3.4 |  | | x +12.20 .. +13.00, z 27.0 |
| C4 | clean the torn end of the old C4: 0.8 strip | 0.8 | 6 | 2400 | 24000 | 11.5 | **410 W** (64% of pull-out) | 11.6 µm | 3.4 |  | | x +12.20 .. +13.00, z 26.0 |
| C5 | clean the torn end of the old C4: 1 strip | 1 | 2 | 2400 | 24000 | 4.8 | **166 W** (26% of pull-out) | 12.9 µm | 3.4 |  | | x +11.20 .. +12.20, z 30.0 |
| C6 | clean the torn end of the old C4: 1 strip | 1 | 3.5 | 2400 | 24000 | 8.4 | **291 W** (45% of pull-out) | 12.9 µm | 3.4 |  | | x +11.20 .. +12.20, z 28.5 |
| C7 | clean the torn end of the old C4: 1 strip | 1 | 5 | 2400 | 24000 | 12.0 | **416 W** (64% of pull-out) | 12.9 µm | 3.4 |  | | x +11.20 .. +12.20, z 27.0 |
| C8 | clean the torn end of the old C4: 1 strip | 1 | 6 | 2400 | 24000 | 14.4 | **499 W** (77% of pull-out) | 12.9 µm | 3.4 | proven | | x +11.20 .. +12.20, z 26.0 |
| C9 | baseline: the first run's C1 again, new tool (1/2) | 0.6 | 6 | 2400 | 24000 | 8.6 | **318 W** (49% of pull-out) | 10.1 µm | 3.4 | proven | | x +10.60 .. +11.20, z 26.0 |
| C10 | baseline: the first run's C1 again, new tool (2/2) | 0.6 | 6 | 2400 | 24000 | 8.6 | **318 W** (49% of pull-out) | 10.1 µm | 3.4 | proven | | x +10.00 .. +10.60, z 26.0 |
| C11 | 12 deep (1/2) | 0.2 | 12 | 2400 | 24000 | 5.8 | **243 W** (38% of pull-out) | 5.9 µm | 3.4 |  | | x -16.00 .. -15.80, z 20.0 |
| C12 | 12 deep (2/2) | 0.2 | 12 | 2400 | 24000 | 5.8 | **243 W** (38% of pull-out) | 5.9 µm | 3.4 |  | | x -15.80 .. -15.60, z 20.0 |
| C13 | 12 deep (1/2) | 0.3 | 12 | 2400 | 24000 | 8.6 | **346 W** (54% of pull-out) | 7.2 µm | 3.4 |  | | x -15.60 .. -15.30, z 20.0 |
| C14 | 12 deep (2/2) | 0.3 | 12 | 2400 | 24000 | 8.6 | **346 W** (54% of pull-out) | 7.2 µm | 3.4 |  | | x -15.30 .. -15.00, z 20.0 |
| C15 | rpm sweep 21k (1/2) | 0.3 | 12 | 2100 | 21000 | 7.6 | **303 W** (54% of pull-out) | 7.2 µm | 3.8 |  | | x -15.00 .. -14.70, z 20.0 |
| C16 | rpm sweep 21k (2/2) | 0.3 | 12 | 2100 | 21000 | 7.6 | **303 W** (54% of pull-out) | 7.2 µm | 3.8 |  | | x -14.70 .. -14.40, z 20.0 |
| C17 | rpm sweep 18k (1/2) | 0.3 | 12 | 1800 | 18000 | 6.5 | **260 W** (54% of pull-out) | 7.2 µm | 4.5 |  | | x -14.40 .. -14.10, z 20.0 |
| C18 | rpm sweep 18k (2/2) | 0.3 | 12 | 1800 | 18000 | 6.5 | **260 W** (54% of pull-out) | 7.2 µm | 4.5 |  | | x -14.10 .. -13.80, z 20.0 |
| C19 | rpm sweep 15k (1/2) | 0.3 | 12 | 1500 | 15000 | 5.4 | **216 W** (54% of pull-out) | 7.2 µm | 5.4 |  | | x -13.80 .. -13.50, z 20.0 |
| C20 | rpm sweep 15k (2/2) | 0.3 | 12 | 1500 | 15000 | 5.4 | **216 W** (54% of pull-out) | 7.2 µm | 5.4 |  | | x -13.50 .. -13.20, z 20.0 |
| C21 | rpm sweep 12k (1/2) | 0.3 | 12 | 1200 | 12000 | 4.3 | **173 W** (54% of pull-out) | 7.2 µm | 6.7 |  | | x -13.20 .. -12.90, z 20.0 |
| C22 | rpm sweep 12k (2/2) | 0.3 | 12 | 1200 | 12000 | 4.3 | **173 W** (54% of pull-out) | 7.2 µm | 6.7 |  | | x -12.90 .. -12.60, z 20.0 |
| C23 | 6 deep (1/2) | 0.8 | 6 | 2400 | 24000 | 11.5 | **410 W** (64% of pull-out) | 11.6 µm | 3.4 |  | | x +9.20 .. +10.00, z 26.0 |
| C24 | 6 deep (2/2) | 0.8 | 6 | 2400 | 24000 | 11.5 | **410 W** (64% of pull-out) | 11.6 µm | 3.4 |  | | x +8.40 .. +9.20, z 26.0 |
| C25 | 12 deep (1/2) | 0.4 | 12 | 2400 | 24000 | 11.5 | **446 W** (69% of pull-out) | 8.3 µm | 3.4 |  | | x -12.60 .. -12.20, z 20.0 |
| C26 | 12 deep (2/2) | 0.4 | 12 | 2400 | 24000 | 11.5 | **446 W** (69% of pull-out) | 8.3 µm | 3.4 |  | | x -12.20 .. -11.80, z 20.0 |
| C27 | THIN-CHIP COMPARISON 12 deep at half the chip (1/2) | 0.6 | 12 | 1200 | 24000 | 8.6 | **378 W** (59% of pull-out) | 5.0 µm | 6.7 |  | | x -11.80 .. -11.20, z 20.0 |
| C28 | THIN-CHIP COMPARISON 12 deep at half the chip (2/2) | 0.6 | 12 | 1200 | 24000 | 8.6 | **378 W** (59% of pull-out) | 5.0 µm | 6.7 |  | | x -11.20 .. -10.60, z 20.0 |
| C29 | 6 deep (bog guard armed) (1/2) | 1 | 6 | 2400 | 24000 | 14.4 | **499 W** (77% of pull-out) | 12.9 µm | 3.4 | proven | | x +7.40 .. +8.40, z 26.0 |
| C30 | 6 deep (bog guard armed) (2/2) | 1 | 6 | 2400 | 24000 | 14.4 | **499 W** (77% of pull-out) | 12.9 µm | 3.4 | proven | | x +6.40 .. +7.40, z 26.0 |
| C31 | 12 deep (heaviest, bog guard armed) (1/2) | 0.45 | 12 | 2400 | 24000 | 13.0 | **494 W** (77% of pull-out) | 8.8 µm | 3.4 |  | | x -10.60 .. -10.15, z 20.0 |
| C32 | 12 deep (heaviest, bog guard armed) (2/2) | 0.45 | 12 | 2400 | 24000 | 13.0 | **494 W** (77% of pull-out) | 8.8 µm | 3.4 |  | | x -10.15 .. -9.70, z 20.0 |

Total steady cutting about 2.1 min (plus rapids back, about the same again).
