# dav1d integration investigation

img uses dav1d 1.5.4, commit `191bdda98ec3c68137754dc97da1db34043d7cd4` (BSD-2-Clause), through the `cc` build and header-backed shim in `codecs/dav1d-sys`. Production does not link rav1d. The shim keeps the picture alive until conversion finishes and initializes row padding before it is borrowed by Rust. The decoder uses one thread, one-frame delay, film grain, and the same frame-size limit as the former rav1d path.

On macOS arm64 (Apple M1), dav1d 1.5.4 was built using upstream Meson 1.12.1 with `--buildtype=release --default-library=static -Denable_tools=false -Denable_tests=true -Denable_examples=false`. The checkasm binary requires `-Dtrim_dsp=false`: with the default it exits zero while reporting that reference functions are unavailable. After reconfiguration, it reported `checkasm: all 2538 tests passed` (exit 0).

The cause of the 26 RGB differences is symbol collision in the qualification executable: it links both rav1d and dav1d, which export identical C API names and assembly symbol names. An instrumented combined executable reported `linked dav1d_version=966d63c1`, rav1d's version string, instead of 1.5.5; its `_dav1d_open` also has a different implementation from the standalone dav1d executable. The combined executable differed from the standalone candidate on 26 of 31 fixtures. A standalone candidate without rav1d matched both the stored reference and a freshly rebuilt current rav1d RGB baseline on all 31. This establishes that the qualification executable does not isolate the two implementations. It does not identify which individual collision accounts for each changed pixel.

The raw-plane experiment decoded primary OBUs in a C executable using header-defined dav1d types, copied each visible plane row while the picture was alive, and set `n_threads=1`, `max_frame_delay=1`, `apply_grain=1`. The 1.5.4 upstream scalar build and its known-good upstream assembly build matched on 31/31; a 1.5.4 build using the parked cc recipe matched the known-good build on 31/31 after removing `lf_mask.S`, which does not exist in 1.5.4. The unmodified parked 1.5.5 cc recipe also matched on 31/31. The C harness avoids the parked Rust FFI and does not itself validate that FFI. The 1.5.4 cc config differs from Meson in `HAVE_ALIGNED_ALLOC` (0 versus 1); both have `HAVE_ASM=1`, 8- and 16-bit templates, `ARCH_AARCH64=1`, `HAVE_AS_FUNC=0`, `PIC=3`, and the same default private assembly prefix. The ARM assembly source files do not reference `BITDEPTH`; their 8- and 16-bit implementations are separate files. A C `sizeof`/`offsetof` probe against 1.5.5 and an equivalent Rust probe of the parked definitions both printed `settings=96 data=72 picture=272 picture.data=16 picture.p=56 picture.ref=256 sequence.still=356`; this checks only those members. These comparisons do not establish safety under coexistence with another dav1d ABI.

The following commands were run from the worktree root. The source copy and all build products are disposable `.work` scratch; the source and fixtures came from `/Users/josh/d/img/.work/bench/decode/avif` and the cc recipe from `git show perf/dav1d-decode:codecs/dav1d-sys/build.rs`. `/opt/homebrew/bin/meson` and `/opt/homebrew/bin/ninja` are the upstream implementations; the first attempt with the `~/.local/bin` replacements omitted required source include paths and failed.

```sh
mkdir -p .work/tmp .work/dav1d
cp -R /Users/josh/d/img/.work/bench/decode/avif/src/dav1d-1.5.4 .work/dav1d/src
TMPDIR="$PWD/.work/tmp" /opt/homebrew/bin/meson setup .work/dav1d/upstream .work/dav1d/src --buildtype=release --default-library=static -Denable_tools=false -Denable_tests=true -Denable_examples=false
TMPDIR="$PWD/.work/tmp" /opt/homebrew/bin/meson configure .work/dav1d/upstream -Dtrim_dsp=false
TMPDIR="$PWD/.work/tmp" /opt/homebrew/bin/ninja -C .work/dav1d/upstream -j4 tests/checkasm
TMPDIR="$PWD/.work/tmp" .work/dav1d/upstream/tests/checkasm
# checkasm: all 2538 tests passed; exit 0
```

The raw C harness and comparison loop are now in `benchmarks/codecs/avif/dav1d-raw-identity.c` and `dav1d-raw-identity.py`. The original experiments used equivalent scratch copies; the following commands are the exact original invocations. A tracked comparison can be repeated after the Meson builds above with `cc -O2 -I .work/dav1d/src/include benchmarks/codecs/avif/dav1d-raw-identity.c /Users/josh/d/img/.work/bench/decode/avif/install/dav1d/lib/libdav1d.a -o .work/dav1d/raw-known` and `python3 benchmarks/codecs/avif/dav1d-raw-identity.py /Users/josh/d/img/.work/bench/decode/avif/fixtures .work/dav1d/raw-known .work/dav1d/raw-cc155` (31 matches, exit 0). The Rust qualification setup, cc builds, and symbol probe remain scratch and are not yet fully reproducible from tracked files:

```sh
python3 .work/dav1d/compare.py .work/dav1d/raw-known .work/dav1d/raw-scalar
# fixtures 31 mismatches 0; exit 0
python3 .work/dav1d/compare.py .work/dav1d/raw-known .work/dav1d/raw-cc
# fixtures 31 mismatches 0; exit 0
python3 .work/dav1d/compare.py .work/dav1d/raw-known .work/dav1d/raw-cc155
# fixtures 31 mismatches 0; exit 0
python3 .work/dav1d/compare.py .work/dav1d/raw-known /Users/josh/d/img/.work/bench/decode/avif/img-target/release/avif-qual-img
# fixtures 31 mismatches 0; exit 0
python3 benchmarks/codecs/avif/decode-identity.py /Users/josh/d/img/.work/bench/decode/avif/fixtures .work/dav1d/qual-target/release/avif-qual-img .work/dav1d/qual-target/release/avif-qual-solo .work/dav1d
# fixtures 31 mismatches 26; exit 1
python3 benchmarks/codecs/avif/decode-identity.py /Users/josh/d/img/.work/bench/decode/avif/fixtures .work/dav1d/base-target/release/avif-qual-img .work/dav1d/qual-target/release/avif-qual-solo .work/dav1d
# fixtures 31 mismatches 0; exit 0
```

An initial prefixing experiment failed to compile on macro-generated DSP names and was removed. Symbol prefixing is unnecessary in the production dependency graph, which contains only dav1d. The independent system-dav1d reference in `avif_codec` runs in a separate process. The exact individual collision responsible for each differing pixel is not known. No gated timing benchmark was run.

The cc build uses ARM64 assembly, both bitdepth templates, `HAVE_ASM=1`, `ARCH_AARCH64=1`, `HAVE_AS_FUNC=0`, `PIC=3`, and `HAVE_ALIGNED_ALLOC=1`, matching Meson on this host. Its 1.5.4 list omits `lf_mask.S` (not present in that release). Upstream source is unmodified.

Build the two qualification binaries in separate processes and compare them. The baseline script extracts source from the pinned pre-integration revision and links only rav1d. The candidate script links only the integrated decoder:

```sh
TMPDIR="$PWD/.work/tmp" CARGO_BUILD_JOBS=4 benchmarks/codecs/avif/decode-build-rav1d-baseline.sh
# exit 0
TMPDIR="$PWD/.work/tmp" CARGO_NET_OFFLINE=true IMG_ONLY=1 AVIF_SCRATCH="$PWD/.work/dav1d/integrated" sh benchmarks/codecs/avif/decode-build.sh
# exit 0
```

With the fixture and Meson reference binaries prepared under `/Users/josh/d/img/.work/bench/decode/avif` as described above, these comparisons passed:

```sh
python3 benchmarks/codecs/avif/decode-identity.py /Users/josh/d/img/.work/bench/decode/avif/fixtures .work/dav1d/rav1d-baseline/target/release/avif-qual-img .work/dav1d/integrated/img-target/release/avif-qual-img .work/dav1d
# fixtures 31 mismatches 0; exit 0; RGB vs rebuilt rav1d baseline
python3 benchmarks/codecs/avif/decode-identity.py /Users/josh/d/img/.work/bench/decode/avif/fixtures /Users/josh/d/img/.work/bench/decode/avif/qual-dav1d-target/release/avif-qual-dav1d .work/dav1d/integrated/img-target/release/avif-qual-img .work/dav1d
# fixtures 31 mismatches 0; exit 0; RGB vs Meson dav1d
python3 benchmarks/codecs/avif/dav1d-raw-identity.py /Users/josh/d/img/.work/bench/decode/avif/fixtures .work/dav1d/raw-known .work/dav1d/integrated/img-target/release/avif-qual-img
# fixtures 31 mismatches 0; exit 0; raw planes vs Meson dav1d
python3 benchmarks/codecs/avif/dav1d-raw-identity.py /Users/josh/d/img/.work/bench/decode/avif/fixtures .work/dav1d/rav1d-baseline/target/release/avif-qual-img .work/dav1d/integrated/img-target/release/avif-qual-img
# fixtures 31 mismatches 0; exit 0; raw planes vs rav1d
```

The separate qualification manifest is generated by `benchmarks/codecs/avif/decode-build.sh`; the rav1d baseline is built from the unchanged pre-integration source. `cargo tree --offline -i rav1d -e normal` exits 101 because the package is absent from the dependency graph.

```sh
TMPDIR="$PWD/.work/tmp" CARGO_BUILD_JOBS=4 cargo test --offline --test avif_codec
# test result: ok. 15 passed; exit 0
TMPDIR="$PWD/.work/tmp" CARGO_BUILD_JOBS=4 cargo test --offline --test codec_boundary
# test result: ok. 8 passed; exit 0
```

`otool -L` on the AVIF test executable showed no `libdav1d` dylib; `nm` showed its linked `_dav1d_open` and `_img_av1_decode`. The separately compiled `test-native` helper linked `/opt/homebrew/opt/dav1d/lib/libdav1d.7.dylib`.

## Linux builds (emulated, compatibility only)

`codecs/dav1d-sys/build.rs` mirrors meson's Linux configuration: `_GNU_SOURCE`, no
`.func` directives, no `PREFIX` in `config.asm` (x86inc tests `%ifdef PREFIX`, so
defining it as 0 still prefixed every x86 symbol with `_`), and meson's full x86
assembly list, including `itx16_avx2` and `itx16_sse`. Both containers build from the
repository `Dockerfile` (Alpine, musl, clang, mold, nasm):

```sh
docker build -q -t img-ci:amd64 --platform linux/amd64 .
docker run --rm --platform linux/amd64 -v "$PWD":/img -e CARGO_TARGET_DIR=/img/.work/tmp/docker-target-amd64 \
  -e TMPDIR=/img/.work/tmp img-ci:amd64 cargo test --locked --test avif_codec
# test result: ok. 15 passed (2026-11-20, emulated on Apple M1)
```

The same command with `linux/arm64` and `img-ci:arm64` also passes its 15 tests. Under emulation
these runs show only that the code builds and decodes correctly. Neither runs checkasm or measures
timing, so performance on native Linux, and its SIMD dispatch, remain unmeasured.
