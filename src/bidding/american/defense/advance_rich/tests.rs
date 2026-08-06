use super::super::tests::{best_call, call};
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Hand, Strain};

#[test]
fn longest_first_advance_bids_the_longer_suit() {
    // (1♣)–X–(P): 5 diamonds + 4 spades, a 7-HCP minimum.  This pins the
    // *flat* book (`--no-ns-rich-advance`): it scores both 4+ suits alike, so
    // the argmax bids the higher-ranking major 1♠; with the knob on, the
    // weight climbs with length and the longer diamonds win the advance.
    let over_1c = [call(1, Strain::Clubs), Call::Double, Call::Pass];
    let hand = "KJ32.32.K8765.32"; // 4 spades, 5 diamonds
    super::advance_rich::set_rich_advance_double(false);
    super::advance_rich::set_longest_first_advance(false);
    let (flat, _) = best_call(&over_1c, hand);
    assert_eq!(
        flat,
        call(1, Strain::Spades),
        "flat advance bids the higher-ranking 4-card major",
    );

    super::advance_rich::set_longest_first_advance(true);
    let (longest, floored) = best_call(&over_1c, hand);
    // Equal-length ties still break to the higher-ranking suit: 4-4 majors → 1♠
    // (the flat book has no invitational jump; the rich book would jump to 2M).
    let (tie, _) = best_call(&over_1c, "KJ32.KQ32.543.32");
    super::advance_rich::set_longest_first_advance(true); // restore defaults
    super::advance_rich::set_rich_advance_double(true);

    assert_eq!(
        longest,
        call(1, Strain::Diamonds),
        "longest-first bids the 5-card diamonds over the 4-card spades",
    );
    assert!(
        !floored,
        "the natural advance is a book node, not the floor"
    );
    assert_eq!(
        tie,
        call(1, Strain::Spades),
        "4-4 majors advance the higher-ranking spades",
    );
}

#[test]
fn longest_first_advance_governs_the_rich_book() {
    // The rich advance's *negative* suit picks obey the same discipline: the
    // weak natural suit and the forced-when-broke suit both go longest-first.
    let over_1c = [call(1, Strain::Clubs), Call::Double, Call::Pass];
    let over_1h = [call(1, Strain::Hearts), Call::Double, Call::Pass];
    super::advance_rich::set_rich_advance_double(true);
    super::advance_rich::set_longest_first_advance(false);

    // Weak two-suiter (7 HCP, 4 spades + 5 diamonds): flat rich advances the
    // higher-ranking 1♠, longest-first advances the longer 1♦.
    let two_suiter = "KJ32.32.K8765.32";
    let (flat_weak, _) = best_call(&over_1c, two_suiter);
    super::advance_rich::set_longest_first_advance(true);
    let (longest_weak, _) = best_call(&over_1c, two_suiter);
    super::advance_rich::set_longest_first_advance(false);

    // Forced bust (0 HCP, no 4-card suit outside their hearts): flat rich is
    // forced by the argmax into the higher-level 2♦; longest-first prefers the
    // higher-ranking — and cheaper — 1♠ among the equal 3-card suits.
    let bust = "432.5432.432.432";
    let (flat_forced, _) = best_call(&over_1h, bust);
    super::advance_rich::set_longest_first_advance(true);
    let (longest_forced, floored) = best_call(&over_1h, bust);

    super::advance_rich::set_longest_first_advance(true); // restore defaults
    super::advance_rich::set_rich_advance_double(true);

    assert_eq!(
        flat_weak,
        call(1, Strain::Spades),
        "flat rich weak → higher spades"
    );
    assert_eq!(
        longest_weak,
        call(1, Strain::Diamonds),
        "rich weak → longer diamonds"
    );
    assert_eq!(
        flat_forced,
        call(2, Strain::Diamonds),
        "flat rich forced → higher index"
    );
    assert_eq!(
        longest_forced,
        call(1, Strain::Spades),
        "rich forced → higher spades"
    );
    assert!(
        !floored,
        "the forced advance is a rich book node, not the floor"
    );
}

/// The weak sit yields to a 4-card unbid major under
/// [`set_advance_pass_yield_major`]: below the cue band the trump stack
/// bids the ladder; a 10+ hand or a majorless one sits as before.
#[test]
fn advance_pass_yields_to_a_major_only_when_weak() {
    let over_1c = [call(1, Strain::Clubs), Call::Double, Call::Pass];
    super::advance_rich::set_rich_advance_double(true);
    super::advance_rich::set_longest_first_advance(true);

    // 4 HCP, five clubs, four spades: sits by default...
    let stack = "KJ32.32.32.87654";
    let (sit, _) = best_call(&over_1c, stack);
    assert_eq!(sit, Call::Pass, "default: the weak stack sits");

    super::advance_rich::set_advance_pass_yield_major(true);
    // ...but yields to the spade major under the knob.
    let (yielded, _) = best_call(&over_1c, stack);
    assert_eq!(yielded, call(1, Strain::Spades), "yield: bid the major");

    // A cue-band sit (10 HCP) stands...
    let (strong, _) = best_call(&over_1c, "QJ32.Q32.2.KQ654");
    assert_eq!(strong, Call::Pass, "strong sit stands");

    // ...and so does a weak sit with no 4-card major.
    let (majorless, _) = best_call(&over_1c, "32.432.32.KJ8765");
    assert_eq!(majorless, Call::Pass, "majorless sit stands");

    // The flat book folds the same yield.
    super::advance_rich::set_rich_advance_double(false);
    let (flat, _) = best_call(&over_1c, "J432.32.32.KQ654");
    assert_eq!(flat, call(1, Strain::Spades), "flat book yields too");

    super::advance_rich::set_rich_advance_double(true);
    super::advance_rich::set_advance_pass_yield_major(false);
}

/// The 4-card sit's quality gate under [`set_advance_sit_hcp_gate`]:
/// `Some(5)` admits exactly AJxx (KJTx still advances), `Some(6)` drops
/// bare KQxx but keeps KQJx, and `None` keeps the shipped honor gate.
#[test]
fn advance_sit_hcp_gate_reshapes_the_4card_sit() {
    let over_1c = [call(1, Strain::Clubs), Call::Double, Call::Pass];
    super::advance_rich::set_rich_advance_double(true);

    // AJxx: 5 suit HCP but one top honor — forced 1♦ by default...
    let ajxx = "432.432.432.AJ32";
    let (default, _) = best_call(&over_1c, ajxx);
    assert_eq!(default, call(1, Strain::Diamonds), "default: AJxx advances");

    // ...sits under the 5+ floor...
    super::advance_rich::set_advance_sit_hcp_gate(Some(5));
    let (sits, _) = best_call(&over_1c, ajxx);
    assert_eq!(sits, Call::Pass, "5+ floor: AJxx sits");

    // ...while KJTx (4) still advances.
    let (kjtx, _) = best_call(&over_1c, "432.432.432.KJT2");
    assert_eq!(kjtx, call(1, Strain::Diamonds), "5+ floor: KJTx advances");

    // The 6+ floor drops bare KQxx (5) but keeps KQJx (6).
    super::advance_rich::set_advance_sit_hcp_gate(Some(6));
    let (kqxx, _) = best_call(&over_1c, "432.432.432.KQ32");
    assert_eq!(
        kqxx,
        call(1, Strain::Diamonds),
        "6+ floor: bare KQxx advances"
    );
    let (kqjx, _) = best_call(&over_1c, "432.432.432.KQJ2");
    assert_eq!(kqjx, Call::Pass, "6+ floor: KQJx sits");

    // Back to the honor gate: bare KQxx sits as shipped.
    super::advance_rich::set_advance_sit_hcp_gate(None);
    let (shipped, _) = best_call(&over_1c, "432.432.432.KQ32");
    assert_eq!(shipped, Call::Pass, "honor gate: KQxx sits");
}

/// The [`longest_unbid`] condition is an exact box union: `eval` and
/// knob-on box membership agree, opener's suit never competes, and an
/// equal-length tie goes to the higher rank — the reading the retired
/// weight ladder could never project.
#[test]
fn longest_unbid_reads_the_relative_length() {
    use super::Context;
    use crate::bidding::constraint::Constraint as _;
    use crate::bidding::inference::set_envelope_union_reading;
    use contract_bridge::Suit;

    set_envelope_union_reading(true);
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    // The ♦ instance over their (1♥): rivals ♠ (higher rank, must stay
    // strictly shorter) and ♣ (lower rank, may equal).
    let diamonds = super::longest_unbid(Suit::Diamonds, Suit::Hearts);
    let boxes = diamonds.project(&context);

    // Holdings are spades.hearts.diamonds.clubs.
    for (hand, held, why) in [
        ("432.32.K8765.432", true, "5♦ over 3♠/3♣ is the longest"),
        ("432.32.K876.5432", true, "the ♦=♣ tie goes to the higher ♦"),
        (
            "5432.32.K876.432",
            false,
            "the ♠=♦ tie goes to the higher ♠",
        ),
        ("65432.32.K876.32", false, "5♠ out-lengths 4♦"),
        ("32.65432.K876.32", true, "opener's longer ♥ never competes"),
    ] {
        let hand: Hand = hand.parse().unwrap();
        assert_eq!(boxes.contains(hand), held, "boxes: {why}");
        assert_eq!(
            diamonds.eval(hand, &context).is_finite(),
            held,
            "eval: {why}"
        );
    }
}

/// The rich advance gives the advancer a cue (invite+) and a forced 3-card
/// response when broke — both absent from the flat floor.
#[test]
fn rich_advance_double_cues_and_forces() {
    // (1♥) X (P) ? — advancer to act.
    let auction = [call(1, Strain::Hearts), Call::Double, Call::Pass];

    // 15 HCP, 4-3-3-3 with 4 spades but no heart stopper: game-forcing with
    // no limited natural home (3NT wants a stopper, 4♠ wants five) — cue.
    let force = "AQxx.xxx.AQx.Kxx";
    // 0 HCP, 3-4-3-3: no 4-card suit outside hearts, no trump stack — must
    // still bid (a takeout double cannot be passed for want of a call).
    let broke = "xxx.xxxx.xxx.xxx";

    super::advance_rich::set_rich_advance_double(true);
    let (cued, _) = best_call(&auction, force);
    let (forced, _) = best_call(&auction, broke);
    super::advance_rich::set_rich_advance_double(false);
    let (flat_force, _) = best_call(&auction, force);
    super::advance_rich::set_rich_advance_double(true); // restore default

    assert_eq!(
        cued,
        call(2, Strain::Hearts),
        "a game force with no limited natural home cues opener's suit"
    );
    assert!(
        matches!(forced, Call::Bid(_)),
        "a broke advancer bids rather than passing partner's takeout (got {forced:?})"
    );
    assert_ne!(
        flat_force,
        call(2, Strain::Hearts),
        "the flat floor has no cue-bid advance"
    );
}

/// The cue is invitational-or-better; the advancer clarifies over the
/// doubler's minimum answer — a game force reaches game, an invite stops.
#[test]
fn advance_cue_rebid_forces_or_invites() {
    // (1♥) X (P) 2♥cue (P) 2♠min (P) ? — advancer to clarify, with a known
    // spade fit from partner's minimum cheap-major answer.
    let auction = [
        call(1, Strain::Hearts),
        Call::Double,
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    // 15 HCP with 3 spades: game force — raise partner's spades to game.
    let force = "Axx.xxx.AKQx.Qxx";
    // 10 HCP with 3 spades: mere invite — partner showed a minimum, so stop.
    let invite = "Axx.xxx.AJxx.xxx";

    super::advance_rich::set_rich_advance_double(true);
    let (driven, _) = best_call(&auction, force);
    let (rested, _) = best_call(&auction, invite);
    super::advance_rich::set_rich_advance_double(true); // restore default

    assert_eq!(
        driven,
        call(4, Strain::Spades),
        "a game-forcing advancer raises the cue answer to game"
    );
    assert_eq!(
        rested,
        Call::Pass,
        "an invitational advancer passes the doubler's minimum"
    );
}

/// Without a Rubens transfer, `4M` is two-way: a shapely *weak* hand blasts
/// game just like a minimum game force (11–15 points), so a long-major
/// preempt is not stranded below a makeable game (the advance-double-v5
/// regression: pure MIN-FG `hcp(12..=15)` missed it).
#[test]
fn rich_advance_weak_shapely_blasts_game() {
    // (1♠) X (P) ? — advancer with a weak two-suiter, six hearts.
    let auction = [call(1, Strain::Spades), Call::Double, Call::Pass];
    // 8 HCP, 6-4 in hearts and clubs: too weak to invite (3♥ wants 10+), but
    // opposite a takeout double the shapely hand belongs in 4♥.
    let weak = "T3.AT9753.5.KQT7";

    super::advance_rich::set_rich_advance_double(true);
    let (blast, _) = best_call(&auction, weak);
    super::advance_rich::set_rich_advance_double(true); // restore default

    assert_eq!(
        blast,
        call(4, Strain::Hearts),
        "a shapely weak long-major hand blasts the two-way 4M, not a quiet 2-level"
    );
}

/// The invitational minor jump (`set_advance_minor_jump`): a three-level
/// minor jump shows a 5+ one-suiter (10–12) denying a 4-card unbid major.
/// With a 4-card major the advancer cues to find the fit; with a stopper it
/// prefers notrump (the jump ranks below the notrump ladder).
#[test]
fn advance_minor_jump_shows_invitational_one_suiter() {
    // (1♥) X (P) ? — advancer to act; the unbid major is spades.
    let auction = [call(1, Strain::Hearts), Call::Double, Call::Pass];
    super::advance_rich::set_rich_advance_double(true);
    super::advance_rich::set_advance_minor_jump(true);

    // 11 HCP, 5 diamonds, 3 spades (no 4-card major), no heart stopper: an
    // invitational one-suiter → jump 3♦.
    let one_suiter = "xxx.xx.AQJxx.KJx";
    let (jump, _) = best_call(&auction, one_suiter);
    // 11 HCP, 4 spades + 5 diamonds, no heart stopper: a 4-card unbid major →
    // cue 2♥ to find the fit, not the minor jump.
    let with_major = "KQxx.xx.AQxxx.xx";
    let (cued, _) = best_call(&auction, with_major);
    // 11 HCP, 5 diamonds, heart stopper, balanced: a minor needs eleven
    // tricks, so it prefers 2NT — the jump ranks below the notrump ladder.
    let stopped = "Qxx.KJx.AJxxx.xx";
    let (notrump, _) = best_call(&auction, stopped);

    super::advance_rich::set_advance_minor_jump(false);
    // Knob off: the same one-suiter has no minor jump, so it cues instead of
    // jumping.
    let (off, _) = best_call(&auction, one_suiter);
    super::advance_rich::set_advance_minor_jump(true); // restore default-on
    super::advance_rich::set_rich_advance_double(true); // restore rich default

    assert_eq!(
        jump,
        call(3, Strain::Diamonds),
        "invitational 5+ minor, no major → 3♦ jump"
    );
    assert_eq!(
        cued,
        call(2, Strain::Hearts),
        "a 4-card unbid major cues, not the minor jump"
    );
    assert_eq!(
        notrump,
        call(2, Strain::Notrump),
        "a stopper prefers notrump over the minor jump"
    );
    assert_ne!(off, call(3, Strain::Diamonds), "knob off → no minor jump");
}

/// The doubler's accept/decline of the minor jump, and the advancer's
/// placement over the doubler's forcing new suit: the limited invite lets the
/// doubler pass (unlike the forcing cue), accept a game-forcing 5+ suit, or
/// bid `3NT` to play — then the advancer raises the shown suit with support.
#[test]
fn doubler_accepts_or_declines_the_minor_jump() {
    // (1♠) X (P) 3♦ (P) ? — doubler acts over the invitational 3♦ jump; the
    // unbid major is hearts.
    let jump = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    super::advance_rich::set_rich_advance_double(true);
    super::advance_rich::set_advance_minor_jump(true);

    // Maximum with a 5-card heart suit: accept by showing it, game-forcing.
    let max_major = "x.AKQxx.Kxx.AQxx"; // 18 HCP, 5♥, no spade stopper
    let (accept, _) = best_call(&jump, max_major);
    // Balanced maximum with a spade stopper, no 5-card suit: 3NT to play.
    let max_flat = "KQx.AJx.Qxx.KQxx"; // 17 HCP, spade stopper
    let (notrump, _) = best_call(&jump, max_flat);
    // Minimum takeout double: decline the limited invitation.
    let minimum = "Kxxx.Qxx.xx.Kxxx"; // ~8 HCP, minimum
    let (decline, _) = best_call(&jump, minimum);

    // Advancer places game over the doubler's forcing 3♥: raise with support.
    let after_major = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let with_fit = "xx.Kxx.AQJxx.xxx"; // 10 HCP, 5♦, 3♥ support (a valid 3♦ jump)
    let (raise, _) = best_call(&after_major, with_fit);

    super::advance_rich::set_advance_minor_jump(true); // restore default-on
    super::advance_rich::set_rich_advance_double(true); // restore rich default

    assert_eq!(
        accept,
        call(3, Strain::Hearts),
        "maximum shows a new 5+ suit, game-forcing"
    );
    assert_eq!(
        notrump,
        call(3, Strain::Notrump),
        "balanced maximum accepts 3NT to play"
    );
    assert_eq!(decline, Call::Pass, "minimum declines the limited invite");
    assert_eq!(
        raise,
        call(4, Strain::Hearts),
        "advancer raises the doubler's shown major to game"
    );
}

/// The doubler's stopper-ask cue over the minor jump: with game values but no
/// stopper (and no biddable side suit) the doubler cues their suit; the
/// advancer bids `3NT` with a stopper (right-sided) or signs off in the minor
/// game.
#[test]
fn doubler_stopper_ask_over_the_minor_jump() {
    // (1♠) X (P) 3♣ (P) ? — doubler acts over the invitational 3♣ jump.
    let jump = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    super::advance_rich::set_rich_advance_double(true);
    super::advance_rich::set_advance_minor_jump(true);

    // 18 HCP, no spade stopper, no 5-card side suit: cue 3♠ to ask.
    let ask = "xxx.AKx.AQx.KQxx";
    let (cue, _) = best_call(&jump, ask);

    // (1♠) X (P) 3♣ (P) 3♠ (P) ? — advancer answers the stopper-ask.
    let after_ask = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    // A spade stopper: right-side the notrump game.
    let stopped = "Kx.xxx.xxx.AQJxx";
    let (notrump, _) = best_call(&after_ask, stopped);
    // No stopper anywhere: play the minor game.
    let no_stop = "xx.xxx.xxx.AKJxx";
    let (minor, _) = best_call(&after_ask, no_stop);

    super::advance_rich::set_advance_minor_jump(true); // restore default-on
    super::advance_rich::set_rich_advance_double(true); // restore rich default

    assert_eq!(
        cue,
        call(3, Strain::Spades),
        "game values without a stopper cue the stopper-ask"
    );
    assert_eq!(
        notrump,
        call(3, Strain::Notrump),
        "the advancer right-sides 3NT with a stopper"
    );
    assert_eq!(
        minor,
        call(5, Strain::Clubs),
        "without a stopper the advancer signs off in the minor game"
    );
}

/// The doubler answers the advancer's invitational `2NT` naturally — declines
/// with a minimum, accepts to play with a balanced maximum, or shows a 5-card
/// major game-forcing — instead of the floor passing a game.
#[test]
fn doubler_accepts_or_declines_the_2nt_invite() {
    // (1♠) X (P) 2NT (P) ? — doubler acts over the invitational 2NT; the
    // unbid major is hearts.
    let invite = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    super::advance_rich::set_rich_advance_double(true);
    super::advance_rich::set_advance_2nt_continuation(true);

    // Maximum with a 5-card heart suit: accept by showing it, game-forcing.
    let max_major = "x.AKQxx.Kxx.AQxx"; // 18 HCP, 5♥
    let (accept, _) = best_call(&invite, max_major);
    // Balanced maximum, no 5-card major: 3NT to play.
    let max_flat = "KQx.AJx.Qxx.KQxx"; // 17 HCP, balanced
    let (notrump, _) = best_call(&invite, max_flat);
    // Minimum takeout double: decline the limited invite, pass 2NT.
    let minimum = "KQxx.Qxx.xx.KQxx"; // 12 HCP, minimum
    let (decline, _) = best_call(&invite, minimum);

    // Advancer places game over the doubler's forcing 3♥: raise with support.
    let after_major = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let with_fit = "Axx.Kxx.Qxx.QJxx"; // 12 HCP, 3♥ support, spade stopper
    let (raise, _) = best_call(&after_major, with_fit);

    super::advance_rich::set_advance_2nt_continuation(true); // restore default
    super::advance_rich::set_rich_advance_double(true); // restore rich default

    assert_eq!(
        accept,
        call(3, Strain::Hearts),
        "maximum shows a 5-card major, game-forcing"
    );
    assert_eq!(
        notrump,
        call(3, Strain::Notrump),
        "balanced maximum accepts 3NT to play"
    );
    assert_eq!(decline, Call::Pass, "minimum declines the limited invite");
    assert_eq!(
        raise,
        call(4, Strain::Hearts),
        "advancer raises the doubler's shown major to game"
    );
}

/// The Rubens layer: a 5+ unbid major transfers via the rank below it, and
/// the doubler completes by declaring that major — over every opening where
/// the transfer is a genuine jump-cue (`1♣`/`1♦`/`1♥`).
#[test]
fn rubens_transfer_completes_into_the_major() {
    super::advance_rich::set_rich_advance_double(true);
    super::advance_rich::set_advance_rubens(true);

    // Advancer with 5 spades, 10 HCP: transfer via 3♥ (the rank below spades)
    // over each opening; the doubler completes to spades and declares.
    let advancer = "KQJ42.xx.KJx.xxx"; // 5 spades, 10 HCP
    for open in [Strain::Clubs, Strain::Diamonds, Strain::Hearts] {
        let start = [call(1, open), Call::Double, Call::Pass];
        let (xfer, _) = best_call(&start, advancer);
        assert_eq!(
            xfer,
            call(3, Strain::Hearts),
            "5-spade INV+ transfers via 3♥ over (1{open:?})"
        );
        let after = [
            call(1, open),
            Call::Double,
            Call::Pass,
            call(3, Strain::Hearts),
            Call::Pass,
        ];
        let (complete, floored) = best_call(&after, "AKx.xxx.Axxx.xxx");
        assert_eq!(
            complete,
            call(3, Strain::Spades),
            "doubler completes the transfer into spades over (1{open:?})"
        );
        assert!(
            !floored,
            "the completion must come from the book, not the floor"
        );
    }

    super::advance_rich::set_advance_rubens(false);
    super::advance_rich::set_rich_advance_double(true); // restore default
}
