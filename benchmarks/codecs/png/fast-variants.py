#!/usr/bin/env python3
"""Gated in-process PNG Fast-effort variants: python3 fast-variants.py PNG_ENCODE_LOOP OUT.json ROUNDS FILES...
The loop must be built (loops-build.sh) from branch exp/jpeg-fast-scans, whose src/still/png.rs reads IMG_PNG_FAST
(base, minsum, l4, l3, minsum-l4). Each variant runs single-threaded (CPU proxy) and with four threads (wall)."""
import json, os, subprocess, sys, time, hashlib
loop, out, rounds = sys.argv[1], sys.argv[2], int(sys.argv[3]); files = sys.argv[4:]
variants = ['base', 'minsum', 'l4', 'l3', 'minsum-l4']
samples, r, discarded = [], 0, 0
while r < rounds:
    if subprocess.run(['perf-gate-ready'], capture_output=True).returncode: discarded += 1; time.sleep(20); continue
    batch = []
    for i, f in enumerate(files):
        order = variants if (r + i) % 2 == 0 else variants[::-1]
        for v in order:
            for threads in ('1', '4'):
                o = subprocess.run([loop, f, 'fast', threads], capture_output=True, text=True, env={**os.environ, 'IMG_PNG_FAST': v})
                if o.returncode: batch.append(dict(round=r, file=f, variant=v, threads=threads, error=o.stderr[-300:])); continue
                p = o.stdout.split(); assert p[-1] == 'eq=true', o.stdout
                batch.append(dict(round=r, file=os.path.basename(f), variant=v, threads=threads, bytes=int(p[0]), encode_ms=float(p[5])))
    if subprocess.run(['perf-gate-ready', '-1'], capture_output=True).returncode: discarded += 1; continue
    samples += batch; r += 1; print('round', r, flush=True)
sha = lambda p: hashlib.sha256(open(p, 'rb').read()).hexdigest()
json.dump(dict(loop=dict(path=loop, sha256=sha(loop)), rounds=rounds, discarded_rounds=discarded, files={os.path.basename(f): sha(f) for f in files},
               note='threads=1 encode_ms approximates CPU; threads=4 is wall with chunk parallelism; exactness asserted by the loop (eq=true)', samples=samples), open(out, 'w'), indent=1)
