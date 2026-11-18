(Op 1a calibration v7 - the 9k ladder on, F004 = 150: about 850 / 950 / 1050 W by the model)
(STOCK 32 x 32 x 120.7 - 6061 bar as op1a v6 left it; NOT for a fresh bar)
(work zero: G54 from v2 run 1 - the bar must not have been unclamped; home the machine first; no Find stock here)
(VFD: F004 = 150 for this run [v5 ran at 200, run 1 at 400])
(fixture fixed jaw [centre block, -X]: X-28.7 to -16, Y-47.75 to 47.75, Z0 to 4.7 [work])
(fixture moving jaw [+X]: X16 to 28.7, Y-47.75 to 47.75, Z0 to 4.7 [work])
(retracts use G53 G0 Z0 - home the machine first)
(T99 YIDOFENG 3D touch probe - touch probe, D2 stylus ball, flute length 12, stickout 40)
(T24 3 FLUTE DLC 1/4 - D6.35 flat end mill, 3 flutes, flute length 19.05, stickout 24)
G21 G17 G40 G49 G80 G90 G91.1 G94 G64
G54
(RCNC Z_REFERENCE Vise)
(RCNC REQUIRE_GUARD: the bog guard [Settings -> Listening, feed hold on] must be armed: rcnc refuses Cycle Start without it)
(RCNC RECORD_AUDIO: the calibration passes; rcnc saves the run's sound beside this file as .wav)

(estimated time 2:00)

(the cutter: T24, measured on the setter; everything at 9000 rpm)
G53 G0 Z0
T24 M6
S9000 M3
M8

(N1 9k ladder on, step 1/3: 1 x 12 F2660 S9000 [850 W by the model, 0.90 N m] - Trace)
(MSG,Pass 1: 9k ladder 1/3: 1 x 12 F2660 S9000 [850 W by the model, 0.90 N m] - spindle in air 3 s, then the cut)
G4 P3
X-1.925 Y67
Z34
G1 Z20 F330
Y-67 F2660
G0 Z37

(N2 9k ladder on, step 2/3: 1.05 x 12 F2910 S9000 [949 W by the model, 1.01 N m] - Trace)
(MSG,Pass 2: 9k ladder 2/3: 1.05 x 12 F2910 S9000 [949 W by the model, 1.01 N m] - spindle in air 3 s, then the cut)
G4 P3
X-0.875 Y67
Z34
G1 Z20 F330
Y-67 F2910
G0 Z37

(N3 9k ladder on, step 3/3: 1.15 x 12 F2990 S9000 [1049 W by the model, 1.11 N m] - Trace)
(MSG,Pass 3: 9k ladder 3/3: 1.15 x 12 F2990 S9000 [1049 W by the model, 1.11 N m] - spindle in air 3 s, then the cut)
G4 P3
X0.275 Y67
Z34
G1 Z20 F330
Y-67 F2990
G0 Z37

G53 G0 Z0
M5
M9
M30
