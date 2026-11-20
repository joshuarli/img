#!/usr/bin/env python3
"""Generate the JPEG decode qualification fixtures: python3 decode-fixtures.py INPUTS IMG CJPEG MOZCJPEG OUT."""
import hashlib, json, pathlib, shutil, subprocess, sys

inputs, img, cjpeg, moz, out = map(pathlib.Path, sys.argv[1:])
out.mkdir(parents=True, exist_ok=True)
sources = sorted(inputs.glob('[0-9][0-9]-*.pnm'))
manifest = []

def emit(cls, name, command):
    target = out / f'{cls}-{name}.jpg'
    target.unlink(missing_ok=True)
    args = [str(c) for c in command]
    if cls.startswith('img'):
        args += ['-o', str(target)]
    else:
        args[-1:-1] = ['-outfile', str(target)]
    subprocess.run(args, check=True, capture_output=True)
    manifest.append({'class': cls, 'file': target.name, 'sha256': hashlib.sha256(target.read_bytes()).hexdigest()})

for source in sources:
    name = source.stem
    gray = source.read_bytes().startswith(b'P5')
    png = source.with_suffix('.png')
    emit('img-fast', name, [img, 'convert', png, '--format', 'jpeg', '--quality', '90', '--effort', 'fast'])
    emit('img-balanced', name, [img, 'convert', png, '--format', 'jpeg', '--quality', '90', '--effort', 'balanced'])
    if gray:
        emit('turbo-gray', name, [cjpeg, '-quality', '90', source])
        continue
    emit('turbo-base444', name, [cjpeg, '-quality', '90', '-sample', '1x1', source])
    emit('turbo-base420', name, [cjpeg, '-quality', '85', '-sample', '2x2', source])
    emit('turbo-prog420', name, [cjpeg, '-quality', '85', '-progressive', source])
    emit('turbo-restart', name, [cjpeg, '-quality', '90', '-restart', '1', source])
    emit('moz-prog', name, [moz, '-quality', '85', source])
for camera in sorted((pathlib.Path(__file__).resolve().parents[3] / 'benchmarks/corpus/validation').glob('*.jpg')):
    target = out / f'camera-{camera.stem}.jpg'
    shutil.copyfile(camera, target)
    manifest.append({'class': 'camera', 'file': target.name, 'sha256': hashlib.sha256(target.read_bytes()).hexdigest()})
(out / 'manifest.json').write_text(json.dumps(manifest, indent=1) + '\n')
print(len(manifest), 'fixtures')
