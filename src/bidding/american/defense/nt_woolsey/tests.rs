use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use crate::bidding::american::NotrumpDefense;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// Best call with Woolsey forced on (default ranges) and the conflicting
/// overlay off, independent of any other test.
fn woolsey(auction: &[Call], hand: &str) -> (Call, bool) {
    // The additive both-minors `2NT` is off for this arm so it cannot outrank
    // a Woolsey call.
    let mut arm = Agreements::default();
    arm.decision.reading.convention_points = (9, 19);
    arm.decision.reading.woolsey_double_floor = 11;
    arm.decision.reading.notrump_defense = NotrumpDefense::Woolsey;
    arm.defense.unusual_notrump_range = None;
    best_call_with(&arm, auction, hand)
}

#[test]
fn woolsey_direct_seat_routes_every_shape() {
    let over_1nt = [call(1, Strain::Notrump)];
    // 2♦ Multi: a single 6-card heart suit (other major short).
    let (multi, floored) = woolsey(&over_1nt, "32.KQJ987.A32.32");
    assert_eq!(multi, call(2, Strain::Diamonds));
    assert!(
        !floored,
        "the Woolsey overcall must come from the book node"
    );
    // 2♣ both majors: 5-4.
    assert_eq!(
        woolsey(&over_1nt, "AJ987.KQ32.32.32").0,
        call(2, Strain::Clubs)
    );
    // 2♥ Muiderberg: exactly 5 hearts + a 4-card minor, short spades.
    assert_eq!(
        woolsey(&over_1nt, "32.AQJ98.K987.2").0,
        call(2, Strain::Hearts)
    );
    // X: a 4-card major + a longer (5-card) minor, 11+.
    assert_eq!(woolsey(&over_1nt, "AKQ8.32.KJ987.32").0, Call::Double);
}

#[test]
fn woolsey_has_no_penalty_double() {
    let over_1nt = [call(1, Strain::Notrump)];
    // A flat 22-count has no Woolsey bid — it passes, exactly as in BBA's read
    // (there is no penalty double in this structure).
    let (strong, floored) = woolsey(&over_1nt, "AQ32.KQ3.KQ3.AQ2");
    assert_eq!(strong, Call::Pass);
    assert!(!floored, "the settling Pass must come from the book node");
    // A bare 5332 with a five-card major (no 4-card minor) also passes.
    assert_eq!(woolsey(&over_1nt, "AKJ32.K32.Q32.32").0, Call::Pass);
}

#[test]
fn woolsey_multi_advance_pass_or_corrects() {
    // `(1NT) 2♦ -` — a weak advancer bids the 2♥ pass-or-correct.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = woolsey(&auction, "32.K32.J32.J5432");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the Multi advance must come from the book node");
}

#[test]
fn woolsey_x_advance_never_sits_for_penalty() {
    // `(1NT) X -` — the X is takeout, so a weak no-major advancer relays 2♣
    // (names the doubler's minor), never passing to defend a phantom 1NTx.
    let auction = [call(1, Strain::Notrump), Call::Double, Call::Pass];
    let (relay, floored) = woolsey(&auction, "432.432.432.5432");
    assert_eq!(relay, call(2, Strain::Clubs));
    assert!(!floored, "the X advance must come from the book node");
    // With a good 5-card major of its own, the advancer bids it to play.
    assert_eq!(
        woolsey(&auction, "KQ982.32.432.432").0,
        call(2, Strain::Spades)
    );
}

#[test]
fn woolsey_muiderberg_advance_raises_and_asks() {
    // `(1NT) 2♥ -` — a known 5-card heart suit.  With support + game values the
    // advancer raises to 4♥; with no fit it asks the minor via 2NT (a book node).
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        Call::Pass,
    ];
    let (raise, floored) = woolsey(&auction, "32.K54.AK32.AQ32");
    assert_eq!(raise, call(4, Strain::Hearts));
    assert!(
        !floored,
        "the Muiderberg advance must come from the book node"
    );
    // No heart fit (singleton), invitational+ → 2NT minor-ask, never a floored guess.
    assert_eq!(
        woolsey(&auction, "KQJ2.2.K432.Q432").0,
        call(2, Strain::Notrump)
    );
}

#[test]
fn woolsey_muiderberg_doubled_escapes_a_misfit() {
    // `(1NT) 2♥ (X)` — a weak hand short in hearts escapes the doubled misfit via
    // the 2NT minor-ask rather than sitting in a doubled 5-1 fit.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        Call::Double,
    ];
    let (escape, floored) = woolsey(&auction, "Q432.2.J432.J432");
    assert_eq!(escape, call(2, Strain::Notrump));
    assert!(!floored, "the doubled escape must come from the book node");
    // With a genuine fit it sits for 2♥x (a known 8-card trump fit).
    assert_eq!(woolsey(&auction, "Q43.K52.J432.432").0, Call::Pass);
}

#[test]
fn woolsey_muiderberg_2nt_names_the_minor() {
    // `(1NT) 2♥ - 2NT -` — the overcaller answers the minor-ask: 3♦ with
    // diamonds, 3♣ with clubs (it always holds a 4+ minor).
    let asked = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(
        woolsey(&asked, "2.AKJ32.Q432.32").0,
        call(3, Strain::Diamonds)
    );
    assert_eq!(woolsey(&asked, "2.AKJ32.32.Q432").0, call(3, Strain::Clubs));
}

/// Best call under Woolsey with the completion knobs set: `sit` =
/// `defense.woolsey_muiderberg_sit`, `tails` = `defense.woolsey_tails`, on the
/// shipped Woolsey band (8:19, so invite = 12 and game = 14 points).
fn completed(sit: bool, tails: bool, auction: &[Call], hand: &str) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.decision.reading.notrump_defense = NotrumpDefense::Woolsey;
    arm.defense.woolsey_muiderberg_sit = sit;
    arm.defense.woolsey_tails = tails;
    best_call_with(&arm, auction, hand)
}

#[test]
fn muiderberg_sit_escapes_only_short() {
    // `(1NT) 2♥ (X)` — a doubleton heart sits 2♥x with the knob on, escapes
    // via 2NT without it.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        Call::Double,
    ];
    let doubleton = "Q432.32.J432.J43";
    assert_eq!(
        completed(false, false, &auction, doubleton).0,
        call(2, Strain::Notrump)
    );
    let (sat, floored) = completed(true, false, &auction, doubleton);
    assert_eq!(sat, Call::Pass);
    assert!(!floored, "the sit must come from the book node");
    // A singleton still escapes; three-card support still sits; a 4-card
    // raise is unchanged.
    for sit in [false, true] {
        assert_eq!(
            completed(sit, false, &auction, "Q432.2.J432.J432").0,
            call(2, Strain::Notrump)
        );
        assert_eq!(
            completed(sit, false, &auction, "Q43.K52.J432.432").0,
            Call::Pass
        );
        assert_eq!(
            completed(sit, false, &auction, "Q43.K542.J432.32").0,
            call(3, Strain::Hearts)
        );
    }
}

#[test]
fn muiderberg_sit_overcaller_passes_their_runout() {
    // `(1NT) 2♥ (X) - (3♦)` and `(1NT) 2♠ (X) - (3♥)` — the advancer sat, they
    // ran: the overcaller passes from the book, never re-competing.
    for (major, run, hand) in [
        (
            Strain::Hearts,
            call(3, Strain::Diamonds),
            "2.AKJ32.KQ32.K32",
        ),
        (Strain::Hearts, call(2, Strain::Spades), "2.AKJ32.KQ32.K32"),
        (Strain::Spades, call(3, Strain::Hearts), "AKJ32.2.KQ32.K32"),
        (Strain::Spades, call(4, Strain::Clubs), "AKJ32.2.KQ32.K32"),
    ] {
        let auction = [
            call(1, Strain::Notrump),
            call(2, major),
            Call::Double,
            Call::Pass,
            run,
        ];
        let (c, floored) = completed(true, false, &auction, hand);
        assert_eq!(c, Call::Pass, "{major:?} over {run:?}");
        assert!(
            !floored,
            "{major:?} over {run:?}: the pass must be authored"
        );
        // Knob off: the node is the floor's.
        assert!(completed(false, false, &auction, hand).1);
    }
}

#[test]
fn woolsey_tails_close_the_two_club_hole() {
    // `(1NT) 2♣ -` — 12 HCP, no four-card major: invitational, no 3M invite
    // (needs four), no signoff (below the invite), no 2NT (game).  Off it falls
    // to the floor; on, the catch-all bids the longer major or relays equal ones.
    let undoubled = [call(1, Strain::Notrump), call(2, Strain::Clubs), Call::Pass];
    let longer_hearts = "K2.AQ3.QJ32.5432";
    let equal = "K32.AQ3.QJ32.432";
    assert!(completed(false, false, &undoubled, longer_hearts).1);
    let (c, floored) = completed(false, true, &undoubled, longer_hearts);
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored);
    assert_eq!(
        completed(false, true, &undoubled, "AQ3.K2.QJ32.5432").0,
        call(2, Strain::Spades)
    );
    assert_eq!(
        completed(false, true, &undoubled, equal).0,
        call(2, Strain::Diamonds)
    );
    // `(1NT) 2♣ (X)` — the doubled table has the same hole; equal majors
    // relay with the SOS redouble.
    let doubled = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
    ];
    assert!(completed(false, false, &doubled, longer_hearts).1);
    assert_eq!(
        completed(false, true, &doubled, longer_hearts).0,
        call(2, Strain::Hearts)
    );
    assert_eq!(completed(false, true, &doubled, equal).0, Call::Redouble);
}

#[test]
fn woolsey_tails_double_of_the_multi_2h() {
    // `(1NT) 2♦ {-|(X)} 2♥ - - (X)` (reopening) and `(1NT) 2♦ {-|(X)} 2♥ (X) - -`
    // (direct) — the overcaller passed 2♥ with exactly six hearts: sit, and run
    // to 2♠ only with six spades.
    for rho in [Call::Double, Call::Pass] {
        let reopening = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            rho,
            call(2, Strain::Hearts),
            Call::Pass,
            Call::Pass,
            Call::Double,
        ];
        let direct = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            rho,
            call(2, Strain::Hearts),
            Call::Double,
            Call::Pass,
            Call::Pass,
        ];
        for auction in [&reopening[..], &direct[..]] {
            let (c, floored) = completed(false, true, auction, "Q432.32.J432.J43");
            assert_eq!(c, Call::Pass);
            assert!(!floored, "the sit must come from the book node");
            assert_eq!(
                completed(false, true, auction, "QJ9842.32.J43.43").0,
                call(2, Strain::Spades)
            );
            // A singleton heart is still a 6-1 fit: a 3=1=5=4 or 4=1=5=3
            // advancer sits rather than run to 2♠ opposite 0–3 spades.
            for short_heart in ["Q43.2.J5432.J432", "Q432.2.J5432.J43"] {
                let (c, floored) = completed(false, true, auction, short_heart);
                assert_eq!(c, Call::Pass, "{short_heart} must not bid 2♠");
                assert!(!floored);
            }
            assert!(completed(false, false, auction, "Q432.32.J432.J43").1);
        }
    }
}

#[test]
fn woolsey_tails_answer_their_2h_over_the_multi() {
    // `(1NT) 2♦ (2♥)` — X = game interest, 2♠ = pass-or-correct, else pass.
    let over_2h = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Hearts),
    ];
    let (pc, floored) = completed(false, true, &over_2h, "Q43.J2.J5432.432");
    assert_eq!(pc, call(2, Strain::Spades));
    assert!(!floored);
    assert_eq!(
        completed(false, true, &over_2h, "AQ3.K2.KJ432.K32").0,
        Call::Double
    );
    assert_eq!(
        completed(false, true, &over_2h, "Q4.2.J65432.Q432").0,
        Call::Pass
    );
    // Spade tolerance alone: a short-heart hand competes too.
    assert_eq!(
        completed(false, true, &over_2h, "Q43.2.J5432.Q432").0,
        call(2, Strain::Spades)
    );
    assert!(completed(false, false, &over_2h, "Q43.J2.J5432.432").1);

    // The overcaller answers the pass-or-correct, passed or doubled: pass with
    // spades, 3♥ with hearts.
    for after in [Call::Pass, Call::Double] {
        let pc = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            call(2, Strain::Hearts),
            call(2, Strain::Spades),
            after,
        ];
        assert_eq!(
            completed(false, true, &pc, "32.KQJ987.A32.32").0,
            call(3, Strain::Hearts)
        );
        let (c, floored) = completed(false, true, &pc, "KQJ987.32.A32.32");
        assert_eq!(c, Call::Pass);
        assert!(!floored);
    }

    // …and the values double: 2♠ with spades, sit 2♥x with hearts.
    let x = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Hearts),
        Call::Double,
        Call::Pass,
    ];
    assert_eq!(
        completed(false, true, &x, "KQJ987.32.A32.32").0,
        call(2, Strain::Spades)
    );
    let (c, floored) = completed(false, true, &x, "32.KQJ987.A32.32");
    assert_eq!(c, Call::Pass);
    assert!(!floored);
}

#[test]
fn woolsey_tails_answer_their_2s_over_the_multi() {
    // `(1NT) 2♦ (2♠)` — 3♥ = pass-or-correct with heart tolerance, else pass.
    let over_2s = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Spades),
    ];
    let (pc, floored) = completed(false, true, &over_2s, "32.Q43.J5432.432");
    assert_eq!(pc, call(3, Strain::Hearts));
    assert!(!floored);
    assert_eq!(
        completed(false, true, &over_2s, "32.Q4.J5432.Q432").0,
        Call::Pass
    );
    // Heart tolerance alone: a short-spade hand competes too.
    assert_eq!(
        completed(false, true, &over_2s, "2.Q43.J5432.Q432").0,
        call(3, Strain::Hearts)
    );
    assert!(completed(false, false, &over_2s, "32.Q43.J5432.432").1);

    // The overcaller passes hearts, corrects to 3♠ with spades.
    for after in [Call::Pass, Call::Double] {
        let pc = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            call(2, Strain::Spades),
            call(3, Strain::Hearts),
            after,
        ];
        let (c, floored) = completed(false, true, &pc, "32.KQJ987.A32.32");
        assert_eq!(c, Call::Pass);
        assert!(!floored);
        assert_eq!(
            completed(false, true, &pc, "KQJ987.32.A32.32").0,
            call(3, Strain::Spades)
        );
    }
}

#[test]
fn woolsey_tails_overcaller_balances_their_major_over_the_multi() {
    // `(1NT) 2♦ (2♥) - -` — 2♠ with the spade Multi, else pass.
    let over_2h = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Hearts),
        Call::Pass,
        Call::Pass,
    ];
    let (c, floored) = completed(false, true, &over_2h, "KQJ987.32.A32.32");
    assert_eq!(c, call(2, Strain::Spades));
    assert!(!floored, "the balance must come from the book node");
    let (c, floored) = completed(false, true, &over_2h, "32.KQJ987.A32.32");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the pass must come from the book node");
    assert!(completed(false, false, &over_2h, "KQJ987.32.A32.32").1);

    // `(1NT) 2♦ (2♠) - -` — 3♥ only with seven hearts, else pass.
    let over_2s = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Spades),
        Call::Pass,
        Call::Pass,
    ];
    let (c, floored) = completed(false, true, &over_2s, "2.KQJ9876.A32.32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the balance must come from the book node");
    for hand in ["32.KQJ987.A32.32", "KQJ987.32.A32.32"] {
        let (c, floored) = completed(false, true, &over_2s, hand);
        assert_eq!(c, Call::Pass, "{hand}");
        assert!(!floored, "the pass must come from the book node");
    }
    assert!(completed(false, false, &over_2s, "2.KQJ9876.A32.32").1);
}

#[test]
fn muiderberg_sit_overcaller_sits_the_double() {
    // `(1NT) 2M (X) - -` — the advancer sat and opener passed: the overcaller
    // sits 2Mx from the book (Muiderberg is exactly five; no escape exists).
    for (major, hand) in [
        (Strain::Hearts, "2.AKJ32.KQ32.K32"),
        (Strain::Spades, "AKJ32.2.KQ32.K32"),
    ] {
        let auction = [
            call(1, Strain::Notrump),
            call(2, major),
            Call::Double,
            Call::Pass,
            Call::Pass,
        ];
        let (c, floored) = completed(true, false, &auction, hand);
        assert_eq!(c, Call::Pass, "{major:?}");
        assert!(!floored, "{major:?}: the sit must be authored");
        // Knob off: the node is the floor's.
        assert!(completed(false, false, &auction, hand).1);
    }
}

/// The completion knobs keep the row invariants (alerts, weight ties); the
/// shipped-default invariant run sees neither, since Woolsey is opt-in.
#[test]
fn completion_knobs_hold_row_invariants() {
    let mut arm = Agreements::default();
    arm.decision.reading.notrump_defense = NotrumpDefense::Woolsey;
    arm.defense.woolsey_muiderberg_sit = true;
    arm.defense.woolsey_tails = true;
    crate::bidding::rows::assert_package_invariants(
        &arm,
        &[
            super::super::woolsey_package(),
            super::super::landy_advance_package(),
        ],
    );
}

/// Woolsey is ours alone (`common::mirror_agreements`, via
/// `ReadingProfile::with_house_defense`): when *we* open `1NT`, their `2♦`,
/// `2♣` and `X` keep the house reading rather than decoding as our Multi,
/// Landy and Woolsey double.
#[test]
fn their_calls_over_our_notrump_keep_the_house_reading() {
    use crate::bidding::Relative;
    use crate::bidding::american::american;
    use contract_bridge::auction::RelativeVulnerability;
    // The envelopes, not the whole `Inferences`: it also carries our own
    // reading profile, whose `notrump_defense` differs by construction.
    let read = |agreements: &Agreements, auction: &[Call]| {
        let read = american(agreements)
            .bind()
            .infer(RelativeVulnerability::NONE, auction);
        [
            Relative::Me,
            Relative::Lho,
            Relative::Partner,
            Relative::Rho,
        ]
        .map(|who| (*read.get(who), read.announced_union(who).clone()))
    };
    let mut ours = Agreements::default();
    ours.decision.reading.notrump_defense = NotrumpDefense::Woolsey;
    let nt = call(1, Strain::Notrump);
    for auction in [
        &[nt, call(2, Strain::Diamonds)][..],
        &[nt, call(2, Strain::Clubs)],
        &[nt, Call::Double],
        &[nt, call(2, Strain::Hearts)],
    ] {
        assert_eq!(
            read(&ours, auction),
            read(&Agreements::default(), auction),
            "{auction:?}"
        );
    }
}

/// Best call over their `1NT` with the shipped unusual `2NT` (8–13) and
/// `defense.woolsey_wide_unusual_2nt` set, under `system`
fn wide_2nt(system: NotrumpDefense, wide: bool, hand: &str) -> (Call, bool) {
    let mut arm = Agreements::default();
    arm.decision.reading.notrump_defense = system;
    arm.defense.woolsey_wide_unusual_2nt = wide;
    best_call_with(&arm, &[call(1, Strain::Notrump)], hand)
}

#[test]
fn woolsey_wide_2nt_houses_the_strong_minors() {
    // 15 HCP, 5-5 in the minors: above the 13-point ceiling, no major for X.
    let strong = "2.A3.AKJ54.QJ432";
    assert_ne!(
        wide_2nt(NotrumpDefense::Woolsey, false, strong).0,
        call(2, Strain::Notrump)
    );
    let (wide, floored) = wide_2nt(NotrumpDefense::Woolsey, true, strong);
    assert_eq!(wide, call(2, Strain::Notrump));
    assert!(!floored, "the wide 2NT must come from the book node");
    // A hand inside the old range bids 2NT either way.
    let weak = "2.43.KQ954.QJ432";
    for wide in [false, true] {
        assert_eq!(
            wide_2nt(NotrumpDefense::Woolsey, wide, weak).0,
            call(2, Strain::Notrump)
        );
    }
}

#[test]
fn woolsey_wide_2nt_is_inert_outside_woolsey() {
    for hand in ["2.A3.AKJ54.QJ432", "2.43.KQ954.QJ432", "AK2.Q3.AKJ5.Q432"] {
        assert_eq!(
            wide_2nt(NotrumpDefense::Natural, true, hand),
            wide_2nt(NotrumpDefense::Natural, false, hand)
        );
    }
}
