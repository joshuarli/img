mod common;
#[path="support/native_avif.rs"] mod native_avif;

use rimage::still::{Effort, EncodingMode, Format, Quality, SampleLayout, StillImage};

fn mode() -> EncodingMode { EncodingMode::Quality(Quality::new(80.0).unwrap()) }

#[test]
fn avif_still_is_eight_bit_and_has_independent_native_decoder_proof() {
    let image = StillImage::from_rgb8(48, 32, common::pixels(48, 32)).unwrap();
    for quality in [0.0, 80.0, 100.0] {
        let bytes = image.encode(Format::Avif, EncodingMode::Quality(Quality::new(quality).unwrap()), Effort::Fast).unwrap();
        check_native(&bytes);
    }
}

fn check_native(bytes: &[u8]) {
    let parser = zenavif_parse::AvifParser::from_bytes(bytes).unwrap();
    assert_eq!(parser.av1_config().unwrap().bit_depth, 8);
    assert!(parser.grid_config().is_none()); assert!(parser.animation_info().is_none());
    let decoded = StillImage::decode(bytes).unwrap();
    assert_eq!(decoded.dimensions(), (48, 32));
    assert_eq!(decoded.bit_depth(), 8);
    let reference=native_avif::decode(bytes);
    let pixels = decoded.samples();
    assert_eq!(reference.len(), pixels.len());
    assert!(reference.iter().zip(pixels).all(|(&a, b)| a.abs_diff(b) <= 3));
}

#[test]
fn film_grain_is_in_displayed_samples_and_lossless_output() {
    let bytes=include_bytes!("files/generated/film-grain.avif");
    let displayed=native_avif::grain_fixture(bytes,true);let ungrained=native_avif::grain_fixture(bytes,false);
    assert!(displayed.iter().zip(&ungrained).filter(|(a,b)|a!=b).count()>1000);
    let decoded=StillImage::decode(bytes).unwrap().samples();
    assert!(displayed.iter().zip(&decoded).all(|(a,b)|a.abs_diff(*b)<=3));
    let output=rimage::optimizer::process(bytes,rimage::optimizer::Request {operation:rimage::optimizer::Operation::Convert(Format::Webp),fidelity:rimage::optimizer::Fidelity::Lossless,effort:Effort::Fast,..Default::default()}).unwrap();
    assert_eq!(StillImage::decode(&output.bytes).unwrap().samples(),decoded);
}

#[test]
fn unverified_alpha_lossless_precision_and_gray_are_explicitly_refused() {
    let rgba: Vec<_> = (0..32 * 24).flat_map(|i| [80, 120, 160, (i % 256) as u8]).collect();
    let image = StillImage::from_samples(32, 24, SampleLayout::Rgba8, rgba).unwrap();
    assert!(image.encode(Format::Avif, mode(), Effort::Fast).unwrap_err().to_string().contains("alpha"));
    assert!(image.encode(Format::Avif, EncodingMode::Lossless, Effort::Fast).is_err());
    for layout in [SampleLayout::Gray8, SampleLayout::Gray16] {
        let samples = vec![0; if layout == SampleLayout::Gray8 { 64 } else { 128 }];
        let image = StillImage::from_samples(8, 8, layout, samples).unwrap();
        assert!(image.encode(Format::Avif, mode(), Effort::Fast).is_err());
    }
}

#[test]
fn high_depth_and_animation_are_refused_from_real_containers() {
    use zencodec::encode::{EncodeJob, Encoder, EncoderConfig};
    let pixels: Vec<_> = common::pixels(16, 16).chunks_exact(3).map(|p| rgb::RGB8::new(p[0], p[1], p[2])).collect();
    let image = imgref::Img::new(pixels, 16, 16);
    let mut config = zenavif::AvifEncoderConfig::new().with_effort_u32(10);
    *config.inner_mut() = config.inner().clone().bit_depth(zenavif::EncodeBitDepth::Ten).threads(Some(1));
    let bytes = config.job().encoder().unwrap().encode(zenpixels::PixelSlice::from(image.as_ref()).erase()).unwrap().into_vec();
    let error = StillImage::decode(&bytes).err().unwrap();
    assert!(error.to_string().contains("eight-bit"), "{error}");
    let frame = zenavif::AnimationFrame { pixels: image.clone(), duration_ms: 100 };
    let animation = zenavif::encode_animation_rgb8(&[frame.clone(), frame],
        &zenavif::EncoderConfig::new().speed(10).threads(Some(1)).bit_depth(zenavif::EncodeBitDepth::Eight),
        zencodec::StopToken::new(zencodec::Unstoppable)).unwrap();
    assert!(StillImage::decode(&animation.avif_file).err().unwrap().to_string().contains("animation"));
}

#[test]
fn real_grid_container_is_refused_without_decoding_tiles() {
    let image = StillImage::from_rgb8(16, 16, common::pixels(16, 16)).unwrap();
    let bytes = image.encode(Format::Avif, mode(), Effort::Fast).unwrap();
    let parser = zenavif_parse::AvifParser::from_bytes(&bytes).unwrap();
    let tile = parser.primary_data().unwrap();
    let av1 = parser.av1_config().unwrap();
    let mut grid = zenavif_serialize::grid::GridImage::new();
    let mut config = zenavif_serialize::Av1CBox::default();
    config.seq_profile = av1.profile; config.seq_level_idx_0 = av1.level; config.seq_tier_0 = av1.tier != 0;
    config.chroma_subsampling_x = av1.chroma_subsampling_x != 0; config.chroma_subsampling_y = av1.chroma_subsampling_y != 0;
    config.chroma_sample_position = av1.chroma_sample_position;
    grid.set_color_config(config);
    let bytes = grid.serialize(1, 2, 32, 16, 16, 16, &[&tile, &tile], None).unwrap();
    assert!(StillImage::decode(&bytes).err().unwrap().to_string().contains("grid"));
    let header=rimage::still::describe(&bytes).unwrap();assert_eq!((header.width,header.height),(32,16));
}

#[test]
fn multiple_still_temporal_units_are_not_silently_truncated() {
    let image=StillImage::from_rgb8(16,16,common::pixels(16,16)).unwrap();let bytes=image.encode(Format::Avif,mode(),Effort::Fast).unwrap();let parser=zenavif_parse::AvifParser::from_bytes(&bytes).unwrap();let data=parser.primary_data().unwrap();
    let mut combined=data.to_vec();combined.extend_from_slice(&data);let config=parser.av1_config().unwrap();
    let mut serializer=zenavif_serialize::Aviffy::new();serializer.set_chroma_subsampling((config.chroma_subsampling_x!=0,config.chroma_subsampling_y!=0)).set_seq_profile(config.profile);
    let bytes=serializer.to_vec(&combined,None,16,16,8);let parser=zenavif_parse::AvifParser::from_bytes(&bytes).unwrap();assert!(parser.animation_info().is_none());assert!(parser.primary_metadata().unwrap().still_picture);
    assert!(StillImage::decode(&bytes).is_err(),"multiple temporal units must not return only the first frame");
}

#[test]
fn header_inspection_exposes_transforms_and_unvalidated_hdr_is_refused() {
    let image=StillImage::from_rgb8(16,16,common::pixels(16,16)).unwrap();let original=image.encode(Format::Avif,mode(),Effort::Fast).unwrap();let parser=zenavif_parse::AvifParser::from_bytes(&original).unwrap();let data=parser.primary_data().unwrap();let config=parser.av1_config().unwrap();
    for kind in 0..3 {
        let mut serializer=zenavif_serialize::Aviffy::new();serializer.set_chroma_subsampling((config.chroma_subsampling_x!=0,config.chroma_subsampling_y!=0)).set_seq_profile(config.profile);
        if kind==0 {serializer.set_content_light_level(1000,400);} else if kind==1 {serializer.set_pixel_aspect_ratio(2,1);} else {serializer.set_rotation(1).set_mirror(1);}
        let bytes=serializer.to_vec(&data,None,16,16,8);
        assert!(StillImage::decode(&bytes).is_err(),"unvalidated HDR, nonsquare pixels or container transforms must not lose their interpretation");
        let header=rimage::still::describe(&bytes).unwrap();
        match kind {0=>assert!(header.hdr_metadata),1=>assert_eq!(header.pixel_aspect_ratio,Some([2,1])),_=>{assert_eq!(header.rotation_ccw_degrees,Some(90));assert_eq!(header.mirror_axis,Some(1));}}
    }
}
