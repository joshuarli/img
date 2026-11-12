mod common;

use std::{fs, io::{self, Read}, os::unix::{ffi::OsStringExt, fs::symlink}, path::Path, sync::{Arc, Barrier}};
use rimage::{publication::{publish, Destination, Source}, still::{StillImage, ErrorKind, MAX_DIMENSION, MAX_INPUT_BYTES, Format, EncodingMode, Effort}};

#[test]
fn bounded_still_adapter_checks_actual_samples() {
    let scratch = common::Scratch::new("boundary-codec");
    let source = scratch.0.join("rgb.png");
    let samples = common::write_png(&source, 32, 24);
    let image = StillImage::decode(&fs::read(&source).unwrap()).unwrap();
    assert_eq!(image.dimensions(), (32, 24));
    assert_eq!(image.samples(), samples);
    let output = scratch.0.join("result.png");
    publish(&image.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap(), Destination::New(&output), None).unwrap();
    common::check_png(&output, 32, 24, &samples);
    let rgb: Vec<_> = samples.chunks_exact(3).map(|p| rgb::RGB8::new(p[0], p[1], p[2])).collect();
    let direct = zenpng::encode_rgb8(imgref::Img::new(rgb, 32, 24).as_ref(), None,
        &zenpng::EncodeConfig::default(), &enough::Unstoppable, &enough::Unstoppable).unwrap();
    let direct_image = StillImage::decode(&direct).unwrap();
    assert_eq!(direct_image.samples(), samples);
    for (w, h) in [(0, 1), (1, 0), (u32::MAX, u32::MAX), (MAX_DIMENSION + 1, 1), (4097, 4097)] {
        assert!(StillImage::from_rgb8(w, h, vec![]).is_err());
    }
    assert!(StillImage::from_rgb8(2, 2, vec![0; 11]).is_err());
    assert_eq!(StillImage::from_rgb8(16_384, 1, vec![]).err().unwrap().kind(), ErrorKind::Invalid);
    assert_eq!(StillImage::from_rgb8(16_385, 1, vec![]).err().unwrap().kind(), ErrorKind::Limit);
    assert_eq!(StillImage::from_rgb8(4096, 4096, vec![]).err().unwrap().kind(), ErrorKind::Invalid);
    assert_eq!(StillImage::from_rgb8(4096, 4097, vec![]).err().unwrap().kind(), ErrorKind::Limit);
    assert!(StillImage::decode(b"not an image").is_err());
    let mut bytes = fs::read(&source).unwrap();
    bytes.truncate(bytes.len() / 2);
    assert!(StillImage::decode(&bytes).is_err());
}

#[test]
fn unwritable_destination_fails_without_residue() {
    use std::os::unix::fs::PermissionsExt;
    // SAFETY: geteuid takes no arguments and has no memory preconditions.
    assert_ne!(unsafe { libc::geteuid() }, 0, "permission proof requires an unprivileged test process");
    let scratch = common::Scratch::new("boundary-permissions");
    let dir = scratch.0.join("locked");
    fs::create_dir(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).unwrap();
    let result = publish(b"output", Destination::New(&dir.join("output")), None);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(!result.unwrap_err().committed());
    assert_eq!(fs::read_dir(dir).unwrap().count(), 0);
}

#[test]
fn input_cap_is_mandatory_for_streams() {
    let error = rimage::still::read_bounded(io::repeat(0).take(MAX_INPUT_BYTES + 1)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Limit);
    assert_eq!(rimage::still::read_bounded(&b"abc"[..]).unwrap(), b"abc");
}

#[test]
fn no_clobber_is_atomic_under_competing_writers() {
    let scratch = common::Scratch::new("boundary-race");
    let target = scratch.0.join("output.bin");
    let barrier = Arc::new(Barrier::new(3));
    let workers: Vec<_> = [b"first".as_slice(), b"second".as_slice()].into_iter().map(|bytes| {
        let barrier = barrier.clone(); let target = target.clone();
        std::thread::spawn(move || { barrier.wait(); publish(bytes, Destination::New(&target), None) })
    }).collect();
    barrier.wait();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    let bytes = fs::read(&target).unwrap();
    assert!(bytes == b"first" || bytes == b"second");
    assert_eq!(fs::read_dir(&scratch.0).unwrap().count(), 1);
    assert!(!results.into_iter().find_map(Result::err).unwrap().committed());
}

#[test]
fn replacement_requires_unchanged_open_source() {
    let scratch = common::Scratch::new("boundary-replace");
    let target = scratch.0.join("source.bin");
    fs::write(&target, b"original").unwrap();
    let source = Source::open(&target).unwrap();
    publish(b"changed", Destination::Replace(&source), None).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"changed");
    assert!(publish(b"stale", Destination::Replace(&source), None).is_err());
    let source = Source::open(&target).unwrap();
    fs::rename(&target, scratch.0.join("moved")).unwrap();
    fs::write(&target, b"another file").unwrap();
    assert!(publish(b"stale", Destination::Replace(&source), None).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"another file");
}

#[test]
fn symlinks_directories_aliases_and_native_names_are_safe() {
    let scratch = common::Scratch::new("boundary-paths");
    let target = scratch.0.join("original");
    fs::write(&target, b"original").unwrap();
    let link = scratch.0.join("link");
    symlink(&target, &link).unwrap();
    assert!(Source::open(&link).is_err());
    assert!(publish(b"bad", Destination::New(&link), None).is_err());
    let alias = scratch.0.join("alias");
    fs::hard_link(&target, &alias).unwrap();
    assert_eq!(Source::open(&target).unwrap().file_id(), Source::open(&alias).unwrap().file_id());
    assert!(publish(b"bad", Destination::New(&alias), None).is_err());
    assert!(publish(b"bad", Destination::New(&scratch.0), None).is_err());
    let dir_link = scratch.0.join("dir-link");
    symlink(&scratch.0, &dir_link).unwrap();
    assert!(publish(b"bad", Destination::New(&dir_link.join("escape")), None).is_err());
    assert!(Source::open(&dir_link.join("original")).is_err());
    let native = scratch.0.join(std::ffi::OsString::from_vec(b"space ! \xff.png".to_vec()));
    match fs::write(&native, b"filesystem probe") {
        Err(error) if error.raw_os_error() == Some(libc::EILSEQ) => {
            assert!(!publish(b"native path", Destination::New(&native), None).unwrap_err().committed());
        },
        result => {
            result.unwrap();
            fs::remove_file(&native).unwrap();
            publish(b"native path", Destination::New(&native), None).unwrap();
            assert_eq!(fs::read(native).unwrap(), b"native path");
        },
    }
    assert_eq!(fs::read(target).unwrap(), b"original");
    assert!(!Path::new(&scratch.0.join("escape")).exists());
}

#[test]
fn cancelled_publication_has_no_visible_destination() {
    let scratch=common::Scratch::new("cancel-publication");let output=scratch.0.join("output.png");
    let cancellation=rimage::cancellation::Cancellation::default();cancellation.cancel();
    let error=rimage::publication::publish_cancellable(b"complete bytes",rimage::publication::Destination::New(&output),None,&cancellation).unwrap_err();
    assert!(!error.committed());assert!(!output.exists());assert_eq!(std::fs::read_dir(&scratch.0).unwrap().count(),0);
}
