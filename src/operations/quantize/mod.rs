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
    /// - dithering: Floyd-Steinberg dithering level between 0.0 and 1.0; `None` keeps the library default (full dithering)
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

        let colors = 2 + (f32::from(self.quality) * 254.0 / 100.0).round() as i32;
        let mut options = quantizr::Options::default();
        options
            .set_max_colors(colors)
            .map_err(|_| ImageOperationsErrors::Generic("palette color count is outside 2..=256"))?;

        for frame in image.frames_mut() {
            let pixels = frame.flatten::<u8>();
            let qimage = quantizr::Image::new(&pixels, src_width, src_height)
                .map_err(|_| ImageOperationsErrors::Generic("palette quantization input rejected"))?;
            let mut quantized = quantizr::QuantizeResult::quantize(&qimage, &options);
            if let Some(level) = self.dithering {
                quantized.set_dithering_level(level.clamp(0.0, 1.0)).map_err(|_| {
                    ImageOperationsErrors::Generic("palette dithering level is outside 0.0..=1.0")
                })?;
            }
            let mut indices = vec![0u8; channel_len];
            quantized
                .remap_image(&qimage, &mut indices)
                .map_err(|_| ImageOperationsErrors::Generic("palette remapping failed"))?;
            let palette = quantized.get_palette();
            let entries = &palette.entries[..palette.count as usize];
            let channels = indices.iter().enumerate().map(|(idx, &index)| {
                let color = &entries[index as usize];
                (idx, color.r, color.g, color.b, color.a)
            }).fold(
                vec![Channel::new_with_bit_type(channel_len, BitType::U8); 4],
                |mut acc, (idx, r, g, b, a)| {
                    // SAFETY: idx is bounded by indices().len() which equals
                    // src_width * src_height, and channels are pre-allocated
                    // with channel_len (= src_width * src_height) bytes of U8.
                    // Each alias_mut slice therefore has at least idx+1 elements.
                    unsafe {
                        debug_assert!(idx < channel_len);
                        acc[0].alias_mut()[idx] = r;
                        acc[1].alias_mut()[idx] = g;
                        acc[2].alias_mut()[idx] = b;
                        acc[3].alias_mut()[idx] = a;
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
