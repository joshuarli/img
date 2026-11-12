//! Bounded still-image decoding and encoding.

use std::{borrow::Cow, fmt, io::Read};
use zencodec::{decode::{DecodeJob, Decode, DecoderConfig}, encode::{EncodeJob, Encoder, EncoderConfig}};
use zenpixels::{PixelBuffer, PixelDescriptor};

mod png;
mod avif;
pub(crate) mod color;
pub use color::{PreparedImage, SamplePreservation};

/// Maximum compressed input, including stdin.
pub const MAX_INPUT_BYTES: u64 = 128 * 1024 * 1024;
/// Maximum side length, matching the retained decoder ceiling.
pub const MAX_DIMENSION: u32 = 16_384;
/// Maximum image area retained from the 512 MiB decoded-image arena.
pub const MAX_PIXELS: u64 = 16 * 1024 * 1024;
/// Conservative working reservation including floating-point color and metric buffers.
pub const IMAGE_MEMORY_BYTES: u64 = MAX_PIXELS * 256;
/// Maximum combined decoded private metadata.
pub const MAX_METADATA_BYTES: usize = 8 * 1024 * 1024;

/// Packed sample representation. Sixteen-bit samples use native byte order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleLayout {
    /// Eight-bit grayscale.
    Gray8,
    /// Eight-bit grayscale with alpha.
    GrayAlpha8,
    /// Eight-bit RGB.
    Rgb8,
    /// Eight-bit RGBA.
    Rgba8,
    /// Sixteen-bit grayscale.
    Gray16,
    /// Sixteen-bit grayscale with alpha.
    GrayAlpha16,
    /// Sixteen-bit RGB.
    Rgb16,
    /// Sixteen-bit RGBA.
    Rgba16,
}

impl SampleLayout {
    pub(crate) fn descriptor(self) -> PixelDescriptor {
        match self {
            Self::Gray8 => PixelDescriptor::GRAY8_SRGB, Self::GrayAlpha8 => PixelDescriptor::GRAYA8_SRGB,
            Self::Rgb8 => PixelDescriptor::RGB8_SRGB, Self::Rgba8 => PixelDescriptor::RGBA8_SRGB,
            Self::Gray16 => PixelDescriptor::GRAY16_SRGB, Self::GrayAlpha16 => PixelDescriptor::GRAYA16_SRGB,
            Self::Rgb16 => PixelDescriptor::RGB16_SRGB, Self::Rgba16 => PixelDescriptor::RGBA16_SRGB,
        }
    }
}

/// Validated direct encoder control, independent of a perceptual score.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quality(f32);
impl Quality {
    /// Accept a finite value from zero through one hundred, inclusive.
    pub fn new(value: f32) -> Result<Self, Error> {
        if value.is_finite() && (0.0..=100.0).contains(&value) { Ok(Self(value)) }
        else { Err(Error::new(ErrorKind::Invalid, "quality must be finite and in [0, 100]")) }
    }
    /// Direct control value.
    pub fn value(self) -> f32 { self.0 }
}

/// Encoding work level and global candidate budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effort {
    /// At most eight encodes.
    Fast,
    /// At most twenty-four encodes.
    Balanced,
    /// At most sixty-four encodes.
    Best,
}
impl Effort {
    /// Total candidate cap across all eligible formats.
    pub fn candidate_cap(self) -> usize { match self { Self::Fast => 8, Self::Balanced => 24, Self::Best => 64 } }
}

/// Exact-sample encoding or direct lossy encoder control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EncodingMode {
    /// Require exact decoded samples.
    Lossless,
    /// Use direct quality without a metric guarantee.
    Quality(Quality),
}

/// Supported still-image containers, in deterministic tie order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Format {
    /// JPEG.
    Jpeg,
    /// PNG.
    Png,
    /// WebP.
    Webp,
    /// AVIF.
    Avif,
}

impl Format {
    /// Canonical output extension.
    pub fn extension(self) -> &'static str {
        match self { Self::Jpeg => "jpg", Self::Png => "png", Self::Webp => "webp", Self::Avif => "avif" }
    }

    /// Recognize a supported extension.
    pub fn from_extension(extension: &std::ffi::OsStr) -> Option<Self> {
        match extension.to_str()?.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => Some(Self::Jpeg), "png" => Some(Self::Png),
            "webp" => Some(Self::Webp), "avif" => Some(Self::Avif), _ => None,
        }
    }

    /// Detect a container from bytes, independently of its filename.
    pub fn sniff(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") { return Ok(Self::Png); }
        if bytes.starts_with(b"\xff\xd8\xff") { return Ok(Self::Jpeg); }
        if bytes.get(..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") { return Ok(Self::Webp); }
        if bytes.get(4..8) == Some(b"ftyp") && bytes.get(8..32).unwrap_or(&[]).chunks_exact(4).any(|v| v == b"avif" || v == b"avis") {
            return Ok(Self::Avif);
        }
        Err(Error::new(ErrorKind::Unsupported, "unsupported image container"))
    }
}

/// Failure category used by image operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorKind {
    /// Invalid bytes or sample layout.
    Invalid,
    /// Unsupported representation or operation.
    Unsupported,
    /// Mandatory resource ceiling exceeded.
    Limit,
    /// Candidate encoding failed.
    Encode,
    /// Candidate could not satisfy the requested fidelity.
    Score,
}

/// Structured image-operation failure with a retained dependency cause.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
    cause: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>) -> Self { Self { kind, message: message.into(), cause: None } }
    pub(crate) fn caused(kind: ErrorKind, cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self { kind, message: cause.to_string(), cause: Some(Box::new(cause)) }
    }
    /// Stable failure category.
    pub fn kind(&self) -> ErrorKind { self.kind }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.message) }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> { self.cause.as_ref().map(|e| &**e as _) }
}

/// Read at most the mandatory input ceiling, without trusting a length hint.
pub fn read_bounded(reader: impl Read) -> Result<Vec<u8>, Error> {
    let mut data = Vec::new();
    reader.take(MAX_INPUT_BYTES + 1).read_to_end(&mut data).map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    if data.len() as u64 > MAX_INPUT_BYTES { return Err(Error::new(ErrorKind::Limit, "compressed input exceeds 128 MiB")); }
    Ok(data)
}

pub(crate) fn limits() -> zencodec::ResourceLimits {
    zencodec::ResourceLimits::none().with_max_pixels(MAX_PIXELS).with_max_width(MAX_DIMENSION)
        .with_max_height(MAX_DIMENSION).with_max_input_bytes(MAX_INPUT_BYTES).with_max_memory(IMAGE_MEMORY_BYTES)
        .with_max_output(MAX_INPUT_BYTES).with_max_frames(1).with_threading(zencodec::ThreadingPolicy::Sequential)
}

fn check_dimensions(width: u32, height: u32) -> Result<(), Error> {
    if width == 0 || height == 0 { return Err(Error::new(ErrorKind::Invalid, "zero image dimension")); }
    limits().check_dimensions(width, height).map_err(|e| Error::caused(ErrorKind::Limit, e))
}

/// A validated still image. Codec storage and metadata adapters stay private.
pub struct StillImage {
    pub(crate) pixels: PixelBuffer,
    pub(crate) metadata: zencodec::Metadata,
    pub(crate) format: Format,
    pub(crate) info: zencodec::ImageInfo,
    pub(crate) png_metadata: Vec<Vec<u8>>,
}

impl Clone for StillImage {
    fn clone(&self) -> Self {
        Self { pixels: self.pixels.crop_copy(0, 0, self.pixels.width(), self.pixels.height()), metadata: self.metadata.clone(), format: self.format, info: self.info.clone(), png_metadata: self.png_metadata.clone() }
    }
}

impl StillImage {
    /// Construct an untagged SDR sRGB image from packed RGB8 samples.
    pub fn from_rgb8(width: u32, height: u32, pixels: Vec<u8>) -> Result<Self, Error> {
        Self::from_samples(width, height, SampleLayout::Rgb8, pixels)
    }

    /// Construct packed, untagged SDR sRGB samples without implicit conversion.
    pub fn from_samples(width: u32, height: u32, layout: SampleLayout, pixels: Vec<u8>) -> Result<Self, Error> {
        check_dimensions(width, height)?;
        let descriptor = layout.descriptor();
        let len = u64::from(width).checked_mul(u64::from(height)).and_then(|n| n.checked_mul(descriptor.bytes_per_pixel() as u64));
        if len != Some(pixels.len() as u64) { return Err(Error::new(ErrorKind::Invalid, "sample count differs from dimensions")); }
        let pixels = PixelBuffer::from_vec(pixels, width, height, descriptor)
            .map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
        let mut info = zencodec::ImageInfo::new(width, height, zencodec::ImageFormat::Png);
        info.has_alpha = descriptor.has_alpha();
        info.source_color.bit_depth = Some(if descriptor.bytes_per_pixel() == descriptor.channels() { 8 } else { 16 });
        Ok(Self { pixels, metadata: zencodec::Metadata::none(), format: Format::Png, info, png_metadata: vec![] })
    }

    /// Decode actual encoded bytes. Animation and unsupported precision fail.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        limits().check_input_size(bytes.len() as u64).map_err(|e| Error::caused(ErrorKind::Limit, e))?;
        let format = Format::sniff(bytes)?;
        match format {
            Format::Png => {
                let chunks = png::metadata_chunks(bytes)?;
                let mut image = decode(zenpng::PngDecoderConfig::new(), bytes, format)?;
                if chunks.iter().any(|chunk| &chunk[4..8] == b"iCCP") && image.metadata.icc_profile.is_none() {
                    return Err(Error::new(ErrorKind::Unsupported, "PNG ICC profile could not be preserved"));
                }
                if matches!(bytes[25], 0 | 4) && image.bit_depth() == 8 && image.pixels.descriptor().channels() == 4 {
                    let samples = image.samples();
                    if samples.chunks_exact(4).any(|p| p[0] != p[1] || p[1] != p[2]) {
                        return Err(Error::new(ErrorKind::Invalid, "grayscale PNG decoder returned unequal RGB channels"));
                    }
                    let gray_alpha = samples.chunks_exact(4).flat_map(|p| [p[0], p[3]]).collect();
                    let (width, height) = image.dimensions();
                    image.pixels = PixelBuffer::from_vec(gray_alpha, width, height, PixelDescriptor::GRAYA8_SRGB)
                        .map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
                }
                image.png_metadata = chunks;
                Ok(image)
            },
            Format::Jpeg => {
                let mut config = zenjpeg::JpegDecoderConfig::new();
                let header = config.inner().read_info(bytes).map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
                if header.num_components == 1 { config.inner_mut().output_format = Some(zenjpeg::decoder::PixelFormat::Gray); }
                decode(config, bytes, format)
            },
            Format::Webp => decode(zenwebp::zencodec::WebpDecoderConfig::new(), bytes, format),
            Format::Avif => avif::decode(bytes),
        }
    }

    /// Image dimensions in stored orientation.
    pub fn dimensions(&self) -> (u32, u32) { (self.pixels.width(), self.pixels.height()) }
    /// Input container.
    pub fn format(&self) -> Format { self.format }
    /// Packed decoded samples, including hidden RGB and alpha.
    pub fn samples(&self) -> Vec<u8> { self.pixels.copy_to_contiguous_bytes() }
    /// Whether decoded samples contain an alpha channel.
    pub fn has_alpha(&self) -> bool { self.pixels.has_alpha() }
    /// Whether decoded samples are grayscale.
    pub fn is_grayscale(&self) -> bool { self.pixels.is_grayscale() }
    /// Precision of the decoded samples.
    pub fn bit_depth(&self) -> u8 {
        if self.pixels.descriptor().bytes_per_pixel() == self.pixels.descriptor().channels() { 8 } else { 16 }
    }

    /// Encode a candidate without silently discarding alpha or reducing precision.
    pub fn encode(&self, format: Format, mode: EncodingMode, effort: Effort) -> Result<Vec<u8>, Error> {
        match format {
            Format::Png => png::encode(self, mode, effort),
            Format::Jpeg => {
                let EncodingMode::Quality(quality) = mode else { return Err(Error::new(ErrorKind::Unsupported, "JPEG lossless requires original-byte preservation")); };
                if self.bit_depth() != 8 { return Err(Error::new(ErrorKind::Unsupported, "JPEG encoding requires eight-bit samples")); }
                let opaque = self.without_opaque_alpha()?;
                let config = if opaque.is_grayscale() { zenjpeg::encoder::EncoderConfig::grayscale(quality.value()) }
                    else { zenjpeg::encoder::EncoderConfig::ycbcr(quality.value(), zenjpeg::encoder::ChromaSubsampling::Quarter) };
                let preset = match effort {
                    Effort::Fast => zenjpeg::encoder::OptimizationPreset::JpegliBaseline,
                    _ => zenjpeg::encoder::OptimizationPreset::JpegliProgressive,
                };
                let config = config.optimization(preset).progressive(true);
                let mut request = config.request();
                if let Some(icc) = &opaque.metadata.icc_profile { request = request.icc_profile(icc); }
                if let Some(exif) = &opaque.metadata.exif { request = request.exif(zenjpeg::encoder::Exif::raw(exif.to_vec())); }
                if let Some(xmp) = &opaque.metadata.xmp { request = request.xmp(xmp); }
                let layout = if opaque.is_grayscale() { zenjpeg::encoder::PixelLayout::Gray8Srgb }
                    else { zenjpeg::encoder::PixelLayout::Rgb8Srgb };
                let bytes = opaque.pixels.as_slice();
                let output = request.encode_bytes(&bytes.contiguous_bytes(), bytes.width(), bytes.rows(), layout)
                    .map_err(|e| Error::caused(ErrorKind::Encode, e))?;
                limits().check_output_size(output.len() as u64).map_err(|e| Error::caused(ErrorKind::Limit, e))?;
                Ok(output)
            },
            Format::Webp => {
                if self.bit_depth() != 8 { return Err(Error::new(ErrorKind::Unsupported, "WebP encoding requires eight-bit samples")); }
                let config = match mode {
                    EncodingMode::Lossless => zenwebp::zencodec::WebpEncoderConfig::lossless().with_exact(true),
                    EncodingMode::Quality(quality) => zenwebp::zencodec::WebpEncoderConfig::lossy().with_quality(quality.value()),
                }.with_alpha_quality_value(100.0).with_effort_u32(match effort { Effort::Fast => 0, Effort::Balanced => 6, Effort::Best => 10 });
                encode(config, self)
            },
            Format::Avif => avif::encode(self, mode, effort),
        }
    }

    fn without_opaque_alpha(&self) -> Result<Cow<'_ , Self>, Error> {
        if !self.has_alpha() { return Ok(Cow::Borrowed(self)); }
        let channels = self.pixels.descriptor().channels();
        let samples = self.samples();
        if samples.chunks_exact(channels).any(|p| p[channels - 1] != 255) {
            return Err(Error::new(ErrorKind::Unsupported, "JPEG cannot represent nonopaque alpha"));
        }
        let samples = samples.chunks_exact(channels).flat_map(|p| p[..channels - 1].iter().copied()).collect();
        let layout = if self.is_grayscale() { SampleLayout::Gray8 } else { SampleLayout::Rgb8 };
        let (width, height) = self.dimensions();
        let mut image = Self::from_samples(width, height, layout, samples)?;
        image.metadata = self.metadata.clone();
        image.info = self.info.clone();
        Ok(Cow::Owned(image))
    }
}

pub(crate) fn decode<C: DecoderConfig>(config: C, bytes: &[u8], format: Format) -> Result<StillImage, Error> {
    let mut policy = zencodec::decode::DecodePolicy::none();
    policy.strict = Some(true);
    policy.allow_truncated = Some(false);
    let job = config.job().with_limits(limits()).with_policy(policy).with_orientation(zencodec::OrientationHint::Preserve);
    let info = job.probe(bytes).map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    check_dimensions(info.width, info.height)?;
    if info.sequence != zencodec::ImageSequence::Single { return Err(Error::new(ErrorKind::Unsupported, "animation is unsupported")); }
    if format == Format::Jpeg && info.source_color.bit_depth.is_some_and(|depth| depth != 8) {
        return Err(Error::new(ErrorKind::Unsupported, "JPEG decoding requires eight-bit precision"));
    }
    let result = job.decoder(Cow::Borrowed(bytes), &[]).map_err(|e| Error::caused(ErrorKind::Invalid, e))?
        .decode().map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    let mut metadata = result.metadata();
    if let Some(exif) = &metadata.exif
        && let Some(orientation) = zencodec::helpers::parse_exif_orientation(exif) {
        metadata.orientation = orientation;
    }
    let metadata_bytes = [&metadata.icc_profile, &metadata.exif, &metadata.xmp].into_iter()
        .flatten().try_fold(0usize, |n, b| n.checked_add(b.len()));
    if metadata_bytes.is_none_or(|n| n > MAX_METADATA_BYTES) { return Err(Error::new(ErrorKind::Limit, "decoded metadata exceeds 8 MiB")); }
    let mut decoded_info = result.info().clone();
    decoded_info.orientation = metadata.orientation;
    if decoded_info.source_color.bit_depth.is_none() { decoded_info.source_color.bit_depth = info.source_color.bit_depth; }
    if decoded_info.source_color.channel_count.is_none() { decoded_info.source_color.channel_count = info.source_color.channel_count; }
    let info = decoded_info;
    Ok(StillImage { pixels: result.into_buffer(), metadata, format, info, png_metadata: vec![] })
}

pub(crate) fn encode<C: EncoderConfig>(config: C, image: &StillImage) -> Result<Vec<u8>, Error>
where <C::Job as EncodeJob>::Enc: Encoder {
    config.job().with_limits(limits()).with_metadata_policy(image.metadata.clone(), zencodec::MetadataPolicy::PreserveExact).encoder()
        .map_err(|e| Error::caused(ErrorKind::Encode, e))?.encode(image.pixels.as_slice())
        .map(|out| out.into_vec()).map_err(|e| Error::caused(ErrorKind::Encode, e))
}
