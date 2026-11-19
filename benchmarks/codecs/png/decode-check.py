import hashlib, json, pathlib, struct, subprocess, sys, zlib
fixtures = pathlib.Path(sys.argv[1]); binary = pathlib.Path(sys.argv[2]).resolve()
manifest = json.loads((fixtures/'manifest.json').read_text())
modes = ['png','zen','spng','libpng']
counts = {mode: {'fixtures':0,'mismatches':0,'errors':0} for mode in modes}
for item in manifest:
    path = fixtures/item['file']; outputs = {}
    for mode in modes:
        p = subprocess.run([str(binary),mode+'-dump',str(path)],capture_output=True)
        if p.returncode: counts[mode]['errors'] += 1; print('ERROR',mode,path.name,p.stderr.decode(errors='replace')[:300],flush=True)
        else: outputs[mode] = hashlib.sha256(p.stdout).digest(); counts[mode]['fixtures'] += 1
    if 'png' in outputs:
        for mode in modes[1:]:
            if mode in outputs and outputs[mode] != outputs['png']:
                counts[mode]['mismatches'] += 1; print('MISMATCH',mode,path.name,flush=True)
for mode in modes: print(mode,counts[mode],flush=True)
if any(v['errors'] or v['mismatches'] for v in counts.values()): sys.exit(1)
source = (fixtures/manifest[0]['file']).read_bytes()
variants = [source[:n] for n in (8, 20, len(source)//2, len(source)-1)]
mutated = bytearray(source); mutated[-5] ^= 1; variants.append(mutated)
pos=8
while pos < len(source):
    n = int.from_bytes(source[pos:pos+4],'big'); kind=source[pos+4:pos+8]
    if kind == b'IDAT' and n > 4:
        mutated=bytearray(source); mutated[pos+8] ^= 0xff
        mutated[pos+8+n:pos+12+n]=struct.pack('>I',zlib.crc32(mutated[pos+4:pos+8+n])); variants.append(mutated); break
    pos += n+12
bad = fixtures.parent/'malformed.png'
for mode in modes:
    accepted = 0
    for data in variants:
        bad.write_bytes(data)
        accepted += subprocess.run([str(binary),mode+'-dump',str(bad)],capture_output=True).returncode == 0
    print(mode,'malformed',len(variants),'accepted',accepted,flush=True)
    if accepted: sys.exit(1)
