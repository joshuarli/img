use std::{env, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let aom = root.join("vendor/libaom");
    let avif = root.join("vendor/libavif");
    println!("cargo:rerun-if-changed={}", aom.display());
    println!("cargo:rerun-if-changed={}", avif.display());
    println!("cargo:rerun-if-changed=shim.c");
    let dst = cmake::Config::new(&aom)
        .profile("Release")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("CONFIG_AV1_ENCODER", "1")
        .define("CONFIG_AV1_DECODER", "0")
        .define("ENABLE_NASM", "ON")
        .define("ENABLE_TESTS", "0")
        .define("ENABLE_EXAMPLES", "0")
        .define("ENABLE_TOOLS", "0")
        .define("ENABLE_DOCS", "0")
        .define("CONFIG_LIBYUV", "0")
        .define("CONFIG_WEBM_IO", "0")
        .build();
    let mut build = cc::Build::new();
    build.include(avif.join("include")).include(dst.join("include"))
        .define("AVIF_CODEC_AOM", "1").define("AVIF_CODEC_AOM_ENCODE", "1")
        .warnings(false);
    for name in ["alpha", "avif", "colr", "colrconvert", "diag", "exif", "gainmap", "io", "mem", "obu", "properties", "rawdata", "read", "reformat", "reformat_libsharpyuv", "reformat_libyuv", "sampletransform", "stream", "utils", "write", "codec_aom"] {
        build.file(avif.join("src").join(format!("{name}.c")));
    }
    build.file("shim.c").compile("avif");
    println!("cargo:rustc-link-search=native={}", dst.join("lib").display());
    println!("cargo:rustc-link-lib=static=aom");
    if env::var("CARGO_CFG_TARGET_OS").unwrap() != "windows" { println!("cargo:rustc-link-lib=m"); }
}
