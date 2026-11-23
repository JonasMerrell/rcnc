---
name: dmc2-vise-geometry
description: Dimensions and layout of Jonas's two-station vise on the DMC2 (from vise.step in DMC2-rcad-machine.json, read 2026-11-15)
metadata:
  type: reference
---
- Two-station vise: one fixed double-sided centre block (37.1 in X x 144.5 in Y); a station on each side; each station's moving jaw is at the outer end on an "M12 Adjustable Base" that can be re-bolted on a 25 mm hole grid for bigger openings. Jaws clamp along X, jaw faces run along Y.
- Jaw length along Y: 95.5 mm at both stations. Jaw faces are 12.7 thick insert bars, 8.7 tall, each with 4 M5 holes (25.4 apart, at +-12.7 and +-38.1 from jaw centre).
- Built-in parallels: a ledge only 1.9 mm wide in front of each insert bar; the middle between ledges is open. Grip height above ledge: 4.13 in CAD, about 4.7 real (Jonas) - use 4.7.
- Centre block cross-section (read 2026-11-19): fixed faces are the bars' OUTER faces, 33.274 apart; a web lies between the two bars at ledge height (bar top -4.13 CAD); past the bar ends (Y +-47.75) the block steps down to 9.97 below bar top out to +-72.26. A probe over the block's middle touches the web, not the jaw top.
- Openings as modelled: left 40.0, right 117.8 (max not modelled). Fixed faces are 33.27 apart. Whole vise 317.7 x 144.5, centred on the 350 x 250 table (placement was a guess).
- Work stops bolt on the fixed (centre) insert bars, so each station can have its own stop; stop slot gives 20 mm adjustment.
- Open question: which physical station past jobs used ("fixed jaw on the left" = right station in CAD, but the test part's 40 gap matches the left station).
- Jonas wants batch jobs to use both stations (2026-11-15). Related: [[dmc2-vise-setup-preferences]], [[dmc2-machine-facts]], [[dmc2-gift-ideas]]
