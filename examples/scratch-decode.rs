use std::{fs,path::Path,time::Instant};
use img::still::StillImage;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut total = 0.0f64;
    for path in args.iter().skip(1) {
        let bytes = fs::read(Path::new(path)).unwrap();
        let t = Instant::now();
        let image = StillImage::decode(&bytes).unwrap();
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        let (w, h) = image.dimensions();
        total += ms;
        println!("{path}: {w}x{h} decode = {ms:.2} ms");
    }
    println!("total = {total:.2} ms");
}
