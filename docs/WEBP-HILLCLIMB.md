# WebP correctness and Pareto hillclimb

## Current brief — reviewed 2026-11-15

This is the authoritative operating brief for the next unattended run in
`joshuarli/img`, branch `master`. The previous runner paused; this documentation
review does not claim to have restarted it or measured a new binary. Reviewed
source: `c0e1085`; latest encoder change: `c3c1abe` (`c0e1085` adds the
owner-granted host-scoped `target-cpu=native` flag and this brief's note;
docs/config only). Start from the current default branch, preserving
intervening work.

**Verdict: useful codec improvements, unfinished qualification, and a search-cost
problem that small kernel changes alone cannot close.** The next priorities are
selection correctness, a representative reproducible baseline, and reducing
repeated full-encode work. Do not treat the old instruction to optimize an 8%
emission hotspot or 2.5% copy hotspot as the default next task.

The complete prior brief and experiment ledger are preserved byte-for-byte in
[WEBP-HILLCLIMB-HISTORY-2026-11-15.md](WEBP-HILLCLIMB-HISTORY-2026-11-15.md).
Use it to avoid repeating experiments and to recover evidence, not as a second
queue. Its paused snapshots, "sole blocker", "unreachable" and "exhausted"
conclusions describe particular revisions and hypotheses, not current facts.

## Mission, scope and authority

Bring the existing permissively licensed pure-Rust WebP encoder to a practical
speed–size–quality frontier competitive with pinned zenwebp and libwebp, while
preserving `img`'s still-image contracts. Finish engineering work: implement,
test, profile, measure, reject or land focused changes, and continue autonomously.
Routine changes with passing gates may be committed and pushed to the default
branch. Use isolated worktrees for parallel experiments and land serially.

Webpkit is **already the production encoder**, through `src/still/webp.rs` and a
path dependency with Rayon enabled. Production decoding is `image-webp 0.2.4`.
The encoder remains experimental. The migration/dependency-removal instructions
in the original Stage 4 are historical; do not repeat the migration or replace
the production decoder to make this campaign look complete. The repository
already records a permissive normal/build closure; verify that closure when
dependencies change.

Read `docs/CODEC-BOUNDARY.md`, `docs/COLOR.md`, `docs/SCORING.md`,
`docs/OPTIMIZATION.md`, `docs/PROVENANCE.md`, `BENCHMARKS.md`,
`src/still/webp.rs`, `src/optimizer.rs`, and `tests/webp_codec.rs`.
Where historical prose disagrees with source, record the discrepancy and repair
the owning document with the corresponding implementation change.

### Preserve the existing decisions

- Pure Rust production codec/color runtime; no C/FFI or native codec build
  dependency. Keep permissive notices. zenwebp is a separately compiled oracle;
  do not copy its AGPL implementation. Permissive libwebp ports need attribution.
- Preserve precision, exact lossless samples including hidden RGB, lossy alpha,
  color/metadata/orientation policy, animation refusal, resource limits,
  cancellation, atomic publication, and supported CLI/API behavior. Never lower
  score thresholds or broaden accepted color conversions to improve the table.
- The owner approved Fast's **10 optimizer candidate slots**, the L6 lossless
  bands **<=16,384 pixels and 200,000–400,000 pixels**, and routine engineering
  autonomy. Keep those production decisions while repairing the selector.
  They are measured policy choices, not evidence that those bands generalize.
- The owner also required **identical selection results for lazy L6 evaluation**.
  Preserve the candidate schedule's observable decisions and tie behavior.
  "The pixels still pass" is insufficient for this selection guarantee.
- The aarch64 `lossless::vp8l::match_kernel` NEON exception is already authorized.
  The crate uses `deny(unsafe_code)` with that local exemption and a scalar
  fallback. Keep the standalone scalar/`no_std + alloc` path viable.
- The owner granted **`target-cpu=native` for aarch64-apple-darwin host builds**
  (`.cargo/config.toml`, host triple only; linux/musl/qemu cross targets keep
  generic codegen). The hot kernels are scalar-bound — trellis DP branches,
  hash-chain walks, the bit-writer — and none autovectorize; `match_length`
  already has hand NEON. Native codegen can still help future arms and
  non-codec paths (ravif/png decode), so it is kept per the owner's call;
  it does **not** move the encode-wall verdict (~4× serial vs the oracles on
  lossy photos). This is a specific grant for that host triple, not a blanket
  approval for native-only or unsafe code anywhere else.
- Codec `Fast` and product `--effort fast` are different: product lossy Fast
  maps to codec AUTO, which resolves to Best on images <=1 MP. Report both
  resolved settings; unchanged codec-Fast goldens do not prove CLI-Fast stability.

Maintain existing codec tests. New Webpkit animation/incremental-decoder features
are a separate library workstream; they do not block this encoder-focused
campaign. Product animation rejection and independent output decoding do.

## Evidence and open gates

The saved [withdrawn comparison](../benchmarks/results/untimed-withdrawn-2026-11-15.json)
at `77429f3` has **33/33 smaller outputs at the same minimum requested score**,
across **11 distinct completing images**, with nine refusals per side. This is a
useful workflow result. It does not establish equal-quality codec dominance.
For example, coffee t80 compares score 80.37 to 85.62; grass t90 compares 90.64 to
93.75. The tighter quality sweeps described in the old prose need retained raw
points before they can support a frontier claim.

The completing photographic inputs are coffee (color) and grass/gravel
(grayscale textures); astronaut refuses scored processing. The remaining
completing cases are procedural. Preserve these refusals and the old corpus.

The latest full [timed capture](../benchmarks/results/timed-neon-2026-11-15.json)
records `3969a55`, five alternating rounds, one Rayon thread, clean source and
an accepted host gate. It predates `c3c1abe`. These are **workflow/harness walls**,
including startup, reference preparation, search, readback, decode and scoring:

| Case / minimum target | img median ms | Withdrawn median ms | Ratio |
| --- | ---: | ---: | ---: |
| coffee / 80 | 1,274.342 | 346.761 | 3.68x |
| coffee / 95 | 10,355.296 | 318.694 | 32.49x |
| grass / 90 | 1,789.276 | 447.098 | 4.00x |
| gravel / 90 | 1,746.147 | 433.682 | 4.03x |
| large gradient / 80 | 25,624.879 | 2,180.883 | 11.75x |
| tiny UI / 80 | 382.345 | 20.860 | 18.33x |

These numbers are historical observations, not current-HEAD codec timings. The
later top-K mode-search patch reports a 4–7% workflow improvement with up to
0.74% more bytes on its tested cells; it needs a full fresh checkpoint. Do not
combine its default-parallel measurements with the serial table above.

| Gate | Reviewed state | Next evidence required |
| --- | --- | --- |
| Product migration and permissive closure | Already integrated; encoder experimental | Retain dependency/linkage proofs when affected |
| Independent sample/alpha/metadata correctness | Existing root and codec oracle tests; partial campaign coverage | Fresh current-source output validation on expanded inputs |
| Lazy selector equivalence | Repaired at `b06f91b`: exact pre-`a0e5a30` schedule restored, probe removed | Retain on regression; qualification sweep still owed |
| Size/quality frontier | Promising narrow minimum-target workflow result | Retained refined frontier points and held-out strata |
| Workflow speed | Material measured deficit | Fresh exact-selector baseline, then structural reductions |
| Isolated encoder/decoder, CPU, memory, tail | Incomplete or older revision evidence | Separate measurements tied to each artifact/configuration |
| Linux musl ARM64/x86-64 | Historical functional evidence; native timing incomplete | Native target evidence; no emulated timing claims |
| Broader conformance/fuzz/resource qualification | Existing tests, no complete campaign closure evidence | Bounded product-relevant qualification with recorded coverage |

**Destination gate: NOT met.** Wall time is the largest measured performance gap,
not the only unfinished technical obligation. The verdict remains ~4× serial vs
the oracles on lossy photos, gated by scalar trellis-DP and `parse_optimal`
work — `target-cpu=native` does not move it. A passing saved-output verifier
does not certify fresh encodes at current HEAD.

## P0 — Restore the promised selection semantics

Start here before claiming further selection-neutral speedups.

`LOSSLESS_PROBE_SKIP_MARGIN = 1.25` in `src/optimizer.rs` uses an empirical
L0-to-L6 compression ratio to skip the full lossless candidate. Even accepting
`L6_bytes <= L0_bytes`, a large L0 result does **not** prove L6 cannot beat the
incumbent: this is an upper bound in the wrong direction. The observed ~11%
improvement on the development corpus is not a universal bound.

1. Establish a test-only reference using the optimizer schedule immediately
   before `a0e5a30`, with current codec functions and the approved cap/L6 bands.
   Compare winning outcomes, refinement choices, failures and canonical ties.
   Physical call order may differ, and proven losers may be skipped, only when
   the logical selection result remains equivalent.
   Do not compare old versus new codec bytes and call that a scheduler test.
2. Remove empirical pruning from the default exact-selection path unless an
   admissible bound proves exclusion. Conservatively restore lossless-first
   evaluation, or prove an equivalent lazy schedule. Removing only the skip is
   insufficient: lossless outcomes can affect the refinement center, and
   reordered evaluation can change first-wins ties.
3. A safe exclusion must account for final container bytes and the existing
   tie rules: smaller size, then higher score, format preference, and canonical
   order. A byte lower bound strictly above the incumbent is sufficient;
   equality alone is not. L0 output, estimated entropy and DP proxy cost are
   not admissible lower bounds without a proof.
4. Count actual encoder calls and their cost, including probes. The current
   probe is outside `evaluations` and the reported cap: 10 slots can conceal an
   eleventh encode. Remove redundant probing or expose its work honestly;
   update `docs/OPTIMIZATION.md` alongside the repaired schedule.
5. Exercise equal-size/higher-score ties, refinement influenced by lossless,
   formats competing for slots, failures/refusals, and cases where deeper
   lossless search wins. Use deterministic selector-level cases plus real
   independently decoded images. Preserve the canonical selector's result.
6. Keep both baselines: the observed heuristic implementation and the repaired
   exact selector. Record any restored work/wall cost explicitly. Do not
   mislabel recovering a semantic guarantee as a performance improvement.

The owner-approved L6 bands and cap remain fixed during this repair. Approximate
pruning or a redesigned Fast budget may be researched separately with measured
tradeoffs; it cannot silently replace the promised production selection policy.

## P0 — Make the next baseline representative and reproducible

Build only the missing evidence needed for the next decision; extend the existing
harness instead of creating a generic benchmarking framework.

**Corpus.** Freeze the old set as development/regression data. Add a pinned,
licensed validation set before further tuning: at least a dozen previously
unused color photographs spanning smooth/detail/noise and multiple resolutions,
several real UI/text and alpha assets, plus odd dimensions and cases on both
sides of 16,384, 200,000, 400,000 and 1,048,576 pixels. Include ordinary 1–4 MP
photos and a 4K workload, not just enlarged synthetic gradients. Record source,
license, hash and preparation. Report per stratum and unique-image counts.
Keep the validation set out of knob selection; if it becomes tuning data,
declare that and obtain fresh validation inputs.

**Two measurement lanes.**

| Lane | What it answers | Required protocol |
| --- | --- | --- |
| Product workflow | What a user pays for the same request | Preserve historical CLI protocol; count calls, separate phases, include publication/startup deliberately |
| Codec frontier | Encoder quality/size/cost on the same pixels | Prepared pixel/layout/color inputs; time encode itself; independent decode/scoring outside timing; matched thread and effort/time budgets |

Keep serial and default-parallel results separate; include batch throughput and
CPU work so extra cores cannot disguise excessive work. Show phase totals for
encode, decode, scoring/preparation, probe, search and I/O. Investigate repeated
decoding/preparation in `optimizer`/`perceptual` before attributing every workflow
millisecond to Webpkit.

For the frontier, retain raw per-image quality/effort points and build
non-dominated size/score/time sets. Refine around target crossings without
assuming monotonic scores; report achieved scores and unresolved brackets.
Compare at the same or better achieved fidelity with a stated time budget.
Include libwebp lossy methods and exact-lossless settings and pinned withdrawn
zenwebp. The historical cwebp six-point lossy-only grid remains a historical
workflow comparator, not the full libwebp frontier. Unavailable optional
competitors do not block testing the available required oracles.

**Small harness repairs required before claims.**

- Identify and hash every executable independently: source/patch, lockfile,
  compiler, profile/flags/features, OS/CPU, thread policy, corpus and scorer.
  Distinguish harness revision from encoder revision. A current HEAD string or
  hardcoded oracle version does not establish the bytes that ran.
- Make new capture/output paths unique and retain historical evidence. Keep
  immutable release binaries; verify cache restores by identity/output or
  bypass the wrapper. Do not time builds or run sibling benchmarks concurrently.
- Record selected WebP encoding mode accurately. The current harness marks
  only PNG as exact; a WebP score of 100 does not prove lossless sample identity.
  For every lossless claim, compare expanded samples, alpha/hidden RGB and
  required metadata through independent decoders. Retain lossy exact-alpha
  checks. Use both libwebp and the product's independent Rust decoder.
- Verify archive integrity and freshly encode at the revision being qualified.
  `benchmark_verify` does useful generated-image checks and revalidates old
  outputs; it is not a fresh photographic matrix.
- Use alternating paired samples and pre/post quiet-host checks. Five rounds
  can screen a median improvement with a range; they do not establish p95.
  Gather enough samples for a stated tail estimator/confidence at qualification.
  Record peak RSS/CPU and call/work counts for shortlisted changes; do not
  substitute admission reservations or absent counters for measurements.

First deliverable: a coherent current-source baseline with the selector contract,
both measurement lanes, per-tool provenance and validation strata, plus a short
ranked gap table. An unavailable native Linux timing window does not block the
macOS checkpoint or portable correctness work.

## P1 — Reduce repeated work at the architecture level

Profile the repaired current pipeline on its worst material gaps. Count
transforms, greedy/optimal parses, entropy plans, full candidate encodes, trellis
calls and decode/score passes. The existing work-count feature is a starting
point. Record each hypothesis's affected wall fraction and plausible upper
bound before investing in it.

The historical profiles identify two promising mechanisms, to be confirmed on
the new baseline:

- **VP8L:** ordinary-size L6 evaluates roughly 26 transform tasks (the small
  tier's expanded sweep can run more), many running a full
  greedy/optimal parse and entropy plan. Prioritize exact reuse of equivalent
  work, avoiding repeated model construction/allocation, or a different
  candidate-generation/scoring architecture. Key reuse on all semantic inputs
  and preserve transform/container costs, cache choice and canonical ties.
  Cheap rankings may order exact work; they may discard it only with a sound
  exclusion proof. Do not promote proxy estimates into bounds.
- **VP8:** mode search multiplied trellis work across candidates. The top-K
  patch is a useful architectural direction, but the reported ~60% reduction
  in trellis calls produced only ~4–7% workflow improvement. Measure what now
  dominates and why. Explore less repeated transform/context/trellis work,
  shared decode/preparation, or an explicitly measured encoder search frontier.
  Distinguish product AUTO/Best paths from the codec's minimal Fast path.

Keep this ordering from the resumption assessment: the next real lever is the
**lossless transform×bitstream architecture** — libwebp `EncodeImageNoHuffman`-style
joint scoring of transform candidates by estimated final bitstream cost, the one
mechanism the campaign's cost-model probes point at — or **owner-approved
effort-tier changes**. Scalar kernel work (trellis DP branches, hash-chain walks,
the bit-writer) is not expected to close the remaining deficit, and a
`target-cpu`-style codegen flag demonstrably does not.

A local 8% hotspot has at most an approximately 1.09x end-to-end speedup even if
eliminated; it cannot explain away a 4–32x deficit. Small proven changes remain
welcome when inexpensive, but do not spend the campaign serially polishing
low-impact kernels while the repeated-work architecture remains untested.

Separate two kinds of patch:

1. **Equivalent-work implementation:** prove output/decision equivalence,
   independent decoded correctness, and time/memory benefit. This includes
   scheduling, caching and lazy evaluation under the selection constraint.
2. **Encoder search/RD change:** compressed bytes may change. Preserve lossless
   decoded identity and lossy alpha/metadata; require independently scored
   frontier improvements on development and validation strata. Do not reject
   every changed bitstream merely because an implementation golden changed.
   Keep the approved outer product selection policy intact.

A production effort-policy change is a separate decision supported by a concrete
frontier: show what an alternative Fast budget buys and costs across resolutions
and content. Prepare that experiment fully if needed; continue independent work
without waiting. Do not covertly lower effort, change thresholds to fit fixture
dimensions, increase the global cap, weaken the scorer, or treat old accepted
losses as permanent waivers for new regressions.

Negative results close a tested mechanism/configuration. They do not prove the
encoder "unreachable", "exhausted" or beyond an unattended campaign. A multi-session
architecture experiment is work to carry forward, not an external blocker.

## Acceptance, qualification and run loop

For each experiment, record the base/source and immutable artifact identities,
hypothesis, affected workloads, expected effect, exactness policy, raw before/after
evidence, and keep/revert decision. Use focused tests during iteration and the
broader relevant gates before landing. Keep only demonstrated gains beyond
measurement spread. As a regression alarm, investigate >5% median runtime or
>10% peak-RSS increases per important stratum; size/quality changes must be judged
on the frontier, not waved through because an aggregate mean improved.

Current baseline commands (native development oracle dependencies may be needed):

```sh
cargo test --locked --all-features --test benchmark_verify
cargo test --locked --all-features --test webp_codec --test color_fidelity --test codec_boundary --test optimizer --test cli
cargo test --manifest-path codecs/webpkit/Cargo.toml --locked --lib
cargo test --manifest-path codecs/webpkit/Cargo.toml --locked --features rayon,oracle --lib
cargo check --manifest-path codecs/webpkit/Cargo.toml --locked --no-default-features --features alloc
```

Use the repository's pinned toolchain and local temporary paths. Follow the
build conventions in CODEC-BOUNDARY; do not run formatters. Before production
qualification, run the full applicable product/codec suites, resource and
cancellation checks, product-relevant conformance/fuzz campaign, decoder/metric
references, pure-Rust/dependency/linkage checks, and native target matrix.
Record duration, seeds/corpus, revisions and actual coverage for fuzzing; a
test count or a few truncation cases does not close that obligation. Keep
unsupported hosts/tools as explicit evidence gaps, without calling them passes.

Qualification means a retained representative and held-out frontier with
competitive or lower bytes at the same achieved fidelity and comparable or
lower time, competitive decode/resource costs, and preserved product contracts.
Report named bounded exceptions by image/stratum; do not bury severe regressions
in a geometric mean or in a "33 wins" count. Validate Fast and other supported
efforts; codec and workflow results are separate claims. Linux-native timing
must be measured before any cross-platform performance claim.

At each checkpoint update the current gate/priority table and a concise
machine-readable result/decision record, with pointers to raw evidence. Keep
new narratives short; the archive is not a template for another growing diary.
A finite milestone is a verified selector, then a trustworthy baseline, then an
accepted structural improvement, then qualification. Completion of an
experiment or milestone is not permission to declare overall parity.

Continue until the destination is evidenced, the owner stops the run, or a real
external dependency blocks all useful in-scope work. If timing is unavailable,
complete correctness, harness, provenance or analysis tasks; never loop on
health checks or repeatedly recapture unchanged state. Report the exact blocker
and the next executable step. Do not ask again for decisions already recorded
above.

## Checkpoint — 2026-11-15 (P0 selector repair landed)

`b06f91b` restores the exact pre-`a0e5a30` score-search schedule: exact PNG/WebP
candidates evaluate first inside the effort cap, then the fixed quality grid and
deterministic refinement (`refine_step`, extracted unchanged). The level-0 probe
skip, the deferred-lossless pass, its cap-slot reservation, and
`StillImage::encode_webp_lossless_probe`/`webp::encode_probe` are deleted, so
every encode is counted in `evaluations` and the advertised cap. `docs/
OPTIMIZATION.md` now states the owner-approved 10/24/64 caps correctly.

Evidence: `examples/scratch-select.rs` ran 60 requests (14 corpus/generated
inputs × convert-webp t80/90/95 Fast, plus auto and Balanced-webp on
coffee/grass/large) against the heuristic build (`add6519`) and the repaired
tree — winning format, bytes, score and `unchanged` identical on all 60; only
evaluation order/refinement paths differ, which the restored schedule intends.
New unit tests cover `better` tie order and `refine_step` straddle/gap/center
choices; integration tests pin exact-candidate-first ordering and a deep
lossless win where no lossy candidate qualifies. `cargo test --locked
--all-features --test benchmark_verify` and the optimizer/webp_codec/
color_fidelity/codec_boundary/cli suites are green.

Restored wall cost (serial, `RAYON_NUM_THREADS=1`, release): grass t90 Fast
convert 1.51 s → 11.95 s with byte-identical 112,946 B output — the price of
the exact-selection guarantee on the calibrated 200–400K px band, matching the
pre-`a0e5a30` ~12.9 s baseline. Raw comparison records: `.work/tmp/select-
{before,after}.json` (disposable scratch, not committed evidence).

Next per the brief: the representative reproducible baseline (P0 corpus +
two measurement lanes), then structural repeated-work reductions. The
workflow-speed gate now measures the exact selector, so fresh timed captures
must use this repaired schedule.

## Checkpoint — 2026-11-15 (representative untimed baseline landed)

Harness repairs (`1a90dc1`, `c23125f`, `c1781ab`, `3d690ec`): every tool
executable is SHA-256'd into `toolSha256`; `rustc`, `cargoProfile` and
`rayonThreads` are recorded; `cwebp-lossless` (`-lossless -z 9 -exact
-metadata all`) is a distinct tool whose records carry `quality: null`;
JPEG-source lossless records verify in the libjpeg/libwebp domain
(`djpeg`/`dwebp` PNM raster equality) because img's JPEG decoder and
libjpeg legitimately differ by ±2 per channel — PNG sources keep canonical
`StillImage` sample equality, and `verify_saved` applies the same domain
rule. Benchmark scratch directories are pid-scoped after a concurrent
capture read another run's in-flight `output.webp`.

Validation corpus (`benchmarks/corpus/validation/manifest.json`): 34 pinned
inputs — 20 licensed photographs (CC0/public-domain, skimage + Wikimedia
Commons, hashes verified), real UI/text/alpha assets, and 12 deterministic
boundary fixtures straddling 16,384 / 200,000 / 400,000 / 1,048,576 pixels
plus odd dimensions and a 13.7 MP workload. Every entry records file,
dimensions, SHA-256, license, author, source and evidence URLs.

Untimed captures at `773dfd3` (repaired selector, `dist` profile, one
Rayon thread), both verified by `benchmark_verify`:

- [Dev corpus](../benchmarks/results/untimed-repaired-2026-11-15.json):
  280 records, 140 completed. astronaut.png refuses scored processing on
  every tool (SDR gamut); img-png refuses coffee.png on unsafe-to-copy PNG
  metadata; original mozjpeg/oxipng/ravif binaries are absent in this
  checkout (explicit gap, not a pass).
- [Validation corpus](../benchmarks/results/untimed-validation-2026-11-15.json):
  680 records, 321 completed.

Untimed frontier observations (bytes at matched minimum score; lossless
records are exact-sample, score null):

- Where img completes lossy, it wins the size frontier at every matched
  target on both strata — e.g. dev coffee t80 53,874 B vs cwebp 87,502 B
  (score 85.8) and withdrawn 93,198 B (score 85.6); validation utrecht
  t80 741,644 B vs cwebp 976,436 B and withdrawn 1,065,772 B; retina t80
  110,648 B vs 184,610/213,876 B.
- Lossless bytes trail libwebp `-z9` on several strata: dev coffee t90/95
  img 343,922 B vs cwebp-lossless 336,030 B (+2.3%); validation
  mm-logo-alpha 33,566 B vs 23,696 B (+42%), photopea-ui 32,834 B vs
  28,792 B (+14%), utrecht 4,093,780 B vs 3,903,928 B (+4.9%). img beats
  cwebp-lossless on dev grass/gravel and validation retina. The alpha/UI
  and >400K-px bands encode at codec level 1 — the lossless gap concentrates
  exactly where `fast_lossless_effort` does not reach level 6.
- **Coverage gap (product policy, not encoder):** the scored path refuses
  most saturated ICC JPEGs — "color transform leaves the SDR sRGB gamut"
  via `checked_sdr` in `StillImage::prepare` — on 10 of 15 validation
  photographs plus chelsea.png; `--lossless` still encodes them. Only
  cwebp-lossless completes on marmolada/piraeus/portrait-woman. Recorded
  as an honest limitation; relaxing `checked_sdr` is a color-fidelity
  contract decision, not a benchmark fix.

Timed five-round capture at `773dfd3`
([timed-repaired](../benchmarks/results/timed-repaired-2026-11-15.json)):
accepted host gate, one Rayon thread, 1400 records / 700 completed.
Workflow walls vs the withdrawn oracle (median ms, img vs withdrawn):

| Case / minimum target | img | Withdrawn | Ratio |
| --- | ---: | ---: | ---: |
| coffee / 80 | 9,977 | 342 | 29.2x |
| coffee / 95 | 9,929 | 312 | 31.8x |
| grass / 90 | 11,773 | 442 | 26.6x |
| gravel / 90 | 15,032 | 432 | 34.8x |
| large / 80 | 25,579 | 2,161 | 11.8x |
| ui / 80 | 376 | 21 | 18.3x |

The photographic band roughly doubled versus the heuristic build (grass
t90 was 1,789 ms) because the exact selector now pays the deep lossless
candidate (~8 s per the L6 band note) on every scored request; coffee and
large were already deep-search-bound. Wall time remains the dominant
measured gap: ~12–35x on photos, ~18x on tiny UI, ~12x on the 2 MP
fixture. Next per the brief: profile the repaired pipeline and attack the
repeated transform/parse/trellis work structurally.

## Checkpoint — 2026-11-16 (VP8L transform pre-score prune)

L6 work counters on 512x512 photographs showed ~26 transform tasks each
running a full greedy/optimal parse plus entropy plan; transform families
predominated by near-duplicate predictor/cross-color compositions. The
encode now pre-scores every task's transformed buffers with an order-0
byte-plane entropy estimate (`stream_entropy`, ~O(pixels) per spec, no
LZ77/DP work) and keeps a deterministic subset before any
`family_search` runs: `Floor`/`Palette` unconditionally, plus per-family
selection — each family's est-best task always runs, near-ties within
`FAMILY_BAND` (6%) of the family best and `GLOBAL_BAND` (2.5x) of the
global best are kept up to `FAMILY_CAP` (4). Estimation itself parallelizes
under `rayon`; the keep mask is computed before the parallel fold so
serial/rayon output stays byte-identical (pinned by
`serial_and_parallel_evaluation_agree`).

This is an **encoder search/RD change, not equivalent work**: the entropy
estimate is a heuristic ranking, not a bound, and can in principle discard
the true byte winner. The family-best hedge exists because order-0 entropy
cannot see LZ77 match structure — a pure-predictor residual can
out-encode every cross-color composition while scoring slightly worse
(observed: `generated_transparent` 96x64, where naive global top-8 pruning
regressed 110 -> 114 B and family-aware selection restored 110 B).

Evidence so far (serial `work-count` scratch builds, not product rayon
timings): grass L6 10.8 s -> 7.4 s, coffee L6 9.2 s -> 5.6 s; kept-task
count ~13 of ~46 on grass. Byte sweep vs the pre-change HEAD worktree:
24 diverse inputs (dev photos, generated synthetics incl. flat/text/ui/
transparent, validation PNGs incl. boundary sizes, alpha logo, UI
screenshot) all byte-identical at L6; L1 and L9 byte-identical on
grass/coffee. `cargo test --locked --features work-count` (918 tests)
green, including `transform_selection_keeps_family_best_and_respects_bands`
and the rayon agreement test. Byte-identity on this sweep is evidence,
not a proof — broader dev/validation frontier recapture is required
before claiming the change is frontier-neutral, and per-image byte
regressions remain possible on untested strata.

Follow-up (same day): `CrossColorCache` now memoizes `cross_color::forward`
output by (composition, predictor bits, cross-color bits) across the
estimate and kept-task search passes — previously each surviving task
re-ran the full multiplier sweep. Bounded by the same pixel cap as
`PlaneCache`, `Mutex` under `std` / `RefCell` under alloc-only, inline
fallback beyond the cap. The order-0 pre-score moved from `f64`/`log2`
to fixed-point `shannon_bits` so the path compiles without `std`, and a
missing `alloc::vec::Vec` import in `lossy/trellis.rs` was repaired.
A tile-row-parallel `cross_color::forward` was tried and reverted: it
regressed coffee ~7% under nested rayon while the serial forward inside
the task-parallel evaluator already saturates cores. Gated A/B
(`perf-gate-ready` 0 before and after, 3 interleaved rounds, rayon
`work-count` scratch): coffee L6 7.4 s -> 6.6 s median (~11% faster,
consistent all rounds); grass L6 14.3 s -> 14.7 s median (~3% slower,
consistent but under the 5% regression alarm — eval/search-bound
stratum pays the cache's clone traffic while estimate-bound strata
gain). Verification: 922 lib tests +
doc tests green, alloc-only check green, serial/rayon byte-identical,
and 24/24 sweep images (dev + validation, L6) byte-identical vs the
pre-cache HEAD.

Phase instrumentation (temporary `Instant` probes, since removed;
rayon `work-count` scratch build, busy host — directional only): at L6
the estimate phase and the kept-task search phase split roughly
evenly but asymmetrically — grass 512x512: est ~5.8 s / eval ~8.9 s
(kept 13/25 tasks); coffee 600x400: est ~5.4 s / eval ~1.0 s (kept
10/25). The estimate phase is dominated by the ~36 distinct
`cross_color::forward` jobs the task set implies (each costs
~pixels x 256 multiplier evals x ~7 sweeps, roughly independent of
`bits`); `CrossColorCache` already makes each run once, so eval pays
only searches. Consequence for the next lever: a cheaper estimate-only
forward does NOT pay — kept tasks still need the real forward in eval,
so total forward work is ~conserved. Real options are (a) prune
candidate tasks BEFORE any cross-color forward using a signal that does
not need the transformed pixels (the current order-0 entropy needs them
by construction — a pre-forward statistic such as inter-channel
correlation or predictor-residual entropy would have to rank
cross-color families, which is exactly where order-0 is blindest), or
(b) make the multiplier sweep itself cheaper — it is the largest single
kernel (`cross_color_eval` work counter) and its inner loops are
integer-only and vectorization-friendly, though the campaign brief
deprioritizes scalar kernels vs architecture. Within one spec the
pipeline is already deduplicated: `parse_lz77` runs once and its hash
chain feeds `parse_optimal`; meta-Huffman shots reuse tokens.
Multiplier-sweep vectorization checked and closed: the released binary
already emits NEON `mul.4s`/vector ops inside `cross_color::forward`,
so LLVM autovectorized the 256-eval inner loops — scalar/kernel work
there is not the remaining lever. Lossy side profiled for contrast
(coffee q90 193 ms / q100 268 ms, `trellis_eval` top counter): lossy
is ~30x faster than lossless on the same input, so the frontier gap
stays lossless-side.

Two-phase estimate LANDED (`1abcc0c`, implemented slightly
differently than sketched — the coarse phase cuts multiplier-sweep
evals rather than subsampling pixels): `cross_color::forward_estimate`
runs `forward_impl` with `mult_step = 4` (every fourth `i8`
candidate) and `refine_rounds = 1` (initial blue sweeps only) —
`forward` itself is `forward_impl(.., 1, MAX_CC_REFINE_ROUNDS)`, so
the exact path is unchanged. `estimate_task` takes `cheap_cc`; cheap
specs call `forward_estimate` directly and never touch
`CrossColorCache`. `select_provisional` keeps Floor/Palette plus each
family's `PROVISIONAL_CAP = 8` best cheap estimates; the real pass
runs `estimate_task(.., cheap_cc = false)` only for those, and
`select_from_estimates` then applies the unchanged real-estimate
bands. `provisional_prunes` skips the coarse pass entirely when no
family exceeds the cap (levels 0-2), so narrow breadths estimate
exactly as before. Gated A/B (`perf-gate-ready` 0 before, 5
interleaved rounds, `work-count` scratch, host busy again at the
post-check from the measurement's own rayon load): coffee L6
6.61 s -> 3.97 s median (~40% faster, consistent all rounds); grass
L6 14.43 s -> 11.06 s (~23% faster). Bytes: 24/24 corpus images at L6
plus grass/coffee/astronaut/generated_transparent at L1 and L9 all
byte-identical vs `bc9140a`; serial/rayon byte-identical; 922 lib
tests + doc tests + alloc-only check green.

Spec-level pruning LANDED (`dda39c0`): the real-estimate pass already
scores every BufferSpec's order-0 entropy but kept only the minimum —
while `run_best_task` searched ALL of a kept task's specs (each a full
`family_search`: literal parse, greedy LZ77, optimal-parse DP,
meta-Huffman shots). Per-spec dump on the corpus showed sibling specs
split bimodally: predictor and cross-color with/without-subtract-green
pairs sit 8-200% apart in entropy (clear losers), while the
predictor+cross-color SG choice ties within ~0.5% (must keep both —
LZ77 structure, not order-0 entropy, decides those). `estimate_specs`
returns the per-spec vector; `spec_keep_masks` keeps every spec within
~5% of the task min (`est * 20 <= min * 21`, ties always survive);
`run_best_task` filters specs through the mask before `family_search`.
Gated A/B vs `1abcc0c` (pre-gate 0, 5 interleaved rounds): coffee L6
4.05 s -> 3.76 s (~7%), grass L6 11.00 s -> 9.03 s (~18%). Cumulative
vs pre-two-phase `bc9140a`: coffee 6.61 -> 3.76 s (~43%), grass
14.43 -> 9.03 s (~37%). Bytes: 24/24 L6 + L1/L9 spot checks
byte-identical; serial/rayon identical; 922 lib + doc + alloc-only
green.

Match-coverage estimate experiment REJECTED (uncommitted): sampled
LZ77 pair coverage (direct-mapped 64K-slot table, stride 4, 16K window)
discounted order-0 entropy as `est * (1024 - cov) / 1024` to model
backreference redundancy order-0 entropy cannot see. Offline replay on
four corpus dumps showed it reorders near-tied predictor+cross-color
specs by up to ~3% — enough to push the true byte winner past
`FAMILY_CAP` on `validation_chelsea` (+164 B, ~0.1%) even with the
family-best band removed. The discount is too coarse at the fine
grain where kept candidates actually differ (<0.5%), so it was
reverted to raw entropy. Coverage remains a plausible ORDERING signal
only if used at coarse granularity (family standing), never inside the
per-spec ordering the cap depends on.

Family-best global band LANDED (`15562c5`): `select_from_estimates`
kept each family's entropy-best task unconditionally as the hedge
against order-0 entropy missing LZ77 structure — but on the corpus
that best never won from >~1.5x the global best estimate
(cross-color-alone families sit at 1.13-1.48x and always lose by
12-46% bytes). The band `best_est * 2 <= global_best * 3` now gates
the family best, but ONLY on wide task sets (`provisional_prunes`):
narrow levels keep the unconditional hedge because their coarse
task sets produced a real regression — `generated_transparent` L1
went 198 -> 228 B (+15%) under the ungated band. Offline replay:
coffee kept tasks 9 -> 7, grass 12 -> 11, astronaut 10 -> 8,
text 12 -> 11, winner kept everywhere. Byte sweep: 24/24 L6 +
L1/L9 spot checks byte-identical vs `dda39c0` (the one L1 diff was
the ungated variant, fixed by the `wide` gate). Gated A/B vs
`dda39c0` (pre-gate 0, `time=` medians of 5 interleaved rounds):
coffee L6 ~3.76 s -> ~3.76 s (neutral — its pruned family bests
were already spec-masked), grass L6 ~9.31 s -> ~8.56 s (~8%
faster).

Greedy-estimate task band REJECTED (uncommitted, reverted): the
libwebp `EncodeImageNoHuffman` analog — score each kept task by its
best spec's greedy-parse token-stream bit estimate, then run
`family_search` only inside a ~3-10% band of the best score. The
signal itself is excellent: offline replay on four corpus dumps put
the byte winner in the top 4 of 8-11 kept tasks at a 3% band, and the
cross-color-alone family trailed by 12-50% (always prunable). The
economics fail: the probe costs a full `parse_lz77` + `RefModel` +
histogram pass per spec — roughly 40-50% of `family_search` — and the
banded specs then re-pay the entire search, so net savings need the
band to cut nearly half of all specs. It does on coffee/astronaut
(~50%) but NOT on tie-heavy content (grass keeps 7/11 genuinely
near-identical tasks), where the probe is pure overhead: grass L6
regressed ~9% even with the probe reduced to a single no-cache
histogram pass. Carrying parse state across the band decision would
fix the double-parse but needs per-spec tokens+chain resident for
every kept task simultaneously — prohibitive on multi-MP images.
Recorded so the next attempt doesn't re-derive it: a viable version
needs a match probe that costs << one greedy parse (subsampled or
capped-chain), or a band applied inside `run_best_task` where spec
count is already small.

Coverage-band task prune LANDED: the cheap-probe economics above are
fixed by replacing the greedy parse with sampled match coverage — a
direct-mapped (key, position) table over pixel pairs at stride 4,
scoring each spec as `entropy * (1024 - coverage) / 1024`. Cost is
~1-3 ms per spec versus ~100-300 ms for `parse_lz77`, so the probe is
noise. Offline replay showed the cross-color-alone family trailing at
1.5-2.4x on this signal on every corpus image while the kept near-tie
cluster sits within ~11%; a 15% band (`v*20 <= min*23`) on wide task
sets therefore removes exactly the CC-alone survivors the entropy
family-best band lets through (entropy ratio 1.48-1.67 vs coverage
ratio 1.5-2.4). Coverage-adjusted estimates are used ONLY for this
coarse task band — never inside `spec_keep_masks` or family ordering,
where the ~3% reordering provably loses byte winners
(`validation_chelsea`, see rejected experiment above). Bytes:
24/24 L6 byte-identical vs `edb6d9d`, L1/L9 spot checks identical,
922 lib + 5 doc tests + alloc-only green, serial/rayon agreement
test updated and passing. Gated A/B vs `edb6d9d` (pre-gate 0,
post-gate 0, `time=` CPU medians of 5 interleaved rounds, all SAME):
coffee L6 ~3.73 s -> ~3.75 s (neutral), grass L6 ~8.93 s -> ~8.25 s
(~8% faster), astronaut L6 ~6.95 s -> ~6.34 s (~9% faster).

Quad-keyed DP chain REJECTED (uncommitted, reverted): lowering the
`build_quad` threshold so the optimal-parse DP walks the
minimum-match-keyed chain on sub-1 MP images too. Under the hop cap
the quad chain reaches strictly more useful candidates, so the DP
input grows — and because the cost model is a heuristic, extra
candidates sometimes win the estimate while losing real bytes:
coffee +2 B, astronaut -14 B, validation_chelsea +24 B at L6. Mixed
and net-negative; any byte-altering DP-input change needs the same
frontier justification as an effort change, and this one doesn't
earn it. Recorded so the next attempt doesn't re-derive it: the
remaining `match_hop` mass is pinned by byte-identity — reducing it
requires either fewer searched specs (the estimate bands are already
at their safe width) or a provably lossless candidate prune inside
`find_candidates`, neither of which has a known construction.

Qualification evidence gathered alongside (untimed, this host):

- Pure-Rust/dependency: `cargo tree` shows webpkit has zero external
  dependencies; `cargo check` passes for `x86_64-unknown-linux-musl`
  and `aarch64-unknown-linux-musl` (both `rayon,work-count` and
  alloc-only feature sets). Host target `aarch64-apple-darwin` is the
  development oracle.
- Decode cost (img decoder, 30 decode reps per file, system libwebp
  `cwebp -lossless -z9` as comparator — not the pinned tool): our
  L6 streams decode within ±25% of libwebp's either direction,
  content-dependent, no systematic penalty — coffee-img 192 ms vs
  cwebp 152 ms, grass 120 vs 129, chelsea 106 vs 117, text 35.5 vs
  30.2 (totals over 30 reps).
- Serial/rayon agreement re-verified on the coverage-band landing:
  alloc-only serial build vs rayon build byte-identical on grass,
  coffee, astronaut, validation_text, validation_chelsea,
  generated_transparent at L6.

Meta-Huffman breadth extension LANDED (this commit): the
`meta_huffman` breadth flag moves from levels 6-9 to levels 2-9.
Schedule arms 2 and 3 gain meta shots; the now-identical `4 | 5`
arm is removed so levels 4-9 share one full-breadth arm. Levels 0-1
unchanged, preserving a fast tier. `meta_shot` self-floors on real
emitted bytes, so output can only shrink for a given search scope;
monotonicity is preserved by construction (higher levels still
search a superset).

Byte evidence — L2 and L3 sweeps over all 24 corpus images, zero
positive deltas: mm-logo-alpha L2 33566 -> 30326 (-9.7%, equal to
former L6 output), validation_camera L2 -13,838 B (-9.3%), coffee
L2/L3 366756 -> 346932 (-19,824 B, -5.4%), validation_chelsea L3
-8,250 B (-5.0%), astronaut L2/L3 ~-10.5 KB (-3.0%), utrecht-meetup
L2/L3 -51,720 B (-1.3%), photopea-ui -566 B, validation_text -280 B,
generated_large -278 B; remaining images unchanged. Several L2/L3
outputs now match former L6 bytes on alpha/UI strata.

Timing cost — gated interleaved A/B vs a fresh pre-change build,
wall-time medians of 5 rounds per level (corrected measurement;
see stale-binary caveat below), all outputs byte-confirmed:
L2 coffee +55%, grass +32%, chelsea +33%, text +18%; L3 coffee
+38%, grass +24%, chelsea +24%, text +12% (medians: L3 coffee
2.50 -> 3.45 s, grass 3.36 -> 4.16 s, chelsea 1.53 -> 1.89 s,
text 1.19 -> 1.33 s). This is a frontier trade, not a free win:
roughly 12-55% more encode time at AUTO-selected levels buys
0.2-9.7% fewer bytes where meta clustering pays. Accepted because (a) the recorded
qualification deficit is lossless bytes trailing pinned libwebp —
coffee vs cwebp-lossless improves from +9.1% to +3.2%, utrecht
+4.9% -> +2.5%, mm-logo-alpha +42% -> +28%; (b) absolute encode
time already trails the oracle by an order of magnitude, so the
marginal ~10% does not change the time story; (c) L0/L1 remain
meta-free for speed-sensitive callers. Cluster/plan counters were
already shown small; the cost is the per-spec meta emit passes.

Verified: 922 lib tests incl. monotone/floor/serial-parity, 5
doctests, alloc-only check, x86_64+aarch64 musl checks, L4=L6
collapse spot-verified. Frontiers vs cwebp-lossless recomputed in
the next qualification pass.

Meta-Huffman at codec level 1 LANDED (this commit): the Fast-effort
lossless path (`fast_lossless_effort`) maps every pixel band outside
<=16,384 px and 200-400K px to `Effort::level(1)`, so the recorded
lossless-vs-libwebp gap concentrated exactly where meta-Huffman was
still off. Level 1 now runs meta shots on the greedy and DP streams;
level 0 (explicit minimum) unchanged. AUTO never selects levels 0-1
(`content_level` floors at 3, `size_cap` at 2), so this affects only
the product Fast tier and explicit `Effort::level(1)` callers.

Byte evidence — L1 sweep over all 25 corpus .rgba fixtures, zero
positive deltas, -115,890 B total: mm-logo-alpha 33566 -> 30326
(-9.7%, equal to L6 bytes), camera -13,838 B (-9.3%), coffee
366756 -> 346932 (-19,824 B, -5.4%), chelsea -8,170 B (-5.0%),
utrecht-meetup -53,640 B (-1.3%), photopea-ui -566 B, clock_motion
-1,704 B, coins -1,760 B, text -280 B, boundary fixtures -374/-1162 B,
plus small gains; generated/grass/gravel near-equal. On coffee,
grass, chelsea and camera the L1+meta output now equals the former
L2/L3+meta bytes — meta was the binding constraint, not tile bits —
so Fast gains roughly a level of byte quality at roughly half of
level 2's encode time.

Timing cost — gated interleaved A/B vs a fresh HEAD build
(`time=` wall medians of 4 rounds, pre-gate 0 / post-gate 0, all
outputs byte-confirmed): coffee 1.24 -> 2.30 s (+85%), grass
1.93 -> 2.75 s (+42%), chelsea 0.74 -> 1.11 s (+50%), text
0.56 -> 0.70 s (+24%), camera 2.22 -> 2.94 s (+33%), utrecht
96.2 -> 104.7 s (+8.9%), mm-logo-alpha 33.7 -> 39.6 s (+17.6%).
Accepted as a frontier trade: absolute Fast-tier times stay far
below levels 2/6 (L1+meta coffee 2.3 s vs L6 ~3.7 s), the largest
relative costs land on sub-3 s encodes where the absolute delta is
~1 s, and the byte gains concentrate on the strata that trail
libwebp. grass is the poor-ratio case (+42% for -0.2%) and is
recorded as such; a content-gated meta shot is a possible future
refinement but adds a heuristic knob for one stratum.

Harness caveat discovered here: the `.work/wt/metabase` binary used
for the earlier L2/L3 meta A/B was a stale artifact (~3.5x slow),
which understated the true time deltas. Re-measured against a
rebuilt 35af988 binary (gate 0 before and after, 5 interleaved
rounds, wall medians): L2 +18-55%, L3 +12-38% across coffee/grass/
chelsea/text — roughly 2-3x the originally recorded +0.9-18.7%.
The keep decision stands (byte gains unchanged, absolute times still
below L6); only the cost figures were wrong. All byte sweeps are
deterministic and unaffected.

Frontier refresh after meta-Huffman at levels 1-9 (untimed captures,
`dist` build at `fc0b421`, `IMG_BENCH_FIXTURES`-filtered recapture of
the pixel bands `fast_lossless_effort` maps to codec level 1 —
(16,384, 200,000) and >400,000 px — plus dev `large.png`):
[validation-l1meta](../benchmarks/results/untimed-validation-l1meta.json),
[dev-l1meta](../benchmarks/results/untimed-dev-l1meta.json). Dev
corpus photographic fixtures are all in the unchanged 200-400K px
level-6 band; `large.png` scored lossy at all targets so no lossless
record moved. On the product path every changed record improved and
none regressed:

- mm-logo-alpha t80/90/95: 33,566 -> 30,326 B; vs cwebp-lossless
  -z9 gap +41.7% -> +28.0%.
- utrecht-meetup t90/95: 4,093,780 -> 4,040,140 B; gap +4.9% -> +3.5%.
- photopea-ui t90/95: 32,834 -> 32,268 B; gap +14.0% -> +12.1%.
- retina.jpg t90/95: 867,854 -> 858,376 B; img's advantage over
  cwebp-lossless widens -4.9% -> -5.9%.
- clock_motion t90/95: 48,096 -> 46,392 B; gap +34.3% -> +29.5%.
- coins t95: 74,784 -> 73,024 B; gap +10.9% -> +8.3%.
- text t95: 43,020 -> 42,740 B; gap +4.1% -> +3.5%.
- boundary-16512/199809/400689/1048575/1048576/1050625 t95: -202 to
  -1,760 B each; ~25% gaps essentially unchanged (synthetic noise
  content gains least from meta clustering).

img-png records are lossless PNG outputs (PNG encoder untouched) and
correctly show zero delta. The scored `img` records' lossless
candidates are the Fast->level-1 WebP path, matching the codec-level
sweep byte-for-byte (mm-logo 30,326, utrecht 4,040,140 vs workledger
4,040,140-equivalent deltas). Remaining lossless-vs-libwebp gaps are
now concentrated in mm-logo-alpha (+28%), clock_motion (+30%),
photopea-ui (+12%), boundary synthetics (~25%) and coins (+8%).

Decode-cost spot check on the meta-Huffman streams (img decoder,
best-of-8 decode wall, `scratch-decode` release): meta streams cost
~3-10% more decode time than the same fixtures' pre-meta L1 streams
— mm-logo-alpha 6.13 -> 6.34 ms, photopea-ui 4.05 -> 4.31 ms,
clock_motion 1.31 -> 1.43 ms, coins 1.35 -> 1.40 ms, utrecht
115.3 -> 126.5 ms — the expected per-block histogram-group lookup.
img still decodes utrecht faster than cwebp-lossless (126.5 vs
135.8 ms) and remains ~1.5x cwebp-lossless on the small alpha/UI
strata, consistent with the earlier ±25% decode finding. Encode
peak-RSS and full decode-matrix refresh remain open.

Qualification status at `286a625` (post level-1 meta landing):

- Product suites green: `benchmark_verify` 2/2 (incl. `verify_saved`
  over the new l1meta result files), `webp_codec` 6/6,
  `color_fidelity` 9/9, `codec_boundary` 8/8, `optimizer` 12/12,
  `cli` 16/16; webpkit 917 lib tests + 5 doctests + alloc-only
  check green (proptests run at default 256 cases — the fuzz
  obligation is partially covered by these plus codec_boundary
  hostile-input tests; a dedicated longer fuzz campaign remains
  open).
- Target matrix: `cargo check` green for `x86_64-unknown-linux-musl`
  and `aarch64-unknown-linux-musl` (rayon,work-count and alloc-only
  feature sets) on current source.
- Decode cost: meta streams +3-10% decode time (recorded above);
  encode peak-RSS recorded above.
- Animated/metadata edge: `animated_webp_is_rejected_before_frame_extraction`
  plus the new `lossless_webp_round_trips_icc_exif_orientation_and_xmp`
  (`c272f8e`) carry ICC/EXIF/XMP/orientation through the product encode
  path and confirm them via `describe` on the output — closing the
  metadata edge refresh at product level.
- Explicit gaps unchanged: mozjpeg/oxipng/ravif originals absent,
  SDR-gamut refusals, img-png unsafe-metadata refusal on coffee.

Encode peak-RSS at level 1 (`/usr/bin/time -l`, workledger-scratch,
meta vs pre-meta HEAD build): coffee 35.0 -> 45.6 MB (+30%),
grass 34.6 -> 37.7 MB (+9%), chelsea 25.5 -> 33.3 MB (+31%),
mm-logo-alpha 375 -> 383 MiB (+2%), utrecht-meetup 1.05 -> 1.26 GiB
(+20%, +226 MB worst case). Meta planning/emit buffers add bounded
scratch on top of the pixel-dominated baseline; absolute deltas are
<=10 MB on sub-1 MP images and <=226 MB on the 8 MP stratum, all
freed at encode end. Recorded as the resource-cost evidence for the
level-1 meta landing.

Fuzz/conformance evidence: `PROPTEST_CASES=1024 cargo test --locked
--release --lib lossless::` at `c272f8e` — 367/367 pass in 21 s,
covering
encode_best round-trips/floors/serial-parity proptests, meta emit
decode-to-source, decode_incr arbitrary/split streams, and
near-lossless bounds. Combined with codec_boundary hostile-input
tests and the 24+25-fixture byte sweeps, this is the bounded
campaign evidence; a longer dedicated fuzz run remains open.

Level-monotonicity repair (this commit): the estimate bands were
gated only on task-set width, so level 3 (20 tasks) pruned while
level 2 (14 tasks) searched everything — generated_transparent
regressed L2 178 B -> L3 386 B. `wide` now also requires
>= PRUNE_MIN_PIXELS (256 Ki px): below that the full wide search is
cheap and small-sample entropy estimates are least reliable. Same-
level bytes can only shrink under the gate (unpruned is a superset);
verified on all 13 sub-256 Ki-px corpus fixtures at L3 and L6 — the
only delta is generated_transparent L3 386 -> 190 B, every other
byte identical. Residual: L3 still emits 190 B vs L2's 178 B because
the deeper optimal-parse chain (MAX_CHAIN_OPTIMAL at level >=3) is
a different candidate, not a superset — a by-design heuristic-depth
effect of a few bytes on tiny files, distinct from the band bug and
recorded here so it is not re-diagnosed as a regression.

Timed frontier recapture at `28073f9` (5 interleaved rounds,
`RAYON_NUM_THREADS=1`, host gate accepted;
[timed-validation-l1meta](../benchmarks/results/timed-validation-l1meta.json),
img vs cwebp-lossless `-z9` medians): the meta-Huffman levels make
the already-large time gap wider on the big alpha/UI strata —
mm-logo-alpha img 179.0 s vs cwebp 4.2 s (42.8x; was ~35x
pre-meta), photopea-ui 16.8 s vs 1.7 s (9.9x), utrecht 112.0 s vs
37.5 s (3.0x), boundary-1050625 10.2 s vs 7.5 s (1.4x), retina
21.2 s vs 15.6 s (1.4x). img remains FASTER than cwebp-lossless on
clock_motion (0.5x), coins (0.3x) and text (0.5x). Byte gaps vs
cwebp narrowed as recorded above; the time story is unchanged in
kind — lossless encode is 1.4-43x the oracle depending on stratum,
with meta adding ~10-20% on the L1 bands. withdrawn produced no
records (binary absent in this checkout — explicit gap).

Harness note: this capture's `text.png` fixture name collides with
the dev corpus's generated `text.png`; the 15 `timed-text_png-img-*`
outputs it overwrote belong to `timed-post-skip.json`. Those were
restored from git and this capture's copies renamed to
`l1meta-timed-text_png-img-*` with `outputPath` rewritten, so both
documents verify. `IMG_BENCH_OUTPUT_PREFIX` exists for exactly this;
future cross-corpus captures should set it.

Meta-Huffman precision sweep: `meta::plan` always emitted the finest
entropy grid `choose_bits` allows.
libwebp tries several histogram precisions; this change emits the chosen
grid plus the two coarser steps, each self-floored by real bytes. A coarser
grid only pays when the fine grid's statistics already collapsed — gated on
`groups * 8 <= entropy_blocks`, so group-rich content (every corpus photo)
skips the extra emits entirely: photopea-ui and coffee work counters are
bit-identical to baseline. Corpus evidence: 44/44 dev+validation fixtures
byte-identical at L6 vs `895948d` (incl. utrecht 4,063,236, mm-logo-alpha
30,326, photopea-ui 32,268); the mechanism wins on regioned synthetic
content (512x512 half-flat/half-noise: 394,270 -> 393,984 B, -286) where a
coarser grid keeps the region split with a smaller entropy image. Also
recorded: codec L9 still trails `cwebp -z9` on photos (camera 130,136 vs
123,720; astronaut 341,490 vs 329,608; coffee 343,922 vs 336,030) — the
residual lossless gap is algorithmic (candidate set / cost model), not
search depth; effort increases buy nothing there.

Meta-Huffman group-cost model: VP8L stream dumps (temporary decoder
diagnostic, since removed) showed our meta streams carry 2-3x the
Huffman groups of `cwebp -z9` on the same images (coffee 38 vs 17,
astronaut 43 vs 17, camera 25 vs 11) because the greedy cluster merge
stops at entropy-of-mixture ties and never charges the ~hundreds of
bits each group's five prefix-code headers actually cost. `ClusterHistogram`
now adds a fixed `META_GROUP_HEADER_BITS = 256` per group inside the
cluster estimate only (cache selection and spec estimates are
untouched); the emitted stream is still self-floored on real bytes.
Constant tuned on coffee/camera/astronaut at L9: 128/384/512 all worse
than 256 on at least one fixture; 1024 regressed all three. Corpus
evidence at L6 vs `f6b278b` (25 fixtures): 15 improved, 10 identical,
2 marginally worse (boundary-16384 +10 B, clock_motion +12 B);
largest wins photopea-ui -598 B (-1.9%), mm-logo-alpha -174 B,
astronaut -368 B, coffee -256 B. L1/L3/L9 spot checks all improved or
matched (clock_motion L9 +12 B is the lone level regression seen).
Groups now land near libwebp's counts (coffee 38->18 vs cwebp 17,
camera 25->12 vs 11, astronaut 43->19 vs 17). Gated interleaved A/B
(`perf-gate-ready` 0, quiet host): encode wall neutral on grass
(9.12->9.12 s) and photopea-ui (+0.7%), +8% on coffee (8.73->9.46 s)
— the coarser-grid gate `groups*8 <= blocks` now passes more often, so
group-rich specs pay up to two extra self-floored meta emits. 368/368
release lossless tests, alloc-only and musl target checks green; the
two meta emit tests and the merge-gate unit test were re-fixtured for
the new economics (256x256 regional images; rich pseudo-random block
histograms). Net: a real but modest frontier gain — the residual gap
to `cwebp -z9` is now mostly transform choice and joint
transform-bitstream scoring, not group count.

Lossless residual-gap probes (all rejected, temporary instrumentation
reverted): (a) forcing EVERY transform task + spec through
`family_search` on coffee L9 still yields 343,646 B — the estimate
bands are NOT dropping the cwebp-style winner; `pred3+xcolor4`+SG
emits 363-366 KB under our pipeline vs cwebp's ~336 KB total, so the
residual ~2.2% is diffuse (LZ77 cost-table details, transform
internals, Huffman subtleties), not candidate coverage. (b) A second
optimal-parse DP iteration seeded by the DP's own histogram
(WEBPKIT_DP2 probe): 343,666 -> 343,662 B, -4 B — the pass-1 cost
model is already converged; iteration is a dead end. (c) Raising
MAX_CHAIN_OPTIMAL 256 -> 1024: coffee L9 byte-identical — photographic
chains exhaust long before the cap, matching the constant's doc.

Scored-path wall analysis (the dominant measured gap, ~12-35x vs the
withdrawn oracle on photos): `Fidelity::Score` settings evaluate
`(Webp, None)` = deep lossless as candidate #2 on every request, and
on photographic content it can never win bytes (lossless ~344 KB vs
lossy ~55-200 KB) yet costs ~4-9 s of the ~10 s wall. Proposed next
lever — deferred gated lossless: under Score fidelity, evaluate the
quality grid first and defer `(format, None)` lossless entries;
when reached, run a Fast-effort lossless probe and skip the deep
encode iff `probe_bytes * margin > incumbent_bytes` (margin covering
the worst observed Fast->deep gain, ~0.80 pending corpus calibration).
Preserves the evaluated set and `better()` outcomes except where the
gate provably cannot change the winner; cap-bound 4-format requests
need explicit handling since deferral changes which tail candidates
run. Requires a corpus output-equivalence sweep plus timed A/B before
landing — selection-policy change, not a codec change.

Scored-path deferred lossless gate (landed): `Fidelity::Score`
requests previously ran the deep lossless candidate first on every
encode even when lossy candidates were certain to win on bytes
(coffee t80: 1030 ms lossless eval vs ~250 ms for all nine lossy
evals, winner 53,874 B vs lossless 343,922 B). Exact `(format, None)`
settings are now deferred behind the quality grid whenever the full
initial setting list fits the candidate cap (cap-bound requests keep
the original order, so the evaluated set is unchanged either way).
Before a deferred WebP lossless encode runs, `lossless_cannot_win`
skips it iff (a) local-delta sample entropy >= 3.5 bits/channel —
below that, palette/meta-Huffman structure makes cheap probes
unreliable (large.png L0->L1 = 0.31x, transparent/icc L1->L6 ~0.5x) —
and (b) a codec-level-0 lossless probe exceeds the incumbent by
margin 1.67x (deep/probe observed >= 0.84 on probe-path fixtures;
margin requires >= 0.60). Skipped evaluations record an explicit
failure entry; winners are unchanged whenever the gate is correct.

Evidence: coffee t80 wall 1.51 s -> 0.60 s; utrecht t80 skips the
~6.4 s probe+deep pair (probe itself ~6.4 s on 2 MP — L0 still pays
LZ77+entropy, so savings scale with image size). Corpus output
sweep vs pre-change binary: every completed record byte-identical
(91+ SAME at time of writing, zero DIFF; pre-existing decode
failures identical on both binaries). Balanced/Best/Auto spot checks
byte-identical. Probe overhead is zero on compressible content
(entropy filter) and ~150-6500 ms on photo content where it replaces
a 0.8-14 s deep encode.

Qualification datapoints (scored convert path, coffee t80 fast):
peak RSS 59.6 MB for `img convert` vs 4.3 MB for `cwebp -q 90 -m 4`
— ~14x, expected given per-candidate decode+SSIMULACRA2 references
and rayon task parallelism; recorded for the resource-limits gate,
not yet a regression vs the withdrawn oracle (unmeasured). Scored
converts now decode each candidate once (score_decoded reuses the
validation decode; ea2bbc9). Output-equivalence sweep vs pre-gate
binary: 105+ records, zero DIFF at time of writing; mm-logo-alpha
(4.7 MP, the heaviest fixture) SAME at all three targets.

Finer meta-Huffman entropy grid (landed, codec levels 7-9 only):
stream-structure dumps of `cwebp -z9` output showed libwebp picks a
FINER entropy-image precision than our `choose_bits` — coffee uses
bits=4 (38x25=950 blocks) where our 256-block cap forces bits=5
(19x13=247). `bits_candidates` now prepends one step below
`choose_bits` when the finer grid stays <= 1024 blocks
(`FINE_META_MAX_BLOCKS`, bounding the quadratic cluster merge),
offered to the DP-token meta shot only — the greedy shot keeps the
old window (both-shots variant measured byte-identical on 8
fixtures, so the cheaper form won). Gated to codec levels 7-9:
level 6 is the product Fast-effort band for <=400K-pixel images
and the ~1000-block merge roughly doubles its wall for ~1% bytes.

Bytes (effort best = codec L9, vs HEAD binary, all improved):
coffee 343,666 -> 340,698 (-2,968); astronaut 341,122 -> 336,928
(-4,194); camera 130,034 -> 129,170 (-864); chelsea 161,846 ->
160,512 (-1,334); grass 212,156 -> 211,820; gravel 188,418 ->
188,250; photopea-ui 31,670 -> 31,346; clock_motion 40,642 ->
40,602. Residual vs `cwebp -z9` coffee 336,030: ~1.4% (was ~2.3%).
Fast/Balanced outputs byte-identical to base (fine_meta unreachable
below level 7 except tiny AUTO->9 images where the grid is trivial).
Cost (RAYON_NUM_THREADS=1 wall, effort best): coffee 4.6 -> 10.1 s,
astronaut 4.0 -> 8.4 s, camera 5.9 -> 9.5 s (~1.6-2.2x); the added
work is one ~1000-block greedy_cluster per kept DP meta shot.
918 webpkit lib tests + alloc-only + musl checks green; serial/rayon
agreement preserved (fine plan is deterministic per spec).

Note: the in-flight timed frontier capture overlapped this rebuild
— img records timed after ~06:26 ran the intermediate fine-meta
binary while `binarySha256` records the pre-change build, so that
capture is internally inconsistent and was discarded (timed.json
reverted); recapture after this lands.

## Checkpoint — 2026-11-17 (Fast workflow cost; effort monotonicity repair)

Phase attribution on the scored Fast path (sentinel verbose runs):
the deferred WebP-lossless candidate dominated wall time and usually
lost — grass t80 ~4.9 s of ~5.8 s in one deep lossless encode that the
75 KB lossy winner beat 3x; photopea-ui ~15.5 s of ~16.8 s likewise.
mm-logo-alpha (4.7 MP) spent ~150 s across six lossy AUTO-tier
encodes (~16.6 s each) vs ~33 s lossless. Decode+score overhead is
small (single-decode scoring already landed).

Landed since `71722b6`:

- `395c8a2` perf(webpkit): `search_lz77_variant` now returns
  (plain, meta) from one shared literal/LZ77/DP/cache/emission pass;
  `floor_streams` folds raw and subtract-green once instead of
  searching each representation twice for the meta shots. Byte-stable.
- `6651fad` perf(img): Fast-only lossless floor policy. For 8-bit
  images with sampled local delta entropy >= 2.2 bits/channel, the
  deferred lossless candidate first runs a codec-level-0 probe
  (`still::webp::probe` now returns the probe bytes, reused as the
  candidate or as fallback if the deep encode fails). When
  `probe*4 > incumbent*5` (empirical margin, NOT a proof — renamed
  the old `lossless_cannot_win` gate to `lossless_unlikely_to_win`
  for Balanced/Best, which keep the 3.5-bit/1.67x rule) the probe
  output is kept and the deep Fast encode is skipped. `Evaluation`
  gained `probe_ms` so probe work is counted and reported separately.
  Low-entropy inputs bypass the probe entirely: deep/probe ratios of
  0.31 (large.png) and 0.51 (transparent.png) make any size margin
  unsafe there. Observed deep/probe on probe-path photos: 0.83-1.0.
- `654feae` perf(img): Fast lossy alpha filter search bounded to one
  predictor. mm-logo t80 ~189 s -> ~50.5 s (3.7x) with a
  byte-identical 30,152 B lossless winner; lossy candidates +0.4%
  bytes total. grass t80 ~5.8 s -> ~1.0 s, same 75,220 B winner;
  photopea-ui ~16.8 s -> ~4.9 s, same 22,658 B winner.
- `25382da` perf(img): dropped the photographic 200-400K px band
  that mapped Fast lossless to codec level 6 — coffee gained 0.82%
  bytes for ~2x time, grass was byte-identical for ~1.6x, gravel
  0.09% for ~1.75x. REVERTED by `70a6c27`: the scored-path A/B below
  showed camera.png t99 lossless 130,034 -> 135,518 B (+4.21%),
  over the 3% per-image guardrail, with cell +1.60% and coffee
  +0.82% as further exceptions. Fast keeps codec L6 for <=16,384 px
  and the 200K-400K px band; everything else uses L1.

Fast size-delta A/B (71722b6 vs head, scored path t80/t99, winners
compared): with the mid-band restored every completed row is
byte-identical to base except generated_ui.png (-2 B, an
improvement). The earlier run against the mid-band-removed binary
is the exception list: camera +4.21%, cell +1.60%, coffee +0.82%,
gravel +0.09%, brick +0.01% — all at t99 only, all inside the band.
Missing comparisons (scratch-evaluator SDR-gamut panic, identical
on both binaries, not encoder regressions): astronaut, chelsea,
charolais-cattle, sous-tour-eiffel, rocket, goose-head, and
generated/deep.png (scratch stride assumption). Geomean size
delta on completed rows: ~0%.

- `3700625` perf(webpkit): byte-identical scheduling win. The floor
  family's raw/subtract-green halves now run under `rayon::join`,
  and the floor task joins against the whole estimate+eval pipeline
  (it is never pruned and depends on no estimate output), removing
  one full search from the eval wave. large.png codec L1 ~4.05 s ->
  ~3.65 s, L9 ~10.8 s -> ~9.7 s, byte-identical.

Fast-L1 task-set probe (per-task minima, codec level 1): the four
PredictorCross tasks are load-bearing on photographs — utrecht
2 MP: best PredictorCross 3,990,311 B vs Predictor-alone 4,471,725
B (~10.8% smaller); retina ~9% smaller. On synthetic/UI content they
trail by >1% and never win. Dropping them for Fast is therefore dead
on arrival against the 3%/1% size guardrails; the remaining Fast
lossless cost is ~9 atomic optimal-parse searches plus a ~35%
estimate/spec phase, neither reducible without an authorized
approximation or a byte-identical kernel win.

Monotonicity repair (this commit): the public `Effort::level`
contract promises a higher level never emits a larger file, but two
mechanisms broke it. (a) Levels >=3 parse the optimal DP at
`MAX_CHAIN_OPTIMAL` while levels <=2 use `MAX_CHAIN`; on images under
`DEEP_CHAIN_MIN_PIXELS` the deeper walk is not a candidate superset
(coffee L2->L3 +4 B, coins +14 B). `search_lz77_variant` now also
parses and emits the shallow `MAX_CHAIN` DP (plus its meta shot)
when the deep cap applies to a small image. (b) The fine-meta grid
prepended one entropy precision and an early `plan_at_bits` failure
returned before the base candidates ran; the finer step is now
isolated so base candidates keep their exact prior evaluation.
(c) `select_from_estimates` family bands key thresholds to the task
set's own minimum, which tightens as higher levels add tile sizes —
`generated_transparent` L2->L3 regressed 178 -> 190 B even with the
chain fix (verified by disabling the bands: L3 emitted 176 B). Below
`PRUNE_MIN_PIXELS` (256K px) `select_keep` now unions the current
level's band selection with the selection each coarser level would
have produced — estimates are level-independent, so this is cheap
and makes the keep set a superset by construction. Above the
threshold the banded path is unchanged; a residual monotonicity
exposure remains there in principle (no violation observed on the
swept corpus: coffee, coins, camera, grass, gravel, clock_motion,
text, transparent, large, ui, brick, cell, astronaut, chelsea, horse,
microaneurysms, boundary-200704/399424 — all monotone at levels
1/2/3/4/7/9). Keep-all was tried first and rejected: it made the
encode proptests exceed 10 min and would have slowed Fast's level-6
small-image path ~4x.

Cost of the monotonicity repair: extra kept tasks only on <256K px
images at levels >=2 (union of coarser-level selections); coffee L9
~6.5 s -> ~8.3 s, transparent L9 ~0.34 s -> ~0.28 s (noise). Fast
(level 1, or 6 on <=16K px) is essentially unaffected — level-1 task
sets are already fully kept by the bands.

Verification: 917-test webpkit lib suite green (incl. byte-stability,
`encode_best_at_is_monotone_and_floored`, serial/rayon agreement);
optimizer integration tests green. Fast-path sentinel A/B vs the
pre-change binary preserved winners byte-for-byte on grass,
photopea-ui, mm-logo-alpha.

Next priorities: (1) recapture the timed frontier now that the Fast
path changed materially, including peak RSS; (2) decide whether the
>=256K px banded selection needs the same superset treatment or a
documented residual risk; (3) Fast lossless on >400K px inputs is
bounded below by ~9 atomic optimal-parse searches — the only
remaining levers are an authorized Fast-only approximation (needs a
codec breadth knob; PredictorCross is load-bearing at ~10% on
photos so candidate cuts are not free) or a byte-identical kernel
win inside `parse_optimal`/the cross-color sweeps (NEON sweep is the
largest single kernel target; the crate's single-unsafe-module grant
currently covers only the match-length kernel).

>=256K px banded-selection repair (`d1044a8`): the residual monotonicity
exposure recorded above was real — once the per-task instrumented
sweep reached the validation photos, utrecht-meetup regressed L1->L2 by
+7,608 B (3,990,332 -> 3,997,940), retina L1->L2 by +1,834 B, and
goose-head L3->L4 by +5,326 B: adding tile sizes tightened
`select_from_estimates`' family bands enough to drop a task the lower
level had searched and won with. `evaluate_best_tasks` (serial and
rayon) now computes one shared estimate pass — per-spec entropy plus
coverage-adjusted scores inside the same `task_specs` materialization —
and `final_keep` simulates each distinct breadth's full band pipeline
(provisional cap, estimate bands, spec masks, coverage band) from that
level-invariant data, unioning the results. The searched set at any
level is a superset of every lower level's by construction; sub-256K
behavior is the existing coarser-level union, unchanged. Verified: the
three violations now emit non-increasing bytes (utrecht L1=L2=
3,990,332; retina 856,644 flat to L2; goose L3=L4=1,744,160), 378
lossless tests green including serial/rayon agreement and the
monotonicity proptest, and coffee/transparent/cell bytes identical.
Cost: levels >=2 on >=256K px images can search a few tasks their own
bands would have dropped (bounded by the union); measured on
photopea-ui L4 ~5.4 s -> ~8.0 s (+47%, bytes identical at 31,670) —
the price of the superset guarantee, concentrated at mid levels; L1
and L9 were flat on large.png/photopea-ui and camera L1/L6 bytes are
identical pre/post. Fast's L1 path is unaffected in kind — L0's three
tasks are a subset of L1's and were already kept there on the swept
photos.

Fast-gate probe analysis (negative result + one opening): per-task
floor mins on the >=1 MP validation photos show floor-stream bits/pixel
cannot gate the transform search — photopea-ui floor wins outright at
0.14 bpp while mm-logo-alpha sits at 0.053 bpp yet a transform still
wins by 4.3% (31,448 -> 30,152 B), so any floor-size threshold either
keeps photopea's ~15 s of doomed transform searches or drops
mm-logo's winner. The product-level L0 probe is more promising: codec
L0 bytes are an upper bound on L1 (L1's candidate set contains L0's),
and the observed deep/L0 ratio is ~0.83-1.0 on photographs but
0.31-0.51 on low-entropy synthetics — which is exactly why the
entropy < 2.2 bypass exists. photopea-ui (entropy 1.26) is the case
the bypass protects yet also the case where the probe margin
(32,834 vs ~22.6 KB incumbent -> skip) would have saved ~15 s; a
correct refinement needs the deep/probe ratio modeled against a
better cheap signal than raw entropy, validated on the reserved
unseen subset before promotion.

Timed frontier recapture at `fdeeaca`
([timed-fast-path](../benchmarks/results/timed-fast-path.json), img vs
cwebp, dev corpus, `RAYON_NUM_THREADS=1`, gate accepted, 5 interleaved
rounds x 3 targets): the accumulated Fast-path work is now visible in
wall time. img medians vs the `2d5c8e0` capture
([timed-post-skip](../benchmarks/results/timed-post-skip.json)): large.png
33.6 s -> 17.2 s (1.96x, bytes 4,674 -> 4,362, -6.7%), ui.png and
compressed.webp ~0.42 s -> ~0.16 s (2.65x, 68 -> 66 B), text/line/
transparent ~2.1x, flat/gray ~1.4-1.8x, coffee t90/t95 1.8x/1.4x with
bytes -0.07%, grass/gravel t80-t90 ~1.3x. One regression cell:
grass.png t95 ran the deep lossless to a 47 KB loss because the probe
compared against a pre-refine incumbent (see below); all other img
cells improved or held. img remains 3-20x slower than cwebp in wall
time on every completed cell but produces far smaller scored outputs
(-6% to -97% bytes vs cwebp's quality grid); the gap is now dominated
by the lossy candidate grid and unavoidable deep lossless on
low-entropy winners, not by skipped-work waste. Peak RSS on
large.png scored Fast: 683.7 MB -> 691.1 MB (+1.1%, union-selection
estimate state). Evaluator-limited cells unchanged: astronaut/icc
(SDR-gamut refusal, both tools), deep.png (high-depth scored refusal),
cwebp quality-grid failures on coffee t90/t95, ui, large t90/t95,
compressed.webp.

Deferred-lossless ordering fix (`4413ea0`): the deferred exact-lossless
candidate was evaluated before the refine bisection finished, so the
floor probe's incumbent was still ~6% high on grass t95 and the deep
encode ran only to lose by 47 KB (~6 s scored-Fast wall time for zero
bytes). Fast now reserves the deferred slots and drains them after
refine exhausts its budget — same evaluation count, same scored set,
only the order moves; Balanced/Best keep the prior order. Byte-identical
outputs verified on coffee/grass/gravel + all generated fixtures and
brick/cell/camera/boundary-399424 at t80/t95 (camera t95 keeps its
130,034 B lossless win — no wrong skip). grass t95 ~2.25 s -> ~0.63 s
parallel. The margin's safety argument is unchanged: skip requires
probe > 1.25x incumbent while deep <= probe always, and observed
deep/probe >= 0.83 on the entropy >= 2.2 stratum leaves ~4% headroom;
the tighter post-refine incumbent spends some of that headroom, which
is why the sentinel A/B above matters.

Recapture at `4413ea0`
([timed-fast-defer](../benchmarks/results/timed-fast-defer.json)): the
grass t95 cell drops 7,957 ms -> 1,430 ms (5.6x vs the fdeeaca
capture, 1.3x vs the 2d5c8e0 baseline) at identical 165,462 B; every
other completed img cell is within noise of the prior capture. Fast
scored workflow vs the campaign baseline is now: ~2x on large.png,
~2.1-2.7x on the tiny structured fixtures, ~1.3x on the
photographic mid-band, with bytes equal or smaller everywhere
completed.

Fast-path phase attribution at `5807820` (scratch-eval + work-count
ledger): the remaining scored-Fast wall time is codec- and
metric-intrinsic, not policy waste. On grass t80 (~1.4 s
single-thread benchmark): ~8 scored lossy candidates at
`webpkit::Effort::AUTO` cost ~30-45 ms encode each (parallel;
~90 ms single-thread on 512x512), the codec-L0 floor probe ~117 ms,
and per-candidate decode (~9 ms) + ssimulacra2 score (~25 ms) ~35 ms
each. Two apparent shortcuts are both dead ends measured this session:
product Fast lossy already maps to codec AUTO because the frame `Fast`
tier emits ~2x the bytes at the same quality setting (grass q75:
130,658 B Fast-tier vs 67,368 B AUTO) — any cheaper tier breaks the
size guardrails outright; and the ssimulacra2 score is the fidelity
contract itself, not reducible work. On large.png the winner is the
deep lossless (~3.6 s encode) which must run to produce the 4,362 B
output. The next real lever is codec throughput — the cross-color
NEON sweep (awaiting the unsafe-grant decision) or AUTO-tier lossy
kernel profiling — not another selection-policy change.

Codec work-count attribution (grass L1, `work-count` feature): the
lossless encode's dominant counters are the LZ77 machinery —
`match_hop` 155M, `resolve_walk` 51M, `match_compare` 19M,
`optimal_dp_step` 3.4M — versus `cross_color_eval` only 15.2M (~5%).
The earlier "cross-color sweep is the largest kernel target" note does
not hold on this stratum; the wall is the per-spec optimal-parse match
search itself, which is already rayon-parallel across specs and NEON-
assisted in the match-length kernel. Combined with the trellis/autovector
analysis above, every remaining Fast cost center is either the fidelity
metric, an already-minimal search tier, or serial-ish match walking —
policy-level Fast work is exhausted; further gains are codec-kernel
efforts gated on the unsafe-grant decision or new algorithmic work.

Lossy codec-vs-libwebp calibration (single-thread, codec level):
product AUTO maps to our top lossy tier, which runs ~1.3-1.7x cwebp
`-m6` wall per encode — coffee q90 83.5 ms vs ~60 ms, astronaut q80
68.8 vs ~40 ms, grass q90 111 vs ~80 ms — while emitting fewer bytes
at the same quality number (coffee 59,772 vs 60,088 B; grass q80
79,660 vs 92,898 B, though fixed-q fidelity is not score-matched).
The per-encode codec gap is therefore modest and already
dominated by the scored-search volume (~8 candidates + decode/score
per target), not by codec throughput; closing the residual ~1.5x
per-encode gap is kernel work (NEON grant) for at most ~25% of a
typical scored wall.

Micro-optimization probes (negative results): a trellis-DP restructure
that hoisted the per-(position, magnitude) distortion term and outgoing
context out of the incoming-context loop measured ~4% SLOWER on grass
q75 AUTO (99.5 ms vs 95.3 ms baseline, bytes identical) — LLVM had
already hoisted the invariant work; reverted. The `find`/
`find_candidates` match-hop loops are already minimal: hoisted
quick-reject guard, chain-walk via `prev[]`, strict-improvement-only
rescans, and byte-invariant candidate sets pinned by proptests — no
redundant work left to remove without changing the search itself.

Lossy tier A/B (rejected): routing scored-Fast lossy through codec
level 5 (Full tier) instead of AUTO halves lossy encode time on
grass (95 -> 42.5 ms at q75) but at matched score the winner grows
+8.0% on coffee t80 (53,874 -> 58,156 B) and +1.2% on grass t80 —
far over the 1% stratum guardrail. AUTO's Best tier on <=1 MP
inputs is load-bearing for output size; reverted.

Decode and mode-decision cost checks: `webpkit::decode` runs 7.9 ms
on a 512x512 lossless stream and 17.6 ms on 2048x1024 — roughly 8-18
ms per scored candidate, small beside the 30-500 ms encodes, so
decode is not a workflow bottleneck. The lossy mode decision is
already two-phase (cheap round-to-nearest ranking, then trellis
re-encode of only the top `WHOLE_BLOCK_TRELLIS_KEEPS`), and
`sse_block` carries a prior measured optimization note — the safe
kernel surface is picked over.

Lossless-size gap localization (L9 vs cwebp -z9): added two
env-gated diagnostic tests (`dump_stream_structure` in
decode.rs, `dump_task_bytes` in encode.rs) that parse a .webp's
VP8L header and report per-task emitted bytes. Correction: the
`.work/tmp/ours9-*.webp` reference files were stale — HEAD emits
camera L9 at 129,170 B (not 130,136), so the real gap vs cwebp
-z9's 123,699 B is 4.4%, not 5.2%. Per-task bytes on camera:
Predictor(5)=129,149 is the winner; Predictor(3)=141,074 — our
predictor-3 stream is far worse than cwebp's predictor-3+meta-4
combination, so the tile-granularity difference does not transfer.
Stream structure: both pick subtract-green + predictor + meta-Huffman
with cache_bits=0; cwebp uses 8x8 predictor tiles and a 1024-block
entropy image (meta bits=4) with 11 Huffman groups, we use 32x32
tiles, 256 blocks (bits=5) and 25 groups. The residual gap is inside
the token/entropy pipeline (LZ77 parse quality or Huffman
clustering/code-length optimization), not transform selection.

Entropy-pipeline follow-up: LZ77 parameters already match libwebp
(1M-pixel window, 4096 max copy, min match 4, deep chain at high
effort) and the Huffman builder is length-limited canonical. The
encode path is single-pass — the optimal-parse DP scores candidates
against one RefModel and never re-parses against the histogram its
own stream implies. libwebp's -z9 path iterates backward-reference
encoding against refreshed histograms; a second parse pass with a
model rebuilt from first-pass tokens is the leading unexplored
hypothesis for the residual ~4.4% camera gap. Untested.

Re-parse iteration landed (levels 7-9 only): the optimal-parse DP
prices matches under the *greedy* stream's histogram, which
systematically misprices the matches an optimal parse prefers.
`Breadth::reparse` re-runs `parse_optimal` seeded by the previous
pass's token stream to a fixed point (cap 4 extra passes, early-out
on token equality; intermediate streams un-emitted, first-pass
stream kept as fallback so a regressing pass can never lose bytes).
Measured at codec L9 (serial workledger): camera 129,170 -> 124,978 B
(-3.2%, vs cwebp -z9 123,699 the gap is now ~1.0%), coffee
343,922 -> 340,688 B (-0.94%), grass 212,156 -> 211,820 B (-0.16%),
large.png (2 MP) -> 3,984 B. Cost: ~2.4x L9 encode wall on camera
(19.1 -> 45.7 s serial); the flag lives only in the level 7-9
breadth arm, and `auto_level`'s size cap keeps AUTO/Balanced at <=6
and Fast at 1/6, so scored Fast latency is untouched. 370 lossless
tests + 87 img lib tests pass, including round-trip, monotonicity,
and serial/rayon agreement proptests.

Reparse generalization at codec L9 (serial): astronaut 338,320 ->
333,736 B (-1.35%), chelsea -> 154,234 B, coins -> 70,356 B — the
gain is consistent across photographic content, largest where the
greedy histogram misprices the DP's preferred matches most.

Group-aware DP cost model (rejected): priced each DP position under
the CostModel of the meta-Huffman group its block belongs to, seeded
by the converged reparse stream's MetaPlan (copy symbols bill to the
start block, matching the decoder's select_group rule — exact, not
approximate). Codec L9 bytes: camera -226 B (-0.18%), astronaut
-28 B, coins -8 B, and identical output on grass/coffee/chelsea —
but the extra pass plus per-stream emit/meta_shot ran inside every
optimal_parse task, costing +40-90% L9 wall (grass 15.3 -> 22.6 s
after a token-equality emit skip, coffee 26.1 -> 43.6 s, camera
45.7 -> 60.6 s even with the grouped pass capped at MAX_CHAIN).
Sub-0.2% maximum gain for ~+50% deep-level latency fails the same
trade the Fast guardrails encode; reverted uncommitted.

Landed (`538e343`): ported libwebp's `OptimizeHuffmanForRle` count
perturbation — near-equal count strides collapse to their mean so the
built lengths run-length-encode smaller. Unlike libwebp (which always
uses the perturbed counts), both assignments are priced exactly as
pixel bits + serialized header bits per channel and the cheaper kept,
so it is self-floored. Codec L9: astronaut -410 B, coffee -314 B,
camera -154 B, coins -130 B, chelsea -62 B, grass -50 B; Fast levels
also improve (coffee L1 -386 B, photopea-ui -140 B, grass -38 B).
380 lossless tests green; byte-stability goldens unchanged (fixtures
don't trigger the variant).

Landed (`ed616e7`): Huffman header serialization now emits libwebp's
trimmed max-symbol field — trailing all-zero code lengths are omitted
through the decoder's token countdown, with both serializations costed
exactly per code so a header can never grow. Corpus deltas are small
but strictly non-positive (L9: camera -10 B, coffee -14 B, astronaut
-18 B, grass -6 B; L1/L6 spot checks similar). Verified by independent
`dwebp` decode; 380 lossless + 87 img tests green, goldens updated.

Meta group header-cost retune (rejected): META_GROUP_HEADER_BITS
256 -> 512 made clustering merge more aggressively — camera's
emitted stream drops 18 -> 15 Huffman groups and its table section
shrinks 13,202 -> 12,010 bits (-149 B), but pixel data grows ~+93 B
back, netting only -56 B at codec L9 (coffee -50 B, astronaut
-18 B). The constant is shared with levels 1-6, where a 13-image
x L1/L6 codec sweep showed net regression: mm-logo-opaque +172 B
(+1.17%), mm-logo-alpha +112 B (+0.37%), clock_motion +104 B, with
only photopea-ui improving (-184 B, -0.58%). 1024 and 768 were
worse still at L9. Net-negative on the product path; reverted. The
emitted-bytes selection already protects each level internally —
the loss is that every level's candidate *plans* shift, not that a
worse plan wins.

Reparse pass cap and chain-depth probes (rejected): raising
REPARSE_MAX_PASSES 4 -> 8 bought camera L9 only -10 B (124,814 ->
124,804) for up to +4 extra optimal parses — the fixed point
converges within the cap on the corpus; reverted. Doubling
MAX_CHAIN_OPTIMAL 256 -> 512 made camera L9 *worse* (+34 B,
124,814 -> 124,848) at higher wall — the DP minimizes its cost
model, not real bytes, so extra candidates can shift the parse onto
a model-cheaper/byte-larger stream; reverted. img lib suite re-run
green (87/87) with the RLE-counts change in place; scored-path size
is provably non-increasing since each channel keeps the cheaper of
direct vs perturbed assignments and selection compares real bytes.

AUTO-path reparse (`encode_best_auto`): AUTO's size cap tops out at
level 6, below the explicit reparse band, so the iterated parse never
ran on mid-size images — the band where AUTO parks most photographs
(levels 3-4 on the corpus). Codec probes at levels 3-4 showed the
gain is concentrated, not uniform: camera -2.5 to -2.9%, coins -0.5%,
coffee/gravel ~0, astronaut/chelsea/grass identical (the fixed point
converges immediately). AUTO now adds `reparse` at resolved levels
4-6 while the image stays under 256K pixels — the same envelope the
explicit level-6 Fast band already pays; larger AUTO levels keep the
single-pass parse and explicit `Effort::level` is untouched, so the
scored Fast path is unaffected. Verified end-to-end (`img convert
--lossless`, default Balanced): camera.png 129,852 -> 126,046 B
(-2.9%), clock_motion.png 40,642 -> 36,714 B (-9.7%), all other
corpus fixtures byte-identical; dwebp decodes clean; 920 webpkit +
87 img tests green.

Sub-image LZ77 (rejected, root cause suspected): emit_subimage coded
transform tile grids, palette maps and the meta-Huffman entropy image
as literals only (`parse(pixels, false)`). Enabling the greedy LZ77
parse shrank camera/astronaut/coffee/chelsea at L9 (-12 to -116 B) but
regressed grass +134 B and coins +40 B. A self-floored pick comparing
literal vs LZ77 token streams by an exact-cost model
(`subimage_coded_bits`: per-channel count*length + serialized_bits +
histogram.extra_bits) still chose LZ77 everywhere and produced
byte-identical output to the unconditional version — so the model
under-prices copies. Suspect: `Histogram::extra_bits` accumulates
`prefix_encode(plane_code)` extra-bit counts for distances, which may
not equal the extra bits `resolve` actually emits; the main stream
never notices because selection compares real bytes, but the sub-image
pick trusted the model. Reverted uncommitted; retry needs the emitted
extra-bit counts threaded through CopyCost or a real-byte A/B emit.

Sub-image LZ77 landed (`3c95f6a`): the earlier rejection's suspect was
right that a histogram-cost floor could not see the truth, so the pick
now emits both token streams into scratch BitWriters and keeps the
smaller by real bit count (`BitWriter::bit_len`). L9 deltas vs the
literal-only baseline: camera -134 B, coffee -82 B, astronaut -52 B,
chelsea -18 B; grass and coins hold at zero (the floor protects them).
Byte-stability goldens unchanged — the synthetic fixtures' sub-images
never prefer copies. Cost is ~+7% L9 wall from the extra emissions;
all levels benefit where sub-images run. dwebp verified.

Timed Fast recapture at `3c95f6a`+ (`timed-fast-rle.json`, img-only,
5 fixtures x 3 targets x 5 rounds, gate accepted, RAYON=1) vs
`timed-fast-path.json`: all cells completed; medians within +1-6%
wall (coffee +4.6%, transparent +5.5%, ui +1-2%, large +2%) except
grass t95 -81.5% (the deferred-lossless ordering fix, expected).
The residual +2-6% is the sub-image A/B emission cost on lossless
candidates; `a76bc65` skips both scratch emits when the greedy parse
finds no copies (byte-identical, coffee L6 back to ~7.6 s serial).
Scored bytes are non-increasing by construction (real-byte floors
throughout); absolute deltas are ~10-400 ms per cell.

Sub-image optimal parse (`83629e7`): the A/B now also tries
`parse_optimal` (MAX_CHAIN) next to literals and greedy LZ77, still
floored on real emitted bits. L9 deltas vs the greedy-only floor:
astronaut -18 B, camera/coffee/chelsea -6 B, coins -2 B, grass
unchanged; ~+3% L9 wall. 370 lossless + 87 img tests green; dwebp
verified.

Exact-cost cache-size re-rank (rejected): `best_cache_bits` picked the
color-cache size by the Shannon `estimate_bits` alone, and UI/alpha
content showed the estimate can mispick badly — an exact-cost variant
(real Huffman lengths + serialized header bits per channel, top-3
estimate shortlist, emitted as a floored alternate alongside the
estimate pick) recovered photopea-ui -468 B (-1.5%) and tworegion
-18/-10 B at L1/L6 with every other fixture byte-identical and zero
regressions. But the code-length builds cost ~+20-33% codec wall on
Fast lossless (coffee L1 2.57 -> 3.15-3.4 s): the estimate top-2 are
near-tied on essentially every model, so no cheap gate (estimate-gap
slack, winner>0) discriminated the paying cases from the waste, and
cheaper cost models (pixel-bits only, direct lengths, single
challenger) still cost ~+20%. Rejected and reverted uncommitted —
the estimate pick stands; a future retry needs a fundamentally
cheaper exact signal or a narrower call site.

Runner-up variant (also rejected): emitting the estimate runner-up
cache size as a floored alternate costs nothing to rank but every
observed win flowed through the meta-Huffman path — plain-only
alternates produced zero byte deltas on coffee/photopea while meta
alternates doubled `meta_shot` planning for +43% codec wall on
coffee L1 (2.57 -> 3.68 s). Reverted uncommitted.

AUTO fine-meta (rejected on breadth grounds): enabling
`Breadth::fine_meta` alongside the AUTO reparse band (resolved
levels 4-6, <=256K px) reaches explicit-L9 sizes on the default
lossless path — camera AUTO 126,024 -> 124,674 B (-1.07%),
astronaut 337,488 -> 333,238 B (-1.26%), chelsea -0.85%,
clock_motion -0.35%, coins/grass/coffee unchanged — at ~+36%
encode wall on camera (35.7 -> 48.7 s serial). Rejected: it is
compression breadth added to the default path while the Fast-cost
problem stands; the numbers are recorded in case an opt-in or a
cheaper finer-grid plan revives it.

libwebp backward-reference mechanisms (source review, not yet ported):
cwebp -z9 tries three LZ77 token streams per image — RLE-only,
hash-chain standard, and a spatial "box" variant restricted to ~32
plane-code window offsets — prices each with
`VP8LHistogramEstimateBits`, keeps the best per cache setting, then
runs `BackwardReferencesHashChainDistanceOnly` + TraceBackwards
once on the winner. Notable differences vs ours: (a) its match
finder walks the hash chain BACKWARD from end-of-image and
left-extends each found match over already-passed pixels;
(b) its DP considers only literal vs the single best chain match
per position but prices distance through `VP8LDistanceToPlaneCode`
(2D-locality buckets) rather than raw distance symbols;
(c) `BackwardReferences2DLocality` rewrites copy distances to plane
codes for histogram/cost purposes; (d) `iter_max` reaches ~86 at
-z9 vs our 64/256 chain caps, window = LZ77_WINDOW_SIZE. Our
staircase DP is stronger per-position but lacks the RLE/box
candidate diversity and the plane-code distance cost model —
candidates for the residual ~0.8% L9 gap if a cheap port pans out.

Timed-capture comparability (measured): `timed-fast-subimg.json`
read ~+57-62% wall on every cell vs `timed-fast-rle.json`, but a
paired A/B on this host (c36530e vs HEAD binaries, interleaved,
RAYON_NUM_THREADS=1) shows ui.png ~0.26-0.28 s and coffee.png
score-90 ~13.7-14.6 s for BOTH binaries — statistically identical.
The delta is host state between capture sessions, not code: the
sub-image LZ77/optimal candidates are wall-neutral at scored Fast.
Cross-session wall comparisons on this machine are unreliable;
latency claims need paired same-session A/B or same-capture deltas.

RLE/previous-row LZ77 candidate (landed, e3a4222): ported libwebp's
`BackwardReferencesRle` as a third token-stream candidate in
`search_lz77_variant` — greedy distance-1/distance-width copies only,
self-floored on real emitted bytes, gated to >=1% copy coverage so
photo variants pay just the O(n) parse. Codec bytes vs prior head:
mm-logo-opaque L6 14,666 -> 11,566 (-21.1%), mm-logo-alpha L1 and
L6 30,038 -> 28,422 (-5.4%), astronaut L9 -498 B, coins L9 -140 B,
chelsea L9 -86 B; every other measured fixture byte-identical.
Interleaved wall A/B (wl-base vs wl-rle2, RAYON_NUM_THREADS=1):
coffee L1 +1-3%, L6 +1-4% codec wall; winners neutral-to-faster.
dwebp decodes the RLE-won mm-logo-alpha stream; 41/41 encode tests
green including byte-stability goldens. This is the first landed
piece of the libwebp backward-reference review; remaining unported
mechanisms: spatial "box" LZ77 variant and plane-code distance
pricing in the DP cost model.

RLE meta-shot probe (rejected, uncommitted): offering
`meta_shot` on the RLE stream when it led the plain candidates bought
mm-logo-opaque L6 another -1,002 B (10,564 total, -28% vs baseline)
and mm-logo-alpha L1 -3,702 B more (24,720, -17.7% total) but cost
+62-74% codec wall on those images and +70% on astronaut L9 for zero
byte gain — the meta-plan merge dominates. Reverted; the plain
coverage-gated RLE candidate (e3a4222) stands.

Sub-image RLE probe (rejected, uncommitted): offering `parse_rle` as a
fourth `emit_subimage` candidate produced byte-identical output on
mm-logo-opaque L6 and mm-logo-alpha L1 while costing +61-65% codec
wall — sub-images are emitted once per transform task per candidate,
so the extra parse+scratch emit multiplies. Reverted; `rle_candidate`
extracted as a shared helper (landed as a pure refactor).

Lz77Box locality variant (assessed, deferred): libwebp's fourth
candidate restricts match search to 32 nearest plane-code offsets
(a +-6 pixel spiral, WINDOW_OFFSETS_SIZE_MAX) using a run-count array
for O(1) length queries. Port cost is bounded but the per-position
offset scan is ~32 first-pixel probes even with early-out — ~30x the
RLE parse's constant —
so it needs a coverage-style gate just to be affordable, and our
plane-code-priced optimal DP already prefers near offsets when they
win on cost. Deferred unless a cheap gate emerges; not implemented.

Lz77Box headroom probe (measured, rejected): sampled near-match
density (any 4-px match at distance 2..64 excluding width, stride 3)
vs RLE coverage across fixtures. photopea-ui 1919x941: rle 98.7%,
near 95.9%; mm-logo-opaque/alpha: rle ~99.7%, near ~98%; tworegion
and grass: 0%/0%; coffee 0.3%/0.1%; astronaut 10.7%/9.5%. On every
fixture the near-match set is essentially the distance-1/width set
RLE already covers, and where RLE fires the hash chain finds the
same repeats plus longer ones — photopea-ui's RLE candidate fired
and still lost on real bytes. The box variant's unique offsets add
no measurable opportunity; not implemented.

RLE-seeded optimal parse (measured, rejected): a probe showed a
single-pass DP seeded from the RLE token histogram beat the
lz77-seeded DP on single-group emits (mm-logo-alpha raw tier
-1265 B, photopea-ui raw -245 B). Wired as a floored second DP
candidate inside `optimal_parse` and measured end-to-end: bytes
were IDENTICAL to HEAD on all 8 cells (mm-logo-alpha L1/L6
28422 B, mm-logo-opaque L1/L6 11566 B, photopea-ui L1/L6 31502 B,
coffee L1/L6 346052/343312 B) — the meta-Huffman shots on the
existing streams already capture every gain the probe saw. The
change only added one full DP walk per family call on run-heavy
images at every level >= 1. Reverted uncommitted.

L9 gap decomposition (measured): decoded both VP8L streams'
token statistics on camera. cwebp -z9 -exact: 244,539 literals +
3,364 copies (17,605 px) at distances {1:2077, 512:1287} only —
an RLE-shaped stream under per-block meta-Huffman (11 groups,
entropy bits=4). Ours: 258,303 lit + 319 copies. Decisive
experiment: re-encoded cwebp's OWN residual stream + transform
plans through our full search — 124,403 B vs their 123,699 B
(0.57%), so ~74% of the 954 B gap is token-stage, ~26%
transform selection. On their residuals our RLE candidate fires
(6.7% coverage) but single-group emits 140,947 B and RLE+meta
124,991 B — still behind our DP+meta 124,403 B, so the residual
~0.5% sits inside libwebp's cost-based match selection, not in
any candidate family we lack. Also tested and rejected: a new
[SG, Predictor] spec (libwebp's composition order without
cross-color) — full-search probe gave camera sg-first best
126,004 B vs incumbent 124,674 B; our per-tile independent mode
scoring does not reproduce libwebp's joint accumulated-histogram
mode selection (PredictionCostSpatialHistogram), which is what
makes their residuals copy-friendly. All probes reverted
uncommitted.
