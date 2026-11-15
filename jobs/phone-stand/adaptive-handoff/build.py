import json, math
R = 10.0; H = 27.0  # rounded square 74, corner R10
def rsq(h, r):
    a = h - r
    it = []
    pts = [((a, -h), (a, h)), ((h, -a), (h, a))]
    it += [{"type": "line", "from": [-a, -h], "to": [a, -h]},
           {"type": "arc", "center": [a, -a], "start": [a, -h], "end": [h, -a]},
           {"type": "line", "from": [h, -a], "to": [h, a]},
           {"type": "arc", "center": [a, a], "start": [h, a], "end": [a, h]},
           {"type": "line", "from": [a, h], "to": [-a, h]},
           {"type": "arc", "center": [-a, a], "start": [-a, h], "end": [-h, a]},
           {"type": "line", "from": [-h, a], "to": [-h, -a]},
           {"type": "arc", "center": [-a, -a], "start": [-h, -a], "end": [-a, -h]}]
    return it
S = [{"tool": "new_document", "args": {"units": "mm"}},
     {"tool": "import_file", "args": {"path": "/home/jonas/dmc2/stand/stand-top.stl", "name": "stand top"}},      # 1
     {"tool": "create_sketch", "args": {"plane": {"kind": "origin_plane", "plane": "Top"}}},                       # 2
     {"tool": "sketch_add", "args": {"sketch_id": 2, "items": rsq(37, 10)}},
     {"tool": "add_feature", "args": {"kind": "extrude", "params": {"profiles": [{"sketch": 2}], "depth": 40, "op": "intersect"}, "name": "Footprint 74 x 74 R10"}},  # 3
     # underside: 2 mm recess, two locating bores through the groove floor
     {"tool": "create_sketch", "args": {"plane": {"kind": "origin_plane", "plane": "Top"}}},                       # 4
     {"tool": "sketch_add", "args": {"sketch_id": 4, "items": rsq(29, 6)}},
     {"tool": "add_feature", "args": {"kind": "extrude", "params": {"profiles": [{"sketch": 4}], "depth": 2, "op": "remove"}, "name": "Underside recess 2"}},  # 5
     {"tool": "create_sketch", "args": {"plane": {"kind": "origin_plane", "plane": "Top"}}},                       # 6
     {"tool": "sketch_add", "args": {"sketch_id": 6, "items": [{"type": "circle", "center": [-18, -19.25], "radius": 4}, {"type": "circle", "center": [18, -19.25], "radius": 4}]}},
     {"tool": "add_feature", "args": {"kind": "extrude", "params": {"profiles": [{"sketch": 6}], "depth": 12, "op": "remove"}, "name": "2x locating bore 8 through the groove floor"}},  # 7
     {"tool": "describe", "args": {}},
     {"tool": "save_document", "args": {"path": "/home/jonas/dmc2/stand/stand.json"}},
     {"tool": "render", "args": {"path": "t-iso.png", "view": "iso", "width": 1200, "height": 900}}]
json.dump(S, open("build.json", "w"), indent=1)
