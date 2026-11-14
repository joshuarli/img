use std::{fs,path::Path,time::Instant};
use img::perceptual::PerceptualReference;
use img::still::{StillImage, EncodingMode, Effort, Format, Quality};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = fs::read(Path::new(&args[1])).unwrap();
    let image = StillImage::decode(&bytes).unwrap();
    let reference = PerceptualReference::new(&image).unwrap();
    for q in args.iter().skip(2) {
        let q8: f32 = q.parse().unwrap();
        let t = Instant::now();
        let out = image.encode_cancellable(Format::Webp, EncodingMode::Quality(Quality::new(q8).unwrap()), Effort::Fast, &img::cancellation::Cancellation::default()).unwrap();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        let score = reference.score_encoded(&out).unwrap();
        println!("q{q8:.1}: {} B {ms:.0} ms score={score:.2}", out.len());
    }
}
