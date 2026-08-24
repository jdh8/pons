//! The two Multi `2♦` variants, exercised knobs-on
//!
//! The shipped agreements have both knobs off, so nothing here is reachable
//! from the default walks — every helper builds its own [`Agreements`].

use crate::bidding::Bidder;
use crate::bidding::agreements::Agreements;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Strain};

/// Agreements with the Multi gate on, and the champion knob as asked
fn multi(champion: bool) -> Agreements {
    let mut agreements = Agreements::default();
    agreements.opening.multi_two_diamonds = true;
    agreements.opening.multi_two_diamonds_champion = champion;
    agreements
}

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

const P: Call = Call::Pass;

/// The Dutch call after `auction`, under `agreements`
fn calls(agreements: &Agreements, auction: &[Call], hand: &str) -> Call {
    let partnership = super::super::dutch(agreements).bind();
    let hand = hand.parse().expect("a valid hand");
    let logits = partnership
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("a decision");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(call, _)| call)
        .unwrap()
}

/// The opening call for a first-seat hand
fn opens(agreements: &Agreements, hand: &str) -> Call {
    calls(agreements, &[], hand)
}

/// `2♦` and its trailing pass — the prefix every responder test shares
const OPENED: [Call; 2] = [Call::Bid(Bid::new(2, Strain::Diamonds)), Call::Pass];

/// Both packages preserve the declarative row invariants, in both variants.
#[test]
fn row_package_invariants() {
    for champion in [false, true] {
        crate::bidding::rows::assert_package_invariants(
            &multi(champion),
            &[
                super::super::openings::package(),
                super::super::responses::package(),
                super::package(),
            ],
        );
    }
}

/// The opening partition: one artificial `2♦` replaces all three weak twos.
#[test]
fn opening_partition() {
    let base = multi(false);
    // Exactly six hearts, 4–10: the Multi.
    assert_eq!(opens(&base, "x.AQxxxx.Kxx.xxx"), bid(2, Strain::Diamonds));
    // Exactly six spades, 4–10: the same call.
    assert_eq!(opens(&base, "KQxxxx.x.xxx.Qxx"), bid(2, Strain::Diamonds));
    // Seven hearts still preempts at the three level.
    assert_eq!(opens(&base, "x.AQxxxxx.Kxx.xx"), bid(3, Strain::Hearts));
    // Six *diamonds* has no opening at all now — it passes.
    assert_eq!(opens(&base, "xxx.xx.AQxxxx.Qx"), P);
    // …and with the knob off it is american's natural weak two again.
    let plain = Agreements::default();
    assert_eq!(opens(&plain, "xxx.xx.AQxxxx.Qx"), bid(2, Strain::Diamonds));
    assert_eq!(opens(&plain, "x.AQxxxx.Kxx.xxx"), bid(2, Strain::Hearts));
    assert_eq!(opens(&plain, "KQxxxx.x.xxx.Qxx"), bid(2, Strain::Spades));
}

/// The `2♦` opening is alerted, and it is the *only* weak two-level opening left.
#[test]
fn multi_replaces_every_weak_two() {
    let rules = super::super::openings::dutch_openings(&multi(false));
    let two_level: Vec<Call> = rules
        .rules()
        .iter()
        .map(|rule| rule.call())
        .filter(|call| matches!(call, Call::Bid(bid) if bid.level.get() == 2))
        .collect();
    assert_eq!(
        two_level,
        [
            bid(2, Strain::Clubs),
            bid(2, Strain::Notrump),
            bid(2, Strain::Diamonds),
        ],
        "the strong 2♣ and 2NT stay; 2♦ is the only weak two",
    );
    let multi_rule = rules
        .rules()
        .iter()
        .find(|rule| rule.call() == bid(2, Strain::Diamonds))
        .expect("the Multi rule");
    assert_eq!(multi_rule.alert(), Some(super::MULTI_2D));
}

/// Responder's base table — BBA's bands, our precedence.
#[test]
fn base_responses() {
    let base = multi(false);
    // Weak with two-card tolerance: the cheap pass-or-correct.
    assert_eq!(
        calls(&base, &OPENED, "xxx.xx.Qxxx.xxxx"),
        bid(2, Strain::Hearts)
    );
    // Six diamonds and a minimum: play `2♦`.
    assert_eq!(calls(&base, &OPENED, "xx.xx.KQxxxx.xxx"), P);
    // 12–17 with heart tolerance: the constructive pass-or-correct.
    assert_eq!(
        calls(&base, &OPENED, "KQx.Qxx.AQxx.xxx"),
        bid(2, Strain::Spades)
    );
    // 16+: the ask.
    assert_eq!(
        calls(&base, &OPENED, "AQx.Kxx.AQxx.Kxx"),
        bid(2, Strain::Notrump)
    );
    // 10–11 with both majors held: the artificial three-level try.
    assert_eq!(
        calls(&base, &OPENED, "Qxx.Kxx.QJxx.Qxx"),
        bid(3, Strain::Diamonds)
    );
    // A weak hand with three-card support either way: pass-or-correct to game.
    assert_eq!(
        calls(&base, &OPENED, "xxx.xxx.Qxxx.xxx"),
        bid(4, Strain::Diamonds)
    );
    // Seven of our own, 10–14: natural, to play.  (`3♦` is the artificial try
    // in this variant, so the natural seven-card rungs are `3♣`/`3♥`/`3♠`.)
    assert_eq!(
        calls(&base, &OPENED, "xx.xx.Qx.AKQxxxx"),
        bid(3, Strain::Clubs)
    );
}

/// Responder's champion table — the pass-or-correct ladder and the INV+ ask.
#[test]
fn champion_responses() {
    let champ = multi(true);
    // Weak with tolerance: still the cheap pass-or-correct.
    assert_eq!(
        calls(&champ, &OPENED, "xxx.xx.Qxxx.xxxx"),
        bid(2, Strain::Hearts)
    );
    // Three-card support both ways and weak: the three-level pass-or-correct,
    // where the base bids the artificial `3♦` try or `4♦`.
    assert_eq!(
        calls(&champ, &OPENED, "xxx.xxx.Qxxx.xxx"),
        bid(3, Strain::Hearts)
    );
    // Four hearts, three spades: `3♠`, which forces `4♥` if opener has hearts.
    assert_eq!(
        calls(&champ, &OPENED, "xxx.Qxxx.Kxx.xxx"),
        bid(3, Strain::Spades)
    );
    // 4-4 in the majors and weak: the ten-card-fit blast.
    assert_eq!(
        calls(&champ, &OPENED, "Qxxx.xxxx.Kxx.xx"),
        bid(4, Strain::Diamonds)
    );
    // Invitational values: the ask, four HCP below the base's 16+ floor.
    assert_eq!(
        calls(&champ, &OPENED, "KQx.Qxx.AQxx.xxx"),
        bid(2, Strain::Notrump),
    );
    // Six diamonds and real values: natural and forcing, not the base's
    // seven-card to-play bid.
    assert_eq!(
        calls(&champ, &OPENED, "Kx.x.AQJxxx.Kxxx"),
        bid(3, Strain::Diamonds)
    );
}

/// The pass-or-correct rebids, both ways.
#[test]
fn pass_or_correct_decisions() {
    let base = multi(false);
    let two_hearts = [OPENED[0], P, bid(2, Strain::Hearts), P];
    // Six hearts: pass and play it.
    assert_eq!(calls(&base, &two_hearts, "x.AQxxxx.Kxx.xxx"), P);
    // Six spades and a minimum: correct.
    assert_eq!(
        calls(&base, &two_hearts, "KQxxxx.x.xxx.xxx"),
        bid(2, Strain::Spades),
    );
    // Six spades and the 10-count maximum: jump.
    assert_eq!(
        calls(&base, &two_hearts, "KQJxxx.x.KQx.xxx"),
        bid(3, Strain::Spades),
    );
    // Over the constructive `2♠`, hearts go to `3♥` and spades pass.
    let two_spades = [OPENED[0], P, bid(2, Strain::Spades), P];
    assert_eq!(
        calls(&base, &two_spades, "x.AQxxxx.Kxx.xxx"),
        bid(3, Strain::Hearts),
    );
    assert_eq!(calls(&base, &two_spades, "KQxxxx.x.xxx.xxx"), P);
}

/// The ask answers, **maximum first** — this is the direction the WJ teacher
/// shares, and the one many European Multi cards invert.
#[test]
fn ask_answers_are_maximum_first() {
    for champion in [false, true] {
        let agreements = multi(champion);
        let asked = [OPENED[0], P, bid(2, Strain::Notrump), P];
        // Hearts, maximum → the cheapest step.
        assert_eq!(
            calls(&agreements, &asked, "x.AQJxxx.KQx.xxx"),
            bid(3, Strain::Clubs),
        );
        // Spades, maximum → the second step.
        assert_eq!(
            calls(&agreements, &asked, "KQJxxx.x.KQx.xxx"),
            bid(3, Strain::Diamonds),
        );
        // Hearts, minimum → the natural rung.
        assert_eq!(
            calls(&agreements, &asked, "x.QJxxxx.xxx.xxx"),
            bid(3, Strain::Hearts),
        );
        // Spades, minimum.
        assert_eq!(
            calls(&agreements, &asked, "QJxxxx.x.xxx.xxx"),
            bid(3, Strain::Spades),
        );
    }
}

/// The `4♣` ask is a transfer, so the strong hand declares the game.
#[test]
fn strong_ask_transfers_the_declaration() {
    let base = multi(false);
    let asked = [OPENED[0], P, bid(4, Strain::Clubs), P];
    assert_eq!(
        calls(&base, &asked, "x.AQxxxx.Kxx.xxx"),
        bid(4, Strain::Diamonds)
    );
    assert_eq!(
        calls(&base, &asked, "KQxxxx.x.xxx.Qxx"),
        bid(4, Strain::Hearts)
    );
    // …and responder completes it.
    let transferred = [
        OPENED[0],
        P,
        bid(4, Strain::Clubs),
        P,
        bid(4, Strain::Diamonds),
        P,
    ];
    assert_eq!(
        calls(&base, &transferred, "AQx.Kx.AQxx.AKxx"),
        bid(4, Strain::Hearts),
    );
}

/// Over their double the table rides unchanged and `XX` joins it — as the same
/// 16+ ask in the base, and as the champion's worse-major ask.
#[test]
fn redouble_over_their_double() {
    let doubled = [OPENED[0], Call::Double];
    // Base: 16+ redoubles, and opener answers on the same max-first ladder.
    let base = multi(false);
    assert_eq!(calls(&base, &doubled, "AQx.Kxx.AQxx.Kxx"), Call::Redouble);
    let asked = [OPENED[0], Call::Double, Call::Redouble, P];
    assert_eq!(
        calls(&base, &asked, "x.AQJxxx.KQx.xxx"),
        bid(3, Strain::Clubs)
    );
    // Champion: a long major of one's own asks for the major opener lacks.
    let champ = multi(true);
    assert_eq!(calls(&champ, &doubled, "AQJxx.Kx.AQx.Kxx"), Call::Redouble);
    assert_eq!(
        calls(&champ, &asked, "x.AQJxxx.KQx.xxx"),
        bid(2, Strain::Spades),
        "six hearts names spades",
    );
    assert_eq!(
        calls(&champ, &asked, "KQJxxx.x.KQx.xxx"),
        bid(2, Strain::Hearts),
        "six spades names hearts",
    );
    // Below responder's first call the double is stripped: the pass-or-correct
    // gets opener's undisturbed decision — and so does their double *of* the
    // pass-or-correct, where the floor used to jump to `4♥` on eight HCP.
    let corrected = [OPENED[0], Call::Double, bid(2, Strain::Hearts), P];
    assert_eq!(
        calls(&base, &corrected, "KQxxxx.x.xxx.xxx"),
        bid(2, Strain::Spades),
    );
    let redoubled = [
        OPENED[0],
        Call::Double,
        bid(2, Strain::Hearts),
        Call::Double,
    ];
    assert_eq!(
        calls(&base, &redoubled, "x.KQxxxx.Qxx.xxx"),
        P,
        "six hearts sits for the doubled pass-or-correct",
    );
    assert_eq!(
        calls(&base, &redoubled, "KQxxxx.x.xxx.xxx"),
        bid(2, Strain::Spades),
    );
}

/// Their `(2♥)` overcall: responder resolves the Multi to spades.
#[test]
fn over_their_heart_overcall() {
    let base = multi(false);
    let overcalled = [OPENED[0], bid(2, Strain::Hearts)];
    // Support and 8+: the cheap raise.
    assert_eq!(
        calls(&base, &overcalled, "KQx.Jxx.Qxxx.xxx"),
        bid(2, Strain::Spades)
    );
    // Support and 14+: the artificial `2NT` raise.
    assert_eq!(
        calls(&base, &overcalled, "Kxx.xxx.AQxx.AJx"),
        bid(2, Strain::Notrump),
    );
    // Limit raise or better: the cue.
    assert_eq!(
        calls(&base, &overcalled, "Kxx.xxx.AQJx.AKx"),
        bid(3, Strain::Hearts),
    );
    // Short in the major opener is assumed to hold: penalty.
    assert_eq!(calls(&base, &overcalled, "x.KJxx.AQxx.KJxx"), Call::Double);
}

/// Their `(2♠)` overcall — **the inherited hole**, pinned deliberately
///
/// BBA's table has no weak rung here: `2♥` is gone, support starts at `2NT` and
/// needs 14+, and the natural `3♥` is 20+.  A weak responder therefore passes
/// them out in `2♠`.  Copying it verbatim is the base's whole point; the repair
/// diverges from the teacher, so it is its own A/B.  If this assertion ever
/// changes, that A/B is what changed it.
#[test]
fn the_two_spade_hole_is_inherited() {
    for champion in [false, true] {
        let agreements = multi(champion);
        let overcalled = [OPENED[0], bid(2, Strain::Spades)];
        // Three-card heart support, weak: nothing to bid.
        assert_eq!(
            calls(&agreements, &overcalled, "xx.Kxx.Qxxx.xxxx"),
            P,
            "the (2♠) hole: a weak responder with support has no rung",
        );
        // The rungs that *do* exist still fire.
        assert_eq!(
            calls(&agreements, &overcalled, "xx.Kxx.AQxx.AJxx"),
            bid(2, Strain::Notrump),
        );
        assert_eq!(
            calls(&agreements, &overcalled, "xx.Kxx.AQJx.AKxx"),
            bid(3, Strain::Spades),
        );
    }
}

/// The champion's three-level pass-or-correct, and the forced `4♥` over `3♠`.
#[test]
fn champion_three_level_corrections() {
    let champ = multi(true);
    let three_hearts = [OPENED[0], P, bid(3, Strain::Hearts), P];
    assert_eq!(calls(&champ, &three_hearts, "x.AQxxxx.Kxx.xxx"), P);
    assert_eq!(
        calls(&champ, &three_hearts, "KQxxxx.x.xxx.xxx"),
        bid(3, Strain::Spades),
    );
    let three_spades = [OPENED[0], P, bid(3, Strain::Spades), P];
    assert_eq!(calls(&champ, &three_spades, "KQxxxx.x.xxx.xxx"), P);
    assert_eq!(
        calls(&champ, &three_spades, "x.AQxxxx.Kxx.xxx"),
        bid(4, Strain::Hearts),
        "the ♥ correction over 3♠ is forced to game",
    );
    // The champion's natural minors are forcing: opener must name the major.
    let three_clubs = [OPENED[0], P, bid(3, Strain::Clubs), P];
    assert_eq!(
        calls(&champ, &three_clubs, "x.AQxxxx.Kxx.xxx"),
        bid(3, Strain::Hearts),
    );
    // The base's `3♣` is natural to play, so opener passes it.
    assert_eq!(calls(&multi(false), &three_clubs, "x.AQxxxx.Kxx.xxx"), P);
}

/// Every american weak-two and Ogust key under `P* 2♦` is re-owned
///
/// american's `weak-two-responses` package authors `2♦ -`, `2♦ - 2♥ -`,
/// `2♦ - 2♠ -`, `2♦ - 3♣ -`, `2♦ - 2NT -` and the four `2♦ - 2NT - 3x -`
/// continuations, and `dutch::book` compiles this package *after* it.  A key
/// left behind would answer a Multi auction with Ogust, so each is probed for a
/// call the Multi table gives and Ogust does not.
#[test]
fn every_weak_two_key_is_re_owned() {
    let base = multi(false);
    // `2♦ -`: Ogust's own responses would raise diamonds; the Multi corrects.
    assert_eq!(
        calls(&base, &OPENED, "xxx.xx.Qxxx.xxxx"),
        bid(2, Strain::Hearts)
    );
    // `2♦ - 2NT -`: Ogust answers minimum-first in steps; the Multi names the
    // major, maximum-first.
    let asked = [OPENED[0], P, bid(2, Strain::Notrump), P];
    assert_eq!(
        calls(&base, &asked, "x.AQJxxx.KQx.xxx"),
        bid(3, Strain::Clubs)
    );
    // `2♦ - 2NT - 3♣ -`: Ogust reads `3♣` as a bad-suit minimum in *diamonds*;
    // here it is a maximum with six hearts, and the asker raises to game.
    let answered = [
        OPENED[0],
        P,
        bid(2, Strain::Notrump),
        P,
        bid(3, Strain::Clubs),
        P,
    ];
    assert_eq!(
        calls(&base, &answered, "AQx.Kxx.AQxx.Kxx"),
        bid(4, Strain::Hearts)
    );
    // `2♦ - 2♥ -` / `2♦ - 2♠ -`: american reads a new suit as forcing and
    // raises or rebids diamonds; the Multi passes or corrects.
    let two_hearts = [OPENED[0], P, bid(2, Strain::Hearts), P];
    assert_eq!(calls(&base, &two_hearts, "x.AQxxxx.Kxx.xxx"), P);
    let two_spades = [OPENED[0], P, bid(2, Strain::Spades), P];
    assert_eq!(calls(&base, &two_spades, "KQxxxx.x.xxx.xxx"), P);
    // `2♦ - 3♣ -`: natural and to play in the base, so opener passes rather
    // than giving american's raise-or-rebid reply.
    let three_clubs = [OPENED[0], P, bid(3, Strain::Clubs), P];
    assert_eq!(calls(&base, &three_clubs, "x.AQxxxx.Kxx.xxx"), P);
}

/// The reading tripwire: what `2♦!` and its pass-or-correct actually decode as
///
/// Rule projection is the whole disclosure mechanism here — nothing else reads
/// these calls — so the two failure modes it has are pinned directly.  A
/// **phantom diamond suit** is the disaster the alert invariant exists to
/// prevent: opener's one certainty is that the diamonds are not real.  A
/// **vacuous `Or`** is the other (`docs/ai-bidder/sampled-projection.md`): a
/// disjunction that projects to `0..=37` reads as nothing at all, and the
/// opening's shape is exactly such a disjunction.
#[test]
fn the_multi_reads_as_a_major_and_never_as_diamonds() {
    use crate::bidding::inference::{Inferences, Relative};

    for champion in [false, true] {
        let partnership = super::super::dutch(&multi(champion)).bind();
        let reading =
            Inferences::read(&partnership.prefixed_context(RelativeVulnerability::NONE, &OPENED));
        let opener = reading.get(Relative::Partner);
        assert_eq!(
            opener.length(contract_bridge::Suit::Diamonds).min,
            0,
            "the Multi must promise no diamonds",
        );
        assert!(
            opener.strength.hcp.max <= 10,
            "the weak-only band survived the disjunction (got {:?})",
            opener.strength.hcp,
        );
        // The `Or` is not vacuous: a hand with no six-card major is excluded,
        // one with six hearts and one with six spades are both admitted.
        let flat: contract_bridge::Hand = "Kxxx.Qxx.xxx.xxx".parse().unwrap();
        let hearts: contract_bridge::Hand = "x.AQxxxx.Kxx.xxx".parse().unwrap();
        let spades: contract_bridge::Hand = "KQxxxx.x.xxx.Qxx".parse().unwrap();
        assert!(
            !reading.admits(Relative::Partner, flat),
            "no six-card major"
        );
        assert!(reading.admits(Relative::Partner, hearts));
        assert!(reading.admits(Relative::Partner, spades));

        // The pass-or-correct stamps no suit of its own: `2♥` is not hearts.
        let corrected = [OPENED[0], P, bid(2, Strain::Hearts), P];
        let reading = Inferences::read(
            &partnership.prefixed_context(RelativeVulnerability::NONE, &corrected),
        );
        assert_eq!(
            reading
                .get(Relative::Partner)
                .length(contract_bridge::Suit::Hearts)
                .min,
            0,
            "the pass-or-correct 2♥ must promise no hearts",
        );
    }
}
