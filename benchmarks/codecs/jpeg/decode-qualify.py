#!/usr/bin/env python3
"""Paired, gated in-process decode timing and separate peak-RSS runs.

python3 decode-qualify.py FIXTURES OUT.json ROUNDS LABEL=BINARY:DECODER...
Each round visits every fixture and runs the decoders in a rotated order; a round
is discarded and repeated when perf-gate-ready fails before or after it.
"""
import json, math, pathlib, platform, random, re, subprocess, sys, hashlib

fixtures, out, rounds, *specs = sys.argv[1:]
rounds = int(rounds)
decoders = [(label, *target.rsplit(':', 1)) for label, target in (spec.split('=', 1) for spec in specs)]
manifest = json.loads((pathlib.Path(fixtures) / 'manifest.json').read_text())
files = [(entry['class'], pathlib.Path(fixtures) / entry['file']) for entry in manifest]
MIN_MS = 150
samples, discarded = [], 0
rng = random.Random(1)
completed = 0
while completed < rounds:
    if subprocess.run(['perf-gate-ready'], capture_output=True).returncode != 0:
        discarded += 1
        subprocess.run(['sleep', '20'])
        continue
    batch = []
    for index, (cls, path) in enumerate(files):
        order = decoders[:]
        rng.shuffle(order)
        for label, binary, decoder in order:
            result = subprocess.run([binary, decoder, str(MIN_MS), str(path)], check=True, text=True, capture_output=True)
            row = json.loads(result.stdout)
            batch.append({'round': completed, 'class': cls, 'file': path.name, 'decoder': label, 'iterations': row['iterations'], 'mean_ms': row['mean_ms']})
    if subprocess.run(['perf-gate-ready', '-1'], capture_output=True).returncode != 0:
        discarded += 1
        continue
    samples += batch
    completed += 1
    print('round', completed, 'of', rounds, flush=True)

memory = []
for cls, path in files:
    for label, binary, decoder in decoders:
        result = subprocess.run(['/usr/bin/time', '-l' if sys.platform == 'darwin' else '-v', binary, decoder, '0', str(path)], check=True, text=True, capture_output=True)
        if sys.platform == 'darwin':
            rss = int(re.search(r'(\d+)\s+maximum resident set size', result.stderr).group(1))
        else:
            rss = int(re.search(r'Maximum resident set size \(kbytes\): (\d+)', result.stderr).group(1)) * 1024
        memory.append({'class': cls, 'file': path.name, 'decoder': label, 'peak_rss_bytes': rss})

def sha(path): return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()
record = {
    'platform': {'system': platform.system(), 'machine': platform.machine(), 'cpu': subprocess.run(['/usr/sbin/sysctl', '-n', 'machdep.cpu.brand_string'], capture_output=True, text=True).stdout.strip() if sys.platform == 'darwin' else platform.processor()},
    'threads': 1, 'rounds': rounds, 'discarded_rounds': discarded, 'min_ms_per_sample': MIN_MS,
    'decoders': [{'label': l, 'binary': b, 'binary_sha256': sha(b), 'mode': d} for l, b, d in decoders],
    'fixtures': manifest, 'samples': samples, 'memory': memory,
}
pathlib.Path(out).write_text(json.dumps(record, indent=1) + '\n')
print('wrote', out, 'discarded rounds', discarded)
