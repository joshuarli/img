#!/usr/bin/env python3
"""Gated batch throughput: python3 batch.py OUT.json ROUNDS INDIR MODE LABEL=BINARY... -- JOBS...
MODE is 'lossless' (PNG lossless, best), 'jpeg90' (JPEG --score 90, balanced), 'avif90' (AVIF --score 90, balanced)
or 'keep90' (--score 90, balanced, each output in its input's format). Each run converts every input in INDIR into a
fresh --out-dir; wall, user+sys CPU and peak RSS come from wait4. Rounds are admitted by evidence.run_rounds (load gate
before and after, no overlapping build) with every attempt's disposition recorded. Every output of every timed run is
hashed after the run exits and retained by hash; BATCH_VERIFIER (a benchmarks/codecs/verify.rs build shared by every
tool) checks each distinct retained output once. An input succeeds in a run only when the run reported it and that
run's own output verified. Throughput is reported as attempted and as successful images per second. Timing comparisons
require every tool, job count and round to succeed on the same inputs; otherwise the run exits 1, and 4 when no input
succeeded everywhere (insufficient evidence)."""
import json, os, pathlib, platform, shutil, statistics, subprocess, sys, time
sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import evidence, verdict
args = sys.argv[1:]; split = args.index('--')
out, rounds, indir, mode = args[0], int(args[1]), args[2], args[3]
specs = [s.split('=', 1) for s in args[4:split]]; jobs = [int(j) for j in args[split + 1:]]
files = sorted(os.path.abspath(os.path.join(indir, f)) for f in os.listdir(indir))
scored = ['--score', '90', '--effort', 'balanced']
flags = {'lossless': ['convert', '--format', 'png', '--lossless', '--effort', 'best'], 'jpeg90': ['optimize', '--format', 'jpeg', *scored],
         'avif90': ['optimize', '--format', 'avif', *scored], 'keep90': ['optimize', *scored]}[mode]
requirement = 'lossless' if mode == 'lossless' else '90'
scratch = os.path.join(os.path.dirname(os.path.abspath(out)), 'batch-out')
verifier = os.environ.get('BATCH_VERIFIER') or sys.exit('BATCH_VERIFIER must name a verify binary')
store = evidence.Store(os.path.join(scratch, 'retained'))


def run_round(r):
    batch = []
    for j in jobs:
        for label, binary in (specs if (r + j) % 2 == 0 else specs[::-1]):
            dest = os.path.join(scratch, label, f'j{j}')
            shutil.rmtree(dest, ignore_errors=True)
            cmd = [binary, flags[0], *files, '--out-dir', dest, *flags[1:], '-j', str(j), '--quiet', '--json']
            t = time.perf_counter(); p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            stdout = p.stdout.read(); _, status, ru = os.wait4(p.pid, 0); wall = time.perf_counter() - t
            doc = json.loads(stdout)
            results = []
            for x in doc['results']:
                source = os.path.abspath(x['input'])
                digest, retained = store.retain(x['output']) if x['output'] else (evidence.sha256(source), source)
                results.append({'input': source, 'output': retained, 'sha256': digest, 'bytes': x.get('outputBytes'), 'format': x['format']})
            batch.append({'round': r, 'tool': label, 'jobs': j, 'exit': os.waitstatus_to_exitcode(status), 'wall': wall,
                          'cpu': ru.ru_utime + ru.ru_stime, 'peak_rss_bytes': ru.ru_maxrss if sys.platform == 'darwin' else ru.ru_maxrss * 1024,
                          'attempted': len(files), 'results': results, 'failures': [{'input': x['input'], 'kind': x['kind']} for x in doc['failures']]})
    return batch


kept, dispositions = evidence.run_rounds(rounds, run_round)
samples = [x for batch in kept for x in batch]
checked = evidence.verify(verifier, [(x['input'], x['output'], requirement) for s in samples for x in s['results']])
for s in samples:
    for x in s['results']:
        x['verified'] = checked[(x['input'], x['output'], requirement)]['accepted']
    s['succeeded'] = sorted(x['input'] for x in s['results'] if x['verified'])
verification = [{'source': c[0], 'output': c[1], 'requirement': c[2], **v} for c, v in sorted(checked.items())]
json.dump({'platform': platform.platform(), 'mode': mode, 'files': [{'path': f, 'sha256': evidence.sha256(f)} for f in files],
           'tools': [{'label': l, 'binary': b, 'sha256': evidence.sha256(b)} for l, b in specs], 'verifier': {'path': verifier, 'sha256': evidence.sha256(verifier)},
           'rounds': rounds, 'discarded_rounds': evidence.discarded(dispositions), 'dispositions': dispositions,
           'verification': verification, 'samples': samples}, open(out, 'w'), indent=1)
for d in dispositions:
    if d['disposition'] != 'kept': print(f'attempt {d["attempt"]}: {d["disposition"]}')
timed = sum(len(s['results']) for s in samples)
print(f'verified {sum(v["accepted"] for v in verification)}/{len(verification)} distinct outputs covering '
      f'{sum(x["verified"] for s in samples for x in s["results"])}/{timed} timed outputs')
for j in jobs:
    for label, _ in specs:
        rows = [s for s in samples if s['tool'] == label and s['jobs'] == j]
        ok = min(len(s['succeeded']) for s in rows)
        print(f"{mode} -j {j} {label}: attempted {statistics.median(s['attempted'] / s['wall'] for s in rows):.2f} img/s, "
              f"successful {statistics.median(len(s['succeeded']) / s['wall'] for s in rows):.2f} img/s, coverage {ok}/{len(files)}, "
              f"cpu {statistics.median(s['cpu'] for s in rows):.1f} s, peak RSS {max(s['peak_rss_bytes'] for s in rows) / 2**20:.0f} MiB, "
              f"bytes {sum(x['bytes'] or 0 for x in rows[-1]['results'])}")
sets = {(s['tool'], s['jobs'], s['round']): set(s['succeeded']) for s in samples}
common, union = set.intersection(*sets.values()), set.union(*sets.values())
if common != union:
    print('equivalent-work check failed: successful inputs differ across tools, job counts or rounds',
          *(f"\n  {t} -j {j} round {r} lacks {' '.join(sorted(union - v)[:5])}" for (t, j, r), v in sorted(sets.items()) if v != union))
if not common:
    print('INSUFFICIENT EVIDENCE: no input succeeded in every tool, job count and round; no comparison is possible')
    sys.exit(4)
if common != union:
    sys.exit(1)
med = lambda label, j, key: statistics.median(s[key] for s in samples if s['tool'] == label and s['jobs'] == j)
new_label, base_label = specs[0][0], specs[-1][0]
regressed = len(specs) == 2 and verdict.write(pathlib.Path(out).with_suffix('.verdict.json'), [
    verdict.judge('seconds_per_image', [(f'{mode} -j {j}', med(new_label, j, 'wall') / med(base_label, j, 'wall')) for j in jobs]),
    verdict.judge('peak_rss_bytes', [(f'{mode} -j {j}', med(new_label, j, 'peak_rss_bytes') / med(base_label, j, 'peak_rss_bytes')) for j in jobs])])
sys.exit(3 if regressed else 0)
