#![cfg(feature="build-binary")]
mod common;
use std::{fs,process::Command};
use filetime::{FileTime,set_file_mtime};
#[test]
fn timestamps_are_preserved_only_when_requested() {
    let scratch=common::Scratch::new("timestamps");let input=scratch.0.join("source.png");common::write_png(&input,32,24);
    set_file_mtime(&input,FileTime::from_unix_time(946_684_800,123_456_789)).unwrap();let original=FileTime::from_last_modification_time(&fs::metadata(&input).unwrap());
    for preserve in [true,false] {
        let output=scratch.0.join(if preserve {"preserved.png"} else {"new.png"});
        let mut command=Command::new(env!("CARGO_BIN_EXE_rimage"));command.arg("convert").arg(&input).args(["--lossless","--quiet","-o"]).arg(&output);if preserve {command.arg("--preserve-timestamps");}
        assert!(command.status().unwrap().success());let actual=FileTime::from_last_modification_time(&fs::metadata(output).unwrap());
        if preserve {assert_eq!(actual,original);} else {assert!(actual>original);}
        assert_eq!(FileTime::from_last_modification_time(&fs::metadata(&input).unwrap()),original);
    }
}
