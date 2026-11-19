mod common;

use img::still::{Effort, EncodingMode, ErrorKind, Format, Quality, SampleLayout, StillImage};
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
            let mut probe = zune_jpeg::JpegDecoder::new(ZCursor::new(&bytes));
            probe.decode_headers().unwrap();
            assert_eq!(probe.info().unwrap().components, 1);
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
    let mut probe=zune_jpeg::JpegDecoder::new(ZCursor::new(&bytes));probe.decode_headers().unwrap();
    assert_eq!(probe.info().unwrap().components,4);
    let error=StillImage::decode(&bytes).err().unwrap();assert_eq!(error.kind(),ErrorKind::Unsupported);assert!(error.to_string().contains("components"));
}

#[test]
fn jpeg_auxiliary_images_cannot_be_silently_extracted_as_a_single_frame() {
    let pixels=common::pixels(24,16);
    let base=StillImage::from_rgb8(24,16,pixels).unwrap().encode(Format::Jpeg,mode(80.0),Effort::Fast).unwrap();
    let mut concatenated=base.clone();concatenated.extend_from_slice(&base);
    assert_eq!(StillImage::decode(&concatenated).err().unwrap().kind(),ErrorKind::Unsupported);
    let payload=b"MPF\0fixture";
    let mut jpeg=base[..2].to_vec();
    jpeg.extend_from_slice(&[0xff,0xe2]);
    jpeg.extend_from_slice(&((payload.len()+2) as u16).to_be_bytes());
    jpeg.extend_from_slice(payload);
    jpeg.extend_from_slice(&base[2..]);
    assert!(jpeg.windows(4).any(|value|value==b"MPF\0"));
    let header=img::still::describe(&jpeg).unwrap();assert_eq!((header.width,header.height),(24,16));assert!(header.animation);
    assert!(StillImage::decode(&jpeg).is_err(),"MPF auxiliary payloads must not disappear through base-image decoding");
}

#[test]
fn jpeg_hdr_and_late_application_metadata_are_refused_before_pixel_changes() {
    let jpeg=StillImage::from_rgb8(24,16,common::pixels(24,16)).unwrap().encode(Format::Jpeg,mode(80.0),Effort::Fast).unwrap();
    let hdr=b"urn:iso:std:iso:ts:21496:-1\0\0\0\0\0";
    let mut tagged=jpeg[..2].to_vec();tagged.extend_from_slice(&[255,226]);tagged.extend_from_slice(&((hdr.len()+2) as u16).to_be_bytes());tagged.extend_from_slice(hdr);tagged.extend_from_slice(&jpeg[2..]);
    assert!(img::still::describe(&tagged).unwrap().hdr_metadata);
    assert_eq!(StillImage::decode(&tagged).err().unwrap().kind(),ErrorKind::Unsupported);
    let payload=b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let mut late=jpeg[..jpeg.len()-2].to_vec();late.extend_from_slice(&[255,225]);late.extend_from_slice(&((payload.len()+2) as u16).to_be_bytes());late.extend_from_slice(payload);late.extend_from_slice(&[255,217]);
    let header=img::still::describe(&late).unwrap();assert!(header.exif);assert_eq!(header.orientation,6);
    assert_eq!(StillImage::decode(&late).err().unwrap().kind(),ErrorKind::Unsupported);
    let mut deep=jpeg.clone();let at=deep.windows(2).position(|pair|pair==[255,194]).unwrap();deep[at+4]=12;
    assert_eq!(img::still::describe(&deep).unwrap().bit_depth,Some(12));
    assert_eq!(StillImage::decode(&deep).err().unwrap().kind(),ErrorKind::Unsupported);
}

#[test]
fn jpeg_component_sampling_and_decoder_agreement() {
    for (layout, channels) in [(SampleLayout::Rgb8, 3usize), (SampleLayout::Gray8, 1)] {
        let pixel = if channels == 3 { vec![100, 120, 140] } else { vec![120] };
        let samples = pixel.repeat(32 * 24);
        let image = StillImage::from_samples(32, 24, layout, samples.clone()).unwrap();
        for effort in [Effort::Fast, Effort::Balanced, Effort::Best] {
            let encoded = image.encode(Format::Jpeg, mode(90.0), effort).unwrap();
            let sof = encoded.windows(2).position(|p| p == [255, 194]).unwrap();
            assert_eq!(encoded[sof + 9] as usize, channels);
            for c in 0..channels { assert_eq!(encoded[sof + 11 + c * 3], 0x11); }
            let color = if channels == 1 { ColorSpace::Luma } else { ColorSpace::RGB };
            let options = DecoderOptions::default().jpeg_set_out_colorspace(color);
            let mut decoder = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(&encoded), options);
            let direct = decoder.decode().unwrap();
            let other = Image::from_decoder(JpegDecoder::new_with_options(ZCursor::new(&encoded), options)).unwrap().flatten_frames::<u8>().remove(0);
            assert_eq!(direct, other);
            assert!(direct.iter().zip(&samples).all(|(&a, &b)| a.abs_diff(b) <= 3));
        }
    }
}

#[test]
fn jpeg_large_icc_exif_and_xmp_survive_encoding() {
    let image = StillImage::from_rgb8(8, 8, [30, 80, 130].repeat(64)).unwrap();
    let base = image.encode(Format::Jpeg, mode(90.0), Effort::Fast).unwrap();
    let icc: Vec<u8> = (0..70000).map(|i| (i % 251) as u8).collect();
    let exif = b"II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let xmp = b"<x:xmpmeta>fixture</x:xmpmeta>";
    let mut input = base[..2].to_vec();
    let mut add = |marker: u8, payload: &[u8]| { input.extend_from_slice(&[255, marker]); input.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes()); input.extend_from_slice(payload); };
    for (index, chunk) in icc.chunks(65519).enumerate() { let mut p = b"ICC_PROFILE\0".to_vec(); p.extend_from_slice(&[(index + 1) as u8, 2]); p.extend_from_slice(chunk); add(0xe2, &p); }
    let mut exif_marker = b"Exif\0\0".to_vec(); exif_marker.extend_from_slice(exif); add(0xe1, &exif_marker);
    let mut xmp_marker = b"http://ns.adobe.com/xap/1.0/\0".to_vec(); xmp_marker.extend_from_slice(xmp); add(0xe1, &xmp_marker);
    input.extend_from_slice(&base[2..]);
    let decoded = StillImage::decode(&input).unwrap();
    let output = decoded.encode(Format::Jpeg, mode(90.0), Effort::Balanced).unwrap();
    let info = img::still::describe(&output).unwrap(); assert!(info.icc && info.exif && info.xmp); assert_eq!(info.orientation, 6);
    let decoded = StillImage::decode(&output).unwrap();
    let output = decoded.encode(Format::Jpeg, mode(90.0), Effort::Fast).unwrap();
    let mut probe = zune_jpeg::JpegDecoder::new(ZCursor::new(&output)); probe.decode_headers().unwrap();
    assert_eq!(probe.icc_profile().unwrap(), icc);
    assert_eq!(probe.info().unwrap().exif_data.as_deref(), Some(exif.as_slice()));
    assert_eq!(probe.info().unwrap().xmp_data.as_deref(), Some(xmp.as_slice()));
    assert_eq!(output.windows(12).filter(|w| *w == b"ICC_PROFILE\0").count(), 2);
}

#[test]
fn invalid_jpeg_dimensions_return_error() {
    assert!(StillImage::from_rgb8(0, 1, Vec::new()).is_err());
    assert!(jpegli_sys::Encoder::new(0, 1, jpegli_sys::Color::Rgb, 90.0, 1024 * 1024, 1024 * 1024).is_err());
}

#[test]
fn jpegli_output_limit_returns_an_error() {
    let mut saw_limit = false;
    for limit in 16..128 {
        if let Ok(mut encoder) = jpegli_sys::Encoder::new(8, 8, jpegli_sys::Color::Rgb, 90.0, 1024 * 1024, limit)
            && encoder.app1(&[42; 100]).is_err() { saw_limit = true; }
    }
    assert!(saw_limit);
}

#[test]
fn fractional_quality_selects_an_intermediate_jpegli_distance() {
    let image = StillImage::from_samples(64, 64, SampleLayout::Rgb8, common::pixels(64, 64)).unwrap();
    let size = |quality| image.encode(Format::Jpeg, mode(quality), Effort::Fast).unwrap().len();
    let (low, middle, high) = (size(90.0), size(90.5), size(91.0));
    assert!(low < middle && middle < high, "{low} {middle} {high}");
}

fn luma_quant_table(bytes: &[u8]) -> Vec<u8> {
    let at = bytes.windows(2).position(|pair| pair == [0xff, 0xdb]).unwrap() + 4;
    assert_eq!(bytes[at], 0, "eight-bit table 0 first");
    bytes[at + 1..at + 65].to_vec()
}

#[test]
fn grayscale_uses_the_perceptual_luma_quantization_of_color_output() {
    let gray = StillImage::from_samples(32, 32, SampleLayout::Gray8, vec![90; 32 * 32]).unwrap();
    let rgb = StillImage::from_samples(32, 32, SampleLayout::Rgb8, vec![90; 32 * 32 * 3]).unwrap();
    for quality in [60.0, 90.0] {
        let encode = |image: &StillImage| luma_quant_table(&image.encode(Format::Jpeg, mode(quality), Effort::Fast).unwrap());
        assert_eq!(encode(&gray), encode(&rgb));
    }
}
