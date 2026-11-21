#!/usr/bin/env python3
# /// script
# requires-python = ">=3.15"
# dependencies = [
#     "laputa-buildlib==0.2.0",
# ]
#
# [[tool.uv.index]]
# name = "stuff"
# url = "https://pypi.17166969.xyz/current/simple/"
#
# [tool.uv.sources]
# laputa-buildlib = { index = "stuff" }
# ///
import importlib
import os
import shutil
import subprocess
import sys
from importlib.metadata import version
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUILDLIB_VERSION = '0.2.0'
BOOTSTRAPPED_ENV = 'PACKAGE_RELEASE_BOOTSTRAPPED'
ARTIFACTS = {
    'img-aarch64-apple-darwin': 'img-aarch64-apple-darwin',
    'img-aarch64-unknown-linux-musl': 'img-aarch64-unknown-linux-musl-static',
    'img-x86_64-unknown-linux-musl': 'img-x86_64-unknown-linux-musl-static',
    'img-source': 'img-source.tar.xz',
}
NOTICES = [
    'LICENSE-MIT', 'LICENSE-APACHE', 'docs/PROVENANCE.md',
    'vendor/oxipng/LICENSE', 'vendor/libdeflater/LICENSE', 'vendor/libdeflate-sys/LICENSE',
    'vendor/libdeflate-sys/libdeflate/COPYING',
    'vendor/libaom/LICENSE', 'vendor/libaom/PATENTS', 'vendor/libavif/LICENSE',
    'vendor/dav1d/COPYING', 'vendor/dav1d/doc/PATENTS',
    'vendor/libaom/third_party/fastfeat/LICENSE',
    'vendor/libaom/third_party/vector/LICENSE', 'vendor/libaom/third_party/x86inc/LICENSE',
    'vendor/libjxl/LICENSE', 'vendor/libjxl/PATENTS', 'vendor/highway/LICENSE',
    'vendor/libjxl/third_party/libjpeg-turbo/LICENSE.md', 'vendor/libjxl/third_party/libjpeg-turbo/README.ijg',
    'vendor/libjpeg-turbo/LICENSE.md', 'vendor/libjpeg-turbo/README.ijg',
    'vendor/fast-ssim2/LICENSE', 'vendor/fast-ssim2/LICENSE.arm', 'vendor/fast-ssim2/LICENSE.libjxl',
]


def ensure_buildlib() -> None:
    try:
        importlib.import_module('buildlib')
        return
    except ModuleNotFoundError:
        pass
    shim = ROOT / 'bootstrap/buildlib-run'
    if not shim.is_file():
        raise SystemExit(f'laputa-buildlib is not importable and {shim} is missing')
    if os.environ.get(BOOTSTRAPPED_ENV):
        raise SystemExit('laputa-buildlib is still not importable after bootstrap/buildlib-run')
    os.environ[BOOTSTRAPPED_ENV] = '1'
    os.execv(str(shim), [str(shim), '--', str(Path(__file__).resolve()), *sys.argv[1:]])


def require_pinned_buildlib() -> None:
    found = version('laputa-buildlib')
    if found != BUILDLIB_VERSION:
        raise SystemExit(f'laputa-buildlib {found} imported, expected {BUILDLIB_VERSION}')


def require_release_targets() -> None:
    from buildlib import targets

    release_targets = {name for name in ARTIFACTS if name != 'img-source'}
    expected_targets = {f'img-{triple}' for triple in targets.TRIPLES}
    if release_targets != expected_targets:
        raise SystemExit(
            f'release targets {sorted(release_targets)} do not match laputa-buildlib {sorted(expected_targets)}'
        )


def package(dist: Path) -> None:
    from buildlib import release

    if {path.name for path in dist.iterdir()} != set(ARTIFACTS):
        raise SystemExit('release must contain exactly the three binary artifacts and corresponding source')
    for directory, filename in ARTIFACTS.items():
        members = list((dist / directory).iterdir())
        if len(members) != 1 or members[0].name != filename or not members[0].is_file() or members[0].is_symlink():
            raise SystemExit(f'unexpected artifact contents: {directory}')

    packages = dist / 'packages'
    packages.mkdir()
    for directory, filename in ARTIFACTS.items():
        source = dist / directory / filename
        if directory == 'img-source':
            shutil.copyfile(source, packages / filename)
            continue
        source.chmod(0o755)
        subprocess.run([
            'tar', '--sort=name', '--mtime=@0', '--owner=0', '--group=0', '--numeric-owner',
            '-cJf', str((packages / f'{filename}.tar.xz').resolve()),
            '-C', str(ROOT), *sorted(NOTICES), '-C', str(source.parent.resolve()), filename,
        ], check=True)
    release.write_sha256sums(packages)


def main(argv: list[str]) -> int:
    ensure_buildlib()
    require_pinned_buildlib()
    require_release_targets()
    package(Path(argv[1]))
    return 0


if __name__ == '__main__':
    raise SystemExit(main(sys.argv))
