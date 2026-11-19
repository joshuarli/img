use std::{env, fs, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let jxl = root.join("vendor/libjxl");
    let hwy = root.join("vendor/highway");
    println!("cargo:rerun-if-changed={}", jxl.display());
    println!("cargo:rerun-if-changed={}", hwy.display());
    println!("cargo:rerun-if-changed=shim.cc");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out.join("jconfig.h"), "#define JPEG_LIB_VERSION 62\n#define MEM_SRCDST_SUPPORTED 1\n#define BITS_IN_JSAMPLE 8\n").unwrap();
    let mut build = cc::Build::new();
    build.cpp(true).std("c++17").opt_level(3).warnings(false)
        .include(&jxl).include(jxl.join("lib/include")).include(&hwy)
        .include(jxl.join("third_party/libjpeg-turbo")).include(&out)
        .define("HWY_STATIC_DEFINE", None)
        .define("HWY_DISABLED_TARGETS", Some("(HWY_SSSE3|HWY_AVX3|HWY_AVX3_SPR|HWY_AVX3_ZEN4|HWY_SVE|HWY_SVE2|HWY_SVE_256|HWY_SVE2_128|HWY_RVV)"))
        .flag_if_supported("-fno-rtti");
    for name in ["adaptive_quantization", "bit_writer", "bitstream", "color_quantize", "color_transform", "common", "decode", "decode_marker", "decode_scan", "destination_manager", "downsample", "encode", "encode_finish", "encode_streaming", "entropy_coding", "error", "huffman", "idct", "input", "memory_manager", "quant", "render", "simd", "source_manager", "upsample"] {
        build.file(jxl.join("lib/jpegli").join(format!("{name}.cc")));
    }
    for name in ["abort", "aligned_allocator", "nanobenchmark", "per_target", "print", "targets", "timer"] {
        build.file(hwy.join("hwy").join(format!("{name}.cc")));
    }
    build.file("shim.cc");
    build.compile("jpegli");
}
