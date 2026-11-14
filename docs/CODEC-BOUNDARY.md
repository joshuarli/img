# Still-image boundary

`still` exposes four formats, validated quality, effort and sample layouts. Codec settings, parser state and zenpixels storage are private. `optimizer` selects verified bytes without filesystem effects; `publication` owns file transactions. Unrelated zune library operations remain available.

The product scope is a still-image CLI for JPEG, PNG, WebP and AVIF on macOS ARM64 and Linux ARM64/x86-64. Windows, animation and other formats are outside that scope. Keep the operation-first CLI and preserve unrelated public library contracts. Codec/color runtimes stay pure Rust; OS interfaces and independent native development references are permitted. Do not add a plugin registry, server or persistent cache to this pipeline.

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

The CLI uses `SystemBudget` for admission. The retained `LimitSet`, `RimageError` and `RunState` APIs serve library callers; they do not define the current CLI's fixed still-image limits or reporting path.

zenpixels and zencodec retain the shared pixel-descriptor, metadata and resource-limit types. A local format enum keeps implementation traits out of the public API. PixelBuffer carries native interleaved 16-bit samples. No registry or generalized pipeline is needed.

JPEG decoding uses zune-jpeg 0.5.15. JPEG encoding uses mozjpeg-rs 0.9.2, BSD-3-Clause, always in progressive mode. Gray remains native gray, 4:2:0 for color. Four-component CMYK/YCCK, non-eight-bit JPEG, MPF auxiliary images, HDR gain-map signaling, trailing payloads and application metadata after scan start are refused. A bounded marker walker distinguishes entropy stuffing/restarts from real markers across progressive scans; this closes base-image extraction and late-profile/orientation loss. Inspection still reports declared precision, multi-image/HDR signals and metadata headers. Same-format lossless JPEG is an optimizer-owned byte copy.

PNG decoding and encoding use png 0.18.1 with eight- and sixteen-bit layouts and CRC-checked, size-bounded ancillary chunks. Direct quality maps to 2–256 palette entries via quantette 0.6.0 (Oklab, Wu/kmeans, Floyd–Steinberg dithering, single-threaded) and refuses grayscale sources, nonopaque alpha, 16-bit samples and wide-gamut inputs it cannot represent. Ancillary chunks are bounded and CRC-checked; unsafe-to-copy or layout-sensitive chunks require strip before rewriting. CICP is written on encode, read as decode color authority, and reported by describe.

WebP decoding uses image-webp 0.2.4 (image-rs), rejecting animation before frame extraction. Encoding uses webpkit 0.1.0 (MIT OR Apache-2.0) for both lossless VP8L and lossy VP8; exact alpha is preserved in lossy mode. High-depth reduction is refused. AVIF's decoder is a bounded unsafe shim over rav1d 1.1.0 (BSD-2-Clause, bitdepth_8, assembler features off); encoding uses ravif 0.13.0 (BSD-3-Clause, scalar rav1e) which embeds EXIF only, so common-sRGB AVIF candidates drop ICC in favor of NCLX signaling and XMP-bearing candidates are refused. [Color](COLOR.md) and [scoring](SCORING.md) own preparation and fidelity rules.

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
make pure-rust-build-check
```

Use `.work/tmp` for disposable images and `.work/wt` for worktrees. Keep tests away from the owner's image collection. `tests/baseline.rs` is the small end-to-end PNG and destination-safety baseline. The larger suites add independent decoder, color, metadata, filesystem and signal checks. Existing performance gates apply to measured claims, not to ordinary correctness checks. Remaining external proofs and retry commands are in [benchmark evidence](../BENCHMARKS.md#remaining-proofs).

The metric executable check needs JPEG XL 0.12.0 tools. Native libwebp, dav1d and LittleCMS bindings are development-only independent references. No production package carries a build script or disables/selects third-party assembler or C FFI paths; the masked-tool proof denies CC/CXX, CMake, NASM, pkg-config and bindgen and points libclang to an absent path. Rust's OS linker remains available. This is a masked-tool proof; a physically clean host is a separate release proof.

The [resolved normal/build inventory](../benchmarks/production-dependencies.json) excludes development edges and unions the three supported targets. No third-party native codec/color runtime is selected. Actual macOS linkage contains only CoreFoundation, Objective-C, IOKit, iconv and libSystem. Static Linux ARM64 has no interpreter or dynamic section and passes an exact conversion/compare smoke test. Linux x86-64 passes the full suite under emulation; its clean static PIE has no interpreter or shared-library dependencies. Native x86-64 hardware timing remains unproven.
