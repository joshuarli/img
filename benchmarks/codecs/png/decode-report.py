#!/usr/bin/env python3
"""Summarize decode-qualify.py output: python3 decode-report.py RESULT.json CANDIDATE REFERENCE..."""
import json, math, random, statistics, sys

record = json.load(open(sys.argv[1]))
candidate, references = sys.argv[2], sys.argv[3:]
by = {}
for s in record['samples']:
    by.setdefault((s['file'], s['decoder']), {})[s['round']] = s['mean_ms']
rss = {(m['file'], m['decoder']): m['peak_rss_bytes'] for m in record['memory']}
classes = {}
for f in record['fixtures']:

    for cls in f['classes']: classes.setdefault(cls, []).append(f['file'])
    classes.setdefault('all', []).append(f['file'])
rng = random.Random(7)
def geo(values): return math.exp(sum(map(math.log, values)) / len(values))
for reference in references:
    print(f'{candidate} / {reference}: class n geomean [95% CI] p95 worst(file) | rss geomean worst')
    for cls, files in classes.items():
        ratios = []
        for f in files:
            a, b = by[(f, candidate)], by[(f, reference)]
            ratios.append((statistics.median(a[r] / b[r] for r in a if r in b), f))
        values = [r for r, _ in ratios]
        boot = sorted(geo([rng.choice(values) for _ in values]) for _ in range(2000))
        ordered = sorted(values)
        p95 = ordered[min(len(ordered) - 1, math.ceil(0.95 * len(ordered)) - 1)]
        worst = max(ratios)
        mem = [rss[(f, candidate)] / rss[(f, reference)] for f in files]
        print(f'  {cls:14} {len(files):3} {geo(values):.3f} [{boot[50]:.3f}, {boot[1949]:.3f}] {p95:.3f} {worst[0]:.3f}({worst[1]}) | {geo(mem):.3f} {max(mem):.3f}')
