mod common;

use std::{io::Cursor, fs};
use img::still::{Effort, EncodingMode, Format, Quality, SampleLayout, StillImage};

fn chunk(kind:&[u8;4],data:&[u8])->Vec<u8> {let mut c=(data.len() as u32).to_be_bytes().to_vec();c.extend_from_slice(kind);c.extend_from_slice(data);let crc=crc32fast::hash(&c[4..]);c.extend_from_slice(&crc.to_be_bytes());c}

#[test]
fn exact_png_matrix_preserves_precision_alpha_gray_and_hidden_samples() {
    for (layout, channels, depth, color) in [
        (SampleLayout::Gray8, 1, 8, png::ColorType::Grayscale),
        (SampleLayout::GrayAlpha8, 2, 8, png::ColorType::GrayscaleAlpha),
        (SampleLayout::Rgb8, 3, 8, png::ColorType::Rgb),
        (SampleLayout::Rgba8, 4, 8, png::ColorType::Rgba),
        (SampleLayout::Gray16, 1, 16, png::ColorType::Grayscale),
        (SampleLayout::GrayAlpha16, 2, 16, png::ColorType::GrayscaleAlpha),
        (SampleLayout::Rgb16, 3, 16, png::ColorType::Rgb),
        (SampleLayout::Rgba16, 4, 16, png::ColorType::Rgba),
    ] {
        let values: Vec<u16> = (0..16 * 12 * channels).map(|i| (i * 971) as u16).collect();
        let samples = if depth == 8 { values.iter().map(|&v| v as u8).collect() }
            else { values.iter().flat_map(|v| v.to_ne_bytes()).collect() };
        let image = StillImage::from_samples(16, 12, layout, samples).unwrap();
        let bytes = image.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
        let mut decoder = png::Decoder::new(Cursor::new(&bytes));
        decoder.set_transformations(png::Transformations::EXPAND);
        let mut reader = decoder.read_info().unwrap();
        let mut output = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut output).unwrap();
        assert_eq!((info.width, info.height), (16, 12));
        assert_eq!(info.color_type, color, "{layout:?}");
        let expected: Vec<_> = if depth == 8 { values.iter().map(|&v| v as u8).collect() }
            else { values.iter().flat_map(|v| v.to_be_bytes()).collect() };
        assert_eq!(&output[..info.buffer_size()], expected, "{layout:?}");
        let decoded = StillImage::decode(&bytes).unwrap();
        assert_eq!(decoded.bit_depth(), depth);
        assert_eq!(decoded.has_alpha(), channels == 2 || channels == 4);
        if channels == 1 || channels == 2 { assert!(decoded.is_grayscale(), "{layout:?}"); }
        assert_eq!(decoded.samples(), image.samples(), "{layout:?}");
    }
}

#[test]
fn indexed_png_compares_expanded_samples_and_rejects_animation_and_corruption() {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 4, 2);
        encoder.set_color(png::ColorType::Indexed); encoder.set_depth(png::BitDepth::Eight);
        encoder.set_palette(vec![240, 10, 20, 30, 200, 40]); encoder.set_trns(vec![0, 255]);
        encoder.write_header().unwrap().write_image_data(&[0, 1, 0, 1, 1, 0, 1, 0]).unwrap();
    }
    let image = StillImage::decode(&bytes).unwrap();
    let expected: Vec<_> = [0, 1, 0, 1, 1, 0, 1, 0].into_iter().flat_map(|i| if i == 0 { [240, 10, 20, 0] } else { [30, 200, 40, 255] }).collect();
    assert_eq!(image.samples(), expected);
    let output = image.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
    assert_eq!(StillImage::decode(&output).unwrap().samples(), expected);
    let mut corrupt = bytes.clone(); corrupt[29] ^= 1;
    assert!(StillImage::decode(&corrupt).is_err());
    let mut animated = Vec::new();
    let mut encoder = png::Encoder::new(&mut animated, 2, 2);
    encoder.set_animated(2, 0).unwrap(); encoder.set_color(png::ColorType::Rgb);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(&[127; 12]).unwrap(); writer.write_image_data(&[128; 12]).unwrap(); drop(writer);
    assert!(StillImage::decode(&animated).err().unwrap().to_string().contains("animation"));
}

#[test]
fn direct_palette_mode_is_explicit_and_keeps_dimensions() {
    let image = StillImage::from_rgb8(32, 24, common::pixels(32, 24)).unwrap();
    let bytes = image.encode(Format::Png, EncodingMode::Quality(Quality::new(70.0).unwrap()), Effort::Fast).unwrap();
    let reader = png::Decoder::new(Cursor::new(&bytes)).read_info().unwrap();
    assert_eq!(reader.info().color_type, png::ColorType::Indexed);
    assert_eq!(StillImage::decode(&bytes).unwrap().dimensions(), (32, 24));
    let deep = StillImage::from_samples(8, 8, SampleLayout::Gray16, vec![0; 128]).unwrap();
    assert!(deep.encode(Format::Png, EncodingMode::Quality(Quality::new(70.0).unwrap()), Effort::Fast).is_err());
    let alpha_samples: Vec<_> = (0..32 * 24).flat_map(|i| [(i * 13) as u8, (i * 7) as u8, (i * 3) as u8, [0, 85, 170, 255][i % 4]]).collect();
    let alpha_image = StillImage::from_samples(32, 24, SampleLayout::Rgba8, alpha_samples).unwrap();
    let alpha_bytes = alpha_image.encode(Format::Png, EncodingMode::Quality(Quality::new(50.0).unwrap()), Effort::Fast).unwrap();
    let reader = png::Decoder::new(Cursor::new(&alpha_bytes)).read_info().unwrap();
    assert_eq!(reader.info().color_type, png::ColorType::Indexed);
    assert!(reader.info().trns.is_some(), "varying alpha must survive palette encoding");
    assert!(StillImage::decode(&alpha_bytes).unwrap().has_alpha());
}

#[test]
fn png_private_metadata_and_color_chunks_survive_exact_encoding() {
    let scratch = common::Scratch::new("png-metadata");
    let source = scratch.0.join("source.png");
    common::write_png(&source, 16, 12);
    let mut bytes = fs::read(source).unwrap();
    let data = b"Comment\0private fixture";
    let mut chunk = (data.len() as u32).to_be_bytes().to_vec(); chunk.extend_from_slice(b"tEXt"); chunk.extend_from_slice(data);
    let crc = crc32fast::hash(&chunk[4..]); chunk.extend_from_slice(&crc.to_be_bytes());
    bytes.splice(33..33, chunk.clone());
    let image = StillImage::decode(&bytes).unwrap();
    let encoded = image.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap();
    assert!(encoded.windows(chunk.len()).any(|part| part == chunk));
    assert_eq!(StillImage::decode(&encoded).unwrap().samples(), image.samples());
}

#[test]
fn lossless_png_strip_preserves_final_content_light_metadata() {
    use img::optimizer::{self,Fidelity,MetadataPolicy,Operation,Request};
    let mut base=Vec::new();let mut encoder=png::Encoder::new(&mut base,1,1);
    encoder.set_color(png::ColorType::Rgb);encoder.set_depth(png::BitDepth::Sixteen);
    encoder.write_header().unwrap().write_image_data(&[0x2e,0xe0,0x55,0xf0,0x7d,0x00]).unwrap();
    let cicp=chunk(b"cICP",&[9,16,0,1]);let light=chunk(b"cLLI",&[0,0x98,0x96,0x80,0,0x3d,0x09,0]);
    let mut tagged=base.clone();tagged.splice(33..33,[cicp.clone(),light.clone()].concat());
    let original=StillImage::decode(&tagged).unwrap();
    let mut no_light=base;no_light.splice(33..33,cicp.clone());
    assert!(!optimizer::samples_equal(&original,&StillImage::decode(&no_light).unwrap()));
    for metadata in [MetadataPolicy::Keep,MetadataPolicy::Strip] {
        let output=optimizer::process(&tagged,Request {operation:Operation::Convert(Format::Png),fidelity:Fidelity::Lossless,metadata,effort:Effort::Fast,..Request::default()}).unwrap();
        assert!(output.bytes.windows(cicp.len()).any(|c|c==cicp));
        assert!(output.bytes.windows(light.len()).any(|c|c==light));
        assert!(optimizer::samples_equal(&original,&StillImage::decode(&output.bytes).unwrap()));
    }
}

#[test]
fn compressed_text_counts_toward_the_cumulative_metadata_limit() {
    let fixture=|count:usize,international:bool| {
        let mut bytes=Vec::new();let mut info=png::Info::with_size(1,1);info.color_type=png::ColorType::Rgb;
        for index in 0..count {
            if international {let mut chunk=png::text_metadata::ITXtChunk::new(format!("Comment{index}"),"A".repeat(1024*1024));chunk.compressed=true;info.utf8_text.push(chunk);}
            else {info.compressed_latin1_text.push(png::text_metadata::ZTXtChunk::new(format!("Comment{index}"),"A".repeat(1024*1024)));}
        }
        let encoder=png::Encoder::with_info(&mut bytes,info).unwrap();
        encoder.write_header().unwrap().write_image_data(&[128,128,128]).unwrap();bytes
    };
    for international in [false,true] {
        let allowed=fixture(2,international);img::still::describe(&allowed).unwrap();StillImage::decode(&allowed).unwrap();
        let oversized=fixture(16,international);assert!(oversized.len()<100_000);
        assert_eq!(img::still::describe(&oversized).unwrap_err().kind(),img::still::ErrorKind::Limit);
        assert_eq!(StillImage::decode(&oversized).err().unwrap().kind(),img::still::ErrorKind::Limit);
    }
}

#[test]
fn layout_sensitive_png_metadata_is_never_silently_discarded() {
    let image = StillImage::from_rgb8(8, 8, [80, 90, 100].repeat(64)).unwrap();
    let mut original = Vec::new();
    let mut encoder = png::Encoder::new(&mut original, 8, 8); encoder.set_color(png::ColorType::Rgb);
    encoder.write_header().unwrap().write_image_data(&image.samples()).unwrap();
    for (kind, data) in [(b"sBIT", vec![7, 7, 7]), (b"bKGD", vec![0, 80, 0, 80, 0, 80]), (b"vpAG", vec![1, 2, 3])] {
        let mut bytes = original.clone();
        let mut chunk = (data.len() as u32).to_be_bytes().to_vec(); chunk.extend_from_slice(kind); chunk.extend_from_slice(&data);
        let crc = crc32fast::hash(&chunk[4..]); chunk.extend_from_slice(&crc.to_be_bytes()); bytes.splice(33..33, chunk);
        let source = StillImage::decode(&bytes).unwrap();
        assert!(source.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).is_err(), "must not discard or misapply {kind:?}");
        let stripped = source.strip_metadata(img::still::SamplePreservation::Exact).unwrap();
        assert_eq!(StillImage::decode(&stripped.encode(Format::Png, EncodingMode::Lossless, Effort::Fast).unwrap()).unwrap().samples(), source.samples());
    }
}

#[test]
fn rgb_icc_profile_keeps_a_compatible_png_color_type_for_gray_valued_pixels() {
    let mut info=png::Info::with_size(16,16);info.color_type=png::ColorType::Rgb;info.bit_depth=png::BitDepth::Eight;info.icc_profile=Some(std::borrow::Cow::Owned(moxcms::ColorProfile::new_srgb().encode().unwrap()));
    let mut bytes=Vec::new();png::Encoder::with_info(&mut bytes,info).unwrap().write_header().unwrap().write_image_data(&[128;16*16*3]).unwrap();
    let image=StillImage::decode(&bytes).unwrap();let encoded=image.encode(Format::Png,EncodingMode::Lossless,Effort::Fast).unwrap();
    let reader=png::Decoder::new(Cursor::new(&encoded)).read_info().unwrap();assert!(matches!(reader.info().color_type,png::ColorType::Rgb|png::ColorType::Rgba|png::ColorType::Indexed),"RGB profile requires an RGB-compatible PNG representation");
    assert_eq!(reader.info().icc_profile.as_deref(),png::Decoder::new(Cursor::new(&bytes)).read_info().unwrap().info().icc_profile.as_deref());
}

#[test]
fn rgb_cicp_signaling_keeps_an_rgb_png_color_type_for_gray_valued_pixels() {
    let mut info=png::Info::with_size(16,16);info.color_type=png::ColorType::Rgb;info.bit_depth=png::BitDepth::Eight;
    let mut bytes=Vec::new();png::Encoder::with_info(&mut bytes,info).unwrap().write_header().unwrap().write_image_data(&[128;16*16*3]).unwrap();
    bytes.splice(33..33,chunk(b"cICP",&[1,13,0,1]));
    let image=StillImage::decode(&bytes).unwrap();
    for effort in [Effort::Fast,Effort::Balanced,Effort::Best] {
        let encoded=image.encode(Format::Png,EncodingMode::Lossless,effort).unwrap();
        let reader=png::Decoder::new(Cursor::new(&encoded)).read_info().unwrap();
        assert!(matches!(reader.info().color_type,png::ColorType::Rgb|png::ColorType::Rgba|png::ColorType::Indexed),"{effort:?}: cICP for RGB requires an RGB-compatible PNG representation");
        assert_eq!(img::still::describe(&encoded).unwrap().cicp,img::still::describe(&bytes).unwrap().cicp,"{effort:?}");
        assert!(img::optimizer::samples_equal(&StillImage::decode(&encoded).unwrap(),&image),"{effort:?}");
    }
}

#[test]
fn icc_profile_from_a_non_png_source_survives_png_encoding() {
    let icc=moxcms::ColorProfile::new_display_p3().encode().unwrap();
    let mut info=png::Info::with_size(16,16);info.color_type=png::ColorType::Rgb;info.bit_depth=png::BitDepth::Eight;info.icc_profile=Some(std::borrow::Cow::Owned(icc.clone()));
    let mut bytes=Vec::new();png::Encoder::with_info(&mut bytes,info).unwrap().write_header().unwrap().write_image_data(&(0..16*16*3).map(|i|(i*7) as u8).collect::<Vec<_>>()).unwrap();
    let jpeg=StillImage::decode(&bytes).unwrap().encode(Format::Jpeg,EncodingMode::Quality(Quality::new(90.0).unwrap()),Effort::Fast).unwrap();
    let source=StillImage::decode(&jpeg).unwrap();
    assert_eq!(img::still::describe(&jpeg).unwrap().icc,true);
    for effort in [Effort::Fast,Effort::Balanced,Effort::Best] {
        let encoded=source.encode(Format::Png,EncodingMode::Lossless,effort).unwrap();
        let reader=png::Decoder::new(Cursor::new(&encoded)).read_info().unwrap();
        assert_eq!(reader.info().icc_profile.as_deref(),Some(icc.as_slice()),"{effort:?}");
    }
}

#[test]
fn reducible_sixteen_bit_samples_still_keep_sixteen_bit_precision() {
    for (layout,channels) in [(SampleLayout::Gray16,1),(SampleLayout::Rgb16,3),(SampleLayout::Rgba16,4)] {
        let values=[0u16,256,32768,65280];let samples=(0..16*16*channels).flat_map(|i|values[i%4].to_ne_bytes()).collect();let image=StillImage::from_samples(16,16,layout,samples).unwrap();
        let encoded=image.encode(Format::Png,EncodingMode::Lossless,Effort::Fast).unwrap();let decoded=StillImage::decode(&encoded).unwrap();assert_eq!(decoded.bit_depth(),16,"{layout:?}");assert_eq!(decoded.samples(),image.samples());
    }
}

#[test]
fn multi_megabyte_lossless_png_is_exact_at_every_effort() {
    let (width, height) = (1024u32, 1100u32);
    let mut state = 0x2545_f491u32;
    let samples: Vec<u8> = (0..width * height * 3).map(|i| {
        state ^= state << 13; state ^= state >> 17; state ^= state << 5;
        let row = i / (width * 3);
        if row % 300 < 150 { (i % 251) as u8 ^ (row as u8 & 7) } else { (state >> 24) as u8 & 0x3f }
    }).collect();
    let image = StillImage::from_samples(width, height, SampleLayout::Rgb8, samples).unwrap();
    for effort in [Effort::Fast, Effort::Balanced, Effort::Best] {
        let bytes = image.encode(Format::Png, EncodingMode::Lossless, effort).unwrap();
        let mut decoder = png::Decoder::new(Cursor::new(&bytes));
        decoder.set_transformations(png::Transformations::EXPAND);
        let mut reader = decoder.read_info().unwrap();
        let mut output = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut output).unwrap();
        assert_eq!(&output[..info.buffer_size()], image.samples(), "{effort:?}");
        assert_eq!(image.encode(Format::Png, EncodingMode::Lossless, effort).unwrap(), bytes, "{effort:?} output is not reproducible");
    }
}

const DEFLATE_CHUNK: usize = 1 << 20;

fn noise(len: usize, seed: u32) -> Vec<u8> {
    let mut state = seed | 1;
    (0..len).map(|i| {
        state ^= state << 13; state ^= state >> 17; state ^= state << 5;
        if (i >> 12) % 3 == 0 { (i % 97) as u8 } else { (state >> 24) as u8 }
    }).collect()
}

#[test]
fn chunked_deflate_accepts_short_final_tails_at_every_chunked_level() {
    for level in 5u8..=12 {
        let passthrough = 55 - 4 * level as usize;
        for chunks in [2, 3] {
            for tail in [0, 1, 6, passthrough, passthrough + 1, 64] {
                let data = noise(chunks * DEFLATE_CHUNK + tail, 0x9e37_79b9 ^ tail as u32);
                let compressed = oxipng::internal_tests::deflate(&data, level, None)
                    .unwrap_or_else(|e| panic!("level {level}, {chunks} chunks + {tail}: {e}"));
                let inflated = fdeflate::decompress_to_vec(&compressed)
                    .unwrap_or_else(|e| panic!("level {level}, {chunks} chunks + {tail}: {e:?}"));
                assert!(inflated == data, "level {level}, {chunks} chunks + {tail} did not round-trip");
            }
        }
    }
}

#[test]
fn nonreducible_png_with_a_short_final_deflate_chunk_is_exact_at_every_effort() {
    // Filtered size (1 + width * bytes per pixel) * height lands just past a chunk boundary.
    for (layout, width, height, tail) in [
        (SampleLayout::Rgb8, 2144, 326, 6),
        (SampleLayout::Gray8, 16256, 129, 1),
        (SampleLayout::Rgba8, 109, 4799, 11),
        (SampleLayout::Rgba8, 732, 716, 12),
        (SampleLayout::Rgb16, 469, 745, 23),
        (SampleLayout::Rgb8, 10280, 68, 36),
    ] {
        let channels = match layout { SampleLayout::Gray8 => 1, SampleLayout::Rgba8 => 4, _ => 3 };
        let sample_bytes = if layout == SampleLayout::Rgb16 { 2 } else { 1 };
        let filtered = (1 + width * channels * sample_bytes) * height;
        assert_eq!(filtered % DEFLATE_CHUNK, tail, "{layout:?} {width}x{height}");
        let mut state = 0x2545_f491u32 ^ tail as u32;
        let samples: Vec<u8> = (0..width * height * channels * sample_bytes).map(|_| {
            state ^= state << 13; state ^= state >> 17; state ^= state << 5;
            (state >> 24) as u8
        }).collect();
        let image = StillImage::from_samples(width as u32, height as u32, layout, samples).unwrap();
        for effort in [Effort::Fast, Effort::Balanced, Effort::Best] {
            let bytes = image.encode(Format::Png, EncodingMode::Lossless, effort)
                .unwrap_or_else(|e| panic!("{layout:?} {width}x{height} {effort:?}: {e}"));
            let mut decoder = png::Decoder::new(Cursor::new(&bytes));
            decoder.set_transformations(png::Transformations::IDENTITY);
            let mut reader = decoder.read_info().unwrap();
            let mut output = vec![0; reader.output_buffer_size().unwrap()];
            let info = reader.next_frame(&mut output).unwrap();
            assert_eq!(info.line_size * info.height as usize, width * channels * sample_bytes * height, "{layout:?} {effort:?} was reduced");
            let expected: Vec<u8> = if sample_bytes == 2 {
                image.samples().chunks_exact(2).flat_map(|s| u16::from_ne_bytes([s[0], s[1]]).to_be_bytes()).collect()
            } else { image.samples().to_vec() };
            assert!(output[..info.buffer_size()] == expected, "{layout:?} {width}x{height} {effort:?} is not exact");
        }
    }
}
