use std::{env, path::PathBuf};

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let turbo = root.join("vendor/libjpeg-turbo");
    println!("cargo:rerun-if-changed={}", turbo.display());
    println!("cargo:rerun-if-changed=shim.c");
    let dst = cmake::Config::new(&turbo)
        .profile("Release")
        .define("ENABLE_SHARED", "OFF")
        .define("ENABLE_STATIC", "ON")
        .define("REQUIRE_SIMD", "ON")
        .define("WITH_TURBOJPEG", "OFF")
        .define("WITH_TOOLS", "OFF")
        .define("WITH_TESTS", "OFF")
        .define("WITH_ARITH_DEC", "OFF")
        .define("WITH_ARITH_ENC", "OFF")
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
        .define("CMAKE_INSTALL_LIBDIR", "lib")
        .build();
    cc::Build::new().include(dst.join("include")).warnings(false).file("shim.c").compile("img_turbo");
    println!("cargo:rustc-link-search=native={}", dst.join("lib").display());
    println!("cargo:rustc-link-lib=static=jpeg");
}
