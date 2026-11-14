# DMC2 test part: how to run it

6061 block, 35 × 72 × 16 mm, from 39 × 77 × 20 stock in two vise setups. rcnc does the probing, the
tool measuring and the work zero; you load tools, jog the probe over the stock once per setup, and press
buttons.

Files: `op1-top.nc`, `op2-bottom.nc`, with their setup sheets `op1-top.html` / `op2-bottom.html` beside
them (rcnc shows them on its Setup sheet tab).

## How the part is referenced

- **Z0 is the bottom of the part**: the top of the vise's built-in parallels, rcnc's **Vise** Z reference
  (the setter's top is 40.610 above it). Every tool, the probe included, is measured on the tool setter and
  zeroed on it, so thicknesses come out right whatever the raw stock's height. Op 1 faces the top to Z19,
  op 2 faces the other side to Z16.
- **Op 1 X/Y**: Find stock (the stock's middle). Op 1 also bores two Ø8 **locating bores** 1 mm into the
  back stock; op 2's facing opens them.
- **Op 2 X/Y**: Find stock roughly, face, then **Find bores**: rcnc probes both bores, checks their spacing
  (±0.1 mm) and how square the part sits (±0.2°), and sets X/Y from them. If either is off, it stops with
  nothing set.

## Before you start

1. **Vise**: a station with the **fixed jaw on the left (-X)**. The part sits on the built-in parallels.
2. **Check the Vise reference once** (Probe tab, Z references): *Calibrate (probe)* with T99 loaded and
   measured: tap, jog over a built-in parallel's top beside the part, Continue, compare with 40.610,
   Save or Cancel. *Calibrate (gauge)* does it with any measured tool on a gauge block.
3. **Tools** (the program fills rcnc's tool table): T99 probe, T24 3 FLUTE DLC 1/4 (stick-out 24),
   T20 1/8 ALUMINUM (15), T1 ALUMINUM chamfer 90 (22). Spindle 24000 rpm, flood coolant.

## Every session

Reset, **Ref All Home** (two-stage). Tool changes go to X163.5 (the middle) and the setter's Y289,
after Z is all the way up; Z never goes above its top.

## Op 1

1. Raw stock in the vise, fixed jaw left, on the built-in parallels.
2. **Open…** `op1-top.nc` (Air cut +25 mm first if you like), **Cycle Start**.
3. **T99**: load the probe, Continue (measured on the setter).
4. **Find stock**: tap the stylus, jog over the middle 2–10 mm above the top, Continue. X/Y are set;
   Z stays on the Vise reference (the log warns if the top isn't about 20 mm above the parallels).
5. **T24**: face to Z19, rough the outline, finish it in one full-height pass, pocket, the two Ø8
   locating bores.
6. **T20**: the slot. **T1**: chamfers (tip 1 mm below the chamfer).

## Op 2

1. Flip the part **pocket side down** on the built-in parallels, a finished 35 mm wall against the
   fixed jaw (left); shim or soft-jaw the finished walls, snug. The leftover raw band sits well above the
   jaws.
2. **Open…** `op2-bottom.nc`, **Cycle Start**.
3. **T99** + **Find stock** (rough X/Y on the raw band).
4. **T24**: face to Z16, which opens the locating bores.
5. **T99** again: **Find bores** runs by itself (no jogging) and sets X/Y from the bores; the log says
   their spacing and the part's turn.
6. **T24**: rough the band, finish it in one full-height pass. **T1**: chamfer, engrave DMC2.

## If something goes wrong

- Stop (or Esc) stops the program; E-stop cuts the drives.
- Find bores stopped red: the part is turned in the vise or the bores aren't where they should be:
  square the part and run op 2 again (Find stock and the facing are harmless to repeat: the face is
  already at Z16).
- The spindle won't start with the probe in.
