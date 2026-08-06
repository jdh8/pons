use super::*;
use crate::bidding::verify::{accepts, compare, empty_context};
use contract_bridge::Hand;
use rand::SeedableRng;
use rand::rngs::StdRng;

const N: usize = 8000;

/// Sample `N` hands; assert `candidate` agrees with the intended `reference`
/// boolean on all of them and that the reference is not vacuously empty.
fn check(label: &str, candidate: impl Constraint, reference: impl Fn(Hand) -> bool) {
    let ctx = empty_context();
    let mut rng = StdRng::seed_from_u64(20_260_625);
    let report = compare(reference, |h| accepts(&candidate, h, &ctx), &mut rng, N);
    assert!(
        report.agrees(),
        "{label}: {} of {} hands disagree, e.g. {:?}",
        report.disagreements.len(),
        report.tested,
        report.disagreements.first(),
    );
    assert!(
        report.reference_accepts > 0,
        "{label}: reference accepts nothing — a vacuous guard",
    );
}

#[test]
fn reauthored_shapes_match_intended_spec() {
    use Suit::{Clubs, Diamonds, Hearts, Spades};
    let ln = |h: Hand, s: Suit| h[s].len();

    // Multi 2♦ (simplified): a 6+ major, both minors ≤4 (now incl. 6-5 / 6-6).
    check("woolsey_multi", woolsey_multi(), |h| {
        (ln(h, Hearts) >= 6 || ln(h, Spades) >= 6) && ln(h, Clubs) <= 4 && ln(h, Diamonds) <= 4
    });

    // Muiderberg 2♥/2♠: exactly 5 in the major, ≤3 the other, a 4+ minor.  The
    // `== 5` pins disjointness from the 6+ Multi; the other-major ≤3 from 2♣.
    for major in [Hearts, Spades] {
        let other = if major == Hearts { Spades } else { Hearts };
        check("woolsey_muiderberg", woolsey_muiderberg(major), move |h| {
            ln(h, major) == 5 && ln(h, other) <= 3 && (ln(h, Clubs) >= 4 || ln(h, Diamonds) >= 4)
        });
    }

    // Woolsey X (unchanged): 4 in one major, ≤3 the other, a 5-6 minor.
    check("woolsey_double_shape", woolsey_double_shape(), |h| {
        let four_major = (ln(h, Hearts) == 4 && ln(h, Spades) <= 3)
            || (ln(h, Spades) == 4 && ln(h, Hearts) <= 3);
        four_major && ((5..=6).contains(&ln(h, Clubs)) || (5..=6).contains(&ln(h, Diamonds)))
    });

    // both_majors_shape: 5-4 (false) / flat 4-4 (true).
    check("both_majors_shape(false)", both_majors_shape(false), |h| {
        (ln(h, Hearts) >= 5 && ln(h, Spades) >= 4) || (ln(h, Hearts) >= 4 && ln(h, Spades) >= 5)
    });
    check("both_majors_shape(true)", both_majors_shape(true), |h| {
        ln(h, Hearts) >= 4 && ln(h, Spades) >= 4
    });

    // DONT one-suiter X: one of ♣/♦/♥ at least `min`, the other three ≤3.
    for min in [5usize, 6] {
        check(
            "dont_one_suiter_direct",
            dont_one_suiter_direct(min),
            move |h| {
                let one = |long: Suit| {
                    ln(h, long) >= min
                        && [Clubs, Diamonds, Hearts, Spades]
                            .iter()
                            .all(|&s| s == long || ln(h, s) <= 3)
                };
                one(Clubs) || one(Diamonds) || one(Hearts)
            },
        );
    }

    // DONT minor+major: the minor 4+, a major 4+, one of them at least `longer`.
    for (minor, a44) in [
        (Clubs, true),
        (Clubs, false),
        (Diamonds, true),
        (Diamonds, false),
    ] {
        let longer = if a44 { 4 } else { 5 };
        check("dont_minor_major", dont_minor_major(minor, a44), move |h| {
            let hi = ln(h, Hearts).max(ln(h, Spades));
            ln(h, minor) >= 4 && hi >= 4 && (ln(h, minor) >= longer || hi >= longer)
        });
    }

    // DONT 2♥ both majors: flat 4-4 (true) / 5-4 (false).
    check("dont_both_majors(true)", dont_both_majors(true), |h| {
        ln(h, Hearts) >= 4 && ln(h, Spades) >= 4
    });
    check("dont_both_majors(false)", dont_both_majors(false), |h| {
        (ln(h, Hearts) >= 5 && ln(h, Spades) >= 4) || (ln(h, Hearts) >= 4 && ln(h, Spades) >= 5)
    });

    // Meckwell two-way X: a 6+ minor (other three ≤3) OR both majors (4-4 / 5-4).
    for a44 in [true, false] {
        let longer = if a44 { 4 } else { 5 };
        check(
            "meckwell_double_shape",
            meckwell_double_shape(6, a44),
            move |h| {
                let one_minor = (ln(h, Clubs) >= 6
                    && ln(h, Diamonds) <= 3
                    && ln(h, Hearts) <= 3
                    && ln(h, Spades) <= 3)
                    || (ln(h, Diamonds) >= 6
                        && ln(h, Clubs) <= 3
                        && ln(h, Hearts) <= 3
                        && ln(h, Spades) <= 3);
                let both_majors = ln(h, Hearts) >= 4
                    && ln(h, Spades) >= 4
                    && (ln(h, Hearts) >= longer || ln(h, Spades) >= longer);
                one_minor || both_majors
            },
        );
    }

    // Meckwell natural 2M: 5+ in the major, ≤3 the other major, both minors ≤3.
    for major in [Hearts, Spades] {
        let other = if major == Hearts { Spades } else { Hearts };
        check(
            "meckwell_natural_major",
            meckwell_natural_major(major),
            move |h| {
                ln(h, major) >= 5 && ln(h, other) <= 3 && ln(h, Clubs) <= 3 && ln(h, Diamonds) <= 3
            },
        );
    }
}
