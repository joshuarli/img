mod common;

use rimage::{perceptual::{PerceptualReference, ScoreTarget, compare}, still::{Effort, EncodingMode, Format, Quality, SampleLayout, StillImage}};

fn png(image: &StillImage) -> Vec<u8> { image.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap() }

#[test]
fn actual_candidate_bytes_are_decoded_and_reference_is_reusable() {
    let image = StillImage::from_rgb8(48, 32, common::pixels(48, 32)).unwrap();
    let reference = PerceptualReference::new(&image).unwrap();
    assert_eq!(reference.score_encoded(&png(&image)).unwrap(), 100.0);
    let candidate = image.encode(Format::Jpeg, EncodingMode::Quality(Quality::new(75.0).unwrap()), Effort::Fast).unwrap();
    let measured = reference.score_encoded(&candidate).unwrap();
    assert!(measured.is_finite() && measured < 100.0);
    assert_eq!(measured, reference.score_encoded(&candidate).unwrap());
    assert!(reference.score_encoded(b"invalid encoded candidate").is_err());
}

#[test]
fn negative_scores_and_requested_score_bounds_are_not_conflated() {
    for value in [f64::NAN, f64::INFINITY, -0.01, 100.01] { assert!(ScoreTarget::new(value).is_err()); }
    assert_eq!(ScoreTarget::new(0.0).unwrap().value(), 0.0); assert_eq!(ScoreTarget::new(100.0).unwrap().value(), 100.0);
    let black = StillImage::from_rgb8(32, 32, common::pixels(32, 32)).unwrap();
    let white = StillImage::from_rgb8(32, 32, common::pixels(32, 32).iter().map(|v| 255 - v).collect()).unwrap();
    let score = compare(&png(&black), &png(&white)).unwrap();
    assert!(score < 0.0, "measured score was clamped: {score}");
}

#[test]
fn dimensions_alpha_and_tiny_image_contracts_are_explicit() {
    let tiny = StillImage::from_rgb8(2, 3, vec![30; 18]).unwrap();
    assert_eq!(compare(&png(&tiny), &png(&tiny)).unwrap(), 100.0);
    let changed = StillImage::from_rgb8(2, 3, vec![31; 18]).unwrap();
    assert!(compare(&png(&tiny), &png(&changed)).is_err());
    let other = StillImage::from_rgb8(3, 2, vec![30; 18]).unwrap();
    assert!(compare(&png(&tiny), &png(&other)).is_err());
    let alpha = StillImage::from_samples(16, 16, SampleLayout::Rgba8, [80, 120, 160, 42].repeat(256)).unwrap();
    let changed = StillImage::from_samples(16, 16, SampleLayout::Rgba8, [80, 120, 160, 43].repeat(256)).unwrap();
    assert!(compare(&png(&alpha), &png(&changed)).is_err());
    let invisible = StillImage::from_samples(16, 16, SampleLayout::Rgba8, [90, 30, 10, 0].repeat(256)).unwrap();
    let hidden = StillImage::from_samples(16, 16, SampleLayout::Rgba8, [10, 90, 30, 0].repeat(256)).unwrap();
    assert!((compare(&png(&invisible), &png(&hidden)).unwrap() - 100.0).abs() < 0.00001);
}

#[test]
fn score_uses_the_lower_of_black_and_white_linear_composites() {
    let source = StillImage::from_samples(32, 24, SampleLayout::Rgba8, common::pixels(32, 24).chunks_exact(3)
        .enumerate().flat_map(|(i, p)| [p[0], p[1], p[2], (i % 256) as u8]).collect()).unwrap();
    let candidate = source.encode(Format::Webp, EncodingMode::Quality(Quality::new(50.0).unwrap()), Effort::Fast).unwrap();
    let measured = compare(&png(&source), &candidate).unwrap();
    assert!(measured.is_finite() && measured < 100.0);
    let prepared_source = source.prepare().unwrap(); let prepared_candidate = StillImage::decode(&candidate).unwrap().prepare().unwrap();
    let score = |background: f32| {
        let composite = |prepared: &rimage::still::PreparedImage| -> Vec<[f32; 3]> {
            prepared.linear_rgb().iter().zip(prepared.alpha()).map(|(p, &a)| p.map(|v| v * (a as f32 / 255.0) + background * (1.0 - a as f32 / 255.0))).collect()
        };
        let a = composite(&prepared_source); let b = composite(&prepared_candidate);
        let a = zenpixels::PixelSlice::new(bytemuck::cast_slice(&a), 32, 24, 32 * 12, zenpixels::PixelDescriptor::RGBF32_LINEAR).unwrap();
        let b = zenpixels::PixelSlice::new(bytemuck::cast_slice(&b), 32, 24, 32 * 12, zenpixels::PixelDescriptor::RGBF32_LINEAR).unwrap();
        fast_ssim2::compute_ssimulacra2(&a, &b).unwrap()
    };
    assert!((measured - score(0.0).min(score(1.0))).abs() < 0.000001);
}
