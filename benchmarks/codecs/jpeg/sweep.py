#!/usr/bin/env python3
import json, os, subprocess, sys
from pathlib import Path

root = Path(__file__).resolve().parents[3]
scratch = Path('/Users/josh/d/img/.work/tmp/jpeg-candidates')
img = Path('/Users/josh/d/img/.work/refs/img-master-2258e42')
zen = Path('/Users/josh/d/img/.work/refs/zen-5424df8')
jpegli = scratch / 'jxl-build/tools/cjpegli'
moz = scratch / 'moz-local/cjpeg-static'
turbo = scratch / 'turbo-build/djpeg-static'
qualities = list(range(50, 100, 5)) + [97]
configs = ['jpegli', 'jpegli-xyb', 'moz-420', 'moz-444', 'img-fast', 'img-best', 'zen-fast', 'zen-best']
paths = sorted((root / 'benchmarks/corpus').glob('*.png')) + sorted((root / 'benchmarks/corpus/validation').glob('*.png')) + sorted((root / 'benchmarks/corpus/validation').glob('*.jpg'))
paths = [p for p in paths if not p.name.startswith('boundary-') and p.name not in ('mm-logo-alpha.png', 'horse.png', 'photopea-ui.png')]
gray = {'grass.png', 'gravel.png', 'brick.png', 'camera.png', 'cell.png', 'clock_motion.png', 'coins.png', 'microaneurysms.png'}
def group(p):
    return 'ui/text' if p.name == 'text.png' else 'grayscale' if p.name in gray else 'color photo'
def run(command):
    p = subprocess.run([str(x) for x in command], stdout=subprocess.PIPE, stderr=subprocess.PIPE, env={**os.environ, 'RAYON_NUM_THREADS': '1'})
    if p.returncode == -9:
        raise SystemExit('process killed by SIGKILL: ' + str(command))
    return p.returncode, p.stdout.decode(errors='replace'), p.stderr.decode(errors='replace')
def command(config, source, dest, quality):
    if config.startswith('jpegli'):
        return [jpegli, source, dest, '-q', quality] + (['--xyb'] if config.endswith('xyb') else [])
    if config.startswith('moz-'):
        return [moz, '-quality', quality, '-sample', '2x2' if config.endswith('420') else '1x1', '-outfile', dest, source]
    binary = img if config.startswith('img-') else zen
    return [binary, 'convert', source, '-o', dest, '--format', 'jpeg', '--quality', quality, '--effort', config.split('-')[1], '--quiet']

def main():
    out = Path(sys.argv[1])
    records = json.loads(out.read_text()) if out.exists() else []
    done = {(r['input'], r['config'], r['quality']) for r in records}
    for source in paths:
        relative = str(source.relative_to(root))
        for config in configs:
            for quality in qualities:
                if (relative, config, quality) in done:
                    continue
                dest = scratch / 'sweep-output.jpg'
                dest.unlink(missing_ok=True)
                code, stdout, stderr = run(command(config, source, dest, quality))
                record = {'input': relative, 'group': group(source), 'config': config, 'quality': quality, 'encode_exit': code}
                if code == 0 and dest.is_file():
                    record['bytes'] = dest.stat().st_size
                    dc, _, de = run([turbo, '-outfile', '/dev/null', dest])
                    record['turbo_exit'] = dc
                    if dc:
                        record['turbo_error'] = de[-500:]
                    sc, so, se = run([img, 'compare', source, dest, '--json'])
                    record['compare_exit'] = sc
                    if sc == 0:
                        record['score'] = json.loads(so)['results'][0]['score']
                    else:
                        record['compare_error'] = (so + se)[-500:]
                else:
                    record['encode_error'] = (stdout + stderr)[-500:]
                records.append(record)
                out.write_text(json.dumps(records, separators=(',', ':')))
                print(relative, config, quality, record.get('score'), flush=True)

if __name__ == "__main__":
    main()
