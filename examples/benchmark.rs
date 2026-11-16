#[path="../benches/support/mod.rs"] mod support;
use std::{fs,path::{Path,PathBuf},process::Command,time::Instant};
use img::{perceptual::PerceptualReference,still::StillImage};
use serde_json::{json,Value};
fn main() {
    if let Err(error)=run() {eprintln!("benchmark: {error}");std::process::exit(1);}
}
fn run()->Result<(),Box<dyn std::error::Error>> {
    let mode=std::env::args().nth(1).unwrap_or_else(||"capture".into());
    if mode=="preparation-probe" {
        let directory=PathBuf::from(std::env::args_os().nth(2).ok_or("preparation-probe requires an output directory")?);fs::create_dir_all(&directory)?;
        for (name,path) in [("source","corpus/coffee.png"),("candidate","results/coffee_png-rimage-80-0.webp")] {
            let image=StillImage::decode(&fs::read(support::root().join(path))?)?;
            fs::write(directory.join(format!("{name}-samples.bin")),image.samples())?;
            let prepared=image.prepare()?;fs::write(directory.join(format!("{name}-linear.bin")),bytemuck::cast_slice(prepared.linear_rgb()))?;
        }
        return Ok(());
    }
    if mode!="run"&&mode!="capture" {return Err("expected run, capture or preparation-probe".into());}
    let timed=mode=="run";
    if timed && !Command::new("perf-gate-ready").status()?.success() {return Err("perf gate deferred: host busy".into());}
    let repository=Path::new(env!("CARGO_MANIFEST_DIR"));let benchmark=support::root();let output_dir=benchmark.join("results");fs::create_dir_all(&output_dir)?;
    let scratch=repository.join(".work/tmp/benchmark-capture");fs::create_dir_all(&scratch)?;
    let generated=benchmark.join("corpus/generated");fs::create_dir_all(&generated)?;
    let validation_corpus=std::env::var("IMG_BENCH_CORPUS").is_ok_and(|v|v=="validation");
    let mut cases=if validation_corpus {support::validation()} else {support::photographic().into_iter().map(|(name,bytes)|(name,bytes,String::from("corpus"))).collect::<Vec<_>>()};
    if !validation_corpus {
        for (name,bytes) in support::generated() {
            let path=generated.join(&name);
            let bytes=if path.exists() {let stored=fs::read(&path)?;if !img::optimizer::samples_equal(&StillImage::decode(&bytes)?,&StillImage::decode(&stored)?) {return Err(format!("generated fixture changed; update {name} explicitly").into());}stored} else {fs::write(path,&bytes)?;bytes};
            cases.push((name,bytes,"corpus/generated".into()));
        }
    }
    let current=std::env::var_os("IMG_BENCH_BIN").map(PathBuf::from).unwrap_or_else(||repository.join("target/dist/img"));
    let original=std::env::var_os("IMG_ORIGINAL_BIN").map(PathBuf::from).unwrap_or_else(||repository.join(".work/tmp/original-target/release/img"));
    let revision=String::from_utf8(Command::new("git").current_dir(repository).args(["rev-parse","HEAD"]).output()?.stdout)?.trim().to_owned();
    let source_diff=Command::new("git").current_dir(repository).args(["diff","HEAD","--","src","codecs/webpkit/src","Cargo.toml","Cargo.lock","examples/benchmark.rs","benches/support/mod.rs"]).output()?.stdout;
    let source_diff_sha256={use sha2::{Digest,Sha256};format!("{:x}",Sha256::digest(&source_diff))};
    let binary_sha256={use sha2::{Digest,Sha256};format!("{:x}",Sha256::digest(fs::read(&current)?))};
    let withdrawn=std::env::var_os("IMG_WITHDRAWN_BIN").map(PathBuf::from).unwrap_or_else(||repository.join(".work/wt/withdrawn/target/release/rimage"));
    let mut tools=vec![("img",current.as_os_str().to_owned(),"new"),("img-png",current.as_os_str().to_owned(),"new-png"),("original-mozjpeg",original.as_os_str().to_owned(),"original-jpeg"),("original-oxipng",original.as_os_str().to_owned(),"original-png"),("original-ravif",original.as_os_str().to_owned(),"original-avif"),("cwebp","cwebp".into(),"cwebp"),("cwebp-lossless","cwebp".into(),"cwebp-lossless")];
    if withdrawn.exists() {tools.push(("withdrawn",withdrawn.as_os_str().to_owned(),"withdrawn"));}
    let mut tool_digests=serde_json::Map::new();
    {
        use sha2::{Digest,Sha256};
        for (tool,executable,_) in &tools {
            let digest=fs::read(executable).ok().map(|bytes|format!("{:x}",Sha256::digest(&bytes))).unwrap_or_else(||"unavailable".into());
            tool_digests.insert((*tool).to_owned(),json!(digest));
        }
    }
    let rustc=Command::new("rustc").arg("--version").output().ok().map(|o|String::from_utf8_lossy(&o.stdout).trim().to_owned());
    let output_prefix=std::env::var("IMG_BENCH_OUTPUT_PREFIX").unwrap_or_default();
    let tool_filter: Option<Vec<String>> = std::env::var("IMG_BENCH_TOOLS").ok().map(|v| v.split(',').map(str::to_string).collect());
    let mut records=vec![];
    for run in 0..if timed {5} else {1} {
        for (name,source,source_directory) in &cases {
            let image=StillImage::decode(source)?;let (width,height)=image.dimensions();
            for target in [80.0,90.0,95.0] {
                let mut order=(0..tools.len()).collect::<Vec<_>>();if run%2==1 {order.reverse();}
                for index in order {
                    let (tool,executable,kind)=&tools[index];
                    if let Some(filter) = &tool_filter {
                        if !filter.iter().any(|t| t == tool) { continue; }
                    }
                    if kind.ends_with("png") && target!=80.0 {continue;}
                    let mut record=support::record_base(tool,name,&revision);record["attempts"]=json!([]);record["run"]=json!(run);record["target"]=json!(target);record["sourcePath"]=json!(format!("{source_directory}/{name}"));record["width"]=json!(width);record["height"]=json!(height);record["inputBytes"]=json!(source.len());record["parameters"]=match *kind {"new"=>json!({"target":target,"effort":"fast","globalCandidateCap":10}),"withdrawn"=>json!({"target":target,"effort":"fast","oracle":"rimage 5424df8 zenwebp 0.4.6"}),"new-png"=>json!({"lossless":true,"effort":"fast"}),"original-jpeg"=>json!({"qualityGrid":[1,25,50,75,95,100],"progressive":true,"qtable":"NRobidoux"}),"original-png"=>json!({"lossless":true,"effort":2}),"original-avif"=>json!({"qualityGrid":[1,25,50,75,95,100],"speed":6,"sourcePrecision":"original ravif defaults to 10-bit; scored decode may refuse"}),_=>json!({"qualityGrid":[0,25,50,75,95,100],"method":4,"alphaQuality":100,"exact":true,"metadata":"all"})};
                    let extension=if *kind=="original-jpeg" {"jpg"} else if kind.ends_with("png") {"png"} else if *kind=="original-avif" {"avif"} else {"webp"};
                    let input=benchmark.join(source_directory).join(name);
                    let target_name=format!("{}{}{}-{}-{}-{}.{}",output_prefix,if timed {"timed-"} else {""},name.replace('.',"_"),tool,target,run,extension);let saved=output_dir.join(&target_name);
                    let start=Instant::now();
                    let result=(||->Result<(Vec<u8>,Option<f64>,bool),String> {
                        let exact_kind=kind.ends_with("png")||*kind=="cwebp-lossless";
                        if image.bit_depth()>8 && !kind.ends_with("png") {return Err("quality comparison excludes high-depth samples; exact proof is separate".into());}
                        let reference=if exact_kind {None} else {Some(PerceptualReference::new(&image).map_err(|e|e.to_string())?)};
                        let mut winner:Option<(Vec<u8>,Option<f64>,bool)>=None;
                        let qualities=if *kind=="new"||*kind=="withdrawn"||kind.ends_with("png")||*kind=="cwebp-lossless" {vec![95]} else if kind.starts_with("original-") {vec![1,25,50,75,95,100]} else {vec![0,25,50,75,95,100]};
                        for quality in qualities {
                            let attempt=scratch.join(format!("{tool}-{quality}"));fs::create_dir_all(&attempt).map_err(|e|e.to_string())?;
                            let candidate=attempt.join(if kind.starts_with("original-") {format!("{}.{}",Path::new(name).file_stem().unwrap().to_str().unwrap(),extension)} else {format!("output.{extension}")});
                            if candidate.exists() {fs::remove_file(&candidate).map_err(|e|e.to_string())?;}
                            let mut command=Command::new(executable);command.env("RAYON_NUM_THREADS","1");
                            match *kind {
                                "new"|"withdrawn"=>{command.arg("convert").arg(&input).arg("-o").arg(&candidate).args(["--score",&target.to_string(),"--effort","fast","--quiet"]);},
                                "new-png"=>{command.arg("convert").arg(&input).arg("-o").arg(&candidate).args(["--lossless","--effort","fast","--quiet"]);},
                                "original-jpeg"=>{command.arg("mozjpeg").arg(&input).arg("-d").arg(&attempt).args(["-q",&quality.to_string(),"-t","1","--quiet"]);},
                                "original-png"=>{command.arg("oxipng").arg(&input).arg("-d").arg(&attempt).args(["--effort","2","-t","1","--quiet"]);},
                                "original-avif"=>{command.arg("avif").arg(&input).arg("-d").arg(&attempt).args(["-q",&quality.to_string(),"--speed","6","-t","1","--quiet"]);},
                                "cwebp-lossless"=>{command.args(["-quiet","-lossless","-z","9","-exact","-metadata","all"]).arg(&input).arg("-o").arg(&candidate);},
                                _=>{command.args(["-quiet","-q",&quality.to_string(),"-m","4","-alpha_q","100","-exact","-metadata","all"]).arg(&input).arg("-o").arg(&candidate);},
                            }
                            let invocation=std::iter::once(command.get_program()).chain(command.get_args()).map(|arg|arg.to_string_lossy().into_owned()).collect::<Vec<_>>();
                            let output=command.output().map_err(|e|format!("tool unavailable: {e}"))?;
                            record["attempts"].as_array_mut().unwrap().push(json!({"quality":if kind.ends_with("png")||*kind=="cwebp-lossless" {None} else {Some(quality)},"command":invocation,"exitCode":output.status.code(),"stderr":String::from_utf8_lossy(&output.stderr)}));
                            if !output.status.success() {continue;}
                            let bytes=fs::read(&candidate).map_err(|e|e.to_string())?;let decoded=StillImage::decode(&bytes).map_err(|e|e.to_string())?;
                            if decoded.dimensions()!=image.dimensions() {return Err("output dimensions differ".into());}
                            record["attempts"].as_array_mut().unwrap().last_mut().unwrap()["bytes"]=json!(bytes.len());
                            let samples_match=if *kind=="cwebp-lossless" && input.extension().is_some_and(|e|e=="jpg"||e=="jpeg") {
                                support::libwebp_domain_exact(&input,&candidate)?
                            } else {
                                img::optimizer::samples_equal(&image,&decoded)
                            };
                            if exact_kind&&!samples_match {continue;}
                            let score=if exact_kind {None} else {let score=reference.as_ref().unwrap().score_encoded(&bytes).map_err(|e|e.to_string())?;record["attempts"].as_array_mut().unwrap().last_mut().unwrap()["score"]=json!(score);if score<target {continue;}Some(score)};
                            if winner.as_ref().is_none_or(|w|bytes.len()<w.0.len()) {winner=Some((bytes,score,samples_match));}
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
    let document=json!({"schemaVersion":1,"revision":revision,"architecture":std::env::consts::ARCH,"rustc":rustc,"cargoProfile":"dist","rayonThreads":1,"toolSha256":Value::Object(tool_digests),"timingGate":if timed {"accepted"} else {"not measured: capture mode"},"runsPerSide":if timed {5} else {1},"sourceDirty":!source_diff.is_empty(),"sourceDiffSha256":source_diff_sha256,"binarySha256":binary_sha256,"tools":{"img":"locked repository dist","original":"66fc0fc plus docs/original-build.patch, release; mozjpeg 0.10.13 (mozjpeg-sys 2.2.3), oxipng 10.2.1, ravif 0.13.0","cwebp":"1.6.0, single-threaded; cwebp-lossless adds -lossless -z 9","withdrawn":"pre-merge master 5424df8 (zenwebp 0.4.6), IMG_WITHDRAWN_BIN or .work/wt/withdrawn","jpegli":"unavailable","dedicatedAvif":"standalone unavailable; original ravif 0.13.0 included but emits unsupported 10-bit"},"summary":support::summarize(&records),"records":records});
    let results_name=std::env::var("IMG_RESULTS_NAME").unwrap_or_else(|_| if timed {"timed.json".to_string()} else {"untimed.json".to_string()});
    fs::write(output_dir.join(results_name),serde_json::to_vec_pretty(&document)?)?;
    println!("saved {} records; {} completed",document["records"].as_array().unwrap().len(),document["records"].as_array().unwrap().iter().filter(|r|r["completed"]==true).count());
    Ok(())
}
