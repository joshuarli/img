#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname "$0")/../../.." && pwd)
scratch=${AVIF_SCRATCH:-/Users/josh/d/img/.work/tmp/avif-candidates}
mkdir -p "$scratch/qual-img" "$scratch/qual-dav1d" "$scratch/qual-zen"
cat > "$scratch/qual-img/Cargo.toml" <<TOML
[package]
name = "avif-qual-img"
version = "0.1.0"
edition = "2021"
[[bin]]
name = "avif-qual-img"
path = "$root/benchmarks/codecs/avif/decode-loop.rs"
[dependencies]
img = { path = "$root", features = ["build-binary"] }
rav1d = { path = "$root/vendor/rav1d", default-features = false, features = ["bitdepth_8", "bitdepth_16", "asm", "asm_arm64_dotprod", "asm_arm64_i8mm", "asm_arm64_sve2"] }
libc = "=0.2.183"
[patch.crates-io]
oxipng = { path = "$root/vendor/oxipng" }
libdeflater = { path = "$root/vendor/libdeflater" }
libdeflate-sys = { path = "$root/vendor/libdeflate-sys" }
rav1d = { path = "$root/vendor/rav1d" }
[profile.release]
opt-level = 3
lto = "thin"
panic = "abort"
strip = false
TOML
cat > "$scratch/qual-zen/Cargo.toml" <<TOML
[package]
name = "avif-qual-zen"
version = "0.1.0"
edition = "2021"
[[bin]]
name = "avif-qual-zen"
path = "$root/benchmarks/codecs/avif/decode-zen.rs"
[dependencies]
zenavif = { version = "=0.1.6", default-features = false }
[profile.release]
opt-level = 3
lto = "thin"
panic = "abort"
strip = false
TOML
cat > "$scratch/qual-dav1d/Cargo.toml" <<TOML
[package]
name = "avif-qual-dav1d"
version = "0.1.0"
edition = "2021"
build = "build.rs"
[[bin]]
name = "avif-qual-dav1d"
path = "$root/benchmarks/codecs/avif/decode-dav1d.rs"
[dependencies]
yuv = { version = "=0.8.20", default-features = false }
[build-dependencies]
cc = "=1.5.1"
[profile.release]
opt-level = 3
lto = "thin"
panic = "abort"
strip = false
TOML
cat > "$scratch/qual-dav1d/build.rs" <<RUST
fn main() {
    cc::Build::new().include("$scratch/src/libavif-v1.4.2/include").include("/opt/homebrew/Cellar/dav1d/1.5.4/include").file("$root/benchmarks/codecs/avif/decode-dav1d.c").compile("qual");
    println!("cargo:rustc-link-search=native=$scratch/build/libavif");
    println!("cargo:rustc-link-search=native=$scratch/install/aom/lib");
    println!("cargo:rustc-link-search=native=/opt/homebrew/Cellar/dav1d/1.5.4/lib");
    println!("cargo:rustc-link-lib=static=avif");
    println!("cargo:rustc-link-lib=static=aom");
    println!("cargo:rustc-link-lib=dylib=dav1d");
}
RUST
cp "$root/Cargo.lock" "$scratch/qual-img/Cargo.lock"
CARGO_BUILD_JOBS=4 cargo build --offline --release --manifest-path "$scratch/qual-img/Cargo.toml" --target-dir "$scratch/img-target"
CARGO_BUILD_JOBS=4 cargo build --offline --release --manifest-path "$scratch/qual-dav1d/Cargo.toml" --target-dir "$scratch/qual-dav1d-target"
CARGO_BUILD_JOBS=4 cargo build --offline --release --manifest-path "$scratch/qual-zen/Cargo.toml" --target-dir "$scratch/qual-zen-target"
