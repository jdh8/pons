use super::super::tests::{
    best_call_with, bid, bid_landy, bid_landy_bba, bid_landy_cues, bid_landy_n1,
    bid_landy_transfer, bid_transfer, call,
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

/// The values double's own game: opener holds the stopper, so opener bids it
///
/// `competition.landy_doubler_notrump`, default off — the 2026-08-27 bucket
/// cut reads the seat's floor-owned pass at −47 plain / −44 PD over 22 boards,
/// all of it in the `hcp 16+`-with-a-stopper cell (§N1, A/B owed).
#[test]
fn landy_doubler_notrump_bids_the_stopped_game() {
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Hearts),
    ];
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.landy_doubler_notrump = true;
    // 16 opposite a double that reads back 8-9 is the 24 that bids the game,
    // and the heart stopper is on this side of the table.
    let (c, floored) = best_call_with(&arm, &auction, "AJ5.KQ5.A932.Q54");
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(!floored, "the notrump out must come from the book");
    // A 15-count is 23 and passes; 16 without their suit stopped is the wrong
    // side of the table and passes too.
    assert_eq!(
        best_call_with(&arm, &auction, "AJ5.KQ5.A932.J54").0,
        Call::Pass
    );
    assert_eq!(
        best_call_with(&arm, &auction, "AJ5.762.AQ32.KQ4").0,
        Call::Pass
    );
    // Same rung on the spade leg.
    let spades = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Spades),
    ];
    assert_eq!(
        best_call_with(&arm, &spades, "KQ5.AJ5.A932.Q54").0,
        call(3, Strain::Notrump)
    );
    // Off — the default — leaves the whole seat to the floor, which passes.
    let mut off = Agreements::default();
    off.decision.their.two_clubs_landy = true;
    let (c, floored) = best_call_with(&off, &auction, "AJ5.KQ5.A932.Q54");
    assert_eq!(c, Call::Pass);
    assert!(floored, "the default arm keeps the floor-owned seat");
}

/// The N1j ladder with the doubler's own rebid armed
///
/// [`landy_doubler_notrump`][crate::bidding::agreements::CompetitionKnobs::landy_doubler_notrump]
/// is pinned **off**, the shipped default: it owns the seat one call earlier on
/// the same two legs, and its A/B is the gate on this one, so the rebid ladder
/// is measured (and tested) against the table that shipped.
fn landy_rebids_arm() -> Agreements {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.landy_doubler_rebids = true;
    arm.competition.landy_doubler_notrump = false;
    arm
}

/// The doubler's rebid once their advance has named the major — the ladder the
/// dying auction needs
///
/// `competition.landy_doubler_rebids`, default off (§N1, A/B owed).  The seat
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

    // Off — the default — leaves the whole seat to the floor.
    let mut off = Agreements::default();
    off.decision.their.two_clubs_landy = true;
    let (c, floored) = best_call_with(&off, &hearts, "A54.KJ98.AQ3.J54");
    assert!(
        floored,
        "the default arm keeps the floor-owned seat (got {c})"
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

    let partnership = crate::bidding::american::american(&landy_rebids_arm()).bind();
    let read = |calls: &[Call]| {
        Inferences::read(&partnership.prefixed_context(RelativeVulnerability::NONE, calls))
    };
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
}

/// The splinters graded on high cards, not on the shortness they are showing
///
/// `competition.landy_splinter_hcp`, default off (§N1, A/B owed).  The shipped
/// `points(10..)` floor counts the singleton the call announces, so a
/// 4=1=4=4 nine-count grades to ten and splinters; the bucket cut prices that
/// sub-cell at −33 plain / −39 PD.
///
/// **What the demoted hand actually does is bid `3NT` itself**, not double —
/// [`landy_bba_responder`]'s ungated `3NT`@168 is also `points(10..)` and
/// catches it one rung lower.  So this knob does not remove the failing game;
/// it **right-sides** it.  The census says 14 of the 17 `3♥` boards end
/// `3♥ - 3NT - - -`, which is opener declaring with responder's singleton in
/// *dummy* and the opening lead running up to it; demoted, responder declares
/// the same contract and the lead comes into the 15-17.  That is a real,
/// double-dummy-visible change, and it is the one this arm prices — but it is
/// a different hypothesis from the one §N1k's finding 3 wrote down, and it is
/// recorded as such.
#[test]
fn landy_splinter_hcp_right_sides_the_game_it_cannot_avoid() {
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    // 4=1=4=4, nine high-card points — ten or more once the singleton is
    // counted, which is exactly the grading the knob removes.
    let inflated = "AJ54.3.KJ54.9542";
    let mut shipped = Agreements::default();
    shipped.decision.their.two_clubs_landy = true;
    assert_eq!(
        best_call_with(&shipped, &auction, inflated).0,
        call(3, Strain::Hearts),
        "the shipped gate splinters on shape points",
    );

    let mut armed = shipped;
    armed.competition.landy_splinter_hcp = true;
    assert_eq!(
        best_call_with(&armed, &auction, inflated).0,
        call(3, Strain::Notrump),
        "demoted, it takes the ungated `3NT`@168 — which is also `points(10..)`",
    );

    // A genuine ten-count with the same shape still splinters on both arms —
    // the knob narrows the floor, it does not delete the rung.
    let genuine = "AQ54.3.KJ54.K542";
    for arm in [&shipped, &armed] {
        assert_eq!(
            best_call_with(arm, &auction, genuine).0,
            call(3, Strain::Hearts)
        );
    }
}

/// The N1j lane's three unfinished tails, as one batch
///
/// `competition.landy_tail_completion`, default off (§N1m, A/B owed): their
/// overcall of our minor transfer, the floored four-level seats of the
/// 2026-08-25 survey, and the manufactured-`4♣` repair.
#[test]
fn landy_tail_completion_closes_the_three_open_tails() {
    let mut shipped = Agreements::default();
    shipped.decision.their.two_clubs_landy = true;
    let mut armed = shipped;
    armed.competition.landy_tail_completion = true;

    // 1. The manufactured `4♣`.  Opener answered `2NT` off the `no_minor`
    //    branch (seven major cards), responder asked for the spade stopper and
    //    opener has none — so the catch-all fires.  Shipped it names clubs on a
    //    singleton; armed it names the three-card minor it actually holds.
    let ask = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    let no_minor = "9432.AKQ32.K43.A";
    assert_eq!(
        best_call_with(&shipped, &ask, no_minor).0,
        call(4, Strain::Clubs),
        "the shipped catch-all manufactures clubs on a singleton",
    );
    assert_eq!(
        best_call_with(&armed, &ask, no_minor).0,
        call(4, Strain::Diamonds),
        "armed, it names the longer three-card minor",
    );

    // 2. Their overcall of the diamond transfer — the batch's named cost, where
    //    the floor blasts `5♦` over a transfer that promised no values.
    let overcalled = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(3, Strain::Clubs),
        call(3, Strain::Spades),
    ];
    let (c, floored) = best_call_with(&armed, &overcalled, "KQ5.AJ4.KQ32.A43");
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(!floored, "the tail must come from the book, not the floor");
    // A minimum without their suit stopped has nothing Pass cannot say.
    assert_eq!(
        best_call_with(&armed, &overcalled, "9432.AJ4.KQ32.A4").0,
        Call::Pass,
    );

    // 3. A floored four-level seat: opener picked a minor over the splinter and
    //    responder, game-forcing with extras, had no way to keycard — the
    //    floor bid a phantom `4♠` instead.
    let four_level = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(3, Strain::Hearts),
        Call::Pass,
        call(4, Strain::Clubs),
        Call::Pass,
    ];
    let extras = "AK54.3.KQ54.KJ54";
    let (c, floored) = best_call_with(&shipped, &four_level, extras);
    assert!(floored, "the seat is the floor's today (it bids {c})");
    assert_eq!(
        best_call_with(&armed, &four_level, extras).0,
        call(4, Strain::Notrump),
        "armed, extras ask keycard",
    );
    // Without extras the game is placed, not probed.
    assert_eq!(
        best_call_with(&armed, &four_level, "9854.3.QJ54.QJ54").0,
        call(5, Strain::Clubs),
    );
}

/// The Kokish–Kraft arm: the declared-Landy agreements with the variant on
fn landy_kk_arm() -> Agreements {
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;
    arm.competition.defense_2c_landy_kk = true;
    // Pinned off, both default-off and both prerequisites of the variant in
    // *design* but not in *code*: the doubler ladder is where this table's
    // 8-9 one-suiters are meant to go, and the splinter regrade changes two of
    // its own rows.  Pinned so the arm keeps meaning "the variant alone".
    arm.competition.landy_doubler_rebids = false;
    arm.competition.landy_splinter_hcp = false;
    arm
}

/// The Kokish–Kraft minor core replaces five rungs and leaves the rest alone
///
/// `competition.defense_2c_landy_kk`, default off (§N1n, A/B owed).
#[test]
fn landy_kk_replaces_the_minor_core() {
    let arm = landy_kk_arm();
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];

    // Both minors, split by strength rather than by shape.
    assert_eq!(
        best_call_with(&arm, &auction, "K54.J8.Q543.T543").0,
        call(2, Strain::Hearts),
        "4-4 minors under eight is competitive",
    );
    assert_eq!(
        best_call_with(&arm, &auction, "AK4.J8.KQ54.T543").0,
        call(2, Strain::Spades),
        "the same shape with values is invitational-plus",
    );

    // The escape relay takes either minor; the cross-transfers are crossed.
    for hand in ["543.J8.Q5.KT8543", "543.J8.KT8543.Q5"] {
        assert_eq!(
            best_call_with(&arm, &auction, hand).0,
            call(2, Strain::Notrump),
            "{hand}: a weak six-card minor escapes through the relay",
        );
    }
    assert_eq!(
        best_call_with(&arm, &auction, "AK4.J8.KQT854.53").0,
        call(3, Strain::Clubs),
        "`3♣` is diamonds",
    );
    assert_eq!(
        best_call_with(&arm, &auction, "AK4.J8.53.KQT854").0,
        call(3, Strain::Diamonds),
        "`3♦` is clubs",
    );
    // The same hand is a wide club transfer at `2NT` on the shipped ladder —
    // the trade the A/B prices.
    let mut shipped = Agreements::default();
    shipped.decision.their.two_clubs_landy = true;
    assert_eq!(
        best_call_with(&shipped, &auction, "AK4.J8.53.KQT854").0,
        call(2, Strain::Notrump),
    );

    // Carried over verbatim: the splinter, the gated `3NT`, the values double.
    assert_eq!(
        best_call_with(&arm, &auction, "AK54.3.KQ54.T543").0,
        call(3, Strain::Hearts),
    );
    assert_eq!(
        best_call_with(&arm, &auction, "AQ5.KJ9.Q943.T54").0,
        call(3, Strain::Notrump),
    );
    // Nine high cards and no shape call left: the values double, unchanged.
    // (Ten *points* would take the ungated `3NT`@168 above it instead — the
    // rung this variant carries over verbatim, and the reason its "8-9
    // one-suiters double and rebid `3m`" routing is only true up to nine.)
    assert_eq!(
        best_call_with(&arm, &auction, "Q54.KJ8.T432.K43").0,
        Call::Double,
    );
}

/// Both sides of the Kokish–Kraft continuations — the relay, the two-suiter
/// answers, and the asymmetric `3♦` leg
#[test]
fn landy_kk_continuations_answer_both_sides() {
    let arm = landy_kk_arm();
    let over = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    let after = |ours: Call, rest: &[Call]| {
        let mut a = over.to_vec();
        a.push(ours);
        a.extend_from_slice(rest);
        a
    };

    // The escape relay: opener is forced to `3♣`, responder passes or corrects.
    let relayed = after(call(2, Strain::Notrump), &[Call::Pass]);
    assert_eq!(
        best_call_with(&arm, &relayed, "AQ5.KJ9.AQ43.T54").0,
        call(3, Strain::Clubs),
        "the completion is forced",
    );
    let completed = after(
        call(2, Strain::Notrump),
        &[Call::Pass, call(3, Strain::Clubs), Call::Pass],
    );
    assert_eq!(
        best_call_with(&arm, &completed, "543.J8.Q5.KT8543").0,
        Call::Pass,
        "clubs pass",
    );
    assert_eq!(
        best_call_with(&arm, &completed, "543.J8.KT8543.Q5").0,
        call(3, Strain::Diamonds),
        "diamonds correct",
    );

    // The competitive `2♥` wants a maximum for notrump; the invitational `2♠`
    // does not, because responder already promised the values.
    let comp = after(call(2, Strain::Hearts), &[Call::Pass]);
    let inv = after(call(2, Strain::Spades), &[Call::Pass]);
    let minimum = "AQ5.KJ9.Q943.T54";
    assert_eq!(
        best_call_with(&arm, &comp, minimum).0,
        call(3, Strain::Clubs)
    );
    assert_eq!(
        best_call_with(&arm, &inv, minimum).0,
        call(2, Strain::Notrump),
    );
    assert_eq!(
        best_call_with(&arm, &comp, "AQ5.KJ9.AQ43.T54").0,
        call(2, Strain::Notrump),
        "a maximum bids it on the competitive leg too",
    );

    // The asymmetric `3♦` leg: stoppers below game, the completion last.
    let clubs = after(call(3, Strain::Diamonds), &[Call::Pass]);
    assert_eq!(
        best_call_with(&arm, &clubs, "AQ5.KJ9.Q432.T5").0,
        call(3, Strain::Notrump),
    );
    assert_eq!(
        best_call_with(&arm, &clubs, "952.KJ9.AQ432.T5").0,
        call(3, Strain::Hearts),
        "one stopper is shown below game, not completed",
    );
    assert_eq!(
        best_call_with(&arm, &clubs, "9542.T92.AKQ3.AJ").0,
        call(4, Strain::Clubs),
        "neither stopper completes the transfer instead",
    );
}

/// The cross-transfers must publish the suit they *mean*, not the one they name
#[test]
fn landy_kk_alerts_publish_the_crossed_suit() {
    use crate::bidding::inference::{Inferences, Relative};
    use contract_bridge::Suit;
    use contract_bridge::auction::RelativeVulnerability;

    let partnership = crate::bidding::american::american(&landy_kk_arm()).bind();
    let read = |last: Call| {
        let calls = [
            call(1, Strain::Notrump),
            call(2, Strain::Clubs),
            last,
            Call::Pass,
        ];
        Inferences::read(&partnership.prefixed_context(RelativeVulnerability::NONE, &calls))
    };

    for (bid, meant, named) in [
        (call(3, Strain::Clubs), Suit::Diamonds, Suit::Clubs),
        (call(3, Strain::Diamonds), Suit::Clubs, Suit::Diamonds),
    ] {
        let inferences = read(bid);
        let partner = inferences.get(Relative::Partner);
        assert_eq!(
            partner.length(meant).min,
            6,
            "{bid} shows six {meant}, not {named}",
        );
        assert!(
            partner.length(named).max <= 5,
            "{bid} must not read as length in {named} (got {:?})",
            partner.length(named),
        );
    }

    // And the strength split reads as the cut it is.
    let competitive = read(call(2, Strain::Hearts));
    let comp = competitive.get(Relative::Partner);
    assert_eq!(comp.length(Suit::Clubs).min, 4);
    assert_eq!(comp.length(Suit::Diamonds).min, 4);
    assert!(comp.strength.hcp.max <= 7, "the competitive leg is capped");
}

/// Off by default and inert without the disclosure: the knob alone must not
/// move a single call of the shipped natural `(2♣)` leg.
#[test]
fn landy_kk_is_inert_without_the_disclosure() {
    let mut knob_only = Agreements::default();
    knob_only.competition.defense_2c_landy_kk = true;
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    for hand in [
        "K54.J8.Q543.T543",
        "AK4.J8.53.KQT854",
        "543.J8.Q5.KT8543",
        "AK54.3.KQ54.T543",
    ] {
        assert_eq!(
            best_call_with(&knob_only, &auction, hand),
            best_call_with(&Agreements::default(), &auction, hand),
            "{hand}: the knob moved a call on the undeclared natural leg",
        );
    }
}

/// Off by default and inert without the disclosure: the knob alone must not
/// move a single call of the shipped natural `(2♣)` leg.
#[test]
fn landy_doubler_rebids_is_inert_without_the_disclosure() {
    let mut knob_only = Agreements::default();
    knob_only.competition.landy_doubler_rebids = true;
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Hearts),
        Call::Pass,
        Call::Pass,
    ];
    for hand in [
        "A54.KJ98.AQ3.J54",
        "KQ5.KJ8.943.T543",
        "K54.J98.83.KJ954",
        "K54.982.Q83.J954",
    ] {
        assert_eq!(
            best_call_with(&knob_only, &auction, hand),
            best_call_with(&Agreements::default(), &auction, hand),
            "{hand}: the knob moved a call on the undeclared natural leg",
        );
    }
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
