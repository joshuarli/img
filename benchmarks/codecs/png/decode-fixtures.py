import hashlib, json, pathlib, shutil, struct, subprocess, sys, urllib.request, zlib
root = pathlib.Path(__file__).resolve().parents[3]
out = root / '.work/tmp/png-qualify/fixtures'
out.mkdir(parents=True, exist_ok=True)
binary = root / '.work/tmp/png-qualify/target/release/png-decode-loop'
img = root / 'target/release/img'
subprocess.run(['python3', str(root / 'benchmarks/codecs/png/decode-exact-fixtures.py')], cwd=root, check=True)
sources = sorted(pathlib.Path('/Users/josh/d/img/.work/tmp/pngdev').glob('*.png'))
have = {p.stem for p in sources}
entries = json.loads((root/'benchmarks/corpus/qualification/manifest.json').read_text())['sources']
originals = out.parent/'corpus'
originals.mkdir(exist_ok=True)
converted = out.parent/'dev-converted'
converted.mkdir(exist_ok=True)
for entry in entries:
    if entry['split'] != 'dev' or pathlib.Path(entry['file']).stem in have: continue
    src = originals/entry['file']
    if not src.exists(): src.write_bytes(urllib.request.urlopen(urllib.request.Request(entry['source'], headers={'User-Agent':'img-benchmark-corpus/1.0'})).read())
    if hashlib.sha256(src.read_bytes()).hexdigest() != entry['sha256']: raise ValueError(f"source hash mismatch: {src}")
    dest = converted/(src.stem+'.png')
    if not dest.exists():
        result = subprocess.run([str(img),'convert',str(src),'-o',str(dest),'--format','png','--lossless','--effort','fast'])
        if result.returncode:
            if src.name not in {'photo-06.jpg','photo-07.jpg','photo-09.jpg','photo-12.jpg','photo-13.jpg'}: raise RuntimeError(f'conversion failed: {src}')
            print('unsupported dev original',src,flush=True); continue
    sources.append(dest)
sources += sorted((root/'benchmarks/corpus').glob('*.png'))
sources += sorted((root/'benchmarks/corpus/generated').glob('*.png'))
sources += sorted((root/'.work/tmp/png-exact/fixtures').glob('*.png'))
manifest = []
def add(src, name, kind):
    dest = out/name
    if src != dest: shutil.copyfile(src, dest)
    result = subprocess.run([str(binary), 'png-dump', str(dest)], capture_output=True, check=True)
    size = len(result.stdout)
    classes = [kind]
    if size < 65536: classes.append('small')
    if size >= 4*1024*1024: classes.append('large')
    manifest.append(dict(file=name, classes=classes, raw_bytes=size, sha256=hashlib.sha256(dest.read_bytes()).hexdigest()))
dev_jpegs = {pathlib.Path(e['file']).stem for e in entries if e['split'] == 'dev' and e['format'] == 'jpeg'}
for i, src in enumerate(sources):
    kind = 'img-fast' if src.parent in (converted, pathlib.Path('/Users/josh/d/img/.work/tmp/pngdev')) and src.stem in dev_jpegs else 'foreign'
    add(src, f'input-{i:03}-{src.name}', kind)
for i, name in enumerate(['coffee.png','astronaut.png']):
    src = root/'benchmarks/corpus'/name
    original = src.read_bytes(); width, height = struct.unpack('>II', original[16:24])
    raw = subprocess.run([str(binary),'png-dump',str(src)],capture_output=True,check=True).stdout
    channels = len(raw)//(width*height)
    if channels not in (3,4): raise ValueError('expected 8-bit RGB or RGBA')
    def chunk(kind, payload): return struct.pack('>I',len(payload))+kind+payload+struct.pack('>I',zlib.crc32(kind+payload))
    rows = b''.join(b'\0'+raw[y*width*channels:(y+1)*width*channels] for y in range(height))
    dest = out/f'zlib-{i}.png'
    dest.write_bytes(b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,8,2 if channels==3 else 6,0,0,0))+chunk(b'IDAT',zlib.compress(rows))+chunk(b'IEND',b''))
    add(dest, dest.name, 'foreign')
    foreign = dest
    for effort in ('fast','best'):
        dest = out/f'img-{effort}-{i}.png'
        if not dest.exists(): subprocess.run([str(img),'convert',str(foreign),'-o',str(dest),'--format','png','--lossless','--effort',effort], check=True)
        add(dest, dest.name, 'img-'+effort)
(out/'manifest.json').write_text(json.dumps(manifest,indent=1)+'\n')
print('fixtures',len(manifest),'classes', {c:sum(c in f['classes'] for f in manifest) for c in ('foreign','img-fast','img-best','small','large')})
