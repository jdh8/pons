use super::*;

/// Synthetic rows that obey **both** structural constraints a real deal
/// does: eight holdings totalling twenty-six cards. Six of size 3 and two
/// of size 4 is the laziest composition that does, and `gauge`'s returned
/// constant is only prediction-preserving on rows shaped like this.
fn synthetic_rows() -> Vec<[usize; HOLDINGS]> {
    let by_size = |size: usize| -> Vec<usize> {
        (0..CELLS)
            .filter(|&c| cell_size(c) as usize == size)
            .collect()
    };
    let (threes, fours) = (by_size(3), by_size(4));
    (0..threes.len() * fours.len())
        .map(|i| {
            std::array::from_fn(|k| {
                if k < 6 {
                    threes[(i + 5 * k) % threes.len()]
                } else {
                    fours[(i / threes.len() + 3 * k) % fours.len()]
                }
            })
        })
        .collect()
}

/// The fit must recover an exact linear law from noiseless rows — the
/// smallest check that fails if the gauge-fixing or the pseudo-inverse
/// breaks. Every gauge in the family predicts identically, so assert on
/// predictions rather than on raw weights.
#[test]
fn recovers_a_known_law() {
    let truth = |c: usize| 0.1 * cell_size(c) + 0.37 * f64::from((c / SPOTS).count_ones());
    let rows = synthetic_rows();

    let mut normals = Normals::new();
    let mut freq = [0.0; CELLS];
    for cells in &rows {
        normals.push(cells, cells.iter().map(|&c| truth(c)).sum());
        for &c in cells {
            freq[c] += 1.0;
        }
    }
    let total: f64 = freq.iter().sum();
    for f in &mut freq {
        *f /= total;
    }

    let mut fitted = normals.solve();
    let constant = gauge(&mut fitted, &freq);

    for cells in rows.iter().step_by(17) {
        let want: f64 = cells.iter().map(|&c| truth(c)).sum();
        let got = constant + cells.iter().map(|&c| fitted[c]).sum::<f64>();
        assert!((want - got).abs() < 1e-6, "want {want}, got {got}");
    }
}

/// The published gauge has to be the *same* gauge on every slice, or two
/// runs publish different tables for the same law.
#[test]
fn gauge_is_pinned() {
    let rows = synthetic_rows();
    let mut freq = [0.0; CELLS];
    for cells in &rows {
        for &c in cells {
            freq[c] += 1.0;
        }
    }
    let total: f64 = freq.iter().sum();
    for f in &mut freq {
        *f /= total;
    }

    // Two representatives of one law, a full gauge transform apart.
    let base: DVector<f64> = DVector::from_fn(CELLS, |c, _| 0.1 * cell_size(c));
    let (alpha, beta) = (0.75, -8.0 * 0.75 / 26.0);
    let shifted = DVector::from_fn(CELLS, |c, _| base[c] + alpha + beta * cell_size(c));

    let (mut a, mut b) = (base, shifted);
    gauge(&mut a, &freq);
    gauge(&mut b, &freq);
    for c in 0..CELLS {
        assert!((a[c] - b[c]).abs() < 1e-9, "cell {c}: {} vs {}", a[c], b[c]);
    }
}

#[test]
fn cell_round_trips() {
    assert_eq!(cell_name(cell_from(&[Rank::A, Rank::Q], 3)), "AQxxx");
    assert_eq!(cell_name(0), "void");
    assert_eq!(cell_size(cell_from(&[Rank::A, Rank::Q], 3)), 5.0);
}

fn cell_from(honours: &[Rank], spots: usize) -> usize {
    let mask = HONOURS.iter().enumerate().fold(0, |acc, (i, r)| {
        acc | (usize::from(honours.contains(r)) << (HONOURS.len() - 1 - i))
    });
    mask * SPOTS + spots
}
