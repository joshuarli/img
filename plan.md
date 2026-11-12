# Build a pure-Rust image optimization CLI

## Status

- Goal: deliver still-image JPEG, PNG, WebP, and AVIF conversion and verified perceptual optimization.
- Baseline: `ed310366d1129d57ad3901404789d48f28fbfcd0`; no campaign slices accepted, red, or parked.
- Next: S0. Use `cargo test --all-features --bin rimage cli::command::tests` as the first check.
- Foundation evidence: `make test` passed at `a1e434b` on macOS ARM64 and in the Alpine Linux ARM64 image; the subsequent error-classification change passed its focused parser tests. A standalone probe using published `zenavif 0.1.6` with `encode` decoded, encoded, and decoded the existing 48×80 AVIF fixture. `fast-ssim2 0.9.1` gave 100 for identical 16×16 RGB images and a lower score for a changed image. These probes do not establish color, alpha, lossless, or independent metric parity.
- Ready for S0 after a separate explicit request to execute this campaign.

## Charter

Work unattended within the execution grant. Decide ordinary engineering choices and record only decisions a later slice needs. An agent-authored plan grants no authority. Never push, publish, deploy, purchase a license, or run a formatter as part of this campaign. Keep worktrees under `.work/wt/` and scratch under `.work/tmp/`. Preserve the pinned Rust nightly unless the owner explicitly changes it.

Keep the integration branch green. Run one writer at a time: every implementation slice shares the manifest, lockfile, CLI, or image pipeline. A separate read-only research lane may run alongside a writer without changing those files. Follow `orchestrate` for any lane and run its parent steering checkpoint after every accepted slice, on resume, and after two failures. The coordinator may change methods and slice order if the contracts and proofs stay intact.

Run the item command and the narrow affected tests before each commit. Run `make test` at the codec, optimizer, and close milestones. Reproduce failures and establish the cause before retrying. After two failed attempts on the same cause, park that item with its cause and retry condition, then continue independent items. Park irreversible or outside-repository actions and an item whose verification loop cannot run. Keep unavailable host-specific proof in **Needs environment**.

Fix a defect encountered on the path in a separate commit when it loses or corrupts user data, violates a supported output contract, or makes a named acceptance command fail. Record lesser unrelated issues without expanding the slice. Preserve useful tests. Add regression tests for observed bugs and do not weaken a test to obtain a pass.

Implement parity and complete behavior before tuning. A measurement does not gate early slices unless the owner states a numeric gate. Before time-based measurements, require `perf-gate-ready` and `perf-gate-ready -1` to pass around each run; discard overlapping runs. If the host stays busy, record `perf gate deferred: host busy`, continue implementation, and do not declare performance or release readiness. Measure binary size and peak memory regardless of host load.

## Decisions

- **Owner:** target macOS ARM64, Linux x86-64, and Linux ARM64. Remove Windows support. Static Linux musl is a separate goal, not a reason to mislabel a dynamically linked build.
- **Owner:** select the AGPL-3.0 open-source path for the Imazen codecs. Preserve rimage's existing MIT/Apache attribution and upstream license notices. Audit the resolved production graph and state the combined distribution's license accurately. Do not assume a commercial license.
- **Owner:** remove the old `--metadata` JSON file report. The future `--json` command output is a distinct machine-readable result on stdout; it must not recreate that report or write a JSON file implicitly. Use one object with `schemaVersion: 1`, `results`, and `failures`. Each result records input, output, format, bytes, score when applicable, and elapsed time. Reject non-UTF-8 paths in JSON mode with a clear error instead of changing their bytes.
- **Owner:** replace the codec-first commands with `optimize`, `convert`, `inspect`, and `compare`. Remove old commands and aliases at the CLI cutover. Keep the previous executable as a test oracle until its retained behavior has been exercised.
- **Prescribed:** keep same-format optimization from producing a larger result. If no candidate wins, leave an in-place source unchanged or copy its bytes to the new output. `convert` honors its requested format even when larger. Refuse an existing destination unless it is the source explicitly authorized by `--in-place`; do not follow a destination symlink when replacing it.
- **Prescribed:** `--lossless` conflicts with `--score` and explicit lossy `--quality`. `--quality` conflicts with `--score`. Reject unsupported lossless, animation, HDR, bit-depth, alpha, or color transformations with a named error. Never flatten alpha or truncate precision silently.
- **Prescribed:** `--format auto` evaluates only formats that preserve required alpha, precision, and color semantics. Select the smallest candidate whose decoded bytes score at least the requested threshold. Break equal-size ties by higher score, then by the stable format order JPEG, PNG, WebP, AVIF, then evaluation order. If none qualifies, preserve the source for same-format optimization and fail conversion without publishing output.
- **Prescribed:** accept finite scores from 0 through 100; default to 95. The default search budget is at most 8, 24, or 64 encoded candidates per image for `fast`, `balanced`, or `best`. Search need not be monotonic. Record the evaluated candidate, bytes, score, and encoder settings; choose deterministically on ties. Do not expose a separate budget option until a concrete caller requires it.
- **Prescribed:** require an output format when writing binary output to stdout. Sniff stdin bytes for the input format. Keep binary stdout free of logs, progress, and JSON. Reject conflicting output flags, including an explicit format that disagrees with the destination extension.
- **Prescribed:** process files with bounded concurrency and disable nested codec threading by default. A candidate owns its buffers and temporary output. On failure or cancellation, clean up unpublished files and retain the original. Use destination-directory temporary files and atomic publication; test symlinks, existing files, errors, and interruption on real filesystems.
- **Delegated:** choose compatible published codec versions first. Enable only features needed for a supported path. Use a coherent pinned Git revision only if a published release cannot meet a named acceptance case. Inspect licenses, transitive dependencies, native code, binary size, and build and runtime costs before accepting each dependency. Do not update the nightly to make a dependency compile.
- **Delegated:** compare a small concrete codec interface with the published `zencodec` API at S1. Use `zencodec` only when it reduces the adapters and preserves the required types; otherwise use a local enum. Choose from running examples and tests, not a framework preference.
- **Prescribed:** for metric validation, compare fixed decoded pixels against an independent SSIMULACRA2 implementation. Record its version and input conversion. Require a maximum absolute score difference of 0.05 on the controlled SDR corpus. Investigate discrepancies rather than silently widening the tolerance. The identity probe above is not independent validation.

## Baseline and loop

The current binary uses lexopt and supports codec-first commands. The library still links native MozJPEG, libwebp, dav1d, and lcms2 on enabled paths. `Cargo.toml` also retains image metadata support; that is separate from the removed JSON report. The README claims MIT/Apache licensing, but this checkout has no license files; S0 must inspect the original upstream notices before changing the distribution license. The Alpine image currently links the system dav1d shared library and is not a static production release.

At preflight, the focused parser check took 0.45 seconds wall time on a warm macOS ARM64 build. This is a single loop measurement, not a performance comparison. `make test` passed at `a1e434b` on macOS ARM64 and Linux ARM64. Use initial commit `66fc0fc` as the original rimage codec baseline and `ed31036` as the retained-behavior oracle. The initial commit fails to build because its Windows-only `build.rs` imports undeclared `winresource`. In a disposable baseline worktree, remove that script and its manifest entry, record that minimal patch with the benchmark commands, and make no other baseline changes. Identify future codec versions in the benchmark report; do not treat debug-binary sizes as a release-size result. No independent score oracle or matched-quality benchmark has yet passed.

## Sequence

All slices below own their named paths plus the relevant manifest, lockfile, and focused tests. Shared paths make the slices serial. Each command is the minimum local acceptance check for that slice; create the named test target with the implementation rather than adding a test that only checks existence. Record real failing cases before fixing them. Commit each passing slice with a Conventional Commit; put the slice ID in the body.

| ID | Size | Prerequisite and owned paths | Deliverable and acceptance command |
| --- | --- | --- | --- |
| S0 | S | Baseline; `tests/`, `benches/` | Capture the retained behavior matrix for parsing, file safety, metadata, color, alpha, precision, and exit codes. Pin the runnable baseline and its fixtures. Add the smallest representative benchmark pilot and a `make baseline-check` target that executes its correctness checks and the retained behavior matrix. `make baseline-check` |
| S1 | M | S0; `src/cli/`, `src/runtime.rs`, `src/codecs/` | Establish a typed still-image boundary, resource limits, and one path for publication. Preserve retained behavior before removing unsupported formats. `cargo test --all-features --test codec_boundary` |
| S2 | M | S1; `src/codecs/`, `src/cli/pipeline.rs` | Integrate pure-Rust JPEG decode and encode. Test progressive, grayscale, alpha rejection, metadata, and independent decode. `cargo test --all-features --test jpeg_codec` |
| S3 | M | S1; `src/codecs/`, `src/cli/pipeline.rs` | Integrate pure-Rust WebP lossless and lossy paths. Verify decoded samples for lossless and alpha for both. `cargo test --all-features --test webp_codec` |
| S4 | M | S1; `src/codecs/`, `src/cli/pipeline.rs` | Integrate PNG and strict sample-equality paths. Keep OxiPNG until S10 establishes whether its measured advantage earns it. `cargo test --all-features --test png_codec` |
| S5 | M | S1; `src/codecs/`, `src/cli/pipeline.rs` | Integrate AVIF decode and encode without native decoder or encoder libraries. Verify unsupported precision and animation fail explicitly. `cargo test --all-features --test avif_codec` |
| S6 | M | S2–S5; `src/operations/`, `src/codecs/` | Replace native color management; preserve or reject ICC, CICP, orientation, alpha, grayscale, and bit depth as the product contract requires. `cargo test --all-features --test color_fidelity` |
| S7 | M | S6; `src/` scoring code | Implement cached-reference scoring and independent-oracle comparison with the declared tolerance. Test mismatched dimensions, alpha policy, small images, and color conversion. `cargo test --all-features --test perceptual_score` |
| S8 | M | S7; `src/` optimizer | Implement bounded candidate evaluation over actual encoded-and-decoded bytes. Test non-monotonic scores, no qualifying candidate, deterministic selection, strict lossless, never-bigger, and publication failure. `cargo test --all-features --test optimizer` |
| S9 | M | S8; `src/cli/`, `src/runtime.rs`, `README.md` | Deliver operation-first commands, explicit paths, batch and stream behavior, documented JSON stdout, exit codes, and metadata policy. Remove the retired codec-first surface. `cargo test --all-features --test cli` |
| S10 | M | S9; `benches/`, `BENCHMARKS.md` | Measure matched-quality codec candidates and real workflows. Audit native production dependencies, duplicated codecs, binary size, build time, CPU, allocations, peak RSS, and concurrency. Retain or remove OxiPNG based on the measured Pareto tradeoff. `make benchmark-verify` |
| S11 | M | S10; all affected source and docs | Remove retired implementations and unused dependencies. Exercise the final suite, license audit, supported hosts, and independent decoding. Run the ultrareview procedure and fix supported P1/P2 findings before close. `make test` |

## Acceptance

The work is complete locally only when every S0–S11 proof passes on the integrated tree and all of these conditions hold. Full release readiness additionally requires the applicable proofs in **Needs environment**:

- JPEG, PNG, WebP, and AVIF decode and encode on the selected pure-Rust production graph. Verify enabled features and native build scripts, not just direct dependencies. JPEG XL, TIFF, SVG, animated images, and the other out-of-scope formats are rejected or removed as described below.
- The requested CLI commands and examples work. Inputs are never overwritten without `--in-place`. Batch partial failures, paths with whitespace and non-UTF-8 bytes, symlinks, permissions, output collisions, stdin/stdout, and interruption have behavior tests.
- Lossless means decoded sample equality under the documented representation and metadata policy. Unsupported lossless output fails. JPEG lossless optimization preserves original bytes unless a genuinely coefficient-preserving implementation is proven.
- Every lossy winner is decoded from its actual encoded bytes and meets the target when compared to the correctly prepared reference. The selection is the smallest qualifying evaluated candidate. Alpha, orientation, ICC/CICP, transfer function, bit depth, and dimensions are handled explicitly.
- The benchmark harness covers the corpus and measurements in **Benchmarks** at matched dimensions and quality, with at least three practical score bands. `BENCHMARKS.md` includes reproducible commands, versions, results, spread, and limitations. Compare a real improvement, a real regression, and an unchanged consumer before treating a gate as meaningful.
- The parser, codec, path, and optimizer boundaries have bounded fuzzing or property tests for malformed inputs and arithmetic limits. Confirm the new reference-parity tests fail when a representative port behavior is intentionally broken.
- `make test` passes. Run relevant local compiler and lint checks without running a formatter or modifying CI. Report the repository, branch, final commit, architecture, removed dependencies and features, selected codec versions, CLI examples, exact checks, benchmark and binary-size results, production graph and licenses, supported host proofs, open environment needs, and deliberate contract changes. Do not call the work complete while a local acceptance item is parked.

## Needs environment

Run and record the final test suite and representative binary-size, peak-RSS, and quiet-host performance checks on Linux x86-64 and macOS ARM64. Linux ARM64 is available through the provided Docker image, but rerun after codec changes. Test static musl release artifacts on each Linux architecture separately and inspect their native linkage. If hardware, an independent score oracle, redistributable corpus images, or a named reference encoder is unavailable, record the exact missing command and continue other slices; do not claim that proof passed.

## Execution boundary and non-goals

Execution is confined to repository files, local tests, local commits, and bounded downloads of source dependencies and benchmark tools. No pushes, release tags, publication, deployment, purchases, external messages, or CI edits are authorized by this document. Document every breaking CLI, API, output schema, license, and format-support change in its slice.

The product excludes animation, TIFF, SVG, HEIC, JPEG XL, RAW, a GUI, network services, plugins, persistent caches, machine learning, and broad image-processing framework features. Explicitly reject unsupported inputs instead of silently decoding a first frame. Keep the scope and quality invariants below; use them as the detailed reference for the sequence and acceptance above.

## Product reference

## Product vision

Build the image optimization tool I wish existed:

- One small, fast native CLI.
- Pure Rust runtime codec stack, with no C/C++ codec dependencies.
- State-of-the-art JPEG, PNG, WebP, and AVIF support.
- Both lossless and perceptually optimized lossy compression.
- Intelligent conversion between formats.
- Ability to select the smallest output satisfying a measurable perceptual-quality target.
- Fast startup, efficient memory usage, and strong batch throughput.
- Reliable, non-destructive file handling.
- Excellent ergonomics for interactive shell use and automated scripts.
- No excessive abstraction, framework machinery, or unnecessary dependencies.

The goal is not to implement another ImageMagick, libvips, or generic image-processing framework.

**Optimize for simplicity, correctness, compression efficiency, performance, and maintainability—in that order, without sacrificing substantial performance unnecessarily.**


## Dependency and codec strategy

Evaluate and integrate the following Imazen projects.

| Format | Preferred implementation | Requirements |
|---|---|---|
| JPEG | https://github.com/imazen/zenjpeg | Encode and decode; progressive JPEG; strong rate–distortion efficiency |
| WebP | https://github.com/imazen/zenwebp | Lossy VP8 and lossless VP8L encoding/decoding |
| PNG | https://github.com/imazen/zenpng | Efficient truecolor and indexed PNG; high-effort lossless compression |
| AVIF | https://github.com/imazen/zenavif | Pure-Rust decode and encode; avoid dav1d/libaom FFI |
| Perceptual scoring | https://github.com/imazen/fast-ssim2 | SSIMULACRA2, SIMD, cached-reference support |
| Shared codec types | https://github.com/imazen/zencodec | Use where it actually reduces integration complexity |

Additional dependencies may be used where justified:

- `moxcms` for pure-Rust color management.
- `fast_image_resize` for resampling.
- `zenpixels` and `zenpixels-convert` where necessary for efficient codec interoperability.
- Imazen's `zenquant` or another proven pure-Rust quantizer for indexed PNG.
- `rayon` for parallel batch processing, if justified by benchmarks.

### Important: Imazen dependency compatibility

The Imazen ecosystem is evolving quickly. Its repositories may contain newer unpublished APIs, pinned cross-repository dependencies, and `zencodec` or `zenpixels` version transitions.

Do not assume that the latest commits across every repository build together.

Instead:

- Prefer compatible published crate releases when suitable.
- Otherwise use a coherent, tested set of Git revisions.
- Pin exact revisions and commit `Cargo.lock`.
- Audit transitive dependencies and duplicate crate versions.
- Avoid blindly copying entire upstream workspace patch tables.
- Document each intentionally pinned Git SHA and why it is needed.
- Avoid vendoring these repositories wholesale unless there is a compelling reproducibility requirement.
- Keep updating the codecs straightforward.

Do not pull in `zenpipe`, `zenpicker`, `zenanalyze`, or a larger Imazen framework merely because it exists. Evaluate their incremental value first.

Use `zencodec` directly only where its stable abstractions materially simplify the pipeline.

### Licensing policy

The current Imazen codec implementations are generally AGPL-3.0/commercial dual-licensed.

For this implementation, choose the **AGPL-3.0 open-source distribution path**, without assuming that a commercial license has been acquired.

Preserve all original rimage MIT/Apache-2.0 copyright notices and applicable licenses. Do not claim that upstream files have been relicensed or that the combined distribution remains MIT/Apache-only.

Add an accurate licensing explanation and verify the license of every selected dependency.

Do not purchase or subscribe to anything.

## Audit and simplify rimage

rimage is the foundation, not a sacred architecture.

It already has useful separation between codecs and operations, but its CLI and orchestration contain substantial complexity.

Pay particular attention to:

- `src/main.rs` and `src/runtime.rs`
- `src/cli/pipeline.rs`
- `src/cli/command.rs`
- `src/codecs/`
- `src/operations/`
- Existing input/output, limits, error, and metadata handling.

Identify code worth retaining versus code that should be removed.

### Keep where useful

- Mature file traversal and glob handling.
- Unicode-safe path handling.
- Atomic output writing.
- File-size and resource-limit safeguards.
- Existing structured errors and exit-code conventions.
- Resize operations.
- Quantization infrastructure where appropriate.
- Useful tests.
- Parallel batch processing.
- Metadata handling that preserves correctness.

### Remove or consolidate

- Native MozJPEG bindings.
- Native libwebp bindings.
- Native dav1d integration.
- lcms2, replacing it with a pure-Rust alternative.
- Duplicated or redundant encoders.
- JPEG XL functionality for now.
- Unnecessary peripheral formats and transformations.
- Unneeded SVG/font machinery.
- Redundant CLI commands and aliases.
- Excessive logging/progress dependencies.
- Redundant image-buffer conversions.
- Unnecessary crate features, build dependencies, and macros.
- Framework-style structures that add indirection without value.

Initial product scope is still-image JPEG, PNG, WebP, and AVIF.

GIF animation, animated WebP, APNG, animated AVIF, TIFF, SVG, HEIC, JPEG XL, RAW development, watermarking, a GUI, HTTP serving, and a persistent daemon are outside scope.

Never silently convert an animation to its first frame. Unsupported animated inputs should fail explicitly.

Audit actual feature utilization before deleting code. Simplicity must come from understanding the implementation, not indiscriminate removal.

## Architecture

Aim for a small number of cohesive subsystems:

```text
CLI / argument parsing
         |
File I/O and batch executor
         |
Image processing pipeline
         |
Optimization / candidate search
         |
Codec adapters + quality metric
         |
Pure-Rust codecs and pixel infrastructure
```

These are conceptual boundaries, not a mandate to create one crate or module per box.

Prefer one Cargo package with a library and a binary unless measurements justify additional crates.

### Codec boundary

Define a small, typed interface representing:

- Supported input/output formats.
- Lossless and lossy capabilities.
- Alpha support.
- Pixel precision and color requirements.
- Codec-specific quality and effort controls.
- Encode and decode operations.
- Resource requirements and errors.

Use enums and concrete types where practical. Avoid unnecessary trait-object hierarchies or runtime registries.

The optimization engine should not know the internal details of JPEG, WebP, AVIF, or PNG.

Where Imazen's `zencodec` already expresses the needed contracts cleanly, use those instead of creating duplicates.

### Pixel representation

Initially preserve the most useful parts of rimage's `zune-image` integration.

Measure whether interoperability with `zenpixels` introduces expensive conversions or copies.

If a `zenpixels`-based representation measurably simplifies the application and reduces memory movement, migrate carefully.

Do not conduct a wholesale pixel-engine rewrite based on aesthetics alone.

Requirements:

- Prefer borrowed pixel views over copying.
- Avoid unconditional RGBA expansion where unnecessary.
- Preserve alpha semantics.
- Preserve grayscale where supported.
- Preserve higher bit depths instead of silently truncating them.
- Treat image dimensions and stride calculations as checked operations.
- Keep lifetime and ownership contracts simple.

Use static dispatch and concrete types where practical; do not pursue zero-copy designs that make ordinary code excessively complicated.

## Perceptual optimization engine

This is the primary new capability.

Use **SSIMULACRA2**, preferably Imazen's accelerated `fast-ssim2` implementation.

The optimizer must search for a compact representation satisfying a perceptual quality threshold.

### Fundamental invariant

Every lossy candidate must be evaluated by decoding its actual encoded bytes and comparing the result against the correct reference image.

Do not estimate quality from encoder settings.

Do not compare numeric JPEG, WebP, and AVIF quality settings directly.

Do not claim that a higher encoder quality setting guarantees a monotonic SSIMULACRA2 score.

### Basic algorithm

1. Decode and validate the source.
2. Construct a canonical, color-correct reference.
3. Identify compatible candidate formats and encoder configurations.
4. Encode candidates.
5. Decode each candidate.
6. Measure SSIMULACRA2.
7. Reject candidates below the requested score.
8. Retain Pareto-efficient candidates by bytes, score, and effort.
9. Refine promising quality settings with bounded search.
10. Select the smallest qualifying output.
11. Verify the final result and write it safely.

Use fast bracket/binary-style search where appropriate, but allow for non-monotonic quality curves and local refinement.

Avoid exhaustive sweeps over huge parameter spaces.

Cache expensive reference-image preparation across candidate evaluations.

Avoid unnecessarily decoding the original source multiple times.

### Search policy

Implement three effort presets:

- `fast`: few candidates, minimal quality search.
- `balanced`: strong compression without excessive CPU usage.
- `best`: more comprehensive search, optimized for output size rather than runtime.

Default to `balanced`.

The user-facing effort setting should map to **computational effort**, not a misleading cross-codec quality number.

Provide an explicit way to cap candidate evaluations or execution budget if useful.

Record the selected codec, quality, effort, score, size, and encode time.

Do not introduce ML models, training pipelines, or complicated image classifiers in v1.

### Perceptual-score correctness

- Score against the source's decoded pixel content in a consistent color space.
- Honor ICC/CICP metadata and orientation.
- Never score mismatched dimensions.
- When resizing is explicitly requested, compare against a canonical resized reference.
- Handle alpha consistently rather than accidentally scoring arbitrary hidden RGB values.
- Do not silently flatten alpha-bearing images onto an invented background.
- Use the metric's documented color and luminance semantics.
- Validate scoring behavior on small images, gradients, text, photographic detail, and unusual color profiles.
- Never claim that a numerical score guarantees human-perceived indistinguishability.

Verify `fast-ssim2` against a reference implementation on a controlled corpus, with documented tolerances.

If HDR or unusual bit-depth comparisons are not correctly supported, reject or explicitly restrict those paths until validated.

## Lossless optimization

Lossless must have a strict, testable meaning.

A `--lossless` operation must preserve decoded image samples exactly, subject to the explicitly documented pixel representation and metadata policy.

It must never silently turn into high-quality lossy encoding.

### PNG

Use zenpng as the default candidate.

Explore its high-effort compression modes, filter selection, color-type reduction, indexed encoding, and related lossless optimizations.

Benchmark against OxiPNG, including high-effort modes.

Do not remove OxiPNG merely to achieve uniform branding if it retains a meaningful Pareto advantage. It is already Rust-based.

However, avoid retaining two PNG encoders without a measurable reason. Choose the smallest maintainable implementation that achieves competitive results.

Lossy palette quantization must be an explicit perceptually optimized operation, not part of strict lossless mode.

### WebP

Support both lossless and lossy output using zenwebp.

Benchmark its effort settings.

Preserve transparency.

### JPEG

Ordinary JPEG decode/re-encode is not lossless.

Investigate whether the chosen codec stack supports a genuine DCT-coefficient-preserving JPEG optimization operation.

Only implement it if supported and correctly tested.

Otherwise, strict lossless optimization of an existing JPEG should preserve the original bytes rather than secretly recompressing its pixels.

### AVIF

Support the actual lossless capabilities of the selected pure-Rust encoder, if available and verified.

Do not infer losslessness from a quality setting of 100.

Unsupported lossless modes must fail clearly rather than silently degrade.

## CLI redesign

Replace the codec-first interface with an ergonomic, operation-first interface.

The primary commands should be:

```text
rimage optimize
rimage convert
rimage inspect
rimage compare
```

Avoid an explosion of subcommands.

### Desired interface

```bash
# Optimize in the original format
rimage optimize photo.jpg

# Explicitly optimize in place
rimage optimize photo.jpg --in-place

# Strictly lossless optimization
rimage optimize diagram.png --lossless

# Perceptual quality targeting
rimage optimize photo.jpg --score 95

# Convert between formats
rimage convert photo.png -o photo.webp

# Explicit output format
rimage convert photo.jpg --format avif -o photo.avif

# Find the smallest suitable format
rimage optimize photo.jpg --format auto --score 90

# Spend more CPU searching for smaller output
rimage optimize photo.jpg --effort best

# Resize explicitly
rimage convert photo.jpg --max-dimension 2048 -o photo.webp

# Batch processing
rimage optimize ./images --recursive --out-dir ./out --jobs 8

# Inspect metadata and dimensions
rimage inspect photo.avif

# Compare perceptual fidelity
rimage compare original.png optimized.webp

# Machine-readable output
rimage optimize photo.jpg --json
```

These are desired interface contracts, not necessarily the exact internal argument structures.

Implement them consistently, with excellent `--help` output.

### Default behavior

Choose conservative, predictable defaults:

- No implicit resizing.
- Same-format output unless conversion is requested.
- Use a high-fidelity SSIMULACRA2 target, initially 95, for lossy optimization.
- Strictly lossless treatment when `--lossless` is specified.
- No silent conversion between lossy and lossless semantics.
- Do not overwrite source files without `--in-place`.
- For a single-file operation with no output specified, use a predictable `.optimized` filename.
- For batch operations, require an output directory or explicit in-place mode.
- Ordinary same-format optimization must not produce a larger output than the original; retain the source when no qualifying smaller candidate exists.
- `convert` should honor the requested output format even if the result grows.
- Resolve conflicts between file extensions and explicit format arguments clearly.
- Preserve color correctness and meaningful metadata by default.
- Never silently downconvert bit depth or flatten transparency.

A default perceptual score of 95 is a starting contract, not proof that the result is visually lossless. Document the distinction.

### Metadata

Provide straightforward `keep` and `strip` policies.

Preserve ICC/CICP information and orientation semantics by default.

Preserve EXIF/XMP where supported. When cross-format metadata cannot be represented faithfully, report what is being discarded rather than claiming perfect preservation.

Do not confuse privacy-related metadata removal with image-quality optimization.

### Shell ergonomics

Support:

- `-o` / `--output`
- `--out-dir`
- `-j` / `--jobs`
- `--in-place`
- `--format`
- `--score`
- `--quality` for explicit direct encoder control
- `--lossless`
- `--effort`
- `--recursive`
- `--quiet`
- `--verbose`
- `--json`
- `--strip-metadata`

Keep argument meanings consistent between commands.

Make incompatible option combinations explicit usage errors.

Support stdin/stdout via `-` where sensible, keeping stdout free of progress messages or diagnostics.

Report per-image size changes, quality scores, selected formats, and elapsed time without excessive noise.

Use stable, meaningful exit codes and a documented JSON schema.

## Color fidelity and image correctness

Treat color handling as part of codec correctness, not an optional enhancement.

Research and integrate moxcms or an equivalent pure-Rust color-management stack.

Key requirements:

- Correct sRGB behavior.
- ICC profile handling.
- Display P3 and wider-gamut images where supported.
- Correct transfer functions.
- Correct range and matrix handling for AVIF/WebP.
- No accidental premultiplied-alpha artifacts.
- No silent gamut clipping or gamma mistakes.
- No accidental bit-depth truncation.
- No double application of EXIF orientation.
- No unrequested HDR-to-SDR conversion.

Do not promise universal high-bit-depth/HDR support if the selected codecs or pipeline cannot actually preserve it.

Unsupported combinations should produce explicit errors.

Build targeted tests for profiles, orientation, alpha, grayscale, bit depth, and chroma subsampling.

Compare outputs with independent decoder oracles when practical.

## Performance engineering

This is a performance-sensitive CLI.

Optimize actual measured costs:

- Process startup time.
- Binary size.
- Build time.
- Decode throughput.
- Encode throughput.
- Peak RSS.
- Intermediate buffer allocations.
- Memory-copy overhead.
- Quality-score calculation time.
- Candidate-search overhead.
- Batch throughput.

Prefer eliminating work to adding caching complexity.

Avoid duplicate image buffers unless genuinely necessary.

Make candidate evaluation memory-bounded.

Batch processing should parallelize independent images using a bounded worker count and a reasonable memory budget.

Avoid nested oversubscription when codec internals already use parallelism.

Use Rust SIMD support from upstream codecs rather than adding architecture-specific handwritten assembly.

### Target environments

Prioritize:

1. macOS ARM64.
2. Linux x86-64.
3. Linux ARM64.
4. Static Linux musl builds where practical.

Native SIMD must remain portable, with correct fallback behavior.

Do not introduce dependencies requiring external codec libraries, system package managers, libclang, CMake, NASM, C++ toolchains, or pkg-config for the normal production build.

Rust platform-runtime dependencies are acceptable. The objective is no third-party native codec dependency, not an impossible absence of all operating-system interfaces.

Disable optional codec assembly/FFI features where necessary to satisfy the pure-Rust requirement.

Audit the **resolved production dependency graph**, not merely the direct Cargo dependencies.

## Benchmarks

Build a reproducible benchmark suite before major implementation changes.

Use the original rimage as the baseline, with its version and commit pinned.

Also compare against relevant dedicated encoders where available:

- MozJPEG.
- jpegli where available.
- libwebp / cwebp.
- OxiPNG.
- A representative AVIF encoder.

Benchmark dependencies may exist as external development tools; they must not be linked into the production binary.

### Corpus

Use a small, redistributable, representative image corpus covering:

- Photographs.
- High-detail foliage and textures.
- Human skin and smooth gradients.
- Screenshots and UI.
- Text and diagrams.
- Line art and graphics.
- Transparent images.
- Flat-color images.
- Grayscale.
- Existing compressed images.
- Large source images.
- ICC-profiled images.
- High-bit-depth sources where supported.

Keep the committed test corpus reasonably small.

Document each corpus source and its license.

### Metrics

For every candidate and format, record:

- Source dimensions.
- Output dimensions.
- Input bytes.
- Output bytes.
- Actual SSIMULACRA2 score.
- Encode time.
- Decode time where relevant.
- Peak memory.
- Encoder parameters.
- CPU/thread configuration.
- Host architecture.

Compare **at matched dimensions and matched perceptual quality**.

Include at least three target quality bands where feasible, such as 80, 90, and 95.

Report both compression efficiency and compute cost.

Do not claim an encoder is superior because its quality parameter happens to produce a smaller image.

For lossless comparisons, verify decoded sample equality rather than relying on a perceptual score.

### Reporting

Create a concise `BENCHMARKS.md` containing:

- Reproduction instructions.
- Exact versions and commands.
- Results tables.
- Compression-size comparisons.
- Encode speed and memory comparisons.
- Known weaknesses and regressions.

Retain raw machine-readable results and an automated harness.

Benchmark findings should drive architecture and default-setting choices.

Do not manipulate benchmark methodology to make the fork look better.

## Tests and regression prevention

Preserve useful upstream rimage tests and add a rigorous focused suite.

### Codec tests

Test all supported formats for:

- Decode correctness.
- Encode correctness.
- Round trips.
- Dimensions and orientation.
- Alpha.
- Grayscale.
- Metadata preservation.
- Resource limits.
- Truncated/corrupt inputs.
- Unsupported modes.
- Boundary dimensions.
- Output validity using independent decoders.

### Optimization tests

Verify:

- Never-bigger behavior for same-format optimization.
- True losslessness when requested.
- Candidate outputs meet the requested score.
- Auto-format selects the smallest *qualifying* candidate among the evaluated set.
- Search handles non-monotonic quality curves.
- Fallbacks do not destroy input.
- Deterministic selection for fixed inputs and settings.
- Explicit resizing produces the expected dimensions.
- No mismatched-dimension quality comparisons.
- Failed encoding does not leave corrupt output.

### CLI tests

Verify:

- Argument parsing.
- Format inference.
- Conflicting arguments.
- Safe overwrite behavior.
- Atomic file publication.
- Unicode filenames.
- Paths with whitespace and special characters.
- Recursive traversal.
- Batch partial failure handling.
- stdin/stdout.
- JSON output.
- Exit codes.
- File permission errors.
- Symlinks and destination-path safety.

### Robustness

Use reasonable decoding and pixel-count limits.

Avoid unchecked integer overflow, excessive allocation, panics on malformed inputs, and unbounded candidate searches.

Use fuzzing or property-based tests for selected dangerous input boundaries.

Do not add an elaborate fuzzing framework if a smaller existing setup is sufficient.

## Scope and simplification discipline

This project is especially vulnerable to scope creep.

Explicitly avoid:

- Building a new codec.
- Reimplementing SSIMULACRA2.
- Writing a new image-resizing engine.
- Creating a plugin system.
- Adding an image server.
- Adding a GUI or browser UI.
- Adding WASM support solely for demonstration.
- Adding generalized DAG/workflow execution.
- Creating a multi-crate framework without justification.
- Introducing machine-learning model downloads.
- Adding persistent caches or databases.
- Supporting every possible image format.
- Creating enormous generated documentation or speculative roadmaps.

Prefer direct, understandable Rust code.

Code should use strong types, explicit invariants, and well-bounded state.

Avoid unnecessary dynamic dispatch, stringly typed codec configuration, and layers of command-routing indirection.

Keep the number of production dependencies as low as reasonably possible.

Reduce code size where doing so improves correctness or maintainability. Do not fetishize line count at the expense of clarity.

