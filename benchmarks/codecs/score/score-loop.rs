use std::{fs, time::Instant};
use img::{perceptual::PerceptualReference, still::StillImage};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let min_ms: f64 = args[1].parse().unwrap();
    let reference = PerceptualReference::new(&StillImage::decode(&fs::read(&args[2]).unwrap()).unwrap()).unwrap();
    for path in &args[3..] {
        let bytes = fs::read(path).unwrap();
        let score = reference.score_encoded(&bytes).unwrap();
        let (mut runs, start) = (0u32, Instant::now());
        while start.elapsed().as_secs_f64() * 1e3 < min_ms {
            assert_eq!(reference.score_encoded(&bytes).unwrap().to_bits(), score.to_bits());
            runs += 1;
        }
        println!("{path}\t{:.17}\t{:.4}", score, start.elapsed().as_secs_f64() * 1e3 / f64::from(runs));
    }
}
