#[path = "../benches/support/mod.rs"]
mod support;
use img::perceptual::PerceptualReference;
use img::still::{Effort, EncodingMode, Format, Quality, StillImage};
use serde_json::json;
use std::collections::HashSet;
use std::time::Instant;
use webpkit::{Dimensions, Encoder, ImageRef, Metadata, PixelLayout};

fn rgba(image: &StillImage) -> Vec<u8> {
    let samples = image.samples();
    if image.is_grayscale() {
        if image.has_alpha() {
            samples.chunks_exact(2).flat_map(|px| [px[0], px[0], px[0], px[1]]).collect()
        } else {
            samples.iter().flat_map(|&g| [g, g, g, 255]).collect()
        }
    } else if image.has_alpha() {
        samples
    } else {
        samples.chunks_exact(3).flat_map(|px| [px[0], px[1], px[2], 255]).collect()
    }
}

fn image_ref(image: &StillImage) -> Option<(Dimensions, ImageRef<'static>)> {
    let (w, h) = image.dimensions();
    let dims = Dimensions::new(w, h).ok()?;
    let samples = rgba(image);
    // Leak the buffer: the collector is a short-lived diagnostic.
    let leaked: &'static [u8] = Box::leak(samples.into_boxed_slice());
    ImageRef::new(dims, PixelLayout::Rgba8, leaked).ok().map(|r| (dims, r))
}

fn metadata(_image: &StillImage) -> Metadata {
    Metadata::none()
}

fn lossless_bytes(image: &StillImage, level: u8) -> Option<(usize, f64)> {
    let (_, reference) = image_ref(image)?;
    let t = Instant::now();
    let out = Encoder::lossless()
        .effort(webpkit::Effort::level(level))
        .metadata(metadata(image))
        .encode_ref(reference)
        .ok()?;
    Some((out.len(), t.elapsed().as_secs_f64() * 1000.0))
}

fn colors_le_256(image: &StillImage) -> bool {
    let samples = image.samples();
    let channels = if image.is_grayscale() {
        if image.has_alpha() { 2 } else { 1 }
    } else if image.has_alpha() {
        4
    } else {
        3
    };
    let mut seen = HashSet::new();
    for px in samples.chunks_exact(channels) {
        if !seen.insert(px.to_vec()) && seen.len() > 256 {
            return false;
        }
    }
    true
}

fn local_delta_entropy_bits(samples: &[u8], bytes_per_pixel: usize) -> f64 {
    let step = (samples.len() / (bytes_per_pixel * 65536)).max(1);
    let mut hist = [0u64; 256];
    let mut n = 0u64;
    let mut prev: Option<&[u8]> = None;
    let mut i = 0usize;
    while i + bytes_per_pixel <= samples.len() {
        let px = &samples[i..i + bytes_per_pixel];
        if let Some(p) = prev {
            for c in 0..bytes_per_pixel {
                hist[px[c].wrapping_sub(p[c]) as usize] += 1;
                n += 1;
            }
        }
        prev = Some(px);
        i += bytes_per_pixel * step;
    }
    let mut h = 0.0f64;
    for &c in &hist {
        if c > 0 {
            let p = c as f64 / n as f64;
            h -= p * p.log2();
        }
    }
    h
}

fn fast_lossless_level(pixels: u64) -> u8 {
    if pixels <= 16_384 || (200_000..=400_000).contains(&pixels) {
        6
    } else {
        1
    }
}

fn run_case(name: &str, bytes: &[u8], corpus: &str) -> serde_json::Value {
    let image = match StillImage::decode(bytes) {
        Ok(i) => i,
        Err(e) => return json!({"name": name, "corpus": corpus, "decodeError": e.to_string()}),
    };
    let (w, h) = image.dimensions();
    let pixels = u64::from(w) * u64::from(h);
    let bpp = if image.is_grayscale() {
        if image.has_alpha() { 2 } else { 1 }
    } else if image.has_alpha() {
        4
    } else {
        3
    };
    let entropy = if image.bit_depth() == 8 {
        local_delta_entropy_bits(&image.samples(), bpp)
    } else {
        f64::NAN
    };
    let palette_ok = colors_le_256(&image);
    let reference = match PerceptualReference::new(&image) {
        Ok(r) => r,
        Err(e) => {
            return json!({"name": name, "corpus": corpus, "w": w, "h": h,
                "refusal": e.to_string(), "entropy": entropy, "paletteOk": palette_ok});
        },
    };
    let mut lossy = vec![];
    for q in [95.0f32, 50.0, 0.0, 100.0, 75.0, 25.0] {
        let t = Instant::now();
        let enc = image.encode_cancellable(
            Format::Webp,
            EncodingMode::Quality(Quality::new(q).unwrap()),
            Effort::Fast,
            &img::cancellation::Cancellation::default(),
        );
        let enc_ms = t.elapsed().as_secs_f64() * 1000.0;
        match enc {
            Ok(candidate) => {
                let score = reference.score_encoded(&candidate).ok();
                lossy.push(json!({"q": q, "bytes": candidate.len(), "score": score, "ms": enc_ms}));
            },
            Err(e) => lossy.push(json!({"q": q, "error": e.to_string(), "ms": enc_ms})),
        }
    }
    // L0 probe (product `webp::probe` equivalent).
    let probe = lossless_bytes(&image, 0);
    // Deep Fast-band lossless — only when --deep is passed (phase 2).
    let deep = if std::env::var("GATE_DEEP").is_ok() {
        lossless_bytes(&image, fast_lossless_level(pixels))
    } else {
        None
    };
    json!({
        "name": name, "corpus": corpus, "w": w, "h": h, "pixels": pixels,
        "entropy": entropy, "paletteOk": palette_ok,
        "probe": probe.map(|(b, ms)| json!({"bytes": b, "ms": ms})),
        "deep": deep.map(|(b, ms)| json!({"bytes": b, "ms": ms})),
        "lossy": lossy,
    })
}

fn main() {
    let deep = std::env::var("GATE_DEEP").is_ok();
    let mut cases: Vec<(String, Vec<u8>, String)> = support::photographic()
        .into_iter()
        .map(|(n, b)| (n, b, "dev".to_string()))
        .collect();
    cases.extend(support::generated().into_iter().map(|(n, b)| (n, b, "generated".to_string())));
    cases.extend(support::validation());
    let filter: Option<Vec<String>> = std::env::var("GATE_ONLY").ok().map(|v| {
        v.split(',').map(str::to_string).collect()
    });
    if let Some(f) = &filter {
        cases.retain(|(n, _, _)| f.iter().any(|x| x == n));
    }
    for (name, bytes, corpus) in &cases {
        eprintln!("case {name} ({corpus}){}", if deep { " deep" } else { "" });
        let record = run_case(name, bytes, corpus);
        println!("{}", serde_json::to_string(&record).unwrap());
    }
}
