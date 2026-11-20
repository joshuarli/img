use super::{Effort, EncodingMode, Error, ErrorKind, Format, SampleLayout, StillImage};
use super::av1::Chroma;
use super::avif_container;

pub(super) fn decode(bytes: &[u8], stop: &crate::cancellation::Cancellation) -> Result<StillImage, Error> {
    let parser = avif_container::read(bytes, stop)?;
    if parser.content_light_level().is_some()||parser.mastering_display().is_some()||parser.gain_map_metadata().is_some() {
        return Err(Error::new(ErrorKind::Unsupported,"AVIF HDR metadata or gain maps require independently validated processing"));
    }
    if parser.pixel_aspect_ratio().is_some_and(|p|p.h_spacing==0||p.v_spacing==0||p.h_spacing!=p.v_spacing) {
        return Err(Error::new(ErrorKind::Unsupported,"AVIF requires square nonzero pixel aspect ratio"));
    }
    if parser.grid_config().is_some() { return Err(Error::new(ErrorKind::Unsupported, "AVIF grid images are unsupported")); }
    if parser.animation_info().is_some() { return Err(Error::new(ErrorKind::Unsupported, "AVIF animation is unsupported")); }
    let config = parser.av1_config().ok_or_else(|| Error::new(ErrorKind::Invalid, "AVIF configuration missing"))?;
    if parser.alpha_metadata().is_some() { return Err(Error::new(ErrorKind::Unsupported, "AVIF alpha decoding is not independently verified with the selected decoder")); }
    if config.monochrome { return Err(Error::new(ErrorKind::Unsupported, "monochrome AVIF decoding is not independently verified")); }
    if config.bit_depth != 8 { return Err(Error::new(ErrorKind::Unsupported, "AVIF decoding requires eight-bit SDR precision")); }
    if parser.rotation().is_some() || parser.mirror().is_some() || parser.clean_aperture().is_some() || parser.premultiplied_alpha() {
        return Err(Error::new(ErrorKind::Unsupported, "AVIF container transforms or premultiplied alpha are unsupported"));
    }
    let header = parser.primary_metadata()?;
    super::check_dimensions(header.max_frame_width.get(), header.max_frame_height.get())?;
    if !header.still_picture { return Err(Error::new(ErrorKind::Unsupported, "AVIF requires a still AV1 bitstream")); }
    let data = parser.primary_data()?;
    if bitstream_hdr(&data)? {return Err(Error::new(ErrorKind::Unsupported,"AVIF HDR metadata requires independently validated processing"));}
    let frame = super::av1::decode_still(&data, stop)?;
    let (width, height) = (frame.width, frame.height);
    let [y, u, v] = frame.planes();
    let (matrix, full_range) = match parser.color_info() {
        Some(avif_container::ColorInformation::Nclx { color_primaries: 1, transfer_characteristics: 13, matrix_coefficients, full_range }) => (matrix_coefficients as u8, full_range),
        Some(avif_container::ColorInformation::IccProfile(_)) | None => (frame.matrix, frame.full_range),
        _ => return Err(Error::new(ErrorKind::Unsupported, "AVIF container requires validated SDR sRGB color signaling")),
    };
    let pixels = if matrix == 0 {
        if frame.chroma != Chroma::I444 || !full_range { return Err(Error::new(ErrorKind::Unsupported, "AVIF identity matrix requires full-range 4:4:4")); }
        let mut pixels = vec![0; width as usize * height as usize * 3];
        for row in 0..height as usize {
            let y_at = row * frame.y_stride as usize;
            let u_at = row * frame.chroma_stride as usize;
            for x in 0..width as usize {
                let at = (row * width as usize + x) * 3;
                pixels[at] = v[u_at + x];
                pixels[at + 1] = y[y_at + x];
                pixels[at + 2] = u[u_at + x];
            }
        }
        pixels
    } else {
        let matrix = match matrix { 1 => yuv::YuvStandardMatrix::Bt709, 6 => yuv::YuvStandardMatrix::Bt601,
            _ => return Err(Error::new(ErrorKind::Unsupported, "AVIF matrix coefficients are unsupported")) };
        let image = yuv::YuvPlanarImage { y_plane: y, y_stride: frame.y_stride,
            u_plane: u, u_stride: frame.chroma_stride, v_plane: v, v_stride: frame.chroma_stride, width, height };
        let range = if full_range { yuv::YuvRange::Full } else { yuv::YuvRange::Limited };
        let mut pixels = vec![0; width as usize * height as usize * 3];
        let result = match frame.chroma {
            Chroma::I420 => yuv::yuv420_to_rgb(&image, &mut pixels, width * 3, range, matrix),
            Chroma::I422 => yuv::yuv422_to_rgb(&image, &mut pixels, width * 3, range, matrix),
            Chroma::I444 => yuv::yuv444_to_rgb(&image, &mut pixels, width * 3, range, matrix),
        };
        result.map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
        pixels
    };
    let mut image = StillImage::from_samples(width, height, SampleLayout::Rgb8, pixels)?;
    image.format = Format::Avif;
    image.info.format = zencodec::ImageFormat::Avif;
    image.metadata.cicp = Some(zencodec::Cicp::new(1, 13, 0, true));
    if let Some(avif_container::ColorInformation::IccProfile(icc)) = parser.color_info() { image.metadata = image.metadata.with_icc(icc.clone()); }
    if let Some(exif) = parser.exif() { image.metadata = image.metadata.with_exif(exif?.into_owned()); }
    if let Some(xmp) = parser.xmp() { image.metadata = image.metadata.with_xmp(xmp?.into_owned()); }
    let total = [&image.metadata.icc_profile, &image.metadata.exif, &image.metadata.xmp].into_iter().flatten()
        .try_fold(0usize, |size, bytes| size.checked_add(bytes.len()));
    if total.is_none_or(|size| size > super::MAX_METADATA_BYTES) { return Err(Error::new(ErrorKind::Limit, "decoded metadata exceeds 8 MiB")); }
    image.info.orientation = image.metadata.orientation;
    image.info.source_color.cicp = image.metadata.cicp;
    if matches!(parser.color_info(),Some(avif_container::ColorInformation::Nclx {..})) {image.info.source_color.color_authority=zencodec::ColorAuthority::Cicp;}
    Ok(image)
}

fn leb(data:&mut &[u8])->Result<usize,Error> {
    let mut value=0u64;
    for shift in (0..56).step_by(7) {
        let (&byte,rest)=data.split_first().ok_or_else(||Error::new(ErrorKind::Invalid,"truncated AV1 OBU length"))?;*data=rest;
        value|=u64::from(byte&127)<<shift;
        if byte&128==0 {return usize::try_from(value).map_err(|_|Error::new(ErrorKind::Limit,"AV1 OBU length overflow"));}
    }
    Err(Error::new(ErrorKind::Invalid,"invalid AV1 OBU length"))
}

pub(super) fn bitstream_hdr(mut data:&[u8])->Result<bool,Error> {
    while !data.is_empty() {
        let header=data[0];data=&data[1..];
        if header&0x81!=0 {return Err(Error::new(ErrorKind::Invalid,"invalid AV1 OBU header"));}
        if header&4!=0 {data=data.get(1..).ok_or_else(||Error::new(ErrorKind::Invalid,"truncated AV1 OBU extension"))?;}
        let size=if header&2!=0 {leb(&mut data)?} else {data.len()};
        let payload=data.get(..size).ok_or_else(||Error::new(ErrorKind::Invalid,"truncated AV1 OBU"))?;data=&data[size..];
        if (header>>3)&15==5 {let mut metadata=payload;if matches!(leb(&mut metadata)?,1|2) {return Ok(true);}}
    }
    Ok(false)
}

/// Images this large encode with four libaom row threads; smaller ones finish faster on one.
const THREADED_PIXELS: u64 = 1 << 18;

pub(super) fn native_quality(quality: super::Quality) -> u8 { libavif_sys::qindex(quality.value()) }

pub(super) fn encode(image: &StillImage, mode: EncodingMode, effort: Effort, stop: &crate::cancellation::Cancellation) -> Result<Vec<u8>, Error> {
    let EncodingMode::Quality(quality) = mode else { return Err(Error::new(ErrorKind::Unsupported, "strict lossless AVIF encoding is unsupported")); };
    if stop.is_cancelled() { return Err(Error::new(ErrorKind::Invalid, "operation cancelled")); }
    if image.bit_depth() != 8 { return Err(Error::new(ErrorKind::Unsupported, "AVIF encoding requires eight-bit SDR samples")); }
    if image.has_alpha() { return Err(Error::new(ErrorKind::Unsupported, "AVIF encoding cannot guarantee exact alpha with the selected encoder")); }
    if image.is_grayscale() { return Err(Error::new(ErrorKind::Unsupported, "native grayscale AVIF encoding is unsupported")); }
    if image.metadata.icc_profile.as_deref().is_some_and(|icc| !zenpixels::icc::is_common_srgb(icc)) {
        return Err(Error::new(ErrorKind::Unsupported, "AVIF encoding cannot embed non-sRGB ICC profiles with the selected encoder"));
    }
    if image.metadata.xmp.is_some() { return Err(Error::new(ErrorKind::Unsupported, "AVIF encoding cannot embed XMP with the selected encoder")); }
    let (width, height) = image.dimensions();
    let speed = match effort { Effort::Fast => 8, Effort::Balanced | Effort::Best => 6 };
    let threads = if u64::from(width) * u64::from(height) >= THREADED_PIXELS { 4 } else { 1 };
    let encoded = libavif_sys::encode(width, height, &image.sample_bytes(), speed, native_quality(quality), threads, image.metadata.exif.as_deref())
        .map_err(|e| Error::new(ErrorKind::Encode, e))?;
    super::limits().check_output_size(encoded.len() as u64).map_err(|e| Error::caused(ErrorKind::Limit, e))?;
    Ok(encoded)
}
