#!/usr/bin/env python3
import json, os, resource, subprocess, sys, time
from pathlib import Path

root = Path(__file__).resolve().parents[3]
scratch = Path(os.environ.get('JPEG_SCRATCH', root / '.work/tmp/jpeg-candidates'))
binaries = dict((label, Path(path)) for label, path in (spec.split('=', 1) for spec in os.environ['SCORED_TOOLS'].split(','))) if os.environ.get('SCORED_TOOLS') else {name: Path(os.environ[f'SCORED_{name.upper()}']) for name in ('img', 'zen') if os.environ.get(f'SCORED_{name.upper()}')}
scores = [int(s) for s in os.environ.get('SCORED_TARGETS', '80,90,95').split(',')]
efforts = os.environ.get('SCORED_EFFORTS', 'fast,balanced,best').split(',')
repeats = int(os.environ.get('SCORED_REPEATS', '3'))

def timed(command):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.perf_counter()
    p = subprocess.run([str(x) for x in command], capture_output=True, env={**os.environ, 'RAYON_NUM_THREADS': '1'})
    wall = time.perf_counter() - start
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    return p, wall * 1000, (after.ru_utime - before.ru_utime + after.ru_stime - before.ru_stime) * 1000

def main():
    out = Path(sys.argv[1])
    manifest = json.loads((scratch / 'inputs/manifest.json').read_text())
    records = json.loads(out.read_text()) if out.exists() else []
    done = {(r['input'], r['encoder'], r['effort'], r['target']) for r in records}
    work = scratch / 'scored-output'
    work.mkdir(exist_ok=True)
    for item in manifest:
        source = scratch / 'inputs' / item['input']
        for target in scores:
            for effort in efforts:
                for encoder in binaries:
                    key = (item['source'], encoder, effort, target)
                    if key in done:
                        continue
                    record = {'input': item['source'], 'group': item['group'], 'encoder': encoder, 'effort': effort, 'target': target, 'wall_ms': [], 'cpu_ms': []}
                    for _ in range(repeats):
                        dest = work / f'{encoder}.jpg'
                        dest.unlink(missing_ok=True)
                        p, wall, cpu = timed([binaries[encoder], 'optimize', source, '-o', dest, '--format', 'jpeg', '--score', target, '--effort', effort, '--quiet'])
                        record['exit'] = p.returncode
                        if p.returncode:
                            record['error'] = (p.stdout + p.stderr).decode(errors='replace')[-500:]
                            break
                        record['wall_ms'].append(wall)
                        record['cpu_ms'].append(cpu)
                    if record['exit'] == 0:
                        record['bytes'] = dest.stat().st_size
                        c = subprocess.run([str(binaries.get('img', next(iter(binaries.values())))), 'compare', str(source), str(dest), '--json'], capture_output=True)
                        record['score'] = json.loads(c.stdout)['results'][0]['score'] if c.returncode == 0 else None
                    records.append(record)
                    out.write_text(json.dumps(records, separators=(',', ':')))
                    print(item['input'], target, effort, encoder, record.get('bytes'), record.get('score'), flush=True)

if __name__ == '__main__':
    main()
