#!/usr/bin/env python3
"""Summarize run.py output: python3 report.py RESULT.json NEW BASE ZEN, labels suffixed F/B/X per effort."""
import json, statistics, collections, math, sys
d = json.load(open(sys.argv[1])); new, base, zen = sys.argv[2:5]
g = collections.defaultdict(list)
for r in d['records']:
    if r['ok']: g[(r['input'], r['tool'])].append(r)
w = lambda rs: statistics.median(x['wall'] for x in rs); c = lambda rs: statistics.median(x['user'] + x['sys'] for x in rs); m = lambda rs: max(x['rss'] for x in rs)
gm = lambda v: math.exp(sum(map(math.log, v)) / len(v))
inputs = sorted({k[0] for k in g}); large = [i for i in inputs if '/pngdev/' in i]
for e in 'FBX':
    for ref in (base, zen):
        for name, sel in (('small', [i for i in inputs if i not in large]), ('large', large)):
            rows = [i for i in sel if (i, new + e) in g and (i, ref + e) in g]
            if not rows: continue
            W = [w(g[(i, new + e)]) / w(g[(i, ref + e)]) for i in rows]; C = [c(g[(i, new + e)]) / c(g[(i, ref + e)]) for i in rows]; M = [m(g[(i, new + e)]) / m(g[(i, ref + e)]) for i in rows]
            B = sum(g[(i, new + e)][0]['bytes'] for i in rows) / sum(g[(i, ref + e)][0]['bytes'] for i in rows)
            print(f'{e} {new}/{ref:5} {name:5} n={len(rows):2} wall {gm(W):.3f} (max {max(W):.2f}) cpu {gm(C):.3f} (max {max(C):.2f}) rss {gm(M):.3f} (max {max(M):.2f}) bytes {B:.4f}')
