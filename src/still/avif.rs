use super::{Effort, EncodingMode, Error, ErrorKind, Format, SampleLayout, StillImage};
use rav1d_safe::src::managed::{ColorPrimaries, ColorRange, Decoder, Frame, PixelLayout, Planes, Settings, TransferCharacteristics};

pub(super) fn decode(bytes: &[u8],stop: &crate::cancellation::Cancellation) -> Result<StillImage, Error> {
    let config = zenavif_parse::DecodeConfig::default().with_peak_memory_limit(super::IMAGE_MEMORY_BYTES);
    let parser = zenavif_parse::AvifParser::from_bytes_with_config(bytes, &config, stop)
        .map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
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
    let header = parser.primary_metadata().map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    super::check_dimensions(header.max_frame_width.get(), header.max_frame_height.get())?;
    if !header.still_picture { return Err(Error::new(ErrorKind::Unsupported, "AVIF requires a still AV1 bitstream")); }
    let data = parser.primary_data().map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    let primary = frame(&data)?;
    if primary.content_light().is_some() || primary.mastering_display().is_some() {
        return Err(Error::new(ErrorKind::Unsupported,"AVIF HDR metadata requires independently validated processing"));
    }
    if primary.pixel_layout() == PixelLayout::I400 { return Err(Error::new(ErrorKind::Unsupported, "monochrome AVIF decoding is not independently verified")); }
    let (width, height) = (primary.width(), primary.height());
    super::check_dimensions(width, height)?;
    let color = primary.color_info();
    if color.primaries != ColorPrimaries::BT709 || color.transfer_characteristics != TransferCharacteristics::SRGB {
        return Err(Error::new(ErrorKind::Unsupported, "AVIF bitstream requires validated SDR sRGB color signaling"));
    }
    let (matrix, full_range) = match parser.color_info() {
        Some(zenavif_parse::ColorInformation::Nclx { color_primaries: 1, transfer_characteristics: 13, matrix_coefficients, full_range }) => (*matrix_coefficients as u8, *full_range),
        Some(zenavif_parse::ColorInformation::IccProfile(_)) | None => (color.matrix_coefficients as u8, color.color_range == ColorRange::Full),
        _ => return Err(Error::new(ErrorKind::Unsupported, "AVIF container requires validated SDR sRGB color signaling")),
    };
    let Planes::Depth8(planes) = primary.planes() else { return Err(Error::new(ErrorKind::Unsupported, "AVIF decoding requires eight-bit precision")); };
    let y = planes.y();
    let (pixels, layout) = if primary.pixel_layout() == PixelLayout::I400 {
        if !full_range { return Err(Error::new(ErrorKind::Unsupported, "limited-range monochrome AVIF is unsupported")); }
        ((0..height as usize).flat_map(|row| y.row(row)[..width as usize].iter().copied()).collect(), SampleLayout::Gray8)
    } else {
        let u = planes.u().ok_or_else(|| Error::new(ErrorKind::Invalid, "missing AVIF chroma"))?;
        let v = planes.v().ok_or_else(|| Error::new(ErrorKind::Invalid, "missing AVIF chroma"))?;
        let mut pixels = vec![0; width as usize * height as usize * 3];
        if matrix == 0 {
            if primary.pixel_layout() != PixelLayout::I444 || !full_range { return Err(Error::new(ErrorKind::Unsupported, "AVIF identity matrix requires full-range 4:4:4")); }
            for row in 0..height as usize { for x in 0..width as usize {
                let at = (row * width as usize + x) * 3;
                pixels[at..at + 3].copy_from_slice(&[v.pixel(x, row), y.pixel(x, row), u.pixel(x, row)]);
            }}
        } else {
            let matrix = match matrix { 1 => yuv::YuvStandardMatrix::Bt709, 6 => yuv::YuvStandardMatrix::Bt601,
                _ => return Err(Error::new(ErrorKind::Unsupported, "AVIF matrix coefficients are unsupported")) };
            let image = yuv::YuvPlanarImage { y_plane: y.as_slice(), y_stride: y.stride() as u32,
                u_plane: u.as_slice(), u_stride: u.stride() as u32, v_plane: v.as_slice(), v_stride: v.stride() as u32, width, height };
            let range = if full_range { yuv::YuvRange::Full } else { yuv::YuvRange::Limited };
            let result = match primary.pixel_layout() {
                PixelLayout::I420 => yuv::yuv420_to_rgb(&image, &mut pixels, width * 3, range, matrix),
                PixelLayout::I422 => yuv::yuv422_to_rgb(&image, &mut pixels, width * 3, range, matrix),
                PixelLayout::I444 => yuv::yuv444_to_rgb(&image, &mut pixels, width * 3, range, matrix),
                PixelLayout::I400 => unreachable!(),
            };
            result.map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
        }
        (pixels, SampleLayout::Rgb8)
    };
    let mut image = StillImage::from_samples(width, height, layout, pixels)?;
    image.format = Format::Avif;
    image.info.format = zencodec::ImageFormat::Avif;
    image.metadata.cicp = Some(zencodec::Cicp::new(1, 13, 0, true));
    if let Some(zenavif_parse::ColorInformation::IccProfile(icc)) = parser.color_info() { image.metadata = image.metadata.with_icc(icc.clone()); }
    if let Some(exif) = parser.exif() { image.metadata = image.metadata.with_exif(exif.map_err(|e| Error::caused(ErrorKind::Invalid, e))?.into_owned()); }
    if let Some(xmp) = parser.xmp() { image.metadata = image.metadata.with_xmp(xmp.map_err(|e| Error::caused(ErrorKind::Invalid, e))?.into_owned()); }
    let total = [&image.metadata.icc_profile, &image.metadata.exif, &image.metadata.xmp].into_iter().flatten()
        .try_fold(0usize, |size, bytes| size.checked_add(bytes.len()));
    if total.is_none_or(|size| size > super::MAX_METADATA_BYTES) { return Err(Error::new(ErrorKind::Limit, "decoded metadata exceeds 8 MiB")); }
    image.info.orientation = image.metadata.orientation;
    image.info.source_color.cicp = image.metadata.cicp;
    if matches!(parser.color_info(),Some(zenavif_parse::ColorInformation::Nclx {..})) {image.info.source_color.color_authority=zencodec::ColorAuthority::Cicp;}
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

fn frame(data: &[u8]) -> Result<Frame, Error> {
    let mut settings = Settings::default();
    settings.threads = 1; settings.apply_grain = true; settings.frame_size_limit = super::MAX_PIXELS as u32;
    let mut decoder = Decoder::with_settings(settings).map_err(|e| Error::caused(ErrorKind::Invalid, e))?;
    let first=decoder.decode(data).map_err(|e|Error::caused(ErrorKind::Invalid,e))?;
    let first=match first {Some(frame)=>Some(frame),None=>decoder.get_frame().map_err(|e|Error::caused(ErrorKind::Invalid,e))?};
    if decoder.get_frame().map_err(|e|Error::caused(ErrorKind::Invalid,e))?.is_some() {
        return Err(Error::new(ErrorKind::Unsupported,"AVIF requires exactly one displayed frame"));
    }
    first.ok_or_else(||Error::new(ErrorKind::Invalid,"AVIF bitstream contains no displayed frame"))
}

pub(super) fn encode(image: &StillImage, mode: EncodingMode, effort: Effort, stop: &crate::cancellation::Cancellation) -> Result<Vec<u8>, Error> {
    let EncodingMode::Quality(quality) = mode else { return Err(Error::new(ErrorKind::Unsupported, "strict lossless AVIF encoding is unsupported")); };
    if image.bit_depth() != 8 { return Err(Error::new(ErrorKind::Unsupported, "AVIF encoding requires eight-bit SDR samples")); }
    if image.has_alpha() { return Err(Error::new(ErrorKind::Unsupported, "AVIF encoding cannot guarantee exact alpha with the selected encoder")); }
    if image.is_grayscale() { return Err(Error::new(ErrorKind::Unsupported, "native grayscale AVIF encoding is unsupported")); }
    let mut config = zenavif::AvifEncoderConfig::new().with_quality(1.0 + quality.value() * 0.98).with_alpha_quality_value(100.0)
        .with_effort_u32(match effort { Effort::Fast => 10, Effort::Balanced => 8, Effort::Best => 6 });
    *config.inner_mut() = config.inner().clone().bit_depth(zenavif::EncodeBitDepth::Eight).threads(Some(1))
        .color_primaries(1).transfer_characteristics(13).matrix_coefficients(6);
    super::encode(config, image,stop)
}
