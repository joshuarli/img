# Benchmark evidence

## Running benchmarks

`scripts/bench` (or `make bench`, `make bench-check`, `make bench-list`) is the
entrypoint. It compares the current checkout's distributable build (`target/dist/img`)
with a base revision (`--base`, default `origin/master`) on the frozen qualification
corpus, and reuses the harnesses under `benchmarks/codecs/`.

```sh
python3 -I scripts/qualification-corpus .work/qcorpus   # once; verifies pinned originals
scripts/bench build                                      # new and base dist binaries, base worktree
scripts/bench check --tier change                        # preflight only
scripts/bench run --tier change                          # or: make bench TIER=change
scripts/bench run --tier scenario --only workflow-scored --dry-run
scripts/bench profile -- optimize IN -o OUT --score 90   # sampled CPU profile of one case
```

| Tier | What runs | Observed cost (M1) |
| --- | --- | --- |
| edit | harness scripts compile; `tests/test_bench.py` selection, preflight, guard and budget rules | 1 s |
| change | one gated round each: SSIMULACRA2 bit-identity and per-score time (`score-exact`), in-process PNG encode (`png-encode`), scored and lossless single-image workflows, three rounds of a 55-PNG batch at `-j 4` | about 1 min warm; about 3 min when the in-process loops rebuild |
| scenario | 5 gated rounds of the workflows and PNG encode on 12 files, 3 batch rounds at `-j 1`/`-j 4`, 9-round JPEG decode qualification | estimated 2 h |
| full | adds AVIF and PNG decode library qualification and `make benchmark-run` | hours; manual only |

Selectors: `--tier` is a ceiling (each workload runs at its highest level at or below
it), `--only` takes workload names, `--area` takes areas, `--skip` takes names. Unknown
names and empty selections fail. `--tier full` refuses to run without `--confirm-full`;
use it only when a full run is explicitly requested. Preflight checks tools, binary
freshness against `src`, `vendor`, `codecs` and the manifests, the corpus hashes, the
decode fixtures a workload needs, free disk and leftover benchmark processes; any
problem stops the run before anything executes and names the fix. `--dry-run` prints
the commands and writes nothing.

**Budgets.** `benchmarks/budgets.json` holds the regression budgets (new/base): 5% for
wall, CPU, peak RSS, encode and scoring time, 0.2% for output bytes, and a 10% limit
per individual case. A metric *regresses* when the bootstrap 95% CI lower bound of its
geomean ratio exceeds the budget, is *within budget* when the upper bound is at or
below it, and is otherwise *inconclusive*. Every case over the per-case limit needs a
written disposition in the JSON file named by `BENCH_DISPOSITIONS` (metric → case label
→ reason); an undisposed case fails the verdict like a regression.
One-round change tiers are often inconclusive, and a one-round result stays a smoke
result whatever its across-image interval; rerun the workload at `--tier scenario`
before acting on a suspected regression. Correctness gates are separate and always
fail the workload: score bits must match the base, the new build must complete every
case the base completes, and lossless encodes must round-trip exactly.

**Verification and equivalent work.** Workflow and batch runs keep one output per
case and tool. After the timed rounds, one verifier binary
(`benchmarks/codecs/verify.rs`, built by `loops-build.sh` from this checkout, or the
binary named by `BENCH_VERIFIER`) decodes every retained output for every tool and checks it
against its source: exact samples for lossless, a measured SSIMULACRA2 score at or above
the target otherwise. A case succeeds only when it exited 0 and was accepted. Batch
reports attempted throughput, successful throughput and coverage separately, and
compares timing only when every tool, job count and round succeeded on the same input
identities. A requested encoder quality is a diagnostic setting, not evidence of equal
achieved quality.

**Holdout.** `--split holdout` runs the corpus workloads on the 50 held-out originals
(all of them, not the dev picks). Use it only to qualify a frozen candidate, never
while tuning; the run directory carries a `-holdout` suffix and `summary.json` records
the split.

Each run writes `.work/bench/runs/<UTC>-<tier>/` with a per-workload `log.txt`, result
JSON, `*.verdict.json` and `summary.json` (commit, dirty state, binary and corpus
hashes, exit status, budget verdicts, duration, kept and discarded rounds, exclusions).
Exit status is nonzero for any failure or regression. `.work/` is disposable: copy
evidence that matters into `benchmarks/results/` and summarize it in `docs/CODECS.md`.

**Decode qualification inputs.** `benchmarks/codecs/decode-prepare {jpeg,png,avif,all} DEST
--corpus DIR --img BINARY [--base-checkout DIR]` fetches or copies the pinned reference
decoders (checksummed archives; libaom by verified commit), builds the in-process
harnesses and generates fixtures from the dev split, all under `DEST`; it is idempotent
and `--dry-run` prints the steps. `scripts/bench check` prints the exact command for the
default `DEST` (`.work/bench/decode`). The PNG library harness needs libpng 1.6.59 via
`pkg-config`; the dav1d build needs upstream Meson (set `MESON=` if the default `meson`
cannot configure it). The jpeg-decode and avif-decode workloads rebuild their img
harnesses on every run, so they always measure the current sources.

**Attributing a change.** `scripts/bench profile [--binary new|base] -- ARGS` runs one
symbolized release build (`cargo build --release --features build-binary`) under
macOS `sample` or Linux `perf` and writes the top-of-stack table to
`.work/bench/profiles/`. Runs shorter than about a second capture little; profile the
in-process loops instead (`.work/bench/loops/target-*/release/score-loop` and
`png-encode-loop`, built by `benchmarks/codecs/loops-build.sh`), which repeat the work.
Peak RSS per case is in every workflow and batch result; keep profiled runs separate
from timed ones.

## Current WebP interpretation — 2026-11-15 review

The current campaign, gate table and next tasks are in
[docs/WEBP-HILLCLIMB.md](docs/WEBP-HILLCLIMB.md), reviewed at `6141369`.
The dated sections below retain evidence from their own revisions; their old
priority lists and blocker conclusions are historical.

- [The withdrawn capture](benchmarks/results/untimed-withdrawn-2026-11-15.json)
  records smaller outputs in 33/33 shared completed cells, **11 distinct images
  at three minimum requested scores**. Achieved scores often differ materially;
  this establishes that workflow result, not an equal-quality codec frontier.
  The completing photo set is coffee plus grayscale grass/gravel. The cwebp
  comparison uses a coarse lossy-only grid, not its full lossless/lossy frontier.
- [The latest full timed capture](benchmarks/results/timed-neon-2026-11-15.json)
  is at `3969a55`, before `c3c1abe` changed encoder search and output bytes.
  It has 1,190 records / 505 completed and five serial rounds. Its `elapsedMs`
  includes startup, reference preparation, search, readback, decode and scoring;
  label it workflow/harness wall, not isolated encode time. See the current
  campaign for verified per-cell medians. Default-parallel diagnostics and this
  serial capture are separate evidence.
- The current harness hashes the img executable but identifies the other tools
  through labels; stronger per-tool artifact provenance is still required.
  Saved WebP outputs are marked non-exact even when the winner is lossless,
  because the harness currently sets exactness only for PNG. A score of 100
  does not prove sample identity. Existing independent WebP tests remain useful;
  archive replay is not a fresh photographic encode matrix at current HEAD.

**WebP qualification is incomplete.** Selection equivalence, representative
held-out frontiers, current-source timing/resource evidence and native platform
proofs remain open. The existing 1.25 lossless-probe ratio is an empirical
heuristic, not a proof that the skipped candidate cannot win; repairing this
against the recorded selection contract is the campaign's first task. No raw
capture was changed or new performance measurement made by this review.

## Historical baseline overview

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

The following table records the **original historical configurations**, including
the former eight-candidate/lossless-first policy. It does not describe current
HEAD; use the current campaign's baseline instructions and record resolved
settings afresh. These historical configurations use one image worker and
RAYON_NUM_THREADS=1:

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
make native-build-check
# Explicit target prevents static CRT flags being applied to host proc macros:
RUSTFLAGS='-C link-arg=-fuse-ld=mold -C target-feature=+crt-static' scripts/check-native-build aarch64-unknown-linux-musl
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

A second full-matrix capture after the 2026-11-15 WebP landings (cap 9, DP tier-gate, `q_ac^2` trellis lambda) is committed as [untimed-full-lambda-2026-11-15.json](benchmarks/results/untimed-full-lambda-2026-11-15.json) with prefixed `2026-11-15-lambdafull-*` outputs (same protocol; revision `2000888`, `sourceDirty: false`; refusal set byte-identical; img cells byte-identical to the lambda capture). **img-WebP now beats original-mozjpeg on all 18 shared matched-score cells** — gravel t90 flips to img (98,420 vs 110,756 @ 90.7); grass t90 is now 128,872 vs 190,382. The img-PNG stack still loses every matched-size cell to oxipng (the recorded honest cost of the permissive migration).

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

### Wall verdict after the landings (fresh timed ledger)

Committed as [timed-lambda-2026-11-15.json](benchmarks/results/timed-lambda-2026-11-15.json) with prefixed `2026-11-15-lambdatimed-timed-*.webp` outputs (quiet host, `perf-gate-ready` 0 pre/post, `RAYON_NUM_THREADS=1`, five interleaved repetitions, `timingGate: accepted`, revision `e11aeca`, 420 records / 280 completed, `sourceDirty: false`; cwebp medians flat). Encode medians (startup-inclusive walls), vs the 2026-11-15 timed ledger:

| Fixture | img med ms (was) | Δ | cwebp med ms | img ≈ ×cwebp |
| --- | --- | --- | --- | --- |
| large.png (2048×1024, deep DP by the 1 MP override) | 40,405–40,442 | 0.0% | 1,499 | ≈27× |
| coffee t80/t90/t95 | 1,100/1,100/1,100 | +7.4/+7.6/+7.8% | 288 | ≈3.8× |
| grass t80/t90/t95 | 1,559/1,577/1,538 | +3.9/+3.5/+3.6% | 370 | ≈4.2× |
| gravel t80/t90/t95 | 1,623/1,672/1,723 | −5.5/−4.0/+0.3% | 341 | ≈4.8× |
| transparent t80/t90/t95 | 94/93/93 | +3.9/+2.0/+1.9% | 32 | ≈2.9× |
| line t80/t90/t95 | 48/49/49 | −63% | 28 | ≈1.7× |
| text t80/t90/t95 | 45/45/45 | −65% | 27 | ≈1.7× |
| ui t80/t90/t95 | 38/36/35 | −28/−27/−29% | refused | — |
| compressed t80/t90/t95 | 37/35/35 | −27/−29/−28% | refused | — |
| flat/gray | 28–29 | flat (±3%) | 24–25 | ≈1.2× |

Median-of-medians 90.8 → 44.8 ms (−51%). The `q_ac^2` trellis keeps more coefficients than the `>>2` rate weight, so the lossy hot path (`trellis_eval`) does slightly more work — coffee/grass +3.6–7.8% is that, not noise between quiet-host captures (cwebp medians were flat). The deep-DP wall on small repetitive cells is gone (tier-gate); the 1 MP override keeps the large.png win at the same ~40.4 s it cost before. Photographic encode walls are otherwise unchanged in shape: 3.6–5.7× cwebp process walls, 3.3–4.4× the withdrawn closure — the open arm is a vectorized trellis/match inner loop, not a capture.

## Remaining proofs

These measurements need the stated environments. Existing results apply to their recorded source and supported fixtures; they do not establish correctness for every accepted image representation.

Optional implementation and dependency changes need a reproducible benefit beyond observed spread. Reject regressions above 10% in median runtime, build time, release size or peak RSS unless a required correctness capability justifies the cost. Record those costs explicitly. Compare equivalent completed workflows; a multi-candidate optimizer is not equivalent to one legacy encode.

| Proof | Prerequisite and retry |
| --- | --- |
| Native Linux timing | Dedicated stable x86-64 and ARM64 hosts. Run `make benchmark-run` and `make benchmark-verify`; require five interleaved completed cases, passing load gates, and medians separated from spread before claiming an ordering. Emulated functional execution is not native timing. |
| Stable startup, file and batch comparisons | Resolved 2026-11-14 on a quiet host with immutable binaries: `scripts/benchmark-workflows` with the retained `2708512` oracle and current master `1424358` (30 samples; pre/post `perf-gate-ready` passed). Results in [workflows-macos-arm64.json](benchmarks/workflows-macos-arm64.json): `startup_version` 3.75 ms vs oracle 3.13 ms (+20%, spreads 58/167 µs), `png_file_conversion` 20.2 ms vs 6.95 ms (+190%, spreads 1.0/0.1 ms), `optimize_scored_file` 24.3 ms, `lossless_batch` 87.7 ms (spread 3.3 ms). The PNG file-conversion gap is the pure-Rust PNG stack's cost vs the pre-rewrite closure; it is an honest tradeoff of the migration, not a regression introduced by these iterations. |
| Physical clean macOS build | macOS ARM64 with only the system compiler. Run `make native-build-check`; require that only allowlisted vendored sources compile and `otool -L` shows OS-only linkage. |
| JPEGli and external eight-bit AVIF comparisons | Install those tools and validate command adapters before extending `examples/benchmark.rs`. `make benchmark-run` does not discover new tools automatically. Rerun verification and retain failures alongside actual decoded scores and dimensions. |
| Allocation and copy totals | An environment where Instruments can attach. Run `xcrun xctrace record --template Allocations --no-prompt --time-limit 30s --output .work/tmp/allocations.trace --launch -- target/dist/img convert benchmarks/corpus/generated/large.png --score 95 --effort fast -o .work/tmp/profile.webp --quiet` with fresh output names. Require an attached trace and attributed counts for completed work; unsupported zero counters are not measurements. |

AVIF alpha, monochrome, high depth/HDR, grids and unsupported display transforms are product refusals, not deferred environment proofs. Their retry conditions belong in [AVIF support](docs/AVIF.md). Open implementation defects belong in [TODO.md](TODO.md).

**Wall-arm negative result (2026-11-15; historical).** A profile-guided micro-hoist in the trellis inner loop (`wdist`/`ac`) built under the campaign protocol produced byte-identical outputs but ran consistently *slower* across five interleaved 100-rep A/B rounds (51.75 vs 51.42 ms per coffee-q80 encode, 5/5) and was reverted. This rejects that micro-hoist on that configuration; it does not prove other safe changes cannot improve wall time or that vectorization is the only remaining route. The contemporary 3.6–5.7× cwebp / 3.3–4.4× withdrawn workflow gaps belong to that revision. Use the current campaign for later evidence and priorities.

**Owner-approved Fast-ladder landings (2026-11-15, master `0ed4a1e`/`5a10a81`; fresh capture committed as [untimed-ladder-2026-11-15.json](benchmarks/results/untimed-ladder-2026-11-15.json) with prefixed `2026-11-15-ladder-*.webp` outputs, `img`/`cwebp`, 84 records / 56 completed, `perf-gate-ready` 0 pre/post, revision `5a10a81`, `sourceDirty: false`, refusal set byte-identical, cwebp cells deterministic).** L6 lossless fallback for the 200k–400k px photographic mid-band (Fast keeps L1 elsewhere) flips the coffee t90/t95 cells to **353,630 @ 100.0** — beating the withdrawn closure's 359,054 (wall ≈8.4 s/cell, owner-accepted) and the lossy ceiling being 87.6. Cap 10 (refined frontier straddle) further improves the photographic t80 cells: coffee 67,296 → **57,762 @ 80.44**, grass 76,818 → **73,350 @ 80.40**, gravel 50,566 → **49,286 @ 80.22**; grass t95 → 212,388 @ 100 (accepted loss). Transparent t80 stays **174 @ 81.0** (+47% vs closure; cap 10 changed nothing there — efficiency of the lossless small-file path untouched; open). `benchmark_verify` re-validates all retained and fresh records. The destination gate remains **NOT met** (gravel t95, transparent cells, encode wall; see `docs/WEBP-HILLCLIMB.md`).

**Small-tier L6 Fast-ladder extension + small-image tile-sweep (2026-11-15, master `cb63deb`/`636d57e`, fresh capture committed as [untimed-small-2026-11-15.json](benchmarks/results/untimed-small-2026-11-15.json) with `2026-11-15-small-*` outputs, `img,cwebp`, 84 records / 56 completed, `perf-gate-ready` 0 pre/post, revision `636d57e`, `sourceDirty: false`, refusal set and cwebp determinism byte-identical).** **transparent t80/t90/t95 flip 174/200/200 -> 110 @ 100.0** — beating the withdrawn closure's 118 B by 8 B on all three (lossless candidate at L6 with the fine tile-bit grid for sub-16,384 px images). Small synthetics: line 64 -> 62, text 72 -> 58; ui and compressed 66 -> 68 (+2, bounded, all still lossless wins); everywhere else unchanged. **The remaining gate gaps vs the withdrawn closure: grass t95 (212,388 @ 100; accepted loss), gravel t95 (176,698 @ 95.04; accepted loss), and encode-time parity (3.6-5.7x cwebp process walls). Matched-score bytes now beat the closure on every other workload group.** `benchmark_verify` passes on retained + fresh records.

**VOID timed run (2026-11-15).** `run`-mode timed capture completed 420 records/280 completed but was invalidated: a sibling opencode session's `perf.py perf-rust-jit` JIT build (pid 38837, 100% CPU) overlapped the measurement window after the pre-check passed. Records moved to `.work/tmp/invalid-timedladder-2026-11-15/` (untracked, diagnostic only). No timed claim is made; [timed-2026-11-14.json](benchmarks/results/timed-2026-11-14.json) and [timed-lambda-2026-11-15.json](benchmarks/results/timed-lambda-2026-11-15.json) are the current wall references. The L6 Fast fallback wall inflation visible in the discarded diagnostics (coffee +8.2 s per convert) motivates either the pending owner ladder-policy decision or acceptance of the tradeoff.

**Newhost timed baseline (2026-11-15, master `d38ac54`).** Fresh checkout on a new host (macOS arm64 M1, 8 cores, 8 GiB RAM); full rebuild with kache bypassed (`RUSTC_WRAPPER=`, clang); `benchmark_verify` 2/2 against retained records. The first timed run completed (420/280) but was **voided**: its post-run gate printed a hot process tree (user's `sync/main.py`, ~345% CPU, 4 s elapsed) and run-4 `compressed.webp` cells were +12–27% (230/262/259 ms vs ~204–210); records and outputs quarantined untracked at `.work/tmp/invalid-newhost-2026-11-15/`. The re-run, identical protocol, is committed as [timed-newhost-2026-11-15.json](benchmarks/results/timed-newhost-2026-11-15.json) with `2026-11-15-newhost-timed-*` outputs (revision `d38ac54`, `sourceDirty: false`, tools `img,cwebp`, 5 interleaved reps, `RAYON_NUM_THREADS=1`, `perf-gate-ready` pre/post exit 0, no advisories, both gates clean). Cross-host byte determinism re-witnessed: coffee/grass/transparent outputs are sha256-identical to the committed ladder/small captures. Medians vs the retained timed-lambda ledger (different host): cwebp process walls are +5–45% on this host (worst on the small startup-bound cells). img cells reflect the mandatory-L6 Fast candidate that landed after that ledger (`0ed4a1e`/`5a10a81`/`636d57e`): coffee 9219/9220/9232 ms, grass 6750/6798/6720, gravel 7510/7576/7660, large.png ≈41.4 s (flat), transparent 431/433/437, line ≈615, text ≈595, ui/compressed ≈204 (+2 B), flat/gray ≈70–76 (+41/+47). The discarded-run wall diagnostics from the old host are confirmed by a clean measurement: every Fast cell pays the L6 candidate where it applies, so photographic cells now sit at ≈17–30× cwebp process walls — the single largest gate gap. The candidate-policy question (tier-gate / effort-gate / lazy-evaluate the L6 lossless candidate) remains an owner decision. **No byte claim is made from this ledger** (untimed captures are host-independent). The destination gate remains NOT met.

**Sorted lossless match memo (2026-11-15, `560864d`).** Binary search replaces the linear lookup of sorted distances in the backward parser's shift memo. Candidate order, lengths, search effort and selection policy stay identical. A separate symbolized profile located 2,991 of 3,775 sampled main-thread stacks in the inlined parser, predominantly at the memo's linear-scan instructions. [Five interleaved CLI rounds](benchmarks/search-memo-macos-arm64.json) compare immutable dist binaries against `b20c6f0` on the same M1 host, with clean pre/post load gates and one Rayon thread:

| Input and score | Baseline median, range (s) | Candidate median, range (s) | Output bytes |
| --- | --- | --- | --- |
| large t80 | 41.310, 41.150–41.576 | 15.176, 15.012–15.371 | 4,846 |
| coffee t80 | 9.238, 9.154–9.297 | 9.219, 9.189–9.297 | 57,762 |
| coffee t95 | 9.253, 9.215–9.306 | 9.261, 9.188–9.274 | 353,630 |
| transparent t80 | 0.433, 0.431–0.437 | 0.432, 0.429–0.438 | 110 |

Large improves 63.3%; the other cases overlap their baseline spread. Every pair has identical SHA-256 output. Separate large-t80 peak-RSS samples are 437,174,272 → 437,256,192 bytes (+0.02%). The production binary grows 32 bytes (4,490,224 → 4,490,256). No dependencies or allocations were added. The 912 default-feature codec tests and 49 focused product tests pass. The campaign gate remains **NOT met**: photographic L6 cost and the two t95 byte losses remain. Skipping L6 after a lossy pass was discarded because it cannot guarantee identical selection on arbitrary images.

The [fresh full WebP capture](benchmarks/results/untimed-memo-2026-11-15.json) records revision `560864d`, `sourceDirty: false`, 84 records and 56 completed outputs. Every completed output compares byte-identically with the small-tier capture, including cwebp; all recorded scores, sizes and refusals match. This capture provides correctness and selection evidence, not new timings.

**Forward lossless match-memo cursor (2026-11-15, `9c9de14`).** Lookup requests increase with hash-chain distance. One cursor consumes the prior position's sorted memo and resets at the existing buffer swap, replacing repeated binary searches. [Five alternating CLI rounds](benchmarks/cursor-memo-macos-arm64.json) compare immutable dist binaries against `85bacd3`, with the same M1, pinned toolchain, one Rayon thread, and clean pre/post load gates:

| Input and score | Binary-search median, range (s) | Cursor median, range (s) | Output bytes |
| --- | --- | --- | --- |
| large t80 | 15.018, 14.925–15.031 | 11.022, 10.955–11.191 | 4,846 |
| coffee t80 | 9.155, 9.146–9.222 | 9.223, 9.197–9.273 | 57,762 |
| coffee t95 | 9.154, 9.144–9.238 | 9.212, 9.131–9.269 | 353,630 |
| transparent t80 | 0.431, 0.429–0.439 | 0.431, 0.430–0.439 | 110 |

Large improves another 26.6%. Coffee t80 is 0.7% slower with overlapping ranges; coffee t95 and transparency also overlap baseline spread. Every pair has identical output hashes. Separate large-t80 peak RSS is 437,207,040 bytes versus the retained binary-search sample's 437,256,192. The binary shrinks 16 bytes to 4,490,240. No new allocation or dependency is introduced. All 912 default-feature codec tests, 49 focused product tests and the no_std + alloc check pass. The [fresh full WebP capture](benchmarks/results/untimed-cursor-2026-11-15.json), revision `9c9de14`, `sourceDirty: false`, contains 84 records / 56 completed outputs. All 56 outputs compare byte-identically with the preceding capture; scores, sizes and refusals match. The campaign gate remains **NOT met**.

**Single-pass merged-histogram cost (2026-11-15, `f5dc88b`).** Totals and weighted logarithms now accumulate in one pass over paired histogram bins. Exact integer costs and candidate rankings stay unchanged. A separate 10-second L6 coffee profile attributed 4,924 of 7,431 sampled main-thread stacks to merge-cost estimation. [Five alternating immutable-dist CLI rounds](benchmarks/histogram-pass-macos-arm64.json), against `1e48112` on the same M1 with clean pre/post load gates, measure:

| Input and score | Baseline median, range (s) | Candidate median, range (s) |
| --- | --- | --- |
| coffee t80 | 9.234, 9.186–9.268 | 8.986, 8.899–9.012 |
| grass t95 | 6.659, 6.641–6.749 | 6.523, 6.501–6.532 |
| large t80 | 10.998, 10.971–11.033 | 11.002, 10.947–11.267 |
| transparent t80 | 0.434, 0.430–0.437 | 0.428, 0.427–0.431 |

Coffee improves 2.7% and grass 2.0%, with disjoint ranges. Large and transparency overlap baseline spread. A separate five-round sparse-image check passes both load gates and shows overlapping ranges on line, text, flat and gray; its raw samples are included in the same ledger. Every output hash matches. Coffee peak RSS is 70,778,880 → 70,828,032 bytes (+0.07%); binary size remains 4,490,240 bytes. No allocations or dependencies were added. All 913 default-feature codec tests, 49 focused product tests and no_std + alloc pass.

The [fresh full WebP capture](benchmarks/results/untimed-histogram-2026-11-15.json), revision `f5dc88b`, has clean source, 84 records and 56 completed outputs. All 56 outputs compare byte-identically with the preceding capture, and every recorded score, size and refusal matches. The destination gate remains **NOT met**.

**Occupied-bin entropy clustering (2026-11-15, `20f1019`).** Each clustering histogram owns its channel totals and occupied-bin masks. Merging histograms updates those summaries together. Pair costs skip uniform channels, visit sparse supports, and reuse totals for dense channels. The uncached cost model remains a test oracle; costs, merge order and selected bytes remain exact.

[Five alternating immutable-dist CLI rounds](benchmarks/histogram-support-macos-arm64.json), against `b5d0dcb` on the same M1 with one Rayon thread and clean pre/post load gates, measure:

| Input and score | Baseline median, range (s) | Candidate median, range (s) |
| --- | --- | --- |
| coffee t80 | 8.925, 8.856–8.964 | 5.347, 5.318–5.382 |
| grass t95 | 6.442, 6.416–6.537 | 4.867, 4.835–4.888 |
| transparent t80 | 0.428, 0.428–0.429 | 0.337, 0.336–0.339 |
| large t80 | 10.923, 10.877–11.036 | 10.924, 10.888–11.164 |

Coffee improves 40.1%, grass 24.5% and transparency 21.2%, with disjoint ranges. Large overlaps baseline spread. A separate five-round sparse check passes both load gates and overlaps baseline spread on line, text, flat and gray. All paired output hashes match. The cache adds five short bitmap allocations per initial nonempty cluster, with at most 256 clusters per planner. Separate coffee peak-RSS samples (one per side) are 70,828,032 → 68,091,904 bytes. Binary size stays 4,490,240 bytes. No dependencies are added.

All 914 default-feature codec tests, 49 focused product tests and no_std + alloc pass. The [fresh full WebP capture](benchmarks/results/untimed-support-2026-11-15.json), revision `20f1019`, has clean source, 84 records and 56 completed outputs. Every completed file compares byte-identically with the preceding capture, and all scores, sizes and refusals match. The destination gate remains **NOT met**.

**Weighted entropy lookup (2026-11-15, `639b312`).** An exact `u32` table supplies `count * log2(count)` for small counts, including zero. Dense merge-cost scans use it without a per-bin zero branch. The table derives from the existing fixed-point log values and has a compile-time overflow guard; larger counts retain the prior calculation. Exhaustive lookup tests and the uncached cost oracle verify exactness.

[Five alternating immutable-dist CLI rounds](benchmarks/weighted-log-macos-arm64.json), against `891c708` on the same M1 with one Rayon thread and clean pre/post load gates, measure:

| Input and score | Baseline median, range (s) | Candidate median, range (s) |
| --- | --- | --- |
| coffee t80 | 5.320, 5.318–5.338 | 4.566, 4.561–4.576 |
| grass t95 | 4.833, 4.818–4.836 | 4.657, 4.655–4.707 |
| transparent t80 | 0.338, 0.337–0.339 | 0.324, 0.322–0.325 |
| large t80 | 10.872, 10.864–10.903 | 10.914, 10.874–11.166 |

Coffee improves 14.2%, grass 3.6% and transparency 4.1%, with disjoint ranges. Large overlaps baseline spread. A separate five-round sparse check passes both gates and overlaps spread on line, text, flat and gray. Every paired output hash matches. Binary size grows 16,512 bytes, 4,490,240 → 4,506,752 (+0.37%); no runtime allocations or dependencies are added. Coffee peak-RSS samples (one per side) are 68,091,904 → 67,354,624 bytes.

All 915 default-feature codec tests, 49 focused product tests and no_std + alloc pass. The [fresh full WebP capture](benchmarks/results/untimed-weighted-2026-11-15.json), revision `639b312`, has clean source, 84 records and 56 completed outputs. Every completed file compares byte-identically with the preceding capture; scores, sizes and refusals match. The destination gate remains **NOT met**.
2026-11-19: retained `original-mozjpeg` outputs were rescored after JPEG sample decoding moved from zune-jpeg to libjpeg-turbo 3.2.1; outputs, byte sizes, timings, completions and the 0.0001 tolerance are unchanged.
