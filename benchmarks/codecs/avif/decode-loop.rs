use std::hint::black_box;
use std::{env, error::Error, fs, time::Instant};
use dav1d_sys as _;
#[repr(C)]
struct View { data: [*const u8; 3], stride: [isize; 2], width: i32, height: i32, layout: i32, bitdepth: i32, has_hdr: i32, has_sequence: i32, still: i32, primaries: i32, transfer: i32, matrix: i32, full_range: i32 }
unsafe extern "C" {
    fn img_av1_decode(data: *const u8, len: usize, limit: u32, picture: *mut *mut std::ffi::c_void, view: *mut View) -> i32;
    fn img_av1_picture_free(picture: *mut std::ffi::c_void);
}
fn decoder_planes(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut view = std::mem::MaybeUninit::<View>::zeroed();
    let mut handle = std::ptr::null_mut();
    let status = unsafe { img_av1_decode(data.as_ptr(), data.len(), 16 * 1024 * 1024, &mut handle, view.as_mut_ptr()) };
    if status != 0 { return Err(format!("decode failed: {status}").into()); }
    let view = unsafe { view.assume_init() };
    let w = view.width as usize; let h = view.height as usize;
    if view.bitdepth != 8 || !(1..=3).contains(&view.layout) { unsafe { img_av1_picture_free(handle) }; return Err("unsupported picture".into()); }
    let cw = if view.layout == 3 { w } else { w.div_ceil(2) };
    let ch = if view.layout == 1 { h.div_ceil(2) } else { h };
    let mut out = Vec::new();
    for p in 0..3 { for y in 0..if p == 0 { h } else { ch } {
        let row = unsafe { view.data[p].offset(view.stride[usize::from(p != 0)] * y as isize) };
        out.extend_from_slice(unsafe { std::slice::from_raw_parts(row, if p == 0 { w } else { cw }) });
    }}
    unsafe { img_av1_picture_free(handle) }; Ok(out)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    let mode = &args[1];
    let ms: f64 = args[2].parse()?;
    let data = fs::read(&args[3])?;
    enum Output { Planes(Vec<u8>), Image(img::still::StillImage) }
    let run = || -> Result<Output, Box<dyn Error>> {
        match mode.as_str() {
            "raw" => Ok(Output::Planes(decoder_planes(&data)?)),
            "img" => Ok(Output::Image(img::still::StillImage::decode(&data)?)),
            _ => Err("unknown mode".into()),
        }
    };
    if args.len() > 4 { let bytes = match run()? { Output::Planes(p) => p, Output::Image(i) => i.samples() }; fs::write(&args[4], bytes)?; return Ok(()); }
    if ms > 0.0 { run()?; }
    let start = Instant::now();
    let mut n = 0;
    while n == 0 || start.elapsed().as_secs_f64() * 1000.0 < ms { black_box(run()?); n += 1; }
    println!("{{\"iterations\":{n},\"mean_ms\":{}}}", start.elapsed().as_secs_f64()*1000.0 / n as f64);
    Ok(())
}
