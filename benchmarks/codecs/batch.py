#!/usr/bin/env python3
"""Gated batch throughput: python3 batch.py OUT.json ROUNDS INDIR MODE LABEL=BINARY... -- JOBS...
MODE is 'lossless' (PNG lossless, best) or 'jpeg90' (JPEG --score 90, balanced). Each run converts every
input in INDIR into a fresh --out-dir; wall, user+sys CPU and peak RSS come from wait4. Throughput is reported as
attempted and as successful images per second; a success is a JSON result whose retained final-round output
BATCH_VERIFIER (a benchmarks/codecs/verify.rs build shared by every tool) accepts. Timing comparisons require
every tool to succeed on the same inputs."""
import json, os, pathlib, subprocess, sys, time, hashlib, shutil, platform
sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import verdict
args = sys.argv[1:]; split = args.index('--')
out, rounds, indir, mode = args[0], int(args[1]), args[2], args[3]
specs = [s.split('=', 1) for s in args[4:split]]; jobs = [int(j) for j in args[split + 1:]]
files = sorted(os.path.join(indir, f) for f in os.listdir(indir))
flags = {'lossless': ['convert', '--format', 'png', '--lossless', '--effort', 'best'],
         'jpeg90': ['optimize', '--format', 'jpeg', '--score', '90', '--effort', 'balanced']}[mode]
scratch = os.path.join(os.path.dirname(os.path.abspath(out)), 'batch-out')
verifier = os.environ.get('BATCH_VERIFIER') or sys.exit('BATCH_VERIFIER must name a verify binary')
requirement = {'lossless': 'lossless', 'jpeg90': '90'}[mode]
samples, discarded, r = [], 0, 0
while r < rounds:
    if subprocess.run(['perf-gate-ready'], capture_output=True).returncode: discarded += 1; time.sleep(20); continue
    batch = []
    for j in jobs:
        order = specs if (r + j) % 2 == 0 else specs[::-1]
        for label, binary in order:
            dest = os.path.join(scratch, label, f'j{j}')
            shutil.rmtree(dest, ignore_errors=True)
            cmd = [binary, flags[0], *files, '--out-dir', dest, *flags[1:], '-j', str(j), '--quiet', '--json']
            t = time.perf_counter(); p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            stdout = p.stdout.read(); _, status, ru = os.wait4(p.pid, 0); wall = time.perf_counter() - t
            doc = json.loads(stdout)
            rss = ru.ru_maxrss if sys.platform == 'darwin' else ru.ru_maxrss * 1024
            batch.append({'round': r, 'tool': label, 'jobs': j, 'wall': wall, 'cpu': ru.ru_utime + ru.ru_stime, 'peak_rss_bytes': rss,
                          'attempted': len(files), 'results': [{'input': x['input'], 'output': x['output'], 'bytes': x.get('outputBytes')} for x in doc['results']],
                          'failures': [{'input': x['input'], 'kind': x['kind']} for x in doc['failures']]})
    if subprocess.run(['perf-gate-ready', '-1'], capture_output=True).returncode: discarded += 1; continue
    samples += batch; r += 1; print('round', r, flush=True)
final = {}
for x in samples: final[(x['tool'], x['jobs'])] = x
pending = [(key, res) for key, x in final.items() for res in x['results']]
lines = ''.join(f"{os.path.abspath(res['input'])}\t{res['output'] or res['input']}\t{requirement}\n" for _, res in pending)
checked = subprocess.run([verifier], input=lines, capture_output=True, text=True, check=True).stdout.splitlines()
verification = [{'tool': key[0], 'jobs': key[1], 'input': res['input'], **json.loads(line)} for (key, res), line in zip(pending, checked, strict=True)]
accepted = {(v['tool'], v['jobs'], v['input']) for v in verification if v['accepted']}
def succeeded(s): return sorted(x['input'] for x in s['results'] if (s['tool'], s['jobs'], x['input']) in accepted)
for s_ in samples: s_['succeeded'] = succeeded(s_)
sha = lambda p: hashlib.sha256(open(p, 'rb').read()).hexdigest()
json.dump({'platform': platform.platform(), 'mode': mode, 'files': [{'path': f, 'sha256': sha(f)} for f in files],
           'tools': [{'label': l, 'binary': b, 'sha256': sha(b)} for l, b in specs], 'verifier': {'path': verifier, 'sha256': sha(verifier)},
           'rounds': rounds, 'discarded_rounds': discarded, 'verification': verification, 'samples': samples}, open(out, 'w'), indent=1)
import statistics
for j in jobs:
    for label, _ in specs:
        rows = [s for s in samples if s['tool'] == label and s['jobs'] == j]
        ok = min(len(s['succeeded']) for s in rows)
        print(f"{mode} -j {j} {label}: attempted {statistics.median(s['attempted'] / s['wall'] for s in rows):.2f} img/s, "
              f"successful {statistics.median(len(s['succeeded']) / s['wall'] for s in rows):.2f} img/s, coverage {ok}/{len(files)}, "
              f"cpu {statistics.median(s['cpu'] for s in rows):.1f} s, peak RSS {max(s['peak_rss_bytes'] for s in rows) / 2**20:.0f} MiB, "
              f"bytes {sum(x['bytes'] or 0 for x in rows[-1]['results'])}")
identities = {tuple(s['succeeded']) for s in samples}
if len(identities) != 1 or not next(iter(identities)):
    sets = {(s['tool'], s['jobs']): set(s['succeeded']) for s in samples}
    common = set.intersection(*sets.values())
    print('equivalent-work check failed: successful inputs differ across tools, job counts or rounds;',
          *(f"\n  {t} -j {j} missing {sorted(set(files) - v)[:5]}" for (t, j), v in sorted(sets.items()) if v != common or not v))
    sys.exit(1)
med = lambda label, j, key: statistics.median(s[key] for s in samples if s['tool'] == label and s['jobs'] == j)
new_label, base_label = specs[0][0], specs[-1][0]
regressed = len(specs) == 2 and verdict.write(pathlib.Path(out).with_suffix('.verdict.json'), [
    verdict.judge('seconds_per_image', [(f'{mode} -j {j}', med(new_label, j, 'wall') / med(base_label, j, 'wall')) for j in jobs]),
    verdict.judge('peak_rss_bytes', [(f'{mode} -j {j}', med(new_label, j, 'peak_rss_bytes') / med(base_label, j, 'peak_rss_bytes')) for j in jobs])])
sys.exit(3 if regressed else 0)
