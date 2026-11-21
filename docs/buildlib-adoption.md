# Buildlib adoption: release packaging

This is the BLD-3 record for `img`. It covers `scripts/package-release.py`; it does not
change the release workflow, the published archive names, the archive bytes, the
`SHA256SUMS` format or any public Make target.

## 1. Bound change

- **Base revision.** `815fb7edcbfc7bfce240141acfa346b7556f5a3d`.
- **Entrypoint.** `scripts/package-release.py <dist>`, invoked by
  `.github/workflows/release.yml` as `python3 scripts/package-release.py dist` and by
  `tests/test_package_release.py`. The single positional argument, the exit behaviour and
  the console output are unchanged.
- **Local implementation replaced.** The local `hashlib` `SHA256SUMS` writer (six lines
  that opened every package and formatted `digest  name`) is gone; the script now calls
  `buildlib.release.write_sha256sums`. The artifact map is also checked against
  `buildlib.targets.TRIPLES` so a release target cannot drift from the shared target
  model. The installed buildlib must be the pinned `0.2.0`.
- **Buildlib API.** `buildlib.release.write_sha256sums`, `buildlib.targets.TRIPLES`, the
  `buildlib` package identity and `importlib.metadata.version("laputa-buildlib")`. The
  tests additionally use `buildlib.release.verify_sha256sums` and `ReleaseError`.
- **Supported targets.** The three release triples are unchanged and equal
  `buildlib.targets.TRIPLES`: `aarch64-apple-darwin`, `aarch64-unknown-linux-musl`,
  `x86_64-unknown-linux-musl`. The artifact directories map to the published file names
  locally; `img-source` is not a buildlib triple.
- **Output and error contracts.** Identical. The archives, the `img-source` copy, the
  four `SHA256SUMS` lines and their order are byte-identical (section 4). The three
  existing artifact checks keep their wording; a drifted target set now fails with
  `release targets ... do not match laputa-buildlib ...`.
- **Consumer policy that stays local.** The `artifacts` directory/file map and the
  `img-source` pass-through, the exact notice list and its ordering, the `chmod 0o755`
  on each binary before archiving, and the release-completeness checks (exactly the
  expected directories, one regular non-symlink member each).

### The archive writer is not replaced

`buildlib.release.create_archive` is **not** byte-compatible with the current command,
so the archive construction stays local. The base script runs

```
tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -cJf <archive> \
    -C <root> <sorted notices> -C <binary dir> <binary>
```

`create_archive` writes a different format. Measured on the same staged tree
(25 notices plus one executable member, buildlib 0.2.0 wheel
`aaee620f...`):

| Property | Current GNU `tar` | `buildlib.release.create_archive` |
| --- | --- | --- |
| tar magic at offset 257 | `ustar  \0` (GNU/ustar) | `ustar\x0000` (POSIX PAX) |
| members for the same tree | 26 (exactly the listed files) | 46 (adds one entry per directory) |
| `.tar.xz` SHA-256 | `e469b192...` | `dc98917f...` |
| raw tar size | 163,840 B | 174,080 B |

The difference also survives a flat (no-subdirectory) tree, so it is the tar encoding,
not only the added directory entries. Changing the writer would change every published
archive's bytes and the `SHA256SUMS` a `~/usr` install verifies; the brief's rule against
changing the release format applies, and this record keeps the archive local. Moving it
later needs a byte-level compatibility mode in buildlib or a deliberate format change.

`scripts/verify-release.py` linkage checks stay local: buildlib's `elf`/`macho`
inspection is not part of this slice and `scripts/verify-release.py` is outside it.

## 2. Pins and provenance

Vendored from `laputa-systems/buildlib` at
`30ad4a0a8b697031a008b1de3b5359fc5cf6f2fe` (release 0.2.0), both files byte-identical to
that revision and identical to the `shck` and `node-build` copies:

| File | SHA-256 |
| --- | --- |
| `bootstrap/buildlib-run` | `e7f957ca6af0b151d25a9ff5bedb5deca0bfc86ba05ff3225a5405d56fa95957` |
| `bootstrap/python.lock` | `cab50541ce8b2c2a6a41829f26aac19344532d239535f4d6795d80d5af933832` |

`bootstrap/python.lock` pins the interpreter archives, uv 0.13.7 with Python 3.15.2, and
wheel `laputa_buildlib-0.2.0-py3-none-any.whl` with SHA-256
`aaee620f0478544b1d32075b44de48e69bbcbd8b974d5f599d037494d59f4b4a`.

The PEP 723 header declares `laputa-buildlib==0.2.0` from the stuff index, the same
wheel digest the bootstrap lock pins. `scripts/package-release.py` refuses any other
installed version. Because the release workflow calls a bare `python3` (which on a
minimal Ubuntu 24.04 is absent, and on 24.04+ is older than buildlib's 3.15 floor), the
script imports buildlib and, on `ModuleNotFoundError`, re-executes itself through
`bootstrap/buildlib-run` once. A second failure exits loudly instead of looping.

The ordinary uv path imports the pinned wheel; the environment is keyed by the script
name, so the path does not depend on the wheel cache seeded by the bootstrap. In the
Ubuntu 26.04 capsule, after `uv run --script scripts/package-release.py dist`:

```
buildlib.__file__: /home/runner/.cache/uv/environments-v2/package-release-5c9aee7223b24721/lib/python3.15/site-packages/buildlib/__init__.py
version: 0.2.0
python: 3.15.2
```

The uv-installed `buildlib/` tree is byte-identical to the pinned wheel's `buildlib/`
tree (a `diff -r` differs only by the generated `__pycache__`), and the
bootstrap-cached wheel is itself the pinned wheel (`sha256sum` =
`aaee620f...f4b4a`), also byte-identical when extracted.

```
$ BUILDLIB_CACHE=... sh bootstrap/buildlib-run -- -c 'import buildlib; print(buildlib.__file__)'
.../laputa-buildlib/aaee620f.../buildlib.whl/buildlib/__init__.py
```

Package pin and bootstrap lock name the same wheel digest. The source revision is
recorded separately from the package version. There are no local differences in either
vendored file. Reverting this slice restores the base script, the base tests and drops
`bootstrap/`; every pin and the release format are untouched.

### A uv script lock is not committed

The `shck` and `node-build` adoptions also commit a uv script lock. Here the writable
scope for this lane is `scripts/package-release.lock`, but uv names a script lock
`<script>.py.lock` (`scripts/package-release.py.lock`) and offers no path override
(`UV_LOCKFILE` was ignored, and `uv lock --script` has no `--lockfile`). Writing
`scripts/package-release.py.lock` is outside this lane's writable paths, so the pin is
carried by the PEP 723 header and the vendored bootstrap lock instead. A follow-up that
owns `scripts/package-release.py.lock` can add `uv lock --script scripts/package-release.py`
for an exact, frozen uv environment; the bootstrap already provides the pinned wheel with
size and SHA-256 verification and is what this slice proves.

## 3. Boundary tests

`tests/test_package_release.py` runs the real script boundary. It keeps the existing
archive-layout, determinism, membership and checksum checks and adds tests for the new
wiring (10 tests; `sh bootstrap/buildlib-run -- -m unittest tests.test_package_release`,
`Ran 10 tests ... OK`).

| Test | Behavior |
| --- | --- |
| `test_archives_preserve_layout_and_are_deterministic` | Existing: member names/order, `0/0/0` ownership and `0` mtime, binary mode `0755`, `tar -tJvf` parse, per-file digest, two runs equal. |
| `test_bootstrap_imports_the_pinned_wheel` | `buildlib.__file__` is the pinned wheel path and `version("laputa-buildlib") == "0.2.0"`. |
| `test_release_targets_match_buildlib_triples` | Local release targets equal `buildlib.targets.TRIPLES`. |
| `test_target_parity_guard_rejects_a_drifted_artifact_set` | Injected extra target makes `require_release_targets` exit. |
| `test_pin_guard_rejects_another_installed_version` | Injected `0.1.0` version makes `require_pinned_buildlib` exit. |
| `test_rejects_an_unexpected_artifact_directory` | Extra directory still fails with the original message. |
| `test_rejects_a_misnamed_artifact` | Renamed member still fails. |
| `test_rejects_a_symlinked_artifact` | A symlinked member still fails. |
| `test_rejects_an_extra_artifact_member` | Two members still fail. |
| `test_buildlib_verifier_detects_a_tampered_package` | `verify_sha256sums` accepts a fresh package and rejects a byte-appended archive with `digest mismatch for ...`. |

Injected faults on the replaced paths, run in an Ubuntu 26.04 capsule against the lane
commit (see section 5 for the harness):

| Fault | Observed |
| --- | --- |
| another installed buildlib (`0.1.0`, `0.2.1`) | `laputa-buildlib 0.1.0 imported, expected 0.2.0`, exit 1 / 7 |
| one extra release target | `release targets [...] do not match laputa-buildlib [...]`, exit 1 |
| tampered archive after packaging | `rejected: .../SHA256SUMS: digest mismatch for img-aarch64-apple-darwin.tar.xz` |
| `release.write_sha256sums(packages)` replaced with `pass` | packaging still succeeds but `SHA256SUMS` is absent, so the call is load-bearing; restoring it recreates the file |
| `create_archive` used for the same tree | magic `b'ustar\x0000'`, members 2 for a two-file tree, i.e. not the current format |

## 4. Outcome comparison

Both packagers ran on the same `dist` tree with the **real** pinned macOS host binary
(`openorange-img` build: Mach-O arm64, 4,440,528 B, SHA-256 `11c69751...`), the real 25
notices and an `img-source` fixture, inside `ubuntu:26.04` with GNU tar 1.35. The base
script ran twice and the new script ran twice (the new script through the vendored
bootstrap).

| Measurement | Base `815fb7e` | New |
| --- | --- | --- |
| `img-aarch64-apple-darwin.tar.xz` SHA-256 | `068d8c46b4538c190a245d682923c6bc5ff5452457e5e27e09e534f345625f3e` | same |
| `img-aarch64-unknown-linux-musl-static.tar.xz` SHA-256 | `44a0ddcba3d78b5dd663f5fd4eea003cf9280a503f884c457614dd891e86e79d` | same |
| `img-x86_64-unknown-linux-musl-static.tar.xz` SHA-256 | `ad4941451d72a173ee6f27d68ec6bf9f7573a71243e6d76ce0fecc11a4c0dc2c` | same |
| `img-source.tar.xz` SHA-256 | `85bed9076b168307d27f0445ef97c2cf68605ce7c61a58d9b5957ad8e8c56efd` | same |
| `SHA256SUMS` bytes | `2b12b26fd26be705704d9c11a679d0cf154c49a70ca8578f29c514ab6301937f` | same |
| Second run | identical to its first | identical to its first |
| `sha256sum -c SHA256SUMS` | n/a (format owner) | all four `OK` |

Each artifact compared byte-for-byte with `cmp` and was identical. The second run of
each packager reproduced its own first run exactly. Nothing was published. A fixture-only
run (1,536,960-byte archive) gave the same byte equality, so the result does not depend
on the fixture size.

## 5. Environment

**Host.** macOS 27.2 arm64, Python 3.15.1, uv 0.13.9, Docker 29.8.2 (native arm64).
Cargo trees used `CARGO_BUILD_JOBS=4` and `TMPDIR=.work/tmp`.

| Command | Base `815fb7e` | After |
| --- | --- | --- |
| `TMPDIR=.work/tmp make lint` | exit 0 | exit 0 |
| `TMPDIR=.work/tmp make test` | exit 0 (all suites `ok`) | exit 0 (all suites `ok`) |
| `python3 tests/test_package_release.py` | bsdtar host: fails (`--sort=name` unsupported), skipped by `make test` | same host, same skip |
| `docker run ubuntu:26.04 python3 tests/test_package_release.py` (GNU tar 1.35) | base test `Ran 1 test ... OK` | `Ran 10 tests ... OK` |
| `sh bootstrap/buildlib-run -- -m unittest tests.test_package_release` | not applicable | exit 0, 10 tests |

The packaging test needs GNU `tar`; `make test` guards it with a `tar --version` check
and this host has Apple `bsdtar`, so the target prints the existing skip on both
revisions and `release.yml` runs it where GNU tar exists. `make lint` on this host
reports the repository's pre-existing Clippy warnings without failing, identically at
both revisions.

**Cold capsules.** Each capsule mounted a bare mirror of the lane commit read-only,
cloned it inside as an unprivileged `runner`, installed only
`git ca-certificates curl xz-utils make` (plus `musl` on 26.04), and ran
`make`-equivalent `sh bootstrap/buildlib-run -- -m unittest tests.test_package_release`.
No buildlib cache was seeded; `TMPDIR` stayed at the distribution default.

| Capsule | Bootstrap branch | Tests | `git status --porcelain` | Setup+test exit | Cold wall | Cold downloads |
| --- | --- | --- | --- | --- | --- | --- |
| `ubuntu:24.04`, arm64 native | uv 0.13.7 + uv-managed CPython 3.15.2 | 10 tests, OK | empty | 0 | 3 s | uv archive 15,896,996 B + uv-managed CPython 3.15.2 (30.3 MiB) + wheel 25,193 B |
| `ubuntu:26.04` + `musl` 1.2.5, arm64 native | python-build CPython 3.15.1 | 10 tests, OK | empty | 0 | 3 s | python-build archive 8,425,748 B + wheel 25,193 B |

Both capsules report `musl: none` on 24.04 and `/lib/ld-musl-aarch64.so.1` on 26.04;
`buildlib-run` printed `using uv` and `using python-build` respectively, so the
documented fallback and the musl branch are both exercised. The capsule exit code is the
mapper's, and the underlying `test_exit: 0` is captured in each log. These durations are
operational cold-capsule cost, not benchmarks.

The lane code commit the capsules and the outcome comparison used is `7de6d6e`. The only
later commit in this slice is this documentation, which no gate executes. Nothing was
published. `git status --porcelain` is empty in both clones after the run.

## 6. Benefit and cost

- **Duplicate code removed.** The local `hashlib` `SHA256SUMS` loop is gone. The script
  now writes the manifest through one `buildlib.release.write_sha256sums` call and
  asserts its target set equals `buildlib.targets.TRIPLES`.
- **Shared operations used.** `SHA256SUMS` writing and validation (producer and verifier
  agree), the supported release-target set, and the package/version identity. The pinned
  wheel is exercised through both the uv path and the vendored bootstrap.
- **Correctness gain.** A release target that drifts from the shared model fails before
  any archive is written, a non-0.2.0 buildlib is refused, and a minimal host without
  `python3` still packages through the pinned wheel instead of failing with
  `python3: not found`.
- **Not moved, with reason.** `create_archive` stays local: its bytes differ (section 1)
  and the release format must not change. The archive names, notice list, artifact
  layout and `scripts/verify-release.py` linkage checks stay local for the same reason:
  buildlib models none of that, or it is outside this slice.
- **Cold setup.** 24.04 downloads 16,671,812 B of uv plus its managed CPython 3.15.2
  (reported 30.3 MiB) and the 25,193 B wheel; 26.04 downloads 8,425,748 B of python-build
  and the same wheel. Cold wall was about 3 s for the test step after apt, in both.
- **Warm no-op.** With a warm bootstrap cache, `bootstrap/buildlib-run -- -c pass` and the
  10-test suite take about 1 s. `make test` never fetches or starts Python for the Rust
  suites; only the guarded `tests/test_package_release.py` line is affected, and on a
  GNU-tar host it now runs 10 tests instead of 1.
