use std::{env, error::Error, fs, hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<_> = env::args().collect(); let ms: f64 = a[2].parse()?; let data = fs::read(&a[3])?;
    let run = || zenavif::decode_with(&data, &zenavif::DecoderConfig::new().threads(1).apply_grain(true), &zenavif::Unstoppable);
    if a.len() > 4 { fs::write(&a[4], run()?.copy_to_contiguous_bytes())?; return Ok(()); }
    if ms > 0.0 { run()?; } let start = Instant::now(); let mut n = 0;
    while n == 0 || start.elapsed().as_secs_f64() * 1000.0 < ms { black_box(run()?); n += 1; }
    println!("{{\"iterations\":{n},\"mean_ms\":{}}}",start.elapsed().as_secs_f64()*1000.0/n as f64); Ok(())
}
