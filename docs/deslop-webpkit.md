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
| Tests (DSLP-7) | Mutation still not run: `cargo-mutants` is absent (nor `cargo-llvm-cov` or `cargo-machete`), so no survivor exists to disposition and the step is the read-only review. Three of the four reviewed gaps are now closed by tests (corpus vs `expected.rgba`, cost vs emitted bits, probability extremes), each proven with an injected fault; the fourth needs unsafe FFI and is recorded as not testable in-crate. |
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
| `lossless/huffman/serialize.rs` | 13 | Simple form (1- and 8-bit first symbol), full form, the 256-symbol boundary, code-16/18 RLE runs, the trimmed max-symbol form, a reader-oracle round trip that asserts the reader consumes exactly the written bits, and `serialized_bits` equal to `BitWriter::bit_len()` after the write. |
| `lossless/huffman/canonical.rs` | 9 | Bit reversal (table and involution), canonical order, `emit_codes` for empty/single/two-symbol alphabets, against a reversed-canonical reference. |
| `lossless/huffman/build.rs` | 6 | Empty/single/two-symbol alphabets, depth limits (Fibonacci weights, skewed 19-entry), decodability of several histograms. |
| `lossy/bool_enc.rs` | 6 | Round trip through `BoolDecoder` at skewed probabilities, resume at every partition boundary, signed literals, signed zero, mixed-probability renormalization, and the 0/255 probability extremes on both branches. |
| `lossy/tokens.rs` | 9 | i4x4/Y2/WHT residuals, intra modes, segment ids, bmodes round trip; the count pass compared event-for-event and nz-context-binned against the emit pass. |
| `lossy/enc_header.rs` | 6 | Frame header in both directions, max dimensions/zero partition, control header with base quantizers, segments and loop filter. |
| `lossy/frame_header.rs` | 8 | Minimal key frame, tag subfields, scale bits, 3-byte partition size, and the five rejections. |
| `container/writer.rs` | 9 | Even/odd payload padding, exact 12-byte header, VP8X ordering and flag clearing, bare VP8/VP8L, metadata round trip, proptest. |
| `container/vp8x.rs` | 7 | Flag accessors, animation bit, parse/build round trip, canvas-minus-one, length/canvas rejection. |
| `container/mux.rs` | 7 | Chunk order per spec, alpha/animation flags, strip-to-bare, metadata replacement, truncated input. |
| `lossy/alpha.rs` | 17 | Raw and lossless ALPH both ways, smallest-candidate choice, deterministic output, tuning identity with the exhaustive search, quantization and filter-candidate bounds. |
| `lossless/vp8l/encode.rs` | 43 | Encode→decode round trips for every tier (including `Fast` and `Best` transform families), candidate selection and cost checks, sub-images, meta plans. |
| `lossy/frame.rs` | 56 | Encoder output decoded and compared with the encoder's own reconstruction across the i16x16/i4x4/chroma/SNS/segment paths. |
| `tests/conformance.rs` (integration) | 1 | Decodes all 16 `level = "must"` corpus fixtures through the public `webpkit::decode` and compares every pixel to the recorded `expected.rgba`; floor of 16, and every fixture must still be `must`. |

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

### Gaps: three closed by tests, one not testable in-crate

The first read-only review found four gaps. Three are now covered, and each test
was proven with an injected fault. The corpus' `meta.toml` carries no layout key
— it holds `feature`, `level` and `note` — so the conformance test uses the
convention those notes state: `expected.rgba` is `dwebp -pam` output with its
header stripped, i.e. RGBA8 in file order. The test requires `level = "must"` for
every directory and checks `expected.rgba.len() == width * height * 4` against
the decoded image before comparing bytes, so a wrong layout or a shrunken corpus
fails.

1. **Corpus pixel oracle** — `tests/conformance.rs`: decodes all 16 fixtures
   through the public `webpkit::decode` and compares every byte to
   `expected.rgba`. The animation's reference is its first composited frame,
   which is what `decode` returns for an animation; the VP8X fixture's reference
   is the same still pixels. Floor: `checked >= 16`. Result: 16/16 match, no
   product defect.
2. **`serialized_bits` vs the writer** — `serialize.rs::serialized_bits_equals_
   the_written_bits`: for seven length vectors covering the empty, simple (1- and
   8-bit), full, trimmed (280-symbol trailing zero) and all-length-8 forms,
   `BitWriter::bit_len()` after `write_huffman_code` equals `serialized_bits`.
   The test also asserts the 280-symbol case takes the trimmed branch, so the
   trimmed cost is exercised.
3. **Probability extremes** — `bool_enc.rs::probability_extremes_round_trip_
   both_branches`: at `prob = 0` and `prob = 255`, both branches round-trip
   through `BoolDecoder` after a renormalizing prefix. All four combinations
   pass, so a one-unit split still carries the decision.
4. **Independent lossy comparison (not testable in-crate).** A differential
   check needs `libwebp-sys`'s FFI, and the crate is `#![deny(unsafe_code)]`
   with the owner's single grant (the NEON match kernel); the `oracle` hooks
   exist for a crate that was not imported. The independent check stays in
   `img`'s `tests/webp_codec.rs` (libwebp decodes lossless, lossy-alpha, gray and
   gray-alpha output). Moving it in-crate needs an unsafe FFI module, i.e. a
   grant/API change; recorded as Needs owner, not done.

#### Injected-fault evidence

Each fault was applied to the protected code, the named test was run alone, and
the fault was reverted (`git diff` empty afterwards); no fault was committed.

| Fault | Change | Command | Observed failure |
| --- | --- | --- | --- |
| Decoder output | `vp8l/decode.rs::decode`: `argb[0] ^= 1` after the image stream | `cargo test --test conformance` | `all_transparent: decoded pixels differ from the libwebp reference`; first pixel blue 66 vs 67 |
| Cost model | `serialize.rs::serialized_bits`: `1 +` → `2 +` in the full-form cost | `cargo test --lib serialized_bits_equals_the_written_bits` | `lengths [1, 2, 3, 3]` left 30, right 31 |
| Bool split | `bool_enc.rs::put_bool`: `1 +` → `2 +` in `split` | `cargo test --lib probability_extremes_round_trip_both_branches` | `prob 0, bit false` decoded `true` |

### Cost

`cargo test --locked --manifest-path codecs/webpkit/Cargo.toml` with
`CARGO_TARGET_DIR` under `.work/tmp`: lib 267.1 s, conformance 0.02 s, doctests
7.7 s, remaining target 0.11 s. A cold build of that target directory is the
long pole; `cargo clippy --all-targets -- -D warnings` took 107 s to compile it
and is near-instant warm.

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
| rustc `dead_code`, default lib | 22 diagnostics + 1 field + 8 unused-import diagnostics covering 28 re-exported names. No item is dead in every configuration; dispositions below. |
| rustc `dead_code`, `__internals` + tooling features | 2 items: `vp8l::encode::encode` and `vp8l::decode_incr::into_pixels`, both `pub(crate)` and used only from `#[cfg(test)]` code. Retained as test helpers; they are not shipped API. |
| `cargo-machete` / `udeps` | Unavailable. `rg libwebp_sys src tests examples` finds no importer: `libwebp-sys` is an unused optional dependency behind `oracle`. `proptest`, `webpkit-samples`, `rayon` and `image` are all imported (tests, dev corpus, `forward_all`, `interop`). |

Disposition per finding (default lib build):

- Read by an inline test, retained: `lossless::{decode, decode_with, decode_rgba,
  decode_alpha, encode_to}`; `lossless::animation::{decode_frames,
  decode_frames_with}`; `container::reader::{extract_metadata, locate_image}`;
  `LocatedImage::alpha`; `vp8l::encode::encode`;
  `vp8l::decode_incr::into_pixels`; `lossy::{decode, decode_argb}`;
  `lossy::encoder::encode_to`; `LossyConfig::{quality, effort}`.
- Read nowhere, but upstream module API reachable under `__internals`, retained:
  `lossless::decode_reader`; `lossless::animation::CompositedFrames`;
  `lossless::decoder::Decoder` with `new`, `with_options`, `read_info`, `decode`,
  `into_frames`, `fill`; `LossyConfig::{metadata, policy}`;
  `lossy::Decoder::frame_image` (also carries a documented `#[expect(unused_self)]`
  for API parity).
- Dev-only shim: `cross_color::sweep_blue`, reached only through the
  `feature = "bench"` forwarder; not dead.
- Unused imports (8 diagnostics, 28 names): the `pub use` re-export lines in
  `lossless.rs:127,129,135,138` and `lossy.rs:190,191,194,196`. They are the
  flattened module API itself (`Effort`, `Metadata`, `PixelLayout`, the animation
  types, `MetadataPolicy`, `encode_to`, `encode_vp8`, the stream vocabulary),
  consumed as re-exports rather than by name inside the library.

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
| `cargo test --locked --manifest-path codecs/webpkit/Cargo.toml` (`CARGO_TARGET_DIR` under `.work/tmp`) | exit 0: 925 lib (923 + the two new unit tests), 1 conformance integration test, 15 doctests, 5 others. |
| `cargo clippy --locked --manifest-path codecs/webpkit/Cargo.toml --all-targets -- -D warnings` | exit 0, including the new integration test. |
| `make lint` (top level, `TMPDIR=.work/tmp`) | exit 0 (14.4 s warm); webpkit produced no warning. The warnings in the log are pre-existing `img` findings outside this scope. |
| `make test` (top level, `TMPDIR=.work/tmp`) | exit 0: 18 `test result: ok` targets (including `webp_codec`) and `tests.test_bench` 14/14 OK; `tests/test_package_release.py` is skipped on macOS because it needs GNU tar (release CI runs it). One earlier attempt on the previous tree was killed by SIGTERM in a rustc child; the rerun passed with no source change in between. |
| New tests and faults | `--test conformance`: 1 passed; `--lib serialized_bits_equals_the_written_bits`: 1 passed; `--lib probability_extremes_round_trip_both_branches`: 1 passed. Each fails only when its recorded fault is injected (table above), and the fault runs are not part of this tree. |
| WebP output bytes | A scratch probe (`img::still::StillImage::encode`) reproduces the eight fixture encodes in `tests/webp_codec.rs`. Seven are deterministic and hashed identically at `a9ee354`, `f446941` and the final tests tree (RGBA lossless/lossy, RGB, gray, gray-alpha lossless/lossy, 2x2). The eighth (PNG-with-ICC metadata) has a stable 840-byte length but varying bytes because `moxcms::ColorProfile::new_srgb().encode()` is itself nondeterministic across processes (612-byte profile, three runs, three hashes), so it cannot witness encoder identity; the metadata path is covered by the lossless suite and `tests/webp_codec.rs`. |
