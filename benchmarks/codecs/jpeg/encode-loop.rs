use std::{env, error::Error, fs, hint::black_box, time::Instant};
use enough::Unstoppable;
use img::still::{Effort, EncodingMode, Format, Quality, SampleLayout, StillImage};

fn pnm(data: &[u8]) -> Result<(bool, u32, u32, &[u8]), Box<dyn Error>> {
    let (mut pos, mut tokens) = (0, Vec::new());
    while tokens.len() < 4 {
        while data[pos].is_ascii_whitespace() { pos += 1; }
        let start = pos;
        while !data[pos].is_ascii_whitespace() { pos += 1; }
        tokens.push(std::str::from_utf8(&data[start..pos])?);
    }
    Ok((tokens[0] == "P5", tokens[1].parse()?, tokens[2].parse()?, &data[pos + 1..]))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    let quality: f32 = args[1].parse()?;
    let iterations: usize = args[2].parse()?;
    let all = ["img-fast", "img-balanced", "img-best", "zen-fast", "zen-best"];
    let selected = env::var("ENCODERS").ok();
    let encoders: Vec<&str> = all.into_iter().filter(|name| selected.as_deref().is_none_or(|list| list.split(',').any(|s| s == *name))).collect();
    for path in &args[3..] {
        let data = fs::read(path)?;
        let (gray, width, height, pixels) = pnm(&data)?;
        let image = StillImage::from_samples(width, height, if gray { SampleLayout::Gray8 } else { SampleLayout::Rgb8 }, pixels.to_vec())?;
        let encode = |name: &str| -> Result<usize, Box<dyn Error>> {
            Ok(match name {
                "img-fast" | "img-balanced" | "img-best" => {
                    let effort = match name { "img-fast" => Effort::Fast, "img-balanced" => Effort::Balanced, _ => Effort::Best };
                    black_box(image.encode(Format::Jpeg, EncodingMode::Quality(Quality::new(quality)?), effort)?).len()
                }
                _ => {
                    use zenjpeg::encoder::{ChromaSubsampling, EncoderConfig, OptimizationPreset, PixelLayout};
                    let config = if gray { EncoderConfig::grayscale(quality) } else { EncoderConfig::ycbcr(quality, ChromaSubsampling::Quarter) };
                    let preset = if name == "zen-fast" { OptimizationPreset::JpegliBaseline } else { OptimizationPreset::JpegliProgressive };
                    black_box(config.optimization(preset).progressive(true).request().stop(&Unstoppable).encode_bytes(pixels, width, height, if gray { PixelLayout::Gray8Srgb } else { PixelLayout::Rgb8Srgb })?).len()
                }
            })
        };
        let mut times = vec![Vec::with_capacity(iterations); encoders.len()];
        let mut sizes = vec![0; encoders.len()];
        for (index, name) in encoders.iter().enumerate() { sizes[index] = encode(name)?; }
        for _ in 0..iterations {
            for (index, name) in encoders.iter().enumerate() {
                let start = Instant::now();
                encode(name)?;
                times[index].push(start.elapsed().as_secs_f64() * 1000.0);
            }
        }
        for (index, name) in encoders.iter().enumerate() {
            let t = &mut times[index];
            t.sort_by(f64::total_cmp);
            println!("{{\"input\":\"{path}\",\"encoder\":\"{name}\",\"quality\":{quality},\"median_ms\":{:.4},\"min_ms\":{:.4},\"max_ms\":{:.4},\"bytes\":{}}}", t[iterations / 2], t[0], t[iterations - 1], sizes[index]);
        }
    }
    Ok(())
}
