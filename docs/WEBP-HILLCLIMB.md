# WebP encoder: throughput parity with cwebp

## Status — reoriented 2026-11-18

This page is the only active brief. It supersedes every queue, "next slice"
and run-loop instruction in the history files:
[ledger to 2026-11-18](WEBP-HILLCLIMB-HISTORY-2026-11-18.md) (base `6d8f0ce`)
and [archive to 2026-11-15](WEBP-HILLCLIMB-HISTORY-2026-11-15.md). Use them to
recover evidence and avoid repeating experiments, not as work queues.

**Goal:** close the speed and memory gap with cwebp without materially
regressing compressed size, then publish the measured tradeoffs and close the
campaign. Overall parity is not a precondition for closing.

Resume prompt: "Continue WEBP-HILLCLIMB per docs/WEBP-HILLCLIMB.md Status."
Next step: queue item 1 (profile).

## Where things stand

All figures are recorded observations, not a fresh paired run.

**Lossless size is competitive on the measured cases.** The table compares
the latest ledger sizes with retained cwebp outputs
([photo baseline](../benchmarks/results/untimed-repaired-2026-11-15.json),
[validation baseline](../benchmarks/results/untimed-validation-2026-11-15.json)):

| Image | img level | img bytes | cwebp `-z9 -exact` | Gap |
| --- | --- | ---: | ---: | ---: |
| camera | codec L9 | 124,614 | 123,720 | +0.72% |
| coffee | codec L9 | 340,156 | 336,030 | +1.23% |
| mm-logo-alpha | codec L1 | 24,720 | 23,696 | +4.32% |

This does not bound the whole corpus. The earlier "beats default cwebp"
claim compared img L9 with cwebp's default effort and recorded no times.

**Direct lossy encode is ~1.3–1.7× cwebp `-m6`**, single thread: coffee q90
83.5 vs ~60 ms, astronaut q80 68.8 vs ~40 ms, grass q90 111 vs ~80 ms. Equal
numeric quality is not matched achieved fidelity.

**The scored workflow is ~3.4–3.8× slower** in the newest capture containing
both tools ([timed-fast-defer.json](../benchmarks/results/timed-fast-defer.json),
five-run serial medians). cwebp's time includes all six grid trials.

| Request | img | cwebp | Ratio | Achieved score img / cwebp |
| --- | ---: | ---: | ---: | ---: |
| coffee, min 80 | 1.040 s | 0.309 s | 3.37× | 80.42 / 85.82 |
| grass, min 95 | 1.430 s | 0.393 s | 3.64× | 95.22 / 95.73 |
| gravel, min 90 | 1.382 s | 0.364 s | 3.80× | 90.10 / 94.19 |

That capture predates the RLE-meta, ALPH-meta and HistogramRemap changes.
Later img-only captures are not comparable with it because of measured
cross-session host drift. No current evidence covers peak memory against cwebp.

## Frozen

- **Compression and search design.** No new candidate families, finer grids,
  iterative cost models or pruning thresholds.
- **Effort monotonicity** (higher explicit effort never yields a larger file).
  It is a public API promise; removing its repair needs an explicit owner
  decision.
- **Standing product decisions:** pure-Rust runtime with no C/FFI; zenwebp
  only as a separately compiled oracle; exact lossless samples including
  hidden RGB; no lowered score thresholds; identical lazy-L6 selection; Fast's
  10 candidate slots and the L6 bands; codec Fast and product `--effort fast`
  reported separately; `deny(unsafe_code)` with local exemptions only; the
  scalar `no_std + alloc` path stays viable; `target-cpu=native` only for the
  aarch64-apple-darwin host build.

## Size budget

A throughput change must produce byte-identical output. The only exception is
removing a weak search addition (queue item 3). Such a removal may grow any
named output at its level by at most 0.25%. After it, the cwebp `-z9` gaps above
must stay within camera 1.0%, coffee 1.5% and mm-logo-alpha 5.0%. These limits
are engineering judgments set at reorientation.

## Queue

1. **Profile current production code.** Use a `dist` build with symbols and a
   sampling profiler (Instruments Time Profiler via `xctrace`, or samply). Profile
   two cases: mm-logo-alpha codec L1 lossless and coffee q90 direct lossy.
   Optionally add the coffee min-80 scored workflow to split encode from
   decode, scoring and search. Record self-time shares. `work-count` counters
   are operation counts, not time, and cannot choose a target alone. Inspect the
   generated code of the chosen loop before proposing unsafe code: unchecked
   indexing removes bounds checks, not dependent loads.
2. **One bounded throughput experiment** on the profile-selected target, under
   the acceptance rule below. Allow at most one follow-up, and only if the
   profile gives a specific reason. If no target can plausibly meet the bar,
   record that and skip to item 3.
3. **Paired check of the weakest recent additions.** Measure L9 wall and bytes
   for each addition on and off: sub-image LZ77 (tens to ~134 B for ~7% L9
   wall), sub-image optimal parsing (2–18 B for ~3%), and cluster reassignment
   (16–116 B for ~1% on meta encodes). Remove an addition when its cost
   exceeds its value and the removal fits the size budget.
4. **One same-session comparison against cwebp.** Cover coffee, grass, camera,
   photopea-ui, mm-logo-alpha, `generated/large.png` and chelsea. Report three
   modes separately:
   - **Direct lossy:** img's resolved codec setting against cwebp `-m4` and
     `-m6` over a quality sweep. Compare bytes at matched achieved
     SSIMULACRA2, interpolating between sweep points.
   - **Direct lossless:** img L1, L6 and L9 against cwebp `-z` levels.
     Compare bytes against time.
   - **Scored workflow:** the existing `examples/benchmark.rs` harness.

   Record bytes, achieved fidelity, median wall, CPU time and peak RSS
   (`/usr/bin/time -l`). Label single-thread and default-thread runs. No
   retained direct-codec harness exists; add a small one for the first two
   modes. Retain results under `benchmarks/results/`.
5. **Publish and close.** Add the item 4 tables and tradeoffs to
   `BENCHMARKS.md` and update the README status. Name any gaps. Native Linux
   timing remains an explicit gap unless a Linux host is available.

## Acceptance rule

Every throughput change needs:

- byte-identical output on the corpus and on proptests;
- an improvement beyond measured spread, from interleaved runs gated by
  `perf-gate-ready`;
- no >5% median-runtime regression on another stratum and no >10% peak-RSS
  regression.

An unsafe or SIMD kernel must additionally:

- be a small private kernel behind a safe interface, following
  `lossless/vp8l/match_kernel.rs`, with the scalar implementation kept as
  fallback and reference;
- pass a differential test against that scalar reference, including ties and
  arithmetic edge cases;
- deliver at least 10% lower latency on an affected real workflow, or 20% lower
  direct-codec latency where direct encoding is the use case;
- stay within the owner's grant: the `parse_optimal` DP inner loop, the
  hash-chain match walk and lossy trellis quantization. Decode kernels are not
  granted, and the grant does not oblige rewriting all three.

## Checks

```sh
cargo test --manifest-path codecs/webpkit/Cargo.toml --locked --release --lib
cargo test --manifest-path codecs/webpkit/Cargo.toml --locked --features rayon,oracle --lib
cargo check --manifest-path codecs/webpkit/Cargo.toml --locked --no-default-features --features alloc
cargo test --locked --all-features --test webp_codec --test codec_boundary --test optimizer
```

At each checkpoint, update this page's status and queue. Do not append to it.
Put experiment detail in a commit message or a short dated history file, and
keep this page to one screen of active work.
