#!/usr/bin/env python3
import json, pathlib, subprocess, sys
root=pathlib.Path(__file__).resolve().parents[3]
scratch=pathlib.Path(sys.argv[1]); inputs=pathlib.Path(sys.argv[2]); corpus=pathlib.Path(sys.argv[3]); out=scratch/'decode-fixtures'; out.mkdir(parents=True,exist_ok=True)
img=scratch/'img-target/release/img'; manifest=[]
for f in json.loads((inputs/'manifest.json').read_text()):
 if f['channels'] != 3: continue
 stem=pathlib.Path(f['input']).stem
 for cls in ('img444','aom420'):
  name=cls+'-'+stem; dest=out/(name+'.avif')
  if not dest.exists():
   if cls=='img444': cmd=[str(img),'convert',str(inputs/f['input']),'-o',str(dest),'--format','avif','--quality','75','--effort','fast']
   else: cmd=[str(scratch/'avifcodec'),'encode',str(inputs/(stem+'.pnm')),str(dest),str(f['width']),str(f['height']),'420','8','75','1']
   subprocess.run(cmd,check=True)
  manifest.append(dict(name=name,classes=[cls],source=f['input'],width=f['width'],height=f['height']))
sources={f['file']:f for f in json.loads((root/'benchmarks/corpus/qualification/manifest.json').read_text())['sources']}
for source, quality in (('photo-01.jpg', 75), ('photo-01.jpg', 85), ('photo-01.jpg', 65)):
 f=sources[source]
 if f['split'] != 'dev' or f['width']*f['height'] < 4000000: raise ValueError(source)
 name='large-'+source.removesuffix('.jpg')+'-q'+str(quality); dest=out/(name+'.avif')
 if not dest.exists(): subprocess.run([str(img),'convert',str(corpus/source),'-o',str(dest),'--format','avif','--quality',str(quality),'--effort','fast','--strip-metadata'],check=True)
 manifest.append(dict(name=name,classes=['img444','large'],source=source))
for f in manifest:
 subprocess.run([sys.executable,str(root/'benchmarks/codecs/avif/extract-obu.py'),str(out/(f['name']+'.avif')),str(out/(f['name']+'.obu'))],check=True)
(out/'manifest.json').write_text(json.dumps(manifest,indent=1)+'\n')
print('prepared',len(manifest),'fixtures')
