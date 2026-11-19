use std::{env, error::Error, ffi::c_void, fs, hint::black_box, io::{Cursor, Write}, time::Instant};

unsafe extern "C" {
    fn native_png(data: *const u8, len: usize, out: *mut *mut u8, out_len: *mut usize) -> i32;
    fn native_spng(data: *const u8, len: usize, out: *mut *mut u8, out_len: *mut usize) -> i32;
    fn free(ptr: *mut c_void);
}
fn chunks(data: &[u8]) -> Result<(), Box<dyn Error>> {
    if data.get(..8) != Some(b"\x89PNG\r\n\x1a\n") { return Err("signature".into()); }
    let mut at = 8usize;
    while at < data.len() {
        let head = data.get(at..at+8).ok_or("chunk header")?;
        let len = u32::from_be_bytes(head[..4].try_into()?) as usize;
        let end = at.checked_add(len).and_then(|v| v.checked_add(12)).filter(|&v| v <= data.len()).ok_or("chunk length")?;
        if crc32fast::hash(&data[at+4..end-4]) != u32::from_be_bytes(data[end-4..end].try_into()?) { return Err("crc".into()); }
        if &head[4..] == b"IEND" { return if end == data.len() { Ok(()) } else { Err("trailing".into()) }; }
        at = end;
    }
    Err("missing IEND".into())
}
enum Output { Rust(Vec<u8>), Native(*mut u8, usize) }
impl Output {
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Rust(v) => v,
            // SAFETY: Native buffers contain the initialized length returned by the decoder and remain owned here.
            Self::Native(ptr, len) => unsafe { std::slice::from_raw_parts(*ptr, *len) },
        }
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        if let Self::Native(ptr, _) = self {
            // SAFETY: Native buffers are malloc-owned and freed exactly once.
            unsafe { free((*ptr).cast()) };
        }
    }
}
fn decode(mode: &str, data: &[u8]) -> Result<Output, Box<dyn Error>> {
    chunks(data)?;
    match mode {
        "png" => {
            let mut decoder = png::Decoder::new(Cursor::new(data));
            decoder.set_transformations(png::Transformations::EXPAND);
            let mut reader = decoder.read_info()?;
            let mut out = vec![0; reader.output_buffer_size().ok_or("size")?];
            let info = reader.next_frame(&mut out)?;
            out.truncate(info.buffer_size());
            if cfg!(target_endian = "little") && info.bit_depth == png::BitDepth::Sixteen { for p in out.chunks_exact_mut(2) { p.swap(0, 1); } }
            Ok(Output::Rust(out))
        }
        "zen" => {
            let result = zenpng::decode(data, &zenpng::PngDecodeConfig::strict(), &enough::Unstoppable)?;
            let mut out = result.pixels.copy_to_contiguous_bytes();
            if data[25] == 0 && out.len() == u32::from_be_bytes(data[16..20].try_into()?) as usize * u32::from_be_bytes(data[20..24].try_into()?) as usize * if data[24] == 16 {8} else {4} {
                let unit = if data[24] == 16 {2} else {1};
                out = out.chunks_exact(4*unit).flat_map(|p| p[..unit].iter().chain(&p[3*unit..4*unit]).copied()).collect();
            }
            Ok(Output::Rust(out))
        }
        "libpng" | "spng" => {
            let mut ptr = std::ptr::null_mut(); let mut len = 0;
            // SAFETY: Both native decoders read the valid input slice and return a malloc-owned buffer on success.
            let status = unsafe { if mode == "spng" { native_spng(data.as_ptr(), data.len(), &mut ptr, &mut len) } else { native_png(data.as_ptr(), data.len(), &mut ptr, &mut len) } };
            if status != 0 { return Err("native decode failed".into()); }
            Ok(Output::Native(ptr, len))
        }
        _ => Err("unknown decoder".into()),
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    let mode = &args[1];
    if let Some(name) = mode.strip_suffix("-dump") {
        let data = fs::read(&args[2])?;
        std::io::stdout().write_all(decode(name, &data)?.bytes())?;
        return Ok(());
    }
    let min_ms: f64 = args[2].parse()?;
    let data = fs::read(&args[3])?;
    if min_ms > 0.0 { black_box(decode(mode, &data)?); }
    let start = Instant::now(); let mut iterations = 0u32;
    while start.elapsed().as_secs_f64()*1000.0 < min_ms || iterations == 0 {
        black_box(decode(mode, &data)?); iterations += 1;
    }
    println!("{{\"iterations\":{iterations},\"mean_ms\":{:.8}}}", start.elapsed().as_secs_f64()*1000.0 / f64::from(iterations));
    Ok(())
}
