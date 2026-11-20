#[path = "../benches/support/mod.rs"]
mod support;
use img::optimizer::{self, Fidelity, FormatSelection, MetadataPolicy, Operation, Request};
use img::perceptual::ScoreTarget;
use img::still::{Effort, Format};
use serde_json::json;

fn main() {
    if let Err(error) = run() {
        eprintln!("scratch-select: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut cases = support::photographic();
    cases.extend(support::generated());
    let mut records = vec![];
    for (name, bytes) in &cases {
        for target in [80.0, 90.0, 95.0] {
            let request = Request {
                operation: Operation::Convert(Format::Webp),
                fidelity: Fidelity::Score(ScoreTarget::new(target).unwrap()),
                effort: Effort::Fast,
                metadata: MetadataPolicy::Keep,
                max_dimension: None,
                ..Request::default()
            };
            records.push(dump(name, &format!("convert-webp-t{target}"), &bytes, request));
        }
    }
    for name in ["coffee.png", "grass.png", "large.png"] {
        let bytes = cases.iter().find(|(n, _)| n == name).map(|(_, b)| b.clone()).unwrap();
        for target in [80.0, 90.0, 95.0] {
            let request = Request {
                operation: Operation::Optimize(FormatSelection::Auto),
                fidelity: Fidelity::Score(ScoreTarget::new(target).unwrap()),
                effort: Effort::Fast,
                metadata: MetadataPolicy::Keep,
                max_dimension: None,
                ..Request::default()
            };
            records.push(dump(name, &format!("auto-t{target}"), &bytes, request));
            let request = Request {
                operation: Operation::Convert(Format::Webp),
                fidelity: Fidelity::Score(ScoreTarget::new(target).unwrap()),
                effort: Effort::Balanced,
                metadata: MetadataPolicy::Keep,
                max_dimension: None,
                ..Request::default()
            };
            records.push(dump(name, &format!("convert-webp-balanced-t{target}"), &bytes, request));
        }
    }
    println!("{}", serde_json::to_string(&records)?);
    Ok(())
}

fn dump(name: &str, label: &str, bytes: &[u8], request: Request) -> serde_json::Value {
    eprintln!("{name} {label}");
    match optimizer::process(bytes, request) {
        Ok(outcome) => json!({
            "case": name, "request": label, "ok": true,
            "format": format!("{:?}", outcome.format),
            "bytes": outcome.bytes.len(),
            "score": outcome.score,
            "unchanged": outcome.unchanged,
            "evaluations": outcome.evaluations.iter().map(|e| json!({
                "format": format!("{:?}", e.format),
                "quality": e.quality,
                "bytes": e.bytes,
                "score": e.score,
                "failure": e.failure,
            })).collect::<Vec<_>>(),
        }),
        Err(error) => json!({"case": name, "request": label, "ok": false, "error": error.to_string()}),
    }
}
