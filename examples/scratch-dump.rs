use std::fs::File;
use std::io::Write;
use std::path::Path;
use img::still::StillImage;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let image = StillImage::decode(&std::fs::read(Path::new(&args[1])).unwrap()).unwrap();
    let (w, h) = image.dimensions();
    let samples = image.samples();
    let stride = samples.len() / (w as usize * h as usize);
    let rgba: Vec<u8> = match stride {
        1 => samples.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        3 => samples.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        4 => samples,
        _ => panic!(),
    };
    let mut f = File::create(&args[2]).unwrap();
    f.write_all(&w.to_le_bytes()).unwrap();
    f.write_all(&h.to_le_bytes()).unwrap();
    f.write_all(&rgba).unwrap();
    eprintln!("wrote {w}x{h} stride={stride}");
}
