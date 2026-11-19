#!/usr/bin/env python3
import json, math, random, statistics, sys
r=json.load(open(sys.argv[1])); rng=random.Random(7)
by={}; rss={(m['file'],m['decoder']):m['peak_rss_bytes'] for m in r['memory']}
for s in r['samples']: by.setdefault((s['file'],s['decoder']),{})[s['round']]=s['mean_ms']
def geo(v): return math.exp(sum(map(math.log,v))/len(v))
comparisons=[('rav1d','dav1d'),('img','dav1d-rgb'),('img','zen')]
if ('img-baseline' in {d['label'] for d in r['decoders']}): comparisons.append(('img','img-baseline'))
for a,b in comparisons:
 print(f'{a} / {b}: class n geomean [95% CI] p95 worst(file) | rss geomean worst')
 for cls in ('img444','aom420','large'):
  files=[f['name'] for f in r['fixtures'] if cls in f['classes']]
  pairs=[]
  for f in files:
   x,y=by[f,a],by[f,b]; pairs.append((statistics.median(x[i]/y[i] for i in x if i in y),f))
  values=[v for v,_ in pairs]; boot=sorted(geo([rng.choice(values) for _ in values]) for _ in range(2000)); p95=sorted(values)[math.ceil(.95*len(values))-1]; worst=max(pairs); mem=[rss[f,a]/rss[f,b] for f in files]
  print(f'  {cls:8} {len(files):3} {geo(values):.3f} [{boot[50]:.3f}, {boot[1949]:.3f}] {p95:.3f} {worst[0]:.3f}({worst[1]}) | {geo(mem):.3f} {max(mem):.3f}')
