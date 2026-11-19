# Still-image boundary

`still` exposes four formats, validated quality, effort and sample layouts. Codec settings, parser state and zenpixels storage are private. `optimizer` selects verified bytes without filesystem effects; `publication` owns file transactions. Unrelated zune library operations remain available.

The product scope is a still-image CLI for JPEG, PNG, WebP and AVIF on macOS ARM64 and Linux ARM64/x86-64. Windows, animation and other formats are outside that scope. Keep the operation-first CLI and preserve unrelated public library contracts. Production codecs may use permissively licensed native code (C/C++, FFI, upstream SIMD/assembly) built from vendored, pinned sources recorded in [VENDOR.md](VENDOR.md); copyleft code never enters the production graph. The WebP encoder is a separate pure-Rust workstream ([WEBP-HILLCLIMB.md](WEBP-HILLCLIMB.md)). Do not add a plugin registry, server or persistent cache to this pipeline.

## Source and contract map

| Boundary | Source | Contract |
| --- | --- | --- |
| Arguments, planning, workers and reporting | `src/cli/command.rs`, `src/runtime.rs` | [CLI](CLI.md) |
| Decode, encode, sample layouts and color | `src/still.rs`, `src/still/` | [Color](COLOR.md), [AVIF](AVIF.md) |
| Candidate selection and fidelity | `src/optimizer.rs`, `src/perceptual.rs` | [Optimization](OPTIMIZATION.md), [scoring](SCORING.md) |
| Filesystem transactions and cancellation | `src/publication.rs`, `src/cancellation.rs` | [CLI file and shutdown behavior](CLI.md) |
| Retained library operations and helpers | `src/operations/`, `src/limits.rs`, `src/error.rs`, `src/exit.rs` | Public Rust API documentation |
| Regression fixtures and measurement | `tests/`, `benches/`, `examples/benchmark.rs` | [Behavior matrix](../tests/parity.md), [benchmarks](../BENCHMARKS.md) |
| Packaging and distribution | `scripts/package-source`, `scripts/package-release.py` | [Release](RELEASE.md), [provenance](PROVENANCE.md) |

The CLI uses `SystemBudget` for admission. The retained `LimitSet`, `ImgError` and `RunState` APIs serve library callers; they do not define the current CLI's fixed still-image limits or reporting path.

zenpixels and zencodec retain the shared pixel-descriptor, metadata and resource-limit types. A local format enum keeps implementation traits out of the public API. PixelBuffer carries native interleaved 16-bit samples. No registry or generalized pipeline is needed.

JPEG decoding uses zune-jpeg 0.5.15. JPEG encoding uses vendored jpegli from libjxl 0.11.1, with progressive level 2 and 4:4:4 color. Fast, balanced and best use the same settings; quality is rounded and clamped to 1–100. Gray remains single-component. Cancellation is checked before encoding, between scanline batches and before finishing; jpegli itself is not interruptible. Four-component CMYK/YCCK, non-eight-bit JPEG, MPF auxiliary images, HDR gain-map signaling, trailing payloads and application metadata after scan start are refused. A bounded marker walker distinguishes entropy stuffing/restarts from real markers across progressive scans; this closes base-image extraction and late-profile/orientation loss. Inspection still reports declared precision, multi-image/HDR signals and metadata headers. Same-format lossless JPEG is an optimizer-owned byte copy.

PNG decoding uses png 0.18.1 with eight- and sixteen-bit layouts and CRC-checked, size-bounded ancillary chunks. Encoding uses the vendored OxiPNG 10.2.1 with libdeflate 1.26: effort fast, balanced and best map to OxiPNG preset 0, preset 0 with libdeflate level 8, and preset 2. Lossless reductions are limited to ones that keep the decoded samples, the declared bit depth (no 16-to-8 reduction), hidden RGB under zero alpha (alpha optimization off), and RGB color signaling (no grayscale reduction when ICC, cICP or another color chunk is present). Direct quality maps to 2–256 palette entries via quantette 0.6.0 (Oklab, Wu/kmeans, Floyd–Steinberg dithering, single-threaded) and refuses grayscale sources, nonopaque alpha, 16-bit samples and wide-gamut inputs it cannot represent. Ancillary chunks are bounded and CRC-checked; unsafe-to-copy or layout-sensitive chunks require strip before rewriting. CICP is written on encode, read as decode color authority, and reported by describe.

WebP decoding uses image-webp 0.2.4 (image-rs), rejecting animation before frame extraction. Encoding uses the vendored Webpkit snapshot at `codecs/webpkit/` (MIT OR Apache-2.0, pinned at `cc1311ebf`, upstream notices preserved) through a path dependency — an experimental backend until the gates in [WEBP-HILLCLIMB.md](WEBP-HILLCLIMB.md) pass. Exact alpha is preserved in lossy mode. High-depth reduction is refused. AVIF's decoder is a bounded unsafe shim over rav1d 1.1.0 (BSD-2-Clause, bitdepth_8, assembler features off); encoding uses ravif 0.13.0 (BSD-3-Clause, scalar rav1e) which embeds EXIF only, so common-sRGB AVIF candidates drop ICC in favor of NCLX signaling and XMP-bearing candidates are refused. [Color](COLOR.md) and [scoring](SCORING.md) own preparation and fidelity rules.

The still-image boundary sets ceilings of 16,384 pixels per side, 16,777,216 pixels total, 128 MiB compressed input/output and 8 MiB decoded metadata. PNG inspection and decoding count retained ancillary chunks together with expanded compressed text and ICC data before probing. Transformation admission reserves 256 bytes/pixel, up to 4 GiB, plus compressed buffers per active image. Comparison reserves 256 bytes/pixel for each input and their compressed buffers before decoding. These reservations are not measured RSS claims.

Publication opens each directory component with O_NOFOLLOW. New files use exclusive staging and linkat no-clobber publication. In-place callers must open a `ReplacementSource`, which takes a nonblocking exclusive parent-directory lock before opening the source and holds it until the source is dropped. Only that type authorizes `Destination::Replace`. Cooperating processes conflict when writing any files in the same directory; CLI jobs in the same batch serialize by directory. An in-place job must have exclusive ownership against external writers: a forced external atomic save between the source identity check and rename was overwritten, and another identity check cannot make the rename conditional. File data and parent directory are synced; later errors identify committed output. RAII removes only owned unpublished staging files. New staging files start owner-only (0600); in-place publication copies source permissions.

Destination keys honor APFS case and Unicode collation without changing actual native path bytes. Output subtree exclusion uses directory IDs. macOS rejects invalid UTF-8 filenames with EILSEQ; Linux's real native-byte success case passes. JSON rejects such paths before writes.

## Development checks

Use the pinned nightly, kache and a repository-local TMPDIR. Do not run formatters.

```sh
mkdir -p .work/tmp
TMPDIR="$PWD/.work/tmp" RUSTC_WRAPPER=kache cargo build --locked --profile dist --features build-binary
make baseline-check
make test
make decoder-reference-check
make metric-reference-check
make benchmark-verify
cargo check --locked --no-default-features
cargo check --locked --no-default-features --features build-binary
cargo clippy --locked --all-features --all-targets
make native-build-check
```

Use `.work/tmp` for disposable images and `.work/wt` for worktrees. Keep tests away from the owner's image collection. `tests/baseline.rs` is the small end-to-end PNG and destination-safety baseline. The larger suites add independent decoder, color, metadata, filesystem and signal checks. Existing performance gates apply to measured claims, not to ordinary correctness checks. Remaining external proofs and retry commands are in [benchmark evidence](../BENCHMARKS.md#remaining-proofs).

The metric executable check needs JPEG XL 0.12.0 tools. Native libwebp, dav1d and LittleCMS bindings are development-only independent references. `make native-build-check` builds the release binary from a fresh target with every C/C++ compiler invocation logged and fails if any native source outside the vendored allowlist (libdeflate, libjxl, Highway and the local jpegli shim) is compiled; CMake, NASM, pkg-config and bindgen remain denied and libclang points to an absent path.

The [resolved normal/build inventory](../benchmarks/production-dependencies.json) excludes development edges and unions the three supported targets. Selected native code includes vendored libdeflate, jpegli and Highway, compiled statically. Actual macOS linkage contains only CoreFoundation, libc++, Objective-C, IOKit, iconv and libSystem. Static Linux ARM64 has no interpreter or dynamic section and passes an exact conversion/compare smoke test. The previous implementation passed the Linux x86-64 suite under emulation and produced a static PIE. The jpegli integration has not been tested on Linux x86-64, and native x86-64 hardware timing remains unproven.
