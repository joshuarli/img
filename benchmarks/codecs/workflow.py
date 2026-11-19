#!/usr/bin/env python3
"""Gated paired single-image workflow: python3 workflow.py OUT.json ROUNDS EFFORT LABEL=BINARY LABEL=BINARY -- INPUT...
WORKFLOW_FORMATS and WORKFLOW_SCORES are comma lists; the score 'lossless' runs --lossless instead of --score.
Each round passes perf-gate-ready before and perf-gate-ready -1 after, or is discarded and repeated. Within a round every
(input, format, score) case runs each binary once in alternating order. Wall, user+sys CPU and peak RSS come from wait4;
output size and SHA-256 are kept so identical searches can be confirmed."""
import hashlib, json, os, platform, random, subprocess, sys, time
args = sys.argv[1:]; split = args.index('--')
out, rounds, effort = args[0], int(args[1]), args[2]
tools = [s.split('=', 1) for s in args[3:split]]; inputs = args[split + 1:]
formats = os.environ.get('WORKFLOW_FORMATS', 'jpeg,png,avif').split(',')
scores = os.environ.get('WORKFLOW_SCORES', '80,90,95').split(',')
scratch = os.path.join(os.path.dirname(os.path.abspath(out)), 'workflow-out'); os.makedirs(scratch, exist_ok=True)
sha = lambda p: hashlib.sha256(open(p, 'rb').read()).hexdigest()
cases = [(i, f, s) for i in inputs for f in formats for s in scores]
gated = os.environ.get('WORKFLOW_GATE', '1') != '0'
gate = lambda *a: gated and subprocess.run(['perf-gate-ready', *a], capture_output=True).returncode != 0
samples, discarded, r = [], 0, 0
while r < rounds:
    if gate(): discarded += 1; time.sleep(20); continue
    batch = []; order = cases[:]; random.Random(r).shuffle(order)
    for k, (source, fmt, score) in enumerate(order):
        for label, binary in (tools if (r + k) % 2 == 0 else tools[::-1]):
            dest = os.path.join(scratch, f'{label}.{fmt}')
            if os.path.exists(dest): os.unlink(dest)
            fidelity = ['--lossless'] if score == 'lossless' else ['--score', score]
            cmd = [binary, 'optimize', source, '-o', dest, '--format', fmt, *fidelity, '--effort', effort, '--quiet']
            t = time.perf_counter(); p = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            err = p.stderr.read(); _, status, ru = os.wait4(p.pid, 0); wall = time.perf_counter() - t
            ok = os.waitstatus_to_exitcode(status) == 0
            batch.append({'round': r, 'tool': label, 'input': os.path.basename(source), 'format': fmt, 'score': score, 'exit': os.waitstatus_to_exitcode(status),
                          'wall': wall, 'cpu': ru.ru_utime + ru.ru_stime, 'peak_rss_bytes': ru.ru_maxrss if sys.platform == 'darwin' else ru.ru_maxrss * 1024,
                          'bytes': os.path.getsize(dest) if ok else None, 'sha256': sha(dest) if ok else None, 'error': None if ok else err.decode(errors='replace')[-300:]})
    if gate('-1'): discarded += 1; continue
    samples += batch; r += 1; print('round', r, flush=True)
json.dump({'gated': gated, 'host': platform.platform(), 'machine': platform.machine(), 'effort': effort, 'rounds': rounds, 'discarded_rounds': discarded,
           'tools': {l: {'path': b, 'sha256': sha(b)} for l, b in tools}, 'inputs': {os.path.basename(i): sha(i) for i in inputs},
           'samples': samples}, open(out, 'w'), indent=1)
