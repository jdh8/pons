use super::*;
use crate::bidding::Bidder;
use crate::bidding::inference::Relative;
use contract_bridge::Suit;
use contract_bridge::auction::RelativeVulnerability;

const P: Call = Call::Pass;

fn bid(level: u8, strain: impl Into<Strain>) -> Call {
    Call::Bid(Bid::new(level, strain.into()))
}

/// The best call of `system` for `hand` after `auction`
fn best(system: &System, auction: &[Call], hand: &str) -> Call {
    let logits = system
        .bind()
        .classify(hand.parse().unwrap(), RelativeVulnerability::NONE, auction)
        .expect("a decision");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(call, _)| call)
        .unwrap()
}

/// Strawberry's call after an undisturbed `auction`
fn calls(auction: &[Call], hand: &str) -> Call {
    best(&strawberry(&Agreements::default()), auction, hand)
}

#[test]
fn row_package_invariants() {
    crate::bidding::rows::assert_package_invariants(
        &Agreements::default(),
        &[openings::package(), one_club::package()],
    );
}

/// The one-level partition, hand type by hand type.
#[test]
fn opening_partition() {
    let opens = |hand| calls(&[], hand);
    // The strong notrump, a five-card major included.
    assert_eq!(opens("AQ32.K53.QJ4.A92"), bid(1, Strain::Notrump));
    assert_eq!(opens("AKQ32.K32.K32.32"), bid(1, Strain::Notrump));
    // The weak notrump opens 1♣, four diamonds or not.
    assert_eq!(opens("KQ32.KJ32.K32.32"), bid(1, Suit::Clubs));
    assert_eq!(opens("KQ32.K2.KJ32.Q32"), bid(1, Suit::Clubs));
    // Five diamonds open 1♦ from 12; a balanced 11 passes.
    assert_eq!(opens("K32.A2.KJ432.Q32"), bid(1, Suit::Diamonds));
    assert_eq!(opens("K32.A2.Q5432.Q32"), P);
    // Four diamonds open 1♦ only unbalanced: 4=4=4=1, (31)45, 2=2=4=5.
    assert_eq!(opens("KQ32.KJ32.KJ32.2"), bid(1, Suit::Diamonds));
    assert_eq!(opens("K32.2.KJ32.AQ932"), bid(1, Suit::Diamonds));
    assert_eq!(opens("A2.32.KJ32.KQ432"), bid(1, Suit::Diamonds));
    // 4=4=1=4 and (40)45 open 1♣; so does 2=2=4=5 from 18.
    assert_eq!(opens("KQ32.KJ32.2.KJ32"), bid(1, Suit::Clubs));
    assert_eq!(opens("KQ32..KJ32.AJ932"), bid(1, Suit::Clubs));
    assert_eq!(opens("A2.K2.AKJ2.KQ432"), bid(1, Suit::Clubs));
    // A five-card major is 11–16: a balanced 11 opens, a 17+ hand opens 1♣.
    assert_eq!(opens("AKQ32.K32.432.32"), bid(1, Suit::Spades));
    assert_eq!(opens("KQ432.A2.432.Q32"), bid(1, Suit::Spades));
    assert_eq!(opens("AKQ32.AK2.K32.32"), bid(1, Suit::Clubs));
    // There is no strong 2♣ and no 2NT: the big hands open 1♣.
    assert_eq!(opens("AKQ2.KQ3.KJ3.432"), bid(1, Suit::Clubs));
    assert_eq!(opens("AKQ2.KQ3.KQ3.A32"), bid(1, Suit::Clubs));
    // 1♦ runs to 20; above it the hand is a 1♣.
    assert_eq!(opens("A2.K2.AKJ432.K32"), bid(1, Suit::Diamonds));
    assert_eq!(opens("AK.K2.AKJ432.KQ2"), bid(1, Suit::Clubs));
}

/// Third and fourth seat open a major on nine; a light 1♥ holds three spades.
#[test]
fn late_seat_majors() {
    let third = |hand| calls(&[P, P], hand);
    assert_eq!(calls(&[], "KQ432.K32.Q32.32"), P);
    assert_eq!(third("KQ432.K32.Q32.32"), bid(1, Suit::Spades));
    assert_eq!(third("Q32.KQ432.K32.32"), bid(1, Suit::Hearts));
    assert_eq!(third("32.KQ432.K32.Q32"), P);
}

/// Responder's first call over the Polish 1♣.
#[test]
fn responses() {
    let responds = |hand| calls(&[bid(1, Suit::Clubs), P], hand);
    // The negative: a bust, a no-major 8, a balanced 17.
    assert_eq!(responds("432.432.432.5432"), bid(1, Suit::Diamonds));
    assert_eq!(responds("K32.2.Q432.K5432"), bid(1, Suit::Diamonds));
    assert_eq!(responds("KQ2.A32.KQ32.K32"), bid(1, Suit::Diamonds));
    // Positive majors from 7, hearts first on 4-4, spades first on five.
    assert_eq!(responds("K32.KJ32.432.432"), bid(1, Suit::Hearts));
    assert_eq!(responds("KJ32.KJ32.32.432"), bid(1, Suit::Hearts));
    assert_eq!(responds("AQ432.KJ32.K2.32"), bid(1, Suit::Spades));
    // Reverse Flannery: five spades and four hearts, 6–9.
    assert_eq!(responds("KJ432.K432.32.32"), bid(2, Suit::Hearts));
    // The notrump ladder.
    assert_eq!(responds("K32.Q32.K432.J32"), bid(1, Strain::Notrump));
    assert_eq!(responds("K32.Q32.KJ32.Q32"), bid(2, Strain::Notrump));
    // The swapped game forces: 2♣ shows diamonds, 2♦ clubs.
    assert_eq!(responds("K32.A2.KQ432.432"), bid(2, Suit::Clubs));
    assert_eq!(responds("K32.A2.432.KQ432"), bid(2, Suit::Diamonds));
    // The balanced game forces.
    assert_eq!(responds("K32.A32.KQ32.432"), bid(3, Strain::Notrump));
    assert_eq!(responds("K32.A2.KQ32.5432"), bid(3, Suit::Diamonds));
    // The invitations and the preempt.
    assert_eq!(responds("32.K2.KQJ432.432"), bid(2, Suit::Spades));
    assert_eq!(responds("32.K2.432.KQJ432"), bid(3, Suit::Clubs));
    assert_eq!(responds("KJ65432.2.432.32"), bid(3, Suit::Spades));
}

/// A passed hand's 2♣ is an invitation with diamonds, not a game force.
#[test]
fn passed_hand_responses() {
    let auction = [P, P, bid(1, Suit::Clubs), P];
    assert_eq!(calls(&auction, "K32.A2.Q5432.432"), bid(2, Suit::Clubs));
    assert_eq!(calls(&auction, "432.432.432.5432"), bid(1, Suit::Diamonds));
}

/// Opener's rebid over the negative 1♦.
#[test]
fn rebids_over_the_negative() {
    let rebids = |hand| calls(&[bid(1, Suit::Clubs), P, bid(1, Suit::Diamonds), P], hand);
    // The weak notrump: a four-card major, else a three-card one.
    assert_eq!(rebids("K32.KJ32.K32.Q32"), bid(1, Suit::Hearts));
    assert_eq!(rebids("K32.Q32.KJ32.K32"), bid(1, Suit::Hearts));
    assert_eq!(rebids("K32.Q2.KJ32.K432"), bid(1, Suit::Spades));
    // Clubs, 18–20 balanced, 18–20 with six clubs, 21+.
    assert_eq!(rebids("K2.Q2.432.AKJ432"), bid(2, Suit::Clubs));
    assert_eq!(rebids("AKQ2.KQ3.KJ3.432"), bid(1, Strain::Notrump));
    assert_eq!(rebids("A2.K2.K32.AKJ432"), bid(2, Strain::Notrump));
    assert_eq!(rebids("AKQ2.KQ3.KQ3.A32"), bid(2, Suit::Diamonds));
    // A strong five-card major rebids it at the one level up to 19.
    assert_eq!(rebids("AKQ32.A432.K32.2"), bid(1, Suit::Spades));
}

/// Responder never passes the artificial 21+ rebid on a bust.
#[test]
fn a_bust_signs_off_over_the_strong_rebid() {
    let auction = [
        bid(1, Suit::Clubs),
        P,
        bid(1, Suit::Diamonds),
        P,
        bid(2, Suit::Diamonds),
        P,
    ];
    assert_eq!(calls(&auction, "432.5432.432.432"), bid(2, Suit::Hearts));
    assert_eq!(calls(&auction, "432.432.432.5432"), bid(2, Strain::Notrump));
    assert_ne!(calls(&auction, "K32.432.Q432.432"), P);
}

/// Game values over the 21+ rebid bid as over a natural 2NT.
#[test]
fn the_strong_rebid_is_a_two_notrump_opening() {
    let auction = [
        bid(1, Suit::Clubs),
        P,
        bid(1, Suit::Diamonds),
        P,
        bid(2, Suit::Diamonds),
        P,
    ];
    assert_eq!(calls(&auction, "432.KJ32.Q432.32"), bid(3, Suit::Clubs));
    assert_eq!(calls(&auction, "KJ432.32.Q32.432"), bid(3, Suit::Hearts));
    assert_eq!(calls(&auction, "432.432.K432.Q32"), bid(3, Strain::Notrump));
    let stayman = [auction.as_slice(), &[bid(3, Suit::Clubs), P]].concat();
    assert_eq!(calls(&stayman, "AKQ2.KQ3.KQ3.A32"), bid(3, Suit::Spades));
}

/// Over the 20–23 major, responder places the game and opener respects it.
#[test]
fn the_strong_major_is_raised_to_game_and_no_further() {
    let auction = [
        bid(1, Suit::Clubs),
        P,
        bid(1, Suit::Diamonds),
        P,
        bid(2, Suit::Spades),
        P,
    ];
    assert_eq!(calls(&auction, "K32.432.Q432.432"), bid(4, Suit::Spades));
    assert_eq!(calls(&auction, "32.Q432.K432.Q32"), bid(3, Strain::Notrump));
    assert_eq!(calls(&auction, "432.432.432.5432"), P);
    let raised = [auction.as_slice(), &[bid(4, Suit::Spades), P]].concat();
    assert_eq!(calls(&raised, "AKQ32.AK2.KQ2.32"), P);
}

/// The balanced game force is weighed for slam; a passed hand's 3♦ is a
/// natural invitation, which a minimum passes.
#[test]
fn the_balanced_force_and_the_passed_hand_invitation() {
    let force = [bid(1, Suit::Clubs), P, bid(3, Suit::Diamonds), P];
    assert_eq!(calls(&force, "K32.KJ32.K32.Q32"), bid(3, Strain::Notrump));
    assert_eq!(calls(&force, "AKQ2.KQ3.KJ3.432"), bid(4, Strain::Notrump));
    assert_eq!(calls(&force, "AKQ2.KQ3.KQ3.A32"), bid(6, Strain::Notrump));
    let invited = [P, P, bid(1, Suit::Clubs), P, bid(3, Suit::Diamonds), P];
    assert_eq!(calls(&invited, "K32.KJ32.K32.Q32"), P);
    assert_eq!(calls(&invited, "KQ2.KJ32.K32.Q32"), bid(3, Strain::Notrump));
}

/// Checkback Gladiator over 1♣ - 1♥ - 1♠: the puppet, its completion, and
/// the hand types behind it.
#[test]
fn the_checkback_puppet() {
    let forcing = [
        bid(1, Suit::Clubs),
        P,
        bid(1, Suit::Hearts),
        P,
        bid(1, Suit::Spades),
        P,
    ];
    // A minimum with four spades and the balanced invitation go through 2♣;
    // a flat minimum bids 1NT.
    assert_eq!(calls(&forcing, "KJ32.KJ32.432.32"), bid(2, Suit::Clubs));
    assert_eq!(calls(&forcing, "K32.KJ32.K32.Q32"), bid(2, Suit::Clubs));
    assert_eq!(calls(&forcing, "Q32.KJ32.K32.432"), bid(1, Strain::Notrump));

    let puppet = [forcing.as_slice(), &[bid(2, Suit::Clubs), P]].concat();
    assert_eq!(calls(&puppet, "KQ32.K2.K432.Q32"), bid(2, Suit::Diamonds));
    assert_eq!(calls(&puppet, "AKQ32.K2.AK32.32"), bid(2, Suit::Spades));

    let completed = [puppet.as_slice(), &[bid(2, Suit::Diamonds), P]].concat();
    assert_eq!(calls(&completed, "KJ32.KJ32.432.32"), bid(2, Suit::Spades));
    assert_eq!(
        calls(&completed, "K32.KJ32.K32.Q32"),
        bid(2, Strain::Notrump)
    );
    let invited = [completed.as_slice(), &[bid(2, Strain::Notrump), P]].concat();
    assert_eq!(calls(&invited, "KQ32.K2.K432.Q32"), P);
    assert_eq!(calls(&invited, "KQ32.K2.KJ32.Q32"), bid(3, Strain::Notrump));

    // Over 1NT a game force shows its fifth spade, and responder picks.
    let notrump = [forcing.as_slice(), &[bid(1, Strain::Notrump), P]].concat();
    assert_eq!(calls(&notrump, "AKQ32.K2.AK32.32"), bid(2, Suit::Spades));
    assert_eq!(calls(&notrump, "KQ32.K2.K432.Q32"), P);
    let shown = [notrump.as_slice(), &[bid(2, Suit::Spades), P]].concat();
    assert_eq!(calls(&shown, "J32.KJ32.K32.432"), bid(4, Suit::Spades));
}

/// Opener's rebid over a positive 1♥, and the Odwrotka steps.
#[test]
fn rebids_over_a_positive() {
    let positive = [bid(1, Suit::Clubs), P, bid(1, Suit::Hearts), P];
    let rebids = |hand| calls(&positive, hand);
    assert_eq!(rebids("K32.KJ32.K32.Q32"), bid(2, Suit::Hearts));
    assert_eq!(rebids("KJ32.K32.K32.Q32"), bid(1, Suit::Spades));
    assert_eq!(rebids("K32.Q32.KJ32.K32"), bid(1, Strain::Notrump));
    assert_eq!(rebids("K2.Q2.432.AKJ432"), bid(2, Suit::Clubs));
    assert_eq!(rebids("2.KQ32.K32.AQJ32"), bid(3, Suit::Hearts));
    // 17+ with three hearts: Odwrotka.
    assert_eq!(rebids("AKQ2.KQ3.KJ3.432"), bid(2, Suit::Diamonds));

    let odwrotka = [positive.as_slice(), &[bid(2, Suit::Diamonds), P]].concat();
    assert_eq!(calls(&odwrotka, "K32.KJ32.432.432"), bid(2, Suit::Spades));
    assert_eq!(
        calls(&odwrotka, "K32.KJ432.A32.32"),
        bid(2, Strain::Notrump)
    );
    // Opener places the game: 3NT facing four hearts on three, 4♥ facing five.
    let four = [odwrotka.as_slice(), &[bid(2, Suit::Spades), P]].concat();
    assert_eq!(calls(&four, "AKQ2.KQ3.KJ3.432"), bid(3, Strain::Notrump));
    let five = [odwrotka.as_slice(), &[bid(2, Strain::Notrump), P]].concat();
    assert_eq!(calls(&five, "AKQ2.KQ3.KJ3.432"), bid(4, Suit::Hearts));
}

/// The swapped minor force is carried to game, and Reverse Flannery is
/// passed on four hearts.
#[test]
fn swapped_minor_and_flannery() {
    let force = [bid(1, Suit::Clubs), P, bid(2, Suit::Clubs), P];
    assert_eq!(calls(&force, "K32.KJ32.K32.Q32"), bid(2, Suit::Diamonds));
    let raised = [force.as_slice(), &[bid(2, Suit::Diamonds), P]].concat();
    assert_eq!(calls(&raised, "K32.A2.KQ432.432"), bid(3, Strain::Notrump));

    let flannery = [bid(1, Suit::Clubs), P, bid(2, Suit::Hearts), P];
    assert_eq!(calls(&flannery, "K32.KJ32.K32.Q32"), P);
    assert_eq!(calls(&flannery, "K32.Q2.KJ32.K432"), bid(2, Suit::Spades));
}

/// The opening reads as the Polish 1♣: no club length, and a 2♣ rebid shows
/// the suit.
#[test]
fn the_polish_club_reads_as_artificial() {
    let partnership = strawberry(&Agreements::default()).bind();
    let clubs = |auction: &[Call]| {
        partnership
            .infer(RelativeVulnerability::NONE, auction)
            .get(Relative::Partner)
            .length(Suit::Clubs)
            .min
    };
    assert!(clubs(&[bid(1, Suit::Clubs), P]) < 3);
    assert!(
        clubs(&[
            bid(1, Suit::Clubs),
            P,
            bid(1, Suit::Diamonds),
            P,
            bid(2, Suit::Clubs),
            P
        ]) >= 5
    );
}

/// A borrowed chapter answers exactly as the house book does, and a board
/// they open is the house's from the first call.
#[test]
fn borrowed_chapters_and_the_defense_are_the_house_book() {
    let house = american::american(&Agreements::default());
    let ours = strawberry(&Agreements::default());
    let hands = [
        "K32.2.KJ432.Q432",
        "KJ32.Q32.432.K32",
        "AQ432.KJ32.K2.32",
        "432.432.432.5432",
    ];
    let auctions: [&[Call]; 6] = [
        // The 12–14 notrump rebid is the house's too: its XYZ answers it.
        &[
            bid(1, Suit::Clubs),
            P,
            bid(1, Suit::Spades),
            P,
            bid(1, Strain::Notrump),
            P,
        ],
        &[bid(1, Suit::Hearts), P],
        &[bid(1, Suit::Diamonds), P],
        &[bid(1, Strain::Notrump), P],
        &[bid(1, Suit::Spades), bid(2, Suit::Clubs)],
        &[bid(1, Suit::Hearts)],
    ];
    for auction in auctions {
        for hand in hands {
            assert_eq!(
                best(&ours, auction, hand),
                best(&house, auction, hand),
                "{auction:?} {hand}"
            );
        }
    }
}

/// Grafting by opening carries a whole chapter only if the house book hangs
/// nothing above the opening call.
#[test]
fn the_house_book_keys_every_fallback_under_an_opening() {
    let house = american::book(&Agreements::default());
    for trie in [&house.constructive.0, &house.competitive.0] {
        for (key, ..) in trie.fallbacks() {
            assert!(
                key.iter().any(|call| *call != P),
                "a fallback at {key:?} would not follow a graft"
            );
        }
    }
}
