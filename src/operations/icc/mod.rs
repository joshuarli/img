use zune_core::{bit_depth::BitType, colorspace::ColorSpace};
use zune_image::{errors::{ImageErrors, ImageOperationsErrors}, frame::Frame, image::Image, traits::OperationsTrait};
use moxcms::{ColorProfile, DataColorSpace, Layout};

/// Apply an ICC destination profile through floating-point color management.
pub struct ApplyICC {
    profile: ColorProfile,
    bytes: Vec<u8>,
}

fn error(value: impl std::fmt::Display) -> ImageErrors {
    ImageErrors::OperationsError(ImageOperationsErrors::GenericString(value.to_string()))
}

impl ApplyICC {
    /// Parse a bounded destination profile. RGB matrix profiles are supported.
    pub fn new(bytes: &[u8]) -> Result<Self, ImageErrors> {
        if bytes.len() > crate::still::MAX_METADATA_BYTES { return Err(error("ICC profile exceeds metadata limit")); }
        let profile = ColorProfile::new_from_slice(bytes).map_err(error)?;
        if profile.color_space != DataColorSpace::Rgb || !profile.is_matrix_shaper() {
            return Err(error("ICC destination requires a validated RGB matrix profile"));
        }
        Ok(Self { profile, bytes: bytes.to_vec() })
    }
}

impl OperationsTrait for ApplyICC {
    fn name(&self) -> &'static str { "apply icc profile" }
    fn execute_impl(&self, image: &mut Image) -> Result<(), ImageErrors> {
        let source = match image.metadata().icc_chunk() {
            Some(bytes) => ColorProfile::new_from_slice(bytes).map_err(error)?,
            None => ColorProfile::new_srgb(),
        };
        if source.color_space != DataColorSpace::Rgb || !source.is_matrix_shaper() { return Err(error("ICC source requires a validated RGB matrix profile")); }
        let transform = source.create_transform_f32(Layout::Rgb, &self.profile, Layout::Rgb, crate::still::color::transform_options()).map_err(error)?;
        let colorspace = image.colorspace();
        if !matches!(colorspace, ColorSpace::RGB | ColorSpace::RGBA) { return Err(error("ICC application requires RGB or RGBA samples")); }
        let channels = colorspace.num_components();
        let depth = image.depth().bit_type();
        for frame in image.frames_mut() {
            let numerator = frame.numerator(); let denominator = frame.denominator();
            let (values, maximum): (Vec<u16>, f32) = match depth {
                BitType::U8 => (frame.flatten::<u8>().into_iter().map(u16::from).collect(), 255.0),
                BitType::U16 => (frame.flatten::<u16>(), 65535.0),
                _ => return Err(error("ICC application requires eight- or sixteen-bit integer samples")),
            };
            let input: Vec<_> = values.chunks_exact(channels).flat_map(|p| p[..3].iter().map(|&v| v as f32 / maximum)).collect();
            let mut output = vec![0.0; input.len()];
            transform.transform(&input, &mut output).map_err(error)?;
            let mut pixels = Vec::with_capacity(values.len());
            for (color, source) in output.chunks_exact(3).zip(values.chunks_exact(channels)) {
                for &value in color { pixels.push((crate::still::color::checked_sdr(value).map_err(error)? * maximum).round() as u16); }
                if channels == 4 { pixels.push(source[3]); }
            }
            *frame = if depth == BitType::U8 {
                Frame::from_u8(&pixels.into_iter().map(|v| v as u8).collect::<Vec<_>>(), colorspace, numerator, denominator)
            } else { Frame::from_u16(&pixels, colorspace, numerator, denominator) };
        }
        image.metadata_mut().set_icc_chunk(self.bytes.clone());
        Ok(())
    }
    fn supported_types(&self) -> &'static [BitType] { &[BitType::U8, BitType::U16] }
    fn supported_colorspaces(&self) -> &'static [ColorSpace] { &[ColorSpace::RGB, ColorSpace::RGBA] }
}

/// Apply the SDR sRGB profile without changing decoded alpha.
pub struct ApplySRGB;

impl OperationsTrait for ApplySRGB {
    fn name(&self) -> &'static str { "apply srgb profile" }
    fn execute_impl(&self, image: &mut Image) -> Result<(), ImageErrors> {
        if image.metadata().icc_chunk().is_none() { return Ok(()); }
        ApplyICC::new(&ColorProfile::new_srgb().encode().map_err(error)?)?.execute_impl(image)
    }
    fn supported_types(&self) -> &'static [BitType] { &[BitType::U8, BitType::U16] }
    fn supported_colorspaces(&self) -> &'static [ColorSpace] { &[ColorSpace::RGB, ColorSpace::RGBA] }
}

#[cfg(test)]
mod tests;
