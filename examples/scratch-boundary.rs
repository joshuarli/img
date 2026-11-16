use img::still::{Effort, EncodingMode, Format, StillImage};
use std::fs;

fn noise(w: u32, h: u32, seed: u64) -> Vec<u8> {
    let mut state = seed;
    let mut rand = move || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 33) as u32
    };
    let cell = 8u32;
    let gw = w / cell + 2;
    let gh = h / cell + 2;
    let grid: Vec<u32> = (0..gw * gh).map(|_| rand() & 0xffff).collect();
    let mut out = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let fx = x as f32 / cell as f32;
            let fy = y as f32 / cell as f32;
            let x0 = fx.floor() as u32;
            let y0 = fy.floor() as u32;
            let tx = fx - x0 as f32;
            let ty = fy - y0 as f32;
            let sx = tx * tx * (3.0 - 2.0 * tx);
            let sy = ty * ty * (3.0 - 2.0 * ty);
            let g = |dx: u32, dy: u32| grid[((y0 + dy) * gw + x0 + dx) as usize] as f32 / 65535.0;
            let v = g(0, 0) * (1.0 - sx) * (1.0 - sy)
                + g(1, 0) * sx * (1.0 - sy)
                + g(0, 1) * (1.0 - sx) * sy
                + g(1, 1) * sx * sy;
            let grain = (rand() & 0xff) as f32 / 255.0 * 0.15;
            let base = ((v * 0.75 + 0.15 + grain).clamp(0.0, 1.0) * 255.0) as u8;
            out.extend_from_slice(&[base, (base as u16 * 3 / 4) as u8, (255 - base) / 2]);
        }
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args().nth(1).ok_or("output directory required")?;
    fs::create_dir_all(&out)?;
    let cases: &[(&str, u32, u32, u64)] = &[
        ("boundary-16383.png", 127, 129, 11),
        ("boundary-16384.png", 128, 128, 12),
        ("boundary-16512.png", 129, 128, 13),
        ("boundary-199809.png", 447, 447, 14),
        ("boundary-200000.png", 500, 400, 15),
        ("boundary-200704.png", 448, 448, 16),
        ("boundary-399424.png", 632, 632, 17),
        ("boundary-400000.png", 625, 640, 18),
        ("boundary-400689.png", 633, 633, 19),
        ("boundary-1048575.png", 1023, 1025, 20),
        ("boundary-1048576.png", 1024, 1024, 21),
        ("boundary-1050625.png", 1025, 1025, 22),
    ];
    for (name, w, h, seed) in cases {
        let image = StillImage::from_rgb8(*w, *h, noise(*w, *h, *seed))?;
        let bytes = image.encode(Format::Png, EncodingMode::Lossless, Effort::Fast)?;
        fs::write(format!("{out}/{name}"), bytes)?;
        println!("{name} {}x{}", w, h);
    }
    Ok(())
}
