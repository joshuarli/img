//! Safe wrapper over the BSD-2-Clause dav1d AV1 decoder.

use std::ptr::NonNull;
use dav1d_sys as _;

#[repr(C)]
struct View {
    data: [*const u8; 3], stride: [isize; 2], width: i32, height: i32, layout: i32, bitdepth: i32,
    has_hdr: i32, has_sequence: i32, still: i32, primaries: i32, transfer: i32, matrix: i32, full_range: i32,
}
unsafe extern "C" {
    fn img_av1_decode(data: *const u8, len: usize, limit: u32, picture: *mut *mut std::ffi::c_void, view: *mut View) -> i32;
    fn img_av1_picture_free(picture: *mut std::ffi::c_void);
}
const DAV1D_PIXEL_LAYOUT_I400: i32 = 0;
const DAV1D_PIXEL_LAYOUT_I420: i32 = 1;
const DAV1D_PIXEL_LAYOUT_I422: i32 = 2;
const DAV1D_PIXEL_LAYOUT_I444: i32 = 3;
const DAV1D_COLOR_PRI_BT709: i32 = 1;
const DAV1D_TRC_SRGB: i32 = 13;

use super::{Error, ErrorKind};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Chroma {
    I420,
    I422,
    I444,
}

pub(super) struct DecodedStill {
    pub width: u32,
    pub height: u32,
    _picture: PictureGuard,
    data: [NonNull<u8>; 3],
    plane_lengths: [usize; 3],
    pub y_stride: u32,
    pub chroma_stride: u32,
    pub chroma: Chroma,
    pub full_range: bool,
    pub matrix: u8,
}

impl DecodedStill {
    pub fn planes(&self) -> [&[u8]; 3] {
        std::array::from_fn(|index| {
            let data = self.data[index];
            // SAFETY: The shim initializes row padding and dav1d initializes the visible pixels.
            // The validated lengths end within those planes, and self owns the picture for the borrow.
            unsafe { std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), self.plane_lengths[index]) }
        })
    }
}

struct PictureGuard(NonNull<std::ffi::c_void>);
impl Drop for PictureGuard {
    fn drop(&mut self) {
        // SAFETY: The shim returned this uniquely owned picture and it is released exactly once.
        unsafe { img_av1_picture_free(self.0.as_ptr()) };
    }
}

pub(super) fn decode_still(data: &[u8], stop: &crate::cancellation::Cancellation) -> Result<DecodedStill, Error> {
    if stop.is_cancelled() { return Err(Error::new(ErrorKind::Invalid, "operation cancelled")); }
    let mut view = std::mem::MaybeUninit::<View>::zeroed();
    let mut handle = std::ptr::null_mut();
    // SAFETY: Input is readable for its length; output pointers are valid and initialized by the shim on success.
    let result = unsafe { img_av1_decode(data.as_ptr(), data.len(), super::MAX_PIXELS as u32, &mut handle, view.as_mut_ptr()) };
    match result {
        0 => (),
        -2 => return Err(Error::new(ErrorKind::Limit, "could not allocate an AV1 decoder buffer")),
        -3 => return Err(Error::new(ErrorKind::Unsupported, "AVIF requires exactly one displayed frame")),
        _ => return Err(Error::new(ErrorKind::Invalid, "AV1 decoder rejected the image")),
    }
    let picture_guard = PictureGuard(NonNull::new(handle).ok_or_else(|| Error::new(ErrorKind::Invalid, "AV1 decoder returned no picture"))?);
    // SAFETY: The successful shim call initialized every field.
    let picture = unsafe { view.assume_init() };
    if picture.has_hdr != 0 {
        return Err(Error::new(ErrorKind::Unsupported, "AVIF HDR metadata requires independently validated processing"));
    }
    if picture.bitdepth != 8 {
        return Err(Error::new(ErrorKind::Unsupported, "AVIF decoding requires eight-bit precision"));
    }
    let layout = picture.layout;
    if layout == DAV1D_PIXEL_LAYOUT_I400 {
        return Err(Error::new(ErrorKind::Unsupported, "monochrome AVIF decoding is not independently verified"));
    }
    if picture.has_sequence == 0 { return Err(Error::new(ErrorKind::Invalid, "AV1 sequence header missing")); }
    if picture.still == 0 { return Err(Error::new(ErrorKind::Unsupported, "AVIF requires a still AV1 bitstream")); }
    if picture.primaries != DAV1D_COLOR_PRI_BT709 || picture.transfer != DAV1D_TRC_SRGB {
        return Err(Error::new(ErrorKind::Unsupported, "AVIF bitstream requires validated SDR sRGB color signaling"));
    }
    let width = u32::try_from(picture.width).map_err(|_| Error::new(ErrorKind::Limit, "AVIF width overflow"))?;
    let height = u32::try_from(picture.height).map_err(|_| Error::new(ErrorKind::Limit, "AVIF height overflow"))?;
    super::check_dimensions(width, height)?;
    let (chroma, sub_x, sub_y) = match layout {
        DAV1D_PIXEL_LAYOUT_I420 => (Chroma::I420, 1usize, 1usize),
        DAV1D_PIXEL_LAYOUT_I422 => (Chroma::I422, 1usize, 0usize),
        DAV1D_PIXEL_LAYOUT_I444 => (Chroma::I444, 0usize, 0usize),
        _ => return Err(Error::new(ErrorKind::Unsupported, "AVIF pixel layout is unsupported")),
    };
    let width_us = width as usize;
    let height_us = height as usize;
    let chroma_width = width_us.div_ceil(1usize << sub_x);
    let chroma_height = height_us.div_ceil(1usize << sub_y);
    let y_stride = usize::try_from(picture.stride[0]).map_err(|_| Error::new(ErrorKind::Invalid, "negative AV1 luma stride"))?;
    let chroma_stride = usize::try_from(picture.stride[1]).map_err(|_| Error::new(ErrorKind::Invalid, "negative AV1 chroma stride"))?;
    if y_stride < width_us { return Err(Error::new(ErrorKind::Invalid, "AV1 luma stride is smaller than the image width")); }
    if chroma_stride < chroma_width { return Err(Error::new(ErrorKind::Invalid, "AV1 chroma stride is smaller than the chroma width")); }
    let plane_length = |index: usize, stride: usize, rows: usize, row_bytes: usize| -> Result<usize, Error> {
        NonNull::new(picture.data[index] as *mut u8).ok_or_else(|| Error::new(ErrorKind::Invalid, "AV1 decoder returned a missing plane"))?;
        let size = rows.checked_sub(1).and_then(|n| n.checked_mul(stride)).and_then(|n| n.checked_add(row_bytes))
            .filter(|&n| n <= isize::MAX as usize).ok_or_else(|| Error::new(ErrorKind::Limit, "AV1 plane size overflow"))?;
        Ok(size)
    };
    let plane_lengths = [plane_length(0, y_stride, height_us, width_us)?,
        plane_length(1, chroma_stride, chroma_height, chroma_width)?,
        plane_length(2, chroma_stride, chroma_height, chroma_width)?];
    let y_stride = u32::try_from(y_stride).map_err(|_| Error::new(ErrorKind::Limit, "AV1 luma stride overflow"))?;
    let chroma_stride = u32::try_from(chroma_stride).map_err(|_| Error::new(ErrorKind::Limit, "AV1 chroma stride overflow"))?;
    let full_range = picture.full_range != 0;
    let matrix = picture.matrix as u8;
    let data = picture.data.map(|p| NonNull::new(p as *mut u8).expect("validated AV1 plane"));
    Ok(DecodedStill {
        width,
        height,
        _picture: picture_guard,
        data,
        plane_lengths,
        y_stride,
        chroma_stride,
        chroma,
        full_range,
        matrix,
    })
}
