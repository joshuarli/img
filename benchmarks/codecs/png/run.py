#!/usr/bin/env python3
import subprocess, sys, os, json, time, hashlib
from pathlib import Path
root=str(Path(__file__).resolve().parents[3]); refs=os.environ.get('IMG_REFS', root+'/.work/refs')
# spec: label=binary[#effort][^threads], binaries resolved in $IMG_REFS
specs=[]
for a in sys.argv[1].split(','):
    label,rest=a.split('=',1); eff=None; thr=None
    if '^' in rest: rest,thr=rest.split('^')
    if '#' in rest: rest,eff=rest.split('#')
    specs.append(dict(label=label,binary=rest,effort=eff,threads=thr))
rounds=int(sys.argv[2]); out=sys.argv[3]; inputs=[l.strip() if l.strip().startswith('/') else root+'/'+l.strip() for l in open(sys.argv[4]) if l.strip()]
scratch=root+'/.work/tmp/png-bench'; os.makedirs(scratch,exist_ok=True)
STRIP=('coffee.png','astronaut.png')
def run(s,path,r):
    name=os.path.basename(path); dst=f"{scratch}/{s['label']}-{r}-{name}"
    if os.path.exists(dst): os.remove(dst)
    cmd=[refs+'/'+s['binary'],'convert',path,'-o',dst,'--format','png','--lossless','--effort',s['effort'] or 'fast','--quiet']+(['--strip-metadata'] if name in STRIP else [])
    env=dict(os.environ)
    if s['threads']: env['RAYON_NUM_THREADS']=s['threads']
    t=time.perf_counter(); p=subprocess.Popen(cmd,env=env,stderr=subprocess.PIPE,stdout=subprocess.DEVNULL)
    _,status,ru=os.wait4(p.pid,0); wall=time.perf_counter()-t; err=p.stderr.read().decode()
    ok=os.waitstatus_to_exitcode(status)==0 and os.path.exists(dst)
    return dict(tool=s['label'],input=path.replace(root+'/',''),round=r,ok=ok,bytes=os.path.getsize(dst) if ok else None,
        wall=wall,user=ru.ru_utime,sys=ru.ru_stime,rss=ru.ru_maxrss,err=None if ok else err[-300:],
        sha=hashlib.sha256(open(dst,'rb').read()).hexdigest() if ok else None)
rec=[]; discarded=0; r=0
while r<rounds:
    if subprocess.run(['perf-gate-ready'],capture_output=True).returncode: discarded+=1; time.sleep(20); continue
    order=specs if r%2==0 else specs[::-1]
    batch=[run(s,path,r) for path in inputs for s in order]
    if subprocess.run(['perf-gate-ready','-1'],capture_output=True).returncode: discarded+=1; continue
    rec+=batch; r+=1
json.dump(dict(rounds=rounds,discarded_rounds=discarded,specs=specs,binaries={s['binary']:hashlib.sha256(open(refs+'/'+s['binary'],'rb').read()).hexdigest() for s in specs},records=rec),open(out,'w'),indent=1)
