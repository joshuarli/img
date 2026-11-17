use super::{Effort, EncodingMode, Error, ErrorKind, Format, SampleLayout, StillImage, MAX_METADATA_BYTES};

pub(super) fn decode(bytes: &[u8], stop: &crate::cancellation::Cancellation) -> Result<StillImage, Error> {
    if stop.is_cancelled() { return Err(Error::new(ErrorKind::Invalid, "operation cancelled")); }
    let mut decoder = image_webp::WebPDecoder::new(std::io::Cursor::new(bytes))
        .map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    if decoder.is_animated() { return Err(Error::new(ErrorKind::Unsupported, "WebP animation is unsupported")); }
    let (width, height) = decoder.dimensions();
    super::check_dimensions(width, height)?;
    decoder.set_memory_limit(super::IMAGE_MEMORY_BYTES as usize);
    let icc = decoder.icc_profile().map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    let exif = decoder.exif_metadata().map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    let xmp = decoder.xmp_metadata().map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    let metadata_bytes = [&icc, &exif, &xmp].into_iter().flatten()
        .try_fold(0usize, |n, b| n.checked_add(b.len()));
    if metadata_bytes.is_none_or(|n| n > MAX_METADATA_BYTES) {
        return Err(Error::new(ErrorKind::Limit, "decoded metadata exceeds 8 MiB"));
    }
    let has_alpha = decoder.has_alpha();
    let layout = if has_alpha { SampleLayout::Rgba8 } else { SampleLayout::Rgb8 };
    let size = decoder.output_buffer_size().ok_or_else(|| Error::new(ErrorKind::Limit, "WebP output size overflow"))?;
    let mut buffer = vec![0; size];
    decoder.read_image(&mut buffer).map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    let mut image = StillImage::from_samples(width, height, layout, buffer)?;
    image.format = Format::Webp;
    image.info.format = zencodec::ImageFormat::WebP;
    image.info.has_alpha = has_alpha;
    image.info.source_color.bit_depth = Some(8);
    let mut metadata = zencodec::Metadata::none();
    if let Some(icc) = icc { metadata = metadata.with_icc(icc); }
    if let Some(orientation) = exif.as_deref().and_then(zencodec::helpers::parse_exif_orientation) {
        metadata.orientation = orientation;
    }
    if let Some(exif) = exif { metadata = metadata.with_exif(exif); }
    if let Some(xmp) = xmp { metadata = metadata.with_xmp(xmp); }
    image.metadata = metadata;
    image.info.orientation = image.metadata.orientation;
    Ok(image)
}

fn rgba(image: &StillImage) -> Vec<u8> {
    let samples = image.samples();
    if image.is_grayscale() {
        if image.has_alpha() {
            samples.chunks_exact(2).flat_map(|px| [px[0], px[0], px[0], px[1]]).collect()
        } else {
            samples.iter().flat_map(|&g| [g, g, g, 255]).collect()
        }
    } else if image.has_alpha() {
        samples
    } else {
        samples.chunks_exact(3).flat_map(|px| [px[0], px[1], px[2], 255]).collect()
    }
}

fn webkit_metadata(image: &StillImage) -> webpkit::Metadata {
    webpkit::Metadata::none()
        .with_icc_profile(image.metadata.icc_profile.as_deref().map(<[u8]>::to_vec))
        .with_exif(image.metadata.exif.as_deref().map(<[u8]>::to_vec))
        .with_xmp(image.metadata.xmp.as_deref().map(<[u8]>::to_vec))
}

// Fast effort skips the Best-tier transform search, which is where the remaining
// lossless byte wins on photographic content live; codec level 1 still runs
// cross-color, optimal parse and meta-Huffman shots. Only the sub-16,384 px small
// tier rides level 6 under the owner's 2026-11-15 extension: the fine tile-bit
// grid for those fixtures costs sub-millisecond planning (transparent.png's
// lossless candidate encodes 200 -> 110 B), and the small synthetics measure
// byte-identical at L6. The photographic mid-band (200-400k px) previously rode
// L6 as well, but measured L1-vs-L6 deltas do not justify ~2x the encode time
// there (coffee +0.82% bytes for +2.4 s; grass byte-identical; gravel +0.09%
// for +2.4 s), so it stays at level 1 like every other band.
const DEEP_LOSSLESS_FAST_SMALL_MAX_PIXELS: u64 = 16_384;

fn fast_lossless_effort(pixels: u64) -> webpkit::Effort {
    if pixels <= DEEP_LOSSLESS_FAST_SMALL_MAX_PIXELS {
        webpkit::Effort::level(6)
    } else {
        webpkit::Effort::level(1)
    }
}

fn webkit_effort(mode: EncodingMode, effort: Effort, pixels: u64) -> webpkit::Effort {
    match (mode, effort) {
        (EncodingMode::Lossless, Effort::Fast) => fast_lossless_effort(pixels),
        (EncodingMode::Lossless, Effort::Balanced) => webpkit::Effort::AUTO,
        (EncodingMode::Lossless, Effort::Best) => webpkit::Effort::level(9),
        (EncodingMode::Quality(_), Effort::Fast) => webpkit::Effort::AUTO,
        (EncodingMode::Quality(_), Effort::Balanced) => webpkit::Effort::AUTO,
        (EncodingMode::Quality(_), Effort::Best) => webpkit::Effort::level(9),
    }
}

pub(super) fn encode(image: &StillImage, mode: EncodingMode, effort: Effort, stop: &crate::cancellation::Cancellation) -> Result<Vec<u8>, Error> {
    let (width, height) = image.dimensions();
    encode_with_effort(image, mode, webkit_effort(mode, effort, u64::from(width) * u64::from(height)), effort == Effort::Fast, stop)
}

pub(crate) fn probe(image: &StillImage, stop: &crate::cancellation::Cancellation) -> Result<Vec<u8>, Error> {
    encode_with_effort(image, EncodingMode::Lossless, webpkit::Effort::level(0), false, stop)
}

fn encode_with_effort(image: &StillImage, mode: EncodingMode, effort: webpkit::Effort, fast_alpha: bool, stop: &crate::cancellation::Cancellation) -> Result<Vec<u8>, Error> {
    if stop.is_cancelled() { return Err(Error::new(ErrorKind::Invalid, "operation cancelled")); }
    if image.bit_depth() != 8 { return Err(Error::new(ErrorKind::Unsupported, "WebP encoding requires eight-bit samples")); }
    let (width, height) = image.dimensions();
    let samples = rgba(image);
    let dims = webpkit::Dimensions::new(width, height)
        .map_err(|_| Error::new(ErrorKind::Limit, "WebP dimensions are outside 1..=16384"))?;
    let reference = webpkit::ImageRef::new(dims, webpkit::PixelLayout::Rgba8, &samples)
        .map_err(|e| Error::new(ErrorKind::Invalid, e.to_string()))?;
    let metadata = webkit_metadata(image);
    let result = match mode {
        EncodingMode::Lossless => webpkit::Encoder::lossless()
            .effort(effort)
            .metadata(metadata)
            .encode_ref(reference),
        EncodingMode::Quality(quality) => {
            let encoder = webpkit::Encoder::lossy()
                .effort(effort)
                .quality((quality.value().round() as u8).clamp(0, 100));
            // Fast effort bounds the lossless ALPH filter search to one estimated
            // predictor: the exhaustive four-filter trial re-encodes the whole
            // alpha plane per filter, which dominates lossy encode time on large
            // transparent inputs. Balanced and Best keep the exhaustive default.
            let encoder = if fast_alpha {
                encoder.tuning(webpkit::LossyTuning::new().with_alpha_filter(webpkit::AlphaFilterMode::Fast))
            } else {
                encoder
            };
            encoder.metadata(metadata).encode_ref(reference)
        },
    };
    let bytes = result.map_err(|e| Error::new(ErrorKind::Encode, e.to_string()))?;
    super::limits().check_output_size(bytes.len() as u64).map_err(|e| Error::caused(ErrorKind::Limit, e))?;
    Ok(bytes)
}
