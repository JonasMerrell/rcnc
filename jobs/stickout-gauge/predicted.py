"""rCAD's predictions for op 1a's passes, from cam.out (rcad run cam.json): writes predicted.json for analyse.py."""
import json
outs = [json.loads(l) for l in open("cam.out") if l.startswith("{")]
sim = [d for d in outs if d.get("tool") == "simulate_cam"][0]["result"]
ch = {x["operation"]: x for x in sim["chatter"]}
pred = {}
for x in sim["loads"]:
    k = x["operation"].split()[0]
    if k.startswith("C"):
        pred[k] = {"name": x["operation"], "pct": x["spindle_power_pct"], "watts": x["power_w"], "width": x["width"], "depth": x["depth"],
                   "force": x["force_n"], "chip": x["chip_mm"], "deflection": x["deflection_mm"], "chatter": ch.get(x["operation"], {}).get("margin")}
json.dump(pred, open("predicted.json", "w"), indent=1)
print(len(pred), "passes")
