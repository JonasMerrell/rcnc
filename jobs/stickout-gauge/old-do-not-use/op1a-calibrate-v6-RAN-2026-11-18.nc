(Op 1a calibration v6 - a 9k mini-ladder with the VFD at F004 = 150: the same watts as v5 passes 3-6 at 12k)
(STOCK 32 x 32 x 120.7 - 6061 bar as op1a v5 left it [passes 1-7]; NOT for a fresh bar)
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

(M1 9k ladder step 1/4: 1 x 12 F1270 S9000 [484 W by the model, 0.51 N m] - Trace)
(MSG,Pass 1: 9k ladder 1/4: 1 x 12 F1270 S9000 [484 W by the model, 0.51 N m] - spindle in air 3 s, then the cut)
G4 P3
X-5.925 Y67
Z34
G1 Z20 F330
Y-67 F1270
G0 Z37

(M2 9k ladder step 2/4: 1 x 12 F1650 S9000 [589 W by the model, 0.63 N m] - Trace)
(MSG,Pass 2: 9k ladder 2/4: 1 x 12 F1650 S9000 [589 W by the model, 0.63 N m] - spindle in air 3 s, then the cut)
G4 P3
X-4.925 Y67
Z34
G1 Z20 F330
Y-67 F1650
G0 Z37

(M3 9k ladder step 3/4: 1 x 12 F1980 S9000 [676 W by the model, 0.72 N m] - Trace)
(MSG,Pass 3: 9k ladder 3/4: 1 x 12 F1980 S9000 [676 W by the model, 0.72 N m] - spindle in air 3 s, then the cut)
G4 P3
X-3.925 Y67
Z34
G1 Z20 F330
Y-67 F1980
G0 Z37

(M4 9k ladder step 4/4: 1 x 12 F2310 S9000 [759 W by the model, 0.80 N m] - Trace)
(MSG,Pass 4: 9k ladder 4/4: 1 x 12 F2310 S9000 [759 W by the model, 0.80 N m] - spindle in air 3 s, then the cut)
G4 P3
X-2.925 Y67
Z34
G1 Z20 F330
Y-67 F2310
G0 Z37

G53 G0 Z0
M5
M9
M30
