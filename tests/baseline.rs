#![cfg(feature = "build-binary")]

mod common;

use std::{fs, path::Path, process::Command};

fn binary() -> std::path::PathBuf {
    std::env::var_os("RIMAGE_ORACLE_BIN").map(Into::into)
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_rimage").into())
}

#[test]
fn real_png_workload_has_independent_sample_proof() {
    common::png_workflow(&binary());
}

#[test]
fn invalid_parser_and_backup_collision_preserve_inputs() {
    let scratch = common::Scratch::new("baseline-safety");
    let input = scratch.0.join("image.png");
    common::write_png(&input, 32, 24);
    let original = fs::read(&input).unwrap();
    let legacy=std::env::var_os("RIMAGE_ORACLE_BIN").is_some();
    let result = Command::new(binary()).arg(if legacy {"png"} else {"convert"}).arg("--unknown").arg(&input).output().unwrap();
    assert_eq!(result.status.code(), Some(2));
    let mut command=Command::new(binary());
    let expected=if legacy {command.arg("png").arg(&input).args(["-b", "-s", "@backup", "--quiet"]);3}
        else {command.arg("convert").arg(&input).args(["--lossless","-o"]).arg(&input);4};
    let result=command.output().unwrap();
    assert_eq!(result.status.code(), Some(expected));
    assert_eq!(fs::read(&input).unwrap(), original);
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
    assert!(!Path::new(&scratch.0.join("image@backup.png")).exists());
}
