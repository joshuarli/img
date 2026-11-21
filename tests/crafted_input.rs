use img::still::{describe, ErrorKind, Format, StillImage};

fn png_fixture() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 8, 8);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&[128; 8 * 8 * 3]).unwrap();
    bytes
}

fn png_with_ihdr(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = png_fixture();
    bytes[16..20].copy_from_slice(&width.to_be_bytes());
    bytes[20..24].copy_from_slice(&height.to_be_bytes());
    let crc = crc32fast::hash(&bytes[12..29]);
    bytes[29..33].copy_from_slice(&crc.to_be_bytes());
    bytes
}

fn riff_chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut chunk = kind.to_vec();
    chunk.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    chunk.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        chunk.push(0);
    }
    chunk
}

fn isobmff_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = ((payload.len() as u32) + 8).to_be_bytes().to_vec();
    bytes.extend_from_slice(kind);
    bytes.extend_from_slice(payload);
    bytes
}

fn webp_with_canvas(width: u32, height: u32) -> Vec<u8> {
    let still = StillImage::from_rgb8(2, 2, vec![127; 12]).unwrap()
        .encode(Format::Webp, img::still::EncodingMode::Lossless, img::still::Effort::Fast).unwrap();
    let mut canvas = vec![0u8; 4];
    canvas.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
    canvas.extend_from_slice(&(height - 1).to_le_bytes()[..3]);
    let mut body = riff_chunk(b"VP8X", &canvas);
    body.extend_from_slice(&still[12..]);
    let mut riff = b"RIFF".to_vec();
    riff.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
    riff.extend_from_slice(b"WEBP");
    riff.extend_from_slice(&body);
    riff
}

fn avif_fixture() -> Vec<u8> {
    StillImage::from_rgb8(16, 16, vec![128; 16 * 16 * 3]).unwrap()
        .encode(Format::Avif, img::still::EncodingMode::Quality(img::still::Quality::new(80.0).unwrap()), img::still::Effort::Fast).unwrap()
}

fn assert_limit(error: img::still::Error, needle: &str) {
    assert_eq!(error.kind(), ErrorKind::Limit, "{error}");
    assert!(error.to_string().contains(needle), "{error} does not name {needle:?}");
}

#[test]
fn png_header_limits_are_refused_before_the_mismatched_idat_is_touched() {
    for (width, height, needle) in [
        (16_385, 8, "width 16385 exceeds limit 16384"),
        (8, 16_385, "height 16385 exceeds limit 16384"),
        (4096, 4097, "pixel count 16781312 exceeds limit 16777216"),
    ] {
        let bytes = png_with_ihdr(width, height);
        for error in [StillImage::decode(&bytes).err().unwrap(), describe(&bytes).err().unwrap()] {
            assert_limit(error, needle);
        }
    }
    for (width, height) in [(0, 8), (8, 0)] {
        assert_eq!(StillImage::decode(&png_with_ihdr(width, height)).err().unwrap().kind(), ErrorKind::Invalid);
        assert_eq!(describe(&png_with_ihdr(width, height)).err().unwrap().kind(), ErrorKind::Invalid);
    }
}

#[test]
fn png_truncation_and_chunk_overrun_are_invalid_not_panics() {
    let bytes = png_fixture();
    for end in [8usize, 16, bytes.len() / 2, bytes.len() - 1] {
        assert_eq!(StillImage::decode(&bytes[..end]).err().unwrap().kind(), ErrorKind::Invalid, "truncated to {end}");
        assert_eq!(describe(&bytes[..end]).err().unwrap().kind(), ErrorKind::Invalid, "truncated to {end}");
    }
    let mut overrun = b"\x89PNG\r\n\x1a\n".to_vec();
    overrun.extend_from_slice(&u32::MAX.to_be_bytes());
    overrun.extend_from_slice(b"IHDR");
    for error in [StillImage::decode(&overrun).err().unwrap(), describe(&overrun).err().unwrap()] {
        assert_eq!(error.kind(), ErrorKind::Invalid);
        assert!(error.to_string().contains("truncated PNG chunk"), "{error}");
    }
}

#[test]
fn webp_canvas_limits_are_refused_at_the_header() {
    for (width, height, needle) in [
        (20_000, 20_000, "width 20000 exceeds limit 16384"),
        (8, 20_000, "height 20000 exceeds limit 16384"),
        (16_384, 16_384, "pixel count 268435456 exceeds limit 16777216"),
        (16_383, 16_383, "pixel count 268402689 exceeds limit 16777216"),
    ] {
        let bytes = webp_with_canvas(width, height);
        for error in [StillImage::decode(&bytes).err().unwrap(), describe(&bytes).err().unwrap()] {
            assert_limit(error, needle);
        }
    }
}

#[test]
fn webp_truncation_is_an_invalid_container() {
    let bytes = StillImage::from_rgb8(8, 8, vec![90; 8 * 8 * 3]).unwrap()
        .encode(Format::Webp, img::still::EncodingMode::Lossless, img::still::Effort::Fast).unwrap();
    for end in [12usize, 16, bytes.len() / 2, bytes.len() - 2] {
        assert_eq!(StillImage::decode(&bytes[..end]).err().unwrap().kind(), ErrorKind::Invalid, "truncated to {end}");
    }
    for end in [12usize, 16] {
        assert_eq!(describe(&bytes[..end]).err().unwrap().kind(), ErrorKind::Invalid, "header cut to {end}");
    }
    assert_eq!((describe(&bytes[..bytes.len() - 2]).unwrap().width, describe(&bytes[..bytes.len() - 2]).unwrap().height), (8, 8));
    let mut empty = b"RIFF".to_vec();
    empty.extend_from_slice(&4u32.to_le_bytes());
    empty.extend_from_slice(b"WEBP");
    assert_eq!(StillImage::decode(&empty).err().unwrap().kind(), ErrorKind::Invalid);
}

#[test]
fn avif_structural_corruption_is_invalid_before_any_plane_decode() {
    let ftyp = isobmff_box(b"ftyp", b"avif\0\0\0\0mif1");

    let mut duplicate = ftyp.clone();
    duplicate.extend_from_slice(&ftyp);
    let mut unsupported = ftyp.clone();
    unsupported.extend_from_slice(&isobmff_box(b"junk", b"data"));
    let mut huge = ftyp.clone();
    huge.extend_from_slice(&1u32.to_be_bytes());
    huge.extend_from_slice(b"free");
    huge.extend_from_slice(&u64::MAX.to_be_bytes());
    let mut truncated = avif_fixture();
    truncated.truncate(32);

    for (label, bytes, needle) in [
        ("duplicate ftyp", duplicate, "duplicate ftyp box"),
        ("unsupported top-level", unsupported, "unsupported top-level box"),
        ("huge 64-bit box", huge, "box exceeds file bounds"),
        ("truncated after ftyp", truncated, "missing pitm box"),
    ] {
        for error in [StillImage::decode(&bytes).err().unwrap(), describe(&bytes).err().unwrap()] {
            assert_eq!(error.kind(), ErrorKind::Invalid, "{label}: {error}");
            assert!(error.to_string().contains(needle), "{label}: {error} does not name {needle:?}");
        }
    }
}

#[test]
fn avif_counts_and_extent_overruns_are_bounded() {
    let ftyp = isobmff_box(b"ftyp", b"avif\0\0\0\0mif1");
    let mut iinf = vec![0u8, 0, 0, 0];
    iinf.extend_from_slice(&2000u16.to_be_bytes());
    let mut meta = vec![0u8, 0, 0, 0];
    meta.extend_from_slice(&isobmff_box(b"pitm", &[0, 0, 0, 0, 0, 1]));
    meta.extend_from_slice(&isobmff_box(b"iinf", &iinf));
    let mut bytes = ftyp.clone();
    bytes.extend_from_slice(&isobmff_box(b"meta", &meta));
    let error = StillImage::decode(&bytes).err().unwrap();
    assert_eq!(error.kind(), ErrorKind::Invalid);
    assert!(error.to_string().contains("too many item info entries"), "{error}");

    let fixture = avif_fixture();
    for end in [40usize, fixture.len() / 2, fixture.len() - 1] {
        let error = StillImage::decode(&fixture[..end]).err().unwrap();
        assert_eq!(error.kind(), ErrorKind::Invalid, "truncated to {end}");
    }
}

#[test]
fn signature_sniffing_refuses_partial_container_magic() {
    for bytes in [b"RIFF".as_slice(), b"RIFF\0\0\0\0WAVEfmt ", b"\xff\xd8", b"\x89PNG\r\n\x1a", b"\0\0\0\x18ftypheic\0\0\0\0mif1heic"] {
        assert_eq!(StillImage::decode(bytes).err().unwrap().kind(), ErrorKind::Unsupported, "{bytes:?}");
    }
    let mut no_riff = b"\0\0\0\0WEBPVP8L".to_vec();
    no_riff.resize(32, 0);
    assert_eq!(StillImage::decode(&no_riff).err().unwrap().kind(), ErrorKind::Unsupported, "{no_riff:?}");
    let mut hoisted = StillImage::from_rgb8(2, 2, vec![127; 12]).unwrap()
        .encode(Format::Webp, img::still::EncodingMode::Lossless, img::still::Effort::Fast).unwrap();
    hoisted[..4].copy_from_slice(b"\0\0\0\0");
    assert_eq!(Format::sniff(&hoisted).unwrap_err().kind(), ErrorKind::Unsupported, "{hoisted:?}");
    assert_eq!(StillImage::decode(&hoisted).err().unwrap().kind(), ErrorKind::Unsupported, "{hoisted:?}");
}
