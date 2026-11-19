import json
import math
import pathlib
import subprocess
import sys

stock, patched, generated, destination = sys.argv[1:]
root = pathlib.Path(generated)
classes = {
    'img progressive': sorted(root.glob('img-balanced-*.jpg')),
    'camera progressive': [pathlib.Path('benchmarks/corpus/validation/marmolada.jpg')],
    'turbo progressive': sorted(root.glob('turbo-prog-*.jpg')),
    'moz progressive': sorted(root.glob('moz-prog-*.jpg')),
    'baseline': sorted(root.glob('turbo-base-*.jpg')),
    'grayscale': sorted(root.glob('turbo-gray-*.jpg')),
}
subprocess.run(['perf-gate-ready'], check=True)
records = []
for name, paths in classes.items():
    if not paths:
        raise ValueError(f'empty class: {name}')
    for i, path in enumerate(paths):
        cases = [('stock', stock, 'zune'), ('patched', patched, 'zune'), ('zen', stock, 'zen'), ('turbo', stock, 'turbo')]
        cases = cases[i % 4:] + cases[:i % 4]
        for label, binary, decoder in cases:
            result = subprocess.run([binary, decoder, '5', str(path)], check=True, text=True, capture_output=True)
            row = json.loads(result.stdout)
            row.update({'class': name, 'decoder': label})
            records.append(row)
    print(name, 'complete', flush=True)
subprocess.run(['perf-gate-ready', '-1'], check=True)
pathlib.Path(destination).write_text(json.dumps({'iterations': 5, 'records': records}, indent=2) + '\n')
for name, paths in classes.items():
    values = []
    for decoder in ('stock', 'patched', 'zen', 'turbo'):
        times = [r['median_ms'] for r in records if r['class'] == name and r['decoder'] == decoder]
        values.append(math.exp(sum(map(math.log, times)) / len(times)))
    print(name, len(paths), *(f'{v:.3f}' for v in values))
