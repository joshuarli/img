use std::{fs, io::Cursor};
use zune_core::{bytestream::ZCursor, result::DecodingResult};
fn main() {
    let mut count = 0;
    let mut mismatches = 0;
    let mut zen_mismatches = 0;
    let mut errors = 0;
    for path in std::env::args().skip(1) {
        let bytes = fs::read(&path).unwrap();
        let mut decoder = png::Decoder::new(Cursor::new(&bytes));
        decoder.set_transformations(png::Transformations::EXPAND);
        let mut reader = decoder.read_info().unwrap();
        let mut reference = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut reference).unwrap();
        reference.truncate(info.buffer_size());
        if cfg!(target_endian = "little") && info.bit_depth == png::BitDepth::Sixteen { for pair in reference.chunks_exact_mut(2) { pair.swap(0, 1); } }
        let zen = zenpng::decode(&bytes, &zenpng::PngDecodeConfig::default(), &enough::Unstoppable).unwrap();
        let mut zenbytes = zen.pixels.copy_to_contiguous_bytes();
        if info.color_type == png::ColorType::GrayscaleAlpha && zenbytes.len() == reference.len()*2 {
            let sample = if info.bit_depth == png::BitDepth::Sixteen { 2 } else { 1 };
            zenbytes = zenbytes.chunks_exact(4*sample).flat_map(|p| p[..sample].iter().chain(&p[3*sample..4*sample]).copied()).collect();
        }
        if reference != zenbytes { println!("ZEN MISMATCH {path}: reference_len={} candidate_len={} first={:?}", reference.len(), zenbytes.len(), reference.iter().zip(&zenbytes).position(|(a,b)| a != b)); zen_mismatches += 1; }
        let mut candidate = zune_png::PngDecoder::new(ZCursor::new(&bytes));
        let decoded = match candidate.decode() {
            Ok(DecodingResult::U8(v)) => v,
            Ok(DecodingResult::U16(v)) => v.into_iter().flat_map(u16::to_ne_bytes).collect(),
            Ok(_) => panic!("unexpected sample type"),
            Err(e) => { eprintln!("ERROR {path}: {e}"); errors += 1; continue; }
        };
        count += 1;
        if reference != decoded {
            mismatches += 1;
            let first = reference.iter().zip(&decoded).position(|(a,b)| a != b);
            println!("MISMATCH {path}: reference_len={} candidate_len={} first={first:?} reference={:?} candidate={:?}", reference.len(), decoded.len(), &reference[..reference.len().min(32)], &decoded[..decoded.len().min(32)]);
        }
    }
    println!("zune-png fixtures={count} mismatches={mismatches} errors={errors}");
    println!("zenpng fixtures={count} mismatches={zen_mismatches}");
    if mismatches + zen_mismatches + errors > 0 { std::process::exit(1); }
}
