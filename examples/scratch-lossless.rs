use std::{fs,path::Path,time::Instant};
use img::still::StillImage;
use webpkit::{Dimensions, ImageRef, PixelLayout, Metadata};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let image = StillImage::decode(&fs::read(Path::new(&args[1])).unwrap()).unwrap();
    let (w, h) = image.dimensions();
    let px = image.samples();
    let stride = px.len() / (w as usize * h as usize);
    let rgba: Vec<u8> = match stride {
        1 => px.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        3 => px.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        4 => px,
        _ => panic!(),
    };
    let dims = Dimensions::new(w, h).unwrap();
    let img = ImageRef::new(dims, PixelLayout::Rgba8, &rgba).unwrap();
    for i in 2..args.len() {
        let effort = args[i].parse::<u8>().unwrap();
        let start = Instant::now();
        let bytes = webpkit::Encoder::lossless().effort(webpkit::Effort::level(effort)).metadata(Metadata::none()).encode_ref(img).unwrap();
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        println!("effort={effort} bytes={} time={ms:.0}ms", bytes.len());
    }
}
