#!/usr/bin/env python3
import json, os, subprocess, sys
from pathlib import Path

root = Path(__file__).resolve().parents[3]
scratch = Path(os.environ.get('AVIF_SCRATCH', root / '.work/tmp/avif-candidates'))
img = Path(os.environ.get('IMG_BIN', root / '.work/refs/img-master-2258e42'))
integrated_img = Path(os.environ.get('IMG_INTEGRATED_BIN', root / 'target/dist/img'))
zen = Path(os.environ.get('ZEN_BIN', root / '.work/refs/zen-5424df8'))
avifcodec = scratch / 'avifcodec'
avifcodec_svt = scratch / 'avifcodec-svt'
encode_bin = scratch / 'encode-target/release/avif-candidate-encode'
decode_bin = scratch / 'decode-target/release/avif-candidate-decode'
djpeg = scratch / 'build/turbo/djpeg-static'

corpus = root / 'benchmarks/corpus'
paths = sorted(corpus.glob('*.png')) + sorted((corpus / 'validation').glob('*.png')) + \
    sorted((corpus / 'validation').glob('*.jpg')) + [corpus / 'generated/large.png']
paths = [p for p in paths if not p.name.startswith('boundary-')]

photo = {
    'astronaut.png', 'coffee.png', 'chelsea.png', 'charolais-cattle.jpg', 'goose-head.jpg',
    'hubble_deep_field.jpg', 'marmolada.jpg', 'mountain-landscape-met.jpg', 'piraeus-ferry.jpg',
    'portrait-woman-met.jpg', 'retina.jpg', 'rocket.jpg', 'sous-tour-eiffel.jpg', 'utrecht-meetup.jpg',
}
gradient = {'large.png'}

def group(path):
    if path.name in photo: return 'photo'
    if path.name in gradient: return 'synthetic gradient'
    return 'unclassified'

configs = ['aom-s4-420', 'aom-s6-420', 'aom-s8-420', 'aom-s4-444', 'aom-s6-444', 'aom-s8-444',
           'svt-s4-420', 'svt-s8-420', 'img-fast', 'img-balanced', 'img-best',
           'zen-fast', 'zen-balanced', 'zen-best', 'img-libaom-fast', 'img-libaom-balanced', 'img-libaom-best']
qualities = [20, 30, 40, 50, 60, 65, 70, 75, 80, 85, 88, 90, 92, 95, 98, 100]

def run(command):
    p = subprocess.run([str(x) for x in command], stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                       env={**os.environ, 'RAYON_NUM_THREADS': '1'})
    if p.returncode == -9:
        raise SystemExit('process killed by SIGKILL: ' + str(command))
    return p.returncode, p.stdout.decode(errors='replace'), p.stderr.decode(errors='replace')

def command(config, item, source_pnm, dest, quality, threads=1):
    if config.startswith('img-libaom-'):
        return [scratch / 'integrated-target/dist/encode-integrated', config, source_pnm, quality, dest]
    if config.startswith('aom-'):
        _, speed, chroma = config.split('-')
        return [avifcodec, 'encode', source_pnm, dest, item['width'], item['height'],
                chroma, speed[1:], quality, threads]
    if config.startswith('svt-'):
        _, speed, chroma = config.split('-')
        return [avifcodec_svt, 'encode', source_pnm, dest, item['width'], item['height'],
                chroma, speed[1:], quality, threads, 'svt']
    return [encode_bin, config, source_pnm, quality, dest]

def main():
    out = Path(sys.argv[1])
    manifest = json.loads((scratch / 'inputs/manifest.json').read_text())['inputs']
    selected = sys.argv[2].split(',') if len(sys.argv) > 2 else configs
    records = json.loads(out.read_text()) if out.exists() else []
    done = {(r['input'], r['config'], r['quality']) for r in records}
    for item in manifest:
        source_pnm = scratch / 'inputs' / item['pnm']
        source_png = scratch / 'inputs' / item['input']
        for config in selected:
            for quality in qualities:
                if (item['source'], config, quality) in done:
                    continue
                dest = scratch / f'sweep-output-{os.getpid()}.avif'
                dest.unlink(missing_ok=True)
                code, stdout, stderr = run(command(config, item, source_pnm, dest, quality))
                record = {'input': item['source'], 'group': item['group'], 'config': config,
                          'quality': quality, 'encode_exit': code}
                if code == 0 and dest.is_file():
                    record['bytes'] = dest.stat().st_size
                    if config.startswith('img-libaom-'):
                        from timed import extract_primary
                        record['payload_bytes'] = len(extract_primary(dest.read_bytes()))
                        reference = scratch / f'candidate-output-{os.getpid()}.avif'
                        candidate = 'aom-s8-444' if config == 'img-libaom-fast' else 'aom-s6-444'
                        rc, _, error = run(command(candidate, item, source_pnm, reference, quality))
                        record['candidate_encode_exit'] = rc
                        if rc == 0:
                            record['candidate_payload_bytes'] = len(extract_primary(reference.read_bytes()))
                            reference.unlink()
                        else:
                            record['candidate_encode_error'] = error[-500:]

                    dc, _, de = run([avifcodec, 'decode', dest])
                    record['dav1d_exit'] = dc
                    if dc:
                        record['dav1d_error'] = de[-500:]
                    sc, so, se = run([integrated_img if config.startswith('img-libaom-') else img, 'compare', source_png, dest, '--json'])
                    record['compare_exit'] = sc
                    if sc == 0:
                        record['score'] = json.loads(so)['results'][0]['score']
                    else:
                        record['compare_error'] = (so + se)[-500:]
                else:
                    record['encode_error'] = (stdout + stderr)[-500:]
                records.append(record)
                out.write_text(json.dumps(records, separators=(',', ':')))
                print(item['source'], config, quality, record.get('score'), record.get('bytes'), flush=True)

if __name__ == '__main__':
    main()
