use img::still::StillImage;

fn entropy_hist(counts: &[u64; 256], n: u64) -> f64 {
    let mut h = 0.0f64;
    for &c in counts {
        if c > 0 { let p = c as f64 / n as f64; h -= p * p.log2(); }
    }
    h
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let image = StillImage::decode(&std::fs::read(&args[1]).unwrap()).unwrap();
    let samples = image.samples();
    let step = (samples.len() / 65536).max(1);
    let mut h0c = [0u64; 256];
    let mut n0 = 0u64;
    for &b in samples.iter().step_by(step) { h0c[b as usize] += 1; n0 += 1; }
    let bpp = if image.is_grayscale() { if image.has_alpha() { 2 } else { 1 } } else if image.has_alpha() { 4 } else { 3 };
    let mut dh = [0u64; 256];
    let mut dn = 0u64;
    let mut prev: Option<&[u8]> = None;
    let mut i = 0usize;
    while i + bpp <= samples.len() {
        let px = &samples[i..i + bpp];
        if let Some(p) = prev {
            for c in 0..bpp { dh[px[c].wrapping_sub(p[c]) as usize] += 1; dn += 1; }
        }
        prev = Some(px);
        i += bpp * step.max(1);
    }
    println!("{}x{} depth={} H0={:.2} Hdelta={:.2}", image.dimensions().0, image.dimensions().1, image.bit_depth(), entropy_hist(&h0c, n0), entropy_hist(&dh, dn));
}
