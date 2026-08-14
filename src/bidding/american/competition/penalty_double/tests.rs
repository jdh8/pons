use super::super::tests::{best_call_with, bid_transfer_dbl, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn takeout_authored_double() {
    // Takeout: short in their suit (2♦) with values doubles from the book —
    // a hand the `Penalty` style (4+ ♦) would never double.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer_dbl(
        super::penalty_double::DoubleStyle::Takeout,
        &auction,
        "K432.K432.32.Q43",
    );
    assert_eq!(c, Call::Double);
    assert!(
        !floored,
        "the authored takeout double must come from the book"
    );
}

#[test]
fn optional_double_two_three_cards() {
    // Optional: exactly 3 cards in their suit (♦) with values doubles…
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer_dbl(
        super::penalty_double::DoubleStyle::Optional,
        &auction,
        "K43.K43.432.Q43",
    );
    assert_eq!(c, Call::Double);
    assert!(!floored, "the optional double must come from the book");

    // …but a singleton in their suit does NOT double (it routes elsewhere).
    let (c, _) = bid_transfer_dbl(
        super::penalty_double::DoubleStyle::Optional,
        &auction,
        "K432.K432.2.Q432",
    );
    assert_ne!(
        c,
        Call::Double,
        "short-in-their-suit must not make an optional double"
    );
}

#[test]
fn opener_pulls_a_takeout_double() {
    // After 1NT (2♦) X -, opener has no authored node and falls to the
    // floor: a maximum with a diamond stopper pulls to 3NT…
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    let (c, floored) = bid_transfer_dbl(
        super::penalty_double::DoubleStyle::Takeout,
        &auction,
        "AQ2.AQ2.A32.Q432",
    );
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(floored, "opener's pull comes from the instinct floor");

    // …while a diamond stack sits for penalty (passes the double).
    let (c, _) = bid_transfer_dbl(
        super::penalty_double::DoubleStyle::Takeout,
        &auction,
        "K32.A32.AKQ2.J32",
    );
    assert_eq!(c, Call::Pass, "a trump stack converts to penalty");
}

#[test]
fn opener_leaves_in_responder_penalty_double_when_penalty_style() {
    use super::penalty_double::DoubleStyle;
    // `1NT (2♥) X -` — responder penalty-doubled their heart overcall.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        Call::Double,
        Call::Pass,
    ];
    // Penalty style + leave-in on: opener SITS, and it is an authored node.
    let mut on = Agreements::default();
    on.competition.lebensohl_style = super::lebensohl::LebensohlStyle::Plain;
    on.competition.double_style = DoubleStyle::Penalty;
    on.competition.penalty_double_leave_in = true;
    let (c_on, floored_on) = best_call_with(&on, &auction, "AQ5.J42.KQ3.K842"); // flat 15, no ♥ stop
    assert_eq!(c_on, Call::Pass, "penalty double left in");
    assert!(
        !floored_on,
        "the leave-in must be a book node, not the floor"
    );
    // Leave-in off: the floor reads the double as takeout and pulls — not a Pass.
    let mut off = on;
    off.competition.penalty_double_leave_in = false;
    let (c_off, floored_off) = best_call_with(&off, &auction, "AQ5.J42.KQ3.K842");
    assert!(
        floored_off,
        "off → the node is gone, opener falls to the floor"
    );
    assert_ne!(
        c_off,
        Call::Pass,
        "the floor advances the double instead of sitting"
    );
}

/// `competition.two_diamond_double` replaces the cooperative `(2♦)` double with a
/// real diamond penalty double, and opener sits on it.
#[test]
fn two_diamond_double_swaps_the_gate() {
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    // 5+ diamonds, 4+ HCP in the suit, 9+ overall.
    let mut armed = Agreements::default();
    armed.competition.two_diamond_double = Some((5, 4, 9));

    // ♦KQ654 with 10 HCP: the armed double fires, from the book.
    let (c, floored) = best_call_with(&armed, &auction, "K32.32.KQ654.Q32");
    assert_eq!(c, Call::Double, "a good five-card diamond suit doubles");
    assert!(!floored, "the diamond penalty double must be a book node");

    // ♦432 with 10 HCP and no major: today's Optional gate doubles it (2-3 in
    // their suit), the armed gate does not.
    let flat = "KQ3.K32.432.Q432";
    let (c, _) = best_call_with(&armed, &auction, flat);
    assert_ne!(
        c,
        Call::Double,
        "three small cannot penalty-double diamonds"
    );
    let (c, _) = best_call_with(&Agreements::default(), &auction, flat);
    assert_eq!(c, Call::Double, "unarmed, the optional double is unchanged");
}

#[test]
fn opener_sits_for_the_two_diamond_penalty_double() {
    // `1NT (2♦) X -` — responder promised the diamonds, so opener defends.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    let mut armed = Agreements::default();
    armed.competition.two_diamond_double = Some((5, 4, 9));
    // A maximum with a diamond stopper — exactly the hand the floor pulls to 3NT.
    let (c, floored) = best_call_with(&armed, &auction, "AQ2.AQ2.A32.Q432");
    assert_eq!(c, Call::Pass, "opener leaves in the diamond penalty double");
    assert!(!floored, "the leave-in must be a book node, not the floor");
}

/// The gate is only half the convention: opener and the floor must *see* the
/// diamonds it promises.  `project_authored` decodes alerted calls only, so an
/// unalerted version of this rule read as `points 8..` with every suit ⊤ — opener
/// competing over their runout blind to the suit it was told about.
#[test]
fn the_two_diamond_double_reads_as_diamonds() {
    use crate::bidding::Relative;
    use contract_bridge::Suit;
    use contract_bridge::auction::RelativeVulnerability;

    // `1NT (2♦) X -`, read from our opener's seat: the doubler is partner.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    let mut armed = Agreements::default();
    armed.competition.two_diamond_double = Some((5, 4, 9));
    let read = crate::bidding::american::american(&armed)
        .bind()
        .infer(RelativeVulnerability::NONE, &auction);
    let shown = read.announced(Relative::Partner);
    assert_eq!(
        shown.length(Suit::Diamonds).min,
        5,
        "the double must read as the diamond length it promised"
    );
    assert!(
        shown.strength.points.min >= 9,
        "the double must read as its strength floor"
    );

    // Unarmed, the cooperative double claims no suit — the contrast that makes
    // the assertion above meaningful rather than a tautology.
    let bare = crate::bidding::american::american(&Agreements::default())
        .bind()
        .infer(RelativeVulnerability::NONE, &auction);
    assert_eq!(
        bare.announced(Relative::Partner).length(Suit::Diamonds).min,
        0,
        "the optional double never promised a suit"
    );
}

#[test]
fn opener_cooperates_with_responder_optional_double() {
    use super::penalty_double::DoubleStyle;
    // `1NT (2♥) X -` — responder's OPTIONAL double (2-3 hearts + values).
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        Call::Double,
        Call::Pass,
    ];
    let mut arm = Agreements::default();
    arm.competition.lebensohl_style = super::lebensohl::LebensohlStyle::Plain;
    arm.competition.double_style = DoubleStyle::Optional;
    arm.competition.penalty_double_leave_in = true;
    // Three-card fit (♥Q93): stand and defend the doubled overcall.
    let (fit, floored) = best_call_with(&arm, &auction, "AK5.Q93.KJ54.Q5");
    assert_eq!(fit, Call::Pass, "a three-card fit stands");
    assert!(!floored, "the cooperation must be an authored node");
    // Doubleton in their suit + a five-card suit (♣AKQ76): run with xx.
    let (run, _) = best_call_with(&arm, &auction, "A52.93.KJ5.AKQ76");
    assert_eq!(
        run,
        call(3, Strain::Clubs),
        "a doubleton runs to the five-card suit"
    );
    // Doubleton but no five-card suit: nowhere to run, so stand.
    let (stuck, _) = best_call_with(&arm, &auction, "A52.93.KJ54.AKQ6");
    assert_eq!(stuck, Call::Pass, "a doubleton with no suit stands");
}
