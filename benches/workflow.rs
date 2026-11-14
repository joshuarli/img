#[path = "../tests/common/mod.rs"]
mod common;

#[rustybench::bench]
fn png_file_conversion() {
    let binary = std::env::var_os("IMG_BENCH_BIN").map(std::path::PathBuf::from)
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_img").into());
    common::png_workflow(&binary);
}

fn main() { rustybench::main(); }

#[rustybench::bench]
fn startup_version() {
    let binary=std::env::var_os("IMG_BENCH_BIN").map(std::path::PathBuf::from).unwrap_or_else(||env!("CARGO_BIN_EXE_img").into());
    let output=std::process::Command::new(binary).arg("--version").output().unwrap();
    assert!(output.status.success());assert!(output.stdout.starts_with(b"img "));
}

#[rustybench::bench]
fn optimize_scored_file() {
    let scratch=common::Scratch::new("bench-scored");let input=scratch.0.join("source.png");common::write_png(&input,96,64);let output=scratch.0.join("result.png");
    let binary=std::env::var_os("IMG_BENCH_BIN").map(std::path::PathBuf::from).unwrap_or_else(||env!("CARGO_BIN_EXE_img").into());
    let result=std::process::Command::new(binary).arg("optimize").arg(&input).arg("-o").arg(&output).args(["--score","95","--effort","fast","--quiet"]).output().unwrap();
    assert!(result.status.success(),"{}",String::from_utf8_lossy(&result.stderr));
    let source=std::fs::read(&input).unwrap();let candidate=std::fs::read(output).unwrap();assert!(candidate.len()<=source.len());assert!(img::perceptual::compare(&source,&candidate).unwrap()>=95.0);
}

#[rustybench::bench]
fn lossless_batch() {
    let scratch=common::Scratch::new("bench-batch");let input=scratch.0.join("input");std::fs::create_dir(&input).unwrap();for index in 0..8 {common::write_png(&input.join(format!("{index}.png")),48,32);}
    let output=scratch.0.join("output");let binary=std::env::var_os("IMG_BENCH_BIN").map(std::path::PathBuf::from).unwrap_or_else(||env!("CARGO_BIN_EXE_img").into());
    let result=std::process::Command::new(binary).arg("convert").arg(&input).arg("--out-dir").arg(&output).args(["--format","png","--lossless","--jobs","2","--quiet"]).output().unwrap();assert!(result.status.success());
    for index in 0..8 {common::check_png(&output.join(format!("{index}.converted.png")),48,32,&common::pixels(48,32));}
    assert_eq!(std::fs::read_dir(output).unwrap().count(),8);
}
