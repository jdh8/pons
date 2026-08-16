use super::*;
use crate::bidding::agreements::Agreements;
use crate::bidding::constraint::point_count;
use crate::bidding::context::Context;
use crate::bidding::inference::tests::{bid, read, read_booked, read_booked_with, read_with};
use crate::bidding::inference::{Envelope, EnvelopeUnion, Inferences, Range, Relative};
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Hand, Strain, Suit};
use proptest::prelude::*;

/// Pins the skew bound `support_band_to_points` is derived from: at every
/// fit-known trump (three-plus cards), `point_count` lies within the
/// image of the hand's own support count — so a support band's image
/// always contains the legacy count of every hand the band admits.
#[test]
fn support_band_points_image_is_sound() {
    use rand::SeedableRng as _;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0x5B);
    let hands = crate::bidding::verify::random_hands(&mut rng)
        .take(4096)
        // Extremes the random pool cannot deal: two side voids attain
        // the +5 skew; working doubletons alone attain the −1 side.
        .chain(
            ["432.AKQJT98765..", "..432.AKQJT98765", "AQJT9.KQJT.A2.K2"]
                .map(|text| text.parse::<Hand>().unwrap_or_else(|_| unreachable!())),
        );
    for hand in hands {
        for trump in Suit::ASC {
            if hand[trump].len() < 3 {
                continue;
            }
            let support =
                crate::bidding::constraint::support_point_count_in(hand, trump).min(POINTS_CAP);
            let image = support_band_to_points(Range::new(support, support));
            let points = point_count(hand);
            assert!(
                image.contains(points),
                "{hand}: trump {trump}, support {support}, points {points}"
            );
        }
    }
}

/// [`ReadingProfile::blind_opponents`] blanks LHO and RHO and *only* those: the
/// deviation panel's blind arm must leave partner and our own reading
/// intact, or it stops measuring what reading *their* calls is worth.
#[test]
fn blind_opponent_reading_spares_our_side() {
    // `1♦ (1♥) 1♠ (2♥)`: all four seats have shown something, so blanking
    // LHO and RHO while retaining our side is visible.
    let auction = [
        bid(1, Strain::Diamonds),
        bid(1, Strain::Hearts),
        bid(1, Strain::Spades),
        bid(2, Strain::Hearts),
    ];
    let seen = read(&auction);
    let mut agreements = Agreements::default();
    agreements.decision.reading.blind_opponents = true;
    let blind = read_with(&agreements, &auction);

    for who in [Relative::Lho, Relative::Rho] {
        assert_eq!(*blind.get(who), Envelope::unknown(), "{who:?} not blanked");
        assert_eq!(blind.announced_union(who), &EnvelopeUnion::unknown());
    }
    assert_ne!(
        *seen.get(Relative::Rho),
        Envelope::unknown(),
        "the fixture must read RHO's 1♥, else the test proves nothing"
    );
    for who in [Relative::Me, Relative::Partner] {
        assert_eq!(*blind.get(who), *seen.get(who), "{who:?} moved");
        assert_eq!(blind.announced_union(who), seen.announced_union(who));
    }
    // Knob off is byte-identical to never having set it.
    let after = read(&auction);
    for who in [
        Relative::Me,
        Relative::Lho,
        Relative::Partner,
        Relative::Rho,
    ] {
        assert_eq!(*after.get(who), *seen.get(who), "{who:?} moved after reset");
        assert_eq!(after.announced_union(who), seen.announced_union(who));
    }
}

#[test]
fn opening_shapes() {
    // `(1♥)`: the opener sits to our right (the call just before ours).
    let one_heart = read(&[bid(1, Strain::Hearts)]);
    assert_eq!(one_heart.rho().length(Suit::Hearts), Range::new(5, 13));
    // `points(12..)` is the Rule of 20, which opens sound 10-11 HCP counts,
    // so the floor is 10.
    assert_eq!(one_heart.rho().strength.points, Range::new(10, 21));

    // A strong notrump is balanced-or-6322-minor (the shipped Wide6322): a
    // major stays 2–5 (a balanced 5332 major), a minor widens to 2–6 (the
    // 6322's six-card minor); an artificial 2♣ says only "strong".
    let one_nt = read(&[bid(1, Strain::Notrump)]);
    assert_eq!(one_nt.rho().length(Suit::Spades), Range::new(2, 5));
    assert_eq!(one_nt.rho().length(Suit::Diamonds), Range::new(2, 6));
    // Plain HCP 15–17: no downgrade on the shipped floored scale, a
    // semi-balanced 5422/6322 reads one over → 15–18.
    assert_eq!(one_nt.rho().strength.points, Range::new(15, 18));

    let two_clubs = read(&[bid(2, Strain::Clubs)]);
    assert_eq!(two_clubs.rho().length(Suit::Spades), Range::FULL_LENGTH);
    assert_eq!(two_clubs.rho().strength.points, Range::new(20, 37));

    // Weak two: exactly six; three-level preempt: seven-plus.
    let weak_two = read(&[bid(2, Strain::Spades)]);
    assert_eq!(weak_two.rho().length(Suit::Spades), Range::new(6, 6));
    assert_eq!(weak_two.rho().strength.points, Range::new(5, 10));
    let preempt = read(&[bid(3, Strain::Diamonds)]);
    assert_eq!(preempt.rho().length(Suit::Diamonds), Range::new(7, 13));

    // A 1♣ opening denies a five-card major.
    let one_club = read(&[bid(1, Strain::Clubs)]);
    assert_eq!(one_club.rho().length(Suit::Clubs), Range::new(3, 13));
    assert_eq!(one_club.rho().length(Suit::Hearts), Range::new(0, 4));
}

/// A two-over-one denies four-card support, and the reading now says so.
///
/// `Flip` had no projection at all, so `!support(4..)` — a plain box, "at
/// most three of partner's suit" — read as ⊤ and responder's spades came
/// back `0..=13` after `1♠ - 2♣`.  The strength half of the same rule is
/// still blind (`Or::project` unions `hcp(13..)` away; see
/// `docs/ai-bidder/sampled-projection.md`), which is why only the length
/// axis is asserted here.
#[test]
fn two_over_one_denies_four_card_support() {
    let auction = [bid(1, Strain::Spades), Call::Pass, bid(2, Strain::Clubs)];
    let read = read_booked(&auction);
    let responder = read.rho();
    assert_eq!(responder.length(Suit::Spades), Range::new(0, 3));
    assert_eq!(responder.length(Suit::Clubs), Range::new(4, 13));
}

#[test]
fn pass_reading_caps_the_no_open_pass() {
    let p = Call::Pass;
    let mut agreements = Agreements::default();
    // Knob off — the pre-ship identity: a pass reads nothing.
    agreements.decision.reading.pass = false;
    assert_eq!(
        read_booked_with(&agreements, &[p, p])
            .partner()
            .strength
            .points,
        Range::FULL_POINTS
    );

    agreements.decision.reading.pass = true;
    agreements.decision.reading.table_alerts = false;
    // Partner's no-open pass reads the opening table's own gate,
    // `points(..12)`; an opponent's pass stays unread until table-wide
    // disclosure is on too.
    let own = read_booked_with(&agreements, &[p, p]);
    assert_eq!(own.partner().strength.points, Range::new(0, 11));
    assert_eq!(own.rho().strength.points, Range::FULL_POINTS);
    agreements.decision.reading.table_alerts = true;
    assert_eq!(
        read_booked_with(&agreements, &[p]).rho().strength.points,
        Range::new(0, 11)
    );
    // A capped passer leaves the opener's own band alone.
    let opened = read_booked_with(&agreements, &[p, bid(1, Strain::Hearts)]);
    assert_eq!(opened.partner().strength.points, Range::new(0, 11));
    assert_eq!(opened.rho().strength.points, Range::new(10, 21));
}

#[test]
fn pass_reading_caps_the_failed_compete() {
    let auction = [bid(1, Strain::Hearts), Call::Pass, Call::Pass];
    let mut agreements = Agreements::default();
    agreements.decision.reading.pass = false;
    assert_eq!(
        read_booked_with(&agreements, &auction)
            .partner()
            .strength
            .points,
        Range::FULL_POINTS
    );

    agreements.decision.reading.pass = true;
    agreements.decision.reading.table_alerts = false;
    // Partner's direct-seat pass: the authored complement of the strong
    // tier ("strong hands double first regardless") — at most 17 raw HCP,
    // 19 on the point-count scale (17 + max upgrade 2).  Their responder's
    // pass stays unread until table-wide disclosure is on.
    let own = read_booked_with(&agreements, &auction);
    assert_eq!(own.partner().strength.points, Range::new(0, 19));
    assert_eq!(own.rho().strength.points, Range::FULL_POINTS);
    agreements.decision.reading.table_alerts = true;
    // Their responder's pass: the response table's `hcp(..6)` gate — at
    // most 5 raw HCP, 7 on the point-count scale (5 + max upgrade 2).
    assert_eq!(
        read_booked_with(&agreements, &auction)
            .rho()
            .strength
            .points,
        Range::new(0, 7)
    );
}

#[test]
fn pass_reading_caps_the_silent_responder() {
    let mut agreements = Agreements::default();
    agreements.decision.reading.pass = true;
    // Our 1♥, silent partner: the response table's `hcp(..6)` gate —
    // at most 5 raw HCP, 7 on the point-count scale (5 + max upgrade 2).
    let caps = read_booked_with(
        &agreements,
        &[bid(1, Strain::Hearts), Call::Pass, Call::Pass, Call::Pass],
    );
    assert_eq!(caps.partner().strength.points, Range::new(0, 7));
}

#[test]
fn pass_reading_caps_the_notrump_signoff() {
    let mut agreements = Agreements::default();
    agreements.decision.reading.pass = true;
    // Pass of partner's 1NT: the authored union of the weak arm and the
    // flat-eight arm — at most 9 points, no six-card major.  The flat-eight
    // arm's own 8 HCP would slack to 10 on the point-count scale, but its
    // lengths force balanced and balanced hands never upgrade, so C2
    // (`upgrade_closure`, default-on since 2026-08-16) hands the band back the
    // point it never had; the 9 is the weaker arm's.
    let nt = read_booked_with(
        &agreements,
        &[bid(1, Strain::Notrump), Call::Pass, Call::Pass, Call::Pass],
    );
    assert_eq!(nt.partner().strength.points, Range::new(0, 9));
    assert!(nt.partner().length(Suit::Hearts).max <= 5);
    assert!(nt.partner().length(Suit::Spades).max <= 5);
}

#[test]
fn pass_reading_skips_trap_and_trivial_passes() {
    let mut agreements = Agreements::default();
    agreements.decision.reading.pass = true;
    agreements.decision.reading.table_alerts = true;
    // The advance of a takeout double authors genuine strong sits (the
    // penalty conversion), so its pass-gate union is trivial: nothing is
    // claimed about the advancer even with every reading knob on.
    let trap = read_booked_with(
        &agreements,
        &[bid(1, Strain::Hearts), Call::Double, Call::Pass, Call::Pass],
    );
    assert_eq!(trap.rho().strength.points, Range::FULL_POINTS);
}

/// [`ReadingProfile::pass_exclusion`] caps the direct-seat pass
/// over their weak two off the *declined* shape-free double tier
/// (`points(17..)`, weight 1.2) — the catch-all `hcp(0..)` Pass gate says
/// nothing on its own, which is why this key read 100% blind in the census.
/// Shaped siblings (the overcalls, the 2NT arm) complement to unions or ⊤
/// and are skipped by the single-box filter, so the lengths stay ⊤.
#[test]
fn pass_exclusion_caps_the_weak_two_defender() {
    let auction = [bid(2, Strain::Spades), Call::Pass, Call::Pass];
    let mut agreements = Agreements::default();
    agreements.decision.reading.pass = true;
    agreements.decision.reading.table_alerts = false;

    // Knob off — today's identity: the catch-all gate reads nothing.
    agreements.decision.reading.pass_exclusion = false;
    let off = read_booked_with(&agreements, &auction);
    assert_eq!(off.partner().strength.points, Range::FULL_POINTS);

    // Knob on — declining the 17+ double caps the passer.
    agreements.decision.reading.pass_exclusion = true;
    let on = read_booked_with(&agreements, &auction);
    assert_eq!(on.partner().strength.points, Range::new(0, 16));
    // The overcall complements are multi-box and skipped: no length claim.
    assert_eq!(on.partner().length(Suit::Hearts), Range::new(0, 13));

    // Off again is byte-identical to never having been on.
    agreements.decision.reading.pass_exclusion = false;
    assert_eq!(
        read_booked_with(&agreements, &auction).partner(),
        off.partner()
    );
}

#[test]
fn opener_extras_ladder_reads_extras() {
    let mut agreements = Agreements::default();
    agreements.decision.reading.opener_extras_ladder = true;
    let d = bid(1, Strain::Diamonds);
    let s = bid(1, Strain::Spades);
    let p = Call::Pass;
    // Opener (partner of the hero to act) after 1♦ - 1♠ - X.
    // Jump-rebid 3♦: a self-sufficient six-plus diamonds, 16+.
    let jr = read_with(&agreements, &[d, p, s, p, bid(3, Strain::Diamonds), p]);
    assert!(jr.partner().length(Suit::Diamonds).min >= 6);
    assert!(jr.partner().strength.points.min >= 16);
    // Reverse 2♥: five-plus diamonds, four-plus hearts, 17+.
    let rev = read_with(&agreements, &[d, p, s, p, bid(2, Strain::Hearts), p]);
    assert!(rev.partner().length(Suit::Diamonds).min >= 5);
    assert!(rev.partner().length(Suit::Hearts).min >= 4);
    assert!(rev.partner().strength.points.min >= 17);
    // Jump-shift 3♣: five-plus diamonds, 18+, and clubs read as the strong
    // 4+ second suit — NOT the weak-jump six (the phantom-suit fix).
    let js = read_with(&agreements, &[d, p, s, p, bid(3, Strain::Clubs), p]);
    assert!(js.partner().length(Suit::Diamonds).min >= 5);
    assert!(js.partner().strength.points.min >= 18);
    assert_eq!(
        js.partner().length(Suit::Clubs),
        Range::at_least(4, LENGTH_CAP)
    );
}

#[test]
fn opener_major_jump_rebid_reads_extras() {
    let mut agreements = Agreements::default();
    agreements.decision.reading.opener_major_jump_rebid = true;
    let h = bid(1, Strain::Hearts);
    let s = bid(1, Strain::Spades);
    let p = Call::Pass;
    // Opener after 1♥ - 1♠ - 3♥: jump-rebid of a six-plus major, 16+.
    let jr = read_with(&agreements, &[h, p, s, p, bid(3, Strain::Hearts), p]);
    assert!(jr.partner().length(Suit::Hearts).min >= 6);
    assert!(jr.partner().strength.points.min >= 16);
}

/// The M6.4 deterministic rule on its canonical auctions: a
/// four-plus-level new suit is a control bid iff the bidder *bypassed*
/// it (available below their first-shown suit at the same level);
/// everything else stays to play — suppressed, nothing floored.
#[test]
fn high_bid_control_vs_natural() {
    // Pin the historic hearts-first reading (knob off): these
    // minor-response verdicts are the knob-off ones — the longer-major
    // default is covered by `high_bid_under_longer_major_response`, and the
    // 1NT-transfer sub-cases below are knob-independent.
    let mut agreements = Agreements::default();
    agreements.decision.reading.longer_major_response = false;
    // 1♦ - 1♠ - 2♦ - 4♥: responder bid spades first, so hearts cannot be their
    // longest — a control bid agreeing diamonds.  Hearts stays unfloored;
    // diamond support and slam-try values are recorded instead.
    let control = read_with(
        &agreements,
        &[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ],
    );
    assert_eq!(control.partner().length(Suit::Hearts).min, 0);
    assert!(control.partner().length(Suit::Diamonds).min >= 3);
    assert!(control.partner().strength.points.min >= 13);

    // 1♦ - 1♠ - 2♦ - 4♠: rebidding one's own suit is natural — six-plus spades.
    let rebid = read_with(
        &agreements,
        &[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ],
    );
    assert!(rebid.partner().length(Suit::Spades).min >= 6);

    // 1♦ - 4♥: the bidder has shown nothing, so hearts can be their
    // longest — to play, no control machinery (and no phantom floor:
    // the honest envelope of an unread jump stays wide).
    let preempt = read_with(
        &agreements,
        &[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ],
    );
    assert!(preempt.control_bid().is_none());

    // 1♣ - 1♥ - 2♣ - 4♠: spades sit *above* the first-shown hearts, so they were
    // never denied — this system's response and transfer styles bid the
    // cheaper suit first holding a longer higher one (the first M6.4 A/B
    // bled six IMPs a fired board pulling these to the "agreed" minor).
    // To play, not a control bid.
    let above = read_with(
        &agreements,
        &[
            bid(1, Strain::Clubs),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ],
    );
    assert!(above.control_bid().is_none());

    // 1NT - 2♦ - 2♥ - 4♠: same shape through a transfer (the overlay attributes
    // the hearts to the bidder) — spades were never denied, so to play.
    let post_transfer = read_booked_with(
        &agreements,
        &[
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ],
    );
    assert!(post_transfer.control_bid().is_none());
    assert!(post_transfer.partner().length(Suit::Hearts).min >= 5);

    // 1NT - 2♥ - 2♠ - 4♥ — the mirror: hearts sit *below* the transferred
    // spades and the cheaper heart transfer was bypassed, so 4♥ cannot be
    // long — a control bid agreeing spades, promising a sixth.
    let mirror = read_booked_with(
        &agreements,
        &[
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ],
    );
    assert_eq!(mirror.partner().length(Suit::Hearts).min, 0);
    assert!(mirror.partner().length(Suit::Spades).min >= 6);
}

/// The longer-major response discipline swaps the M6.4 verdicts on the
/// two major-response auctions: a 1♥ response denies longer spades (so
/// the spade jump becomes a control bid), and a 1♠ response may conceal
/// equal-length five-plus hearts (so the heart jump reads to play).
#[test]
fn high_bid_under_longer_major_response() {
    let mut agreements = Agreements::default();

    // 1♣ - 1♥ - 2♣ - 4♠, discipline on: 1♥ denied longer spades, so 4♠ is a
    // bypass — a control bid agreeing clubs, spades left unfloored.
    agreements.decision.reading.longer_major_response = true;
    let control = read_with(
        &agreements,
        &[
            bid(1, Strain::Clubs),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ],
    );
    // The mirror 1♣ - 1♠ - 2♣ - 4♥: a 1♠ response no longer proves short
    // hearts (5-5 responds 1♠), so the heart jump reads to play.
    let to_play = read_with(
        &agreements,
        &[
            bid(1, Strain::Clubs),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ],
    );
    assert_eq!(control.partner().length(Suit::Spades).min, 0);
    assert!(control.partner().length(Suit::Clubs).min >= 3);
    assert!(control.partner().strength.points.min >= 13);
    assert!(to_play.control_bid().is_none());

    // Knob off (the historic hearts-first opt-in): the original verdicts
    // stand — the spade jump above the 1♥ response is to play.
    agreements.decision.reading.longer_major_response = false;
    let above = read_with(
        &agreements,
        &[
            bid(1, Strain::Clubs),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ],
    );
    assert!(above.control_bid().is_none());
}

#[test]
fn gambling_3nt_over_double_reads_unbalanced() {
    // `1NT (X) 3NT -`: opener reads partner's gambling 3NT.  The floor alerts the
    // call as the long-minor gamble, so the natural balanced-3NT reading is
    // suppressed and a six-card minor stays within range — the search sampler must
    // be free to deal responder its running suit, not pin it to a flat hand.
    let mut agreements = Agreements::default();
    agreements.decision.instinct.gambling_3nt_over_double = true;
    let read = read_booked_with(
        &agreements,
        &[
            bid(1, Strain::Notrump),
            Call::Double,
            bid(3, Strain::Notrump),
            Call::Pass,
        ],
    );
    assert!(read.partner().length(Suit::Clubs).contains(6));
    assert!(read.partner().length(Suit::Diamonds).contains(6));
}

#[test]
fn artificial_witness_covers_doubles() {
    // A projection that floors a suit it would not name — the witness a transfer
    // or two-suiter trips (5+ hearts).
    let mut floors_hearts = Envelope::unknown();
    floors_hearts.narrow_length(Suit::Hearts, Range::at_least(5, LENGTH_CAP));

    // A *bid* that did not name hearts is artificial (Jacoby 2♦ → 5+♥); a bid
    // naming its own suit is natural (1♥ → 5+♥).
    assert!(artificial(&floors_hearts, bid(2, Strain::Diamonds), None));
    assert!(!artificial(&floors_hearts, bid(1, Strain::Hearts), None));

    // A pass redirects from nothing → never artificial, even flooring a suit.
    assert!(!artificial(
        &floors_hearts,
        Call::Pass,
        Some(Strain::Spades)
    ));

    // A double/redouble "names" the *doubled strain*.  Doubling spades while the
    // projection floors hearts is takeout — it points partner at hearts → artificial;
    // doubling hearts while flooring hearts defends the doubled strain → natural
    // (penalty).  A redouble inherits the same doubled strain.
    assert!(artificial(
        &floors_hearts,
        Call::Double,
        Some(Strain::Spades)
    ));
    assert!(!artificial(
        &floors_hearts,
        Call::Double,
        Some(Strain::Hearts)
    ));
    assert!(artificial(
        &floors_hearts,
        Call::Redouble,
        Some(Strain::Spades)
    ));
    assert!(!artificial(
        &floors_hearts,
        Call::Redouble,
        Some(Strain::Hearts)
    ));

    // A double of notrump defends no suit, so any floored side suit is takeout.
    assert!(artificial(
        &floors_hearts,
        Call::Double,
        Some(Strain::Notrump)
    ));
}

#[test]
fn narrowed_points_intersects_one_player() {
    // 1NT shows 15-18; narrow the opener (here our RHO) to the upper half.
    let inf = read(&[bid(1, Strain::Notrump)]);
    assert_eq!(inf.rho().strength.points, Range::new(15, 18));

    let upper = inf.narrowed_points(Relative::Rho, Range::new(17, 18));
    assert_eq!(
        upper.rho().strength.points,
        Range::new(17, 18),
        "narrowed to the half"
    );
    assert_eq!(
        inf.rho().strength.points,
        Range::new(15, 18),
        "original unchanged"
    );
    // Shape and the other players are untouched.
    assert_eq!(
        upper.rho().length(Suit::Spades),
        inf.rho().length(Suit::Spades)
    );
    assert_eq!(
        upper.partner().strength.points,
        inf.partner().strength.points
    );

    // Intersection, not replacement: a wider request cannot widen what was shown.
    let clamped = inf.narrowed_points(Relative::Rho, Range::new(0, POINTS_CAP));
    assert_eq!(clamped.rho().strength.points, Range::new(15, 18));
}

#[test]
fn third_seat_openings_are_light() {
    // `- - (1♠)`: a third-seat opponent may open on as few as nine points.
    let third = read(&[Call::Pass, Call::Pass, bid(1, Strain::Spades)]);
    assert_eq!(third.rho().strength.points, Range::new(9, 21));
}

#[test]
fn responses_narrow_partner_and_opener() {
    // `1♥ - 2♣ -`: we opened 1♥ (partner is us at index 0... no — at
    // len 4, index 0 is Me), partner responded 2♣ (game-forcing 2/1).
    let auction = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
    ];
    let inf = read(&auction);
    // Index 0 (1♥) is four before the actor → Me, the opener.
    assert_eq!(inf.me().length(Suit::Hearts), Range::new(5, 13));
    // Index 2 (2♣) is two before → Partner, the 2/1 responder.
    assert_eq!(inf.partner().length(Suit::Clubs), Range::new(4, 13));
    assert_eq!(inf.partner().strength.points, Range::new(13, 37));
}

#[test]
fn opener_rebid_reads_five_plus_by_default() {
    // `1♥ - 1♠ - 2♥ -`: the opener (who bid 1♥ and rebid 2♥) sits as
    // partner, and the 1♠ responder is us.  The shipped sound reading
    // keeps the rebid at five-plus (the floor routinely rebids a good
    // five); the legacy six-card claim needs the knob off.
    let auction = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read(&auction);
    assert_eq!(inf.partner().length(Suit::Hearts), Range::new(5, 13));
    // Our 1♠ response showed four spades and six-plus points.
    assert_eq!(inf.me().length(Suit::Spades), Range::new(4, 13));
    assert_eq!(inf.me().strength.points, Range::new(6, 37));
    let mut agreements = Agreements::default();
    agreements.decision.reading.length_soundness = false;
    let legacy = read_with(&agreements, &auction);
    assert_eq!(legacy.partner().length(Suit::Hearts), Range::new(6, 13));
}

#[test]
fn competitive_opener_rebid_shows_sixth_card() {
    // `1♦ (1♥) - (2♥) 3♦ -`: partner opened 1♦ and, over the opponents'
    // heart auction, rebid 3♦ (the opt-in `InstinctKnobs::competitive_rebid` floor).
    // The natural length reading applies in competition too — only the
    // *strength* reading is suppressed when opponents act — so partner is
    // still read with six-plus diamonds, keeping the sampler and any further
    // interference sound.  Knob-independent: `read` interprets the auction.
    let auction = [
        bid(1, Strain::Diamonds),
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Hearts),
        bid(3, Strain::Diamonds),
        Call::Pass,
    ];
    let inf = read(&auction);
    assert_eq!(inf.partner().length(Suit::Diamonds), Range::new(6, 13));
}

#[test]
fn overcall_shows_five_cards() {
    // `1♦ (1♠)`: partner opened 1♦ and RHO overcalled 1♠.
    let auction = [bid(1, Strain::Diamonds), bid(1, Strain::Spades)];
    let inf = read(&auction);
    // Index 0 (1♦ opening) → Partner; index 1 (1♠ overcall) → Rho.
    assert_eq!(inf.partner().length(Suit::Diamonds), Range::new(3, 13));
    assert_eq!(inf.rho().length(Suit::Spades), Range::new(5, 13));
    assert_eq!(inf.rho().strength.points, Range::new(8, 37));
}

#[test]
fn transfers_are_not_read_as_natural() {
    // `1NT - 2♦ -`: 2♦ is a Jacoby transfer, not diamonds — the
    // opening side's artificial response leaves shape unknown.
    let auction = [
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
    ];
    let inf = read(&auction);
    assert_eq!(inf.partner().length(Suit::Diamonds), Range::FULL_LENGTH);
}

#[test]
fn three_level_suit_over_one_notrump_is_natural() {
    // `1NT - 3♥ -`: with the splinter *not* authored, a three-level suit
    // bid over 1NT is forcing and natural in the instinct reading —
    // five-plus hearts.  This is the knob-off control for
    // `nt_splinter_is_read_as_shortness_not_length`; the splinter is on by
    // default, so the walk has to be asked for explicitly.
    let mut agreements = Agreements::default();
    agreements.decision.reading.nt_splinter = false;
    let auction = [
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read_with(&agreements, &auction);
    assert_eq!(inf.partner().length(Suit::Hearts), Range::new(5, 13));
}

#[test]
fn nt_splinter_is_read_as_shortness_not_length() {
    // `1NT - 3♥ -` with the splinter authored: the *same* call that reads
    // as five-plus hearts above now decodes off its alert into the pinned
    // shape — short hearts, 2-3 spades, exactly four diamonds, 5-6 clubs.
    // The natural walk would floor a phantom heart suit responder is void in.
    let mut agreements = Agreements::default();
    agreements.decision.reading.nt_splinter = true;
    let auction = [
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read_booked_with(&agreements, &auction);

    let partner = inf.partner();
    assert!(partner.length(Suit::Hearts).max <= 1);
    assert_eq!(partner.length(Suit::Spades), Range::new(2, 3));
    assert_eq!(partner.length(Suit::Diamonds), Range::new(4, 4));
    assert_eq!(partner.length(Suit::Clubs), Range::new(5, 6));

    // Knob off, the book has no 3♥ rule and the walk is back: five-plus.
    agreements.decision.reading.nt_splinter = false;
    let off = read_booked_with(&agreements, &auction);
    assert_eq!(off.partner().length(Suit::Hearts), Range::new(5, 13));
}

#[test]
fn systems_on_overcall_transfer_is_not_read_as_diamonds() {
    // `(1♦) 1NT - 2♦ -`: their 1♦, our 1NT overcall, the advancer's 2♦ is a
    // Jacoby transfer (grafted opening-1NT structure), not natural diamonds.
    // Stripping their opening reads it as `1NT - 2♦ -`, so the floor never
    // raises a phantom diamond suit into a doubled disaster (the iron rule).
    let auction = [
        bid(1, Strain::Diamonds),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
    ];
    let inf = read(&auction);
    assert_eq!(inf.partner().length(Suit::Diamonds), Range::FULL_LENGTH);
}

#[test]
fn systems_on_stripped_read_is_separate_from_the_full_decision_cache() {
    let auction = [
        bid(1, Strain::Diamonds),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
    ];
    let hand: Hand = "AQ32.K53.QJ4.A92".parse().expect("valid test hand");
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.scope = ReadingScope::All;
    let partnership = crate::american(&agreements).bind();
    let uncached = partnership.infer(RelativeVulnerability::NONE, &auction);
    let context = partnership
        .prefixed_context(RelativeVulnerability::NONE, &auction)
        .with_decision_cache(hand);
    let cached = context.inferences();

    assert_eq!(*cached, uncached);
    assert_eq!(context.decision_cache_init_counts(), Some((1, 0, 0)));
    assert_eq!(cached.me().strength.hcp, Range::new(15, 18));
    assert_eq!(cached.partner().length(Suit::Diamonds), Range::FULL_LENGTH);
}

/// `ReadingScope::All` publishes what an unalerted authored rule promises.
///
/// `gladiator_advances` authors the game-forcing `3♦` as
/// `len(♦, 5..) & points(game..)`.  It is natural, so it carries no alert and
/// the projection pass skips it: the walk supplies a length floor and the
/// game force is simply lost.  Knob-on the rule's own box is intersected in.
#[test]
fn all_reading_publishes_an_unalerted_rules_promise() {
    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(3, Strain::Diamonds),
        Call::Pass,
    ];

    let mut agreements = Agreements::default();
    agreements.decision.reading.nt_overcall_gladiator = true;
    agreements.decision.reading.envelope_union = true;
    assert_eq!(ReadingScope::default(), ReadingScope::All);
    agreements.decision.reading.scope = ReadingScope::Alerted;
    let off = read_booked_with(&agreements, &auction);
    agreements.decision.reading.scope = ReadingScope::All;
    let on = read_booked_with(&agreements, &auction);

    assert_eq!(
        off.partner().strength.points,
        Range::FULL_POINTS,
        "knob-off the game force is unread"
    );
    assert!(
        on.partner().strength.points.min >= 10,
        "knob-on the rule's `points(game..)` reaches the reading, got {:?}",
        on.partner().strength.points,
    );
    // The walk's natural reading survives: the call is not suppressed, so
    // the diamond suit is still read from the auction, not only from the box.
    assert!(on.partner().length(Suit::Diamonds).min >= 5);
}

#[test]
fn completed_major_transfer_shows_five() {
    // `1NT - 2♦ - 2♥ -`: partner transferred to hearts and we
    // completed; at length 6 the responder is us (Me).  The transfer shows a
    // five-card major even before a jump confirms the sixth, while the
    // transferred-*from* suit stays unread.
    let auction = [
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read_booked(&auction);
    assert_eq!(inf.me().length(Suit::Hearts), Range::new(5, 13));
    assert_eq!(inf.me().length(Suit::Diamonds), Range::FULL_LENGTH);
}

#[test]
fn transfer_jump_to_game_shows_at_least_five() {
    // `1NT - 2♦ - 2♥ - 4♥ -`: partner transferred then jumped to 4♥.
    // The projection reads the 2♦ transfer's authored rule — a five-card floor;
    // the old reader's six-card upgrade off the jump is dropped (soundness over
    // tightness, M6.2c).  At length 8 the responder sits as Partner.
    let auction = [
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(4, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read_booked(&auction);
    assert_eq!(inf.partner().length(Suit::Hearts), Range::new(5, 13));
}

#[test]
fn transfer_then_three_major_shows_at_least_five() {
    // `1NT - 2♦ - 2♥ - 3♥ -`: a raise of the transferred suit.  The
    // projection pins the transfer's five-card floor; the old reader's six-card
    // upgrade and the 8–9 invitational points are dropped (soundness over
    // tightness, M6.2c).
    let auction = [
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read_booked(&auction);
    assert!(inf.partner().length(Suit::Hearts).min >= 5);
}

#[test]
fn transfer_projection_covers_spades_and_two_notrump() {
    // Spade transfer (2♥ → 2♠) jumped to 4♠: the 2♥ transfer rule projects a
    // five-card spade floor.
    let spades = read_booked(&[
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Spades),
        Call::Pass,
        bid(4, Strain::Spades),
        Call::Pass,
    ]);
    assert_eq!(spades.partner().length(Suit::Spades), Range::new(5, 13));

    // The same shape over a 2NT opening (3♦ → 3♥, jump 4♥).
    let two_nt = read_booked(&[
        bid(2, Strain::Notrump),
        Call::Pass,
        bid(3, Strain::Diamonds),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
        bid(4, Strain::Hearts),
        Call::Pass,
    ]);
    assert_eq!(two_nt.partner().length(Suit::Hearts), Range::new(5, 13));
}

#[test]
fn contested_transfer_auction_is_not_specially_read() {
    // `1NT (2♣) 2♦ - 2♥ - 4♥ -`: with the opponents in, the transfer
    // positions shift, so the special reading must not pin a six-card suit.
    let auction = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(4, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read(&auction);
    assert!(inf.partner().length(Suit::Hearts).min < 6);
}

#[test]
fn contested_transfer_lebensohl_reads_the_target_under_intervention() {
    // Board 881510: `1NT (2♠) 3♦ (3♠)` — responder's 3♦ is a Transfer-
    // Lebensohl transfer to hearts (up the line through their spade suit).  RHO's
    // (3♠) skips opener's completion node; the default-on fallback projection
    // re-resolves 3♦'s authoring rule and pins hearts, so opener does not read it
    // as natural diamonds and raise the phantom suit to 5♦x.  Needs the prefixed
    // `read_booked` (the projection reads the rule off the book).
    let auction = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Spades),
        bid(3, Strain::Diamonds),
        bid(3, Strain::Spades),
    ];
    let inf = read_booked(&auction);
    assert!(
        inf.partner().length(Suit::Hearts).min >= 5,
        "transfer target pinned"
    );
    assert!(
        inf.partner().length(Suit::Diamonds).min < 5,
        "phantom suit not read"
    );
}

/// The §7.3.1 union poison (docs/ai-bidder/bba-kickback.md): with
/// `set_kickback` on, the relocated-ask and answer rules on 4♥/4♠ were
/// structurally alerted, so a **natural** 4♠'s box was unioned with the
/// ask's ⊤ projection — partner's `length(Spades).min` collapsed to 0 and
/// the natural walk's lane bookkeeping was suppressed on top.  The face
/// gate makes those rules as-if-absent on faces where `kickback_ladder`
/// claims nothing (here no suit is bid twice by one side, so the ladder is
/// all-`None`): the knob-on reading must equal the knob-off one.
#[test]
fn kickback_face_gate_keeps_natural_four_spades_natural() {
    use crate::bidding::instinct::RkcbVariant;
    // The audited C−B shape: 1♦ - 1♠ - 2♦ - 4♠ - — the reader is the
    // opener, partner is the natural 4♠ bidder.
    let auction = [
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(4, Strain::Spades),
        Call::Pass,
    ];
    let baseline = read_booked(&auction).partner().length(Suit::Spades).min;
    let mut agreements = Agreements::default();
    agreements.decision.reading.rkcb_variant = RkcbVariant::Kickback;
    let gated = read_booked_with(&agreements, &auction)
        .partner()
        .length(Suit::Spades)
        .min;
    assert!(baseline >= 4, "the natural walk floors responder's spades");
    assert_eq!(gated, baseline, "kickback must not erase the natural floor");
}

/// The face gate's positive control: where the ladder *does* claim the
/// call (hearts agreed, spades unguarded → 4♠ asks), the rule stays live —
/// alerted, so the ask is not read as a natural spade suit.
#[test]
fn kickback_relocated_ask_still_reads_as_the_convention() {
    use crate::bidding::instinct::RkcbVariant;
    let auction = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
        bid(4, Strain::Spades),
        Call::Pass,
    ];
    let mut agreements = Agreements::default();
    agreements.decision.reading.rkcb_variant = RkcbVariant::Kickback;
    let spades = read_booked_with(&agreements, &auction)
        .partner()
        .length(Suit::Spades)
        .min;
    assert!(spades < 4, "the relocated ask is not a natural spade suit");
}

/// The default-system twin of the kickback poison: the plain 1430 answers
/// (5♣–5♠) and DOPI/ROPI/DEPO on X/XX are present in every partnership and
/// always alerted, so a **natural** floor 5♦ — no ask anywhere on the
/// face — reads as a keycard answer: the union with the answer rules' ⊤
/// projection erases partner's diamond floor and the `alerted` bit
/// suppresses the natural walk.  The `Rules::face` gates confine the
/// rules to a live ask window, so the natural reading survives.
///
/// This was a differential test against `set_keycard_answer_gates`.  That
/// knob is gone — its off arm was the poison itself, not an agreement any
/// partnership could play — so the guard is now absolute: partner's
/// diamond floor must not be erased.  Remove the gates and it goes to
/// nothing, which is exactly the regression being pinned.
#[test]
fn answer_gates_spare_a_natural_five_diamonds() {
    use crate::bidding::instinct::RkcbVariant;
    // The plain arm on purpose (also the default): the poison this pins is
    // the *default system's* five-level answers, not the relocated
    // ladder's.
    let mut agreements = Agreements::default();
    agreements.decision.reading.rkcb_variant = RkcbVariant::Plain;
    let auction = [
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(5, Strain::Diamonds),
        Call::Pass,
    ];
    let diamonds = read_booked_with(&agreements, &auction)
        .partner()
        .length(Suit::Diamonds)
        .min;
    assert!(
        diamonds >= 2,
        "a natural 5♦ with no ask anywhere on the face must keep its \
         diamond floor, got {diamonds}"
    );
}

/// The gates' positive control: inside a live ask window the answer is
/// still alerted — a 5♦ answering 4NT is a keycard count, not diamonds.
#[test]
fn answer_gates_keep_the_live_window_alerted() {
    let auction = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
        bid(4, Strain::Notrump),
        Call::Pass,
        bid(5, Strain::Diamonds),
        Call::Pass,
    ];
    // The gates are the default: the in-window answer must stay alerted.
    let diamonds = read_booked(&auction).partner().length(Suit::Diamonds).min;
    assert!(
        diamonds < 4,
        "the in-window answer is not a natural diamond suit"
    );
}

#[test]
fn contested_transfer_lebensohl_direct_jacoby_over_2d() {
    // Over (2♦) the transfers are direct Jacoby: 3♦→♥.  `1NT (2♦) 3♦ (X)`.
    let auction = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Diamonds),
        bid(3, Strain::Diamonds),
        Call::Double,
    ];
    let inf = read_booked(&auction);
    assert!(inf.partner().length(Suit::Hearts).min >= 5);
}

#[test]
fn contested_transfer_lebensohl_cue_is_not_a_transfer() {
    // The cue of their suit is Stayman (a 4-card unbid major), not a 5+ transfer:
    // `1NT (2♠) 3♠ -` projects hearts as only 4-card interest, and the
    // natural-spades reading of the cue is suppressed (not a long spade suit).
    let auction = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Spades),
        bid(3, Strain::Spades),
        Call::Pass,
    ];
    let inf = read_booked(&auction);
    assert!(inf.partner().length(Suit::Hearts).min < 5);
    assert!(inf.partner().length(Suit::Spades).min < 5);
}

#[test]
fn relative_seat_tracks_the_actor() {
    // The same 1♥ opening lands on a different relative seat as the
    // auction grows by one call.
    assert_eq!(
        read(&[bid(1, Strain::Hearts)]).rho().strength.points,
        Range::new(10, 21)
    );
    assert_eq!(
        read(&[bid(1, Strain::Hearts), Call::Pass])
            .partner()
            .strength
            .points,
        Range::new(10, 21)
    );
}

#[test]
fn limited_notrump_rebids_narrow_strength() {
    // `1♦ - 1♥ - 1NT -`: the opener (partner) showed a 12–16 minimum.
    let one_nt = read(&[
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(1, Strain::Notrump),
        Call::Pass,
    ]);
    assert_eq!(one_nt.partner().strength.points, Range::new(12, 16));

    // A jump to 2NT is the strong 18–19 rebid (sound bound 18–21).
    let two_nt = read(&[
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Notrump),
        Call::Pass,
    ]);
    assert_eq!(two_nt.partner().strength.points, Range::new(18, 21));
}

#[test]
fn cheapest_two_notrump_over_a_response_is_not_strong() {
    // `1♦ - 2♣ - 2NT -`: 2NT is the *cheapest* notrump over a 2/1, a
    // minimum — it must not be read as the 18–19 jump.  Opener stays at the
    // opening floor (10–21).
    let inf = read(&[
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(2, Strain::Notrump),
        Call::Pass,
    ]);
    assert_eq!(inf.partner().strength.points, Range::new(10, 21));
}

#[test]
fn raises_and_one_notrump_response_narrow_the_responder() {
    // `1♥ - 2♥ -`: a single raise is 6–10 — a support-scale band, so
    // the dedicated gauge carries it exactly and the legacy axis holds
    // only its sound image (4-point shapely raises are measured fact:
    // the `1♠ - 2♠` divergence-meter defect).
    let single = read(&[
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
    ]);
    let hearts = Suit::Hearts as usize;
    assert_eq!(
        single.partner().strength.support_points[hearts],
        Range::new(6, 10)
    );
    assert_eq!(single.partner().strength.points, Range::new(1, 11));
    assert_eq!(single.partner().strength.shown_floor(), 6);
    // `1♥ - 3♥ -`: a limit (jump) raise is 10–12.
    let limit = read(&[
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
    ]);
    assert_eq!(
        limit.partner().strength.support_points[hearts],
        Range::new(10, 12)
    );
    assert_eq!(limit.partner().strength.points, Range::new(5, 13));
    // `1♥ - 1NT -`: a 1NT response is 6–12.
    let one_nt = read(&[
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(1, Strain::Notrump),
        Call::Pass,
    ]);
    assert_eq!(one_nt.partner().strength.points, Range::new(6, 12));
}

#[test]
fn competition_suppresses_the_limited_rebid_reading() {
    // `1♦ - 1♥ (1♠) 1NT -`: with the opponents in, opener's 1NT is not
    // the quiet 12–16 rebid — leave the strength at the opening floor
    // (10–21).
    let inf = read(&[
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Hearts),
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Pass,
    ]);
    assert_eq!(inf.partner().strength.points, Range::new(10, 21));
}

#[test]
fn their_cue_of_our_overcall_is_a_raise() {
    // 1♥ (2♦) 3♦: responder's cue of the overcalled suit is the limit-plus
    // heart raise — three-plus hearts, ten-plus points, and no diamond
    // length (the probe: two diamonds read as four).
    let mut agreements = Agreements::default();
    agreements.decision.reading.cue = true;
    let inf = read_with(
        &agreements,
        &[
            Call::Pass,
            Call::Pass,
            bid(1, Strain::Hearts),
            bid(2, Strain::Diamonds),
            bid(3, Strain::Diamonds),
        ],
    );
    assert_eq!(inf.rho().length(Suit::Diamonds), Range::FULL_LENGTH);
    assert!(inf.rho().length(Suit::Hearts).min >= 3);
    assert!(inf.rho().strength.support_points[Suit::Hearts as usize].min >= 10);
    assert!(inf.rho().strength.points.min >= 5);
}

#[test]
fn a_doublers_jump_is_not_a_weak_jump() {
    // `(2♠) X - 3♦ - 4♥`: the doubler's jump to game is strength, made
    // on as few as three hearts — never a weak six-card jump.
    let mut agreements = Agreements::default();
    agreements.decision.reading.length_soundness = true;
    let auction = [
        bid(2, Strain::Spades),
        Call::Double,
        Call::Pass,
        bid(3, Strain::Diamonds),
        Call::Pass,
        bid(4, Strain::Hearts),
    ];
    let inf = read_with(&agreements, &auction);
    assert_eq!(inf.rho().length(Suit::Hearts), Range::FULL_LENGTH);
    agreements.decision.reading.length_soundness = false;
    let off = read_with(&agreements, &auction);
    assert!(off.rho().length(Suit::Hearts).min >= 6);
}

#[test]
fn an_agreed_suit_re_raise_adds_no_length() {
    // 1♥ - 2♥ - 3♥: opener's game-try re-raise of the agreed suit adds
    // no length — the five from the opening stands, not a phantom sixth.
    let mut agreements = Agreements::default();
    agreements.decision.reading.length_soundness = true;
    let auction = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(3, Strain::Hearts),
    ];
    let inf = read_with(&agreements, &auction);
    assert_eq!(inf.rho().length(Suit::Hearts).min, 5);
    agreements.decision.reading.length_soundness = false;
    let off = read_with(&agreements, &auction);
    assert_eq!(off.rho().length(Suit::Hearts).min, 6);
}

#[test]
fn opener_minor_rebid_reads_five_plus() {
    // 1♦ - 1♠ - 2♦: opener's two-level rebid of the opened minor is
    // routinely a good five-card suit, not six (the probe: five of eight
    // rebids were made on five).
    let mut agreements = Agreements::default();
    agreements.decision.reading.length_soundness = true;
    let auction = [
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Diamonds),
    ];
    let inf = read_with(&agreements, &auction);
    assert_eq!(inf.rho().length(Suit::Diamonds).min, 5);
    agreements.decision.reading.length_soundness = false;
    let off = read_with(&agreements, &auction);
    assert_eq!(off.rho().length(Suit::Diamonds).min, 6);
}

#[test]
fn their_splinter_is_disclosed_to_the_table() {
    // 1♠ - 4♦ read by a defender: their splinter is alerted and
    // explained at the table, so it decodes off their authoring rule —
    // diamond shortness with spade support, never diamond length.
    let auction = [bid(1, Strain::Spades), Call::Pass, bid(4, Strain::Diamonds)];
    let mut agreements = Agreements::default();
    agreements.decision.reading.table_alerts = true;
    let inf = read_booked_with(&agreements, &auction);
    assert!(inf.rho().length(Suit::Diamonds).max <= 1);
    agreements.decision.reading.table_alerts = false;
    let off = read_booked_with(&agreements, &auction);
    assert_eq!(off.rho().length(Suit::Diamonds).max, 13);
}

#[test]
fn their_checkback_is_disclosed_to_the_table() {
    // 1♦ - 1♠ - 1NT - 2♣ read by a defender: their artificial
    // checkback 2♣ promises no clubs — the natural walk floored four (the
    // probe: four-plus clubs read on a singleton).
    let auction = [
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Clubs),
    ];
    let mut agreements = Agreements::default();
    agreements.decision.reading.table_alerts = true;
    let inf = read_booked_with(&agreements, &auction);
    assert!(inf.rho().length(Suit::Clubs).min < 4);
    agreements.decision.reading.table_alerts = false;
    let off = read_booked_with(&agreements, &auction);
    assert!(off.rho().length(Suit::Clubs).min >= 4);
}

/// The alerted choice-of-games 3NT decodes: opener reads responder as
/// (4333) with 3+ in every suit (so the 5-3 major fit is known), exactly
/// three spades over 1♥, and 12+ points.
#[test]
fn choice_of_games_three_notrump_reads_support() {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.response.major_choice_of_games = true;
    let partnership = crate::american(&agreements).bind();

    let auction = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(3, Strain::Notrump),
        Call::Pass,
    ];
    let read =
        Inferences::read(&partnership.prefixed_context(RelativeVulnerability::NONE, &auction));
    assert!(read.partner().length(Suit::Hearts).min >= 3);
    assert!(read.partner().length(Suit::Diamonds).min >= 3);
    assert!(read.partner().length(Suit::Clubs).min >= 3);
    assert_eq!(read.partner().length(Suit::Spades), Range::new(3, 3));
    assert!(read.partner().strength.points.min >= 12);
}

proptest! {
    /// Soundness: a hand that opens the book's choice falls within the
    /// opening inference.  Tests rule 1 (the opening table) over random hands.
    #[test]
    fn opening_inference_contains_the_opener(seed in any::<u64>()) {
        use crate::bidding::trie::Classifier;
        use crate::bidding::american::openings;
        use contract_bridge::deck::full_deal;
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let deal = full_deal(&mut rng);
        let hand: Hand = deal[contract_bridge::Seat::North];

        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let logits = openings(&Agreements::default()).classify(hand, &context);
        let Some((call, _)) = (&logits.0)
            .into_iter()
            .filter(|(_, l)| l.is_finite())
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("not NaN"))
        else {
            return Ok(());
        };
        let Call::Bid(_) = call else { return Ok(()); };

        // The opener sits to the actor's right after a single call.
        let inf = read(&[call]);
        let opener = inf.rho();
        let points = point_count(hand);
        prop_assert!(
            opener.strength.points.contains(points),
            "{call} opener with {points} points outside {:?}",
            opener.strength.points
        );
        for suit in Suit::ASC {
            let length = hand[suit].len();
            // SAFETY: a suit length is at most 13.
            #[allow(clippy::cast_possible_truncation)]
            let length = length as u8;
            prop_assert!(
                opener.length(suit).contains(length),
                "{call} opener with {length} {suit:?} outside {:?}",
                opener.length(suit)
            );
        }
    }

    /// The load-bearing C1/C2 pin: closing the boxes is **membership-inert**
    /// on the real reading path, so the sampler cannot move.  Every hand a
    /// reading admitted knob-off it still admits knob-on, and vice versa —
    /// on the lenient `EnvelopeUnion::contains` the sampler uses *and* the strict
    /// `Envelope::accepts` gate.  If this ever fires, the closure is
    /// dropping legal hands and the A/B verdict means nothing.
    #[test]
    fn closure_is_membership_inert(seed in any::<u64>()) {
        use crate::bidding::constraint::{Constraint as _, and, balanced, hcp, len, or, points};
        use contract_bridge::deck::full_deal;
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let deal = full_deal(&mut rng);
        let hand: Hand = deal[contract_bridge::Seat::North];

        let mut agreements = Agreements::default();
        agreements.decision.reading.envelope_union = true;
        agreements.decision.reading.sum_closure = false;
        agreements.decision.reading.upgrade_closure = false;
        let loose_profile = agreements.decision.reading;
        let context =
            Context::new(RelativeVulnerability::NONE, &[]).with_profile(agreements.decision);
        let readings = [
            (balanced() & points(15..17)).project_band(&context),
            (or([Suit::Hearts, Suit::Spades], 5..) & points(8..)).project(&context),
            (and([Suit::Hearts, Suit::Spades], 5..) & hcp(6..11)).project_band(&context),
            (len(Suit::Spades, 6..) & points(13..)).project(&context),
            (!balanced() & points(12..)).project(&context),
        ];

        for reading in readings {
            let loose = reading.clone().tidy(loose_profile);
            let mut closed_profile = loose_profile;
            closed_profile.sum_closure = true;
            closed_profile.upgrade_closure = true;
            let closed = reading.tidy(closed_profile);

            prop_assert_eq!(
                loose.contains_on(hand, loose_profile),
                closed.contains_on(hand, closed_profile),
                "contains moved: {:?} vs {:?}", loose, closed
            );
            prop_assert_eq!(
                loose.boxes().iter().any(|b| b.accepts(hand)),
                closed.boxes().iter().any(|b| b.accepts(hand)),
                "accepts moved: {:?} vs {:?}", loose, closed
            );
        }
    }
}
