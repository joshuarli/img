#!/usr/bin/env python3
import hashlib, json, pathlib, platform, random, re, subprocess, sys, time

fixtures, output, rounds, img, dav, zen, *baseline = sys.argv[1:]
rounds = int(rounds)
if rounds < 9: raise ValueError('at least nine rounds required')
fixtures = pathlib.Path(fixtures)
manifest = json.loads((fixtures / 'manifest.json').read_text())
engines = [('rav1d', img, 'raw', 'obu'), ('dav1d', dav, 'raw', 'obu'), ('img', img, 'img', 'avif'), ('dav1d-rgb', dav, 'rgb', 'avif'), ('zen', zen, 'zen', 'avif')]
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
for f in manifest:
    for ext in ('avif', 'obu'):
        f[ext + '_sha256'] = sha(fixtures / (f['name'] + '.' + ext))
if baseline: engines.append(('img-baseline', baseline[0], 'img', 'avif'))
rng = random.Random(1)
samples, memory, discarded, completed, consecutive = [], [], 0, 0, 0
while completed < rounds:
    if subprocess.run(['perf-gate-ready']).returncode:
        discarded += 1; consecutive += 1; print('discarded before', discarded, flush=True)
        if consecutive >= 5: raise RuntimeError('persistent gate failure')
        time.sleep(20); continue
    batch = []
    for f in manifest:
        order = engines[:]; rng.shuffle(order)
        for label, binary, mode, ext in order:
            row = json.loads(subprocess.run([binary, mode, '150', str(fixtures / (f['name'] + '.' + ext))], check=True, text=True, capture_output=True).stdout)
            batch.append(dict(round=completed, file=f['name'], decoder=label, **row))
    if subprocess.run(['perf-gate-ready', '-1']).returncode:
        discarded += 1; consecutive += 1; print('discarded after', discarded, flush=True)
        if consecutive >= 5: raise RuntimeError('persistent gate failure')
        continue
    samples.extend(batch); completed += 1; consecutive = 0; print('round', completed, 'of', rounds, flush=True)
for f in manifest:
    for label, binary, mode, ext in engines:
        r = subprocess.run(['/usr/bin/time', '-l', binary, mode, '0', str(fixtures / (f['name'] + '.' + ext))], check=True, text=True, capture_output=True)
        memory.append(dict(file=f['name'], decoder=label, peak_rss_bytes=int(re.search(r'(\d+)\s+maximum resident set size', r.stderr).group(1))))
record = dict(platform=platform.platform(), rounds=rounds, discarded_rounds=discarded, min_ms=150, threads=1,
              dav1d_library_sha256=sha(pathlib.Path('/opt/homebrew/Cellar/dav1d/1.5.4/lib/libdav1d.7.dylib')), decoders=[dict(label=l, binary=b, binary_sha256=sha(pathlib.Path(b)), mode=m) for l,b,m,_ in engines], fixtures=manifest, samples=samples, memory=memory)
pathlib.Path(output).write_text(json.dumps(record, indent=1)+'\n')
print('wrote', output, 'completed', completed, 'discarded', discarded, flush=True)
