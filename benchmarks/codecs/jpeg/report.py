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
    print('| Role | Configuration | Wall ms | Wall range ms | User CPU ms | RSS MiB |')
    print('| --- | --- | ---: | ---: | ---: | ---: |')
    for role, candidates in [('encode', configs), ('decode', ['turbo', 'jpegli', 'zune', 'zen'])]:
        all_samples = collections.defaultdict(list)
        for r in timing:
            if r['role'] == role and r['group'] == group and r['exit'] == 0:
                all_samples[r['config'], r['input']].append(r)
        images = {r['input'] for r in timing if r['role'] == role and r['group'] == group}
        common = {image for image in images if all(len(all_samples[config, image]) == 5 for config in candidates if config != 'jpegli-xyb')}
        for config in candidates:
            selected = images if config == 'jpegli-xyb' else common
            complete = [all_samples[config, image] for image in sorted(selected) if len(all_samples[config, image]) == 5]
            if not complete:
                values = [timing_status] * 4 if not timing else ['unavailable'] * 4
            else:
                means = [statistics.mean(statistics.median(r[key] for r in s) for s in complete) / scale for key, scale in [('wall_ms', 1), ('cpu_ms', 1), ('rss_bytes', 1048576)]]
                low = statistics.mean(min(r['wall_ms'] for r in s) for s in complete)
                high = statistics.mean(max(r['wall_ms'] for r in s) for s in complete)
                values = [f'{means[0]:.2f} ({len(complete)})', f'{low:.2f}–{high:.2f}', f'{means[1]:.2f}', f'{means[2]:.2f}']
            print('| ' + ' | '.join([role, config] + values) + ' |')
