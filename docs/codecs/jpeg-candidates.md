# JPEG candidates

This is an ARM64 macOS comparison of single-threaded encoders and decoders on scaled, untagged inputs. Zen is a benchmark reference only and is ineligible for production.

## Sources and notices

Source archive SHA-256 values below cover the named archive, not submodules. [libjxl v0.11.1](https://github.com/libjxl/libjxl/tree/v0.11.1) commit `794a5dcf0d54f9f0b20d288a12e87afb91d20dfc`, archive `1492dfef8dd6c3036446ac3b340005d92ab92f7d48ee3271b5dac1d36945d3d9`, provides jpegli under BSD-3-Clause. Its linked tool notices include Highway `457c891775a7397bdb0376bb1031e6e027af1c48` (Apache-2.0), Brotli `36533a866ed1ca4b75cf049f4521e4ec5fe24727` (MIT), skcms `42030a771244ba67f86b1c1c76a6493f873c5f91` (BSD-3-Clause), libpng `f135775ad4e5d4408d2e12ffcc71bb36e6b48551` (libpng-2.0), zlib `51b7f2abdade71cd9bb0e7a373ef2610ec6f9daf` (Zlib), and the separately pinned libjpeg-turbo below. The codec library itself links Highway; the CLI also links the other listed libraries and libjxl. Recheck the actual linked set and notices at vendoring.

[Native MozJPEG v4.1.5](https://github.com/mozilla/mozjpeg/tree/v4.1.5), commit `6c9f0897afa1c2738d7222a0a9ab49e8b536a267`, archive `9fcbb7171f6ac383f5b391175d6fb3acde5e64c4c4727274eade84ed0998fcc1`: IJG, BSD-3-Clause and Zlib notices; the measured CLI additionally links the libpng and zlib above. [libjpeg-turbo 3.1.1](https://github.com/libjpeg-turbo/libjpeg-turbo/tree/3.1.1), commit `7723f50f3f66b9da74376e6d8badb6162464212c`, archive `304165ae11e64ab752e9cfc07c37bfdc87abd0bfe4bc699e59f34036d9c84f72`: IJG, BSD-3-Clause and Zlib notices.

[mozjpeg-rs 0.9.2](https://crates.io/crates/mozjpeg-rs/0.9.2), crate SHA-256 `c53b32459137fac7d3266dc6f268d58f7c23dd2dc82028d4b0c3edd024275e8f`, is BSD-3-Clause. Its locked dependency notices are archmage and archmage-macros, autocfg, bytemuck and bytemuck_derive, enough, imgref, libm, linear-srgb, magetypes, num-traits, proc-macro2, quote, rgb, rustversion, safe_unaligned_simd, syn, unicode-ident, wide and zenyuv. Their license choices include MIT, Apache-2.0, Zlib, CC0-1.0 and Unicode-3.0; retain the selected notices. [zune-jpeg 0.5.15](https://crates.io/crates/zune-jpeg/0.5.15), crate SHA-256 `27bc9d5b815bc103f142aa054f561d9187d191692ec7c2d1e2b4737f8dbd7296`, and zune-core 0.5.1 offer MIT, Apache-2.0 or Zlib.

[zenjpeg 0.8.4](https://crates.io/crates/zenjpeg/0.8.4), crate SHA-256 `4fc00260b76e859d71f31ceab888e6691508569e69ed6eeed3a42ef9791cfd7f`, is AGPL-3.0-only or commercial. It is measured only as a reference. The prebuilt `zen-5424df8` SHA-256 is `38a2d911f6bae232ac4b25e6fb283a22e349b35d1e827f0b4245f3d02307a9be`; `img-master-2258e42` is `a6bbb769e61d8a2d34f75928cab39b2f84bc18a35a2d0b560f93cea5a9d3d617`.

## Method and reproduction

The 23 eligible images comprise 14 color photos, eight grayscale images and one text image. Alpha-bearing inputs and boundary fixtures are excluded. `prepare.py` decodes PNG samples without color conversion and decodes JPEG with pinned libjpeg-turbo at the largest M/8 scale with long side at most 2048 pixels. It writes untagged 8-bit PNG and identical PNM samples, and verifies the PNG chunk list and round-trip samples. The results JSON records each prepared PNG SHA-256 and scale. Treating samples as untagged sRGB intentionally disregards embedded color profiles and orientation; this is a qualification corpus, not a faithful color-managed rendering of every original.

Each encoder uses every prepared input. Sweep qualities 50 through 95 in steps of five, plus 97, 99 and 100; grayscale and text also use 20 through 45 in steps of five. The current img binary supplies every SSIMULACRA2 score. Interpolation uses adjacent successful sweep points without extrapolation. Each production row within a group and target uses the same images bracketed by every production configuration; counts can differ between targets. A positive relative byte percentage means the numerator is larger. XYB is context and is ineligible for production. Native MozJPEG uses progressive trellis defaults; jpegli uses its YCbCr defaults with the named sampling. The pinned libjpeg-turbo decoder checks each output.

Run `sh benchmarks/codecs/jpeg/build.sh` in a fresh scratch directory with the two supplied immutable reference binaries and cached Rust crates. Run `python3 benchmarks/codecs/jpeg/prepare.py`, then `python3 benchmarks/codecs/jpeg/sweep.py /Users/josh/d/img/.work/tmp/jpeg-candidates/qualification-sweep.json` and `python3 benchmarks/codecs/jpeg/parity.py`. The result JSON contains the prepared input manifest and per-image sweep records. `python3 benchmarks/codecs/jpeg/report.py benchmarks/results/codecs/jpeg/results.json` reproduces all tables. The gated timing command is `python3 benchmarks/codecs/jpeg/timed.py /Users/josh/d/img/.work/tmp/jpeg-candidates/qualification-timing.json`; it resumes completed per-image batches and discards a batch when either gate fails.

Encoder timing uses direct in-memory library APIs, excluding input reads, process startup and output writes. It includes encoder setup, encoding and in-memory output allocation. Each image and configuration is run five times at both sweep qualities bracketing score 90, in alternating configuration order. The report interpolates the per-quality medians and wall ranges to score 90. Encoder RSS is the larger median process peak at the two bracket points. Decoder timing includes process startup and I/O; native decoders write PNM while Rust decoders retain pixels in memory. Counts are common completed images. Wall ranges are means of per-image minima and maxima; CPU is user CPU and RSS is process peak, including input buffers.

### color photo (14 images)

| Configuration | Score 80: mean bytes (n) | Score 90: mean bytes (n) | Score 95: mean bytes (n) |
| --- | ---: | ---: | ---: |
| jpegli-420 | 315781 (14) | 849843 (7) | unavailable (0) |
| jpegli-444 | 301076 (14) | 798309 (7) | unavailable (0) |
| jpegli-xyb | unavailable (0) | unavailable (0) | unavailable (0) |
| moz-420 | 316084 (14) | 853244 (7) | unavailable (0) |
| moz-444 | 350986 (14) | 873400 (7) | unavailable (0) |
| img-fast | 318070 (14) | 857850 (7) | unavailable (0) |
| img-best | 317252 (14) | 851937 (7) | unavailable (0) |
| zen-fast | 321547 (14) | 830954 (7) | unavailable (0) |
| zen-best | 321547 (14) | 830954 (7) | unavailable (0) |

| Relative bytes | Score 80 | Score 90 | Score 95 |
| --- | ---: | ---: | ---: |
| jpegli-420 / zen-best | -1.8% (14) | +2.3% (7) | unavailable (0) |
| jpegli-444 / zen-best | -6.4% (14) | -3.9% (7) | unavailable (0) |
| img-fast / zen-best | -1.1% (14) | +3.2% (7) | unavailable (0) |
| img-best / zen-best | -1.3% (14) | +2.5% (7) | unavailable (0) |
| jpegli-420 / img-fast | -0.7% (14) | -0.9% (7) | unavailable (0) |
| jpegli-444 / img-fast | -5.3% (14) | -6.9% (7) | unavailable (0) |

### grayscale (8 images)

| Configuration | Score 80: mean bytes (n) | Score 90: mean bytes (n) | Score 95: mean bytes (n) |
| --- | ---: | ---: | ---: |
| jpegli-420 | 31132 (7) | 45034 (8) | 62001 (7) |
| jpegli-444 | 31132 (7) | 45034 (8) | 62001 (7) |
| jpegli-xyb | unavailable (0) | unavailable (0) | unavailable (0) |
| moz-420 | 29536 (7) | 47361 (8) | 67746 (7) |
| moz-444 | 29536 (7) | 47349 (8) | 67746 (7) |
| img-fast | 29837 (7) | 47293 (8) | 68238 (7) |
| img-best | 29837 (7) | 47293 (8) | 68238 (7) |
| zen-fast | 27979 (7) | 41492 (8) | 60467 (7) |
| zen-best | 27979 (7) | 41492 (8) | 60467 (7) |

| Relative bytes | Score 80 | Score 90 | Score 95 |
| --- | ---: | ---: | ---: |
| jpegli-420 / zen-best | +11.3% (7) | +8.5% (8) | +2.5% (7) |
| jpegli-444 / zen-best | +11.3% (7) | +8.5% (8) | +2.5% (7) |
| img-fast / zen-best | +6.6% (7) | +14.0% (8) | +12.9% (7) |
| img-best / zen-best | +6.6% (7) | +14.0% (8) | +12.9% (7) |
| jpegli-420 / img-fast | +4.3% (7) | -4.8% (8) | -9.1% (7) |
| jpegli-444 / img-fast | +4.3% (7) | -4.8% (8) | -9.1% (7) |

### ui/text (1 images)

| Configuration | Score 80: mean bytes (n) | Score 90: mean bytes (n) | Score 95: mean bytes (n) |
| --- | ---: | ---: | ---: |
| jpegli-420 | 9645 (1) | 21802 (1) | 35658 (1) |
| jpegli-444 | 9645 (1) | 21802 (1) | 35658 (1) |
| jpegli-xyb | unavailable (0) | unavailable (0) | unavailable (0) |
| moz-420 | 8917 (1) | 21271 (1) | 36930 (1) |
| moz-444 | 8917 (1) | 21271 (1) | 36930 (1) |
| img-fast | 8983 (1) | 21341 (1) | 37212 (1) |
| img-best | 8983 (1) | 21341 (1) | 37212 (1) |
| zen-fast | 8664 (1) | 18821 (1) | 32386 (1) |
| zen-best | 8664 (1) | 18821 (1) | 32386 (1) |

| Relative bytes | Score 80 | Score 90 | Score 95 |
| --- | ---: | ---: | ---: |
| jpegli-420 / zen-best | +11.3% (1) | +15.8% (1) | +10.1% (1) |
| jpegli-444 / zen-best | +11.3% (1) | +15.8% (1) | +10.1% (1) |
| img-fast / zen-best | +3.7% (1) | +13.4% (1) | +14.9% (1) |
| img-best / zen-best | +3.7% (1) | +13.4% (1) | +14.9% (1) |
| jpegli-420 / img-fast | +7.4% (1) | +2.2% (1) | -4.2% (1) |
| jpegli-444 / img-fast | +7.4% (1) | +2.2% (1) | -4.2% (1) |

### Failures

- jpegli-xyb: 353 compare failures: input.score: ICC profile is not a validated RGB matrix or gray profile

### color photo timing

| Role | Configuration | Wall ms | Wall range ms | User CPU ms | RSS MiB |
| --- | --- | ---: | ---: | ---: | ---: |
| encode | jpegli-420 | 40.69 (7) | 40.55–40.92 | 39.50 | 23.23 |
| encode | jpegli-444 | 48.36 (7) | 48.12–48.55 | 46.66 | 29.20 |
| encode | moz-420 | 413.04 (7) | 412.55–413.79 | 409.47 | 28.16 |
| encode | moz-444 | 440.77 (7) | 440.16–441.50 | 436.08 | 41.66 |
| encode | img-fast | 236.23 (7) | 235.94–236.75 | 232.82 | 40.44 |
| encode | img-best | 507.61 (7) | 507.12–508.37 | 501.87 | 54.39 |
| encode | zen-fast | 47.82 (7) | 47.72–48.07 | 46.54 | 25.30 |
| encode | zen-best | 47.85 (7) | 47.74–48.02 | 46.52 | 25.29 |
| decode | turbo | 20.59 (14) | 20.32–21.62 | 11.32 | 7.10 |
| decode | jpegli | 20.46 (14) | 20.24–21.07 | 15.64 | 17.78 |
| decode | zune | 17.43 (14) | 17.26–18.02 | 14.12 | 12.79 |
| decode | zen | 17.26 (14) | 17.12–17.45 | 13.94 | 13.31 |

### grayscale timing

| Role | Configuration | Wall ms | Wall range ms | User CPU ms | RSS MiB |
| --- | --- | ---: | ---: | ---: | ---: |
| encode | jpegli-420 | 2.20 (8) | 2.19–2.23 | 2.13 | 2.94 |
| encode | jpegli-444 | 2.21 (8) | 2.19–2.26 | 2.13 | 2.94 |
| encode | moz-420 | 22.08 (8) | 21.98–22.16 | 21.89 | 3.43 |
| encode | moz-444 | 22.05 (8) | 21.99–22.11 | 21.84 | 3.42 |
| encode | img-fast | 14.91 (8) | 14.83–14.97 | 14.75 | 4.01 |
| encode | img-best | 14.86 (8) | 14.78–14.94 | 14.70 | 4.01 |
| encode | zen-fast | 2.86 (8) | 2.84–2.92 | 2.76 | 3.72 |
| encode | zen-best | 2.86 (8) | 2.85–2.88 | 2.76 | 3.72 |
| decode | turbo | 4.52 (8) | 4.42–4.71 | 1.89 | 2.30 |
| decode | jpegli | 4.73 (8) | 4.67–4.85 | 2.29 | 2.67 |
| decode | zune | 4.68 (8) | 4.60–4.80 | 2.40 | 3.13 |
| decode | zen | 4.79 (8) | 4.70–4.92 | 2.43 | 4.86 |

### ui/text timing

| Role | Configuration | Wall ms | Wall range ms | User CPU ms | RSS MiB |
| --- | --- | ---: | ---: | ---: | ---: |
| encode | jpegli-420 | 1.07 (1) | 1.06–1.09 | 1.03 | 2.38 |
| encode | jpegli-444 | 1.07 (1) | 1.07–1.10 | 1.03 | 2.38 |
| encode | moz-420 | 9.08 (1) | 9.00–9.19 | 9.00 | 2.48 |
| encode | moz-444 | 9.10 (1) | 8.98–9.15 | 8.98 | 2.47 |
| encode | img-fast | 6.35 (1) | 6.30–6.48 | 6.25 | 2.97 |
| encode | img-best | 6.36 (1) | 6.31–6.41 | 6.28 | 2.95 |
| encode | zen-fast | 1.32 (1) | 1.31–1.36 | 1.26 | 3.22 |
| encode | zen-best | 1.33 (1) | 1.31–1.34 | 1.25 | 3.22 |
| decode | turbo | 3.54 (1) | 3.50–3.63 | 1.14 | 2.05 |
| decode | jpegli | 3.87 (1) | 3.83–3.97 | 1.47 | 2.30 |
| decode | zune | 3.64 (1) | 3.59–3.77 | 1.48 | 2.50 |
| decode | zen | 3.72 (1) | 3.72–3.83 | 1.50 | 3.23 |

## Recommendation

Encoder: jpegli from libjxl v0.11.1, linked as the library with Highway, emitting standard YCbCr. Color uses 4:4:4: at matched SSIMULACRA2 it is 6.4% smaller than Zen at score 80 (14 images) and 3.9% smaller at score 90 (7 images), at 48.4 ms against Zen's 47.8 ms. It is 4.9 times faster than the current img Fast and 10.5 times faster than img Best, with lower RSS. jpegli 4:2:0 is not selected: it is only 1.8% smaller than Zen at score 80 and 2.3% larger at 90. Grayscale encodes in 2.2 ms against Zen's 2.9 ms, but output is 11.3%, 8.5% and 2.5% larger at scores 80, 90 and 95; the single text image is 10–16% larger. Both are open deficits below.

Effort mapping: Fast, Balanced and Best all use one configuration: jpegli YCbCr (4:4:4 for color, single channel for gray), progressive level 2 and optimized Huffman coding. Only this configuration was measured. Effort tiers should split only when a cheaper Fast or smaller Best measures better.

Decoder: libjpeg-turbo 3.1.1. On color photos its user CPU is 11.3 ms, against 13.9 ms for Zen and 14.1 ms for zune-jpeg, and its peak RSS is 7.1 MiB, against 13.3 MiB for Zen. Its wall time is 18% higher than Zen's. That gap is not decoder cost: the native tools write PNM output, while the Rust decoders keep pixels in memory. In-process decode timing is required before claiming a wall-time result.

## Open deficits

Grayscale and text bytes trail Zen with jpegli; candidates are jpegli quantization tables tuned for luma-only output or a gray-specific adaptive quantization setting, measured before adoption. Decoder wall time needs in-process measurement. Timing covers color score 90 only where every configuration brackets it (7 of 14 images). Score 95 has no common color-photo bracket for the 4:2:0 configurations. XYB cannot be scored by the shared comparison because its ICC profile is rejected. The original images retain their profiles and full resolution; these untagged, scaled inputs do not qualify color-management, metadata, alpha, or full-resolution behavior. The candidate comparison did not measure Linux timing, binary size, build cost or end-to-end throughput.

## Integrated encoder

The integrated `img-jpegli` encoder uses the same prepared inputs and quality sweep. All 353 matching input and quality pairs have identical byte counts and scores to `jpegli-444`. The following means use the images bracketed by both jpegli configurations at each score.

| Group | Score 80 bytes (n) | Score 90 bytes (n) | Score 95 bytes (n) |
| --- | ---: | ---: | ---: |
| Color photo | 301076 (14) | 722874 (14) | 280480 (5) |
| Grayscale | 27299 (8) | 45034 (8) | 62001 (7) |
| UI/text | 9645 (1) | 21802 (1) | 35658 (1) |

The integrated and reference jpegli values are identical in every cell. Against `zen-best` on their common bracketed images, integrated bytes are −6.4% and −3.9% for color at scores 80 and 90, +11.3%, +8.5%, and +2.5% for grayscale, and +11.3%, +15.8%, and +10.1% for UI/text at scores 80, 90, and 95. Zen does not bracket score 95 for any color photo. The macOS dist binary grew from 4,788,976 bytes at the base revision to 4,798,432 bytes, an increase of 9,456 bytes (0.20%).

At score 90, the timing run used five alternating single-thread repetitions at each quality bracket. Each input batch passed `perf-gate-ready` before and after, and the full run passed a final `perf-gate-ready -1`. The table shows the mean of per-image interpolated medians, the mean of the interpolated per-image ranges, user CPU time, and process peak RSS. All 23 inputs completed for both jpegli encoders. Zen did not bracket score 90 on seven color photos, so the table uses the common images.

| Group | Encoder | Wall ms (n) | Wall range ms | User CPU ms | Peak RSS MiB |
| --- | --- | ---: | ---: | ---: | ---: |
| Color photo | img-jpegli | 48.91 (7) | 48.67–49.45 | 47.00 | 45.06 |
| Color photo | jpegli-444 | 48.98 (7) | 48.80–49.23 | 47.40 | 29.20 |
| Color photo | zen-best | 48.46 (7) | 48.34–48.68 | 47.22 | 25.29 |
| Grayscale | img-jpegli | 2.27 (8) | 2.25–2.31 | 2.17 | 6.83 |
| Grayscale | jpegli-444 | 2.24 (8) | 2.21–2.27 | 2.17 | 2.94 |
| Grayscale | zen-best | 2.92 (8) | 2.89–2.98 | 2.81 | 3.72 |
| UI/text | img-jpegli | 1.10 (1) | 1.09–1.11 | 1.05 | 6.06 |
| UI/text | jpegli-444 | 1.10 (1) | 1.08–1.12 | 1.05 | 2.38 |
| UI/text | zen-best | 1.35 (1) | 1.34–1.37 | 1.29 | 3.22 |

The timing harness constructs a `StillImage` from a copy of the prepared PNM samples before starting its timer. Its RSS therefore includes that copy and the adapter's sample copy, while the standalone encoder reads the PNM into one buffer. RSS is process peak memory, not encoder-only allocation. The color wall-time ranges overlap, so this run does not establish a speed difference between the three encoders. The [integration records](../../benchmarks/results/codecs/jpeg/integration.json) contain the quality points and gated samples.

To repeat the integration sweep, seed a JSON list with the `quality` array from `benchmarks/results/codecs/jpeg/results.json`, set `JPEG_INTEGRATED_IMG` to the dist binary, and run `sweep.py` on that list. Put the resulting list in `integration.json` as `quality`. The `build.sh` helper builds `encode-integrated` with the current checkout and lockfile. Run `JPEG_INTEGRATION=1 python3 benchmarks/codecs/jpeg/timed.py <scratch-output>` after `perf-gate-ready` succeeds, then run `perf-gate-ready -1` immediately afterward. Add the resulting array as `timing` and run `report.py` on `integration.json`.
