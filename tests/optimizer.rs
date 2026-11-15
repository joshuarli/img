mod common;
use img::{optimizer::{self, Fidelity, FormatSelection, MetadataPolicy, Operation, Request}, perceptual::ScoreTarget, still::{Effort, EncodingMode, Format, Quality, SampleLayout, StillImage}};
fn source() -> Vec<u8> { StillImage::from_rgb8(48,32,common::pixels(48,32)).unwrap().encode(Format::Png,EncodingMode::Lossless,Effort::Fast).unwrap() }
#[test]
fn global_budget_smallest_evaluated_winner_and_repeatability() {
    let bytes=source();
    let request=Request { operation:Operation::Optimize(FormatSelection::Auto),fidelity:Fidelity::Score(ScoreTarget::new(70.0).unwrap()),effort:Effort::Fast,..Request::default() };
    let a=optimizer::process(&bytes,request).unwrap(); let b=optimizer::process(&bytes,request).unwrap();
    assert_eq!(a.bytes,b.bytes); assert_eq!(a.format,b.format);
    assert_eq!(a.evaluations.len(),9);
    for format in [Format::Jpeg,Format::Png,Format::Webp,Format::Avif] { assert!(a.evaluations.iter().any(|e|e.format==format)); }
    assert!(a.bytes.len()<=bytes.len());
    let smallest=a.evaluations.iter().filter(|e|e.failure.is_none()).filter_map(|e|e.bytes).chain([bytes.len()]).min().unwrap();
    assert_eq!(a.bytes.len(),smallest);
    assert_eq!(a.evaluations.iter().filter(|e|e.format==Format::Png && e.quality.is_none()).count(),1);
}
#[test]
fn search_finishes_coarse_grid_before_refining_and_does_not_assume_monotonicity() {
    let bytes=StillImage::from_rgb8(48,32,(0..32u8).flat_map(|y|(0..48u8).flat_map(move|x|[x*4,y*6,100])).collect()).unwrap().encode(Format::Png,EncodingMode::Lossless,Effort::Fast).unwrap();
    let result=optimizer::process(&bytes,Request { operation:Operation::Convert(Format::Webp),fidelity:Fidelity::Score(ScoreTarget::new(40.0).unwrap()),effort:Effort::Balanced,..Request::default() }).unwrap();
    assert_eq!(result.evaluations.len(),24);
    assert_eq!(result.evaluations[1..7].iter().map(|e|e.quality.unwrap()).collect::<Vec<_>>(),[95.0,50.0,0.0,100.0,75.0,25.0]);
    assert_eq!(result.bytes.len(),result.evaluations.iter().filter(|e|e.failure.is_none()).map(|e|e.bytes.unwrap()).min().unwrap());
    let impossible=optimizer::process(&bytes,Request { operation:Operation::Convert(Format::Jpeg),fidelity:Fidelity::Score(ScoreTarget::new(100.0).unwrap()),effort:Effort::Fast,..Request::default() });
    assert!(impossible.is_err());
}
#[test]
fn strict_lossless_preserves_hidden_rgb_depth_and_jpeg_identity() {
    let image=StillImage::from_samples(16,16,SampleLayout::Rgba8,[80,90,100,0].repeat(256)).unwrap();
    let bytes=image.encode(Format::Png,EncodingMode::Lossless,Effort::Fast).unwrap();
    let output=optimizer::process(&bytes,Request { operation:Operation::Convert(Format::Webp),fidelity:Fidelity::Lossless,..Request::default() }).unwrap();
    assert!(optimizer::samples_equal(&image,&StillImage::decode(&output.bytes).unwrap()));
    let deep=StillImage::from_samples(16,16,SampleLayout::Gray16,(0..256u16).flat_map(|v|(v*251).to_ne_bytes()).collect()).unwrap();
    let deep_bytes=deep.encode(Format::Png,EncodingMode::Lossless,Effort::Fast).unwrap();
    let output=optimizer::process(&deep_bytes,Request { operation:Operation::Convert(Format::Png),fidelity:Fidelity::Lossless,..Request::default() }).unwrap();
    assert!(optimizer::samples_equal(&deep,&StillImage::decode(&output.bytes).unwrap()));
    let jpeg=StillImage::from_rgb8(48,32,common::pixels(48,32)).unwrap().encode(Format::Jpeg,EncodingMode::Quality(Quality::new(80.0).unwrap()),Effort::Fast).unwrap();
    let output=optimizer::process(&jpeg,Request { fidelity:Fidelity::Lossless,..Request::default() }).unwrap();
    assert!(output.unchanged); assert!(output.evaluations.is_empty()); assert_eq!(output.bytes,jpeg); assert_eq!(output.score,None);
    assert!(optimizer::process(&bytes,Request { operation:Operation::Convert(Format::Jpeg),fidelity:Fidelity::Lossless,..Request::default() }).is_err());
}
#[test]
fn original_fallback_is_explicit_and_cannot_bypass_stripping() {
    let bytes=source();
    let request=Request { fidelity:Fidelity::Score(ScoreTarget::new(100.0).unwrap()),effort:Effort::Fast,..Request::default() };
    let output=optimizer::process(&bytes,request).unwrap();
    assert!(output.bytes.len()<=bytes.len()); if output.unchanged { assert_eq!(output.bytes,bytes); }
    let mut tagged=bytes.clone(); let data=vec![b'x';200];
    let mut chunk=(data.len() as u32).to_be_bytes().to_vec();chunk.extend_from_slice(b"tEXt");chunk.extend_from_slice(&data);
    let crc=crc32fast::hash(&chunk[4..]);chunk.extend_from_slice(&crc.to_be_bytes());tagged.splice(33..33,chunk);
    let stripped=optimizer::process(&tagged,Request {metadata:MetadataPolicy::Strip,..request}).unwrap();
    assert!(!stripped.unchanged); assert!(stripped.bytes.len()<tagged.len()); assert!(!stripped.bytes.windows(4).any(|p|p==b"tEXt"));
    let scratch=common::Scratch::new("optimizer-publication");let path=scratch.0.join("existing.png");std::fs::write(&path,b"owner bytes").unwrap();
    assert!(img::publication::publish(&stripped.bytes,img::publication::Destination::New(&path),None).is_err());
    assert_eq!(std::fs::read(path).unwrap(),b"owner bytes");
}
#[test]
fn no_op_resize_keeps_the_original_eligible() {
    let mut bytes=Vec::new();let mut encoder=png::Encoder::new(&mut bytes,1,1);
    encoder.set_color(png::ColorType::Grayscale);encoder.write_header().unwrap().write_image_data(&[128]).unwrap();
    let request=Request {fidelity:Fidelity::Score(ScoreTarget::new(100.0).unwrap()),effort:Effort::Fast,max_dimension:std::num::NonZeroU32::new(10),..Request::default()};
    let result=optimizer::process(&bytes,request).unwrap();assert!(result.unchanged);assert_eq!(result.bytes,bytes);
    let large=source();let resized=optimizer::process(&large,Request {fidelity:Fidelity::Score(ScoreTarget::new(0.0).unwrap()),max_dimension:std::num::NonZeroU32::new(8),..request}).unwrap();
    assert!(!resized.unchanged);assert_eq!(StillImage::decode(&resized.bytes).unwrap().dimensions(),(8,5));
}
#[test]
fn direct_quality_converts_even_when_larger_and_cancellation_prevents_work() {
    let bytes=source();
    let result=optimizer::process(&bytes,Request {operation:Operation::Convert(Format::Webp),fidelity:Fidelity::Quality(Quality::new(100.0).unwrap()),..Request::default()}).unwrap();
    assert_eq!(result.format,Format::Webp); assert_eq!(result.evaluations.len(),1); assert!(!result.unchanged);
    assert!(optimizer::process_cancellable(&bytes,Request::default(),&{ let c=img::cancellation::Cancellation::default(); c.cancel(); c }).is_err());
}

#[test]
fn jpeg_identity_cannot_bypass_stripping_private_app_zero_data() {
    let image=StillImage::from_rgb8(24,16,common::pixels(24,16)).unwrap();let jpeg=image.encode(Format::Jpeg,EncodingMode::Quality(Quality::new(80.0).unwrap()),Effort::Fast).unwrap();
    for payload in [b"PRIVATE\0owner secret".to_vec(),[b"JFIF\0\x01\x02\0\0\x01\0\x01\0\0".as_slice(),b"owner secret"].concat()] {
        let mut tagged=jpeg[..2].to_vec();tagged.extend_from_slice(&[255,224]);tagged.extend_from_slice(&((payload.len()+2) as u16).to_be_bytes());tagged.extend_from_slice(&payload);tagged.extend_from_slice(&jpeg[2..]);
        let result=optimizer::process(&tagged,Request {fidelity:Fidelity::Lossless,metadata:MetadataPolicy::Strip,..Request::default()});
        assert!(result.is_err(),"original JPEG bytes contain private APP0 data and cannot satisfy strip");
    }
}

#[test]
fn jpeg_strip_cannot_copy_late_segments_or_trailing_payloads() {
    let image=StillImage::from_rgb8(24,16,common::pixels(24,16)).unwrap();let jpeg=image.encode(Format::Jpeg,EncodingMode::Quality(Quality::new(80.0).unwrap()),Effort::Fast).unwrap();
    let payload=b"private owner metadata";
    let mut late=jpeg[..jpeg.len()-2].to_vec();late.extend_from_slice(&[255,254]);late.extend_from_slice(&((payload.len()+2) as u16).to_be_bytes());late.extend_from_slice(payload);late.extend_from_slice(&[255,217]);
    let mut trailing=jpeg.clone();trailing.extend_from_slice(payload);
    StillImage::decode(&late).unwrap();
    for bytes in [late,trailing] {
        assert!(optimizer::process(&bytes,Request {fidelity:Fidelity::Lossless,metadata:MetadataPolicy::Strip,..Request::default()}).is_err(),"private data after scan start cannot survive strip fallback");
    }
}

#[test]
fn direct_quality_is_refused_for_a_lossless_only_png_representation() {
    let gray=StillImage::from_samples(16,16,SampleLayout::Gray8,vec![80;256]).unwrap().encode(Format::Png,EncodingMode::Lossless,Effort::Fast).unwrap();
    assert!(optimizer::process(&gray,Request {fidelity:Fidelity::Quality(Quality::new(80.0).unwrap()),..Request::default()}).is_err());
}

#[test]
fn lossless_cross_format_cannot_discard_hdr_color_signaling() {
    let image=StillImage::from_rgb8(16,16,common::pixels(16,16)).unwrap();let mut bytes=image.encode(Format::Png,EncodingMode::Lossless,Effort::Fast).unwrap();
    let mut chunk=4u32.to_be_bytes().to_vec();chunk.extend_from_slice(b"cICP");chunk.extend_from_slice(&[9,16,0,1]);let crc=crc32fast::hash(&chunk[4..]);chunk.extend_from_slice(&crc.to_be_bytes());bytes.splice(33..33,chunk);
    assert!(optimizer::process(&bytes,Request {operation:Operation::Convert(Format::Webp),fidelity:Fidelity::Lossless,..Request::default()}).is_err(),"destination has no representation of the source HDR transfer");
}
