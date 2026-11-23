---
name: dmc2-nut-spinning-atc
description: 2026-11-18 assessment of a RapidChange-style nut-spinning ER20 tool changer for the DMC2's current spindle
metadata:
  type: project
---
Verdict (2026-11-18): the current spindle CAN drive a RapidChange-style changer; torque is not the blocker.
- RapidChange tightens by inertia "taps" (plunge at ~700-1500 rpm, 2-3 taps), not steady motor torque; vendor claims 12-14 ft-lb (16-19 N·m) on ER20, "like collet wrenches". Typical users run 2.2 kW 24k spindles (~0.9 N·m rated).
- DMC2 spindle: below 150 Hz (F004) it is in the constant-torque region; the 9k ladders showed ~1.1 N·m without bog, so ~1 N·m at 1500 rpm. A 220 V VFD would NOT change low-speed torque (only high-rpm power).
- REGO-FIX ER20 standard nut spec: 32 N·m for 1.5-6.5 mm shanks, 80 N·m for 7-13 mm. So clamp would be ~half spec for 1/4 in, ~quarter for 3/8-1/2 in: pull-out risk for heavy roughing/large shanks.
- Needs: VFD reverse run from the controller, compatible hex (Type A) ER20 nuts, rigid magazine aligned within 0.2 mm, rcnc macro, tool length remeasure every change. Cost ~$400 (ER11/16/20 linear magazine).
Related: ISO20 ATC spindle upgrade and the standalone arm/carousel changer in this project. See [[dmc2-machine-facts]].
