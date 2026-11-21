# Deslop: the img crate

Types (DSLP-3), Tests (DSLP-4) and Dead code (DSLP-5) for `src/`, `tests/`, `benches/`, `examples/`, `codecs/*-sys` and the Python scripts. `codecs/webpkit` is a separate slice ([deslop-webpkit.md](deslop-webpkit.md)). Platform: macOS ARM64, nightly from `rust-toolchain.toml`, host load average 30 to 60 (timings are upper bounds). Base `a974ad1`.

Scratch tools were built offline from the local registry cache into `.work/tools` (not committed): `cargo install --offline --locked --root .work/tools --target-dir .work/tools-target cargo-machete@0.10.1 cargo-mutants@27.1.0`.

## DSLP-3 Types

Measured with `cargo clippy --locked --all-features --all-targets --message-format=json -- -W <lint>...`, counted by primary span. "src" is first-party non-test source; "other" is `tests/`, `benches/`, `examples/` and `src/**/tests.rs`. Default clippy already emits 46 warnings on this nightly (`make lint` has no `-D warnings`); the table is on top of those.

| Candidate | src | other | Decision |
| --- | --- | --- | --- |
| `clippy::wildcard_enum_match_arm` | 32 | 2 | Rejected. Reviewed `command.rs:41`, `error.rs`, `exit.rs:75`, `limits.rs:658`: each is a documented fallback over a foreign `non_exhaustive` enum or a closed set with a stated default. No defect. |
| `clippy::match_wildcard_for_single_variants` | 8 | 0 | Rejected, same reason. |
| `clippy::cast_possible_truncation` / `cast_sign_loss` / `cast_possible_wrap` | 82 / 22 / 2 | 167 / 23 / 5 | Rejected. Sizes are bounded by `MAX_DIMENSION` and `MAX_PIXELS`; no hit has a witness. |
| `clippy::unwrap_used` | 89 | 1158 | Rejected. Non-test hits are `slice.try_into().unwrap()` on a slice whose length was just checked (26 in `avif_container.rs`), `Layout::from_size_align` on constants, and `Quality::new` on values already in range. |
| `clippy::expect_used` | 1 | 8 | One real hatch: `av1.rs` re-derived a checked `NonNull` with `expect("validated AV1 plane")`. Fixed (below); lint not adopted, no other hit. |
| `clippy::redundant_clone` | 6 | 3 | Rejected (nursery). `avif.rs:68` is a false positive: the clone turns a borrowed `&Vec<u8>` into the owned `Vec` `with_icc` takes. |
| `clippy::needless_pass_by_value` | 10 | 0 | Rejected. Public signatures (`publish`, `Destination`) that callers construct inline. |
| `clippy::undocumented_unsafe_blocks` / `multiple_unsafe_ops_per_block` | 34 / 16 | 2 / 1 | Rejected. Documentation style, not a type defect; the comment policy forbids bulk comments. |
| `clippy::struct_excessive_bools` | 8 | 1 | Rejected. `Description` and `Request` are public carriers whose fields are the JSON schema. |
| `clippy::indexing_slicing` / `as_conversions` | 454 / 369 | 368 / 284 | Rejected (recorded false-positive classes). |
| rustc `unreachable_pub`, `trivial_casts`, `trivial_numeric_casts`, `unused_results`, `unused_qualifications` | 1, 1, 3, 8, 49 | 0, 0, 0, 0, 5 | Rejected. Style; none rejects a defect. |
| rustc `missing_debug_implementations` | 14 | 0 | Rejected. Adding `Debug` to public types is an API addition. |
| `unsafe_code = "forbid"` | n/a | n/a | Not applicable: 146 first-party hatch lines in `src/`, most of them FFI (`libc` file and signal calls, the dav1d shim, the page-returning allocator). |

Adopted settings: none. No candidate rejected a real defect at fewer than the allowed false-positive rate.

Fixes:

| Finding | Commit | Witness |
| --- | --- | --- |
| `encode_cancellable` carried no doc (the doc line sat on `quantized_quality`), so rustc `missing_docs` warned on every build | `3cc0d72` | the warning is gone from `cargo check` |
| `av1.rs` `decode_still` expected a `NonNull` it had just null-checked | `123aa51` | `plane` now returns `(NonNull<u8>, usize)`, so the type carries the proof; `tests/avif_codec.rs` passes (16 tests) |

Escape hatches (`rg -c '\bunsafe\b|#!?\[allow\(|\.unwrap\(\)|\.expect\('`, lines): `src` 147 to 146; `tests` 581, `benches` 35, `examples` 64, `codecs/*-sys` 34, unchanged.

## DSLP-4 Tests

### Tools

| Tool | Result |
| --- | --- |
| `cargo-mutants` 27.1.0 | Built offline; run in place with `--cap-lints false` so the warm debug target is reused. |
| coverage | Needs environment. `cargo-llvm-cov` 0.9.1 failed to build twice (`rustc` killed by signal 9 under host memory pressure). `llvm-cov` and `llvm-profdata` exist in the toolchain sysroot, but an instrumented cold rebuild of the C codecs was not attempted. No coverage numbers. No `make coverage` or `make mutants-diff` goal was added: neither recipe is qualified in CI. |
| `make test` cost | Cold 17m42s on this host (base `a974ad1`, all 18 suites green, `benchmark_verify` 96 s of it). The two tests added here run in under 0.3 s together; warm cost is unchanged. |

### Pyramid

| Tier | Suites (tests, time) |
| --- | --- |
| Unit | `src/lib.rs` 96 (0.16 s), `src/main.rs` 5 (0.21 s) |
| Codec integration | `jpeg_codec` 17 (0.03 s), `png_codec` 14 (3.9 s), `webp_codec` 7 (0.05 s), `avif_codec` 16 (2.1 s), `codec_boundary` 9 (0.25 s), `color_fidelity` 10, `optimizer` 17 (0.6 s), `perceptual_score` 4, `score_planar` 2 (4.2 s) |
| CLI end to end | `cli` 16, `baseline` 2, `deadlock` 2, `signals` 3, `timestamps` 1 (all under 1 s) |
| Benchmark verification | `benchmark_verify` 2 (96 s), Python `tests.test_bench` 14 (14 s); `test_package_release.py` skipped without GNU tar |

False passes reviewed in `jpeg_codec.rs` and `codec_boundary.rs`: none found; loops that guard on `if let Ok` (`jpegli_output_limit_returns_an_error`) end in an `assert!` on the flag they set.

### Contracts

| Contract | Guarding test | Disposition |
| --- | --- | --- |
| Frame dimensions are checked at the JPEG header, before libjpeg-turbo allocates (DSLP-11) | `tests/jpeg_codec.rs:298` `oversized_frame_headers_are_refused_as_limits_before_any_decode_allocation` | New. Failed before the fix (`Quantization table 0x00 was not defined`, kind `Invalid`), fails with the check removed, passes with it. |
| At most 256 JPEG scans, rejected before any decode | `tests/jpeg_codec.rs:257` | Existing; mutants in the marker walker (below). |
| Cancellation interrupts a progressive decode | `tests/jpeg_codec.rs:271` | Existing. |
| Pixel and side ceilings in `from_samples` | `tests/codec_boundary.rs:7` (lines 28 to 34) | Existing; both sides of 16,384 and 4096x4096 are asserted. |
| 128 MiB input cap for streams | `tests/codec_boundary.rs:56` | Existing. |
| Container detection (RIFF/WEBP, `ftyp` with `avif` or `avis`) | `tests/codec_boundary.rs:161` `container_signatures_are_matched_whole` | New. Four `Format::sniff` mutants survived; all caught now. |
| PNG metadata, text and ICC bounded together | `tests/png_codec.rs:121`, `:134` | Existing. |
| AVIF extents larger than the file refused before allocation | `tests/avif_codec.rs:277` | Existing. |
| WebP and AVIF canvas dimensions checked at the header | none | Gap: no crafted oversized WebP or AVIF header test. Filed below. |
| Exit codes, admission reservations | `src/exit.rs` and `src/runtime.rs` unit tests | Not mutated in this run. |

### Mutation (bounded, incomplete)

Scope listed with `cargo mutants --list`: 90 mutants in `src/still/jpeg.rs` (`structure`, `decode`: 59) and `src/still.rs`, `src/still/webp.rs` (`sniff`, `read_bounded`, `check_dimensions`, `from_samples`, `decode_cancellable`, `limits`, WebP `decode`: 31). Tests per mutant: `jpeg_codec` and `codec_boundary` for the first group, plus `png_codec`, `webp_codec` and `avif_codec` for the second. Each mutant cost 1.5 to 4 minutes on this host, so the run was stopped on the coordinator's instruction after 29 of 90.

| Result | Count | Mutants |
| --- | --- | --- |
| Caught | 19 | |
| Timeout | 4 | `jpeg.rs:38:48` (`+=` to `-=` and `*=`), `jpeg.rs:40:64` (`-=`), `jpeg.rs:45:44` (`*=`): the marker walker stops advancing and hangs, which the suite reports as a timeout. Detected by hang, not by an assertion; the real code cannot loop. |
| Missed, equivalent | 1 | `jpeg.rs:40:64` `+=` to `*=`: after a stuffed zero or restart marker the walker skips that marker byte, but the next step searches for the next `0xFF` and the marker byte is never `0xFF`, so not skipping it yields the same position. Simplifying the line to `continue` would remove the mutant; left unchanged because the decode path was not otherwise touched. |
| Missed, killed by a new test | 4 | `still.rs:138:44`, `:139:45`, `:139:106`, `:139:122` in `Format::sniff`. Re-run after the new test: 9 `sniff` mutants, 8 caught, 1 unviable, 0 missed. |
| Unviable | 1 | `sniff -> Ok(Default::default())` |
| Not run | 61 | the rest of `structure`/`decode`, `read_bounded`, `check_dimensions`, `from_samples`, `decode_cancellable`, `limits`, WebP `decode` |

Acceptance for DSLP-4 is not met: 61 mutants in the critical scope are untested, and `src/publication.rs`, `src/runtime.rs` and `src/exit.rs` were not mutated at all. Resume with `cargo mutants --in-place --cap-lints false --baseline skip --iterate --file src/still/jpeg.rs --file src/still.rs --re 'in structure|jpeg::decode|Format::sniff|read_bounded|check_dimensions|from_samples|decode_cancellable' -C=--all-features -C=--locked -- --test jpeg_codec --test codec_boundary`.

### Injected fault

Critical decision: the JPEG header bound added for DSLP-11. Removed `super::check_dimensions(structure.width, structure.height)?;` from `jpeg::decode` and ran `cargo test --locked --all-features --test jpeg_codec`: exit 101, `oversized_frame_headers_are_refused_as_limits_before_any_decode_allocation` failed (`left: Invalid`, `right: Limit`), the other 16 passed. Restored from a saved copy; `cmp src/still/jpeg.rs .work/tmp/jpeg.rs.orig` exit 0, working tree clean.

## DSLP-5 Dead code

| Tool | Command | Result |
| --- | --- | --- |
| rustc `dead_code` | `cargo check --locked` at `--no-default-features`, `--no-default-features --features build-binary`, and default features | No warnings at any of the three feature sets. The ticket's `checked_sdr` is used (`src/still/color.rs:240`); that seeded fact was stale. |
| `cargo-machete` 0.10.1 | `cargo-machete .` | No unused dependency in `img`. One hit in `codecs/webpkit` (`libwebp-sys`, see [deslop-webpkit.md](deslop-webpkit.md)). |
| `cargo udeps` | | Needs environment (not installed; not in the registry cache). Machete is clean, so no disputed dependency to settle. |
| Python | AST scan of 10 files for unreferenced functions, classes and imports (no `ruff` or `vulture` installed) | Three unused imports removed (`e4a6128`): `os` in `scripts/qualification-corpus`, `contextlib` and `io` in `tests/test_bench.py`. `tests.test_bench` passes (14 tests). The `Selection`, `Guards`, `Budgets`, `PackageReleaseTest` hits are `unittest.TestCase` classes discovered by name. |

Public items with no caller outside `src/**/tests.rs`, workspace-wide (`rg -w` over `src`, `tests`, `benches`, `examples`): `LimitSet::check_bytes`, `LimitSet::suggested_side`, `SystemBudget::is_probed`, `error::input_size_limit`, `output_size_limit`, `input_open_error`, `input_config_error`, `input_operation_error` and `classify_output`. They are the retained library API (`docs/CODEC-BOUNDARY.md`: "Consumers: none declared in this repository"), so they are consumer questions, not removals.

`examples/scratch-*.rs` (16 files): 7 are referenced nowhere (`breakdown`, `dump`, `entropy`, `gateprobe`, `probe`, `sharp`, `sweep`); the other 9 are named in `docs/HANDOFF.md`, the WebP hillclimb histories or `benchmarks/corpus/validation/manifest.json`. All compile under `--all-targets`. They are the WebP hillclimb workbench and removing them drops reproduction commands, so this is a walk-back, filed as a consumer question rather than cut. The `#![allow(dead_code)]` in `tests/common/mod.rs`, `benches/support/mod.rs`, `tests/support/avif_raw.rs` and `src/test_utils.rs` is the per-binary shared-helper pattern (each test binary uses a subset), not dead code.

Allowlist entries added: 0.

## DSLP-11 JPEG dimension limits

`jpeg::decode` handed `structure.width` and `structure.height` to `libjpeg_turbo_sys::decode` before any limit check. That wrapper reserves `width * height * 3` bytes with `Vec::with_capacity` (up to 12.9 GB for a 65,500 by 65,500 header) and libjpeg-turbo allocates its coefficient arrays, and only then did `StillImage::from_samples` call `check_dimensions`. Commit `e800696` calls `check_dimensions` after the precision and component checks and before any allocation, as PNG, WebP and AVIF already do. `describe` still reports the declared size of an oversized JPEG; `docs/CLI.md` now says so.

Regression test: `tests/jpeg_codec.rs:298`, header-only JPEGs of 8192x8192, 16384x16384 (kind `Limit`, message `pixel count`) and 65500x65500 (kind `Limit`), zero width (kind `Invalid`), and `describe` of the 65500x65500 header still succeeds. Failed before the fix, passes after (`cargo test --locked --all-features --test jpeg_codec --test codec_boundary`: 17 and 9 passed).

## Findings not fixed

- P4, WebP and AVIF header dimension limits have no crafted-input test (`tests/webp_codec.rs`, `tests/avif_codec.rs`); a VP8X canvas of 16384x16384 and an `ispe` of the same size should assert `ErrorKind::Limit` without a large allocation.
- P4, `Format::sniff` needs at least 32 bytes to recognise AVIF (`bytes.get(8..32).unwrap_or(&[])`): a 24-byte `ftyp` box alone is `Unsupported`. Harmless for decodable files; state it or use the declared box size.
- P4, mutation of `src/publication.rs`, `src/runtime.rs`, `src/exit.rs` and the remaining 61 decode mutants is outstanding (DSLP-4 stays open).
- P4, consumer question: the nine unused public `limits`/`error` items and the seven unreferenced `examples/scratch-*.rs` above.
