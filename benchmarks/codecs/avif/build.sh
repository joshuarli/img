#!/bin/sh
set -eu
repo=$(CDPATH= cd -- "$(dirname "$0")/../../.." && pwd)
scratch=${AVIF_SCRATCH:-$repo/.work/tmp/avif-candidates}
mkdir -p "$scratch/src" "$scratch/build" "$scratch/install"
cd "$scratch/src"

fetch() {
    name=$1; url=$2; sha=$3
    [ -f "$name" ] || curl -sSL -o "$name" "$url"
    echo "$sha  $name" | shasum -a 256 -c -
}

[ -d libaom-v3.13.1/.git ] || git clone --depth 1 --branch v3.13.1 https://aomedia.googlesource.com/aom libaom-v3.13.1
[ "$(git -C libaom-v3.13.1 rev-parse HEAD)" = d772e334cc724105040382a977ebb10dfd393293 ]
fetch dav1d-1.5.4.tar.gz "https://github.com/videolan/dav1d/archive/refs/tags/1.5.4.tar.gz" a1d5b63d2d38ec9bd03acf643caa51fa22edd1e89c5a109c4807717216bbec07
fetch libavif-v1.4.2.tar.gz "https://github.com/AOMediaCodec/libavif/archive/refs/tags/v1.4.2.tar.gz" 2b645287340ba5a631d268b551dc2d72bd73ac33335962dd36dcdb6d8366921d
[ "${DECODE_ONLY:-0}" = 1 ] || fetch svt-av1-v4.2.0.tar.gz "https://github.com/AOMediaCodec/SVT-AV1/archive/refs/tags/v4.2.0.tar.gz" d9cb8d5fccbed565d038ae304cb00c5610f1f62388868102ca22132e7844921b
fetch libjpeg-turbo-3.1.1.tar.gz "https://github.com/libjpeg-turbo/libjpeg-turbo/archive/refs/tags/3.1.1.tar.gz" 304165ae11e64ab752e9cfc07c37bfdc87abd0bfe4bc699e59f34036d9c84f72

[ -d dav1d-1.5.4 ] || { mkdir dav1d-1.5.4 && tar xzf dav1d-1.5.4.tar.gz -C dav1d-1.5.4 --strip-components=1; }
[ -d libavif-v1.4.2 ] || { mkdir libavif-v1.4.2 && tar xzf libavif-v1.4.2.tar.gz -C libavif-v1.4.2 --strip-components=1; }
[ "${DECODE_ONLY:-0}" = 1 ] || [ -d svt-av1-v4.2.0 ] || { mkdir svt-av1-v4.2.0 && tar xzf svt-av1-v4.2.0.tar.gz -C svt-av1-v4.2.0 --strip-components=1; }
[ -d libjpeg-turbo-3.1.1 ] || { mkdir libjpeg-turbo-3.1.1 && tar xzf libjpeg-turbo-3.1.1.tar.gz -C libjpeg-turbo-3.1.1 --strip-components=1; }

cmake -S libaom-v3.13.1 -B "$scratch/build/aom" -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DCONFIG_AV1_ENCODER=1 -DCONFIG_AV1_DECODER=1 -DENABLE_TESTS=0 -DENABLE_EXAMPLES=0 -DENABLE_TOOLS=0 -DENABLE_DOCS=0 -DCONFIG_LOWBITDEPTH=1 -DCMAKE_INSTALL_PREFIX="$scratch/install/aom"
cmake --build "$scratch/build/aom" -j4
cmake --install "$scratch/build/aom"

"${MESON:-meson}" setup "$scratch/build/dav1d" dav1d-1.5.4 --buildtype=release --default-library=static -Denable_tools=false -Denable_tests=false -Denable_examples=false --prefix="$scratch/install/dav1d"
ninja -C "$scratch/build/dav1d" -j4 install

cmake -S libjpeg-turbo-3.1.1 -B "$scratch/build/turbo" -G Ninja -DCMAKE_BUILD_TYPE=Release -DENABLE_SHARED=OFF -DWITH_JAVA=OFF -DWITH_TURBOJPEG=OFF
cmake --build "$scratch/build/turbo" --target djpeg-static -j4

export PKG_CONFIG_PATH="$scratch/install/aom/lib/pkgconfig:$scratch/install/dav1d/lib/pkgconfig"
cmake -S libavif-v1.4.2 -B "$scratch/build/libavif" -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DAVIF_CODEC_AOM=SYSTEM -DAVIF_CODEC_DAV1D=SYSTEM -DAVIF_LIBYUV=OFF -DAVIF_BUILD_APPS=OFF -DAVIF_BUILD_TESTS=OFF
cmake --build "$scratch/build/libavif" -j4

cc -O2 -DNDEBUG -I "$scratch/install/aom/include" -I "$scratch/install/dav1d/include" -I libavif-v1.4.2/include \
    "$repo/benchmarks/codecs/avif/avifcodec.c" "$scratch/build/libavif/libavif.a" \
    "$scratch/install/aom/lib/libaom.a" "$scratch/install/dav1d/lib/libdav1d.a" -lpthread -lm -o "$scratch/avifcodec"

[ "${DECODE_ONLY:-0}" = 1 ] && exit 0
cmake -S svt-av1-v4.2.0 -B "$scratch/build/svt" -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DBUILD_APPS=OFF -DBUILD_TESTING=OFF -DCMAKE_INSTALL_PREFIX="$scratch/install/svt"
cmake --build "$scratch/build/svt" -j4
cmake --install "$scratch/build/svt"

export PKG_CONFIG_PATH="$PKG_CONFIG_PATH:$scratch/install/svt/lib/pkgconfig"
cmake -S libavif-v1.4.2 -B "$scratch/build/libavif-svt" -G Ninja -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF -DAVIF_CODEC_AOM=SYSTEM -DAVIF_CODEC_DAV1D=SYSTEM -DAVIF_CODEC_SVT=SYSTEM -DAVIF_LIBYUV=OFF -DAVIF_BUILD_APPS=OFF -DAVIF_BUILD_TESTS=OFF
cmake --build "$scratch/build/libavif-svt" -j4

cc -O2 -DNDEBUG -I "$scratch/install/aom/include" -I "$scratch/install/dav1d/include" -I "$scratch/install/svt/include" -I libavif-v1.4.2/include \
    "$repo/benchmarks/codecs/avif/avifcodec.c" "$scratch/build/libavif-svt/libavif.a" \
    "$scratch/install/aom/lib/libaom.a" "$scratch/install/dav1d/lib/libdav1d.a" "$scratch/install/svt/lib/libSvtAv1Enc.a" \
    -lpthread -lm -o "$scratch/avifcodec-svt"

mkdir -p "$scratch/encode" "$scratch/decode"
cat > "$scratch/encode/Cargo.toml" <<EOF
[package]
name = "avif-candidate-encode"
version = "0.1.0"
edition = "2021"
[[bin]]
name = "avif-candidate-encode"
path = "$repo/benchmarks/codecs/avif/encode.rs"
[dependencies]
ravif = { version = "=0.13.0", default-features = false }
zenavif = { version = "=0.1.6", default-features = false, features = ["encode"] }
imgref = "=1.12.3"
rgb = "=0.8.53"
almost-enough = "=0.4.5"
libc = "=0.2.183"
EOF
cat > "$scratch/decode/Cargo.toml" <<EOF
[package]
name = "avif-candidate-decode"
version = "0.1.0"
edition = "2021"
[[bin]]
name = "avif-candidate-decode"
path = "$repo/benchmarks/codecs/avif/decode.rs"
[dependencies]
rav1d = { version = "=1.1.0", default-features = false, features = ["bitdepth_8"] }
yuv = "=0.8.20"
zenavif = { version = "=0.1.6", default-features = false }
libc = "=0.2.183"
EOF
cargo build --release -j4 --manifest-path "$scratch/encode/Cargo.toml" --target-dir "$scratch/encode-target"
cargo build --release -j4 --manifest-path "$scratch/decode/Cargo.toml" --target-dir "$scratch/decode-target"
mkdir -p "$scratch/decode-asm"
cat > "$scratch/decode-asm/Cargo.toml" <<EOF
[package]
name = "avif-candidate-decode"
version = "0.1.0"
edition = "2021"
[[bin]]
name = "avif-candidate-decode"
path = "$repo/benchmarks/codecs/avif/decode.rs"
[dependencies]
rav1d = { path = "$repo/vendor/rav1d", default-features = false, features = ["bitdepth_8", "bitdepth_16", "asm", "asm_arm64_dotprod", "asm_arm64_i8mm", "asm_arm64_sve2"] }
yuv = "=0.8.20"
zenavif = { version = "=0.1.6", default-features = false }
libc = "=0.2.183"
EOF
cargo build --release -j4 --manifest-path "$scratch/decode-asm/Cargo.toml" --target-dir "$scratch/decode-asm-target"

mkdir -p "$scratch/integrated"
cat > "$scratch/integrated/Cargo.toml" <<EOF
[package]
name = "libaom-integrated-measure"
version = "0.1.0"
edition = "2024"
[[bin]]
name = "encode-integrated"
path = "$repo/benchmarks/codecs/avif/encode-integrated.rs"
[dependencies]
img = { path = "$repo", features = ["build-binary"] }
libc = "0.2"
[profile.dist]
inherits = "release"
lto = "fat"
codegen-units = 1
strip = true
[patch.crates-io]
oxipng = { path = "$repo/vendor/oxipng" }
libdeflater = { path = "$repo/vendor/libdeflater" }
libdeflate-sys = { path = "$repo/vendor/libdeflate-sys" }
EOF
cp "$repo/Cargo.lock" "$scratch/integrated/Cargo.lock"
cargo build --profile dist -j4 --manifest-path "$scratch/integrated/Cargo.toml" --target-dir "$scratch/integrated-target"
