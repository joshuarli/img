//! Bounded, deterministic selection over verified encoded still images.
use std::{num::NonZeroU32, time::Instant};
use crate::{perceptual::{PerceptualReference, ReferenceMemory, ScoreTarget}, still::{Effort, EncodingMode, Error, ErrorKind, Format, JpegSampling, Quality, SamplePreservation, StillImage}};

/// Requested fidelity contract.
#[derive(Clone, Copy, Debug)]
pub enum Fidelity {
    /// Minimum measured SSIMULACRA2 score.
    Score(ScoreTarget),
    /// Direct encoder control without a score guarantee.
    Quality(Quality),
    /// Exact expanded decoded samples and color semantics.
    Lossless,
}
/// Output selection for optimization.
#[derive(Clone, Copy, Debug)]
pub enum FormatSelection {
    /// Retain source format.
    Source,
    /// Choose the smallest qualifying evaluated compatible format.
    Auto,
    /// Optimize to a concrete format while retaining the never-bigger rule.
    Exact(Format),
}
/// Whether output must shrink or must honor a conversion.
#[derive(Clone, Copy, Debug)]
pub enum Operation {
    /// Require a strictly smaller output, or compliant original-byte identity.
    Optimize(FormatSelection),
    /// Produce the requested format regardless of size.
    Convert(Format),
}
/// Metadata handling independent of fidelity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetadataPolicy {
    /// Preserve representable metadata and warn about cross-format losses.
    Keep,
    /// Remove private metadata and normalize orientation once.
    Strip,
}
/// One image's processing request. Lossless and resizing conflict.
#[derive(Clone, Copy, Debug)]
pub struct Request {
    /// Operation and destination selection.
    pub operation: Operation,
    /// Fidelity requirement.
    pub fidelity: Fidelity,
    /// Work level and total encode cap.
    pub effort: Effort,
    /// Metadata requirement.
    pub metadata: MetadataPolicy,
    /// Optional aspect-preserving maximum side; never enlarges.
    pub max_dimension: Option<NonZeroU32>,
    /// Scoring reference memory; results are identical in either mode.
    pub scoring_memory: ReferenceMemory,
}
impl Default for Request {
    fn default() -> Self { Self { operation: Operation::Optimize(FormatSelection::Source), fidelity: Fidelity::Score(ScoreTarget::default()), effort: Effort::Balanced, metadata: MetadataPolicy::Keep, max_dimension: None, scoring_memory: ReferenceMemory::Fast } }
}
/// One actual encode attempt, including unsuccessful or disqualified attempts.
#[derive(Clone, Debug)]
pub struct Evaluation {
    /// Attempted output format.
    pub format: Format,
    /// None denotes exact encoding.
    pub quality: Option<f32>,
    /// JPEG chroma sampling chosen by the JPEG score search; None elsewhere.
    pub sampling: Option<JpegSampling>,
    /// Encoded length, if encoding succeeded.
    pub bytes: Option<usize>,
    /// Actual decoded score, when measured.
    pub score: Option<f64>,
    /// Wall time for encoding only; diagnostic, not a benchmark.
    pub encode_ms: f64,
    /// Wall time spent on the Fast lossless floor probe, when one ran.
    pub probe_ms: Option<f64>,
    /// Named encode, validation, or fidelity failure.
    pub failure: Option<String>,
}
/// Verified winner buffered before publication.
pub struct Outcome {
    /// Complete encoded output, or the compliant original bytes.
    pub bytes: Vec<u8>,
    /// Actual output format.
    pub format: Format,
    /// Source bytes were selected as identity rather than an encode.
    pub unchanged: bool,
    /// Actual measured score; absent for unsupported scoring or exact modes.
    pub score: Option<f64>,
    /// Attempts in stable evaluation order.
    pub evaluations: Vec<Evaluation>,
    /// Explicit metadata losses for cross-format conversion.
    pub warnings: Vec<String>,
}

fn expanded(image: &StillImage) -> Vec<[u16; 4]> {
    let samples = image.samples();
    let channels = image.pixels.descriptor().channels();
    let values: Vec<u16> = if image.bit_depth() == 8 { samples.iter().map(|&v| v as u16).collect() }
        else { samples.chunks_exact(2).map(|p| u16::from_ne_bytes([p[0],p[1]])).collect() };
    let opaque = if image.bit_depth() == 8 { 255 } else { 65535 };
    values.chunks_exact(channels).map(|p| {
        let rgb = if image.is_grayscale() { [p[0]; 3] } else { [p[0],p[1],p[2]] };
        [rgb[0],rgb[1],rgb[2],if image.has_alpha() { p[channels-1] } else { opaque }]
    }).collect()
}
fn required_png_color(image:&StillImage)->Vec<&[u8]> {
    let mut facts:Vec<_>=image.png_metadata.iter().filter(|chunk|match &chunk[4..8] {
        b"cLLI"|b"mDCV"=>true,
        b"cICP"=>chunk.get(8..12)!=Some(&[1,13,0,1])||image.metadata.icc_profile.is_some(),
        b"gAMA"|b"cHRM"=>image.metadata.icc_profile.is_none(),
        _=>false,
    }).map(Vec::as_slice).collect();
    facts.sort_by(|a,b|a[4..8].cmp(&b[4..8]));facts
}
/// Exact expanded samples, precision, hidden RGB, orientation and required color facts.
pub fn samples_equal(first: &StillImage, second: &StillImage) -> bool {
    first.dimensions() == second.dimensions() && first.bit_depth() == second.bit_depth()
        && first.metadata.orientation == second.metadata.orientation
        && first.metadata.icc_profile == second.metadata.icc_profile
        && required_png_color(first)==required_png_color(second)
        && if first.pixels.descriptor() == second.pixels.descriptor() { first.samples() == second.samples() } else { expanded(first) == expanded(second) }
}
fn compatible(image: &StillImage, format: Format, fidelity: Fidelity) -> bool {
    if image.bit_depth() > 8 { return format == Format::Png && matches!(fidelity, Fidelity::Lossless); }
    match format {
        Format::Jpeg => !matches!(fidelity, Fidelity::Lossless) && expanded(image).iter().all(|p| p[3] == 255),
        Format::Png => !(matches!(fidelity,Fidelity::Quality(_)) && image.is_grayscale()),
        Format::Webp => true,
        Format::Avif => !image.has_alpha() && !image.is_grayscale() && !matches!(fidelity, Fidelity::Lossless),
    }
}
fn better(size: usize, score: Option<f64>, format: Format, best: &Outcome) -> bool {
    size < best.bytes.len() || (size == best.bytes.len()
        && (score.unwrap_or(f64::NEG_INFINITY) > best.score.unwrap_or(f64::NEG_INFINITY)
        || (score == best.score && format < best.format)))
}
fn refine_step(formats: &[Format], evaluations: &[Evaluation], settings: &mut Vec<(Format,Option<f32>)>) -> bool {
    let mut refined = false;
    for &f in formats {
        let mut qualities: Vec<f32> = evaluations.iter().filter(|e| e.format == f).filter_map(|e|e.quality).collect();
        qualities.sort_by(f32::total_cmp); qualities.dedup();
        let pass = |q: f32| evaluations.iter().any(|e| e.format == f && e.quality == Some(q) && e.failure.is_none());
        let center = evaluations.iter().filter(|e|e.format==f && e.failure.is_none()).min_by_key(|e|e.bytes.unwrap_or(usize::MAX)).and_then(|e|e.quality).unwrap_or(75.0);
        let quantized = |q: f32| StillImage::quantized_quality(f, Quality::new(q).unwrap());
        let converges = matches!(f, Format::Avif | Format::Png);
        let resolvable = |lo: f32, hi: f32| if converges { quantized(lo).zip(quantized(hi)).is_some_and(|(a,b)| a.abs_diff(b) > 1) } else { hi-lo >= 0.5 };
        let mut straddle: Vec<(f32,f32)> = qualities.windows(2).filter(|p|resolvable(p[0],p[1]) && pass(p[0]) != pass(p[1])).map(|p|(p[0],p[1])).collect();
        if straddle.is_empty() && converges { continue; }
        if straddle.is_empty() {
            let mut gaps: Vec<_> = qualities.windows(2).filter(|p|p[1]-p[0]>=0.5).map(|p|((p[0]+p[1])*0.5,((p[0]+p[1])*0.5-center).abs())).collect();
            gaps.sort_by(|a,b|a.1.total_cmp(&b.1).then(a.0.total_cmp(&b.0)));
            if let Some(&(q,_))=gaps.first() { settings.push((f,Some(q))); refined=true; }
        } else {
            straddle.sort_by(|a,b|(b.1-b.0).total_cmp(&(a.1-a.0)).then(((a.0+a.1)*0.5-center).abs().total_cmp(&((b.0+b.1)*0.5-center).abs())));
            let (lo,hi)=straddle[0];
            settings.push((f,Some((lo+hi)*0.5)));
            refined=true;
        }
    }
    refined
}
fn jpeg_refine_step(target: f64, effort: Effort, samplings: &[JpegSampling], evaluations: &[Evaluation], settings: &mut Vec<(f32,JpegSampling)>) -> bool {
    let mut seed = target.clamp(1.0, 100.0) as f32;
    for &sampling in samplings {
        let attempts: Vec<_> = evaluations.iter().filter(|e| e.format == Format::Jpeg && e.sampling == Some(sampling)).collect();
        let Some(points) = attempts.iter().map(|e| Some((e.quality?, e.score?))).collect::<Option<Vec<(f32,f64)>>>() else { return false };
        if points.is_empty() { settings.push((seed, sampling)); return true; }
        if let Some(next) = jpeg_next_quality(target, effort, &points) { settings.push((next, sampling)); return true; }
        if let Some(pass) = points.iter().filter(|p| p.1 >= target).map(|p| p.0).min_by(f32::total_cmp) { seed = pass; }
    }
    false
}
fn jpeg_next_quality(target: f64, effort: Effort, points: &[(f32,f64)]) -> Option<f32> {
    let (slack, width) = match effort { Effort::Fast => (0.5, 1.0), Effort::Balanced => (0.15, 0.25), Effort::Best => (0.05, 0.1) };
    let goal = target + slack * 0.5;
    let pass = points.iter().filter(|p| p.1 >= target).min_by(|a,b| a.0.total_cmp(&b.0)).copied();
    let fail = points.iter().filter(|p| p.1 < target && pass.is_none_or(|h| p.0 < h.0)).max_by(|a,b| a.0.total_cmp(&b.0)).copied();
    if pass.is_some_and(|h| h.1 - target <= slack) { return None; }
    let slope = |near: (f32,f64)| {
        let other = points.iter().filter(|p| p.0 != near.0).min_by(|a,b| (a.0-near.0).abs().total_cmp(&(b.0-near.0).abs()));
        other.map(|o| (o.1 - near.1) / f64::from(o.0 - near.0)).filter(|m| *m > 0.05).unwrap_or(0.6)
    };
    let next = match (fail, pass) {
        (Some(lo), Some(hi)) => {
            if hi.0 - lo.0 <= width { return None; }
            let span = f64::from(hi.0 - lo.0);
            let t = if hi.1 > lo.1 { ((goal - lo.1) / (hi.1 - lo.1)).clamp(0.1, 0.9) } else { 0.5 };
            lo.0 + (t * span) as f32
        },
        (None, Some(hi)) if hi.0 > 1.0 => (hi.0 - ((hi.1 - goal) / slope(hi) * 1.2) as f32).max(1.0),
        (Some(lo), None) if lo.0 < 100.0 => (lo.0 + ((goal - lo.1) / slope(lo) * 1.2) as f32).min(100.0),
        _ => return None,
    };
    (!points.iter().any(|p| (p.0 - next).abs() < 0.01)).then_some(next)
}
fn strip_compliant(bytes: &[u8], format: Format) -> bool {
    match format {
        Format::Png => {
            let mut at = 8;
            while let Some(header) = bytes.get(at..at+8) {
                let len = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
                if !matches!(&header[4..8], b"IHDR"|b"PLTE"|b"IDAT"|b"IEND"|b"tRNS") && !crate::still::png::is_color_chunk(&header[4..8]) { return false; }
                at += len+12;
            }
            at == bytes.len()
        },
        Format::Jpeg => false,
        Format::Webp => {
            let mut at = 12;
            while let Some(header) = bytes.get(at..at+8) {
                if !matches!(&header[..4], b"VP8 "|b"VP8L"|b"VP8X"|b"ALPH"|b"ICCP") { return false; }
                let len = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
                at += 8 + len + (len&1);
            }
            at == bytes.len()
        },
        Format::Avif => false,
    }
}
fn local_delta_entropy_bits(samples: &[u8], bytes_per_pixel: usize) -> f64 {
    let step = (samples.len() / (bytes_per_pixel * 65536)).max(1);
    let mut hist = [0u64; 256];
    let mut n = 0u64;
    let mut prev: Option<&[u8]> = None;
    let mut i = 0usize;
    while i + bytes_per_pixel <= samples.len() {
        let px = &samples[i..i + bytes_per_pixel];
        if let Some(p) = prev {
            for c in 0..bytes_per_pixel { hist[px[c].wrapping_sub(p[c]) as usize] += 1; n += 1; }
        }
        prev = Some(px);
        i += bytes_per_pixel * step;
    }
    let mut h = 0.0f64;
    for &c in &hist {
        if c > 0 { let p = c as f64 / n as f64; h -= p * p.log2(); }
    }
    h
}
fn lossless_unlikely_to_win(image: &StillImage, cancelled: &crate::cancellation::Cancellation, incumbent: usize) -> bool {
    if image.bit_depth() != 8 { return false; }
    let bytes_per_pixel = if image.is_grayscale() { if image.has_alpha() { 2 } else { 1 } } else if image.has_alpha() { 4 } else { 3 };
    if local_delta_entropy_bits(&image.samples(), bytes_per_pixel) < 3.5 { return false; }
    crate::still::webp::probe(image, cancelled).is_ok_and(|p| p.len().saturating_mul(3) > incumbent.saturating_mul(5))
}
fn warnings(image: &StillImage, format: Format, policy: MetadataPolicy) -> Vec<String> {
    if policy == MetadataPolicy::Keep && format != image.format() {
        image.png_metadata.iter().filter(|p| !matches!(&p[4..8], b"iCCP"|b"eXIf") && !( &p[4..8] == b"iTXt" && image.metadata.xmp.is_some()))
            .map(|p| format!("cross-format metadata loss: PNG {}", String::from_utf8_lossy(&p[4..8]))).collect()
    } else { vec![] }
}
fn validate_metadata(source: &StillImage, candidate: &StillImage, format: Format) -> Result<(), Error> {
    let icc_equal = source.metadata.icc_profile == candidate.metadata.icc_profile
        || (format == Format::Avif && candidate.metadata.icc_profile.is_none()
            && source.metadata.icc_profile.as_deref().is_some_and(zenpixels::icc::is_common_srgb));
    if !icc_equal || source.metadata.orientation != candidate.metadata.orientation
        || source.metadata.exif != candidate.metadata.exif || source.metadata.xmp != candidate.metadata.xmp {
        return Err(Error::new(ErrorKind::Unsupported, "destination cannot preserve ICC, orientation, EXIF or XMP semantics"));
    }
    if format != Format::Png && !required_png_color(source).is_empty() {
        return Err(Error::new(ErrorKind::Unsupported, "cross-format conversion cannot preserve PNG color signaling without ICC"));
    }
    Ok(())
}
/// Process one bounded source. This function performs no publication.
pub fn process(bytes: &[u8], request: Request) -> Result<Outcome, Error> {
    process_cancellable(bytes, request, &crate::cancellation::Cancellation::default())
}
/// Process with cooperative cancellation between encode attempts.
pub fn process_cancellable(bytes: &[u8], request: Request, cancelled: &crate::cancellation::Cancellation) -> Result<Outcome, Error> {
    if cancelled.is_cancelled() { return Err(Error::new(ErrorKind::Invalid,"processing cancelled")); }
    if matches!(request.fidelity, Fidelity::Lossless) && request.max_dimension.is_some() { return Err(Error::new(ErrorKind::Invalid, "lossless conflicts with resizing")); }
    let mut image = StillImage::decode_cancellable(bytes,cancelled)?;
    let source_format = image.format();
    let (width,height)=image.dimensions();
    let source_eligible = request.max_dimension.is_none_or(|maximum|width.max(height)<=maximum.get())
        && (request.metadata == MetadataPolicy::Keep || strip_compliant(bytes, source_format));
    if request.metadata == MetadataPolicy::Strip {
        image = image.strip_metadata(if matches!(request.fidelity,Fidelity::Lossless) { SamplePreservation::Exact } else { SamplePreservation::AllowReorientation })?;
    }
    if let Some(dimension) = request.max_dimension { image = image.resize_to_fit(dimension)?; }
    let metric = match request.fidelity { Fidelity::Lossless => None, _ => Some(PerceptualReference::new_cancellable(&image,request.scoring_memory,cancelled)?) };
    let identity_score = metric.as_ref().map(|_|100.0);
    let optimize = matches!(request.operation, Operation::Optimize(_));
    let formats = match request.operation {
        Operation::Optimize(FormatSelection::Source) => vec![source_format],
        Operation::Optimize(FormatSelection::Auto) => vec![Format::Jpeg,Format::Png,Format::Webp,Format::Avif],
        Operation::Optimize(FormatSelection::Exact(f)) | Operation::Convert(f) => vec![f],
    };
    if matches!(request.fidelity,Fidelity::Quality(_)) && formats==[Format::Png] && image.is_grayscale() {
        return Err(Error::new(ErrorKind::Unsupported,"direct palette quality requires an eight-bit RGB or RGBA representation"));
    }
    let mut best = if optimize && source_eligible && formats.contains(&source_format) {
        Some(Outcome { bytes:bytes.to_vec(), format:source_format, unchanged:true, score:identity_score, evaluations:vec![], warnings:vec![] })
    } else { None };
    if matches!(request.fidelity,Fidelity::Lossless) && source_format == Format::Jpeg && formats == [Format::Jpeg] {
        if source_eligible { return Ok(Outcome { bytes:bytes.to_vec(),format:source_format,unchanged:true,score:None,evaluations:vec![],warnings:vec![] }); }
        return Err(Error::new(ErrorKind::Unsupported,"JPEG lossless byte copy cannot satisfy metadata stripping or transformation"));
    }
    let formats: Vec<_> = formats.into_iter().filter(|&f| compatible(&image,f,request.fidelity)).collect();
    if formats.is_empty() { return best.ok_or_else(|| Error::new(ErrorKind::Unsupported,"no compatible destination for the requested precision, alpha or lossless contract")); }
    let cap = request.effort.candidate_cap();
    let jpeg_only = formats == [Format::Jpeg];
    let mut settings = Vec::new();
    let mut deferred: Vec<(Format,Option<f32>)> = Vec::new();
    let jpeg_samplings: &[JpegSampling] = if image.is_grayscale() || request.effort == Effort::Fast { &[JpegSampling::Yuv444] } else { &[JpegSampling::Yuv444, JpegSampling::Yuv420] };
    let mut jpeg_settings: Vec<(f32,JpegSampling)> = Vec::new();
    let mut jpeg_next = 0;
    let mut progressive_repack: Option<(f32,JpegSampling)> = None;
    let mut repacked = false;
    match request.fidelity {
        Fidelity::Lossless => for &f in &formats { settings.push((f,None)); },
        Fidelity::Quality(q) => for &f in &formats { settings.push((f,Some(q.value()))); },
        Fidelity::Score(_) if jpeg_only => {},
        Fidelity::Score(_) => {
            let mut exact = vec![];
            for format in [Format::Png,Format::Webp] {if formats.contains(&format) {exact.push((format,None));}}
            let mut scored = vec![];
            for q in [95.0,50.0,0.0,100.0,75.0,25.0] {
                for &f in &formats { if f != Format::Png || !image.is_grayscale() { scored.push((f,Some(q))); } }
            }
            if exact.len() + scored.len() <= cap { deferred = exact; settings = scored; }
            else { settings = exact.into_iter().chain(scored).collect(); }
        },
    }
    let mut evaluations = Vec::new();
    let mut encoded_any = false;
    let mut encode_error = false;
    let mut next = 0;
    let mut deferred_at = 0;
    let fast_defer = request.effort == Effort::Fast && !deferred.is_empty();
    let scored_budget = if fast_defer { cap.saturating_sub(deferred.len()) } else { cap };
    let mut scored_evals = 0usize;
    'eval: while evaluations.len() < cap {
        if cancelled.is_cancelled() { return Err(Error::new(ErrorKind::Invalid,"processing cancelled")); }
        let mut repack_attempt = false;
        let (format,quality,from_deferred,sampling) = loop {
            if let Some((q,s)) = progressive_repack.take() { scored_evals+=1; repack_attempt=true; break (Format::Jpeg,Some(q),false,Some(s)); }
            if jpeg_next < jpeg_settings.len() && scored_evals < scored_budget { jpeg_next+=1; scored_evals+=1; let (q,s)=jpeg_settings[jpeg_next-1]; break (Format::Jpeg,Some(q),false,Some(s)); }
            if next < settings.len() && scored_evals < scored_budget { next+=1; scored_evals+=1; break (settings[next-1].0,settings[next-1].1,false,None); }
            if !matches!(request.fidelity,Fidelity::Score(_)) { break 'eval; }
            if deferred_at < deferred.len() && !fast_defer { deferred_at+=1; let s=deferred[deferred_at-1]; break (s.0,s.1,true,None); }
            if scored_evals < scored_budget && match (jpeg_only, request.fidelity) {
                (true, Fidelity::Score(target)) => jpeg_refine_step(target.value(), request.effort, jpeg_samplings, &evaluations, &mut jpeg_settings) || (request.effort == Effort::Fast && !repacked && {
                    repacked = true;
                    progressive_repack = evaluations.iter().filter(|e| e.format == Format::Jpeg && e.failure.is_none()).min_by_key(|e| e.bytes).and_then(|e| Some((e.quality?, e.sampling?)));
                    progressive_repack.is_some()
                }),
                _ => refine_step(&formats,&evaluations,&mut settings),
            } { continue; }
            if deferred_at < deferred.len() { deferred_at+=1; let s=deferred[deferred_at-1]; break (s.0,s.1,true,None); }
            break 'eval;
        };
        let start=Instant::now();
        let incumbent = best.as_ref().map(|b| b.bytes.len());
        let deferred_lossless = from_deferred && format == Format::Webp && quality.is_none();
        let fast_floor_probe = deferred_lossless && request.effort == Effort::Fast && incumbent.is_some()
            && image.bit_depth() == 8 && {
                let bpp = if image.is_grayscale() { if image.has_alpha() { 2 } else { 1 } } else if image.has_alpha() { 4 } else { 3 };
                local_delta_entropy_bits(&image.samples(), bpp) >= 2.2
            };
        let mut probe_ms = None;
        let mut probe_bytes;
        let mut encode_ms = 0.0;
        let encoded = if fast_floor_probe {
            match crate::still::webp::probe(&image,cancelled) {
                Ok(bytes) => {
                    probe_ms = Some(start.elapsed().as_secs_f64()*1000.0);
                    if bytes.len().saturating_mul(4) > incumbent.unwrap().saturating_mul(5) {
                        encode_ms = probe_ms.unwrap();
                        Some(Ok(bytes))
                    } else {
                        probe_bytes = Some(bytes);
                        let t = Instant::now();
                        let r = image.encode_cancellable(format,EncodingMode::Lossless,request.effort,cancelled);
                        encode_ms = t.elapsed().as_secs_f64()*1000.0;
                        Some(match r { Err(_) => probe_bytes.take().map_or(r, Ok), Ok(b) => Ok(b) })
                    }
                },
                Err(_) => {
                    let t = Instant::now();
                    let r = image.encode_cancellable(format,EncodingMode::Lossless,request.effort,cancelled);
                    encode_ms = t.elapsed().as_secs_f64()*1000.0;
                    Some(r)
                },
            }
        } else {
            let gated = deferred_lossless && incumbent.is_some_and(|n| lossless_unlikely_to_win(&image,cancelled,n));
            if gated { None } else {
                let t = Instant::now();
                let r = match (sampling, quality) {
                    (Some(sampling), Some(q)) => image.encode_jpeg_cancellable(Quality::new(q).unwrap(), sampling, if repack_attempt { Effort::Balanced } else { request.effort }, cancelled),
                    _ => image.encode_cancellable(format,quality.map(|q|EncodingMode::Quality(Quality::new(q).unwrap())).unwrap_or(EncodingMode::Lossless),request.effort,cancelled),
                };
                encode_ms = t.elapsed().as_secs_f64()*1000.0;
                Some(r)
            }
        };
        let mut evaluation=Evaluation { format,quality,sampling,bytes:None,score:None,encode_ms,probe_ms,failure:None };
        match encoded {
            None => evaluation.failure=Some("skipped: lossless floor probe is far larger than the incumbent".into()),
            Some(Err(error)) => { encode_error |= error.kind() == ErrorKind::Encode; evaluation.failure=Some(error.to_string()); },
            Some(Ok(candidate_bytes)) => {
                encoded_any=true; evaluation.bytes=Some(candidate_bytes.len());
                let validated=(|| {
                    let candidate=StillImage::decode_cancellable(&candidate_bytes,cancelled)?;
                    validate_metadata(&image,&candidate,format)?;
                    if matches!(request.fidelity,Fidelity::Lossless) {
                        if !samples_equal(&image,&candidate) { return Err(Error::new(ErrorKind::Score,"exact decoded samples differ")); }
                        Ok(None)
                    } else {
                        let score=if samples_equal(&image,&candidate) {100.0} else {metric.as_ref().unwrap().score_decoded(&candidate)?};
                        evaluation.score=Some(score);
                        if let Fidelity::Score(target)=request.fidelity && score < target.value() { return Err(Error::new(ErrorKind::Score,"candidate is below requested score")); }
                        Ok(Some(score))
                    }
                })();
                match validated {
                    Err(error) => { encode_error |= error.kind() == ErrorKind::Encode; evaluation.failure=Some(error.to_string()); },
                    Ok(score) => if (!optimize || candidate_bytes.len() < bytes.len()) && best.as_ref().is_none_or(|b|better(candidate_bytes.len(),score,format,b)) {
                        best=Some(Outcome { bytes:candidate_bytes,format,unchanged:false,score,evaluations:vec![],warnings:warnings(&image,format,request.metadata) });
                    },
                }
            },
        }
        evaluations.push(evaluation);
    }
    let mut best=best.ok_or_else(|| Error::new(if encoded_any { ErrorKind::Score } else if encode_error { ErrorKind::Encode } else { ErrorKind::Unsupported },format!("no qualifying output: {}",evaluations.iter().filter_map(|e|e.failure.as_deref()).next().unwrap_or("no compatible candidates"))))?;
    best.evaluations=evaluations;
    Ok(best)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn outcome(size: usize, score: Option<f64>, format: Format) -> Outcome {
        Outcome { bytes: vec![0; size], format, unchanged: false, score, evaluations: vec![], warnings: vec![] }
    }
    fn eval(format: Format, quality: Option<f32>, bytes: usize, score: Option<f64>, failure: Option<&str>) -> Evaluation {
        Evaluation { format, quality, sampling: None, bytes: Some(bytes), score, encode_ms: 0.0, probe_ms: None, failure: failure.map(str::to_string) }
    }
    #[test]
    fn better_prefers_size_then_score_then_format_then_incumbent() {
        let best = outcome(100, Some(90.0), Format::Webp);
        assert!(better(99, None, Format::Avif, &best));
        assert!(better(100, Some(95.0), Format::Avif, &best));
        assert!(better(100, Some(90.0), Format::Png, &best));
        assert!(!better(100, Some(90.0), Format::Webp, &best));
        assert!(!better(100, Some(89.0), Format::Jpeg, &best));
        assert!(!better(100, None, Format::Webp, &best));
        assert!(better(100, None, Format::Png, &outcome(100, None, Format::Webp)));
    }
    #[test]
    fn refine_step_centers_gap_choice_on_the_smallest_successful_candidate() {
        let mut evaluations = vec![
            eval(Format::Webp, None, 50, Some(100.0), None),
            eval(Format::Webp, Some(0.0), 100, Some(10.0), None),
            eval(Format::Webp, Some(50.0), 160, Some(50.0), None),
            eval(Format::Webp, Some(95.0), 200, Some(90.0), None),
        ];
        let mut settings = vec![];
        assert!(refine_step(&[Format::Webp], &evaluations, &mut settings));
        assert_eq!(settings, vec![(Format::Webp, Some(72.5))]);
        evaluations.remove(0);
        let mut settings = vec![];
        assert!(refine_step(&[Format::Webp], &evaluations, &mut settings));
        assert_eq!(settings, vec![(Format::Webp, Some(25.0))]);
    }
    #[test]
    fn jpeg_refine_step_brackets_then_lands_within_effort_slack() {
        let curves: [fn(f32) -> f64; 3] = [|q| 20.0 + 0.75 * f64::from(q), |q| 100.0 - 900.0 / (f64::from(q) + 10.0), |q| 30.0 + 60.0 * (f64::from(q) / 100.0).powi(3)];
        for (curve, target) in curves.into_iter().zip([80.0, 90.0, 85.0]) {
            for (effort, slack) in [(Effort::Fast, 0.5), (Effort::Balanced, 0.15), (Effort::Best, 0.05)] {
                let mut settings = vec![];
                let mut evaluations: Vec<Evaluation> = vec![];
                while evaluations.len() < effort.candidate_cap() {
                    if evaluations.len() == settings.len() && !jpeg_refine_step(target, effort, &[JpegSampling::Yuv444], &evaluations, &mut settings) { break; }
                    let (q, sampling) = settings[evaluations.len()];
                    let score = curve(q);
                    evaluations.push(Evaluation { sampling: Some(sampling), ..eval(Format::Jpeg, Some(q), 1, Some(score), (score < target).then_some("below")) });
                }
                let best = evaluations.iter().filter_map(|e| e.score).filter(|&s| s >= target).fold(f64::INFINITY, f64::min);
                assert!(best - target <= slack || evaluations.len() == effort.candidate_cap(), "{effort:?} {target} {best} {}", evaluations.len());
                assert!(evaluations.len() <= 6, "{effort:?} {target} used {}", evaluations.len());
            }
        }
    }
    #[test]
    fn jpeg_refine_step_searches_each_sampling_and_seeds_the_next_from_the_first() {
        let samplings = [JpegSampling::Yuv444, JpegSampling::Yuv420];
        let score = |q: f32, sampling| 20.0 + 0.75 * f64::from(q) - if sampling == JpegSampling::Yuv420 { 3.0 } else { 0.0 };
        let mut settings = vec![];
        let mut evaluations: Vec<Evaluation> = vec![];
        while evaluations.len() < Effort::Balanced.candidate_cap() {
            if evaluations.len() == settings.len() && !jpeg_refine_step(80.0, Effort::Balanced, &samplings, &evaluations, &mut settings) { break; }
            let (q, sampling) = settings[evaluations.len()];
            let s = score(q, sampling);
            evaluations.push(Evaluation { sampling: Some(sampling), ..eval(Format::Jpeg, Some(q), 1, Some(s), (s < 80.0).then_some("below")) });
        }
        let first_420 = evaluations.iter().position(|e| e.sampling == Some(JpegSampling::Yuv420)).unwrap();
        let passing_444 = evaluations[..first_420].iter().filter(|e| e.failure.is_none()).filter_map(|e| e.quality).fold(f32::INFINITY, f32::min);
        assert_eq!(evaluations[first_420].quality, Some(passing_444));
        for sampling in samplings {
            let best = evaluations.iter().filter(|e| e.sampling == Some(sampling)).filter_map(|e| e.score).filter(|&s| s >= 80.0).fold(f64::INFINITY, f64::min);
            assert!(best - 80.0 <= 0.15, "{sampling:?} {best}");
        }
    }
    #[test]
    fn refine_step_bisects_the_widest_passing_failing_straddle() {
        let evaluations = vec![
            eval(Format::Webp, Some(0.0), 300, Some(10.0), Some("below")),
            eval(Format::Webp, Some(50.0), 200, Some(50.0), None),
            eval(Format::Webp, Some(95.0), 190, Some(90.0), None),
            eval(Format::Webp, Some(100.0), 180, Some(99.0), Some("below")),
        ];
        let mut settings = vec![];
        assert!(refine_step(&[Format::Webp], &evaluations, &mut settings));
        assert_eq!(settings, vec![(Format::Webp, Some(25.0))]);
    }
    #[test]
    fn refine_step_returns_false_without_a_usable_gap() {
        let evaluations = vec![eval(Format::Webp, Some(50.0), 100, Some(50.0), None)];
        let mut settings = vec![];
        assert!(!refine_step(&[Format::Webp], &evaluations, &mut settings));
        assert!(settings.is_empty());
    }
}
