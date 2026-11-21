mod common;

use std::{fs, io::{self, Read}, os::unix::{ffi::OsStringExt, fs::symlink}, path::Path, sync::{Arc, Barrier}};
use img::{publication::{create_parent, destination_key, publish, read_directory, Destination, ReplacementSource, Source}, still::{StillImage, ErrorKind, MAX_DIMENSION, MAX_INPUT_BYTES, Format, EncodingMode, Effort}};

#[test]
fn read_bounded_accepts_exactly_the_cap_and_refuses_one_more_byte() {
    let at_cap = img::still::read_bounded(io::repeat(7).take(MAX_INPUT_BYTES)).unwrap();
    assert_eq!(at_cap.len() as u64, MAX_INPUT_BYTES);
    assert!(at_cap.iter().all(|&byte| byte == 7));
    let over = img::still::read_bounded(io::repeat(7).take(MAX_INPUT_BYTES + 1)).unwrap_err();
    assert_eq!(over.kind(), ErrorKind::Limit);
    assert!(over.to_string().contains("128 MiB"), "{over}");
}

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
    let mut raw = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut raw, 32, 24);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&samples).unwrap();
    }
    let direct_image = StillImage::decode(&raw).unwrap();
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
    let error = img::still::read_bounded(io::repeat(0).take(MAX_INPUT_BYTES + 1)).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Limit);
    assert_eq!(img::still::read_bounded(&b"abc"[..]).unwrap(), b"abc");
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
    let source = ReplacementSource::open(&target).unwrap();
    publish(b"changed", Destination::Replace(&source), None).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"changed");
    assert!(publish(b"stale", Destination::Replace(&source), None).is_err());
    drop(source);
    let source = ReplacementSource::open(&target).unwrap();
    fs::rename(&target, scratch.0.join("moved")).unwrap();
    fs::write(&target, b"another file").unwrap();
    assert!(publish(b"stale", Destination::Replace(&source), None).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"another file");
}

#[test]
fn replacement_lock_is_released_when_its_process_exits() {
    use std::{io::{BufRead,BufReader},process::{Command,Stdio}};
    const OWNER_PATH:&str="IMG_TEST_REPLACEMENT_OWNER_PATH";
    if let Some(path)=std::env::var_os(OWNER_PATH) {
        let _owner=ReplacementSource::open(Path::new(&path)).unwrap();
        println!("replacement-owner-ready");
        let mut byte=[0];let _=io::stdin().read(&mut byte);
        return;
    }
    let scratch=common::Scratch::new("replacement-owner-exit");let target=scratch.0.join("source");fs::write(&target,b"source").unwrap();
    let mut child=Command::new(std::env::current_exe().unwrap()).args(["--exact","replacement_lock_is_released_when_its_process_exits","--nocapture"]).env(OWNER_PATH,&target).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    let mut output=BufReader::new(child.stdout.take().unwrap());
    loop {let mut line=String::new();assert!(output.read_line(&mut line).unwrap()>0,"owner process exited before acquiring lock");if line.contains("replacement-owner-ready") {break;}}
    assert_eq!(ReplacementSource::open(&target).err().unwrap().kind(),io::ErrorKind::WouldBlock);
    child.kill().unwrap();child.wait().unwrap();
    drop(ReplacementSource::open(&target).unwrap());
}

#[test]
fn symlinks_directories_aliases_and_native_names_are_safe() {
    let scratch = common::Scratch::new("boundary-paths");
    let target = scratch.0.join("original");
    fs::write(&target, b"original").unwrap();
    let link = scratch.0.join("link");
    symlink(&target, &link).unwrap();
    assert!(Source::open(&link).is_err());
    assert!(ReplacementSource::open(&link).is_err());
    drop(ReplacementSource::open(&target).unwrap());
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
    let cancellation=img::cancellation::Cancellation::default();cancellation.cancel();
    let error=img::publication::publish_cancellable(b"complete bytes",img::publication::Destination::New(&output),None,&cancellation).unwrap_err();
    assert!(!error.committed());assert!(!output.exists());assert_eq!(std::fs::read_dir(&scratch.0).unwrap().count(),0);
}

#[test]
fn container_signatures_are_matched_whole() {
    let sniff = |bytes: &[u8]| { let mut padded = bytes.to_vec(); padded.resize(32, 0); Format::sniff(&padded).map_err(|error| error.kind()) };
    assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Ok(Format::Webp));
    for refused in [&b"RIFF\0\0\0\0WAVEfmt "[..], b"RIFX\0\0\0\0WEBPVP8 ", b"\0\0\0\0\0\0\0\0WEBPVP8 "] {
        assert_eq!(sniff(refused), Err(ErrorKind::Unsupported), "{refused:?}");
    }
    assert_eq!(Format::sniff(b"RIFF").unwrap_err().kind(), ErrorKind::Unsupported);
    assert_eq!(sniff(b"\0\0\0\x18ftypavif\0\0\0\0avifmif1"), Ok(Format::Avif));
    assert_eq!(sniff(b"\0\0\0\x18ftypavis\0\0\0\0mif1msf1"), Ok(Format::Avif));
    assert_eq!(sniff(b"\0\0\0\x18ftypmif1\0\0\0\0mif1avif"), Ok(Format::Avif));
    assert_eq!(sniff(b"\0\0\0\x18ftypmif1\0\0\0\0mif1avis"), Ok(Format::Avif));
    for refused in [&b"\0\0\0\x18ftypmif1\0\0\0\0mif1msf1"[..], b"\0\0\0\x18moovavif\0\0\0\0avifavis", b"\0\0\0\x18ftypheic\0\0\0\0mif1heic"] {
        assert_eq!(sniff(refused), Err(ErrorKind::Unsupported), "{refused:?}");
    }
}

#[test]
fn destination_key_resolves_missing_ancestors_without_creating_them() {
    let scratch = common::Scratch::new("destination-key");
    let leaf = scratch.0.join("missing/deeper/output.png");
    let key = destination_key(&leaf).unwrap();
    assert_eq!(key, scratch.0.canonicalize().unwrap().join("missing/deeper/output.png"));
    assert!(!scratch.0.join("missing").exists(), "resolving a key must not create directories");
    assert_eq!(destination_key(Path::new("")).unwrap_err().to_string(), "missing filename");
    assert_eq!(destination_key(Path::new("/")).unwrap_err().to_string(), "missing filename");
    let bare = destination_key(Path::new("bare-output.png")).unwrap();
    assert!(bare.is_absolute() && bare.ends_with("bare-output.png"), "{bare:?}");
    fs::write(scratch.0.join("plain-file"), b"file").unwrap();
    assert!(destination_key(&scratch.0.join("plain-file/output.png")).is_err(), "a file ancestor must not resolve");
}

#[test]
fn create_parent_makes_every_ancestor_and_tolerates_existing_ones() {
    let scratch = common::Scratch::new("create-parent");
    let leaf = scratch.0.join("one/two/three/output.png");
    create_parent(&leaf).unwrap();
    assert!(scratch.0.join("one/two/three").is_dir());
    assert!(!leaf.exists(), "creating the parent must not create the file");
    create_parent(&leaf).unwrap();
    create_parent(&scratch.0.join("output.png")).unwrap();
    fs::create_dir(scratch.0.join("mid")).unwrap();
    create_parent(&scratch.0.join("mid/../out/output.png")).unwrap();
    assert!(scratch.0.join("out").is_dir(), "the parent-directory component must resolve");
    assert!(scratch.0.join("mid").is_dir());
    fs::write(scratch.0.join("file-ancestor"), b"file").unwrap();
    assert!(create_parent(&scratch.0.join("file-ancestor/deeper/output.png")).is_err(), "a file ancestor must fail rather than pass");
}

#[test]
fn read_directory_returns_sorted_native_entries_without_following_links() {
    let scratch = common::Scratch::new("read-directory");
    fs::write(scratch.0.join("z.txt"), b"z").unwrap();
    fs::write(scratch.0.join("a.txt"), b"a").unwrap();
    fs::create_dir(scratch.0.join("sub")).unwrap();
    symlink(&scratch.0.join("a.txt"), scratch.0.join("link")).unwrap();
    let entries = read_directory(&scratch.0).unwrap();
    let names: Vec<&std::ffi::OsStr> = entries.iter().map(|entry| entry.name.as_os_str()).collect();
    assert_eq!(names, [
        std::ffi::OsStr::new("a.txt"),
        std::ffi::OsStr::new("link"),
        std::ffi::OsStr::new("sub"),
        std::ffi::OsStr::new("z.txt"),
    ]);
    let by_name = |name: &str| entries.iter().find(|entry| entry.name == std::ffi::OsStr::new(name)).unwrap();
    assert!(by_name("a.txt").file && !by_name("a.txt").directory && !by_name("a.txt").symlink);
    assert!(by_name("sub").directory && !by_name("sub").file);
    assert!(by_name("link").symlink && !by_name("link").file && !by_name("link").directory);
    let expected_id = Source::open(&scratch.0.join("a.txt")).unwrap().file_id();
    assert_eq!(by_name("a.txt").file_id, expected_id);
}
