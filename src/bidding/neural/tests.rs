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

/// The honest-reading compact net clears the same parity bar.
#[test]
fn matches_candle_fixture_bba_v6() {
    check_fixture(
        include_str!("../weights/american_bba_v6.fixture.json"),
        |x| classify_bba_v6(x).iter().map(|(_, l)| *l).collect(),
    );
}

#[test]
fn matches_candle_fixture_bba_v6_their() {
    check_fixture(
        include_str!("../weights/american_bba_v6_their.fixture.json"),
        |x| classify_bba_v6_their(x).iter().map(|(_, l)| *l).collect(),
    );
}

/// Export gate: the full corpus scan found and folded exactly 30 constants.
#[test]
fn folded_v6_columns_are_exactly_zero() {
    let w1 = &WEIGHTS_BBA_V6[..HID * IN_V6];
    let zero = (0..IN_V6)
        .filter(|&i| (0..HID).all(|h| w1[h * IN_V6 + i].to_bits() == 0))
        .count();
    assert_eq!(zero, 30, "v6 artifact was not folded against its corpus");
}

#[test]
fn folded_v6_their_columns_are_exactly_zero() {
    let w1 = &WEIGHTS_BBA_V6_THEIR[..HID * IN_V6];
    let zero = (0..IN_V6)
        .filter(|&i| (0..HID).all(|h| w1[h * IN_V6 + i].to_bits() == 0))
        .count();
    assert_eq!(
        zero, 30,
        "BBA-reading twin was not folded against its corpus"
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

// ── v7 sequence forward pass ─────────────────────────────────────────────────

/// A deterministic pseudo-random weight blob — enough structure that a wrong
/// gate order or a transposed matrix cannot pass by symmetry.
///
/// The ±0.3 scale is chosen for *discrimination*, and it is a real tension.  At
/// ±0.1 (candle's Kaiming draw) every gate pre-activation sits near zero, so
/// every sigmoid is ≈0.5 and swapping the `i` and `f` gates changes almost
/// nothing — a mutation test confirmed such a swap passes unnoticed there.  At
/// ±1 the gates saturate and twenty recurrent steps amplify pure `f32`
/// summation-order noise past any useful tolerance.  ±0.3 spans the sigmoids
/// (pre-activations ≈ ±3) while keeping the recurrence stable, and the
/// reference below accumulates in `f64` so the residual is the tested path's
/// error alone.
fn pseudo_weights(n: usize) -> Vec<f32> {
    // A cheap LCG: reproducible across platforms, unlike hashing floats.
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    (0..n)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            (f32::from((state >> 40) as u16) / 32768.0 - 1.0) * 0.3
        })
        .collect()
}

/// An independent, deliberately naive transcription of candle's `LSTM::step`
/// plus the head — written from the equations rather than from `classify_lstm`,
/// so agreement means the shipped path is right, not that a bug was copied.
fn reference_lstm(weights: &[f32], features: &[f32], tokens: &[[f32; TOKEN_LEN_V7]]) -> Vec<f32> {
    let (w_ih, rest) = weights.split_at(G * TOK);
    let (w_hh, rest) = rest.split_at(G * HL);
    let (b_ih, rest) = rest.split_at(G);
    let (b_hh, head) = rest.split_at(G);

    let mut h = vec![0f64; HL];
    let mut c = vec![0f64; HL];
    for token in tokens {
        // Row-major `(G, in)`: gate row `g` dots the whole input.
        let gates: Vec<f64> = (0..G)
            .map(|g| {
                let from_input: f64 = (0..TOK)
                    .map(|k| f64::from(w_ih[g * TOK + k]) * f64::from(token[k]))
                    .sum::<f64>()
                    + f64::from(b_ih[g]);
                let from_hidden: f64 = (0..HL)
                    .map(|k| f64::from(w_hh[g * HL + k]) * h[k])
                    .sum::<f64>()
                    + f64::from(b_hh[g]);
                from_input + from_hidden
            })
            .collect();
        let sig = |x: f64| 1.0 / (1.0 + (-x).exp());
        for i in 0..HL {
            // candle chunks the 4H block as i, f, g̃, o.
            let (in_gate, forget, cell, out_gate) = (
                sig(gates[i]),
                sig(gates[HL + i]),
                gates[2 * HL + i].tanh(),
                sig(gates[3 * HL + i]),
            );
            c[i] = forget * c[i] + in_gate * cell;
            h[i] = out_gate * c[i].tanh();
        }
    }

    let mut joined = h;
    joined.extend(features.iter().copied().map(f64::from));
    assert_eq!(joined.len(), IN_HEAD);

    // The head, as three explicit affine layers with ReLU between.
    let (w1, rest) = head.split_at(HID * IN_HEAD);
    let (b1, rest) = rest.split_at(HID);
    let (w2, rest) = rest.split_at(N_W2);
    let (b2, rest) = rest.split_at(HID);
    let (w3, b3) = rest.split_at(N_W3);
    let layer = |w: &[f32], b: &[f32], x: &[f64], out: usize| -> Vec<f64> {
        (0..out)
            .map(|r| {
                (0..x.len())
                    .map(|k| f64::from(w[r * x.len() + k]) * x[k])
                    .sum::<f64>()
                    + f64::from(b[r])
            })
            .collect()
    };
    let mut h1 = layer(w1, b1, &joined, HID);
    h1.iter_mut().for_each(|v| *v = v.max(0.0));
    let mut h2 = layer(w2, b2, &h1, HID);
    h2.iter_mut().for_each(|v| *v = v.max(0.0));
    layer(w3, b3, &h2, OUT)
        .into_iter()
        .map(|v| v as f32)
        .collect()
}

fn pseudo_tokens(count: usize) -> Vec<[f32; TOKEN_LEN_V7]> {
    let flat = pseudo_weights(count * TOKEN_LEN_V7);
    flat.as_chunks::<TOKEN_LEN_V7>().0.to_vec()
}

#[test]
fn v7_blob_layout_is_pinned() {
    // 4·128·(98 + 128 + 2) LSTM floats, plus the (128+176)-input head.
    assert_eq!(G * TOK + G * HL + 2 * G, 116_736);
    assert_eq!(total_lstm(), 116_736 + total(IN_HEAD));
    assert_eq!(IN_HEAD, 128 + 176);
    assert_eq!(total_lstm(), 270_374);
}

#[test]
fn v7_matches_an_independent_transcription() {
    let weights = pseudo_weights(total_lstm());
    let features = pseudo_weights(FEATURES_LEN_V6);
    for steps in [0, 1, 2, 7, super::super::features::MAX_STEPS_V7] {
        let tokens = pseudo_tokens(steps);
        let mine = classify_lstm(&weights, &features, &tokens);
        let reference = reference_lstm(&weights, &features, &tokens);
        let mine: Vec<f32> = mine.into_values().collect();
        assert_eq!(
            argmax(&mine),
            argmax(&reference),
            "argmax differs at {steps}"
        );
        let worst = mine
            .iter()
            .zip(&reference)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        assert!(worst < 1e-3, "logits differ by {worst} at {steps} steps");
    }
}

#[test]
fn v7_an_empty_auction_runs_from_the_zero_state() {
    // The zero state must be what the head sees, i.e. the first 128 head inputs
    // are exactly 0 — this is what makes the trainer's `gather` at index 0
    // agree with serving a dealer's decision.
    let weights = pseudo_weights(total_lstm());
    let features = pseudo_weights(FEATURES_LEN_V6);
    let empty = classify_lstm(&weights, &features, &[]);

    let head = &weights[G * TOK + G * HL + 2 * G..];
    let mut joined = vec![0f32; HL];
    joined.extend_from_slice(&features);
    let direct = forward::<IN_HEAD>(head, &joined);
    assert_eq!(
        empty.into_values().collect::<Vec<_>>(),
        direct.into_values().collect::<Vec<_>>()
    );
}

#[test]
fn v7_later_calls_change_the_answer() {
    // A recurrence that ignored its input, or that overwrote rather than
    // accumulated state, would still pass the parity test above.
    let weights = pseudo_weights(total_lstm());
    let features = pseudo_weights(FEATURES_LEN_V6);
    let tokens = pseudo_tokens(4);
    let all: Vec<f32> = classify_lstm(&weights, &features, &tokens)
        .into_values()
        .collect();
    let dropped: Vec<f32> = classify_lstm(&weights, &features, &tokens[..3])
        .into_values()
        .collect();
    assert_ne!(all, dropped, "the last token must move the logits");

    let mut swapped = tokens.clone();
    swapped.swap(0, 1);
    let reordered: Vec<f32> = classify_lstm(&weights, &features, &swapped)
        .into_values()
        .collect();
    assert_ne!(
        all, reordered,
        "order must matter — that is the whole point"
    );
}
