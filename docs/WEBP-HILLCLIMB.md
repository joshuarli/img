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
