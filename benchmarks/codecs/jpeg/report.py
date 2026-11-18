#!/usr/bin/env python3
import collections, json, statistics, sys

data = json.load(open(sys.argv[1]))
records = data if isinstance(data, list) else data['quality']
timing = [] if isinstance(data, list) else data.get('timing', [])
timing_status = 'not measured' if isinstance(data, list) else data.get('timing_status', 'not measured')
configs = ['jpegli', 'jpegli-xyb', 'moz-420', 'moz-444', 'img-fast', 'img-best', 'zen-fast', 'zen-best']
groups = ['color photo', 'grayscale', 'ui/text']
series = collections.defaultdict(list)
for r in records:
    if r.get('compare_exit') == 0 and r.get('turbo_exit') == 0:
        series[r['group'], r['input'], r['config']].append(r)

def interpolate(points, target):
    points = sorted(points, key=lambda x: x['quality'])
    for a, b in zip(points, points[1:]):
        if b['quality'] - a['quality'] > 5 and (a['quality'], b['quality']) != (95, 97):
            continue
        if a['score'] <= target <= b['score'] and a['score'] < b['score']:
            return a['bytes'] + (b['bytes'] - a['bytes']) * (target - a['score']) / (b['score'] - a['score'])
    return None

for group in groups:
    images = sorted({r['input'] for r in records if r['group'] == group})
    print(f'\n### {group} ({len(images)} images)\n')
    print('| Configuration | Score 80: mean bytes (n) | Score 90: mean bytes (n) | Score 95: mean bytes (n) |')
    print('| --- | ---: | ---: | ---: |')
    common = {target: [image for image in images if all(interpolate(series[group, image, config], target) is not None for config in configs if config != 'jpegli-xyb')] for target in (80, 90, 95)}
    for config in configs:
        values = []
        for target in (80, 90, 95):
            selected = images if config == 'jpegli-xyb' else common[target]
            matched = [v for image in selected if (v := interpolate(series[group, image, config], target)) is not None]
            values.append(f'{round(statistics.mean(matched))} ({len(matched)})' if matched else 'unavailable (0)')
        print('| ' + ' | '.join([config] + values) + ' |')
print('\n### Failures\n')
counts = collections.Counter((r['config'], 'encode' if r['encode_exit'] else 'decode' if r.get('turbo_exit', 0) else 'compare') for r in records if r['encode_exit'] or r.get('turbo_exit', 0) or r.get('compare_exit', 0))
for (config, stage), count in sorted(counts.items()):
    print(f'- {config}: {count} {stage} failures')

for group in groups:
    print(f'\n### {group} timing\n')
    print('| Role | Configuration | Wall ms | CPU ms | RSS MiB |')
    print('| --- | --- | ---: | ---: | ---: |')
    for role, candidates in [('encode', configs), ('decode', ['turbo', 'jpegli', 'zune', 'zen'])]:
        for config in candidates:
            samples = collections.defaultdict(list)
            for r in timing:
                if r['role'] == role and r['group'] == group and r['config'] == config and r['exit'] == 0:
                    samples[r['input']].append(r)
            complete = [s for s in samples.values() if len(s) == 5]
            if not complete:
                values = [timing_status] * 3 if not timing else ['unavailable'] * 3
            else:
                values = [f'{statistics.mean(statistics.median(r[key] for r in s) for s in complete) / scale:.2f} ({len(complete)})' for key, scale in [('wall_ms', 1), ('cpu_ms', 1), ('rss_bytes', 1048576)]]
            print('| ' + ' | '.join([role, config] + values) + ' |')
