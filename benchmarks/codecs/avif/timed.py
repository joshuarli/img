#!/usr/bin/env python3
import json, os, struct, subprocess, sys, time
from pathlib import Path
from sweep import root, scratch, img, avifcodec, avifcodec_svt, encode_bin, decode_bin, configs
from matching import bracket

env = {**os.environ, 'RAYON_NUM_THREADS': '1'}
fixtures = scratch / 'qualification-fixtures'
fixtures.mkdir(parents=True, exist_ok=True)
encode_configs = os.environ.get('AVIF_TIMING_CONFIGS', ','.join(configs + ['aom-s6-t4'])).split(',')
series_config = {'aom-s6-t4': 'aom-s6-420'}
decode_configs = os.environ.get('AVIF_TIMING_DECODE', 'dav1d,rav1d,zen').split(',') if os.environ.get('AVIF_TIMING_DECODE', 'dav1d,rav1d,zen') else []

def gate(direction=None):
    deadline = time.monotonic() + 5400
    while True:
        code = subprocess.run(['perf-gate-ready'] + ([direction] if direction else []), stdout=subprocess.DEVNULL).returncode
        if code == 0:
            return True
        if time.monotonic() > deadline:
            print('perf-gate-ready unavailable for 90 minutes', file=sys.stderr)
            return False
        time.sleep(20)

def measure(args, output=None):
    p = subprocess.Popen([str(x) for x in args], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    _, status, usage = os.wait4(p.pid, 0)
    stdout, stderr = p.stdout.read(), p.stderr.read()
    code = os.waitstatus_to_exitcode(status)
    if code == -9:
        raise SystemExit('process killed by SIGKILL: ' + str(args))
    if code == 0:
        data = json.loads(stdout)
    else:
        data = {'wall_ms': 0.0, 'cpu_ms': 0.0, 'error': stderr.decode(errors='replace')[-500:]}
    data['rss_bytes'] = usage.ru_maxrss
    data['exit'] = code
    if output:
        data['output_bytes'] = output.stat().st_size if output.exists() else None
        output.unlink(missing_ok=True)
    return data

def boxes(data, start, end):
    while start + 8 <= end:
        size = struct.unpack('>I', data[start:start+4])[0]
        kind = data[start+4:start+8]
        if size == 0:
            size = end - start
        elif size == 1:
            size = struct.unpack('>Q', data[start+8:start+16])[0]
        yield start, size, kind
        start += size

def extract_primary(data):
    meta = mdat = None
    for start, size, kind in boxes(data, 0, len(data)):
        if kind == b'meta': meta = (start, size)
        if kind == b'mdat': mdat = (start, size)
    if meta is None or mdat is None:
        raise ValueError('missing meta or mdat')
    start, size = meta
    pitm = iloc = None
    for at, length, kind in boxes(data, start + 12, start + size):
        if kind == b'pitm': pitm = (at, length)
        if kind == b'iloc': iloc = (at, length)
    if pitm is None or iloc is None:
        raise ValueError('missing pitm or iloc')
    at, length = pitm
    payload = data[at+8:at+length]
    primary = struct.unpack('>H', payload[4:6])[0] if payload[0] == 0 else struct.unpack('>I', payload[4:8])[0]
    at, length = iloc
    payload = data[at+8:at+length]
    version = payload[0]
    offset_size, length_size = payload[4] >> 4, payload[4] & 15
    base_size = payload[5] >> 4
    index_size = payload[5] & 15 if version in (1, 2) else 0
    at = 6
    count = struct.unpack('>H', payload[at:at+2])[0] if version < 2 else struct.unpack('>I', payload[at:at+4])[0]
    at += 2 if version < 2 else 4
    def read(n):
        nonlocal at
        value = int.from_bytes(payload[at:at+n], 'big'); at += n
        return value
    for _ in range(count):
        item = read(2) if version < 2 else read(4)
        if version in (1, 2): at += 2
        at += 2
        base = read(base_size)
        extents = []
        for _ in range(read(2)):
            if version in (1, 2) and index_size:
                at += index_size
            extents.append((base + read(offset_size), read(length_size)))
        if item == primary:
            return b''.join(data[offset:offset+length] for offset, length in extents)
    raise ValueError('primary item missing')

def load_series(path):
    records = json.loads(path.read_text())['quality']
    series = {}
    for record in records:
        if record.get('compare_exit') == 0 and record.get('dav1d_exit') == 0:
            series.setdefault((record['input'], record['config']), []).append(record)
    return series

def main():
    manifest = json.loads((scratch / 'inputs/manifest.json').read_text())['inputs']
    series = load_series(Path(sys.argv[2]) if len(sys.argv) > 2 else root / 'benchmarks/results/codecs/avif/results.json')
    out = Path(sys.argv[1])
    records = json.loads(out.read_text()) if out.exists() else []
    done = {r['input'] for r in records}
    for index, item in enumerate(manifest):
        if item['source'] in done:
            continue
        fixture = fixtures / f'{index}.avif'
        obu = fixtures / f'{index}.obu'
        if not fixture.exists():
            source_pnm = scratch / 'inputs' / item['pnm']
            temporary = fixtures / f'{index}.tmp.avif'
            run = subprocess.run([str(x) for x in [avifcodec, 'encode', source_pnm, temporary, item['width'], item['height'], 420, 6, 90, 1]],
                                 env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            if run.returncode:
                raise SystemExit(f'fixture failed for {source_pnm}: {run.stderr.decode()}')
            temporary.replace(fixture)
        if not obu.exists():
            obu.write_bytes(extract_primary(fixture.read_bytes()))
        while True:
            if not gate():
                sys.exit(75)
            batch = []
            for iteration in range(5):
                encode_order = encode_configs if iteration % 2 == 0 else list(reversed(encode_configs))
                decode_order = decode_configs if iteration % 2 == 0 else list(reversed(decode_configs))
                source_pnm = scratch / 'inputs' / item['pnm']
                for config in encode_order:
                    points = bracket(series.get((item['source'], series_config.get(config, config)), []), 90)
                    if points is None:
                        batch.append({'role': 'encode', 'input': item['source'], 'group': item['group'], 'config': config,
                                      'iteration': iteration, 'exit': None, 'error': 'score 90 not bracketed'})
                        continue
                    for point in points:
                        dest = scratch / f'qualification-encode-{iteration}-{index}-{config}.avif'
                        if config.startswith('img-libaom-'):
                            args = [scratch / 'integrated-target/dist/encode-integrated', config, source_pnm, point['quality'], dest]
                        elif config == 'aom-s6-t4':
                            args = [avifcodec, 'encode', source_pnm, dest, item['width'], item['height'], 420, 6, point['quality'], 4]
                        elif config.startswith('aom-'):
                            _, speed, chroma = config.split('-')
                            args = [avifcodec, 'encode', source_pnm, dest, item['width'], item['height'], chroma, speed[1:], point['quality'], 1]
                        elif config.startswith('svt-'):
                            _, speed, chroma = config.split('-')
                            args = [avifcodec_svt, 'encode', source_pnm, dest, item['width'], item['height'], chroma, speed[1:], point['quality'], 1, 'svt']
                        else:
                            args = [encode_bin, config, source_pnm, point['quality'], dest]
                        batch.append({'role': 'encode', 'input': item['source'], 'group': item['group'], 'config': config,
                                      'iteration': iteration, 'quality': point['quality'], 'score': point['score'], **measure(args, dest)})
                for config in decode_order:
                    dest = scratch / f'qualification-decode-{iteration}-{index}-{config}.rgb'
                    if config == 'dav1d':
                        args = [avifcodec, 'decode', fixture, dest]
                    elif config == 'zen':
                        args = [decode_bin, 'zen', fixture, dest]
                    else:
                        args = [decode_bin, 'rav1d', obu, dest]
                    batch.append({'role': 'decode', 'input': item['source'], 'group': item['group'], 'config': config,
                                  'iteration': iteration, **measure(args, dest)})
            if not gate('-1'):
                print(f'discarded: post-check failed for {item["source"]}', file=sys.stderr)
                continue
            records.extend(batch)
            out.write_text(json.dumps(records, separators=(',', ':')))
            print(f'completed {index+1}/{len(manifest)} {item["source"]}', flush=True)
            break

if __name__ == '__main__':
    main()
