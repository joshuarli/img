#!/usr/bin/env python3
import json, pathlib, subprocess, sys
fixtures, img, dav, zen, scratch = map(pathlib.Path, sys.argv[1:])
manifest=json.loads((fixtures/'manifest.json').read_text()); errors=[]; differences=[]; zen_error=0; zen_max=0; zen_bytes=0; zen_different=0
for f in manifest:
 name=f['name']; outputs=[]
 for binary,mode,ext in ((img,'raw','obu'),(dav,'raw','obu'),(img,'img','avif'),(dav,'rgb','avif'),(zen,'zen','avif')):
  out=scratch/(mode+'-'+binary.name+'.bytes')
  r=subprocess.run([str(binary),mode,'0',str(fixtures/(name+'.'+ext)),str(out)],capture_output=True,text=True)
  if r.returncode: errors.append((name,mode,r.returncode,r.stderr)); break
  outputs.append(out.read_bytes())
 if len(outputs)==5:
  if outputs[0]!=outputs[1]: differences.append((name,'planes'))
  if outputs[2]!=outputs[3]: differences.append((name,'img-dav1d-rgb'))
  if len(outputs[2])!=len(outputs[4]): differences.append((name,'zen-length'))
  else:
   delta=[abs(a-b) for a,b in zip(outputs[2],outputs[4])]; zen_error+=sum(delta); zen_max=max(zen_max,max(delta,default=0)); zen_bytes+=len(delta); zen_different+=int(any(delta))
print('fixtures',len(manifest),'errors',len(errors),'differences',len(differences))
if zen_bytes: print('zen nonidentical',zen_different,'mean absolute error',round(zen_error/zen_bytes,4),'max error',zen_max)
for item in errors+differences: print(item)
if errors or differences: sys.exit(1)
