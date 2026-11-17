"""Read what rcnc heard during op 1a and set it against what rCAD predicted, pass by pass.

    python3 analyse.py op1a-calibrate.nc.cutlog.json [op1a-calibrate.nc.air.cutlog.json] [predicted.json]

The cut log (rcnc's listen crate) keys every program line by its operation's name: "C4 width ladder: 1.8 x 6
F2400 S24000", the pass's name in the program. Per pass this sums the lines that ran while cutting and lists:
- the spindle's speed droop it heard (the VFD slips with torque, so droop stands in for the spindle's load);
- the teeth's beat level (tooth-passing frequency, dB) and the overall level; with the air run beside it, how
  far over the air run's both stood (the cut's own vibration);
- how often chatter was heard and at what frequency;
- what rCAD predicted: spindle %, watts, chatter index.
Then it fits:
- spindle efficiency, as rCAD's import_cutlog does: droop / rated slip = the share of the shaft power used, so
  efficiency = predicted power / (that share x 1500 W x min(rpm/24000, 1)); the median over the passes;
- how droop follows predicted power (a straight line through the passes: a good fit means the cutting model's
  ordering of the cuts is right, its slope is the slip per watt);
- the chatter scale: the chattering pass with the lowest predicted index should read 1 (rCAD's chatter_scale
  0.078 today), and whether any clean pass would then read 1 or more (then the index can't separate them).
predicted.json (written by predicted.py from rCAD's simulation of op 1a) holds rCAD's numbers per pass.
"""
import json
import math
import statistics
import sys

RATED_SLIP = 0.04       # rCAD Machine.spindle_rated_slip default
SPINDLE_IN = 1500.0
RATED_RPM = 24000.0
CHATTER_SCALE = 0.078


def passes(log):
    """Per pass name: the lines' windows merged (weighted by windows heard)."""
    out = {}
    for line_no, l in sorted(log["lines"].items(), key=lambda kv: int(kv[0])):
        name = l.get("op", "")
        if not name.startswith("C") or not l.get("cutting"):
            continue
        p = out.setdefault(name, {"frames": 0, "heard": 0, "chatter": 0, "droop": [], "droop_max": 0.0, "loud": [], "loud_max": -1e9,
                                  "tooth": [], "chatter_hz": [], "rpm": l["rpm"], "feed": 0.0, "lines": []})
        p["lines"].append(int(line_no))
        p["frames"] += l["frames"]
        p["heard"] += l["heard"]
        p["chatter"] += l["chatter"]
        p["feed"] = max(p["feed"], l["feed"])
        if l["heard"]:
            p["droop"].append((l["droop_mean"], l["heard"]))
            p["droop_max"] = max(p["droop_max"], l["droop_max"])
        if l["frames"]:
            p["loud"].append((l["loud_mean_db"], l["frames"]))
            p["loud_max"] = max(p["loud_max"], l["loud_max_db"])
        if l.get("tooth_frames"):
            p["tooth"].append((l["tooth_mean_db"], l["tooth_frames"]))
        if l["chatter"]:
            p["chatter_hz"].append(l["chatter_hz"])
    def wmean(v):
        n = sum(w for _, w in v)
        return sum(x * w for x, w in v) / n if n else None
    for p in out.values():
        p["droop_mean"] = wmean(p["droop"])
        p["loud_mean"] = wmean(p["loud"])
        p["tooth_mean"] = wmean(p["tooth"])
    return out


def key(name):
    """'C4 width ladder: ...' -> 'C4'."""
    return name.split()[0]


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return
    cut = passes(json.load(open(sys.argv[1])))
    air = passes(json.load(open(sys.argv[2]))) if len(sys.argv) > 2 and sys.argv[2].endswith(".json") and "air" in sys.argv[2] else {}
    pred_path = next((a for a in sys.argv[2:] if a.endswith("predicted.json")), "predicted.json")
    try:
        pred = json.load(open(pred_path))
    except OSError:
        pred = {}
    air_by = {key(n): p for n, p in air.items()}
    rows, fit = [], []
    print(f"{'pass':5} {'rpm':>6} {'feed':>5} {'pred %':>6} {'pred W':>6} {'droop %':>7} {'max %':>6} {'tooth dB':>8} {'over air':>8} {'level dB':>8} {'over air':>8} {'chatter':>8} {'Hz':>6} {'pred idx':>8}")
    for name, p in sorted(cut.items(), key=lambda kv: int(key(kv[0])[1:])):
        k = key(name)
        pr = pred.get(k, {})
        a = air_by.get(k, {})
        d = p["droop_mean"]
        # the air run's own droop at that speed (the motor's slip with nothing to cut) comes off
        d_air = a.get("droop_mean") if a else None
        d_net = d - d_air if d is not None and d_air is not None else d
        t_over = p["tooth_mean"] - a["tooth_mean"] if a and p["tooth_mean"] is not None and a.get("tooth_mean") is not None else None
        l_over = p["loud_mean"] - a["loud_mean"] if a and p["loud_mean"] is not None and a.get("loud_mean") is not None else None
        share = p["chatter"] / p["frames"] if p["frames"] else 0.0
        hz = statistics.median(p["chatter_hz"]) if p["chatter_hz"] else None
        f = lambda v, fmt: (fmt % v) if v is not None else "–"
        print(f"{k:5} {p['rpm']:6.0f} {p['feed']:5.0f} {f(pr.get('pct'), '%6.0f')} {f(pr.get('watts'), '%6.0f')} {f(d_net * 100 if d_net is not None else None, '%7.2f')} "
              f"{p['droop_max'] * 100:6.2f} {f(p['tooth_mean'], '%8.1f')} {f(t_over, '%8.1f')} {f(p['loud_mean'], '%8.1f')} {f(l_over, '%8.1f')} {share * 100:7.0f}% {f(hz, '%6.0f')} {f(pr.get('chatter'), '%8.2f')}")
        rows.append((k, p, pr, d_net, share))
        if d_net is not None and pr.get("watts"):
            fit.append((pr["watts"], d_net, p["rpm"]))

    print()
    if len(fit) >= 3:
        # efficiency per pass, then the median (rCAD import_cutlog's rule)
        effs = []
        for w, d, rpm in fit:
            used = d / RATED_SLIP
            avail_in = SPINDLE_IN * min(rpm / RATED_RPM, 1.0)
            if used > 0.01:
                effs.append(w / (used * avail_in))
        if effs:
            print(f"spindle efficiency (median of {len(effs)} passes): {statistics.median(effs):.2f}   (rCAD now 0.78; spread {min(effs):.2f}-{max(effs):.2f})")
        xs, ys = [w for w, _, _ in fit], [d for _, d, _ in fit]
        mx, my = statistics.mean(xs), statistics.mean(ys)
        sxx = sum((x - mx) ** 2 for x in xs)
        sxy = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
        syy = sum((y - my) ** 2 for y in ys)
        if sxx > 0 and syy > 0:
            slope = sxy / sxx
            r = sxy / math.sqrt(sxx * syy)
            print(f"droop vs predicted power: {slope * 1000 * 100:.2f} % per 1000 W, r = {r:.2f}  (r near 1: the model orders the cuts' loads right)")
            print(f"   at the rated 1170 W of shaft power that line gives {slope * 1170 * 100 + (my - slope * mx) * 100:.2f} % droop (rCAD assumes {RATED_SLIP * 100:.0f} %)")
    else:
        print("not enough passes with droop heard to fit the spindle (is the transducer picking up the teeth? see the tooth column)")

    chattered = [(k, pr.get("chatter")) for k, p, pr, d, share in rows if share > 0.3 and pr.get("chatter") is not None]
    clean = [(k, pr.get("chatter")) for k, p, pr, d, share in rows if share < 0.05 and pr.get("chatter") is not None]
    if chattered:
        lo = min(chattered, key=lambda c: c[1])
        scale = CHATTER_SCALE / lo[1]
        print(f"chatter heard on {', '.join(k for k, _ in chattered)}; the lowest predicted index among them is {lo[0]} at {lo[1]:.2f}:")
        print(f"   chatter_scale {scale:.3f} would make it read 1 (now {CHATTER_SCALE})")
        over = [k for k, m in clean if m / lo[1] >= 1.0]
        if over:
            print(f"   but the clean passes {', '.join(over)} would then read 1 or more: the index doesn't separate them")
    else:
        top = max(clean, key=lambda c: c[1]) if clean else None
        print("no chatter heard on any pass" + (f": the highest predicted index was {top[0]} at {top[1]:.2f}, so the scale could go up to {CHATTER_SCALE / top[1]:.3f} before predicting chatter there" if top else ""))
    print("\nrCAD can take the log directly too: import_cutlog {cam: 0, path: <this cut log>, apply: false} on gauge-cam.json shows its own fit.")


if __name__ == "__main__":
    main()
