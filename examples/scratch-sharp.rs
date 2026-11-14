use std::{fs,path::Path,time::Instant};
use img::perceptual::PerceptualReference;
use img::still::StillImage;
use webpkit::{Dimensions, ImageRef, Image, Metadata, LossyTuning, Effort};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = fs::read(Path::new(&args[1])).unwrap();
    let image = StillImage::decode(&bytes).unwrap();
    let (w, h) = image.dimensions();
    let samples = image.samples();
    let stride = samples.len() / (w as usize * h as usize);
    let rgba: Vec<u8> = match stride {
        1 => samples.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        3 => samples.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        4 => samples,
        _ => panic!(),
    };
    let dims = Dimensions::new(w, h).unwrap();
    let img = ImageRef::new(dims, webpkit::PixelLayout::Rgba8, &rgba).unwrap();
    let reference = PerceptualReference::new(&image).unwrap();
    for q in [90u8, 95, 100] {
        for sharp in [false, true] {
            let tune = LossyTuning::new().with_sharp_yuv(sharp);
            let t = Instant::now();
            let out = webpkit::Encoder::lossy().quality(q).effort(Effort::level(9)).tuning(tune).metadata(Metadata::none()).encode_ref(img).unwrap();
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            let score = reference.score_encoded(&out).unwrap();
            println!("q{q} sharp={sharp}: {} B {ms:.0} ms score={score:.2}", out.len());
        }
    }
}
