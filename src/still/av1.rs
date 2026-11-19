//! Safe wrapper over the BSD-2-Clause rav1d AV1 decoder.
//!
//! The raw dav1d-style API is the only permissive pure-Rust AV1 decode path.

use std::mem::MaybeUninit;
use std::ptr::NonNull;

use rav1d::include::dav1d::dav1d::{Dav1dContext, Dav1dSettings};
use rav1d::include::dav1d::data::Dav1dData;
use rav1d::include::dav1d::headers::*;
use rav1d::include::dav1d::picture::Dav1dPicture;
use rav1d::src::lib::{dav1d_close, dav1d_data_create, dav1d_data_unref, dav1d_default_settings, dav1d_get_picture, dav1d_open, dav1d_picture_unref, dav1d_send_data};

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
    picture: PictureGuard,
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
            let data = self.picture.0.data[index].expect("validated AV1 plane");
            // SAFETY: The default rav1d allocator initializes each full plane, including stride padding.
            // The validated lengths end within those planes, and self owns the picture for the borrow.
            unsafe { std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), self.plane_lengths[index]) }
        })
    }
}

struct PictureGuard(Dav1dPicture);

impl Drop for PictureGuard {
    fn drop(&mut self) {
        unsafe { dav1d_picture_unref(Some(NonNull::from(&mut self.0))) };
    }
}

struct ContextGuard(Option<Dav1dContext>);

impl Drop for ContextGuard {
    fn drop(&mut self) {
        if self.0.is_some() {
            unsafe { dav1d_close(Some(NonNull::from(&mut self.0))) };
        }
    }
}

fn error_code(code: std::ffi::c_int) -> std::ffi::c_int { code.unsigned_abs() as std::ffi::c_int }

pub(super) fn decode_still(data: &[u8], stop: &crate::cancellation::Cancellation) -> Result<DecodedStill, Error> {
    if stop.is_cancelled() { return Err(Error::new(ErrorKind::Invalid, "operation cancelled")); }
    let mut settings = MaybeUninit::<Dav1dSettings>::uninit();
    unsafe { dav1d_default_settings(NonNull::from(&mut settings).cast::<Dav1dSettings>()) };
    let settings = unsafe { settings.assume_init_mut() };
    settings.n_threads = 1;
    settings.max_frame_delay = 1;
    settings.apply_grain = 1;
    settings.frame_size_limit = super::MAX_PIXELS as u32;
    let mut context = None;
    let open = unsafe { dav1d_open(Some(NonNull::from(&mut context)), Some(NonNull::from(settings))) };
    if open.0 != 0 {
        return Err(Error::new(ErrorKind::Invalid, format!("AV1 decoder open failed with error {}", error_code(open.0))));
    }
    let _guard = ContextGuard(context);
    let ctx = context.ok_or_else(|| Error::new(ErrorKind::Invalid, "AV1 decoder open returned no context"))?;

    let mut data_slot = Dav1dData::default();
    let buffer = unsafe { dav1d_data_create(Some(NonNull::from(&mut data_slot)), data.len()) };
    let Some(buffer) = NonNull::new(buffer) else {
        unsafe { dav1d_data_unref(Some(NonNull::from(&mut data_slot))) };
        return Err(Error::new(ErrorKind::Limit, "could not allocate an AV1 decoder buffer"));
    };
    unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), buffer.as_ptr(), data.len()) };
    let send = (|| -> Result<Vec<PictureGuard>, Error> {
        let mut displayed = Vec::new();
        loop {
            let result = unsafe { dav1d_send_data(Some(ctx), Some(NonNull::from(&mut data_slot))) };
            if result.0 == 0 { return Ok(displayed); }
            if result.0 != -(libc::EAGAIN as std::ffi::c_int) {
                return Err(Error::new(ErrorKind::Invalid, format!("AV1 decoder rejected the image with error {}", error_code(result.0))));
            }
            let mut pending = Dav1dPicture::default();
            let drain = unsafe { dav1d_get_picture(Some(ctx), Some(NonNull::from(&mut pending))) };
            if drain.0 != 0 {
                unsafe { dav1d_picture_unref(Some(NonNull::from(&mut pending))) };
                return Err(Error::new(ErrorKind::Invalid, format!("AV1 decoder rejected the image with error {}", error_code(result.0))));
            }
            displayed.push(PictureGuard(pending));
        }
    })();
    unsafe { dav1d_data_unref(Some(NonNull::from(&mut data_slot))) };
    let mut displayed = send?;

    loop {
        let mut picture = Dav1dPicture::default();
        let result = unsafe { dav1d_get_picture(Some(ctx), Some(NonNull::from(&mut picture))) };
        if result.0 != 0 {
            unsafe { dav1d_picture_unref(Some(NonNull::from(&mut picture))) };
            break;
        }
        displayed.push(PictureGuard(picture));
    }
    if displayed.len() != 1 {
        return Err(Error::new(ErrorKind::Unsupported, "AVIF requires exactly one displayed frame"));
    }
    let picture = &displayed[0].0;
    if picture.content_light.is_some() || picture.mastering_display.is_some() {
        return Err(Error::new(ErrorKind::Unsupported, "AVIF HDR metadata requires independently validated processing"));
    }
    if picture.p.bpc != 8 {
        return Err(Error::new(ErrorKind::Unsupported, "AVIF decoding requires eight-bit precision"));
    }
    let layout = picture.p.layout;
    if layout == DAV1D_PIXEL_LAYOUT_I400 {
        return Err(Error::new(ErrorKind::Unsupported, "monochrome AVIF decoding is not independently verified"));
    }
    let sequence: &Dav1dSequenceHeader = unsafe {
        picture.seq_hdr.map(|pointer| pointer.as_ref())
            .ok_or_else(|| Error::new(ErrorKind::Invalid, "AV1 sequence header missing"))?
    };
    if sequence.still_picture == 0 {
        return Err(Error::new(ErrorKind::Unsupported, "AVIF requires a still AV1 bitstream"));
    }
    if sequence.pri != DAV1D_COLOR_PRI_BT709 || sequence.trc != DAV1D_TRC_SRGB {
        return Err(Error::new(ErrorKind::Unsupported, "AVIF bitstream requires validated SDR sRGB color signaling"));
    }
    let width = u32::try_from(picture.p.w).map_err(|_| Error::new(ErrorKind::Limit, "AVIF width overflow"))?;
    let height = u32::try_from(picture.p.h).map_err(|_| Error::new(ErrorKind::Limit, "AVIF height overflow"))?;
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
        picture.data[index].ok_or_else(|| Error::new(ErrorKind::Invalid, "AV1 decoder returned a missing plane"))?;
        let size = rows.checked_sub(1).and_then(|n| n.checked_mul(stride)).and_then(|n| n.checked_add(row_bytes))
            .filter(|&n| n <= isize::MAX as usize).ok_or_else(|| Error::new(ErrorKind::Limit, "AV1 plane size overflow"))?;
        Ok(size)
    };
    let plane_lengths = [plane_length(0, y_stride, height_us, width_us)?,
        plane_length(1, chroma_stride, chroma_height, chroma_width)?,
        plane_length(2, chroma_stride, chroma_height, chroma_width)?];
    let y_stride = u32::try_from(y_stride).map_err(|_| Error::new(ErrorKind::Limit, "AV1 luma stride overflow"))?;
    let chroma_stride = u32::try_from(chroma_stride).map_err(|_| Error::new(ErrorKind::Limit, "AV1 chroma stride overflow"))?;
    let full_range = sequence.color_range != 0;
    let matrix = sequence.mtrx as u8;
    let picture = displayed.pop().expect("validated single AV1 picture");
    Ok(DecodedStill {
        width,
        height,
        picture,
        plane_lengths,
        y_stride,
        chroma_stride,
        chroma,
        full_range,
        matrix,
    })
}
