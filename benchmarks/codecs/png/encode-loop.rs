use std::{fs, time::Instant};
use img::still::{Effort, EncodingMode, Format, StillImage};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, path, effort, threads] = &args[..] else { panic!("usage: png-encode-loop FILE fast|balanced|best THREADS") };
    // SAFETY: single-threaded here, before any worker pool exists.
    unsafe { std::env::set_var("RAYON_NUM_THREADS", threads) };
    let effort = match effort.as_str() { "fast" => Effort::Fast, "balanced" => Effort::Balanced, "best" => Effort::Best, other => panic!("unknown effort {other}") };
    let input = fs::read(path).unwrap();
    let start = Instant::now();
    let image = StillImage::decode(&input).unwrap();
    let decode_ms = start.elapsed().as_secs_f64() * 1e3;
    let (mut runs, start) = (0u32, Instant::now());
    let mut encoded = Vec::new();
    while runs == 0 || start.elapsed().as_secs_f64() < 0.15 {
        encoded = image.encode(Format::Png, EncodingMode::Lossless, effort).unwrap();
        runs += 1;
    }
    let encode_ms = start.elapsed().as_secs_f64() * 1e3 / f64::from(runs);
    let start = Instant::now();
    let decoded = StillImage::decode(&encoded).unwrap();
    let verify_ms = start.elapsed().as_secs_f64() * 1e3;
    let start = Instant::now();
    let equal = img::optimizer::samples_equal(&image, &decoded);
    let compare_ms = start.elapsed().as_secs_f64() * 1e3;
    println!("{} bytes decode {decode_ms:.3} encode {encode_ms:.3} verify-decode {verify_ms:.3} compare {compare_ms:.3} eq={equal}", encoded.len());
    if !equal { std::process::exit(1); }
}
