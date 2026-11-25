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

mod chromium {
    use img::png_stream::chromium::{Level, Outcome, Reader, Writer};
    use std::io::Cursor;

    #[derive(Clone, Default)]
    struct Sink(std::rc::Rc<std::cell::RefCell<Vec<u8>>>);

    impl std::io::Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn encoded(level: Level) -> Vec<u8> {
        let output = Sink::default();
        let mut writer = Writer::new(output.clone(), 3, 2, png::ColorType::Rgba, png::BitDepth::Eight, level, &[]).unwrap();
        for row in 0..2u8 {
            writer.write_row(&(0..12).map(|value| value + row).collect::<Vec<u8>>()).unwrap();
        }
        writer.finish().unwrap();
        let bytes = output.0.borrow().clone();
        bytes
    }

    #[test]
    fn rows_round_trip_at_every_level() {
        for level in [Level::Level1WithUpFilter, Level::Fastest, Level::Fast, Level::Balanced, Level::High] {
            let bytes = encoded(level);
            let mut reader = Reader::new(Cursor::new(bytes)).unwrap();
            assert_eq!(reader.output_color_type(), (png::ColorType::Rgba, png::BitDepth::Eight));
            for row in 0..2u8 {
                let data = reader.next_interlaced_row().unwrap().unwrap().to_vec();
                assert_eq!(data, (0..12).map(|value| value + row).collect::<Vec<u8>>());
            }
            assert!(reader.next_interlaced_row().unwrap().is_none());
        }
    }

    #[test]
    fn truncated_input_reports_incomplete() {
        let mut bytes = encoded(Level::Balanced);
        bytes.truncate(bytes.len() - 20);
        let mut reader = Reader::new(Cursor::new(bytes)).unwrap();
        let mut outcome = Outcome::Success;
        loop {
            match reader.next_interlaced_row() {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(error) => {
                    outcome = error;
                    break;
                }
            }
        }
        assert_eq!(outcome, Outcome::IncompleteInput);
    }
}
