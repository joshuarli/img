# Setting up a development host

This guide prepares a fresh Linux machine to build img and run its checks.
macOS needs the same tools through Homebrew.

## Required: build and test

On Debian, Ubuntu or Alpine:

```sh
git clone git@github.com:joshuarli/img && cd img
./scripts/setup-dev
export PATH="$HOME/.cargo/bin:$PATH"
make lint test
```

`scripts/setup-dev` installs the system packages, rustup, and the pinned
nightly from `rust-toolchain.toml` with `rust-src` and `clippy`. It then
fetches the locked crates. Run it again whenever the pinned toolchain changes.

The system packages on other distributions:

| Need | Debian/Ubuntu package | Used by |
| --- | --- | --- |
| C and C++ compilers, make | `build-essential`, `clang` | jpegli, libjpeg-turbo, libaom, libavif, dav1d, libdeflate |
| `cmake` | `cmake` | libjpeg-turbo, libaom |
| `nasm` | `nasm` | x86-64 assembly in dav1d, libaom, libjpeg-turbo |
| `perl` | `perl` | libaom's build |
| `pkg-config`, system dav1d | `pkg-config`, `libdav1d-dev` | the native AVIF reference helper in `tests/avif_codec.rs` |
| `git`, `python3` (3.11+), GNU `tar` | `git`, `python3` | `tests/test_bench.py`, `tests/test_package_release.py` |

All production codecs are vendored and compiled from source. The system
dav1d serves only as a test oracle. Under Alpine/musl, use the environment
from the `Dockerfile` (`CC=clang`, mold, `-C target-feature=-crt-static`).

On a 4-core host the first `make test` builds everything from cold, which
takes about 40 minutes. `CARGO_BUILD_JOBS=4` bounds memory. `make test` sets
`TMPDIR=.work/tmp`; `.work/` is disposable scratch.

## Release packaging

`scripts/package-release.py` builds the published `.tar.xz` archives and
`SHA256SUMS`. It pins `laputa-buildlib==0.2.0` in its PEP 723 header, so the
ordinary path is:

```sh
TMPDIR=.work/tmp uv run --script scripts/package-release.py dist
```

When uv is unavailable or the distribution interpreter is older than the 3.15
floor buildlib declares, the script re-executes itself through the vendored
`bootstrap/buildlib-run`, which fetches the pinned wheel by SHA-256 and runs it
on the pinned interpreter:

```sh
sh bootstrap/buildlib-run -- scripts/package-release.py dist
BUILDLIB_CACHE=.work/tmp/buildlib-cache sh bootstrap/buildlib-run -- \
  -m unittest tests.test_package_release
```

The bootstrap needs `curl`, CA certificates, `tar`, `xz` and `sha256sum` or
`shasum`. On Linux with musl 1.2.5 or newer (Debian 13, Ubuntu 26.04+ or
Alpine) it takes python-build; glibc hosts without a compatible musl loader
take the pinned uv release and its managed CPython. Apple Silicon needs macOS
27 or newer. Packaging still constructs the archives with GNU `tar`: recorded
archive bytes must not change, so buildlib supplies the `SHA256SUMS` writer and
the supported-target set only. See [buildlib adoption](buildlib-adoption.md)
for the pins, the archive difference and the proof.

## Optional tools

- **kache**: a compiler cache. `make` uses it as `RUSTC_WRAPPER` when it is
  on `PATH` and builds without it otherwise. Release binaries:
  github.com/joshuarli/kache (the `Dockerfile` pins Linux checksums).
- **CI shape**: `docker build -t img-ci . && docker run --rm -v "$PWD":/img img-ci make ci`
  runs the Linux musl CI checks.
- **Benchmarks** (`scripts/bench`, [BENCHMARKS.md](../BENCHMARKS.md)):
  - `perf-gate-ready` on `PATH` (from the owner's dotfiles) gates every timed round.
  - The qualification corpus is materialized with
    `python3 -I scripts/qualification-corpus .work/qcorpus`, or point
    `BENCH_CORPUS` elsewhere. It downloads the originals and checks each SHA-256.
  - `jaq` is needed by a few older `scripts/benchmark-*` helpers.
  - Competitor builds are listed in [benchmarks/competitors.json](../benchmarks/competitors.json).
- **Metric reference check** (`make metric-reference-check`): JPEG XL 0.12.0's
  `ssimulacra2`, through `scripts/metric-oracle` or `SSIMULACRA2_REFERENCE`.

Performance has been measured only on macOS ARM64. Treat the first Linux
timings as a new platform lane, recorded separately ([CODECS.md](CODECS.md)).
