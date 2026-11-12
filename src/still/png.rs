use super::{Effort, EncodingMode, Error, ErrorKind, StillImage, MAX_METADATA_BYTES};

pub(super) fn metadata_chunks(bytes: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
    let mut chunks = Vec::new();
    let mut at = 8usize;
    let mut total = 0usize;
    let mut ended = false;
    while at < bytes.len() {
        let header = bytes.get(at..at + 8).ok_or_else(|| Error::new(ErrorKind::Invalid, "truncated PNG chunk header"))?;
        let len = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
        let end = at.checked_add(len).and_then(|n| n.checked_add(12)).filter(|&n| n <= bytes.len())
            .ok_or_else(|| Error::new(ErrorKind::Invalid, "truncated PNG chunk"))?;
        let chunk = &bytes[at..end];
        let expected = u32::from_be_bytes(chunk[chunk.len() - 4..].try_into().unwrap());
        if crc32fast::hash(&chunk[4..chunk.len() - 4]) != expected { return Err(Error::new(ErrorKind::Invalid, "PNG chunk checksum mismatch")); }
        let kind = &header[4..8];
        if matches!(kind, b"acTL" | b"fcTL" | b"fdAT") { return Err(Error::new(ErrorKind::Unsupported, "PNG animation is unsupported")); }
        if kind[0].is_ascii_lowercase() && kind != b"tRNS" {
            total = total.checked_add(chunk.len()).ok_or_else(|| Error::new(ErrorKind::Limit, "PNG metadata length overflow"))?;
            if total > MAX_METADATA_BYTES { return Err(Error::new(ErrorKind::Limit, "PNG metadata exceeds 8 MiB")); }
            chunks.push(chunk.to_vec());
        }
        at = end;
        if kind == b"IEND" { ended = true; break; }
    }
    if !ended || at != bytes.len() { return Err(Error::new(ErrorKind::Invalid, "missing PNG end or trailing data")); }
    Ok(chunks)
}

pub(super) fn restore_metadata(bytes: &[u8], metadata: &[Vec<u8>]) -> Result<Vec<u8>, Error> {
    if metadata.is_empty() { return Ok(bytes.to_vec()); }
    let mut output = bytes[..33].to_vec();
    for chunk in metadata { output.extend_from_slice(chunk); }
    let mut at = 33;
    while at < bytes.len() {
        let len = u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        let end = at + len + 12;
        let kind = &bytes[at + 4..at + 8];
        if !metadata.iter().any(|chunk| &chunk[4..8] == kind) { output.extend_from_slice(&bytes[at..end]); }
        at = end;
    }
    Ok(output)
}

pub(super) fn encode(image: &StillImage, mode: EncodingMode, effort: Effort, stop: &crate::cancellation::Cancellation) -> Result<Vec<u8>, Error> {
    if image.png_metadata.iter().any(|chunk| {
        let kind = &chunk[4..8];
        matches!(kind, b"bKGD" | b"sBIT") || (kind[3].is_ascii_uppercase()
            && !matches!(kind, b"cHRM" | b"gAMA" | b"iCCP" | b"sRGB" | b"cICP" | b"cLLi" | b"mDCV" | b"eXIf"))
    }) { return Err(Error::new(ErrorKind::Unsupported, "PNG layout-sensitive or unknown unsafe-to-copy metadata requires explicit stripping")); }
    let compression = match effort { Effort::Fast => zenpng::Compression::Fastest, Effort::Balanced => zenpng::Compression::Fast, Effort::Best => zenpng::Compression::Balanced };
    let bytes = match mode {
        EncodingMode::Quality(quality) => {
            if image.bit_depth() != 8 || image.is_grayscale() { return Err(Error::new(ErrorKind::Unsupported, "palette quality requires eight-bit RGB or RGBA")); }
            let channels = image.pixels.descriptor().channels();
            let samples = image.samples();
            let rgba: Vec<_> = samples.chunks_exact(channels).map(|p| rgb::RGBA8::new(p[0], p[1], p[2], if channels == 4 { p[3] } else { 255 })).collect();
            let (width, height) = image.dimensions();
            let pixels = imgref::Img::new(rgba, width as usize, height as usize);
            let config = zenpng::EncodeConfig::default().with_compression(compression);
            let colors = 2 + (quality.value() * 254.0 / 100.0).round() as u16;
            let quantizer = zenpng::ZenquantQuantizer::new().with_max_colors(colors);
            zenpng::encode_indexed(pixels.as_ref(), &config, &quantizer, Some(&image.metadata), stop, stop)
                .map_err(|e| Error::caused(ErrorKind::Encode, e))?
        },
        EncodingMode::Lossless if image.is_grayscale() && image.has_alpha() => {
            use zune_image::traits::EncoderTrait;
            let (width, height) = image.dimensions();
            let samples = image.samples();
            let image = if image.bit_depth() == 8 {
                zune_image::image::Image::from_u8(&samples, width as usize, height as usize, zune_core::colorspace::ColorSpace::LumaA)
            } else {
                let samples: Vec<_> = samples.chunks_exact(2).map(|p| u16::from_ne_bytes([p[0], p[1]])).collect();
                zune_image::image::Image::from_u16(&samples, width as usize, height as usize, zune_core::colorspace::ColorSpace::LumaA)
            };
            let mut encoder = zune_image::codecs::png::PngEncoder::new();
            let mut output = std::io::Cursor::new(Vec::new());
            encoder.encode(&image, &mut output).map_err(|e| Error::new(ErrorKind::Encode, e.to_string()))?;
            output.into_inner()
        },
        EncodingMode::Lossless => super::encode(zenpng::PngEncoderConfig::new().with_compression(compression), image,stop)?,
    };
    let mut metadata = image.png_metadata.clone();
    if image.is_grayscale() && image.has_alpha() {
        for (kind, value) in [(b"eXIf", image.metadata.exif.as_deref()), (b"iTXt", image.metadata.xmp.as_deref())] {
            if let Some(value) = value && !metadata.iter().any(|chunk| &chunk[4..8] == kind) {
                if kind == b"iTXt" { continue; }
                metadata.push(chunk(kind, value));
            }
        }
        if image.metadata.icc_profile.is_some() && !metadata.iter().any(|chunk| &chunk[4..8] == b"iCCP") {
            return Err(Error::new(ErrorKind::Unsupported, "gray-alpha PNG conversion cannot emit an ICC profile"));
        }
    }
    let bytes = restore_metadata(&bytes, &metadata)?;
    super::limits().check_output_size(bytes.len() as u64).map_err(|e| Error::caused(ErrorKind::Encode, e))?;
    Ok(bytes)
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut chunk = (data.len() as u32).to_be_bytes().to_vec();
    chunk.extend_from_slice(kind); chunk.extend_from_slice(data);
    let crc = crc32fast::hash(&chunk[4..]); chunk.extend_from_slice(&crc.to_be_bytes()); chunk
}
