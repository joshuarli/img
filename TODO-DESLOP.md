Prefix: DSLP
Next: 13

Deslop campaign backlog ([plan](https://github.com/laputa-systems/stuff/blob/main/docs/campaigns/deslop.md), tier C). One agent works this file with the `deslop` skill: take the lowest-numbered open ticket whose area does not start with `Needs owner ·`. After DSLP-5, work the open P1 and P2 findings, then fill in `## Report` below.

## Report

Step 1, Ground, done at a180d19 (the seeded base ba0b9b1 was stale). Code lines outside `vendor/` (non-blank, non-comment) are Rust 48.3k, of which `codecs/webpkit` is 37.7k, plus Python 2.1k: 50.4k, not the seeded 43.5k and 0.2k, which counted differently, so DSLP-3 to DSLP-5 now cover the img crate (about 13k lines) and DSLP-6 to DSLP-8 cover `codecs/webpkit`. Performance gate: yes, decode and encode are the hot paths; the workload is `scripts/bench` (AGENTS.md). PR CI now runs `make lint` and `make test` as separate steps on macOS; the Linux musl job runs `make ci`, which runs the same checks. Warm `make test` 56 s, cold 39 min; `make lint` warm 53 s, cold 29 min (busy host, observations). Two CI breaks were found by running the checks in the Alpine CI shape and fixed: the Alpine image has busybox tar, so the package test now runs only where GNU tar exists (`release.yml` still runs it), and `tests/test_bench.py` needs git, which refuses a root-owned workspace as the unprivileged verifier, so `scripts/ci-check` marks the workspace safe. Fleet lint findings seeded in DSLP-1 (linux-musl-toolchain, rust-collections, audit-upgrades and others) were not re-measured here; the repomod fleet lint owns them.

Step 2, Diagnose, done: reviewed decode of untrusted input (dimension and allocation limits, offset arithmetic, malformed headers, output files). A scratch mutation sweep (13 seeds across all four formats, 700 mutants each, `inspect` plus two `convert` modes; 30,900 runs) found no crash, hang or allocation above 13 MB. Crafted inputs produced three defects (DSLP-9 to DSLP-11). WebP container sizes, PNG chunk and text bounds, AV1 frame limits and the publication path held up against crafted inputs and code reading.

## Tickets

### DSLP-3 · P2 · Types: strict profile and typing findings, img crate

- **Area:** deslop · types
- **Where:** src/, tests/, benches/, examples/, codecs/*-sys, scripts/ (about 13k lines; public contracts: the still, optimizer, publication, limits and error library API and the CLI JSON schema)
- **Problem:** The strict-profile candidates in the deslop skill tools reference have not been measured on the img crate. The WebP encoder snapshot codecs/webpkit is a separate slice (DSLP-6).
- **Recommendation:** Run the deslop Types step on the img crate only. Consumers to run against the candidate: tests/*.rs, benches/, examples/, src/still/webp.rs for any webpkit-facing type. Accept: `make lint` and `make test` clean at the adopted profile, escape hatches not increased, a witness for each fixed contract.
- **Fix with:** deslop
- **Effort:** M

### DSLP-4 · P2 · Tests: coverage, boundary gaps, mutation, img crate

- **Area:** deslop · tests
- **Where:** tests/, src/ unit tests, scripts/ tests, CI (img crate; the decode boundaries in DSLP-9 to DSLP-11 are the first uncovered boundaries)
- **Problem:** Coverage and mutation results are unmeasured; no deslop mutation lane exists; corrupt-input decoding has no fuzz or crafted-input test beyond the cases in tests/jpeg_codec.rs, png_codec.rs and avif_codec.rs.
- **Recommendation:** Run the deslop Tests step on the img crate: critical contracts are src/still/ decode and limits, src/publication.rs, src/runtime.rs admission, src/exit.rs. Consumers: the CLI tests/cli.rs and the library tests. Accept: no undispositioned survivor in the critical modules, warm `make test` (56 s) up at most 20% or 30 s, the lane green where the tools are mirrored.
- **Fix with:** deslop
- **Effort:** L

### DSLP-5 · P2 · Dead code: tools and debloat, img crate

- **Area:** deslop · dead code
- **Where:** src/, tests/, benches/, examples/ (17 scratch-*.rs examples), scripts/, codecs/*-sys
- **Problem:** No dead-code scan has run. rustc already reports one unused function (checked_sdr) and the examples/scratch-*.rs files are experiment history.
- **Recommendation:** Run the deslop Dead code step on the img crate. Consumers to check per removal: tests, benches, examples, docs that name the item, scripts/bench and benchmarks/. Accept: tools clean or allowlisted with reasons; walk-backs filed as Needs owner tickets; `make lint` and `make test` pass.
- **Fix with:** deslop
- **Effort:** M

### DSLP-6 · P2 · Types: strict profile and typing findings, codecs/webpkit

- **Area:** deslop · types
- **Where:** codecs/webpkit/ (37.7k lines, WebP encoder snapshot of P4suta/webpkit cc1311e with local hillclimb changes)
- **Problem:** The webpkit snapshot is 75 percent of the non-vendored code and has not been measured against the strict-profile candidates.
- **Recommendation:** Run the deslop Types step on codecs/webpkit only. Contracts: its public encoder API as used by src/still/webp.rs and the WebP bytes it emits. Consumers: src/still/webp.rs, tests/webp_codec.rs, codecs/webpkit-lossless-conformance fixtures. Accept: `cargo test --locked --manifest-path codecs/webpkit/Cargo.toml` and `make test` pass at the adopted profile with escape hatches not increased.
- **Fix with:** deslop
- **Effort:** M

### DSLP-7 · P2 · Tests: coverage and mutation, codecs/webpkit

- **Area:** deslop · tests
- **Where:** codecs/webpkit/ tests and the webpkit-lossless-conformance fixtures
- **Problem:** Coverage and mutation of the 911 library tests recorded in docs/WEBP-IMPORT.md are unmeasured, and the encoder changes made during the hillclimb have no recorded mutation result.
- **Recommendation:** Run the deslop Tests step on codecs/webpkit, bounded to the bitstream writers and the decode-verification path; shard the mutation run (it is large). Consumers: src/still/webp.rs, tests/webp_codec.rs. Accept: no undispositioned survivor in the scoped modules; `cargo test --locked --manifest-path codecs/webpkit/Cargo.toml` and `make test` pass.
- **Fix with:** deslop
- **Effort:** L

### DSLP-8 · P2 · Dead code: tools and debloat, codecs/webpkit

- **Area:** deslop · dead code
- **Where:** codecs/webpkit/ and the webpkit-samples and webpkit-lossless-conformance crates
- **Problem:** No dead-code scan has run on the snapshot; img uses only the encoder entry points, so unused decoder and optimizer surface may be large.
- **Recommendation:** Run the deslop Dead code step on codecs/webpkit. A public item of the upstream crate API is a contract: removals that drop upstream API are walk-backs, filed as Needs owner tickets. Consumers to check: src/still/webp.rs, tests/webp_codec.rs, examples/. Accept: tools clean or allowlisted with reasons; `make lint` and `make test` pass.
- **Fix with:** deslop
- **Effort:** M

### DSLP-11 · P3 · JPEG pixel and dimension limits are checked after the decode has allocated

- **Area:** decode · jpeg
- **Where:** src/still/jpeg.rs decode (structure.width and height reach libjpeg_turbo_sys::decode before StillImage::from_samples calls check_dimensions); src/still.rs describe; src/runtime.rs reservation
- **Problem:** PNG, WebP and AVIF call check_dimensions on the header before allocating; JPEG does not. A flat 8192x8192 grayscale progressive JPEG of 131,212 bytes is refused with `pixel count 67108864 exceeds limit 16777216`, but only after libjpeg-turbo allocated and zeroed its coefficient arrays and the output buffer: 210 MB peak RSS (debug build of a180d19), against 7 MB for a WebP whose VP8X canvas declares 16384x16384, which is refused at the header. The protection against larger JPEGs is only the admission reservation, which on this host (14.7 GB budget) rejects 16384x16384 but would admit it on a host with a budget above about 20 GB and then allocate several GB before the 16 MP check fires. JPEG describe() also never applies the limits, so inspect prints 65500x65500.
- **Recommendation:** Call check_dimensions(structure.width, structure.height) in jpeg::decode before libjpeg_turbo_sys::decode, and keep describe() as is if inspecting oversized headers is intended (state it in docs/CLI.md). Extend tests/jpeg_codec.rs with a header-only oversized JPEG that must fail with ErrorKind::Limit without a large allocation.
- **Fix with:** security
- **Effort:** S
