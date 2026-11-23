---
name: dmc2-heard-rpm-readout
description: Live heard spindle rpm readout in rcnc (PR 24): what it shows, the comb-search fix, open rotor-vs-field question, mill PC build
metadata:
  type: project
  modified: 2026-11-20T05:52:02.252Z
---
- rcnc PR 24 (branch claude/project-thread-desf2x, opened 2026-11-20): status bar "mic: 23 879 rpm (−0.5%)"; DRO line under Spindle "heard N rpm (x%) · VFD N · slip N" (VFD part only with RS-485 connected). listen::SpeedMeter = consensus of windows in last 0.7 s (within 0.5%), held 1 s. Cut log Moment gains vfd_rpm and no_load_rpm.
- Analysis fix: comb search reads harmonics at exact frequency (was max of 3 bins); 24k air outlier windows 26% -> 4% (DMC2 fixed ~3.1 kHz line was being taken as 8th harmonic). Guard tests still pass; slip readings shift tens of rpm; v5 pass 6 (759 W) now holds once.
- Open question: v2 run (F004=400) at 12k shows two combs, 194.9 Hz (harmonics 1-3) and 191.2 Hz (8/14/16/18); short windows lock on 191.2. VFD output Hz logged on a real run will tell rotor vs field.
- On the mill PC since 2026-11-20 05:50 UTC: build 2632e3e, branch mill/wash-rpm (main + PR 22/23 + PR 24 merged in). PR 24 MERGED to main 2026-11-21 (745d257); mill PC can return to main.
- Related: [[dmc2-vfd-rs485]], [[dmc2-cutting-simulation]]
