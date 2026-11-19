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
| G1 | Lossy bytes at matched SSIMULACRA2 80/90/95: JPEG and AVIF at least 5% smaller than Zen in every group | JPEG color −6.4/−3.9% (95 unmeasured); gray +2.5–11.3%, text +10–16%. AVIF photo Fast −5.1%, Balanced/Best −9.7% at 90 (shipping); other groups absent |
| G2 | Exact PNG bytes: at least 3% smaller than Zen in every group at every effort, and no image more than 1% larger | Photo 0.97–1.00×; gravel +3.4% and text +2.0% at balanced |
| G3 | Encode latency: Fast at most half of Zen Fast's wall time; Balanced and Best no slower than Zen at the same effort | JPEG ≈ parity (48 vs 48 ms); AVIF Fast 300 ms (0.09× Zen Best, below Zen Fast's 1,063 ms), Balanced/Best 862 ms (0.26× Zen Best); PNG best up to 2.04× slower |
| G4 | Decode latency at most 0.8× Zen; CPU and peak memory no higher than Zen | AVIF dav1d 0.42×; JPEG CPU 0.81× (wall unmeasured); PNG unmeasured |
| G5 | Scored workflow (`--score T`) reaches the target score with fewer bytes than Zen, in less time | JPEG +15–23% bytes at t95; PNG and AVIF unmeasured |
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
| AVIF decode | libavif 1.4.2 + dav1d 1.5.4 | Selected; rav1d 1.1.0 ships until integrated |

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
5. **JPEG scored workflow (bytes).** First explicit `img-jpeg`/`zen-jpeg`
   capture (untimed, grass and gravel): at score 95 img is 15–23% larger
   than Zen (grass 167,538 vs 146,139 B; gravel 181,695 vs 147,844 B). At
   score 80 img is smaller only because Zen overshoots. The candidate
   comparison ([jpeg-candidates.md](codecs/jpeg-candidates.md), 23 scaled
   untagged images, single thread, gated) selected jpegli 4:4:4 for color:
   −6.4% / −3.9% bytes against Zen at scores 80 / 90 at equal latency
   (48.4 vs 47.8 ms), and 4.9× faster than img fast. jpegli still trails Zen
   on grayscale (+11.3 / +8.5 / +2.5% at 80 / 90 / 95) and the one text image
   (+10–16%). libjpeg-turbo decodes with 19% less CPU and half the RSS of
   Zen; in-process decode wall time is unmeasured. jpegli now ships
   (all efforts one configuration); integrated output is byte-identical to
   the measured candidate at equal latency. Its color encode peak RSS is
   45 MiB against Zen's 25 MiB because the adapter copies samples twice.
   Next: remove those copies, re-measure the scored workflow, and time
   libjpeg-turbo decode in process.
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
   862 ms). Encode peak RSS is 75–81 MiB against the harness's 62–67 MiB
   because img copies the samples first. The vendored libaom accepts NASM 3
   through a one-line CMake patch (see [VENDOR.md](VENDOR.md)). Next: measure
   rav1d with its assembly enabled against dav1d, then pick the decoder.

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
