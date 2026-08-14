use super::super::tests::{
    best_call_with, bid, bid_landy, bid_landy_cues, bid_landy_n1, bid_landy_transfer, bid_transfer,
    call,
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
fn landy_declaration_engages_the_shipped_stack() {
    // The 2026-08-14 default flip: a bare declaration — no knobs touched —
    // now plays the full N1c+N1d/e/f stack (the pooled two-seed win|win).
    let mut arm = Agreements::default();
    arm.decision.their.two_clubs_landy = true;

    // N1c: the weak six-card club hand transfers.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Clubs)];
    let (c, _) = best_call_with(&arm, &auction, "32.43.432.QJ8765");
    assert_eq!(c, call(2, Strain::Notrump));

    // N1d: the 8-count with five clubs defends instead of cueing.
    let (c, _) = best_call_with(&arm, &auction, "43.432.J32.AK432");
    assert_eq!(c, Call::Double);

    // N1f: the doubled cue is answered from the book, not the floor.
    let doubled_cue = [
        call(1, Strain::Notrump),
        call(2, Strain::Clubs),
        call(2, Strain::Hearts),
        Call::Double,
    ];
    let (c, floored) = best_call_with(&arm, &doubled_cue, "AQ32.KQ2.A432.32");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the shipped stack answers the doubled cue");
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
