use std::{fs,path::Path};
use img::perceptual::PerceptualReference;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let image = img::still::StillImage::decode(&fs::read(Path::new(&args[1])).unwrap()).unwrap();
    let reference = PerceptualReference::new(&image).unwrap();
    for p in args.iter().skip(2) {
        let bytes = fs::read(p).unwrap();
        let score = reference.score_encoded(&bytes).unwrap();
        println!("{p}: len={} score={score:.2}", bytes.len());
    }
}
