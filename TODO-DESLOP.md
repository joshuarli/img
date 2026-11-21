Prefix: DSLP
Next: 13

Deslop campaign backlog ([plan](https://github.com/laputa-systems/stuff/blob/main/docs/campaigns/deslop.md), tier C). One agent works this file with the `deslop` skill: take the lowest-numbered open ticket whose area does not start with `Needs owner ·`. After DSLP-5, work the open P1 and P2 findings, then fill in `## Report` below.

## Report

Step 1, Ground, done at a180d19 (the seeded base ba0b9b1 was stale). Code lines outside `vendor/` (non-blank, non-comment) are Rust 48.3k, of which `codecs/webpkit` is 37.7k, plus Python 2.1k: 50.4k, not the seeded 43.5k and 0.2k, which counted differently, so DSLP-3 to DSLP-5 now cover the img crate (about 13k lines) and DSLP-6 to DSLP-8 cover `codecs/webpkit`. Performance gate: yes, decode and encode are the hot paths; the workload is `scripts/bench` (AGENTS.md). PR CI now runs `make lint` and `make test` as separate steps on macOS; the Linux musl job runs `make ci`, which runs the same checks. Warm `make test` 56 s, cold 39 min; `make lint` warm 53 s, cold 29 min (busy host, observations). Two CI breaks were found by running the checks in the Alpine CI shape and fixed: the Alpine image has busybox tar, so the package test now runs only where GNU tar exists (`release.yml` still runs it), and `tests/test_bench.py` needs git, which refuses a root-owned workspace as the unprivileged verifier, so `scripts/ci-check` marks the workspace safe. Fleet lint findings seeded in DSLP-1 (linux-musl-toolchain, rust-collections, audit-upgrades and others) were not re-measured here; the repomod fleet lint owns them.

Step 2, Diagnose, done: reviewed decode of untrusted input (dimension and allocation limits, offset arithmetic, malformed headers, output files). A scratch mutation sweep (13 seeds across all four formats, 700 mutants each, `inspect` plus two `convert` modes; 30,900 runs) found no crash, hang or allocation above 13 MB. Crafted inputs produced three defects (DSLP-9 to DSLP-11). WebP container sizes, PNG chunk and text bounds, AV1 frame limits and the publication path held up against crafted inputs and code reading.

## Tickets

### DSLP-4 · P2 · Tests: coverage, boundary gaps, mutation, img crate

- **Area:** deslop · tests
- **Where:** tests/, src/ unit tests, scripts/ tests, CI (img crate; the decode boundaries in DSLP-9 to DSLP-11 are the first uncovered boundaries)
- **Problem:** Coverage and mutation results are unmeasured; no deslop mutation lane exists; corrupt-input decoding has no fuzz or crafted-input test beyond the cases in tests/jpeg_codec.rs, png_codec.rs and avif_codec.rs.
- **Recommendation:** Run the deslop Tests step on the img crate: critical contracts are src/still/ decode and limits, src/publication.rs, src/runtime.rs admission, src/exit.rs. Consumers: the CLI tests/cli.rs and the library tests. Accept: no undispositioned survivor in the critical modules, warm `make test` (56 s) up at most 20% or 30 s, the lane green where the tools are mirrored.
- **Fix with:** deslop
- **Effort:** L

