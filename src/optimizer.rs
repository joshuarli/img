//! Bounded, deterministic selection over verified encoded still images.
use std::{num::NonZeroU32, time::Instant};
use crate::{perceptual::{PerceptualReference, ScoreTarget}, still::{Effort, EncodingMode, Error, ErrorKind, Format, Quality, SamplePreservation, StillImage}};

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
}
impl Default for Request {
    fn default() -> Self { Self { operation: Operation::Optimize(FormatSelection::Source), fidelity: Fidelity::Score(ScoreTarget::default()), effort: Effort::Balanced, metadata: MetadataPolicy::Keep, max_dimension: None } }
}
/// One actual encode attempt, including unsuccessful or disqualified attempts.
#[derive(Clone, Debug)]
pub struct Evaluation {
    /// Attempted output format.
    pub format: Format,
    /// None denotes exact encoding.
    pub quality: Option<f32>,
    /// Encoded length, if encoding succeeded.
    pub bytes: Option<usize>,
    /// Actual decoded score, when measured.
    pub score: Option<f64>,
    /// Wall time for encoding only; diagnostic, not a benchmark.
    pub encode_ms: f64,
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
        && expanded(first) == expanded(second)
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
fn warnings(image: &StillImage, format: Format, policy: MetadataPolicy) -> Vec<String> {
    if policy == MetadataPolicy::Keep && format != image.format() {
        image.png_metadata.iter().filter(|p| !matches!(&p[4..8], b"iCCP"|b"eXIf") && !( &p[4..8] == b"iTXt" && image.metadata.xmp.is_some()))
            .map(|p| format!("cross-format metadata loss: PNG {}", String::from_utf8_lossy(&p[4..8]))).collect()
    } else { vec![] }
}
fn validate_metadata(source: &StillImage, candidate: &StillImage, format: Format) -> Result<(), Error> {
    if source.metadata.icc_profile != candidate.metadata.icc_profile || source.metadata.orientation != candidate.metadata.orientation
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
    let source_eligible = request.max_dimension.is_none() && (request.metadata == MetadataPolicy::Keep || strip_compliant(bytes, source_format));
    if request.metadata == MetadataPolicy::Strip {
        image = image.strip_metadata(if matches!(request.fidelity,Fidelity::Lossless) { SamplePreservation::Exact } else { SamplePreservation::AllowReorientation })?;
    }
    if let Some(dimension) = request.max_dimension { image = image.resize_to_fit(dimension)?; }
    let metric = match request.fidelity { Fidelity::Lossless => None, _ => Some(PerceptualReference::new_cancellable(&image,cancelled)?) };
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
    let mut settings = Vec::new();
    match request.fidelity {
        Fidelity::Lossless => for &f in &formats { settings.push((f,None)); },
        Fidelity::Quality(q) => for &f in &formats { settings.push((f,Some(q.value()))); },
        Fidelity::Score(_) => {
            for format in [Format::Png,Format::Webp] {if formats.contains(&format) {settings.push((format,None));}}
            for q in [95.0,50.0,0.0,100.0,75.0,25.0] {
                for &f in &formats { if f != Format::Png || !image.is_grayscale() { settings.push((f,Some(q))); } }
            }
        },
    }
    let mut evaluations = Vec::new();
    let mut encoded_any = false;
    let mut encode_error = false;
    let mut next = 0;
    while evaluations.len() < cap {
        if cancelled.is_cancelled() { return Err(Error::new(ErrorKind::Invalid,"processing cancelled")); }
        if next >= settings.len() {
            if !matches!(request.fidelity,Fidelity::Score(_)) { break; }
            let mut refined = false;
            for &f in &formats {
                let mut qualities: Vec<f32> = evaluations.iter().filter(|e: &&Evaluation| e.format == f).filter_map(|e|e.quality).collect();
                qualities.sort_by(f32::total_cmp); qualities.dedup();
                let center = evaluations.iter().filter(|e|e.format==f && e.failure.is_none()).min_by_key(|e|e.bytes.unwrap_or(usize::MAX)).and_then(|e|e.quality).unwrap_or(75.0);
                let mut gaps: Vec<_> = qualities.windows(2).filter(|p|p[1]-p[0]>=0.5).map(|p|((p[0]+p[1])*0.5,((p[0]+p[1])*0.5-center).abs())).collect();
                gaps.sort_by(|a,b|a.1.total_cmp(&b.1).then(a.0.total_cmp(&b.0)));
                if let Some(&(q,_))=gaps.first() { settings.push((f,Some(q))); refined=true; }
            }
            if !refined { break; }
        }
        let (format,quality) = settings[next]; next+=1;
        let start=Instant::now();
        let encoded=image.encode_cancellable(format,quality.map(|q|EncodingMode::Quality(Quality::new(q).unwrap())).unwrap_or(EncodingMode::Lossless),request.effort,cancelled);
        let mut evaluation=Evaluation { format,quality,bytes:None,score:None,encode_ms:start.elapsed().as_secs_f64()*1000.0,failure:None };
        match encoded {
            Err(error) => { encode_error |= error.kind() == ErrorKind::Encode; evaluation.failure=Some(error.to_string()); },
            Ok(candidate_bytes) => {
                encoded_any=true; evaluation.bytes=Some(candidate_bytes.len());
                let validated=(|| {
                    let candidate=StillImage::decode_cancellable(&candidate_bytes,cancelled)?;
                    validate_metadata(&image,&candidate,format)?;
                    if matches!(request.fidelity,Fidelity::Lossless) {
                        if !samples_equal(&image,&candidate) { return Err(Error::new(ErrorKind::Score,"exact decoded samples differ")); }
                        Ok(None)
                    } else {
                        let score=metric.as_ref().unwrap().score_encoded(&candidate_bytes)?;
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
