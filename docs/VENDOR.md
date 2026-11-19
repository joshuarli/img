# Vendored upstream sources

Production codec dependencies that are vendored live under `vendor/<crate>` and
are wired in through `[patch.crates-io]` in `Cargo.toml`, so the lockfile names
the exact snapshot. Every snapshot is an unmodified copy of a published crate
unless its entry lists local patches. `scripts/check-native-build` compiles the
release binary and fails if any native source outside an allowlisted snapshot
is compiled.

## oxipng 10.2.1

- Role: exact PNG optimization and encoding (`src/still/png.rs`).
- Upstream: <https://github.com/oxipng/oxipng>, commit
  `36f3ef8aac65ecf1761739ea2f2e530281f1312b` (crates.io `oxipng 10.2.1`,
  checksum `52a88e3e5c2ad82e691b219e2b45a90eb5a51d26a434c7e191563e957a48e07a`).
- License: MIT (`vendor/oxipng/LICENSE`).
- Features: `default-features = false`, `parallel`. The CLI (`binary`),
  `zopfli`, `sanity-checks`, `freestanding` and `system-libdeflate` are off.
- Local patches: none.

## libdeflater 1.26.1 and libdeflate-sys 1.26.1

- Role: Rust bindings (libdeflater) and bundled C build (libdeflate-sys) for
  libdeflate, the DEFLATE compressor OxiPNG uses.
- Upstream: <https://github.com/libdeflater/libdeflater>, commit
  `260e1e9a333e32e769855c1ad1f203289e0a3c8d` (crates.io checksums
  `0cbb8285d2e2bd4b8d4155840ae81929354b4ba0d7b3c272c84761ffa2141e1f` and
  `d7870e5fbd2766179a937c725fb11f4ca0ef025d982beb61bd3ce755425bd19c`).
- Bundled C: libdeflate v1.26 from <https://github.com/ebiggers/libdeflate>.
  `vendor/libdeflate-sys/libdeflate/` is byte-identical to the v1.26 release
  tarball (sha256
  `bba03fffc5538576213675ce6968fcff6ce2e67d82e4d5febea2d05f9f13cf85`).
- Licenses: libdeflater/libdeflate-sys Apache-2.0
  (`vendor/libdeflater/LICENSE`, `vendor/libdeflate-sys/LICENSE`);
  libdeflate MIT (`vendor/libdeflate-sys/libdeflate/COPYING`).
- Build: the crate's `build.rs` compiles the eleven libdeflate library
  sources statically with `cc`, including libdeflate's ARM and x86 runtime
  CPU-feature dispatch. The `dynamic` feature is off, so no system libdeflate
  is probed or linked.
- Supported targets: aarch64-apple-darwin, aarch64-unknown-linux-musl,
  x86_64-unknown-linux-musl (static).
- Local patches: none.

## rav1d 1.1.0

- Role: AV1 decode for AVIF (`src/still/av1.rs`), with upstream assembly.
- Upstream: <https://github.com/memorysafety/rav1d>, crates.io `rav1d 1.1.0`,
  checksum `1932f060d5e7bd49dc9f8b272c1dc5e9ce0ffe141c28be900265d3989b36c9ed`.
- License: BSD-2-Clause (`vendor/rav1d/COPYING`).
- Local patches: none to Rust or C sources. The published package excludes
  `*.h`, so it omits `src/arm/asm-offsets.h`, which `src/arm/asm.S` includes.
  The snapshot adds that file byte-identical to upstream tag v1.1.0 (commit
  `782dab2135ea64a057c097088a13eb8ed3cc3320`, blob
  `538de71b32f73894f0855b23ea625a89108929b6`, sha256
  `b658d1084ebb60e504050fd85800c7d3b5658eb4a76d24ed9c59bb49af96d94f`).
- Features: `bitdepth_8`, `bitdepth_16`, `asm`, `asm_arm64_dotprod`,
  `asm_arm64_i8mm`, `asm_arm64_sve2`. The assembler build needs
  `bitdepth_16`: `src/ipred.rs` references `dav1d_ipred_pixel_set_16bpc_neon`
  from a runtime-dispatched match even when only eight-bit output is enabled,
  and upstream supplies that symbol in `ipred16.S`. Enabling upstream's 16-bit
  build is preferred to patching the crate. img still refuses high-precision
  AVIF in its own shim, so eight-bit decoding is unchanged. SVE2 is
  runtime-detected and provides 16-bit paths only; it builds with Apple clang
  and Alpine clang.
- Build: ARM64 compiles `src/arm/64/*.S` with `cc -march=armv8.6-a(+sve2)`.
  x86-64 assembles `src/x86/*.asm` with NASM through nasm-rs (minimum 2.14.0;
  NASM 3.01 works) and archives the objects with `cc`. LTO and `panic=abort`
  come from the img profile.
- Regeneration: reinstall the published crate, copy it verbatim, and re-add
  `src/arm/asm-offsets.h` from the same upstream tag. Confirm with `diff -r`
  against the registry source, then rerun `make native-build-check` and the
  AVIF tests.

## Updating a snapshot

1. Bump the version in `Cargo.toml` and run `cargo update -p <crate>` with the
   patch entry temporarily removed, so Cargo records the new registry checksum.
2. Replace `vendor/<crate>` with the extracted crate
   (`~/.cargo/registry/src/*/<crate>-<version>`) and confirm with
   `diff -r` that it is byte-identical.
3. For libdeflate-sys, diff `libdeflate/` against the upstream release tarball
   and record its sha256.
4. Restore the patch entry, update this file, and rerun
   `make native-build-check`, the PNG codec tests and the PNG benchmark.

Binary distributions carry the license files listed above; see
`docs/PROVENANCE.md`.

## jpegli and Highway

jpegli is from libjxl v0.11.1, commit `794a5dcf0d54f9f0b20d288a12e87afb91d20dfc`, BSD-3-Clause with the upstream PATENTS file. Highway is commit `457c891775a7397bdb0376bb1031e6e027af1c48`, Apache-2.0. The libjpeg API headers are from libjxl's libjpeg-turbo submodule at `8ecba3647edb6dd940463fedf38ca33a8e2a73d1`, under the IJG license. This software is based in part on the work of the Independent JPEG Group.

The libjxl snapshot contains the files in `JPEGXL_INTERNAL_JPEGLI_SOURCES`, `lib/jxl/base/{bits,byte_order,common,compiler_specific,include_jpeglib,printf_macros,sanitizer_definitions,status}.h`, and the public `jxl/types.h`. It also contains the libjpeg API headers, their license texts, and libjxl's LICENSE, PATENTS, and AUTHORS. Highway contains the transitive headers needed by the library sources, including the architecture-specific `hwy/ops` headers, and `abort.cc`, `aligned_allocator.cc`, `nanobenchmark.cc`, `per_target.cc`, `print.cc`, `targets.cc`, and `timer.cc`. Tests, programs, and unrelated codecs are omitted.

To regenerate the current trimmed snapshot, check out libjxl at the pinned commit with its submodules. Set `jxl_checkout` to that checkout and copy the tracked file list:

```sh
git ls-files vendor/libjxl | while IFS= read -r path; do
    mkdir -p "$(dirname "$path")"
    cp "$jxl_checkout/${path#vendor/libjxl/}" "$path"
done
git ls-files vendor/highway | while IFS= read -r path; do
    mkdir -p "$(dirname "$path")"
    cp "$jxl_checkout/third_party/highway/${path#vendor/highway/}" "$path"
done
```

One local patch, in `lib/jpegli/quant.cc`: grayscale output uses the perceptual YCbCr luma table, non-linear distance scaling and luma zero-bias that jpegli applies to YCbCr, instead of the Annex K table with linear scaling. Upstream applies its tuned quantization only to YCbCr. At matched SSIMULACRA2 on the grayscale and text inputs this makes the smallest attainable output 3.0%, 2.2% and 1.8% smaller than Zen Best at scores 80, 90 and 95, against 9.5%, 9.2% and 2.1% larger before. `grayscale_uses_the_perceptual_luma_quantization_of_color_output` in `tests/jpeg_codec.rs` fails without it. Reapply it after regenerating: every `JCS_YCbCr` test in `SetQuantMatrices` table selection and in `InitQuantizer`'s zero-bias setup also accepts `JCS_GRAYSCALE`, and the third chroma table is assigned only for YCbCr.

A second local patch, in `lib/jpegli/entropy_coding.cc`, speeds the progressive AC tokenizers: each block's coded and nonzero coefficients are packed into 64-bit masks and only set bits are visited, instead of testing every coefficient of every scan. Output is byte-identical (322 encodes: all 23 qualification inputs at qualities 40–100, balanced and best). It cuts color balanced encode at quality 90 from 30.6 to 19.2 ms. Reapply it by replacing the per-coefficient loops in `TokenizeACProgressiveScan` and `TokenizeACRefinementScan` with the mask walk.

For an update, compare `JPEGXL_INTERNAL_JPEGLI_SOURCES` and the transitive include closure against the new upstream build. Add or remove files as needed, update the pins, and run `make native-build-check` and the JPEG tests.

The local `jpegli-sys` build compiles the sources with `cc` in C++17 mode. It writes the libjpeg configuration header with the upstream values: ABI 62, 8-bit samples, and memory source and destination support. The build does not run CMake. The shim's `error_exit` throws a C++ exception that each exported shim function catches and turns into an error return, so jpegli destroys its stack objects and nothing unwinds into Rust. `HWY_DISABLED_TARGETS` limits Highway's compiled targets to NEON on ARM64 and SSE2, SSE4 and AVX2 on x86-64; SVE, AVX-512 and SSSE3 code is not built.

On Alpine, the release image uses `g++` and `linux-headers`: GCC supplies its C++ headers and Highway uses the ARM hardware-capability definitions. The musl release link uses the image's static `libstdc++.a` under the existing static-link policy; macOS uses the operating system's libc++.

## libavif and libaom

The AVIF encoder uses libavif v1.4.2, commit `bd32fb40a3b2028eb555b67f769c57a32ed8a1b4`, and libaom v3.13.1, commit `d772e334cc724105040382a977ebb10dfd393293`. Both are BSD-2-Clause; libaom also includes the AOM Patent License 1.0. Retained files are byte-identical to the upstream tarballs except one patch: `build/cmake/aom_optimization.cmake` also reads `nasm -h` and accepts `-Oflags`, because NASM 3 prints options only in `-h` and spells multipass optimization `-Oflags` and libaom's check otherwise rejects the Alpine release image's NASM 3.01. Drop the patch once upstream accepts NASM 3. libaom's bundled fastfeat, vector and x86inc licenses accompany release binaries.

To regenerate, verify and extract the tarballs pinned by `benchmarks/codecs/avif/build.sh` into `vendor/libaom` and `vendor/libavif` (strip each archive's top directory), then trim. From libaom, remove `apps/`, `doc/`, `examples/`, `tools/`, `third_party/googletest`, `third_party/highway`, `third_party/libwebm`, `third_party/libyuv`, and everything in `test/` except `test.cmake` and `test_data_util.cmake`, which the top-level CMakeLists includes unconditionally. From libavif, keep only `LICENSE`, `include/` and `src/`. The build sets `CONFIG_LIBYUV=0`, `CONFIG_WEBM_IO=0` and leaves `CONFIG_HIGHWAY` at its default of 0, so the removed trees are never referenced. The local `libavif-sys` crate builds libaom statically with CMake, with decoding and upstream tools disabled, and compiles libavif with only AOM encoding enabled. It does not discover system codec libraries. CMake requires Perl for libaom's generated headers; x86-64 SIMD requires NASM.
