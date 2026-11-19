use std::{env, error::Error, fs, hint::black_box, time::Instant};

#[cfg(feature = "direct")]
mod direct {
    use std::{error::Error, ffi::c_void, hint::black_box};
    unsafe extern "C" {
        fn img_jpegli_decode(data: *const u8, size: usize, out: *mut *mut u8) -> isize;
        fn free(ptr: *mut c_void);
    }

    fn header(data: &[u8]) -> Result<(u32, u32, bool), Box<dyn Error>> {
        let mut probe = zune_jpeg::JpegDecoder::new(zune_core::bytestream::ZCursor::new(data));
        probe.decode_headers()?;
        let info = probe.info().ok_or("info")?;
        Ok((info.width.into(), info.height.into(), info.components == 1))
    }

    pub fn turbo(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        let (width, height, gray) = header(data)?;
        Ok(libjpeg_turbo_sys::decode(data, width, height, gray, usize::MAX >> 1)?)
    }

    pub fn jpegli(data: &[u8]) -> Result<usize, Box<dyn Error>> {
        let mut out = std::ptr::null_mut();
        let len = unsafe { img_jpegli_decode(data.as_ptr(), data.len(), &mut out) };
        black_box(out);
        unsafe { free(out.cast()) };
        if len < 0 { return Err("jpegli".into()); }
        Ok(len as usize)
    }

    pub fn zune(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
        use zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};
        let (_, _, gray) = header(data)?;
        let options = DecoderOptions::default().jpeg_set_out_colorspace(if gray { ColorSpace::Luma } else { ColorSpace::RGB });
        Ok(zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(data), options).decode()?)
    }

    pub fn zen(data: &[u8]) -> Result<usize, Box<dyn Error>> {
        let image = zenjpeg::decoder::Decoder::new().decode(data, enough::Unstoppable)?;
        black_box(&image);
        Ok(0)
    }
}

fn img(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> { Ok(img::still::StillImage::decode(data)?.samples()) }

fn run(decoder: &str, data: &[u8]) -> Result<usize, Box<dyn Error>> {
    Ok(match decoder {
        "img" => black_box(img(data)?).len(),
        #[cfg(feature = "direct")]
        "zune" => black_box(direct::zune(data)?).len(),
        #[cfg(feature = "direct")]
        "turbo" => black_box(direct::turbo(data)?).len(),
        #[cfg(feature = "direct")]
        "jpegli" => direct::jpegli(data)?,
        #[cfg(feature = "direct")]
        "zen" => direct::zen(data)?,
        _ => return Err(format!("unknown decoder {decoder}").into()),
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    let decoder = args[1].as_str();
    if let Some(name) = decoder.strip_suffix("-dump") {
        use std::io::Write;
        let data = fs::read(&args[2])?;
        let bytes = match name {
            "img" => img(&data),
            #[cfg(feature = "direct")]
            "zune" => direct::zune(&data),
            #[cfg(feature = "direct")]
            "turbo" => direct::turbo(&data),
            _ => Err("unknown decoder".into()),
        };
        match bytes {
            Ok(bytes) => std::io::stdout().write_all(&bytes)?,
            Err(err) => { eprintln!("{err}"); std::process::exit(1); }
        }
        return Ok(());
    }
    let min_ms: f64 = args[2].parse()?;
    for path in &args[3..] {
        let data = fs::read(path)?;
        run(decoder, &data)?;
        let start = Instant::now();
        let mut iterations = 0u32;
        while start.elapsed().as_secs_f64() * 1000.0 < min_ms || iterations == 0 {
            run(decoder, &data)?;
            iterations += 1;
        }
        let total = start.elapsed().as_secs_f64() * 1000.0;
        println!("{{\"decoder\":\"{decoder}\",\"input\":\"{path}\",\"iterations\":{iterations},\"mean_ms\":{:.5}}}", total / f64::from(iterations));
    }
    Ok(())
}
