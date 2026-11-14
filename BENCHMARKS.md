# Benchmark evidence

Performance captures use production source `0e4a4a2`; corrected comparison commands and verification are in `6535c42`. Final JPEG safety repairs are at `7f1b45b`; final artifact/resource proofs were refreshed, while timing archives retain their original source and binary identities. Results are observations of specified configurations, not a general codec competitiveness claim. All accepted timings passed perf-gate-ready before and after their run. Initial busy-host timings were discarded.

## Correctness and corpus

`make benchmark-verify` verifies four pinned licensed photographs and ten CC0 procedural representations, actual decoded output samples/scores, dimensions, source/output bytes, saved schema, completion counts, tool commands and gate arithmetic. [Corpus provenance](benchmarks/corpus/README.md) pins attribution, licenses and hashes. Cases cover photos, texture, UI/text/lines, transparency, grayscale, ICC, 16-bit, existing WebP and a 2048×1024 gradient.

[Untimed capture](benchmarks/results/untimed.json) has 196 records, 100 completed and 96 explicit failures/refusals. [Five-run capture](benchmarks/results/timed.json) has 980 records and 500 completed. Every completed output is retained and reverified. Summaries group by tool, fixture and target; different completion sets are never pooled into a cross-tool win. Failed work contributes no successful timing or size result.

The astronaut's nominal sRGB profile leaves the supported gamut under both independent LittleCMS and Rimage; scored processing refuses it. Required precision/color facts and unsafe PNG ancillary metadata also cause named refusals. The original ravif 0.13.0 command emits ten-bit AVIF, outside the independently verified eight-bit scoring matrix. These are recorded failures, not zero-cost successes. Synthetic cases do not establish photographic competitiveness.

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
| new rimage | fast effort, eight total candidates, exact WebP eligible before lossy search |
| new exact PNG | fast effort, lossless |
| original MozJPEG | mozjpeg 0.10.13 / mozjpeg-sys 2.2.3; native quality grid 1,25,50,75,95,100; progressive, NRobidoux tables |
| original OxiPNG | oxipng 10.2.1, native preset 2 |
| original ravif | ravif 0.13.0, same native quality grid, speed 6; ten-bit outputs refused |
| cwebp | 1.6.0, lossy grid 0,25,50,75,95,100, method 4, exact hidden RGB and alpha quality 100; no -mt |

These grids and effort settings are not equally tuned implementations. In particular cwebp's grid does not try lossless. JPEGli and an externally configurable eight-bit AVIF encoder were unavailable. The generic historical jpeg/png commands select zune encoders; earlier results mislabeled as MozJPEG/OxiPNG were discarded and replaced. The verifier now rejects that label/command mismatch.

The corrected data exposes tradeoffs. For procedural UI, exact new PNG is 142 bytes versus OxiPNG's 120; for 16-bit gradient, 520 versus 147. Both preserve the checked samples. This is not a reason to retain native production code. On coffee at minimum score 80, the evaluated WebP outputs are 93,198 bytes at score 85.62 for rimage and 87,502 at 85.82 for cwebp. This one close-score case favors cwebp bytes; it does not establish corpus-wide ordering. Read raw scores and failures before comparing configurations.

## Original and retained baselines

The retained-behavior oracle is `2708512`; the original comparison source is `66fc0fc`. Apply only [original-build.patch](docs/original-build.patch) in a disposable checkout: remove its undeclared Windows build script and mechanically prune those stale lock entries.

```sh
git worktree add .work/wt/original 66fc0fc
git -C .work/wt/original apply ../../../docs/original-build.patch
cd .work/wt/original
TMPDIR=/absolute/path/to/rimage/.work/tmp RUSTC_WRAPPER=kache cargo build --locked --release --features build-binary
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

[Workflow samples](benchmarks/workflows-macos-arm64.json) warm each binary once and interleave five samples per side for startup and the independently checked 256×192 file conversion. Current scored optimize and eight-file batch also receive five samples each. The production executables are immutable snapshots: Cargo can rebuild a CLI with development features while preparing benches, so measuring a mutable target/dist/rimage path misidentifies the artifact. Those earlier runs were discarded.

The final short-workflow ranges are broad (startup medians about 97–103 ms with 71–115 ms range widths; exact file medians 131–164 ms with 126–133 ms range widths). No runtime/startup ordering is claimed. Scored optimize is 152 ms median with 233 ms range width; batch is 120 ms with 123 ms range width. These are harness workflow observations under background system load, not stable production latency budgets. Retry on a stable quiet host.

[Build samples](benchmarks/build-cost-macos-arm64.json) contain 20 successful builds: five fresh-target and five cached-target runs per side, interleaved, with a warm shared kache compiler cache and CARGO_BUILD_JOBS=1. The clean-target median is 28.964 s / 0.917 s range width for the oracle and 24.096 s / 8.096 s for current; cached medians are 0.171 s / 0.036 s and 0.151 s / 0.798 s. Differences are within observed spread. This is not a cold compiler-cache claim; cargo child CPU excludes kache backend work. Built versions and hashes were checked after every timed build.

```sh
scripts/benchmark-workflows ORACLE_BINARY CURRENT_PRODUCTION_BINARY
scripts/benchmark-build-cost ORACLE_CHECKOUT
/usr/bin/time -l target/dist/rimage convert benchmarks/resource-rgb.png --lossless -o RESULT.png --quiet
```

## Permissive codec stack comparison (merged to master 2026-11-13)

This section records the comparison that supported merging the `experiment/permissive-codecs` branch (merged 2026-11-13), which swaps the production AGPL/GPL codec closure (zenjpeg, zenwebp, zenpng, zenavif, zenravif/zenrav1e, rav1d-safe, zenflate, zenquant and GPLv3 imagequant) for permissive pure-Rust implementations: zune-jpeg + mozjpeg-rs (JPEG), png 0.18 with quantette/quantizr (PNG), image-webp + vendored Webpkit (WebP) and rav1d/rav1e via ravif (AVIF). Both sides are `--profile dist --features build-binary` builds; the master baseline was built 2026-11-13 from `5424df8` (`7edc6ed` is docs-only). The protocol mirrors the production matrix: fast effort, eight-candidate cap, score targets 80/90/95, five interleaved rounds with alternating binary order, one worker and `RAYON_NUM_THREADS=1`, scored by the same PerceptualReference matrix. No record landed below its score target. Results are one corpus (14 fixtures, `benchmarks/corpus/generated` and photographs) on one shared host; they are comparison observations, not a release claim.

| Format | target 80 median ratio / range | target 90 median ratio / range | target 95 median ratio / range |
| --- | --- | --- | --- |
| WebP | −5.3% / [−37.3%, +80.8%] | −5.3% / [−37.3%, +80.8%] | −5.3% / [−37.3%, +80.8%] |
| AVIF | +15.5% / [−7.3%, +212.5%] | −1.9% / [−3.2%, −0.7%] | +18.2% / [−0.7%, +37.0%] |
| JPEG | +5.7% / [−42.2%, +67.2%] | +3.7% / [−15.4%, +96.0%] | +12.0% / [−7.1%, +26.8%] |

Completion sets: master completed 310/630 records and permissive completed 340/630; every cell master completed also completes on permissive (JPEG and WebP completion sets are identical). Permissive additionally completes six AVIF cells master refuses — coffee t80/t90 (116,027-byte outputs) and `ui.png`/`compressed.webp` at t90/t95 (1,314/1,436-byte outputs). The 290 both-refused records are identical cases: astronaut and icc (gamut), deep and ten-bit (precision), transparent/ui/grass JPEG (alpha), astronaut and most photographic AVIF cells at t90/t95, and coffee JPEG at t90/t95.

WebP is bimodal. Synthetic and flat content improves 5–37% (`line.png` −37.3%, `flat.png` −27.3%, `ui.png` −19.5%, `text.png` −5.3%), but photographic captures regress heavily: coffee +77.1% at t80, grass +61.2% at t80, gravel +48.3%, transparent +69.5% (both t80/t90/t95 bytes equal across targets), large +80.8%, coffee t90/t95 +7.7%. The vendored Webpkit snapshot at `codecs/webpkit/` is pre-1.0 and unmodified from its pinned import; closing this gap is the subject of the unattended parity campaign in [WEBP-HILLCLIMB](docs/WEBP-HILLCLIMB.md). AVIF's worst case is `large.png` t80 at +212.5% (39,202 vs 12,544 bytes); most photographic AVIF cells at t90/t95 still refuse on both stacks. JPEG is mixed (best case `line.png` t80 −42.2%; `grass.png` t80 +67.2%).

Timing medians (process startup included; host shared with background load; ranges dominated by the large AVIF encodes at ≈7–8 s; ordered as observations, not orderings): WebP ≈49.8–52.6 ms permissive vs ≈36.5–36.7 ms master, consistent across 70 samples per target; AVIF and JPEG within ~1 ms of master. The WebP delta is not a verified ordering under the busy-host protocol — re-measure on a quiet host before drawing conclusions. Binary size falls from 7,265,712 to 4,523,200 bytes (−37.8%).

The normal/build [license inventory](benchmarks/production-dependencies.json) was regenerated at `c088ac8`: 149 packages across `aarch64-apple-darwin`, `aarch64-unknown-linux-musl` and `x86_64-unknown-linux-musl`, zero declared AGPL/GPL/LGPL entries, zero build scripts. `quantizr` 1.4.3 declares no SPDX expression (recorded as `license-file: LICENSE`); its shipped license text is verbatim MIT and is recorded as the selected license. `webpkit` is a vendored path dependency pinned at `cc1311ebf`, not a registry artifact. The 137-package inventory earlier in this file records the pre-merge production closure.

This benchmark is the evidence for the 2026-11-13 merge to master. The WebP backend remains experimental and its photographic size regressions plus the AVIF color-description change ([docs/AVIF.md](docs/AVIF.md)) are recorded limitations; gating them is the job of the unattended parity campaign, and any claim of WebP parity requires its gates to pass.

## Remaining proofs

These measurements need the stated environments. Existing results apply to their recorded source and supported fixtures; they do not establish correctness for every accepted image representation.

Optional implementation and dependency changes need a reproducible benefit beyond observed spread. Reject regressions above 10% in median runtime, build time, release size or peak RSS unless a required correctness capability justifies the cost. Record those costs explicitly. Compare equivalent completed workflows; a multi-candidate optimizer is not equivalent to one legacy encode.

| Proof | Prerequisite and retry |
| --- | --- |
| Native Linux timing | Dedicated stable x86-64 and ARM64 hosts. Run `make benchmark-run` and `make benchmark-verify`; require five interleaved completed cases, passing load gates, and medians separated from spread before claiming an ordering. Emulated functional execution is not native timing. |
| Stable startup, file and batch comparisons | A quiet host and immutable normal-dist binaries. Run `scripts/benchmark-workflows ORACLE_BINARY CURRENT_PRODUCTION_BINARY`; retain correct outputs and report inconclusive differences when spread overlaps. |
| Physical clean macOS build | macOS ARM64 without third-party native build tools. Run `make pure-rust-build-check`, inspect its empty native-tool log and `otool -L`; require OS-only linkage. |
| JPEGli and external eight-bit AVIF comparisons | Install those tools and validate command adapters before extending `examples/benchmark.rs`. `make benchmark-run` does not discover new tools automatically. Rerun verification and retain failures alongside actual decoded scores and dimensions. |
| Allocation and copy totals | An environment where Instruments can attach. Run `xcrun xctrace record --template Allocations --no-prompt --time-limit 30s --output .work/tmp/allocations.trace --launch -- target/dist/rimage convert benchmarks/corpus/generated/large.png --score 95 --effort fast -o .work/tmp/profile.webp --quiet` with fresh output names. Require an attached trace and attributed counts for completed work; unsupported zero counters are not measurements. |

AVIF alpha, monochrome, high depth/HDR, grids and unsupported display transforms are product refusals, not deferred environment proofs. Their retry conditions belong in [AVIF support](docs/AVIF.md). Open implementation defects belong in [TODO.md](TODO.md).
