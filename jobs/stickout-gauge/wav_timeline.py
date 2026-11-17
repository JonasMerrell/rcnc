"""What the transducer heard during a run, pass by pass, from the recording alone (no cut log needed).

    python3 wav_timeline.py run.wav program.nc [--teeth 3] [--step 0.25] [--from S] [--to S]

The program gives the passes in order (their "(C4 ... - Kind)" comments) with their S and F. The recording is
cut into windows (0.17 s, every --step s). For each window it finds:
- the spindle's speed: the strongest comb of rotation harmonics within 15% under to 2% over the programmed
  speed (whichever S is active), refined on its strongest harmonic; droop = 1 - heard / programmed;
- the teeth's beat (rotation x teeth) level, dB: it rises with the load;
- the broadband level, dB re full scale;
- the strongest peak that isn't a rotation harmonic (the chatter candidate) and how far over the median it
  stands, dB. Lines that are there in air too (the spindle's own: VFD, bearings) are marked as such.
Passes are found as stretches where the teeth's beat stands well over its air level (cutting), and matched to
the program's passes in order; the windows before the first cut are the air reference. The break is taken as
the last moment the teeth's beat is still there before the sound falls to air or quiet (E-stop), and the last
seconds before it are printed in detail.
"""
import argparse
import re
import sys

import numpy as np
from scipy.io import wavfile


def passes_of(nc_path):
    """[(name, S, F)] in program order, from the operation comments and the S/F words after them."""
    out = []
    s_now = None
    cur = None
    for line in open(nc_path):
        line = line.strip()
        m = re.match(r"\((.+) - [A-Za-z ]+\)$", line)
        if m and not m.group(1).startswith("T") and "made by" not in m.group(1):
            cur = [m.group(1), s_now, None]
            out.append(cur)
            continue
        ms = re.search(r"\bS(\d+(?:\.\d+)?)", line)
        if ms:
            s_now = float(ms.group(1))
            if cur is not None and (cur[1] is None or cur[2] is None):
                cur[1] = s_now
        mf = re.search(r"\bF(\d+(?:\.\d+)?)", line)
        if mf and cur is not None and line.startswith(("G1", "X", "Y")) and float(mf.group(1)) > 400:
            cur[2] = float(mf.group(1))
    return [(n, s if s else 24000.0, f) for n, s, f in out if f]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("wav")
    ap.add_argument("nc")
    ap.add_argument("--teeth", type=int, default=3)
    ap.add_argument("--step", type=float, default=0.25)
    ap.add_argument("--rpm", type=float, default=24000.0, help="spindle speed before the first pass names one")
    ap.add_argument("--from", dest="t0", type=float, default=0.0)
    ap.add_argument("--to", dest="t1", type=float, default=None)
    a = ap.parse_args()

    rate, x = wavfile.read(a.wav)
    x = x.astype(np.float64)
    if x.ndim > 1:
        x = x[:, 0]
    if np.issubdtype(np.asarray(wavfile.read(a.wav)[1]).dtype, np.integer):
        x /= 32768.0
    n = 8192 if rate >= 32000 else 4096
    win = np.hanning(n)
    hop = int(a.step * rate)
    progs = passes_of(a.nc)
    t_end = len(x) / rate if a.t1 is None else a.t1

    rows = []
    t = a.t0
    while t + n / rate <= t_end:
        i = int(t * rate)
        w = x[i:i + n] * win
        spec = np.abs(np.fft.rfft(w)) / n + 1e-12
        db = 20 * np.log10(spec)
        rms = np.sqrt(np.mean(x[i:i + n] ** 2)) + 1e-12
        rows.append((t, db, 20 * np.log10(rms)))
        t += a.step
    if not rows:
        print("no audio in that range")
        return
    bin_hz = rate / n

    def comb(db, rot):
        lin = 10 ** (db / 20)
        k = np.arange(1, 13)
        idx = np.round(k * rot / bin_hz).astype(int)
        idx = idx[idx < len(lin) - 1]
        return sum(max(lin[j - 1], lin[j], lin[j + 1]) for j in idx)

    def speed(db, s_prog):
        rot0 = s_prog / 60.0
        best = (-1.0, rot0)
        for rot in np.arange(rot0 * 0.85, rot0 * 1.02, bin_hz / 24):
            c = comb(db, rot)
            if c > best[0]:
                best = (c, rot)
        off = max(comb(db, best[1] * 1.37), comb(db, best[1] * 0.71))
        if best[0] < 2 * off:
            return None
        return best[1] * 60.0

    # first pass: speed and tooth level per window at the first programmed speed; find cutting stretches
    s_prog = progs[0][1] if progs else a.rpm
    info = []
    for t, db, lvl in rows:
        rpm = speed(db, s_prog)
        tooth_hz = (rpm or s_prog) / 60.0 * a.teeth
        j = int(round(tooth_hz / bin_hz))
        tooth = float(db[max(j - 1, 0):j + 2].max())
        info.append([t, rpm, tooth, lvl, db])
    tooth_air = np.percentile([r[2] for r in info], 20)
    cutting = [r[2] > tooth_air + 10 for r in info]
    # stretches of cutting at least 1 s long
    stretches = []
    k = 0
    while k < len(info):
        if cutting[k]:
            j = k
            while j + 1 < len(info) and (cutting[j + 1] or (j + 2 < len(info) and cutting[j + 2])):
                j += 1
            if info[j][0] - info[k][0] >= 1.0:
                stretches.append((k, j))
            k = j + 1
        else:
            k += 1
    # the spindle's own lines: strong peaks in the air windows before the first cut
    first = stretches[0][0] if stretches else len(info)
    air_db = np.median(np.array([r[4] for r in info[:max(first, 1)]]), axis=0)

    def chatter(db, rpm):
        rot = (rpm or s_prog) / 60.0
        lo, hi = int(300 / bin_hz), int(0.45 * rate / bin_hz)
        floor = np.median(db[lo:hi])
        best = None
        for j in range(lo + 1, hi - 1):
            if db[j] < db[j - 1] or db[j] < db[j + 1] or db[j] - floor < 20:
                continue
            hz = j * bin_hz
            kk = round(hz / rot)
            if kk >= 1 and abs(hz - kk * rot) <= max(0.004 * hz, 2.5 * bin_hz):
                continue
            own = db[j] < air_db[j] + 10
            if best is None or db[j] > best[1]:
                best = (hz, db[j], db[j] - floor, own)
        return best

    print(f"{len(progs)} passes in the program, {len(stretches)} cutting stretches heard; teeth's beat in air {tooth_air:.1f} dB\n")
    print(f"{'t s':>7} {'pass':<8} {'rpm':>7} {'droop %':>7} {'tooth dB':>8} {'level dB':>8} {'chatter Hz':>10} {'over dB':>7}")
    for si, (k0, k1) in enumerate(stretches):
        name, s_p, f_p = progs[si] if si < len(progs) else (f"stretch {si + 1}", s_prog, None)
        key = name.split()[0]
        drs, tts, chs = [], [], []
        for r in info[k0:k1 + 1]:
            t, rpm, tooth, lvl, db = r
            rpm = speed(db, s_p)
            d = 1 - rpm / s_p if rpm else None
            c = chatter(db, rpm)
            drs.append(d)
            tts.append(tooth)
            if c and not c[3]:
                chs.append(c[0])
            print(f"{t:7.2f} {key:<8} {rpm or 0:7.0f} {(d or 0) * 100:7.2f} {tooth:8.1f} {lvl:8.1f} {(f'{c[0]:.0f}' + ('*' if c[3] else '')) if c else '-':>10} {(f'{c[2]:.0f}') if c else '':>7}")
        dd = [d for d in drs if d is not None]
        print(f"   == {name}: S{s_p:.0f} F{f_p}: {info[k1][0] - info[k0][0]:.1f} s, droop mean {np.mean(dd) * 100 if dd else float('nan'):.2f}% max {max(dd) * 100 if dd else float('nan'):.2f}%, "
              f"tooth mean {np.mean(tts):.1f} dB, chatter in {len(chs)}/{k1 - k0 + 1} windows{(' at ~%.0f Hz' % np.median(chs)) if chs else ''}\n")
    if stretches:
        k0, k1 = stretches[-1]
        print(f"last cutting heard at {info[k1][0]:.2f} s ({progs[len(stretches) - 1][0] if len(stretches) <= len(progs) else 'past the program'}); "
              f"the sound after it: level {info[min(k1 + 1, len(info) - 1)][3]:.1f} dB, tooth {info[min(k1 + 1, len(info) - 1)][2]:.1f} dB")
    print("(* = a line that is there in air too: the spindle's own, not chatter)")


if __name__ == "__main__":
    main()
