#!/usr/bin/env python3
"""Untimed AVIF search comparison: python3 search-probe.py OUT.json SCORES LABEL=BINARY... -- INPUT...
Runs optimize --format avif --score S --effort balanced --verbose for every input, score and binary, and records the
output bytes, achieved score and the number of encodes each search made (a host-independent work count)."""
import json, os, re, subprocess, sys, tempfile
args = sys.argv[1:]; split = args.index('--')
out, scores = args[0], args[1].split(',')
tools = [a.split('=', 1) for a in args[2:split]]; inputs = args[split + 1:]
line = re.compile(r'Avif quality=Some\(([0-9.]+)\) sampling=None bytes=Some\((\d+)\) score=Some\(([-0-9.]+)\)')
rows = []
with tempfile.TemporaryDirectory(dir=os.environ.get('TMPDIR')) as scratch:
    for source in inputs:
        for score in scores:
            for label, binary in tools:
                dest = os.path.join(scratch, f'{label}.avif')
                p = subprocess.run([binary, 'optimize', source, '-o', dest, '--format', 'avif', '--score', score, '--effort', 'balanced', '--verbose'],
                                   capture_output=True, text=True)
                evals = [(float(q), int(b), float(s)) for q, b, s in line.findall(p.stderr)]
                rows.append(dict(input=os.path.basename(source), score=score, tool=label, exit=p.returncode, encodes=len(evals),
                                 bytes=os.path.getsize(dest) if p.returncode == 0 else None, evaluations=evals))
                if os.path.exists(dest): os.unlink(dest)
            print(os.path.basename(source), score, *(f"{r['tool']}={r['bytes']}/{r['encodes']}" for r in rows[-len(tools):]), flush=True)
json.dump(dict(tools={l: b for l, b in tools}, scores=scores, rows=rows), open(out, 'w'), indent=1)
