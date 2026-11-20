//! Bounded still-image decoding and encoding.

use std::{borrow::Cow, fmt, io::Read};
use zenpixels::{PixelBuffer, PixelDescriptor};

pub(crate) mod png;
mod jpeg;
mod avif;
mod avif_container;
mod av1;
pub(crate) mod webp;
pub(crate) mod color;
pub use color::{PreparedImage, SamplePreservation};
pub(crate) use color::PreparedPlanes;

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
    /// Fast runs nine format candidates plus one reserve (ten) under the owner-granted cap,
    /// covering the transparent-t80 refinement and the lossless-fallback Fast pool.
    pub fn candidate_cap(self) -> usize { match self { Self::Fast => 10, Self::Balanced => 24, Self::Best => 64 } }
}

/// Exact-sample encoding or direct lossy encoder control.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EncodingMode {
    /// Require exact decoded samples.
    Lossless,
    /// Use direct quality without a metric guarantee.
    Quality(Quality),
}

/// JPEG chroma sampling for color output; grayscale output has no chroma.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JpegSampling {
    /// Full-resolution chroma.
    Yuv444,
    /// Chroma halved in both directions.
    Yuv420,
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
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> { Self::decode_cancellable(bytes,&crate::cancellation::Cancellation::default()) }

    /// Decode with cooperative codec cancellation where supported.
    pub fn decode_cancellable(bytes: &[u8], stop: &crate::cancellation::Cancellation) -> Result<Self, Error> {
        limits().check_input_size(bytes.len() as u64).map_err(|e| Error::caused(ErrorKind::Limit, e))?;
        let format = Format::sniff(bytes)?;
        match format {
            Format::Png => png::decode(bytes, stop),
            Format::Jpeg => {
                let structure=jpeg::structure(bytes)?;
                if structure.auxiliary || structure.hdr {return Err(Error::new(ErrorKind::Unsupported,"JPEG auxiliary images and HDR gain maps are unsupported"));}
                if structure.end!=bytes.len() {return Err(Error::new(ErrorKind::Unsupported,"JPEG trailing payloads are unsupported"));}
                if structure.late_metadata {return Err(Error::new(ErrorKind::Unsupported,"JPEG application metadata after scan start is unsupported"));}
                jpeg::decode(bytes, stop, &structure)
            },
            Format::Webp => webp::decode(bytes, stop),
            Format::Avif => avif::decode(bytes,stop),
        }
    }

    /// Image dimensions in stored orientation.
    pub fn dimensions(&self) -> (u32, u32) { (self.pixels.width(), self.pixels.height()) }
    /// Input container.
    pub fn format(&self) -> Format { self.format }
    /// Packed decoded samples, including hidden RGB and alpha.
    pub fn samples(&self) -> Vec<u8> { self.pixels.copy_to_contiguous_bytes() }
    pub(crate) fn sample_bytes(&self) -> Cow<'_, [u8]> { self.pixels.as_contiguous_bytes().map_or_else(|| Cow::Owned(self.samples()), Cow::Borrowed) }
    /// Whether decoded samples contain an alpha channel.
    pub fn has_alpha(&self) -> bool { self.pixels.has_alpha() }
    /// Whether decoded samples are grayscale.
    pub fn is_grayscale(&self) -> bool { self.pixels.is_grayscale() }
    /// Precision of the decoded samples.
    pub fn bit_depth(&self) -> u8 {
        if self.pixels.descriptor().bytes_per_pixel() == self.pixels.descriptor().channels() { 8 } else { 16 }
    }

    /// Encode a candidate without silently discarding alpha or reducing precision.
    pub fn encode(&self, format: Format, mode: EncodingMode, effort: Effort) -> Result<Vec<u8>, Error> { self.encode_cancellable(format,mode,effort,&crate::cancellation::Cancellation::default()) }

    pub(crate) fn encode_jpeg_cancellable(&self, quality: Quality, sampling: JpegSampling, effort: Effort, stop: &crate::cancellation::Cancellation) -> Result<Vec<u8>, Error> { jpeg::encode_sampled(self, quality, sampling, effort, stop) }

    /// Encode with cooperative codec cancellation where supported.
    pub fn encode_cancellable(&self, format: Format, mode: EncodingMode, effort: Effort, stop: &crate::cancellation::Cancellation) -> Result<Vec<u8>, Error> {
        match format {
            Format::Png => png::encode(self, mode, effort,stop),
            Format::Jpeg => jpeg::encode(self, mode, effort,stop),
            Format::Webp => webp::encode(self, mode, effort,stop),
            Format::Avif => avif::encode(self, mode, effort,stop),
        }
    }

    fn without_opaque_alpha(&self) -> Result<Cow<'_ , Self>, Error> {
        if !self.has_alpha() { return Ok(Cow::Borrowed(self)); }
        let channels = self.pixels.descriptor().channels();
        let samples = self.sample_bytes();
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

/// Header details available without decoding pixel planes.
#[derive(Clone, Debug)]
pub struct Description {
    /// Detected container.
    pub format: Format,
    /// Stored width.
    pub width: u32,
    /// Stored height.
    pub height: u32,
    /// Declared precision when available.
    pub bit_depth: Option<u8>,
    /// Presence of alpha.
    pub alpha: bool,
    /// Declared grayscale when available.
    pub grayscale: Option<bool>,
    /// Embedded ICC profile.
    pub icc: bool,
    /// CICP primaries, transfer, matrix and full-range flag.
    pub cicp: Option<[u16;4]>,
    /// EXIF orientation, one through eight.
    pub orientation: u8,
    /// Animation or multiple-image sequence.
    pub animation: bool,
    /// Container declares a grid of coded tiles.
    pub grid: bool,
    /// Counter-clockwise container rotation in degrees, separately from EXIF.
    pub rotation_ccw_degrees: Option<u16>,
    /// Container mirror axis: zero flips left/right, one flips top/bottom.
    pub mirror_axis: Option<u8>,
    /// Container pixel aspect ratio, horizontal and vertical spacing.
    pub pixel_aspect_ratio: Option<[u32;2]>,
    /// Explicit HDR signaling, mastering/luminance or gain-map metadata.
    pub hdr_metadata: bool,
    /// Embedded EXIF.
    pub exif: bool,
    /// Embedded XMP.
    pub xmp: bool,
}
/// Inspect bounded encoded headers, including representations refused for pixel processing.
pub fn describe(bytes: &[u8]) -> Result<Description, Error> {
    if bytes.len() as u64 > MAX_INPUT_BYTES { return Err(Error::new(ErrorKind::Limit,"compressed input exceeds 128 MiB")); }
    let format=Format::sniff(bytes)?;
    match format {
        Format::Png => describe_png(bytes),
        Format::Jpeg => {
            let structure=jpeg::structure(bytes)?;
            Ok(Description {format,width:structure.width,height:structure.height,bit_depth:(structure.precision>0).then_some(structure.precision),alpha:false,grayscale:Some(structure.components==1),icc:structure.icc,cicp:None,orientation:structure.orientation.unwrap_or(1),animation:structure.auxiliary,grid:false,rotation_ccw_degrees:None,mirror_axis:None,pixel_aspect_ratio:None,hdr_metadata:structure.hdr,exif:structure.exif,xmp:structure.xmp})
        },
        Format::Webp => {
            let mut decoder=image_webp::WebPDecoder::new(std::io::Cursor::new(bytes)).map_err(|e|Error::caused(ErrorKind::Invalid,e))?;
            let (width,height)=decoder.dimensions();
            check_dimensions(width,height)?;
            let icc=decoder.icc_profile().map_err(|e|Error::caused(ErrorKind::Invalid,e))?;
            let exif=decoder.exif_metadata().map_err(|e|Error::caused(ErrorKind::Invalid,e))?;
            let xmp=decoder.xmp_metadata().map_err(|e|Error::caused(ErrorKind::Invalid,e))?;
            let orientation=exif.as_deref().and_then(zencodec::helpers::parse_exif_orientation).map(|o|o as u8).unwrap_or(1);
            Ok(Description {format,width,height,bit_depth:Some(8),alpha:decoder.has_alpha(),grayscale:None,icc:icc.is_some(),cicp:None,orientation,animation:decoder.is_animated(),grid:false,rotation_ccw_degrees:None,mirror_axis:None,pixel_aspect_ratio:None,hdr_metadata:false,exif:exif.is_some(),xmp:xmp.is_some()})
        },
        Format::Avif => {
            let parser=avif_container::read(bytes,&crate::cancellation::Cancellation::default())?;
            let (width,height)=if let Some(grid)=parser.grid_config() {
                if grid.output_width==0||grid.output_height==0 {return Err(Error::new(ErrorKind::Unsupported,"grid inspection requires declared canvas dimensions"));}
                (grid.output_width,grid.output_height)
            } else {let header=parser.primary_metadata()?;(header.max_frame_width.get(),header.max_frame_height.get())};
            let exif=parser.exif().transpose()?;
            let cicp=match parser.color_info() { Some(avif_container::ColorInformation::Nclx {color_primaries,transfer_characteristics,matrix_coefficients,full_range})=>Some([color_primaries,transfer_characteristics,matrix_coefficients,full_range as u16]), _=>None };
            let bitstream_hdr=if parser.grid_config().is_none() {avif::bitstream_hdr(&parser.primary_data()?)?} else {false};
            Ok(Description {format,width,height,bit_depth:parser.av1_config().map(|c|c.bit_depth),alpha:parser.alpha_metadata().is_some(),grayscale:parser.av1_config().map(|c|c.monochrome),icc:matches!(parser.color_info(),Some(avif_container::ColorInformation::IccProfile(_))),cicp,orientation:exif.as_deref().and_then(zencodec::helpers::parse_exif_orientation).map(|o|o as u8).unwrap_or(1),animation:parser.animation_info().is_some(),grid:parser.grid_config().is_some(),rotation_ccw_degrees:parser.rotation().map(|r|r.angle),mirror_axis:parser.mirror().map(|m|m.axis),pixel_aspect_ratio:parser.pixel_aspect_ratio().map(|p|[p.h_spacing,p.v_spacing]),hdr_metadata:parser.content_light_level().is_some()||parser.mastering_display().is_some()||parser.gain_map_metadata().is_some()||bitstream_hdr||cicp.is_some_and(|c|matches!(c[1],16|18)),exif:exif.is_some(),xmp:parser.xmp().is_some()})
        },
    }
}

fn describe_png(bytes: &[u8]) -> Result<Description, Error> {
    let chunks = png::metadata_chunks(bytes, false)?;
    let header = bytes.get(16..29).ok_or_else(|| Error::new(ErrorKind::Invalid, "truncated PNG header"))?;
    if bytes.get(12..16) != Some(b"IHDR") { return Err(Error::new(ErrorKind::Invalid, "missing PNG IHDR")); }
    if bytes.get(8..12) != Some(b"\0\0\0\r") { return Err(Error::new(ErrorKind::Invalid, "PNG IHDR length is not thirteen")); }
    let width = u32::from_be_bytes(header[..4].try_into().unwrap());
    let height = u32::from_be_bytes(header[4..8].try_into().unwrap());
    check_dimensions(width, height)?;
    let bit_depth = header[8];
    let color_type = header[9];
    let alpha = matches!(color_type, 4 | 6) || chunks.iter().any(|chunk| &chunk[4..8] == b"tRNS");
    let grayscale = matches!(color_type, 0 | 4);
    let icc = chunks.iter().any(|chunk| &chunk[4..8] == b"iCCP");
    let cicp = chunks.iter().find_map(|chunk| {
        if &chunk[4..8] != b"cICP" { return None; }
        let data = &chunk[8..chunk.len() - 4];
        (data.len() == 4).then_some([u16::from(data[0]), u16::from(data[1]), u16::from(data[2]), u16::from(data[3])])
    });
    let exif_chunk = chunks.iter().find(|chunk| &chunk[4..8] == b"eXIf");
    let exif = exif_chunk.is_some();
    let xmp = chunks.iter().any(|chunk| {
        &chunk[4..8] == b"iTXt" && chunk[8..].starts_with(b"XML:com.adobe.xmp\0")
    });
    let orientation = exif_chunk.and_then(|chunk| zencodec::helpers::parse_exif_orientation(&chunk[8..chunk.len() - 4])).map(|o| o as u8).unwrap_or(1);
    let animation = chunks.iter().any(|chunk| &chunk[4..8] == b"acTL");
    Ok(Description { format: Format::Png, width, height, bit_depth: (bit_depth > 0).then_some(bit_depth), alpha, grayscale: Some(grayscale), icc, cicp, orientation, animation, grid: false, rotation_ccw_degrees: None, mirror_axis: None, pixel_aspect_ratio: None, hdr_metadata: false, exif, xmp })
}
