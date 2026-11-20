# Perceptual comparison

img compares actual decoded candidate bytes with an orientation-correct,
eight-bit SDR reference. Color preparation uses floating-point linear sRGB.
Validated ICC sources, including wide-gamut Display P3 phone photos, convert
directly to extended-range linear sRGB (a linear-TRC sRGB destination profile)
without clamping, as libjxl's `ssimulacra2` does. Converting to encoded sRGB
and linearizing afterwards both cost two transfer-function evaluations per
sample and mis-mapped negative values: the CMS extends sRGB's linear segment
below zero, so mirroring the power curve made Adobe RGB green read -1.02
instead of -0.078 in linear red. On four ICC-tagged photographs scored as PNG
pairs, the direct path moved img 0.006–0.058 from jpeg-xl 0.13.0
`ssimulacra2`, against 0.015–0.075 before, and made `img compare` 2.5–3.9x
faster.
On three P3 photos at JPEG qualities 60 and 85, img stayed within 0.003–0.052
of jpeg-xl 0.13.0 `ssimulacra2` on the same files, against 0.007–0.068 on
sRGB photos where both tools differ only in JPEG decoding. Clamping instead
refused such photos outright, and even sRGB-tagged inputs whose profile
transform overshot by 0.0002.
Alpha must match exactly; transparent content is scored over black and white
in linear light, and the lower score wins. Hidden RGB remains relevant to
strict lossless equality, but contributes nothing at zero alpha to this metric.
Measured SSIMULACRA2 scores may be negative. Requested minima are finite 0–100.
Identity is 100; nonidentical images with either side below eight pixels fail.

`fast-ssim2` 0.9.1 supplies owned reference pyramids reused across candidates.
Its BSD-2-Clause AND BSD-3-Clause AND MIT license is compatible with the
permissive distribution. Default features are disabled; no native runtime
or codec threads are introduced.

Run `make metric-reference-check` with JPEG XL 0.12.0 `cjxl` and `ssimulacra2`
on PATH. The checked script wraps raw little-endian, bottom-up RGB float PFM
samples in lossless JPEG XL with explicit `RGB_D65_SRG_Rel_Lin` color encoding.
This step matters: the reference tool assumes **sRGB transfer** for untagged
PFM, even for float samples. Merely writing linear PFM, or round-tripping its
values through a nonlinear transfer, does not establish identical prepared
pixels. Lossless float JPEG XL supplies the original linear values directly.
The tool is a development oracle and is never linked into production.

Goldens were captured on macOS ARM64 with Homebrew jpeg-xl 0.12.0_1.
The ssimulacra2 executable SHA-256 was
`3186139b975c9e0ca429c3a7c93053887d6a804f707507eec2a36024220959fa`.
Four controlled cases cover the eight-pixel floor, noise, text-like edges,
non-power-of-two dimensions, and gradients. Their maximum observed absolute
difference was below 0.000001; the prescribed tolerance stays 0.05.
`tests/metric_reference.rs` stores fixed offline goldens. Preparation and alpha
policy have separate tests; these metric goldens do not establish photographic
compression competitiveness or validate unsupported HDR/profile paths.
