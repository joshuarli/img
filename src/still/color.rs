use super::{Error, ErrorKind, StillImage, MAX_METADATA_BYTES};
#[cfg(feature="resize")]
use super::SampleLayout;
use moxcms::{ColorProfile, DataColorSpace, Layout, TransformOptions};

/// Whether a metadata change must keep sample order exactly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SamplePreservation {
    /// Refuse orientation normalization that would reorder samples.
    Exact,
    /// Normalize orientation before removing orientation tags.
    AllowReorientation,
}

/// Orientation-correct SDR sRGB pixels in linear light, with separate exact alpha.
pub struct PreparedImage {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgb: Vec<[f32; 3]>,
    pub(crate) alpha: Vec<u8>,
}

/// Planar linear-light sRGB samples in display orientation, for scoring.
pub(crate) struct PreparedPlanes {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) planes: [Vec<f32>; 3],
    /// Exact alpha, or `None` when every sample is opaque.
    pub(crate) alpha: Option<Vec<u8>>,
}

impl PreparedImage {
    /// Display dimensions after orientation correction.
    pub fn dimensions(&self) -> (u32, u32) { (self.width, self.height) }
    /// Linear-light sRGB samples. Alpha is not premultiplied.
    pub fn linear_rgb(&self) -> &[[f32; 3]] { &self.rgb }
    /// Exact decoded alpha, with 255 for opaque images.
    pub fn alpha(&self) -> &[u8] { &self.alpha }
}

pub(crate) fn transform_options() -> TransformOptions {
    TransformOptions { prefer_fixed_point: false, allow_extended_range_rgb_xyz: true, ..TransformOptions::default() }
}

pub(crate) fn checked_sdr(value: f32) -> Result<f32, Error> {
    if !value.is_finite() || !(-0.00001..=1.00001).contains(&value) {
        return Err(Error::new(ErrorKind::Unsupported, format!("color transform leaves the SDR sRGB gamut: {value}")));
    }
    Ok(value.clamp(0.0, 1.0))
}

fn linear_srgb() -> ColorProfile {
    let mut profile = ColorProfile::new_srgb();
    let linear = moxcms::ToneReprCurve::Lut(Vec::new());
    profile.red_trc = Some(linear.clone()); profile.green_trc = Some(linear.clone()); profile.blue_trc = Some(linear);
    profile.cicp = None;
    profile
}

pub(crate) fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 { value / 12.92 } else { libm::powf((value + 0.055) / 1.055,2.4) }
}

#[cfg(feature="resize")]
fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.0031308 { value * 12.92 } else { 1.055 * libm::powf(value,1.0 / 2.4) - 0.055 }
}

enum ColorInterpretation {
    Srgb,
    Gamma(f32),
    Profile(ColorProfile),
}

impl StillImage {
    fn color_interpretation(&self) -> Result<ColorInterpretation, Error> {
        if self.metadata.content_light_level.is_some() || self.metadata.mastering_display.is_some() {
            return Err(Error::new(ErrorKind::Unsupported, "HDR metadata is unsupported for SDR processing"));
        }
        if let Some(cicp) = self.metadata.cicp
            && (cicp.color_primaries != 1 || cicp.transfer_characteristics != 13) {
            return Err(Error::new(ErrorKind::Unsupported, "unrecognized or non-SDR CICP color signaling"));
        }
        for chunk in &self.png_metadata {
            match &chunk[4..8] {
                b"cICP" if chunk.get(8..12) != Some(&[1, 13, 0, 1]) => return Err(Error::new(ErrorKind::Unsupported, "PNG CICP signaling is unsupported")),
                b"cLLI" | b"mDCV" => return Err(Error::new(ErrorKind::Unsupported, "PNG HDR signaling is unsupported")),
                b"cHRM" => {
                    let expected: Vec<_> = [31270u32, 32900, 64000, 33000, 30000, 60000, 15000, 6000].into_iter().flat_map(u32::to_be_bytes).collect();
                    if chunk.get(8..40) != Some(expected.as_slice()) && self.metadata.icc_profile.is_none() {
                        return Err(Error::new(ErrorKind::Unsupported, "PNG chromaticities require an independently validated profile"));
                    }
                },
                _ => {},
            }
        }
        if self.info.source_color.color_authority==zencodec::ColorAuthority::Cicp {return Ok(ColorInterpretation::Srgb);}
        if let Some(icc) = &self.metadata.icc_profile {
            if icc.len() > MAX_METADATA_BYTES { return Err(Error::new(ErrorKind::Limit, "ICC profile exceeds metadata limit")); }
            let profile = ColorProfile::new_from_slice(icc).map_err(|e| Error::caused(ErrorKind::Unsupported, e))?;
            if !matches!(profile.color_space, DataColorSpace::Rgb | DataColorSpace::Gray)
                || (profile.color_space == DataColorSpace::Rgb && !profile.is_matrix_shaper()) {
                return Err(Error::new(ErrorKind::Unsupported, "ICC profile is not a validated RGB matrix or gray profile"));
            }
            return Ok(ColorInterpretation::Profile(profile));
        }
        if !self.png_metadata.iter().any(|chunk| matches!(&chunk[4..8], b"sRGB" | b"cICP"))
            && let Some(gamma) = self.png_metadata.iter().find(|chunk| &chunk[4..8] == b"gAMA") {
            let bytes = gamma.get(8..12).ok_or_else(|| Error::new(ErrorKind::Invalid, "invalid PNG gamma"))?;
            let value = u32::from_be_bytes(bytes.try_into().unwrap());
            if value == 0 { return Err(Error::new(ErrorKind::Invalid, "zero PNG gamma")); }
            return Ok(ColorInterpretation::Gamma(100000.0 / value as f32));
        }
        Ok(ColorInterpretation::Srgb)
    }

    /// Prepare eight-bit SDR pixels for scoring without clamping gamut or changing alpha.
    pub fn prepare(&self) -> Result<PreparedImage, Error> {
        if self.bit_depth() != 8 || self.info.source_color.bit_depth.is_some_and(|d| d > 8) {
            return Err(Error::new(ErrorKind::Unsupported, "scoring and resizing require eight-bit SDR samples"));
        }
        let color = self.color_interpretation()?;
        let channels = self.pixels.descriptor().channels();
        let samples = self.samples();
        let srgb_transfer: Option<[f32;256]> = matches!(&color,ColorInterpretation::Srgb)
            .then(|| std::array::from_fn(|value| srgb_to_linear(value as f32 / 255.0)));
        let mut rgb: Vec<f32> = samples.chunks_exact(channels).flat_map(|p| {
            let rgb = if self.is_grayscale() { [p[0], p[0], p[0]] } else { [p[0], p[1], p[2]] };
            rgb.map(|v| match &srgb_transfer {Some(transfer)=>transfer[v as usize],None=>v as f32 / 255.0})
        }).collect();
        match color {
            ColorInterpretation::Profile(profile) => {
                let layout = if profile.color_space == DataColorSpace::Gray {
                    if !self.is_grayscale() { return Err(Error::new(ErrorKind::Unsupported, "gray ICC profile on RGB samples")); }
                    Layout::Gray
                } else { Layout::Rgb };
                let input = if layout == Layout::Gray { rgb.chunks_exact(3).map(|p| p[0]).collect() } else { rgb };
                rgb = vec![0.0; samples.len() / channels * 3];
                let transform = profile.create_transform_f32(layout, &linear_srgb(), Layout::Rgb, transform_options())
                    .map_err(|e| Error::caused(ErrorKind::Unsupported, e))?;
                transform.transform(&input, &mut rgb).map_err(|e| Error::caused(ErrorKind::Unsupported, e))?;
            },
            ColorInterpretation::Gamma(gamma) => for value in &mut rgb { *value = libm::powf(*value,gamma); },
            ColorInterpretation::Srgb => {},
        }
        if rgb.iter().any(|v| !v.is_finite()) { return Err(Error::new(ErrorKind::Unsupported, "nonfinite prepared color sample")); }
        let alpha: Vec<_> = samples.chunks_exact(channels).map(|p| if self.has_alpha() { p[channels - 1] } else { 255 }).collect();
        let (width, height) = self.dimensions();
        let orientation = self.metadata.orientation as u8;
        let pixels: Vec<_> = rgb.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect();
        drop(rgb); drop(samples);
        let (rgb, width, height) = orient(pixels, width, height, 1, orientation);
        let (alpha, _, _) = orient(alpha, self.dimensions().0, self.dimensions().1, 1, orientation);
        Ok(PreparedImage { width, height, rgb, alpha })
    }

    /// [`Self::prepare`] as owned planes, without intermediate interleaved copies.
    pub(crate) fn prepare_planar(&self) -> Result<PreparedPlanes, Error> {
        if self.bit_depth() != 8 || self.info.source_color.bit_depth.is_some_and(|d| d > 8) {
            return Err(Error::new(ErrorKind::Unsupported, "scoring and resizing require eight-bit SDR samples"));
        }
        let color = self.color_interpretation()?;
        if matches!(color, ColorInterpretation::Profile(_)) {
            let prepared = self.prepare()?;
            let planes = std::array::from_fn(|c| prepared.rgb.iter().map(|p| p[c]).collect());
            let alpha = prepared.alpha.iter().any(|&a| a != 255).then_some(prepared.alpha);
            return Ok(PreparedPlanes { width: prepared.width, height: prepared.height, planes, alpha });
        }
        let channels = self.pixels.descriptor().channels();
        let gray = self.is_grayscale();
        let (width, height) = self.dimensions();
        let orientation = self.metadata.orientation as u8;
        let (out_width, out_height) = if orientation >= 5 { (height, width) } else { (width, height) };
        let transfer: [f32; 256] = match color {
            ColorInterpretation::Srgb => std::array::from_fn(|value| srgb_to_linear(value as f32 / 255.0)),
            ColorInterpretation::Gamma(gamma) => std::array::from_fn(|value| libm::powf(value as f32 / 255.0, gamma)),
            ColorInterpretation::Profile(_) => unreachable!(),
        };
        if transfer.iter().any(|v| !v.is_finite()) { return Err(Error::new(ErrorKind::Unsupported, "nonfinite prepared color sample")); }
        let count = out_width as usize * out_height as usize;
        let mut planes: [Vec<f32>; 3] = std::array::from_fn(|_| Vec::with_capacity(count));
        let mut alpha = self.has_alpha().then(|| Vec::with_capacity(count));
        let source = self.pixels.as_slice();
        for y in 0..out_height { for x in 0..out_width {
            let (sx, sy) = source_position(orientation, width, height, x, y);
            let at = sx as usize * channels;
            let p = &source.row(sy)[at..at + channels];
            let rgb = if gray { [p[0]; 3] } else { [p[0], p[1], p[2]] };
            for c in 0..3 { planes[c].push(transfer[rgb[c] as usize]); }
            if let Some(alpha) = &mut alpha { alpha.push(p[channels - 1]); }
        }}
        let alpha = alpha.filter(|a| a.iter().any(|&v| v != 255));
        Ok(PreparedPlanes { width: out_width, height: out_height, planes, alpha })
    }

    /// Strip private metadata while retaining required color interpretation.
    pub fn strip_metadata(&self, preservation: SamplePreservation) -> Result<Self, Error> {
        let orientation = self.metadata.orientation as u8;
        if orientation != 1 && preservation == SamplePreservation::Exact {
            return Err(Error::new(ErrorKind::Unsupported, "lossless stripping cannot normalize orientation without reordering samples"));
        }
        let mut image = self.clone();
        if orientation != 1 {
            let bpp = self.pixels.descriptor().bytes_per_pixel();
            let samples = self.samples();
            let (samples, width, height) = orient(samples, self.dimensions().0, self.dimensions().1, bpp, orientation);
            image.pixels = zenpixels::PixelBuffer::from_vec(samples, width, height, self.pixels.descriptor())
                .map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
            image.info.width = width; image.info.height = height;
        }
        image.metadata.exif = None; image.metadata.xmp = None;
        image.metadata.orientation = zencodec::Orientation::Identity;
        image.info.orientation = zencodec::Orientation::Identity;
        image.png_metadata.retain(|chunk| super::png::is_color_chunk(&chunk[4..8]));
        Ok(image)
    }

    /// Resize in linear light with premultiplied alpha, preserving aspect ratio and never enlarging.
    pub fn resize_to_fit(&self, maximum: std::num::NonZeroU32) -> Result<Self, Error> {
        #[cfg(not(feature = "resize"))]
        { let _ = maximum; Err(Error::new(ErrorKind::Unsupported, "resize support is disabled")) }
        #[cfg(feature = "resize")]
        {
            use fast_image_resize::{images::TypedImage, pixels::F32x4, FilterType, ResizeAlg, ResizeOptions, Resizer};
            let prepared = self.prepare()?;
            let largest = prepared.width.max(prepared.height);
            if largest <= maximum.get() { return Ok(self.clone()); }
            let width = ((u64::from(prepared.width) * u64::from(maximum.get())) / u64::from(largest)).max(1) as u32;
            let height = ((u64::from(prepared.height) * u64::from(maximum.get())) / u64::from(largest)).max(1) as u32;
            let pixels = prepared.rgb.iter().zip(&prepared.alpha).map(|(rgb, &a)| {
                let a = a as f32 / 255.0; F32x4::new([rgb[0] * a, rgb[1] * a, rgb[2] * a, a])
            }).collect();
            let source = TypedImage::from_pixels(prepared.width, prepared.height, pixels).map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
            let mut resized = TypedImage::<F32x4>::new(width, height);
            let options = ResizeOptions { algorithm: ResizeAlg::Convolution(FilterType::Bilinear), mul_div_alpha: false, ..ResizeOptions::default() };
            Resizer::new().resize_typed(&source, &mut resized, &options).map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
            let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
            let mut alpha = Vec::with_capacity(width as usize * height as usize);
            for pixel in resized.pixels() {
                let a = checked_sdr(pixel.0[3])?;
                alpha.push((a * 255.0).round() as u8);
                for &color in &pixel.0[..3] { rgb.push(linear_to_srgb(if a > 0.0 { checked_sdr(color / a)? } else { 0.0 })); }
            }
            match self.color_interpretation()? {
                ColorInterpretation::Profile(profile) => {
                    let gray_profile = profile.color_space == DataColorSpace::Gray;
                    let input = rgb; rgb = vec![0.0; if gray_profile { input.len() / 3 } else { input.len() }];
                    let transform = ColorProfile::new_srgb().create_transform_f32(Layout::Rgb, &profile, if gray_profile { Layout::Gray } else { Layout::Rgb }, transform_options())
                        .map_err(|e| Error::caused(ErrorKind::Unsupported, e))?;
                    transform.transform(&input, &mut rgb).map_err(|e| Error::caused(ErrorKind::Unsupported, e))?;
                    if gray_profile { rgb = rgb.into_iter().flat_map(|v| [v, v, v]).collect(); }
                },
                ColorInterpretation::Gamma(gamma) => for value in &mut rgb { *value = libm::powf(srgb_to_linear(*value),1.0 / gamma); },
                ColorInterpretation::Srgb => {},
            }
            let gray = self.is_grayscale();
            let has_alpha = self.has_alpha();
            let channels = if gray { 1 } else { 3 };
            let bpp = channels + usize::from(has_alpha);
            let mut pixels = Vec::with_capacity(width as usize * height as usize * bpp);
            for (color, &a) in rgb.chunks_exact(3).zip(&alpha) {
                for &value in &color[..channels] { pixels.push((checked_sdr(value)? * 255.0).round() as u8); }
                if has_alpha { pixels.push(a); }
            }
            let orientation = self.metadata.orientation as u8;
            let inverse = match orientation { 6 => 8, 8 => 6, v => v };
            let (pixels, width, height) = orient(pixels, width, height, bpp, inverse);
            let layout = match (gray, has_alpha) { (true, true) => SampleLayout::GrayAlpha8, (true, false) => SampleLayout::Gray8, (false, true) => SampleLayout::Rgba8, (false, false) => SampleLayout::Rgb8 };
            let mut image = Self::from_samples(width, height, layout, pixels)?;
            image.metadata = self.metadata.clone(); image.png_metadata = self.png_metadata.clone(); image.format = self.format;
            image.info = self.info.clone(); image.info.width = width; image.info.height = height;
            Ok(image)
        }
    }
}

fn source_position(orientation: u8, width: u32, height: u32, x: u32, y: u32) -> (u32, u32) {
    match orientation {
        2 => (width - 1 - x, y), 3 => (width - 1 - x, height - 1 - y), 4 => (x, height - 1 - y),
        5 => (y, x), 6 => (y, height - 1 - x), 7 => (width - 1 - y, height - 1 - x), 8 => (width - 1 - y, x),
        _ => (x, y),
    }
}

fn orient<T: Clone>(pixels: Vec<T>, width: u32, height: u32, channels: usize, orientation: u8) -> (Vec<T>, u32, u32) {
    if orientation == 1 { return (pixels, width, height); }
    let (out_width, out_height) = if orientation >= 5 { (height, width) } else { (width, height) };
    let mut output = Vec::with_capacity(pixels.len());
    for y in 0..out_height { for x in 0..out_width {
        let (sx, sy) = source_position(orientation, width, height, x, y);
        let at = (sy * width + sx) as usize * channels;
        output.extend_from_slice(&pixels[at..at + channels]);
    }}
    (output, out_width, out_height)
}
