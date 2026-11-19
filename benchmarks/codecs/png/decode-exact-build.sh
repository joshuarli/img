#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
scratch="$root/.work/tmp/png-exact"
mkdir -p "$scratch" "$root/.work/tmp"
cat > "$scratch/Cargo.toml" <<END
[package]
name = "png-exact"
version = "0.1.0"
edition = "2024"
[[bin]]
name = "png-exact"
path = "$root/benchmarks/codecs/png/decode-exact.rs"
[dependencies]
png = "=0.18.1"
zune-png = "=0.5.2"
zune-core = "=0.5.1"
zenpng = "=0.1.4"
enough = "=0.4.5"
END
CARGO_BUILD_JOBS=4 TMPDIR="$root/.work/tmp" cargo build --offline --release --manifest-path "$scratch/Cargo.toml"
