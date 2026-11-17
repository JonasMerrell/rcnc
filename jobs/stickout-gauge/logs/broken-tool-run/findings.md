# Broken T24 run, op1a-calibrate (32 mm bar), 2026-11-17

Source: `op1a-calibrate.nc.wav` in this folder (48 kHz mono, 166.52 s). The recording ends at the E-stop
(file written 05:26:44Z), so it starts at about 05:23:57.5Z; "t" below is seconds from that start.
Tool T24: 6.35 mm 3-flute DLC flat, stick-out 24, 24000 rpm commanded, passes at 6 mm axial depth.

Method: STFT with a 0.2 s Hann window and 50 ms hop. Spindle speed comes from a harmonic-sum pitch of the
rotation tone (harmonics 1-6, 250-410 Hz). Up to the break this agrees with the 1st and 3rd harmonics alone
to within about 1 %. RMS is in dBFS over the same windows. Script: `timeline2.py`; plots: `broken-tool-run-timeline.png`
(whole recording) and `broken-tool-run-timeline-zoom.png` (C1-C4).

## Per pass

Air speed (spindle on, not cutting): **23826 rpm** (0.7 % under the commanded 24000).

| pass | t (s) | rpm at entry | lowest rpm | sag vs air | RMS mean |
|---|---|---|---|---|---|
| C1 0.6 x 6 F2400 | 144.75-147.85 | 23460 | 23394 | 1.8 % | -8.6 dBFS |
| C2 1.0 x 6 F2400 | 150.90-154.00 | 23082 | 23058 | 3.2 % | -7.6 dBFS |
| C3 1.4 x 6 F2400 | 157.00-160.20 | 22632 | 22038 | 7.5 % | -5.6 dBFS |
| C4 1.8 x 6 F2400 | 163.10-break | 22254 (first 0.5 s, already falling) | about 15940 at 164.80 | 33 % | -5.1 dBFS |

- C1 and C2 held a flat speed through the cut.
- C3 never settled. It kept sagging through the whole pass, from 22632 to 22038 rpm (about -200 rpm/s at the end).
- C4 began to sag at once: 2 % under air at 163.35 s and 6 % at 163.45 s. From there it fell steadily at about
  4500 rpm/s, down to about 15940 rpm at 164.80 s.

## Break and E-stop

- **Break at about 164.80-164.85 s (about 05:26:42.3Z).** The rotation tone falls apart there: from 164.85 s the
  harmonics no longer agree, and the pitch flips between about 23900 rpm (unloaded) and 2/3 of that. The level
  drops from -4 to about -9/-10 dBFS by 165.5 s.
- The E-stop is at the end of the recording, 166.52 s, about **1.7 s after the break**.
- The break came about 1.7 s into C4. At F2400 (40 mm/s) that is about 65 mm into the cut, counting from the
  start of loud cutting.

## Sag before the break

- In C4, the sag began within 0.25 s of entering the cut (2 % at 163.35 s) and reached 6 % at 163.45 s, 1.35 s
  before the break. It was 33 % deep (about 15940 rpm) when the tool went.
- C3 was already a warning: it was the first pass whose speed kept falling instead of settling, and it ended
  7.5 % under air. That was about 4.6 s before C4 started and 6.8 s before the break.

## "Feed hold at >= 6 % sag for >= 0.3 s"

- **Against the air speed** (23826 rpm, limit 22396 rpm; this is how the listen-guard branch's `guard.rs` judges
  it, droop over the no-load droop):
  - It trips **in C3 at 158.0 s** (below the limit from 157.7 s). That is **6.8 s before the break**, and C4 would
    never have started.
  - If C3 had been let through, it trips again **in C4 at 163.75 s**, **1.05 s before the break**. At that
    moment the speed was 21460 rpm, 10 % sag.
- **Against the pass's own entry speed** (its first 0.5 s):
  - It doesn't trip in C3, because C3's entry was already 5 % under air.
  - In C4 it trips at 164.30 s, only 0.5 s before the break.
  - So the air speed is the right reference.

Caveats: the absolute times rely on the file's write time (1 s resolution) being the E-stop. The sag times
come from 0.2 s windows, so they are about ±0.1 s.
