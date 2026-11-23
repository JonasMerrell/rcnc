---
name: dmc2-touch-probe
description: Jonas's 3D touch probe for DMC2 stock probing (YIDOFENG NPN-NO), its calibration wizard in rcnc, and what is still unknown about it
metadata:
  type: project
  modified: 2026-11-15T04:49:37.409Z
---
- 2026-11-14: Jonas pointed to a YIDOFENG 3D touch probe / edge finder, NPN, normally open (Amazon B0D4V9B8MX) as the probe for automatic stock probing; he owns one.
- Wired to the same input as the BTER tool setter (BlueSkyT input 11, NO), per Jonas 2026-11-14. Stylus ball 2 mm.
- rcnc already has G38.2 and probing.rs jobs (BossCentre etc.); the jog-to-centre stock wizard is built on those in thread "Make a first real part end to end".
- Stylus is short: Find stock side touches 20 mm below the top are too deep (Jonas, 2026-11-15); 10 mm is OK. Keep probe side depth <= 10 mm.
- Related: [[dmc2-manual-toolchange]]
- Mach3 (DMC2 PC, DMC2.xml user DROs) has no real probe calibration, only settings: FRate1 500 mm/min, FRate2 10 mm/min, DMax 50, latch back-off 1, XY clearance 5, edge length 90, ProbeD 2 mm, Zdepth 5. Scripts just add/subtract ProbeD/2. PROBETLO=0. M924: ToolSetterDiameter 10, Zoffset 4. tools3.dat not decoded (may hold probe length).
- 2026-11-15: Calibrate probe wizard merged into rcnc mill/2026-11-14 (PR #12, merge df7a9b4). Jonas asked for exactly four shape options: inner bore, outer cylinder, inner rectangle, outer rectangle. Also turn-180 runout (kept until the probe is unloaded) and Z offset on a trusted Z reference. Stored in ProbeSettings.calibration. Not yet run on the real mill; what reference Jonas owns (ring gauge etc.) not yet answered.
