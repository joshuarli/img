# AVIF candidates

This is an ARM64 macOS comparison of eight-bit AVIF encoders and decoders on scaled, untagged opaque color inputs. Zen is a benchmark reference only and is ineligible for production.

## Sources and notices

[libavif v1.4.2](https://github.com/AOMediaCodec/libavif/tree/v1.4.2), commit `bd32fb40a3b2028eb555b67f769c57a32ed8a1b4`, archive `2b645287340ba5a631d268b551dc2d72bd73ac33335962dd36dcdb6d8366921d`: BSD-2-Clause (`LICENSE`). [libaom v3.13.1](https://aomedia.googlesource.com/aom/+/refs/tags/v3.13.1), commit `d772e334cc724105040382a977ebb10dfd393293`, archive `23f9c4051255b190ecfaed53f41ffc5e97582bfb09736b4b22f891783dfd8510`: BSD-2-Clause (`LICENSE`) and the Alliance for Open Media Patent License 1.0 (`PATENTS`). [dav1d 1.5.4](https://github.com/videolan/dav1d/tree/1.5.4), commit `191bdda98ec3c68137754dc97da1db34043d7cd4`, archive `a1d5b63d2d38ec9bd03acf643caa51fa22edd1e89c5a109c4807717216bbec07`: BSD-2-Clause (`COPYING`). [SVT-AV1 v4.2.0](https://github.com/AOMediaCodec/SVT-AV1/tree/v4.2.0), commit `9292ec8e32bce26f781f277ec8739b53426c4300`, archive `d9cb8d5fccbed565d038ae304cb00c5610f1f62388868102ca22132e7844921b`: BSD-3-Clause-Clear (`LICENSE.md`), AOM Patent License 1.0 (`PATENTS.md`) and BSD-2-Clause components (`LICENSE-BSD2.md`). [libjpeg-turbo 3.1.1](https://github.com/libjpeg-turbo/libjpeg-turbo/tree/3.1.1), commit `7723f50f3f66b9da74376e6d8badb6162464212c`, archive `304165ae11e64ab752e9cfc07c37bfdc87abd0bfe4bc699e59f34036d9c84f72`: IJG, BSD-3-Clause and Zlib (`LICENSE.md`, `README.ijg`); used only to decode qualification inputs.

[ravif 0.13.0](https://crates.io/crates/ravif/0.13.0) is BSD-3-Clause; its encoder dependency rav1e 0.8.1 is BSD-2-Clause with an AOM patent grant (`LICENSE`, `PATENTS`), and its container writer avif-serialize 0.8.9 is BSD-3-Clause. [rav1d 1.1.0](https://crates.io/crates/rav1d/1.1.0) is BSD-2-Clause. The decode candidate also links yuv 0.8.20 (BSD-3-Clause or Apache-2.0), imgref 1.12.3 (CC0-1.0 or Apache-2.0), rgb 0.8.53 (MIT), almost-enough 0.4.5 and libc 0.2.183 (MIT or Apache-2.0). Retain the locked dependency notices at vendoring.

[zenavif 0.1.6](https://crates.io/crates/zenavif/0.1.6) is AGPL-3.0-only or commercial and is measured only as a reference. The prebuilt `zen-5424df8` SHA-256 is `38a2d911f6bae232ac4b25e6fb283a22e349b35d1e827f0b4245f3d02307a9be`; `img-master-2258e42` is `a6bbb769e61d8a2d34f75928cab39b2f84bc18a35a2d0b560f93cea5a9d3d617`.

## Method and reproduction

The 27 candidate inputs yield 15 eligible images: 14 color photographs and the 2048x1024 synthetic gradient. No eligible texture or UI/text image exists: nine inputs are eight-bit grayscale (grass, gravel, brick, camera, cell, clock_motion, coins, microaneurysms, text) and three carry alpha (horse, mm-logo-alpha, photopea-ui); all twelve are refused and recorded. The other eight procedural generated PNGs and the generated WebP are out of scope.

`prepare.py` decodes PNG samples without color conversion and JPEG with pinned libjpeg-turbo at the largest M/8 scale whose long side is at most 2048 pixels. It writes an untagged eight-bit RGB PNG and an identical P6 PNM per input, verifies the PNG chunk list and round-trip samples, and records SHA-256 and scale. Treating samples as untagged sRGB intentionally disregards embedded profiles and orientation; this qualifies codec efficiency, not color management or full resolution.

Native encoders call libavif from a small C helper, explicitly selecting libaom or SVT-AV1, with eight-bit full-range BT.601 samples and either 4:2:0 or 4:4:4 output. libaom uses speeds 4, 6 and 8; SVT-AV1 uses presets 4 and 8, to which libavif passes `speed` directly. Current img uses ravif 0.13.0 exactly as shipped: quality clamped to 1..100, speeds 10, 8 and 6, one thread, 4:4:4. Zen uses zenavif 0.1.6 exactly as the reference configures it: quality `1 + 0.98q`, speeds 10, 8 and 6, eight-bit, one thread, CICP 1/13/6, 4:4:4.

libavif's `colr`/nclx property is renamed to `free` after native encoding. Current img reads `full_range_flag` from bit 0 of the nclx flags byte instead of bit 7 (`src/still/avif_container.rs:651`), so it misdecodes standard full-range output; with the optional property absent, the AV1 sequence header carries CICP, as current ravif output already does. This is a measurement workaround, not a container recommendation.

Each encoder uses every prepared input. Qualities are 20, 30, 40, 50, 60, 65, 70, 75, 80, 85, 88, 90, 92, 95, 98 and 100. The current img binary supplies every SSIMULACRA2 score. Interpolation uses adjacent successful points at most ten quality steps apart, without extrapolation. Bytes at a target are means over images where that configuration brackets it; counts therefore differ by row. Relative rows use the intersection of the two configurations. Native encoders were also checked with dav1d; every swept output decodes with dav1d and img with zero failures.

Timing calls the same libraries in process and takes wall and user CPU from the program, five interleaved runs at the two sweep qualities bracketing score 90, blended to score 90. Encoder RSS is the larger median peak of the two bracket points; decode RSS is the process peak. Every image batch is preceded and followed by `perf-gate-ready`, and a failed post-check discards the batch. Runs are single-threaded except `aom-s6-t4`, which uses four threads. Decode fixtures are one AVIF per input (native libaom 4:2:0, speed 6, quality 90). dav1d and zenavif perform full container reads; rav1d 1.1.0 decodes the extracted primary OBU with img's settings plus yuv conversion, so its timing excludes container parsing.

Run `sh benchmarks/codecs/avif/build.sh` then `python3 benchmarks/codecs/avif/prepare.py`. It builds libaom with `-DCONFIG_AV1_ENCODER=1 -DCONFIG_LOWBITDEPTH=1 -DENABLE_TOOLS=0`, dav1d with `--default-library=static -Denable_tools=false`, and libavif with `-DAVIF_CODEC_AOM=SYSTEM -DAVIF_CODEC_DAV1D=SYSTEM -DAVIF_LIBYUV=OFF -DBUILD_SHARED_LIBS=OFF`, adding `-DAVIF_CODEC_SVT=SYSTEM` for the SVT helper. `python3 benchmarks/codecs/avif/sweep.py SWEEP.json` captures bytes and scores, `python3 benchmarks/codecs/avif/timed.py TIMING.json` captures gated timing, and `python3 benchmarks/codecs/avif/report.py benchmarks/results/codecs/avif/results.json` reproduces every table below.

## Results


### photo (14 images)

| Configuration | Score 80: mean bytes (n) | Score 90: mean bytes (n) | Score 95: mean bytes (n) |
| --- | ---: | ---: | ---: |
| aom-s4-420 | 233104 (14) | 904716 (9) | unavailable (0) |
| aom-s6-420 | 244416 (14) | 937871 (9) | unavailable (0) |
| aom-s8-420 | 256438 (14) | 958992 (9) | unavailable (0) |
| aom-s4-444 | 228410 (14) | 665990 (14) | 330853 (4) |
| aom-s6-444 | 239018 (14) | 670354 (14) | 335693 (4) |
| aom-s8-444 | 249502 (14) | 704535 (14) | 366732 (4) |
| svt-s4-420 | 272350 (14) | 940999 (9) | unavailable (0) |
| svt-s8-420 | 292922 (14) | 998273 (9) | unavailable (0) |
| img-fast | 350387 (14) | 1110146 (13) | 1123169 (1) |
| img-balanced | 282766 (14) | 774510 (14) | 375118 (2) |
| img-best | 281983 (14) | 771088 (14) | 376551 (2) |
| zen-fast | 348133 (14) | 946232 (9) | unavailable (0) |
| zen-balanced | 285229 (14) | 741319 (14) | 609078 (1) |
| zen-best | 283317 (14) | 742361 (14) | 609594 (1) |

| Relative bytes | Score 80 | Score 90 | Score 95 |
| --- | ---: | ---: | ---: |
| aom-s4-420 / zen-best | -17.7% (14) | +8.9% (9) | unavailable (0) |
| aom-s6-420 / zen-best | -13.7% (14) | +12.9% (9) | unavailable (0) |
| aom-s8-420 / zen-best | -9.5% (14) | +15.4% (9) | unavailable (0) |
| aom-s4-444 / zen-best | -19.4% (14) | -10.3% (14) | +10.5% (1) |
| aom-s6-444 / zen-best | -15.6% (14) | -9.7% (14) | +11.7% (1) |
| aom-s8-444 / zen-best | -11.9% (14) | -5.1% (14) | +28.8% (1) |
| svt-s4-420 / zen-best | -3.9% (14) | +13.3% (9) | unavailable (0) |
| svt-s8-420 / zen-best | +3.4% (14) | +20.2% (9) | unavailable (0) |
| aom-s6-444 / img-fast | -31.8% (14) | -35.7% (13) | -39.4% (1) |
| img-fast / zen-fast | +0.6% (14) | +0.5% (9) | unavailable (0) |
| img-best / zen-best | -0.5% (14) | +3.9% (14) | +6.2% (1) |
| img-fast / img-best | +24.3% (14) | +34.9% (13) | +73.5% (1) |

### synthetic gradient (1 images)

| Configuration | Score 80: mean bytes (n) | Score 90: mean bytes (n) | Score 95: mean bytes (n) |
| --- | ---: | ---: | ---: |
| aom-s4-420 | 1478 (1) | unavailable (0) | unavailable (0) |
| aom-s6-420 | 3031 (1) | unavailable (0) | unavailable (0) |
| aom-s8-420 | 3774 (1) | unavailable (0) | unavailable (0) |
| aom-s4-444 | 1916 (1) | unavailable (0) | unavailable (0) |
| aom-s6-444 | 3264 (1) | unavailable (0) | unavailable (0) |
| aom-s8-444 | 4342 (1) | unavailable (0) | unavailable (0) |
| svt-s4-420 | 1339 (1) | unavailable (0) | unavailable (0) |
| svt-s8-420 | 1205 (1) | unavailable (0) | unavailable (0) |
| img-fast | 9632 (1) | unavailable (0) | unavailable (0) |
| img-balanced | 4453 (1) | unavailable (0) | unavailable (0) |
| img-best | 4447 (1) | unavailable (0) | unavailable (0) |
| zen-fast | 9121 (1) | unavailable (0) | unavailable (0) |
| zen-balanced | 4294 (1) | unavailable (0) | unavailable (0) |
| zen-best | 4293 (1) | unavailable (0) | unavailable (0) |

| Relative bytes | Score 80 | Score 90 | Score 95 |
| --- | ---: | ---: | ---: |
| aom-s4-420 / zen-best | -65.6% (1) | unavailable (0) | unavailable (0) |
| aom-s6-420 / zen-best | -29.4% (1) | unavailable (0) | unavailable (0) |
| aom-s8-420 / zen-best | -12.1% (1) | unavailable (0) | unavailable (0) |
| aom-s4-444 / zen-best | -55.4% (1) | unavailable (0) | unavailable (0) |
| aom-s6-444 / zen-best | -24.0% (1) | unavailable (0) | unavailable (0) |
| aom-s8-444 / zen-best | +1.1% (1) | unavailable (0) | unavailable (0) |
| svt-s4-420 / zen-best | -68.8% (1) | unavailable (0) | unavailable (0) |
| svt-s8-420 / zen-best | -71.9% (1) | unavailable (0) | unavailable (0) |
| aom-s6-444 / img-fast | -66.1% (1) | unavailable (0) | unavailable (0) |
| img-fast / zen-fast | +5.6% (1) | unavailable (0) | unavailable (0) |
| img-best / zen-best | +3.6% (1) | unavailable (0) | unavailable (0) |
| img-fast / img-best | +116.6% (1) | unavailable (0) | unavailable (0) |

### Failures


### photo timing

| Role | Configuration | Threads | Wall ms | Wall range ms | User CPU ms | RSS MiB |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| encode | aom-s4-420 | 1 | 6149.37 (9) | 6052.95–6728.30 | 6095.73 | 72.84 |
| encode | aom-s6-420 | 1 | 687.82 (9) | 683.18–751.28 | 687.42 | 44.69 |
| encode | aom-s8-420 | 1 | 284.94 (9) | 283.57–301.12 | 284.91 | 43.64 |
| encode | aom-s4-444 | 1 | 8542.68 (14) | 8490.94–9115.48 | 8529.16 | 101.56 |
| encode | aom-s6-444 | 1 | 863.05 (14) | 860.56–998.26 | 862.94 | 67.45 |
| encode | aom-s8-444 | 1 | 299.47 (14) | 298.87–321.96 | 299.45 | 62.03 |
| encode | svt-s4-420 | 1 | 932.78 (9) | 929.38–947.92 | 931.96 | 70.66 |
| encode | svt-s8-420 | 1 | 127.28 (9) | 125.32–152.82 | 126.69 | 54.28 |
| encode | img-fast | 1 | 1189.86 (13) | 1187.78–1460.61 | 1176.55 | 103.98 |
| encode | img-balanced | 1 | 3096.62 (14) | 3092.18–3370.95 | 3073.35 | 94.13 |
| encode | img-best | 1 | 3996.22 (14) | 3926.77–4603.56 | 3941.27 | 94.00 |
| encode | zen-fast | 1 | 1057.71 (9) | 1055.91–1277.97 | 1046.00 | 86.10 |
| encode | zen-balanced | 1 | 2557.37 (14) | 2552.94–2853.40 | 2538.00 | 77.92 |
| encode | zen-best | 1 | 3375.62 (14) | 3326.92–3701.82 | 3333.27 | 77.78 |
| encode | aom-s6-t4 | 4 | 282.07 (9) | 255.76–307.30 | 772.41 | 47.47 |
| decode | dav1d | 1 | 46.71 (14) | 46.32–50.63 | 46.47 | 16.94 |
| decode | rav1d | 1 | 69.45 (14) | 68.62–76.13 | 67.41 | 15.77 |
| decode | zen | 1 | 111.99 (14) | 110.46–115.97 | 108.42 | 24.58 |

### synthetic gradient timing

| Role | Configuration | Threads | Wall ms | Wall range ms | User CPU ms | RSS MiB |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| decode | dav1d | 1 | 24.30 (1) | 24.19–24.48 | 24.30 | 19.00 |
| decode | rav1d | 1 | 30.28 (1) | 30.20–30.71 | 29.38 | 17.02 |
| decode | zen | 1 | 68.35 (1) | 68.00–68.52 | 66.48 | 28.03 |

## Recommendation

Encoder: libavif v1.4.2 with libaom v3.13.1, eight-bit 4:4:4, full-range BT.601, one thread. At score 90 speed 4 uses 665990 bytes against Zen Best's 742361 on all 14 photos, 10.3% less, but takes 8543 ms against Zen Best's 3376 ms. Speed 6 uses 670354 bytes at 863 ms, 9.6% less than Zen Balanced's 741319 at 2557 ms. At score 80 speed 6 is 15.6% smaller than Zen and 31.8% smaller than current img Fast; speed 4 is 19.4% smaller than Zen. 4:2:0 is not selected: it reaches score 90 on only nine photos and is 8.9-15.4% larger than Zen where it does, so it cannot cover the high-fidelity range.

Effort mapping: Fast uses `speed=8`, Balanced and Best use `speed=6`, all with `AVIF_PIXEL_FORMAT_YUV444`, `AVIF_RANGE_FULL`, `AVIF_MATRIX_COEFFICIENTS_BT601`, `maxThreads=1` and direct quality 0..100. Speed 8 measures 299 ms for 5.1% fewer bytes than Zen at score 90; speed 6 measures 863 ms for 9.7% fewer. Speed 4 was measured at 8543 ms and only 0.65% fewer bytes than speed 6, so no tier uses it. Threads help: four threads at speed 6 in 4:2:0 measure 282 ms on the nine photos that bracket 90, against 688 ms single-threaded on the same set, with 772 ms user CPU against 687 ms.

Decoder: libavif with dav1d 1.5.4. On 14 photos its mean is 46.7 ms against 69.5 ms for rav1d 1.1.0 and 112.0 ms for Zen, with 16.9 MiB peak RSS against 15.8 and 24.6. rav1d is the pure-Rust fallback at 1.49 times the time and slightly lower memory, even though it excludes container parsing that dav1d includes; Zen's stack is slower and is AGPL.

## Open deficits and unknowns

- The candidate sweep renamed the `colr` property to `free` because img then misread nclx full range. Commit a9ce3fa fixed that, and the integrated encoder keeps the standard `colr` nclx property. The box has the same size as the `free` box, so the file bytes match too.
- 4:2:0 cannot reach SSIMULACRA2 90 on five of fourteen photos and caps at 87.8 on the gradient. 4:4:4 reaches 95.2 at quality 100 and 88.1 on the gradient, so score 95 exists only for four photos. The gradient fails every encoder at score 90 and is reported only at 80.
- Alpha, grayscale, 10/12-bit, grids, animation, HDR, ICC, EXIF/XMP and non-square pixels are excluded. Only BT.601, default tuning, no tiling and no adaptive-quantization sweep were measured. SVT-AV1 was measured only in 4:2:0 at presets 4 and 8.
- Timing covers score 90 only, at a 2048-pixel long side; full resolution and color-managed inputs are unmeasured.
- Decode timing excludes container parsing for rav1d only; dav1d and Zen include it. zenavif 0.1.6 with rav1d-safe 0.5.7 panics on ravif-produced 4:4:4 output (`looprestoration_arm.rs`), so it cannot decode current img files reliably.
- Linux, Windows, other architectures and integration cost are unmeasured. Static libraries are 7.9 MB (libaom), 5.4 MB (SVT-AV1), 1.0 MB (dav1d) and 0.3 MB (libavif); the libaom-linked helper is 7.2 MB. The capture took about 2h55m: roughly 20m building, 83m sweeping and 73m gated timing.

## Integrated encoder

`src/still/avif.rs` encodes through `codecs/libavif-sys` with the selected configuration: speed 8 for Fast, speed 6 for Balanced and Best. The sweep used the same 15 inputs and qualities, `benchmarks/codecs/avif/encode-integrated.rs` and the integration commit's `compare`. At each point it also ran the matching candidate (`aom-s8-444` for Fast, `aom-s6-444` otherwise). All 720 AV1 payloads are byte-identical to the candidate's, and the matched-score file bytes equal the candidate rows in every cell. Results are in `benchmarks/results/codecs/avif/integration.json`.

| Group | Encoder | Score 80 bytes (n) | Score 90 bytes (n) | Score 95 bytes (n) |
| --- | --- | ---: | ---: | ---: |
| photo | img-libaom-fast | 249502 (14) | 704535 (14) | 366732 (4) |
| photo | img-libaom-balanced, best | 239018 (14) | 670354 (14) | 335693 (4) |
| photo | zen-best | 283317 (14) | 742361 (14) | 609594 (1) |
| synthetic gradient | img-libaom-fast | 4342 (1) | unavailable | unavailable |
| synthetic gradient | img-libaom-balanced, best | 3264 (1) | unavailable | unavailable |
| synthetic gradient | zen-best | 4293 (1) | unavailable | unavailable |

At score 90 on photos, Fast uses 5.1% fewer bytes than Zen Best and Balanced/Best 9.7% fewer. At score 80 the savings are 11.9% and 15.6%. On the gradient, Fast is 1.1% larger than Zen Best at score 80; Balanced/Best is 24.0% smaller.

Timing used five alternating single-thread repetitions at the score-90 brackets, with `perf-gate-ready` before and `-1` after each input batch and a final `-1`. Times are means of per-image interpolated medians. RSS is the process peak.

| Encoder | Wall ms (n) | Wall range ms | User CPU ms | Peak RSS MiB |
| --- | ---: | ---: | ---: | ---: |
| img-libaom-fast | 300.24 (14) | 299.71–308.41 | 294.13 | 75.46 |
| aom-s8-444 | 300.10 (14) | 299.25–307.96 | 300.09 | 62.03 |
| img-libaom-balanced | 862.24 (14) | 860.98–880.77 | 848.52 | 80.78 |
| img-libaom-best | 862.54 (14) | 861.15–881.03 | 848.66 | 80.78 |
| aom-s6-444 | 864.31 (14) | 861.25–870.92 | 864.27 | 67.45 |
| zen-fast | 1062.97 (9) | 1056.77–1066.69 | 1051.23 | 86.10 |
| zen-best | 3343.04 (14) | 3329.16–3349.04 | 3318.41 | 77.78 |

Wall time matches the standalone candidates. Fast takes 0.09 times Zen Best's time; it is also well under Zen Fast's mean, although Zen Fast brackets score 90 on only nine photos. Peak RSS is 13 MiB above the standalone harness because img copies the decoded samples before encoding. The Fast row is 10.7 MiB under Zen Fast's mean (on Zen's nine photos), and Balanced/Best is 3.0 MiB above Zen Best.

On x86-64 the vendored libaom builds its SSE2, SSSE3 and AVX2 assembly with NASM. Linux timing is unmeasured.

## rav1d with assembly

rav1d 1.1.0 was rebuilt with `asm`, `asm_arm64_dotprod`, `asm_arm64_i8mm` and `asm_arm64_sve2` (config `rav1d-asm`; `benchmarks/codecs/avif/build.sh` now builds against `vendor/rav1d`). Two packaging defects in the published crate block this. It omits `src/arm/asm-offsets.h`, so the ARM assembly does not build; the vendored snapshot adds that file from upstream tag v1.1.0 (commit `782dab2135ea64a057c097088a13eb8ed3cc3320`). And `asm` with only `bitdepth_8` fails to compile (missing `dav1d_ipred_pixel_set_16bpc_neon`), so `bitdepth_16` is also enabled; img still refuses high-precision input. SVE2 is compiled and runtime-detected but only supplies 16-bit paths, so eight-bit output is unchanged. Decoded RGB is byte-identical to the pure-Rust build on the fixtures checked (inputs 0, 3 and 7), and the vendored build reproduces the scratch rav1d-asm output byte-for-byte on all 15 qualification fixtures. Reproduce the gated timing with `AVIF_TIMING_CONFIGS= AVIF_TIMING_DECODE=rav1d,rav1d-asm python3 benchmarks/codecs/avif/timed.py OUT`.

The same score-90 4:2:0 fixtures were rerun for all decoders, with five alternating single-thread repetitions and `perf-gate-ready` before and `-1` after each batch (`benchmarks/results/codecs/avif/decode-asm.json`):

| Group | Decoder | Wall ms (n) | Wall range ms | User CPU ms | Peak RSS MiB |
| --- | --- | ---: | ---: | ---: | ---: |
| photo | rav1d-asm | 34.63 (14) | 34.54–34.78 | 33.72 | 15.87 |
| photo | dav1d | 45.71 (14) | 45.57–46.19 | 45.67 | 16.94 |
| photo | rav1d | 67.13 (14) | 67.01–67.59 | 66.12 | 15.77 |
| photo | zen | 108.80 (14) | 108.42–109.83 | 106.99 | 24.58 |
| synthetic gradient | rav1d-asm | 8.36 (1) | 8.27–8.48 | 7.42 | 17.08 |
| synthetic gradient | dav1d | 23.92 (1) | 23.83–24.12 | 23.92 | 19.00 |

rav1d-asm decodes in 0.32 times Zen's time and 0.76 times dav1d's, with lower memory than either. The rav1d timing excludes container parsing, which dav1d's includes; img's parser is in-tree and bounded. The dav1d selection is withdrawn: img keeps rav1d with assembly enabled. 4:4:4 decode and Linux timing are unmeasured.
