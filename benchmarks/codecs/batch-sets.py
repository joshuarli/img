#!/usr/bin/env python3
"""Materializes the batch qualification sets: python3 -I batch-sets.py CORPUS IMG DEST

mix:   27 development inputs mixing sizes (48 px icons to 14 MP), opaque and nonopaque PNG, JPEG, and AVIF made
       from development images, so optimize --score keeps JPEG, PNG and AVIF outputs in one run. Two large
       nonopaque PNGs come from the corpus generator with seeds no corpus entry uses.
large: the 16 large development JPEGs of the 2026-11-20 compact-scheduler measurement.
fresh: 30 generated opaque RGB images with unused seeds (synthetic, texture, ui-text at the corpus's medium sizes),
       untouched by tuning, for qualifying AVIF search policy changes.
IMG converts the AVIF members (--quality 95); every file's sha256 is printed so runs can record their set."""
import hashlib, importlib.machinery, importlib.util, os, pathlib, shutil, subprocess, sys

corpus, img, dest = map(pathlib.Path, sys.argv[1:4])
loader = importlib.machinery.SourceFileLoader('corpus', str(pathlib.Path(__file__).resolve().parents[2] / 'scripts/qualification-corpus'))
gen = importlib.util.module_from_spec(importlib.util.spec_from_loader('corpus', loader)); loader.exec_module(gen)
MIX_COPY = ['photo-10.jpg', 'photo-04.jpg', 'texture-original-00.jpg', 'ui-text-original-02.jpg', 'grayscale-original-01.jpg', 'photo-01.jpg',
            'photo-21.jpg', 'photo-18.jpg', 'texture-original-04.jpg', 'grayscale-original-07.jpg', 'ui-text-original-05.jpg',
            'texture-24.png', 'ui-text-12.png', 'grayscale-16.png', 'synthetic-09.png', 'synthetic-13.png', 'texture-15.png', 'ui-text-25.png',
            'ui-text-10.png', 'synthetic-12.png', 'synthetic-04.png', 'synthetic-00.png']
MIX_AVIF = ['photo-22.jpg', 'texture-27.png', 'photo-03.jpg']
MIX_ALPHA = [dict(width=2048, height=1536, seed=900, kind='synthetic', bit_depth=8, alpha=True),
             dict(width=3000, height=2000, seed=901, kind='synthetic', bit_depth=8, alpha=True)]
LARGE = ['photo-04.jpg', 'photo-10.jpg', 'photo-15.jpg', 'photo-19.jpg', 'photo-27.jpg', 'photo-28.jpg', 'texture-original-00.jpg',
         'texture-original-01.jpg', 'texture-original-02.jpg', 'texture-original-03.jpg', 'texture-original-05.jpg', 'texture-original-06.jpg',
         'texture-original-07.jpg', 'texture-original-08.jpg', 'texture-original-09.jpg', 'ui-text-original-02.jpg']
SIZES = [(416, 320), (448, 288), (480, 256), (384, 288), (416, 256)]
FRESH = [dict(width=SIZES[i % 5][0], height=SIZES[i % 5][1], seed=1000 + i, kind=('synthetic', 'texture', 'ui-text')[i % 3], bit_depth=8, alpha=False)
         for i in range(30)]
for name in ('mix', 'large', 'fresh'):
    shutil.rmtree(dest / name, ignore_errors=True); (dest / name).mkdir(parents=True)
for f in MIX_COPY:
    shutil.copyfile(corpus / f, dest / 'mix' / f)
for f in MIX_AVIF:
    subprocess.run([str(img), 'convert', str(corpus / f), '-o', str(dest / 'mix' / (pathlib.Path(f).stem + '.avif')), '--quality', '95', '--quiet'], check=True)
for p in MIX_ALPHA:
    (dest / 'mix' / f"generated-alpha-{p['seed']}.png").write_bytes(gen.generate(p))
for f in LARGE:
    shutil.copyfile(corpus / f, dest / 'large' / f)
for p in FRESH:
    (dest / 'fresh' / f"fresh-{p['kind']}-{p['seed']}.png").write_bytes(gen.generate(p))
for path in sorted(dest.glob('*/*')):
    print(hashlib.sha256(path.read_bytes()).hexdigest(), path.relative_to(dest))
