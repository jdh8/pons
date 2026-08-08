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

/// The compact-config net clears the same parity bar at its narrower input.
#[test]
fn matches_candle_fixture_bba_v5() {
    check_fixture(
        include_str!("../weights/american_bba_v5.fixture.json"),
        |x| classify_bba_v5(x).iter().map(|(_, l)| *l).collect(),
    );
}

/// The shipped blob is folded (`scripts/fold-constant-inputs.py`): every card
/// column the v4 corpus never varied is bit-exactly zero in `W1`, its constant
/// contribution absorbed into `b1`, so a frozen card row is inert at serving
/// instead of a random hidden-layer vector — the frozen-coordinate tax of
/// `docs/ai-bidder/card-manifold.md`.  This is the export gate: a freshly
/// trained blob fails here until the fold is re-run, and a future net that
/// thaws more card axes updates the live set below to match its corpus.
#[test]
fn folded_card_columns_are_exactly_zero() {
    use crate::bidding::features::{FEATURES_LEN_V3, LEN_CARD};
    // The four in-block slots the v4 corpus varied per side: base-system
    // one-hot bits 0 (2/1 GF) and 2 (WJ), `1D opening with 5 cards`, and
    // `Kickback 1430`.
    let live: Vec<usize> = [FEATURES_LEN_V3, FEATURES_LEN_V3 + LEN_CARD]
        .into_iter()
        .flat_map(|base| [0, 2, 7, 77].into_iter().map(move |slot| base + slot))
        .collect();
    let w1 = &WEIGHTS_BBA_V4[..HID * IN_V4];
    for i in FEATURES_LEN_V3..IN_V4 {
        let column_zero = (0..HID).all(|h| w1[h * IN_V4 + i].to_bits() == 0);
        if live.contains(&i) {
            assert!(
                !column_zero,
                "live card slot {i} is all-zero: wrong live set"
            );
        } else {
            assert!(
                column_zero,
                "frozen card slot {i} has weight: unfolded blob"
            );
        }
    }
}

/// The v5 sibling of the fold gate, on the compact blocks: the scan-mode fold
/// zeroed every `Agreements` dim the v5 corpus held constant.  The live set is
/// the 13 dims per side the dump varied — the six flipped bools, both poles of
/// each flipped one-hot, and `dutch` (the `DEFAULT_CELLS` rotation).
#[test]
fn folded_compact_columns_are_exactly_zero() {
    use crate::bidding::features::{FEATURES_LEN_V3, LEN_COMPACT};
    // dutch, relocating, nmf, xyz, jordan, shape {Balanced, Wide6322},
    // defense {Natural, Woolsey}, lebensohl {Off, Transfer}, minors, landy.
    let live: Vec<usize> = [FEATURES_LEN_V3, FEATURES_LEN_V3 + LEN_COMPACT]
        .into_iter()
        .flat_map(|base| {
            [0, 1, 3, 4, 7, 13, 15, 16, 19, 23, 25, 26, 27]
                .into_iter()
                .map(move |slot| base + slot)
        })
        .collect();
    let w1 = &WEIGHTS_BBA_V5[..HID * IN_V5];
    for i in FEATURES_LEN_V3..IN_V5 {
        let column_zero = (0..HID).all(|h| w1[h * IN_V5 + i].to_bits() == 0);
        if live.contains(&i) {
            assert!(
                !column_zero,
                "live compact slot {i} is all-zero: wrong live set"
            );
        } else {
            assert!(
                column_zero,
                "frozen compact slot {i} has weight: unfolded blob"
            );
        }
    }
}
