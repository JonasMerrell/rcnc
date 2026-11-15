(Op1 chamfer restart - phone stand underside: re-locate on the finished part, then the T1 chamfers)
(work zero: stock bottom, centre [on the vise]; Z0 = the Vise reference)
(the part as op 1 left it: underside up at Z34, 74 x 74 plinth walls finished, 2 mm recess, two O8 bores; leave it clamped as it is)
(fixture fixed vise jaw [LEFT, -X]; jaws grip the bottom 4.7 mm)
(retracts use G53 G0 Z0 - home the machine first)
(T99 YIDOFENG 3D touch probe - touch probe, D2 stylus ball, flute length 12, stickout 40)
(T1 ALUMINUM chamfer 90 - D6.35 90deg chamfer mill, flute length 18, stickout 22)
(estimated time 2:00)
G21 G17 G40 G49 G80 G90 G91.1 G94 G64
G54
(RCNC Z_REFERENCE Vise)

(Rough X/Y from the finished plinth: jog the probe over the middle, a few mm above it [the recess floor is Z32])
G53 G0 Z0
T99 M6
(rcnc Find stock: jog the probe over the middle of the part, a few mm above it, and press Continue)
(RCNC FIND_STOCK SIZE=74,74,34 AT=0,0,34 TOL=1 DEPTH=8 KEEPZ=1)

(Exact X/Y from the two O8 bores op 1 cut: touches 3 mm below the recess floor)
(RCNC FIND_BORES A=-18,19.25 B=18,19.25 D=8 TOP=32 DEPTH=3 TOL=0.1 ROT=0.2)

(Chamfer 0.5 underside edges - Chamfer)
G53 G0 Z0
M5
M9
T1 M6
S24000 M3
M8
G0 Z39
X-37.997 Y27.252
Z36
G1 Z32.5 F200
G2 X-27 Y38 I10.998 J-0.253 F1850
G1 X27
G2 X38 Y27 I-0.001 J-11.001
G1 Y-27
G2 X27 Y-38 I-11.001 J0.001
G1 X-27
G2 X-38 Y-27 I0.001 J11.001
G1 X-37.997 Y27.252
G0 Z39
X-27.695 Y24.709
Z36
G1 Z32.5 F200
X-27.92 Y23.868 F1850
X-28 Y22.956
Y-22.956
X-27.92 Y-23.868
G3 X-23.868 Y-27.92 I4.92 J0.868
G1 X-22.956 Y-28
X22.956
X23.868 Y-27.92
G3 X27.92 Y-23.868 I-0.868 J4.92
G1 X28 Y-22.956
Y22.956
X27.92 Y23.868
G3 X23.868 Y27.92 I-4.92 J-0.868
G1 X22.956 Y28
X-22.956
X-23.868 Y27.92
G3 X-27.695 Y24.709 I0.868 J-4.92
G0 Z39

G53 G0 Z0
M5
M9
M30
