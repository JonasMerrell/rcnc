"""Spindle rpm (harmonic-sum pitch of the rotation tone, harmonics 1-6) and RMS vs time for a cut recording,
plus the passes, the break and a 'feed hold at >= 6 % sag for >= 0.3 s' check.
usage: python timeline2.py rec.wav out.png out.json"""
import sys, json, numpy as np
from scipy.io import wavfile
from scipy.signal import stft
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

wav, out_png, out_json = sys.argv[1:4]
fs, x = wavfile.read(wav)
x = x.astype(np.float64) / 32768.0
dur = len(x) / fs
WIN, HOP = 0.2, 0.05
nper, hop = int(WIN * fs), int(HOP * fs)
f, t, Z = stft(x, fs, window="hann", nperseg=nper, noverlap=nper - hop, nfft=1 << 15, boundary=None, padded=False)
A = np.abs(Z).astype(np.float32); del Z
df = f[1] - f[0]
n = A.shape[1]
rms_db = np.array([20 * np.log10(np.sqrt(np.mean(x[j * hop:j * hop + nper] ** 2)) + 1e-12) for j in range(n)])
cands = np.arange(250.0, 410.0, 0.1)
logA = np.log(A + 1e-9)
score = sum(logA[np.round(cands * k / df).astype(int)] for k in range(1, 7))
rpm = cands[np.argmax(score, axis=0)] * 60.0
spin = rms_db > -30.0
rpm_plot = np.where(spin, rpm, np.nan)

# segments: spindle-on air vs cutting (cut ~10 dB louder than air)
air_mask = spin & (rms_db < -15) & (t > 139.8)
air_rpm = float(np.median(rpm[air_mask]))
cut = spin & (rms_db > -13)
edges = np.flatnonzero(np.diff(np.r_[0, cut.astype(int), 0]))
segs = [(t[a], t[b - 1]) for a, b in zip(edges[::2], edges[1::2]) if t[b - 1] - t[a] > 1.0]
names = ["C1 0.6 x 6 F2400", "C2 1.0 x 6 F2400", "C3 1.4 x 6 F2400", "C4 1.8 x 6 F2400"]

# break: first jump of >= 8 % back up within 0.1 s during the last pass
last = segs[-1]
jj = np.flatnonzero((t >= last[0]) & (t <= last[1]))
brk = None
for a, b in zip(jj[:-2], jj[2:]):
    if rpm[b] - rpm[a] > 0.08 * air_rpm:
        brk = float(t[b - 1]); break   # last coherent window; the tone falls apart in the next

def guard(ref, lo, hi):
    """first time the rpm has been >= 6 % below ref for >= 0.3 s within [lo, hi]."""
    since = None
    for j in np.flatnonzero((t >= lo) & (t <= hi)):
        if rpm[j] <= 0.94 * ref:
            since = t[j] if since is None else since
            if t[j] - since >= 0.3 - 1e-9:
                return float(since), float(t[j])
        else:
            since = None
    return None

rep = {"duration_s": dur, "air_rpm": air_rpm, "break_s": brk, "end_s": dur, "passes": []}
for i, (a, b) in enumerate(segs):
    m = (t >= a + 0.25) & (t <= (b - 0.1 if brk is None or b < brk else brk))
    p = {"name": names[i] if i < len(names) else f"pass {i+1}", "start_s": float(a), "end_s": float(b),
         "rpm_first_0.5s": float(np.median(rpm[(t >= a + 0.1) & (t <= a + 0.6)])),
         "rpm_min": float(rpm[m].min()), "t_rpm_min": float(t[m][np.argmin(rpm[m])]),
         "rms_mean_db": float(np.mean(rms_db[m])), "rms_max_db": float(rms_db[m].max())}
    p["sag_min_vs_air_pct"] = 100 * (1 - p["rpm_min"] / air_rpm)
    p["guard_vs_air"] = guard(air_rpm, a, brk if (brk and a < brk < b + 1) else b)
    p["guard_vs_pass_start"] = guard(p["rpm_first_0.5s"], a, brk if (brk and a < brk < b + 1) else b)
    # when the sag began: first window 2 % below air that stays below until the min
    below = np.flatnonzero(m & (rpm <= 0.98 * air_rpm))
    p["sag_2pct_from_s"] = float(t[below[0]]) if len(below) else None
    rep["passes"].append(p)
json.dump(rep, open(out_json, "w"), indent=1)
print(json.dumps(rep, indent=1))

fig, (ax1, ax2) = plt.subplots(2, 1, sharex=True, figsize=(13, 7), gridspec_kw={"height_ratios": [3, 2]})
pre = t <= (brk if brk else dur)
ax1.plot(t[pre], rpm_plot[pre], lw=0.8, color="tab:blue", label="heard spindle speed (rotation tone)")
ax1.plot(t[~pre], rpm_plot[~pre], ".", ms=2, color="lightgray", label="after the break (no clean tone)")
ax1.axhline(air_rpm, color="gray", ls=":", lw=1, label=f"air {air_rpm:.0f} rpm")
ax1.axhline(0.94 * air_rpm, color="tab:red", ls="--", lw=1, label="6 % below air (feed-hold limit)")
ax2.plot(t, rms_db, lw=0.6, color="tab:green")
for ax in (ax1, ax2):
    for i, (a, b) in enumerate(segs):
        ax.axvspan(a, b, color="orange", alpha=0.18)
    if brk: ax.axvline(brk, color="red", lw=1.2)
    ax.axvline(dur, color="black", lw=1.5)
for i, (a, b) in enumerate(segs):
    ax1.text((a + b) / 2, 24250, names[i].split()[0] if i < len(names) else "", ha="center", fontsize=8)
if brk: ax1.text(brk, 17000, "break ", color="red", fontsize=8, ha="right")
ax1.text(dur, 15300, " E-stop", ha="left", fontsize=8)
for p in rep["passes"]:
    g = p["guard_vs_air"]
    if g: ax1.plot([g[1]], [0.94 * air_rpm], "rv", ms=7)
ax1.set_ylim(15000, 24500); ax1.set_ylabel("rpm"); ax1.legend(loc="lower left", fontsize=8)
ax1.set_title("op1a-calibrate (32 mm bar), T24 6.35 mm 3FL, 2026-11-17 05:24-05:26Z: spindle speed and level")
ax2.set_ylim(-80, 2); ax2.set_ylabel("RMS dBFS (0.2 s)"); ax2.set_xlabel("s from start of recording (05:23:58Z)")
fig.tight_layout(); fig.savefig(out_png, dpi=130)
# zoom
fig2, ax = plt.subplots(figsize=(10, 4))
z = (t > 143) & (t < dur)
ax.plot(t[z & pre], rpm_plot[z & pre], lw=1); ax.plot(t[z & ~pre], rpm_plot[z & ~pre], ".", ms=2, color="lightgray")
for p in rep["passes"]:
    g = p["guard_vs_air"]
    if g: ax.plot([g[1]], [0.94 * air_rpm], "rv", ms=7)
ax.axhline(0.94 * air_rpm, color="tab:red", ls="--", lw=1); ax.axhline(air_rpm, color="gray", ls=":")
for a, b in segs: ax.axvspan(a, b, color="orange", alpha=0.18)
if brk: ax.axvline(brk, color="red")
ax.set_ylim(15000, 24500); ax.set_xlabel("s"); ax.set_ylabel("rpm"); ax.set_title("passes C1-C4, zoom")
fig2.tight_layout(); fig2.savefig(out_png.replace(".png", "-zoom.png"), dpi=130)


