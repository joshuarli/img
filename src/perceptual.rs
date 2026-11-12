//! SSIMULACRA2 scoring of actual encoded candidates against a cached reference.

use crate::still::{Error, ErrorKind, PreparedImage, StillImage};
use fast_ssim2::Ssimulacra2Reference;

/// Finite requested score in the inclusive range zero through one hundred.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScoreTarget(f64);

impl ScoreTarget {
    /// Validate a requested score. Measured scores have no lower bound.
    pub fn new(value: f64) -> Result<Self, Error> {
        if value.is_finite() && (0.0..=100.0).contains(&value) { Ok(Self(value)) }
        else { Err(Error::new(ErrorKind::Invalid, "score must be finite and in [0, 100]")) }
    }
    /// Requested minimum.
    pub fn value(self) -> f64 { self.0 }
}

impl Default for ScoreTarget {
    fn default() -> Self { Self(95.0) }
}

enum MetricReferences {
    Tiny,
    Opaque(Ssimulacra2Reference),
    Alpha { black: Ssimulacra2Reference, white: Ssimulacra2Reference },
}

/// A prepared reference and its owned metric pyramids, reused for each candidate.
pub struct PerceptualReference {
    pixels: PreparedImage,
    metric: MetricReferences,
    stop: crate::cancellation::Cancellation,
}

fn pixels<'a>(rgb: &'a [[f32; 3]], width: u32, height: u32) -> Result<zenpixels::PixelSlice<'a>, Error> {
    zenpixels::PixelSlice::new(bytemuck::cast_slice(rgb), width, height, width as usize * 12, zenpixels::PixelDescriptor::RGBF32_LINEAR)
        .map_err(|e| Error::caused(ErrorKind::Score, e))
}

fn composite(image: &PreparedImage, background: f32) -> Vec<[f32; 3]> {
    image.rgb.iter().zip(&image.alpha).map(|(rgb, &a)| {
        let alpha = a as f32 / 255.0;
        rgb.map(|value| value * alpha + background * (1.0 - alpha))
    }).collect()
}

impl PerceptualReference {
    /// Prepare an orientation-correct SDR reference exactly once.
    pub fn new(image: &StillImage) -> Result<Self, Error> { Self::new_cancellable(image,&crate::cancellation::Cancellation::default()) }

    pub(crate) fn new_cancellable(image: &StillImage, stop: &crate::cancellation::Cancellation) -> Result<Self, Error> {
        let mut config=fast_ssim2::Ssimulacra2Config::default();config.stop=Some(stop);
        let image = image.prepare()?;
        let (width, height) = image.dimensions();
        let metric = if width < 8 || height < 8 { MetricReferences::Tiny }
        else if image.alpha.iter().all(|&a| a == 255) {
            MetricReferences::Opaque(Ssimulacra2Reference::new_with_config(&pixels(&image.rgb, width, height)?,&config)
                .map_err(|e| Error::caused(ErrorKind::Score, e))?)
        } else {
            let black = composite(&image, 0.0);
            let black = Ssimulacra2Reference::new_with_config(&pixels(&black, width, height)?,&config)
                .map_err(|e| Error::caused(ErrorKind::Score, e))?;
            let white = composite(&image, 1.0);
            let white = Ssimulacra2Reference::new_with_config(&pixels(&white, width, height)?,&config)
                .map_err(|e| Error::caused(ErrorKind::Score, e))?;
            MetricReferences::Alpha { black, white }
        };
        Ok(Self { pixels: image, metric, stop:stop.clone() })
    }

    /// Decode and score actual candidate bytes. Alpha must match exactly.
    pub fn score_encoded(&self, bytes: &[u8]) -> Result<f64, Error> {
        let candidate = StillImage::decode_cancellable(bytes,&self.stop)?.prepare()?;
        self.score_prepared(&candidate)
    }

    fn score_prepared(&self, candidate: &PreparedImage) -> Result<f64, Error> {
        if self.pixels.dimensions() != candidate.dimensions() { return Err(Error::new(ErrorKind::Score, "score dimensions differ")); }
        if self.pixels.alpha != candidate.alpha { return Err(Error::new(ErrorKind::Score, "decoded alpha differs from reference")); }
        if self.pixels.rgb == candidate.rgb { return Ok(100.0); }
        let (width, height) = self.pixels.dimensions();
        let mut config=fast_ssim2::Ssimulacra2Config::default();config.stop=Some(&self.stop);
        let score = match &self.metric {
            MetricReferences::Tiny => return Err(Error::new(ErrorKind::Unsupported, "nonidentical images below the metric's eight-pixel floor cannot be scored")),
            MetricReferences::Opaque(reference) => reference.compare_with_config(&pixels(&candidate.rgb, width, height)?,&config)
                .map_err(|e| Error::caused(ErrorKind::Score, e))?,
            MetricReferences::Alpha { black, white } => {
                let black_pixels = composite(candidate, 0.0);
                let black = black.compare_with_config(&pixels(&black_pixels, width, height)?,&config).map_err(|e| Error::caused(ErrorKind::Score, e))?;
                drop(black_pixels);
                let white_pixels = composite(candidate, 1.0);
                let white = white.compare_with_config(&pixels(&white_pixels, width, height)?,&config).map_err(|e| Error::caused(ErrorKind::Score, e))?;
                black.min(white)
            },
        };
        if !score.is_finite() { return Err(Error::new(ErrorKind::Score, "metric returned a nonfinite score")); }
        Ok(score)
    }
}

/// Compare encoded representations without adding a pass/fail threshold.
pub fn compare(reference: &[u8], candidate: &[u8]) -> Result<f64, Error> {
    PerceptualReference::new(&StillImage::decode(reference)?)?.score_encoded(candidate)
}
