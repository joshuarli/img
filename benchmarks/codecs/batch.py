#!/usr/bin/env python3
"""Gated batch throughput: python3 batch.py OUT.json ROUNDS INDIR MODE LABEL=BINARY... -- JOBS...
MODE is 'lossless' (PNG lossless, best) or 'jpeg90' (JPEG --score 90, balanced). Each run converts every
input in INDIR into a fresh --out-dir; wall, user+sys CPU and peak RSS come from wait4."""
import json, os, subprocess, sys, time, hashlib, shutil, platform
args = sys.argv[1:]; split = args.index('--')
out, rounds, indir, mode = args[0], int(args[1]), args[2], args[3]
specs = [s.split('=', 1) for s in args[4:split]]; jobs = [int(j) for j in args[split + 1:]]
files = sorted(os.path.join(indir, f) for f in os.listdir(indir))
pixels = 0
flags = {'lossless': ['convert', '--format', 'png', '--lossless', '--effort', 'best'],
         'jpeg90': ['optimize', '--format', 'jpeg', '--score', '90', '--effort', 'balanced']}[mode]
scratch = os.path.join(os.path.dirname(out) or '.', 'batch-out')
samples, discarded, r = [], 0, 0
while r < rounds:
    if subprocess.run(['perf-gate-ready'], capture_output=True).returncode: discarded += 1; time.sleep(20); continue
    batch = []
    for j in jobs:
        order = specs if (r + j) % 2 == 0 else specs[::-1]
        for label, binary in order:
            shutil.rmtree(scratch, ignore_errors=True)
            cmd = [binary, flags[0], *files, '--out-dir', scratch, *flags[1:], '-j', str(j), '--quiet', '--json']
            t = time.perf_counter(); p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            stdout = p.stdout.read(); _, status, ru = os.wait4(p.pid, 0); wall = time.perf_counter() - t
            doc = json.loads(stdout)
            rss = ru.ru_maxrss if sys.platform == 'darwin' else ru.ru_maxrss * 1024
            batch.append({'round': r, 'tool': label, 'jobs': j, 'wall': wall, 'cpu': ru.ru_utime + ru.ru_stime, 'peak_rss_bytes': rss,
                          'completed': len(doc['results']), 'failed': len(doc['failures']), 'output_bytes': sum(x.get('outputBytes') or 0 for x in doc['results'])})
    if subprocess.run(['perf-gate-ready', '-1'], capture_output=True).returncode: discarded += 1; continue
    samples += batch; r += 1; print('round', r, flush=True)
json.dump({'platform': platform.platform(), 'mode': mode, 'files': [{'path': f, 'sha256': hashlib.sha256(open(f, 'rb').read()).hexdigest()} for f in files],
           'tools': [{'label': l, 'binary': b, 'sha256': hashlib.sha256(open(b, 'rb').read()).hexdigest()} for l, b in specs],
           'discarded_rounds': discarded, 'samples': samples}, open(out, 'w'), indent=1)
