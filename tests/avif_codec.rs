mod common;
#[path="support/native_avif.rs"] mod native_avif;
#[path="support/avif_raw.rs"] mod avif_raw;

use img::still::{Effort, EncodingMode, Format, Quality, SampleLayout, StillImage};

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
    let header = img::still::describe(bytes).unwrap();
    assert_eq!(header.bit_depth, Some(8));
    assert!(!header.grid); assert!(!header.animation);
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
    let output=img::optimizer::process(bytes,img::optimizer::Request {operation:img::optimizer::Operation::Convert(Format::Webp),fidelity:img::optimizer::Fidelity::Lossless,effort:Effort::Fast,..Default::default()}).unwrap();
    assert_eq!(StillImage::decode(&output.bytes).unwrap().samples(),decoded);
}

#[test]
fn yuv_avif_to_png_signals_full_range_rgb() {
    let bytes=include_bytes!("files/generated/film-grain.avif");
    let source=StillImage::decode(bytes).unwrap();
    let output=img::optimizer::process(bytes,img::optimizer::Request {operation:img::optimizer::Operation::Convert(Format::Png),fidelity:img::optimizer::Fidelity::Lossless,effort:Effort::Fast,..Default::default()}).unwrap();
    let header=img::still::describe(&output.bytes).unwrap();assert_eq!(header.cicp,Some([1,13,0,1]));
    let decoded=StillImage::decode(&output.bytes).unwrap();assert_eq!(decoded.samples(),source.samples());decoded.prepare().unwrap();
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
    let ten_bit = include_bytes!("files/generated/ten-bit.avif");
    let error = StillImage::decode(ten_bit).err().unwrap();
    assert!(error.to_string().contains("eight-bit"), "{error}");
    let animation = include_bytes!("files/generated/animation.avif");
    let error = StillImage::decode(animation).err().unwrap();
    assert!(error.to_string().contains("animation"), "{error}");
}

#[test]
fn real_grid_container_is_refused_without_decoding_tiles() {
    let image = StillImage::from_rgb8(16, 16, common::pixels(16, 16)).unwrap();
    let bytes = image.encode(Format::Avif, mode(), Effort::Fast).unwrap();
    let tile = avif_raw::primary_data(&bytes);
    let av1 = avif_raw::av1c_fields(&bytes);
    let mut grid = zenavif_serialize::grid::GridImage::new();
    let mut config = zenavif_serialize::Av1CBox::default();
    config.seq_profile = av1.profile; config.seq_level_idx_0 = av1.level; config.seq_tier_0 = av1.tier;
    config.chroma_subsampling_x = av1.chroma_subsampling_x; config.chroma_subsampling_y = av1.chroma_subsampling_y;
    config.chroma_sample_position = av1.chroma_sample_position;
    grid.set_color_config(config);
    let bytes = grid.serialize(1, 2, 32, 16, 16, 16, &[&tile, &tile], None).unwrap();
    assert!(StillImage::decode(&bytes).err().unwrap().to_string().contains("grid"));
    let header=img::still::describe(&bytes).unwrap();assert_eq!((header.width,header.height),(32,16));
}

#[test]
fn multiple_still_temporal_units_are_not_silently_truncated() {
    let image=StillImage::from_rgb8(16,16,common::pixels(16,16)).unwrap();let bytes=image.encode(Format::Avif,mode(),Effort::Fast).unwrap();let data=avif_raw::primary_data(&bytes);
    let mut combined=data.clone();combined.extend_from_slice(&data);let config=avif_raw::av1c_fields(&bytes);
    let mut serializer=zenavif_serialize::Aviffy::new();serializer.set_chroma_subsampling((config.chroma_subsampling_x,config.chroma_subsampling_y)).set_seq_profile(config.profile);
    let bytes=serializer.to_vec(&combined,None,16,16,8);let header=img::still::describe(&bytes).unwrap();assert!(!header.animation);assert!(avif_raw::still_picture(&bytes));
    assert!(StillImage::decode(&bytes).is_err(),"multiple temporal units must not return only the first frame");
}

#[test]
fn header_inspection_exposes_transforms_and_unvalidated_hdr_is_refused() {
    let image=StillImage::from_rgb8(16,16,common::pixels(16,16)).unwrap();let original=image.encode(Format::Avif,mode(),Effort::Fast).unwrap();let data=avif_raw::primary_data(&original);let config=avif_raw::av1c_fields(&original);
    for kind in 0..3 {
        let mut serializer=zenavif_serialize::Aviffy::new();serializer.set_chroma_subsampling((config.chroma_subsampling_x,config.chroma_subsampling_y)).set_seq_profile(config.profile);
        if kind==0 {serializer.set_content_light_level(1000,400);} else if kind==1 {serializer.set_pixel_aspect_ratio(2,1);} else {serializer.set_rotation(1).set_mirror(1);}
        let bytes=serializer.to_vec(&data,None,16,16,8);
        assert!(StillImage::decode(&bytes).is_err(),"unvalidated HDR, nonsquare pixels or container transforms must not lose their interpretation");
        let header=img::still::describe(&bytes).unwrap();
        match kind {0=>assert!(header.hdr_metadata),1=>assert_eq!(header.pixel_aspect_ratio,Some([2,1])),_=>{assert_eq!(header.rotation_ccw_degrees,Some(90));assert_eq!(header.mirror_axis,Some(1));}}
    }
}

#[test]
fn bitstream_and_container_hdr_metadata_are_reported_and_refused() {
    let image=StillImage::from_rgb8(16,16,common::pixels(16,16)).unwrap();let original=image.encode(Format::Avif,mode(),Effort::Fast).unwrap();
    StillImage::decode(&original).unwrap();let data=avif_raw::primary_data(&original);let config=avif_raw::av1c_fields(&original);
    let mut at=0;let insert=loop {
        let header=data[at];at+=1;if header&4!=0 {at+=1;}assert_ne!(header&2,0);
        let mut size=0usize;let mut shift=0;loop {let byte=data[at];at+=1;size|=((byte&127) as usize)<<shift;if byte&128==0 {break;}shift+=7;}
        at+=size;if (header>>3)&15==1 {break at;}
    };
    for kind in 0..4 {
        let mut stream=data.to_vec();
        if kind<2 {
            let obu=if kind==0 {vec![0x2a,6,1,3,0xe8,1,0x90,0x80]} else {let mut obu=vec![0x2a,26,2];obu.extend_from_slice(&[0;24]);obu.push(0x80);obu};
            stream.splice(insert..insert,obu);
        }
        let mut serializer=zenavif_serialize::Aviffy::new();serializer.set_chroma_subsampling((config.chroma_subsampling_x,config.chroma_subsampling_y)).set_seq_profile(config.profile);
        if kind==2 {serializer.set_content_light_level(1000,400);} else if kind==3 {serializer.set_mastering_display([(100,200);3],(100,200),10000,1);}
        let bytes=serializer.to_vec(&stream,None,16,16,8);
        assert!(img::still::describe(&bytes).unwrap().hdr_metadata,"kind {kind}");
        let error=StillImage::decode(&bytes).err().unwrap();assert!(error.to_string().contains("HDR"),"kind {kind}: {error}");
    }
}

#[test]
fn nclx_full_range_flag_is_the_high_bit_and_reserved_bits_are_ignored() {
    let bytes=include_bytes!("files/generated/film-grain.avif");
    let flags=bytes.windows(8).position(|w|w==b"colrnclx").unwrap()+14;
    for (byte,full_range) in [(0x80,1),(0xff,1),(0x7f,0),(0x00,0)] {
        let mut patched=bytes.to_vec();patched[flags]=byte;
        assert_eq!(img::still::describe(&patched).unwrap().cicp.unwrap()[3],full_range,"flags byte {byte:#04x}");
    }
}
