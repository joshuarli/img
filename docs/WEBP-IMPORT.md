# WebP source import and license boundary

## Snapshot

- Origin: https://github.com/P4suta/webpkit
- Pinned upstream commit: `cc1311ebf5a7b287b22f8ae2efb22aae90da49ef` (2026-11-08).
- Original path: `crates/webpkit/`.
- Import path: `codecs/webpkit/`.
- Original license: **MIT OR Apache-2.0**, with complete original texts retained in `codecs/webpkit/LICENSE-MIT` and `codecs/webpkit/LICENSE-APACHE`.
- Source tree: the original `src/` files, `README.md`, `CHANGELOG.md`, and `examples/roundtrip.rs` are imported without changes. The local `Cargo.toml` is a **new, simplified standalone manifest** replacing the upstream workspace-inherited manifest so the codec can be isolated from `rimage`'s production dependency graph.
- The upstream remains the author of the imported source; modifications here must retain copyright/license notices and be attributed. A source import is **not** a GitHub network fork and does not retain Git ancestry. Do not claim it does.
- The import currently excludes the upstream workspace's larger benchmark/test/tooling crates and binary fixture files. Restore the exact upstream fixtures and independent test harness from the pinned commit before asserting full upstream test parity. Do not treat their absence as test success.

## Permissive boundary and oracles

Our production code may derive from Webpkit (MIT/Apache-2.0) and, with its BSD-style notices and patent/license caveats observed, libwebp. However, the target is an idiomatic pure-Rust codec that needs no C/FFI system library. Do **not** introduce c2rust translation as the default strategy: this snapshot already implements VP8 and VP8L.

Use libwebp, cwebp/dwebp, and zenwebp only as **development-only independent behavioral/measurement oracles**. The current zenwebp release has AGPL/commercial terms: do not copy, adapt, translate or paraphrase its AGPL source, tables, tests or private implementation into this permissive codec. Run its published binaries/API in a separate disposable oracle harness where legal, with version and license recorded. Test inputs/outputs and observable behavior can be compared; keep the record of implementation provenance independent of the AGPL source.

libwebp's `COPYING` and `PATENTS` should be reviewed before incorporating any of its source. A C->Rust translation of libwebp would be a derivative port **not** a clean-room rewrite. Prefer a spec-first enhancement of Webpkit; port specific verified permissive routines only where evidence justifies them.

This import **does not yet replace** `zenwebp` in `rimage`. The current `Cargo.toml` and `Cargo.lock` are intentionally untouched, and the current binaries continue carrying AGPL/GPL dependencies. Consult `docs/WEBP-HILLCLIMB.md` for replacement gates.
