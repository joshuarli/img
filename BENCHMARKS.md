# Benchmark reference

## S0 pilot

The retained behavior oracle is `2708512`. `make baseline-check` runs a real file-to-file PNG conversion, checks every decoded RGB sample with the independent `png` crate, and tests parser and backup safety. The benchmark uses that same correctness path, including input generation, process startup, file publication, independent decoding, and cleanup. It is a bounded procedural workload, not evidence of photographic compression competitiveness.

```sh
make baseline-check
cargo build --locked --profile dist --features build-binary
RIMAGE_BENCH_BIN="$PWD/target/dist/rimage" cargo bench --locked --all-features --bench workflow -- --help
```

Run timed benchmarks only between successful `perf-gate-ready` and `perf-gate-ready -1` checks. S0 timing gates are deferred because the host was busy. There is no accepted build-time, startup, or runtime comparison yet.

`benchmarks/s0-macos-arm64.json` records size and memory observations. The dist binary is 6,547,968 bytes. One 256×192 PNG conversion completed with peak RSS 4,685,824 bytes according to macOS `/usr/bin/time -l`. Its raw resource log is retained beside the JSON. Timings in that raw log are not accepted measurements. The small fixture does not model large-image working memory.

The development dependencies are `png` 0.18.1 for independent decoding and `rustybench` 0.5.0 for the required harness. Neither enters the normal production graph. Build-time impact remains unmeasured while the host is busy.

## Reproduce the original codec baseline

Use a disposable worktree of `66fc0fc`. Apply `docs/original-build.patch`: remove the undeclared Windows build script and manifest entry. Cargo also prunes the stale `winresource` lock entries without changing any remaining resolved package version. This mechanical lock pruning is included in the exact patch.

```sh
git worktree add .work/wt/original 66fc0fc
git -C .work/wt/original apply ../../../docs/original-build.patch
cd .work/wt/original
TMPDIR=/absolute/path/to/rimage/.work/tmp RUSTC_WRAPPER=kache cargo build --locked --release --features build-binary
```

The original manifest has no dist profile. Its release output must not be compared directly to a stripped, LTO-enabled dist build as an implementation-size result. Do not restore SVG to the parity corpus. Original runtime comparisons use only the supported raster corpus. Final matched-quality and quiet-host comparisons belong to S10.

## Verified corpus and current limits

`make benchmark-verify` checks the pinned corpus hashes and licenses, decoded
headers, exact sample constraints, score targets, saved output bytes, dimensions,
recomputed scores, result schema, completed-work counts and gate arithmetic.
Its fixtures exercise improvement, regression, baseline debt, unchanged
consumers and deferred timing. It deliberately rejects fast failed work.

The corpus contains four licensed photographs from scikit-image v0.25.2 and ten
CC0 procedural representations. See [corpus provenance](benchmarks/corpus/README.md).
The astronaut image's nominal sRGB ICC has quantized colorants. Both the
independent float CMS and Rimage leave the destination gamut near endpoints.
Scored processing is explicitly refused, rather than clipping it. PNG files
with layout-sensitive ancillary chunks also require strip before re-encoding.
These failures are retained in raw results. No failed fixture contributes to
a size or runtime win. Synthetic cases do not establish photo competitiveness.

Capture untimed correctness and size observations with the dist executable:

```sh
cargo build --locked --profile dist --features build-binary
cargo run --locked --all-features --example benchmark -- capture
make benchmark-verify
```

`benchmarks/results/untimed.json` records each completed or failed evaluation
and points to its retained output. Scores are measured from actual decoded bytes.
Targets are minimum scores 80, 90 and 95. `matchedWithinHalfPoint` distinguishes
close matches from overshoot. Do not call overshooting outputs equal-quality
wins. Exact PNG comparisons use exact samples, precision, ICC and orientation.
Summaries are per tool, fixture and target. Different completion sets are never
aggregated into a cross-tool performance claim. Timings are null when deferred.

The development comparison runs the patched original rimage's MozJPEG and
OxiPNG, the new CLI, and cwebp 1.6.0. Original JPEG and cwebp evaluate the fixed
0,25,50,75,95,100 grid. New search uses fast's eight-attempt cap. This compares
specified search configurations, not equally tuned encoder implementations.
The original uses one image worker and `RAYON_NUM_THREADS=1`; cwebp omits `-mt`.
JPEGli and a dedicated AVIF encoder were unavailable. No substitute result is
invented. Installed cjpeg is libjpeg-turbo 3.2.0, not MozJPEG or jpegli.

Run timed comparisons only on an otherwise quiet supported host:

```sh
make benchmark-run
RIMAGE_BENCH_BIN="$PWD/target/dist/rimage" cargo bench --locked --all-features --bench workflow
```

The runner interleaves tool order across five repetitions, includes process
startup, encode, output readback and fidelity checks, and rejects the complete
timing capture if either perf gate fails. Per-case medians and full ranges are
saved. Workflows cover version startup, exact file conversion, scored optimize
and an eight-file batch. The original workload uses `RIMAGE_BENCH_LEGACY=1` and
the `png_file_conversion` filter; operation-first workloads have no legacy alias.

Current performance verdict: **inconclusive; perf gate deferred: host busy**.
No runtime, startup, clean-build or cached-build speedup is claimed. Profiling,
allocation accounting and scoring/search cost attribution remain deferred with
the timing proof. Useful retry commands are the runner and rustybench workloads
above, plus a separate `sample` or `samply` capture of the scored workload.
Do not include profiler overhead in accepted timings.

An intermediate S9 dist executable was 7,364,448 bytes and used 10,747,904 peak
RSS bytes for the same 256×192 exact PNG file workflow as S0. Both exceed S0's
size/RSS by more than 10%. These are required costs of the new bounded codec,
color and fidelity capabilities, not an accepted speed/size improvement.
Final graph removal and resource observations are recorded at close.

OxiPNG is removed from production: its selected libdeflate path requires native
code and violates the normal-build contract. Untimed PNG byte observations do
not override that requirement. No optional compression path is retained on an
unmeasured runtime claim. Native original tools remain development comparisons.

Quiet-host timing, clean/cached build cost, allocation/copy profiles, missing
dedicated encoders and other target hosts remain Needs environment at the final
revision. Saved local correctness evidence does not imply release readiness.
