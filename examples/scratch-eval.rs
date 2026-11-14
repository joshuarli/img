use img::optimizer::{self, Fidelity, Operation, Request};
use img::perceptual::ScoreTarget;
use img::still::{Effort, Format};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1]).unwrap();
    let t80 = Request { operation: Operation::Convert(Format::Webp), fidelity: Fidelity::Score(ScoreTarget::new(args[2].parse().unwrap()).unwrap()), effort: Effort::Fast, ..Request::default() };
    let out = optimizer::process(&bytes, t80).unwrap();
    for e in &out.evaluations {
        println!("format={:?} quality={:?} bytes={:?} score={:?} failure={:?}", e.format, e.quality, e.bytes, e.score, e.failure);
    }
    println!("winner format={:?} bytes={} score={:?}", out.format, out.bytes.len(), out.score);
}
