use std::{fs, sync::{Arc, atomic::{AtomicBool, Ordering::Relaxed}}};
use img::still::StillImage;
use jpegli_sys::{Color, Encoder, Scans};

fn footprint() -> u64 {
    let mut info: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
    // SAFETY: proc_pid_rusage fills the provided struct for this process.
    unsafe { libc::proc_pid_rusage(std::process::id() as i32, libc::RUSAGE_INFO_V4, (&mut info as *mut libc::rusage_info_v4).cast()) };
    info.ri_phys_footprint
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let image = StillImage::decode(&fs::read(&args[1]).unwrap()).unwrap();
    let (color, scans, quality) = (match args[2].as_str() { "420" => Color::Rgb420, _ => Color::Rgb }, match args[3].as_str() { "seq" => Scans::Sequential, _ => Scans::Progressive }, args[4].parse().unwrap());
    let samples = image.samples();
    let (w, h) = image.dimensions();
    drop(image);
    let base = footprint();
    let done = Arc::new(AtomicBool::new(false));
    let sampler = { let done = done.clone(); std::thread::spawn(move || { let mut peak = 0; while !done.load(Relaxed) { peak = peak.max(footprint()); std::thread::sleep(std::time::Duration::from_millis(1)); } peak }) };
    let mut encoder = Encoder::new(w, h, color, scans, quality, usize::MAX >> 1, 1 << 30).unwrap();
    encoder.rows(&samples, h).unwrap();
    let out = encoder.finish().unwrap();
    done.store(true, Relaxed);
    let peak = sampler.join().unwrap();
    let px = f64::from(w) * f64::from(h);
    println!("{} {} q{quality}: encoder adds {:.1} B/px above input (output {:.2} B/px)", args[2], args[3], (peak - base) as f64 / px, out.len() as f64 / px);
}
