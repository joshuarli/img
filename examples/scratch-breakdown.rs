use std::{fs,path::Path,time::Instant};
use img::perceptual::PerceptualReference;
use img::still::StillImage;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let t0 = Instant::now();
    let image = StillImage::decode(&fs::read(Path::new(&args[1])).unwrap()).unwrap();
    let decode_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let (w, h) = image.dimensions();
    eprintln!("decode {w}x{h} = {decode_ms:.1} ms");
    let reference = PerceptualReference::new(&image).unwrap();
    // WebP lossless candidate (production fast path: level 1)
    let pw = Instant::now();
    let _out = image.encode_cancellable(img::still::Format::Webp, img::still::EncodingMode::Lossless, img::still::Effort::Fast, &img::cancellation::Cancellation::default()).unwrap();
    eprintln!("lossless candidate: {} B in {:.1} ms", _out.len(), pw.elapsed().as_secs_f64()*1000.0);
    for q in [95u8, 50, 0, 100, 75, 25] {
        let pw = Instant::now();
        let bytes = image.encode_cancellable(img::still::Format::Webp, img::still::EncodingMode::Quality(img::still::Quality::new(f32::from(q)).unwrap()), img::still::Effort::Fast, &img::cancellation::Cancellation::default()).unwrap();
        let enc_ms = pw.elapsed().as_secs_f64()*1000.0;
        let ps = Instant::now();
        let score = reference.score_encoded(&bytes).unwrap();
        let score_ms = ps.elapsed().as_secs_f64()*1000.0;
        eprintln!("q{q:3}: {} B encode={enc_ms:.1} ms score={score:.2} (score_ms={score_ms:.1})", bytes.len());
    }
    // PNG lossless candidate
    let pw = Instant::now();
    let png = image.encode_cancellable(img::still::Format::Png, img::still::EncodingMode::Lossless, img::still::Effort::Fast, &img::cancellation::Cancellation::default()).unwrap();
    eprintln!("png candidate: {} B in {:.1} ms", png.len(), pw.elapsed().as_secs_f64()*1000.0);
}
