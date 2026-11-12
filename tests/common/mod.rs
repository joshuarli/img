#![allow(dead_code)]
use std::{fs, path::{Path, PathBuf}, process::Command, sync::atomic::{AtomicU64, Ordering}};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new(label: &str) -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".work/tmp")
            .join(format!("{label}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}

pub fn pixels(width: u32, height: u32) -> Vec<u8> {
    (0..height).flat_map(|y| (0..width).flat_map(move |x| [
        (x * 17 + y * 3) as u8, (y * 11 + x / 2) as u8, ((x ^ y) * 7) as u8,
    ])).collect()
}

pub fn write_png(path: &Path, width: u32, height: u32) -> Vec<u8> {
    let samples = pixels(width, height);
    let mut encoder = png::Encoder::new(fs::File::create(path).unwrap(), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&samples).unwrap();
    samples
}

pub fn check_png(path: &Path, width: u32, height: u32, samples: &[u8]) {
    let mut reader = png::Decoder::new(std::io::BufReader::new(fs::File::open(path).unwrap())).read_info().unwrap();
    let mut data = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut data).unwrap();
    assert_eq!((info.width, info.height), (width, height));
    assert_eq!(info.color_type, png::ColorType::Rgb);
    assert_eq!(&data[..info.buffer_size()], samples);
}

pub fn png_workflow(binary: &Path) {
    let scratch = Scratch::new("png-workflow");
    let input = scratch.0.join("generated.png");
    let output = scratch.0.join("out");
    fs::create_dir(&output).unwrap();
    let samples = write_png(&input, 256, 192);
    let legacy=std::env::var_os("RIMAGE_ORACLE_BIN").is_some()||std::env::var_os("RIMAGE_BENCH_LEGACY").is_some();
    let mut command=Command::new(binary);
    let target=if legacy {command.arg("png").arg(&input).arg("-d").arg(&output);output.join("generated.png")}
        else {let target=output.join("generated.png");command.arg("convert").arg(&input).args(["--lossless","-o"]).arg(&target);target};
    let result=command.arg("--quiet").output().unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    check_png(&target, 256, 192, &samples);
}
