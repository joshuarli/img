import json
import math
import os
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

source = Path('/Users/josh/d/img/.work/tmp/jpeg-candidates')
work = Path(sys.argv[1])
binaries = dict(arg.split('=', 1) for arg in sys.argv[2:])
judge = '/Users/josh/d/img/.work/refs/img-da66fa9'
decoder = source / 'turbo-build/djpeg-static'
group = os.environ.get('GROUP', 'gray')
manifest = [item for item in json.loads((source / 'inputs/manifest.json').read_text())
            if (item['channels'] == 1) == (group == 'gray')]
qualities = [q / 2 for q in range(40 if group == 'gray' else 130, 201)]
work.mkdir(parents=True, exist_ok=True)


def run(args):
    item, name, quality = args
    original = source / 'inputs' / item['input']
    output = work / f'{name}-{item["input"]}-{quality}.jpg'
    output.unlink(missing_ok=True)
    subprocess.run([binaries[name], 'convert', original, '-o', output, '--format', 'jpeg',
                    '--quality', str(quality), '--effort', 'balanced', '--quiet'],
                   check=True, capture_output=True)
    size = output.stat().st_size
    if name == 'candidate':
        subprocess.run([decoder, '-outfile', '/dev/null', output], check=True, capture_output=True)
    result = subprocess.run([judge, 'compare', original, output, '--json'],
                            check=True, capture_output=True)
    score = json.loads(result.stdout)['results'][0]['score']
    output.unlink()
    return [item['input'], name, quality, size, score]


with ThreadPoolExecutor(max_workers=4) as executor:
    records = list(executor.map(run, ((item, name, q) for item in manifest
                                      for name in binaries for q in qualities)))
summary = {}
for score in (80, 90, 95):
    ratios = []
    for item in manifest:
        sizes = {}
        for name in ('base', 'candidate'):
            matches = [record[3] for record in records if record[0] == item['input']
                       and record[1] == name and record[4] >= score]
            if matches:
                sizes[name] = min(matches)
        if len(sizes) == 2:
            ratios.append(sizes['candidate'] / sizes['base'])
    summary[str(score)] = {'count': len(ratios), 'geomean': math.exp(sum(map(math.log, ratios)) / len(ratios)) if ratios else None}
result = {'group': group, 'inputs': len(manifest), 'decode_failures': 0,
          'candidate_decodes': sum(record[1] == 'candidate' for record in records),
          'summary': summary, 'records': records}
(work / os.environ.get('OUT', 'sweep.json')).write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({key: value for key, value in result.items() if key != 'records'}, indent=2))
