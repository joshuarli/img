import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / 'scripts' / 'package-release.py'
SHIM = ROOT / 'bootstrap' / 'buildlib-run'
PINNED_WHEEL = 'aaee620f0478544b1d32075b44de48e69bbcbd8b974d5f599d037494d59f4b4a'
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
    'vendor/dav1d/COPYING', 'vendor/dav1d/doc/PATENTS',
    'vendor/libaom/third_party/fastfeat/LICENSE',
    'vendor/libaom/third_party/vector/LICENSE', 'vendor/libaom/third_party/x86inc/LICENSE',
    'vendor/libjxl/LICENSE', 'vendor/libjxl/PATENTS', 'vendor/highway/LICENSE',
    'vendor/libjxl/third_party/libjpeg-turbo/LICENSE.md', 'vendor/libjxl/third_party/libjpeg-turbo/README.ijg',
    'vendor/libjpeg-turbo/LICENSE.md', 'vendor/libjpeg-turbo/README.ijg',
    'vendor/fast-ssim2/LICENSE', 'vendor/fast-ssim2/LICENSE.arm', 'vendor/fast-ssim2/LICENSE.libjxl',
)


LOAD_MODULE = (
    'import importlib.util, sys\n'
    f'spec = importlib.util.spec_from_file_location("package_release", {str(SCRIPT)!r})\n'
    'module = importlib.util.module_from_spec(spec)\n'
    'sys.modules[spec.name] = module\n'
    'spec.loader.exec_module(module)\n'
)


def buildlib_probe(expression):
    if not SHIM.is_file():
        raise AssertionError(f'{SHIM} is missing')
    completed = subprocess.run(
        ['sh', str(SHIM), '--', '-c', expression],
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise AssertionError(f'buildlib-run failed: {completed.stderr.strip()}')
    return completed.stdout


def module_probe(expression):
    return buildlib_probe(LOAD_MODULE + expression)


def staged_release(destination):
    dist = Path(destination) / 'dist'
    for name in BINARIES:
        directory = dist / name.removesuffix('-static')
        directory.mkdir(parents=True)
        (directory / name).write_bytes(b'binary fixture')
    source = dist / 'img-source'
    source.mkdir()
    (source / 'img-source.tar.xz').write_bytes(b'source fixture')
    return dist


def package(dist, env):
    return subprocess.run(
        [sys.executable, str(SCRIPT), str(dist)],
        capture_output=True,
        text=True,
        check=False,
        env=env,
    )


class PinnedWheelTests(unittest.TestCase):
    def test_bootstrap_imports_the_pinned_wheel(self):
        record = json.loads(buildlib_probe(
            'import json, buildlib; from importlib.metadata import version; '
            'print(json.dumps({"file": buildlib.__file__, "version": version("laputa-buildlib")}))'
        ).splitlines()[-1])
        self.assertEqual(record['version'], '0.2.0')
        self.assertIn(f'/{PINNED_WHEEL}/buildlib.whl/buildlib/__init__.py', record['file'])

    def test_release_targets_match_buildlib_triples(self):
        triples = json.loads(buildlib_probe(
            'import json; from buildlib import targets; print(json.dumps(list(targets.TRIPLES)))'
        ).splitlines()[-1])
        self.assertEqual({f'img-{triple}' for triple in triples}, {
            'img-aarch64-apple-darwin',
            'img-aarch64-unknown-linux-musl',
            'img-x86_64-unknown-linux-musl',
        })

    def test_target_parity_guard_rejects_a_drifted_artifact_set(self):
        completed = subprocess.run(
            ['sh', str(SHIM), '--', '-c', LOAD_MODULE + (
                'module.ARTIFACTS["img-riscv64-unknown-linux-musl"] = "unexpected"\n'
                'module.require_release_targets()'
            )],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn('do not match laputa-buildlib', completed.stderr)

    def test_pin_guard_rejects_another_installed_version(self):
        completed = subprocess.run(
            ['sh', str(SHIM), '--', '-c', LOAD_MODULE + (
                'module.version = lambda name: "0.1.0"\n'
                'module.require_pinned_buildlib()'
            )],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn('laputa-buildlib 0.1.0 imported, expected 0.2.0', completed.stderr)


class PackageReleaseTest(unittest.TestCase):
    def setUp(self):
        self._temp = tempfile.TemporaryDirectory(prefix='package-release-test-')
        self.addCleanup(self._temp.cleanup)
        self.root = Path(self._temp.name)
        self.env = dict(os.environ, TMPDIR=str(self.root / 'tmp'))

    def test_archives_preserve_layout_and_are_deterministic(self):
        outputs = []
        for run in ('first', 'second'):
            dist = staged_release(self.root / run)
            completed = package(dist, self.env)
            self.assertEqual(completed.returncode, 0, completed.stderr)
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
                subprocess.run(['tar', '-tJvf', str(archive)], check=True, capture_output=True)
            for line in (packages / 'SHA256SUMS').read_text().splitlines():
                digest, name = line.split('  ')
                self.assertEqual(digest, hashlib.sha256((packages / name).read_bytes()).hexdigest())
            outputs.append({path.name: path.read_bytes() for path in packages.iterdir()})
        self.assertEqual(outputs[0], outputs[1])

    def test_rejects_an_unexpected_artifact_directory(self):
        dist = staged_release(self.root)
        (dist / 'img-extra').mkdir()
        completed = package(dist, self.env)
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn('release must contain exactly the three binary artifacts and corresponding source', completed.stderr)

    def test_rejects_a_misnamed_artifact(self):
        dist = staged_release(self.root)
        directory = dist / 'img-aarch64-apple-darwin'
        (directory / 'img-aarch64-apple-darwin').rename(directory / 'other')
        completed = package(dist, self.env)
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn('unexpected artifact contents: img-aarch64-apple-darwin', completed.stderr)

    def test_rejects_a_symlinked_artifact(self):
        dist = staged_release(self.root)
        directory = dist / 'img-x86_64-unknown-linux-musl'
        ordinary = directory / 'img-x86_64-unknown-linux-musl-static'
        target = self.root / 'shared-binary'
        ordinary.rename(target)
        ordinary.symlink_to(target)
        completed = package(dist, self.env)
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn('unexpected artifact contents: img-x86_64-unknown-linux-musl', completed.stderr)

    def test_rejects_an_extra_artifact_member(self):
        dist = staged_release(self.root)
        (dist / 'img-source' / 'extra').write_bytes(b'extra')
        completed = package(dist, self.env)
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn('unexpected artifact contents: img-source', completed.stderr)

    def test_buildlib_verifier_detects_a_tampered_package(self):
        dist = staged_release(self.root)
        completed = package(dist, self.env)
        self.assertEqual(completed.returncode, 0, completed.stderr)
        packages = dist / 'packages'
        accepted = module_probe(
            'from pathlib import Path\n'
            f'module.require_pinned_buildlib()\n'
            f'from buildlib.release import verify_sha256sums\n'
            f'verify_sha256sums(Path({str(packages)!r}))\n'
            'print("verified")\n'
        )
        self.assertIn('verified', accepted)
        archive = packages / 'img-aarch64-apple-darwin.tar.xz'
        archive.write_bytes(archive.read_bytes() + b'tampered')
        completed = subprocess.run(
            ['sh', str(SHIM), '--', '-c', (
                f'from pathlib import Path\n'
                f'from buildlib.release import verify_sha256sums, ReleaseError\n'
                f'try:\n'
                f'    verify_sha256sums(Path({str(packages)!r}))\n'
                f'except ReleaseError as error:\n'
                f'    print(f"rejected: {{error}}")\n'
            )],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        self.assertIn('rejected: ', completed.stdout)
        self.assertIn('digest mismatch for img-aarch64-apple-darwin.tar.xz', completed.stdout)


if __name__ == '__main__':
    unittest.main()
