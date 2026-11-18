(Op 1a calibration v4 - the VFD before/after test: v2 run 1's 12k cut again [passes 21/22], then 23-32 at 24k)
(STOCK 32 x 32 x 120.7 - 6061 bar as v2 run 1 left it [passes 1-20 cut, pass 21 cut from Y+67 to Y-8.1]; NOT for a fresh bar)
(work zero: G54 from v2 run 1 - the bar must not have been unclamped; home the machine first; no Find stock here)
(VFD: F004 = 200 for this run [it was 400 in run 1])
(fixture fixed jaw [centre block, -X]: X-28.7 to -16, Y-47.75 to 47.75, Z0 to 4.7 [work])
(fixture moving jaw [+X]: X16 to 28.7, Y-47.75 to 47.75, Z0 to 4.7 [work])
(retracts use G53 G0 Z0 - home the machine first)
(T99 YIDOFENG 3D touch probe - touch probe, D2 stylus ball, flute length 12, stickout 40)
(T24 3 FLUTE DLC 1/4 - D6.35 flat end mill, 3 flutes, flute length 19.05, stickout 24)
(estimated time 3:00)
G21 G17 G40 G49 G80 G90 G91.1 G94 G64
G54
(RCNC Z_REFERENCE Vise)
(RCNC REQUIRE_GUARD: the bog guard [Settings -> Listening, feed hold on] must be armed: rcnc refuses Cycle Start without it)
(RCNC RECORD_AUDIO: the calibration passes; rcnc saves the run's sound beside this file as .wav)

(the cutter: T24, measured on the setter; the program starts at 12k)
G53 G0 Z0
T24 M6
S12000 M3
M8

(C21 rpm sweep 12k [1/2]: 0.3 x 12 F1200 S12000 - Trace)
S12000 M3
(MSG,Pass 21: rpm sweep 12k [1/2]: 0.3 x 12 F1200 S12000 - spindle in air 3 s, then the cut)
G4 P3
X-16.075 Y67
Z34
G1 Z20 F330
Y-67 F1200
G0 Z37

(C22 rpm sweep 12k [2/2]: 0.3 x 12 F1200 S12000 - Trace)
(MSG,Pass 22: rpm sweep 12k [2/2]: 0.3 x 12 F1200 S12000 - spindle in air 3 s, then the cut)
G4 P3
X-15.775 Y67
Z34
G1 Z20 F330
Y-67 F1200
G0 Z37

(C23 6 deep [1/2]: 0.8 x 6 F2400 S24000 - Trace)
S24000 M3
(MSG,Pass 23: 6 deep [1/2]: 0.8 x 6 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X12.375
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C24 6 deep [2/2]: 0.8 x 6 F2400 S24000 - Trace)
(MSG,Pass 24: 6 deep [2/2]: 0.8 x 6 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X11.575 Y-67
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C25 12 deep [1/2]: 0.4 x 12 F2400 S24000 - Trace)
(MSG,Pass 25: 12 deep [1/2]: 0.4 x 12 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X-15.375
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

(C26 12 deep [2/2]: 0.4 x 12 F2400 S24000 - Trace)
(MSG,Pass 26: 12 deep [2/2]: 0.4 x 12 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X-14.975 Y67
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

(C27 THIN-CHIP COMPARISON 12 deep at half the chip [1/2]: 0.6 x 12 F1200 S24000 - Trace)
(MSG,Pass 27: THIN-CHIP COMPARISON 12 deep at half the chip [1/2]: 0.6 x 12 F1200 S24000 - spindle in air 3 s, then the cut)
G4 P3
X-14.375 Y67
Z34
G1 Z20 F330
Y-67 F1200
G0 Z37

(C28 THIN-CHIP COMPARISON 12 deep at half the chip [2/2]: 0.6 x 12 F1200 S24000 - Trace)
(MSG,Pass 28: THIN-CHIP COMPARISON 12 deep at half the chip [2/2]: 0.6 x 12 F1200 S24000 - spindle in air 3 s, then the cut)
G4 P3
X-13.775 Y67
Z34
G1 Z20 F330
Y-67 F1200
G0 Z37

(C29 6 deep [bog guard armed] [1/2]: 1 x 6 F2400 S24000 - Trace)
(MSG,Pass 29: 6 deep [bog guard armed] [1/2]: 1 x 6 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X10.575
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C30 6 deep [bog guard armed] [2/2]: 1 x 6 F2400 S24000 - Trace)
(MSG,Pass 30: 6 deep [bog guard armed] [2/2]: 1 x 6 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X9.575 Y-67
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C31 12 deep [heaviest, bog guard armed] [1/2]: 0.45 x 12 F2400 S24000 - Trace)
(MSG,Pass 31: 12 deep [heaviest, bog guard armed] [1/2]: 0.45 x 12 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X-13.325
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

(C32 12 deep [heaviest, bog guard armed] [2/2]: 0.45 x 12 F2400 S24000 - Trace)
(MSG,Pass 32: 12 deep [heaviest, bog guard armed] [2/2]: 0.45 x 12 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X-12.875 Y67
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

G53 G0 Z0
M5
M9
M30
