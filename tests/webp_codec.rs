mod common;

use rimage::still::{Effort, EncodingMode, Format, Quality, SampleLayout, StillImage};

fn transparent() -> Vec<u8> {
    (0..32 * 24).flat_map(|i| [(i * 13) as u8, (i * 7) as u8, (i * 3) as u8, (i % 256) as u8]).collect()
}

#[test]
fn exact_webp_preserves_hidden_rgb_and_alpha_with_independent_decode() {
    let samples = transparent();
    let image = StillImage::from_samples(32, 24, SampleLayout::Rgba8, samples.clone()).unwrap();
    let bytes = image.encode(Format::Webp, EncodingMode::Lossless, Effort::Fast).unwrap();
    let output = StillImage::decode(&bytes).unwrap();
    assert_eq!(output.dimensions(), (32, 24));
    assert_eq!(output.samples(), samples);
    let independent = webp::Decoder::new(&bytes).decode().unwrap();
    assert_eq!((independent.width(), independent.height()), (32, 24));
    assert_eq!(&*independent, samples);
}

#[test]
fn lossy_webp_preserves_actual_alpha_and_refuses_precision_reduction() {
    let samples = transparent();
    let image = StillImage::from_samples(32, 24, SampleLayout::Rgba8, samples.clone()).unwrap();
    let bytes = image.encode(Format::Webp, EncodingMode::Quality(Quality::new(75.0).unwrap()), Effort::Fast).unwrap();
    let decoded = StillImage::decode(&bytes).unwrap();
    let independent = webp::Decoder::new(&bytes).decode().unwrap();
    assert_eq!(decoded.dimensions(), (32, 24));
    for output in [decoded.samples(), independent.to_vec()] {
        assert_eq!(output.chunks_exact(4).map(|p| p[3]).collect::<Vec<_>>(), samples.chunks_exact(4).map(|p| p[3]).collect::<Vec<_>>());
    }
    let deep = StillImage::from_samples(8, 8, SampleLayout::Gray16, vec![0; 128]).unwrap();
    assert!(deep.encode(Format::Webp, EncodingMode::Lossless, Effort::Fast).is_err());
    for end in [12, 20, bytes.len() / 2] { assert!(StillImage::decode(&bytes[..end]).is_err()); }
}

#[test]
fn animated_webp_is_rejected_before_frame_extraction() {
    let mut bytes = b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0\x02\0\0\0\x01\0\0\x01\0\0ANIM\x06\0\0\0\0\0\0\0\0\0".to_vec();
    let still = StillImage::from_rgb8(2, 2, vec![127; 12]).unwrap().encode(Format::Webp, EncodingMode::Lossless, Effort::Fast).unwrap();
    assert_eq!(&still[12..16], b"VP8L");
    let mut frame = vec![0; 16]; frame[6] = 1; frame[9] = 1; frame[12] = 1;
    frame.extend_from_slice(&still[12..]);
    bytes.extend_from_slice(b"ANMF"); bytes.extend_from_slice(&(frame.len() as u32).to_le_bytes()); bytes.extend_from_slice(&frame);
    let len = (bytes.len() - 8) as u32; bytes[4..8].copy_from_slice(&len.to_le_bytes());
    let error = StillImage::decode(&bytes).err().unwrap();
    assert!(error.to_string().contains("animation"), "{error}");
}

#[test]
fn rgb_and_explicit_gray_conversion_are_exact() {
    for layout in [SampleLayout::Rgb8, SampleLayout::Gray8] {
        let samples = if layout == SampleLayout::Rgb8 { common::pixels(16, 16) } else { (0..=255u8).collect() };
        let image = StillImage::from_samples(16, 16, layout, samples.clone()).unwrap();
        let bytes = image.encode(Format::Webp, EncodingMode::Lossless, Effort::Fast).unwrap();
        let output = StillImage::decode(&bytes).unwrap();
        let expected = if layout == SampleLayout::Gray8 { samples.into_iter().flat_map(|v| [v, v, v]).collect() } else { samples };
        assert_eq!(output.samples(), expected);
    }
}

#[test]
fn decoded_untagged_png_samples_can_be_encoded_without_color_conversion() {
    let samples=transparent();
    let image=StillImage::from_samples(32,24,SampleLayout::Rgba8,samples.clone()).unwrap();
    let png=image.encode(Format::Png,EncodingMode::Lossless,Effort::Fast).unwrap();
    let decoded=StillImage::decode(&png).unwrap();
    let webp=decoded.encode(Format::Webp,EncodingMode::Lossless,Effort::Fast).unwrap();
    assert_eq!(webp::Decoder::new(&webp).decode().unwrap().to_vec(),samples);
}
