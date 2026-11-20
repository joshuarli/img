#!/usr/bin/env python3
"""Gated direct-quality JPEG encode comparison: run from a directory holding in/*.png,
python3 standalone-compare.py VERIFIER OUT.json ROUNDS LABEL=BINARY...; labels img-fast/img-balanced run
`img convert --quality Q --effort E`, others run `BINARY IN OUT -q Q` (cjpegli). Qualities 70-95; every
retained output is scored by VERIFIER (benchmarks/codecs/verify.rs) against its PNG source."""
import json, os, subprocess, sys, time, hashlib
verifier, out, rounds = sys.argv[1], sys.argv[2], int(sys.argv[3]); tools = dict(a.split('=', 1) for a in sys.argv[4:])
inputs = sorted(os.path.join('in', f) for f in os.listdir('in')); qs = [70, 80, 85, 90, 95]
def cmd(tool, src, q, dest):
    if tool.startswith('img-'): return [tools[tool], 'convert', src, '-o', dest, '--format', 'jpeg', '--quality', str(q), '--effort', tool[4:], '--quiet']
    return [tools[tool], src, dest, '-q', str(q)]
samples = []; r = 0; discarded = 0
while r < rounds:
    if subprocess.run(['perf-gate-ready'], capture_output=True).returncode: discarded += 1; time.sleep(20); continue
    batch = []
    for i, src in enumerate(inputs):
        for q in qs:
            order = list(tools) if (r + i + q) % 2 == 0 else list(tools)[::-1]
            for tool in order:
                dest = f'out/{tool}/{os.path.basename(src)}.{q}.jpg'; os.makedirs(os.path.dirname(dest), exist_ok=True)
                if os.path.exists(dest): os.unlink(dest)
                t = time.perf_counter(); p = subprocess.Popen(cmd(tool, src, q, dest), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                _, st, ru = os.wait4(p.pid, 0); wall = time.perf_counter() - t
                batch.append(dict(round=r, tool=tool, input=src, q=q, exit=os.waitstatus_to_exitcode(st), wall=wall, cpu=ru.ru_utime + ru.ru_stime,
                                  peak_rss_bytes=ru.ru_maxrss, bytes=os.path.getsize(dest) if os.path.exists(dest) and os.waitstatus_to_exitcode(st) == 0 else None))
    if subprocess.run(['perf-gate-ready', '-1'], capture_output=True).returncode: discarded += 1; continue
    samples += batch; r += 1; print('round', r, flush=True)
last = {(s['tool'], s['input'], s['q']): s for s in samples if s['bytes'] is not None}
lines = ''.join(f"{k[1]}\tout/{k[0]}/{os.path.basename(k[1])}.{k[2]}.jpg\t0\n" for k in last)
scores = [json.loads(l) for l in subprocess.run([verifier], input=lines, capture_output=True, text=True, check=True).stdout.splitlines()]
sha = lambda p: hashlib.sha256(open(p, 'rb').read()).hexdigest()
json.dump(dict(tools={k: dict(path=v, sha256=sha(v)) for k, v in tools.items()}, verifier=dict(path=verifier, sha256=sha(verifier)),
               inputs={f: sha(f) for f in inputs}, rounds=rounds, discarded_rounds=discarded, samples=samples,
               scores=[dict(tool=k[0], input=k[1], q=k[2], score=s['score'], error=s['error']) for k, s in zip(last, scores)]), open(out, 'w'), indent=1)
