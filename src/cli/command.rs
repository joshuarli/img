use std::{ffi::OsString, path::PathBuf};

use lexopt::{Arg, Parser};

#[cfg(feature = "resize")]
use super::preprocessors::{ResizeFilter, ResizeValue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Help, Version, Directory, Recursive, Suffix, Backup, Threads, Strip, PreserveTimestamps,
    NoProgress, Quiet, PrintLimits,
    #[cfg(feature = "resize")] Resize,
    #[cfg(feature = "resize")] Downscale,
    #[cfg(feature = "resize")] NoDownscale,
    #[cfg(feature = "resize")] Upscale,
    #[cfg(feature = "resize")] NoUpscale,
    #[cfg(feature = "resize")] Filter,
    #[cfg(feature = "quantization")] Quantization,
    #[cfg(feature = "quantization")] Dithering,
    Premultiply, Quality, AlphaQuality, Speed, Colorspace, AlphaMode,
    ChromaQuality, Baseline, NoOptimizeCoding, Smoothing, Multipass, Subsample, Qtable,
    Interlace, Effort, Lossless, SlightLoss, Discrete, Exact, Progressive,
}

#[derive(Clone, Copy)]
pub enum ValueMode { Flag, Required(&'static str), Optional(&'static str) }

pub struct OptionSpec {
    pub key: Key,
    pub short: Option<char>,
    pub long: &'static str,
    pub alias: Option<&'static str>,
    pub value: ValueMode,
    pub repeatable: bool,
    pub help: &'static str,
    pub commands: &'static [&'static str],
}

macro_rules! option {
    ($key:ident, $short:expr, $long:literal, $alias:expr, $value:expr, $repeat:expr, $help:literal, $commands:expr) => {
        OptionSpec { key: Key::$key, short: $short, long: $long, alias: $alias, value: $value, repeatable: $repeat, help: $help, commands: $commands }
    };
}

pub const OPTIONS: &[OptionSpec] = &[
    option!(Help, Some('h'), "help", None, ValueMode::Flag, false, "Print help", &[]),
    option!(Version, Some('V'), "version", None, ValueMode::Flag, false, "Print version", &[]),
    option!(Directory, Some('d'), "directory", None, ValueMode::Required("DIR"), false, "Write output files to this directory", &[]),
    option!(Recursive, Some('r'), "recursive", None, ValueMode::Flag, false, "Preserve input directory structure", &[]),
    option!(Suffix, Some('s'), "suffix", None, ValueMode::Optional("SUFFIX"), false, "Append a suffix (default when present: updated)", &[]),
    option!(Backup, Some('b'), "backup", None, ValueMode::Flag, false, "Keep an @backup copy of each input", &[]),
    option!(Threads, Some('t'), "threads", None, ValueMode::Required("NUM"), false, "Maximum concurrent images (default: 1)", &[]),
    option!(Strip, Some('x'), "strip", None, ValueMode::Flag, false, "Strip embedded metadata", &[]),
    option!(PreserveTimestamps, Some('p'), "preserve-timestamps", None, ValueMode::Flag, false, "Preserve input modification times", &[]),
    option!(NoProgress, None, "no-progress", None, ValueMode::Flag, false, "Disable progress bars", &[]),
    option!(Quiet, None, "quiet", None, ValueMode::Flag, false, "Suppress output summaries", &[]),
    option!(PrintLimits, None, "print-limits", None, ValueMode::Flag, false, "Print runtime size limits", &[]),
    #[cfg(feature = "resize")]
    option!(Resize, None, "resize", None, ValueMode::Required("RESIZE"), true, "Resize an image; may be repeated", &[]),
    #[cfg(feature = "resize")]
    option!(Downscale, None, "downscale", None, ValueMode::Flag, false, "Enable downscaling", &[]),
    #[cfg(feature = "resize")]
    option!(NoDownscale, None, "no-downscale", Some("enlarge-only"), ValueMode::Flag, false, "Disable downscaling", &[]),
    #[cfg(feature = "resize")]
    option!(Upscale, None, "upscale", None, ValueMode::Flag, false, "Enable upscaling", &[]),
    #[cfg(feature = "resize")]
    option!(NoUpscale, None, "no-upscale", Some("reduce-only"), ValueMode::Flag, false, "Disable upscaling", &[]),
    #[cfg(feature = "resize")]
    option!(Filter, None, "filter", None, ValueMode::Required("FILTER"), false, "Resize filter (default: lanczos3)", &[]),
    #[cfg(feature = "quantization")]
    option!(Quantization, None, "quantization", None, ValueMode::Optional("QUALITY"), true, "Reduce the color palette (default: 75)", &[]),
    #[cfg(feature = "quantization")]
    option!(Dithering, None, "dithering", None, ValueMode::Optional("QUALITY"), false, "Dither quantized colors (default: 75)", &[]),
    option!(Premultiply, None, "premultiply", None, ValueMode::Flag, true, "Premultiply alpha before the next operation", &[]),
    option!(Quality, Some('q'), "quality", None, ValueMode::Required("NUM"), false, "Encoder quality (1-100)", &["jpeg", "mozjpeg", "webp", "avif"]),
    option!(AlphaQuality, None, "alpha_quality", None, ValueMode::Required("NUM"), false, "AVIF alpha quality (1-100)", &["avif"]),
    option!(Speed, None, "speed", None, ValueMode::Required("NUM"), false, "AVIF speed (1-10; default: 6)", &["avif"]),
    option!(Colorspace, None, "colorspace", None, ValueMode::Required("COLOR"), false, "Output color space", &["avif", "mozjpeg"]),
    option!(AlphaMode, None, "alpha_mode", None, ValueMode::Required("MODE"), false, "AVIF alpha color mode", &["avif"]),
    option!(ChromaQuality, None, "chroma_quality", None, ValueMode::Required("NUM"), false, "JPEG chroma quality (1-100)", &["mozjpeg"]),
    option!(Baseline, None, "baseline", None, ValueMode::Flag, false, "Use baseline JPEG encoding", &["mozjpeg"]),
    option!(NoOptimizeCoding, None, "no_optimize_coding", None, ValueMode::Flag, false, "Disable JPEG coding optimization", &["mozjpeg"]),
    option!(Smoothing, None, "smoothing", None, ValueMode::Required("NUM"), false, "JPEG smoothing (1-100)", &["mozjpeg"]),
    option!(Multipass, None, "multipass", None, ValueMode::Flag, false, "Use multipass trellis quantization", &["mozjpeg"]),
    option!(Subsample, None, "subsample", None, ValueMode::Required("NUM"), false, "JPEG chroma subsampling (1-4)", &["mozjpeg"]),
    option!(Qtable, None, "qtable", None, ValueMode::Required("TABLE"), false, "JPEG quantization table (default: NRobidoux)", &["mozjpeg"]),
    option!(Interlace, None, "interlace", None, ValueMode::Flag, false, "Interlace PNG output", &["oxipng"]),
    option!(Effort, None, "effort", None, ValueMode::Required("NUM"), false, "PNG effort (0-6; default: 2)", &["oxipng"]),
    option!(Lossless, None, "lossless", None, ValueMode::Flag, false, "Use lossless WebP", &["webp"]),
    option!(SlightLoss, None, "slight_loss", None, ValueMode::Required("NUM"), false, "WebP near-lossless amount (0-100)", &["webp"]),
    option!(Discrete, None, "discrete", None, ValueMode::Flag, false, "Use discrete tones with lossless WebP", &["webp"]),
    option!(Exact, None, "exact", None, ValueMode::Flag, false, "Preserve transparent WebP pixels", &["webp"]),
    option!(Progressive, None, "progressive", None, ValueMode::Flag, false, "Use progressive JPEG", &["jpeg"]),
];

pub struct CommandSpec { pub name: &'static str, pub alias: Option<&'static str>, pub help: &'static str }
pub const COMMANDS: &[CommandSpec] = &[
    CommandSpec { name: "avif", alias: None, help: "Encode AVIF images" },
    CommandSpec { name: "farbfeld", alias: None, help: "Encode Farbfeld images" },
    CommandSpec { name: "jpeg", alias: Some("jpg"), help: "Encode JPEG images" },
    CommandSpec { name: "jpeg_xl", alias: Some("jxl"), help: "Encode lossless JPEG XL images" },
    CommandSpec { name: "mozjpeg", alias: Some("moz"), help: "Encode JPEG images using MozJPEG" },
    CommandSpec { name: "oxipng", alias: Some("oxi"), help: "Encode PNG images using OxiPNG" },
    CommandSpec { name: "png", alias: None, help: "Encode PNG images" },
    CommandSpec { name: "ppm", alias: None, help: "Encode PPM images" },
    CommandSpec { name: "qoi", alias: None, help: "Encode QOI images" },
    CommandSpec { name: "webp", alias: None, help: "Encode WebP images" },
];

#[derive(Clone, Debug)]
pub enum Step { #[cfg(feature = "resize")] Resize(ResizeValue), #[cfg(feature = "quantization")] Quantize(u8), Premultiply }

#[derive(Clone, Debug, Default)]
pub struct Arguments {
    pub command: &'static str,
    pub files: Vec<PathBuf>, pub directory: Option<PathBuf>, pub suffix: Option<String>,
    pub recursive: bool, pub backup: bool, pub threads: Option<u16>, pub strip: bool,
    pub preserve_timestamps: bool, pub no_progress: bool, pub quiet: bool, pub print_limits: bool,
    pub steps: Vec<(usize, Step)>,
    #[cfg(feature = "resize")] pub downscale: bool,
    #[cfg(feature = "resize")] pub upscale: bool,
    #[cfg(feature = "resize")] pub filter: Option<ResizeFilter>,
    #[cfg(feature = "quantization")] pub dithering: Option<u8>, pub quality: Option<u8>, pub alpha_quality: Option<u8>, pub speed: Option<u8>,
    pub colorspace: Option<String>, pub alpha_mode: Option<String>, pub chroma_quality: Option<u8>,
    pub baseline: bool, pub no_optimize_coding: bool, pub smoothing: Option<u8>, pub multipass: bool,
    pub subsample: Option<u8>, pub qtable: Option<String>, pub interlace: bool, pub effort: Option<u8>,
    pub lossless: bool, pub slight_loss: Option<u8>, pub discrete: bool, pub exact: bool, pub progressive: bool,
}

pub enum Outcome { Run(Arguments), Help(Option<&'static str>), Version }

#[derive(Debug)]
pub enum ParseError {
    UnknownCommand(String),
    UnknownOption(String),
    InvalidValue(String),
    Constraint(String),
    Syntax(lexopt::Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCommand(name) => write!(f, "unknown command {name:?}"),
            Self::UnknownOption(name) => write!(f, "unknown option {name}"),
            Self::InvalidValue(message) | Self::Constraint(message) => f.write_str(message),
            Self::Syntax(error) => error.fmt(f),
        }
    }
}

impl From<String> for ParseError {
    fn from(message: String) -> Self { Self::InvalidValue(message) }
}

impl From<&str> for ParseError {
    fn from(message: &str) -> Self { Self::Constraint(message.into()) }
}

fn text(value: OsString, option: &str) -> Result<String, String> {
    value.into_string().map_err(|_| format!("--{option} requires UTF-8"))
}

fn number<T: std::str::FromStr + PartialOrd>(value: OsString, option: &str, min: T, max: T) -> Result<T, String> {
    let value = text(value, option)?;
    let parsed = value.parse::<T>().map_err(|_| format!("invalid --{option} value {value:?}"))?;
    if parsed < min || parsed > max { return Err(format!("invalid --{option} value {value:?}")); }
    Ok(parsed)
}

pub fn parse<I, T>(args: I) -> Result<Outcome, ParseError>
where I: IntoIterator<Item = T>, T: Into<OsString> {
    let mut parser = Parser::from_args(args);
    let mut result = Arguments {
        #[cfg(feature = "resize")] downscale: true,
        #[cfg(feature = "resize")] upscale: true,
        ..Arguments::default()
    };
    let mut seen = Vec::new();
    let mut index = 0;
    while let Some(arg) = parser.next().map_err(ParseError::Syntax)? {
        index += 1;
        let spec = match arg {
            Arg::Value(value) if result.command.is_empty() => {
                let name = text(value, "command")?;
                if name == "help" {
                    return match parser.next().map_err(ParseError::Syntax)? {
                        None => Ok(Outcome::Help(None)),
                        Some(Arg::Value(value)) => {
                            let name = text(value, "command")?;
                            let command = COMMANDS.iter().find(|c| c.name == name || c.alias == Some(name.as_str()))
                                .ok_or_else(|| ParseError::UnknownCommand(name.clone()))?;
                            if parser.next().map_err(ParseError::Syntax)?.is_some() { return Err("unexpected argument after help command".into()); }
                            Ok(Outcome::Help(Some(command.name)))
                        }
                        Some(_) => Err("expected a command after help".into()),
                    };
                }
                let command = COMMANDS.iter().find(|c| c.name == name || c.alias == Some(name.as_str()))
                    .ok_or_else(|| ParseError::UnknownCommand(name.clone()))?;
                result.command = command.name;
                continue;
            }
            Arg::Value(value) => { result.files.push(value.into()); continue; }
            Arg::Long(name) => OPTIONS.iter().find(|s| s.long == name || s.alias == Some(name)).ok_or_else(|| ParseError::UnknownOption(format!("--{name}")))?,
            Arg::Short(name) => OPTIONS.iter().find(|s| s.short == Some(name) || (s.key == Key::Version && name == 'v')).ok_or_else(|| ParseError::UnknownOption(format!("-{name}")))?,
        };
        if !spec.commands.is_empty() && !spec.commands.contains(&result.command) { return Err((format!("--{} is not valid for {}", spec.long, result.command)).into()); }
        if spec.key == Key::Help { return Ok(Outcome::Help((!result.command.is_empty()).then_some(result.command))); }
        if spec.key == Key::Version { return Ok(Outcome::Version); }
        if result.command.is_empty() { return Err((format!("--{} requires a command", spec.long)).into()); }
        if !spec.repeatable && seen.contains(&spec.key) { return Err((format!("--{} may not be repeated", spec.long)).into()); }
        seen.push(spec.key);
        let value = match spec.value {
            ValueMode::Flag => None,
            ValueMode::Required(_) => { index += 1; Some(parser.value().map_err(ParseError::Syntax)?) },
            ValueMode::Optional(_) => {
                let value = parser.optional_value().or_else(|| parser.values().ok().and_then(|mut v| v.next()));
                index += 1;
                value
            }
        };
        let missing = || format!("--{} requires a value", spec.long);
        let required = || value.clone().ok_or_else(missing);
        match spec.key {
            Key::Directory => result.directory = Some(required()?.into()),
            Key::Recursive => result.recursive = true,
            Key::Suffix => result.suffix = Some(text(value.unwrap_or_else(|| "updated".into()), spec.long)?),
            Key::Backup => result.backup = true,
            Key::Threads => result.threads = Some(number(required()?, spec.long, 1, u16::MAX)?),
            Key::Strip => result.strip = true,
            Key::PreserveTimestamps => result.preserve_timestamps = true,
            Key::NoProgress => result.no_progress = true,
            Key::Quiet => result.quiet = true,
            Key::PrintLimits => result.print_limits = true,
            #[cfg(feature = "resize")]
            Key::Resize => result.steps.push((index, Step::Resize(text(required()?, spec.long)?.parse().map_err(|e| format!("invalid --resize: {e}"))?))),
            #[cfg(feature = "resize")]
            Key::Downscale => result.downscale = true,
            #[cfg(feature = "resize")]
            Key::NoDownscale => result.downscale = false,
            #[cfg(feature = "resize")]
            Key::Upscale => result.upscale = true,
            #[cfg(feature = "resize")]
            Key::NoUpscale => result.upscale = false,
            #[cfg(feature = "resize")]
            Key::Filter => result.filter = Some(text(required()?, spec.long)?.parse().map_err(|e| format!("invalid --filter: {e}"))?),
            #[cfg(feature = "quantization")]
            Key::Quantization => result.steps.push((index, Step::Quantize(number(value.unwrap_or_else(|| "75".into()), spec.long, 1, 100)?))),
            #[cfg(feature = "quantization")]
            Key::Dithering => result.dithering = Some(number(value.unwrap_or_else(|| "75".into()), spec.long, 1, 100)?),
            Key::Premultiply => { index += 1; result.steps.push((index, Step::Premultiply)); }
            Key::Quality => result.quality = Some(number(required()?, spec.long, 1, 100)?),
            Key::AlphaQuality => result.alpha_quality = Some(number(required()?, spec.long, 1, 100)?),
            Key::Speed => result.speed = Some(number(required()?, spec.long, 1, 10)?),
            Key::Colorspace => result.colorspace = Some(text(required()?, spec.long)?),
            Key::AlphaMode => result.alpha_mode = Some(text(required()?, spec.long)?),
            Key::ChromaQuality => result.chroma_quality = Some(number(required()?, spec.long, 1, 100)?),
            Key::Baseline => result.baseline = true,
            Key::NoOptimizeCoding => result.no_optimize_coding = true,
            Key::Smoothing => result.smoothing = Some(number(required()?, spec.long, 1, 100)?),
            Key::Multipass => result.multipass = true,
            Key::Subsample => result.subsample = Some(number(required()?, spec.long, 1, 4)?),
            Key::Qtable => result.qtable = Some(text(required()?, spec.long)?),
            Key::Interlace => result.interlace = true,
            Key::Effort => result.effort = Some(number(required()?, spec.long, 0, 6)?),
            Key::Lossless => result.lossless = true,
            Key::SlightLoss => result.slight_loss = Some(number(required()?, spec.long, 0, 100)?),
            Key::Discrete => result.discrete = true,
            Key::Exact => result.exact = true,
            Key::Progressive => result.progressive = true,
            Key::Help | Key::Version => unreachable!(),
        }
    }
    if result.command.is_empty() { return Err("a command is required".into()); }
    if result.files.is_empty() && !result.print_limits { return Err("at least one input file is required".into()); }
    if result.recursive && result.directory.is_none() { return Err("--recursive requires --directory".into()); }
    #[cfg(feature = "resize")]
    let resize = result.steps.iter().any(|(_, s)| matches!(s, Step::Resize(_)));
    #[cfg(feature = "quantization")]
    let quantize = result.steps.iter().any(|(_, s)| matches!(s, Step::Quantize(_)));
    #[cfg(feature = "resize")]
    if seen.iter().any(|k| matches!(k, Key::Downscale | Key::NoDownscale | Key::Upscale | Key::NoUpscale | Key::Filter)) && !resize { return Err("resize options require --resize".into()); }
    #[cfg(feature = "quantization")]
    if result.dithering.is_some() && !quantize { return Err("--dithering requires --quantization".into()); }
    if result.command == "webp" && result.lossless && result.quality.is_some() { return Err("--lossless conflicts with --quality".into()); }
    if (result.slight_loss.is_some() || result.discrete) && !result.lossless { return Err("--slight_loss and --discrete require --lossless".into()); }
    if let Some(ref c) = result.colorspace {
        if !["ycbcr", "rgb"].contains(&c.as_str()) && !(result.command == "mozjpeg" && c == "grayscale") { return Err((format!("invalid --colorspace value {c:?}")).into()); }
    }
    if let Some(ref c) = result.alpha_mode {
        if !["UnassociatedDirty", "UnassociatedClean", "Premultiplied"].contains(&c.as_str()) { return Err((format!("invalid --alpha_mode value {c:?}")).into()); }
    }
    if let Some(ref c) = result.qtable {
        if !["AhumadaWatsonPeterson", "AnnexK", "Flat", "KleinSilversteinCarney", "MSSSIM", "NRobidoux", "PSNRHVS", "PetersonAhumadaWatson", "WatsonTaylorBorthwick"].contains(&c.as_str()) { return Err((format!("invalid --qtable value {c:?}")).into()); }
    }
    if result.command == "mozjpeg" && result.qtable.is_none() { result.qtable = Some("NRobidoux".into()); }
    #[cfg(feature = "resize")]
    if result.filter.is_none() { result.filter = Some(ResizeFilter::Lanczos3); }
    Ok(Outcome::Run(result))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use super::{parse, Key, Outcome, ParseError, Step, COMMANDS, OPTIONS};

    #[test]
    fn schema_names_are_unique_in_each_command() {
        let names: HashSet<_> = COMMANDS.iter().map(|command| command.name).collect();
        assert_eq!(names.len(), COMMANDS.len());
        for command in COMMANDS {
            assert!(!command.help.is_empty());
            let mut longs = HashSet::new();
            let mut shorts = HashSet::new();
            for option in OPTIONS.iter().filter(|o| o.commands.is_empty() || o.commands.contains(&command.name)) {
                assert!(!option.help.is_empty());
                assert!(longs.insert(option.long));
                if let Some(alias) = option.alias { assert!(longs.insert(alias)); }
                if let Some(short) = option.short { assert!(shorts.insert(short)); }
            }
        }
    }

    #[cfg(all(feature = "resize", feature = "quantization"))]
    #[test]
    fn values_and_preprocessing_order_are_preserved() {
        let result = parse(["png", "--resize=100x100", "--quantization", "80", "--resize", "50%", "file.png"]);
        let Ok(Outcome::Run(args)) = result else { panic!("expected valid arguments") };
        assert!(matches!(args.steps[0].1, Step::Resize(_)));
        assert!(matches!(args.steps[1].1, Step::Quantize(80)));
        assert!(matches!(args.steps[2].1, Step::Resize(_)));
        assert_eq!(args.files[0], std::path::Path::new("file.png"));
    }

    #[test]
    fn defaults_and_duplicate_validation_are_explicit() {
        let Ok(Outcome::Run(args)) = parse(["moz", "--suffix", "--print-limits"]) else { panic!("expected valid arguments") };
        assert_eq!(args.command, "mozjpeg");
        assert_eq!(args.suffix.as_deref(), Some("updated"));
        assert_eq!(args.qtable.as_deref(), Some("NRobidoux"));
        assert!(parse(["png", "--quiet", "--quiet", "file.png"]).is_err());
        assert!(parse(["png", "--resize", "1x1", "--resize", "2x2", "file.png"]).is_ok());
        assert!(OPTIONS.iter().any(|option| option.key == Key::Help));
    }

    #[test]
    fn parser_classifies_errors() {
        assert!(matches!(parse(["bad"]), Err(ParseError::UnknownCommand(_))));
        assert!(matches!(parse(["png", "--bad"]), Err(ParseError::UnknownOption(_))));
        assert!(matches!(parse(["jpeg", "--quality", "101", "image.jpg"]), Err(ParseError::InvalidValue(_))));
        assert!(matches!(parse(["png", "--recursive", "image.png"]), Err(ParseError::Constraint(_))));
        assert!(matches!(parse(["png", "--directory"]), Err(ParseError::Syntax(_))));
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_paths_are_kept_and_non_utf8_values_are_rejected() {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        let path = OsString::from_vec(vec![b'f', 0xff]);
        let Ok(Outcome::Run(args)) = parse([OsString::from("png"), path.clone()]) else { panic!("expected valid arguments") };
        assert_eq!(args.files[0].as_os_str(), path);
        assert!(parse([OsString::from("png"), OsString::from("--suffix"), path, OsString::from("file.png")]).is_err());
    }
}
