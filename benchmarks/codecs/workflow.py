#!/usr/bin/env python3
"""Gated paired single-image workflow: python3 workflow.py OUT.json ROUNDS EFFORT LABEL=BINARY LABEL=BINARY -- INPUT...
WORKFLOW_FORMATS and WORKFLOW_SCORES are comma lists; the score 'lossless' runs --lossless instead of --score.
Each round passes perf-gate-ready before and perf-gate-ready -1 after, or is discarded and repeated. Within a round every
(input, format, score) case runs each binary once in alternating order. Wall, user+sys CPU and peak RSS come from wait4;
output size and SHA-256 are kept so identical searches can be confirmed. Every case keeps its own output per tool, and after
the rounds WORKFLOW_VERIFIER (a benchmarks/codecs/verify.rs build, the same binary for every tool) checks each retained
output against its source: exact samples for lossless, measured score at or above the target otherwise."""
import hashlib, json, os, platform, random, subprocess, sys, time
args = sys.argv[1:]; split = args.index('--')
out, rounds, effort = args[0], int(args[1]), args[2]
tools = [s.split('=', 1) for s in args[3:split]]; inputs = args[split + 1:]
formats = os.environ.get('WORKFLOW_FORMATS', 'jpeg,png,avif').split(',')
scores = os.environ.get('WORKFLOW_SCORES', '80,90,95').split(',')
scratch = os.path.join(os.path.dirname(os.path.abspath(out)), 'workflow-out')
verifier = os.environ.get('WORKFLOW_VERIFIER') or sys.exit('WORKFLOW_VERIFIER must name a verify binary')
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
            dest = os.path.join(scratch, label, f'{os.path.basename(source)}.{score}.{fmt}')
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            if os.path.exists(dest): os.unlink(dest)
            fidelity = ['--lossless'] if score == 'lossless' else ['--score', score]
            cmd = [binary, 'optimize', source, '-o', dest, '--format', fmt, *fidelity, '--effort', effort, '--quiet']
            t = time.perf_counter(); p = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            err = p.stderr.read(); _, status, ru = os.wait4(p.pid, 0); wall = time.perf_counter() - t
            ok = os.waitstatus_to_exitcode(status) == 0
            batch.append({'round': r, 'tool': label, 'input': os.path.basename(source), 'format': fmt, 'score': score, 'exit': os.waitstatus_to_exitcode(status),
                          'wall': wall, 'cpu': ru.ru_utime + ru.ru_stime, 'peak_rss_bytes': ru.ru_maxrss if sys.platform == 'darwin' else ru.ru_maxrss * 1024,
                          'output': dest, 'bytes': os.path.getsize(dest) if ok else None, 'sha256': sha(dest) if ok else None, 'error': None if ok else err.decode(errors='replace')[-300:]})
    if gate('-1'): discarded += 1; continue
    samples += batch; r += 1; print('round', r, flush=True)
last = {}
for x in samples: last[(x['tool'], x['input'], x['format'], x['score'])] = x
pending = [x for x in last.values() if x['exit'] == 0]
sources = {os.path.basename(i): os.path.abspath(i) for i in inputs}
lines = ''.join(f"{sources[x['input']]}\t{x['output']}\t{x['score']}\n" for x in pending)
checked = subprocess.run([verifier], input=lines, capture_output=True, text=True, check=True).stdout.splitlines()
verification = []
for x, line in zip(pending, checked, strict=True):
    v = json.loads(line)
    hashes = {y['sha256'] for y in samples if (y['tool'], y['input'], y['format'], y['score']) == (x['tool'], x['input'], x['format'], x['score']) and y['exit'] == 0}
    verification.append({'tool': x['tool'], 'input': x['input'], 'format': x['format'], 'score': x['score'], 'sha256': x['sha256'],
                         'exact': v['exact'], 'measured_score': v['score'], 'accepted': v['accepted'], 'error': v['error'],
                         'identical_across_rounds': len(hashes) == 1})
json.dump({'gated': gated, 'host': platform.platform(), 'machine': platform.machine(), 'effort': effort, 'rounds': rounds, 'discarded_rounds': discarded,
           'tools': {l: {'path': b, 'sha256': sha(b)} for l, b in tools}, 'inputs': {os.path.basename(i): sha(i) for i in inputs},
           'verifier': {'path': verifier, 'sha256': sha(verifier)}, 'verification': verification,
           'samples': samples}, open(out, 'w'), indent=1)
rejected = [v for v in verification if not v['accepted']]
print(f'verified {len(verification) - len(rejected)}/{len(verification)} retained outputs', *(f"\n  rejected {v['tool']} {v['input']} {v['format']} {v['score']}: "
      f"exact {v['exact']} score {v['measured_score']} {v['error'] or ''}" for v in rejected))
