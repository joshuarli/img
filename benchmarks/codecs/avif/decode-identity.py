#!/usr/bin/env python3
import json, pathlib, subprocess, sys
fixtures, baseline, candidate, scratch = map(pathlib.Path, sys.argv[1:])
manifest = json.loads((fixtures/'manifest.json').read_text()); mismatches=[]
for f in manifest:
    name=f['name']; outputs=[]
    for binary in (baseline, candidate):
        out=scratch/(binary.name+'.rgb')
        subprocess.run([str(binary),'img','0',str(fixtures/(name+'.avif')),str(out)],check=True)
        outputs.append(out.read_bytes())
    if outputs[0] != outputs[1]: mismatches.append(name)
print('fixtures',len(manifest),'mismatches',len(mismatches))
for name in mismatches: print(name)
sys.exit(bool(mismatches))
