use std::{alloc::{GlobalAlloc, Layout, System}, fs, sync::atomic::{AtomicUsize, Ordering::Relaxed}};
use img::{perceptual::PerceptualReference, still::{Effort, EncodingMode, Format, Quality, StillImage}};

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

fn phase(name: &str, pixels: f64) {
    let (live, peak) = (LIVE.load(Relaxed), PEAK.swap(LIVE.load(Relaxed), Relaxed));
    println!("{name:28} live {:7.1} B/px  phase peak {:7.1} B/px", live as f64 / pixels, peak as f64 / pixels);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input = fs::read(&args[1]).unwrap();
    let format = match args.get(2).map(String::as_str).unwrap_or("jpeg") { "avif" => Format::Avif, "png" => Format::Png, _ => Format::Jpeg };
    let image = StillImage::decode(&input).unwrap();
    let (w, h) = image.dimensions();
    let pixels = f64::from(w) * f64::from(h);
    phase("decode source", pixels);
    let reference = PerceptualReference::new(&image).unwrap();
    phase("perceptual reference", pixels);
    let encoded = image.encode(format, EncodingMode::Quality(Quality::new(80.0).unwrap()), Effort::Balanced).unwrap();
    phase("encode candidate", pixels);
    let score = reference.score_encoded(&encoded).unwrap();
    phase("decode+score candidate", pixels);
    println!("score {score:.3} bytes {}", encoded.len());
}
