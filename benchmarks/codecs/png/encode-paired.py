#!/usr/bin/env python3
"""Paired in-process PNG lossless encode: python3 encode-paired.py OUT.json ROUNDS LABEL=BINARY... -- FILES...
Each binary is a pngprof-style harness printing 'BYTES bytes decode D encode E verify-decode V compare C eq=true'.
Rounds alternate binary order; a round is retried when perf-gate-ready fails around it."""
import json, subprocess, sys, hashlib, platform
args = sys.argv[1:]; split = args.index('--')
out, rounds, specs, files = args[0], int(args[1]), [s.split('=', 1) for s in args[2:split]], args[split + 1:]
samples, discarded, r = [], 0, 0
while r < rounds:
    if subprocess.run(['perf-gate-ready'], capture_output=True).returncode: discarded += 1; subprocess.run(['sleep', '20']); continue
    batch = []
    for i, f in enumerate(files):
        for effort in ('fast', 'balanced', 'best'):
            order = specs if (r + i) % 2 == 0 else specs[::-1]
            for label, binary in order:
                o = subprocess.run([binary, f, effort, '1'], capture_output=True, text=True)
                if o.returncode: batch.append({'round': r, 'file': f, 'effort': effort, 'tool': label, 'error': o.stderr[-200:]}); continue
                p = o.stdout.split()
                assert p[-1] == 'eq=true', o.stdout
                batch.append({'round': r, 'file': f, 'effort': effort, 'tool': label, 'bytes': int(p[0]), 'encode_ms': float(p[5])})
    if subprocess.run(['perf-gate-ready', '-1'], capture_output=True).returncode: discarded += 1; continue
    samples += batch; r += 1; print('round', r, flush=True)
json.dump({'platform': platform.platform(), 'rounds': rounds, 'discarded_rounds': discarded,
           'tools': [{'label': l, 'binary': b, 'sha256': hashlib.sha256(open(b, 'rb').read()).hexdigest()} for l, b in specs],
           'files': [{'path': f, 'sha256': hashlib.sha256(open(f, 'rb').read()).hexdigest()} for f in files], 'samples': samples}, open(out, 'w'), indent=1)
