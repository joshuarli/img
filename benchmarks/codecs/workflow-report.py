#!/usr/bin/env python3
"""python3 workflow-report.py RUN.json NEW BASE: per-case median ratios NEW/BASE, geomean with bootstrap 95% CI, p95, worst."""
import json, math, random, statistics, sys
from collections import defaultdict
doc = json.load(open(sys.argv[1])); new, base = sys.argv[2], sys.argv[3]
by = defaultdict(lambda: defaultdict(list))
for s in doc['samples']:
    if s['exit'] == 0: by[(s['input'], s['format'], s['score'])][s['tool']].append(s)
def summary(metric, keys):
    ratios = [(k, statistics.median(x[metric] for x in by[k][new]) / statistics.median(x[metric] for x in by[k][base])) for k in keys]
    logs = [math.log(v) for _, v in ratios]; rng = random.Random(0)
    boot = sorted(math.exp(statistics.fmean(rng.choices(logs, k=len(logs)))) for _ in range(2000))
    vals = sorted(v for _, v in ratios); worst = max(ratios, key=lambda kv: kv[1])
    return f'{metric:15} n={len(ratios):3} geomean {math.exp(statistics.fmean(logs)):.3f} [{boot[50]:.3f}, {boot[1949]:.3f}] p95 {vals[min(len(vals) - 1, int(0.95 * len(vals)))]:.3f} worst {worst[1]:.3f} {worst[0]}'
keys = [k for k in by if by[k][new] and by[k][base]]
same = sum(1 for k in keys if {x['sha256'] for x in by[k][new]} == {x['sha256'] for x in by[k][base]})
failed = sorted({(s['tool'], s['input'], s['format'], s['score']) for s in doc['samples'] if s['exit']})
print(f'{len(keys)} cases, identical outputs {same}, failed cases {len(failed)}, gated {doc.get("gated", True)}, discarded rounds {doc["discarded_rounds"]}')
for f in failed: print('  failed', *f)
ok = {(s['tool'], s['input'], s['format'], s['score']) for s in doc['samples'] if s['exit'] == 0}
lost = sorted({k for t, *k in failed if t == new and (base, *k) in ok})
for group in sorted({k[1] for k in keys}) + ['all']:
    if not keys: break
    ks = [k for k in keys if group in ('all', k[1])]
    print(group); [print('  ' + summary(m, ks)) for m in ('wall', 'cpu', 'peak_rss_bytes', 'bytes')]
if lost:
    print(f'{new} failed cases {base} completed:', *lost); sys.exit(1)
