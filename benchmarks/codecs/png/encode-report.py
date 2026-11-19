#!/usr/bin/env python3
"""Summarize encode-paired.py output: python3 encode-report.py RESULT.json CANDIDATE BASE"""
import json, math, statistics, sys, collections, random
d = json.load(open(sys.argv[1])); cand, base = sys.argv[2:4]
by = collections.defaultdict(dict)
for s in d['samples']:
    if 'bytes' in s: by[(s['file'], s['effort'], s['tool'])][s['round']] = s
def cls(f):
    n = f.split('/')[-1]
    return n.split('-')[0] if '/pngdev/' in f else 'legacy'
geo = lambda v: math.exp(sum(map(math.log, v)) / len(v))
rng = random.Random(5)
for effort in ('fast', 'balanced', 'best'):
    groups = collections.defaultdict(list)
    for (f, e, t), rounds in by.items():
        if e != effort or t != cand or (f, e, base) not in by: continue
        b = by[(f, e, base)]
        ratio = statistics.median(rounds[r]['encode_ms'] / b[r]['encode_ms'] for r in rounds if r in b)
        groups[cls(f)].append((f, ratio, rounds[0]['bytes'], b[0]['bytes'], statistics.median(x['encode_ms'] for x in rounds.values()), statistics.median(x['encode_ms'] for x in b.values())))
    print(effort)
    allrows = [x for v in groups.values() for x in v]
    for name, rows in sorted(groups.items()) + [('ALL', allrows)]:
        r = [x[1] for x in rows]; boot = sorted(geo([rng.choice(r) for _ in r]) for _ in range(2000))
        tb = sum(x[2] for x in rows) / sum(x[3] for x in rows); tt = sum(x[4] for x in rows) / sum(x[5] for x in rows)
        worst_b = max(rows, key=lambda x: x[2] / x[3]); worst_t = max(rows, key=lambda x: x[1])
        print(f'  {name:10} n={len(rows):3} time geomean {geo(r):.3f} [{boot[50]:.3f},{boot[1949]:.3f}] total {tt:.3f} worst {worst_t[1]:.2f} | bytes {tb:.5f} worst {worst_b[2]/worst_b[3]:.4f} ({worst_b[0].split("/")[-1]})')
