(Op 1a calibration - made by rCAD)
(STOCK 32 x 32 x 120.7 - 6061 bar as measured 2026-11-17; NOT for a 25.4 bar)
(work zero: part bottom, stock centre [on the vise])
(carries on from Already cut [first run]: clamp the part it left with its top side up)
(fixture fixed jaw [centre block, -X]: X-28.7 to -16, Y-47.75 to 47.75, Z0 to 4.7 [work])
(fixture moving jaw [+X]: X16 to 28.7, Y-47.75 to 47.75, Z0 to 4.7 [work])
(retracts use G53 G0 Z0 - home the machine first)
(T99 YIDOFENG 3D touch probe - touch probe, D2 stylus ball, flute length 12, stickout 40)
(T24 3 FLUTE DLC 1/4 - D6.35 flat end mill, 3 flutes, flute length 19.05, stickout 24)
(estimated time 5:26)
G21 G17 G40 G49 G80 G90 G91.1 G94 G64
G54
(RCNC Z_REFERENCE Vise)
(RCNC REQUIRE_GUARD: the bog guard [Settings -> Listening, feed hold on] must be armed: rcnc refuses Cycle Start without it)
(RCNC RECORD_AUDIO: the calibration passes; rcnc saves the run's sound beside this file as .wav)

(Probe stock top - Probe)
G53 G0 Z0
T99 M6
(rcnc Find stock: jog the probe over the middle of the stock, a few mm above it, and press Continue)
(RCNC FIND_STOCK SIZE=32,120.7,32 AT=0,0,32 TOL=3 DEPTH=8 KEEPZ=1)

(Probe stock centre - Probe)

(C1 clean the torn end of the old C4: 0.8 strip: 0.8 x 2 F2400 S24000 - Trace)
G53 G0 Z0
T24 M6
S24000 M3
M8
(MSG,Pass 1: clean the torn end of the old C4: 0.8 strip: 0.8 x 2 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
G0 Z37
X15.375 Y-67
Z34
G1 Z30 F330
Y67 F2400
G0 Z37

(C2 clean the torn end of the old C4: 0.8 strip: 0.8 x 3.5 F2400 S24000 - Trace)
(MSG,Pass 2: clean the torn end of the old C4: 0.8 strip: 0.8 x 3.5 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
Y-67
Z34
G1 Z28.5 F330
Y67 F2400
G0 Z37

(C3 clean the torn end of the old C4: 0.8 strip: 0.8 x 5 F2400 S24000 - Trace)
(MSG,Pass 3: clean the torn end of the old C4: 0.8 strip: 0.8 x 5 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
Y-67
Z34
G1 Z27 F330
Y67 F2400
G0 Z37

(C4 clean the torn end of the old C4: 0.8 strip: 0.8 x 6 F2400 S24000 - Trace)
(MSG,Pass 4: clean the torn end of the old C4: 0.8 strip: 0.8 x 6 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
Y-67
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C5 clean the torn end of the old C4: 1 strip: 1 x 2 F2400 S24000 - Trace)
(MSG,Pass 5: clean the torn end of the old C4: 1 strip: 1 x 2 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X14.375 Y-67
Z34
G1 Z30 F330
Y67 F2400
G0 Z37

(C6 clean the torn end of the old C4: 1 strip: 1 x 3.5 F2400 S24000 - Trace)
(MSG,Pass 6: clean the torn end of the old C4: 1 strip: 1 x 3.5 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
Y-67
Z34
G1 Z28.5 F330
Y67 F2400
G0 Z37

(C7 clean the torn end of the old C4: 1 strip: 1 x 5 F2400 S24000 - Trace)
(MSG,Pass 7: clean the torn end of the old C4: 1 strip: 1 x 5 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
Y-67
Z34
G1 Z27 F330
Y67 F2400
G0 Z37

(C8 clean the torn end of the old C4: 1 strip: 1 x 6 F2400 S24000 - Trace)
(MSG,Pass 8: clean the torn end of the old C4: 1 strip: 1 x 6 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
Y-67
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C9 baseline: the first run's C1 again, new tool [1/2]: 0.6 x 6 F2400 S24000 - Trace)
(MSG,Pass 9: baseline: the first run's C1 again, new tool [1/2]: 0.6 x 6 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X13.775 Y-67
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C10 baseline: the first run's C1 again, new tool [2/2]: 0.6 x 6 F2400 S24000 - Trace)
(MSG,Pass 10: baseline: the first run's C1 again, new tool [2/2]: 0.6 x 6 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X13.175 Y-67
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C11 12 deep [1/2]: 0.2 x 12 F2400 S24000 - Trace)
(MSG,Pass 11: 12 deep [1/2]: 0.2 x 12 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X-18.975
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

(C12 12 deep [2/2]: 0.2 x 12 F2400 S24000 - Trace)
(MSG,Pass 12: 12 deep [2/2]: 0.2 x 12 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X-18.775 Y67
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

(C13 12 deep [1/2]: 0.3 x 12 F2400 S24000 - Trace)
(MSG,Pass 13: 12 deep [1/2]: 0.3 x 12 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X-18.475 Y67
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

(C14 12 deep [2/2]: 0.3 x 12 F2400 S24000 - Trace)
(MSG,Pass 14: 12 deep [2/2]: 0.3 x 12 F2400 S24000 - spindle in air 3 s, then the cut)
G4 P3
X-18.175 Y67
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

(C15 rpm sweep 21k [1/2]: 0.3 x 12 F2100 S21000 - Trace)
S21000 M3
(MSG,Pass 15: rpm sweep 21k [1/2]: 0.3 x 12 F2100 S21000 - spindle in air 3 s, then the cut)
G4 P3
X-17.875 Y67
Z34
G1 Z20 F330
Y-67 F2100
G0 Z37

(C16 rpm sweep 21k [2/2]: 0.3 x 12 F2100 S21000 - Trace)
(MSG,Pass 16: rpm sweep 21k [2/2]: 0.3 x 12 F2100 S21000 - spindle in air 3 s, then the cut)
G4 P3
X-17.575 Y67
Z34
G1 Z20 F330
Y-67 F2100
G0 Z37

(C17 rpm sweep 18k [1/2]: 0.3 x 12 F1800 S18000 - Trace)
S18000 M3
(MSG,Pass 17: rpm sweep 18k [1/2]: 0.3 x 12 F1800 S18000 - spindle in air 3 s, then the cut)
G4 P3
X-17.275 Y67
Z34
G1 Z20 F330
Y-67 F1800
G0 Z37

(C18 rpm sweep 18k [2/2]: 0.3 x 12 F1800 S18000 - Trace)
(MSG,Pass 18: rpm sweep 18k [2/2]: 0.3 x 12 F1800 S18000 - spindle in air 3 s, then the cut)
G4 P3
X-16.975 Y67
Z34
G1 Z20 F330
Y-67 F1800
G0 Z37

(C19 rpm sweep 15k [1/2]: 0.3 x 12 F1500 S15000 - Trace)
S15000 M3
(MSG,Pass 19: rpm sweep 15k [1/2]: 0.3 x 12 F1500 S15000 - spindle in air 3 s, then the cut)
G4 P3
X-16.675 Y67
Z34
G1 Z20 F330
Y-67 F1500
G0 Z37

(C20 rpm sweep 15k [2/2]: 0.3 x 12 F1500 S15000 - Trace)
(MSG,Pass 20: rpm sweep 15k [2/2]: 0.3 x 12 F1500 S15000 - spindle in air 3 s, then the cut)
G4 P3
X-16.375 Y67
Z34
G1 Z20 F330
Y-67 F1500
G0 Z37

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
