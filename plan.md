# Build a pure-Rust image optimization CLI

## Status

- Goal: still-image JPEG, PNG, WebP, and AVIF conversion and verified perceptual optimization.
- Accepted preparation: SVG removal at `2708512`. S0 accepted: independent PNG sample pilot, parser and backup safety, source notices, benchmark correctness workload, and size/RSS baseline. `make baseline-check` passes against integration, `2708512`, and the patched original codec binary. Integrated `make test`: 249 passed.
- S1 accepted: bounded still-image API and published PNG adapter, mandatory limits, shared no-clobber publisher and source identity checks. Six `codec_boundary` tests pass, including real races, replacement, symlinks, permissions and stream caps. Retained parser, backup, deadlock and timestamp suites pass. The minimal binary build passes. Distribution selects AGPL-3.0-only and preserves upstream notices.
- S2 accepted: `zenjpeg` 0.8.4 pure-Rust JPEG encode/decode, progressive and native grayscale, ICC/EXIF preservation, alpha and precision refusal, corruption and fixed-seed dimensions. Four `jpeg_codec` tests and all six boundary tests pass. Native request encoding preserves progressive mode; grayscale decode explicitly selects gray for one-component headers.
- S3 accepted: `zenwebp` 0.4.6 pure-Rust lossy/lossless WebP, hidden RGB exactness, exact alpha in both modes, explicit gray expansion, animation and high-depth refusal. Four `webp_codec` tests pass with independent libwebp output decoding. Prior JPEG and boundary suites remain green.
- S4 accepted: exact PNG matrix across gray/RGB/alpha at 8 and 16 bits, independent expanded-palette proof, CRC and animation refusal, metadata retention, and explicit palette quality using single-threaded zenquant 0.1.3. Four `png_codec` tests and prior codec/boundary suites pass. Published zenpng lacks gray-alpha encoding; the retained pure-Rust zune PNG encoder covers that representation. OxiPNG remains pending S10.
- S5 accepted for the explicitly supported/refused representation matrix: opaque eight-bit SDR AVIF through zenavif 0.1.6 and rav1d-safe 0.5.7. Four `avif_codec` tests pass, including independent dav1d decoding at direct quality endpoints and real grid/animation/high-depth refusal. Milestone `make test`: 271 passed. Alpha and monochrome AVIF, strict lossless and container transforms are named refusals. The failing alpha experiment is preserved at `campaign/s5-alpha-probe` (`a4d2fb5`), with retry conditions in `docs/AVIF.md`; it is not claimed supported or deferred to environment.
- S6 accepted: moxcms 0.8.1 float color preparation, independent LittleCMS P3 comparison, all EXIF orientations, exact alpha, gamut/HDR/depth refusal, metadata stripping, linear-light alpha-correct resizing and source-profile retention. Six `color_fidelity` tests, legacy ICC tests, JPEG/PNG and parser suites pass. Float/metric buffers raise the conservative reservation to 256 bytes/pixel. `ApplyICC::new` now parses ICC bytes; native LittleCMS is development-only.
- S7 accepted: cached fast-ssim2 0.9.1 references, exact alpha and black/white linear composites, negative scores, tiny-image and actual-byte checks. Four independent JPEG XL 0.12.0 linear-float goldens agree within 0.000001; offline and executable oracle checks pass.
- S8 accepted: deterministic coarse-grid/refinement search with global effort caps, actual-byte validation, smallest qualifying candidate selection, exact sample checks, explicit identity fallback and strip compliance. Five optimizer tests and milestone make test pass.
- Next: S9, serial implementation. Delivery is limited by implementation. The unsupported AVIF alpha experiment remains preserved; verified PNG/WebP alpha paths allow color/scoring work to continue.
- Execution authorized by the owner on 2026-11-11. Local commits only. Timing gates are deferred while this host is busy. S0 evidence and reproduction commands are in `BENCHMARKS.md`.
- Foundation: current macOS ARM64 tests pass. Published AVIF encode/decode and metric discrimination probes pass. Independent metric, color, alpha, and matched-quality proofs remain outstanding.

## Charter

Decide ordinary engineering choices and continue within the owner's execution grant. Record only decisions later slices depend on. A plan does not grant missing authority. Never push, publish, deploy, purchase, edit CI, run formatters, or change the pinned nightly.

Use one writer. Every slice shares pipeline, manifest, or test contracts, so no parallel implementation pairs qualify. Optional read-only research must follow the installed `orchestrate` skill. Use its campaign mechanics and parent steering checkpoint (installed skill reference: `orchestrate/references/campaigns.md#parent-steering-checkpoint`) after every accepted slice, on resume, and after two failures. The coordinator may change tactics and subdivide items while preserving contracts, prerequisites, and proof. Put worktrees in `.work/wt/<slice>`, scratch in `.work/tmp`, and ignore `.work/` through `.git/info/exclude` before writing. Set `TMPDIR` to that scratch directory and use `RUSTC_WRAPPER=kache`. Linux musl links with mold.

Each item ends with one local acceptance command and the narrowest affected suite on the integrated tree. Commands for future targets below are implementation deliverables, not commands available today. Use `--locked` after deliberate dependency resolution. Run `make test` at S5, S8, and S11. Keep integration green. Commit verified slices as Conventional Commits with workstream scope and item ID in the body. Do not commit red work to integration.

Reproduce a failure before repair and test one causal hypothesis at a time. After two failed fixes for the same cause, park the item on a preserved branch with cause and retry condition. Continue independent work. A blocked prerequisite parks its dependants. A single coordinator owns shared environment failures. Stop only when local deliverables pass or every remaining item is behind a stated hard stop. A parked local requirement prevents a claim of local completion.

Fix encountered data loss, supported-output corruption, or acceptance-command failures in separate commits. Also repair an execution check that takes over 180 seconds warm or a small-fixture operation that exceeds its 30-second test deadline. First prove the cause. Bound loop repairs to two attempts and one acceptance command. Record unrelated lesser issues without expanding scope.

Deliver feature and test parity before optimization. Measure early product costs without gating them. Final performance gates follow the installed performance instructions: require `perf-gate-ready` before and `perf-gate-ready -1` after timed runs. Discard overlapping runs. Busy hosts defer performance readiness, not implementation. Memory and artifact-size measurements still run. Never call deferred proof passed.

## Decisions

### Source requirements

These requirements come from the supplied draft. They are binding product intent, not new owner answers or authority to execute.

- Support macOS ARM64, Linux x86-64, and Linux ARM64. Remove Windows support. Static Linux musl is a separate best-effort artifact, never a label for a dynamically linked build.
- Use the AGPL-3.0 open-source path for selected Imazen dependencies. Preserve upstream MIT/Apache notices. Audit resolved licenses and describe the combined distribution accurately. No assumed commercial license.
- Replace codec-first commands and aliases with `optimize`, `convert`, `inspect`, and `compare`. Remove the old surface at S9. The already removed `--metadata` JSON file report stays removed.
- No third-party C/C++ codec or color-management runtime. No normal production build requiring libclang, CMake, NASM, pkg-config, or system codec packages. OS runtime interfaces remain allowed.
- JPEG, PNG, WebP, and AVIF are the final format scope. No implicit resizing, alpha flattening, precision reduction, HDR tone mapping, or first-frame extraction.

**Owner, 2026-11-11:** remove SVG support and its dependencies before the rewrite. Implemented at `2708512`. SVG is outside parity; do not restore its fixtures or reference implementation.

### Prescribed contracts

- Ordinary `optimize` keeps the source format. Default score is 95 and effort is `balanced`. Explicit `--quality` selects direct encoder control without a score guarantee. It still decodes and validates output. `--quality` and `--score` conflict. `--lossless` conflicts with both explicit flags and with resizing.
- Same-format optimization publishes only strictly smaller candidates satisfying all requested transformations. Otherwise preserve source bytes, in place or copied to a new destination, **only when those bytes satisfy the request**. A request to strip metadata cannot fall back to bytes containing that metadata. If no smaller compliant result exists, fail without publishing. Conversion honors its requested format even when larger. Convert uses score 95 unless explicit quality or lossless selects another mode.
- A source-byte fallback is an explicit unchanged result, not an encoded candidate with a fabricated score. Its identity score may be reported only when scoring that source representation is supported.
- `--format auto` is optimize-only. It considers compatible formats, including the source as an identity option. Choose the smallest qualifying evaluated output. Tie-break by score, then JPEG, PNG, WebP, AVIF, then evaluation order. No unsupported alpha, precision, or color path is eligible. PNG exact candidates count within the budget.
- Accept finite requested scores in [0, 100]. Actual SSIMULACRA2 scores can be negative; never clamp measured scores. Reject nonfinite metric results. A score is not a claim of human indistinguishability.
- Candidate caps are 8, 24, and 64 encodes per image for fast, balanced, and best, across all formats and refinements combined. Do not assume monotonic quality. Cache reference preparation; score each candidate only after decoding its actual encoded bytes. Record evaluated settings, size, score, and encode time. No separate budget flag in v1.
- Lossless means exact decoded sample equality, including hidden RGB under zero alpha, channel precision, and alpha. Compare expanded palette samples rather than palette indices. Preserve orientation and color interpretation. No decode/re-encode JPEG called lossless: same-format lossless JPEG copies original bytes unless a tested coefficient-preserving implementation exists. Unsupported cross-format or AVIF lossless modes fail explicitly.
- For scored lossy paths, prepare orientation-correct SDR sRGB references, using floating-point intermediates for color management. Treat alpha separately: require exact decoded alpha, and score composites on both black and white using the same linear-light preparation. Use the lower score. Never publish a flattened composite. This conservative v1 rule prevents unnoticed alpha damage and arbitrary hidden-RGB scoring.
- Support 8-bit SDR scoring first. Preserve 16-bit PNG in exact lossless paths. Reject scored high-bit-depth, HDR, unrecognized color signaling, or unsupported profiles until independently validated. Inspect must still describe their headers when safely readable. No silent gamut clipping. Non-sRGB content may be preserved with its profile for exact paths; reject lossy transformations that cannot preserve its semantics.
- Preserve grayscale where the destination can represent it. An explicit conversion to an RGB-only format may expand gray to equal RGB channels without reducing precision. JPEG rejects nonopaque alpha. For other unsupported combinations, return a named error.
- Default metadata policy is keep. Preserve ICC/CICP, orientation semantics, EXIF, and XMP where representable. Emit warnings listing cross-format metadata losses. Strip removes ancillary private metadata; retain required color signaling. Normalize orientation once when stripping orientation tags. If exact sample preservation prevents normalization, reject the lossless strip request. JPEG byte-copy fallback cannot satisfy stripping.
- Plan all destinations before writing. No overwrites except the source explicitly selected by `--in-place`. Refuse destination symlinks and directories. Reject symlink inputs and symlink traversal entries; do not follow directory cycles. Detect hard-link aliases to sources and duplicate outputs before publishing. Do not use lossy path strings as identity keys.
- Publish each file atomically from a destination-directory temporary file with exclusive creation. New destinations require atomic no-clobber publication, not exists-then-rename. In-place replaces only the authorized source and detects source identity changes. Flush and sync file data before publication. Sync the parent directory for replacement durability where supported. Distinguish a committed output with a later durability error from an unpublished failure.
- Output is a per-file transaction; batches are best effort, not globally atomic. Keep completed results, report every failure, and remove unpublished temporary files on errors. Cancellation stops admission, cancels codec work where supported, joins workers, cleans unpublished files, and preserves already published results. Graceful INT, TERM, HUP, and QUIT terminate with signal semantics. Use async-signal-safe fatal handlers to restore signal disposition and re-raise; fatal cleanup must not allocate or lock. Do not promise cleanup after SIGKILL or power loss; never delete unknown temporary files on restart.
- Preserve numeric exit categories 0 success, 2 usage, 3 input, 4 output, 5 mixed/partial. Existing mixed input/output failures with zero successes also return 5; retain this convention and document it. Score failure and unsupported input are input errors; candidate-encode or publication failures are output errors. Compare reports a score and succeeds; it does not silently introduce a pass/fail threshold.
- New code exposes operation capabilities, not codec implementation details. Keep parser and search internals private. Use concrete types or enums for format, validated score, effort, pixel representation, encoding mode, and publication mode. Resolve process globals once and keep selection logic separate from I/O. No plugin registry or generalized pipeline framework.

### Bounded executor choices

- Prefer compatible published releases. Pin the resolved lockfile and explicitly document Git revisions only when a published API cannot meet a named case. Do not import upstream workspace patch tables or vendor entire repositories. Inspect maintenance, licenses, transitive weight, native build scripts, duplicate codecs, and feature unification.
- Evaluate `zenjpeg`, `zenwebp`, `zenpng`, `zenavif`, and `fast-ssim2` first. Use `moxcms` for color management if its verified supported profiles meet the corpus. Keep existing `fast_image_resize` for resizing. Use `zenpixels`, conversion helpers, a pure-Rust quantizer, or Rayon only for a concrete accepted path. No `zenpipe`, `zenpicker`, or `zenanalyze` without demonstrated adapter reduction and the same dependency gates.
- At S1, compare a small concrete codec enum with published `zencodec` using one running adapter. Adopt the latter only if it reduces code and preserves required contracts. Prefer the smaller local enum on a tie. Keep useful `zune-image` operations initially; migrate pixel storage only for measured conversion savings or a required capability.
- At S0, capture build time, release binary size, startup, runtime, and RSS baselines. Dependencies must stay within the final comparison gates below. Before S0 has release numbers, record costs without inventing numeric baselines. No nightly update to rescue compatibility.
- Candidate scheduling and codec effort mapping are executor choices within the global caps. Evaluate a fixed coarse quality grid before deterministic local refinement. Reserve at least one attempt for every eligible format. Favor lower code and memory cost on equal compression results.
- Default jobs is 1; `--jobs N` is positive and bounded by available CPUs and the memory reservation model. Never run multiple candidates from one image concurrently initially. Disable codec-internal threading. Queue at most twice the worker count, with blocking backpressure.
- Derive numeric input, pixel, metadata, and memory ceilings at S1 from the existing limits and codec requirements. Never leave the limits feature as the only protection against arithmetic overflow or unbounded stdin. Use checked dimension/stride arithmetic and reject before allocation. No new public tuning flags unless a real fixture needs one.
- No compatibility aliases or shims for deleted commands or formats. Update affected exports, feature flags, call sites, tests, and docs in the same slice. Preserve unrelated library contracts; park any additional breaking library change not implied by the enumerated codec/format removals.

## Baseline and loop

The implementation baseline is `2708512`, with lexopt, native MozJPEG, libwebp, dav1d, and lcms2 paths, plus TIFF and other formats still exposed through zune-image. SVG and its font/rendering graph are gone. `metadata` remains image metadata support, separate from the removed JSON file report. There are no independent scoring tests, optimizer, operation-first commands, or benchmark targets yet.

`src/runtime.rs` has destination-directory temporary files and rename publication. It does not yet implement the planned atomic no-clobber contract. It also derives path keys through lossy lowercase strings. Existing tests protect backup collisions, recursion, timestamps, and single-thread deadlock. Preserve those guarantees while replacing their mechanisms.

`src/lib.rs` documents current AVIF limitations: still-only, 10/12-bit decoded to 8-bit, ICC not applied. Treat these as known fidelity debt, not reference behavior to preserve. Some encoder tests currently accept multiple frames; the final still-only contract deliberately supersedes those expectations. Retain useful still-image coverage.

Local preflight evidence on macOS ARM64:

| Check | Result | Wall time |
| --- | --- | --- |
| Parser: `cargo test --locked --all-features --bin rimage cli::command::tests` before SVG removal | 5 passed | 0.32 s warm |
| `cargo test --locked --all-features --test cli` after removal | 5 passed, including unsupported-vector safety | 5.57 s including compilation |
| `make test` after removal | 247 passed: 127 library, 106 binary, 14 integration | 9.64 s including compilation |
| `cargo check --locked --no-default-features --features build-binary` | Passed; existing unused-anyhow warning | 4.96 s including build-lock wait |
| Isolated AVIF and metric probe, published `zenavif 0.1.6` with only `encode`, `fast-ssim2 0.9.1` | AVIF 48×80 decode/encode/decode; identity 100, changed pixels lower | 5.40 s including compilation |

These are loop observations, not speed comparisons. The isolated probe proves a basic published API path on the pinned nightly, not independent decode, licenses of the final graph, alpha, color, losslessness, or absence of native build tools on every host. Recreate these probes as durable tests during their slices; scratch is not acceptance evidence.

Use `66fc0fc` for the original codec benchmark only. Its build script imports undeclared winresource even on non-Windows hosts. In a disposable worktree, remove that script and its manifest entry, record the exact patch and commands, and make no other baseline changes. Use `2708512` as the retained-behavior oracle. Do not restore SVG for parity. The current README asserts MIT/Apache licensing, but no license files are checked in; verify upstream notices and provenance at S0.

The Alpine Dockerfile provides a native-dependency test environment, not a static release proof. It sets `-crt-static`, links system dav1d, and installs native toolchains. Do not use its installed tools as evidence that the final build needs none. No current Linux proof is claimed by this preflight.

## Sequence

All items run serially. Each owns its named files, relevant manifest/lock entries, and focused tests. Sizes S/M describe an intended small/medium end-to-end slice; split any item whose focused command cannot finish within 180 seconds warm. S0 adds only the first behavior pilot, not a full evaluator as an entry fee.

| ID | Size | Prerequisites; ownership | Deliverable; local acceptance |
| --- | --- | --- | --- |
| S0 | S | Baseline; `tests/`, `benches/`, `Makefile`, licensing docs | Pin oracle, capture retained behavior pilot and goldens, establish provenance, add one real benchmark workload. `make baseline-check` |
| S1 | M | S0; `src/codecs/`, `src/cli/pipeline.rs`, `src/runtime.rs`, `src/limits.rs`, `src/error.rs`, `src/exit.rs` | Typed still-image boundary, mandatory resource bounds, one publication path. Tests exercise actual no-clobber and replacement behavior. `cargo test --locked --all-features --test codec_boundary` |
| S2 | M | S1; JPEG adapter, pipeline | Pure-Rust JPEG encode/decode; progressive and grayscale, metadata, corrupt input, alpha rejection, independent decode. `cargo test --locked --all-features --test jpeg_codec` |
| S3 | M | S2; WebP adapter, pipeline | Pure-Rust lossy/lossless WebP; exact samples and hidden RGB in lossless, exact alpha in both, animation rejection. `cargo test --locked --all-features --test webp_codec` |
| S4 | M | S3; PNG adapter, pipeline | PNG exact samples, indexed, grayscale, alpha, 16-bit and explicit palette-lossy path. Keep OxiPNG pending S10. `cargo test --locked --all-features --test png_codec` |
| S5 | M | S4; AVIF adapter, pipeline | Pure-Rust AVIF decode/encode; explicit precision, grid, animation, alpha, and color support/refusal. `cargo test --locked --all-features --test avif_codec`; milestone `make test` |
| S6 | M | S5; `src/operations/`, codec metadata paths | Replace lcms2; execute metadata/color/orientation/alpha/gray/depth matrix, including resize. `cargo test --locked --all-features --test color_fidelity` |
| S7 | M | S6; scoring module | Cached-reference metric, black/white alpha policy, dimensions and tiny-image behavior, fixed pixel oracle fixtures where available. `cargo test --locked --all-features --test perceptual_score` |
| S8 | M | S7; optimizer module | Deterministic bounded search over actual encoded bytes, direct quality and strict lossless modes, auto selection, compliant fallback, publication failures. `cargo test --locked --all-features --test optimizer`; milestone `make test` |
| S9 | M | S8; CLI, runtime, README, library docs | Complete operation-first commands, batch/streams, JSON, exit codes, signals, metadata policy. Delete old commands, aliases, unsupported formats and feature paths. `cargo test --locked --all-features --test cli` |
| S10 | M | S9; `benches/`, `BENCHMARKS.md`, dependencies as justified | Matched-quality workflow tuning, graph audit, size/build/RSS/CPU/allocation/copy costs, OxiPNG decision. `make benchmark-verify` validates outputs and saved measurements without requiring unavailable external tools |
| S11 | M | S10; affected source, tests and docs | Remove dead implementations/dependencies, complete license and native-graph audits, update docs, run installed ultrareview procedure and fix demonstrated P1/P2 issues. `make test`; final checks below |

S0's `make baseline-check` must run an end-to-end PNG encode with independently checked dimensions/samples, parser failure and backup-safety pilot, and the benchmark workload's correctness path. Add the parity matrix incrementally in `tests/parity.md`: each behavior is retained, intentionally changed with a reason, or open, with its fixture and test. Cover parsing, errors, side effects, traversal/globs/file lists, metadata, color, alpha, precision, exit aggregation, timestamps, and resource limits. Run reference and port on the same corpus. Normalize only genuine nondeterminism. No open retained behavior at close.

Codec and optimizer tests share small checked fixtures rather than one test per private helper. If a named independent decoder is unavailable, preserve locally executable sample/property tests and move only external verification to Needs environment. Never relabel self-round-tripping as independent proof.

## Acceptance

### CLI contract and proof at S9

`cargo test --locked --all-features --test cli` must spawn the real binary and check these behaviors:

- `rimage optimize photo.jpg` writes `photo.optimized.jpg`. `--in-place` alone authorizes replacing the source; it conflicts with explicit output and out-dir.
- `rimage optimize photo.jpg --format auto --score 90` chooses a compatible winner and writes `photo.optimized.<winner-extension>`. Auto conflicts with explicit `-o` and in-place. Batch out-dir is allowed and preserves the source extension in the stem, e.g. `photo.jpg.optimized.webp`, to avoid cross-format stem collisions.
- `rimage convert photo.png -o photo.webp` infers format. Explicit format must agree with the destination extension. Convert without explicit output uses `<stem>.converted.<format>` and requires format. `--format auto` is rejected by convert.
- `rimage convert photo.jpg --max-dimension 2048 -o photo.webp` preserves aspect ratio, never enlarges, and compares against the canonical resized reference. Reject zero or overflowing dimensions. No implicit resize elsewhere.
- `rimage optimize diagram.png --lossless`, `--score 95`, direct `--quality`, and `--effort fast|balanced|best` obey the decisions above. Direct quality is finite [0,100] and maps to documented per-codec settings, not equivalent cross-codec fidelity. Reject direct quality for lossless-only paths.
- `rimage optimize ./images --recursive --out-dir ./out --jobs 8` traverses regular supported files in stable native-path order. Require out-dir or in-place for directory or multiple-file inputs. Without recursive, visit only immediate files. Exclude the output subtree. Unsupported explicit files fail; unrelated extensions discovered in directories are skipped, recognized animations fail. Empty input sets are usage errors.
- Support explicit file lists and shell-expanded paths. Retain tested glob/file-list behavior until its matrix entries are deliberately resolved. Do not introduce shell command evaluation.
- Support `-o/--output`, `--out-dir`, `-j/--jobs`, `--format`, `--lossless`, `--score`, `--quality`, `--effort`, `--recursive`, `--quiet`, `--verbose`, `--json`, and `--strip-metadata`. Keep `--preserve-timestamps` on file outputs. Quiet and verbose conflict. Good command help and invalid-combination tests are required.
- Sniff stdin input bytes. Accept `-` for one input and `-o -` for binary stdout only with a concrete explicit format. Reject batch stdout, in-place stdin, JSON plus binary stdout, auto stdout, and output/out-dir conflicts. Diagnostics and progress use stderr. Buffer stdout candidates before emitting a verified winner; downstream broken pipes are output failures.
- `inspect` reads dimensions, format, precision, alpha, profiles, orientation and animation flags without changing files. `compare original.png optimized.webp` scores matched dimensions using the same canonical preparation and alpha policy; mismatches and unsupported scoring representations fail.
- JSON is one stdout object: `schemaVersion: 1`, `results`, `failures`. No implicit report file. Results have `input`, nullable `output`, `format`, `inputBytes`, nullable `outputBytes`, nullable `score`, `elapsedMs`, `status` (written, unchanged, inspected, compared), and `warnings`. Inspect adds `image` details; compare adds `reference`. Failures have input, nullable output, stable kind, message, and numeric exitCode. Use null only for inapplicable fields. Arrays follow input order, including concurrent execution. Document one example per command. Reject non-UTF-8 paths in JSON mode before any writes, preserve them in ordinary mode. Consumers may ignore unknown fields; changing existing meanings requires a schema version change.

### Image, file, and metric proof

The S1–S8 commands must cover every supported codec for decode, encode, dimensions, orientation, grayscale, alpha, metadata, precision, corrupt/truncated data, unsupported modes, and resource bounds. Exercise boundary sizes and checked arithmetic with fixed-seed property cases; no new elaborate fuzzing framework. Use independent decoders for output validity where available.

The scoring corpus includes tiny images, gradients, text, photos, transparent edges, unusual profiles, and explicit resizing. Define tiny-image handling from the metric's verified minimum dimensions: exact equality returns identity; reject unsupported nonidentical sizes instead of inventing padding. Score the actual decoded candidate. No NaN acceptance, mismatched dimensions, premultiplication artifacts, double orientation, or unrequested HDR conversion.

Independent SSIMULACRA2 validation uses a separately implemented reference on identical prepared SDR pixels. Pin version, input conversion, and fixed goldens. Maximum absolute difference is 0.05 across the controlled corpus. Investigate failures; do not widen tolerance to get green. Separate score-only validation from image-preparation tests. If the oracle is unavailable, local metric behavior can pass but independent validation remains listed as unproven.

Optimizer proof includes non-monotonic quality, cap exhaustion, no winner, deterministic ties, smallest qualifying evaluated output, strict lossless, never-bigger, identity fallback, incompatible strip fallback, and publication failures. Candidate codec quality alone is never evidence of fidelity.

Filesystem proof uses real directories under `.work/tmp`: spaces, punctuation, Unicode, non-UTF-8 bytes, symlinks, hard-link aliases, preexisting destinations and backups, competing destination creation, read-only output directories, short writes, interrupted workers, and source changes before replacement. Permission tests must actually run without privileges that bypass permissions. Completed files survive cancellation; unpublished files do not appear as successes.

Spot-check that parity tests fail on representative wrong output, dropped errors, and off-by-one resource bounds, then restore implementation. Preserve the correct tests.

### Final optimization and close

S10 expands the S0 pilot to a small licensed redistributable corpus: photos, skin/gradients, foliage/textures, screenshots/UI, text/diagrams, line art, flat colors, transparency, grayscale, existing compressed images, large images, ICC profiles, and supported high-bit-depth samples. Generated fixtures are valid for robustness but do not establish photographic compression competitiveness.

Compare original rimage and available MozJPEG, jpegli, cwebp, OxiPNG, and one AVIF encoder as development tools. Never link benchmark tools into production. Compare equal dimensions at actual scores 80, 90, and 95, and verify exact sample equality for lossless. Record source/output bytes and dimensions, actual scores, encode/decode time, peak RSS, parameters, threads, architecture, versions, and failures. Use release/dist binaries only for final measurements.

Use the installed `make-it-fast` and `benchmark-checklist` procedures. Rust benchmark targets use rustybench as a dev dependency with harness disabled. Capture raw machine-readable results plus concise `BENCHMARKS.md` reproduction commands, spread, limitations, wins and regressions. Interleave at least five runs per side; report median and spread and profile the limiting work separately. Include startup, end-to-end optimize, batch throughput, scoring/search overhead, allocation/copy costs, artifact size, and clean versus cached build costs.

There is no owner-specified absolute performance budget. For dependency and optional implementation choices, accept only reproducible benefit beyond observed spread. Reject over 10% median runtime/build regression or over 10% release-size/RSS growth against the same pre-change workflow unless required for a stated correctness capability; document such required costs rather than hiding them. These prescribed comparison gates do not require the new optimizer to match one legacy encode's runtime. Compare equal workloads. Keep OxiPNG only if it improves median PNG bytes by at least 1% at equal exact samples or reduces median runtime by at least 10% without larger outputs, beyond spread; otherwise remove it. Do not retain redundant encoders on branding grounds.

`make benchmark-verify` checks corpus validity, quality/sample constraints, matched dimensions, completed-work counts, saved-result schema and gate arithmetic. Test it against improvement, regression, newly exposed baseline debt, and unchanged consumers. Timed runs are separate from this minutes-long correctness command. Unavailable licensed real corpus, external tools, or quiet hosts go to Needs environment, not a fabricated comparison.

S11 removes native MozJPEG, webp/dav1d bindings, lcms2, JPEG XL, TIFF, peripheral formats, redundant encoders/conversions, unused features/macros/progress dependencies, and dead tests/docs once their retained behaviors are covered. Search source, tests, exports, manifests, scripts and docs for consumers. No compatibility tombstones. Keep one package with a library and binary; no architecture rewrite for aesthetics.

Final local checks: `make test`, `cargo check --locked --no-default-features`, `cargo check --locked --no-default-features --features build-binary`, and `cargo clippy --locked --all-features --all-targets`. Audit `cargo tree --locked -e features`, resolved production licenses, duplicate codecs, native build scripts, and actual binary linkage. Run a build in a clean supported environment without third-party native build tools when available. Do not infer native independence solely from crate names.

Use the installed ultrareview procedure at close. Review boundaries, types, config, effects, errors, concurrency, persisted outputs, input limits, tests, and navigability. Fix demonstrated P1/P2 defects before closing.

Finish with repository, branch, commit, architecture, removed features/dependencies, selected codec versions and licenses, breaking contracts, CLI examples, exact checks, benchmark/size results, limitations, and Needs environment. Local completion requires every local slice green. Release/performance readiness requires its external proofs too.

## Needs environment

These proofs do not hold local implementation open. Record the unavailable prerequisite, exact revision, command, expected result, and retry condition. Turn future commands below into runnable targets in the owning slices.

| Proof | Command and expected result |
| --- | --- |
| Final macOS ARM64, Linux x86-64, Linux ARM64 behavior | `make test` on each host; all pass at final revision. Use repo Dockerfile for Linux baseline reproduction. |
| Pure-Rust normal production build | `cargo build --locked --profile dist --features build-binary` on clean supported hosts without native codec packages/build tools; inspect linkage and build logs. Current Alpine image cannot prove this. |
| Independent metric | S7 adds `make metric-reference-check` with pinned external oracle; every corpus score within 0.05. Saved goldens keep ordinary local tests offline. |
| Independent output decoding | S2–S6 add `make decoder-reference-check`; independent dimensions, samples/alpha, and color matrix agree. |
| Corpus or dedicated encoder unavailable | S10 adds `make benchmark-run` with documented pinned tools and corpus acquisition; actual equal-quality outputs and completed work verified. |
| Quiet-host performance | `perf-gate-ready && make benchmark-run && perf-gate-ready -1`; interleaved results plus binary size/RSS on supported hosts. Busy host means deferred. |
| Optional static musl artifacts | `cargo build --locked --profile dist --features build-binary --target x86_64-unknown-linux-musl`, and aarch64 equivalent, using kache/mold and static CRT. Run each artifact and inspect ELF linkage on its target; no dynamic interpreter or shared codec dependencies. |

Independent metric and decoder proof block fidelity claims for unverified cases. Static musl remains best effort. Keep any unsupported target or representation explicit; an environment label cannot excuse a reproducible local implementation failure.

## Execution boundary and non-goals

The campaign, once separately requested, permits repository edits, local commits, bounded dependency/tool downloads, generated test fixtures, and disposable test outputs. It permits replacement only within isolated tests or an explicitly requested runtime `--in-place` invocation. Never process or overwrite the owner's image collection as a test. No external messages or remote state changes.

Intentional breaking changes are the operation-first CLI and removal of old aliases/options, Windows support, unsupported format/codec modules and features, corrected precision/color refusals, distribution licensing, and the new JSON schema. Preserve unrelated public library behavior; no speculative broad API redesign. The SVG removal is already authorized and complete.

Exclude animation, TIFF, SVG, HEIC, JPEG XL, RAW, watermarking, GUI, HTTP server, daemon, plugins, WASM demonstrations, persistent caches/databases, machine learning, new codecs/metrics/resizers, multi-crate frameworks, and generalized DAG execution. Do not confuse palette quantization with strict lossless compression. Favor simplicity, correctness, compression efficiency, performance, and maintainability in that order.

## Reference

Verify resolved versions rather than assuming repository HEAD APIs are compatible:

- [zenjpeg](https://github.com/imazen/zenjpeg), [zenwebp](https://github.com/imazen/zenwebp), [zenpng](https://github.com/imazen/zenpng), [zenavif](https://github.com/imazen/zenavif).
- [fast-ssim2](https://github.com/imazen/fast-ssim2) and optional [zencodec](https://github.com/imazen/zencodec).
- Current local owners: `src/cli/command.rs` (lexopt/schema), `src/cli/pipeline.rs` (decode/operations/encode), `src/runtime.rs` (batch/publication/metadata), `src/limits.rs` (budgets), `src/error.rs` and `src/exit.rs` (errors/status), `src/codecs/`, and `src/operations/`.
- Regression foundations: `tests/cli.rs`, `tests/deadlock.rs`, `tests/timestamps.rs`, and codec/operation tests.
