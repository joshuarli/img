#!/usr/bin/env python3
import json, os, subprocess, sys, time
from pathlib import Path
from sweep import root, scratch, img, jpegli, turbo, configs, paths, group, command

env = {**os.environ, 'RAYON_NUM_THREADS': '1'}
fixtures = scratch / 'fixtures'
fixtures.mkdir(exist_ok=True)
for i, source in enumerate(paths):
    dest = fixtures / f'{i}.jpg'
    if not dest.exists():
        subprocess.run([str(x) for x in command('img-fast', source, dest, 90)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

if subprocess.run(['perf-gate-ready']).returncode:
    sys.exit(75)

records = []
def measure(args, output=None):
    start = time.perf_counter()
    p = subprocess.Popen([str(x) for x in args], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    _, status, usage = os.wait4(p.pid, 0)
    elapsed = time.perf_counter() - start
    code = os.waitstatus_to_exitcode(status)
    if code == -9:
        raise SystemExit('process killed by SIGKILL: ' + str(args))
    result = {'exit': code, 'wall_ms': elapsed * 1000, 'cpu_ms': (usage.ru_utime + usage.ru_stime) * 1000, 'rss_bytes': usage.ru_maxrss}
    if output:
        result['output_bytes'] = output.stat().st_size if output.exists() else None
        output.unlink(missing_ok=True)
    return result

for iteration in range(5):
    order = configs if iteration % 2 == 0 else list(reversed(configs))
    for index, source in enumerate(paths):
        for config in order:
            dest = scratch / f'time-{iteration}-{index}-{config}.jpg'
            dest.unlink(missing_ok=True)
            result = measure(command(config, source, dest, 90), dest)
            records.append({'role': 'encode', 'input': str(source.relative_to(root)), 'group': group(source), 'config': config, 'iteration': iteration, **result})
        fixture = fixtures / f'{index}.jpg'
        for config in (['turbo', 'jpegli', 'zune', 'zen'] if iteration % 2 == 0 else ['zen', 'zune', 'jpegli', 'turbo']):
            if not fixture.exists():
                records.append({'role': 'decode', 'input': str(source.relative_to(root)), 'group': group(source), 'config': config, 'iteration': iteration, 'exit': None, 'error': 'fixture encoder refused'})
                continue
            dest = scratch / f'decode-{iteration}-{index}-{config}.pnm'
            if config == 'turbo':
                args = [turbo, '-outfile', dest, fixture]
            elif config == 'jpegli':
                args = [scratch / 'jxl-build/tools/djpegli', fixture, dest]
            else:
                args = [scratch / 'decode-target/release/jpeg-candidate-decode', config, fixture]
            result = measure(args, dest if config in ('turbo', 'jpegli') else None)
            records.append({'role': 'decode', 'input': str(source.relative_to(root)), 'group': group(source), 'config': config, 'iteration': iteration, **result})
    print(f'completed round {iteration + 1}', flush=True)

if subprocess.run(['perf-gate-ready', '-1']).returncode:
    print('discarded: post-check failed', file=sys.stderr)
    sys.exit(75)
Path(sys.argv[1]).write_text(json.dumps(records, separators=(',', ':')))
