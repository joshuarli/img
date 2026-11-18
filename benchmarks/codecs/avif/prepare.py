#!/usr/bin/env python3
import hashlib, json, struct, subprocess, zlib
from pathlib import Path
from sweep import root, scratch, paths, group, djpeg

outdir = scratch / 'inputs'
outdir.mkdir(parents=True, exist_ok=True)

def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))

def write_png(path, width, height, channels, pixels):
    assert len(pixels) == width * height * channels
    rows = b''.join(b'\0' + pixels[y * width * channels:(y + 1) * width * channels] for y in range(height))
    header = struct.pack('>IIBBBBB', width, height, 8, 0 if channels == 1 else 2, 0, 0, 0)
    path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', header) + chunk(b'IDAT', zlib.compress(rows, 6)) + chunk(b'IEND', b''))

def read_png(path):
    data = path.read_bytes()
    assert data[:8] == b'\x89PNG\r\n\x1a\n'
    at = 8
    payload = []
    while at < len(data):
        size = int.from_bytes(data[at:at+4], 'big'); kind = data[at+4:at+8]; part = data[at+8:at+8+size]; at += size + 12
        if kind == b'IHDR':
            w, h, depth, color, compression, filtering, interlace = struct.unpack('>IIBBBBB', part)
            if (depth, compression, filtering, interlace) != (8, 0, 0, 0):
                raise ValueError(f'unsupported PNG structure: depth {depth} interlace {interlace}')
            if color in (4, 6):
                raise ValueError('alpha channel')
            if color in (0,):
                raise ValueError('grayscale samples')
            if color != 2:
                raise ValueError(f'non-RGB color type {color}')
            channels = 3
        if kind == b'tRNS':
            raise ValueError('alpha channel')
        if kind == b'IDAT':
            payload.append(part)
        if kind == b'IEND':
            break
    raw = zlib.decompress(b''.join(payload)); stride = w * channels; assert len(raw) == (stride + 1) * h
    pixels = bytearray(); prev = bytearray(stride)
    for y in range(h):
        row = bytearray(raw[y*(stride+1)+1:(y+1)*(stride+1)]); filt = raw[y*(stride+1)]
        for x in range(stride):
            left = row[x-channels] if x >= channels else 0; up = prev[x]; diag = prev[x-channels] if x >= channels else 0
            if filt == 0: predictor = 0
            elif filt == 1: predictor = left
            elif filt == 2: predictor = up
            elif filt == 3: predictor = (left + up) // 2
            elif filt == 4:
                p = left + up - diag; a, b, c = abs(p-left), abs(p-up), abs(p-diag)
                predictor = left if a <= b and a <= c else up if b <= c else diag
            else: raise ValueError('PNG filter')
            row[x] = (row[x] + predictor) & 255
        pixels.extend(row); prev = row
    return w, h, channels, bytes(pixels)

def dimensions_jpeg(path):
    data = path.read_bytes(); at = 2
    while at < len(data):
        assert data[at] == 255
        while data[at] == 255: at += 1
        marker = data[at]; at += 1
        if marker in (1, 216, 217) or 208 <= marker <= 215: continue
        length = int.from_bytes(data[at:at+2], 'big')
        if marker in (192, 193, 194):
            h, w = struct.unpack('>HH', data[at+3:at+7]); return w, h
        at += length
    raise ValueError(f'JPEG dimensions: {path}')

def read_pnm(path):
    data = path.read_bytes(); at = 0; tokens = []
    while len(tokens) < 4:
        while data[at] in b' \t\r\n': at += 1
        if data[at] == 35:
            at = data.index(b'\n', at) + 1; continue
        end = at
        while data[end] not in b' \t\r\n': end += 1
        tokens.append(data[at:end]); at = end
    assert tokens[0] in (b'P5', b'P6') and tokens[3] == b'255'
    at += 1
    w, h = map(int, tokens[1:3]); channels = 1 if tokens[0] == b'P5' else 3
    pixels = data[at:]; assert len(pixels) == w*h*channels
    return w, h, channels, pixels

manifest = []
refusals = []
for index, source in enumerate(paths):
    try:
        scale = None
        if source.suffix == '.jpg':
            w, h = dimensions_jpeg(source)
            scale = max(m for m in range(1, 9) if (max(w, h) * m + 7) // 8 <= 2048)
            pnm = outdir / f'{index:02}.source.pnm'
            subprocess.run([djpeg, '-scale', f'{scale}/8', '-outfile', pnm, source], check=True)
            w, h, channels, pixels = read_pnm(pnm)
            if channels == 1:
                raise ValueError('grayscale samples')
            pnm.unlink()
        else:
            w, h, channels, pixels = read_png(source)
            if max(w, h) > 2048:
                raise ValueError(f'PNG exceeds 2048 px long side ({w}x{h})')
        assert channels == 3
        png = outdir / f'{index:02}-{source.stem}.png'
        pnm = outdir / f'{index:02}-{source.stem}.pnm'
        write_png(png, w, h, channels, pixels)
        pnm.write_bytes((f'P{5 if channels == 1 else 6}\n{w} {h}\n255\n').encode() + pixels)
        raw = png.read_bytes()
        at = 8; kinds = []
        while at < len(raw):
            length = int.from_bytes(raw[at:at+4], 'big')
            kinds.append(raw[at+4:at+8]); at += 12 + length
        assert kinds == [b'IHDR', b'IDAT', b'IEND'] and at == len(raw)
        assert read_png(png) == (w, h, channels, pixels)
        manifest.append({'source': str(source.relative_to(root)), 'input': png.name, 'pnm': pnm.name, 'group': group(source),
                         'width': w, 'height': h, 'channels': channels, 'jpeg_scale': scale,
                         'sha256': hashlib.sha256(raw).hexdigest()})
        print(source.name, w, h, channels, scale, flush=True)
    except ValueError as error:
        refusals.append({'source': str(source.relative_to(root)), 'reason': str(error)})
        print(f'refused {source.name}: {error}', flush=True)

(outdir / 'manifest.json').write_text(json.dumps({'inputs': manifest, 'refusals': refusals}, indent=1) + '\n')
print(f'{len(manifest)} prepared, {len(refusals)} refused')
