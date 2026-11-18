# JPEG candidates

This is an ARM64 macOS comparison. Encoder and decoder selections are provisional until gated equivalent-work timing is complete. Zen is a benchmark reference only and is ineligible for production.

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
| encode | jpegli-420 | not measured | not measured | not measured | not measured |
| encode | jpegli-444 | not measured | not measured | not measured | not measured |
| encode | moz-420 | not measured | not measured | not measured | not measured |
| encode | moz-444 | not measured | not measured | not measured | not measured |
| encode | img-fast | not measured | not measured | not measured | not measured |
| encode | img-best | not measured | not measured | not measured | not measured |
| encode | zen-fast | not measured | not measured | not measured | not measured |
| encode | zen-best | not measured | not measured | not measured | not measured |
| decode | turbo | not measured | not measured | not measured | not measured |
| decode | jpegli | not measured | not measured | not measured | not measured |
| decode | zune | not measured | not measured | not measured | not measured |
| decode | zen | not measured | not measured | not measured | not measured |

### grayscale timing

| Role | Configuration | Wall ms | Wall range ms | User CPU ms | RSS MiB |
| --- | --- | ---: | ---: | ---: | ---: |
| encode | jpegli-420 | not measured | not measured | not measured | not measured |
| encode | jpegli-444 | not measured | not measured | not measured | not measured |
| encode | moz-420 | not measured | not measured | not measured | not measured |
| encode | moz-444 | not measured | not measured | not measured | not measured |
| encode | img-fast | not measured | not measured | not measured | not measured |
| encode | img-best | not measured | not measured | not measured | not measured |
| encode | zen-fast | not measured | not measured | not measured | not measured |
| encode | zen-best | not measured | not measured | not measured | not measured |
| decode | turbo | not measured | not measured | not measured | not measured |
| decode | jpegli | not measured | not measured | not measured | not measured |
| decode | zune | not measured | not measured | not measured | not measured |
| decode | zen | not measured | not measured | not measured | not measured |

### ui/text timing

| Role | Configuration | Wall ms | Wall range ms | User CPU ms | RSS MiB |
| --- | --- | ---: | ---: | ---: | ---: |
| encode | jpegli-420 | not measured | not measured | not measured | not measured |
| encode | jpegli-444 | not measured | not measured | not measured | not measured |
| encode | moz-420 | not measured | not measured | not measured | not measured |
| encode | moz-444 | not measured | not measured | not measured | not measured |
| encode | img-fast | not measured | not measured | not measured | not measured |
| encode | img-best | not measured | not measured | not measured | not measured |
| encode | zen-fast | not measured | not measured | not measured | not measured |
| encode | zen-best | not measured | not measured | not measured | not measured |
| decode | turbo | not measured | not measured | not measured | not measured |
| decode | jpegli | not measured | not measured | not measured | not measured |
| decode | zune | not measured | not measured | not measured | not measured |
| decode | zen | not measured | not measured | not measured | not measured |

## Open deficits

Timing is pending. Score 95 has no common color-photo bracket for the 4:2:0 configurations. XYB cannot be scored by the shared comparison because its ICC profile is rejected. The original images retain their profiles and full resolution; these untagged, scaled inputs do not qualify color-management, metadata, alpha, or full-resolution behavior. Linux, binary size, build cost and end-to-end throughput remain unmeasured.
