#!/bin/sh
set -eu
scratch=${JPEG_SCRATCH:-/Users/josh/d/img/.work/tmp/jpeg-candidates}
repo=/Users/josh/d/img/.work/wt/jpeg-candidates
mkdir -p "$scratch"
git clone --depth 1 --branch 3.1.1 https://github.com/libjpeg-turbo/libjpeg-turbo.git "$scratch/libjpeg-turbo"
git clone --depth 1 --branch v4.1.5 https://github.com/mozilla/mozjpeg.git "$scratch/mozjpeg"
git clone --depth 1 --branch v0.11.1 --recurse-submodules --shallow-submodules https://github.com/libjxl/libjxl.git "$scratch/libjxl"
cmake -S "$scratch/libjpeg-turbo" -B "$scratch/turbo-build" -G Ninja -DCMAKE_BUILD_TYPE=Release -DENABLE_SHARED=OFF
cmake --build "$scratch/turbo-build" -j4
cmake --install "$scratch/turbo-build" --prefix "$scratch/turbo-install"
cmake -S "$scratch/libjxl" -B "$scratch/jxl-build" -G Ninja -DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF -DJPEGXL_ENABLE_BENCHMARK=OFF -DJPEGXL_ENABLE_EXAMPLES=OFF -DJPEGXL_ENABLE_MANPAGES=OFF -DJPEGXL_ENABLE_DOXYGEN=OFF -DJPEGXL_ENABLE_OPENEXR=OFF -DJPEGXL_ENABLE_SJPEG=OFF -DJPEGXL_BUNDLE_LIBPNG=ON -DBUILD_SHARED_LIBS=OFF -DCMAKE_DISABLE_FIND_PACKAGE_GIF=ON -DJPEG_LIBRARY="$scratch/turbo-install/lib/libjpeg.a" -DJPEG_INCLUDE_DIR="$scratch/turbo-install/include" '-DCMAKE_C_FLAGS=-include math.h' -DCMAKE_CXX_FLAGS="-I$scratch/jxl-build/third_party/libpng"
cmake --build "$scratch/jxl-build" --target cjpegli djpegli -j4
cmake -S "$scratch/mozjpeg" -B "$scratch/moz-local" -G Ninja -DCMAKE_POLICY_VERSION_MINIMUM=3.5 -DCMAKE_BUILD_TYPE=Release -DENABLE_SHARED=OFF -DPNG_PNG_INCLUDE_DIR="$scratch/libjxl/third_party/libpng" -DZLIB_INCLUDE_DIR="$scratch/libjxl/third_party/zlib" -DCMAKE_LIBRARY_PATH="$scratch/jxl-build/third_party/libpng;$scratch/jxl-build/third_party/zlib" -DCMAKE_C_FLAGS="-I$scratch/jxl-build/third_party/libpng -I$scratch/jxl-build/third_party/zlib"
cmake --build "$scratch/moz-local" -j4
mkdir -p "$scratch/decode"
cat > "$scratch/decode/Cargo.toml" <<EOF
[package]
name = "jpeg-candidate-decode"
version = "0.1.0"
edition = "2021"
[[bin]]
name = "jpeg-candidate-decode"
path = "$repo/benchmarks/codecs/jpeg/decode.rs"
[dependencies]
zune-jpeg = "=0.5.15"
zenjpeg = "=0.8.4"
enough = "=0.4.5"
EOF
cargo build --offline --release -j4 --manifest-path "$scratch/decode/Cargo.toml" --target-dir "$scratch/decode-target"
mkdir -p "$scratch/encode"
cat > "$scratch/encode/Cargo.toml" <<EOF
[package]
name = "jpeg-candidate-encode"
version = "0.1.0"
edition = "2021"
[[bin]]
name = "jpeg-candidate-encode"
path = "$repo/benchmarks/codecs/jpeg/encode.rs"
[dependencies]
mozjpeg-rs = "=0.9.2"
zenjpeg = "=0.8.4"
enough = "=0.4.5"
libc = "=0.2.183"
EOF
cargo build --offline --release -j4 --manifest-path "$scratch/encode/Cargo.toml" --target-dir "$scratch/decode-target"
c++ -std=c++17 -O3 -DUSE_JPEGLI -I"$scratch/libjxl" -I"$scratch/turbo-install/include" "$repo/benchmarks/codecs/jpeg/encode.cc" "$scratch/jxl-build/lib/libjpegli-static.a" "$scratch/jxl-build/third_party/highway/libhwy.a" -o "$scratch/encode-jpegli"
c++ -std=c++17 -O3 -I"$scratch/mozjpeg" -I"$scratch/moz-local" "$repo/benchmarks/codecs/jpeg/encode.cc" "$scratch/moz-local/libjpeg.a" -o "$scratch/encode-moz"
