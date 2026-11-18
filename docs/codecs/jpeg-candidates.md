# JPEG candidates

This is an ARM64 macOS comparison, not an integration decision. Retain mozjpeg-rs for encoding and zune-jpeg for decoding provisionally. Use ProgressiveBalanced for Fast and Balanced and ProgressiveSmallest for Best, matching the current adapter. Revisit both roles after timing and the fidelity refusals below are resolved. Zen is a benchmark reference only and is ineligible for production.

## Sources and notices

Source archive SHA-256 values below cover the named archive, not submodules. [libjxl v0.11.1](https://github.com/libjxl/libjxl/tree/v0.11.1) commit `794a5dcf0d54f9f0b20d288a12e87afb91d20dfc`, archive `1492dfef8dd6c3036446ac3b340005d92ab92f7d48ee3271b5dac1d36945d3d9`, provides jpegli under BSD-3-Clause. Its linked tool notices include Highway `457c891775a7397bdb0376bb1031e6e027af1c48` (Apache-2.0), Brotli `36533a866ed1ca4b75cf049f4521e4ec5fe24727` (MIT), skcms `42030a771244ba67f86b1c1c76a6493f873c5f91` (BSD-3-Clause), libpng `f135775ad4e5d4408d2e12ffcc71bb36e6b48551` (libpng-2.0), zlib `51b7f2abdade71cd9bb0e7a373ef2610ec6f9daf` (Zlib), and the separately pinned libjpeg-turbo below. The codec library itself links Highway; the CLI also links the other listed libraries and libjxl. Recheck the actual linked set and notices at vendoring.

[Native MozJPEG v4.1.5](https://github.com/mozilla/mozjpeg/tree/v4.1.5), commit `6c9f0897afa1c2738d7222a0a9ab49e8b536a267`, archive `9fcbb7171f6ac383f5b391175d6fb3acde5e64c4c4727274eade84ed0998fcc1`: IJG, BSD-3-Clause and Zlib notices; the measured CLI additionally links the libpng and zlib above. [libjpeg-turbo 3.1.1](https://github.com/libjpeg-turbo/libjpeg-turbo/tree/3.1.1), commit `7723f50f3f66b9da74376e6d8badb6162464212c`, archive `304165ae11e64ab752e9cfc07c37bfdc87abd0bfe4bc699e59f34036d9c84f72`: IJG, BSD-3-Clause and Zlib notices.

[mozjpeg-rs 0.9.2](https://crates.io/crates/mozjpeg-rs/0.9.2), crate SHA-256 `c53b32459137fac7d3266dc6f268d58f7c23dd2dc82028d4b0c3edd024275e8f`, is BSD-3-Clause. Its locked dependency notices are archmage and archmage-macros, autocfg, bytemuck and bytemuck_derive, enough, imgref, libm, linear-srgb, magetypes, num-traits, proc-macro2, quote, rgb, rustversion, safe_unaligned_simd, syn, unicode-ident, wide and zenyuv. Their license choices include MIT, Apache-2.0, Zlib, CC0-1.0 and Unicode-3.0; retain the selected notices. [zune-jpeg 0.5.15](https://crates.io/crates/zune-jpeg/0.5.15), crate SHA-256 `27bc9d5b815bc103f142aa054f561d9187d191692ec7c2d1e2b4737f8dbd7296`, and zune-core 0.5.1 offer MIT, Apache-2.0 or Zlib.

[zenjpeg 0.8.4](https://crates.io/crates/zenjpeg/0.8.4), crate SHA-256 `4fc00260b76e859d71f31ceab888e6691508569e69ed6eeed3a42ef9791cfd7f`, is AGPL-3.0-only or commercial. It is measured only as a reference. The prebuilt `zen-5424df8` SHA-256 is `38a2d911f6bae232ac4b25e6fb283a22e349b35d1e827f0b4245f3d02307a9be`; `img-master-2258e42` is `a6bbb769e61d8a2d34f75928cab39b2f84bc18a35a2d0b560f93cea5a9d3d617`.

## Method and reproduction

The 23 eligible inputs comprise 14 color photos, eight grayscale images and one text image. Alpha-bearing horse, photopea UI and logo images and boundary fixtures are excluded. The text image is the UI/text group. Sweep qualities 50 through 95 in steps of five, plus 97. `img-master compare` supplies every score; failures remain in the JSON. Linear interpolation uses adjacent successful sweep points without extrapolation. Each production row in a group and score uses the intersection of images bracketed by every production configuration. The parenthesized count is that intersection; XYB is context and has its own count. Empty intersections are unavailable. Mean bytes are rounded to whole bytes. Different score columns can have different images and are not comparable as totals.

jpegli's default has XYB disabled; the XYB row explicitly enables it and is not a production candidate. Native MozJPEG uses progressive trellis defaults and the named chroma sampling. The two Rust CLIs use the named effort. The pinned libjpeg-turbo decoder checks each output. Progressive JPEG here means a standard JPEG bitstream decodable by libjpeg-turbo, not baseline sequential scan mode.

On this host, use `sh benchmarks/codecs/jpeg/build.sh` in a fresh scratch directory, with the two immutable reference binaries supplied by the brief. The script builds pinned source with at most four jobs and needs the cached Rust crates for the benchmark-only decoder harness. Run `python3 benchmarks/codecs/jpeg/sweep.py /Users/josh/d/img/.work/tmp/jpeg-candidates/full-sweep.json`. Run `python3 benchmarks/codecs/jpeg/report.py benchmarks/results/codecs/jpeg/results.json` to reproduce the tables. The standalone timing batch is `python3 benchmarks/codecs/jpeg/timed.py /Users/josh/d/img/.work/tmp/jpeg-candidates/timing.json`. It checks `perf-gate-ready` before the batch and `perf-gate-ready -1` after, discarding a failed batch. Timing uses five alternating rounds, unique output paths, `perf_counter` wall time and `wait4` CPU and peak RSS. Per-image medians are averaged by group; parenthesized counts are complete images. Decoder CLI output work differs: the native tools write PNM and the Rust harness retains decoded pixels in memory.

### color photo (14 images)

| Configuration | Score 80: mean bytes (n) | Score 90: mean bytes (n) | Score 95: mean bytes (n) |
| --- | ---: | ---: | ---: |
| jpegli | 71534 (2) | 217417 (1) | unavailable (0) |
| jpegli-xyb | unavailable (0) | unavailable (0) | unavailable (0) |
| moz-420 | 71974 (2) | 167221 (1) | unavailable (0) |
| moz-444 | 85005 (2) | 206664 (1) | unavailable (0) |
| img-fast | 72365 (2) | 172259 (1) | unavailable (0) |
| img-best | 71849 (2) | 171193 (1) | unavailable (0) |
| zen-fast | 67856 (2) | 179812 (1) | unavailable (0) |
| zen-best | 67856 (2) | 179812 (1) | unavailable (0) |

### grayscale (8 images)

| Configuration | Score 80: mean bytes (n) | Score 90: mean bytes (n) | Score 95: mean bytes (n) |
| --- | ---: | ---: | ---: |
| jpegli | 40142 (5) | 51354 (7) | 18229 (3) |
| jpegli-xyb | unavailable (0) | unavailable (0) | unavailable (0) |
| moz-420 | 38426 (5) | 54022 (7) | 20868 (3) |
| moz-444 | 38426 (5) | 54009 (7) | 20868 (3) |
| img-fast | 38670 (5) | 53928 (7) | 21173 (3) |
| img-best | 38670 (5) | 53928 (7) | 21173 (3) |
| zen-fast | 36022 (5) | 47299 (7) | 18299 (3) |
| zen-best | 36022 (5) | 47299 (7) | 18299 (3) |

### ui/text (1 images)

| Configuration | Score 80: mean bytes (n) | Score 90: mean bytes (n) | Score 95: mean bytes (n) |
| --- | ---: | ---: | ---: |
| jpegli | 9645 (1) | 21802 (1) | unavailable (0) |
| jpegli-xyb | unavailable (0) | unavailable (0) | unavailable (0) |
| moz-420 | 8917 (1) | 21271 (1) | unavailable (0) |
| moz-444 | 8917 (1) | 21271 (1) | unavailable (0) |
| img-fast | 8983 (1) | 21341 (1) | unavailable (0) |
| img-best | 8983 (1) | 21341 (1) | unavailable (0) |
| zen-fast | 8664 (1) | 18821 (1) | unavailable (0) |
| zen-best | 8664 (1) | 18821 (1) | unavailable (0) |

### Failures

- img-best: 11 compare failures
- img-best: 121 encode failures
- img-fast: 11 compare failures
- img-fast: 121 encode failures
- jpegli: 132 compare failures
- jpegli-xyb: 253 compare failures
- moz-420: 132 compare failures
- moz-444: 132 compare failures
- zen-best: 132 encode failures
- zen-fast: 132 encode failures

### color photo timing

| Role | Configuration | Wall ms | CPU ms | RSS MiB |
| --- | --- | ---: | ---: | ---: |
| encode | jpegli | not measured | not measured | not measured |
| encode | jpegli-xyb | not measured | not measured | not measured |
| encode | moz-420 | not measured | not measured | not measured |
| encode | moz-444 | not measured | not measured | not measured |
| encode | img-fast | not measured | not measured | not measured |
| encode | img-best | not measured | not measured | not measured |
| encode | zen-fast | not measured | not measured | not measured |
| encode | zen-best | not measured | not measured | not measured |
| decode | turbo | not measured | not measured | not measured |
| decode | jpegli | not measured | not measured | not measured |
| decode | zune | not measured | not measured | not measured |
| decode | zen | not measured | not measured | not measured |

### grayscale timing

| Role | Configuration | Wall ms | CPU ms | RSS MiB |
| --- | --- | ---: | ---: | ---: |
| encode | jpegli | not measured | not measured | not measured |
| encode | jpegli-xyb | not measured | not measured | not measured |
| encode | moz-420 | not measured | not measured | not measured |
| encode | moz-444 | not measured | not measured | not measured |
| encode | img-fast | not measured | not measured | not measured |
| encode | img-best | not measured | not measured | not measured |
| encode | zen-fast | not measured | not measured | not measured |
| encode | zen-best | not measured | not measured | not measured |
| decode | turbo | not measured | not measured | not measured |
| decode | jpegli | not measured | not measured | not measured |
| decode | zune | not measured | not measured | not measured |
| decode | zen | not measured | not measured | not measured |

### ui/text timing

| Role | Configuration | Wall ms | CPU ms | RSS MiB |
| --- | --- | ---: | ---: | ---: |
| encode | jpegli | not measured | not measured | not measured |
| encode | jpegli-xyb | not measured | not measured | not measured |
| encode | moz-420 | not measured | not measured | not measured |
| encode | moz-444 | not measured | not measured | not measured |
| encode | img-fast | not measured | not measured | not measured |
| encode | img-best | not measured | not measured | not measured |
| encode | zen-fast | not measured | not measured | not measured |
| encode | zen-best | not measured | not measured | not measured |
| decode | turbo | not measured | not measured | not measured |
| decode | jpegli | not measured | not measured | not measured |
| decode | zune | not measured | not measured | not measured |
| decode | zen | not measured | not measured | not measured |

## Open deficits

The common color-photo sample is too small to choose a production encoder. The shared comparison rejects many files because its color transform leaves the SDR sRGB gamut; the raw records retain the diagnostics. The reference CLIs also refuse some inputs for their memory reservation or metadata. XYB scores are unavailable through the required comparison and XYB is excluded from production. The quality sweep cannot bracket every requested score; extend the sweep only after the comparison refusals are understood. Compatibility beyond successful libjpeg-turbo decoding, metadata retention, build size, Linux and end-to-end throughput remain unmeasured. Decoder selection and effort mapping need valid timing and memory evidence.
