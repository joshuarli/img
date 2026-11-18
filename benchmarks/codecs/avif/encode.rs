use std::{env, error::Error, fs, time::Instant};

use imgref::ImgExt;

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
    assert_eq!(tokens[0], "P6");
    let width: u32 = tokens[1].parse()?;
    let height: u32 = tokens[2].parse()?;
    assert_eq!(tokens[3], "255");
    let pixels = &data[pos..];
    assert_eq!(pixels.len(), width as usize * height as usize * 3);
    let q: f32 = a[3].parse()?;
    let speed = match a[1].as_str() {
        "img-fast" | "zen-fast" => 10,
        "img-balanced" | "zen-balanced" => 8,
        "img-best" | "zen-best" => 6,
        _ => return Err("unknown encoder".into()),
    };
    let rgb: Vec<rgb::RGB8> = pixels.chunks_exact(3).map(|px| rgb::RGB8::new(px[0], px[1], px[2])).collect();
    let image = imgref::Img::new(rgb.as_slice(), width as usize, height as usize);
    let before = usage();
    let start = Instant::now();
    let output = if a[1].starts_with("img-") {
        ravif::Encoder::new()
            .with_quality(q.clamp(1.0, 100.0))
            .with_speed(speed)
            .with_bit_depth(ravif::BitDepth::Eight)
            .with_num_threads(Some(1))
            .encode_rgb(image.as_ref())?
            .avif_file
    } else {
        let config = zenavif::EncoderConfig::new()
            .quality(1.0 + q * 0.98)
            .speed(speed)
            .bit_depth(zenavif::EncodeBitDepth::Eight)
            .threads(Some(1))
            .color_primaries(1)
            .transfer_characteristics(13)
            .matrix_coefficients(6)
            .alpha_quality(100.0);
        zenavif::encode_rgb8(image.as_ref(), &config, almost_enough::StopToken::new(almost_enough::Unstoppable))?
            .avif_file
    };
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    let after = usage();
    if a.len() > 4 { fs::write(&a[4], &output)?; }
    println!("{{\"wall_ms\":{},\"cpu_ms\":{},\"rss_bytes\":{},\"output_bytes\":{}}}",
             elapsed, user(&after) - user(&before), after.ru_maxrss, output.len());
    Ok(())
}
