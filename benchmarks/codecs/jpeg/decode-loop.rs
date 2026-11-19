use std::{env, error::Error, ffi::c_void, fs, hint::black_box, time::Instant};

unsafe extern "C" {
    fn tj3Init(init_type: i32) -> *mut c_void;
    fn tj3DecompressHeader(handle: *mut c_void, jpeg: *const u8, size: usize) -> i32;
    fn tj3Get(handle: *mut c_void, param: i32) -> i32;
    fn tj3Decompress8(handle: *mut c_void, jpeg: *const u8, size: usize, dst: *mut u8, pitch: i32, format: i32) -> i32;
    fn tj3Destroy(handle: *mut c_void);
    fn img_jpegli_decode(data: *const u8, size: usize, out: *mut *mut u8) -> isize;
    fn free(ptr: *mut c_void);
}
const TJINIT_DECOMPRESS: i32 = 1;
const TJPARAM_JPEGWIDTH: i32 = 5;
const TJPARAM_JPEGHEIGHT: i32 = 6;
const TJPARAM_COLORSPACE: i32 = 4;
const TJCS_GRAY: i32 = 2;
const TJPF_RGB: i32 = 0;
const TJPF_GRAY: i32 = 6;

fn turbo(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    unsafe {
        let handle = tj3Init(TJINIT_DECOMPRESS);
        if handle.is_null() { return Err("tj3Init".into()); }
        if tj3DecompressHeader(handle, data.as_ptr(), data.len()) != 0 { tj3Destroy(handle); return Err("header".into()); }
        let (width, height) = (tj3Get(handle, TJPARAM_JPEGWIDTH) as usize, tj3Get(handle, TJPARAM_JPEGHEIGHT) as usize);
        let (format, channels) = if tj3Get(handle, TJPARAM_COLORSPACE) == TJCS_GRAY { (TJPF_GRAY, 1) } else { (TJPF_RGB, 3) };
        let mut out = vec![0u8; width * height * channels];
        let status = tj3Decompress8(handle, data.as_ptr(), data.len(), out.as_mut_ptr(), (width * channels) as i32, format);
        tj3Destroy(handle);
        if status != 0 { return Err("decompress".into()); }
        Ok(out)
    }
}

fn jpegli(data: &[u8]) -> Result<usize, Box<dyn Error>> {
    let mut out = std::ptr::null_mut();
    let len = unsafe { img_jpegli_decode(data.as_ptr(), data.len(), &mut out) };
    black_box(out);
    unsafe { free(out.cast()) };
    if len < 0 { return Err("jpegli".into()); }
    Ok(len as usize)
}

fn zune(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    use zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};
    let mut probe = zune_jpeg::JpegDecoder::new(ZCursor::new(data));
    probe.decode_headers()?;
    let gray = probe.info().ok_or("info")?.components == 1;
    let options = DecoderOptions::default().jpeg_set_out_colorspace(if gray { ColorSpace::Luma } else { ColorSpace::RGB });
    Ok(zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(data), options).decode()?)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    let decoder = args[1].as_str();
    let iterations: usize = args[2].parse()?;
    for path in &args[3..] {
        let data = fs::read(path)?;
        let run = |data: &[u8]| -> Result<usize, Box<dyn Error>> {
            Ok(match decoder {
                "zune" => black_box(zune(data)?).len(),
                "turbo" => black_box(turbo(data)?).len(),
                "jpegli" => jpegli(data)?,
                "zen" => { let image = zenjpeg::decoder::Decoder::new().decode(data, enough::Unstoppable)?; black_box(&image); 0 },
                _ => return Err("unknown decoder".into()),
            })
        };
        run(&data)?;
        let mut times = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            run(&data)?;
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        println!("{{\"decoder\":\"{decoder}\",\"input\":\"{path}\",\"median_ms\":{:.4},\"min_ms\":{:.4}}}", times[iterations / 2], times[0]);
    }
    Ok(())
}
