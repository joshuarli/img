# PNG decode qualification

Do not integrate a candidate: neither permissive native decoder is 10% faster than png 0.18.1. zune-png 0.5.2 is disqualified because it changes alpha on valid 16-bit grayscale and RGB tRNS samples; [the fixtures](../../benchmarks/codecs/png/decode-exact-fixtures.py) reproduce the difference. Zen is AGPL and benchmark-only.

On an Apple M1, 137 fixtures had zero sample-hash mismatches and zero decode errors for png, zenpng 0.1.4, libspng 0.7.4 and libpng 1.6.59; each rejected six corrupt or truncated variants. The set includes 95 supported dev images, generated palette, gray, alpha, 16-bit and Adam7 cases, and img fast/best and zlib re-encodings. Five JPEG dev originals were rejected by the base CLI because of auxiliary images or trailing payloads.

Nine shuffled paired rounds passed the pre/post load gates with zero discards, each sample running for at least 150 ms. Ratios below are candidate time divided by png time: lower is better. Confidence intervals bootstrap per-file median paired ratios; p95 and worst are across files. RSS uses a separate single-decode run per fixture.

| Candidate | All files time geomean [95% CI] | p95 | Worst | RSS geomean / worst |
| --- | --- | --- | --- | --- |
| libspng + miniz | 2.040 [1.972, 2.111] | 2.899 | 4.109 | 0.960 / 1.023 |
| libpng | 1.413 [1.347, 1.484] | 2.432 | 3.284 | 0.981 / 1.047 |

The [per-class report](../../benchmarks/results/codecs/png/decode-report-2026-11-19.txt) compares each exact candidate with png and Zen and identifies worst files. [Raw samples and fixture hashes](../../benchmarks/results/codecs/png/decode-qualify-2026-11-19.json) include the binary and native-library hashes. An initial RSS run accidentally skipped decoding; it was discarded and rerun with the separately hashed corrected binary. The timed binary always decoded. The benchmark links libpng 1.6.59 locally; libspng uses miniz 2.2.0. libspng is BSD-2-Clause with libpng-2.0 portions, miniz is MIT, and libpng uses the libpng license. No candidate was added to production.
