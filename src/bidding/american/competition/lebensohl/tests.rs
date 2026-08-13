use super::super::tests::{bid, bid_landy, bid_landy_cues, bid_transfer, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn lebensohl_forcing_three_level_is_a_book_node() {
    // 1NT (2♦); responder 5 spades, game values, no diamond stopper →
    // forcing 3♠ (a jump), not a partscore.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid(&auction, "KQT95.A43.32.J32");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(!floored, "the forcing 3-level bid must come from the book");
}

#[test]
fn lebensohl_weak_long_suit_relays_then_completes() {
    // Weak hand (6 HCP), 6 clubs, over 2♦ → 2NT relay; opener forced to 3♣.
    let responder = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid(&responder, "J2.43.32.KQ9876");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the Lebensohl relay must come from the book");

    let opener = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (completion, _) = bid(&opener, "AQ32.KQ5.AQ4.A32");
    assert_eq!(completion, call(3, Strain::Clubs));
}

#[test]
fn lebensohl_weak_bids_natural_two_level() {
    // A weak hand with 5 hearts bids natural 2♥ (below 2NT), to play.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid(&auction, "K2.QJ976.432.432");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the natural 2-level bid must come from the book");
}

#[test]
fn lebensohl_cue_is_stayman() {
    // 1NT (2♥): a game-force with 4 spades and no 5-card suit cues 3♥ = Stayman
    // (it cannot bid a forcing 3-level suit, and the cue outranks direct 3NT).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let (c, floored) = bid(&auction, "AQ32.K43.A32.K32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the cue must come from the book");

    // Opener answers Stayman with the 4-card spade fit.
    let opener = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let (a, floored) = bid(&opener, "KJ54.A32.K43.Q32");
    assert_eq!(a, call(3, Strain::Spades));
    assert!(!floored, "the Stayman answer must come from the book");
}

#[test]
fn lebensohl_five_card_suit_relays_then_signs_off_at_the_three_level() {
    // Weak hand, a 5-card heart suit it cannot show at the 2 level (below
    // their 2♠): relay 2NT, then correct 3♣→3♥ as a 3-level sign-off.
    let responder = [call(1, Strain::Notrump), call(2, Strain::Spades)];
    let (c, floored) = bid(&responder, "32.KQJ32.432.432");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the relay must come from the book");

    let after_3c = [
        call(1, Strain::Notrump),
        call(2, Strain::Spades),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = bid(&after_3c, "32.KQJ32.432.432");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the 3-level sign-off must come from the book");
}

#[test]
fn lebensohl_maximum_raises_weak_signoff_to_game() {
    // 1NT (2♠) 2NT - 3♣ - 3♥ -: responder's weak heart sign-off. A maximum
    // (17) opener with three-card support stretches to 4♥; a minimum passes.
    let after_signoff = [
        call(1, Strain::Notrump),
        call(2, Strain::Spades),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    // 4-3-4-2, 17: a flat 4-3-3-3 17-count would read 16 on the shipped
    // rule-of-N+8 scale and rightly decline the stretch.
    let (c, floored) = bid(&after_signoff, "AK32.K43.A432.K3");
    assert_eq!(c, call(4, Strain::Hearts));
    assert!(!floored, "the game raise must come from the book");

    let (c, _) = bid(&after_signoff, "AK32.K43.KQ3.432");
    assert_eq!(c, Call::Pass, "a minimum passes the weak sign-off");
}

#[test]
fn landy_counter_replaces_the_stolen_stayman_with_a_values_double() {
    // 1NT (2♣ Landy = both majors).  Systems-on would classify this hand's X as
    // the stolen 2♣ Stayman — asking for a four-card major against a hand that
    // has shown both.  The counter makes it a values double instead.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    let (c, floored) = bid_landy(&auction, "KQ32.43.KJ32.432");
    assert_eq!(c, Call::Double);
    assert!(!floored, "the values double must come from the book");

    // And opener sits for it — the double is values, not a question.  Under
    // systems-on this node answers Stayman and bids a phantom major.
    let after_double = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        Call::Pass,
    ];
    let (c, _) = bid_landy(&after_double, "A54.AQ4.AQ54.K32");
    assert_eq!(c, Call::Pass, "opener leaves the values double in");
}

#[test]
fn landy_counter_bids_naturally_in_the_suits_they_did_not_show() {
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];

    // Weak with long diamonds: natural 2♦, the one suit below their majors.
    // Systems-on reads this bid as a Jacoby transfer to hearts — one of the two
    // suits they just showed.
    let (c, floored) = bid_landy(&auction, "32.43.KQJ876.432");
    assert_eq!(c, call(2, Strain::Diamonds));
    assert!(!floored, "the natural escape must come from the book");

    // Game values with long clubs: natural forcing 3♣ (their 2♣ was artificial).
    let (c, floored) = bid_landy(&auction, "32.43.A32.AKJ876");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the forcing minor must come from the book");
}

#[test]
fn landy_cues_name_the_unshown_minors_game_forcing() {
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];

    // Game values with five clubs: cue their hearts.  Without the overlay this
    // hand has no call between a stopperless 3NT and a stretched values X.
    let (c, floored) = bid_landy_cues(&auction, "32.A3.K32.AQJ54");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the GF club cue must come from the book");

    // Game values with five diamonds: cue their spades.
    let (c, floored) = bid_landy_cues(&auction, "32.A3.AQJ54.K32");
    assert_eq!(c, call(2, Strain::Spades));
    assert!(!floored, "the GF diamond cue must come from the book");

    // The cues carry the six-carders too: with a GF cue below it, a forcing 3m
    // would be redundant, so the skeleton routes every GF one-suiter through
    // the cue and frees the direct 3m for the weak escape.
    let (c, _) = bid_landy_cues(&auction, "32.43.A32.AKJ876");
    assert_eq!(c, call(2, Strain::Hearts));

    // The freed direct 3♣: a natural weak escape, as in Michaels' UvU twin.
    let (c, floored) = bid_landy_cues(&auction, "32.43.432.QJ8765");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the weak escape must come from the book");

    // Under the base counter the same weak hand has no call — 3m is forcing.
    let (c, _) = bid_landy(&auction, "32.43.432.QJ8765");
    assert_eq!(c, Call::Pass);

    // Purity of the addition: the base counter sends the 5-card hand to 3NT.
    let (c, _) = bid_landy(&auction, "32.A3.K32.AQJ54");
    assert_eq!(c, call(3, Strain::Notrump));
}

#[test]
fn landy_cue_answer_places_the_game() {
    // 1NT (2♣) 2♥ - : the cue is GF with 5+ clubs; opener describes.
    let after_cue = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Pass,
    ];

    // Both majors stopped: 2NT keeps 3NT live from the strong side.
    let (c, floored) = bid_landy_cues(&after_cue, "A54.KQ4.A954.K32");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the cue answer must come from the book");

    // No spade stopper: raise the named minor instead.
    let (c, _) = bid_landy_cues(&after_cue, "543.KQ4.AQ54.KQ3");
    assert_eq!(c, call(3, Strain::Clubs));
}

#[test]
fn landy_natural_answers_stop_the_phantom_completions() {
    // Opener's answers over the counter's natural calls.  Left to the floor —
    // which cannot see the counter's regime — each of these was completed as
    // the default-system gadget it replaced (audit, ab-results/landy-counter).
    let base = [call(1, Strain::Notrump), call(2, Strain::Clubs)];

    // 2♦ is a weak sign-off: opener passes, even a maximum.  The floor bid the
    // phantom Jacoby 2♥ here on 82% of the audited boards.
    let after = [base[0], base[1], call(2, Strain::Diamonds), Call::Pass];
    let (c, floored) = bid_landy(&after, "A54.AQ4.A954.K32");
    assert_eq!(c, Call::Pass, "a minor sign-off is never raised");
    assert!(!floored, "the sign-off answer must come from the book");

    // 2NT is the natural invite: decline on 15, accept on 16 — the same
    // size_ask_accept_floor as the uncontested invite.
    let after = [base[0], base[1], call(2, Strain::Notrump), Call::Pass];
    let (c, floored) = bid_landy(&after, "A54.KQ4.A954.Q32");
    assert_eq!(c, Call::Pass, "a minimum declines the invite");
    assert!(!floored, "the invite answer must come from the book");
    let (c, _) = bid_landy(&after, "A54.KQ4.A954.K32");
    assert_eq!(c, call(3, Strain::Notrump), "a maximum accepts the invite");

    // Base-arm 3♣ is forcing with a six-card suit: 3NT with both of their
    // majors stopped, else raise.  The floor answered phantom Puppet here.
    let after = [base[0], base[1], call(3, Strain::Clubs), Call::Pass];
    let (c, floored) = bid_landy(&after, "A54.KQ4.A954.K32");
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(!floored, "the forcing-minor answer must come from the book");
    let (c, _) = bid_landy(&after, "543.KQ4.AQ54.KQ3");
    assert_eq!(c, call(4, Strain::Clubs), "no spade stopper: raise instead");

    // Under the cues, the direct 3♣ is the weak escape — opener sits.
    let (c, floored) = bid_landy_cues(&after, "A54.AQ4.A954.K32");
    assert_eq!(c, Call::Pass, "a weak escape is never raised");
    assert!(!floored, "the weak-escape answer must come from the book");
}

#[test]
fn landy_counter_is_inert_when_the_knob_is_off() {
    // The default arm keeps systems-on: the same values hand doubles as the
    // stolen Stayman, and opener answers it.  This is the byte-identity guard —
    // if it ever fails, the knob has stopped being opt-in.
    let after_double = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        Call::Pass,
    ];
    let (c, _) = bid_transfer(&after_double, "A54.AQ4.AQ54.K32");
    assert_ne!(
        c,
        Call::Pass,
        "systems-on answers the stolen Stayman; only the Landy arm sits"
    );
}
