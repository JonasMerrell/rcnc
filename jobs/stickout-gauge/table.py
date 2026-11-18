"""Per-step summary of an rCAD run's last simulation: python3 table.py cam.out"""
import json, sys, collections
rs = [json.loads(l) for l in open(sys.argv[1])]
res = [r for r in rs if r['tool'] == 'simulate_cam'][-2]['result']
info = [r for r in rs if r['tool'] == 'cam_info'][-1]['result']
t = {o['id']: o['time_s'] for o in info['operations']}
ch = {c['op_id']: c for c in res['chips']}
g = collections.OrderedDict()
for o in info['operations']:
    n = o['name']
    key = n.split(', ')[0] if n.startswith(('Band', 'Lug', 'Side', 'Carrier')) else n
    e = g.setdefault(key, dict(n=0, p=0, w=0, mrr=0, t=0, chip=0, pct=0))
    e['n'] += 1; e['t'] += o['time_s']
for L in res['loads']:
    n = L['operation']
    key = n.split(', ')[0] if n.startswith(('Band', 'Lug', 'Side', 'Carrier')) else n
    e = g[key]
    e['p'] = max(e['p'], L['power_w']); e['w'] = max(e['w'], L['width']); e['mrr'] = max(e['mrr'], L['mrr_cm3_min'])
    e['pct'] = max(e['pct'], L['spindle_power_pct']); e['chip'] = max(e['chip'], ch.get(L['op_id'], {}).get('chip_fill', 0))
for k, e in g.items():
    print(f"{k[:75]:75s} n{e['n']:3d} {e['p']:4.0f}W {e['pct']:3.0f}% w{e['w']:.2f} mrr{e['mrr']:5.1f} t{e['t']:6.0f} chip{e['chip']:.2f}")
print('cycle', info['cycle_time_s'], 's; ok', res['ok'], 'problems', res['problems'][:5])
print('unfinished', res.get('unfinished'))
