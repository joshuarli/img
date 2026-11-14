use std::{fs,path::Path};
use img::still::StillImage;
use webpkit::{Dimensions, ImageRef, PixelLayout, Metadata, LossyTuning, Effort};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let image = StillImage::decode(&fs::read(Path::new(&args[1])).unwrap()).unwrap();
    let quality: u8 = args[2].parse().unwrap();
    let sns: u8 = args[3].parse().unwrap();
    let (w, h) = image.dimensions();
    let samples = image.samples();
    let rgba: Vec<u8> = match samples.len() as u64 / (w as u64 * h as u64) {
        1 => samples.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        3 => samples.chunks_exact(3).flat_map(|px| [px[0], px[1], px[2], 255]).collect(),
        4 => samples,
        _ => panic!(),
    };
    let dims = Dimensions::new(w, h).unwrap();
    let img = ImageRef::new(dims, PixelLayout::Rgba8, &rgba).unwrap();
    let bytes = webpkit::Encoder::lossy().quality(quality).effort(Effort::level(9)).tuning(LossyTuning::new().with_sns_strength(sns)).metadata(Metadata::none()).encode_ref(img).unwrap();
    fs::write(Path::new(&args[4]), &bytes).unwrap();
    println!("{}", bytes.len());
}
