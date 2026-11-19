use super::{Effort, EncodingMode, Error, ErrorKind, MAX_METADATA_BYTES, SampleLayout, StillImage};

pub(crate) fn is_color_chunk(kind:&[u8])->bool {
    matches!(kind,b"iCCP"|b"cICP"|b"sRGB"|b"gAMA"|b"cHRM"|b"cLLI"|b"mDCV")
}

pub(super) fn metadata_chunks(bytes: &[u8], reject_animation:bool) -> Result<Vec<Vec<u8>>, Error> {
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
        if reject_animation && matches!(kind, b"acTL" | b"fcTL" | b"fdAT") { return Err(Error::new(ErrorKind::Unsupported, "PNG animation is unsupported")); }
        if kind[0].is_ascii_lowercase() && !matches!(kind,b"tRNS"|b"fdAT") {
            total = total.checked_add(chunk.len()).ok_or_else(|| Error::new(ErrorKind::Limit, "PNG metadata length overflow"))?;
            if total > MAX_METADATA_BYTES { return Err(Error::new(ErrorKind::Limit, "PNG metadata exceeds 8 MiB")); }
            if let Some(compressed)=compressed_metadata(kind,&chunk[8..chunk.len()-4])? {
                let remaining=MAX_METADATA_BYTES-total;
                if remaining==0 {return Err(Error::new(ErrorKind::Limit,"decoded PNG metadata exceeds 8 MiB"));}
                let decoded=fdeflate::decompress_to_vec_bounded(compressed,remaining).map_err(|error|match error {
                    fdeflate::BoundedDecompressionError::OutputTooLarge {..}=>Error::new(ErrorKind::Limit,"decoded PNG metadata exceeds 8 MiB"),
                    fdeflate::BoundedDecompressionError::DecompressionError {inner}=>Error::new(ErrorKind::Invalid,format!("invalid compressed PNG metadata: {inner:?}")),
                })?;
                total+=decoded.len();
            }
            chunks.push(chunk.to_vec());
        }
        at = end;
        if kind == b"IEND" { ended = true; break; }
    }
    if !ended || at != bytes.len() { return Err(Error::new(ErrorKind::Invalid, "missing PNG end or trailing data")); }
    Ok(chunks)
}

fn compressed_metadata<'a>(kind:&[u8],data:&'a [u8])->Result<Option<&'a [u8]>,Error> {
    if !matches!(kind,b"iCCP"|b"zTXt"|b"iTXt") {return Ok(None);}
    let end=data.iter().position(|&b|b==0).ok_or_else(||Error::new(ErrorKind::Invalid,"invalid PNG metadata keyword"))?;
    let rest=&data[end+1..];
    if kind==b"iTXt" {
        if rest.len()<2 || rest[0]>1 || rest[1]!=0 {return Err(Error::new(ErrorKind::Invalid,"invalid PNG text compression"));}
        let mut text=&rest[2..];
        for _ in 0..2 {let end=text.iter().position(|&b|b==0).ok_or_else(||Error::new(ErrorKind::Invalid,"invalid PNG international text"))?;text=&text[end+1..];}
        Ok((rest[0]==1).then_some(text))
    } else {
        if rest.first()!=Some(&0) {return Err(Error::new(ErrorKind::Invalid,"invalid PNG metadata compression"));}
        Ok(Some(&rest[1..]))
    }
}

fn chunk_data(chunk: &[u8]) -> Result<&[u8], Error> {
    let data = chunk.get(8..chunk.len().saturating_sub(4)).ok_or_else(|| Error::new(ErrorKind::Invalid, "invalid PNG chunk"))?;
    Ok(data)
}

fn uncompressed_text(kind:&[u8],data:&[u8])->Result<Option<Vec<u8>>,Error> {
    let Some(compressed) = compressed_metadata(kind, data)? else { return Ok(None); };
    let decoded = fdeflate::decompress_to_vec_bounded(compressed, MAX_METADATA_BYTES)
        .map_err(|e| match e {
            fdeflate::BoundedDecompressionError::OutputTooLarge {..} => Error::new(ErrorKind::Limit, "PNG text exceeds 8 MiB"),
            fdeflate::BoundedDecompressionError::DecompressionError {inner} => Error::new(ErrorKind::Invalid, format!("invalid PNG text: {inner:?}")),
        })?;
    Ok(Some(decoded))
}

fn text_payload(kind:&[u8],data:&[u8])->Result<Option<Vec<u8>>,Error> {
    match kind {
        b"iTXt" => {
            let end=data.iter().position(|&b|b==0).ok_or_else(||Error::new(ErrorKind::Invalid,"invalid PNG metadata keyword"))?;
            let rest=&data[end+1..];
            if rest.len()<2 || rest[0]>1 || rest[1]!=0 {return Err(Error::new(ErrorKind::Invalid,"invalid PNG text compression"));}
            let mut text=&rest[2..];
            for _ in 0..2 {let end=text.iter().position(|&b|b==0).ok_or_else(||Error::new(ErrorKind::Invalid,"invalid PNG international text"))?;text=&text[end+1..];}
            if rest[0]==1 {uncompressed_text(b"iTXt",data)} else {Ok(Some(text.to_vec()))}
        },
        _ => Ok(None),
    }
}

fn xmp_from_chunks(chunks: &[Vec<u8>]) -> Result<Option<Vec<u8>>, Error> {
    for chunk in chunks {
        if &chunk[4..8] != b"iTXt" { continue; }
        let data = chunk_data(chunk)?;
        let end = data.iter().position(|&b| b == 0).ok_or_else(|| Error::new(ErrorKind::Invalid, "invalid PNG keyword"))?;
        if &data[..end] == b"XML:com.adobe.xmp" { return text_payload(b"iTXt", data); }
    }
    Ok(None)
}

fn exif_from_chunks(chunks: &[Vec<u8>]) -> Result<Option<Vec<u8>>, Error> {
    for chunk in chunks {
        if &chunk[4..8] == b"eXIf" { return Ok(Some(chunk_data(chunk)?.to_vec())); }
    }
    Ok(None)
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

fn oxipng_options(image: &StillImage, effort: Effort) -> oxipng::Options {
    let mut options = match effort {
        Effort::Fast => oxipng::Options::from_preset(0),
        Effort::Balanced => oxipng::Options { deflater: oxipng::Deflater::Libdeflater { compression: 8 }, ..oxipng::Options::from_preset(0) },
        Effort::Best => oxipng::Options::from_preset(2),
    };
    options.strip = oxipng::StripChunks::None;
    options.interlace = Some(false);
    // Exact output keeps hidden RGB under zero alpha and the declared sample depth.
    options.optimize_alpha = false;
    options.scale_16 = false;
    if image.bit_depth() == 16 { options.bit_depth_reduction = false; }
    // Grayscale output would detach cICP, cHRM, sRGB or ICC signaling written for RGB.
    if image.metadata.icc_profile.is_some() || image.metadata.cicp.is_some()
        || image.png_metadata.iter().any(|chunk| is_color_chunk(&chunk[4..8])) {
        options.grayscale_reduction = false;
    }
    options
}

fn png_encode(image: &StillImage, effort: Effort, samples: Vec<u8>, indexed: Option<(Vec<u8>, Option<Vec<u8>>)>) -> Result<Vec<u8>, Error> {
    use oxipng::{BitDepth, ColorType, RawImage};
    let (width, height) = image.dimensions();
    let (color_type, bit_depth) = match indexed {
        Some((palette, trns)) => {
            let palette = palette.as_chunks::<3>().0.iter().enumerate().map(|(i, c)| {
                let alpha = trns.as_ref().and_then(|t| t.get(i).copied()).unwrap_or(255);
                rgb::RGBA8::new(c[0], c[1], c[2], alpha)
            }).collect();
            (ColorType::Indexed { palette }, BitDepth::Eight)
        },
        None => (
            match (image.is_grayscale(), image.has_alpha()) {
                (true, false) => ColorType::Grayscale { transparent_shade: None },
                (true, true) => ColorType::GrayscaleAlpha,
                (false, false) => ColorType::RGB { transparent_color: None },
                (false, true) => ColorType::RGBA,
            },
            if image.bit_depth() == 16 { BitDepth::Sixteen } else { BitDepth::Eight },
        ),
    };
    let encode_error = |e: oxipng::PngError| Error::caused(ErrorKind::Encode, e);
    let mut raw = RawImage::new(width, height, color_type, bit_depth, samples).map_err(encode_error)?;
    if let Some(icc) = image.metadata.icc_profile.as_deref() {
        let mut data = b"ICC Profile\0\0".to_vec();
        data.extend_from_slice(&fdeflate::compress_to_vec(icc));
        raw.add_png_chunk(*b"iCCP", data);
    }
    if let Some(exif) = image.metadata.exif.as_deref() { raw.add_png_chunk(*b"eXIf", exif.to_vec()); }
    for chunk in &image.png_metadata {
        let kind: [u8; 4] = chunk[4..8].try_into().unwrap();
        if matches!(&kind, b"sRGB" | b"cHRM" | b"gAMA") { raw.add_png_chunk(kind, chunk_data(chunk)?.to_vec()); }
    }
    let mut bytes = raw.create_optimized_png(&oxipng_options(image, effort)).map_err(encode_error)?;
    if let Some(cicp) = &image.metadata.cicp {
        let data = [cicp.color_primaries, cicp.transfer_characteristics, cicp.matrix_coefficients, cicp.full_range as u8];
        let chunk = chunk(b"cICP", &data);
        bytes.splice(33..33, chunk);
    }
    Ok(bytes)
}

fn palette_path(image: &StillImage, quality: super::Quality, effort: Effort) -> Result<Vec<u8>, Error> {
    if image.bit_depth() != 8 || image.is_grayscale() { return Err(Error::new(ErrorKind::Unsupported, "palette quality requires eight-bit RGB or RGBA samples")); }
    let channels = image.pixels.descriptor().channels();
    let samples = image.samples();
    let colors = (2.0 + quality.value() * 254.0 / 100.0).round().clamp(2.0, 256.0) as usize;
    let (width, height) = image.dimensions();
    if channels == 4 && samples.chunks_exact(4).any(|p| p[3] != 255) {
        let qimage = quantizr::Image::new(&samples, width as usize, height as usize)
            .map_err(|_| Error::new(ErrorKind::Invalid, "palette quantization input rejected"))?;
        let mut options = quantizr::Options::default();
        options.set_max_colors(colors as i32)
            .map_err(|_| Error::new(ErrorKind::Invalid, "palette color count is outside 2..=256"))?;
        let mut quantized = quantizr::QuantizeResult::quantize(&qimage, &options);
        quantized.set_dithering_level(1.0)
            .map_err(|_| Error::new(ErrorKind::Invalid, "palette dithering level is outside 0.0..=1.0"))?;
        let mut indices = vec![0u8; samples.len() / 4];
        quantized.remap_image(&qimage, &mut indices)
            .map_err(|_| Error::new(ErrorKind::Invalid, "palette remapping failed"))?;
        let palette = quantized.get_palette();
        let entries = &palette.entries[..palette.count as usize];
        let palette_bytes: Vec<u8> = entries.iter().flat_map(|c| [c.r, c.g, c.b]).collect();
        let alpha_bytes: Vec<u8> = entries.iter().map(|c| c.a).collect();
        let trns = alpha_bytes.iter().any(|&a| a != 255).then_some(alpha_bytes);
        return png_encode(image, effort, indices, Some((palette_bytes, trns)));
    }
    let size = quantette::PaletteSize::from_usize_clamped(colors);
    let rgb: Vec<quantette::deps::palette::Srgb<u8>> = samples.chunks_exact(channels).map(|p| quantette::deps::palette::Srgb::new(p[0], p[1], p[2])).collect();
    let reference = quantette::ImageRef::new(width, height, &rgb)
        .map_err(|_| Error::new(ErrorKind::Invalid, "palette quantization input rejected"))?;
    let quantized = quantette::Pipeline::new()
        .palette_size(size)
        .ditherer(Some(quantette::dither::FloydSteinberg::new()))
        .input_image(reference)
        .output_srgb8_indexed_image();
    let palette_bytes: Vec<u8> = quantized.palette().iter().flat_map(|c| [c.red, c.green, c.blue]).collect();
    png_encode(image, effort, quantized.indices().to_vec(), Some((palette_bytes, None)))
}

pub(super) fn decode(bytes: &[u8], stop: &crate::cancellation::Cancellation) -> Result<StillImage, Error> {
    use ::png::{BitDepth, ColorType, Transformations};
    if stop.is_cancelled() { return Err(Error::new(ErrorKind::Invalid, "operation cancelled")); }
    let chunks = metadata_chunks(bytes, true)?;
    let mut decoder = ::png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_limits(::png::Limits { bytes: super::IMAGE_MEMORY_BYTES as usize });
    decoder.set_transformations(Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    let info = reader.info();
    let (width, height) = info.size();
    super::check_dimensions(width, height)?;
    let buffer_size = reader.output_buffer_size().ok_or_else(|| Error::new(ErrorKind::Limit, "PNG output size overflow"))?;
    let mut buffer = vec![0; buffer_size];
    let output = reader.next_frame(&mut buffer).map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    buffer.truncate(output.line_size.checked_mul(height as usize).ok_or_else(|| Error::new(ErrorKind::Limit, "PNG output size overflow"))?);
    #[cfg(target_endian = "little")]
    if matches!(output.bit_depth, BitDepth::Sixteen) {
        for pair in buffer.chunks_exact_mut(2) { pair.swap(0, 1); }
    }
    let layout = match (output.color_type, output.bit_depth) {
        (ColorType::Grayscale, BitDepth::Eight) => SampleLayout::Gray8,
        (ColorType::Grayscale, BitDepth::Sixteen) => SampleLayout::Gray16,
        (ColorType::GrayscaleAlpha, BitDepth::Eight) => SampleLayout::GrayAlpha8,
        (ColorType::GrayscaleAlpha, BitDepth::Sixteen) => SampleLayout::GrayAlpha16,
        (ColorType::Rgb, BitDepth::Eight) => SampleLayout::Rgb8,
        (ColorType::Rgb, BitDepth::Sixteen) => SampleLayout::Rgb16,
        (ColorType::Rgba, BitDepth::Eight) => SampleLayout::Rgba8,
        (ColorType::Rgba, BitDepth::Sixteen) => SampleLayout::Rgba16,
        _ => return Err(Error::new(ErrorKind::Invalid, "PNG decoder produced an unsupported sample layout")),
    };
    let mut image = StillImage::from_samples(width, height, layout, buffer)?;
    image.format = super::Format::Png;
    image.info.format = zencodec::ImageFormat::Png;
    image.info.source_color.bit_depth = Some(image.bit_depth());
    image.info.source_color.channel_count = Some(image.pixels.descriptor().channels() as u8);
    image.png_metadata = chunks;
    if let Some([primaries, transfer, matrix, full_range]) = image.png_metadata.iter().find_map(|chunk| {
        if &chunk[4..8] != b"cICP" { return None; }
        let data = &chunk[8..chunk.len().saturating_sub(4)];
        (data.len() == 4).then_some([data[0], data[1], data[2], data[3]])
    }) {
        image.metadata.cicp = Some(zencodec::Cicp::new(primaries, transfer, matrix, full_range != 0));
        image.info.source_color.color_authority = zencodec::ColorAuthority::Cicp;
    }
    let xmp = xmp_from_chunks(&image.png_metadata)?;
    let exif = exif_from_chunks(&image.png_metadata)?;
    let mut metadata = zencodec::Metadata::none();
    if let Some(icc) = reader.info().icc_profile.as_ref() { metadata = metadata.with_icc(icc.as_ref().to_vec()); }
    else if image.png_metadata.iter().any(|chunk| &chunk[4..8] == b"iCCP") {
        return Err(Error::new(ErrorKind::Unsupported, "PNG ICC profile could not be preserved"));
    }
    if let Some(orientation) = exif.as_deref().and_then(zencodec::helpers::parse_exif_orientation) { metadata.orientation = orientation; }
    if let Some(exif) = exif { metadata = metadata.with_exif(exif); }
    if let Some(xmp) = xmp { metadata = metadata.with_xmp(xmp); }
    image.metadata = metadata;
    image.info.orientation = image.metadata.orientation;
    Ok(image)
}

pub(super) fn encode(image: &StillImage, mode: EncodingMode, effort: Effort, stop: &crate::cancellation::Cancellation) -> Result<Vec<u8>, Error> {
    if image.png_metadata.iter().any(|chunk| {
        let kind = &chunk[4..8];
        matches!(kind, b"bKGD" | b"sBIT") || (kind[3].is_ascii_uppercase()
            && !is_color_chunk(kind) && kind != b"eXIf")
    }) { return Err(Error::new(ErrorKind::Unsupported, "PNG layout-sensitive or unknown unsafe-to-copy metadata requires explicit stripping")); }
    if stop.is_cancelled() { return Err(Error::new(ErrorKind::Invalid, "operation cancelled")); }
    let bytes = match mode {
        EncodingMode::Quality(quality) => palette_path(image, quality, effort)?,
        EncodingMode::Lossless => {
            let samples = if image.bit_depth() == 16 {
                image.sample_bytes().as_chunks::<2>().0.iter().flat_map(|p| u16::from_ne_bytes(*p).to_be_bytes()).collect()
            } else { image.samples() };
            png_encode(image, effort, samples, None)?
        },
    };
    let mut metadata = image.png_metadata.clone();
    if let Some(xmp)=image.metadata.xmp.as_deref() && !metadata.iter().any(|chunk|&chunk[4..8]==b"iTXt") {
        if std::str::from_utf8(xmp).is_err()||xmp.contains(&0) {return Err(Error::new(ErrorKind::Unsupported,"XMP cannot be represented as PNG UTF-8 text"));}
        let mut data=b"XML:com.adobe.xmp\0\0\0\0\0".to_vec();data.extend_from_slice(xmp);metadata.push(chunk(b"iTXt",&data));
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
