import json,sys
for l in open(sys.argv[1]):
    try: d=json.loads(l)
    except: continue
    r=d.get('result') or {}
    if d.get('tool')=='cam_op' and r.get('warnings'): print('WARN', r.get('name','')[:40], [w[:120] for w in r['warnings'] if 'm/min' not in w])
    if d.get('tool')=='simulate_cam':
        L={x['operation']:x for x in r['loads']}; C={x['operation']:x for x in r['chatter']}; H={x['operation']:x for x in r['chips']}; A={x['operation']:x for x in r['air']}
        for o in A:
            x=L.get(o,{})
            print('%-58s air %3d%% pow %4s%% worst %4s w %s d %s chat %s chip %s'%(o[:58],A[o]['air_pct'],x.get('spindle_power_pct','-'),round(x['load']*100) if x else '-',x.get('width','-'),x.get('depth','-'),C.get(o,{}).get('margin','-'),H.get(o,{}).get('chip_fill','-')))
        print('problems',len(r['problems']),[p if isinstance(p,str) else json.dumps(p)[:200] for p in r['problems'][:6]],'unfinished',(r.get('unfinished') or {}).get('area_mm2'))
    if d.get('tool')=='cam_info': print('TIME', json.dumps(r)[:300])
