use std::{env, error::Error, fs, time::Instant};
use enough::Unstoppable;

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
    let width: u32 = tokens[1].parse()?;
    let height: u32 = tokens[2].parse()?;
    assert_eq!(tokens[3], "255");
    let pixels = &data[pos..];
    assert_eq!(pixels.len(), width as usize * height as usize * if gray { 1 } else { 3 });
    let q: u8 = a[3].parse()?;
    let before = usage();
    let start = Instant::now();
    let output = match a[1].as_str() {
        "img-fast" | "img-best" => {
            let preset = if a[1] == "img-fast" { mozjpeg_rs::Preset::ProgressiveBalanced } else { mozjpeg_rs::Preset::ProgressiveSmallest };
            let mut encoder = mozjpeg_rs::Encoder::new(preset).quality(q);
            if !gray { encoder = encoder.subsampling(mozjpeg_rs::Subsampling::S420); }
            if gray { encoder.encode_gray_with_stop(pixels, width, height, &Unstoppable)? } else { encoder.encode_rgb_with_stop(pixels, width, height, &Unstoppable)? }
        }
        "zen-fast" | "zen-best" => {
            use zenjpeg::encoder::{ChromaSubsampling, EncoderConfig, OptimizationPreset, PixelLayout};
            let config = if gray { EncoderConfig::grayscale(q as f32) } else { EncoderConfig::ycbcr(q as f32, ChromaSubsampling::Quarter) };
            let preset = if a[1] == "zen-fast" { OptimizationPreset::JpegliBaseline } else { OptimizationPreset::JpegliProgressive };
            let config = config.optimization(preset).progressive(true);
            config.request().stop(&Unstoppable).encode_bytes(pixels, width, height, if gray { PixelLayout::Gray8Srgb } else { PixelLayout::Rgb8Srgb })?
        }
        _ => return Err("unknown encoder".into()),
    };
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    let after = usage();
    if a.len() > 4 { fs::write(&a[4], &output)?; }
    println!("{{\"wall_ms\":{},\"cpu_ms\":{},\"rss_bytes\":{},\"output_bytes\":{}}}", elapsed, user(&after)-user(&before), after.ru_maxrss, output.len());
    Ok(())
}
