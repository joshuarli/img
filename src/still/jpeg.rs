use super::{Error, ErrorKind};

pub(super) struct Structure {
    pub auxiliary: bool,
    pub hdr: bool,
    pub late_metadata: bool,
    pub icc: bool,
    pub exif: bool,
    pub xmp: bool,
    pub orientation: Option<u8>,
    pub end: usize,
    pub precision: u8,
    pub components: u8,
    pub width: u32,
    pub height: u32,
}

enum Position { Headers, Entropy, BetweenScans }

fn invalid() -> Error { Error::new(ErrorKind::Invalid,"invalid or truncated JPEG marker stream") }

pub(super) fn structure(bytes: &[u8]) -> Result<Structure, Error> {
    if !bytes.starts_with(&[255,216]) { return Err(invalid()); }
    let mut result=Structure {auxiliary:false,hdr:false,late_metadata:false,icc:false,exif:false,xmp:false,orientation:None,end:0,precision:0,components:0,width:0,height:0};
    let mut position=Position::Headers;
    let mut at=2;
    loop {
        if matches!(position,Position::Entropy) {
            at+=bytes.get(at..).ok_or_else(invalid)?.iter().position(|&byte|byte==255).ok_or_else(invalid)?;
            let start=at;
            while bytes.get(at)==Some(&255) {at+=1;}
            let marker=*bytes.get(at).ok_or_else(invalid)?;
            if marker==0 || (0xD0..=0xD7).contains(&marker) {at+=1;continue;}
            at=start;
            position=Position::BetweenScans;
        }
        if bytes.get(at)!=Some(&255) {return Err(invalid());}
        while bytes.get(at)==Some(&255) {at+=1;}
        let marker=*bytes.get(at).ok_or_else(invalid)?;at+=1;
        match marker {
            0xD9=>{result.end=at;return Ok(result);},
            0|0xD8=>return Err(invalid()),
            1|0xD0..=0xD7=>continue,
            _=>{},
        }
        let length=bytes.get(at..at+2).ok_or_else(invalid)?;
        let length=u16::from_be_bytes(length.try_into().unwrap()) as usize;
        if length<2 {return Err(invalid());}
        let end=at.checked_add(length).ok_or_else(invalid)?;
        let payload=bytes.get(at+2..end).ok_or_else(invalid)?;
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker,0xC4|0xC8|0xCC) {
            let sof=payload.get(..6).ok_or_else(invalid)?;
            result.precision=sof[0];
            result.height=u16::from_be_bytes([sof[1],sof[2]]) as u32;
            result.width=u16::from_be_bytes([sof[3],sof[4]]) as u32;
            result.components=sof[5];
        }
        if (0xE0..=0xEF).contains(&marker) {
            result.late_metadata|=matches!(position,Position::BetweenScans);
            result.auxiliary|=marker==0xE2 && payload.starts_with(b"MPF\0");
            result.icc|=marker==0xE2 && payload.starts_with(b"ICC_PROFILE\0");
            result.hdr|=marker==0xE2 && payload.starts_with(b"urn:iso:std:iso:ts:21496:-1\0");
            if marker==0xE1 && payload.starts_with(b"Exif\0\0") {
                result.exif=true;
                result.orientation=zencodec::helpers::parse_exif_orientation(&payload[6..]).map(|value|value as u8).or(result.orientation);
            }
            if marker==0xE1 && (payload.starts_with(b"http://ns.adobe.com/xap/1.0/\0") || payload.starts_with(b"http://ns.adobe.com/xmp/extension/\0")) {
                result.xmp=true;
                let namespace=b"http://ns.adobe.com/hdr-gain-map/1.0/";
                result.hdr|=payload.windows(namespace.len()).any(|part|part==namespace);
            }
        }
        at=end;
        if marker==0xDA {position=Position::Entropy;}
    }
}

use super::{Effort, EncodingMode, Format, SampleLayout, StillImage, MAX_METADATA_BYTES};

pub(super) fn decode(bytes: &[u8], stop: &crate::cancellation::Cancellation, structure: &Structure) -> Result<StillImage, super::Error> {
    use zune_core::{bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions};
    if stop.is_cancelled() { return Err(super::Error::new(super::ErrorKind::Invalid, "operation cancelled")); }
    if structure.precision != 8 {
        return Err(super::Error::new(super::ErrorKind::Unsupported, "JPEG decoding requires eight-bit precision"));
    }
    if !matches!(structure.components, 1 | 3) {
        return Err(super::Error::new(super::ErrorKind::Unsupported, "JPEG requires one or three components; CMYK/YCCK conversion is unsupported"));
    }
    let gray = structure.components == 1;
    let layout = if gray { SampleLayout::Gray8 } else { SampleLayout::Rgb8 };
    let options = DecoderOptions::default().jpeg_set_out_colorspace(if gray { ColorSpace::Luma } else { ColorSpace::RGB });
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    decoder.decode_headers().map_err(|e| super::Error::caused(super::ErrorKind::Invalid, e))?;
    let icc = decoder.icc_profile();
    if structure.icc && icc.is_none() {
        return Err(super::Error::new(super::ErrorKind::Unsupported, "JPEG ICC profile could not be preserved"));
    }
    let exif = decoder.info().and_then(|info| info.exif_data.clone());
    let xmp = decoder.info().and_then(|info| info.xmp_data.clone());
    let metadata_bytes = [&icc, &exif, &xmp].into_iter().flatten()
        .try_fold(0usize, |n, b| n.checked_add(b.len()));
    if metadata_bytes.is_none_or(|n| n > MAX_METADATA_BYTES) {
        return Err(super::Error::new(super::ErrorKind::Limit, "decoded metadata exceeds 8 MiB"));
    }
    let samples = decoder.decode().map_err(|e| super::Error::caused(super::ErrorKind::Invalid, e))?;
    let mut image = StillImage::from_samples(structure.width, structure.height, layout, samples)?;
    image.format = Format::Jpeg;
    image.info.format = zencodec::ImageFormat::Jpeg;
    image.info.source_color.bit_depth = Some(8);
    image.info.source_color.channel_count = Some(structure.components);
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

fn xmp_app1(xmp: &[u8]) -> Vec<u8> {
    let mut payload = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
    payload.extend_from_slice(xmp);
    payload
}

pub(super) fn encode(image: &StillImage, mode: super::EncodingMode, effort: Effort, stop: &crate::cancellation::Cancellation) -> Result<Vec<u8>, super::Error> {
    let EncodingMode::Quality(quality) = mode else {
        return Err(super::Error::new(super::ErrorKind::Unsupported, "JPEG lossless requires original-byte preservation"));
    };
    if stop.is_cancelled() { return Err(super::Error::new(super::ErrorKind::Invalid, "operation cancelled")); }
    if image.bit_depth() != 8 {
        return Err(super::Error::new(super::ErrorKind::Unsupported, "JPEG encoding requires eight-bit samples"));
    }
    let opaque = image.without_opaque_alpha()?;
    let samples = opaque.samples();
    let (width, height) = opaque.dimensions();
    let quality = (quality.value().round() as u8).clamp(1, 100);
    let _ = effort;
    let gray = opaque.is_grayscale();
    let limits = super::limits();
    let mut encoder = jpegli_sys::Encoder::new(width, height, if gray { jpegli_sys::Color::Gray } else { jpegli_sys::Color::Rgb }, quality,
        limits.max_memory_bytes.unwrap_or(super::IMAGE_MEMORY_BYTES) as usize,
        limits.max_output_bytes.unwrap_or(super::MAX_INPUT_BYTES) as usize)
        .map_err(|e| super::Error::new(super::ErrorKind::Encode, e))?;
    if let Some(icc) = &opaque.metadata.icc_profile { encoder.icc_profile(icc).map_err(|e| super::Error::new(super::ErrorKind::Encode, e))?; }
    if let Some(exif) = &opaque.metadata.exif {
        let mut payload = b"Exif\0\0".to_vec();
        payload.extend_from_slice(exif.as_ref().strip_prefix(b"Exif\0\0").unwrap_or(exif.as_ref()));
        encoder.app1(&payload).map_err(|e| super::Error::new(super::ErrorKind::Encode, e))?;
    }
    if let Some(xmp) = &opaque.metadata.xmp { encoder.app1(&xmp_app1(xmp)).map_err(|e| super::Error::new(super::ErrorKind::Encode, e))?; }
    let stride = width as usize * if gray { 1 } else { 3 };
    for rows in samples.chunks(stride * 16) {
        if stop.is_cancelled() { return Err(super::Error::new(super::ErrorKind::Invalid, "operation cancelled")); }
        encoder.rows(rows, (rows.len() / stride) as u32).map_err(|e| super::Error::new(super::ErrorKind::Encode, e))?;
    }
    if stop.is_cancelled() { return Err(super::Error::new(super::ErrorKind::Invalid, "operation cancelled")); }
    let bytes = encoder.finish().map_err(|e| super::Error::new(super::ErrorKind::Encode, e))?;
    super::limits().check_output_size(bytes.len() as u64).map_err(|e| super::Error::caused(super::ErrorKind::Limit, e))?;
    Ok(bytes)
}
