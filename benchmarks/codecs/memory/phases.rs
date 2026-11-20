use std::{alloc::{GlobalAlloc, Layout, System}, fs, sync::atomic::{AtomicUsize, Ordering::Relaxed}};
use img::{optimizer::{process, Fidelity, FormatSelection, Operation, Request}, perceptual::{PerceptualReference, ScoreTarget}, still::{Effort, EncodingMode, Format, Quality, StillImage}};

struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() { PEAK.fetch_max(LIVE.fetch_add(layout.size(), Relaxed) + layout.size(), Relaxed); }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) { unsafe { System.dealloc(p, layout) }; LIVE.fetch_sub(layout.size(), Relaxed); }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, layout, size) };
        if !q.is_null() {
            if size > layout.size() { PEAK.fetch_max(LIVE.fetch_add(size - layout.size(), Relaxed) + size - layout.size(), Relaxed); }
            else { LIVE.fetch_sub(layout.size() - size, Relaxed); }
        }
        q
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn footprint() -> (u64, u64) {
    let mut info: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
    // SAFETY: proc_pid_rusage fills the provided struct for this process.
    unsafe { libc::proc_pid_rusage(std::process::id() as i32, libc::RUSAGE_INFO_V4, (&mut info as *mut libc::rusage_info_v4).cast()) };
    (info.ri_phys_footprint, info.ri_lifetime_max_phys_footprint)
}

fn phase(name: &str, pixels: f64) {
    let (live, peak) = (LIVE.load(Relaxed), PEAK.swap(LIVE.load(Relaxed), Relaxed));
    let (now, max) = footprint();
    println!("{name:28} live {:7.1} B/px  phase peak {:7.1} B/px  footprint now {:7.1} max {:7.1} B/px", live as f64 / pixels, peak as f64 / pixels, now as f64 / pixels, max as f64 / pixels);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input = fs::read(&args[1]).unwrap();
    let format = match args.get(2).map(String::as_str).unwrap_or("jpeg") { "avif" => Format::Avif, "png" => Format::Png, _ => Format::Jpeg };
    let image = StillImage::decode(&input).unwrap();
    let (w, h) = image.dimensions();
    let pixels = f64::from(w) * f64::from(h);
    if args.get(3).map(String::as_str) == Some("process") {
        drop(image);
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let sampler = { let done = done.clone(); std::thread::spawn(move || {
            let (mut best, mut at_heap) = (0u64, 0usize);
            let mut timeline = Vec::new();
            while !done.load(Relaxed) {
                let mut info: libc::rusage_info_v4 = unsafe { std::mem::zeroed() };
                // SAFETY: proc_pid_rusage fills the provided struct for this process.
                unsafe { libc::proc_pid_rusage(std::process::id() as i32, libc::RUSAGE_INFO_V4, (&mut info as *mut libc::rusage_info_v4).cast()) };
                if info.ri_phys_footprint > best { best = info.ri_phys_footprint; at_heap = LIVE.load(Relaxed); }
                let mut stats: libc::malloc_statistics_t = unsafe { std::mem::zeroed() };
                // SAFETY: a null zone asks for statistics over all zones.
                unsafe { libc::malloc_zone_statistics(std::ptr::null_mut(), &mut stats) };
                timeline.push((info.ri_phys_footprint, LIVE.load(Relaxed), stats.size_in_use, stats.size_allocated));
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            if std::env::var_os("TIMELINE").is_some() { for (i, (f, l, u, a)) in timeline.iter().enumerate().step_by(10) { eprintln!("{:6} footprint {:7.1} MB rust {:7.1} MB malloc in use {:7.1} MB reserved {:7.1} MB", i * 2, *f as f64 / 1e6, *l as f64 / 1e6, *u as f64 / 1e6, *a as f64 / 1e6); } }
            (best, at_heap)
        }) };
        let request = Request { operation: Operation::Optimize(FormatSelection::Exact(format)), fidelity: Fidelity::Score(ScoreTarget::new(90.0).unwrap()), effort: Effort::Balanced, ..Request::default() };
        let outcome = process(&input, request).map(|o| o.bytes.len());
        phase("optimizer::process", pixels);
        done.store(true, Relaxed);
        let (footprint, heap) = sampler.join().unwrap();
        println!("peak footprint {:.1} B/px with live heap {:.1} B/px at that moment", footprint as f64 / pixels, heap as f64 / pixels);
        println!("outcome {outcome:?}");
        return;
    }
    phase("decode source", pixels);
    let reference = PerceptualReference::new(&image).unwrap();
    phase("perceptual reference", pixels);
    let encoded = image.encode(format, EncodingMode::Quality(Quality::new(std::env::var("Q").ok().and_then(|q| q.parse().ok()).unwrap_or(80.0)).unwrap()), Effort::Balanced).unwrap();
    phase("encode candidate", pixels);
    let mut score = reference.score_encoded(&encoded).unwrap();
    phase("decode+score candidate", pixels);
    for _ in 1..std::env::var("N").ok().and_then(|n| n.parse().ok()).unwrap_or(1) {
        let encoded = image.encode(format, EncodingMode::Quality(Quality::new(80.0).unwrap()), Effort::Balanced).unwrap();
        score = reference.score_encoded(&encoded).unwrap();
    }
    phase("repeated candidates", pixels);
    println!("score {score:.3} bytes {}", encoded.len());
}
