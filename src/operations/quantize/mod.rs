use zune_core::{bit_depth::BitType, colorspace::ColorSpace};
use zune_image::{
    channel::Channel,
    errors::{ImageErrors, ImageOperationsErrors},
    traits::OperationsTrait,
};

/// Reduce image palette
pub struct Quantize {
    quality: u8,
    dithering: Option<f32>,
}

impl Quantize {
    /// Create a new quantization operation
    ///
    /// # Arguments
    /// - quality: resulting quality of the palette; maps to at most `2 + quality * 254 / 100` colors
    /// - dithering: when `Some`, apply Floyd-Steinberg dithering; the value is otherwise ignored
    #[must_use]
    pub fn new(quality: u8, dithering: Option<f32>) -> Self {
        Self { quality, dithering }
    }
}

impl OperationsTrait for Quantize {
    fn name(&self) -> &'static str {
        "quantize"
    }

    fn execute_impl(&self, image: &mut zune_image::image::Image) -> Result<(), ImageErrors> {
        let (src_width, src_height) = image.dimensions();
        let channel_len =
            src_width
                .checked_mul(src_height)
                .ok_or(ImageOperationsErrors::Generic(
                    "Image dimensions overflow during quantization",
                ))?;

        let colors = 2 + (f32::from(self.quality) * 254.0 / 100.0).round() as usize;
        let size = quantette::PaletteSize::from_usize_clamped(colors);

        for frame in image.frames_mut() {
            let pixels = frame.flatten::<u8>();
            if pixels.chunks_exact(4).any(|px| px[3] != 255) {
                return Err(ImageOperationsErrors::Generic(
                    "palette quantization requires fully opaque samples",
                )
                .into());
            }
            let rgb: Vec<quantette::deps::palette::Srgb<u8>> = pixels
                .chunks_exact(4)
                .map(|px| quantette::deps::palette::Srgb::new(px[0], px[1], px[2]))
                .collect();
            let width = u32::try_from(src_width)
                .map_err(|_| ImageOperationsErrors::Generic("image width exceeds u32"))?;
            let height = u32::try_from(src_height)
                .map_err(|_| ImageOperationsErrors::Generic("image height exceeds u32"))?;
            let reference = quantette::ImageRef::new(width, height, &rgb)
                .map_err(|_| ImageOperationsErrors::Generic("palette quantization input rejected"))?;
            let quantized = quantette::Pipeline::new()
                .palette_size(size)
                .ditherer(self.dithering.map(|_| quantette::dither::FloydSteinberg::new()))
                .input_image(reference)
                .output_srgb8_indexed_image();
            let palette = quantized.palette();
            let channels = quantized.indices().iter().enumerate().map(|(idx, &index)| {
                let px = &palette[index as usize];
                (idx, px.red, px.green, px.blue)
            }).fold(
                vec![Channel::new_with_bit_type(channel_len, BitType::U8); 4],
                |mut acc, (idx, r, g, b)| {
                    // SAFETY: idx is bounded by indices().len() which equals
                    // src_width * src_height, and channels are pre-allocated
                    // with channel_len (= src_width * src_height) bytes of U8.
                    // Each alias_mut slice therefore has at least idx+1 elements.
                    unsafe {
                        debug_assert!(idx < channel_len);
                        acc[0].alias_mut()[idx] = r;
                        acc[1].alias_mut()[idx] = g;
                        acc[2].alias_mut()[idx] = b;
                        acc[3].alias_mut()[idx] = 255;
                    }

                    acc
                },
            );

            frame.set_channels(channels);
        }

        Ok(())
    }

    fn supported_types(&self) -> &'static [BitType] {
        &[BitType::U8][..]
    }

    fn supported_colorspaces(&self) -> &'static [ColorSpace] {
        &[ColorSpace::RGBA][..]
    }
}

#[cfg(test)]
mod tests;
