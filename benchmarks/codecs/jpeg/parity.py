#!/usr/bin/env python3
import json, subprocess
from sweep import scratch, configs, command

items = json.loads((scratch / 'inputs/manifest.json').read_text())
for item in items:
    source = scratch / 'inputs' / item['input']
    pnm = source.with_suffix('.pnm')
    for config in configs:
        if config == 'jpegli-xyb': continue
        cli = scratch / 'parity-cli.jpg'; api = scratch / 'parity-api.jpg'
        cli.unlink(missing_ok=True); api.unlink(missing_ok=True)
        p = subprocess.run([str(x) for x in command(config, source, cli, 90)], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        if p.returncode: raise SystemExit(f'{item["source"]} {config} CLI failed: {p.stderr.decode()}')
        exe = scratch / ('encode-jpegli' if config.startswith('jpegli-') else 'encode-moz' if config.startswith('moz-') else 'decode-target/release/jpeg-candidate-encode')
        p = subprocess.run([str(exe),config,str(pnm),'90',str(api)], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        if p.returncode: raise SystemExit(f'{item["source"]} {config} API failed: {p.stderr.decode()}')
        if cli.read_bytes() != api.read_bytes(): raise SystemExit(f'{item["source"]} {config}: output mismatch')
    print(item['source'], 'all outputs match', flush=True)
