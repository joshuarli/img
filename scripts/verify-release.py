#!/usr/bin/env python3
import json
import subprocess
import sys
import tempfile
from pathlib import Path

binary = Path(sys.argv[1]).resolve()
target = sys.argv[2]
root = Path(__file__).resolve().parent.parent

def output(*args):
    return subprocess.check_output(args, text=True)

assert output(str(binary), '--version').startswith('img ')
if target.endswith('-linux-musl'):
    header = output('readelf', '-hW', str(binary))
    expected = 'AArch64' if target.startswith('aarch64-') else 'Advanced Micro Devices X86-64'
    assert expected in header, header
    assert 'INTERP' not in output('readelf', '-lW', str(binary))
    assert 'NEEDED' not in output('readelf', '-dW', str(binary))
elif target == 'aarch64-apple-darwin':
    assert 'arm64' in output('file', '-b', str(binary))
    for line in output('otool', '-L', str(binary)).splitlines()[1:]:
        assert line.strip().startswith(('/usr/lib/', '/System/Library/')), line
else:
    raise SystemExit(f'unsupported release target: {target}')

with tempfile.TemporaryDirectory(prefix='release-smoke-', dir=root / '.work/tmp') as scratch:
    image = root / 'benchmarks/resource-rgb.png'
    destination = Path(scratch) / 'exact.webp'
    subprocess.run([str(binary), 'convert', str(image), '--lossless', '-o', str(destination), '--quiet'], check=True)
    result = json.loads(output(str(binary), 'compare', str(image), str(destination), '--json'))
    assert not result['failures'], result
    assert result['results'][0]['score'] == 100, result
print(f'verified {target}: linkage and actual-byte identity 100')
