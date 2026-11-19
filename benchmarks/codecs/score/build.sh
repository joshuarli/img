#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../../.." && pwd)
base=${IMG_BASE:-$root}
scratch=${SCORE_SCRATCH:-$root/.work/tmp/score-bench}
for variant in img:$root base:$base; do
  name=${variant%%:*}; src=${variant#*:}
  mkdir -p "$scratch/$name"
  cat > "$scratch/$name/Cargo.toml" <<TOML
[package]
name = "score-loop-$name"
version = "0.1.0"
edition = "2021"
[[bin]]
name = "score-loop-$name"
path = "$root/benchmarks/codecs/score/score-loop.rs"
[dependencies]
img = { path = "$src" }
[patch.crates-io]
$(sed -n '/^\[patch.crates-io\]/,/^\[/p' "$src/Cargo.toml" | rg '^[a-z0-9_-]+ = \{ path' | sed "s#path = \"#path = \"$src/#")
[profile.release]
opt-level = 3
lto = "thin"
panic = "abort"
TOML
  cp "$src/Cargo.lock" "$scratch/$name/Cargo.lock"
  CARGO_BUILD_JOBS=4 cargo build --offline --release --manifest-path "$scratch/$name/Cargo.toml" --target-dir "$scratch/target-$name"
done
