# Corpus provenance

The four photographs are unchanged files from scikit-image v0.25.2.
`manifest.json` pins source URLs, dimensions, SHA-256, attribution and license
evidence from that release's data fetchers. Astronaut is the NASA Eileen Collins
portrait identified there as public domain. Coffee is Rachel Michetti's CC0
photograph, courtesy of Pikolo Espresso Bar. Grass is linolafett's CC0 photograph,
cropped and converted to grayscale by scikit-image. Gravel is CC0Textures' CC0
photograph, cropped and converted to grayscale by scikit-image.
These documented transformations precede acquisition by this repository.
No identity endorsement or publicity right is asserted for the NASA portrait.

The procedural corpus generator is CC0-1.0, written for this repository.
It supplies UI-like blocks, text-like strokes, line art, flat colors,
transparency and hidden RGB, grayscale, a large 2048×1024 gradient, a standard
sRGB ICC profile, 16-bit samples and an already compressed WebP. These synthetic
cases establish robustness and exactness, not photographic competitiveness.

Do not add the repository's older unproven photographic fixtures to this corpus.

## Validation corpus (`validation/`)

`validation/manifest.json` is a held-out set for qualification, kept out of
development knob selection. It contains 34 pinned files: unused
scikit-image v0.25.2 data images with the same fetcher-documented licenses
(eagle-sized strata replaced by larger Commons files since eagle.png is not in
the v0.25.2 source tree), Wikimedia Commons CC0 photographs spanning
0.4–14 MP including odd dimensions and 1–4 MP and >4K workloads, a real UI
screenshot, a real RGBA logo, a scanned-text page, and twelve procedural
boundary fixtures whose pixel counts straddle the 16,384 / 200,000 / 400,000 /
1,048,576 lossless-effort band edges on both sides. If validation inputs are
ever used for tuning, declare it and obtain fresh validation inputs.
