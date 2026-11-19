use std::{env, error::Error, fs, hint::black_box, time::Instant};
#[repr(C)]
#[derive(Default)]
struct Planes { w: u32, h: u32, chroma: u32, matrix: u32, full: u32, pixels: *mut u8 }
unsafe extern "C" {
    fn decode_raw(data: *const u8, len: usize, out: *mut Planes) -> i32;
    fn decode_avif(data: *const u8, len: usize, out: *mut Planes) -> i32;
    fn free(data: *mut std::ffi::c_void);
}
impl Drop for Planes { fn drop(&mut self) { unsafe { free(self.pixels.cast()) } } }
enum Output { Planes(Planes), Rgb(Vec<u8>) }
impl Output { fn bytes(&self) -> &[u8] { match self { Self::Planes(p) => { let w=p.w as usize; let h=p.h as usize; let cw=if p.chroma == 3 {w} else {w.div_ceil(2)}; let ch=if p.chroma == 1 {h.div_ceil(2)} else {h}; unsafe { std::slice::from_raw_parts(p.pixels, w*h+2*cw*ch) } }, Self::Rgb(v) => v } } }
fn run(mode: &str, data: &[u8]) -> Result<Output, Box<dyn Error>> {
    let mut p = Planes::default();
    let rc = unsafe { if mode == "raw" { decode_raw(data.as_ptr(), data.len(), &mut p) } else { decode_avif(data.as_ptr(), data.len(), &mut p) } };
    if rc != 0 { return Err("dav1d decode failed".into()); }
    let (w,h) = (p.w as usize, p.h as usize);
    let cw = if p.chroma == 3 { w } else { w.div_ceil(2) };
    let ch = if p.chroma == 1 { h.div_ceil(2) } else { h };
    let planes = unsafe { std::slice::from_raw_parts(p.pixels, w*h+2*cw*ch) };
    if mode == "raw" { return Ok(Output::Planes(p)); }
    let (y, uv) = planes.split_at(w*h); let (u,v) = uv.split_at(cw*ch);
    let mut rgb = vec![0; w*h*3];
    if p.matrix == 0 {
        if p.chroma != 3 || p.full == 0 { return Err("unsupported identity matrix".into()); }
        for i in 0..w*h { rgb[3*i] = v[i]; rgb[3*i+1] = y[i]; rgb[3*i+2] = u[i]; }
    } else {
        let matrix = match p.matrix { 1 => yuv::YuvStandardMatrix::Bt709, 6 => yuv::YuvStandardMatrix::Bt601, _ => return Err("unsupported matrix".into()) };
        let range = if p.full != 0 { yuv::YuvRange::Full } else { yuv::YuvRange::Limited };
        let image = yuv::YuvPlanarImage { y_plane:y, y_stride:p.w, u_plane:u, u_stride:cw as u32, v_plane:v, v_stride:cw as u32, width:p.w, height:p.h };
        match p.chroma { 1 => yuv::yuv420_to_rgb(&image,&mut rgb,p.w*3,range,matrix), 2 => yuv::yuv422_to_rgb(&image,&mut rgb,p.w*3,range,matrix), 3 => yuv::yuv444_to_rgb(&image,&mut rgb,p.w*3,range,matrix), _ => return Err("unsupported chroma".into()) }?;
    }
    Ok(Output::Rgb(rgb))
}
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<_> = env::args().collect(); let ms: f64 = a[2].parse()?; let data = fs::read(&a[3])?;
    if a.len() > 4 { fs::write(&a[4],run(&a[1], &data)?.bytes())?; return Ok(()); }
    if ms > 0.0 { run(&a[1],&data)?; } let start = Instant::now(); let mut n = 0;
    while n == 0 || start.elapsed().as_secs_f64()*1000.0 < ms { black_box(run(&a[1],&data)?); n += 1; }
    println!("{{\"iterations\":{n},\"mean_ms\":{}}}",start.elapsed().as_secs_f64()*1000.0/n as f64); Ok(())
}
