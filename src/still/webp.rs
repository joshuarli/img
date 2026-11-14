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

fn webkit_effort(effort: Effort) -> webpkit::Effort {
    match effort { Effort::Fast => webpkit::Effort::level(1), Effort::Balanced => webpkit::Effort::AUTO, Effort::Best => webpkit::Effort::level(9) }
}

pub(super) fn encode(image: &StillImage, mode: EncodingMode, effort: Effort, stop: &crate::cancellation::Cancellation) -> Result<Vec<u8>, Error> {
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
            .effort(webkit_effort(effort))
            .metadata(metadata)
            .encode_ref(reference),
        EncodingMode::Quality(quality) => webpkit::Encoder::lossy()
            .effort(webkit_effort(effort))
            .quality((quality.value().round() as u8).clamp(0, 100))
            .metadata(metadata)
            .encode_ref(reference),
    };
    let bytes = result.map_err(|e| Error::new(ErrorKind::Encode, e.to_string()))?;
    super::limits().check_output_size(bytes.len() as u64).map_err(|e| Error::caused(ErrorKind::Limit, e))?;
    Ok(bytes)
}
