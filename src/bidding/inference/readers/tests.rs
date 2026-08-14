use crate::bidding::agreements::Agreements;
use crate::bidding::context::Context;
use crate::bidding::inference::tests::{
    bid, chosen_call, read, read_booked, read_booked_with, read_with,
};
use crate::bidding::inference::{EnvelopeUnion, Range, ReadingProfile, Relative};
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Hand, Strain, Suit};

fn gladiator_agreements() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.decision.reading.nt_overcall_gladiator = true;
    agreements
}

fn rubens_agreements() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.decision.reading.rubens_advances = true;
    agreements
}

/// [`ReadingProfile::envelope_union`] gates the `Or` wall: off,
/// `or([♥, ♠], 6..)` hulls to one
/// box that admits a 5-4 hand with no six-card major; on, it keeps the two
/// boxes and rejects that hand while still admitting each true one-suiter.
#[test]
fn envelope_union_reading_pins_the_two_suiter() {
    use crate::bidding::constraint::{Constraint, or};
    assert!(
        std::thread::spawn(|| ReadingProfile::default().envelope_union)
            .join()
            .unwrap(),
        "the envelope-union reading must default on"
    );

    // Holdings are spades.hearts.diamonds.clubs.
    let six_spades: Hand = "AKQJ32.KQ4.32.32".parse().unwrap();
    let six_hearts: Hand = "KQ4.AKQJ32.32.32".parse().unwrap();
    let five_four: Hand = "AKQJ3.KQ42.32.32".parse().unwrap(); // no six-card major
    let reading = or([Suit::Hearts, Suit::Spades], 6..);

    let mut agreements = Agreements::default();
    agreements.decision.reading.envelope_union = true;
    let on = Context::new(RelativeVulnerability::NONE, &[]).with_profile(agreements.decision);
    let boxes = reading.project(&on);
    let expected_legacy_hull = EnvelopeUnion::from(boxes.hull());
    assert_eq!(boxes.boxes().len(), 2, "on: one box per major");
    assert!(boxes.contains(six_spades) && boxes.contains(six_hearts));
    assert!(
        !boxes.contains(five_four),
        "on: neither box holds the 5-4 hand"
    );

    agreements.decision.reading.envelope_union = false;
    let off = Context::new(RelativeVulnerability::NONE, &[]).with_profile(agreements.decision);
    let hull = reading.project(&off);
    assert_eq!(hull, expected_legacy_hull, "off: the legacy span");
    assert_eq!(hull.boxes().len(), 1, "off: one bounding box");
    assert!(
        hull.contains(five_four),
        "off: the hull admits the 5-4 slop"
    );
}

#[test]
fn leaping_michaels_conditions_partner() {
    use crate::bidding::agreements::Agreements;

    // (2♥) 4♣ -: the advancer reads partner's two-suiter — five-plus clubs
    // AND five-plus spades, game-forcing — so the search sampler deals partner
    // the right shape rather than a natural club one-suiter.
    let mut on = Agreements::default();
    on.defense.leaping_michaels_enabled = true;
    let advance = read_booked_with(
        &on,
        &[bid(2, Strain::Hearts), bid(4, Strain::Clubs), Call::Pass],
    );
    assert_eq!(advance.partner().length(Suit::Clubs), Range::new(5, 13));
    assert_eq!(advance.partner().length(Suit::Spades), Range::new(5, 13));
    assert_eq!(advance.partner().strength.points, Range::new(14, 37));

    // Over 2♦, the 4♦ cue shows both majors; 4♣ shows clubs + an unknown
    // major, so only clubs is pinned.
    let cue = read_booked_with(
        &on,
        &[
            bid(2, Strain::Diamonds),
            bid(4, Strain::Diamonds),
            Call::Pass,
        ],
    );
    assert_eq!(cue.partner().length(Suit::Hearts), Range::new(5, 13));
    assert_eq!(cue.partner().length(Suit::Spades), Range::new(5, 13));

    // Disabled (the convention ships on): a 4♣ jump reads as a natural
    // one-suiter, so spades stay unconstrained — it must not leak when off.
    let mut disabled = Agreements::default();
    disabled.defense.leaping_michaels_enabled = false;
    let off = read_booked_with(
        &disabled,
        &[bid(2, Strain::Hearts), bid(4, Strain::Clubs), Call::Pass],
    );
    assert_eq!(off.partner().length(Suit::Spades), Range::FULL_LENGTH);
}

#[test]
fn landy_conditions_partner() {
    use crate::bidding::agreements::Agreements;

    // (1NT) 2♣ -: the advancer reads partner's both-majors two-suiter (at
    // least 4-4 in the majors, 8+ points) rather than a natural club suit.
    let mut on = Agreements::default();
    on.decision.reading.landy = true;
    on.decision.reading.convention_points = (8, 15);
    on.defense.unusual_notrump_range = Some((8, 15));
    let advance = read_booked_with(
        &on,
        &[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass],
    );
    assert_eq!(advance.partner().length(Suit::Hearts), Range::new(4, 13));
    assert_eq!(advance.partner().length(Suit::Spades), Range::new(4, 13));
    assert_eq!(advance.partner().length(Suit::Clubs), Range::FULL_LENGTH);
    assert_eq!(advance.partner().strength.points, Range::new(8, 37));

    // (1NT) 2NT -: both minors, 5-5 (the independent unusual-2NT toggle).
    let minors = read_booked_with(
        &on,
        &[bid(1, Strain::Notrump), bid(2, Strain::Notrump), Call::Pass],
    );
    assert_eq!(minors.partner().length(Suit::Clubs), Range::new(5, 13));
    assert_eq!(minors.partner().length(Suit::Diamonds), Range::new(5, 13));

    // The advancer's 2♦ relay is artificial — read from the overcaller's seat,
    // partner's (the relayer's) diamonds stay unconstrained.
    let relay = read_booked_with(
        &on,
        &[
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
        ],
    );
    assert_eq!(relay.partner().length(Suit::Diamonds), Range::FULL_LENGTH);

    // Disabled: 2♣ reads as a natural club one-suiter, so spades stay
    // unconstrained — the convention must not leak when off.  Landy is still a
    // agreement, so the disabled arm is built separately.
    let mut disabled = Agreements::default();
    disabled.decision.reading.landy = false;
    disabled.defense.unusual_notrump_range = None;
    let off = read_booked_with(
        &disabled,
        &[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass],
    );
    assert_eq!(off.partner().length(Suit::Spades), Range::FULL_LENGTH);
}

#[test]
fn woolsey_conditions_partner() {
    use crate::bidding::agreements::Agreements;
    use crate::bidding::american::NotrumpDefense;
    // Landy off, Woolsey on: the 2♣ must read through the Woolsey path.
    let mut arm = Agreements::default();
    arm.decision.reading.landy = false;
    arm.decision.reading.notrump_defense = NotrumpDefense::Woolsey;
    arm.decision.reading.convention_points = (10, 19);
    arm.defense.unusual_notrump_range = None;

    // (1NT) 2♣ -: Woolsey's 2♣ is both majors, 10+, never a natural club suit.
    // Read off the authored rule's projection (on a prefixed/booked context),
    // which pins each major to 4-5 exactly — Woolsey sends a six-card major to
    // the Multi/Muiderberg calls, a distinction the old loose reader missed.
    let two_c = read_booked_with(
        &arm,
        &[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass],
    );
    assert_eq!(two_c.partner().length(Suit::Hearts), Range::new(4, 5));
    assert_eq!(two_c.partner().length(Suit::Spades), Range::new(4, 5));
    assert_eq!(two_c.partner().length(Suit::Clubs), Range::FULL_LENGTH);
    assert_eq!(two_c.partner().strength.points, Range::new(10, 37));

    // (1NT) 2♦ -: the Multi names diamonds it does NOT hold, so the natural
    // ≥5 reading is suppressed and BOTH minors narrow to ≤4 — the floor can no
    // longer "raise diamonds" into a doubled 5♦ (the 6+ major falls out of the
    // residual the per-suit framework cannot pin).
    let multi = read_booked_with(
        &arm,
        &[
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            Call::Pass,
        ],
    );
    assert_eq!(multi.partner().length(Suit::Diamonds), Range::new(0, 4));
    assert_eq!(multi.partner().length(Suit::Clubs), Range::new(0, 4));

    // (1NT) 2♥ -: Muiderberg — exactly 5 hearts, ≤3 spades.
    let muiderberg = read_booked_with(
        &arm,
        &[bid(1, Strain::Notrump), bid(2, Strain::Hearts), Call::Pass],
    );
    assert_eq!(muiderberg.partner().length(Suit::Hearts), Range::new(5, 5));
    assert_eq!(muiderberg.partner().length(Suit::Spades), Range::new(0, 3));

    // The advancer's 2♥/2♠ over 2♣ (both majors) or 2♦ (Multi) is a PREFERENCE
    // among partner's two majors — not own length — so its natural ≥4 reading is
    // suppressed throughout (here, read from the advancer's seat as partner).
    let pref_2c = read_booked_with(
        &arm,
        &[
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ],
    );
    assert_eq!(pref_2c.partner().length(Suit::Hearts), Range::FULL_LENGTH);
    let pref_2d = read_booked_with(
        &arm,
        &[
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
        ],
    );
    assert_eq!(pref_2d.partner().length(Suit::Spades), Range::FULL_LENGTH);

    // Off: the Multi 2♦ reads as a natural diamond one-suiter again (≥5) — the
    // convention must not leak when disabled.
    arm.decision.reading.notrump_defense = NotrumpDefense::Natural;
    let off = read_booked_with(
        &arm,
        &[
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            Call::Pass,
        ],
    );
    assert_eq!(off.partner().length(Suit::Diamonds), Range::new(5, 13));
}

#[test]
fn woolsey_double_and_advances_read() {
    use crate::bidding::american::NotrumpDefense;
    let mut agreements = Agreements::default();
    agreements.decision.reading.landy = false;
    agreements.decision.reading.notrump_defense = NotrumpDefense::Woolsey;
    agreements.decision.reading.convention_points = (10, 19);
    agreements.decision.reading.woolsey_double_floor = 12;

    // (1NT) X -: the takeout double names no suit, so nothing is misread — but
    // the doubler's strength (12+) is recorded, where a bare double of 1NT would
    // otherwise read as nothing.
    let x = read_booked_with(
        &agreements,
        &[bid(1, Strain::Notrump), Call::Double, Call::Pass],
    );
    assert_eq!(x.partner().strength.points, Range::new(12, 37));

    // (1NT) X - 2♣ -: the advancer's 2♣ is a "name your minor" relay, not own
    // clubs, so its natural ≥4 reading is suppressed (read from the advancer seat).
    let relay = read_booked_with(
        &agreements,
        &[
            bid(1, Strain::Notrump),
            Call::Double,
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ],
    );
    assert_eq!(relay.partner().length(Suit::Clubs), Range::FULL_LENGTH);

    // (1NT) 2♥ - 2NT -: the Muiderberg minor-ask 2NT is a relay, never read as
    // a natural notrump invite.  Alerted, its own rule decodes: no-fit (≤2
    // hearts) and invitational-plus — `20 - lo` = 10 here — where the retired
    // reader could only suppress and left the points unconstrained.
    let ask = read_booked_with(
        &agreements,
        &[
            bid(1, Strain::Notrump),
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Notrump),
            Call::Pass,
        ],
    );
    assert_eq!(ask.partner().strength.points, Range::new(10, 37));
    assert_eq!(ask.partner().length(Suit::Hearts), Range::new(0, 2));

    // Off: the Woolsey 12+ reading must not leak — the double now falls through to
    // the default-on natural penalty reading (15+), not Woolsey's 12+.
    agreements.decision.reading.notrump_defense = NotrumpDefense::Natural;
    let off = read_booked_with(
        &agreements,
        &[bid(1, Strain::Notrump), Call::Double, Call::Pass],
    );
    assert_eq!(off.partner().strength.points, Range::new(15, 37));
}

#[test]
fn dont_overcalls_and_advances_read() {
    use crate::bidding::american::NotrumpDefense;
    let mut agreements = Agreements::default();
    agreements.decision.reading.landy = false;
    agreements.decision.reading.notrump_defense = NotrumpDefense::DirectDont;

    // (1NT) X -: a one-suiter in ♣/♦/♥ — spades short (≤3, the one sound fact),
    // strength recorded (the default 8+ overcall floor) where a bare double of 1NT
    // would otherwise read as nothing.
    let x = read_booked_with(
        &agreements,
        &[bid(1, Strain::Notrump), Call::Double, Call::Pass],
    );
    assert_eq!(x.partner().length(Suit::Spades), Range::new(0, 3));
    assert_eq!(x.partner().strength.points, Range::new(8, 37));

    // (1NT) X - 2♣ -: the advancer's 2♣ is a "name your suit" relay, not own
    // clubs, so its natural ≥4 reading is suppressed (read from the advancer seat).
    let relay = read_booked_with(
        &agreements,
        &[
            bid(1, Strain::Notrump),
            Call::Double,
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ],
    );
    assert_eq!(relay.partner().length(Suit::Clubs), Range::FULL_LENGTH);

    // (1NT) 2♣ -: a real ≥4 club suit + an unknown major.  The natural ≥5 reading
    // is suppressed (a 4-club / 5-major DONT hand makes this call), re-pinned to ≥4.
    let two_c = read_booked_with(
        &agreements,
        &[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass],
    );
    assert_eq!(two_c.partner().length(Suit::Clubs), Range::new(4, 13));
    assert_eq!(two_c.partner().strength.points, Range::new(8, 37));

    // (1NT) 2♣ - 2♦ -: the advancer's 2♦ is a "name your higher suit" relay,
    // not own diamonds — suppressed.
    let pref = read_booked_with(
        &agreements,
        &[
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
        ],
    );
    assert_eq!(pref.partner().length(Suit::Diamonds), Range::FULL_LENGTH);

    // (1NT) 2♥ -: both majors, ≥4-4 — exactly a Landy two-suiter on the 2♥ bid.
    let two_h = read_booked_with(
        &agreements,
        &[bid(1, Strain::Notrump), bid(2, Strain::Hearts), Call::Pass],
    );
    assert_eq!(two_h.partner().length(Suit::Hearts), Range::new(4, 13));
    assert_eq!(two_h.partner().length(Suit::Spades), Range::new(4, 13));

    // Off: the 2♣ reads as a natural club one-suiter again (≥5) — no leak.
    agreements.decision.reading.notrump_defense = NotrumpDefense::Natural;
    let off = read_booked_with(
        &agreements,
        &[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass],
    );
    assert_eq!(off.partner().length(Suit::Clubs), Range::new(5, 13));
}

#[test]
fn meckwell_overcalls_and_advances_read() {
    use crate::bidding::american::NotrumpDefense;
    let mut agreements = Agreements::default();
    agreements.decision.reading.landy = false;
    agreements.decision.reading.notrump_defense = NotrumpDefense::Meckwell;

    // (1NT) X -: the two-way double (single 6+ minor OR both majors) shares no
    // sound per-suit fact, so ONLY the points floor is recorded — no length is
    // narrowed (unlike DONT's X, which pins spades ≤ 3).
    let x = read_booked_with(
        &agreements,
        &[bid(1, Strain::Notrump), Call::Double, Call::Pass],
    );
    assert_eq!(x.partner().strength.points, Range::new(8, 37));
    assert_eq!(x.partner().length(Suit::Spades), Range::FULL_LENGTH);
    assert_eq!(x.partner().length(Suit::Hearts), Range::FULL_LENGTH);

    // (1NT) X - 2♣ -: the advancer's 2♣ is a "name your suit" relay, not own
    // clubs, so its natural ≥ 4 reading is suppressed.
    let relay = read_booked_with(
        &agreements,
        &[
            bid(1, Strain::Notrump),
            Call::Double,
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ],
    );
    assert_eq!(relay.partner().length(Suit::Clubs), Range::FULL_LENGTH);

    // (1NT) 2♣ -: a real ≥ 4 club suit + an unknown major.  The natural ≥ 5
    // reading is suppressed (a 4-club / 5-major hand makes this call), re-pinned ≥ 4.
    let two_c = read_booked_with(
        &agreements,
        &[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass],
    );
    assert_eq!(two_c.partner().length(Suit::Clubs), Range::new(4, 13));
    assert_eq!(two_c.partner().strength.points, Range::new(8, 37));

    // (1NT) 2♦ -: diamonds + a major, real ≥ 4.
    let two_d = read_booked_with(
        &agreements,
        &[
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            Call::Pass,
        ],
    );
    assert_eq!(two_d.partner().length(Suit::Diamonds), Range::new(4, 13));

    // (1NT) 2♥ -: NATURAL hearts (Meckwell's 2♥ is a single-suiter, not DONT's
    // both-majors), so spades are not floored — the DONT-vs-Meckwell fork.
    let two_h = read_booked_with(
        &agreements,
        &[bid(1, Strain::Notrump), bid(2, Strain::Hearts), Call::Pass],
    );
    assert_eq!(
        two_h.partner().length(Suit::Spades).min,
        0,
        "natural 2♥ shows no spades",
    );

    // Off: the 2♣ reads as a natural club one-suiter again (≥ 5) — no leak.
    agreements.decision.reading.notrump_defense = NotrumpDefense::Natural;
    let off = read_booked_with(
        &agreements,
        &[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass],
    );
    assert_eq!(off.partner().length(Suit::Clubs), Range::new(5, 13));
}

#[test]
fn gladiator_cue_is_not_read_as_their_major() {
    // `(1♠) 1NT - 2♠ -`: our 1NT overcall of their 1♠; the advancer's 2♠ is
    // Gladiator Stayman for hearts (exactly 4, INV+) — NOT a natural spade
    // suit.  The major-strip is suppressed for Gladiator, so `gladiator_reading`
    // reads the cue.
    let agreements = gladiator_agreements();
    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Spades),
        Call::Pass,
    ];
    let inf = read_with(&agreements, &auction);
    // Their major is never floored into the advancer's hand (the iron rule)...
    assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
    // ...and the cue pins the four-card heart holding it promised.
    assert_eq!(inf.partner().length(Suit::Hearts), Range::new(4, 13));
}

#[test]
fn gladiator_relay_is_not_read_as_clubs() {
    // `(1♠) 1NT - 2♣ -`: the advancer's 2♣ is the Gladiator relay (weak /
    // invitational, any suit), not a natural club suit.
    let agreements = gladiator_agreements();
    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
    ];
    let inf = read_with(&agreements, &auction);
    assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
}

#[test]
fn gladiator_delayed_cue_is_read_as_exactly_three_not_spades() {
    // `(1♠) 1NT - 2♣ - 2♦ - 2♠ -`: the advancer's SECOND 2♠ (after the 2♣ relay
    // and forced 2♦) is the Gladiator delayed cue — exactly 3 hearts, INV+ —
    // NOT a natural spade suit.  The suppression must cover it too, else the
    // floor raises a phantom spade suit into a doubled disaster (the iron rule).
    let agreements = gladiator_agreements();
    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Spades),
        Call::Pass,
    ];
    let inf = read_with(&agreements, &auction);
    // Their major is never floored into the advancer's hand...
    assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
    // ...and the delayed cue pins exactly 3 hearts.
    assert_eq!(inf.partner().length(Suit::Hearts), Range::new(3, 3));
}

#[test]
fn gladiator_stolen_relay_double_is_read_as_the_relay() {
    // `(1♠) 1NT (2♣) X -`: over RHO's systems-on 2♣, the advancer's Double is
    // the stolen Gladiator relay (weak-or-invitational, any suit) — NOT a
    // penalty double naming clubs.  The reader mirrors the book rebase.
    let agreements = gladiator_agreements();
    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        Call::Double,
        Call::Pass,
    ];
    let inf = read_with(&agreements, &auction);
    // No phantom club suit raised from the doubled strain...
    assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
    // ...and no point cap: the relay's third arm is game-forcing, so the
    // `0..=9` this used to assert excluded hands the agreement admits (see
    // the `Relay` arm of the post-walk block).
    assert_eq!(inf.partner().strength.points, Range::FULL_POINTS);
}

/// Do we play the card we claim to play?
///
/// Our Gladiator (`ReadingProfile::nt_overcall_gladiator`) adapts the Crowborough card
/// — <https://www.bridgewebs.com/crowborough/NT%20Responses.htm> — from a
/// 1NT *opening* to our 1NT *overcall*, where `2♦` is natural and the cue
/// is Stayman, so the relay must also park the hands that card's `2♦`
/// Extended Stayman takes.  This replays the **bidder** (not the rule
/// table) over one representative hand per advance and per relay
/// continuation, so a floor that drifts under the structure shows up as a
/// red test rather than as a convention that quietly stops firing.
#[test]
fn gladiator_advances_follow_the_card() {
    let agreements = gladiator_agreements();
    let partnership = crate::american(&agreements).bind();
    let node = [bid(1, Strain::Spades), bid(1, Strain::Notrump), Call::Pass];
    // After the relay and its forced 2♦ puppet: the XYZ-style sort.
    let sorted: Vec<Call> = node
        .iter()
        .copied()
        .chain([
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
        ])
        .collect();

    // (hand, auction, expected call, what the hand is)
    let rows: &[(&str, &[Call], Call, &str)] = &[
        // Their major is ♠, so the one unbid major `o` is ♥ throughout.
        (
            "873.93.KJ973.T94",
            &node,
            bid(2, Strain::Clubs),
            "weak with 5+♦ — the relay's weak takeout arm",
        ),
        (
            "K872.Q93.J84.Q93",
            &node,
            bid(2, Strain::Clubs),
            "invitational, nothing to bid directly — the relay's INV arm",
        ),
        (
            "K3.Q876.KJ84.972",
            &node,
            bid(2, Strain::Spades),
            "INV with exactly 4♥, not 4333 — the cue, Stayman for ♥",
        ),
        (
            "K3.972.KJ864.Q93",
            &node,
            bid(2, Strain::Diamonds),
            "INV with exactly 5♦ — natural",
        ),
        (
            "93.KJ864.K73.Q92",
            &node,
            bid(2, Strain::Hearts),
            "INV with exactly 5♥ — natural",
        ),
        (
            "93.874.J6.KQ9764",
            &node,
            bid(2, Strain::Notrump),
            "weak with 6+♣ — the transfer to clubs",
        ),
        (
            "3.KQ86.AJ84.K976",
            &node,
            bid(3, Strain::Spades),
            "GF raise of ♥ with a singleton spade — the splinter",
        ),
        // The relay's continuations over the forced 2♦.
        (
            "873.93.KJ973.T94",
            &sorted,
            Call::Pass,
            "weak with ♦ — pass the puppet",
        ),
        (
            "93.KJ864.T73.972",
            &sorted,
            bid(2, Strain::Hearts),
            "weak with 5+♥ — the takeout",
        ),
        (
            "K872.Q93.J84.Q93",
            &sorted,
            bid(2, Strain::Notrump),
            "balanced INV (flat 4333: no delayed cue)",
        ),
        (
            "K872.Q93.KJ84.9",
            &sorted,
            bid(2, Strain::Spades),
            "INV with exactly 3♥, not 4333 — the delayed cue",
        ),
        (
            "932.7.QJ9764.KJ2",
            &sorted,
            bid(3, Strain::Diamonds),
            "INV with a good 6-card suit",
        ),
        // The relay's *third* arm — a game-forcing balanced hand with
        // exactly 3♥ — is authored but weight-shadowed: at 0.5 it loses to
        // `3NT` (1.2) and to the 3-level naturals (1.3), so no hand plays
        // it.  Deliberate (the box is too confined to adjudicate an A/B on),
        // and pinned here so the divergence is documented rather than
        // hidden: the arm is read, never played.
        (
            "K942.Q76.AJ83.K4",
            &node,
            bid(3, Strain::Notrump),
            "GF balanced with exactly 3♥ — arm 3 is shadowed by 3NT",
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for &(text, auction, expected, what) in rows {
        let hand: Hand = text.parse().expect("a hand");
        let made = chosen_call(&partnership, hand, auction);
        if made != expected {
            failures.push(format!("{text} ({what}): bid {made}, carded {expected}"));
        }
    }
    assert!(
        failures.is_empty(),
        "Gladiator diverges from the card:\n{}",
        failures.join("\n"),
    );
}

/// A doubled 1NT overcall runs out — it does not jump to the three level.
///
/// Gladiator turns off `systems_on_overcall_strip`, which is what let the
/// floor read `(1M) 1NT (X)` as a doubled *opening* 1NT.  Without it the
/// distilled net escaped a 1-count to `3♥`; `gladiator_doubled_runout` is
/// the book node that shadows it.
#[test]
fn gladiator_runs_out_of_the_doubled_overcall() {
    let agreements = gladiator_agreements();
    let partnership = crate::american(&agreements).bind();
    let node = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Double,
    ];

    // (hand, expected, what it is)
    let rows: &[(&str, Call, &str)] = &[
        ("873.93.KJ973.T94", bid(2, Strain::Diamonds), "bust, 5♦"),
        ("93.KJ864.T73.972", bid(2, Strain::Hearts), "bust, 5♥"),
        ("93.874.J6.KQ9764", bid(2, Strain::Clubs), "bust, 6♣"),
        (
            "8732.932.J973.T4",
            Call::Pass,
            "1-count, no five-bagger: sit",
        ),
        (
            "T9843.93.J973.T4",
            Call::Pass,
            "bust with five of THEIR major: sit, never run into it",
        ),
        ("K872.Q93.J84.Q93", Call::Redouble, "values: play 1NT××"),
    ];

    let mut failures: Vec<String> = Vec::new();
    for &(text, expected, what) in rows {
        let hand: Hand = text.parse().expect("a hand");
        let made = chosen_call(&partnership, hand, &node);
        if made != expected {
            failures.push(format!("{text} ({what}): bid {made}, carded {expected}"));
        }
    }
    assert!(
        failures.is_empty(),
        "the doubled 1NT overcall misplays its runout:\n{}",
        failures.join("\n"),
    );
}

/// Every Gladiator continuation ends where the card says, not where the
/// floor guesses.
///
/// Authoring a node **shadows** the floor, so this sweep is also the record
/// of what is deliberately *not* authored: every "advancer passes the game
/// opposite a limited hand" leaf below is answered by the floor and answered
/// right, and a bare `Pass` node there would only cost the floor its slam
/// machinery.  The three that are authored are the ones the floor got wrong
/// — it raised a weak signoff on three trumps, bid `3NT` opposite a hand
/// that had denied 8 points, and answered Leaping Michaels `4♣` with `5NT`.
#[test]
fn gladiator_continuations_are_authored_to_the_leaf() {
    let agreements = gladiator_agreements();
    let partnership = crate::american(&agreements).bind();
    let p = Call::Pass;
    let base = [bid(1, Strain::Spades), bid(1, Strain::Notrump), p];
    let seq =
        |tail: &[Call]| -> Vec<Call> { base.iter().copied().chain(tail.iter().copied()).collect() };
    let relay = bid(2, Strain::Clubs);
    let forced = bid(2, Strain::Diamonds);

    // (auction, hand, expected, what)
    let rows: Vec<(Vec<Call>, &str, Call, &str)> = vec![
        // --- authored: the floor was wrong here ---
        (
            seq(&[relay, p, forced, p, bid(2, Strain::Hearts), p]),
            "AQ8.AK9.Q852.A93",
            p,
            "16 with three hearts: pass the weak signoff (floor raised)",
        ),
        (
            seq(&[relay, p, forced, p, bid(2, Strain::Hearts), p]),
            "AQ86.AKJ.Q85.A93",
            p,
            "17 with three hearts: pass (floor bid 3NT opposite a bust)",
        ),
        (
            seq(&[relay, p, forced, p, bid(2, Strain::Hearts), p]),
            "AQ8.AKJ2.Q85.A9",
            bid(3, Strain::Hearts),
            "18 with four hearts: the one sound push",
        ),
        (
            seq(&[bid(4, Strain::Clubs), p]),
            "AQ8.AK9.Q852.A93",
            bid(4, Strain::Hearts),
            "Leaping 4♣ (5-5 hearts+clubs GF), three-card fit (floor bid 5NT)",
        ),
        (
            seq(&[bid(4, Strain::Diamonds), p]),
            "AQ86.AKJ.Q85.A93",
            bid(4, Strain::Hearts),
            "Leaping 4♦, three-card fit",
        ),
        (
            seq(&[bid(4, Strain::Spades), p]),
            "AQ8.AK9.Q852.A93",
            bid(5, Strain::Diamonds),
            "Leaping 4♠ (both minors), diamonds the longer",
        ),
        // --- deliberately left to the floor, and it answers right ---
        (
            seq(&[bid(2, Strain::Notrump), p, bid(3, Strain::Clubs), p]),
            "93.874.J6.KQ9764",
            p,
            "weak club transfer completed: pass",
        ),
        (
            seq(&[forced, p, bid(3, Strain::Notrump), p]),
            "K3.972.KJ864.Q93",
            p,
            "invitational 2♦ accepted to 3NT: pass",
        ),
        (
            seq(&[bid(2, Strain::Hearts), p, bid(4, Strain::Hearts), p]),
            "93.KJ864.K73.Q92",
            p,
            "invitational 2♥ raised to game: pass",
        ),
        (
            seq(&[
                relay,
                p,
                forced,
                p,
                bid(2, Strain::Notrump),
                p,
                bid(3, Strain::Notrump),
                p,
            ]),
            "K872.Q93.J84.Q93",
            p,
            "balanced invitation accepted: pass",
        ),
        (
            seq(&[bid(3, Strain::Spades), p, bid(4, Strain::Hearts), p]),
            "3.KQ86.AJ84.K976",
            p,
            "splinter raised to game: pass",
        ),
        (
            seq(&[bid(3, Strain::Diamonds), p, bid(3, Strain::Notrump), p]),
            "KQT.K8.AJT64.QJ4",
            p,
            "game-forcing 3♦ placed in 3NT: pass",
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (auction, text, expected, what) in rows {
        let hand: Hand = text.parse().expect("a hand");
        let made = chosen_call(&partnership, hand, &auction);
        if made != expected {
            failures.push(format!("{text} ({what}): bid {made}, wanted {expected}"));
        }
    }
    assert!(
        failures.is_empty(),
        "Gladiator continuations land in the wrong place:\n{}",
        failures.join("\n"),
    );
}

/// Gladiator keeps the systems-on strip where it has no structure of its own.
///
/// Over RHO's **X** and over 3-level-or-higher interference, Gladiator and
/// systems-on play the same auction (a natural runout, then the floor), so
/// the strip identity still holds and the inference-aware floor keeps the
/// picture it was distilled on.  Over a pass or a 2-level bid it does not —
/// the advances, the stolen relay and Transfer Lebensohl all diverge.
#[test]
fn gladiator_keeps_the_strip_where_it_has_no_structure() {
    let agreements = gladiator_agreements();
    let p = Call::Pass;
    let one_s = bid(1, Strain::Spades);
    let one_nt = bid(1, Strain::Notrump);
    // (auction after `(1♠) 1NT`, stripped?)
    let rows: &[(&[Call], bool, &str)] = &[
        (&[Call::Double], true, "their X — a runout in both systems"),
        (
            &[bid(3, Strain::Clubs)],
            true,
            "3-level — the floor in both",
        ),
        (
            &[bid(4, Strain::Hearts)],
            true,
            "4-level — the floor in both",
        ),
        (&[], false, "quiet — the Gladiator advances"),
        (&[p], false, "quiet — the Gladiator advances"),
        (
            &[bid(2, Strain::Clubs)],
            false,
            "their 2♣ — the stolen relay",
        ),
        (
            &[bid(2, Strain::Hearts)],
            false,
            "their 2♥ — Transfer Lebensohl",
        ),
    ];
    let mut failures: Vec<String> = Vec::new();
    for &(tail, want, what) in rows {
        let auction: Vec<Call> = [one_s, one_nt]
            .into_iter()
            .chain(tail.iter().copied())
            .collect();
        let got = crate::bidding::inference::read::systems_on_overcall_strip(
            &auction,
            agreements.decision.reading,
        )
        .is_some();
        if got != want {
            failures.push(format!("{what}: stripped = {got}, wanted {want}"));
        }
    }
    assert!(
        failures.is_empty(),
        "strip scope wrong:\n{}",
        failures.join("\n")
    );
}

#[test]
fn gladiator_contested_transfer_lebensohl_pins_the_target() {
    // `(1♠) 1NT (2♥) 3♦ -`: over RHO's 2♥ there is no room for the relay
    // tree, so advancer plays Transfer Lebensohl; 3♦ transfers up through their
    // hearts (showing spades), read via the builders' alerts — opener must not
    // raise a phantom diamond suit.
    let agreements = gladiator_agreements();
    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        bid(2, Strain::Hearts),
        bid(3, Strain::Diamonds),
        Call::Pass,
    ];
    let inf = read_booked_with(&agreements, &auction);
    assert!(
        inf.partner().length(Suit::Spades).min >= 5,
        "transfer target pinned"
    );
    assert!(
        inf.partner().length(Suit::Diamonds).min < 5,
        "phantom suit not read"
    );
}

#[test]
fn fallback_projection_decodes_contested_leaping_michaels() {
    // `1NT (2♦) 4♦ -`: Leaping Michaels = both majors 5-5, authored as a
    // *guarded fallback* in the (2♦) Transfer block — invisible to the exact-node
    // projection, and with no hand reader.  The default-on fallback projection
    // re-resolves its authoring rule and pins both majors (no reader involved).
    let auction = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Diamonds),
        bid(4, Strain::Diamonds),
        Call::Pass,
    ];
    let inf = read_booked(&auction);
    assert!(
        inf.partner().length(Suit::Hearts).min >= 5 && inf.partner().length(Suit::Spades).min >= 5,
        "fallback projection pins both majors for contested Leaping Michaels"
    );
}

#[test]
fn rubens_cue_raise_shows_support() {
    // (1♠) 2♣ - 2♠ -: we overcalled 2♣, partner cue-raised 2♠ — a
    // limit-plus club raise.  The overcaller reads three-plus clubs and
    // ten-plus points, but no spade length (the cue is a relay).
    let inf = read(&[
        bid(1, Strain::Spades),
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(2, Strain::Spades),
        Call::Pass,
    ]);
    assert!(inf.partner().length(Suit::Clubs).min >= 3);
    // A support-scale promise: exact on the club slot, only its sound
    // image on the legacy axis.
    assert!(inf.partner().strength.support_points[Suit::Clubs as usize].min >= 10);
    assert!(inf.partner().strength.points.min >= 5);
    assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
}

#[test]
fn rubens_transfer_is_not_read_as_natural() {
    // (1♣) 1♠ - 2♣ -: we overcalled 1♠, partner transferred 2♣ (a relay
    // to diamonds).  The bid suit must not be read as a club holding.
    let agreements = rubens_agreements();
    let inf = read_with(
        &agreements,
        &[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ],
    );
    assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
}

#[test]
fn rubens_reading_respects_the_knob() {
    // With Rubens advances off — the default since the layer A/B — the same
    // 2♣ is a genuine club suit: the suppression lifts and it reads naturally.
    let mut agreements = Agreements::default();
    agreements.decision.reading.rubens_advances = false;
    agreements.decision.reading.cue = false;
    let inf = read_with(
        &agreements,
        &[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ],
    );
    assert!(inf.partner().length(Suit::Clubs).min >= 4);
}

#[test]
fn their_minor_cue_reads_as_michaels() {
    // (1♣) 2♣: the direct cue of their minor opening is Michaels — both
    // majors, five-five, and no club length (the probe caught a club void
    // read as five clubs).  Off, the old overcall reading returns.
    let mut agreements = Agreements::default();
    agreements.decision.reading.cue = true;
    let inf = read_with(&agreements, &[bid(1, Strain::Clubs), bid(2, Strain::Clubs)]);
    assert_eq!(inf.rho().length(Suit::Clubs), Range::FULL_LENGTH);
    assert!(inf.rho().length(Suit::Hearts).min >= 5);
    assert!(inf.rho().length(Suit::Spades).min >= 5);
    agreements.decision.reading.cue = false;
    let off = read_with(&agreements, &[bid(1, Strain::Clubs), bid(2, Strain::Clubs)]);
    assert!(off.rho().length(Suit::Clubs).min >= 5);
}

#[test]
fn their_jump_cue_over_a_weak_two_is_leaping_michaels() {
    // (2♦) 4♦: the jump cue of a weak-two minor is Leaping Michaels — both
    // majors, no diamond length (the probe: a diamond void read as six).
    let mut agreements = Agreements::default();
    agreements.decision.reading.cue = true;
    let inf = read_with(
        &agreements,
        &[
            Call::Pass,
            bid(2, Strain::Diamonds),
            bid(4, Strain::Diamonds),
        ],
    );
    assert_eq!(inf.rho().length(Suit::Diamonds), Range::FULL_LENGTH);
    assert!(inf.rho().length(Suit::Hearts).min >= 5);
    assert!(inf.rho().length(Suit::Spades).min >= 5);
}

#[test]
fn their_michaels_is_disclosed_to_the_table() {
    // 1♠ (2♠) read by the opening side: their Michaels cue resolves in
    // *their* phase-routed book (defensive at their turn) and decodes off
    // the authored rule — five-plus hearts *with the rule's strength
    // floor*, which the retired `two_suiter_reading` never knew (chop 1,
    // `docs/reader-retirement.md`).  This knob is now the only owner of
    // the reading, so its off arm is the honest record of what the
    // retirement gives up: the shape floor goes too.
    let auction = [bid(1, Strain::Spades), bid(2, Strain::Spades)];
    let mut agreements = Agreements::default();
    agreements.decision.reading.table_alerts = true;
    let inf = read_booked_with(&agreements, &auction);
    assert!(inf.rho().length(Suit::Hearts).min >= 5);
    assert!(inf.rho().strength.points.min >= 8);
    assert_eq!(inf.rho().length(Suit::Spades).min, 0);
    agreements.decision.reading.table_alerts = false;
    let off = read_booked_with(&agreements, &auction);
    assert_eq!(off.rho().strength.points.min, 0);
    assert_eq!(off.rho().length(Suit::Hearts).min, 0);
}

#[test]
fn rubens_limit_raise_transfer_records_support() {
    let agreements = rubens_agreements();
    // (1♣) 1♠ - 2♥ -: partner's transfer into our spades is the
    // limit-plus raise — the overcaller reads three-plus spades and
    // ten-plus points, while the named hearts stay unread (a relay).
    let inf = read_with(
        &agreements,
        &[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ],
    );
    assert!(inf.partner().length(Suit::Spades).min >= 3);
    assert!(inf.partner().strength.points.min >= 10);
    assert_eq!(inf.partner().length(Suit::Hearts), Range::FULL_LENGTH);
}

#[test]
fn rubens_new_suit_transfer_records_the_target() {
    let agreements = rubens_agreements();
    // (1♣) 1♠ - 2♣ -: the new-suit transfer shows the advancer's own
    // five-card diamond suit and ten-plus points; clubs stay unread.
    let inf = read_with(
        &agreements,
        &[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ],
    );
    assert!(inf.partner().length(Suit::Diamonds).min >= 5);
    assert!(inf.partner().strength.points.min >= 10);
    assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
}

#[test]
fn rubens_transfer_records_despite_intervention() {
    let agreements = rubens_agreements();
    // (1♣) 1♠ - 2♥ (X): opener doubles the transfer — the completion
    // never comes, but the shown limit raise is exactly what the
    // overcaller needs for the competitive decision.
    let inf = read_with(
        &agreements,
        &[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Double,
        ],
    );
    assert!(inf.partner().length(Suit::Spades).min >= 3);
    assert!(inf.partner().strength.points.min >= 10);
}

#[test]
fn rubens_transfer_is_not_read_for_the_opponents() {
    // Same auction read from the opening side (the advancer is now our
    // LHO): the opponents' agreement is not assumed — an in-band advance
    // from the other side may be a genuine suit, so nothing is recorded.
    let inf = read(&[
        bid(1, Strain::Clubs),
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        Call::Pass,
    ]);
    assert_eq!(inf.lho().length(Suit::Spades), Range::FULL_LENGTH);
    assert_eq!(inf.lho().strength.points, Range::FULL_POINTS);
}

/// Their Michaels cue of our opened major, post-retirement (chop 1)
///
/// The reading now comes from the authored `.alert(MICHAELS)` rule's own
/// projection, so the auction must be read **keyed** (`read_booked`) and
/// the setting that owns the reading is `ReadingProfile::table_alerts`, not
/// `agreements.competition.uvu_over_majors` (which kept only its book
/// half).  The projection also carries the rule's strength floor, which
/// the retired reader never did.
#[test]
fn michaels_cue_over_our_major_reads_the_other_major() {
    // `1♥ (2♥)`: their direct cue of our opened major is Michaels — 5+
    // spades with the rule's 8+ floor, and NOT a natural heart suit (the
    // walk's misread suppressed by the alert).
    let inf = read_booked(&[bid(1, Strain::Hearts), bid(2, Strain::Hearts)]);
    assert!(inf.rho().length(Suit::Spades).min >= 5, "the shown major");
    assert!(inf.rho().strength.points.min >= 8, "the rule's floor");
    assert_eq!(
        inf.rho().length(Suit::Hearts),
        Range::FULL_LENGTH,
        "the cue is not natural hearts"
    );

    // Table-wide disclosure and the shipped cue reading both off: the
    // pre-package natural reading is preserved verbatim.
    let mut agreements = Agreements::default();
    agreements.decision.reading.table_alerts = false;
    agreements.decision.reading.cue = false;
    let inf = read_booked_with(
        &agreements,
        &[bid(1, Strain::Hearts), bid(2, Strain::Hearts)],
    );
    assert!(inf.rho().length(Suit::Hearts).min >= 5);
    assert_eq!(inf.rho().length(Suit::Spades), Range::FULL_LENGTH);
}

/// Their unusual `(2NT)` over our major, post-retirement (chop 1) — as
/// above, but the authored rule is a single box, so it pins both minors
/// *and* the strength floor.
#[test]
fn unusual_2nt_over_our_major_reads_both_minors() {
    let inf = read_booked(&[bid(1, Strain::Spades), bid(2, Strain::Notrump)]);
    assert!(inf.rho().length(Suit::Clubs).min >= 5);
    assert!(inf.rho().length(Suit::Diamonds).min >= 5);
    assert!(inf.rho().strength.points.min >= 8, "the rule's floor");

    // Table-wide disclosure off: nothing recorded for their 2NT (a notrump
    // bid never entered the natural suit walk either).
    let mut agreements = Agreements::default();
    agreements.decision.reading.table_alerts = false;
    let inf = read_booked_with(
        &agreements,
        &[bid(1, Strain::Spades), bid(2, Strain::Notrump)],
    );
    assert_eq!(inf.rho().length(Suit::Clubs), Range::FULL_LENGTH);
    assert_eq!(inf.rho().length(Suit::Diamonds), Range::FULL_LENGTH);
    assert_eq!(inf.rho().strength.points, Range::FULL_POINTS);
}

/// The retirement guard for chop 1 (`docs/reader-retirement.md`)
///
/// `two_suiter_reading` claimed `other_major >= 5` for their Michaels cue
/// and `♣ >= 5 && ♦ >= 5` for their unusual `(2NT)`.  Every one of those
/// claims is a **subset** of the authoring rule's projection on every
/// auction the reader used to fire on — both seat-fans of the opening and
/// both reading seats (the opponents' call decoded by the table-alert
/// walk, and the same call decoded own-side at the advancer's turn) — and
/// the projection adds the rule's `points >= 8` on top.  That subset
/// property is why the chop needed no A/B: the reader's `narrow_length`
/// was already an idempotent intersect against a hull folded in before it.
#[test]
fn retired_two_suiter_reader_is_subsumed_by_the_projection() {
    let michaels: [(&[Call], Relative); 3] = [
        (
            &[bid(1, Strain::Hearts), bid(2, Strain::Hearts)],
            Relative::Rho,
        ),
        (
            &[Call::Pass, bid(1, Strain::Hearts), bid(2, Strain::Hearts)],
            Relative::Rho,
        ),
        // The advancer's turn: index 1 is now our own side, decoded by the
        // exact-node walk rather than the table-alert one.
        (
            &[bid(1, Strain::Hearts), bid(2, Strain::Hearts), Call::Pass],
            Relative::Partner,
        ),
    ];
    for (auction, who) in michaels {
        let inf = read_booked(auction);
        let shown = inf.get(who);
        assert!(
            shown.length(Suit::Spades).min >= 5,
            "{auction:?}: the retired reader's other-major floor"
        );
        assert!(
            shown.strength.points.min >= 8,
            "{auction:?}: the floor the reader never carried"
        );
        assert_eq!(
            shown.length(Suit::Hearts),
            Range::FULL_LENGTH,
            "{auction:?}: the cue is not natural hearts"
        );
    }

    let unusual: [(&[Call], Relative); 2] = [
        (
            &[bid(1, Strain::Spades), bid(2, Strain::Notrump)],
            Relative::Rho,
        ),
        (
            &[bid(1, Strain::Spades), bid(2, Strain::Notrump), Call::Pass],
            Relative::Partner,
        ),
    ];
    for (auction, who) in unusual {
        let inf = read_booked(auction);
        let shown = inf.get(who);
        assert!(
            shown.length(Suit::Clubs).min >= 5,
            "{auction:?}: the retired reader's club floor"
        );
        assert!(
            shown.length(Suit::Diamonds).min >= 5,
            "{auction:?}: the retired reader's diamond floor"
        );
        assert!(
            shown.strength.points.min >= 8,
            "{auction:?}: the floor the reader never carried"
        );
    }
}

#[test]
fn uvu_major_cue_projects_the_raise() {
    // `1♥ (2NT) 3♣ -` from opener's seat: partner's cheap cue is the
    // alerted limit-plus raise — decoded off its authored rule's
    // projection (3+ hearts, 10+), not as natural clubs.  `read_booked`
    // builds under `Agreements::default()`, whose
    // `competition.uvu_over_majors` is on by default — the arming this
    // test needs.
    let inf = read_booked(&[
        bid(1, Strain::Hearts),
        bid(2, Strain::Notrump),
        bid(3, Strain::Clubs),
        Call::Pass,
    ]);
    let cue_bidder = inf.partner();
    assert!(
        cue_bidder.length(Suit::Hearts).min >= 3,
        "the projected fit"
    );
    assert!(
        cue_bidder.strength.points.min >= 10,
        "the projected strength"
    );
    assert_eq!(
        cue_bidder.length(Suit::Clubs),
        Range::FULL_LENGTH,
        "not natural clubs"
    );
}

#[test]
fn rubens_transfer_reading_knob_recovers_suppress_only() {
    // Stage-2 knob off: the transfer is still suppressed (not natural
    // hearts) but records nothing — the pre-fix shape.
    let mut agreements = Agreements::default();
    agreements.decision.reading.rubens_advances = true;
    agreements.decision.reading.rubens_transfer = false;
    let inf = read_with(
        &agreements,
        &[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ],
    );
    assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
    assert_eq!(inf.partner().length(Suit::Hearts), Range::FULL_LENGTH);
    assert_eq!(inf.partner().strength.points, Range::FULL_POINTS);
}

fn their_landy_agreements() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.decision.their.two_clubs_landy = true;
    agreements.decision.reading.their_landy_reading = true;
    agreements
}

#[test]
fn their_disclosed_landy_conditions_the_overcaller() {
    // 1NT (2♣): under the disclosure + wiring, responder reads RHO's 2♣ as
    // 4-4+ in the majors with no club or strength claim — not the natural
    // walk's 5+ clubs and 8+.
    let on = their_landy_agreements();
    let reading = read_booked_with(&on, &[bid(1, Strain::Notrump), bid(2, Strain::Clubs)]);
    assert_eq!(reading.rho().length(Suit::Hearts), Range::new(4, 13));
    assert_eq!(reading.rho().length(Suit::Spades), Range::new(4, 13));
    assert_eq!(reading.rho().length(Suit::Clubs), Range::FULL_LENGTH);
    assert_eq!(reading.rho().strength.points, Range::FULL_POINTS);

    // The disclosure without the wiring — the pre-N1g state — was the false
    // envelope the wiring fixed: natural 5+ clubs, 8+.
    let mut unwired = Agreements::default();
    unwired.decision.their.two_clubs_landy = true;
    unwired.decision.reading.their_landy_reading = false;
    let legacy = read_booked_with(&unwired, &[bid(1, Strain::Notrump), bid(2, Strain::Clubs)]);
    assert_eq!(legacy.rho().length(Suit::Clubs), Range::new(5, 13));
    assert_eq!(legacy.rho().strength.points, Range::new(8, 37));
}

#[test]
fn their_disclosed_landy_suppresses_the_advances() {
    let on = their_landy_agreements();

    // 1NT (2♣) X (2♥), opener to act: RHO's 2♥ is a preference among
    // partner's majors, playable on a doubleton — suppressed, nothing
    // recorded.  LHO (the overcaller) still reads 4-4+.
    let advance = read_booked_with(
        &on,
        &[
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Double,
            bid(2, Strain::Hearts),
        ],
    );
    assert_eq!(advance.lho().length(Suit::Hearts), Range::new(4, 13));
    assert_eq!(advance.lho().length(Suit::Spades), Range::new(4, 13));
    assert_eq!(advance.rho().length(Suit::Hearts), Range::FULL_LENGTH);

    // 1NT (2♣) - (3♥): the direct raise would otherwise read as a weak-jump
    // six-carder — false under Landy, where it is an invitational preference.
    let jump = read_booked_with(
        &on,
        &[
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(3, Strain::Hearts),
        ],
    );
    assert_eq!(jump.rho().length(Suit::Hearts), Range::FULL_LENGTH);
}

#[test]
fn their_landy_reading_is_seat_gated() {
    // The mirror image — (1NT) 2♣ (P), OUR 2♣ overcall of THEIR 1NT — must
    // stay the natural walk's club overcall even with the disclosure + wiring
    // on: the disclosure is a fact about the *reader's* opponents.  This is
    // the seat-correctness the cue-constraint arms' mirror leak lacked.
    let on = their_landy_agreements();
    let mirror = read_booked_with(
        &on,
        &[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass],
    );
    assert_eq!(mirror.partner().length(Suit::Clubs), Range::new(5, 13));
    assert_eq!(mirror.partner().length(Suit::Spades), Range::FULL_LENGTH);
    assert_eq!(mirror.partner().strength.points, Range::new(8, 37));
}

#[test]
fn their_landy_reading_needs_the_disclosure() {
    // The knob without the declared disclosure is inert: flipping it off
    // changes nothing about an undeclared 2♣, seat by seat (the `Inferences`
    // values differ only in their embedded profile).
    let mut knob_only = Agreements::default();
    knob_only.decision.reading.their_landy_reading = false;
    let auction = [bid(1, Strain::Notrump), bid(2, Strain::Clubs)];
    let with_knob = read_booked_with(&knob_only, &auction);
    let plain = read_booked_with(&Agreements::default(), &auction);
    for who in [
        Relative::Me,
        Relative::Lho,
        Relative::Partner,
        Relative::Rho,
    ] {
        assert_eq!(with_knob.get(who), plain.get(who), "{who:?}");
        assert_eq!(with_knob.announced(who), plain.announced(who), "{who:?}");
    }
}

#[test]
fn their_landy_reading_does_not_extrapolate_through_the_strip() {
    // (1♣) 1NT (2♣): the systems-on strip re-reads this as an opening-1NT
    // auction, where the seat gate alone would pass — but their 2♣ here is
    // *responder's* call on the side that opened, never a Landy defense.
    // The disclosure must not reach through the strip: their 2♣ stays the
    // natural walk's club overcall (the first A/B's worst boards were this
    // leak firing on `(1♣) 1NT (2♣)` lanes).
    let on = their_landy_agreements();
    let reading = read_booked_with(
        &on,
        &[
            bid(1, Strain::Clubs),
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
        ],
    );
    assert_eq!(reading.rho().length(Suit::Clubs), Range::new(5, 13));
    assert_eq!(reading.rho().length(Suit::Hearts), Range::FULL_LENGTH);
    assert_eq!(reading.rho().length(Suit::Spades), Range::FULL_LENGTH);
}
