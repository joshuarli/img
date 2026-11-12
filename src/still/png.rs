use super::{Effort, EncodingMode, Error, ErrorKind, StillImage, MAX_METADATA_BYTES};

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
            && !is_color_chunk(kind) && kind != b"eXIf")
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
        EncodingMode::Lossless if image.bit_depth()==16 || image.metadata.icc_profile.is_some() || (image.is_grayscale() && image.has_alpha()) => encode_preserving_layout(image,effort)?,
        EncodingMode::Lossless => super::encode(zenpng::PngEncoderConfig::new().with_compression(compression), image,stop)?,
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

fn encode_preserving_layout(image:&StillImage,effort:Effort)->Result<Vec<u8>,Error> {
    let (width,height)=image.dimensions();let mut info=::png::Info::with_size(width,height);
    info.color_type=match (image.is_grayscale(),image.has_alpha()) {(true,false)=>::png::ColorType::Grayscale,(true,true)=>::png::ColorType::GrayscaleAlpha,(false,false)=>::png::ColorType::Rgb,(false,true)=>::png::ColorType::Rgba};
    info.bit_depth=if image.bit_depth()==16 {::png::BitDepth::Sixteen} else {::png::BitDepth::Eight};
    info.icc_profile=image.metadata.icc_profile.as_deref().map(std::borrow::Cow::Borrowed);
    info.exif_metadata=image.metadata.exif.as_deref().map(std::borrow::Cow::Borrowed);
    let samples=image.samples();let samples=if image.bit_depth()==16 {samples.chunks_exact(2).flat_map(|p|u16::from_ne_bytes([p[0],p[1]]).to_be_bytes()).collect()} else {samples};
    let mut bytes=Vec::new();
    {
        let mut encoder=::png::Encoder::with_info(&mut bytes,info).map_err(|e|Error::caused(ErrorKind::Encode,e))?;
        encoder.set_compression(match effort {Effort::Fast=>::png::Compression::Fast,Effort::Balanced=>::png::Compression::Balanced,Effort::Best=>::png::Compression::High});
        let mut writer=encoder.write_header().map_err(|e|Error::caused(ErrorKind::Encode,e))?;writer.write_image_data(&samples).map_err(|e|Error::caused(ErrorKind::Encode,e))?;writer.finish().map_err(|e|Error::caused(ErrorKind::Encode,e))?;
    }
    Ok(bytes)
}
