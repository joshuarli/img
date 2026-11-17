use img::still::StillImage;
use webpkit::{Dimensions, Effort, Encoder, ImageRef, Metadata, PixelLayout};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let level: u8 = args[2].parse().unwrap();
    let image = StillImage::decode(&std::fs::read(&args[1]).unwrap()).unwrap();
    assert_eq!(image.bit_depth(), 8);
    let (w, h) = image.dimensions();
    let samples = image.samples();
    let rgba: Vec<u8> = if image.is_grayscale() {
        if image.has_alpha() {
            samples.chunks_exact(2).flat_map(|px| [px[0], px[0], px[0], px[1]]).collect()
        } else {
            samples.iter().flat_map(|&g| [g, g, g, 255]).collect()
        }
    } else if image.has_alpha() {
        samples
    } else {
        samples.chunks_exact(3).flat_map(|px| [px[0], px[1], px[2], 255]).collect()
    };
    let dims = Dimensions::new(w, h).unwrap();
    let img = ImageRef::new(dims, PixelLayout::Rgba8, &rgba).unwrap();
    let t = std::time::Instant::now();
    let out = Encoder::lossless().effort(Effort::level(level)).metadata(Metadata::none()).encode_ref(img).unwrap();
    println!("{}x{} level={} bytes={} ms={:.0}", w, h, level, out.len(), t.elapsed().as_secs_f64() * 1000.0);
}
