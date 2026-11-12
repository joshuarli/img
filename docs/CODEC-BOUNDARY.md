# Still-image boundary

`still` exposes four formats, validated quality, effort and sample layouts. Codec settings, parser state, zencodec jobs and zenpixels storage are private. `optimizer` selects verified bytes without filesystem effects; `publication` owns file transactions. Unrelated zune library operations remain available.

The shared published zencodec job carries bounded decode requests, metadata and pixel descriptors. A local format enum keeps implementation traits out of the public API. PixelBuffer carries native interleaved 16-bit samples. No registry or generalized pipeline is needed.

JPEG encoding uses zenjpeg 0.8.4's native request: its generic adapter applies the effort preset after progressive settings, disabling progressive output at fast effort. The native path applies the preset first. Remove this workaround when the generic adapter passes the independent progressive regression. Color uses ApproxJpegli quality and 4:2:0; gray remains native gray. Four-component CMYK/YCCK, non-eight-bit JPEG, MPF auxiliary images, HDR gain-map signaling, trailing payloads and application metadata after scan start are refused. A bounded marker walker distinguishes entropy stuffing/restarts from real markers across progressive scans; this closes base-image extraction and late-profile/orientation loss that the shared header adapter does not expose. Inspection still reports declared precision, multi-image/HDR signals and metadata headers. Same-format lossless JPEG is an optimizer-owned byte copy.

PNG uses zenpng 0.1.4 and single-threaded zenquant 0.1.3 for ordinary exact eight-bit and explicit palette paths. Direct quality maps to 2–256 palette entries and refuses grayscale sources. The published encoder can reduce some 16-bit samples to eight bits and reduce RGB color type while retaining an RGB ICC profile; it also lacks gray-alpha encoding. Exact 16-bit, ICC and gray-alpha paths therefore use png 0.18.1 with fixed color type and depth. Remove this adapter when zenpng can disable those reductions and passes the independent sample/header cases. Ancillary chunks are bounded and CRC-checked; unsafe-to-copy or layout-sensitive chunks require strip before rewriting.

WebP uses zenwebp 0.4.6, exact hidden RGB in lossless mode, and alpha quality 100 in lossy mode. High-depth reduction is refused. AVIF's managed plane adapter and narrower supported representation are documented in [AVIF](AVIF.md). [Color](COLOR.md) and [scoring](SCORING.md) own preparation and fidelity rules.

Mandatory ceilings apply without optional system probing: 16,384 pixels per side, 16,777,216 pixels total, 128 MiB compressed input/output and 8 MiB decoded metadata. Checked arithmetic precedes allocation. Admission reserves 256 bytes/pixel, up to 4 GiB, plus compressed buffers per active image. This reservation is not a measured RSS claim.

Publication opens each directory component with O_NOFOLLOW. New files use exclusive staging and linkat no-clobber publication. In-place publication rechecks the opened source's device, inode, size, modification and change times immediately before renameat. File data and parent directory are synced; later errors identify committed output. RAII removes only owned unpublished staging files. New staging files are owner-only (0600).

Destination keys honor APFS case and Unicode collation without changing actual native path bytes. Output subtree exclusion uses directory IDs. macOS rejects invalid UTF-8 filenames with EILSEQ; Linux's real native-byte success case passes. JSON rejects such paths before writes.

## Development checks

Use the pinned nightly, kache and a repository-local TMPDIR. Do not run formatters.

```sh
make test
make decoder-reference-check
make metric-reference-check
make benchmark-verify
cargo check --locked --no-default-features
cargo check --locked --no-default-features --features build-binary
cargo clippy --locked --all-features --all-targets
make pure-rust-build-check
```

The metric executable check needs JPEG XL 0.12.0 tools. Native libwebp, dav1d and LittleCMS bindings are development-only independent references. Production build scripts in rav1d-safe and zenrav1e emit Rust configuration with assembly/FFI disabled; cc and nasm-rs are dormant Rust build dependencies, not evidence of executed native compilation. The fresh-target guarded build denies CC/CXX, CMake, NASM, pkg-config and bindgen and points libclang to an absent path. Rust's OS linker remains available. This is a masked-tool proof; a physically clean host is a separate release proof.

The [resolved normal/build inventory](../benchmarks/production-dependencies.json) excludes development edges and unions the three supported targets. No third-party native codec/color runtime or codec threading feature is selected. zenavif unconditionally selects rav1d-safe eight- and sixteen-bit decoder implementations and HDR metadata helpers even though this CLI refuses high-depth/HDR processing; published feature separation is needed to remove that transitive cost. Actual macOS linkage contains only CoreFoundation, Objective-C, IOKit, iconv and libSystem. Static Linux ARM64 has no interpreter or dynamic section and passes an exact conversion/compare smoke test. Linux x86-64 passes the full suite under emulation; its clean static PIE has no interpreter or shared-library dependencies. Native x86-64 hardware timing remains unproven.
