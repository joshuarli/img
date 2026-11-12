mod common;

use rimage::still::{Effort, EncodingMode, ErrorKind, Format, Quality, SampleLayout, StillImage};
use zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};
use zune_image::{codecs::jpeg::JpegDecoder, image::Image};

fn mode(quality: f32) -> EncodingMode { EncodingMode::Quality(Quality::new(quality).unwrap()) }

#[test]
fn progressive_rgb_and_gray_have_independent_decode_proof() {
    for layout in [SampleLayout::Rgb8, SampleLayout::Gray8] {
        let samples = if layout == SampleLayout::Rgb8 { common::pixels(48, 32) } else { vec![137; 48 * 32] };
        let image = StillImage::from_samples(48, 32, layout, samples).unwrap();
        let bytes = image.encode(Format::Jpeg, mode(90.0), Effort::Fast).unwrap();
        assert!(bytes.windows(2).any(|pair| pair == [0xff, 0xc2]));
        if layout == SampleLayout::Gray8 {
            assert_eq!(zenjpeg::decoder::DecodeConfig::new().read_info(&bytes).unwrap().num_components, 1);
        }
        let decoded = StillImage::decode(&bytes).unwrap();
        assert_eq!(decoded.dimensions(), (48, 32));
        let options = DecoderOptions::default().jpeg_set_out_colorspace(if layout == SampleLayout::Gray8 { ColorSpace::Luma } else { ColorSpace::RGB });
        let independent = Image::from_decoder(JpegDecoder::new_with_options(ZCursor::new(&bytes), options)).unwrap();
        assert_eq!(independent.dimensions(), (48, 32));
        if layout == SampleLayout::Gray8 {
            assert!(decoded.is_grayscale());
            assert_eq!(independent.colorspace(), ColorSpace::Luma);
            assert!(independent.flatten_frames::<u8>()[0].iter().all(|&value| value.abs_diff(137) <= 1));
            assert!(decoded.samples().iter().all(|&value| value.abs_diff(137) <= 1));
        }
    }
}

#[test]
fn invalid_quality_alpha_precision_and_corruption_are_named_errors() {
    for value in [f32::NAN, f32::INFINITY, -0.1, 100.1] { assert!(Quality::new(value).is_err()); }
    assert!(Quality::new(0.0).is_ok()); assert!(Quality::new(100.0).is_ok());
    let rgba = StillImage::from_samples(8, 8, SampleLayout::Rgba8, [90, 40, 20, 0].repeat(64)).unwrap();
    assert_eq!(rgba.encode(Format::Jpeg, mode(80.0), Effort::Fast).unwrap_err().kind(), ErrorKind::Unsupported);
    let opaque = StillImage::from_samples(8, 8, SampleLayout::Rgba8, [90, 40, 20, 255].repeat(64)).unwrap();
    assert!(opaque.encode(Format::Jpeg, mode(80.0), Effort::Fast).is_ok());
    let deep = StillImage::from_samples(8, 8, SampleLayout::Gray16, vec![0; 128]).unwrap();
    assert_eq!(deep.encode(Format::Jpeg, mode(80.0), Effort::Fast).unwrap_err().kind(), ErrorKind::Unsupported);
    assert!(opaque.encode(Format::Jpeg, EncodingMode::Lossless, Effort::Fast).is_err());
    let bytes = opaque.encode(Format::Jpeg, mode(80.0), Effort::Fast).unwrap();
    for end in [2, 16, bytes.len() / 2] { assert!(StillImage::decode(&bytes[..end]).is_err()); }
}

#[test]
fn exif_orientation_is_preserved_without_changing_stored_dimensions() {
    let image = StillImage::from_rgb8(24, 16, common::pixels(24, 16)).unwrap();
    let bytes = image.encode(Format::Jpeg, mode(90.0), Effort::Fast).unwrap();
    let exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let mut tagged = bytes[..2].to_vec();
    tagged.extend_from_slice(&[0xff, 0xe1]);
    tagged.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
    tagged.extend_from_slice(exif); tagged.extend_from_slice(&bytes[2..]);
    let icc = include_bytes!("files/icc/tinysrgb.icc");
    let mut segment = b"ICC_PROFILE\0\x01\x01".to_vec(); segment.extend_from_slice(icc);
    let mut profiled = tagged[..2].to_vec(); profiled.extend_from_slice(&[0xff, 0xe2]);
    profiled.extend_from_slice(&((segment.len() + 2) as u16).to_be_bytes());
    profiled.extend_from_slice(&segment); profiled.extend_from_slice(&tagged[2..]);
    tagged = profiled;
    let decoded = StillImage::decode(&tagged).unwrap();
    assert_eq!(decoded.dimensions(), (24, 16));
    let output = decoded.encode(Format::Jpeg, mode(90.0), Effort::Fast).unwrap();
    assert!(output.windows(exif.len()).any(|part| part == exif));
    assert!(output.windows(icc.len()).any(|part| part == icc));
    assert_eq!(StillImage::decode(&output).unwrap().dimensions(), (24, 16));
}

#[test]
fn fixed_seed_dimension_cases_stay_within_the_boundary() {
    let mut seed = 0x6543_210fu32;
    for _ in 0..12 {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let (w, h) = (1 + seed % 31, 1 + (seed >> 16) % 31);
        let image = StillImage::from_rgb8(w, h, common::pixels(w, h)).unwrap();
        let bytes = image.encode(Format::Jpeg, mode(75.0), Effort::Fast).unwrap();
        assert_eq!(StillImage::decode(&bytes).unwrap().dimensions(), (w, h));
    }
}

#[test]
fn four_component_jpeg_is_refused_before_implicit_cmyk_conversion() {
    let image=StillImage::from_rgb8(8,8,common::pixels(8,8)).unwrap();let mut bytes=image.encode(Format::Jpeg,mode(80.0),Effort::Fast).unwrap();
    let at=bytes.windows(2).position(|p|p==[255,194]).unwrap();let length=u16::from_be_bytes([bytes[at+2],bytes[at+3]]) as usize;
    bytes[at+9]=4;bytes[at+2..at+4].copy_from_slice(&((length+3) as u16).to_be_bytes());bytes.splice(at+2+length..at+2+length,[4,17,0]);
    assert_eq!(zenjpeg::decoder::DecodeConfig::new().read_info(&bytes).unwrap().num_components,4);
    let error=StillImage::decode(&bytes).err().unwrap();assert_eq!(error.kind(),ErrorKind::Unsupported);assert!(error.to_string().contains("components"));
}
