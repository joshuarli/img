use rav1d::include::dav1d::dav1d::{Dav1dContext, Dav1dSettings};
use rav1d::include::dav1d::data::Dav1dData;
use rav1d::include::dav1d::headers::*;
use rav1d::include::dav1d::picture::Dav1dPicture;
use rav1d::src::lib::{dav1d_close, dav1d_data_create, dav1d_data_unref, dav1d_default_settings, dav1d_get_picture, dav1d_open, dav1d_picture_unref, dav1d_send_data};
use std::hint::black_box;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::{env, error::Error, fs, time::Instant};


struct PictureGuard(Dav1dPicture);
impl Drop for PictureGuard {
    fn drop(&mut self) { unsafe { dav1d_picture_unref(Some(NonNull::from(&mut self.0))) }; }
}
struct ContextGuard(Option<Dav1dContext>);
impl Drop for ContextGuard {
    fn drop(&mut self) { if self.0.is_some() { unsafe { dav1d_close(Some(NonNull::from(&mut self.0))) }; } }
}

fn rav1d_decode(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut settings = MaybeUninit::<Dav1dSettings>::uninit();
    unsafe { dav1d_default_settings(NonNull::from(&mut settings).cast::<Dav1dSettings>()) };
    let settings = unsafe { settings.assume_init_mut() };
    settings.n_threads = 1;
    settings.max_frame_delay = 1;
    settings.apply_grain = 1;
    settings.frame_size_limit = 16 * 1024 * 1024;
    let mut context = None;
    let open = unsafe { dav1d_open(Some(NonNull::from(&mut context)), Some(NonNull::from(settings))) };
    if open.0 != 0 { return Err(format!("dav1d_open {}", open.0).into()); }
    let _guard = ContextGuard(context);
    let ctx = context.ok_or("no context")?;

    let mut data_slot = Dav1dData::default();
    let buffer = unsafe { dav1d_data_create(Some(NonNull::from(&mut data_slot)), data.len()) };
    let buffer = NonNull::new(buffer).ok_or("no buffer")?;
    unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), buffer.as_ptr(), data.len()) };
    let mut displayed = Vec::new();
    loop {
        let result = unsafe { dav1d_send_data(Some(ctx), Some(NonNull::from(&mut data_slot))) };
        if result.0 == 0 { break; }
        if result.0 != -(libc::EAGAIN as std::ffi::c_int) { unsafe { dav1d_data_unref(Some(NonNull::from(&mut data_slot))) }; return Err(format!("send {}", result.0).into()); }
        let mut pending = Dav1dPicture::default();
        let drain = unsafe { dav1d_get_picture(Some(ctx), Some(NonNull::from(&mut pending))) };
        if drain.0 != 0 { unsafe { dav1d_picture_unref(Some(NonNull::from(&mut pending))) }; return Err(format!("drain {}", drain.0).into()); }
        displayed.push(PictureGuard(pending));
    }
    unsafe { dav1d_data_unref(Some(NonNull::from(&mut data_slot))) };
    loop {
        let mut picture = Dav1dPicture::default();
        let result = unsafe { dav1d_get_picture(Some(ctx), Some(NonNull::from(&mut picture))) };
        if result.0 != 0 { unsafe { dav1d_picture_unref(Some(NonNull::from(&mut picture))) }; break; }
        displayed.push(PictureGuard(picture));
    }
    if displayed.len() != 1 { return Err("expected one displayed frame".into()); }
    let picture = &displayed[0].0;
    if picture.p.bpc != 8 { return Err("requires eight-bit".into()); }
    let (chroma, chroma_width) = match picture.p.layout {
        DAV1D_PIXEL_LAYOUT_I420 => (0u8, (picture.p.w as usize).div_ceil(2)),
        DAV1D_PIXEL_LAYOUT_I422 => (1u8, (picture.p.w as usize).div_ceil(2)),
        DAV1D_PIXEL_LAYOUT_I444 => (2u8, picture.p.w as usize),
        _ => return Err("unsupported layout".into()),
    };
    let (width, height) = (picture.p.w as usize, picture.p.h as usize);
    let chroma_height = if chroma == 0 { height.div_ceil(2) } else { height };
    let y_stride = picture.stride[0] as usize;
    let c_stride = picture.stride[1] as usize;
    let mut planes = Vec::with_capacity(width * height + 2 * chroma_width * chroma_height);
    let mut plane = |index: usize, stride: usize, rows: usize, row_bytes: usize| -> Result<(), Box<dyn Error>> {
        let base = picture.data[index].ok_or("missing plane")?;
        for row in 0..rows {
            let ptr = unsafe { base.as_ptr().cast::<u8>().add(row * stride) };
            planes.extend_from_slice(unsafe { std::slice::from_raw_parts(ptr, row_bytes) });
        }
        Ok(())
    };
    plane(0, y_stride, height, width)?;
    plane(1, c_stride, chroma_height, chroma_width)?;
    plane(2, c_stride, chroma_height, chroma_width)?;
    Ok(planes)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    let mode = &args[1];
    let ms: f64 = args[2].parse()?;
    let data = fs::read(&args[3])?;
    enum Output { Planes(Vec<u8>), Image(img::still::StillImage) }
    let run = || -> Result<Output, Box<dyn Error>> {
        match mode.as_str() {
            "raw" => Ok(Output::Planes(rav1d_decode(&data)?)),
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
