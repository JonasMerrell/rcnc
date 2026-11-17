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
SPINDLE_IN = 1500.0       # rCAD: 1500 W x 0.43 = 645 W, the measured pull-out at 24k
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


def passes_from_timeline(log, cal):
    """Per pass from the cut log's timeline (rcnc's listen-guard build): each window sorted to its pass by where
    the tool was (centre x within 0.5 mm of the pass's line, at its floor), not by the line number."""
    out = {}
    for m in log.get("timeline", []):
        if not m.get("cutting") or m.get("velocity", 0) <= 1:
            continue
        x, z = m["pos"][0], m["pos"][2]
        hit = [c for c in cal if abs(x - c[7]) < 0.5 and abs(z - (32.0 - c[8])) < 0.5]
        if not hit:
            continue
        c = hit[0]
        name = f"{c[1]} {c[2]}: {c[3]:g} x {c[4]:g} F{c[5]} S{c[6]}"
        p = out.setdefault(name, {"frames": 0, "heard": 0, "chatter": 0, "droop": [], "droop_max": 0.0, "loud": [], "loud_max": -1e9,
                                  "tooth": [], "chatter_hz": [], "rpm": c[6], "feed": 0.0, "lines": [], "t": [m["t"], m["t"]]})
        p["t"][1] = m["t"]
        p["frames"] += 1
        p["feed"] = max(p["feed"], m.get("velocity", 0))
        if m.get("droop") is not None:
            p["heard"] += 1
            p["droop"].append((m["droop"], 1))
            p["droop_max"] = max(p["droop_max"], m["droop"])
        p["loud"].append((m["loud_db"], 1))
        p["loud_max"] = max(p["loud_max"], m["loud_db"])
        if m.get("tooth_db") is not None:
            p["tooth"].append((m["tooth_db"], 1))
        if m.get("chatter"):
            p["chatter"] += 1
            p["chatter_hz"].append(m["chatter"][0])
    def wmean(v):
        n = sum(w for _, w in v)
        return sum(x * w for x, w in v) / n if n else None
    for p in out.values():
        p["droop_mean"] = wmean(p["droop"])
        p["loud_mean"] = wmean(p["loud"])
        p["tooth_mean"] = wmean(p["tooth"])
    return out


def last_seconds(log, secs=3.0):
    """The timeline's last seconds (a run stopped by E-stop or a feed hold)."""
    tl = log.get("timeline", [])
    if not tl:
        return
    end = tl[-1]["t"]
    print(f"\nthe last {secs:g} s of the run (ends at {end:.2f} s):")
    print(f"{'t':>8} {'line':>5} {'cut':>4} {'x':>8} {'y':>8} {'z':>7} {'mm/min':>7} {'droop %':>8} {'tooth dB':>8} {'level dB':>8} {'chatter':>8}")
    for m in tl:
        if m["t"] >= end - secs:
            d = m.get("droop")
            c = m.get("chatter")
            print(f"{m['t']:8.2f} {m['line']:5d} {'yes' if m['cutting'] else '':>4} {m['pos'][0]:8.2f} {m['pos'][1]:8.2f} {m['pos'][2]:7.2f} {m.get('velocity', 0):7.0f} "
                  f"{(d * 100) if d is not None else float('nan'):8.2f} {m.get('tooth_db') if m.get('tooth_db') is not None else float('nan'):8.1f} {m['loud_db']:8.1f} {(c[0] if c else 0):8.0f}")


def key(name):
    """'C4 width ladder: ...' -> 'C4'."""
    return name.split()[0]


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return
    log = json.load(open(sys.argv[1]))
    try:
        cal = json.load(open("cal_passes.json"))
    except OSError:
        cal = []
    # the timeline sorts windows to passes by position (the line can be wrong on the real board); else by line
    cut = passes_from_timeline(log, cal) if log.get("timeline") and cal else passes(log)
    # wall_watts.json (filled in by hand from the meter): {"air": {"24000": W, ...}, "C9": W, ...}
    wall_path = next((a for a in sys.argv[2:] if a.endswith("wall_watts.json")), "wall_watts.json")
    try:
        wall = json.load(open(wall_path))
    except OSError:
        wall = {}
    air_path = next((a for a in sys.argv[2:] if "air" in a and a.endswith("cutlog.json")), None)
    air = {}
    if air_path:
        alog = json.load(open(air_path))
        air = passes_from_timeline({"timeline": [dict(m, pos=[m["pos"][0], m["pos"][1], m["pos"][2] - 25.0]) for m in alog.get("timeline", [])]}, cal) if alog.get("timeline") and cal else passes(alog)
    pred_path = next((a for a in sys.argv[2:] if a.endswith("predicted.json")), "predicted.json")
    try:
        pred = json.load(open(pred_path))
    except OSError:
        pred = {}
    air_by = {key(n): p for n, p in air.items()}
    rows, fit = [], []
    print(f"{'pass':5} {'rpm':>6} {'feed':>5} {'pred %':>6} {'pred W':>6} {'wall W':>6} {'cut W':>6} {'droop %':>7} {'max %':>6} {'tooth dB':>8} {'over air':>8} {'level dB':>8} {'over air':>8} {'chatter':>8} {'Hz':>6} {'pred idx':>8}")
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
        ww = wall.get(k)
        aw = wall.get("air", {}).get(str(int(p["rpm"])))
        cut_w = ww - aw if ww is not None and aw is not None else None
        print(f"{k:5} {p['rpm']:6.0f} {p['feed']:5.0f} {f(pr.get('pct'), '%6.0f')} {f(pr.get('watts'), '%6.0f')} {f(ww, '%6.0f')} {f(cut_w, '%6.0f')} {f(d_net * 100 if d_net is not None else None, '%7.2f')} "
              f"{p['droop_max'] * 100:6.2f} {f(p['tooth_mean'], '%8.1f')} {f(t_over, '%8.1f')} {f(p['loud_mean'], '%8.1f')} {f(l_over, '%8.1f')} {share * 100:7.0f}% {f(hz, '%6.0f')} {f(pr.get('chatter'), '%8.2f')}")
        rows.append((k, p, pr, d_net, share, cut_w))
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
            print(f"   at the pull-out (645 W by the model) that line gives {slope * 645 * 100 + (my - slope * mx) * 100:.2f} % droop (rCAD assumes {RATED_SLIP * 100:.0f} %)")
    else:
        print("not enough passes with droop heard to fit the spindle (is the transducer picking up the teeth? see the tooth column)")

    chattered = [(k, pr.get("chatter")) for k, p, pr, d, share, cw in rows if share > 0.3 and pr.get("chatter") is not None]
    clean = [(k, pr.get("chatter")) for k, p, pr, d, share, cw in rows if share < 0.05 and pr.get("chatter") is not None]
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
    # the wall meter: what the spindle really drew in the cut (wall minus air at that speed) against the model
    meas = [(pr.get("watts"), cw, d) for k, p, pr, d, share, cw in rows if cw is not None and pr.get("watts")]
    if len(meas) >= 2:
        ratio = statistics.median(c / m for m, c, _ in meas if m)
        print(f"\nwall meter: the cut drew {ratio:.2f} x the model's watts from the wall (median over {len(meas)} passes):")
        print("   that ratio is 1 / (motor x VFD efficiency) if the model's watts are right at the tool")
        for m, c, d in meas:
            print(f"   model {m:5.0f} W   wall-air {c:5.0f} W   sag {(d or 0) * 100:5.2f} %")
    last_seconds(log)
    print("\nrCAD can take the log directly too: import_cutlog {cam: 0, path: <this cut log>, apply: false} on gauge-cam.json shows its own fit.")


if __name__ == "__main__":
    main()
