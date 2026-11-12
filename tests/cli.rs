#![cfg(feature="build-binary")]
mod common;
use std::{fs,io::Write,process::{Command,Output,Stdio}};
use serde_json::Value;
fn run(args:&[&std::ffi::OsStr])->Output {Command::new(env!("CARGO_BIN_EXE_rimage")).args(args).output().unwrap()}
fn args(args:&[&str])->Output {Command::new(env!("CARGO_BIN_EXE_rimage")).args(args).output().unwrap()}
fn success(output:Output)->Output {assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));output}
fn json(output:Output)->Value {serde_json::from_slice(&success(output).stdout).unwrap()}
#[test]
fn command_help_invalid_combinations_and_removed_surface() {
    for command in ["optimize","convert","inspect","compare"] {assert!(String::from_utf8(success(args(&[command,"--help"])).stdout).unwrap().contains("--score"));}
    for argument in ["--help","-h","help","--version"] {success(args(&[argument]));}
    for invalid in [
        vec!["png","file.png"],vec!["jpg","--help"],vec!["optimize","x","--unknown"],
        vec!["optimize","x","--score","NaN"],vec!["optimize","x","--score","101"],vec!["convert","x","--quality","inf"],
        vec!["optimize","x","--quality","50","--score","80"],vec!["optimize","x","--lossless","--score","80"],
        vec!["optimize","x","--lossless","--max-dimension","8"],vec!["convert","x","--max-dimension","0"],
        vec!["convert","x","--max-dimension","4294967296"],vec!["optimize","x","--jobs","0"],
        vec!["optimize","x","--quiet","--verbose"],vec!["optimize","x","--format","auto","--in-place"],
        vec!["convert","x","--format","auto"],vec!["optimize","x","--in-place","-o","x.png"],
        vec!["convert","x","-o","x.png","--out-dir","out"],vec!["inspect","x","--quality","80"],
        vec!["compare","x"],vec!["optimize","-","--in-place"],vec!["convert","-","--format","png","-o","-","--json"],
        vec!["convert","-","-o","-"],vec!["optimize","x","--format","auto","-o","out.png"],
    ] {let output=args(&invalid);assert_eq!(output.status.code(),Some(2),"{invalid:?}: {}",String::from_utf8_lossy(&output.stderr));}
}
#[test]
fn optimize_defaults_identity_conversion_inference_and_json_schema() {
    let scratch=common::Scratch::new("cli-operations");let input=scratch.0.join("space ! café.png");let samples=common::write_png(&input,48,32);
    let output=json(run(&["optimize".as_ref(),input.as_os_str(),"--json".as_ref(),"--effort".as_ref(),"fast".as_ref()]));
    assert_eq!(output["schemaVersion"],1);assert_eq!(output["failures"].as_array().unwrap().len(),0);
    let optimized=input.with_file_name("space ! café.optimized.png");common::check_png(&optimized,48,32,&samples);
    for field in ["input","output","format","inputBytes","outputBytes","score","elapsedMs","status","warnings"] {assert!(output["results"][0].get(field).is_some(),"{field}");}
    assert!(fs::metadata(&optimized).unwrap().len()<=fs::metadata(&input).unwrap().len());
    let destination=scratch.0.join("converted.webp");
    let converted=json(run(&["convert".as_ref(),input.as_os_str(),"-o".as_ref(),destination.as_os_str(),"--json".as_ref(),"--effort".as_ref(),"fast".as_ref()]));
    assert!(converted["results"][0]["score"].as_f64().unwrap()>=95.0);
    let compared=json(run(&["compare".as_ref(),input.as_os_str(),destination.as_os_str(),"--json".as_ref()]));assert!(compared["results"][0]["score"].as_f64().unwrap()>=95.0);assert_eq!(compared["results"][0]["status"],"compared");
    let identity=json(run(&["compare".as_ref(),input.as_os_str(),input.as_os_str(),"--json".as_ref()]));assert_eq!(identity["results"][0]["score"],100.0);
    let inspected=json(run(&["inspect".as_ref(),input.as_os_str(),"--json".as_ref()]));assert_eq!(inspected["results"][0]["image"]["width"],48);assert_eq!(inspected["results"][0]["output"],Value::Null);
    let mismatch=run(&["convert".as_ref(),input.as_os_str(),"-o".as_ref(),scratch.0.join("mismatch.jpg").as_os_str(),"--format".as_ref(),"webp".as_ref()]);assert_eq!(mismatch.status.code(),Some(2));
    let unchanged=fs::read(&input).unwrap();success(run(&["optimize".as_ref(),input.as_os_str(),"--lossless".as_ref(),"--in-place".as_ref(),"--quiet".as_ref()]));assert_eq!(rimage::still::StillImage::decode(&fs::read(&input).unwrap()).unwrap().samples(),samples);assert!(!unchanged.is_empty());
}
#[test]
fn explicit_resize_matches_new_reference_and_never_enlarges() {
    let scratch=common::Scratch::new("cli-resize");let input=scratch.0.join("source.png");common::write_png(&input,48,32);
    let output=scratch.0.join("small.webp");success(run(&["convert".as_ref(),input.as_os_str(),"-o".as_ref(),output.as_os_str(),"--max-dimension".as_ref(),"24".as_ref(),"--effort".as_ref(),"fast".as_ref()]));
    assert_eq!(rimage::still::StillImage::decode(&fs::read(output).unwrap()).unwrap().dimensions(),(24,16));
}
#[test]
fn compare_accepts_matching_display_dimensions_after_orientation() {
    let scratch=common::Scratch::new("compare-orientation");let reference=scratch.0.join("reference.png");let candidate=scratch.0.join("candidate.png");
    let mut info=png::Info::with_size(8,16);info.color_type=png::ColorType::Rgb;
    let mut exif=b"II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x01\0\0\0\0\0\0\0".to_vec();exif[18]=6;info.exif_metadata=Some(exif.into());
    png::Encoder::with_info(fs::File::create(&reference).unwrap(),info).unwrap().write_header().unwrap().write_image_data(&[128;8*16*3]).unwrap();
    let mut encoder=png::Encoder::new(fs::File::create(&candidate).unwrap(),16,8);encoder.set_color(png::ColorType::Rgb);encoder.write_header().unwrap().write_image_data(&[128;8*16*3]).unwrap();
    let report=json(run(&["compare".as_ref(),reference.as_os_str(),candidate.as_os_str(),"--json".as_ref()]));assert_eq!(report["results"][0]["score"],100.0);
}
#[test]
fn file_list_and_existing_positional_paths_are_literal() {
    let scratch=common::Scratch::new("literal-inputs");
    for (listed,other) in [("photo[1].png","photo1.png"),("photo*.png","photoX.png"),("photo?.png","photoY.png")] {
        let listed=scratch.0.join(listed);let other=scratch.0.join(other);
        common::write_png(&listed,32,32);common::write_png(&other,32,32);
        let before=fs::read(&other).unwrap();let list=scratch.0.join("selected.list");
        fs::write(&list,format!("{}\n",listed.file_name().unwrap().to_str().unwrap())).unwrap();
        success(run(&["optimize".as_ref(),"--file-list".as_ref(),list.as_os_str(),"--in-place".as_ref(),"--max-dimension".as_ref(),"8".as_ref(),"--score".as_ref(),"0".as_ref(),"--effort".as_ref(),"fast".as_ref()]));
        assert_eq!(rimage::still::StillImage::decode(&fs::read(&listed).unwrap()).unwrap().dimensions(),(8,8));
        assert_eq!(fs::read(&other).unwrap(),before);
        let report=json(run(&["inspect".as_ref(),listed.as_os_str(),"--json".as_ref()]));
        assert_eq!(report["results"][0]["input"],listed.to_str().unwrap());
    }
    common::write_png(&scratch.0.join("-"),8,8);fs::write(scratch.0.join("selected.list"),"-\n").unwrap();
    let output=Command::new(env!("CARGO_BIN_EXE_rimage")).current_dir(&scratch.0).args(["inspect","--file-list","selected.list","--json"]).stdin(Stdio::null()).output().unwrap();
    assert_eq!(json(output)["results"][0]["image"]["width"],8);
}

#[test]
fn in_place_conflicts_with_an_existing_directory_writer() {
    use std::os::fd::AsRawFd;
    let scratch=common::Scratch::new("in-place-ownership");let input=scratch.0.join("source.png");common::write_png(&input,32,32);
    let other=scratch.0.join("other.png");common::write_png(&other,32,32);
    let original=fs::read(&input).unwrap();let other_original=fs::read(&other).unwrap();let owner=fs::File::open(&scratch.0).unwrap();
    // SAFETY: owner is an open directory descriptor held for the duration of the test.
    assert_eq!(unsafe {libc::flock(owner.as_raw_fd(),libc::LOCK_EX|libc::LOCK_NB)},0);
    let command=["optimize".as_ref(),input.as_os_str(),"--in-place".as_ref(),"--max-dimension".as_ref(),"8".as_ref(),"--score".as_ref(),"0".as_ref(),"--effort".as_ref(),"fast".as_ref(),"--json".as_ref()];
    let output=run(&command);assert_eq!(output.status.code(),Some(3));
    let report:Value=serde_json::from_slice(&output.stdout).unwrap();assert_eq!(report["failures"][0]["kind"],"input.conflict");assert_eq!(fs::read(&input).unwrap(),original);
    let output=run(&["optimize".as_ref(),other.as_os_str(),"--in-place".as_ref(),"--lossless".as_ref(),"--json".as_ref()]);assert_eq!(output.status.code(),Some(3));
    let report:Value=serde_json::from_slice(&output.stdout).unwrap();assert_eq!(report["failures"][0]["kind"],"input.conflict");assert_eq!(fs::read(&other).unwrap(),other_original);
    drop(owner);success(run(&command));assert_eq!(rimage::still::StillImage::decode(&fs::read(&input).unwrap()).unwrap().dimensions(),(8,8));
}

#[test]
fn replacement_owner_keeps_the_lock_across_commit_until_drop() {
    use rimage::{publication::{publish,Destination,ReplacementSource},still::{Effort,EncodingMode,Format,StillImage}};
    let scratch=common::Scratch::new("in-place-commit-lock");let input=scratch.0.join("source.png");common::write_png(&input,32,32);
    let owner=ReplacementSource::open(&input).unwrap();
    let replacement=StillImage::from_rgb8(16,16,common::pixels(16,16)).unwrap().encode(Format::Png,EncodingMode::Lossless,Effort::Fast).unwrap();
    let command=["optimize".as_ref(),input.as_os_str(),"--in-place".as_ref(),"--max-dimension".as_ref(),"8".as_ref(),"--score".as_ref(),"0".as_ref(),"--effort".as_ref(),"fast".as_ref(),"--json".as_ref()];
    assert_eq!(run(&command).status.code(),Some(3));
    publish(&replacement,Destination::Replace(&owner),None).unwrap();assert_eq!(fs::read(&input).unwrap(),replacement);
    assert_eq!(run(&command).status.code(),Some(3));assert_eq!(fs::read(&input).unwrap(),replacement);
    drop(owner);success(run(&command));assert_eq!(StillImage::decode(&fs::read(&input).unwrap()).unwrap().dimensions(),(8,8));
}

#[test]
fn parallel_in_place_batch_serializes_same_directory_jobs() {
    let scratch=common::Scratch::new("in-place-batch");
    for index in 0..8 {common::write_png(&scratch.0.join(format!("{index}.png")),32,32);}
    let report=json(run(&["optimize".as_ref(),scratch.0.as_os_str(),"--in-place".as_ref(),"--jobs".as_ref(),"8".as_ref(),"--max-dimension".as_ref(),"8".as_ref(),"--score".as_ref(),"0".as_ref(),"--effort".as_ref(),"fast".as_ref(),"--json".as_ref()]));
    assert_eq!(report["results"].as_array().unwrap().len(),8);
    for index in 0..8 {assert_eq!(rimage::still::StillImage::decode(&fs::read(scratch.0.join(format!("{index}.png"))).unwrap()).unwrap().dimensions(),(8,8));}
}
#[test]
fn traversal_auto_order_output_exclusion_and_collisions() {
    let scratch=common::Scratch::new("cli-batch");let root=scratch.0.join("images");fs::create_dir_all(root.join("nested")).unwrap();
    for name in ["a.png","b.png","nested/c.png"] {common::write_png(&root.join(name),24,16);}
    fs::write(root.join("ignored.txt"),b"unrelated").unwrap();let out=root.join("out");
    let report=json(run(&["optimize".as_ref(),root.as_os_str(),"--recursive".as_ref(),"--out-dir".as_ref(),out.as_os_str(),"--jobs".as_ref(),"8".as_ref(),"--format".as_ref(),"auto".as_ref(),"--effort".as_ref(),"fast".as_ref(),"--json".as_ref()]));
    let results=report["results"].as_array().unwrap();assert_eq!(results.len(),3);let paths:Vec<_>=results.iter().map(|r|r["input"].as_str().unwrap()).collect();assert!(paths[0].ends_with("a.png") && paths[1].ends_with("b.png") && paths[2].ends_with("nested/c.png"));
    assert!(results.iter().all(|r|r["output"].as_str().unwrap().contains(".png.optimized.")));
    let out2=root.join("out2");let duplicate=run(&["convert".as_ref(),root.join("a.png").as_os_str(),root.join("a.png").as_os_str(),"--out-dir".as_ref(),out2.as_os_str(),"--format".as_ref(),"png".as_ref(),"--lossless".as_ref()]);assert_eq!(duplicate.status.code(),Some(3));assert!(!out2.exists());
    let exists=run(&["convert".as_ref(),root.join("a.png").as_os_str(),"-o".as_ref(),root.join("b.png").as_os_str(),"--lossless".as_ref()]);assert_eq!(exists.status.code(),Some(4));
}
#[test]
fn streams_verify_before_binary_stdout_and_broken_pipe_is_output_failure() {
    let scratch=common::Scratch::new("cli-streams");let input=scratch.0.join("source.png");common::write_png(&input,24,16);let bytes=fs::read(&input).unwrap();
    let mut child=Command::new(env!("CARGO_BIN_EXE_rimage")).args(["convert","-","--format","webp","--lossless","-o","-","--quiet"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(&bytes).unwrap();let output=success(child.wait_with_output().unwrap());assert_eq!(rimage::still::Format::sniff(&output.stdout).unwrap(),rimage::still::Format::Webp);
    assert!(rimage::optimizer::samples_equal(&rimage::still::StillImage::decode(&bytes).unwrap(),&rimage::still::StillImage::decode(&output.stdout).unwrap()));
    let mut child=Command::new(env!("CARGO_BIN_EXE_rimage")).arg("convert").arg(&input).args(["--format","webp","--lossless","-o","-","--quiet"]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();drop(child.stdout.take());assert_eq!(child.wait().unwrap().code(),Some(4));
}
#[test]
fn failures_are_complete_ordered_and_exit_categories_include_mixed_zero_success() {
    let scratch=common::Scratch::new("cli-exits");let input=scratch.0.join("source.png");common::write_png(&input,16,16);let invalid=scratch.0.join("invalid.svg");fs::write(&invalid,b"<svg/>").unwrap();let out=scratch.0.join("out");fs::create_dir(&out).unwrap();fs::write(out.join("source.converted.png"),b"owner").unwrap();
    let output=run(&["convert".as_ref(),input.as_os_str(),invalid.as_os_str(),"--out-dir".as_ref(),out.as_os_str(),"--format".as_ref(),"png".as_ref(),"--lossless".as_ref(),"--json".as_ref()]);assert_eq!(output.status.code(),Some(5));let report:Value=serde_json::from_slice(&output.stdout).unwrap();assert!(report["results"].as_array().unwrap().is_empty());let failures=report["failures"].as_array().unwrap();assert_eq!(failures.len(),2);assert_eq!(failures[0]["exitCode"],4);assert_eq!(failures[1]["exitCode"],3);assert_eq!(fs::read(out.join("source.converted.png")).unwrap(),b"owner");
    let output=run(&["convert".as_ref(),input.as_os_str(),invalid.as_os_str(),"--out-dir".as_ref(),scratch.0.join("new").as_os_str(),"--format".as_ref(),"png".as_ref(),"--lossless".as_ref(),"--json".as_ref()]);assert_eq!(output.status.code(),Some(5));let report:Value=serde_json::from_slice(&output.stdout).unwrap();assert_eq!(report["results"].as_array().unwrap().len(),1);assert_eq!(report["failures"].as_array().unwrap().len(),1);
}
#[test]
fn symlinks_hardlinks_and_non_utf8_json_preflight_do_not_publish() {
    use std::os::unix::{ffi::OsStringExt,fs::symlink};
    let scratch=common::Scratch::new("cli-native-paths");let input=scratch.0.join("source.png");common::write_png(&input,16,16);let alias=scratch.0.join("alias.png");fs::hard_link(&input,&alias).unwrap();let out=scratch.0.join("out");
    assert_eq!(run(&["optimize".as_ref(),input.as_os_str(),alias.as_os_str(),"--out-dir".as_ref(),out.as_os_str(),"--lossless".as_ref()]).status.code(),Some(3));assert!(!out.exists());
    let link=scratch.0.join("link.png");symlink(&input,&link).unwrap();assert_eq!(run(&["inspect".as_ref(),link.as_os_str()]).status.code(),Some(3));
    let name=std::ffi::OsString::from_vec(b"invalid-\xff.png".to_vec());let native=scratch.0.join(name);
    match fs::copy(&input,&native) {Ok(_)=>{},Err(e) if e.raw_os_error()==Some(libc::EILSEQ)=>return,Err(e)=>panic!("{e}")}
    let json_output=run(&["convert".as_ref(),input.as_os_str(),native.as_os_str(),"--out-dir".as_ref(),out.as_os_str(),"--format".as_ref(),"png".as_ref(),"--lossless".as_ref(),"--json".as_ref()]);assert_eq!(json_output.status.code(),Some(2));assert!(!out.exists());
    success(run(&["convert".as_ref(),native.as_os_str(),"--format".as_ref(),"png".as_ref(),"--lossless".as_ref(),"--quiet".as_ref()]));
}
#[test]
fn inspect_describes_high_depth_and_animation_without_pixel_processing() {
    let scratch=common::Scratch::new("inspect-headers");let deep=scratch.0.join("deep.png");
    let mut encoder=png::Encoder::new(fs::File::create(&deep).unwrap(),8,8);encoder.set_color(png::ColorType::Grayscale);encoder.set_depth(png::BitDepth::Sixteen);encoder.write_header().unwrap().write_image_data(&vec![80;128]).unwrap();
    let report=json(run(&["inspect".as_ref(),deep.as_os_str(),"--json".as_ref()]));assert_eq!(report["results"][0]["image"]["bitDepth"],16);
    assert_eq!(run(&["compare".as_ref(),deep.as_os_str(),deep.as_os_str()]).status.code(),Some(3));
    let animated=scratch.0.join("animated.png");let mut encoder=png::Encoder::new(fs::File::create(&animated).unwrap(),8,8);encoder.set_color(png::ColorType::Rgb);encoder.set_animated(2,0).unwrap();let mut writer=encoder.write_header().unwrap();writer.write_image_data(&[80;192]).unwrap();writer.write_image_data(&[90;192]).unwrap();drop(writer);
    let report=json(run(&["inspect".as_ref(),animated.as_os_str(),"--json".as_ref()]));assert_eq!(report["results"][0]["image"]["animation"],true);
    assert_eq!(run(&["optimize".as_ref(),animated.as_os_str(),"--lossless".as_ref()]).status.code(),Some(3));assert!(!animated.with_file_name("animated.optimized.png").exists());
    let input=scratch.0.join("normal.png");common::write_png(&input,16,16);let output=success(run(&["compare".as_ref(),input.as_os_str(),input.as_os_str()]));assert_eq!(std::str::from_utf8(&output.stdout).unwrap().trim().parse::<f64>().unwrap(),100.0);
}

#[cfg(target_os="macos")]
#[test]
fn equivalent_case_and_unicode_destinations_fail_before_any_publication() {
    let scratch=common::Scratch::new("native-collisions");let probe=scratch.0.join("probe");fs::write(&probe,b"probe").unwrap();if !scratch.0.join("PROBE").exists() {return;}
    for (index,(a,b)) in [("foo.jpg","FOO.png"),("café.jpg","cafe\u{301}.png")].into_iter().enumerate() {
        let first=scratch.0.join(a);let second=scratch.0.join(b);common::write_png(&first,16,16);common::write_png(&second,16,16);
        let out=scratch.0.join(format!("out-{index}"));let output=run(&["convert".as_ref(),first.as_os_str(),second.as_os_str(),"--out-dir".as_ref(),out.as_os_str(),"--format".as_ref(),"png".as_ref(),"--lossless".as_ref(),"--json".as_ref()]);
        assert_eq!(output.status.code(),Some(4),"{}",String::from_utf8_lossy(&output.stdout));assert!(!out.exists(),"colliding outputs must all fail during preflight");
    }
}

#[test]
fn quiet_suppresses_diagnostics_but_keeps_comparison_data() {
    let scratch=common::Scratch::new("quiet-data");let input=scratch.0.join("source.png");common::write_png(&input,16,16);
    let output=success(run(&["compare".as_ref(),input.as_os_str(),input.as_os_str(),"--quiet".as_ref()]));assert_eq!(std::str::from_utf8(&output.stdout).unwrap().trim().parse::<f64>().unwrap(),100.0);assert!(output.stderr.is_empty());
}
#[cfg(target_os="macos")]
#[test]
fn case_variant_output_subtree_is_excluded_from_traversal() {
    let scratch=common::Scratch::new("output-subtree");let input=scratch.0.join("input");let out=input.join("out");fs::create_dir_all(&out).unwrap();if !input.join("OUT").exists() {return;}
    common::write_png(&input.join("source.png"),16,16);common::write_png(&out.join("previous.png"),16,16);
    let output=json(run(&["convert".as_ref(),input.as_os_str(),"--out-dir".as_ref(),input.join("OUT").as_os_str(),"--recursive".as_ref(),"--lossless".as_ref(),"--format".as_ref(),"png".as_ref(),"--json".as_ref()]));assert_eq!(output["results"].as_array().unwrap().len(),1);assert!(!out.join("out").exists());
}
