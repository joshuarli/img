#!/bin/sh
# Builds the in-process JPEG decode harness; IMG_BASE=<checkout> also builds an img-only baseline.
set -eu
scratch=${JPEG_SCRATCH:-/Users/josh/d/img/.work/tmp/jpeg-candidates}
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
decode_loop() {
  name=$1 img=$2 features=$3
  mkdir -p "$scratch/$name"
  turbo= direct_turbo= zune=
  if [ -d "$img/vendor/zune-jpeg" ]; then zune="zune-jpeg = { path = \"$img/vendor/zune-jpeg\" }"; fi
  if [ -n "$features" ]; then turbo="libjpeg-turbo-sys = { path = \"$img/codecs/libjpeg-turbo-sys\", optional = true }" direct_turbo=', "dep:libjpeg-turbo-sys"'; fi
  cat > "$scratch/$name/Cargo.toml" <<EOF4
[package]
name = "jpeg-decode-loop"
version = "0.1.0"
edition = "2024"
build = "build.rs"
[[bin]]
name = "jpeg-decode-loop"
path = "$root/benchmarks/codecs/jpeg/decode-loop.rs"
[features]
direct = ["dep:zune-jpeg", "dep:zune-core", "dep:zenjpeg", "dep:enough"$direct_turbo]
[dependencies]
img = { path = "$img", default-features = false }
zune-jpeg = { version = "=0.5.15", optional = true }
zune-core = { version = "=0.5.1", optional = true }
zenjpeg = { version = "=0.8.4", optional = true }
enough = { version = "=0.4.5", optional = true }
$turbo
[build-dependencies]
cc = "=1.4.7"
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
[patch.crates-io]
oxipng = { path = "$img/vendor/oxipng" }
libdeflater = { path = "$img/vendor/libdeflater" }
libdeflate-sys = { path = "$img/vendor/libdeflate-sys" }
rav1d = { path = "$img/vendor/rav1d" }
$zune
EOF4
  cat > "$scratch/$name/build.rs" <<EOF5
fn main() {
    if std::env::var_os("CARGO_FEATURE_DIRECT").is_none() { return; }
    cc::Build::new().cpp(true).std("c++17").opt_level(3).include("$scratch/libjxl").include("$scratch/jxl-build/lib/include/jpegli").include("$scratch/jxl-build/lib/include").file("$root/benchmarks/codecs/jpeg/decode-jpegli.cc").compile("decodejpegli");
    for d in ["$scratch/jxl-build/lib", "$scratch/jxl-build/third_party/highway"] { println!("cargo:rustc-link-search=native={d}"); }
    for l in ["jpegli-static", "hwy"] { println!("cargo:rustc-link-lib=static={l}"); }
    println!("cargo:rustc-link-lib=c++");
}
EOF5
  cp "$img/Cargo.lock" "$scratch/$name/Cargo.lock"
  cargo build --offline --release -j4 --manifest-path "$scratch/$name/Cargo.toml" --target-dir "$scratch/$name-target" --features "$features"
}
decode_loop decode-loop "$root" direct
if [ -n "${IMG_BASE:-}" ]; then decode_loop decode-loop-base "$IMG_BASE" ""; fi
