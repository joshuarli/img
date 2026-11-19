#!/usr/bin/env python3
import hashlib
import shutil
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parent.parent
dist = Path(sys.argv[1])
artifacts = {
    'img-aarch64-apple-darwin': 'img-aarch64-apple-darwin',
    'img-aarch64-unknown-linux-musl': 'img-aarch64-unknown-linux-musl-static',
    'img-x86_64-unknown-linux-musl': 'img-x86_64-unknown-linux-musl-static',
    'img-source': 'img-source.tar.xz',
}
if {path.name for path in dist.iterdir()} != set(artifacts):
    raise SystemExit('release must contain exactly the three binary artifacts and corresponding source')
for directory, filename in artifacts.items():
    members = list((dist / directory).iterdir())
    if len(members) != 1 or members[0].name != filename or not members[0].is_file() or members[0].is_symlink():
        raise SystemExit(f'unexpected artifact contents: {directory}')

packages = dist / 'packages'
packages.mkdir()
notices = [
    'LICENSE-MIT', 'LICENSE-APACHE', 'docs/PROVENANCE.md',
    'vendor/oxipng/LICENSE', 'vendor/libdeflater/LICENSE', 'vendor/libdeflate-sys/LICENSE',
    'vendor/libdeflate-sys/libdeflate/COPYING',
    'vendor/libaom/LICENSE', 'vendor/libaom/PATENTS', 'vendor/libavif/LICENSE',
    'vendor/rav1d/COPYING',
    'vendor/libaom/third_party/fastfeat/LICENSE',
    'vendor/libaom/third_party/vector/LICENSE', 'vendor/libaom/third_party/x86inc/LICENSE',
    'vendor/libjxl/LICENSE', 'vendor/libjxl/PATENTS', 'vendor/highway/LICENSE',
    'vendor/libjxl/third_party/libjpeg-turbo/LICENSE.md', 'vendor/libjxl/third_party/libjpeg-turbo/README.ijg',
    'vendor/libjpeg-turbo/LICENSE.md', 'vendor/libjpeg-turbo/README.ijg',
]
for directory, filename in artifacts.items():
    source = dist / directory / filename
    if directory == 'img-source':
        shutil.copyfile(source, packages / filename)
        continue
    source.chmod(0o755)
    subprocess.run([
        'tar', '--sort=name', '--mtime=@0', '--owner=0', '--group=0', '--numeric-owner',
        '-cJf', str((packages / f'{filename}.tar.xz').resolve()),
        '-C', str(root), *sorted(notices), '-C', str(source.parent.resolve()), filename,
    ], check=True)
with (packages / 'SHA256SUMS').open('w') as manifest:
    for path in sorted(packages.iterdir()):
        if path.name == 'SHA256SUMS':
            continue
        with path.open('rb') as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        manifest.write(f'{digest}  {path.name}\n')
