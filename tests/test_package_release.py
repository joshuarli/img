import hashlib
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent
BINARIES = (
    'img-aarch64-apple-darwin',
    'img-aarch64-unknown-linux-musl-static',
    'img-x86_64-unknown-linux-musl-static',
)
NOTICES = (
    'LICENSE-MIT', 'LICENSE-APACHE', 'docs/PROVENANCE.md',
    'vendor/oxipng/LICENSE', 'vendor/libdeflater/LICENSE', 'vendor/libdeflate-sys/LICENSE',
    'vendor/libdeflate-sys/libdeflate/COPYING',
    'vendor/libaom/LICENSE', 'vendor/libaom/PATENTS', 'vendor/libavif/LICENSE',
    'vendor/dav1d/COPYING',
    'vendor/libaom/third_party/fastfeat/LICENSE',
    'vendor/libaom/third_party/vector/LICENSE', 'vendor/libaom/third_party/x86inc/LICENSE',
    'vendor/libjxl/LICENSE', 'vendor/libjxl/PATENTS', 'vendor/highway/LICENSE',
    'vendor/libjxl/third_party/libjpeg-turbo/LICENSE.md', 'vendor/libjxl/third_party/libjpeg-turbo/README.ijg',
    'vendor/libjpeg-turbo/LICENSE.md', 'vendor/libjpeg-turbo/README.ijg',
)


class PackageReleaseTest(unittest.TestCase):
    def test_archives_preserve_layout_and_are_deterministic(self):
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            outputs = []
            for run in ('first', 'second'):
                dist = base / run
                dist.mkdir()
                for name in BINARIES:
                    directory = dist / name.removesuffix('-static')
                    directory.mkdir()
                    (directory / name).write_bytes(b'binary fixture')
                source = dist / 'img-source'
                source.mkdir()
                (source / 'img-source.tar.xz').write_bytes(b'source fixture')
                subprocess.run(['python3', str(ROOT / 'scripts/package-release.py'), str(dist)], check=True)
                packages = dist / 'packages'
                expected = {f'{name}.tar.xz' for name in BINARIES} | {'img-source.tar.xz', 'SHA256SUMS'}
                self.assertEqual({path.name for path in packages.iterdir()}, expected)
                for name in BINARIES:
                    archive = packages / f'{name}.tar.xz'
                    with tarfile.open(archive, 'r:xz') as tar:
                        members = tar.getmembers()
                        names = [member.name for member in members]
                        self.assertEqual(names, sorted(NOTICES) + [name])
                        for member in members:
                            self.assertEqual((member.uid, member.gid, member.mtime), (0, 0, 0))
                        self.assertEqual(tar.getmember(name).mode, 0o755)
                    subprocess.run(['tar', '-tJvf', str(archive)], check=True)
                for line in (packages / 'SHA256SUMS').read_text().splitlines():
                    digest, name = line.split('  ')
                    self.assertEqual(digest, hashlib.sha256((packages / name).read_bytes()).hexdigest())
                outputs.append({path.name: path.read_bytes() for path in packages.iterdir()})
            self.assertEqual(outputs[0], outputs[1])


if __name__ == '__main__':
    unittest.main()
