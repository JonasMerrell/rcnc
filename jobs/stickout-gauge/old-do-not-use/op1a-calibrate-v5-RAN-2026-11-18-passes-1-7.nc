(Op 1a calibration v5 - a 12k ladder: how much the spindle carries at 12000 rpm since F004 = 200)
(STOCK 32 x 32 x 120.7 - 6061 bar as op1a v4 left it [all of v2 cut]; NOT for a fresh bar)
(work zero: G54 from v2 run 1 - the bar must not have been unclamped; home the machine first; no Find stock here)
(VFD: F004 = 200 for this run [it was 400 in run 1])
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

(estimated time 3:30)

(the cutter: T24, measured on the setter; everything at 12000 rpm)
G53 G0 Z0
T24 M6
S12000 M3
M8

(L1 12k ladder step 1/10: 0.45 x 12 F1200 S12000 [247 W by the model, 0.20 N m] - Trace)
(MSG,Pass 1: 12k ladder 1/10: 0.45 x 12 F1200 S12000 [247 W by the model, 0.20 N m] - spindle in air 3 s, then the cut)
G4 P3
X-12.425 Y67
Z34
G1 Z20 F330
Y-67 F1200
G0 Z37

(L2 12k ladder step 2/10: 0.7 x 12 F1200 S12000 [364 W by the model, 0.29 N m] - Trace)
(MSG,Pass 2: 12k ladder 2/10: 0.7 x 12 F1200 S12000 [364 W by the model, 0.29 N m] - spindle in air 3 s, then the cut)
G4 P3
X-11.725 Y67
Z34
G1 Z20 F330
Y-67 F1200
G0 Z37

(L3 12k ladder step 3/10: 0.8 x 12 F1500 S12000 [484 W by the model, 0.39 N m] - Trace)
(MSG,Pass 3: 12k ladder 3/10: 0.8 x 12 F1500 S12000 [484 W by the model, 0.39 N m] - spindle in air 3 s, then the cut)
G4 P3
X-10.925 Y67
Z34
G1 Z20 F330
Y-67 F1500
G0 Z37

(L4 12k ladder step 4/10: 1 x 12 F1500 S12000 [590 W by the model, 0.47 N m] - Trace)
(MSG,Pass 4: 12k ladder 4/10: 1 x 12 F1500 S12000 [590 W by the model, 0.47 N m] - spindle in air 3 s, then the cut)
G4 P3
X-9.925 Y67
Z34
G1 Z20 F330
Y-67 F1500
G0 Z37

(L5 12k ladder step 5/10: 1 x 12 F1800 S12000 [676 W by the model, 0.54 N m] - Trace)
(MSG,Pass 5: 12k ladder 5/10: 1 x 12 F1800 S12000 [676 W by the model, 0.54 N m] - spindle in air 3 s, then the cut)
G4 P3
X-8.925 Y67
Z34
G1 Z20 F330
Y-67 F1800
G0 Z37

(L6 12k ladder step 6/10: 1 x 12 F2100 S12000 [759 W by the model, 0.60 N m] - Trace)
(MSG,Pass 6: 12k ladder 6/10: 1 x 12 F2100 S12000 [759 W by the model, 0.60 N m] - spindle in air 3 s, then the cut)
G4 P3
X-7.925 Y67
Z34
G1 Z20 F330
Y-67 F2100
G0 Z37

(L7 12k ladder step 7/10: 1 x 12 F2400 S12000 [839 W by the model, 0.67 N m] - Trace)
(MSG,Pass 7: 12k ladder 7/10: 1 x 12 F2400 S12000 [839 W by the model, 0.67 N m] - spindle in air 3 s, then the cut)
G4 P3
X-6.925 Y67
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

(L8 12k ladder step 8/10: 1.15 x 12 F2400 S12000 [949 W by the model, 0.76 N m] - Trace)
(MSG,Pass 8: 12k ladder 8/10: 1.15 x 12 F2400 S12000 [949 W by the model, 0.76 N m] - spindle in air 3 s, then the cut)
G4 P3
X-5.775 Y67
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

(L9 12k ladder step 9/10: 1.3 x 12 F2400 S12000 [1058 W by the model, 0.84 N m] - Trace)
(MSG,Pass 9: 12k ladder 9/10: 1.3 x 12 F2400 S12000 [1058 W by the model, 0.84 N m] - spindle in air 3 s, then the cut)
G4 P3
X-4.475 Y67
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

(L10 12k ladder step 10/10: 1.4 x 12 F2400 S12000 [1130 W by the model, 0.90 N m] - Trace)
(MSG,Pass 10: 12k ladder 10/10: 1.4 x 12 F2400 S12000 [1130 W by the model, 0.90 N m] - spindle in air 3 s, then the cut)
G4 P3
X-3.075 Y67
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

G53 G0 Z0
M5
M9
M30
