# JPEG, PNG and AVIF codec campaign

## Status — 2026-11-19

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
`python3 -I scripts/qualification-corpus DEST`. It is not yet used by any
qualification run.

## Scoreboard

R is the strongest current qualifying reference for the row; B is `f0ceea6`.
All timings are macOS ARM64 (Apple M1), gated by `perf-gate-ready`. Linux
ARM64 and x86-64 performance is **unmeasured** (no native hardware; emulated
runs establish compatibility only).

| Objective | Next milestone | Status |
| --- | --- | --- |
| Direct JPEG encode | ≤ 0.80× R wall, no more CPU/RSS | Historical only: Fast 0.25–0.48× Zen Fast, Balanced/Best 0.42–0.75× Zen Best at quality 90 (not fidelity-matched). Current jpegli/MozJPEG references unmeasured |
| Direct AVIF encode | ≤ 0.80× R wall | Historical only: speed 8 0.09×, speed 6 0.26× Zen Best |
| Exact PNG encode | ≥ 20% less wall and CPU than R at the frozen envelope | **Measured** (CLI, 5 rounds). Large images (2–6 MP): wall 0.97/0.45/0.33× Zen at fast/balanced/best with 0.95–0.97× its bytes; 0.74/0.52/0.46× B. Small images 0.82–0.98× Zen wall. CPU 1.1–2.7× Zen — open |
| JPEG decode | ≤ 0.80× R, RSS ≤ R | **Measured.** vs Zen 0.31–0.88× by class; vs R (stock libjpeg-turbo 3.2.1 or patched zune) 0.90× on jpegli progressive, 0.95× on other progressive, 1.00× on baseline — open |
| PNG decode | ≤ 0.80× R | Unmeasured |
| AVIF decode | ≤ 0.80× R | Historical: rav1d-asm 0.32× Zen; fair dav1d boundary pending |
| Scored workflow | ≤ 0.75× B, ≤ 0.80× R | JPEG historical: 0.07–0.53× Zen wall at 0.60–0.97× bytes; not re-run against B |
| Large-image auxiliary memory | ≥ 25% below B | Unmeasured |
| Batch throughput | ≥ 1.5× R and 2× Zen at equal cores and memory | Unmeasured |

## Selected implementations

| Role | Selection | State |
| --- | --- | --- |
| JPEG encode | jpegli (libjxl v0.11.1 + Highway, vendored, two patches) | Shipping |
| JPEG decode | libjpeg-turbo 3.2.1 (vendored, refinement patch); zune-jpeg 0.5.15 parses headers and metadata | Shipping since `7b2ba88` |
| PNG encode/optimize | OxiPNG 10.2.1 + libdeflate 1.26 (vendored, chunked-deflate patch) | Shipping |
| PNG decode | png 0.18.1 | Unchanged; not compared |
| Palette quantization | quantizr 1.4.3 (RGBA), quantette 0.6.0 (RGB) | Unchanged |
| AVIF encode | libavif 1.4.2 + libaom 3.13.1, 8-bit 4:4:4 | Shipping |
| AVIF decode | rav1d 1.1.0 with assembly (vendored) | Shipping |

Mappings: PNG fast → OxiPNG preset 0, balanced → preset 0 with libdeflate 8,
best → preset 2. JPEG fast → sequential scans, balanced/best → progressive
level 2. AVIF fast → libaom speed 8, balanced/best → speed 6. Vendor pins and
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

1. **PNG CPU and small-image best latency**: filter trials still cost 1.1–2.7×
   Zen's CPU; best on small images up to 1.9× Zen wall.
2. **Admission reservation**: replace the flat 256 B/pixel estimate with
   measured per-operation peaks so 8–16 MP inputs and more batch workers fit.
3. **JPEG decode vs stock libjpeg-turbo** (0.80× R): Huffman/AC-first and the
   per-MCU dispatch on large progressive images.
4. **AVIF decode fairness**: identical timed boundaries for dav1d/rav1d, 4:4:4
   shipping outputs and 4:2:0 inputs, raw AV1→YUV separately from AVIF→RGB.
5. **Workflow and batch**: re-run the scored workflow against B and current
   references; batch at 1/2/4/8 workers under fixed memory caps.

## Deferred (low priority)

AVIF alpha, grayscale, 10/12-bit, grids, animation and HDR; AVIF 4:2:0 as an
option; JPEG XYB; SVT-AV1 presets beyond 4 and 8.

## Archive

- [Permissive pure-Rust codec audit, 2026-11-13](PERMISSIVE-CODEC-AUDIT-HISTORY-2026-11-13.md).
