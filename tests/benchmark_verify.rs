#![cfg(feature="build-binary")]
#[path="../benches/support/mod.rs"] mod support;
use rimage::{optimizer::{self,Fidelity,Operation,Request},perceptual::ScoreTarget,still::{Effort,Format,StillImage}};
use serde_json::json;
#[test]
fn licensed_corpus_and_actual_quality_sample_constraints() {
    assert_eq!(support::photographic().len(),4);
    let generated=support::generated();assert_eq!(generated.len(),10);
    for (name,bytes) in &generated {
        let image=StillImage::decode(bytes).unwrap();
        let exact=optimizer::process(bytes,Request {operation:Operation::Convert(if image.bit_depth()>8 {Format::Png} else {Format::Webp}),fidelity:Fidelity::Lossless,effort:Effort::Fast,..Request::default()}).unwrap();
        assert!(optimizer::samples_equal(&image,&StillImage::decode(&exact.bytes).unwrap()),"{name}");
    }
    let bytes=&generated[0].1;
    for target in [80.0,90.0,95.0] {
        let result=optimizer::process(bytes,Request {operation:Operation::Convert(Format::Webp),fidelity:Fidelity::Score(ScoreTarget::new(target).unwrap()),effort:Effort::Fast,..Request::default()}).unwrap();
        let score=rimage::perceptual::compare(bytes,&result.bytes).unwrap();assert!(score>=target);assert_eq!(result.evaluations.len(),8);
    }
    let saved=support::verify_saved(&support::root().join("results"));
    if support::root().join("results/untimed.json").exists() {assert!(saved>0);}
}
#[test]
fn saved_schema_and_gate_arithmetic_reject_fake_work_and_regressions() {
    use support::{Gate,gate,validate_record};
    assert_eq!(gate(Some((100.0,2.0)),Some((80.0,3.0)),true,true),Gate::Improvement);
    assert_eq!(gate(Some((100.0,2.0)),Some((120.0,3.0)),true,true),Gate::Regression);
    assert_eq!(gate(Some((100.0,2.0)),Some((101.0,3.0)),true,true),Gate::Unchanged);
    assert_eq!(gate(Some((100.0,2.0)),Some((80.0,3.0)),false,true),Gate::BaselineDebt);
    assert_eq!(gate(None,None,true,true),Gate::Deferred);assert_eq!(gate(None,None,true,false),Gate::Regression);
    let mut valid=json!({"schemaVersion":1,"tool":"fixture","fixture":"ui.png","revision":"fixed","completed":true,"width":8,"height":8,"outputWidth":8,"outputHeight":8,"inputBytes":100,"outputBytes":90,"lossless":false,"score":95.1,"target":95,"timingAccepted":false,"elapsedMs":null});
    validate_record(&valid).unwrap();valid["score"]=json!(94.9);assert!(validate_record(&valid).is_err());valid["score"]=json!(95.1);valid["outputWidth"]=json!(9);assert!(validate_record(&valid).is_err());valid["outputWidth"]=json!(8);valid["elapsedMs"]=json!(0.01);assert!(validate_record(&valid).is_err());valid["elapsedMs"]=json!(null);valid["lossless"]=json!(true);valid["exactSamples"]=json!(false);assert!(validate_record(&valid).is_err());valid["completed"]=json!(false);assert!(validate_record(&valid).is_err());valid["failure"]=json!("tool unavailable");validate_record(&valid).unwrap();
}
