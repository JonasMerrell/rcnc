# Cycloidal A axis: how each part is made, and what rcnc needs

Nothing here has been cut. Feeds, speeds and toolpaths are not written yet; when they are, the CAM rules of the DMC2
work apply (one 1/4 in carbide end mill, every surface finished, every edge deburred, op 2 located from a notch that
op 1 cuts in the carrier and probed before roughing).

Tags: C calculated, M Monte Carlo study (cycloid.md), G guess to be checked on the machine.

## What accuracy is needed where

| Feature | Why it matters | Target | How it is reached |
|---|---|---|---|
| Disc outline (both discs) | 0.01 mm of error at the pins is 1.15 arc-min at the table (C) | form within 0.008 mm, size set by the pins chosen | stacked, one clamping, probe and correct, graded pins |
| Disc bore Ø37 and the six holes, relative to the outline | run-out of the outline on its bearing shows as error once per input turn | 0.008 mm | same clamping as the outline |
| Pin holes Ø8 on the Ø88 circle (two plates) | pin position error is error once per table turn and uneven preload | position 0.008 mm, both plates alike | both plates bored stacked, probe and correct |
| Eccentric throw 1.25 mm (two bushes) | scales the ratio lever; a difference between the bushes shifts one disc | 0.005 mm, both alike | bore and journal in one clamping, both bushes in one program |
| Sleeve seats Ø10 in carrier 1 and the spider | sleeve position error is error once per table turn | 0.01 mm | one clamping with the part's centre bore |
| Everything else | fits, clearances | 0.03 to 0.05 mm | as milled |

## Before any of it: what the machine itself must be shown to do

These decide whether 0.008 mm is realistic. None needs the 4th axis.

1. **Reversal error of X and Y.** Probe a gauge block or 1-2-3 block from both sides, or indicate a reversal with a
   dial test indicator. Every circle and the disc outline get a step of this size at each quadrant change. If it is
   more than about 0.005 mm it has to be compensated in rcnc or the screws adjusted first.
2. **Scale of X and Y.** Probe the length of a 1-2-3 block (76.2 mm) or a long gauge block in X and in Y, cold and
   after a 20 minute warm-up. Enter the scale corrections.
3. **Squareness of X to Y.** Probe a 1-2-3 block or a square, turn it 90 degrees, probe again (reversal). A squareness
   error of 0.01 mm in 100 mm ovalises every circle by 0.009 mm on the 88 mm pin circle.
4. **Probe calibration.** Effective tip radius in at least 8 directions on a ring gauge or a known pin
   (the cheap 3D probes trigger a few µm earlier in some directions than others). Repeatability: 20 touches on
   one point; if the spread is more than 0.004 mm the 0.008 mm row of the study is not reachable by probing.
5. **Spindle run-out with the finishing end mill** in the collet it will be used in (run-out adds to the cut radius;
   the probe-and-correct loop removes its mean, not its variation between tool changes, so the tool stays in the
   spindle from semi-finish to finish).

## The probe-and-correct loop (the same for every precise feature)

1. Rough, leaving 0.2 mm.
2. Semi-finish, leaving 0.05 mm, with exactly the finishing cut: same tool, same direction (climb), same depth, same feed.
3. Blow off, probe. For a bore: 8 points, fit a circle, get diameter and centre. For the disc outline: 46 points
   (the tip and the valley of each lobe), each approached along the outline's normal; fit one offset and one
   centre shift to the measured-minus-nominal distances.
4. The offset found is what the cutter really leaves (its real radius, run-out and deflection together). Correct the
   tool radius by offset minus 0.05 and run the finish pass. The cut is the same as step 2, so the deflection is too.
5. Probe again, log the numbers with the part. If out, a second spring pass with a 0.005 mm correction is allowed;
   more than that means something else is wrong.

What it catches: cutter size, wear, deflection, run-out. What it cannot catch: the machine's own scale, squareness and
reversal error (the probe moves on the same axes), which is why the five checks above come first. What it leaves:
form error within a pass (chatter marks, quadrant steps) and the probe's own repeatability.

## Part by part

**Cycloid discs A and B** (O1 ground flat stock 1/4 in, left soft; 6.35 x 82 mm)
- Two blanks stacked on a faced aluminium fixture plate. First cut the bore and the six holes through both; then
  bolt the stack down through three of the holes (M6 into nuts in the fixture) and take the edge clamps away.
- Outline: rough, semi-finish, probe, finish, probe (loop above). Smallest inside radius 4.29 mm against the cutter's
  3.175 (C), so the finishing pass never dwells in a corner.
- The bore and holes get the same loop. Bore to a light push fit on the 6904 (37.000 to 37.010).
- Deburr both faces by hand on a stone; mark each disc's top face and the 0 degree lobe before unclamping.
- Assembly: disc B goes in turned half a turn about its own axis and flipped face for face, so taper from the cut
  (the stack's lower disc is cut by the tool's tip end) cancels between the two.

**Pin plates G1 and G2** (1018 cold-rolled flat 1/2 in from iteration c2 on)
- Both plates stacked and bolted through the four tie-rod holes. Bore the 24 pin holes through both in one go:
  helical rough, then two full-depth circular finishing passes, probe four holes (0, 90, 180, 270 degrees) for
  diameter and position, correct, finish all 24, probe all 24 and log.
- Target a close sliding fit on the pins chosen (hole 0.005 to 0.010 over the pin). The pins may turn in their holes;
  that is wanted (rolling, not sliding, at the disc).
- Centre bore and outline in the same clamping; mark the plates so they go into the housing in the cut orientation.

**Ring pins** (bought dowels 8 m6 x 40, or pin gauges)
- Measure the finished discs on the machine: put the disc on a fixture pin through its bore and probe the lobes, or
  measure over two pins with a micrometer. Choose the pin diameter that leaves 0.010 to 0.015 mm at the tightest
  measured point: dowel pins m6 run 8.006 to 8.015; pin gauges come in 0.01 mm steps either side of 8.00.

**Eccentric bushes A and B** (steel 3/8 in faced to 9.00)
- Bore Ø12 and journal Ø20 in one clamping, both bushes side by side in one program. Bore to a close sliding fit on
  the shaft; bonded with Loctite 638 with the two throws opposite (a 1 degree error in that angle only shifts where
  the preload sits, the spring takes it up).

**Carrier 1** (1018 1/2 in): bearing pocket Ø28, three sleeve seats Ø10 x 2.5 and the hub bolt circle from the rear
face in one clamping; nut pockets from the front in the second. Seats to a push fit on the sleeves.

**Carrier 2 spider** (4140 pre-hardened or O1, 3/8 in): outline with the three 2 mm blades in one clamping with the
seats; the blades are finished last with light full-depth passes from both sides alternately so they stay straight.
Blade thickness is measured with a micrometer and logged: the preload stiffness goes with its cube.

**Housing plates P1, C1, M, D, base, faceplate, nose, hubs, brake parts, tailstock**: as iteration 6; only the bearing
bores (Ø95 for the RU66, Ø28 for the 6001) and the seal bore need the probe loop.

## Assembly and setting the preload

1. Shaft: front 6001, shim, bush A with its 6904 and disc A, bush B with its 6904 and disc B, spacer tube. Bond the
   bushes; let it cure standing in carrier 1's pocket so the stack is square.
2. Spider: bolt the three B sleeves to the blade pads on the bench (M6 x 35, nut on the sleeve's front end).
3. Housing: P1 with the main bearing and spindle, brake, C1 with the seal, carrier 1 on the hub, G1, the 24 pins,
   discs, M, G2, spider with the A sleeves and their three screws left slack, D with the rear bearing.
4. Preload: pulley off. A two-pin spanner through D's windows into the spider's holes; hold the faceplate (brake on),
   turn the spider until a spring balance on the spanner shows the preload torque (6 N·m: 40 N at 150 mm), tighten
   the three A screws through the same windows. Turn the input by hand through a full table turn: it must turn
   everywhere without a tight spot; if it binds somewhere, back the preload off and look at the log for that angle.
5. Run in: 2 hours each way at 300 motor rpm with the preload on and plain grease; open, wash, regrease, reset.

## Calibration of the finished axis (in-situ)

- **Axis position**: a ground pin in the vise, probed from both sides at A and A + 180: gives the centreline's Y and Z.
- **Zero**: a 1-2-3 block in the vise, probe two points 60 mm apart on its top, turn until level.
- **Error map**: the same block, probed at two points at each of many table angles; the angle is the height
  difference over the baseline. A 60 mm baseline and 0.004 mm probe repeatability resolve 0.25 arc-min per touch
  pair (C). The block can be probed over about +/- 30 degrees before the faces run out; re-clamp or use a
  hexagon bar for the full turn. The pin-passing ripple repeats every 0.65 degrees, so the map needs steps of
  about 0.1 degrees over at least a few ripple periods at several places round the turn, plus coarse steps all
  the way round. About 1500 touches: an unattended hour or two.
- Limits: the map is only as good as the probe (0.25 arc-min) and it drifts with wear and temperature; redo it after
  run-in and when the preload is reset.

## What rcnc needs

1. **Probing cycles** with results in variables and a log file: bore, boss, web, pocket, single surface, and a
   contour check (a list of nominal points and normals; returns each deviation and the best-fit offset and shift).
2. **Probe calibration cycle**: tip radius per direction on a ring or pin, stored and applied.
3. **Tool radius correction usable from a program**: the finish pass reads a radius offset the probing step wrote
   (cutter compensation, or a variable the toolpath generator's output can take).
4. **Program flow**: conditions and loops on probe results (re-cut, stop with a message when out of tolerance).
5. **Machine compensation**: scale per axis, X-Y squareness, reversal error per axis.
6. **A axis**: step/dir output, degrees, 69:1 with 4000 counts per motor turn (766.67 steps per degree at 1x), speed and
   acceleration limits, homing on the inductive sensor (switch edge approached from one side), soft limits optional
   (it turns endlessly).
7. **Rotary error map** for A: a table of corrections against angle, interpolated, with at least 4000 entries, plus a
   reversal correction for A.
8. **Brake**: two M-codes (clamp, release) on an output, with a dwell and an interlock: no A move while clamped, and
   clamp required before a roughing move if the program says so. A pressure switch input if one is fitted.
9. **Rotary probing helpers**: find the axis centreline (pin, two sides, A and A + 180), level a flat, run the map.
10. Feed rate handling for A with X, Y, Z (inverse time or the usual degrees-as-mm rule), and the work offset of the
    axis centreline so programs are written about the axis.
