#[path = "../tests/common/mod.rs"]
mod common;

#[rustybench::bench]
fn png_file_conversion() {
    let binary = std::env::var_os("RIMAGE_BENCH_BIN").map(std::path::PathBuf::from)
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_rimage").into());
    common::png_workflow(&binary);
}

fn main() { rustybench::main(); }
