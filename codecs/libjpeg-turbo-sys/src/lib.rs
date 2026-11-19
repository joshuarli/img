use std::ffi::{c_char, c_int, CStr};

unsafe extern "C" {
    fn img_turbo_decode(data: *const u8, size: usize, gray: c_int, out: *mut u8, width: usize, height: usize, max_memory: usize, message: *mut c_char, message_len: usize) -> c_int;
}

/// Decode an eight-bit JPEG into packed RGB8 or Gray8 rows; warnings are errors.
pub fn decode(data: &[u8], width: u32, height: u32, gray: bool, max_memory: usize) -> Result<Vec<u8>, String> {
    let len = (width as usize).checked_mul(height as usize).and_then(|n| n.checked_mul(if gray { 1 } else { 3 })).ok_or("JPEG dimensions overflow")?;
    let mut out = Vec::with_capacity(len);
    let mut message = [0 as c_char; 200];
    let ok = unsafe { img_turbo_decode(data.as_ptr(), data.len(), gray as c_int, out.as_mut_ptr(), width as usize, height as usize, max_memory, message.as_mut_ptr(), message.len()) };
    if ok == 0 { return Err(unsafe { CStr::from_ptr(message.as_ptr()) }.to_string_lossy().into_owned()); }
    unsafe { out.set_len(len) };
    Ok(out)
}
