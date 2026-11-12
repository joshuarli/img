#![cfg(feature="build-binary")]
mod common;
use std::{fs,process::{Command,Stdio},time::{Instant,Duration}};
#[test]
fn multiple_files_single_worker_preserve_layout_without_deadlock() {
    let scratch=common::Scratch::new("one-worker");let root=scratch.0.join("input");fs::create_dir_all(root.join("sub")).unwrap();for name in ["a.png","sub/b.png"] {common::write_png(&root.join(name),16,16);}
    let out=scratch.0.join("out");let mut child=Command::new(env!("CARGO_BIN_EXE_rimage")).arg("convert").arg(&root).args(["--recursive","--out-dir"]).arg(&out).args(["--format","png","--lossless","--jobs","1","--quiet"]).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let start=Instant::now();let status=loop {if let Some(status)=child.try_wait().unwrap() {break status;}if start.elapsed()>Duration::from_secs(30) {child.kill().unwrap();child.wait().unwrap();panic!("single-worker batch exceeded deadline");}std::thread::sleep(Duration::from_millis(5));};
    assert!(status.success());assert!(out.join("a.converted.png").exists());assert!(out.join("sub/b.converted.png").exists());
}
#[test]
fn quoted_globs_and_explicit_relative_file_lists_expand_safely() {
    let scratch=common::Scratch::new("lists-globs");for name in ["first.png","second.png","ignored.png"] {common::write_png(&scratch.0.join(name),16,16);}
    let list=scratch.0.join("selected.list");fs::write(&list,"first.png\nsecond.png\n").unwrap();let out=scratch.0.join("list-out");
    let result=Command::new(env!("CARGO_BIN_EXE_rimage")).args(["convert","--file-list"]).arg(&list).arg("--out-dir").arg(&out).args(["--format","png","--lossless","--quiet"]).output().unwrap();assert!(result.status.success(),"{}",String::from_utf8_lossy(&result.stderr));
    assert!(out.join("first.converted.png").exists());assert!(out.join("second.converted.png").exists());assert!(!out.join("ignored.converted.png").exists());
    let glob=scratch.0.join("*.png");let out=scratch.0.join("glob-out");let result=Command::new(env!("CARGO_BIN_EXE_rimage")).arg("convert").arg(glob).arg("--out-dir").arg(&out).args(["--format","png","--lossless","--quiet"]).output().unwrap();assert!(result.status.success());assert_eq!(fs::read_dir(out).unwrap().count(),3);
}
