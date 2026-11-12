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
