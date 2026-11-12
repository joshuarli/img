#![cfg(feature = "build-binary")]

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_rimage"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn commands_and_aliases_have_help_without_files() {
    for command in ["avif", "farbfeld", "jpeg", "jpg", "jpeg_xl", "jxl", "mozjpeg", "moz", "oxipng", "oxi", "png", "ppm", "qoi", "webp"] {
        let output = run(&[command, "--help"]);
        assert!(output.status.success(), "{command}: {}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stdout).contains("--help"));
    }
}

#[test]
fn invalid_arguments_fail_before_processing() {
    for args in [
        &["png", "--unknown", "file.png"][..],
        &["png", "--resize", "0x0", "file.png"],
        &["jpeg", "--quality", "101", "file.png"],
        &["webp", "--lossless", "--quality", "75", "file.png"],
        &["png", "--recursive", "file.png"],
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn version_and_top_level_help_succeed() {
    for arg in ["--version", "--help", "-h", "help"] {
        let output = run(&[arg]);
        assert!(output.status.success(), "{arg}");
        assert!(!output.stdout.is_empty(), "{arg}");
    }
}

#[test]
fn help_subcommand_accepts_aliases() {
    let output = run(&["help", "jpg"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("rimage jpeg"));
}
