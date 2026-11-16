# WebP correctness and Pareto hillclimb

## Current brief — reviewed 2026-11-15

This is the authoritative operating brief for the next unattended run in
`joshuarli/img`, branch `master`. The previous runner paused; this documentation
review does not claim to have restarted it or measured a new binary. Reviewed
source: `6141369e1648494d8eb3c4d50d424c4e3cd1a04b`; latest encoder change:
`c3c1abe`. Start from the current default branch, preserving intervening work.

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
  fallback. This is not a blanket grant for new unsafe code or native-only
  release settings. Keep the standalone scalar/`no_std + alloc` path viable.
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
| Lazy selector equivalence | Unproven empirical pruning contradicts recorded constraint | P0 reference-selector comparison and sound scheduling |
| Size/quality frontier | Promising narrow minimum-target workflow result | Retained refined frontier points and held-out strata |
| Workflow speed | Material measured deficit | Fresh exact-selector baseline, then structural reductions |
| Isolated encoder/decoder, CPU, memory, tail | Incomplete or older revision evidence | Separate measurements tied to each artifact/configuration |
| Linux musl ARM64/x86-64 | Historical functional evidence; native timing incomplete | Native target evidence; no emulated timing claims |
| Broader conformance/fuzz/resource qualification | Existing tests, no complete campaign closure evidence | Bounded product-relevant qualification with recorded coverage |

**Destination gate: NOT met.** Wall time is the largest measured performance gap,
not the only unfinished technical obligation. A passing saved-output verifier
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
