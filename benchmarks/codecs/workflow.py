#!/usr/bin/env python3
"""Gated paired single-image workflow: python3 workflow.py OUT.json ROUNDS EFFORT LABEL=BINARY LABEL=BINARY -- INPUT...
WORKFLOW_FORMATS and WORKFLOW_SCORES are comma lists; the score 'lossless' runs --lossless instead of --score.
Rounds are admitted by evidence.run_rounds (load gate before and after, no overlapping build); every attempt's
disposition is recorded. Within a round every (input, format, score) case runs each binary once in alternating order.
Wall, user+sys CPU and peak RSS come from wait4. Each output is hashed after its process exits and retained by hash;
after the rounds WORKFLOW_VERIFIER (a benchmarks/codecs/verify.rs build, the same binary for every tool) checks every
distinct retained output once against its source: exact samples for lossless, measured score at or above the target
otherwise. Every sample records the verdict of its own output's hash."""
import json, os, pathlib, platform, random, subprocess, sys, time
sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import evidence
args = sys.argv[1:]; split = args.index('--')
out, rounds, effort = args[0], int(args[1]), args[2]
tools = [s.split('=', 1) for s in args[3:split]]; inputs = [os.path.abspath(i) for i in args[split + 1:]]
formats = os.environ.get('WORKFLOW_FORMATS', 'jpeg,png,avif').split(',')
scores = os.environ.get('WORKFLOW_SCORES', '80,90,95').split(',')
scratch = os.path.join(os.path.dirname(os.path.abspath(out)), 'workflow-out')
verifier = os.environ.get('WORKFLOW_VERIFIER') or sys.exit('WORKFLOW_VERIFIER must name a verify binary')
store = evidence.Store(os.path.join(scratch, 'retained'))
cases = [(i, f, s) for i in inputs for f in formats for s in scores]


def run_round(r):
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
            code = os.waitstatus_to_exitcode(status)
            ok = code == 0 and os.path.exists(dest)
            digest, retained = store.retain(dest) if ok else (None, None)
            batch.append({'round': r, 'tool': label, 'input': os.path.basename(source), 'format': fmt, 'score': score, 'exit': code,
                          'wall': wall, 'cpu': ru.ru_utime + ru.ru_stime, 'peak_rss_bytes': ru.ru_maxrss if sys.platform == 'darwin' else ru.ru_maxrss * 1024,
                          'output': retained, 'bytes': os.path.getsize(retained) if ok else None, 'sha256': digest,
                          'error': None if ok else (err.decode(errors='replace')[-300:] or f'exit {code} without an output')})
    return batch


kept, dispositions = evidence.run_rounds(rounds, run_round)
samples = [x for batch in kept for x in batch]
sources = {os.path.basename(i): i for i in inputs}
claim = lambda x: (sources[x['input']], x['output'], 'lossless' if x['score'] == 'lossless' else x['score'])
checked = evidence.verify(verifier, [claim(x) for x in samples if x['sha256']])
for x in samples:
    v = checked.get(claim(x)) if x['sha256'] else None
    x['verified'] = bool(v and v['accepted'])
    x['measured_score'], x['exact'] = (v['score'], v['exact']) if v else (None, None)
    x['verify_error'] = v['error'] if v else None
verification = [{'source': c[0], 'output': c[1], 'requirement': c[2], **v} for c, v in sorted(checked.items())]
json.dump({'gated': evidence.gated(), 'host': platform.platform(), 'machine': platform.machine(), 'effort': effort, 'rounds': rounds,
           'discarded_rounds': evidence.discarded(dispositions), 'dispositions': dispositions,
           'tools': {l: {'path': b, 'sha256': evidence.sha256(b)} for l, b in tools}, 'inputs': {os.path.basename(i): evidence.sha256(i) for i in inputs},
           'verifier': {'path': verifier, 'sha256': evidence.sha256(verifier)}, 'verification': verification,
           'samples': samples}, open(out, 'w'), indent=1)
rejected = [v for v in verification if not v['accepted']]
unverified = sum(1 for x in samples if not x['verified'])
print(f'verified {len(verification) - len(rejected)}/{len(verification)} distinct outputs covering {len(samples) - unverified}/{len(samples)} timed samples',
      *(f"\n  rejected {v['output']} for {os.path.basename(v['source'])} {v['requirement']}: exact {v['exact']} score {v['score']} {v['error'] or ''}" for v in rejected))
