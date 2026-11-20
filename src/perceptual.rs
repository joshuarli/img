//! SSIMULACRA2 scoring of actual encoded candidates against a cached reference.

use crate::still::{Error, ErrorKind, PreparedPlanes, StillImage};
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
    Tiny([Vec<f32>; 3]),
    Opaque(Ssimulacra2Reference),
    Alpha { black: Ssimulacra2Reference, white: Ssimulacra2Reference },
}

/// A prepared reference and its owned metric pyramids, reused for each candidate.
pub struct PerceptualReference {
    width: u32,
    height: u32,
    alpha: Option<Vec<u8>>,
    metric: MetricReferences,
    stop: crate::cancellation::Cancellation,
}

fn composited(planes: &[Vec<f32>; 3], alpha: &[u8], background: f32) -> [Vec<f32>; 3] {
    std::array::from_fn(|c| planes[c].iter().zip(alpha).map(|(&value, &a)| over(value, a, background)).collect())
}

fn composite_in_place(planes: &mut [Vec<f32>; 3], alpha: &[u8], background: f32) {
    for plane in planes { for (value, &a) in plane.iter_mut().zip(alpha) { *value = over(*value, a, background); } }
}

fn over(value: f32, a: u8, background: f32) -> f32 {
    let alpha = a as f32 / 255.0;
    value * alpha + background * (1.0 - alpha)
}

impl PerceptualReference {
    /// Prepare an orientation-correct SDR reference exactly once.
    pub fn new(image: &StillImage) -> Result<Self, Error> { Self::new_cancellable(image,&crate::cancellation::Cancellation::default()) }

    pub(crate) fn new_cancellable(image: &StillImage, stop: &crate::cancellation::Cancellation) -> Result<Self, Error> {
        let mut config=fast_ssim2::Ssimulacra2Config::default();config.stop=Some(stop);
        let PreparedPlanes { width, height, mut planes, alpha } = image.prepare_planar()?;
        let (w, h) = (width as usize, height as usize);
        let reference = |planes| Ssimulacra2Reference::from_linear_planes(planes, w, h, &config).map_err(|e| Error::caused(ErrorKind::Score, e));
        let metric = if width < 8 || height < 8 { MetricReferences::Tiny(planes) }
        else if let Some(alpha) = &alpha {
            let black = reference(composited(&planes, alpha, 0.0))?;
            composite_in_place(&mut planes, alpha, 1.0);
            MetricReferences::Alpha { black, white: reference(planes)? }
        } else { MetricReferences::Opaque(reference(planes)?) };
        Ok(Self { width, height, alpha, metric, stop:stop.clone() })
    }

    /// Decode and score actual candidate bytes. Alpha must match exactly.
    pub fn score_encoded(&self, bytes: &[u8]) -> Result<f64, Error> {
        self.score_decoded(&StillImage::decode_cancellable(bytes,&self.stop)?)
    }

    /// Score a decoded candidate. Identical prepared pixels score exactly 100.
    pub(crate) fn score_decoded(&self, decoded: &StillImage) -> Result<f64, Error> {
        let PreparedPlanes { width, height, mut planes, alpha } = decoded.prepare_planar()?;
        if (self.width, self.height) != (width, height) { return Err(Error::new(ErrorKind::Score, "score dimensions differ")); }
        if self.alpha != alpha { return Err(Error::new(ErrorKind::Score, "decoded alpha differs from reference")); }
        let (w, h) = (width as usize, height as usize);
        let mut config=fast_ssim2::Ssimulacra2Config::default();config.stop=Some(&self.stop);
        let compare = |reference: &Ssimulacra2Reference, planes| reference.compare_linear_planes(planes, w, h, &config).map_err(|e| Error::caused(ErrorKind::Score, e));
        let score = match &self.metric {
            MetricReferences::Tiny(reference) => if *reference == planes { return Ok(100.0) }
                else { return Err(Error::new(ErrorKind::Unsupported, "nonidentical images below the metric's eight-pixel floor cannot be scored")) },
            MetricReferences::Opaque(reference) => compare(reference, planes)?,
            MetricReferences::Alpha { black, white } => {
                let alpha = alpha.as_deref().unwrap_or_default();
                let black = compare(black, composited(&planes, alpha, 0.0))?;
                composite_in_place(&mut planes, alpha, 1.0);
                black.min(compare(white, planes)?)
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
