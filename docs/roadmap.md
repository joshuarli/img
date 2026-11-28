# Roadmap

This is the single plan for this repository: all current and future work, and the handoff for resuming it. Campaigns are sections below, and their items interleave in **Sequence**. "Resume the roadmap" means continue from **Status**.

## Status

- Vacation 2026-11-27: `.work/` deleted. Resume with "resume the roadmap"; no slice is in progress.
- Last accepted: `6df34a9` (master, clean, pushed). No green check is recorded at that revision; run `make lint` and `make test` before claiming one. WebP's last recorded green integration was `1057014`: 923 library tests, 373 lossless tests at 4096 cases, musl checks and independent pixel-exact decoding.
- Rebuild before the first gate, in order:
  1. `mkdir -p .work/tmp && TMPDIR=$PWD/.work/tmp CARGO_BUILD_JOBS=4 scripts/bench build --base f0ceea6` — builds `target/dist/img`, `.work/wt/bench-base-f0ceea6aae09` and `.work/bench/bin/img-f0ceea6aae093ba50d747dca51737458257a2caf`.
  2. `python3 -I scripts/qualification-corpus .work/qcorpus` — needs network access to the manifest's source URLs.
  3. `python3 -I benchmarks/codecs/batch-sets.py .work/qcorpus target/dist/img .work/sets > .work/sets.sha`, then rebuild `.work/sets/mix3` with the loop in `archive/img-scratch:tmp/qualify.sh`.
  4. `IMG_BASE=$PWD/.work/wt/bench-base-f0ceea6aae09 LOOPS_SCRATCH=$PWD/.work/tmp/loops TMPDIR=$PWD/.work/tmp sh benchmarks/codecs/loops-build.sh`.
  5. Reference and variant binaries: `archive/img-scratch:tmp/refbuilds.sh` and `tmp/variants.sh` from pins `5424df8`, `b89205c`, `77eb60a`; `img-avifprobe` from the committed `benchmarks/results/codecs/avif/search-probe-2026-11-22.patch`. `img-sysalloc` had no committed source change and is gone; redo that A/B only by reintroducing the change.
  6. First gate: `perf-gate-ready` before and after any timing, then `scripts/bench run --tier scenario --base f0ceea6` as in `benchmarks/results/checkpoints/2026-11-20/README.md`.
- Recoverable: none; every deleted path is rebuildable from the pins and recipes above. Scratch recipes and q1-q6 run records are on the pushed orphan `archive/img-scratch`; no credentials were stored there.
- Parked `experiment/webp-match-chunks`: unfinished and predates the NEON match kernel; retry only if profiling still identifies match comparison as a wall-time hotspot. Parked `perf/dav1d-decode`: the asm build differs from upstream meson on 26 of 31 fixtures by up to 7 levels; retry when the asm build is bit-identical. Remote-only experiment branches hold no local work.
- Next WebP slice: profile current production code. Codec campaign next experiments are listed in Sequence.

## Sequence

| ID | Campaign and slice | Owned files | Prerequisites | Acceptance command |
| --- | --- | --- | --- | --- |
| W1 | WebP: profile mm-logo-alpha L1 and coffee q90 | `codecs/webpkit/`, `benchmarks/` | Symbolized dist build and sampling profiler | `perf-gate-ready` before and after measurement |
| W2 | WebP: one bounded throughput experiment, at most one profile-justified follow-up | `codecs/webpkit/` | W1 | WebP checks below and gated interleaved measurements |
| W3 | WebP: paired evaluation of sub-image LZ77, sub-image optimal parsing and cluster reassignment | `codecs/webpkit/` | W2 or no plausible target | WebP checks below and gated interleaved measurements |
| W4 | WebP: compare direct lossy, direct lossless and scored workflow against cwebp | `benchmarks/`, `examples/`, `BENCHMARKS.md` | W3; direct-codec harness | `perf-gate-ready` before and after measurement |
| W5 | WebP: publish tables, tradeoffs and remaining gaps | `BENCHMARKS.md`, `README.md` | W4 | `make benchmark-verify` |
| C1 | Codecs: profile PNG Best's level-11 matchfinder across filter trials | `vendor/libdeflater/`, `vendor/oxipng/`, `benchmarks/` | Gated baseline | `scripts/bench run --only workflow-scored --base f0ceea6` |
| C2 | Codecs: investigate bounded column stripes for streamed scoring | `vendor/fast-ssim2/`, `benchmarks/` | Preserve summation order | `cargo test --locked --all-features --test score_planar` |
| C3 | Codecs: profile JPEG AC refinement or investigate equal-size scan scripts | `vendor/libjpeg-turbo/`, `benchmarks/` | Large progressive fixtures | `make decoder-reference-check` |
| C4 | Codecs: measure size-gated libdeflate PNG decoding | `src/still/`, `benchmarks/` | Large single-IDAT fixtures | `make decoder-reference-check` |
| C5 | Codecs: profile AVIF conversion and identity-check a dav1d refresh separately | `src/still/`, `codecs/`, `benchmarks/` | Large 4:4:4 fixtures | `make decoder-reference-check` |
| C6 | Codecs: investigate holdout AVIF byte regression using development data | `src/optimizer.rs`, `benchmarks/` | Log quantizers on synthetic-17 | `scripts/bench run --only workflow-scored --base f0ceea6` |
| C7 | Codecs: qualify batch mixtures, alpha and allocator A/B | `src/`, `benchmarks/` | Fixed `IMG_MEMORY_BUDGET` | `make bench TIER=scenario` |

## Campaign: WebP hillclimb

### Goal and state

Close the speed and memory gap with cwebp without materially regressing compressed size, publish measured tradeoffs and close. Overall parity is not a precondition for closing. The backend remains experimental. Profile mm-logo-alpha codec L1 lossless and coffee q90 direct lossy, optionally coffee min-80 workflow. Record self-time shares; operation counts alone do not choose a target. Inspect generated code before proposing unsafe code.

### Decisions

Freeze candidate families, finer grids, iterative cost models and pruning thresholds. Preserve effort monotonicity, pure-Rust runtime, zenwebp only as a separately compiled oracle, exact hidden RGB, score thresholds, lazy-L6 selection, Fast's ten candidate slots and L6 bands. Report codec Fast separately from product Fast. Keep `deny(unsafe_code)` with local exemptions and scalar `no_std + alloc`; `target-cpu=native` is limited to the aarch64-apple-darwin host build.

Throughput changes must be byte-identical, except removal of a weak search addition may grow any named output at its level by at most 0.25%. After removal, cwebp `-z9` gaps must remain within camera 1.0%, coffee 1.5% and mm-logo-alpha 5.0%.

### Baseline

Recorded, not a fresh paired run: lossless L9 camera 124,614 bytes versus cwebp 123,720; coffee 340,156 versus 336,030; L1 mm-logo-alpha 24,720 versus 23,696. Direct lossy is approximately 1.3–1.7× cwebp `-m6` at equal numeric quality, not matched fidelity. The newest shared scored-workflow capture is 3.4–3.8× cwebp and predates later codec changes; cross-session drift prevents comparing later img-only captures. Peak memory versus cwebp is unmeasured. See [WebP measurements](webp-measurements.md) and [BENCHMARKS](../BENCHMARKS.md).

### Acceptance

Every throughput change needs byte-identical corpus and proptest output, an improvement beyond spread in interleaved gated runs, no median runtime regression over 5% on another stratum and no peak RSS regression over 10%. An unsafe or SIMD kernel must be private behind a safe interface, retain a scalar fallback and reference, pass differential tests including ties and arithmetic edges, and improve an affected real workflow by at least 10% or direct encoding by at least 20%.

```sh
cargo test --manifest-path codecs/webpkit/Cargo.toml --locked --release --lib
cargo test --manifest-path codecs/webpkit/Cargo.toml --locked --features rayon,oracle --lib
cargo check --manifest-path codecs/webpkit/Cargo.toml --locked --no-default-features --features alloc
cargo test --locked --all-features --test webp_codec --test codec_boundary --test optimizer
```

Compare coffee, grass, camera, photopea-ui, mm-logo-alpha, `generated/large.png` and chelsea. For direct lossy compare matched SSIMULACRA2 across quality sweeps against cwebp `-m4` and `-m6`; for lossless compare L1, L6 and L9 with cwebp `-z` levels; for scored workflows use `examples/benchmark.rs`. Record bytes, achieved fidelity, median wall, CPU and peak RSS, distinguishing single and default threads.

### Needs environment

Use Instruments Time Profiler or samply with a symbolized dist build; native Linux timing remains unmeasured. Run timing only on a host admitted by `perf-gate-ready` before and after each measurement.

### Execution boundary and non-goals

The unsafe/SIMD grant covers the `parse_optimal` DP inner loop, hash-chain match walk and lossy trellis quantization, not decode kernels. Do not expand the search design. The grant does not require rewriting all three kernels.

### Reference

[WebP import](WEBP-IMPORT.md), [WebP measurements](webp-measurements.md), [benchmark results](../benchmarks/results/).

## Campaign: JPEG, PNG and AVIF codecs

### Goal and state

Build a permissively licensed stack with reproducible encode and decode leadership, complete workflows, batch throughput and memory against capable competitors. Fidelity, compression, metadata, color, precision, exact alpha, limits and publication are constraints. Adoption may ship while stretch goals remain open.

### Decisions

B is img `f0ceea6`; historical Zen is `5424df8`, a benchmark-only AGPL reference. Pin competitors and options in [competitors.json](../benchmarks/competitors.json). Adoption targets about 1.05× the same upstream path; stretch usually targets 0.80× the strongest qualifying reference. Do not repeat failed experiments without new evidence.

### Baseline

The 2026-11-20 five-round checkpoint qualified scored workflow on development and holdout: wall 0.495× and 0.482× B. Holdout bytes 1.002× [0.997, 1.008] were inconclusive; AVIF was 1.012× and synthetic-17 was 10.6% larger. PNG encode CPU, JPEG and PNG decode stretch, AVIF decode stretch, scored AVIF bytes and equal-memory batch throughput remain open. [Codec measurements](CODECS.md) records the selected implementations, full scoreboard and methods.

### Acceptance

Preserve correctness and compare per case against the strongest qualifying reference using interleaved gated measurements. Use at least five rounds for workflows and nine for decoders. Verify the relevant codec and boundary tests, and `make benchmark-verify` for recorded results.

### Needs environment

Qualification uses the 150-image [corpus](../benchmarks/corpus/qualification/README.md), with development and holdout kept separate. Native Linux ARM64 and x86-64 performance remains unmeasured. Timing requires a host admitted before and after by `perf-gate-ready`.

### Execution boundary and non-goals

No copyleft enters production. WebP is a separate campaign. Do not trade away fidelity or boundary behavior for measured speed.

### Reference

[Codec measurements](CODECS.md), [codec boundary](CODEC-BOUNDARY.md), [vendor pins](VENDOR.md), [checkpoint evidence](../benchmarks/results/checkpoints/2026-11-20/README.md).

## Later

- RIMG-11: make the ICC profile test independent of the header clock; see [TODO](../TODO.md).
- RIMG-12: add a decode fuzz harness and mutation-test codec bodies; see [TODO](../TODO.md).
- Unticketed: AVIF alpha, grayscale, 10/12-bit, grids, animation, HDR and optional 4:2:0; JPEG XYB; SVT-AV1 presets beyond 4 and 8.
- Unticketed: resolve the retained public limits and error API, unused scratch examples and optional Webpkit oracle dependency with their consumers.
