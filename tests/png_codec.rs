mod common;

use std::{io::Cursor, fs};
use rimage::still::{Effort, EncodingMode, Format, Quality, SampleLayout, StillImage};

#[test]
fn exact_png_matrix_preserves_precision_alpha_gray_and_hidden_samples() {
    for (layout, channels, depth, color) in [
        (SampleLayout::Gray8, 1, 8, png::ColorType::Grayscale),
        (SampleLayout::GrayAlpha8, 2, 8, png::ColorType::GrayscaleAlpha),
        (SampleLayout::Rgb8, 3, 8, png::ColorType::Rgb),
        (SampleLayout::Rgba8, 4, 8, png::ColorType::Rgba),
        (SampleLayout::Gray16, 1, 16, png::ColorType::Grayscale),
        (SampleLayout::GrayAlpha16, 2, 16, png::ColorType::GrayscaleAlpha),
        (SampleLayout::Rgb16, 3, 16, png::ColorType::Rgb),
        (SampleLayout::Rgba16, 4, 16, png::ColorType::Rgba),
    ] {
        let values: Vec<u16> = (0..16 * 12 * channels).map(|i| (i * 971) as u16).collect();
        let samples = if depth == 8 { values.iter().map(|&v| v as u8).collect() }
            else { values.iter().flat_map(|v| v.to_ne_bytes()).collect() };
        let image = StillImage::from_samples(16, 12, layout, samples).unwrap();
        let bytes = image.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
        let mut decoder = png::Decoder::new(Cursor::new(&bytes));
        decoder.set_transformations(png::Transformations::EXPAND);
        let mut reader = decoder.read_info().unwrap();
        let mut output = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut output).unwrap();
        assert_eq!((info.width, info.height), (16, 12));
        assert_eq!(info.color_type, color, "{layout:?}");
        let expected: Vec<_> = if depth == 8 { values.iter().map(|&v| v as u8).collect() }
            else { values.iter().flat_map(|v| v.to_be_bytes()).collect() };
        assert_eq!(&output[..info.buffer_size()], expected, "{layout:?}");
        let decoded = StillImage::decode(&bytes).unwrap();
        assert_eq!(decoded.bit_depth(), depth);
        assert_eq!(decoded.has_alpha(), channels == 2 || channels == 4);
        if channels == 1 || channels == 2 { assert!(decoded.is_grayscale(), "{layout:?}"); }
        assert_eq!(decoded.samples(), image.samples(), "{layout:?}");
    }
}

#[test]
fn indexed_png_compares_expanded_samples_and_rejects_animation_and_corruption() {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 4, 2);
        encoder.set_color(png::ColorType::Indexed); encoder.set_depth(png::BitDepth::Eight);
        encoder.set_palette(vec![240, 10, 20, 30, 200, 40]); encoder.set_trns(vec![0, 255]);
        encoder.write_header().unwrap().write_image_data(&[0, 1, 0, 1, 1, 0, 1, 0]).unwrap();
    }
    let image = StillImage::decode(&bytes).unwrap();
    let expected: Vec<_> = [0, 1, 0, 1, 1, 0, 1, 0].into_iter().flat_map(|i| if i == 0 { [240, 10, 20, 0] } else { [30, 200, 40, 255] }).collect();
    assert_eq!(image.samples(), expected);
    let output = image.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
    assert_eq!(StillImage::decode(&output).unwrap().samples(), expected);
    let mut corrupt = bytes.clone(); corrupt[29] ^= 1;
    assert!(StillImage::decode(&corrupt).is_err());
    let mut animated = Vec::new();
    let mut encoder = png::Encoder::new(&mut animated, 2, 2);
    encoder.set_animated(2, 0).unwrap(); encoder.set_color(png::ColorType::Rgb);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&[127; 12]).unwrap(); writer.write_image_data(&[128; 12]).unwrap(); drop(writer);
    assert!(StillImage::decode(&animated).err().unwrap().to_string().contains("animation"));
}

#[test]
fn direct_palette_mode_is_explicit_and_keeps_dimensions() {
    let image = StillImage::from_rgb8(32, 24, common::pixels(32, 24)).unwrap();
    let bytes = image.encode(Format::Png, EncodingMode::Quality(Quality::new(70.0).unwrap()), Effort::Fast).unwrap();
    let reader = png::Decoder::new(Cursor::new(&bytes)).read_info().unwrap();
    assert_eq!(reader.info().color_type, png::ColorType::Indexed);
    assert_eq!(StillImage::decode(&bytes).unwrap().dimensions(), (32, 24));
    let deep = StillImage::from_samples(8, 8, SampleLayout::Gray16, vec![0; 128]).unwrap();
    assert!(deep.encode(Format::Png, EncodingMode::Quality(Quality::new(70.0).unwrap()), Effort::Fast).is_err());
}

#[test]
fn png_private_metadata_and_color_chunks_survive_exact_encoding() {
    let scratch = common::Scratch::new("png-metadata");
    let source = scratch.0.join("source.png");
    common::write_png(&source, 16, 12);
    let mut bytes = fs::read(source).unwrap();
    let data = b"Comment\0private fixture";
    let mut chunk = (data.len() as u32).to_be_bytes().to_vec(); chunk.extend_from_slice(b"tEXt"); chunk.extend_from_slice(data);
    let crc = crc32fast::hash(&chunk[4..]); chunk.extend_from_slice(&crc.to_be_bytes());
    bytes.splice(33..33, chunk.clone());
    let image = StillImage::decode(&bytes).unwrap();
    let encoded = image.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
    assert!(encoded.windows(chunk.len()).any(|part| part == chunk));
    assert_eq!(StillImage::decode(&encoded).unwrap().samples(), image.samples());
}

#[test]
fn layout_sensitive_png_metadata_is_never_silently_discarded() {
    let image = StillImage::from_rgb8(8, 8, [80, 90, 100].repeat(64)).unwrap();
    let mut original = Vec::new();
    let mut encoder = png::Encoder::new(&mut original, 8, 8); encoder.set_color(png::ColorType::Rgb);
    encoder.write_header().unwrap().write_image_data(&image.samples()).unwrap();
    for (kind, data) in [(b"sBIT", vec![7, 7, 7]), (b"bKGD", vec![0, 80, 0, 80, 0, 80]), (b"vpAG", vec![1, 2, 3])] {
        let mut bytes = original.clone();
        let mut chunk = (data.len() as u32).to_be_bytes().to_vec(); chunk.extend_from_slice(kind); chunk.extend_from_slice(&data);
        let crc = crc32fast::hash(&chunk[4..]); chunk.extend_from_slice(&crc.to_be_bytes()); bytes.splice(33..33, chunk);
        let source = StillImage::decode(&bytes).unwrap();
        assert!(source.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).is_err(), "must not discard or misapply {kind:?}");
        let stripped = source.strip_metadata(rimage::still::SamplePreservation::Exact).unwrap();
        assert_eq!(StillImage::decode(&stripped.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap()).unwrap().samples(), source.samples());
    }
}
