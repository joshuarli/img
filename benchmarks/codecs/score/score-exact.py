#!/usr/bin/env python3
"""Bit-identity and timing of SSIMULACRA2 scoring, base vs new:
python3 score-exact.py OUT_DIR IMG NEW_LOOP BASE_LOOP CORPUS_DIR FILE:FORMAT...
IMG decodes each FILE to a lossless PNG, encodes it once at quality 80 in FORMAT and decodes that candidate to a lossless PNG,
so both score loops see identical pixels and must return the same bits."""
import json, pathlib, subprocess, sys
sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import verdict
out, img, new_loop, base_loop, corpus = map(pathlib.Path, sys.argv[1:6])
out.mkdir(parents=True, exist_ok=True)
rows, differ = [], []
for spec in sys.argv[6:]:
    name, fmt = spec.rsplit(':', 1)
    source, encoded, candidate = out / f'{name}.source.png', out / f'{name}.{fmt}', out / f'{name}.{fmt}.png'
    convert = lambda src, dst, *args: subprocess.run([img, 'convert', src, '-o', dst, *args, '--quiet'], check=True)
    convert(corpus / name, source, '--format', 'png', '--lossless')
    convert(source, encoded, '--format', fmt, '--quality', '80')
    convert(encoded, candidate, '--format', 'png', '--lossless')
    result = {}
    for label, loop in (('base', base_loop), ('new', new_loop)):
        _, score, ms = subprocess.run([loop, '300', source, candidate], check=True, capture_output=True, text=True).stdout.split()
        result[label] = (score, float(ms))
    same = result['base'][0] == result['new'][0]
    differ += [] if same else [name]
    rows.append(dict(file=name, candidate=fmt, base_score=result['base'][0], new_score=result['new'][0], base_ms=result['base'][1], new_ms=result['new'][1]))
    print(f'{name}: {"same score" if same else "SCORE DIFFERS"} {result["new"][0]}; per-score ms base {result["base"][1]:.3f} new {result["new"][1]:.3f}')
(out / 'scores.json').write_text(json.dumps(rows, indent=1))
regressed = verdict.write(out / 'scores.verdict.json', [verdict.judge('score_ms', [(r['file'], r['new_ms'] / r['base_ms']) for r in rows])])
if differ: print('scores differ from base:', *differ)
sys.exit(1 if differ else 3 if regressed else 0)
