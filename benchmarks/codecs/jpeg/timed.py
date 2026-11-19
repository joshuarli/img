#!/usr/bin/env python3
import json, os, subprocess, sys, time
from pathlib import Path
from sweep import root, scratch, configs, command, turbo
from matching import bracket

env = {**os.environ, 'RAYON_NUM_THREADS': '1'}
items = json.loads((scratch / 'inputs/manifest.json').read_text())
integrated = os.environ.get('JPEG_INTEGRATION') == '1'
quality = json.loads((root / ('benchmarks/results/codecs/jpeg/integration.json' if integrated else 'benchmarks/results/codecs/jpeg/results.json')).read_text())['quality']
series = {}
for record in quality:
    if record.get('compare_exit') == 0 and record.get('turbo_exit') == 0:
        series.setdefault((record['input'],record['config']), []).append(record)
fixtures = scratch / 'qualification-fixtures'
fixtures.mkdir(exist_ok=True)
out = Path(sys.argv[1])
records = json.loads(out.read_text()) if out.exists() else []
done = {r['input'] for r in records}

def measure(args, encoder, output=None):
    start = time.perf_counter()
    p = subprocess.Popen([str(x) for x in args], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    _, status, usage = os.wait4(p.pid, 0)
    elapsed = time.perf_counter() - start
    stdout, stderr = p.stdout.read(), p.stderr.read()
    code = os.waitstatus_to_exitcode(status)
    if code == -9: raise SystemExit('process killed by SIGKILL: ' + str(args))
    if encoder and code == 0:
        data = json.loads(stdout)
        data['rss_bytes'] = usage.ru_maxrss
    else:
        data = {'wall_ms': elapsed * 1000, 'cpu_ms': usage.ru_utime * 1000, 'rss_bytes': usage.ru_maxrss}
    data['exit'] = code
    if code: data['error'] = stderr.decode(errors='replace')[-500:]
    if output:
        data['output_bytes'] = output.stat().st_size if output.exists() else None
        output.unlink(missing_ok=True)
    return data

encode_configs = ['img-jpegli', 'jpegli-444', 'zen-best'] if integrated else [x for x in configs if x not in ('img-jpegli', 'jpegli-xyb')]
for index, item in enumerate(items):
    if item['source'] in done: continue
    fixture = fixtures / f'{index}.jpg'
    if not fixture.exists():
        source = scratch / 'inputs' / item['input']
        p = subprocess.run([str(x) for x in command('img-fast', source, fixture, 90)], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        if p.returncode: raise SystemExit(f'fixture failed for {source}: {p.stderr.decode()}')
    gate = subprocess.run(['perf-gate-ready']).returncode
    if gate: sys.exit(gate)
    batch = []
    for iteration in range(5):
        order = encode_configs if iteration % 2 == 0 else list(reversed(encode_configs))
        pnm = (scratch / 'inputs' / item['input']).with_suffix('.pnm')
        for config in order:
            points = bracket(series.get((item['source'],config), []), 90)
            if points is None:
                batch.append({'role':'encode', 'input':item['source'], 'group':item['group'], 'config':config, 'iteration':iteration, 'exit':None, 'error':'score 90 not bracketed'})
                continue
            exe = scratch / ('encode-integrated' if config == 'img-jpegli' else 'encode-jpegli' if config.startswith('jpegli-') else 'encode-moz' if config.startswith('moz-') else 'decode-target/release/jpeg-candidate-encode')
            for point in points:
                result = measure([exe, config, pnm, str(point['quality'])], True)
                batch.append({'role':'encode', 'input':item['source'], 'group':item['group'], 'config':config, 'iteration':iteration, 'quality':point['quality'], 'score':point['score'], **result})
        if not integrated:
            for config in (['turbo', 'jpegli', 'zune', 'zen'] if iteration % 2 == 0 else ['zen', 'zune', 'jpegli', 'turbo']):
                dest = scratch / f'qualification-decode-{iteration}-{index}-{config}.pnm'
                if config == 'turbo': args = [turbo, '-outfile', dest, fixture]
                elif config == 'jpegli': args = [scratch / 'jxl-build/tools/djpegli', fixture, dest]
                else: args = [scratch / 'decode-target/release/jpeg-candidate-decode', config, fixture]
                result = measure(args, False, dest if config in ('turbo','jpegli') else None)
                batch.append({'role':'decode', 'input':item['source'], 'group':item['group'], 'config':config, 'iteration':iteration, **result})
    gate = subprocess.run(['perf-gate-ready', '-1']).returncode
    if gate:
        print(f'discarded: post-check failed for {item["source"]}', file=sys.stderr)
        sys.exit(gate)
    records.extend(batch)
    out.write_text(json.dumps(records, separators=(',', ':')))
    print(f'completed {index+1}/{len(items)} {item["source"]}', flush=True)
