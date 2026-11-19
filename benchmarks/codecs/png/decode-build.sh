#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
scratch="$root/.work/tmp/png-qualify"
mkdir -p "$scratch" "$root/.work/tmp"
curl -fsSL https://github.com/randy408/libspng/archive/refs/tags/v0.7.4.tar.gz -o "$scratch/libspng.tar.gz"
printf '%s  %s\n' 47ec02be6c0a6323044600a9221b049f63e1953faf816903e7383d4dc4234487 "$scratch/libspng.tar.gz" | shasum -a 256 -c -
curl -fsSL https://github.com/richgel999/miniz/archive/refs/tags/2.2.0.tar.gz -o "$scratch/miniz.tar.gz"
printf '%s  %s\n' bd1136d0a1554520dcb527a239655777148d90fd2d51cf02c36540afc552e6ec "$scratch/miniz.tar.gz" | shasum -a 256 -c -
tar -xf "$scratch/libspng.tar.gz" -C "$scratch"
tar -xf "$scratch/miniz.tar.gz" -C "$scratch"
printf "#define MINIZ_EXPORT\n" > "$scratch/miniz-2.2.0/miniz_export.h"
cat > "$scratch/Cargo.toml" <<END
[package]
name = "png-decode-loop"
version = "0.1.0"
edition = "2024"
build = "build.rs"
[[bin]]
name = "png-decode-loop"
path = "$root/benchmarks/codecs/png/decode-loop.rs"
[dependencies]
png = "=0.18.1"
crc32fast = "=1.5.0"
zenpng = "=0.1.4"
enough = "=0.4.5"
[build-dependencies]
cc = "=1.4.7"
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
END
cat > "$scratch/build.rs" <<END
fn main() {
 let root = "$root"; let scratch = "$scratch";
 cc::Build::new().opt_level(3).include(format!("{scratch}/miniz-2.2.0")).include(format!("{scratch}/libspng-0.7.4/spng")).include("/opt/homebrew/include/libpng16").define("SPNG_USE_MINIZ",None).file(format!("{scratch}/miniz-2.2.0/miniz.c")).file(format!("{scratch}/miniz-2.2.0/miniz_tinfl.c")).file(format!("{scratch}/miniz-2.2.0/miniz_tdef.c")).file(format!("{scratch}/libspng-0.7.4/spng/spng.c")).file(format!("{root}/benchmarks/codecs/png/decode-native.c")).compile("pngnative");
 println!("cargo:rustc-link-search=native=/opt/homebrew/lib"); println!("cargo:rustc-link-lib=png16");
}
END
CARGO_BUILD_JOBS=4 TMPDIR="$root/.work/tmp" cargo build --offline --release --manifest-path "$scratch/Cargo.toml"
