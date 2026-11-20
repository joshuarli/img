#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)
base=${IMG_BASE:-$root}
scratch=${LOOPS_SCRATCH:-$root/.work/tmp/loops}
for variant in img:$root base:$base; do
  name=${variant%%:*}; src=${variant#*:}
  mkdir -p "$scratch/$name"
  cat > "$scratch/$name/Cargo.toml" <<TOML
[package]
name = "loops-$name"
version = "0.1.0"
edition = "2024"
[[bin]]
name = "score-loop"
path = "$root/benchmarks/codecs/score/score-loop.rs"
[[bin]]
name = "png-encode-loop"
path = "$root/benchmarks/codecs/png/encode-loop.rs"
[[bin]]
name = "verify"
path = "$root/benchmarks/codecs/verify.rs"
[[bin]]
name = "memory-jpegli"
path = "$root/benchmarks/codecs/memory/jpegli.rs"
[[bin]]
name = "memory-phases"
path = "$root/benchmarks/codecs/memory/phases.rs"
[dependencies]
img = { path = "$src" }
libc = "0.2"
jpegli-sys = { path = "$src/codecs/jpegli-sys" }
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
