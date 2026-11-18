import json,sys
for v in sys.argv[1:]:
    print('==',v)
    times={}
    for l in open(v+'.out'):
        try: d=json.loads(l)
        except: continue
        r=d.get('result') or {}
        if d.get('tool')=='cam_op' and r.get('time_s') is not None: times[r['name']]=(r['time_s'], [w[:70] for w in r.get('warnings',[]) if 'm/min' not in w and 'flutes are only' not in w])
        if d.get('tool')=='simulate_cam':
            L={x['operation']:x for x in r['loads']}; C={x['operation']:x for x in r['chatter']}; H={x['operation']:x for x in r['chips']}; A={x['operation']:x for x in r['air']}
            for op in A:
                x=L.get(op,{})
                print('%-30s %6ss air %3d%% pow %3s%% worst %3s%% chat %4s chip %4s %s'%(op[:30],times.get(op,('?',))[0],A[op]['air_pct'],x.get('spindle_power_pct','-'),round(x['load']*100) if x else '-',C.get(op,{}).get('margin','-'),H.get(op,{}).get('chip_fill','-'),times.get(op,(0,[]))[1]))
            print('problems',len(r['problems']), 'unfinished', (r.get('unfinished') or {}).get('area_mm2'))
        if 'error' in d: print('ERR',d['error'][:150])
