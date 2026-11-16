#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptionScope { Common, Transform, Input, Optimize }
pub(crate) struct OptionSpec { pub name: &'static str, pub short: Option<char>, pub value: &'static str, pub description: &'static str, pub scope: OptionScope }
const HELP_SHORT: char = 'h';
const OUTPUT_SHORT: char = 'o';
const JOBS_SHORT: char = 'j';
const HELP: &str = "help";
const VERSION: &str = "version";
const JOBS: &str = "jobs";
const QUIET: &str = "quiet";
const VERBOSE: &str = "verbose";
const JSON: &str = "json";
const OUTPUT: &str = "output";
const OUT_DIR: &str = "out-dir";
const FORMAT: &str = "format";
const SCORE: &str = "score";
const QUALITY: &str = "quality";
const LOSSLESS: &str = "lossless";
const EFFORT: &str = "effort";
const MAX_DIMENSION: &str = "max-dimension";
const STRIP_METADATA: &str = "strip-metadata";
const PRESERVE_TIMESTAMPS: &str = "preserve-timestamps";
const RECURSIVE: &str = "recursive";
const FILE_LIST: &str = "file-list";
const IN_PLACE: &str = "in-place";
pub(crate) const OPTIONS: &[OptionSpec] = &[
    OptionSpec { name: HELP, short: Some(HELP_SHORT), value: "", description: "Show this reference", scope: OptionScope::Common },
    OptionSpec { name: VERSION, short: None, value: "", description: "Show version", scope: OptionScope::Common },
    OptionSpec { name: JOBS, short: Some(JOBS_SHORT), value: "N", description: "Workers, positive (default 1; CPU/memory bounded)", scope: OptionScope::Common },
    OptionSpec { name: QUIET, short: None, value: "", description: "Suppress success diagnostics", scope: OptionScope::Common },
    OptionSpec { name: VERBOSE, short: None, value: "", description: "Include candidate settings on stderr; conflicts with --quiet", scope: OptionScope::Common },
    OptionSpec { name: JSON, short: None, value: "", description: "Emit one ordered schemaVersion:1 object", scope: OptionScope::Common },
    OptionSpec { name: OUTPUT, short: Some(OUTPUT_SHORT), value: "PATH", description: "One output; '-' writes binary stdout", scope: OptionScope::Transform },
    OptionSpec { name: OUT_DIR, short: None, value: "DIR", description: "Batch destination; conflicts with --output and --in-place", scope: OptionScope::Transform },
    OptionSpec { name: FORMAT, short: None, value: "FORMAT", description: "jpeg, png, webp, avif; optimize defaults to source format and permits auto", scope: OptionScope::Transform },
    OptionSpec { name: SCORE, short: None, value: "NUMBER", description: "Minimum SSIMULACRA2 0..100 (default 95)", scope: OptionScope::Transform },
    OptionSpec { name: QUALITY, short: None, value: "NUMBER", description: "Encoder control 0..100; conflicts with --score", scope: OptionScope::Transform },
    OptionSpec { name: LOSSLESS, short: None, value: "", description: "Exact samples; conflicts with score, quality and resize", scope: OptionScope::Transform },
    OptionSpec { name: EFFORT, short: None, value: "LEVEL", description: "fast, balanced (default), best", scope: OptionScope::Transform },
    OptionSpec { name: MAX_DIMENSION, short: None, value: "N", description: "Positive maximum dimension; never enlarge", scope: OptionScope::Transform },
    OptionSpec { name: STRIP_METADATA, short: None, value: "", description: "Remove private metadata (default: keep)", scope: OptionScope::Transform },
    OptionSpec { name: PRESERVE_TIMESTAMPS, short: None, value: "", description: "Copy input modification time", scope: OptionScope::Transform },
    OptionSpec { name: RECURSIVE, short: None, value: "", description: "Descend into directories", scope: OptionScope::Input },
    OptionSpec { name: FILE_LIST, short: None, value: "PATH", description: "Read newline-separated UTF-8 paths relative to file (repeatable)", scope: OptionScope::Input },
    OptionSpec { name: IN_PLACE, short: None, value: "", description: "Replace source; conflicts with destinations and auto", scope: OptionScope::Optimize },
];
use std::{ffi::OsString, num::NonZeroU32, path::PathBuf};
use img::{optimizer::{Fidelity,MetadataPolicy},perceptual::ScoreTarget,still::{Effort,Format,Quality}};
pub(crate) struct CommandSpec { pub name: &'static str, pub command: Command, pub usage: &'static str, pub description: &'static str, pub example: &'static str }
pub(crate) const COMMANDS: &[CommandSpec] = &[
    CommandSpec { name: "optimize", command: Command::Optimize, usage: "[INPUT...] [--file-list PATH]", description: "Optimize images, keeping their format by default.", example: "img optimize photo.png --lossless" },
    CommandSpec { name: "convert", command: Command::Convert, usage: "[INPUT...] [--file-list PATH]", description: "Convert images using a format or output extension.", example: "img convert photo.png -o photo.webp" },
    CommandSpec { name: "inspect", command: Command::Inspect, usage: "[INPUT...] [--file-list PATH]", description: "Inspect image headers without decoding pixels.", example: "img inspect photo.png --json" },
    CommandSpec { name: "compare", command: Command::Compare, usage: "REFERENCE CANDIDATE", description: "Compare two images and report their score.", example: "img compare photo.png photo.webp" },
];
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum Command { Optimize,Convert,Inspect,Compare }
#[derive(Clone)]
pub(crate) struct Arguments {
    pub command:Command,pub inputs:Vec<PathBuf>,pub output:Option<PathBuf>,pub out_dir:Option<PathBuf>,pub in_place:bool,
    pub format:Option<Format>,pub auto:bool,pub fidelity:Fidelity,pub effort:Effort,pub metadata:MetadataPolicy,pub max_dimension:Option<NonZeroU32>,
    pub jobs:usize,pub recursive:bool,pub quiet:bool,pub verbose:bool,pub json:bool,pub timestamps:bool,pub lists:Vec<PathBuf>,
}
pub(crate) enum Outcome { Run(Arguments),Help,Version }
pub(crate) fn parse(args:impl IntoIterator<Item=OsString>)->Result<Outcome,String> {
    use lexopt::prelude::*;
    let mut args=args.into_iter().peekable();
    if args.peek().is_none() {return Ok(Outcome::Help);}
    let mut parser=lexopt::Parser::from_args(args);
    let mut command=None;let mut inputs=vec![];let mut output=None;let mut out_dir=None;let mut in_place=false;let mut format=None;let mut auto=false;
    let mut score=None;let mut quality=None;let mut lossless=false;let mut effort=Effort::Balanced;let mut metadata=MetadataPolicy::Keep;let mut max_dimension=None;
    let mut jobs=1;let mut recursive=false;let mut quiet=false;let mut verbose=false;let mut json=false;let mut timestamps=false;let mut lists=vec![];let mut processing_option=false;
    while let Some(arg)=parser.next().map_err(|e|e.to_string())? {
        match arg {
            Short(HELP_SHORT)|Long(HELP)=>return Ok(Outcome::Help),
            Long(VERSION)=>return Ok(Outcome::Version),
            Short(OUTPUT_SHORT)|Long(OUTPUT)=>output=Some(PathBuf::from(parser.value().map_err(|e|e.to_string())?)),
            Long(OUT_DIR)=>out_dir=Some(PathBuf::from(parser.value().map_err(|e|e.to_string())?)),
            Long(IN_PLACE)=>in_place=true,
            Long(FORMAT)=>{ processing_option=true;let value=parser.value().map_err(|e|e.to_string())?; if value=="auto" { auto=true;format=None; } else { format=Some(Format::from_extension(&value).ok_or("format must be jpeg, png, webp, avif or auto")?);auto=false; } },
            Long(SCORE)=>{processing_option=true;let value:f64=parser.value().map_err(|e|e.to_string())?.parse().map_err(|_|"invalid score")?;score=Some(ScoreTarget::new(value).map_err(|e|e.to_string())?);},
            Long(QUALITY)=>{processing_option=true;let value:f32=parser.value().map_err(|e|e.to_string())?.parse().map_err(|_|"invalid quality")?;quality=Some(Quality::new(value).map_err(|e|e.to_string())?);},
            Long(LOSSLESS)=>{processing_option=true;lossless=true;},
            Long(EFFORT)=>{processing_option=true;effort=match parser.value().map_err(|e|e.to_string())?.to_str() {Some("fast")=>Effort::Fast,Some("balanced")=>Effort::Balanced,Some("best")=>Effort::Best,_=>return Err("effort must be fast, balanced or best".into())};},
            Long(STRIP_METADATA)=>{processing_option=true;metadata=MetadataPolicy::Strip;},
            Long(MAX_DIMENSION)=>{processing_option=true;let n:u32=parser.value().map_err(|e|e.to_string())?.parse().map_err(|_|"invalid maximum dimension")?;max_dimension=Some(NonZeroU32::new(n).ok_or("maximum dimension must be positive")?);},
            Short(JOBS_SHORT)|Long(JOBS)=>{jobs=parser.value().map_err(|e|e.to_string())?.parse().map_err(|_|"invalid jobs")?;if jobs==0 {return Err("jobs must be positive".into());}},
            Long(RECURSIVE)=>recursive=true,
            Long(QUIET)=>quiet=true,
            Long(VERBOSE)=>verbose=true,
            Long(JSON)=>json=true,
            Long(PRESERVE_TIMESTAMPS)=>timestamps=true,
            Long(FILE_LIST)=>lists.push(PathBuf::from(parser.value().map_err(|e|e.to_string())?)),
            Value(value) if command.is_none()=>{ command=Some(match value.to_str() {Some("help")=>return Ok(Outcome::Help),Some(name)=>COMMANDS.iter().find(|spec| spec.name==name).map(|spec| spec.command).ok_or_else(||format!("unknown command: {value:?}"))?,_=>return Err(format!("unknown command: {value:?}"))}); },
            Value(value)=>inputs.push(PathBuf::from(value)),
            other=>return Err(other.unexpected().to_string()),
        }
    }
    let command=command.ok_or("expected optimize, convert, inspect or compare")?;
    if inputs.is_empty() && lists.is_empty() {return Err("at least one input is required".into());}
    if quiet && verbose {return Err("quiet and verbose conflict".into());}
    if output.is_some() && out_dir.is_some() {return Err("output and out-dir conflict".into());}
    if in_place && (output.is_some()||out_dir.is_some()) {return Err("in-place conflicts with explicit destinations".into());}
    if score.is_some() && quality.is_some() {return Err("score and quality conflict".into());}
    if lossless && (score.is_some()||quality.is_some()||max_dimension.is_some()) {return Err("lossless conflicts with score, quality and resizing".into());}
    if auto && (command!=Command::Optimize||output.is_some()||in_place) {return Err("auto requires optimize and conflicts with output and in-place".into());}
    if command==Command::Convert && in_place {return Err("conversion cannot replace a source in place; use optimize".into());}
    if matches!(command,Command::Inspect|Command::Compare) && (processing_option||output.is_some()||out_dir.is_some()||in_place||timestamps) {return Err("inspect and compare accept no transformation or output options".into());}
    if command==Command::Compare && (inputs.len()!=2||!lists.is_empty()||recursive) {return Err("compare requires exactly two explicit inputs".into());}
    if inputs.iter().any(|p|p==std::path::Path::new("-")) && (inputs.len()!=1||!lists.is_empty()||in_place) {return Err("stdin requires one input and no in-place".into());}
    if output.as_deref()==Some(std::path::Path::new("-")) && (format.is_none()||auto||json||inputs.len()!=1||!lists.is_empty()) {return Err("binary stdout requires one input, concrete explicit format and no JSON".into());}
    let fidelity=if lossless {Fidelity::Lossless} else if let Some(q)=quality {Fidelity::Quality(q)} else {Fidelity::Score(score.unwrap_or_default())};
    Ok(Outcome::Run(Arguments {command,inputs,output,out_dir,in_place,format,auto,fidelity,effort,metadata,max_dimension,jobs,recursive,quiet,verbose,json,timestamps,lists}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn option_names_are_unique() {
        let mut long=std::collections::HashSet::new(); let mut short=std::collections::HashSet::new();
        for option in OPTIONS { assert!(long.insert(option.name)); if let Some(name)=option.short {assert!(short.insert(name));} }
    }
    #[test]
    fn documented_examples_parse() {
        for spec in COMMANDS {
            let args=spec.example.split_whitespace().skip(1).map(OsString::from);
            match parse(args) { Ok(Outcome::Run(arguments))=>assert_eq!(arguments.command,spec.command), _=>panic!("invalid example: {}",spec.example) }
        }
    }
}
