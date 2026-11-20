# Checkpoint qualification, 2026-11-20

Frozen candidate `0e4d339` (sources identical to `cc7639b`) against B `f0ceea6`,
macOS 27.2.1 ARM64 (Apple M1), every round gated by `perf-gate-ready`.

| Binary | sha256 |
| --- | --- |
| candidate `target/dist/img` | `8e77f81b76b0c343e9206e9333de9048102168739ffca1fe66b64e1e8fcd8852` |
| B `img-f0ceea6` | `b0a0487876ac2e75b5d77f88b6dd47ca283b44249e12ff752cbbeb17c7a8c223` |
| verifier (`benchmarks/codecs/verify.rs`) | `797b703c02d2e88ebd2fcb32958eb3a63d523eb0e53e902388422f6a76a953c0` |

The only competitor in this checkpoint is B. Settings: scored workflows at score
90, balanced effort, JPEG/PNG/AVIF; lossless workflows to PNG; batch lossless
`-j 1` and `-j 4`; 5 workflow rounds, 3 batch rounds, 9 JPEG decode rounds.

## Reproduce

```
scripts/bench build --base f0ceea6
scripts/bench run --tier scenario --base f0ceea6
scripts/bench run --tier scenario --base f0ceea6 --split holdout --skip jpeg-decode
```

The JPEG decode fixtures come from development images, so holdout skips them.
`score-exact` always uses the development smoke set (see below). The dev
`score-exact` and `jpeg-decode` rows were rerun with the same binaries after the
harness fixes in this commit.

Each `run.json` keeps every sample's tool, input, exit status, wall, CPU, peak
RSS, output bytes and sha256, plus the verifier's per-case verdict (exact or
measured score, accepted, identical across rounds). Local path prefixes are
stripped. `report.txt` files and `*.verdict.json` were regenerated from these
files with `BENCH_DISPOSITIONS=dispositions.json`; `index.json` lists every
verdict.

## Results (candidate / B)

| Workload | Dev | Holdout |
| --- | --- | --- |
| Scored workflow, 36 / 150 cases | 18 vs 16 succeeded; 16 both: wall 0.495 [0.376, 0.636], CPU 0.543, RSS 0.619, bytes 0.987 | 68 vs 55 succeeded; 55 both: wall 0.482 [0.419, 0.554], CPU 0.508, RSS 0.603, bytes 1.002 [0.997, 1.008] (inconclusive against the 1.002 budget) |
| Lossless workflow, 12 / 50 cases | 7 vs 6; wall 1.007, CPU 0.986, RSS 0.917, bytes 1.000 | 21 vs 18; wall 0.829, CPU 1.033 (inconclusive), RSS 0.836, bytes 1.000 |
| PNG encode loop (fast/balanced/best) | encode 0.775 [0.676, 0.880], bytes 1.000 | 0.781 [0.723, 0.846], bytes 1.000 |
| Batch lossless | 55/55 both; `-j 4` 102 vs 37 img/s | 18/18 both; `-j 4` 13.0 vs 4.1 img/s |
| JPEG decode, 9 rounds | 0.72–0.93 by class, RSS 0.98–1.03 | not run (dev fixtures) |
| SSIMULACRA2 bit identity | 5/5 identical, per-score 0.758 | not applicable |

No case fails for the candidate where B succeeds, and every retained output is
identical across rounds. Failures shared by both are contract refusals
(non-sRGB ICC or XMP in AVIF, auxiliary images and gain maps, trailing JPEG
payloads, 16-bit scoring) or no qualifying candidate at the score. B
additionally rejects large photos at admission and wide-gamut sources.

## Dispositions

`dispositions.json` explains every case over the per-case limit: one 11 ms JPEG
search case that ends with a smaller output nearer the target, three parallel
deflate CPU-for-latency cases, B's serial `-j 4` batch, and one AVIF file
(`synthetic-17.png`, +10.6% bytes at score 90.48 against 90.18). The AVIF case
is a real compression loss from the converged-frontier stop rule and stays open.

## Score bit identity against B

Against B the original `score-exact` reported different scores on four of
five files, because both scorers decoded the same JPEG bytes differently:
`7b2ba88` moved JPEG decoding to libjpeg-turbo. With the source and candidate
first decoded to lossless PNG by one binary, B's and the candidate's score
loops return identical bits on all five files. `score-exact` now measures that
directly. On holdout, B's scorer rejects wide-gamut sources (photo-02, 12 MP:
"color transform leaves the SDR sRGB gamut", fixed by `6dc42a1`), so the check
stays on the development smoke set.
