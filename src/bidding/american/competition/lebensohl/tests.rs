use super::super::tests::{
    best_call_with, bid, bid_landy, bid_landy_bba, bid_landy_cues, bid_landy_lia, bid_landy_n1,
    bid_landy_n1p, bid_landy_transfer, bid_transfer, call,
};
use crate::bidding::agreements::Agreements;
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
fn landy_notrump_no_major_doubles_the_four_card_major() {
    // 1NT (2♣ Landy).  Eleven points with four spades — one of the suits they
    // showed, so their fit is at best 4-3 and defending beats declaring.  The
    // shipped table buries the hand in the ungated `3NT`@168, which is what
    // caps the values double at nine points.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    let hand = "KQ32.K54.J432.Q3";
    let (c, floored) = bid_landy_bba(true, &auction, hand);
    assert_eq!(c, call(3, Strain::Notrump), "the default arm declares");
    assert!(!floored, "and it comes from the book");

    // §N1p: `3NT` denies a four-card major, so the same hand reaches the `X`.
    let (c, floored) = bid_landy_n1p(false, &auction, hand);
    assert_eq!(c, Call::Double);
    assert!(
        !floored,
        "the widened values double must come from the book"
    );

    // Three small in each major still declares — the restriction is length,
    // not values, and short stoppers stay welcome.
    let (c, _) = bid_landy_n1p(false, &auction, "K32.K54.QJ32.Q32");
    assert_eq!(c, call(3, Strain::Notrump));
}

#[test]
fn landy_major_jam_bids_game_on_a_six_card_major() {
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    let hand = "43.KQJ432.KQ3.42";

    // Without the jam a six-card major is just more length in a suit they
    // showed, so §N1p routes it to the double like any other.
    let (c, _) = bid_landy_n1p(false, &auction, hand);
    assert_eq!(c, Call::Double);

    // With it, the strong six-carder takes the game and jams the auction.
    let (c, floored) = bid_landy_n1p(true, &auction, hand);
    assert_eq!(c, call(4, Strain::Hearts));
    assert!(!floored, "the jam must come from the book");

    // A weak six-carder keeps defending — the rung is `points(10..)`.
    let (c, _) = bid_landy_n1p(true, &auction, "43.J98432.Q43.42");
    assert_ne!(c, call(4, Strain::Hearts), "the jam is for strong hands");

    // And opener sits: the jam is a sign-off, not a slam try.
    let after_jam = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(4, Strain::Hearts),
        Call::Pass,
    ];
    let (c, _) = bid_landy_n1p(true, &after_jam, "A54.A54.AQ54.K32");
    assert_eq!(c, Call::Pass, "opener passes the jam");
}

/// The jam rung standalone, which is the arm §N1p never ran
///
/// `landy_major_jam` used to be conjoined with `landy_notrump_no_major`, so the
/// only measurement of it — §N1p's `jam vs nt` pair — priced `4M` against the
/// **double**, because a gated `3NT`@168 denies four-plus of a major and six of
/// one is four-plus.  Alone, `3NT`@168 is ungated and `4M`@171 outranks it, so
/// the rung substitutes for the **game** instead.  Different experiment, and
/// the +5.541 IMPs/fired does not transfer.
#[test]
fn landy_major_jam_alone_replaces_the_notrump_not_the_double() {
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    let hand = "43.KQJ432.KQ3.42";

    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    // The rung under test is the *direct* `4M`; package C's transfer, shipped
    // default-on since 2026-08-31, carries the same hand one call lower and is
    // measured on its own (`landy_texas_reroutes_the_four_level`).
    arm.competition.landy_texas = false;

    // Neither knob: the ungated `3NT`@168 takes the six-card major. This is
    // what the standalone jam displaces — not the `X` §N1p measured against.
    arm.competition.landy_major_jam = false;
    let (c, _) = best_call_with(&arm, &auction, hand);
    assert_eq!(c, call(3, Strain::Notrump));

    // The jam alone, with `3NT` left ungated: `4♥`@171 outranks 168.
    arm.competition.landy_major_jam = true;
    let (c, floored) = best_call_with(&arm, &auction, hand);
    assert_eq!(c, call(4, Strain::Hearts));
    assert!(!floored, "the jam must come from the book");

    // Its sit node is no longer gated on `landy_notrump_no_major` either.
    let after_jam = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(4, Strain::Hearts),
        Call::Pass,
    ];
    let (c, _) = best_call_with(&arm, &after_jam, "A54.A54.AQ54.K32");
    assert_eq!(c, Call::Pass, "opener sits without the notrump gate");
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

    // Both majors stopped and a maximum: accept, and place the game.  A
    // sub-game answer here let the auction die in 3NT where the base arm's
    // game-level answer reaches 6♣/6♦ (see `landy_minor_answer`).
    let (c, floored) = bid_landy_cues(&after_cue, "A54.KQ4.A954.K32");
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(!floored, "the cue answer must come from the book");

    // Both majors stopped, minimum: 2NT, and responder places it — the cue is
    // invitational-or-better, so opener may decline.
    let (c, _) = bid_landy_cues(&after_cue, "A54.KQ4.AJ54.J32");
    assert_eq!(c, call(2, Strain::Notrump));

    // Only hearts stopped, with club tolerance — the ask names the major
    // opener *lacks*, and its LEVEL carries opener's strength.  Maximum (16):
    // the 3-level cue.
    let (c, _) = bid_landy_cues(&after_cue, "543.KQ4.AQ54.KQ3");
    assert_eq!(c, call(3, Strain::Spades));

    // Same shape, minimum (13): the club cue leaves `2♠` below the 3-level, so
    // a minimum can still ask.  Over the diamond cue there is no such rung.
    let (c, _) = bid_landy_cues(&after_cue, "543.KQ4.AJ54.K32");
    assert_eq!(c, call(2, Strain::Spades));
}

#[test]
fn landy_re_cue_resolves_the_stopper_over_openers_minimum() {
    // 1NT (2♣) 2♥ - 3♣ - : opener's 3♣ is the one minimum showing NO stopper,
    // so responder's own worry can still be asked here.
    let after_minimum = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
    ];

    // Game force with hearts stopped and spades wide open: cue the major we
    // lack.  (With both stopped responder bids 3NT itself.)
    let (c, floored) = bid_landy_cues(&after_minimum, "432.A5.KQ4.AQ765");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(!floored, "the re-cue must come from the book");

    // Opener holding the asked stopper bids the game...
    let mut after_recue = after_minimum.to_vec();
    after_recue.push(call(3, Strain::Spades));
    after_recue.push(Call::Pass);
    let (c, _) = bid_landy_cues(&after_recue, "AQ4.KQ4.J954.K32");
    assert_eq!(c, call(3, Strain::Notrump));

    // ...and without it takes the minor its own 3♣ already promised.
    let (c, _) = bid_landy_cues(&after_recue, "432.KQ4.AJ54.KQ3");
    assert_eq!(c, call(4, Strain::Clubs));
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
fn landy_transfer_re_rungs_the_minors_around_a_club_transfer() {
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];

    // Weak with six clubs — N1b's biggest earner, moved a level down and
    // right-sided: 2NT transfers, opener declares 3♣.
    let (c, floored) = bid_landy_transfer(&auction, "32.43.432.QJ8765");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the club transfer must come from the book");

    let after_transfer = [auction[0], auction[1], call(2, Strain::Notrump), Call::Pass];
    let (c, floored) = bid_landy_transfer(&after_transfer, "A54.KQ4.A954.K32");
    assert_eq!(c, call(3, Strain::Clubs), "the transfer is forced");
    assert!(!floored, "the completion must come from the book");

    // ...and responder, who has already said everything, passes.
    let mut after_completion = after_transfer.to_vec();
    after_completion.push(call(3, Strain::Clubs));
    after_completion.push(Call::Pass);
    let (c, floored) = bid_landy_transfer(&after_completion, "32.43.432.QJ8765");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the transfer sign-off must come from the book");

    // Weak with six diamonds still bids the weak 2♦ below — N1b's weak 3♦ was
    // redundant with it, and measured negative trying to duplicate it.
    let (c, _) = bid_landy_transfer(&auction, "32.43.KQJ876.432");
    assert_eq!(c, call(2, Strain::Diamonds));

    // The freed direct 3m: invitational with a six-card suit.
    let (c, floored) = bid_landy_transfer(&auction, "32.43.AQJ876.432");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the invitational minor must come from the book");

    // The game-forcing six-carder still cues — 3m is capped at the invitation.
    let (c, _) = bid_landy_transfer(&auction, "32.43.A32.AKJ876");
    assert_eq!(c, call(2, Strain::Hearts));

    // Opener answers the invitation with the same size decision as any other:
    // 3NT from the top of the range with both of their majors stopped...
    let after_invite = [
        auction[0],
        auction[1],
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = bid_landy_transfer(&after_invite, "A54.KQ4.A954.K32");
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(!floored, "the invite answer must come from the book");

    // ...else sit for the partscore, since minor game is out of reach.
    let (c, _) = bid_landy_transfer(&after_invite, "A54.KQ4.A954.Q32");
    assert_eq!(c, Call::Pass, "a minimum declines the invitation");
    let (c, _) = bid_landy_transfer(&after_invite, "543.KQ4.AQ54.KQ3");
    assert_eq!(c, Call::Pass, "no spade stopper: 3NT is not on offer");
}

#[test]
fn landy_cue_gets_a_slam_try_over_openers_minimums() {
    // 1NT (2♣) 2♥ - 3♣ - : opener's minimum with no stopper.  A six-card source
    // of tricks opposite 15-17 is worth more than the 3NT that is not even on;
    // 4♣ leaves a suit contract the floor can cue-bid over.
    let after_minimum = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = bid_landy_transfer(&after_minimum, "3.A5.AK4.AQJ8765");
    assert_eq!(c, call(4, Strain::Clubs));
    assert!(!floored, "the slam try must come from the book");

    // N1b had no rung for it and settled for the game.
    let (c, _) = bid_landy_cues(&after_minimum, "3.A5.AK4.AQJ8765");
    assert_ne!(c, call(4, Strain::Clubs), "the slam try is N1c only");

    // Over opener's minimum 2NT (both stoppers) the same hand tries too, rather
    // than signing off in the 3NT that ranks below it.
    let after_notrump = [
        after_minimum[0],
        after_minimum[1],
        after_minimum[2],
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (c, floored) = bid_landy_transfer(&after_notrump, "3.A5.AK4.AQJ8765");
    assert_eq!(c, call(4, Strain::Clubs));
    assert!(!floored, "the slam try must come from the book");

    // A plain game force with no extras still bids the game.
    let (c, _) = bid_landy_transfer(&after_notrump, "432.A5.K54.AQJ76");
    assert_eq!(c, call(3, Strain::Notrump));
}

#[test]
fn landy_cue_floor_returns_the_eight_count_to_the_values_double() {
    // 8 hcp, five clubs, ~9 total points: under N1c the cue's points(8..)
    // floor takes it at weight 173 over the double's 145 — the poached-double
    // rows the per-bid decomposition priced at −0.92/−2.53 PD per fired.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    let hand = "43.432.J32.AK432";
    let (c, _) = bid_landy_transfer(&auction, hand);
    assert_eq!(
        c,
        call(2, Strain::Hearts),
        "N1c: the points(8..) cue takes it"
    );
    let (c, floored) = bid_landy_n1(true, false, false, &auction, hand);
    assert_eq!(c, Call::Double, "N1d: it defends instead");
    assert!(!floored, "the values double must come from the book");

    // The ten-count still cues — the floor trims the bottom, nothing else.
    let (c, _) = bid_landy_n1(true, false, false, &auction, "43.432.Q32.AKJ32");
    assert_eq!(c, call(2, Strain::Hearts));
}

#[test]
fn landy_low_minors_prices_every_minor_rung_a_point_lower() {
    // The N1h arm: the shipped stack, plus the point off each minor rung.
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.defense_2c_landy_low_minors = true;
    arm.competition.defense_2c_landy_bba = false; // the stack arm, not N1j
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    let low = |hand| best_call_with(&arm, &auction, hand);
    let shipped = |hand| bid_landy_n1(true, true, true, &auction, hand);

    // 9 points, five clubs: N1d's floor(10) defends, N1h's floor(9) cues.
    // This is the slice N1h deliberately takes back off the values double.
    let hand = "43.432.Q32.AK432";
    assert_eq!(shipped(hand).0, Call::Double, "N1d: it defends");
    let (c, floored) = low(hand);
    assert_eq!(c, call(2, Strain::Hearts), "N1h: the cue reaches it");
    assert!(!floored, "the cue must come from the book");

    // The eight-count is still below both floors — one point, not two.
    assert_eq!(low("43.432.J32.AK432").0, Call::Double);

    // 7 points, six diamonds: the weak 2♦ under the shipped band, the
    // invitational 3♦ once the band drops to 7-8.
    let hand = "32.43.KQJ876.432";
    assert_eq!(shipped(hand).0, call(2, Strain::Diamonds));
    let (c, floored) = low(hand);
    assert_eq!(
        c,
        call(3, Strain::Diamonds),
        "N1h: the invitation reaches it"
    );
    assert!(!floored, "the invitational minor must come from the book");

    // 9 points, six diamonds: the band shifts whole, so the top of the old
    // rung falls through to the cue rather than overlapping it.
    let hand = "32.43.AKJ876.432";
    assert_eq!(shipped(hand).0, call(3, Strain::Diamonds));
    assert_eq!(low(hand).0, call(2, Strain::Spades), "N1h: 3♦ caps at 8");

    // The eight-count six-carder is inside both bands and does not move.
    let hand = "32.43.AQJ876.432";
    assert_eq!(shipped(hand).0, call(3, Strain::Diamonds));
    assert_eq!(low(hand).0, call(3, Strain::Diamonds));
}

#[test]
fn landy_hcp_rungs_regrade_the_minors_on_high_cards() {
    // The N1i arm: the shipped stack, with the minors cut on `hcp` into three
    // non-overlapping bands — cue 9+, 3m INV 7-8, weak 2♦/2NT 0-6.
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.defense_2c_landy_hcp_rungs = true;
    arm.competition.defense_2c_landy_bba = false; // the stack arm, not N1j
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    let hcp_arm = |hand| best_call_with(&arm, &auction, hand);
    let shipped = |hand| bid_landy_n1(true, true, true, &auction, hand);

    // 9 hcp, five clubs: the cue's own band, where `points` needed 10.
    let hand = "43.432.Q32.AK432";
    assert_eq!(shipped(hand).0, Call::Double, "points(10..): it defends");
    let (c, floored) = hcp_arm(hand);
    assert_eq!(c, call(2, Strain::Hearts), "hcp(9..): the cue takes it");
    assert!(!floored, "the cue must come from the book");

    // 8 hcp is below the cue and lands on the values double, which floors on
    // the same scale — the two rungs now partition instead of overlapping.
    assert_eq!(hcp_arm("43.432.J32.AK432").0, Call::Double);

    // 7 hcp, six diamonds: the invitational rung.  Under `points` the
    // unbalanced upgrade made this hand an 8 and it still bid 3♦; under
    // `hcp` the shape no longer votes.
    let hand = "32.43.AQJ876.432";
    assert_eq!(shipped(hand).0, call(3, Strain::Diamonds));
    let (c, floored) = hcp_arm(hand);
    assert_eq!(c, call(3, Strain::Diamonds), "7 hcp is inside 7-8");
    assert!(!floored, "the invitational minor must come from the book");

    // 6 hcp, six diamonds: below the invitation, so the weak 2♦ — and the
    // 2NT club transfer keeps its own 0-6 band.
    assert_eq!(hcp_arm("32.43.KQJ876.432").0, call(2, Strain::Diamonds));
    assert_eq!(hcp_arm("32.43.432.QJ8765").0, call(2, Strain::Notrump));

    // The known hole, pinned so it cannot be lost silently: 7 hcp with a
    // *five*-card diamond suit clears neither the 2♦ ceiling (6) nor the
    // double's floor (8), so it passes where `points` bid 2♦.
    let hand = "432.432.AK432.43";
    assert_eq!(shipped(hand).0, call(2, Strain::Diamonds));
    assert_eq!(hcp_arm(hand).0, Call::Pass, "N1i's deliberate hole");
}

#[test]
fn landy_fit_answers_offer_notrump_on_a_doubleton() {
    // 1NT (2♣) 2♥ - holding two clubs and an unstopped major: the base
    // table's weight-20 catch-all raises to 3♣ on the 5-2 — the fit
    // forensic's −10.0/−8.2 PD per fired — where N1e answers notrump at the
    // strength level, so the raises and asks come to promise 3+.
    let after_cue = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Pass,
    ];

    // Minimum (15), hearts unstopped, doubleton club.
    let hand = "AQ32.432.AK42.Q2";
    let (c, _) = bid_landy_transfer(&after_cue, hand);
    assert_eq!(
        c,
        call(3, Strain::Clubs),
        "the base catch-all raises on two"
    );
    let (c, floored) = bid_landy_n1(false, true, false, &after_cue, hand);
    assert_eq!(
        c,
        call(2, Strain::Notrump),
        "N1e: notrump instead of the 5-2"
    );
    assert!(!floored, "the doubleton answer must come from the book");

    // Maximum (17), same shape: the level still carries the strength.
    let (c, _) = bid_landy_n1(false, true, false, &after_cue, "AQ32.432.AKQ2.Q2");
    assert_eq!(c, call(3, Strain::Notrump));

    // With three-card support nothing moves: the raise still promises a fit.
    let (c, _) = bid_landy_n1(false, true, false, &after_cue, "AQ32.432.AK4.Q32");
    assert_eq!(c, call(3, Strain::Clubs));
}

#[test]
fn landy_competition_answers_the_doubled_cue_as_if_undoubled() {
    // 1NT (2♣) 2♥ (X): their X takes no room.  Without N1f every registered
    // suffix ends in `-`, so the whole node is the floor's — the priced
    // interference hole.
    let doubled_cue = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Double,
    ];
    let hand = "AQ32.KQ2.A432.32"; // both majors stopped, minimum
    let (_, floored) = bid_landy_transfer(&doubled_cue, hand);
    assert!(floored, "without N1f the doubled cue drops to the floor");
    let (c, floored) = bid_landy_n1(false, false, true, &doubled_cue, hand);
    assert_eq!(c, call(2, Strain::Notrump), "the clean ladder, verbatim");
    assert!(!floored, "the doubled-cue answer must come from the book");

    // And the systems-on rebase carries the whole subtree: deeper rungs
    // answer as if the X had never happened — here the re-cue's 3NT with the
    // asked stopper (compare `landy_re_cue_resolves_the_stopper…`).
    let deep = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Double,
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let (c, floored) = bid_landy_n1(false, false, true, &deep, "AQ4.KQ4.J954.K32");
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(!floored, "the rebase must reach the re-cue answer");
}

#[test]
fn landy_competition_answers_the_raise_over_the_cue() {
    // 1NT (2♣) 2♥ (2♠): the advancer's raise — 28 of the 47 priced
    // interference boards.  The compressed ladder keeps what Pass cannot say:
    // game with both of their majors stopped, and the fit, by size.  Pass is
    // safe — responder is INV+ and guaranteed another turn — where the floor
    // was bidding 3-5 card majors at the four level (−14…−18 PD).
    let raised = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        call(2, Strain::Spades),
    ];
    let (c, floored) = bid_landy_n1(false, false, true, &raised, "AQ32.KQ2.AK42.32");
    assert_eq!(c, call(3, Strain::Notrump), "both stopped, maximum: game");
    assert!(!floored, "the raise answer must come from the book");
    let (c, _) = bid_landy_n1(false, false, true, &raised, "A432.432.AKJ.QJ32");
    assert_eq!(c, call(3, Strain::Clubs), "the fit, cheaply, on a minimum");
    let (c, _) = bid_landy_n1(false, false, true, &raised, "AQ32.432.AKQ2.32");
    assert_eq!(c, Call::Pass, "nothing to say: responder places it");

    // Their jump raise leaves no 3-level raise, so the fit answer folds to a
    // maximum 4♣.
    let jumped = [raised[0], raised[1], raised[2], call(3, Strain::Spades)];
    let (c, floored) = bid_landy_n1(false, false, true, &jumped, "A432.432.A2.AKQ32");
    assert_eq!(c, call(4, Strain::Clubs));
    assert!(!floored, "the jump-raise answer must come from the book");
}

#[test]
fn landy_competition_rescues_the_doubled_ask() {
    // 1NT (2♣) 2♥ - 3♠ (X): the nine-board defect — every registration ended
    // in `-`, so the floor passed the doubled ask out and we played their
    // major doubled.  Responder answers the ask exactly as if undoubled.
    let doubled_ask = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Double,
    ];
    let hand = "A2.32.J32.AQJ432";
    let (_, floored) = bid_landy_transfer(&doubled_ask, hand);
    assert!(floored, "without N1f the doubled ask is the floor's");
    let (c, floored) = bid_landy_n1(false, false, true, &doubled_ask, hand);
    assert_eq!(
        c,
        call(3, Strain::Notrump),
        "the asked stopper, as if undoubled"
    );
    assert!(!floored, "the doubled-ask answer must come from the book");
}

#[test]
fn landy_competition_completes_the_doubled_transfer() {
    // 1NT (2♣) 2NT (X): the transfer is alerted and owed its tail on the
    // meta-rule — opener completes anyway, and responder still signs off.
    let doubled_transfer = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Notrump),
        Call::Double,
    ];
    let opener = "A54.KQ4.A954.K32";
    let (_, floored) = bid_landy_transfer(&doubled_transfer, opener);
    assert!(floored, "without N1f the doubled transfer is the floor's");
    let (c, floored) = bid_landy_n1(false, false, true, &doubled_transfer, opener);
    assert_eq!(c, call(3, Strain::Clubs), "the completion is still forced");
    assert!(!floored, "the completion must come from the book");
}

#[test]
fn landy_declaration_engages_the_default_counter() {
    // The 2026-08-15 default flip: a bare declaration — no knobs touched —
    // now plays the N1j BBA ladder (which superseded the 2026-08-14 stack
    // default).  The three probes hold across both flips: each hand's call
    // is the same under stack and ladder, so this test watches the
    // declaration's engagement, not the arm choice.
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;

    // The weak six-card club hand transfers (N1c's rung; the ladder widens
    // its band without moving this hand).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    let (c, _) = best_call_with(&arm, &auction, "32.43.432.QJ8765");
    assert_eq!(c, call(2, Strain::Notrump));

    // The 8-count with five clubs defends — the stack's N1d floor kept it
    // off the cue; the ladder has no cue at all.
    let (c, _) = best_call_with(&arm, &auction, "43.432.J32.AK432");
    assert_eq!(c, Call::Double);

    // The doubled 2♥ (stack: the club cue; ladder: the GF takeout) is
    // answered 2NT from the book either way, not floored.
    let doubled_cue = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Double,
    ];
    let (c, floored) = best_call_with(&arm, &doubled_cue, "AQ32.KQ2.A432.32");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the default counter answers the doubled call");
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

#[test]
fn landy_bba_ladder_routes_the_both_minor_family() {
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    // 2=2=4=5 game force: the takeout names the doubleton, and 2-2 bids 2♥.
    let (c, floored) = bid_landy_bba(false, &auction, "A3.43.KQ32.AJ432");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the takeout must come from the book");
    // 2=3=4=4 — the spade doubleton with three hearts — is the whole of 2♠.
    let (c, _) = bid_landy_bba(false, &auction, "43.K43.KQ32.A432");
    assert_eq!(c, call(2, Strain::Spades));
    // 0-1 in a major splinters instead, even holding a doubleton in the
    // other, and even holding a six-card minor the transfer would take.
    let (c, _) = bid_landy_bba(false, &auction, "4.K432.KQ32.A432");
    assert_eq!(c, call(3, Strain::Spades));
    let (c, _) = bid_landy_bba(false, &auction, "K432.4.KQ32.A432");
    assert_eq!(c, call(3, Strain::Hearts));
    let (c, _) = bid_landy_bba(false, &auction, "A3.4.KQ32.AJ5432");
    assert_eq!(c, call(3, Strain::Hearts));
}

#[test]
fn landy_bba_wide_transfers_carry_every_one_suiter() {
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    // The weak six-card club escape still transfers (the N1c earner) …
    let (c, floored) = bid_landy_bba(false, &auction, "32.43.432.QJ8765");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the transfer must come from the book");
    // … and the game force rides the *same* call — the one-minor cues and the
    // INV 3♣/3♦ rungs are gone.
    let (c, _) = bid_landy_bba(false, &auction, "32.A43.K2.AQJ876");
    assert_eq!(c, call(2, Strain::Notrump));
    // Six diamonds → the 3♣ transfer, BBA's own slot.
    let (c, _) = bid_landy_bba(false, &auction, "32.A43.AQJ876.K2");
    assert_eq!(c, call(3, Strain::Clubs));
    // Opener completes both, forced.
    let club_xfer = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (c, _) = bid_landy_bba(false, &club_xfer, "AQ32.KQ54.A4.432");
    assert_eq!(c, call(3, Strain::Clubs));
    let diamond_xfer = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, _) = bid_landy_bba(false, &diamond_xfer, "AQ32.KQ54.A4.432");
    assert_eq!(c, call(3, Strain::Diamonds));
    // Over the completion the game force shows its one major stopper, and
    // opener supplies 3NT holding the other.
    let completed = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = bid_landy_bba(false, &completed, "32.A43.K2.AQJ876");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the stopper show must come from the book");
    let shown = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let (c, _) = bid_landy_bba(false, &shown, "KQ32.Q54.A43.K32");
    assert_eq!(c, call(3, Strain::Notrump));
}

#[test]
fn landy_minor_slam_answer_is_authored_through_a_double() {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.defense_2c_landy_bba = true;
    arm.competition.landy_minor_slam_answer = true;
    let responder = "32.K42.A2.AKQJ32";

    for tail in [Call::Pass, Call::Double] {
        let mut asked = vec![
            call(1, Strain::Notrump),
            call(2, Strain::Clubs),
            call(2, Strain::Notrump),
            Call::Pass,
            call(3, Strain::Clubs),
            Call::Pass,
            call(4, Strain::Clubs),
            tail,
        ];
        let (c, floored) = best_call_with(&arm, &asked, "AQ32.KQ54.A4.K32");
        assert_eq!(c, call(4, Strain::Notrump));
        assert!(!floored, "the slam try's answer must come from the book");

        asked.extend([call(4, Strain::Notrump), Call::Pass]);
        let (c, floored) = best_call_with(&arm, &asked, responder);
        assert_eq!(c, call(5, Strain::Diamonds), "0-or-3 keycards");
        assert!(!floored, "the RKCB ladder must survive {tail:?}");
    }
}

#[test]
fn landy_bba_keeps_the_values_double_and_sweeps_the_escape() {
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    // The values X is byte-identical to the stack's row.
    let (c, floored) = bid_landy_bba(false, &auction, "KQ32.43.KJ32.432");
    assert_eq!(c, Call::Double);
    assert!(!floored, "the values double must come from the book");
    // The weak 2♦ survives at the shipped band …
    let (c, _) = bid_landy_bba(false, &auction, "32.43.KQJ87.J432");
    assert_eq!(c, call(2, Strain::Diamonds));
    // … and the cap arm narrows it to hcp(..=6): the 7-count passes, the
    // 6-count keeps its escape.
    let (c, _) = bid_landy_bba(true, &auction, "32.43.KQJ87.J432");
    assert_eq!(c, Call::Pass);
    let (c, _) = bid_landy_bba(true, &auction, "32.43.KQJ87.5432");
    assert_eq!(c, call(2, Strain::Diamonds));
}

#[test]
fn landy_bba_takeout_answers_notrump_for_the_short_stopper() {
    let base = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Pass,
    ];
    // Opener with the bid (short) major stopped answers notrump even holding
    // a four-card minor …
    let (c, floored) = bid_landy_bba(false, &base, "A543.AJ4.KQ32.Q2");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the answer must come from the book");
    // … without it, the cheapest four-card minor …
    let (c, _) = bid_landy_bba(false, &base, "AQ54.432.AQ32.K2");
    assert_eq!(c, call(3, Strain::Diamonds));
    // … and with neither stopper nor minor, notrump is the forced catch-all.
    let (c, _) = bid_landy_bba(false, &base, "AQ54.5432.A32.K2");
    assert_eq!(c, call(2, Strain::Notrump));
    // The splinter answer is the same rule a level up.
    let spl = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let (c, _) = bid_landy_bba(false, &spl, "A543.AJ4.KQ32.Q2");
    assert_eq!(c, call(3, Strain::Notrump));
}

#[test]
fn landy_bba_tails_survive_their_interference() {
    // Their X of a takeout takes no room: opener answers verbatim.
    let doubled = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Double,
    ];
    let (c, floored) = bid_landy_bba(false, &doubled, "A543.AJ4.KQ32.Q2");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the doubled takeout must still be answered");
    // Their raise gets the compressed ladder: notrump with the short-major
    // stopper, else the minor, else Pass — responder's game force comes again.
    let raised = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        call(2, Strain::Spades),
    ];
    let (c, _) = bid_landy_bba(false, &raised, "A543.AJ4.KQ32.Q2");
    assert_eq!(c, call(2, Strain::Notrump));
    let (c, _) = bid_landy_bba(false, &raised, "AQ54.432.AQ32.K2");
    assert_eq!(c, call(3, Strain::Diamonds));
    let (c, _) = bid_landy_bba(false, &raised, "AQ543.5432.A2.K2");
    assert_eq!(c, Call::Pass);
    // The doubled club transfer is still completed.
    let doubled_xfer = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Notrump),
        Call::Double,
    ];
    let (c, _) = bid_landy_bba(false, &doubled_xfer, "AQ32.KQ54.A4.432");
    assert_eq!(c, call(3, Strain::Clubs));
}

#[test]
fn landy_bba_makes_the_stack_knobs_inert() {
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    // The GF five-card-minor hand the stack cues bids BBA's ungated 3NT on
    // the ladder — with or without the stack knobs armed on top.
    let hand = "432.A43.K32.AQJ54";
    let (c, _) = bid_landy_bba(false, &auction, hand);
    assert_eq!(c, call(3, Strain::Notrump));
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.defense_2c_landy_bba = true;
    arm.competition.defense_2c_landy_cues = true;
    arm.competition.defense_2c_landy_hcp_rungs = true;
    let (c, _) = best_call_with(&arm, &auction, hand);
    assert_eq!(c, call(3, Strain::Notrump));
}

/// The §N1l arm: their `2♣` disclosed as Landy, the doubler's rebid ladder on.
fn landy_rebids_arm() -> Agreements {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.landy_doubler_rebids = true;
    pin_n1l_cells(&mut arm);
    arm
}

/// Pin the §N1l doubler seat as measured (2026-08-28): the `Pass`@0
/// catch-all on, no three-card cells.  §N1-lia's 2026-08-30 default flips
/// must not leak into the tests that pin the historical arms' rung sets.
fn pin_n1l_cells(arm: &mut Agreements) {
    arm.competition.landy_doubler_catchall = true;
    arm.competition.landy_doubler_three_honors = false;
    arm.competition.landy_doubler_three_small = false;
}

/// The §N1l flip's `px` arm: the penalty `X` and the catch-all, nothing else.
fn landy_px_arm() -> Agreements {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.landy_doubler_px = true;
    pin_n1l_cells(&mut arm);
    arm
}

/// The §N1l flip's `white` arm: `px` plus `3NT`, with the rest of the
/// constructive family gated non-vulnerable.
fn landy_white_arm() -> Agreements {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.landy_doubler_white = true;
    pin_n1l_cells(&mut arm);
    arm
}

/// The doubler's rebid seat with a vulnerability, which
/// [`best_call_with`] pins to `NONE`
///
/// The `white` arm's constructive rungs are the only ones in this lane that
/// read the context's vulnerability, so they are the only thing that needs
/// this.
fn best_call_vul(
    agreements: &Agreements,
    vul: contract_bridge::auction::RelativeVulnerability,
    auction: &[Call],
    hand: &str,
) -> Call {
    use contract_bridge::Hand;
    let hand: Hand = hand.parse().expect("valid test hand");
    let (logits, _) = crate::bidding::american::american(agreements)
        .bind()
        .classify_with_provenance(hand, vul, auction)
        .expect("a legal auction classifies");
    (&logits.0)
        .into_iter()
        .reduce(|best, next| if next.1 > best.1 { next } else { best })
        .map(|(call, _)| call)
        .expect("array is never empty")
}

/// The four calls that reach the doubler's rebid seat over their `2♥`
fn landy_doubler_seat() -> [Call; 6] {
    [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Hearts),
        Call::Pass,
        Call::Pass,
    ]
}

/// The §N1l flip: the two arms the per-rung split asked for
///
/// The 2026-08-28 measurement was mixed **by rung** — the penalty `X` carried
/// the whole vulnerable plain win, every constructive rung dragged vulnerable —
/// so [`CompetitionKnobs::landy_doubler_px`][crate::bidding::agreements::CompetitionKnobs::landy_doubler_px]
/// keeps only the `X` and
/// [`landy_doubler_white`][crate::bidding::agreements::CompetitionKnobs::landy_doubler_white]
/// keeps the whole constructive family behind `!vulnerable()`.  The
/// divergence stream re-read by first differing call says the family splits by
/// **colour**, not by kind — every rung is positive white and negative red, and
/// the natural minors are the *cheaper* half white.  This pins each arm's rung
/// set on the same four hands, so a rung leaking into the wrong arm fails here
/// rather than in an A/B.
#[test]
fn landy_doubler_flip_arms_carry_their_own_rungs() {
    use contract_bridge::auction::RelativeVulnerability;
    let seat = landy_doubler_seat();
    // Four of their major — every arm's one shared rung.
    let doubles = "A54.KJ98.AQ3.J54";
    // 9 points with their suit stopped: the invitation.
    let invites = "KQ5.KJ8.943.T543";
    // 10 points with their suit stopped: the game bid.
    let games = "KQ5.KQ8.943.T543";
    // 8 points, five clubs, no stopper: the full ladder's natural minor.
    let minor = "K54.J98.83.KJ954";

    for (name, arm) in [("px", landy_px_arm()), ("white", landy_white_arm())] {
        let (c, floored) = best_call_with(&arm, &seat, doubles);
        assert_eq!(c, Call::Double, "{name} keeps the penalty X");
        assert!(!floored, "{name}'s X is authored, not the floor's");
    }

    // `px` deletes every constructive rung, colour or no colour.
    for hand in [invites, games, minor] {
        assert_eq!(best_call_with(&landy_px_arm(), &seat, hand).0, Call::Pass);
        assert_eq!(
            best_call_vul(&landy_px_arm(), RelativeVulnerability::WE, &seat, hand),
            Call::Pass,
        );
    }

    // `white` keeps the whole family — including the natural minors, which the
    // re-read prices as the *cheaper* half of it, not the drag.
    let white = landy_white_arm();
    assert_eq!(
        best_call_with(&white, &seat, invites).0,
        call(2, Strain::Notrump),
    );
    assert_eq!(
        best_call_with(&white, &seat, games).0,
        call(3, Strain::Notrump),
    );
    assert_eq!(
        best_call_with(&white, &seat, minor).0,
        call(3, Strain::Clubs)
    );

    // The gate: every constructive rung is white-only, and `3NT` — the table's
    // one game rung, 28 fires in 9.2M boards — is deliberately not gated.
    for (hand, red) in [
        (invites, Call::Pass),
        (minor, Call::Pass),
        (games, call(3, Strain::Notrump)),
    ] {
        assert_eq!(
            best_call_vul(&white, RelativeVulnerability::WE, &seat, hand),
            red,
            "the white arm's constructive rungs are non-vulnerable only",
        );
    }
    // The full ladder, kept for the comparison arm, still bids them red — that
    // is exactly what the flip is flipping.
    let full = landy_rebids_arm();
    assert_eq!(
        best_call_vul(&full, RelativeVulnerability::WE, &seat, invites),
        call(2, Strain::Notrump),
    );
    assert_eq!(
        best_call_vul(&full, RelativeVulnerability::WE, &seat, minor),
        call(3, Strain::Clubs),
    );
    // Vulnerability moves the `X` nowhere.
    assert_eq!(
        best_call_vul(&white, RelativeVulnerability::WE, &seat, doubles),
        Call::Double,
    );
}

/// §N1m's arms: **opener's** own seat, one call before the doubler's
fn landy_opener_arm(rungs: bool) -> Agreements {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.landy_opener_px = true;
    arm.competition.landy_opener_rungs = rungs;
    arm
}

/// Opener's penalty double of the major their advance named, and the gate the
/// oracle drew for it
///
/// `probe-landy-opener-oracle` prices defending their major **doubled** as the
/// winner of every four-plus-trump bucket at both vulnerabilities (+2.8…+8.1
/// IMPs/board over today's floor, PD flat) and −0.7…−4.5 on two or three
/// trumps, so the whole gate is `len(major, 4..)`: no HCP floor, no stopper.
/// The seat is the floor's by default and passes 98.5% of the time.
#[test]
fn landy_opener_doubles_four_of_their_advanced_major() {
    use contract_bridge::auction::RelativeVulnerability;
    let seat = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Hearts),
    ];
    let arm = landy_opener_arm(false);

    // Four of their major doubles, on a bare minimum and on a maximum alike.
    for hand in ["AQ32.J432.AQ3.K3", "Q432.9432.AQ3.KJ3"] {
        let (c, floored) = best_call_with(&arm, &seat, hand);
        assert_eq!(c, Call::Double, "{hand} holds four of their hearts");
        assert!(!floored, "and the double is authored, not the floor's");
        assert_eq!(
            best_call_vul(&arm, RelativeVulnerability::WE, &seat, hand),
            Call::Double,
            "the gate is length, so colour moves nothing",
        );
    }
    // Three is not a trump stack — the oracle prices that double negative on
    // every HCP band but seventeen, where `3NT` matches it anyway.
    assert_eq!(
        best_call_with(&arm, &seat, "AQ32.A43.AQ32.K3").0,
        Call::Pass,
    );
    // The spade leg is the same table one step up.
    let spades = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Spades),
    ];
    assert_eq!(
        best_call_with(&arm, &spades, "J432.AQ32.AQ3.K3").0,
        Call::Double,
    );

    // Opener's partner sits for it.
    let sat = [seat.as_slice(), &[Call::Double, Call::Pass]].concat();
    let (c, floored) = best_call_with(&arm, &sat, "5.KQ98.KT32.T543");
    assert_eq!(
        c,
        Call::Pass,
        "the doubler sits for opener's penalty double"
    );
    assert!(!floored, "and the sit is authored");

    // Off — the default — leaves the whole seat to the floor.
    let mut off = Agreements::default();
    off.decision.their.two_clubs_landy = true;
    assert!(
        best_call_with(&off, &seat, "AQ32.J432.AQ3.K3").1,
        "the default arm keeps the floor-owned seat",
    );
}

/// Opener's two notrump rungs, and the three the oracle threw out
///
/// `3NT`@135 on `hcp(16..) & stopper_in` and `2NT`@120 on fifteen, white only.
/// The `X`@150 above them is what caps them at three trumps — the length cap
/// `has_stopper` cannot express, and the hole that refuted §N1k.
#[test]
fn landy_opener_rungs_declare_only_under_the_double() {
    use contract_bridge::auction::RelativeVulnerability;
    let seat = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Hearts),
    ];
    let px = landy_opener_arm(false);
    let rungs = landy_opener_arm(true);

    // Sixteen with their suit stopped, three trumps: the game.
    let max = "AQ32.AJ3.Q432.K3";
    assert_eq!(best_call_with(&px, &seat, max).0, Call::Pass);
    assert_eq!(
        best_call_with(&rungs, &seat, max).0,
        call(3, Strain::Notrump),
    );
    // Fifteen with their suit stopped: the part-score, and white only.
    let min = "AQ32.AJ3.J432.K3";
    assert_eq!(
        best_call_with(&rungs, &seat, min).0,
        call(2, Strain::Notrump),
    );
    assert_eq!(
        best_call_vul(&rungs, RelativeVulnerability::WE, &seat, min),
        Call::Pass,
        "declaring from this seat is a non-vulnerable idea",
    );
    // Four trumps outrank both rungs — the ordering IS the length cap that
    // §N1k's `hcp(16..) & has_stopper` was missing.
    assert_eq!(
        best_call_with(&rungs, &seat, "AQ3.AJ32.Q432.K3").0,
        Call::Double,
    );
    // No stopper in their suit, no notrump.
    assert_eq!(
        best_call_with(&rungs, &seat, "AQ32.932.AQ3.KJ3").0,
        Call::Pass
    );
    // And the two rungs the oracle threw out are *absent*, not deferred: a
    // fifteen with five clubs bids the notrump part-score white and passes
    // red, never `3♣`; the same with five of the major they did not name
    // never bids `3♠`.  Both would have had a rung under the plan's sketch.
    for hand in ["AJ2.AJ3.32.KQ432", "AQJ32.AJ3.32.K32"] {
        assert_eq!(
            best_call_with(&rungs, &seat, hand).0,
            call(2, Strain::Notrump),
            "{hand} declares notrump, it does not bid a suit",
        );
        assert_eq!(
            best_call_vul(&rungs, RelativeVulnerability::WE, &seat, hand),
            Call::Pass,
            "{hand} defends red rather than bidding a suit",
        );
    }

    // Both rungs are sign-offs: partner passes them.
    for rung in [call(2, Strain::Notrump), call(3, Strain::Notrump)] {
        let after = [seat.as_slice(), &[rung, Call::Pass]].concat();
        let (c, floored) = best_call_with(&rungs, &after, "5.KQ98.KT32.T543");
        assert_eq!(c, Call::Pass, "the doubler passes opener's {rung}");
        assert!(!floored, "and that pass is authored");
        // `px` alone never asks, so it registers no answer either.
        assert!(
            best_call_with(&px, &after, "5.KQ98.KT32.T543").1,
            "px asks no {rung}, so its answer table is not registered",
        );
    }
}

/// An answer table is registered only where its question exists
///
/// A `2NT -` node under an arm with no `2NT` rung is a book node with finite
/// mass shadowing the floor for a call nobody makes
/// (docs/bidding-architecture.md).  The rung and its answer move together.
#[test]
fn landy_doubler_flip_registers_only_the_answers_it_asks_for() {
    let seat = landy_doubler_seat();
    let after = |rung: Call| [seat.as_slice(), &[rung, Call::Pass]].concat();
    let opener = "AQ54.A65.KQ4.Q83";

    // Every arm carries the `X`, so every arm carries opener's sit over it.
    for (name, arm) in [
        ("px", landy_px_arm()),
        ("white", landy_white_arm()),
        ("full", landy_rebids_arm()),
    ] {
        let (c, floored) = best_call_with(&arm, &after(Call::Double), opener);
        assert_eq!(c, Call::Pass, "{name}: the repeated double is penalty");
        assert!(!floored, "{name}: and the sit is authored");
    }

    for rung in [
        call(2, Strain::Notrump),
        call(3, Strain::Clubs),
        call(3, Strain::Diamonds),
    ] {
        let asked = after(rung);
        assert!(
            best_call_with(&landy_px_arm(), &asked, opener).1,
            "px asks no {rung}, so its answer table is not registered",
        );
        assert!(
            !best_call_with(&landy_white_arm(), &asked, opener).1,
            "white asks {rung}, so the answer is authored",
        );
    }

    // The `3m` answer itself — §N1l's completeness debt, paid here.  16
    // opposite a capped 8–9 with their suit stopped is the 25 that bids the
    // game; everything else passes the known minor fit.
    let clubs = after(call(3, Strain::Clubs));
    assert_eq!(
        best_call_with(&landy_white_arm(), &clubs, "AQ54.A65.KQ4.Q83").0,
        call(3, Strain::Notrump),
        "a maximum with their hearts stopped bids the game",
    );
    assert_eq!(
        best_call_with(&landy_white_arm(), &clubs, "AQ54.865.KQ4.Q83").0,
        Call::Pass,
        "without the stopper it passes the minor part-score",
    );
    assert_eq!(
        best_call_with(&landy_white_arm(), &clubs, "AQ54.A65.KQ4.983").0,
        Call::Pass,
        "and so does a minimum",
    );

    let quantitative = after(call(4, Strain::Notrump));
    for (name, arm) in [("px", landy_px_arm()), ("white", landy_white_arm())] {
        assert!(
            best_call_with(&arm, &quantitative, "AQ54.AK5.KQ4.J83").1,
            "{name} deletes the quantitative 4NT, so its answer goes too",
        );
    }
    assert!(
        !best_call_with(&landy_rebids_arm(), &quantitative, "AQ54.AK5.KQ4.J83").1,
        "the full ladder still asks it",
    );
}

/// The doubler's rebid once their advance has named the major — the ladder the
/// dying auction needs
///
/// `competition.landy_doubler_rebids`, default off (§N1l, A/B owed).  The seat
/// is the floor's today on every hand, and the floor bids `3NT` holding four of
/// *their* major rather than doubling, and passes both the 8–9 invitation and
/// the 8–9 five-card minor.
#[test]
fn landy_doubler_rebids_ladders_the_dying_auction() {
    let arm = landy_rebids_arm();
    let hearts = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Hearts),
        Call::Pass,
        Call::Pass,
    ];
    // Four of their major is a penalty double against a pair who have shown
    // 4-4+ in the majors and then chosen this one.
    let (c, floored) = best_call_with(&arm, &hearts, "A54.KJ98.AQ3.J54");
    assert_eq!(c, Call::Double);
    assert!(!floored, "the ladder must come from the book");
    // 8-9 with their suit stopped invites; without a stopper it passes.
    assert_eq!(
        best_call_with(&arm, &hearts, "KQ5.KJ8.943.T543").0,
        call(2, Strain::Notrump)
    );
    assert_eq!(
        best_call_with(&arm, &hearts, "K54.982.Q83.J954").0,
        Call::Pass
    );
    // The natural minors — the only route for an 8-9 one-suiter, the wide
    // transfers above being game-forcing.  Clubs first when both.
    assert_eq!(
        best_call_with(&arm, &hearts, "K54.J98.83.KJ954").0,
        call(3, Strain::Clubs)
    );
    assert_eq!(
        best_call_with(&arm, &hearts, "K54.982.KQJ54.83").0,
        call(3, Strain::Diamonds)
    );

    // The spade leg is the same table one step up.
    let spades = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Spades),
        Call::Pass,
        Call::Pass,
    ];
    assert_eq!(
        best_call_with(&arm, &spades, "KJ98.A54.AQ3.J54").0,
        Call::Double
    );
    assert_eq!(
        best_call_with(&arm, &spades, "KJ8.KQ5.943.T543").0,
        call(2, Strain::Notrump)
    );

    // And the escape leg: their artificial `2♦` pulled to a major by the
    // overcaller, which is what happens to it 79.4% of the time.
    let escape = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Hearts),
    ];
    assert_eq!(
        best_call_with(&arm, &escape, "A54.KJ98.AQ3.J54").0,
        Call::Double
    );

    // Opener's three answers: accept the invitation from the top, the
    // quantitative slam from the very top, and sit for the penalty double.
    let invited = [hearts.as_slice(), &[call(2, Strain::Notrump), Call::Pass]].concat();
    assert_eq!(
        best_call_with(&arm, &invited, "AQ54.A65.KQ4.Q83").0,
        call(3, Strain::Notrump)
    );
    assert_eq!(
        best_call_with(&arm, &invited, "AQ54.A65.KQ4.983").0,
        Call::Pass
    );
    let quantitative = [hearts.as_slice(), &[call(4, Strain::Notrump), Call::Pass]].concat();
    assert_eq!(
        best_call_with(&arm, &quantitative, "AQ54.AK5.KQ4.J83").0,
        call(6, Strain::Notrump)
    );
    let repeated = [hearts.as_slice(), &[Call::Double, Call::Pass]].concat();
    let (c, floored) = best_call_with(&arm, &repeated, "AQ54.A65.KQ4.Q83");
    assert_eq!(c, Call::Pass, "the repeated double is penalty; opener sits");
    assert!(!floored, "and the sit is authored, not the floor's");

    // The shipped default is `px` (2026-08-29) plus §N1-lia's cells
    // (2026-08-30): the penalty `X` on four-plus of their major and the
    // exactly-three cells below it, no catch-all.
    let mut shipped = Agreements::default();
    shipped.decision.their.two_clubs_landy = true;
    let (c, floored) = best_call_with(&shipped, &hearts, "A54.KJ98.AQ3.J54");
    assert_eq!(c, Call::Double, "px's penalty X is the shipped default");
    assert!(!floored, "and it is authored, not the floor's");

    // Three trumps to one top honor doubles from the small cell — §N1-lia
    // package A (2026-08-30) deleted the shadowing catch-all and re-bought
    // exactly-three trumps cell by cell, all under the one re-worded
    // `comp:landy-penalty` claim.
    let (c, floored) = best_call_with(&shipped, &hearts, "A543.KJ9.AQ3.J54");
    assert_eq!(c, Call::Double, "three trumps doubles from the small cell");
    assert!(!floored, "the cell is book, not the floor");

    // Off (`--no-ns-landy-doubler-px`) leaves the whole seat to the floor.
    let mut off = Agreements::default();
    off.decision.their.two_clubs_landy = true;
    off.competition.landy_doubler_px = false;
    let (c, floored) = best_call_with(&off, &hearts, "A54.KJ98.AQ3.J54");
    assert!(floored, "the off arm keeps the floor-owned seat (got {c})");
}

/// The polarity rule, stated as the asymmetry it is: the **same hand** doubles
/// for penalty after our own `X`, and does whatever the floor does after our
/// `P`
///
/// Nothing mechanises this split.  `penalty_x_reading_with_profile` — the
/// reader behind `penalty_latch` — requires *their* 1NT opening and returns
/// `None` at both of these auctions, so no latch converts anything here; the
/// authored node is the entire difference.  Assert it directly, because the
/// two branches look identical from responder's hand.
#[test]
fn landy_doubler_rebid_inverts_only_after_our_own_double() {
    let arm = landy_rebids_arm();
    let hand = "A54.KJ98.AQ3.J54";
    let doubled = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Hearts),
        Call::Pass,
        Call::Pass,
    ];
    let passed = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        Call::Pass,
    ];

    let (after_x, floored_x) = best_call_with(&arm, &doubled, hand);
    assert_eq!(
        after_x,
        Call::Double,
        "after our own X the rebid is penalty"
    );
    assert!(!floored_x, "and it comes from the authored node");

    let (_, floored_p) = best_call_with(&arm, &passed, hand);
    assert!(
        floored_p,
        "after our pass the seat is still the floor's takeout ladder",
    );
}

/// The penalty double is `X`-after-`X`, so its alert has to publish **length**
/// in their major — an unalerted second double reads as the takeout this lane's
/// polarity rule gives to the *pass* branch instead (`docs/pdi.md`).
#[test]
fn landy_doubler_rebid_alerts_publish_the_trump_length() {
    use crate::bidding::inference::{Inferences, Relative};
    use contract_bridge::Suit;
    use contract_bridge::auction::RelativeVulnerability;

    // Two scopes.  `All` (the default since 2026-08-16) reads the rule's own
    // `len(major, 4..)` back through the ordinary projection, so it would pass
    // even with the alert dropped; `Alerted` decodes alerted calls *only*, so
    // it is the arm that actually guards `LANDY_PENALTY`'s presence.  The
    // package invariant cannot: `unalerted_artificial` skips `Double` rules in
    // row-package fallbacks on purpose (the node key cannot witness which
    // strain a suffix-guarded double doubles).
    let read_arm = |mut arm: Agreements, scope, calls: &[Call]| {
        arm.decision.reading.scope = scope;
        let partnership = crate::bidding::american::american(&arm).bind();
        Inferences::read(&partnership.prefixed_context(RelativeVulnerability::NONE, calls))
    };
    let read_with = |scope, calls: &[Call]| read_arm(landy_rebids_arm(), scope, calls);
    let read = |calls: &[Call]| read_with(crate::bidding::inference::ReadingScope::All, calls);
    let after = |last: Call| {
        [
            call(1, Strain::Notrump),
            call(2, Strain::Clubs),
            Call::Double,
            call(2, Strain::Hearts),
            Call::Pass,
            Call::Pass,
            last,
            Call::Pass,
        ]
    };

    let penalty = read(&after(Call::Double));
    let partner = penalty.get(Relative::Partner);
    assert_eq!(
        partner.length(Suit::Hearts).min,
        4,
        "the repeated double shows four of their major",
    );

    // And the invitation below it denies exactly that, by the weight ordering.
    let invite = read(&after(call(2, Strain::Notrump)));
    assert!(
        invite.get(Relative::Partner).length(Suit::Hearts).max <= 3,
        "the `2NT` invitation denies the double it declined (got {:?})",
        invite.get(Relative::Partner).length(Suit::Hearts),
    );

    // Under `Alerted` the length survives only because the double carries
    // `LANDY_PENALTY`.  Drop the alert and this is the assertion that fails.
    let alerted_only = read_with(
        crate::bidding::inference::ReadingScope::Alerted,
        &after(Call::Double),
    );
    assert_eq!(
        alerted_only.get(Relative::Partner).length(Suit::Hearts).min,
        4,
        "the alert is what publishes the trump length to an alert-only reader",
    );

    // The `X` is the one rung the §N1l flip's two smaller arms keep, so the
    // alert has to survive into both of them — a rung set is not allowed to
    // change what a call means.
    for (name, arm) in [("px", landy_px_arm()), ("white", landy_white_arm())] {
        let read = read_arm(
            arm,
            crate::bidding::inference::ReadingScope::Alerted,
            &after(Call::Double),
        );
        assert_eq!(
            read.get(Relative::Partner).length(Suit::Hearts).min,
            4,
            "{name}'s penalty X publishes the trump length too",
        );
    }

    // §N1m shares the slug one seat earlier, and the claim is the same: four
    // of the major their advance named.  `artificial_calls_are_alerted` cannot
    // see this double either, for the same reason, so this is its only guard.
    let opener_seat = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Hearts),
        Call::Double,
        Call::Pass,
    ];
    for scope in [
        crate::bidding::inference::ReadingScope::All,
        crate::bidding::inference::ReadingScope::Alerted,
    ] {
        let read = read_arm(landy_opener_arm(true), scope, &opener_seat);
        assert_eq!(
            read.get(Relative::Partner).length(Suit::Hearts).min,
            4,
            "opener's penalty X publishes the trump length under {scope:?}",
        );
    }
}

/// §N1-lia's arm: their `2♣` disclosed as Landy, the lia ladder on
fn landy_lia_arm() -> Agreements {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.defense_2c_landy_lia = true;
    arm
}

/// §N1-lia package A's arms: the catch-all deleted, then the three-card cells
fn landy_cells_arm(honors: bool, small: bool) -> Agreements {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.landy_doubler_catchall = false;
    arm.competition.landy_doubler_three_honors = honors;
    arm.competition.landy_doubler_three_small = small;
    arm
}

/// §N1-lia package C's arm: the jam riding South African Texas
fn landy_texas_arm() -> Agreements {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.landy_texas = true;
    arm
}

/// §N1-lia's direct seat: the minor ladder a level down, one takeout, natural
/// invitations
///
/// `competition.defense_2c_landy_lia`, default off (package B, A/B owed).  The
/// ladder matches BBA's own coherent self-play tree: `2♠` = 5+♣ weak or GF,
/// `2NT` = 7+♦ / a good six / GF, `3♣`/`3♦` = natural invitations, and `2♥`
/// the only GF takeout (the 2=3=4=4 split dies into it).
#[test]
fn landy_lia_re_rungs_the_minor_ladder() {
    let direct = [call(1, Strain::Notrump), call(2, Strain::Clubs)];

    // The two-way 2♠: a weak five-card club hand and a game-forcing six both
    // take it; the invitational band bids its suit naturally instead.
    let (c, floored) = bid_landy_lia(&direct, "432.32.432.KJ543");
    assert_eq!(c, call(2, Strain::Spades), "weak clubs ride the 2♠ rung");
    assert!(!floored, "the rung must come from the book");
    assert_eq!(
        bid_landy_lia(&direct, "A2.32.A32.KQJ432").0,
        call(2, Strain::Spades),
        "a game force with long clubs takes the same rung",
    );
    assert_eq!(
        bid_landy_lia(&direct, "K32.432.32.KQ543").0,
        call(3, Strain::Clubs),
        "the 8-9 band invites naturally instead",
    );
    assert_eq!(
        bid_landy_lia(&direct, "432.432.KQ543.K2").0,
        call(3, Strain::Diamonds),
        "and the diamond invitation mirrors it",
    );

    // The diamond rung narrows because the weak 2♦ escape already exists:
    // seven-plus or a good six ride 2NT, a bad weak five keeps the escape.
    assert_eq!(
        bid_landy_lia(&direct, "32.32.QJ76543.32").0,
        call(2, Strain::Notrump),
        "seven diamonds transfer at 2NT",
    );
    assert_eq!(
        bid_landy_lia(&direct, "32.432.KQJ432.32").0,
        call(2, Strain::Notrump),
        "a good six (two of the top three) transfers too",
    );
    assert_eq!(
        bid_landy_lia(&direct, "432.432.QJ543.Q2").0,
        call(2, Strain::Diamonds),
        "the weak natural 2♦ is untouched",
    );

    // The 2=3=4=4 game force: N1j names the doubleton with 2♠; lia's only
    // takeout is 2♥, so the split dies into it.
    let two_suiter = "32.432.AQ32.KQ32";
    assert_eq!(
        bid_landy_bba(true, &direct, two_suiter).0,
        call(2, Strain::Spades),
        "the shipped ladder names the spade doubleton",
    );
    let (c, floored) = bid_landy_lia(&direct, two_suiter);
    assert_eq!(c, call(2, Strain::Hearts), "lia's takeout absorbs it");
    assert!(!floored);
}

/// §N1-lia's minor rungs: opener answers by length, no forced completion
///
/// The cheap raise shows three-card support, the step below it a doubleton
/// (`2NT` over `2♠` a contract; `3♣` over `2NT` safe — balanced with two
/// diamonds implies 3+ clubs).  The weak hand signs off over the doubleton
/// answer and opener sits; the game force cues or drives, and the N4-KK `4m`
/// slam try re-hangs byte-identical with its authored answer.  The restored
/// natural invitations get the stack lane's accept table — the floor cannot
/// see the lia regime and would answer a phantom `3♦` transfer completion.
#[test]
fn landy_lia_answers_the_minor_rungs_by_length() {
    let arm = landy_lia_arm();

    // The invitation's acceptance seat: 3NT from the top with both of their
    // majors stopped, else sit for the partscore — authored, not the floor's.
    let invited = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = best_call_with(&arm, &invited, "AK4.KQ42.A95.K32");
    assert_eq!(c, call(3, Strain::Notrump), "a maximum accepts the invite");
    assert!(!floored, "the acceptance must come from the book");
    let (c, floored) = best_call_with(&arm, &invited, "A54.KQ42.A95.Q32");
    assert_eq!(c, Call::Pass, "a minimum sits for the partscore");
    assert!(
        !floored,
        "and the decline is authored, not a phantom transfer"
    );
    let clubs = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let (c, floored) = best_call_with(&arm, &clubs, "A54.KQ4.A954.K32");
    assert_eq!(c, call(3, Strain::Clubs), "three-card support raises");
    assert!(!floored, "the length answer must come from the book");
    assert_eq!(
        best_call_with(&arm, &clubs, "A54.KQ42.A954.K3").0,
        call(2, Strain::Notrump),
        "a doubleton offers the 2NT contract",
    );

    let diamonds = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(
        best_call_with(&arm, &diamonds, "A54.KQ4.A954.K32").0,
        call(3, Strain::Diamonds),
        "three-card support raises the diamond rung",
    );
    assert_eq!(
        best_call_with(&arm, &diamonds, "A54.KQ42.A9.K543").0,
        call(3, Strain::Clubs),
        "two diamonds land in the implied club fit",
    );

    // The weak hand signs off over the doubleton answer — and passes the fit
    // answer, which is already its contract.
    let misfit = [clubs.as_slice(), &[call(2, Strain::Notrump), Call::Pass]].concat();
    let (c, floored) = best_call_with(&arm, &misfit, "432.32.432.KJ543");
    assert_eq!(c, call(3, Strain::Clubs), "the weak hand signs off");
    assert!(!floored);
    let fit = [clubs.as_slice(), &[call(3, Strain::Clubs), Call::Pass]].concat();
    assert_eq!(
        best_call_with(&arm, &fit, "432.32.432.KJ543").0,
        Call::Pass,
        "the fit answer is already the weak hand's contract",
    );
    let signed_off = [misfit.as_slice(), &[call(3, Strain::Clubs), Call::Pass]].concat();
    let (c, floored) = best_call_with(&arm, &signed_off, "A54.KQ42.A954.K3");
    assert_eq!(c, Call::Pass, "opener sits for the sign-off");
    assert!(!floored, "and the sit is authored, not the floor's");

    // The game force shows its one stopper (the transfer-rebid cue verbatim),
    // and the six-card source of tricks starts the re-hung 4m slam try, whose
    // answer asks keycard from a maximum.
    assert_eq!(
        best_call_with(&arm, &fit, "A2.32.A32.KQJ432").0,
        call(3, Strain::Spades),
        "the game force cues the stopper it holds",
    );
    let diamond_fit = [
        diamonds.as_slice(),
        &[call(3, Strain::Diamonds), Call::Pass],
    ]
    .concat();
    assert_eq!(
        best_call_with(&arm, &diamond_fit, "A2.K2.AQJ5432.32").0,
        call(4, Strain::Diamonds),
        "the six-card source starts the slam try",
    );
    let tried = [
        diamond_fit.as_slice(),
        &[call(4, Strain::Diamonds), Call::Pass],
    ]
    .concat();
    let (c, floored) = best_call_with(&arm, &tried, "AK4.KQ42.K95.A32");
    assert_eq!(c, call(4, Strain::Notrump), "a maximum asks keycard");
    assert!(!floored, "the slam answer re-hangs with the rung");
}

/// §N1-lia's takeout: the answer priority reverses and the ask seat is new
///
/// Opener offers a four-card minor *before* notrump (the 4-4 fit is the
/// takeout's point), `2NT` claims the spade stopper specifically, and `2♠`
/// denies both (asking).  Responder resolves hearts with the reused `3♥` cue.
#[test]
fn landy_lia_reverses_the_takeout_answer() {
    let arm = landy_lia_arm();
    let takeout = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Pass,
    ];
    // A four-card minor now outranks the stopper answer — the reversal.
    let (c, floored) = best_call_with(&arm, &takeout, "A54.K42.A95.KQ32");
    assert_eq!(c, call(3, Strain::Clubs), "the minor comes before notrump");
    assert!(!floored);
    assert_eq!(
        best_call_with(&arm, &takeout, "A54.KQ42.A95.K32").0,
        call(2, Strain::Notrump),
        "minor-less with the spade stopper answers 2NT",
    );
    let (c, floored) = best_call_with(&arm, &takeout, "543.AKQ2.A95.K32");
    assert_eq!(c, call(2, Strain::Spades), "neither: the 2♠ ask");
    assert!(!floored, "the ask is authored, not the floor's");

    // Over the ask, responder needs its own spade stopper for notrump; with
    // it but no heart stopper, the cue asks opener for hearts, and opener
    // answers the game holding it.
    let asked = [takeout.as_slice(), &[call(2, Strain::Spades), Call::Pass]].concat();
    assert_eq!(
        best_call_with(&arm, &asked, "A2.K2.AQ32.Q432").0,
        call(3, Strain::Notrump),
        "both stoppers held: bid the game",
    );
    assert_eq!(
        best_call_with(&arm, &asked, "A2.32.AQ32.KQ32").0,
        call(3, Strain::Hearts),
        "spades only: cue for the heart stopper",
    );
    assert_eq!(
        best_call_with(&arm, &asked, "32.A2.AQ32.KQ432").0,
        call(4, Strain::Clubs),
        "no spade stopper anywhere: the game force lands in the minor",
    );
    let cued = [asked.as_slice(), &[call(3, Strain::Hearts), Call::Pass]].concat();
    assert_eq!(
        best_call_with(&arm, &cued, "543.AKQ2.A95.K32").0,
        call(3, Strain::Notrump),
        "opener supplies the heart stopper",
    );

    // Over the 2NT answer the live cue flips to hearts — opener's notrump
    // claimed spades, and nothing in lia's structure ever promises hearts.
    let shown = [takeout.as_slice(), &[call(2, Strain::Notrump), Call::Pass]].concat();
    assert_eq!(
        best_call_with(&arm, &shown, "A2.32.AQ32.KQ32").0,
        call(3, Strain::Hearts),
        "responder asks for the heart stopper over 2NT",
    );
}

/// §N1-lia package A: the catch-all knob and the three-card cells
///
/// `landy_doubler_catchall=false` un-shadows the floor below the rungs (the
/// measured −14,171-IMP suppression); the two cells then buy exactly-three
/// trumps back by top-honor class, each under the re-worded
/// `comp:landy-penalty` tag.
#[test]
fn landy_doubler_cells_split_three_trumps() {
    let seat = landy_doubler_seat();

    // Catch-all deleted, no cells: four trumps still double from the book,
    // three or fewer fall through to the floor the catch-all suppressed.
    let nocatch = landy_cells_arm(false, false);
    let (c, floored) = best_call_with(&nocatch, &seat, "A54.KJ98.AQ3.J54");
    assert_eq!(c, Call::Double, "the four-trump penalty X is untouched");
    assert!(!floored);
    let (c, floored) = best_call_with(&nocatch, &seat, "A54.KJ9.AQ3.J543");
    assert!(
        floored,
        "three trumps now reach the floor (got {c}, floored={floored})"
    );
    let (_, floored) = best_call_with(&nocatch, &seat, "A54.98.AQ32.KJ54");
    assert!(floored, "a doubleton reaches the floor in every arm");

    // The honors cell: two of the top three re-buys the double; one does not.
    let honors = landy_cells_arm(true, false);
    let (c, floored) = best_call_with(&honors, &seat, "A54.KQ9.AQ3.J543");
    assert_eq!(c, Call::Double, "KQx doubles under the honors cell");
    assert!(!floored);
    let (_, floored) = best_call_with(&honors, &seat, "A54.KJ9.AQ3.J543");
    assert!(floored, "a single top honor stays the floor's");

    // The small cell on top: zero-or-one top honor doubles too.
    let cells = landy_cells_arm(true, true);
    let (c, floored) = best_call_with(&cells, &seat, "A54.982.AQ3.KJ54");
    assert_eq!(c, Call::Double, "three small doubles under the small cell");
    assert!(!floored);
    let (c, _) = best_call_with(&cells, &seat, "A54.KJ9.AQ3.J543");
    assert_eq!(c, Call::Double, "one top honor is the small cell's too");

    // The shipped default (2026-08-30) is the full ladder: catch-all gone,
    // both cells on — the same hand doubles with no knob touched.
    let mut shipped = Agreements::default();
    shipped.decision.their.two_clubs_landy = true;
    let (c, floored) = best_call_with(&shipped, &seat, "A54.KJ9.AQ3.J543");
    assert_eq!(c, Call::Double, "the default ships the small cell");
    assert!(!floored);
    let (_, floored) = best_call_with(&shipped, &seat, "A54.98.AQ32.KJ54");
    assert!(floored, "a doubleton still reaches the floor by default");
}

/// §N1-lia package C: the jam rides South African Texas, the direct major is
/// the slam try, and 16+ drives keycard above the completion
#[test]
fn landy_texas_reroutes_the_four_level() {
    let arm = landy_texas_arm();
    let direct = [call(1, Strain::Notrump), call(2, Strain::Clubs)];

    // The jam hand transfers so opener declares; the pre-C arm jammed 4♠ from
    // responder's side, which is the +0.6…+0.7 IMPs/fired this package bought.
    let jam_hand = "AQJ543.32.432.A2";
    let mut pre_c = Agreements::default();
    pre_c.decision.their.two_clubs_landy = true;
    pre_c.competition.landy_texas = false;
    assert_eq!(
        best_call_with(&pre_c, &direct, jam_hand).0,
        call(4, Strain::Spades),
        "the direct jam declares from the wrong side",
    );
    let (c, floored) = best_call_with(&arm, &direct, jam_hand);
    assert_eq!(
        c,
        call(4, Strain::Diamonds),
        "under Texas the jam transfers"
    );
    assert!(!floored);

    // Fifteen takes the freed direct slam try; sixteen-plus transfers and
    // will drive its own keycard.  (Both hands leave hearts unstopped — the
    // gated `3NT`@180 outranks the whole four-level family, as it always has.)
    assert_eq!(
        best_call_with(&arm, &direct, "AQJ543.32.A32.A2").0,
        call(4, Strain::Spades),
        "the 15-count invites with the direct call",
    );
    assert_eq!(
        best_call_with(&arm, &direct, "AKJ543.32.A32.A2").0,
        call(4, Strain::Diamonds),
        "sixteen-plus transfers instead",
    );

    // Opener completes the transfer, doubled or not.
    let transfer = [direct.as_slice(), &[call(4, Strain::Diamonds), Call::Pass]].concat();
    let (c, floored) = best_call_with(&arm, &transfer, "A54.KQ42.A95.K32");
    assert_eq!(c, call(4, Strain::Spades), "opener completes");
    assert!(!floored);
    let doubled = [
        direct.as_slice(),
        &[call(4, Strain::Diamonds), Call::Double],
    ]
    .concat();
    assert_eq!(
        best_call_with(&arm, &doubled, "A54.KQ42.A95.K32").0,
        call(4, Strain::Spades),
        "their double takes no room; the completion stands",
    );

    // Above the completion: the game hand sits by authorship (the learned
    // floor must not pull a completed transfer — §N1o's forensic), and the
    // 16+ hand drives keycard.
    let completed = [transfer.as_slice(), &[call(4, Strain::Spades), Call::Pass]].concat();
    let (c, floored) = best_call_with(&arm, &completed, jam_hand);
    assert_eq!(c, Call::Pass, "the game hand passes the completion");
    assert!(!floored, "and the sit is authored, not the floor's");
    let (c, floored) = best_call_with(&arm, &completed, "AKJ543.32.A32.A2");
    assert_eq!(c, call(4, Strain::Notrump), "sixteen-plus drives keycard");
    assert!(!floored);

    // The direct try is opener-decides: a maximum launches keycard, a
    // minimum signs off in the game.
    let tried = [direct.as_slice(), &[call(4, Strain::Spades), Call::Pass]].concat();
    assert_eq!(
        best_call_with(&arm, &tried, "AK4.KQ42.A95.K32").0,
        call(4, Strain::Notrump),
        "a maximum accepts the try with RKCB",
    );
    assert_eq!(
        best_call_with(&arm, &tried, "A54.KQ42.A95.K32").0,
        Call::Pass,
        "a minimum signs off",
    );
}

/// §N1-lia's rungs publish sound readings: the minor rungs their suit and
/// length bands, the takeout its minors, and the ask no phantom spade suit
#[test]
fn landy_lia_rungs_publish_their_shapes() {
    use crate::bidding::inference::{Inferences, Relative};
    use contract_bridge::Suit;
    use contract_bridge::auction::RelativeVulnerability;

    let partnership = crate::bidding::american::american(&landy_lia_arm()).bind();
    let read = |calls: &[Call]| {
        Inferences::read(&partnership.prefixed_context(RelativeVulnerability::NONE, calls))
    };
    let base = [call(1, Strain::Notrump), call(2, Strain::Clubs)];

    // The 2♠ rung is clubs, not spades.
    let clubs = read(&[base.as_slice(), &[call(2, Strain::Spades), Call::Pass]].concat());
    let partner = clubs.get(Relative::Partner);
    assert!(
        partner.length(Suit::Clubs).min >= 5,
        "2♠ publishes five-plus clubs (got {:?})",
        partner.length(Suit::Clubs),
    );
    assert_eq!(
        partner.length(Suit::Spades).min,
        0,
        "and floors no phantom spade suit",
    );

    // The 2NT rung is six-plus diamonds.
    let diamonds = read(&[base.as_slice(), &[call(2, Strain::Notrump), Call::Pass]].concat());
    assert!(
        diamonds.get(Relative::Partner).length(Suit::Diamonds).min >= 6,
        "2NT publishes six-plus diamonds (got {:?})",
        diamonds.get(Relative::Partner).length(Suit::Diamonds),
    );

    // Opener's length answers: the raise floors three, the doubleton caps two
    // — the exact bands, not the walk's four-card raise floor.
    let raised = read(
        &[
            base.as_slice(),
            &[
                call(2, Strain::Spades),
                Call::Pass,
                call(3, Strain::Clubs),
                Call::Pass,
            ],
        ]
        .concat(),
    );
    let support = raised.get(Relative::Partner).length(Suit::Clubs);
    assert!(
        support.min >= 3 && support.min < 4,
        "the raise shows exactly a three-card floor (got {support:?})",
    );
    let short = read(
        &[
            base.as_slice(),
            &[
                call(2, Strain::Spades),
                Call::Pass,
                call(2, Strain::Notrump),
                Call::Pass,
            ],
        ]
        .concat(),
    );
    assert!(
        short.get(Relative::Partner).length(Suit::Clubs).max <= 2,
        "the doubleton answer caps clubs at two (got {:?})",
        short.get(Relative::Partner).length(Suit::Clubs),
    );

    // The takeout publishes both minors; opener's 2♠ ask floors no spades.
    let takeout = read(&[base.as_slice(), &[call(2, Strain::Hearts), Call::Pass]].concat());
    let partner = takeout.get(Relative::Partner);
    assert!(
        partner.length(Suit::Clubs).min >= 4 && partner.length(Suit::Diamonds).min >= 4,
        "the takeout publishes four-four minors (got ♣{:?} ♦{:?})",
        partner.length(Suit::Clubs),
        partner.length(Suit::Diamonds),
    );
    let ask = read(
        &[
            base.as_slice(),
            &[
                call(2, Strain::Hearts),
                Call::Pass,
                call(2, Strain::Spades),
                Call::Pass,
            ],
        ]
        .concat(),
    );
    assert!(
        ask.get(Relative::Partner).length(Suit::Spades).min <= 2,
        "the alerted ask adds no spade claim past the balanced opening (got {:?})",
        ask.get(Relative::Partner).length(Suit::Spades),
    );
}
