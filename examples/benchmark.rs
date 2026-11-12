#[path="../benches/support/mod.rs"] mod support;
use std::{fs,path::{Path,PathBuf},process::Command,time::Instant};
use rimage::{perceptual::PerceptualReference,still::StillImage};
use serde_json::json;
fn main() {
    if let Err(error)=run() {eprintln!("benchmark: {error}");std::process::exit(1);}
}
fn run()->Result<(),Box<dyn std::error::Error>> {
    let timed=std::env::args().nth(1).as_deref()==Some("run");
    if timed && !Command::new("perf-gate-ready").status()?.success() {return Err("perf gate deferred: host busy".into());}
    let repository=Path::new(env!("CARGO_MANIFEST_DIR"));let benchmark=support::root();let output_dir=benchmark.join("results");fs::create_dir_all(&output_dir)?;
    let scratch=repository.join(".work/tmp/benchmark-capture");fs::create_dir_all(&scratch)?;
    let generated=benchmark.join("corpus/generated");fs::create_dir_all(&generated)?;
    let mut cases=support::photographic().into_iter().map(|(name,bytes)|(name,bytes,String::from("corpus"))).collect::<Vec<_>>();
    for (name,bytes) in support::generated() {fs::write(generated.join(&name),&bytes)?;cases.push((name,bytes,"corpus/generated".into()));}
    let current=std::env::var_os("RIMAGE_BENCH_BIN").map(PathBuf::from).unwrap_or_else(||repository.join("target/dist/rimage"));
    let original=std::env::var_os("RIMAGE_ORIGINAL_BIN").map(PathBuf::from).unwrap_or_else(||repository.join(".work/tmp/original-target/release/rimage"));
    let revision=String::from_utf8(Command::new("git").args(["rev-parse","HEAD"]).output()?.stdout)?.trim().to_owned();
    let tools=[("rimage",current.as_os_str().to_owned(),"new"),("rimage-png",current.as_os_str().to_owned(),"new-png"),("original-mozjpeg",original.as_os_str().to_owned(),"original-jpeg"),("original-oxipng",original.as_os_str().to_owned(),"original-png"),("cwebp","cwebp".into(),"cwebp")];
    let mut records=vec![];
    for run in 0..if timed {5} else {1} {
        for (name,source,source_directory) in &cases {
            let image=StillImage::decode(source)?;let (width,height)=image.dimensions();
            for target in [80.0,90.0,95.0] {
                let mut order=(0..tools.len()).collect::<Vec<_>>();if run%2==1 {order.reverse();}
                for index in order {
                    let (tool,executable,kind)=&tools[index];
                    if kind.ends_with("png") && target!=80.0 {continue;}
                    let mut record=support::record_base(tool,name,&revision);record["attempts"]=json!([]);record["run"]=json!(run);record["target"]=json!(target);record["sourcePath"]=json!(format!("{source_directory}/{name}"));record["width"]=json!(width);record["height"]=json!(height);record["inputBytes"]=json!(source.len());record["parameters"]=json!({"target":target,"effort":"fast","globalCandidateCap":8,"legacyGrid":[0,25,50,75,95,100]});
                    let extension=if *kind=="original-jpeg" {"jpg"} else if kind.ends_with("png") {"png"} else {"webp"};
                    let input=benchmark.join(source_directory).join(name);
                    let target_name=format!("{}{}-{}-{}-{}.{}",if timed {"timed-"} else {""},name.replace('.',"_"),tool,target,run,extension);let saved=output_dir.join(&target_name);
                    let start=Instant::now();
                    let result=(||->Result<(Vec<u8>,Option<f64>,bool),String> {
                        if image.bit_depth()>8 && !kind.ends_with("png") {return Err("quality comparison excludes high-depth samples; exact proof is separate".into());}
                        let reference=if kind.ends_with("png") {None} else {Some(PerceptualReference::new(&image).map_err(|e|e.to_string())?)};
                        let mut winner:Option<(Vec<u8>,Option<f64>,bool)>=None;
                        let qualities=if *kind=="new"||kind.ends_with("png") {vec![95]} else {vec![0,25,50,75,95,100]};
                        for quality in qualities {
                            let attempt=scratch.join(format!("{tool}-{quality}"));fs::create_dir_all(&attempt).map_err(|e|e.to_string())?;
                            let candidate=attempt.join(if *kind=="original-jpeg"||*kind=="original-png" {format!("{}.{}",Path::new(name).file_stem().unwrap().to_str().unwrap(),extension)} else {format!("output.{extension}")});
                            if candidate.exists() {fs::remove_file(&candidate).map_err(|e|e.to_string())?;}
                            let mut command=Command::new(executable);command.env("RAYON_NUM_THREADS","1");
                            match *kind {
                                "new"=>{command.arg("convert").arg(&input).arg("-o").arg(&candidate).args(["--score",&target.to_string(),"--effort","fast","--quiet"]);},
                                "new-png"=>{command.arg("convert").arg(&input).arg("-o").arg(&candidate).args(["--lossless","--effort","fast","--quiet"]);},
                                "original-jpeg"=>{command.arg("jpeg").arg(&input).arg("-d").arg(&attempt).args(["-q",&quality.to_string(),"-t","1","--quiet"]);},
                                "original-png"=>{command.arg("png").arg(&input).arg("-d").arg(&attempt).args(["-t","1","--quiet"]);},
                                _=>{command.args(["-quiet","-q",&quality.to_string(),"-m","4","-alpha_q","100","-exact","-metadata","all"]).arg(&input).arg("-o").arg(&candidate);},
                            }
                            let output=command.output().map_err(|e|format!("tool unavailable: {e}"))?;
                            record["attempts"].as_array_mut().unwrap().push(json!({"quality":quality,"exitCode":output.status.code(),"stderr":String::from_utf8_lossy(&output.stderr)}));
                            if !output.status.success() {continue;}
                            let bytes=fs::read(candidate).map_err(|e|e.to_string())?;let decoded=StillImage::decode(&bytes).map_err(|e|e.to_string())?;
                            if decoded.dimensions()!=image.dimensions() {return Err("output dimensions differ".into());}
                            record["attempts"].as_array_mut().unwrap().last_mut().unwrap()["bytes"]=json!(bytes.len());
                            let exact=kind.ends_with("png");
                            let score=if exact {if !rimage::optimizer::samples_equal(&image,&decoded) {continue;}None} else {let score=reference.as_ref().unwrap().score_encoded(&bytes).map_err(|e|e.to_string())?;record["attempts"].as_array_mut().unwrap().last_mut().unwrap()["score"]=json!(score);if score<target {continue;}Some(score)};
                            if winner.as_ref().is_none_or(|w|bytes.len()<w.0.len()) {winner=Some((bytes,score,exact));}
                        }
                        winner.ok_or("no qualifying evaluated output (tool failure or fidelity refusal)".into())
                    })();
                    let elapsed=start.elapsed().as_secs_f64()*1000.0;
                    match result {
                        Ok((bytes,score,exact))=>{fs::write(&saved,&bytes)?;record["completed"]=json!(true);record["lossless"]=json!(exact);record["exactSamples"]=json!(exact);record["score"]=json!(score);record["outputBytes"]=json!(bytes.len());record["outputWidth"]=json!(width);record["outputHeight"]=json!(height);record["outputPath"]=json!(format!("results/{target_name}"));record["matchedWithinHalfPoint"]=json!(score.is_some_and(|s|s-target<=0.5));if timed {record["timingAccepted"]=json!(true);record["elapsedMs"]=json!(elapsed);}},
                        Err(error)=>record["failure"]=json!(error),
                    }
                    support::validate_record(&record)?;records.push(record);
                }
            }
        }
    }
    if timed && !Command::new("perf-gate-ready").arg("-1").status()?.success() {return Err("discard overlapping timings; post-run perf gate failed".into());}
    let document=json!({"schemaVersion":1,"revision":revision,"architecture":std::env::consts::ARCH,"timingGate":if timed {"accepted"} else {"deferred: host busy"},"runsPerSide":if timed {5} else {1},"tools":{"rimage":"locked repository dist","original":"66fc0fc plus docs/original-build.patch, release; runtime comparison only","cwebp":"1.6.0, single-threaded","jpegli":"unavailable","dedicatedAvif":"unavailable"},"summary":support::summarize(&records),"records":records});
    fs::write(output_dir.join(if timed {"timed.json"} else {"untimed.json"}),serde_json::to_vec_pretty(&document)?)?;
    println!("saved {} records; {} completed",document["records"].as_array().unwrap().len(),document["records"].as_array().unwrap().iter().filter(|r|r["completed"]==true).count());
    Ok(())
}
