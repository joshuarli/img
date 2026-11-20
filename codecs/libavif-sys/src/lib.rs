use std::ffi::{c_char, c_int, CStr};

#[repr(C)]
struct Data { data: *mut u8, size: usize }
unsafe extern "C" {
    fn img_avif_encode(width: u32, height: u32, pixels: *const u8, speed: c_int, qindex: c_int, exif: *const u8, exif_size: usize, output: *mut Data) -> c_int;
    fn img_avif_qindex(quality: f32) -> c_int;
    fn avifRWDataFree(data: *mut Data);
    fn avifResultToString(result: c_int) -> *const c_char;
}

/// The libaom base qindex for a quality in [0, 100]; 0 is lossless.
pub fn qindex(quality: f32) -> u8 { unsafe { img_avif_qindex(quality) as u8 } }

/// Encodes at a libaom base qindex from [`qindex`].
pub fn encode(width: u32, height: u32, pixels: &[u8], speed: u8, qindex: u8, exif: Option<&[u8]>) -> Result<Vec<u8>, String> {
    if (width as usize).checked_mul(height as usize).and_then(|n| n.checked_mul(3)) != Some(pixels.len()) || width > u32::MAX / 3 {
        return Err("invalid RGB dimensions".into());
    }
    let mut output = Data { data: std::ptr::null_mut(), size: 0 };
    let result = unsafe { img_avif_encode(width, height, pixels.as_ptr(), speed.into(), qindex.into(), exif.map_or(std::ptr::null(), |v| v.as_ptr()), exif.map_or(0, |v| v.len()), &mut output) };
    if result != 0 {
        unsafe { avifRWDataFree(&mut output) };
        return Err(unsafe { CStr::from_ptr(avifResultToString(result)) }.to_string_lossy().into_owned());
    }
    if output.data.is_null() {
        unsafe { avifRWDataFree(&mut output) };
        return Err("libavif returned empty output".into());
    }
    let bytes = unsafe { std::slice::from_raw_parts(output.data, output.size) }.to_vec();
    unsafe { avifRWDataFree(&mut output) };
    Ok(bytes)
}
