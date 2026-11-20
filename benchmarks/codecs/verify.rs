//! Independent output verification for the workflow and batch runners.
//! stdin: one `SOURCE\tOUTPUT\tREQUIREMENT` line per output, REQUIREMENT `lossless` or a score target.
//! stdout: one JSON object per line with the exact-sample check, the measured score and the verdict.
use std::{collections::HashMap, fs, io::BufRead};
use img::{optimizer::samples_equal, perceptual::PerceptualReference, still::StillImage};

fn quote(s: &str) -> String { format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ")) }

fn main() {
    let mut references: HashMap<String, (StillImage, Option<PerceptualReference>)> = HashMap::new();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let [source, output, requirement] = line.split('\t').collect::<Vec<_>>()[..] else { panic!("malformed line {line:?}") };
        let result = (|| -> Result<(bool, f64), String> {
            if !references.contains_key(source) {
                let image = StillImage::decode(&fs::read(source).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                references.insert(source.to_owned(), (image, None));
            }
            let (image, metric) = references.get_mut(source).unwrap();
            let bytes = fs::read(output).map_err(|e| e.to_string())?;
            if samples_equal(image, &StillImage::decode(&bytes).map_err(|e| e.to_string())?) { return Ok((true, 100.0)); }
            if metric.is_none() { *metric = Some(PerceptualReference::new(image).map_err(|e| e.to_string())?); }
            Ok((false, metric.as_ref().unwrap().score_encoded(&bytes).map_err(|e| e.to_string())?))
        })();
        let (exact, score, error) = match result { Ok((e, s)) => (Some(e), Some(s), None), Err(e) => (None, None, Some(e)) };
        let accepted = match (requirement, exact, score) {
            ("lossless", Some(exact), _) => exact,
            (target, _, Some(score)) if target != "lossless" => score >= target.parse::<f64>().expect("score target"),
            _ => false,
        };
        println!("{{\"source\":{},\"output\":{},\"requirement\":{},\"exact\":{},\"score\":{},\"accepted\":{},\"error\":{}}}",
            quote(source), quote(output), quote(requirement),
            exact.map_or("null".into(), |e| e.to_string()), score.map_or("null".into(), |s| format!("{s:.17}")),
            accepted, error.map_or("null".into(), |e| quote(&e)));
    }
}
