#!/usr/bin/env python3
import hashlib
import shutil
import sys
import zipfile
from pathlib import Path

root = Path(__file__).resolve().parent.parent
dist = Path(sys.argv[1])
artifacts = {
    'rimage-aarch64-apple-darwin': 'rimage-aarch64-apple-darwin',
    'rimage-aarch64-unknown-linux-musl': 'rimage-aarch64-unknown-linux-musl-static',
    'rimage-x86_64-unknown-linux-musl': 'rimage-x86_64-unknown-linux-musl-static',
    'rimage-source': 'rimage-source.tar.gz',
}
if {path.name for path in dist.iterdir()} != set(artifacts):
    raise SystemExit('release must contain exactly the three binary artifacts and corresponding source')
for directory, filename in artifacts.items():
    members = list((dist / directory).iterdir())
    if len(members) != 1 or members[0].name != filename or not members[0].is_file() or members[0].is_symlink():
        raise SystemExit(f'unexpected artifact contents: {directory}')

packages = dist / 'packages'
packages.mkdir()
notices = ['LICENSE-AGPL3', 'LICENSE-MIT', 'LICENSE-APACHE', 'docs/PROVENANCE.md']
for directory, filename in artifacts.items():
    source = dist / directory / filename
    if directory == 'rimage-source':
        shutil.copyfile(source, packages / filename)
        continue
    with zipfile.ZipFile(packages / f'{filename}.zip', 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        archive.write(source, filename)
        for notice in notices:
            archive.write(root / notice, notice)
with (packages / 'SHA256SUMS').open('w') as manifest:
    for path in sorted(packages.iterdir()):
        if path.name == 'SHA256SUMS':
            continue
        with path.open('rb') as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        manifest.write(f'{digest}  {path.name}\n')
