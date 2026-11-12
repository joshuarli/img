mod common;

use std::{borrow::Cow, io::Cursor, num::NonZeroU32};
use rimage::still::{Effort, EncodingMode, Format, SampleLayout, SamplePreservation, StillImage};

fn png_image(width: u32, height: u32, samples: &[u8], color: png::ColorType, icc: Option<Vec<u8>>, orientation: Option<u8>) -> StillImage {
    let mut info = png::Info::default(); info.width = width; info.height = height;
    info.color_type = color; info.bit_depth = png::BitDepth::Eight; info.icc_profile = icc.map(Cow::Owned);
    let mut bytes = Vec::new();
    if let Some(orientation) = orientation {
        let mut exif = b"II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x01\0\0\0\0\0\0\0".to_vec();
        exif[18] = orientation;
        info.exif_metadata = Some(Cow::Owned(exif));
    }
    let encoder = png::Encoder::with_info(&mut bytes, info).unwrap();
    encoder.write_header().unwrap().write_image_data(samples).unwrap();
    StillImage::decode(&bytes).unwrap()
}

fn linear(value: f32) -> f32 { if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) } }

#[test]
fn all_exif_orientations_are_applied_once_and_stripping_respects_exactness() {
    let expected = [
        [10, 20, 30, 40, 50, 60], [30, 20, 10, 60, 50, 40], [60, 50, 40, 30, 20, 10], [40, 50, 60, 10, 20, 30],
        [10, 40, 20, 50, 30, 60], [40, 10, 50, 20, 60, 30], [60, 30, 50, 20, 40, 10], [30, 60, 20, 50, 10, 40],
    ];
    for orientation in 1..=8 {
        let image = png_image(3, 2, &[10, 20, 30, 40, 50, 60], png::ColorType::Grayscale, None, Some(orientation));
        let prepared = image.prepare().unwrap();
        assert_eq!(prepared.dimensions(), if orientation >= 5 { (2, 3) } else { (3, 2) });
        for (pixel, value) in prepared.linear_rgb().iter().zip(expected[orientation as usize - 1]) {
            assert!((pixel[0] - linear(value as f32 / 255.0)).abs() < 0.00001, "orientation {orientation}");
        }
        if orientation != 1 { assert!(image.strip_metadata(SamplePreservation::Exact).is_err()); }
        let stripped = image.strip_metadata(SamplePreservation::AllowReorientation).unwrap();
        let encoded = stripped.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
        assert!(!encoded.windows(4).any(|part| part == b"eXIf"));
        let decoded = StillImage::decode(&encoded).unwrap().prepare().unwrap();
        assert_eq!(prepared.dimensions(), decoded.dimensions());
        assert_eq!(prepared.linear_rgb(), decoded.linear_rgb());
    }
}

#[test]
fn floating_point_profile_preparation_matches_independent_lcms() {
    let profile = moxcms::ColorProfile::new_display_p3().encode().unwrap();
    let samples = [60, 70, 80, 90, 100, 110, 130, 120, 110, 160, 155, 150];
    let image = png_image(2, 2, &samples, png::ColorType::Rgb, Some(profile.clone()), None);
    let prepared = image.prepare().unwrap();
    let source = lcms2::Profile::new_icc(&profile).unwrap();
    let target = lcms2::Profile::new_srgb();
    let transform: lcms2::Transform<[f32; 3], [f32; 3]> = lcms2::Transform::new_flags(&source, lcms2::PixelFormat::RGB_FLT,
        &target, lcms2::PixelFormat::RGB_FLT, lcms2::Intent::Perceptual, lcms2::Flags::NO_CACHE).unwrap();
    let input: Vec<_> = samples.chunks_exact(3).map(|p| [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0]).collect();
    let mut expected = vec![[0.0; 3]; input.len()]; transform.transform_pixels(&input, &mut expected);
    for (actual, expected) in prepared.linear_rgb().iter().zip(expected) {
        for (&a, e) in actual.iter().zip(expected) { assert!((a - linear(e)).abs() < 0.001, "{a} vs {}", linear(e)); }
    }
    let outside = png_image(1, 1, &[255, 0, 0], png::ColorType::Rgb, Some(profile), None);
    assert!(outside.prepare().is_err(), "out-of-sRGB colors must not be silently clipped");
}

#[test]
fn resize_uses_linear_light_and_premultiplied_alpha() {
    let image = StillImage::from_rgb8(2, 1, vec![0, 0, 0, 255, 255, 255]).unwrap();
    let resized = image.resize_to_fit(NonZeroU32::new(1).unwrap()).unwrap();
    assert_eq!(resized.dimensions(), (1, 1)); assert_eq!(resized.samples(), [188, 188, 188]);
    let edge = StillImage::from_samples(2, 1, SampleLayout::Rgba8, vec![255, 0, 0, 255, 0, 0, 255, 0]).unwrap();
    let resized = edge.resize_to_fit(NonZeroU32::new(1).unwrap()).unwrap();
    assert_eq!(resized.samples(), [255, 0, 0, 128]);
    let image = StillImage::from_rgb8(100, 50, common::pixels(100, 50)).unwrap();
    assert_eq!(image.resize_to_fit(NonZeroU32::new(20).unwrap()).unwrap().dimensions(), (20, 10));
    assert_eq!(image.resize_to_fit(NonZeroU32::new(200).unwrap()).unwrap().samples(), image.samples());
}

#[test]
fn profile_and_gray_resize_keep_color_semantics_and_exact_alpha() {
    let profile = moxcms::ColorProfile::new_srgb().encode().unwrap();
    let image = png_image(4, 2, &[128, 128, 128, 42].repeat(8), png::ColorType::Rgba, Some(profile.clone()), Some(6));
    let prepared = image.prepare().unwrap(); assert!(prepared.alpha().iter().all(|&a| a == 42));
    let resized = image.resize_to_fit(NonZeroU32::new(2).unwrap()).unwrap();
    assert_eq!(resized.prepare().unwrap().dimensions(), (1, 2));
    assert!(resized.prepare().unwrap().alpha().iter().all(|&a| a == 42));
    let stripped = resized.strip_metadata(SamplePreservation::AllowReorientation).unwrap();
    let encoded = stripped.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
    let reader = png::Decoder::new(Cursor::new(&encoded)).read_info().unwrap();
    assert_eq!(reader.info().icc_profile.as_deref(), Some(profile.as_slice()));
    let gray = png_image(8, 4, &[91; 32], png::ColorType::Grayscale, None, None);
    let resized = gray.resize_to_fit(NonZeroU32::new(4).unwrap()).unwrap();
    assert!(resized.is_grayscale()); assert_eq!(resized.samples(), vec![91; 8]);
}

#[test]
fn high_depth_remains_exact_but_is_not_scored_or_resized() {
    let samples: Vec<_> = (0..64u16).flat_map(|v| (v * 997).to_ne_bytes()).collect();
    let image = StillImage::from_samples(8, 8, SampleLayout::Gray16, samples.clone()).unwrap();
    assert!(image.prepare().is_err()); assert!(image.resize_to_fit(NonZeroU32::new(4).unwrap()).is_err());
    let image = image.strip_metadata(SamplePreservation::Exact).unwrap();
    let bytes = image.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
    assert_eq!(StillImage::decode(&bytes).unwrap().samples(), samples);
}

#[test]
fn gray_profiles_and_explicit_png_signaling_have_defined_behavior() {
    let profile = moxcms::ColorProfile::new_gray_with_gamma(2.2).encode().unwrap();
    let gray = png_image(4, 2, &[128; 8], png::ColorType::Grayscale, Some(profile), None);
    let prepared = gray.prepare().unwrap();
    assert!((prepared.linear_rgb()[0][0] - (128.0f32 / 255.0).powf(2.2)).abs() < 0.001);
    assert!(gray.resize_to_fit(NonZeroU32::new(2).unwrap()).unwrap().is_grayscale());
    let image = StillImage::from_rgb8(2, 2, vec![128; 12]).unwrap();
    let bytes = image.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
    let tagged = |kind: &[u8; 4], data: &[u8]| {
        let mut bytes = bytes.clone();
        let mut chunk = (data.len() as u32).to_be_bytes().to_vec(); chunk.extend_from_slice(kind); chunk.extend_from_slice(data);
        let crc = crc32fast::hash(&chunk[4..]); chunk.extend_from_slice(&crc.to_be_bytes()); bytes.splice(33..33, chunk);
        StillImage::decode(&bytes).unwrap()
    };
    let linear_image = tagged(b"gAMA", &100000u32.to_be_bytes());
    assert!((linear_image.prepare().unwrap().linear_rgb()[0][0] - 128.0 / 255.0).abs() < 0.00001);
    assert!(tagged(b"cICP", &[9, 16, 0, 1]).prepare().is_err());
    let stripped = linear_image.strip_metadata(SamplePreservation::Exact).unwrap();
    let output = stripped.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
    assert!(output.windows(4).any(|part| part == b"gAMA"));
}

#[test]
fn quantized_nominal_srgb_profile_is_refused_when_independent_cms_leaves_gamut() {
    let bytes=include_bytes!("../benchmarks/corpus/astronaut.png");
    let reader=png::Decoder::new(std::io::Cursor::new(bytes)).read_info().unwrap();
    let profile=reader.info().icc_profile.as_deref().unwrap();
    let source=lcms2::Profile::new_icc(profile).unwrap();let target=lcms2::Profile::new_srgb();
    let transform=lcms2::Transform::<[f32;3],[f32;3]>::new(&source,lcms2::PixelFormat::RGB_FLT,&target,lcms2::PixelFormat::RGB_FLT,lcms2::Intent::RelativeColorimetric).unwrap();
    let image=StillImage::decode(bytes).unwrap();let input:Vec<[f32;3]>=image.samples().chunks_exact(3).map(|p|[p[0] as f32/255.0,p[1] as f32/255.0,p[2] as f32/255.0]).collect();
    let mut output=vec![[0.0;3];input.len()];transform.transform_pixels(&input,&mut output);
    let minimum=output.iter().flatten().copied().fold(f32::INFINITY,f32::min);let maximum=output.iter().flatten().copied().fold(f32::NEG_INFINITY,f32::max);
    assert!(minimum < -0.00001 || maximum > 1.00001);
    assert!(image.prepare().err().unwrap().to_string().contains("gamut"));
}

#[test]
fn canonical_srgb_transfer_has_platform_stable_float_samples() {
    for (value,expected) in [(0,0u32),(71,0x3d810b68),(180,0x3ee9aeb6),(255,0x3f800000)] {
        let image=StillImage::from_rgb8(1,1,vec![value;3]).unwrap();
        assert_eq!(image.prepare().unwrap().linear_rgb()[0].map(f32::to_bits),[expected;3],"sRGB code {value}");
    }
}

#[test]
fn png_cicp_authority_is_preserved_when_an_icc_profile_is_also_present() {
    let samples=[80,120,160].repeat(16*16);let profile=moxcms::ColorProfile::new_display_p3().encode().unwrap();
    let image=png_image(16,16,&samples,png::ColorType::Rgb,Some(profile),None);let ordinary=image.prepare().unwrap();
    let mut bytes=image.encode(Format::Png,EncodingMode::Lossless,Effort::Fast).unwrap();let mut chunk=4u32.to_be_bytes().to_vec();chunk.extend_from_slice(b"cICP");chunk.extend_from_slice(&[1,13,0,1]);let crc=crc32fast::hash(&chunk[4..]);chunk.extend_from_slice(&crc.to_be_bytes());bytes.splice(33..33,chunk);
    let tagged=StillImage::decode(&bytes).unwrap();let reference=StillImage::from_rgb8(16,16,samples).unwrap().prepare().unwrap();
    assert!((ordinary.linear_rgb()[0][0]-reference.linear_rgb()[0][0]).abs()>0.001);
    assert_eq!(tagged.prepare().unwrap().linear_rgb(),reference.linear_rgb(),"PNG CICP supersedes ICC for preparation");
    assert!(rimage::optimizer::process(&bytes,rimage::optimizer::Request {operation:rimage::optimizer::Operation::Convert(Format::Webp),fidelity:rimage::optimizer::Fidelity::Lossless,..Default::default()}).is_err(),"conversion must not drop authoritative CICP and activate a different ICC interpretation");
}
