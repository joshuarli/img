# Deslop: codecs/webpkit

DSLP-6 (Types), DSLP-7 (Tests) and DSLP-8 (Dead code) for the imported codec at
`codecs/webpkit/` (37.7k non-vendored lines, snapshot of P4suta/webpkit
`cc1311e` plus the hillclimb's encoder changes). Base revision `a9ee354`; work
tree `deslop/webpkit`; Types commit `f446941`. webpkit is its own Cargo
workspace, so its checks run from the repository root with
`--manifest-path codecs/webpkit/Cargo.toml`, and the crate's own tests build into
the crate's `codecs/webpkit/target` (untracked build output, 380 MB here,
removed before the pass closed).

The file was seeded by an earlier, unmerged attempt whose commit hashes
(`29f1703`, `503cd94`, `a70e771`, `7e433e6`) are dangling in this branch. The
facts below were re-measured here; where the seeded record is still right it is
cited as such.

## Outcome

| Step | Result |
| --- | --- |
| Types (DSLP-6) | 34 default-clippy findings cleared at `-D warnings`; `unwrap_used` and `needless_pass_by_value` adopted in `[lints.clippy]` with a test exemption; no emitted-byte change (7/7 deterministic fixture hashes identical). Escape hatches: `.unwrap()` 833 → 831, all other counts flat. |
| Tests (DSLP-7) | Mutation not run: `cargo-mutants` is not installed in this environment (nor `cargo-llvm-cov` or `cargo-machete`), so the fallback is the read-only gap review below. No survivor is dispositioned because no mutant was generated. Four verified gaps, one of them material (the corpus' `expected.rgba` files are never read). |
| Dead code (DSLP-8) | One allowlist entry (the crate's existing `cfg_attr` for `dead_code`/`unreachable_pub`/`unused_imports`), re-validated and its reason corrected; zero deletions, because every default-build finding is used by an inline test, a `cfg(feature)` shim, or is upstream module API. One unused optional dependency (`libwebp-sys`, feature `oracle`) is a Needs-owner walk-back. |

## DSLP-6 Types

### Default profile, before and after

`cargo clippy --locked --manifest-path codecs/webpkit/Cargo.toml --all-targets -- -D warnings`
failed before the pass with 34 errors (28 in the lib, 6 in the lib-test target;
the manifest had no `[lints]` table because upstream's workspace inheritance was
dropped at import). After `f446941` it exits 0.

| Lint | Findings | Fix |
| --- | ---: | --- |
| `large_const_arrays` | 2 | `const` → `static` for the 4097-entry log2 tables; the weighted table now takes the log2 slice as an argument, since a static cannot be read from a const initializer. |
| `needless_range_loop` | 9 | Iterator rewrites. The `i == length` sentinel in `optimize_huffman_for_rle` keeps its identity through `chain(once(&0u32))`; the frame tests iterate `&mb_data[..mb_w]` so the length invariant still panics. |
| `manual_range_contains` | 1 | `(1..=WINDOW_SIZE).contains(&width)`. |
| `too_many_arguments` | 5 | `PixelSource`, `StreamSwitches`, `BlockEntry`, `RateSlot` group the repeated parameters; no suppression added. |
| `type_complexity` | 2 | `CrossColorMap` and `SpecScores` aliases. |
| `collapsible_if` / `needless_borrow` | 6 | `if k && let Some(i) = index_of(t)` (edition 2024 let-chains). |
| `needless_lifetimes` / `unnecessary_cast` / `useless_conversion` | 5 | Elision, and the casts/useless `usize::from` removed. |
| `unfulfilled_lint_expectations` | 1 | The `missing_const_for_fn` expect on `match_length` fired only on the scalar path; it is now `all(not(work-count), not(target_arch = "aarch64"))`. On aarch64 the NEON kernel is not const-evaluable. |
| `chunks_exact_to_as_chunks` | 3 | Test-only `as_chunks::<4>()`. |

### Strict-profile candidates

Measured on the lib target (default features, `CLIPPY_CONF_DIR` with
`allow-unwrap-in-tests = true`), then `--all-targets` for the test/example view.

| Candidate | Hits | Decision |
| --- | ---: | --- |
| `clippy::unwrap_used` | 2 (+4 in one example) | **Adopted.** Both are `Mutex::lock().unwrap()` in `CrossColorCache::forward`; poison recovery (`unwrap_or_else(PoisonError::into_inner)`) keeps the cache usable and deletes the panic. The example (`examples/decodebench-scratch.rs`) now returns `Result` and uses `?`. None of the 4 are false positives. |
| `clippy::needless_pass_by_value` | 1 | **Adopted.** `finish` takes `Option<&[u8]>` and no longer moves the ALPH plane it only borrows. |
| `clippy::expect_used` | 6 | Rejected. All six assert an invariant established by the same function (`heap.pop().expect("heap has at least two")` after `len() > 1`; `trim_fields(..)` inside the branch that selected a representable plan). Converting them would invent unreachable error paths in `huffman/build.rs` and `huffman/serialize.rs`, and one in `vp8l/encode.rs`. |
| `clippy::cast_possible_truncation` | 13 | Rejected: 13 false positives, well over the one-per-5k-lines budget. All are `(LOG2_SMALL_MAX + 1) as usize` with a 4097 value, or table values the surrounding `assert!`/clamp bounds. |
| `clippy::cast_sign_loss` | 2 | Rejected, both values are range-checked immediately above (`cc` in `MIN_TRANSFORM_BITS..=MIN_TRANSFORM_BITS+7`; `base_q.clamp(0, 32)`). |
| `clippy::wildcard_enum_match_arm` | 6 | Rejected. These are `_ => None`/`_ => {}` fallbacks for internal enums (`Mode`, `BestTask`) whose match already handles every variant that carries data; enumerating them adds a compile error for future variants, not a defect. Matches the fleet's earlier rejection. |
| `clippy::match_wildcard_for_single_variants` | 0 | No witness. |
| `clippy::redundant_clone` | 0 | No witness. |
| `unsafe_code = "forbid"` | — | Not available: the owner granted one `unsafe` module (the aarch64 NEON match-length kernel, `lossless/vp8l/match_kernel.rs`) and `forbid` turns its local `#[allow(unsafe_code)]` into an error. The crate keeps `#![deny(unsafe_code)]`. |

Not re-measured: `indexing_slicing` (1187) and `as_conversions` (425). The
seeded record measured them on the same tree and rejected both as false-positive
classes (bounded cursors, masked indices); nothing in this pass changes that
population.

### Escape hatches (`src/`, the skill's Rust regex)

| Hatch | Base `a9ee354` | After `f446941` |
| --- | ---: | ---: |
| `.unwrap()` | 833 | 831 |
| `.expect(` | 69 | 69 |
| `#[allow(` / `#![allow(` | 25 | 25 |
| `#[expect(` | 37 | 37 |
| `unsafe` | 8 | 8 |

No `#[allow]` or `#[expect]` was added. The two `unwrap()` calls removed are the
poisoned-lock pair; their replacement cannot fail.

### Contract witnesses

The typing changes are value-preserving: statics hold the same compile-time
tables, loop rewrites visit the same indices in the same order, the cache
recovery only changes the poisoning case (no output), and `finish` passes the
same bytes. The encoder contracts (`src/still/webp.rs`'s `Encoder`/`Effort`/
`Dimensions`/`ImageRef`/`Metadata`/`LossyTuning`/`AlphaFilterMode` surface and
the WebP bytes it emits) are pinned by:

- `codecs/webpkit` 923 lib tests + 15 doctests (below), including exact
  encode→decode round trips and the `Fast`/`Best` tier proptests;
- `tests/webp_codec.rs` in `img` (independent libwebp decode of lossless, lossy
  alpha, gray, gray-alpha and metadata output);
- the byte-identity probe below.

## DSLP-7 Tests

### Tool availability

```
$ cargo mutants --version   -> error: no such command: `mutants`
$ cargo llvm-cov --version  -> error: no such command: `llvm-cov`
$ cargo machete --version   -> error: no such command: `machete`
$ command -v cargo-udeps    -> (absent)
```

So no mutation run was possible, and this is the read-only gap review the ticket
allows in that case. The seeded record's `cargo mutants --list` counts
(`container/writer.rs` 21, `lossy/enc_header.rs` 15, `lossy/bool_enc.rs` 59,
`container/vp8x.rs` 45, `lossless/huffman/serialize.rs` 169,
`lossless/vp8l/encode.rs` 522) came from the unmerged attempt and could not be
re-derived here. Survivor dispositions: **none**, because no mutant exists.
`cargo test --locked --manifest-path codecs/webpkit/Cargo.toml` is the guard in
the meantime (923 lib + 15 doctests + 5 more, exit 0).

### Bitstream-writer inventory

`#[test]` count includes proptest-generated tests; the merged suite reports 923
of 938 because 15 are feature-gated.

| Writer | Tests | What the tests bind |
| --- | ---: | --- |
| `lossless/bit_io/writer.rs` | 7 | Exact bytes for one/many bits, zero-width no-op, zero padding on align, write→read over an LCG, byte-identity against an independent `BitBuf` oracle, proptest round trip. |
| `lossless/huffman/serialize.rs` | 12 | Simple form (1- and 8-bit first symbol), full form, the 256-symbol boundary, code-16/18 RLE runs, the trimmed max-symbol form, and a reader-oracle round trip that asserts the reader consumes exactly the written bits. |
| `lossless/huffman/canonical.rs` | 9 | Bit reversal (table and involution), canonical order, `emit_codes` for empty/single/two-symbol alphabets, against a reversed-canonical reference. |
| `lossless/huffman/build.rs` | 6 | Empty/single/two-symbol alphabets, depth limits (Fibonacci weights, skewed 19-entry), decodability of several histograms. |
| `lossy/bool_enc.rs` | 5 | Round trip through `BoolDecoder` at skewed probabilities, resume at every partition boundary, signed literals, signed zero, mixed-probability renormalization. |
| `lossy/tokens.rs` | 9 | i4x4/Y2/WHT residuals, intra modes, segment ids, bmodes round trip; the count pass compared event-for-event and nz-context-binned against the emit pass. |
| `lossy/enc_header.rs` | 6 | Frame header in both directions, max dimensions/zero partition, control header with base quantizers, segments and loop filter. |
| `lossy/frame_header.rs` | 8 | Minimal key frame, tag subfields, scale bits, 3-byte partition size, and the five rejections. |
| `container/writer.rs` | 9 | Even/odd payload padding, exact 12-byte header, VP8X ordering and flag clearing, bare VP8/VP8L, metadata round trip, proptest. |
| `container/vp8x.rs` | 7 | Flag accessors, animation bit, parse/build round trip, canvas-minus-one, length/canvas rejection. |
| `container/mux.rs` | 7 | Chunk order per spec, alpha/animation flags, strip-to-bare, metadata replacement, truncated input. |
| `lossy/alpha.rs` | 17 | Raw and lossless ALPH both ways, smallest-candidate choice, deterministic output, tuning identity with the exhaustive search, quantization and filter-candidate bounds. |
| `lossless/vp8l/encode.rs` | 43 | Encode→decode round trips for every tier (including `Fast` and `Best` transform families), candidate selection and cost checks, sub-images, meta plans. |
| `lossy/frame.rs` | 56 | Encoder output decoded and compared with the encoder's own reconstruction across the i16x16/i4x4/chroma/SNS/segment paths. |

### Decode-verification path

- VP8L: in-crate decoder tests (`vp8l/decode.rs` 14, `decoder.rs` 32) and the
  16-directory conformance corpus (14 still `VP8L` payloads), driven both
  one-shot and by the incremental decoder with every split from
  `split_patterns`; `stream_equals_one_shot_on_conformance_fixtures` requires at
  least 10.
- VP8: four `tests/fixtures/*.vp8` blobs plus `lossy/decoder.rs` (22) and
  `lossy/decode_incr.rs` (12).
- Independent libwebp decode of webpkit's output happens only in `img`'s
  `tests/webp_codec.rs`, not in webpkit's own suite.

### Gaps (read-only findings)

1. **The conformance corpus' pixel oracle is unused.** Every fixture directory
   carries `expected.rgba` (54 blobs total: 16 `input.webp`, 16 `expected.rgba`,
   16 `meta.toml`, 6 others), but nothing in the crate reads `expected.rgba` or
   `meta.toml` (`rg` finds no reference). The corpus therefore verifies
   stream-vs-one-shot self-consistency, not correctness against the recorded
   reference. Proposed test: decode each still fixture and compare to
   `expected.rgba` under the fixture's `meta.toml` layout, with a floor on the
   fixture count.
2. **`serialized_bits` is not pinned to the writer.** `best_code_lengths`
   (`lossless/vp8l/encode.rs:2603`) prices a code as
   `Σ count·length + serialized_bits(lengths)` and picks the cheaper build; if
   `serialized_bits` (`lossless/huffman/serialize.rs:125`, doc says "exact bit
   count") drifts from `write_huffman_code`, the choice silently picks the larger
   stream. No test asserts the two are equal, and a mutant of
   `serialized_bits` would be a size regression, not a decode failure, so the
   round-trip proptests cannot see it. Proposed test:
   `serialized_bits(lengths)` equals the bits `write_huffman_code` consumes
   (the existing `round_trip` helper already knows how to measure that).
3. **Probability zero is a legal table value with no test.** `COEFFS_PROBA_0`
   ends `…, 3, 1, 0` (`lossy/constants.rs:481`), so `BoolEncoder::put_bool(0, …)`
   is reachable; the probability optimizer clamps its *output* to `1..=255`
   (`prob_opt.rs:50`) but the default table still contains 0. A zero probability
   cannot encode the opposite bit, so the invariant is "never encode the
   impossible branch". `bool_enc.rs`'s round-trip set covers 1 and 255 but not 0.
   Proposed test: encode the permitted branch at `prob = 0` (and 255) and decode
   it, and assert the encoder never asks for the impossible branch on a token
   stream that reaches that node.
4. **No independent lossy comparison inside webpkit.** The `oracle` feature's
   hooks (`lossless.rs:680+`, `lossy.rs:340+`) exist for a differential crate
   that was not imported, and `libwebp-sys` is never referenced. The only
   independent lossy check is `img`'s `tests/webp_codec.rs`.

Injected faults were not run: with no new boundary test in a read-only pass
there is nothing to prove, and the three command-level checks below stand in.

### Cost

`cargo test --locked --manifest-path codecs/webpkit/Cargo.toml` on the committed
tree: lib 290.6 s, doctests 7.2 s, remaining target 0.08 s; the first run on a
cold `codecs/webpkit/target` took 516.6 s wall. `cargo clippy ... --all-targets
-- -D warnings` warm: 1.2 s; the full recompile after a source edit took 17.3 s.

## DSLP-8 Dead code

Tools: rustc `dead_code`/`unused_imports` with the crate's allow overridden via
`RUSTFLAGS='--force-warn dead_code --force-warn unused_imports'` (`--force-warn`
cannot be overridden by attributes), once on the default lib and once on
`--features __internals,work-count,bench,rayon,image` where the internal modules
are `pub` and only crate-private items can still be reported. `cargo-machete`
and `cargo +nightly udeps` are unavailable, so dependency use was checked by
`rg`.

| Tool / configuration | Result |
| --- | --- |
| rustc `dead_code`, default lib | 22 items + 1 field + 8 `pub use` imports. All are internal-module API (`lossless::decode`, `decode_with`, `decode_rgba`, `decode_alpha`, `decode_reader`, `encode_to`, `animation::{decode_frames, decode_frames_with, CompositedFrames}`, `decoder::Decoder` + 6 methods, `lossy::{decode, decode_argb}`, `encoder::encode_to`, `LocatedImage.alpha`, `extract_metadata`, `locate_image`, `vp8l::encode::encode`, `vp8l::decode_incr::into_pixels`, `lossy::Decoder::frame_image`, the four `LossyConfig` accessors). Each is read by an inline test, or is reachable as `pub` under `__internals`, or both. |
| rustc `dead_code`, `__internals` + tooling features | 2 items: `vp8l::encode::encode` and `vp8l::decode_incr::into_pixels`, both `pub(crate)` and used only from `#[cfg(test)]` code. Retained as test helpers; they are not shipped API. |
| `cargo-machete` / `udeps` | Unavailable. `rg libwebp_sys src tests examples` finds no importer: `libwebp-sys` is an unused optional dependency behind `oracle`. `proptest`, `webpkit-samples`, `rayon` and `image` are all imported (tests, dev corpus, `forward_all`, `interop`). |

Allowlist: one entry, the crate-level
`cfg_attr(not(feature = "__internals"), allow(unreachable_pub, dead_code, unused_imports, reason = …))`.
Its reason was corrected in this pass: it claimed the tooling surface is
"exercised only under `__internals` … never from within the lib", but this
repository did not import the bench/oracle/conformance crates and the inline
tests are the actual readers. The entry now says what is true: the items are
upstream module API, unreachable from the facade, read by the inline tests (and
by external callers only under `__internals`).

Removed: nothing. No default-build finding was dead in every configuration, so
there was no behavior-free cut to make. `cross_color::sweep_blue` (private) is
used by the `bench`-gated shim and is not dead.

Walk-backs (public upstream API or feature surface; **not done**):

- `libwebp-sys` and the `oracle` feature: remove the dependency (and feature)
  if the differential oracle is never imported. Removing it changes the upstream
  feature set, so it needs the owner's decision. Recorded as Needs owner.
- Deleting any of the unused-per-build internal API listed above (decoders,
  animation, mux/metadata helpers, `Decoder`, `LossyConfig` accessors) would drop
  the upstream crate's module API. Not done; Needs owner if the module surface is
  to shrink.

## Acceptance evidence

| Command | Result |
| --- | --- |
| `cargo test --locked --manifest-path codecs/webpkit/Cargo.toml` | exit 0: 923 lib, 15 doctests, 5 others. |
| `cargo clippy --locked --manifest-path codecs/webpkit/Cargo.toml --all-targets -- -D warnings` | exit 0. |
| `make lint` (top level, `TMPDIR=.work/tmp`) | exit 0 (131.9 s cold, 30.2 s warm); webpkit produced no warning. The warnings in the log are pre-existing `img` findings outside this scope. |
| `make test` (top level, `TMPDIR=.work/tmp`) | exit 0 (165.0 s warm): every `cargo test --all-features` target ok (including `webp_codec`), `tests.test_bench` 14/14 OK, and `tests/test_package_release.py` skipped on macOS because it needs GNU tar (release CI runs it). One earlier attempt was killed by SIGTERM in a rustc child while compiling an `img` example on the same tree; the rerun above passed, and no source changed in between. |
| WebP output bytes | A scratch probe (`img::still::StillImage::encode`) reproduces the eight fixture encodes in `tests/webp_codec.rs`. Seven are deterministic and hashed identically at `a9ee354` and `f446941` (RGBA lossless/lossy, RGB, gray, gray-alpha lossless/lossy, 2x2). The eighth (PNG-with-ICC metadata) has a stable 840-byte length but varying bytes because `moxcms::ColorProfile::new_srgb().encode()` is itself nondeterministic across processes (612-byte profile, three runs, three hashes), so it cannot witness encoder identity; the metadata path is covered by the lossless suite and `tests/webp_codec.rs`. |
