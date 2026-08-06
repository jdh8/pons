use super::*;

fn argmax(v: &[f32]) -> usize {
    v.iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap()
}

/// The hand-rolled forward pass must reproduce the trainer's candle logits
/// on the exported fixture (the M1.2 bit-match check), with an identical
/// arg-max on every row.
fn check_fixture(fixture: &str, classify: impl Fn(&[f32]) -> Vec<f32>) {
    let fx: serde_json::Value = serde_json::from_str(fixture).unwrap();
    let rows = fx["features"].as_array().unwrap();
    let golds = fx["logits"].as_array().unwrap();
    assert_eq!(rows.len(), golds.len());
    assert!(!rows.is_empty(), "fixture has no rows");

    let to_vec = |v: &serde_json::Value| -> Vec<f32> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap() as f32)
            .collect()
    };

    let mut max_abs = 0f32;
    for (frow, grow) in rows.iter().zip(golds) {
        let x = to_vec(frow);
        let gold = to_vec(grow);
        let pred = classify(&x);
        assert_eq!(pred.len(), gold.len());
        for (p, g) in pred.iter().zip(&gold) {
            max_abs = max_abs.max((p - g).abs());
        }
        assert_eq!(
            argmax(&pred),
            argmax(&gold),
            "arg-max (chosen call) differs"
        );
    }
    assert!(
        max_abs < 1.0e-3,
        "max abs logit diff {max_abs} exceeds tolerance"
    );
}

/// The configured net clears the parity bar at its wider input.  No knob is
/// armed: it reads the regime off the features, which is the whole point,
/// and it is the only artifact left to pin since the v3 twins went.
#[test]
fn matches_candle_fixture_bba_v4() {
    check_fixture(
        include_str!("../weights/american_bba_v4.fixture.json"),
        |x| classify_bba_v4(x).iter().map(|(_, l)| *l).collect(),
    );
}
