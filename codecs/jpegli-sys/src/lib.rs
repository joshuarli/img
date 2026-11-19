use std::{ffi::{c_float, c_int, c_void}, ptr::NonNull};

unsafe extern "C" {
    fn img_jpegli_create(width: u32, height: u32, gray: c_int, quality: c_float, memory: usize, limit: usize) -> *mut c_void;
    fn img_jpegli_marker(encoder: *mut c_void, marker: c_int, data: *const u8, len: usize) -> c_int;
    fn img_jpegli_rows(encoder: *mut c_void, data: *const u8, stride: usize, count: u32) -> c_int;
    fn img_jpegli_finish(encoder: *mut c_void, data: *mut *const u8, len: *mut usize) -> c_int;
    fn img_jpegli_destroy(encoder: *mut c_void);
}
#[derive(Clone, Copy)]
pub enum Color { Gray, Rgb }
pub struct Encoder { raw: NonNull<c_void>, stride: usize, remaining: u32 }
impl Encoder {
    pub fn new(width: u32, height: u32, color: Color, quality: f32, memory: usize, limit: usize) -> Result<Self, &'static str> {
        if !(1.0..=100.0).contains(&quality) { return Err("invalid quality"); }
        let gray = matches!(color, Color::Gray);
        let stride = (width as usize).checked_mul(if gray { 1 } else { 3 }).ok_or("invalid dimensions")?;
        let raw = unsafe { img_jpegli_create(width, height, gray as c_int, quality, memory, limit) };
        Ok(Self { raw: NonNull::new(raw).ok_or("jpegli initialization failed")?, stride, remaining: height })
    }
    pub fn icc_profile(&mut self, data: &[u8]) -> Result<(), &'static str> { self.marker(2, data) }
    pub fn app1(&mut self, data: &[u8]) -> Result<(), &'static str> { self.marker(1, data) }
    fn marker(&mut self, marker: u8, data: &[u8]) -> Result<(), &'static str> {
        if !matches!(marker, 1 | 2) || data.len() > u32::MAX as usize { return Err("invalid marker"); }
        if unsafe { img_jpegli_marker(self.raw.as_ptr(), marker.into(), data.as_ptr(), data.len()) } == 0 { return Err("jpegli marker failed"); }
        Ok(())
    }
    pub fn rows(&mut self, data: &[u8], count: u32) -> Result<(), &'static str> {
        if count > self.remaining || self.stride.checked_mul(count as usize) != Some(data.len()) { return Err("invalid scanlines"); }
        if unsafe { img_jpegli_rows(self.raw.as_ptr(), data.as_ptr(), self.stride, count) } == 0 { return Err("jpegli scanlines failed"); }
        self.remaining -= count; Ok(())
    }
    pub fn finish(self) -> Result<Vec<u8>, &'static str> {
        if self.remaining != 0 { return Err("incomplete image"); }
        let mut data = std::ptr::null(); let mut len = 0;
        if unsafe { img_jpegli_finish(self.raw.as_ptr(), &mut data, &mut len) } == 0 { return Err("jpegli finish failed"); }
        Ok(unsafe { std::slice::from_raw_parts(data, len) }.to_vec())
    }
}
impl Drop for Encoder { fn drop(&mut self) { unsafe { img_jpegli_destroy(self.raw.as_ptr()) } } }
