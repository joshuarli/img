# dav1d integration investigation

The integration is not complete. Do not use the parked branch as an identity-qualified decoder.

On macOS arm64 (Apple M1), dav1d 1.5.4 was built using upstream Meson 1.12.1 with `--buildtype=release --default-library=static -Denable_tools=false -Denable_tests=true -Denable_examples=false`. The checkasm binary requires `-Dtrim_dsp=false`: with the default it exits zero while reporting that reference functions are unavailable. After reconfiguration, it reported `checkasm: all 2538 tests passed` (exit 0).

The strongest supported diagnosis for the 26 RGB differences is symbol collision in the qualification executable: it links both rav1d and dav1d, which export identical C API names and assembly symbol names. An instrumented combined executable reported `linked dav1d_version=966d63c1`, rav1d's version string, instead of 1.5.5; its `_dav1d_open` also has a different implementation from the standalone dav1d executable. The combined executable differed from the standalone candidate on 26 of 31 fixtures. A standalone candidate without rav1d matched both the stored reference and a freshly rebuilt current rav1d RGB baseline on all 31. This establishes that the qualification executable does not isolate the two implementations. It does not identify which individual collision accounts for each changed pixel.

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

An initial prefixing experiment failed to compile: macro-generated DSP names such as `dav1d_film_grain_dsp_init_16bpc` were not consistently renamed. It was removed, and no production integration is retained. Remaining work: isolate all public and private native symbols, build a header-backed ownership-safe shim, prove the integrated raw and RGB paths against both independent references, and run `cargo test --test avif_codec`. The exact collision responsible for the differing pixels remains unverified; a version-string collision alone cannot identify the called decoder for every operation. No gated timing benchmark was run.
