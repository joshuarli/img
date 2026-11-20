#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../../.." && pwd)
scratch="$root/.work/dav1d/rav1d-baseline"
mkdir -p "$scratch/source" "$scratch/harness" "$root/.work/tmp"
git -C "$root" archive e7f01acf1da136f33c437f565509b6d326697b39 src Cargo.toml Cargo.lock benchmarks/codecs/avif/decode-loop.rs | tar -x -C "$scratch/source"
[ -e "$scratch/source/vendor" ] || ln -s "$root/vendor" "$scratch/source/vendor"
[ -e "$scratch/source/codecs" ] || ln -s "$root/codecs" "$scratch/source/codecs"
cat > "$scratch/harness/Cargo.toml" <<TOML
[package]
name = "avif-qual-img"
version = "0.1.0"
edition = "2021"
[[bin]]
name = "avif-qual-img"
path = "$scratch/source/benchmarks/codecs/avif/decode-loop.rs"
[dependencies]
img = { path = "$scratch/source", features = ["build-binary"] }
rav1d = { path = "$root/vendor/rav1d", default-features = false, features = ["bitdepth_8", "bitdepth_16", "asm", "asm_arm64_dotprod", "asm_arm64_i8mm", "asm_arm64_sve2"] }
libc = "=0.2.183"
[patch.crates-io]
oxipng = { path = "$root/vendor/oxipng" }
libdeflater = { path = "$root/vendor/libdeflater" }
libdeflate-sys = { path = "$root/vendor/libdeflate-sys" }
fast-ssim2 = { path = "$root/vendor/fast-ssim2" }
rav1d = { path = "$root/vendor/rav1d" }
[profile.release]
opt-level = 3
lto = "thin"
panic = "abort"
strip = false
TOML
cp "$scratch/source/Cargo.lock" "$scratch/harness/Cargo.lock"
TMPDIR="$root/.work/tmp" CARGO_BUILD_JOBS=4 cargo build --release --offline --manifest-path "$scratch/harness/Cargo.toml" --target-dir "$scratch/target"
