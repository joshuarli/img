use img::{perceptual::{PerceptualReference, ReferenceMemory}, still::{Effort, EncodingMode, Format, SampleLayout, StillImage}};
use zenpixels::{PixelDescriptor, PixelSlice};

fn noise(len: usize, seed: u32) -> Vec<u8> {
    let mut state = seed.wrapping_mul(2654435761).max(1);
    (0..len).map(|i| { state ^= state << 13; state ^= state >> 17; state ^= state << 5; ((state >> 24) as u8 / 4).wrapping_add((i % 97) as u8 * 2) }).collect()
}

fn oracle(source: &StillImage, candidate: &StillImage) -> f64 {
    let (a, b) = (source.prepare().unwrap(), candidate.prepare().unwrap());
    let (w, h) = a.dimensions();
    fn slice(p: &[[f32; 3]], w: u32, h: u32) -> PixelSlice<'_> { PixelSlice::new(bytemuck::cast_slice(p), w, h, w as usize * 12, PixelDescriptor::RGBF32_LINEAR).unwrap() }
    let score = |x: &[[f32; 3]], y: &[[f32; 3]]| fast_ssim2::compute_ssimulacra2(&slice(x, w, h), &slice(y, w, h)).unwrap();
    if a.alpha().iter().all(|&v| v == 255) { return score(a.linear_rgb(), b.linear_rgb()); }
    let over = |p: &img::still::PreparedImage, bg: f32| -> Vec<[f32; 3]> {
        p.linear_rgb().iter().zip(p.alpha()).map(|(rgb, &a)| { let a = a as f32 / 255.0; rgb.map(|v| v * a + bg * (1.0 - a)) }).collect()
    };
    score(&over(&a, 0.0), &over(&b, 0.0)).min(score(&over(&a, 1.0), &over(&b, 1.0)))
}

#[test]
fn planar_streamed_scores_match_the_uncached_metric_bit_for_bit() {
    for (w, h) in [(8, 8), (9, 8), (8, 9), (15, 9), (17, 23), (23, 17), (31, 33), (64, 40), (100, 91), (257, 129), (13, 300), (300, 13)] {
        for layout in [SampleLayout::Rgb8, SampleLayout::Rgba8, SampleLayout::Gray8] {
            let channels = match layout { SampleLayout::Rgb8 => 3, SampleLayout::Rgba8 => 4, _ => 1 };
            let samples = noise(w * h * channels, (w * 31 + h) as u32);
            let mut changed = noise(w * h * channels, (w * 7 + h * 3) as u32);
            if channels == 4 { for (c, s) in changed.chunks_exact_mut(4).zip(samples.chunks_exact(4)) { c[3] = s[3]; } }
            let source = StillImage::from_samples(w as u32, h as u32, layout, samples).unwrap();
            let candidate = StillImage::from_samples(w as u32, h as u32, layout, changed).unwrap();
            let bytes = candidate.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
            let expected = oracle(&source, &StillImage::decode(&bytes).unwrap());
            for memory in [ReferenceMemory::Fast, ReferenceMemory::Compact] {
                let measured = PerceptualReference::with_memory(&source, memory).unwrap().score_encoded(&bytes).unwrap();
                assert_eq!(measured.to_bits(), expected.to_bits(), "{w}x{h} {layout:?} {memory:?}: {measured} vs {expected}");
            }
        }
    }
}

#[test]
fn parallel_streamed_scores_match_serial_bits() {
    let serial = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
    let parallel = rayon::ThreadPoolBuilder::new().num_threads(3).build().unwrap();
    for (w, h, layout) in [
        (257, 129, SampleLayout::Rgb8),
        (513, 257, SampleLayout::Rgba8),
        (1025, 1025, SampleLayout::Rgb8),
        (1025, 1025, SampleLayout::Rgba8),
        (1201, 901, SampleLayout::Rgb8),
        (2001, 2000, SampleLayout::Rgb8),
    ] {
        let channels = if layout == SampleLayout::Rgba8 { 4 } else { 3 };
        let samples = noise(w * h * channels, (w * 31 + h) as u32);
        let mut changed = noise(w * h * channels, (w * 7 + h * 3) as u32);
        if channels == 4 { for (c, s) in changed.chunks_exact_mut(4).zip(samples.chunks_exact(4)) { c[3] = s[3]; } }
        let source = StillImage::from_samples(w as u32, h as u32, layout, samples).unwrap();
        let candidate = StillImage::from_samples(w as u32, h as u32, layout, changed).unwrap();
        let bytes = candidate.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
        for memory in [ReferenceMemory::Fast, ReferenceMemory::Compact] {
            let reference = PerceptualReference::with_memory(&source, memory).unwrap();
            let a = serial.install(|| reference.score_encoded(&bytes).unwrap());
            let b = parallel.install(|| reference.score_encoded(&bytes).unwrap());
            assert_eq!(a.to_bits(), b.to_bits(), "{w}x{h} {layout:?} {memory:?}: {a} vs {b}");
        }
    }
}
