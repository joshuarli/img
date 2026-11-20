#!/usr/bin/env python3
"""python3 workflow-report.py RUN.json NEW BASE: per-case median ratios NEW/BASE, geomean with bootstrap 95% CI, p95, worst.
A case succeeds for a tool only when every round exited 0 and the verifier accepted its retained output; timing ratios
use only cases both tools completed."""
import json, math, pathlib, random, statistics, sys
from collections import defaultdict
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
sys.dont_write_bytecode = True
import verdict
doc = json.load(open(sys.argv[1])); new, base = sys.argv[2], sys.argv[3]
case = lambda s: (s['input'], s['format'], s['score'])
accepted = {(v['tool'], *case(v)) for v in doc['verification'] if v['accepted']}
exited = defaultdict(set)
for s in doc['samples']: exited[(s['tool'], *case(s))].add(s['exit'])
succeeded = {k for k, codes in exited.items() if codes == {0} and k in accepted}
by = defaultdict(lambda: defaultdict(list))
for s in doc['samples']:
    if (s['tool'], *case(s)) in succeeded: by[case(s)][s['tool']].append(s)
def ratios(metric, keys): return [(k, statistics.median(x[metric] for x in by[k][new]) / statistics.median(x[metric] for x in by[k][base])) for k in keys]
def summary(metric, keys):
    ratios_ = ratios(metric, keys)
    logs = [math.log(v) for _, v in ratios_]; rng = random.Random(0)
    boot = sorted(math.exp(statistics.fmean(rng.choices(logs, k=len(logs)))) for _ in range(2000))
    vals = sorted(v for _, v in ratios_); worst = max(ratios_, key=lambda kv: kv[1])
    return f'{metric:15} n={len(ratios_):3} geomean {math.exp(statistics.fmean(logs)):.3f} [{boot[50]:.3f}, {boot[1949]:.3f}] p95 {vals[min(len(vals) - 1, int(0.95 * len(vals)))]:.3f} worst {worst[1]:.3f} {worst[0]}'
keys = [k for k in by if by[k][new] and by[k][base]]
same = sum(1 for k in keys if {x['sha256'] for x in by[k][new]} == {x['sha256'] for x in by[k][base]})
failed = sorted(set(exited) - succeeded)
attempted = {t: sum(1 for k in exited if k[0] == t) for t in (new, base)}
print(f'{len(keys)} cases both completed, identical outputs {same}, gated {doc.get("gated", True)}, discarded rounds {doc["discarded_rounds"]}')
for t in (new, base):
    print(f'  {t}: attempted {attempted[t]}, succeeded {sum(1 for k in succeeded if k[0] == t)} (exit 0 and verified)')
for f in failed: print('  failed', *f, 'verifier rejected' if exited[f] == {0} else f'exit {sorted(exited[f])}')
lost = sorted(tuple(k) for t, *k in failed if t == new and (base, *k) in succeeded)
for group in sorted({k[1] for k in keys}) + ['all']:
    if not keys: break
    ks = [k for k in keys if group in ('all', k[1])]
    print(group); [print('  ' + summary(m, ks)) for m in ('wall', 'cpu', 'peak_rss_bytes', 'bytes')]
regressed = bool(keys) and verdict.write(pathlib.Path(sys.argv[1]).with_suffix('.verdict.json'),
                                        [verdict.judge(m, [('/'.join(map(str, k)), r) for k, r in ratios(m, keys)]) for m in ('wall', 'cpu', 'peak_rss_bytes', 'bytes')])
if lost:
    print(f'{new} failed cases {base} completed:', *lost)
sys.exit(1 if lost else 3 if regressed else 0)
