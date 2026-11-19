# JPEG, PNG and AVIF codec campaign

## Status — 2026-11-18

**Goal:** the strongest practical permissively licensed image optimizer. For
JPEG, PNG and AVIF, adopt the strongest maintained upstream implementation per
role, vendor it, conform it to img's contracts, then improve measured
weaknesses until img beats the Zen codecs on the qualification matrix: bytes at
equal or better achieved fidelity, encode and decode latency, CPU, throughput,
and peak memory. Existing correctness, metadata and precision contracts must
not regress, but extending feature coverage is not a goal.

Permissive native code (C/C++, FFI, upstream SIMD/assembly, native build
tools) is authorized for these codecs; copyleft never enters production. WebP
is a separate workstream ([WEBP-HILLCLIMB.md](WEBP-HILLCLIMB.md)); this
campaign does not change `codecs/webpkit` or `src/still/webp.rs`.

## Goals

Each goal holds per group (photo, grayscale, texture, UI/text, synthetic) on
the qualification corpus and on macOS ARM64 and Linux ARM64/x86-64. A goal
is met only when it is measured; an unmeasured cell counts as unmet.

| # | Goal | Now |
| --- | --- | --- |
| G1 | Lossy bytes at matched SSIMULACRA2 80/90/95: JPEG and AVIF at least 5% smaller than Zen in every group | JPEG smallest attainable vs Zen Best's passing outputs: color −12.7/−4.8% (80/90, per-image 4:4:4 or 4:2:0; Zen reaches 95 on none), grayscale −2.8/−5.1/−2.3%; unmet at gray 80/95 and color 90 by 0.2 points. AVIF photo Fast −5.1%, Balanced/Best −9.7% at 90 (shipping); other groups absent |
| G2 | Exact PNG bytes: at least 3% smaller than Zen in every group at every effort, and no image more than 1% larger | Photo 0.97–1.00×; gravel +3.4% and text +2.0% at balanced |
| G3 | Encode latency: Fast at most half of Zen Fast's wall time; Balanced and Best no slower than Zen at the same effort | JPEG met (direct, quality 90, in process): Fast 0.48/0.25/0.30× Zen Fast for color/gray/text (sequential scans), Balanced/Best 0.75/0.42/0.44× Zen Best; AVIF Fast 300 ms (0.09× Zen Best, below Zen Fast's 1,063 ms), Balanced/Best 862 ms (0.26× Zen Best); PNG best up to 2.04× slower |
| G4 | Decode latency at most 0.8× Zen; CPU and peak memory no higher than Zen | AVIF rav1d-asm 0.32× (shipping); JPEG (vendored zune-jpeg, in process): img progressive outputs 0.97×, camera progressive 0.99×, baseline 0.43×, libjpeg-turbo/MozJPEG progressive 0.70/0.53×, grayscale 0.62× — parity, 0.8× unmet on jpegli-style progressive; PNG unmeasured |
| G5 | Scored workflow (`--score T`) reaches the target score with fewer bytes than Zen, in less time | JPEG met: every group and effort at 80/90 (95 where both reach it) is 0.63–0.99× Zen bytes at 0.08–0.51× wall time and no higher peak RSS; img reaches color 95 on 5/14 photos, Zen on none. PNG and AVIF unmeasured |
| G6 | Batch throughput at least 1.5× Zen on the production thread pool | Unmeasured |

**Zen reference:** pre-migration `rimage` at `5424df8` (zenpng 0.1.4, zenjpeg
0.8.4, zenavif 0.1.6; AGPL), built as an immutable benchmark binary outside
the repository (release profile, sha256
`38a2d911f6bae232ac4b25e6fb283a22e349b35d1e827f0b4245f3d02307a9be`). Rebuild:
`git worktree add .work/wt/zen-ref 5424df8` and
`cargo build --locked --release --features build-binary`.

## Selected implementations

| Role | Selection | State |
| --- | --- | --- |
| PNG encode/optimize | OxiPNG 10.2.1 + libdeflate 1.26 (vendored, [VENDOR.md](VENDOR.md)) | Shipping |
| PNG decode | png 0.18.1 | Unchanged; not yet compared with Zen |
| Palette quantization | quantizr 1.4.3 (RGBA), quantette 0.6.0 (opaque RGB) | Unchanged; output now passes through OxiPNG |
| JPEG encode | jpegli (libjxl v0.11.1 + Highway, vendored) YCbCr, 4:4:4 color, progressive level 2 ([jpeg-candidates.md](codecs/jpeg-candidates.md)) | Shipping; byte-identical to the measured candidate |
| JPEG decode | libjpeg-turbo 3.1.1 | Selected; zune-jpeg 0.5.15 ships until integrated |
| AVIF encode | libavif 1.4.2 + libaom 3.13.1, 8-bit 4:4:4 ([avif-candidates.md](codecs/avif-candidates.md)) | Shipping (vendored, `codecs/libavif-sys`) |
| AVIF decode | rav1d 1.1.0 with assembly ([avif-candidates.md](codecs/avif-candidates.md#rav1d-with-assembly)) | Shipping (vendored, `vendor/rav1d`) |

PNG effort mapping: fast → OxiPNG preset 0; balanced → preset 0 with
libdeflate level 8; best → preset 2. The adapter disables alpha optimization,
16-to-8 bit reduction and, when ICC, cICP or another color chunk is present,
grayscale reduction (`src/still/png.rs`).

## PNG results against Zen

[2026-11-18-lossless.json](../benchmarks/results/codecs/png/2026-11-18-lossless.json):
24 unique images (5 color photo, 7 grayscale photo, 9 synthetic, 3 UI/alpha;
coffee and astronaut need `--strip-metadata`), lossless `convert` through
each CLI, 5 interleaved rounds, `perf-gate-ready` passed before and after,
macOS ARM64. Every output is verified sample-exact by the CLI before
publication. Ratios are img/Zen sums per group at the same effort; wall time
uses the production rayon pool (OxiPNG evaluates filters in parallel; Zen is
serial).

| Effort | Bytes photo / gray / synthetic / UI | Wall | CPU |
| --- | --- | --- | --- |
| fast | 0.990 / 0.932 / 0.700 / 0.930 | 0.49–1.07 | 0.49–1.91 |
| balanced | 0.995 / 0.932 / 0.416 / 0.947 | 0.38–1.05 | 0.36–1.46 |
| best | 0.968 / 0.939 / 0.647 / 0.922 | 0.70–1.55 | 0.96–1.91 |

Against the previous img encoder (png crate), synthetic output is 7.4× smaller
at fast (1.2× balanced, 1.05× best) and UI/alpha 20–49% smaller. Peak RSS is within 0.82–1.30× of Zen. Binary
grows 149,856 bytes (4,622,592 → 4,772,448).

## Open deficits and next experiments

1. **Grayscale textures (bytes).** gravel +3.0–3.4% and text +1.6–2.0% at
   fast/balanced, grass +1.1% at balanced. They need libdeflate's near-optimal
   levels (≥ 10), which are too slow on long-match images at these efforts.
   Next: measure whether a per-image entropy estimate already computed by
   OxiPNG can select the level without a second trial.
2. **Best wall time** on gradients/alpha/UI (large 1.70×, horse 2.04×,
   mm-logo-alpha 1.51×, astronaut 1.72×). Next: profile libdeflate level 11 on
   `large.png` and test level 12 with fewer filter trials.
3. **CPU work.** Parallel filter trials cost 1.8–1.9× Zen's CPU on photos at
   fast. The serial configuration (`RAYON_NUM_THREADS=1`) uses up to 2.3× and
   is 1.3–1.4× Zen's wall time on photos. Next: test fast with a single filter strategy.
4. **Not yet measured:** PNG decode against Zen, the scored optimize workflow
   and batch throughput with the new encoder, and Linux native timing.
5. **JPEG.** jpegli ships for every effort ([jpeg-candidates.md](codecs/jpeg-candidates.md)).
   Three changes close the scored workflow (G5): fractional quality reaches
   jpegli's distance instead of rounding; grayscale uses jpegli's perceptual
   luma quantization (local patch, [VENDOR.md](VENDOR.md)) instead of Annex K
   tables; and JPEG-only `--score` searches by interpolating between
   bracketing attempts from a seed at the target, stopping within an effort
   slack (fast 0.5, balanced 0.15, best 0.05 score points) instead of bisecting
   fixed seeds. Fast searches with sequential scans and publishes a
   progressive re-encode of the winner (same coefficients, so the same
   score, about 4% smaller). Color at balanced and best also searches 4:2:0, seeded from
   the 4:4:4 result, and keeps the smaller verified output; adaptive
   quantization is on only below distance 0.3. Gated interleaved run
   (`benchmarks/codecs/jpeg/scored.py`, 23 images, single thread, img's compare
   re-scores every output; `benchmarks/results/codecs/jpeg/scored.json`),
   img/Zen geometric means:

   | Group | Effort | Bytes 80 / 90 / 95 | Wall 80 / 90 / 95 |
   | --- | --- | --- | --- |
   | color (14) | fast | 0.60 / 0.88 / — | 0.44 / 0.47 / — |
   | color | balanced | 0.85 / 0.88 / — | 0.31 / 0.29 / — |
   | color | best | 0.90 / 0.86 / — | 0.17 / 0.14 / — |
   | grayscale (8) | fast | 0.76 / 0.65 / 0.91 | 0.53 / 0.45 / 0.42 |
   | grayscale | balanced | 0.85 / 0.91 / 0.97 | 0.24 / 0.17 / 0.18 |
   | grayscale | best | 0.94 / 0.94 / 0.97 | 0.11 / 0.11 / 0.09 |
   | text (1) | all | 0.74–1.005 | 0.07–0.52 |

   Zen fails score 90 on 6 color photos and 95 on all 14; img reaches 95 on
   5 and every grayscale image. Zen's internal scorer accepted some outputs
   img scores below target; those are still counted, which favors Zen. Peak
   RSS is at or below Zen (marmolada 488 vs 510 MiB, gravel 47 vs 49). Open:
   G1 matched-score bytes are −3.0/−2.2/−1.8% for grayscale and −6.4/−3.9%
   for color at 80/90 with 4:4:4 alone; per-image 4:4:4/4:2:0 choice reaches
   −12.7/−4.8% for color. A MozJPEG trellis port (branch `feat/jpeg-trellis`,
   parked) saved under 1%, and full MozJPEG trellis with img's tables is only
   0.5–3.9% smaller at 4.8× the encode time, so trellis is not pursued. Text
   at best/80 is 1.005× Zen (one image). Decode: vendored zune-jpeg with a
   nonzero-mask refinement patch decodes img's progressive output in 0.96×
   and a progressive camera original in 0.99× Zen's time (gated, in process;
   `benchmarks/results/codecs/jpeg/decode-progressive.json`), and baseline,
   libjpeg-turbo, MozJPEG and grayscale files in 0.43–0.71×. Batching
   correction-bit reads was measured 3% slower and dropped. 0.8× on jpegli
   progressive output remains open: refinement decoding is still about half
   the decode time. JPEG decode wall time (zune-jpeg) against Zen is unmeasured.
6. **AVIF.** The candidate comparison ([avif-candidates.md](codecs/avif-candidates.md),
   14 scaled untagged photos plus one gradient, single thread, gated) selected
   libaom 4:4:4: at score 90 speed 6 is 9.7% smaller than Zen Best at 863 vs
   3,376 ms, and speed 8 is 5.1% smaller at 299 ms. Mapping: fast → speed 8;
   balanced and best → speed 6 (speed 4 costs 10× for 0.65%). dav1d decodes in
   46.7 ms against rav1d's 69.5 ms and Zen's 112.0 ms. 4:2:0 cannot reach score
   90 on five photos. No eligible texture or UI/text input exists (all are gray
   or alpha, which img's AVIF path refuses). The lane found img read the nclx
   `full_range_flag` from bit 0 instead of bit 7, misdecoding standard
   full-range files; fixed with a regression test in `tests/avif_codec.rs`.
   libavif + libaom now ship as the encoder: AV1 payloads are byte-identical
   to the candidate at all 720 sweep points, with equal wall time (300 and
   862 ms). Encode peak RSS was 75–81 MiB against the harness's 62–67 MiB
   because img copied the samples first; that copy is now removed. The vendored libaom accepts NASM 3
   through a one-line CMake patch (see [VENDOR.md](VENDOR.md)). rav1d with its
   assembly decodes in 34.6 ms against dav1d's 45.7 and Zen's 108.8, so dav1d
   is withdrawn. rav1d now ships vendored with its assembly (the published
   crate omits `asm-offsets.h`; the assembler build needs `bitdepth_16`):
   34.65 ms against 67.27 ms pure Rust, with identical decoded samples.

## Deferred (low priority)

Not pursued unless a concrete need appears; img keeps refusing these
explicitly.

- AVIF alpha, grayscale and 10/12-bit encode and decode.
- AVIF grids, animation and HDR (gain maps, PQ/HLG).
- AVIF 4:2:0 as an option (cannot reach score 90 on 5 of 14 photos).
- JPEG XYB output (not baseline-compatible YCbCr; unscorable today).
- SVT-AV1 presets beyond 4 and 8, libaom tuning/tiling variants.

## Needs environment

- Native Linux ARM64/x86-64 timing (no Linux host here).
- `tests/test_package_release.py` needs GNU tar; run it in the repository's
  Alpine image.

## Archive

- [Permissive pure-Rust codec audit, 2026-11-13](PERMISSIVE-CODEC-AUDIT-HISTORY-2026-11-13.md)
  (superseded policy; candidate survey retained as evidence).
