# JPEG, PNG and AVIF codec campaign

## Status — 2026-11-20

**Goal:** a permissively licensed JPEG/PNG/AVIF stack with reproducible
leadership in encode and decode speed, complete CLI workflows, batch
throughput and memory, against current, competently configured competitors.
Fidelity, compression efficiency, metadata/color, precision, exact alpha,
limits and publication behavior are constraints, not trade goods. No copyleft
enters production; WebP is a separate campaign ([WEBP-HILLCLIMB.md](WEBP-HILLCLIMB.md)).

**Baselines.** B is img at `f0ceea6` (frozen starting point). Historical Zen is
rimage `5424df8` (zenjpeg 0.8.4, zenpng 0.1.4, zenavif 0.1.6; AGPL, benchmark
only; sha256 `38a2d911…a9be`). Current competitor pins, build options and
toolchains: [benchmarks/competitors.json](../benchmarks/competitors.json). The
latest Zen releases equal the historical pins except zenavif's unpublished
v0.1.7 tag.

**Corpora.** Rapid diagnosis: `benchmarks/corpus/` (small) and the JPEG decode
fixtures below. Qualification: [benchmarks/corpus/qualification](../benchmarks/corpus/qualification/README.md),
150 independent originals (100 development, 50 held out) across photo,
grayscale, texture, UI/text and synthetic, materialized by
`python3 -I scripts/qualification-corpus DEST`. `scripts/bench run --split holdout`
selects the held-out half explicitly; the first held-out run qualified the
2026-11-20 checkpoint ([bundle](../benchmarks/results/checkpoints/2026-11-20/README.md)).

**Milestones.** Each row has two milestones. *Adoption*: integrate the strongest
available implementation with low overhead (within about 5% of the same upstream
path, comparable memory, identical required work). *Stretch*: beat the strongest
qualifying reference R, usually by 0.80×, through measured local work. An adoption
win ships even while its stretch target is open.

**Evidence levels** used below: *implemented*; *directional* (ungated, one round, or
few cases); *qualified on dev* (gated, repeated paired rounds on development data:
at least 5 for workflows, 9 for decoders); *qualified on holdout*; *qualified per
native platform*. Everything below is macOS ARM64 unless stated; nothing is
qualified on native Linux.

## Scoreboard

R is the strongest current qualifying reference for the row; B is `f0ceea6`.
All timings are macOS ARM64 (Apple M1), gated by `perf-gate-ready`. Linux
ARM64 and x86-64 performance is **unmeasured** (no native hardware; emulated
runs establish compatibility only).

| Objective | Next milestone | Status |
| --- | --- | --- |
| Direct JPEG encode | ≤ 0.80× R wall, no more CPU/RSS | **Measured** (CLI `convert --quality 90`, 23 prepared inputs, 3 gated rounds, `c5de9b1`, 2026-11-20). vs MozJPEG 4.1.5 `cjpeg -quality 90`: fast 0.36× wall, 0.23× CPU; balanced 0.42× wall, 0.29× CPU — wall/CPU met, RSS 2.3× (small inputs, where img's fixed footprint dominates). vs cjpegli (libjxl v0.11.1, the same encoder library img vendors): 1.5–1.7× wall and 1.0–1.25× CPU on these 0.2–1 MP inputs, equal wall (0.27 s) on a 7.4 MP photo; its q90 scores 82.6 against img's 84.5, so byte gaps are setting differences. Peak footprint on the 7.4 MP photo is 111 MiB against cjpegli's 99 MiB, down from 602 MiB before direct quality stopped scoring and 160 MiB before opacity checks stopped expanding the image. The remainder is the 13 MB input buffer the caller owns. Adoption met (img wraps the same encoder library). Standalone google/jpegli `c7d342c` (current main, no release tag), 12 dev inputs, 3 gated rounds, matched-score interpolation on the 3–6 inputs where every tool brackets the score (directional): img balanced 0.97–1.01× its bytes at 1.10–1.42× wall and 0.92–1.06× CPU; img fast 1.04–1.08× bytes at 0.79–0.83× CPU ([run](../benchmarks/results/codecs/jpeg/standalone-jpegli-c7d342c-2026-11-20.json)). Stretch open; no tuning experiment against it has been run |
| Direct AVIF encode | ≤ 0.80× R wall | Historical only: speed 8 0.09×, speed 6 0.26× Zen Best, single-threaded. Images of 512×512 and up now encode with four libaom row threads (0.49× wall, 1.04× CPU at matched score), and direct quality reaches all 256 qindex values |
| Exact PNG encode | ≥ 20% less wall and CPU than R at the frozen envelope | **Measured** (CLI, 5 rounds). Large images (2–7 MP): wall 0.94/0.44/0.33× Zen at fast/balanced/best with 0.95–0.97× its bytes. Small images 0.84–1.01× Zen wall. CPU 1.0–1.4× Zen on small and 1.2–2.1× on large images — open (filter trials and libdeflate work; the efficiency cores inflate CPU of parallel chunks; see Next experiments 1). Fixed 2026-11-20: filtered data a few bytes past a 1 MiB chunk boundary (e.g. RGB8 2144×326) failed to encode at every effort |
| JPEG decode | ≤ 0.80× R, RSS ≤ R | **Measured.** vs Zen 0.31–0.88× by class; vs R (stock libjpeg-turbo 3.2.1 or patched zune) 0.90× on jpegli progressive, 0.95× on other progressive, 1.00× on baseline — adoption met, stretch open (Next experiments 3) |
| PNG decode | ≤ 0.80× R | **Measured** ([png-decode.md](codecs/png-decode.md), 137 exact fixtures, 9 gated rounds). png 0.18.1 is the fastest exact permissive decoder, so it is R: libpng 1.6.59 1.41×, libspng 0.7.4 2.04×, zune-png disqualified (16-bit tRNS alpha). img's decoder vs Zen 0.56× time [0.52, 0.59], worst file 0.94×, at 0.83× RSS. Adoption met; stretch needs decoder work beyond png itself (Next experiments 4) |
| AVIF decode | Adoption: ≤ 1.05× upstream dav1d; stretch ≤ 0.80× | **Adoption qualified on dev** (31 fixtures, 9 gated rounds, 0 discarded, 2026-11-20, [report](../benchmarks/results/codecs/avif/decode-report-2026-11-20-dav1d.txt)). img now decodes with vendored dav1d 1.5.4 ([integration](codecs/dav1d-integration.md)); RGB is byte-identical to the former rav1d path and raw planes to upstream meson dav1d on 31/31. AVIF→RGB img / (libavif + dav1d + yuv) 0.992–0.995× time; RSS 1.09–1.10× on large images, 1.24–1.29× on small ones (img's larger binary). img vs its own rav1d build (`e7f01ac`): 0.904–0.907× time, 0.96× RSS. img/Zen 0.14–0.19×. Stretch open |
| Scored workflow | ≤ 0.75× B, ≤ 0.80× R | **B milestone met on the change tier** (`scripts/bench run --only workflow-scored --base f0ceea6`, 1 gated round, 5 dev files, score 90, balanced, `cb8be1a`): wall 0.57× [0.43, 0.73], CPU 0.54×, peak RSS 0.76×, bytes 0.999×; by format AVIF 0.48×, PNG 0.48×, JPEG 0.83× wall. Full-resolution AVIF against `01c2300`: 7.4 MP at score 90 in 17.3 s instead of 97.9 s with 8% fewer bytes ([AVIF.md](AVIF.md)). ICC-tagged JPEG scoring is 2.5–3.9× faster after converting straight to linear sRGB. R (Zen, rebuilt `5424df8`, 1 gated round, 7 inputs × JPEG/PNG/AVIF × 80/90/95, balanced, 2026-11-20): **met** on the 21 cases Zen completes — wall 0.13× [0.07, 0.23], CPU 0.13×, peak RSS 0.71×, bytes 0.77×; Zen fails every AVIF case and 12 JPEG cases. **B milestone qualified on dev and holdout** (checkpoint `0e4d339`, scenario tier, 5 gated rounds, [bundle](../benchmarks/results/checkpoints/2026-11-20/README.md)): dev 16 shared cases wall 0.495× [0.376, 0.636], CPU 0.54×, RSS 0.62×, bytes 0.987×; holdout 55 shared cases wall 0.482× [0.419, 0.554], CPU 0.51×, RSS 0.60×, bytes 1.002× [0.997, 1.008] (inconclusive against the 1.002 byte budget; AVIF 1.012×, worst `synthetic-17.png` +10.6%, open). The candidate completes 68/150 holdout cases against B's 55 and fails none that B completes |
| Large-image auxiliary memory | ≥ 25% below B | **Met: scored JPEG, PNG and AVIF** (directional, single runs). Peak footprint at score 90 (macOS `time -l`), B → now: JPEG 8 MP 201 → 73 B/px (0.36×), PNG 8 MP 199 → 77 (0.39×), AVIF 3 MP 197 → 86 (0.44×) with identical bytes. Levers: planar owned SSIMULACRA2 input, row-streamed distorted blurs, and returning freed large blocks to the OS (`src/allocator.rs`, jpegli, libjpeg-turbo and libaom patches; libmalloc's large-block cache held 29 B/px). Admission reservations by layout are under [Batch admission](#batch-admission-2026-11-20). Lossless PNG 36–71 B/px; WebP and auto unchanged (500–720 B/px, WebP lane) |
| Batch throughput | ≥ 1.5× R and 2× Zen at equal cores and memory | **Measured** (48 dev PNGs, gated, 3 rounds). Lossless best: 19.6 img/s at `-j 4` vs B 6.1 and Zen 5.7 (3.2×/3.4×) at 1.04×/1.14× their peak RSS; at `-j 2` 15.5 img/s within B's RSS. JPEG score 90: 5.5 img/s vs B 3.3 (1.67×) at 1.10× RSS; Zen completes 14/48 at 0.9 img/s. Against rebuilt Zen (lossless best, 55 dev images, 1 gated round, 2026-11-20): Zen does not scale with `-j` (30.2–30.8 img/s, 22 MiB, 49/55 completed at every job count) while img reaches 37.1/67.7/102.7/129.6 img/s at `-j 1/2/4/8` with 24/31/42/59 MiB. At equal memory (img `-j 1`, 24 vs 22 MiB) img is 1.20× Zen; 2× needs `-j 2` at 1.4× its RSS — open: per-worker memory (about 7–11 MiB per extra worker). Memory-bound scored work: **qualified on dev** for the compact-reference scheduler (see Batch admission): 1.50–1.51× successful throughput at `-j 2`/`-j 4` in the same budget |

## Selected implementations

| Role | Selection | State |
| --- | --- | --- |
| JPEG encode | jpegli (libjxl v0.11.1 + Highway, vendored, two patches) | Shipping |
| JPEG decode | libjpeg-turbo 3.2.1 (vendored, refinement patch); zune-jpeg 0.5.15 parses headers and metadata | Shipping since `7b2ba88` |
| PNG encode/optimize | OxiPNG 10.2.1 + libdeflate 1.26 (vendored, chunked-deflate patch) | Shipping |
| PNG decode | png 0.18.1 | Shipping; qualified against libpng, libspng, zune-png and Zen |
| Palette quantization | quantizr 1.4.3 (RGBA), quantette 0.6.0 (RGB) | Unchanged |
| AVIF encode | libavif 1.4.2 + libaom 3.13.1, 8-bit 4:4:4 | Shipping |
| AVIF decode | dav1d 1.5.4 with assembly (vendored, `cc` build, C shim); planes borrowed through RGB conversion | Shipping since `e73123c`; rav1d retired |

Mappings: PNG fast → OxiPNG preset 0, balanced → preset 0 with libdeflate 8,
best → preset 2. JPEG fast → sequential trials and a final progressive level 2
repack that reuses the sequential score after an identical-sample check,
balanced/best → progressive level 2. AVIF fast → libaom speed 8, balanced/best → speed 6. Vendor pins and
patches: [VENDOR.md](VENDOR.md).

## JPEG decode (measured 2026-11-19)

[decode-qualify-2026-11-19.json](../benchmarks/results/codecs/jpeg/decode-qualify-2026-11-19.json):
136 fixtures in 9 classes (`benchmarks/codecs/jpeg/decode-fixtures.py` from the
23 JPEG candidate inputs plus the 11 validation camera originals), 9 paired
rounds in shuffled decoder order, ≥ 150 ms of in-process repetitions per
sample, 5 rounds discarded by the load gate. Every decoder returns packed
RGB8/Gray8 from encoded bytes, including allocation. Peak RSS comes from separate single-decode
runs. Build and run: `benchmarks/codecs/jpeg/decode-build.sh`,
`decode-qualify.py`, `decode-report.py`.

| Class (n) | img / B | img / Zen | patched turbo / stock turbo |
| --- | --- | --- | --- |
| img fast, sequential (23) | 0.76 | 0.52 | 1.00 |
| img balanced, jpegli progressive (23) | 0.90 | 0.88 [0.73, 1.03] | 0.86 |
| libjpeg-turbo 4:4:4 / 4:2:0 baseline (14 each) | 0.74 / 0.71 | 0.67 / 0.40 | 1.00 |
| libjpeg-turbo progressive 4:2:0 (14) | 0.90 | 0.76 | 0.94 |
| restart markers (14) | 0.75 | 0.46 | 1.00 |
| MozJPEG progressive (14) | 0.88 | 0.67 | 0.95 |
| grayscale (9) | 0.73 | 0.31 | 1.00 |
| camera originals (11) | 0.76 | 0.67 | 1.00 |

Geometric means of per-file paired-median ratios; brackets are 95% bootstrap
intervals over files. No class or file is slower than B (worst file 0.98×).
Against Zen the worst files are large jpegli progressive images (p95 1.23×,
worst 1.33× on a 9 MP photo) — the remaining named deficit. Peak RSS of the
direct libjpeg-turbo decode is 0.72–1.00× Zen's by class (worst file 1.01×); the integrated `img` rows
read 1.0–1.5× because the harness copies decoded samples once more through
the public `samples()` accessor. The integrated path adds about 5% over the
bare decoder (marker walk, zune header parse, that copy).

Output: identical to `djpeg` (libjpeg-turbo) on the checked files; differs from
zune-jpeg by at most 9 levels (mean under 0.14), mostly from 4:2:0 upsampling. Errors: all 816
valid, truncated (50%, 90%) and bit-flipped variants decode or fail
identically to unpatched libjpeg-turbo.

Rejected: jpegli progressive level 1 output decodes faster but is 1.6% larger
(up to 5%), outside the frozen size envelope.

## PNG encode (measured 2026-11-19)

Large inputs now compress as 1 MiB deflate chunks in parallel, each primed
with the preceding 32 KiB, joined by sync flushes (local libdeflate and
OxiPNG patches, [VENDOR.md](VENDOR.md)); output is independent of the
thread count. In process, gated, paired, 3 rounds on 98 development PNGs
([encode-chunked-2026-11-19.json](../benchmarks/results/codecs/png/encode-chunked-2026-11-19.json),
`benchmarks/codecs/png/encode-paired.py`, `encode-report.py`): total encode
time 0.64× (fast), 0.48× (balanced), 0.48× (best) of B; total bytes
+0.007%, −0.010%, +0.015%; largest group change best photo +0.06%, largest
file +0.5% (mm-logo-alpha, best) — a named size-for-speed tradeoff.
CLI workflow ([cli-chunked-2026-11-19.json](../benchmarks/results/codecs/png/cli-chunked-2026-11-19.json),
`run.py` then `report.py`): on the large images CPU rises about 20% over B
because the chunks also run on the efficiency cores, and peak RSS is within
1.11× of B (the adapter now hands one sample copy to OxiPNG instead of two).
Images above about 8 MP are refused on this 8 GB host by the CLI's 256
bytes-per-pixel admission reservation, as they are by Zen; measured peaks
are about 45 bytes per pixel.

## Batch admission (2026-11-20)

All images share one working-memory budget (probed, or lowered with
`IMG_MEMORY_BUDGET`). A job reserves its compressed input before reading it and
grows that reservation to its working set under one admission lock, so waiting
jobs never hold uncounted memory. Lossless reserves 48 B/pixel plus 24 B per
decoded sample byte; scored and quality work reserve by layout; all add three
times the input and 64 MiB. WebP and auto keep the original formula, which is below
their actual peak (open, WebP lane).

Scored reservations come from measured peak footprints (macOS `time -l`, score 90,
balanced, [data](../benchmarks/results/batch/scored-footprint-2026-11-20.json)):

| Layout (measured) | Fast reference: peak → reserved | Compact reference: peak → reserved |
| --- | --- | --- |
| Opaque RGB, gray and ICC-tagged JPEG, PNG and AVIF work, 1.2–16 MP | 67–93 → 112 B/px | 26–60 → 72 B/px |
| Declared alpha (nonopaque PNG, two metric references), 16 MP | 124 → 160 B/px | 60 → 80 B/px |

Images under about 1 MP exceed these per-pixel figures (up to 225 B/px at 0.1 MP)
but stay inside the 64 MiB headroom. Sixteen-bit inputs refuse scoring, so no scored
reservation applies. The fast reference is taken only when its reservation fits in the
memory free at admission; otherwise the bit-identical compact reference starts the
job immediately. On 16 large dev JPEGs (3–16 MP) at score 90, balanced, default
budget (2.66 GB), 3 gated rounds ([data](../benchmarks/results/batch/jpeg90-large-compact-2026-11-20.json)):

| `-j` | Successful img/s before → after | CPU s | Peak RSS MiB |
| --- | --- | --- | --- |
| 1 | 0.16 → 0.18 (rounds 2–3 equal; round 1 overlapped a build) | 96 → 90 | 1302 → 1295 |
| 2 | 0.19 → 0.29 (1.51×) | 94 → 104 | 1779 → 1778 |
| 4 | 0.20 → 0.30 (1.50×) | 91 → 107 | 1795 → 1608 |

Outputs are identical and verified (16/16 at every job count). The gain costs
11–18% more CPU, the compact reference's known overhead. Before, memory admitted one
large job at a time, so `-j 2` and `-j 4` used one core. Not yet measured: mixtures of
large and small jobs, the alpha path in a batch, and the page-returning allocator
A/B on sustained workloads. Native musl behavior needs native Linux hardware:
`docker run --platform linux/arm64` on this host is a VM, not native.

## Fast operating point (2026-11-20)

Best keeps its regression envelope. Fast may trade size for speed within an explicit
policy: at most 3% larger by content-group geometric mean, no file more than 10%
larger, unchanged achieved fidelity and semantics, in exchange for at least 20% lower
encode, decode or workflow latency or peak memory, with no unexplained regression in
the other resource metrics. Adaptive trials count against the workflow.

- **JPEG final repack at progressive level 1** instead of 2 (12 dev inputs × scores
  80/90/95, 5 gated rounds, [workflow](../benchmarks/results/codecs/jpeg/fast-repack-level1-2026-11-20.json),
  9-round [decode](../benchmarks/results/codecs/jpeg/fast-repack-level1-decode-2026-11-20.json)):
  bytes +1.4% [0.8, 2.0], worst +4.7% (inside the size limits); output decode 0.86×
  geomean, best 0.36×, 10 of 24 outputs unchanged; workflow 0.99× wall. **Misses the
  20% requirement**; not adopted.
- **PNG Fast** (55 dev PNGs plus 7 large, in-process, 3 gated rounds,
  [data](../benchmarks/results/codecs/png/fast-variants-2026-11-20.json)), against preset 0 at libdeflate 5:
  MinSum-only filter: CPU 0.89×, bytes +3.1% on dev, worst +37% — fails.
  libdeflate 4: CPU 0.87–0.91×, bytes +1.3%, worst +5.5%, but unchunked, so 4-thread
  wall on large images is 1.68× — fails. libdeflate 3: CPU 0.81×, bytes +3.6%, worst
  +8% — fails on size and wall. MinSum at level 4: CPU 0.80×, bytes +5.5%, worst
  +43% — fails. No candidate qualifies.
- **Adopted (not a tradeoff):** Fast scored JPEG reuses the sequential score for its
  progressive repack, which keeps the quantized coefficients (12 dev inputs ×
  80/90/95, 5 gated rounds, [data](../benchmarks/results/codecs/jpeg/fast-repack-reuse-2026-11-20.json)):
  wall 0.928× [0.901, 0.956], CPU 0.920×, outputs byte-identical.

## Historical results (pre-2026-11-19, against historical Zen)

- **JPEG scored workflow** (`benchmarks/results/codecs/jpeg/scored.json`, 23
  images): img/Zen bytes 0.60–0.97× at 0.07–0.53× wall per group and effort,
  except text best/80 at 1.005×; img reaches score 95 on 5/14 photos, Zen on
  none. Matched-score bytes vs Zen Best: color −12.7/−4.8% (80/90), grayscale
  −2.8/−5.1/−2.3% (80/90/95). Trellis (MozJPEG port, branch `feat/jpeg-trellis`)
  saved under 1% and is parked.
- **PNG** ([2026-11-18-lossless.json](../benchmarks/results/codecs/png/2026-11-18-lossless.json),
  24 images): img/Zen bytes 0.42–0.995× by group and effort; best wall up to
  2.04× (horse), 1.70× (large), 1.72× (astronaut); fast CPU 1.8–1.9×.
- **AVIF** ([avif-candidates.md](codecs/avif-candidates.md)): libaom 4:4:4 speed 6
  9.7% smaller than Zen Best at score 90 in 0.26× the time; rav1d-asm decode
  34.6 ms vs Zen 108.8 ms. The dav1d comparison timed output-file writing that
  the Rust timing excluded, so dav1d vs rav1d is unresolved.

## Next experiments

Each item lists what was tried, the result, the remaining profile and the next
hypothesis. Failed experiments are not repeated without new evidence.

1. **PNG CPU vs Zen** (stretch). Tried against a rebuilt `5424df8` (3 gated rounds, 6
   images of 1–7 MP): img is 0.74/0.36/0.46× Zen wall at fast/balanced/best with
   0.98/0.997/0.953× its bytes and about half its RSS, but 1.30/0.94/1.89× its CPU,
   the same single-threaded. Profile: best spends the CPU in libdeflate's level-11
   near-optimal parse (the source of the 5–7% byte lead), fast and balanced in its
   lazy matcher. Lower levels and single filters for Fast (above) cut CPU 9–20% at
   size costs outside the policy. Chunking the greedy levels would restore level 4's
   parallel wall but not its CPU (0.87–0.91×), short of the 20% policy gain, so it
   is not pursued. The evaluation trial's deflate output is already kept when the
   main level equals the evaluation level (Fast at 5; `Evaluator` `final_round`);
   Balanced at 8 evaluates at 5 and must recompress. Next: profile Best's level-11
   parse for matchfinder work repeated across the filter trials.
2. **Streamed scoring parallelism.** fast-ssim2's `rayon` feature cut per-score time to
   0.84× but materialized full planes: workflow peak RSS 1.27× (2.55× on a 1.2 MP JPEG)
   and CPU up to 2.3× (`8412759`). Next: split each scale into horizontal bands with
   per-band streamed blur state and halo rows, keep per-row partial sums and add them
   in the existing order so scores stay bit-identical, and bound scratch to
   bands × band height. Accept only on the complete workflow (wall, CPU, peak RSS).
3. **JPEG decode vs stock libjpeg-turbo** (stretch 0.80× R). Adoption is met: the
   production decoder is libjpeg-turbo 3.2.1 with a refinement patch, at 0.90× R on
   jpegli progressive and 1.00× on baseline. Tried: single-block buffer dispatch,
   skipping all-zero correction batches, scalar masks for narrow bands; together
   0.974× of master over 52 progressive fixtures (5 gated rounds), dropped as too
   small. Remaining profile of img's own progressive output: 39% AC refinement, 11%
   AC first, 9% DC first. Encoder-side alternative: progressive level 1 for Fast
   decodes 0.86× (above), not enough under the Fast policy. Next: profile the AC
   refinement loop at instruction level on the large jpegli photos (worst file 1.33×
   Zen) for a NEON refinement-bit decoder, or a scan script that merges refinement
   passes at equal size.
4. **PNG decode vs png 0.18.1** (stretch 0.80× R; png 0.18.1 is R). Profile: 71% in
   fdeflate's inflate loop, 12% in Paeth. Tried with no gain: removing literal-write
   bounds checks, another Paeth predictor, a larger literal table, deferred
   speculative lookups. libdeflate's decompressor ran 0.82–0.95× on large IDAT and
   1.2–1.4× on small (about 0.92× overall projected, and it gives up streaming).
   Next: size-gated libdeflate for single-IDAT-run images above about 1 MP, measured
   for peak memory as well as time.
5. **AVIF decode beyond dav1d** (stretch). Adoption done. Next: profile img's AVIF→RGB
   path above the decoder (YUV→RGB conversion, plane copies) on the large 4:4:4
   fixtures, and record a dav1d 1.5.5 or later refresh as a separate, identity-checked
   step between milestones.
6. **AVIF byte regression on holdout.** The checkpoint's holdout AVIF bytes are
   1.012× B [0.996, 1.030]; `synthetic-17.png` ends at 50537 bytes and score 90.48
   where B found 45680 at 90.18. Hypothesis: the converged-frontier stop
   (`23e171c`) ends before the lowest qualifying quantizer on flat synthetic
   content. Next: log the visited quantizers on that file, then test one
   confirming probe below the stop point on the development split only.
7. **Batch qualification.** The checkpoint covered lossless batch at `-j 1`/`-j 4`
   (dev 55/55 and holdout 18/18, 2.8× and 3.2× B at `-j 4`). Next: large/small job
   mixtures, the alpha path and the allocator A/B under a fixed `IMG_MEMORY_BUDGET`.

## Verification lanes

`cargo test --all-features --test benchmark_verify` passes again by replaying
saved records in explicit lanes ([registry](../benchmarks/results/historical.json)):
9,460 records replay exactly under the current decoder and scorer. 113 JPEG-source
records captured before `7b2ba88` changed JPEG decoding are historical and
unreplayed (their replay path is recorded, not yet exercised). 156 records have no
replayable artifact: three WebP-lane captures (`668bb86`) never committed their 81
outputs, and `timed-post-skip.json` shares output paths that `timed.json` later
overwrote. Tolerances are unchanged and no record was rewritten. Current claims
come from current runners, whose outputs are verified per case by
`benchmarks/codecs/verify.rs`. The earlier direct-encode comparison against
MozJPEG and libjxl cjpegli (`f8c155d`) has no retained artifact; treat it as
directional.

## Deferred (low priority)

AVIF alpha, grayscale, 10/12-bit, grids, animation and HDR; AVIF 4:2:0 as an
option; JPEG XYB; SVT-AV1 presets beyond 4 and 8.

## Archive

- [Permissive pure-Rust codec audit, 2026-11-13](PERMISSIVE-CODEC-AUDIT-HISTORY-2026-11-13.md).
