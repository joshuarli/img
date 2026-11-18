#!/usr/bin/env python3
import collections, json, statistics, sys
from matching import bracket, interpolate

data = json.load(open(sys.argv[1]))
records = data if isinstance(data, list) else data['quality']
timing = [] if isinstance(data, list) else data.get('timing', [])
timing_status = 'not measured' if isinstance(data, list) else data.get('timing_status', 'not measured')
configs = ['jpegli-420', 'jpegli-444', 'jpegli-xyb', 'moz-420', 'moz-444', 'img-fast', 'img-best', 'zen-fast', 'zen-best']
groups = ['color photo', 'grayscale', 'ui/text']
series = collections.defaultdict(list)
for r in records:
    if r.get('compare_exit') == 0 and r.get('turbo_exit') == 0:
        series[r['group'], r['input'], r['config']].append(r)

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
    print('\n| Relative bytes | Score 80 | Score 90 | Score 95 |')
    print('| --- | ---: | ---: | ---: |')
    for config, baseline in [('jpegli-420','zen-best'), ('jpegli-444','zen-best'), ('img-fast','zen-best'), ('img-best','zen-best'), ('jpegli-420','img-fast'), ('jpegli-444','img-fast')]:
        values = []
        for target in (80, 90, 95):
            selected = common[target]
            if not selected:
                values.append('unavailable (0)')
                continue
            candidate = sum(interpolate(series[group, image, config], target) for image in selected)
            reference = sum(interpolate(series[group, image, baseline], target) for image in selected)
            values.append(f'{(candidate / reference - 1) * 100:+.1f}% ({len(selected)})')
        print('| ' + ' | '.join([f'{config} / {baseline}'] + values) + ' |')
print('\n### Failures\n')
counts = collections.Counter()
for r in records:
    stage = 'encode' if r['encode_exit'] else 'decode' if r.get('turbo_exit', 0) else 'compare' if r.get('compare_exit', 0) else None
    if stage is None: continue
    detail = r.get({'encode':'encode_error','decode':'turbo_error','compare':'compare_error'}[stage], '')
    if stage == 'compare':
        try:
            failure = json.loads(detail)['failures'][0]
            detail = failure.get('kind','unknown') + ': ' + failure.get('message','unknown')
        except (ValueError, KeyError, IndexError):
            pass
    counts[r['config'],stage,detail.strip()] += 1
for (config, stage, detail), count in sorted(counts.items()):
    print(f'- {config}: {count} {stage} failures: {detail}')

for group in groups:
    print(f'\n### {group} timing\n')
    print('| Role | Configuration | Wall ms | Wall range ms | User CPU ms | RSS MiB |')
    print('| --- | --- | ---: | ---: | ---: | ---: |')
    for role, candidates in [('encode', [x for x in configs if x != 'jpegli-xyb']), ('decode', ['turbo', 'jpegli', 'zune', 'zen'])]:
        all_samples = collections.defaultdict(list)
        for r in timing:
            if r['role'] == role and r['group'] == group and r['exit'] == 0:
                all_samples[r['config'], r['input']].append(r)
        images = {r['input'] for r in timing if r['role'] == role and r['group'] == group}
        summaries = {}
        for key, samples in all_samples.items():
            if role == 'decode':
                if len(samples) != 5: continue
                summaries[key] = {'wall_ms':statistics.median(r['wall_ms'] for r in samples), 'low':min(r['wall_ms'] for r in samples), 'high':max(r['wall_ms'] for r in samples), 'cpu_ms':statistics.median(r['cpu_ms'] for r in samples), 'rss_bytes':statistics.median(r['rss_bytes'] for r in samples)}
                continue
            by_quality = collections.defaultdict(list)
            for r in samples: by_quality[r['quality']].append(r)
            if len(by_quality) != 2 or any(len(s) != 5 for s in by_quality.values()): continue
            endpoints = [s[0] for s in by_quality.values()]
            pair = bracket(endpoints, 90)
            if pair is None: continue
            a,b = pair; weight = (90-a['score'])/(b['score']-a['score'])
            lo,hi = by_quality[a['quality']],by_quality[b['quality']]
            def blend(fn, field): return fn(r[field] for r in lo)*(1-weight)+fn(r[field] for r in hi)*weight
            summaries[key] = {'wall_ms':blend(statistics.median,'wall_ms'), 'low':blend(min,'wall_ms'), 'high':blend(max,'wall_ms'), 'cpu_ms':blend(statistics.median,'cpu_ms'), 'rss_bytes':max(statistics.median(r['rss_bytes'] for r in s) for s in (lo,hi))}
        common = {image for image in images if all((config,image) in summaries for config in candidates)}
        for config in candidates:
            complete = [summaries[config,image] for image in sorted(common)]
            if not complete:
                values = [timing_status] * 4 if not timing else ['unavailable'] * 4
            else:
                means = [statistics.mean(s[key] for s in complete) / scale for key, scale in [('wall_ms', 1), ('cpu_ms', 1), ('rss_bytes', 1048576)]]
                low = statistics.mean(s['low'] for s in complete)
                high = statistics.mean(s['high'] for s in complete)
                values = [f'{means[0]:.2f} ({len(complete)})', f'{low:.2f}–{high:.2f}', f'{means[1]:.2f}', f'{means[2]:.2f}']
            print('| ' + ' | '.join([role, config] + values) + ' |')
