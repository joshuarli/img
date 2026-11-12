use std::{ffi::OsString, num::NonZeroU32, path::PathBuf};
use rimage::{optimizer::{Fidelity,MetadataPolicy},perceptual::ScoreTarget,still::{Effort,Format,Quality}};
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
    let mut parser=lexopt::Parser::from_args(args);
    let mut command=None;let mut inputs=vec![];let mut output=None;let mut out_dir=None;let mut in_place=false;let mut format=None;let mut auto=false;
    let mut score=None;let mut quality=None;let mut lossless=false;let mut effort=Effort::Balanced;let mut metadata=MetadataPolicy::Keep;let mut max_dimension=None;
    let mut jobs=1;let mut recursive=false;let mut quiet=false;let mut verbose=false;let mut json=false;let mut timestamps=false;let mut lists=vec![];let mut processing_option=false;
    while let Some(arg)=parser.next().map_err(|e|e.to_string())? {
        match arg {
            Short('h')|Long("help")=>return Ok(Outcome::Help),
            Long("version")=>return Ok(Outcome::Version),
            Short('o')|Long("output")=>output=Some(PathBuf::from(parser.value().map_err(|e|e.to_string())?)),
            Long("out-dir")=>out_dir=Some(PathBuf::from(parser.value().map_err(|e|e.to_string())?)),
            Long("in-place")=>in_place=true,
            Long("format")=>{ processing_option=true;let value=parser.value().map_err(|e|e.to_string())?; if value=="auto" { auto=true;format=None; } else { format=Some(Format::from_extension(&value).ok_or("format must be jpeg, png, webp, avif or auto")?);auto=false; } },
            Long("score")=>{processing_option=true;let value:f64=parser.value().map_err(|e|e.to_string())?.parse().map_err(|_|"invalid score")?;score=Some(ScoreTarget::new(value).map_err(|e|e.to_string())?);},
            Long("quality")=>{processing_option=true;let value:f32=parser.value().map_err(|e|e.to_string())?.parse().map_err(|_|"invalid quality")?;quality=Some(Quality::new(value).map_err(|e|e.to_string())?);},
            Long("lossless")=>{processing_option=true;lossless=true;},
            Long("effort")=>{processing_option=true;effort=match parser.value().map_err(|e|e.to_string())?.to_str() {Some("fast")=>Effort::Fast,Some("balanced")=>Effort::Balanced,Some("best")=>Effort::Best,_=>return Err("effort must be fast, balanced or best".into())};},
            Long("strip-metadata")=>{processing_option=true;metadata=MetadataPolicy::Strip;},
            Long("max-dimension")=>{processing_option=true;let n:u32=parser.value().map_err(|e|e.to_string())?.parse().map_err(|_|"invalid maximum dimension")?;max_dimension=Some(NonZeroU32::new(n).ok_or("maximum dimension must be positive")?);},
            Short('j')|Long("jobs")=>{jobs=parser.value().map_err(|e|e.to_string())?.parse().map_err(|_|"invalid jobs")?;if jobs==0 {return Err("jobs must be positive".into());}},
            Long("recursive")=>recursive=true,
            Long("quiet")=>quiet=true,
            Long("verbose")=>verbose=true,
            Long("json")=>json=true,
            Long("preserve-timestamps")=>timestamps=true,
            Long("file-list")=>lists.push(PathBuf::from(parser.value().map_err(|e|e.to_string())?)),
            Value(value) if command.is_none()=>{ command=Some(match value.to_str() {Some("optimize")=>Command::Optimize,Some("convert")=>Command::Convert,Some("inspect")=>Command::Inspect,Some("compare")=>Command::Compare,Some("help")=>return Ok(Outcome::Help),_=>return Err(format!("unknown command: {value:?}"))}); },
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
