(Op 1a calibration - made by rCAD)
(STOCK 32 x 32 x 120.7 - 6061 bar as measured 2026-11-17; NOT for a 25.4 bar)
(work zero: part bottom, stock centre [on the vise])
(fixture fixed jaw [centre block, -X]: X-28.7 to -16, Y-47.75 to 47.75, Z0 to 4.7 [work])
(fixture moving jaw [+X]: X16 to 28.7, Y-47.75 to 47.75, Z0 to 4.7 [work])
(retracts use G53 G0 Z0 - home the machine first)
(T99 YIDOFENG 3D touch probe - touch probe, D2 stylus ball, flute length 12, stickout 40)
(T24 3 FLUTE DLC 1/4 - D6.35 flat end mill, 3 flutes, flute length 19.05, stickout 24)
(estimated time 3:05)
G21 G17 G40 G49 G80 G90 G91.1 G94 G64
G54
(RCNC Z_REFERENCE Vise)
(RCNC RECORD_AUDIO: the calibration passes; rcnc saves the run's sound beside this file as .wav)

(Probe stock top - Probe)
G53 G0 Z0
T99 M6
(rcnc Find stock: jog the probe over the middle of the stock, a few mm above it, and press Continue)
(RCNC FIND_STOCK SIZE=32,120.7,32 AT=0,0,32 TOL=3 DEPTH=8 KEEPZ=1)

(Probe stock centre - Probe)

(C1 width ladder: 0.6 x 6 F2400 S24000 - Trace)
G53 G0 Z0
T24 M6
S24000 M3
M8
G0 Z37
X18.575 Y-67
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C2 width ladder: 1 x 6 F2400 S24000 - Trace)
X17.575 Y-67
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C3 width ladder: 1.4 x 6 F2400 S24000 - Trace)
X16.175 Y-67
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C4 width ladder: 1.8 x 6 F2400 S24000 - Trace)
X14.375 Y-67
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C5 feed ladder: 1.4 x 6 F1800 S24000 - Trace)
X12.975 Y-67
Z34
G1 Z26 F330
Y67 F1800
G0 Z37

(C6 feed ladder: 1.4 x 6 F3000 S24000 - Trace)
X11.575 Y-67
Z34
G1 Z26 F330
Y67 F3000
G0 Z37

(C7 rpm sweep [same chip]: 1 x 6 F1800 S18000 - Trace)
S18000 M3
X10.575 Y-67
Z34
G1 Z26 F330
Y67 F1800
G0 Z37

(C8 rpm sweep [same chip]: 1 x 6 F1950 S19500 - Trace)
S19500 M3
X9.575 Y-67
Z34
G1 Z26 F330
Y67 F1950
G0 Z37

(C9 rpm sweep [same chip]: 1 x 6 F2100 S21000 - Trace)
S21000 M3
X8.575 Y-67
Z34
G1 Z26 F330
Y67 F2100
G0 Z37

(C10 rpm sweep [same chip]: 1 x 6 F2250 S22500 - Trace)
S22500 M3
X7.575 Y-67
Z34
G1 Z26 F330
Y67 F2250
G0 Z37

(C11 rpm sweep [same chip]: 1 x 6 F2400 S24000 - Trace)
S24000 M3
X6.575 Y-67
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C12 repeat of C3 [drift, repeatability]: 1.4 x 6 F2400 S24000 - Trace)
X5.175 Y-67
Z34
G1 Z26 F330
Y67 F2400
G0 Z37

(C13 depth ladder: 1.4 x 4.5 F2400 S24000 - Trace)
X3.775 Y-67
Z34
G1 Z27.5 F330
Y67 F2400
G0 Z37

(C14 depth ladder: 1.4 x 3 F2400 S24000 - Trace)
X2.375 Y-67
Z34
G1 Z29 F330
Y67 F2400
G0 Z37

(C15 12 deep: 0.3 x 12 F2400 S24000 - Trace)
X-18.875
Z34
G1 Z20 F330
Y-67 F2400
G0 Z37

(C16 12 deep: Jonas's known 0.6 x 12 at 2500: 0.6 x 12 F2500 S24000 - Trace)
X-18.275 Y67
Z34
G1 Z20 F330
Y-67 F2500
G0 Z37

(C17 12 deep, wider [F2000 keeps it under 75%]: 0.9 x 12 F2000 S24000 - Trace)
X-17.375 Y67
Z34
G1 Z20 F330
Y-67 F2000
G0 Z37

G53 G0 Z0
M5
M9
M30
