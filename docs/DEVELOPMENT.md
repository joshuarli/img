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
