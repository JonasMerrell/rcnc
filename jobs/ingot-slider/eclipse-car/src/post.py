"""After rCAD posts: bores pause before op 2's Find bores, 3 s dwells after S changes, Find stock from the vise
location (car op 2 grips the 17.3 wall: JAWAT=-8.65), REQUIRE_GUARD; then verify.json (verify_gcode on every final file)."""
import json, subprocess
H = "/home/jonas/dmc2/eclipse-car/"
def run(*a):
    print(subprocess.run(["python3", H + a[0], *a[1:]], capture_output=True, text=True, check=True).stdout.strip())
has = lambda p, t: t in open(p).read()
CAR = [f"eclipse-car/eclipse-car-{n}.nc" for n in ("op1", "op2", "cycle", "first", "last")]
BASE = [f"eclipse-base-thick/eclipse-base-thick-{n}.nc" for n in ("op1", "op2", "cycle", "first", "last")]
for p in CAR + BASE:
    if any(k in p for k in ("op2", "cycle", "last")) and not has(p, "BORES PAUSE"):
        run("add_bores_pause.py", p)
    run("add_dwell.py", p)
    run("add_vise.py", p)
    if not has(p, "REQUIRE_GUARD"):
        run("mark_guard.py", p)
V = [{"tool": "open_document", "args": {"path": H + "eclipse-car-cam.json"}}]
for lead, op2, st, files in ((0, 1, ("Eclipse car op1 bottom", "Eclipse car op2 top"), CAR), (2, 3, ("Thick base op1 bottom", "Thick base op2 top"), BASE)):
    for p in files:
        a = {"cam": op2 if p.endswith("op2.nc") else lead, "path": H + p, "resolution": 1000}
        if p.endswith(("op1.nc", "first.nc")):
            a["stations"] = [st[0]]
        if p.endswith("last.nc"):
            a["stations"] = [st[1]]
        V.append({"tool": "verify_gcode", "args": a})
json.dump(V, open(H + "verify.json", "w"), indent=1)
print("verify.json:", len(V) - 1, "files")
