mod common;
use std::{io::Write, path::Path, process::Command};

fn fixture(case: u32) -> (u32, u32, Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let (w, h) = [(8, 8), (32, 32), (63, 47), (128, 96)][case as usize];
    let a: Vec<_> = (0..h).flat_map(|y| (0..w).map(move |x| {
        match case {
            0 => [x as f32 / 7.0, y as f32 / 7.0, 0.25],
            1 => std::array::from_fn(|k| ((x * 17 + y * 3 + k as u32 * 31) % 256) as f32 / 255.0),
            2 => [if (x / 3 + y / 5) % 2 == 0 { 0.9 } else { 0.05 }; 3],
            _ => [x as f32 / w as f32, y as f32 / h as f32, (x ^ y) as f32 / 255.0],
        }
    })).collect();
    let b = a.iter().enumerate().map(|(i, p)| p.map(|v| (v * 0.92 + (i % 7) as f32 * 0.003).clamp(0.0, 1.0))).collect();
    (w, h, a, b)
}
fn local(w: u32, h: u32, a: &[[f32; 3]], b: &[[f32; 3]]) -> f64 {
    let view = |p| zenpixels::PixelSlice::new(p, w, h, w as usize * 12, zenpixels::PixelDescriptor::RGBF32_LINEAR).unwrap();
    fast_ssim2::Ssimulacra2Reference::new(&view(bytemuck::cast_slice(a))).unwrap().compare(&view(bytemuck::cast_slice(b))).unwrap()
}
fn pfm(path: &Path, w: u32, h: u32, pixels: &[[f32; 3]]) {
    let mut file = std::fs::File::create(path).unwrap();
    write!(file, "PF\n{w} {h}\n-1.0\n").unwrap();
    for row in pixels.chunks_exact(w as usize).rev() { for pixel in row { for value in pixel { file.write_all(&value.to_le_bytes()).unwrap(); } } }
}
#[test]
#[ignore = "requires separately installed JPEG XL 0.12.0 ssimulacra2 oracle"]
fn independently_prepared_linear_pixels() {
    let scratch = common::Scratch::new("metric-reference");
    let oracle = std::env::var_os("SSIMULACRA2_REFERENCE").unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/metric-oracle").into_os_string());
    for case in 0..4 {
        let (w,h,a,b) = fixture(case);
        let first = scratch.0.join("source.pfm"); let second = scratch.0.join("candidate.pfm");
        pfm(&first,w,h,&a); pfm(&second,w,h,&b);
        let output = Command::new(&oracle).arg(first).arg(second).output().expect("install JPEG XL ssimulacra2 oracle");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let expected: f64 = std::str::from_utf8(&output.stdout).unwrap().trim().parse().unwrap();
        let actual = local(w,h,&a,&b);
        eprintln!("case {case}: oracle={expected:.10} actual={actual:.10}");
        assert!(expected.is_finite() && (actual-expected).abs() <= 0.05);
    }
}

#[test]
fn pinned_linear_pixel_goldens() {
    for (case, expected) in [97.71711900, 90.51222015, 93.48609084, 87.52320950].into_iter().enumerate() {
        let (w,h,a,b) = fixture(case as u32);
        assert!((local(w,h,&a,&b)-expected).abs() <= 0.05, "case {case}");
    }
}
