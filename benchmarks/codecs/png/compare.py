#!/usr/bin/env python3
import json, sys, statistics as st, collections
d=json.load(open(sys.argv[1])); R=[r for r in d['records'] if r['ok']]; base=sys.argv[2]; others=sys.argv[3].split(',')
fails=[r for r in d['records'] if not r['ok']]
if fails: print('FAILURES',[(f['tool'],f['input']) for f in fails][:20])
def group(p):
    n=p.rsplit('/',1)[1]
    if '/generated/' in p: return 'synthetic'
    if n in ('photopea-ui.png','text.png','mm-logo-alpha.png') and '/validation/' in p: return 'ui/alpha'
    if n in ('camera.png','coins.png','brick.png','cell.png','clock_motion.png','microaneurysms.png','horse.png'): return 'photo-gray'
    return 'photo'
m=collections.defaultdict(list)
for r in R: m[(r['tool'],r['input'])].append(r)
def v(t,i,k):
    rs=m[(t,i)]; return rs[0]['bytes'] if k=='bytes' else st.median(x[k] for x in rs)
inputs=sorted({r['input'] for r in R})
print(f'vs {base}: group: bytes ratio | wall ratio | cpu ratio | rss ratio (sums) | per-image losses (bytes>0%, wall>5%)')
for o in others:
    g=collections.defaultdict(lambda: collections.defaultdict(float)); L=[]
    for i in inputs:
        if not m.get((o,i)) or not m.get((base,i)): continue
        a=g[group(i)]
        for k in ('bytes','wall','user','rss'): a['b'+k]+=v(base,i,k); a['o'+k]+=v(o,i,k)
        a['n']+=1
        n=i.rsplit('/',1)[1]
        if v(o,i,'bytes')>v(base,i,'bytes'): L.append(f"{n}:B+{100*(v(o,i,'bytes')/v(base,i,'bytes')-1):.1f}%")
        if v(o,i,'wall')>1.05*v(base,i,'wall'): L.append(f"{n}:T{v(o,i,'wall')/v(base,i,'wall'):.2f}")
    print(f' {o:10} '+' | '.join(f"{k}(n{int(a['n'])}) B{a['obytes']/a['bbytes']:.3f} T{a['owall']/a['bwall']:.2f} C{(a['ouser'])/max(a['buser'],1e-9):.2f} M{a['orss']/a['brss']:.2f}" for k,a in sorted(g.items())))
    print('            losses:',' '.join(L) if L else 'none')
