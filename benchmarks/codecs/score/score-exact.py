#!/usr/bin/env python3
"""Bit-identity and timing of SSIMULACRA2 scoring, base vs new:
python3 score-exact.py OUT_DIR BASE_IMG NEW_LOOP BASE_LOOP CORPUS_DIR FILE:FORMAT...
Each FILE is encoded once at quality 80 in FORMAT by BASE_IMG; both score loops must return the same bits."""
import json, pathlib, subprocess, sys
sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import verdict
out, base_img, new_loop, base_loop, corpus = map(pathlib.Path, sys.argv[1:6])
out.mkdir(parents=True, exist_ok=True)
rows, differ = [], []
for spec in sys.argv[6:]:
    name, fmt = spec.rsplit(':', 1)
    candidate = out / f'{name}.{fmt}'
    subprocess.run([base_img, 'convert', corpus / name, '-o', candidate, '--format', fmt, '--quality', '80', '--quiet'], check=True)
    result = {}
    for label, loop in (('base', base_loop), ('new', new_loop)):
        _, score, ms = subprocess.run([loop, '300', corpus / name, candidate], check=True, capture_output=True, text=True).stdout.split()
        result[label] = (score, float(ms))
    same = result['base'][0] == result['new'][0]
    differ += [] if same else [name]
    rows.append(dict(file=name, candidate=fmt, base_score=result['base'][0], new_score=result['new'][0], base_ms=result['base'][1], new_ms=result['new'][1]))
    print(f'{name}: {"same score" if same else "SCORE DIFFERS"} {result["new"][0]}; per-score ms base {result["base"][1]:.3f} new {result["new"][1]:.3f}')
(out / 'scores.json').write_text(json.dumps(rows, indent=1))
regressed = verdict.write(out / 'scores.verdict.json', [verdict.judge('score_ms', [(r['file'], r['new_ms'] / r['base_ms']) for r in rows])])
if differ: print('scores differ from base:', *differ)
sys.exit(1 if differ else 3 if regressed else 0)
