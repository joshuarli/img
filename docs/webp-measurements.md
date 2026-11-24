# WebP measurements

The current campaign baseline and measurement protocol are in the [roadmap](roadmap.md#campaign-webp-hillclimb). Detailed matched-score captures and reproduction context are in [BENCHMARKS](../BENCHMARKS.md). Measurements below describe the configurations tested, not a claim of current parity.

The 2026-11-15 matched-score capture deepened the optimal-parse chain walk to 256 and reduced large.png lossless output from 12,682 to 4,846 bytes. Its wall rose from 8,640 to 40,385 ms. The captured destination gate remained unmet.

The 2026-11-18 lossless comparison recorded camera L9 at 124,614 bytes versus cwebp -z9 at 123,720, coffee L9 at 340,156 versus 336,030, and mm-logo-alpha L1 at 24,720 versus 23,696. These cases do not bound the corpus.

In the newest capture containing both tools, five-run serial workflow medians were coffee minimum 80: 1.040 s versus 0.309 s, grass minimum 95: 1.430 s versus 0.393 s, and gravel minimum 90: 1.382 s versus 0.364 s. Achieved scores were respectively 80.42 versus 85.82, 95.22 versus 95.73, and 90.10 versus 94.19. See [timed capture](../benchmarks/results/timed-fast-defer.json). Later img-only runs cannot be compared because host drift was measured.

Several searches were rejected on the measured configurations: a trellis inner-loop micro-hoist was slower in all five paired rounds; an all-literal meta-Huffman shot saved 38 bytes on coins L6 while costing 5–11% wall on other inputs; transform-spec hashing found 20 distinct pixel streams among 20 mm-logo-alpha specs. These results do not rule out other changes.

At revision `ddb143b`, 373 lossless library tests passed with 4096 proptest cases, and independent dwebp decoding of L6 chelsea, text and coins matched source RGBA pixels. Native Linux timing was not measured.
