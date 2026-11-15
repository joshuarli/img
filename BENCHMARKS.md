# Benchmark evidence

Performance captures use production source `0e4a4a2`; corrected comparison commands and verification are in `6535c42`. Final JPEG safety repairs are at `7f1b45b`; final artifact/resource proofs were refreshed, while timing archives retain their original source and binary identities. Results are observations of specified configurations, not a general codec competitiveness claim. All accepted timings passed perf-gate-ready before and after their run. Initial busy-host timings were discarded.

2026-11-14: retained `original-mozjpeg` outputs were rescored under the merged permissive JPEG decoder without re-encoding; outputs, byte sizes, timings, completions and the 0.0001 tolerance are unchanged, and `gravel_png-original-mozjpeg-95-0` now reads `matchedWithinHalfPoint`.

2026-11-15: a fresh matched-score WebP capture under the handoff's planned `2026-11-15-` output prefix (single run, tools `img,cwebp`, quiet host, made 2026-11-14; the prefix does not denote the calendar day) lands as `untimed-2026-11-15.json` with prefixed `2026-11-15-*.webp` outputs; the large.png lossless cells move 12,682 → 4,846 B after the optimal-parse DP chain walk deepened to 256 (`MAX_CHAIN_OPTIMAL`; `5a0c65e`). Refusal set and cwebp cells are unchanged; the backend remains experimental. A fresh timed capture records the honest wall cost of that landing (`timed-2026-11-15.json`: large.png 8640 → 40,385 ms, line/text ≈2.9×, ui/compressed ≈+33%/+2 B, gravel +11.8%, grass +3%, photos otherwise flat). A full-matrix re-score (`2026-11-15-full-` prefix; 196 records / 100 completed, tools all six) and the never-before-measured withdrawn zenwebp t90/t95 matched-score baselines are committed in the sections below; the destination gate is **not** met (lossy RD-efficiency and wall), and it is not claimed.

## Correctness and corpus

`make benchmark-verify` verifies four pinned licensed photographs and ten CC0 procedural representations, actual decoded output samples/scores, dimensions, source/output bytes, saved schema, completion counts, tool commands and gate arithmetic. [Corpus provenance](benchmarks/corpus/README.md) pins attribution, licenses and hashes. Cases cover photos, texture, UI/text/lines, transparency, grayscale, ICC, 16-bit, existing WebP and a 2048×1024 gradient.

[Untimed capture](benchmarks/results/untimed.json) has 196 records, 100 completed and 96 explicit failures/refusals. [Five-run capture](benchmarks/results/timed.json) has 980 records and 500 completed. Every completed output is retained and reverified. Summaries group by tool, fixture and target; different completion sets are never pooled into a cross-tool win. Failed work contributes no successful timing or size result.

The astronaut's nominal sRGB profile leaves the supported gamut under both independent LittleCMS and img; scored processing refuses it. Required precision/color facts and unsafe PNG ancillary metadata also cause named refusals. The original ravif 0.13.0 command emits ten-bit AVIF, outside the independently verified eight-bit scoring matrix. These are recorded failures, not zero-cost successes. Synthetic cases do not establish photographic competitiveness.

Targets are minimum actual SSIMULACRA2 scores 80, 90 and 95. `matchedWithinHalfPoint` flags proximity to the requested score, not proximity between arbitrary outputs. Overshoot is not an equal-quality win. Exact PNG compares expanded samples, depth, alpha, ICC, orientation and required color signaling. Transfer preparation is platform-stable; saved scores retain their 0.0001 tolerance. Development JSON parsing enables float_roundtrip so saved timing summaries recompute exactly.

## Reproduce comparisons

```sh
cargo build --locked --profile dist --features build-binary
cargo run --locked --profile dist --all-features --example benchmark -- capture
make benchmark-verify
make benchmark-run
```

The runner interleaves tool order across five repetitions and reverses it on alternate runs. It times process startup, encoding/search, output readback and fidelity verification with a release-built harness. Raw attempt commands, parameters, failures, architecture, revision, source-diff hash and binary hash are saved. The source-diff hash covers production and harness files; precommit captures explicitly report dirty source. Medians and full range widths are per fixture/tool/target.

The compared configurations use one image worker and RAYON_NUM_THREADS=1:

| Tool | Configuration |
| --- | --- |
| new img | fast effort, eight total candidates, exact WebP eligible before lossy search |
| new exact PNG | fast effort, lossless |
| original MozJPEG | mozjpeg 0.10.13 / mozjpeg-sys 2.2.3; native quality grid 1,25,50,75,95,100; progressive, NRobidoux tables |
| original OxiPNG | oxipng 10.2.1, native preset 2 |
| original ravif | ravif 0.13.0, same native quality grid, speed 6; ten-bit outputs refused |
| cwebp | 1.6.0, lossy grid 0,25,50,75,95,100, method 4, exact hidden RGB and alpha quality 100; no -mt |

These grids and effort settings are not equally tuned implementations. In particular cwebp's grid does not try lossless. JPEGli and an externally configurable eight-bit AVIF encoder were unavailable. The generic historical jpeg/png commands select zune encoders; earlier results mislabeled as MozJPEG/OxiPNG were discarded and replaced. The verifier now rejects that label/command mismatch.

The corrected data exposes tradeoffs. For procedural UI, exact new PNG is 142 bytes versus OxiPNG's 120; for 16-bit gradient, 520 versus 147. Both preserve the checked samples. This is not a reason to retain native production code. On coffee at minimum score 80, the evaluated WebP outputs are 93,198 bytes at score 85.62 for img and 87,502 at 85.82 for cwebp. This one close-score case favors cwebp bytes; it does not establish corpus-wide ordering. Read raw scores and failures before comparing configurations.

## Original and retained baselines

The retained-behavior oracle is `2708512`; the original comparison source is `66fc0fc`. Apply only [original-build.patch](docs/original-build.patch) in a disposable checkout: remove its undeclared Windows build script and mechanically prune those stale lock entries.

```sh
git worktree add .work/wt/original 66fc0fc
git -C .work/wt/original apply ../../../docs/original-build.patch
cd .work/wt/original
TMPDIR=/absolute/path/to/img/.work/tmp RUSTC_WRAPPER=kache cargo build --locked --release --features build-binary
```

The original has no dist profile. Its release binary is used for runtime configurations, never directly as a stripped/LTO size comparison. SVG remains excluded from parity. The initial file workload is the historical generic PNG command, with independent dimensions and every RGB sample checked; it is not an OxiPNG-specific workload.

## Transfer-table tuning

[Raw profile](benchmarks/profile-macos-arm64.txt) samples an unstripped dist build on the 2048×1024 scored WebP workflow, separately from timing. Repeated libm powers dominate a large part of candidate preparation alongside fast-ssim2 comparison. The accepted change builds 256 exact sRGB transfer values per eight-bit preparation; ICC transforms retain their float path. All 256 codes, captured prepared pixels and all 30 compared output files are byte-identical.

[Five interleaved runs per side](benchmarks/transfer-table-macos-arm64.json) include child startup through atomic publication, with validation afterward. Both binaries use the pinned compiler, normal features, dist profile, score/fast search and jobs 1. CPU is child rusage. Baseline is d21db7b, candidate is 0e4a4a2.

| Fixture | Before median ms / range width | After median ms / range width | Verdict |
| --- | --- | --- | --- |
| 2048×1024 gradient, score 95 | 2621.09 / 198.18 | 1666.85 / 98.27 | 36.4% less elapsed time, beyond spread |
| coffee, score 80 | 378.31 / 48.41 | 283.85 / 216.86 | wall-time difference inconclusive |
| UI, score 95 | 52.00 / 24.91 | 48.22 / 78.90 | inconclusive |

The demanding case's median CPU also falls from 2499.23 to 1609.64 ms, consistent with removing transfer work. This does not claim the same gain for arbitrary photos. Binary size is unchanged and the table adds 1 KiB per preparation. Reproduce with two preserved/rebuilt dist binaries:

```sh
scripts/benchmark-transfer-table BEFORE_BINARY AFTER_BINARY
```

No other optional tuning is retained on an unmeasured benefit. OxiPNG is removed because its libdeflate/native path violates the production-build contract, independently of these compression results.

## Artifact, memory and build proof

[S0 observations](benchmarks/s0-macos-arm64.json) record a 6,547,968-byte dist binary and 4,685,824-byte peak RSS for one generated 256×192 RGB PNG file conversion. [Final verification](benchmarks/s11-verification.json) records the same completed workflow with a 7,232,432-byte binary and 11,075,584-byte peak RSS. That is +10.45% size and +136.36% RSS against the initial single S0 observation. [Five interleaved resource runs](benchmarks/resources-macos-arm64.json) reproduce a 4,751,360-byte oracle median (16,384-byte range width) and 11,075,584-byte current median (49,152-byte range width), about +133.1%. These required capability costs are retained for pure-Rust codecs, resource control, decoded fidelity and durable publication; allocation attribution remains unproven. The [raw resource log](benchmarks/s11-macos-arm64-resource.txt) includes unaccepted incidental timings. The [CC0 resource fixture](benchmarks/resource-rgb.png) has exactly the common::pixels samples, pinned input/sample hashes and 147,942 input bytes. The exact outputs are 147,942 bytes for the generic oracle PNG encoder and 2,607 for the new encoder. The small image does not model maximum working memory.

The normal/build [license inventory](benchmarks/production-dependencies.json) covers 137 resolved packages. Actual macOS linkage is OS-only. Fresh masked-tool macOS builds pass. Linux ARM64 also builds after removing Clang, CMake, NASM, pkg-config and dav1d from the disposable environment, using cc only as Rust's OS linker. Its normal linkage is musl/libgcc only. The static ARM64 binary has no interpreter or dynamic section and runs an exact conversion/compare proof. Linux x86-64 passes the full suite under Docker emulation, and builds a static PIE after the same native-tool package removal. Its relocation dynamic section has no DT_NEEDED entries or PT_INTERP; it runs the exact smoke proof. These are target execution proofs, not native x86-64 timing results.

```sh
make pure-rust-build-check
# Explicit target prevents static CRT flags being applied to host proc macros:
RUSTFLAGS='-C link-arg=-fuse-ld=mold -C target-feature=+crt-static' scripts/check-pure-rust-build aarch64-unknown-linux-musl
```

The guard always uses a fresh repository-local target, denies native build-tool invocations and checks its invocation log. Rust build scripts still execute for configuration. Physical Linux package removal and ELF/Mach-O linkage checks provide evidence beyond crate names. macOS physical clean-host and native target timing remain separate proofs.

Allocation/copy counts remain unproven: xctrace Allocations failed to attach even with --no-prompt and saved an error trace. Rustybench's zero allocation counters are unsupported, not zero allocations. Retry the same scored command under an attach-capable profiling environment. The sample profile establishes limiting work, not allocation totals.

## Standalone workflows and build costs

[Workflow samples](benchmarks/workflows-macos-arm64.json) warm each binary once and interleave five samples per side for startup and the independently checked 256×192 file conversion. Current scored optimize and eight-file batch also receive five samples each. The production executables are immutable snapshots: Cargo can rebuild a CLI with development features while preparing benches, so measuring a mutable target/dist/img path misidentifies the artifact. Those earlier runs were discarded.

The final short-workflow ranges are broad (startup medians about 97–103 ms with 71–115 ms range widths; exact file medians 131–164 ms with 126–133 ms range widths). No runtime/startup ordering is claimed. Scored optimize is 152 ms median with 233 ms range width; batch is 120 ms with 123 ms range width. These are harness workflow observations under background system load, not stable production latency budgets. Retry on a stable quiet host.

[Build samples](benchmarks/build-cost-macos-arm64.json) contain 20 successful builds: five fresh-target and five cached-target runs per side, interleaved, with a warm shared kache compiler cache and CARGO_BUILD_JOBS=1. The clean-target median is 28.964 s / 0.917 s range width for the oracle and 24.096 s / 8.096 s for current; cached medians are 0.171 s / 0.036 s and 0.151 s / 0.798 s. Differences are within observed spread. This is not a cold compiler-cache claim; cargo child CPU excludes kache backend work. Built versions and hashes were checked after every timed build.

```sh
scripts/benchmark-workflows ORACLE_BINARY CURRENT_PRODUCTION_BINARY
scripts/benchmark-build-cost ORACLE_CHECKOUT
/usr/bin/time -l target/dist/img convert benchmarks/resource-rgb.png --lossless -o RESULT.png --quiet
```

## Permissive codec stack comparison (merged to master 2026-11-13)

This section records the comparison that supported merging the `experiment/permissive-codecs` branch (merged 2026-11-13), which swaps the production AGPL/GPL codec closure (zenjpeg, zenwebp, zenpng, zenavif, zenravif/zenrav1e, rav1d-safe, zenflate, zenquant and GPLv3 imagequant) for permissive pure-Rust implementations: zune-jpeg + mozjpeg-rs (JPEG), png 0.18 with quantette/quantizr (PNG), image-webp + vendored Webpkit (WebP) and rav1d/rav1e via ravif (AVIF). Both sides are `--profile dist --features build-binary` builds; the master baseline was built 2026-11-13 from `5424df8` (`7edc6ed` is docs-only). The protocol mirrors the production matrix: fast effort, eight-candidate cap, score targets 80/90/95, five interleaved rounds with alternating binary order, one worker and `RAYON_NUM_THREADS=1`, scored by the same PerceptualReference matrix. No record landed below its score target. Results are one corpus (14 fixtures, `benchmarks/corpus/generated` and photographs) on one shared host; they are comparison observations, not a release claim.

| Format | target 80 median ratio / range | target 90 median ratio / range | target 95 median ratio / range |
| --- | --- | --- | --- |
| WebP | −5.3% / [−37.3%, +80.8%] | −5.3% / [−37.3%, +80.8%] | −5.3% / [−37.3%, +80.8%] |
| AVIF | +15.5% / [−7.3%, +212.5%] | −1.9% / [−3.2%, −0.7%] | +18.2% / [−0.7%, +37.0%] |
| JPEG | +5.7% / [−42.2%, +67.2%] | +3.7% / [−15.4%, +96.0%] | +12.0% / [−7.1%, +26.8%] |

Completion sets: master completed 310/630 records and permissive completed 340/630; every cell master completed also completes on permissive (JPEG and WebP completion sets are identical). Permissive additionally completes six AVIF cells master refuses — coffee t80/t90 (116,027-byte outputs) and `ui.png`/`compressed.webp` at t90/t95 (1,314/1,436-byte outputs). The 290 both-refused records are identical cases: astronaut and icc (gamut), deep and ten-bit (precision), transparent/ui/grass JPEG (alpha), astronaut and most photographic AVIF cells at t90/t95, and coffee JPEG at t90/t95.

WebP is bimodal. Synthetic and flat content improves 5–37% (`line.png` −37.3%, `flat.png` −27.3%, `ui.png` −19.5%, `text.png` −5.3%), but photographic captures regress heavily: coffee +77.1% at t80, grass +61.2% at t80, gravel +48.3%, transparent +69.5% (both t80/t90/t95 bytes equal across targets), large +80.8%, coffee t90/t95 +7.7%. The lossy t80 regressions are materially closed by the 2026-11-14 iteration below; the lossless (`large.png`, `transparent.png`) and wall-time gaps persist. The vendored Webpkit snapshot at `codecs/webpkit/` is pre-1.0 and unmodified from its pinned import; closing this gap is the subject of the unattended parity campaign in [WEBP-HILLCLIMB](docs/WEBP-HILLCLIMB.md). AVIF's worst case is `large.png` t80 at +212.5% (39,202 vs 12,544 bytes); most photographic AVIF cells at t90/t95 still refuse on both stacks. JPEG is mixed (best case `line.png` t80 −42.2%; `grass.png` t80 +67.2%).

Timing medians (process startup included; host shared with background load; ranges dominated by the large AVIF encodes at ≈7–8 s; ordered as observations, not orderings): WebP ≈49.8–52.6 ms permissive vs ≈36.5–36.7 ms master, consistent across 70 samples per target; AVIF and JPEG within ~1 ms of master. The WebP delta is not a verified ordering under the busy-host protocol — re-measure on a quiet host before drawing conclusions. Binary size falls from 7,265,712 to 4,523,200 bytes (−37.8%). (The permissive timing portion of this line is superseded for the production binary by the 2026-11-14 iteration below; the shared-host corpus average remains the 2026-11-13 observation.)

Peak RSS (`/usr/bin/time -l`, startup included, three interleaved repetitions per side, `--score 80 --effort fast -j 1`, one worker, `RAYON_NUM_THREADS=1`; medians; macOS ARM64; shared host):

| Fixture | JPEG master → permissive | WebP master → permissive | AVIF master → permissive |
| --- | --- | --- | --- |
| resource-rgb.png (256×192) | 16.7 → 15.6 MB (−6.4%) | 16.3 → 16.8 MB (+3.0%) | 24.3 → 25.1 MB (+3.3%) |
| coffee.png | 61.1 → 51.2 MB (−16.2%) | 52.4 → 62.0 MB (+18.4%) | 58.9 → 60.1 MB (+2.0%) |
| large.png (2048×1024) | 400.5 → 391.6 MB (−2.2%) | 388.3 → 422.9 MB (+8.9%) | 422.1 → 418.1 MB (−1.0%) |

JPEG is modestly better, AVIF is within ±4 MB (≈±1–3%), and WebP is consistently higher (+0.5 / +9.6 / +34.6 MB across the fixture range), i.e. the webpkit encoder carries a working-memory cost that scales with image size. These were startup-inclusive process peaks on one shared host, not a Linux measurement; the 2026-11-14 quiet-host re-measurement (see below) records WebP at +3.6% (resource-rgb), +7.1% (coffee) and +8.6% (large.png) vs the same baseline binary.

The normal/build [license inventory](benchmarks/production-dependencies.json) was regenerated at `c088ac8`: 149 packages across `aarch64-apple-darwin`, `aarch64-unknown-linux-musl` and `x86_64-unknown-linux-musl`, zero declared AGPL/GPL/LGPL entries, zero build scripts. `quantizr` 1.4.3 declares no SPDX expression (recorded as `license-file: LICENSE`); its shipped license text is verbatim MIT and is recorded as the selected license. `webpkit` is a vendored path dependency pinned at `cc1311ebf`, not a registry artifact. The 137-package inventory earlier in this file records the pre-merge production closure.

This benchmark is the evidence for the 2026-11-13 merge to master. The WebP backend remains experimental and its photographic size regressions plus the AVIF color-description change ([docs/AVIF.md](docs/AVIF.md)) are recorded limitations; gating them is the job of the unattended parity campaign, and any claim of WebP parity requires its gates to pass.

## WebP parity iteration — 2026-11-14 (quiet host)

The 2026-11-13 WebP photographic gap was decomposed at matched score on one quiet macOS ARM64 host (`perf-gate-ready` 0 pre/post, `RAYON_NUM_THREADS=1`, one worker, `--score 80 --effort fast -j 1`): the lossless winner cells (`large.png`, `transparent.png`) reproduced byte-exactly across rebuilds, and the lossy cells traced two defects in the production path — the fast WebP lossy tier engaged none of the codec's rate-distortion machinery (fixed DC prediction, round-to-nearest quantization, no mode search, trellis, segments, filter or probability optimization), and the optimizer's single refinement candidate targeted the gap nearest the best passing quality instead of bisecting the pass/fail frontier, so steep quality-to-score curves overshot the frontier at the t80 gate. Both defects were fixed on master (`da744ab` lossy fast rides the size-adaptive search breadth, lossless fast stays shallow; `e6a0d02` refines the widest frontier-straddling gap; `40d6630` replaces the encoder's O(n²) Huffman forest scan with a min-heap, byte-identical). The fixes live in the `rimage` adapter, the optimizer and the vendored codec; the pinned snapshot's only prior local change remains `11a321d`. The t80 winners now sit closer to the gate with thinner margins (coffee 80.6, grass 83.1, gravel 80.7 — verified passing), replacing the 2026-11-13 winners at 86.7 / 83.6 / 83.6.

| Fixture | baseline `5424df8` bytes / wall / RSS | merge-era `4fc40dc` bytes / wall / RSS | master `1424358` bytes / wall / RSS |
| --- | --- | --- | --- |
| resource-rgb.png | 4,616 / 80 ms / 16.9 MB | 4,680 / 156 ms / 17.6 MB | 4,680 / 227 ms / 17.7 MB |
| coffee.png | 93,198 / 303 ms / 54.9 MB | 165,090 / 1482 ms / 58.9 MB | **62,224** / 1002 ms / 58.4 MB |
| grass.png | 81,068 / 386 ms / 61.9 MB | 130,658 / 1078 ms / 56.7 MB | **84,652** / 1423 ms / 55.5 MB |
| gravel.png | 65,226 / 377 ms / 59.6 MB | 96,700 / 1163 ms / 57.1 MB | **49,966** / 1503 ms / 58.4 MB |
| large.png | 7,014 / 1924 ms / 408.3 MB | 12,682 / 7452 ms / 441.4 MB | 12,682 / 8517 ms / 439.8 MB |
| transparent.png | 118 / 26.6 ms / 11.2 MB | 200 / 89.7 ms / 10.6 MB | 196 / 85 ms / 10.8 MB |

Sizes at t80 are now equal to or better than the master baseline on coffee (−33.2%) and gravel (−23.4%), within +4.4% on grass; lossless paths are unchanged (`large.png` +80.8%, `transparent.png` +66.1% vs baseline — the VP8L encoder's smooth-gradient gap, which all search breadths land on, worst case 11,712 B at `best` effort). The lossy candidates now evaluate the full RD search, so per-candidate encode time rises ≈25–45%. Iteration 2 (`40d6630`, `perf(webpkit)`) replaced the encoder's quadratic Huffman forest scan with a min-heap over the same `(count, pool index)` key — byte-identical output (corpus-wide, 13/13 encodable fixtures, plus the 911-case lib suite) — recovering most of the VP8L wall: coffee's mandatory lossless candidate drops 1.28 s → 0.48 s and the t80 coffee workflow drops 1806 → 1002 ms; the whole permissive stack remains 3.3–4.4× the withdrawn closure's encode wall on the measured photographic inputs (coffee 1002 vs 304 ms; large.png ~4.4×). Binary size falls from 7,265,712 bytes (`5424df8`) to 4,440,528 bytes (−38.9%).

Matched-score decomposition (direct `--quality` sweeps, scored by the same PerceptualReference): at score ≈ 85.6, coffee costs 165,090 / 114,840 / 93,198 bytes under the former level(1) tier, the size-adaptive tier, and the withdrawn default; grass at score ≈ 83 is 99,506 bytes under the size-adaptive tier vs 130,658 under level(1). Even the deepest VP8L search lands at 11,712 B for `large.png` (vs 12,682 at the production level and 7,014 for the withdrawn default) — a codec capability gap, not an effort-selection gap; the AVIF comparison and its records are untouched by this iteration.

Matched-score WebP re-score vs `cwebp` 1.6.0 (method 4, single run, `RIMAGE_BENCH_TOOLS=rimage,cwebp`), committed as [untimed-2026-11-14.json](benchmarks/results/untimed-2026-11-14.json) with outputs under `2026-11-14-*.webp` — the retained 2026-11-13 outputs are untouched. Scores by the same PerceptualReference; cwebp passes the same gate independently.

| Fixture | Target | rimage bytes @ score | cwebp bytes @ score | rimage Δ vs cwebp |
| --- | --- | --- | --- | --- |
| coffee.png | 80 | 62,224 @ 80.6 | 87,502 @ 85.8 | −28.9% |
| grass.png | 80 | 84,652 @ 83.1 | 82,802 @ 84.2 | +2.2% |
| gravel.png | 80 | 49,966 @ 80.7 | 59,728 @ 83.5 | −16.3% |
| large.png | 80 | 12,682 @ 100.0 (lossless) | 42,032 @ 84.1 | −69.8% |
| transparent.png | 80 | 196 @ 81.3 | 168 @ 81.3 | +16.7% |
| flat.png | 80 | 32 @ 100.0 (lossless) | 82 @ 90.2 | −61.0% |
| gray.png | 80 | 56 @ 100.0 (lossless) | 88 @ 80.3 | −36.4% |
| line.png | 80 | 64 @ 100.0 (lossless) | 1,308 @ 88.2 | −95.1% |
| text.png | 80 | 72 @ 100.0 (lossless) | 490 @ 81.2 | −85.3% |
| ui.png | 80 | 66 @ 100.0 (lossless) | refused | — |
| compressed.webp | 80 | 66 @ 100.0 (lossless) | refused | — |
| grass.png | 90 | 155,570 @ 90.9 | 146,582 @ 94.4 | +6.1% |
| gravel.png | 90 | 130,144 @ 92.7 | 124,510 @ 94.2 | +4.5% |
| coffee.png | 90 | 386,802 @ 100.0 (lossless fallback) | refused | — |
| transparent.png | 90 | 200 @ 100.0 (lossless) | 254 @ 92.0 | −21.3% |
| large.png | 90 | 12,682 @ 100.0 (lossless) | refused | — |
| grass.png | 95 | 212,590 @ 100.0 (lossless fallback) | 176,330 @ 95.7 | +20.6% |
| gravel.png | 95 | 188,642 @ 100.0 (lossless fallback) | 152,426 @ 95.6 | +23.7% |
| coffee.png | 95 | 386,802 @ 100.0 (lossless fallback) | refused | — |
| transparent.png | 95 | 200 @ 100.0 (lossless) | 770 @ 95.2 | −74.0% |
| large.png | 95 | 12,682 @ 100.0 (lossless) | refused | — |

Where both stacks pass at the target gate, rimage wins coffee, gravel, large and the synthetic lossless cells, and loses grass (+2.2%) and transparent (+16.7%) at t80. At t90/t95 rimage completes photographic cells only via lossless fallback (score 100) and lands 4.5–23.7% above cwebp's lossy outputs at 92–96; rimage's fast candidate ladder never produced a passing lossy candidate above t80 on those inputs. `cwebp` refuses coffee/large at t90/t95 (no passing quality on its grid) and every `ui.png`/`compressed.webp` cell; `astronaut.png`/`icc.png` refuse on both stacks (ICC transform leaves the SDR sRGB gamut) and `deep.png` is excluded (high depth). This is one corpus, one host, single-run capture — not a release claim, and it does not re-score the 2026-11-13 cells.

Fresh timed WebP cells (committed as [timed-2026-11-14.json](benchmarks/results/timed-2026-11-14.json) with prefixed outputs `2026-11-14-timed-*.webp`; quiet host, `perf-gate-ready` 0 pre/post, `RAYON_NUM_THREADS=1`, five interleaved repetitions, 420 records / 280 completed, revision `9542c98`, `timingGate: accepted`). Encode medians in ms, startup-inclusive process walls measured exactly as in the retained 2026-11-13 timed records (source decode + convert + candidate decode + harness scoring included for both tools): coffee t80 1029 vs 288 (≈3.6×; cwebp refuses t90/t95); grass t80/t90/t95 1461/1482/1445 vs 372/371/372 (≈3.9×); gravel 1540/1558/1537 vs 343/343/342 (≈4.5×); large.png t80 8640 vs 1506 (≈5.7×); flat/gray ≈27–28 vs 24 (≈1.1×), line/text ≈46–47/43 vs 28/27 (≈1.6×), transparent ≈81–82 vs 32 (≈2.6×); `ui.png`/`compressed.webp` rimage-only ≈34–38. Encoding this ladder costs what the retained records showed: the wall verdict is unchanged at ≈3.6–5.7× the `cwebp` process wall on photographic inputs, consistent with the 3.3–4.4× workflow comparison against the withdrawn closure (coffee 1002 ms there). One corpus, one host, fast effort; not a release claim. `cargo test --locked --all-features --test benchmark_verify` passes with the fresh records.

Post-deep-DP timed refresh (committed as [timed-2026-11-15.json](benchmarks/results/timed-2026-11-15.json) with prefixed `2026-11-15-timed-*.webp` outputs; identical protocol, quiet host, `perf-gate-ready` 0 pre/post, 5 interleaved repetitions, tools `img,cwebp`, revision `f043b4f`, `sourceDirty: false`, `timingGate: accepted`, 420 records / 280 completed; refusal set identical to 2026-11-14). Encode medians in ms vs the same measurement one iteration earlier ([timed-2026-11-14](benchmarks/results/timed-2026-11-14.json), pre-DP binary): coffee t80 1032 (flat), grass 1501/1525/1484 (+3%), gravel 1717/1743/1718 (**+11.8%**), large.png **40,385/40,401/40,379 ms vs 8640** (≈4.7× — the documented cost of closing the large.png byte gap to 4,846 B), line 132/133/134 (≈+185%), text 127/127/128 (≈+190%), transparent 90/91/91 (+11%), ui 53/49/50 (+35–45%; +2 B output too — a documented net negative on this cell), compressed 51/50/49 (≈+33%; +2 B), flat/gray ≈28 (flat). The synthetic cells that regressed carry the deep-DP's wall cost on saturated chains; photographic cells are flat-to-+12%. `ui.png`/`compressed.webp` +2 B and the line/text/gravel wall costs are accepted as part of the documented `MAX_CHAIN_OPTIMAL=256` Pareto trade (offsetting a −62% byte result on large.png); if the wall matters more for those cells, the cap can be tier-gated by level or image size — a user decision, not changed silently. `cargo test --locked --all-features --test benchmark_verify` passes against all retained and fresh records.

These are one corpus (14 fixtures), one host, fast effort, startup-inclusive process walls and medians of five interleaved repetitions; they are not a release claim. The lossy size regressions at the t80 gate are materially closed; the backend remains experimental pending the best-effort and Linux measurements and a full matrix re-score.

## WebP parity iteration — 2026-11-15 capture (made 2026-11-14, quiet host)

> The lossy cells of this table are superseded by the [2026-11-15-lambda capture](#webp-matched-score-capture-after-cap-9-the-dp-tier-gate-and-the-q²-trellis-lambda-2026-11-15-quiet-host) below (cap 9, DP tier-gate, `q_ac²` trellis lambda); the lossless and cwebp cells are unchanged.

The largest remaining lossless gap closed: the optimal-parse DP's monotonically improving hash-chain staircase walk now probes up to 256 hops instead of 64 (`MAX_CHAIN_OPTIMAL`, `5a0c65e`), while the greedy seed keeps the 64-hop cap so floor streams and ordinary-photo walls are unchanged. `large.png` at level 1 goes 12,682 → **4,846 B** (−61.8%), beating the withdrawn closure's 7,014 by −30.9%; `text.png` 72 → 58, `line.png` 64 → 62, `transparent.png` 196 → 194, `astronaut.png` −142, coffee −14; `ui.png`/`compressed.webp` +2 B (the DP candidate is self-floored against greedy, never against the previous DP). Photographic chains exhaust long before the cap, so the lossy wall is unchanged (coffee ≈460 ms, grass 0.82 s, gravel 1.08 s at level 1); the 2048×1024 smooth gradient is the worst case and rises ~4.5× on the lossless path (≈25 s), the case this landing targets. `ui.png`/`compressed.webp` parity note: both remain lossless-exact and two bytes above the 2026-11-14 capture; named and bounded.

A fresh matched-score single-run capture is committed as [untimed-2026-11-15.json](benchmarks/results/untimed-2026-11-15.json) with prefixed `2026-11-15-*.webp` outputs (tools `img,cwebp`; 84 records, 56 completed; refusal set byte-identical to 2026-11-14; cwebp cells byte-identical). Tool label is `img` post-rename. Scores by the same PerceptualReference; cwebp passes the same gate independently.

| Fixture | Target | img bytes @ score | cwebp bytes @ score | img Δ vs cwebp |
| --- | --- | --- | --- | --- |
| coffee.png | 80 | 62,224 @ 80.6 | 87,502 @ 85.8 | −28.9% |
| grass.png | 80 | 84,652 @ 83.1 | 82,802 @ 84.2 | +2.2% |
| gravel.png | 80 | 49,966 @ 80.7 | 59,728 @ 83.5 | −16.3% |
| large.png | 80 | 4,846 @ 100.0 (lossless) | 42,032 @ 84.1 | −88.5% |
| transparent.png | 80 | 194 @ 100.0 (lossless) | 168 @ 81.3 | +15.5% |
| flat.png | 80 | 32 @ 100.0 (lossless) | 82 @ 90.2 | −61.0% |
| gray.png | 80 | 56 @ 100.0 (lossless) | 88 @ 80.3 | −36.4% |
| line.png | 80 | 62 @ 100.0 (lossless) | 1,308 @ 88.2 | −95.3% |
| text.png | 80 | 58 @ 100.0 (lossless) | 490 @ 81.2 | −88.2% |
| ui.png | 80 | 68 @ 100.0 (lossless) | refused | — |
| compressed.webp | 80 | 68 @ 100.0 (lossless) | refused | — |
| grass.png | 90 | 155,570 @ 90.9 | 146,582 @ 94.4 | +6.1% |
| gravel.png | 90 | 130,144 @ 92.7 | 124,510 @ 94.2 | +4.5% |
| coffee.png | 90 | 386,788 @ 100.0 (lossless fallback) | refused | — |
| transparent.png | 90 | 194 @ 100.0 (lossless) | 254 @ 92.0 | −23.6% |
| large.png | 90 | 4,846 @ 100.0 (lossless) | refused | — |
| grass.png | 95 | 212,590 @ 100.0 (lossless fallback) | 176,330 @ 95.7 | +20.6% |
| gravel.png | 95 | 188,642 @ 100.0 (lossless fallback) | 152,426 @ 95.6 | +23.7% |
| coffee.png | 95 | 386,788 @ 100.0 (lossless fallback) | refused | — |
| transparent.png | 95 | 194 @ 100.0 (lossless) | 770 @ 95.2 | −74.8% |
| large.png | 95 | 4,846 @ 100.0 (lossless) | refused | — |

vs the withdrawn baseline `5424df8` (t80): coffee 62,224 vs 93,198, gravel 49,966 vs 65,226, large.png 4,846 vs 7,014 (all better); grass 84,652 vs 81,068 (+4.4%), transparent 194 vs 118 (+64%). The lossy t80 winners are unchanged (coffee stays 62,224 @ 80.6; perf gates `perf-gate-ready` 0 pre/post; `cargo test --locked --all-features --test benchmark_verify` passes against the fresh records; the retained 2026-11-13 and 2026-11-14 records are untouched).

Decode-side competitiveness vs `dwebp` 1.6.0 (one interleaved warm-host comparison; process walls and `instructions retired`, both sides incl. startup; decode to a sink, no I/O of the decoded pixels):

| Fixture | kind | img user / instr / footprint | dwebp user / instr / footprint |
| --- | --- | --- | --- |
| large_png (2048×1024) | lossless VP8L | 0.02 s / 197,577,345 / 16.50 MB | 0.05 s / 783,185,285 / 16.52 MB |
| grass_png (512×512) | lossy VP8 | 0.00 s / 94,871,104 / 3.34 MB | 0.05 s / 379,771,270 / 2.49 MB |
| coffee_png (600×400) | lossy VP8 | 0.00 s / 71,447,183 / 3.18 MB | 0.05 s / 332,261,753 / 2.39 MB |
| transparent_png (96×64) | lossy VP8 | 0.00 s / 28,364,623 / 1.77 MB | 0.00 s / 27,952,588 / 1.56 MB |
| flat_png (96×64) | lossless VP8L | 0.00 s / 28,016,982 / 1.80 MB | 0.00 s / 26,746,227 / 1.54 MB |

img decodes 2.8–4.0× fewer instructions on real files with equal footprint on large.png and +0.2–0.9 MB on small ones (smaller-file cells are startup-tied). dwebp writes a file sink and img decodes to memory then drops, so this is directional evidence that img's decoder is competitive, not a release claim.

Lossy t90/t95 cells historically fell back to lossless on grass/gravel. The trellis lambda was then relaxed to `q_ac^2` (`d6b22c2`, see the 2026-11-15-lambda section below), which flips gravel t95 to lossy; SNS remains refuted as the cause (the ceiling is encoder RD efficiency, not coarsening). `cargo test --locked --all-features --test benchmark_verify` passes.

## WebP matched-score baseline vs the withdrawn zenwebp stack (harness-direct, 2026-11-14)

The withdrawn matched-score baselines beyond t80 had never been captured. This session rebuilt the pre-merge master (`5424df8`, zenwebp `=0.4.6`) in `.work/wt/withdrawn` (built via kache, 26 s; its t80 cells reproduce the recorded `5424df8` column byte-for-byte — coffee 93,198 @ 85.62, grass 81,068 @ 82.42, gravel 65,226 @ 83.34, large.png 7,014 @ 100, transparent 118 @ 100) and ran `convert --score T --effort fast --quiet` per fixture per target, scored with the same PerceptualReference. Outputs are in `.work/tmp/withdrawn-cells/` (scratch, not committed; reproduce with the worktree). t80 matched-score cells match the 2026-11-13 iteration table exactly; t90/t95 are new evidence:

| Fixture | Withdrawn t80 @ score | t90 @ score | t95 @ score | img t80 | img t90 | img t95 |
| --- | --- | --- | --- | --- | --- | --- |
| coffee.png | 93,198 @ 85.6 | 359,054 @ 100 (lossless fb) | 359,054 @ 100 (lossless fb) | 62,224 @ 80.6 (−33.2%) | 386,788 @ 100 (+7.7%) | 386,788 @ 100 (+7.7%) |
| grass.png | 81,068 @ 82.4 | 143,080 @ 93.75 | 177,586 @ 95.53 | 84,652 @ 83.1 (+4.4%) | 155,570 @ 90.9 (+8.7%) | 212,590 @ 100 fb (+19.7%) |
| gravel.png | 65,226 @ 83.3 | 127,486 @ 93.83 | 157,330 @ 95.41 | 49,966 @ 80.7 (−23.4%) | 130,144 @ 92.7 (+2.1%) | 188,642 @ 100 fb (+19.9%) |
| large.png | 7,014 @ 100 | 7,014 @ 100 | 7,014 @ 100 | 4,846 @ 100 (−30.9%) | 4,846 @ 100 (−30.9%) | 4,846 @ 100 (−30.9%) |
| transparent.png | 118 @ 100 | 118 @ 100 | 118 @ 100 | 194 @ 100 (+64.4%) | 194 @ 100 (+64.4%) | 194 @ 100 (+64.4%) |
| flat.png | 44 @ 100 | 44 @ 100 | 44 @ 100 | 32 (−27.3%) | 32 (−27.3%) | 32 (−27.3%) |
| gray.png | 62 @ 100 | 62 @ 100 | 62 @ 100 | 56 (−9.7%) | 56 (−9.7%) | 56 (−9.7%) |
| line.png | 102 @ 100 | 102 @ 100 | 102 @ 100 | 62 (−39.2%) | 62 (−39.2%) | 62 (−39.2%) |
| text.png | 76 @ 100 | 76 @ 100 | 76 @ 100 | 58 (−23.7%) | 58 (−23.7%) | 58 (−23.7%) |
| ui.png | 82 @ 100 | 82 @ 100 | 82 @ 100 | 68 (−17.1%) | 68 (−17.1%) | 68 (−17.1%) |
| compressed.webp | 82 @ 100 | 82 @ 100 | 82 @ 100 | 68 (−17.1%) | 68 (−17.1%) | 68 (−17.1%) |
| astronaut.png | refused (exit 3) | refused | refused | refused | refused | refused |
| icc.png | refused (exit 1) | refused | refused | refused | refused | refused |
| deep.png | refused (exit 1) | refused | refused | refused | refused | refused |

Verdict: the withdrawn stack also falls back to lossless on coffee t90/t95 (score 100), and its lossless output there is smaller than ours (359,054 vs 386,788, −7.2% — the same VP8L entropy-stage gap that made our old large.png 12,682 vs their 7,014; ours now beats them only on large.png-class repetitive content). The destination gate vs the withdrawn stack: t80 wins 9 of 11 completing cells (losses grass +4.4%, transparent +64%); t90/t95 complete 11 cells each and lose coffee (+7.7% lossless-to-lossless), grass (+8.7% / +19.7% fallback), gravel (+2.1% / +19.9% fallback), transparent (+64%). Decode is competitive; encode wall remains multiple times the withdrawn stack. **The destination gate is NOT met and is not claimed.**

## Full-matrix re-score vs the original toolchain (2026-11-15 prefix, quiet host)

Committed as [untimed-full-2026-11-15.json](benchmarks/results/untimed-full-2026-11-15.json) with prefixed `2026-11-15-full-*` outputs: flags `IMG_BENCH_TOOLS` unset (all six tools), `IMG_ORIGINAL_BIN=.work/wt/original/target/release/rimage`. 196 records, 100 completed; `perf-gate-ready` 0 pre/post not required (capture mode records no timings); revision `f043b4f`, `sourceDirty: false`. Refusal set identical to the retained 2026-11-13 records (astronaut/icc/deep both stacks; img-png astronaut/coffee t80; cwebp coffee/large t90/t95 and ui/compressed; original-ravif completes zero cells, ten-bit refusal). img-WebP beats original-mozjpeg on **17 of 18** matched-score cells where both pass (mozjpeg takes gravel t90: 110,756 @ 90.7 vs 130,144 @ 92.7) — a WebP-vs-JPEGCLI-level comparison, not a like-for-like container claim. The fresh capture re-covers cwebp byte-identically. `cargo test --locked --all-features --test benchmark_verify` re-verifies every retained and fresh record (sizes, scores, exact samples).

## WebP matched-score capture after cap 9, the DP tier-gate and the q² trellis lambda (2026-11-15, quiet host)

Committed as [untimed-lambda-2026-11-15.json](benchmarks/results/untimed-lambda-2026-11-15.json) with prefixed `2026-11-15-lambda-*.webp` outputs (tools `img,cwebp`, 84 records, 56 completed; `perf-gate-ready` 0 pre/post; revision `73da8c2`, `sourceDirty: false`; refusal set byte-identical; cwebp cells deterministic and byte-identical to the 2026-11-15 capture). Scores by the same PerceptualReference. This capture supersedes the lossy cells of the 2026-11-15 table. The changes landed in sequence: **`d89b390`** tier-gates the optimal-parse DP staircase walk (level ≥ 3 or ≥ 1 MP), restoring the small-cell walls (ui/compressed 66, line 64, text 72; large.png stays 4,846 @ 100 at all targets); **`4f00b19`** raises Fast to cap 9 (no measured byte effect on the named cells — the binding constraint was the lossy score curve); **`d6b22c2`** relaxes the trellis rate weight to `q_ac^2` (the committed `(7·q_ac^2)>>2` was measured too aggressive on the gates; `>>3` regressed gravel t80 and was rejected earlier). `benchmark_verify` passes.

| Fixture | Target | img bytes @ score | cwebp bytes @ score | img Δ vs cwebp | img Δ vs 2026-11-15 |
| --- | --- | --- | --- | --- | --- |
| coffee.png | 80 | 67,296 @ 82.0 | 87,502 @ 85.8 | −23.1% | +8.2% (regression, named; still beats both oracles) |
| grass.png | 80 | 76,818 @ 81.6 | 82,802 @ 84.2 | −7.2% | −9.2% (beats cwebp) |
| gravel.png | 80 | 50,566 @ 80.8 | 59,728 @ 83.5 | −15.4% | +1.2% |
| large.png | 80 | 4,846 @ 100.0 (lossless) | 42,032 @ 84.1 | −88.5% | 0 |
| transparent.png | 80 | 174 @ 81.0 | 168 @ 81.3 | +3.6% | −11.2% (still 64% above the withdrawn 118) |
| line.png | 80 | 64 @ 100.0 (lossless) | 1,308 @ 88.2 | −95.1% | +3.2% |
| text.png | 80 | 72 @ 100.0 (lossless) | 490 @ 81.2 | −85.3% | +24.1% |
| ui.png | 80 | 66 @ 100.0 (lossless) | refused | — | 0 |
| compressed.webp | 80 | 66 @ 100.0 (lossless) | refused | — | 0 |
| grass.png | 90 | 128,872 @ 90.0 | 146,582 @ 94.4 | −12.1% | −11.7% (beats cwebp) |
| gravel.png | 90 | 98,420 @ 90.6 | 124,510 @ 94.2 | −20.9% | −24.4% (beats cwebp) |
| transparent.png | 90 | 200 @ 100.0 (lossless) | 254 @ 92.0 | −21.3% | 0 |
| large.png | 90 | 4,846 @ 100.0 (lossless) | refused | — | 0 |
| coffee.png | 95 | 386,802 @ 100.0 (lossless fallback) | refused | — | 0 |
| grass.png | 95 | 212,590 @ 100.0 (lossless fallback) | 176,330 @ 95.7 | +20.6% | 0 |
| gravel.png | 95 | **176,698 @ 95.04 (lossy)** | 152,426 @ 95.6 | +15.9% | flips from the 188,642 lossless fallback (−6.3%) |
| transparent.png | 95 | 200 @ 100.0 (lossless) | 770 @ 95.2 | −74.0% | 0 |
| large.png | 95 | 4,846 @ 100.0 (lossless) | refused | — | 0 |

vs the withdrawn closure (`5424df8`) after this landing: grass t80 76,818 < 81,068; grass t90 128,872 < 143,080; gravel t90 98,420 < 127,486 — all three flip from losses to wins. Remaining losses vs the withdrawn stack: coffee t90/t95 and grass t95 (lossless-to-lossless, the VP8L entropy-stage gap: 386,802 vs 359,054, +7.7%; grass fallback 212,590 vs 177,586 @ 95.53, +19.7%), gravel t95 lossy-to-lossy (176,698 @ 95.04 vs 157,330 @ 95.41, +12.3%), transparent t80/t90/t95 (174/200/200 vs 118, +47%/+64%/+64%). Encode wall is unchanged (3.6–5.7× cwebp process walls on photographic cells; 3.3–4.4× the withdrawn closure; a vectorized-kernel program). Decode stays competitive. **The destination gate is NOT met and is not claimed.**

The intervening capture `untimed-2026-11-15-r2.json` (cap 9 + tier-gate, pre-lambda) was measured, confirmed refusal-identical and discarded as byte-redundant with this one; the retained 2026-11-13/14 records and the 2026-11-15 records are untouched.

## Remaining proofs

These measurements need the stated environments. Existing results apply to their recorded source and supported fixtures; they do not establish correctness for every accepted image representation.

Optional implementation and dependency changes need a reproducible benefit beyond observed spread. Reject regressions above 10% in median runtime, build time, release size or peak RSS unless a required correctness capability justifies the cost. Record those costs explicitly. Compare equivalent completed workflows; a multi-candidate optimizer is not equivalent to one legacy encode.

| Proof | Prerequisite and retry |
| --- | --- |
| Native Linux timing | Dedicated stable x86-64 and ARM64 hosts. Run `make benchmark-run` and `make benchmark-verify`; require five interleaved completed cases, passing load gates, and medians separated from spread before claiming an ordering. Emulated functional execution is not native timing. |
| Stable startup, file and batch comparisons | Resolved 2026-11-14 on a quiet host with immutable binaries: `scripts/benchmark-workflows` with the retained `2708512` oracle and current master `1424358` (30 samples; pre/post `perf-gate-ready` passed). Results in [workflows-macos-arm64.json](benchmarks/workflows-macos-arm64.json): `startup_version` 3.75 ms vs oracle 3.13 ms (+20%, spreads 58/167 µs), `png_file_conversion` 20.2 ms vs 6.95 ms (+190%, spreads 1.0/0.1 ms), `optimize_scored_file` 24.3 ms, `lossless_batch` 87.7 ms (spread 3.3 ms). The PNG file-conversion gap is the pure-Rust PNG stack's cost vs the pre-rewrite closure; it is an honest tradeoff of the migration, not a regression introduced by these iterations. |
| Physical clean macOS build | macOS ARM64 without third-party native build tools. Run `make pure-rust-build-check`, inspect its empty native-tool log and `otool -L`; require OS-only linkage. |
| JPEGli and external eight-bit AVIF comparisons | Install those tools and validate command adapters before extending `examples/benchmark.rs`. `make benchmark-run` does not discover new tools automatically. Rerun verification and retain failures alongside actual decoded scores and dimensions. |
| Allocation and copy totals | An environment where Instruments can attach. Run `xcrun xctrace record --template Allocations --no-prompt --time-limit 30s --output .work/tmp/allocations.trace --launch -- target/dist/img convert benchmarks/corpus/generated/large.png --score 95 --effort fast -o .work/tmp/profile.webp --quiet` with fresh output names. Require an attached trace and attributed counts for completed work; unsupported zero counters are not measurements. |

AVIF alpha, monochrome, high depth/HDR, grids and unsupported display transforms are product refusals, not deferred environment proofs. Their retry conditions belong in [AVIF support](docs/AVIF.md). Open implementation defects belong in [TODO.md](TODO.md).
