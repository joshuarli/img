#![allow(dead_code)]
use std::{fs,path::{Path,PathBuf}};
use rimage::still::{Effort,EncodingMode,Format,SampleLayout,StillImage};
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
pub fn root()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks")}
pub fn photographic()->Vec<(String,Vec<u8>)> {
    let manifest:Value=serde_json::from_slice(&fs::read(root().join("corpus/manifest.json")).unwrap()).unwrap();
    manifest["sources"].as_array().unwrap().iter().map(|entry| {
        let name=entry["file"].as_str().unwrap();let bytes=fs::read(root().join("corpus").join(name)).unwrap();
        assert_eq!(format!("{:x}",Sha256::digest(&bytes)),entry["sha256"].as_str().unwrap());
        assert!(!entry["license"].as_str().unwrap().is_empty());assert!(!entry["evidence"].as_str().unwrap().is_empty());
        let decoded=StillImage::decode(&bytes).unwrap();assert_eq!(decoded.dimensions(),(entry["width"].as_u64().unwrap() as u32,entry["height"].as_u64().unwrap() as u32));
        (name.to_owned(),bytes)
    }).collect()
}
pub fn generated()->Vec<(String,Vec<u8>)> {
    let mut cases=vec![];
    for name in ["ui","text","line","flat","transparent","gray","large","icc","deep"] {
        let (w,h)=if name=="large" {(2048,1024)} else {(96,64)};
        let layout=match name {"transparent"=>SampleLayout::Rgba8,"gray"=>SampleLayout::Gray8,"deep"=>SampleLayout::Gray16,_=>SampleLayout::Rgb8};
        let samples=(0..h).flat_map(|y|(0..w).flat_map(move|x| {
            let rgb=match name {
                "ui"=>[if x%32<30 {230} else {50},if y%16<14 {200} else {20},180],
                "text"=>[if x%7<2 && y%12<9 {20} else {250};3],
                "line"=>[if x%13==y%13 {0} else {255};3],
                "flat"=>[80,120,160],
                _=>[(x*255/w) as u8,(y*255/h) as u8,((x+y)*255/(w+h)) as u8],
            };
            match name {"transparent"=>vec![rgb[0],rgb[1],rgb[2],((x+y)%256) as u8],"gray"=>vec![rgb[0]],"deep"=>((x*65535/w) as u16).to_ne_bytes().to_vec(),_=>rgb.to_vec()}
        })).collect();
        let image=StillImage::from_samples(w,h,layout,samples).unwrap();
        let mut bytes=image.encode(Format::Png,EncodingMode::Lossless,Effort::Fast).unwrap();
        if name=="icc" {
            let mut info=png::Info::with_size(w,h);info.color_type=png::ColorType::Rgb;info.bit_depth=png::BitDepth::Eight;let mut profile=moxcms::ColorProfile::new_srgb().encode().unwrap();
            // moxcms 0.8.1 writes wall-clock creation time regardless of the profile field.
            profile[24..36].copy_from_slice(&[7,208,0,1,0,1,0,0,0,0,0,0]);info.icc_profile=Some(std::borrow::Cow::Owned(profile));
            bytes.clear();png::Encoder::with_info(&mut bytes,info).unwrap().write_header().unwrap().write_image_data(&image.samples()).unwrap();
        }
        cases.push((format!("{name}.png"),bytes));
    }
    let bytes=StillImage::decode(&cases[0].1).unwrap().encode(Format::Webp,EncodingMode::Lossless,Effort::Fast).unwrap();cases.push(("compressed.webp".into(),bytes));cases
}
#[derive(Debug,PartialEq)]
pub enum Gate {Improvement,Regression,Unchanged,BaselineDebt,Deferred}
pub fn gate(baseline:Option<(f64,f64)>,candidate:Option<(f64,f64)>,baseline_complete:bool,candidate_complete:bool)->Gate {
    if !candidate_complete {return Gate::Regression;}
    if !baseline_complete {return Gate::BaselineDebt;}
    let (Some((base,base_spread)),Some((new,new_spread)))=(baseline,candidate) else {return Gate::Deferred;};
    assert!(base.is_finite()&&base>0.0&&new.is_finite()&&new>0.0);
    let noise=base_spread.max(new_spread);
    if new-base>base*0.1 && new-base>noise {Gate::Regression} else if base-new>noise {Gate::Improvement} else {Gate::Unchanged}
}
pub fn validate_record(value:&Value)->Result<(),String> {
    let fail=||Err("invalid saved benchmark record".into());
    if value["schemaVersion"]!=1 || !value["tool"].is_string()||!value["fixture"].is_string()||!value["revision"].is_string() {return fail();}
    if value["completed"]!=true {return if value["failure"].is_string() {Ok(())} else {fail()};}
    let width=value["width"].as_u64().ok_or("missing width")?;let height=value["height"].as_u64().ok_or("missing height")?;
    if width==0||height==0||value["outputWidth"]!=width||value["outputHeight"]!=height||value["inputBytes"].as_u64().is_none_or(|n|n==0)||value["outputBytes"].as_u64().is_none_or(|n|n==0) {return fail();}
    if value["lossless"]==true {if value["exactSamples"]!=true {return fail();}}
    else {let score=value["score"].as_f64().ok_or("missing score")?;let target=value["target"].as_f64().ok_or("missing target")?;if !score.is_finite()||score<target {return fail();}}
    if value["timingAccepted"]==true {if value["run"].as_u64().is_none_or(|n|n>=5)||value["elapsedMs"].as_f64().is_none_or(|n|!n.is_finite()||n<=0.0) {return fail();}}
    else if !value["elapsedMs"].is_null() {return fail();}
    Ok(())
}
pub fn verify_saved(directory:&Path)->usize {
    let mut count=0;if !directory.exists() {return 0;}
    for file in fs::read_dir(directory).unwrap() {let file=file.unwrap().path();if file.extension().is_some_and(|e|e=="json") {let document:Value=serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();assert_eq!(document["summary"],summarize(document["records"].as_array().unwrap()));for record in document["records"].as_array().unwrap() {validate_record(record).unwrap();if record["completed"]==true {let input=root().join(record["sourcePath"].as_str().unwrap());let output=root().join(record["outputPath"].as_str().unwrap());let source_bytes=fs::read(input).unwrap();assert_eq!(source_bytes.len() as u64,record["inputBytes"].as_u64().unwrap());let a=StillImage::decode(&source_bytes).unwrap();let bytes=fs::read(output).unwrap();let b=StillImage::decode(&bytes).unwrap();assert_eq!(a.dimensions(),b.dimensions());assert_eq!(a.dimensions(),(record["width"].as_u64().unwrap() as u32,record["height"].as_u64().unwrap() as u32));assert_eq!(bytes.len() as u64,record["outputBytes"].as_u64().unwrap());if record["lossless"]==true {assert!(rimage::optimizer::samples_equal(&a,&b));}else {let actual=rimage::perceptual::PerceptualReference::new(&a).unwrap().score_encoded(&bytes).unwrap();assert!(actual>=record["target"].as_f64().unwrap());assert!((actual-record["score"].as_f64().unwrap()).abs()<0.0001,"{} {}: actual={actual} saved={}",record["fixture"],record["tool"],record["score"]);}}count+=1;}}}count
}
pub fn record_base(tool:&str,name:&str,revision:&str)->Value {json!({"schemaVersion":1,"tool":tool,"fixture":name,"revision":revision,"threads":1,"architecture":std::env::consts::ARCH,"timingAccepted":false,"elapsedMs":null,"completed":false})}
pub fn summarize(records:&[Value])->Value {
    let mut groups=std::collections::BTreeMap::<String,Vec<&Value>>::new();
    for record in records {groups.entry(format!("{}:{}:{}",record["tool"].as_str().unwrap(),record["fixture"].as_str().unwrap(),record["target"])).or_default().push(record);}
    let median=|values:&mut Vec<f64>| {values.sort_by(f64::total_cmp);let len=values.len();if len==0 {None} else if len%2==0 {Some((values[len/2-1]+values[len/2])*0.5)} else {Some(values[len/2])}};
    let mut result=serde_json::Map::new();
    for (tool,rows) in groups {
        let completed=rows.iter().filter(|r|r["completed"]==true).copied().collect::<Vec<_>>();
        let mut sizes=completed.iter().map(|r|r["outputBytes"].as_u64().unwrap() as f64).collect::<Vec<_>>();let mut timings=completed.iter().filter_map(|r|r["elapsedMs"].as_f64()).collect::<Vec<_>>();
        let time=median(&mut timings);let size=median(&mut sizes);let spread=timings.last().zip(timings.first()).map(|(max,min)|max-min);
        result.insert(tool,json!({"completed":completed.len(),"failures":rows.len()-completed.len(),"medianOutputBytes":size,"medianElapsedMs":time,"spreadMs":spread,"timingGate":if timings.is_empty() {"deferred"} else {"accepted"}}));
    }
    Value::Object(result)
}
