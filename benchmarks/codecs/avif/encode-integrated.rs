use std::{env, error::Error, fs, time::Instant};
use img::still::{Effort, EncodingMode, Format, Quality, SampleLayout, StillImage};

fn usage() -> libc::rusage { unsafe { let mut r: libc::rusage = std::mem::zeroed(); assert_eq!(libc::getrusage(libc::RUSAGE_SELF, &mut r), 0); r } }
fn user(r: &libc::rusage) -> f64 { r.ru_utime.tv_sec as f64 * 1000.0 + r.ru_utime.tv_usec as f64 / 1000.0 }
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<_> = env::args().collect();
    let data = fs::read(&a[2])?;
    let mut pos = 0;
    let mut tokens = Vec::new();
    while tokens.len() < 4 {
        while data[pos].is_ascii_whitespace() { pos += 1; }
        let start = pos;
        while !data[pos].is_ascii_whitespace() { pos += 1; }
        tokens.push(std::str::from_utf8(&data[start..pos])?);
    }
    pos += 1;
    let gray = tokens[0] == "P5";
    assert!(gray || tokens[0] == "P6");
    let width = tokens[1].parse()?;
    let height = tokens[2].parse()?;
    assert_eq!(tokens[3], "255");
    let image = StillImage::from_samples(width, height, if gray { SampleLayout::Gray8 } else { SampleLayout::Rgb8 }, data[pos..].to_vec())?;
    let q: f32 = a[3].parse()?;
    let before = usage();
    let start = Instant::now();
    let output = image.encode(Format::Avif, EncodingMode::Quality(Quality::new(q)?), match a[1].as_str() { "img-libaom-fast" => Effort::Fast, "img-libaom-balanced" => Effort::Balanced, "img-libaom-best" => Effort::Best, _ => return Err("unknown configuration".into()) })?;
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    let after = usage();
    if a.len() > 4 { fs::write(&a[4], &output)?; }
    println!("{{\"wall_ms\":{},\"cpu_ms\":{},\"rss_bytes\":{},\"output_bytes\":{}}}", elapsed, user(&after)-user(&before), after.ru_maxrss, output.len());
    Ok(())
}
