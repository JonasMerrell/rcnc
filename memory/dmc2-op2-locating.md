---
name: dmc2-op2-locating
description: Jonas's rules for locating second operations on the DMC2 (through feature or carrier notch, Find stock then Find bores, rotation)
metadata:
  type: feedback
---
- Second-op referencing: never reference op 2 off raw stock. Leave a feature in op 1 that goes all the way through (e.g. locating bores that open up when op 2 faces the back) and probe it in op 2. Probing the fixed vise jaw is OK for X; Y needs another reference (stop or feature).
- Op 2 locating feature, preferred trick (Jonas, 2026-11-15): cut a small notch in the carrier (the sacrificial stock left around/under the part that is held in the vise) in op 1 and probe that in op 2, so the part itself needs no bore or other locating feature. Default when there is carrier stock (my reading); else through bores. Locate BEFORE roughing, or leave ~0.6 mm for passes after a re-locate (phone stand: 0.38 shift vs 0.3 left showed).
- Op 2 programs start with Find stock (coarse X/Y, size = what op 1 really left at the probe height; sawn ends are not square, measure there), then Find bores (Jonas 2026-11-17, stick-out gauge: Find bores alone landed beside the bore after the flip).
- Find bores measures rotation; rcnc PR 18 (2026-11-18) rotates the program by it, not on the mill PC until Jonas says rebuild. Gauge sat -0.094 deg.
- Related: [[dmc2-cam-preferences]], [[dmc2-vise-setup-preferences]], [[dmc2-touch-probe]]
