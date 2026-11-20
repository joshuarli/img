use std::ffi::{c_char, c_int, c_void, CStr};

unsafe extern "C" {
    fn img_turbo_decode(data: *const u8, size: usize, gray: c_int, out: *mut u8, width: usize, height: usize, max_memory: usize, cancelled: extern "C" fn(*mut c_void) -> c_int, context: *mut c_void, message: *mut c_char, message_len: usize) -> c_int;
}

/// Why a decode returned no pixels.
#[derive(Debug)]
pub enum Failure {
    /// libjpeg refused the stream, or the frame differs from its markers.
    Invalid(String),
    /// The caller's stop predicate returned true during decoding.
    Cancelled,
}

extern "C" fn poll<F: Fn() -> bool>(context: *mut c_void) -> c_int {
    // SAFETY: `context` is the `&F` that `decode` passed and keeps alive for the whole call.
    c_int::from(unsafe { &*(context as *const F) }())
}

/// Decode an eight-bit JPEG into packed RGB8 or Gray8 rows; warnings are errors.
///
/// `cancelled` is polled once per MCU row while scans are absorbed and once per output batch.
pub fn decode<F: Fn() -> bool>(data: &[u8], width: u32, height: u32, gray: bool, max_memory: usize, cancelled: &F) -> Result<Vec<u8>, Failure> {
    let len = (width as usize).checked_mul(height as usize).and_then(|n| n.checked_mul(if gray { 1 } else { 3 })).ok_or_else(|| Failure::Invalid("JPEG dimensions overflow".into()))?;
    let mut out = Vec::with_capacity(len);
    let mut message = [0 as c_char; 200];
    let status = unsafe { img_turbo_decode(data.as_ptr(), data.len(), gray as c_int, out.as_mut_ptr(), width as usize, height as usize, max_memory, poll::<F>, cancelled as *const F as *mut c_void, message.as_mut_ptr(), message.len()) };
    match status {
        0 => Err(Failure::Invalid(unsafe { CStr::from_ptr(message.as_ptr()) }.to_string_lossy().into_owned())),
        2 => Err(Failure::Cancelled),
        _ => { unsafe { out.set_len(len) }; Ok(out) }
    }
}
