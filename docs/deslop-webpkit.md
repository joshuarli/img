# Deslop: codecs/webpkit

Partial: Types (DSLP-6) and Dead code (DSLP-8) were measured and fixed where the change was proven; Tests (DSLP-7) was started and stopped before any mutant finished. Tickets DSLP-6, DSLP-7 and DSLP-8 stay open. Platform and base as in [deslop-img.md](deslop-img.md); webpkit is its own Cargo workspace, so every command runs in `codecs/webpkit` with `--locked --offline`.

## DSLP-6 Types

Baseline: `cargo test --locked --offline` failed to build before running a test. `examples/workledger-scratch.rs` and `examples/workledger-lossy-scratch.rs` import `webpkit::work_count`, which exists only under the `work-count` feature, and the local manifest declared no `required-features` (error E0432). The ticket's acceptance command therefore could not pass, and `docs/WEBP-IMPORT.md` claiming it did was stale. Commit `29f1703` adds the two `[[example]]` entries. After it: 923 lib tests, 15 doctests and 5 more pass (`cargo test --locked --offline`, 17 min wall on a host at load 30 to 60; the lib suite alone is 896 s).

Strict candidates, measured with `cargo clippy --locked --offline --lib --features rayon -- -W <lint>`:

| Candidate | Hits | Decision |
| --- | --- | --- |
| `clippy::wildcard_enum_match_arm` | 8 | Rejected, except two real hatches: `select_from_estimates` and `select_provisional` filtered `BestTask::Floor` and `Palette` with `matches!` and then ended a second `match` in `_ => unreachable!()`. Fixed in `503cd94`: one exhaustive `match`, same order of effects. The other six are `_ => None` fallbacks over foreign or documented sets. |
| `clippy::unwrap_used` / `expect_used` | 2 / 6 | Rejected. All eight are invariants of a just-built structure (`heap.pop().expect("heap has at least two")` in `huffman/build.rs`, a poisoned-lock `unwrap` in `vp8l/encode.rs`). |
| `clippy::cast_possible_truncation` / `cast_sign_loss` | 13 / 2 | Rejected; the rest are already `#[expect]`ed with a reason. |
| `clippy::indexing_slicing`, `as_conversions` | 1187, 425 | Rejected (recorded false-positive classes). |
| `clippy::fn_params_excessive_bools`, `struct_excessive_bools` | 2, 1 | Rejected. `encode_stream`, `search_lz77_variant` and `Breadth` thread encoder switches the call graph depends on; a type change would be a hot-path refactor with no defect to witness. |
| `clippy::needless_pass_by_value` | 1 | Rejected. |
| `unfulfilled_lint_expectations` (rustc) | 5 to 8 by feature set | Adopted as a cleanup: three `#[expect]` entries never fire under default, `rayon`, `image` and `work-count` builds (`type_complexity` on `search_lz77_variant`, `similar_names` on the test-only meta plan, `inconsistent_struct_constructor` on the reconstruct tests). Removed in `a70e771`; clippy reports no new warning in the default and `work-count` configurations. Three more (`unnecessary_wraps` in `lossless.rs:243`, `unused_self` in `lossy/decoder.rs:340`, `missing_const_for_fn` in `lossless/vp8l/backref.rs:486`) are unfulfilled in one configuration only, so they stay. |

Adopted settings: none. The standalone manifest has no `[lints]` table (upstream's workspace inheritance was dropped at import), and the crate keeps `#![deny(unsafe_code)]` with the one NEON kernel module granted locally, so `unsafe_code = "forbid"` is not available.

Escape hatches in `src/` (`unsafe`, `#[allow(`, `#[expect(`, `.unwrap()`, `.expect(`, `unreachable!`; lines): 989 to 986.

## DSLP-7 Tests

Not run to a result. Baseline recorded above. `cargo mutants --list` for the bitstream writers: `container/writer.rs` 21, `lossy/enc_header.rs` 15 or more, `lossy/bool_enc.rs` 59, `container/vp8x.rs` 45, `lossless/huffman/serialize.rs` 169, `lossless/vp8l/encode.rs` 522. A 54-mutant shard (`writer.rs`, `enc_header.rs`, the `Dimensions` constructors) was started in a scratch copy (`cargo-mutants` cannot copy `../webpkit-samples` and the conformance fixtures, so the copy keeps all three crates side by side) and stopped by the coordinator's wind-down before it reported a result. Coverage: needs environment (see [deslop-img.md](deslop-img.md)).

Injected fault: not run. Candidate for the next session: drop the height test from `Dimensions::new` (`valid(width) && valid(height)`, `src/image.rs`), which `cargo mutants` does not generate.

## DSLP-8 Dead code

| Tool | Result |
| --- | --- |
| rustc `dead_code`, crate-wide `allow` lifted | The crate allows `dead_code` and `unused_imports` unless the `__internals` feature is on. In a scratch copy without that allow, default features report 22 unused items and 8 unused imports, and `--features rayon` the same; nearly all are internal wrappers (`lossless::decode`, `decode_with`, `decode_rgba`, `Decoder`, `lossy::decode`) kept for the tooling crates that were not imported. |
| rustc `dead_code` under `work-count` (which enables `__internals`) | Three private items: `cross_color::sweep_blue` and `vp8l::encode::encode` (both used only by tests and bench shims, so not dead), and `I4X4_SUBMODE_COST` in `lossy/frame.rs`. |
| `cargo-machete` | `libwebp-sys`, an optional dependency behind the `oracle` feature. No source file imports it (`rg libwebp_sys` is empty); the `oracle` hooks only gate test entry points, and the oracle test crate was not imported. It is also in `Cargo.lock` of `img`. Dropping the feature changes the upstream feature set, so it is a consumer question. |
| `cargo udeps` | Needs environment. |

Removed: `I4X4_SUBMODE_COST`, a `const` with no reader anywhere in the repository (`rg I4X4_SUBMODE_COST` finds only its definition); intra-4x4 sub-block modes are charged by `i4_bmode_cost`. Commit `7e433e6`; the neighbouring doc claimed the 16x16 constant covered sub-block modes and now says what the code does. The lossy and lossless selection tests pass (`cargo test --locked --offline --lib -- transform_selection provisional best_ i16x16_mode_cost`). The full webpkit suite was last run before this change and the `BestTask` change.

Public webpkit API that `img` never calls (`src/still/webp.rs` uses `Encoder`, `Effort`, `Dimensions`, `ImageRef`, `PixelLayout`, `Metadata`, `LossyTuning`, `AlphaFilterMode`): the decoders, animation (`AnimationEncoder`, `AnimationMux`, `AnimationOptimizer`), `Chunk`, `YuvImage`, `read_metadata`/`write_metadata`. These are upstream API: walk-backs, not cuts.

Fixtures: `codecs/webpkit-lossless-conformance/fixtures/decode/` (54 blobs) is read by `lossless/vp8l/decode_incr.rs:640,671`, `lossless/decoder.rs:1139` and the `lib.rs:1043` doctest; `codecs/webpkit-samples` is a dev-dependency. Neither is dead.

## Findings not fixed

- P4, `libwebp-sys` is an unused optional dependency of `webpkit` (feature `oracle`) and appears in `img`'s `Cargo.lock`; decide whether to keep the upstream feature.
- P4, `Dimensions::new` and the VP8L/VP8 writers have no recorded mutation result (DSLP-7).
- P4, 62 default clippy warnings on this nightly (`needless_range_loop` 15, `too_many_arguments` 10, `collapsible_if` 6, `needless_borrow` 6) in a manifest with no lint table; `make lint` does not run clippy on webpkit.
