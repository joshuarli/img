use img::png_stream::{self, Decoder, Event};

fn animated() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_animated(2, 3).unwrap();
        encoder.set_frame_delay(1, 10).unwrap();
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 0, 0]).unwrap();
        writer.set_frame_delay(2, 10).unwrap();
        writer.set_blend_op(png::BlendOp::Over).unwrap();
        writer.set_dispose_op(png::DisposeOp::Background).unwrap();
        writer.write_image_data(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]).unwrap();
        writer.finish().unwrap();
    }
    bytes
}

#[test]
fn partial_input_preserves_rows_and_animation_metadata() {
    let bytes = animated();
    let mut decoder = Decoder::new(1_000_000);
    let mut rows = Vec::new();
    let mut frames = Vec::new();
    let mut ended = 0;
    for byte in bytes {
        decoder.push(&[byte]).unwrap();
        loop {
            match decoder.next_event().unwrap() {
                Event::NeedMore | Event::Complete => break,
                Event::Header(info) => {
                    assert_eq!(info.animation_control.unwrap().num_plays, 3);
                    assert_eq!(info.frame_control.unwrap().delay_num, 1);
                }
                Event::Frame(frame) => frames.push((frame.delay_num, frame.delay_den, frame.blend_op, frame.dispose_op)),
                Event::Row { data, .. } => rows.extend(data),
                Event::FrameEnd => ended += 1,
            }
        }
    }
    assert_eq!(ended, 2);
    assert_eq!(frames, [(2, 10, png::BlendOp::Over, png::DisposeOp::Background)]);
    assert_eq!(rows, [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
}

#[test]
fn final_truncated_input_is_an_error() {
    let bytes = animated();
    let mut decoder = Decoder::new(1_000_000);
    decoder.push(&bytes[..bytes.len() / 2]).unwrap();
    decoder.finish();
    loop {
        match decoder.next_event() {
            Err(_) => break,
            Ok(Event::Complete | Event::NeedMore) => panic!("truncated input was accepted"),
            Ok(_) => (),
        }
    }
}

#[test]
fn encoder_preserves_alpha_and_reports_short_input() {
    let pixels = [0, 1, 2, 255, 200, 50, 4, 17];
    for compression in [png_stream::Compression::Low, png_stream::Compression::Medium] {
        let bytes = png_stream::encode(2, 1, png::ColorType::Rgba, png::BitDepth::Eight, &pixels, compression).unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info().unwrap();
        let mut decoded = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut decoded).unwrap();
        assert_eq!(decoded, pixels);
    }
    assert!(png_stream::encode(2, 1, png::ColorType::Rgba, png::BitDepth::Eight, &pixels[..7], png_stream::Compression::Low).is_err());
}
