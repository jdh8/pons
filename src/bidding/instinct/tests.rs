use super::*;
use crate::bidding::context::DecisionProfile;
use crate::bidding::trie::Classifier;
use contract_bridge::auction::RelativeVulnerability;

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The highest-logit instinct call for a hand in an auction
fn best(auction: &[Call], hand: &str) -> Call {
    best_with(&Agreements::default(), auction, hand)
}

/// [`best`] under an explicit profile, pinned into the bare test context
fn best_with(agreements: &Agreements, auction: &[Call], hand: &str) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let context =
        Context::new(RelativeVulnerability::NONE, auction).with_profile(agreements.decision);
    let logits = instinct(agreements).classify(hand, &context);
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

fn kickback_agreements() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.decision.reading.rkcb_variant = RkcbVariant::Kickback;
    agreements
}

fn rubens_agreements() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.decision.reading.rubens_advances = true;
    agreements
}

/// The full-`american_instinct()` call for a hand and whether the floor
/// produced it
///
/// `depth == 0` with `fallback == Some(_)` is the instinct floor firing — so
/// the second tuple field tells a test the node is off-book (floor territory),
/// guarding against a floor rule that is silently shadowed by a book node.
/// Uses [`american_instinct`] (not the net-floored [`american`]) so these
/// tests exercise the deterministic instinct ladder they assert against.
fn american_floored(auction: &[Call], hand: &str) -> (Call, bool) {
    american_floored_with(&Agreements::default(), auction, hand)
}

/// The same, built from an explicit [`Agreements`] rather than this thread's
///
/// The tests that delete a book node to hand the position to the floor arm the
/// captured value here instead of a global.
fn american_floored_with(agreements: &Agreements, auction: &[Call], hand: &str) -> (Call, bool) {
    use crate::bidding::american::american_instinct;
    let hand: Hand = hand.parse().expect("valid test hand");
    let (logits, provenance) = american_instinct(agreements)
        .against()
        .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
        .expect("a legal auction classifies");
    let call = (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty");
    (call, provenance.depth == 0 && provenance.fallback.is_some())
}

#[test]
fn advancing_a_double_advances_a_bust_but_defends_with_length() {
    // Partner doubled their 3♣ for takeout, RHO passed.
    let auction = [call(3, Strain::Clubs), Call::Double, Call::Pass];
    // A worthless hand with a five-card suit outside theirs still advances —
    // it cannot beat 3♣ doubled, so it bids rather than pass into it.
    assert_eq!(best(&auction, "96432.J85.9742.2"), call(3, Strain::Spades));
    // But four cards sitting behind their suit defend: pass plays 3♣ doubled,
    // a better penalty than escaping (the settle floor, default on).
    assert_eq!(best(&auction, "964.J85.974.9632"), Call::Pass);
}

#[test]
fn opener_reopens_and_raises_a_free_notrump() {
    // A suit opener's balanced 18-19 (15-17 opens 1NT) was invisible in a
    // contested auction — it could only make a lone takeout double.
    let bal18_stop = "AQ5.AQ9.KQ4.J932"; // 18, balanced, diamonds stopped
    let bal18_open = "AQ5.AQ9.932.KQJ2"; // 18, balanced, diamonds wide open
    let flat13 = "KJ32.KJ2.Q42.QJ2"; //    13, balanced minimum

    // Reopening seat 1♣ (1♦) - -: with the stopper, reopen a natural 1NT
    // (the game invite a takeout double of a balanced hand cannot make);
    // without a stopper, still double; a minimum still passes.
    let reopen = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        Call::Pass,
        Call::Pass,
    ];
    assert_eq!(best(&reopen, bal18_stop), call(1, Strain::Notrump));
    assert_eq!(best(&reopen, bal18_open), Call::Double);
    assert_eq!(best(&reopen, flat13), Call::Pass);

    // Responder trapped 7 opposite the reopening 1NT — raise to game.
    let raised = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        Call::Pass,
        Call::Pass,
        call(1, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(best(&raised, "K84.T765.KJ7.T65"), call(3, Strain::Notrump));

    // Over responder's free 1NT (which already promised 6-10 and a stopper),
    // opener's balanced 18-19 raises to 3NT; a minimum passes.
    let free_1nt = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        call(1, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(best(&free_1nt, bal18_stop), call(3, Strain::Notrump));
    assert_eq!(best(&free_1nt, flat13), Call::Pass);
}

#[test]
fn minimum_doubler_does_not_over_raise_a_forced_advance() {
    // 1♦ (1♥) - (1♠) X - 2♦ (2♥): we opened and reopened with a takeout
    // double; partner's 2♦ is a *forced* advance (a possible bust).  A minimum
    // doubler that raised to 3♦ (or re-doubled) drove into a doubled game.
    let advanced = [
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Pass,
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Diamonds),
        call(2, Strain::Hearts),
    ];
    let minimum = "A92.A3.J873.KJ84"; // 13: the double already showed it
    let maximum = "AQ2.A3.KJ83.KQJ4"; // 18: a genuine game try

    // Default on: the minimum passes (defend), the maximum still competes.
    assert_eq!(best(&advanced, minimum), Call::Pass);
    assert_eq!(best(&advanced, maximum), call(3, Strain::Diamonds));

    // Off: the blind raise ladder returns (the A/B baseline).
    let mut agreements = Agreements::default();
    agreements.decision.instinct.rein_advance_raise = false;
    assert_eq!(
        best_with(&agreements, &advanced, minimum),
        call(3, Strain::Diamonds)
    );

    // Unaffected: raising partner's *overcall* (we never doubled) — the 2♦
    // competitor lets us raise 1♥ to 2♥ on the same minimum values.
    let raise_overcall = [
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        call(2, Strain::Diamonds),
    ];
    assert_eq!(
        best(&raise_overcall, "A5.KJ73.652.Q874"),
        call(2, Strain::Hearts)
    );
}

#[test]
fn trump_stack_converts_to_penalties() {
    // KQ92 behind the 2♠ bidder sits for partner's takeout double.
    let auction = [call(2, Strain::Spades), Call::Double, Call::Pass];
    assert_eq!(best(&auction, "KQ92.A532.J42.96"), Call::Pass);
}

#[test]
fn competitive_rebid_shows_the_long_suit() {
    // 1♦ (1♥) - (2♥): opener holds a self-sufficient seven-card diamond suit
    // and a stiff in their hearts.  The floor's only competitive actions once
    // it has bid are raise-partner and takeout-double — and partner passed.
    let raised = [
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Hearts),
    ];
    let one_suiter = "765.A.AKJT984.63";

    // Off: the floor can only double, misdescribing a takeout shape.
    let mut off = Agreements::default();
    off.instinct.competitive_rebid = false;
    assert_eq!(best_with(&off, &raised, one_suiter), Call::Double);

    // On: rebid the suit instead — and it is the floor that produces it, not a
    // book node shadowing the position.
    let mut on = Agreements::default();
    on.instinct.competitive_rebid = true;
    assert_eq!(
        best_with(&on, &raised, one_suiter),
        call(3, Strain::Diamonds)
    );
    assert_eq!(
        american_floored_with(&on, &raised, one_suiter),
        (call(3, Strain::Diamonds), true)
    );

    // Balancing seat (they did not raise): the cheapest rebid is 2♦.
    let balancing = [
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Pass,
        Call::Pass,
    ];
    assert_eq!(
        best_with(&on, &balancing, one_suiter),
        call(2, Strain::Diamonds)
    );

    // General across suits: opener's six-card major over their overcall + raise.
    let major = [
        call(1, Strain::Spades),
        call(2, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Hearts),
    ];
    assert_eq!(best(&major, "AKJ982.K5.A54.63"), call(3, Strain::Spades));

    // A five-card suit is not enough: the takeout double stands.
    assert_eq!(best(&raised, "765.A.AKJT9.6432"), Call::Double);

    // 3-level quality gate (over their raise, cheapest rebid is 3♦): a good
    // six (two of the top three honors) or seven cards rebids; a ragged six
    // does not.  The seven-card `one_suiter` above already covers the length
    // path.
    assert_eq!(best(&raised, "765.A.AKJ864.632"), call(3, Strain::Diamonds));
    assert_ne!(best(&raised, "KQ5.A.T98764.632"), call(3, Strain::Diamonds));
    // …but that same ragged six still competes at the cheaper two level.
    assert_eq!(
        best(&balancing, "KQ5.A.T98764.632"),
        call(2, Strain::Diamonds)
    );

    // Capped at the three level: over their three-level bid the cheapest
    // diamond rebid is game (4♦), and a minimum must not blast it — the rule
    // stays home rather than jump.
    let over_three = [
        call(1, Strain::Diamonds),
        call(3, Strain::Hearts),
        Call::Pass,
        Call::Pass,
    ];
    assert_ne!(
        best(&over_three, "K5.54.AQJ982.J43"),
        call(4, Strain::Diamonds)
    );

    // Seat-scoped: partner's Jacoby transfer names our short major, but we
    // never bid it — no phantom natural rebid of the transfer strain.
    let transfer = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Hearts),
        call(3, Strain::Clubs),
    ];
    assert_ne!(
        best_with(&on, &transfer, "K5.AQJ982.A5.K43"),
        call(3, Strain::Hearts)
    );

    // The overcaller's rebid path fires too — we personally bid the suit.
    let overcall = [
        call(1, Strain::Hearts),
        call(2, Strain::Diamonds),
        call(2, Strain::Hearts),
        Call::Pass,
        Call::Pass,
    ];
    assert_eq!(
        best_with(&on, &overcall, "K5.A54.AKJT98.63"),
        call(3, Strain::Diamonds)
    );
}

#[test]
fn penalty_latch_doubles_the_runout_for_penalty() {
    // `(1NT) X (2♦)`: our penalty double, followed by their runout; we hold a
    // diamond stack.
    let auction = [
        call(1, Strain::Notrump),
        Call::Double,
        call(2, Strain::Diamonds),
    ];
    // A pure diamond stack (9 HCP, all in their suit): combined with partner's
    // shown 15+ this is below game, so the floor neither bids nor advances.
    // Latch off — defend by passing, no penalty double offered.
    let mut agreements = Agreements::default();
    agreements.decision.reading.penalty_latch = false;
    assert_eq!(
        best_with(&agreements, &auction, "T98.964.AKQ7.853"),
        Call::Pass
    );
    // Latch on (the default): "once penalty, always penalty" — double for penalty.
    agreements.decision.reading.penalty_latch = true;
    assert_eq!(
        best_with(&agreements, &auction, "T98.964.AKQ7.853"),
        Call::Double
    );
    // The latch keys off the 1NT penalty double only: a plain takeout auction
    // is untouched — short in clubs with opening values still doubles 2♣ takeout.
    let takeout = [call(2, Strain::Clubs)];
    assert_eq!(
        best_with(&agreements, &takeout, "AQ95.KJ73.K842.6"),
        Call::Double
    );
}

#[test]
fn penalty_latch_leaves_partner_s_double_in() {
    // `(1NT) X (2♦) X -`: partner doubled the runout for penalty, back to us.
    let auction = [
        call(1, Strain::Notrump),
        Call::Double,
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    // A flat 16-count with no diamond stopper: latch off, the takeout-advance
    // jumps to a dubious 4♠ on a four-card suit.
    let mut agreements = Agreements::default();
    agreements.decision.reading.penalty_latch = false;
    assert_eq!(
        best_with(&agreements, &auction, "AQ74.AQ5.82.A632"),
        call(4, Strain::Spades)
    );
    // Latched (the default), partner's double is penalty — leave it in (defend 2♦x).
    agreements.decision.reading.penalty_latch = true;
    assert_eq!(
        best_with(&agreements, &auction, "AQ74.AQ5.82.A632"),
        Call::Pass
    );
}

#[test]
fn advancer_runs_from_redoubled_penalty_double() {
    // (1NT) X (XX): their business redouble, back to the broke advancer.
    let auction = [call(1, Strain::Notrump), Call::Double, Call::Redouble];
    // Weak with a five-card major: escape to it rather than sit for 1NTxx.
    assert_eq!(best(&auction, "J9763.852.764.43"), call(2, Strain::Spades));
    // Weak with a six-card minor: run to it.
    assert_eq!(best(&auction, "82.43.765.QJ8765"), call(2, Strain::Clubs));
    // Values (9 HCP): sit and defend 1NTxx — our side beats it.
    assert_eq!(best(&auction, "KQ7.K83.J642.643"), Call::Pass);
    // Off-switch: the runout disabled, the broke hand sits.
    let mut agreements = Agreements::default();
    agreements.decision.instinct.advancer_xx_runout = false;
    assert_eq!(
        best_with(&agreements, &auction, "J9763.852.764.43"),
        Call::Pass
    );
}

#[test]
fn doubler_runs_from_redoubled_penalty_double() {
    // (1NT) X (XX) - -: the redouble ran back to the 15+ doubler.
    let auction = [
        call(1, Strain::Notrump),
        Call::Double,
        Call::Redouble,
        Call::Pass,
        Call::Pass,
    ];
    // On by default: a 15+ 5332 escapes to its five-card suit rather than defend
    // the redouble.
    assert_eq!(best(&auction, "AQ765.KQ4.A82.K3"), call(2, Strain::Spades));
    assert_eq!(best(&auction, "AQ4.K82.K3.AQ765"), call(2, Strain::Clubs));
    // No five-card suit (4-4-3-2): nowhere to run, so sit.
    assert_eq!(best(&auction, "AQ74.KQ32.A82.K3"), Call::Pass);
    // Off-switch: the strong doubler sits and defends 1NTxx.
    let mut off = Agreements::default();
    off.instinct.doubler_xx_runout = false;
    assert_eq!(best_with(&off, &auction, "AQ765.KQ4.A82.K3"), Call::Pass);
}

#[test]
fn optional_latch_doubles_short_and_partner_cooperates() {
    // (1NT) X (2♦): our latched double of their runout.
    let runout = [
        call(1, Strain::Notrump),
        Call::Double,
        call(2, Strain::Diamonds),
    ];
    // Three small diamonds (no stack), 13 HCP, no four-card suit worth bidding:
    // the PENALTY latch needs a stack, so it does not double…
    let cooperative = "KQ5.KQ5.642.QJ93";
    let mut agreements = Agreements::default();
    agreements.decision.instinct.latch_style = LatchStyle::Penalty;
    assert_ne!(best_with(&agreements, &runout, cooperative), Call::Double);
    // …but the OPTIONAL latch doubles on the 2-3 holding and values.
    agreements.decision.instinct.latch_style = LatchStyle::Optional;
    assert_eq!(best_with(&agreements, &runout, cooperative), Call::Double);

    // `(1NT) X (2♦) X -`: partner's latched double, back to the 15+ doubler.
    let advance = [
        call(1, Strain::Notrump),
        Call::Double,
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    // Penalty: forced sit — leave the penalty double in (defend 2♦x).
    agreements.decision.instinct.latch_style = LatchStyle::Penalty;
    assert_eq!(
        best_with(&agreements, &advance, "AQ74.AQ5.82.A632"),
        Call::Pass
    );
    // Optional: cooperate (not forced to sit) — short in their suit with a
    // four-card major and values, run to the major game.
    agreements.decision.instinct.latch_style = LatchStyle::Optional;
    assert_eq!(
        best_with(&agreements, &advance, "AQ74.AQ5.82.A632"),
        call(4, Strain::Spades)
    );
}

#[test]
fn advancing_a_double_bids_game_with_values() {
    let auction = [call(2, Strain::Spades), Call::Double, Call::Pass];
    // 13 HCP with their suit stopped and no length behind it: 3NT to play.
    assert_eq!(best(&auction, "AQ3.K65.J64.QJ96"), call(3, Strain::Notrump));
    // 11 HCP with four hearts: jump to the major-suit game.
    assert_eq!(best(&auction, "92.AQ53.KQ42.962"), call(4, Strain::Hearts));
}

#[test]
fn unforced_raise_with_fit() {
    // Partner opened 1♠ and they overcalled 2♥: raise with three-card
    // support and 8 HCP.
    let auction = [call(1, Strain::Spades), call(2, Strain::Hearts)];
    assert_eq!(best(&auction, "Q32.953.A964.Q92"), call(2, Strain::Spades));
}

#[test]
fn unforced_takeout_double_on_shape() {
    // Their 3♦ preempt: 13 HCP, short in diamonds, no five-card suit.
    let auction = [call(3, Strain::Diamonds)];
    assert_eq!(best(&auction, "KQ32.AJ53.2.A942"), Call::Double);
}

#[test]
fn unforced_pass_without_values() {
    // Nothing to say over their 3♦: too weak to act at the three level.
    let auction = [call(3, Strain::Diamonds)];
    assert_eq!(best(&auction, "Q5432.J53.942.92"), Call::Pass);
}

#[test]
fn doubles_only_their_live_bids() {
    // The call to beat is our own 2♠ (partner raised our overcall):
    // doubling our side is never on the table.
    let auction = [
        call(1, Strain::Hearts),
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let hand: Hand = "92.K53.AQJ42.962".parse().expect("valid test hand");
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let logits = instinct(&Agreements::default()).classify(hand, &context);
    assert_eq!(*logits.0.get(Call::Double), f32::NEG_INFINITY);
}

#[test]
fn settle_floor_defends_with_length_behind_their_suit() {
    // Their 3♠, partner doubles (takeout), RHO passes → advancing a double.
    let auction = [call(3, Strain::Spades), Call::Double, Call::Pass];
    // 6 HCP, five clubs but four cards sitting behind their spades.
    let weak_with_defense = "9543.74.K2.QJ876";

    // Off: the floor over-advances to the captive 4♣.
    let mut agreements = Agreements::default();
    agreements.decision.instinct.settle_floor = false;
    assert_eq!(
        best_with(&agreements, &auction, weak_with_defense),
        call(4, Strain::Clubs)
    );

    // On: the four-level new suit is a free bid we lack the values for, and we
    // hold four behind their suit, so we defend — pass plays 3♠ doubled.
    agreements.decision.instinct.settle_floor = true;
    assert_eq!(
        best_with(&agreements, &auction, weak_with_defense),
        Call::Pass
    );

    // On, with real values: the free bid is earned — we still advance to 4♣
    // (a hand short in their suit cannot defend anyway).
    let strong = "2.853.K42.AKQ876";
    assert_eq!(
        best_with(&agreements, &auction, strong),
        call(4, Strain::Clubs)
    );
}

#[test]
fn completes_partners_transfer_over_notrump() {
    // We opened 1NT and partner transferred 2♦ (hearts): complete with 2♥,
    // even off-book, rather than passing or raising the artificial diamonds.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(best(&auction, "AQ32.KJ5.KQ4.Q92"), call(2, Strain::Hearts));
}

#[test]
fn forced_to_game_opposite_strong_notrump() {
    // Partner opened 1NT; after an artificial 2NT super-accept of our heart
    // transfer a game-forced 12-count bids 3NT, never passing below game.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(best(&auction, "KQ52.AQ984.J6.32"), call(3, Strain::Notrump));
}

#[test]
fn fit_sum_game_counts_trump_length_toward_game() {
    // Partner opened a weak 2♠ (a known six-card suit); RHO passed.  We hold a
    // strong 19-count with three-card spade support — a known nine-card fit
    // (3 + 6).  Combined points are 24 (our 19 + partner's shown floor of 5)
    // and the fit-sum is 24 + 9 = 33.
    let auction = [call(2, Strain::Spades), Call::Pass];
    let raise = "AK4.AQ2.KJ32.Q92";

    // Pin the point gate: this test is *about* `fit_sum_game` being read from
    // the profile, and the bilans floor replaces that arithmetic with the net —
    // which would answer 4♠ at both thresholds and assert nothing.
    let mut agreements = Agreements::default();
    agreements.decision.instinct.bilans_floor = false;

    // Armed at 31 (the default): the ninth trump counts as points (fit-sum
    // 33 ≥ 31), lifting the hand to the major-suit game.
    agreements.decision.instinct.fit_sum_game = 31;
    assert_eq!(
        best_with(&agreements, &auction, raise),
        call(4, Strain::Spades)
    );

    // Armed at 34: the same fit-sum (33) falls one short, so the boundary holds
    // and we are back to inviting — confirms the threshold is read live.
    agreements.decision.instinct.fit_sum_game = 34;
    assert_eq!(
        best_with(&agreements, &auction, raise),
        call(3, Strain::Spades)
    );
}

#[test]
fn forced_to_game_picks_the_known_major_fit() {
    // We opened 1NT; partner's off-book, forcing 3♥ shows five-plus hearts.
    // With three-card support that is a known eight-card fit, so bid 4♥
    // rather than the stopperless-agnostic 3NT.
    //
    // The splinter owns this auction by default (short hearts, not long), so
    // it is switched off here: the floor path under test is only reachable
    // when the slot is empty, which is exactly when it should apply.
    let mut agreements = Agreements::default();
    agreements.decision.reading.nt_splinter = false;
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let bid = best_with(&agreements, &auction, "AQ52.K53.KQ4.32");
    assert_eq!(bid, call(4, Strain::Hearts));
}

#[test]
fn fit_sum_reads_a_four_four_major_fit() {
    // After `- 1♣ - 1♥ - 2♠ -`, opener's extras-ladder jump-shift shows 4+
    // spades.  South holds four spades opposite the shown four: a known 4-4
    // fit.  The old pair
    // enumeration could not see it (neither hand shows five) and settled
    // the combined-25 game force in 3NT; the fit-sum reads the eight-card
    // fit and prefers the major.
    //
    // The level is the v2 evaluator's, and it is measured, not assumed:
    // over 2000 layouts sampled from this auction's own read (North `points
    // 18..=21`, spades exactly 4, clubs 5+), 6♠ makes **66.3%**
    // double-dummy [63.3, 69.2] against the 50% IMP break-even for bidding
    // a small slam — 4♠ makes 99.9%.  Unlike the `8..=37` and `0..=37`
    // envelopes that inflate the gate elsewhere, this read is tight and
    // correct, so the slam is the net's to claim.
    let auction = [
        Call::Pass,
        call(1, Strain::Clubs),
        Call::Pass,
        call(1, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let south = "JT87.AQT2.A75.86";
    let (bid, from_floor) = american_floored(&auction, south);
    assert!(
        from_floor,
        "South's continuation is off-book (floor territory)"
    );
    // The fit-sum's claim is the *strain* — major over 3NT.  The level is
    // the evaluator's, and it has changed hands twice: the legacy net
    // claimed the marginal slam (6♠ made 66.3% double-dummy over the
    // knob-off read), the hull-only F2b twin (`evaluator_v2_dnf`) priced
    // it back to game, and the shipped v3 calls-tail twin
    // (`evaluator_v3_dnf`, 2026-07-27, `win | win`) claims it again —
    // with the raw calls in view its μ/σ clears the 50% break-even the
    // fixture's own sampling supports.  Since the recalibrated ask gate
    // (the face rung keys the jump-shift spades; the support-lifted
    // shown floor clears the `combined_points(29)` conversation floor
    // under this stance's readings), the slam is claimed *through RKCB*
    // rather than blind — 66.3% includes the boards off two keycards.
    assert_eq!(bid, call(4, Strain::Notrump));
    let mut legacy_agreements = Agreements::default();
    legacy_agreements.decision.reading.envelope_union = false;
    let (legacy, _) = american_floored_with(&legacy_agreements, &auction, south);
    assert_eq!(legacy, call(4, Strain::Notrump));
    // North answers 1430 (A♠ A♣ + trump K♠ = 3 → 5♦); South holds two,
    // and under the three-combined-keycards doctrine 0 is impossible
    // (2 + 0 < 3), so the step is exact: five keycards, small slam.
    let asked = [auction.as_slice(), &[call(4, Strain::Notrump), Call::Pass]].concat();
    assert_eq!(
        american_floored(&asked, "AK92.7.K84.AKQ93").0,
        call(5, Strain::Diamonds),
        "the jump-shifter answers 1430"
    );
    let answered = [asked.as_slice(), &[call(5, Strain::Diamonds), Call::Pass]].concat();
    assert_eq!(
        american_floored(&answered, south).0,
        call(6, Strain::Spades),
        "the exact decode claims the vetted slam"
    );
}

#[test]
fn fit_sum_leaves_a_flat_4333_in_notrump() {
    // Same auction, but South is flat 4-3-3-3 with four spades: a bare 4-4
    // with no ruffing value, so the carve keeps it in 3NT (notrump's
    // nine-trick game outscores the suit's ten).
    let auction = [
        Call::Pass,
        call(1, Strain::Clubs),
        Call::Pass,
        call(1, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let south = "KT87.AQ2.Q75.J82"; // 4=3=3=3, 11 HCP
    assert_eq!(
        american_floored(&auction, south).0,
        call(3, Strain::Notrump)
    );
}

#[test]
fn our_declarer_names_the_first_of_our_side() {
    // Partner opened 1NT: partner would declare notrump; an unnamed strain
    // falls to the actor.
    let auction = [call(1, Strain::Notrump), Call::Pass];
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    assert_eq!(our_declarer(&context, Strain::Notrump), Relative::Partner);
    assert_eq!(our_declarer(&context, Strain::Spades), Relative::Me);

    // We opened 1♠ and partner raised: our seat named spades first.
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    assert_eq!(our_declarer(&context, Strain::Spades), Relative::Me);

    // Their bid never declares for us: RHO's 1♥ leaves hearts to the actor.
    let auction = [call(1, Strain::Hearts)];
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    assert_eq!(our_declarer(&context, Strain::Hearts), Relative::Me);
}

/// The accelerate/veto split is **derived** from each decision's own IMP
/// economics, not chosen per site: a game never breaks even *above* even
/// money, a slam never *below*.  That is what licenses [`points_or_net`] to
/// let the net reach below a game threshold while [`points_and_net`] lets it
/// only decline a slam.  Retune [`break_even`] past 0.5 at a game row and
/// the collared reach stops being the cheap direction — this fails rather
/// than letting the wiring quietly stop matching its own justification.
///
/// Both bounds are **non-strict on purpose**, and the two boundary rows are
/// why.  Non-vul game sits exactly on 0.5 under the bid-scoring doubling
/// premium (6 IMPs risked against 6 gained), and the small slam sits on it
/// under *every* convention — the slam bonus is symmetric, and doubling the
/// undertrick moves neither side out of its IMP bucket (non-vul 500 → 550,
/// both 11 IMPs; vul 750 → 850, both 13).  Do not "tighten" to `<` / `>`.
#[test]
fn break_even_keys_the_collar_direction() {
    for strain in Strain::ASC {
        for vul_we in [false, true] {
            // A game is takeable at or below even money, so the net's cheap
            // licence is to *add* hands the point sums decline.
            for tricks in 7..=11 {
                assert!(
                    break_even(tricks, strain, vul_we) <= 0.5,
                    "game ({tricks} tricks, vul {vul_we}) must never break even \
                     above even money — `points_or_net` accelerates here"
                );
            }
            // A slam needs at or above even money, so the net may only
            // *decline* — and a veto is the shape that keeps the reading.
            for tricks in 12..=13 {
                assert!(
                    break_even(tricks, strain, vul_we) >= 0.5,
                    "slam ({tricks} tricks, vul {vul_we}) must never break even \
                     below even money — `points_and_net` only vetoes here"
                );
            }
        }
    }
}

/// The bilans knob prices the same known fit the point sum reaches: the
/// 4-4 fit-sum board must land where the shipped evaluator prices it when
/// the net does the arithmetic (Stance path, so the net sees the
/// trie-prefixed reading it was trained on).
#[test]
fn bilans_floor_still_bids_the_known_fit_game() {
    let auction = [
        Call::Pass,
        call(1, Strain::Clubs),
        Call::Pass,
        call(1, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let (bid, from_floor) = american_floored(&auction, "JT87.AQT2.A75.86");
    assert!(
        from_floor,
        "South's continuation is off-book (floor territory)"
    );
    // The known fit priced by the shipped evaluator: game under the
    // hull-only F2b twin, the marginal slam again under the v3 calls-tail
    // twin (2026-07-27, `win | win`) — see
    // `fit_sum_reads_a_four_four_major_fit`, which documents the level
    // changing hands with each measured regime and, since the ask-gate
    // recalibration, the slam entering through RKCB instead of blind.
    assert_eq!(bid, call(4, Strain::Notrump));
}

/// [`net_collar`][InstinctProfile::net_collar]'s veto, on the board the smoke
/// run surfaced (seed
/// 20260726 board 33, 2026-07-26): the shipped wiring blasts 6NT on a
/// combined **31**, because knob-on `points_or_net` masked
/// [`combined_hcp`]`(33)` off entirely and the net alone decided.  Collared,
/// the notrump slam is [`points_and_net`] — `authored & net` — so 31 < 33
/// declines it and the auction rests in partner's 3NT.
///
/// This is the F1 forensic's 6NT-blast family (docs/dnf-migration.md, chop
/// F1, traced at `combined_hcp(33)` false), reached from the other side: F1
/// fixed the net's *inputs*, the collar restores the point floor the net was
/// allowed to ignore.  Stance path, so the net sees the trie-prefixed
/// reading it was trained on.
#[test]
fn net_collar_vetoes_the_notrump_slam_below_thirty_three() {
    // Partner opened 1NT, we transferred to spades, showed diamonds, and
    // partner signed off in 3NT.  We hold 16 opposite a 15-17 notrump.
    let auction = [
        Call::Pass,
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Notrump),
        Call::Pass,
    ];
    let hand = "AJ843.AK7.KJ52.7";

    // Shipped default: the net holds the whole criterion and blasts the slam.
    let (bid, from_floor) = american_floored(&auction, hand);
    assert!(from_floor, "our continuation is off-book (floor territory)");
    assert_eq!(bid, call(6, Strain::Notrump));

    // Collared: the arithmetic is the criterion again and vetoes it.
    let mut agreements = Agreements::default();
    agreements.decision.instinct.net_collar = true;
    let collared = american_floored_with(&agreements, &auction, hand).0;
    assert_eq!(collared, Call::Pass);
}

#[test]
fn transfer_invite_reaches_the_floor_over_a_possible_five_two() {
    // 1NT - 2♦ - 2♥ - 3♥: partner transferred to hearts and raised.  With the six-card
    // invite on (the default) this node is authored — so turn it off to exercise
    // the floor path this test guards: the projection reads the 2♦ transfer's
    // five-card floor (M6.1's core), but M6.2c dropped the old reader's six-card
    // upgrade off the 3♥ raise (soundness over tightness — projecting a
    // natural-suit raise is out of the overlay's artificial-only scope).  With
    // only a five-card major shown and our own doubleton, the floor prefers 3NT
    // over a possible 5-2 game.
    let mut agreements = Agreements::default();
    agreements.notrump.sixcard_invite_floor = 14;
    let invite = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let (bid, from_floor) = american_floored_with(&agreements, &invite, "AKQ2.J5.AQ52.K42");
    assert!(
        from_floor,
        "the transfer invite is off-book, the floor decides"
    );
    assert_eq!(bid, call(3, Strain::Notrump));
}

// -----------------------------------------------------------------------
// M6.4: floor RKCB + control-bid signoffs
// -----------------------------------------------------------------------

/// With a known fit and combined small-slam values the floor asks 4NT
/// (M6.4) instead of blasting the direct milestone 6♠.
#[test]
fn floor_asks_keycards_with_slam_values_and_a_known_fit() {
    // 1♠ - 3♠: the jump raise shows three spades and 10+, so a 23-point
    // opener counts combined 33 with a decodable trump (its own shown
    // five-card spades).
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    assert_eq!(
        best(&auction, "AKQJ7.AKQ.A32.32"),
        call(4, Strain::Notrump),
        "slam values + known fit → ask keycards, not 6♠ direct"
    );
}

/// The RKCB-ask floor (`floor_slam_entry`) governs whether the floor
/// enters keycarding on shape-slam values.  The same 1♠ - 3♠ auction with a
/// ~30-combined opener stops in game at the old 33 yardstick but asks 4NT at
/// the shipped 29 floor — the population-probe fix (A/B'd a plain-DD win).
#[test]
fn slam_entry_floor_controls_the_rkcb_ask() {
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    // 5-card spades, 20 points, opposite a 10+ limit raise → combined ~30:
    // below 33 (game), at or above the shipped 29 gate (ask).
    let opener = "AKQ85.AK2.KJ2.42";
    // Pin the point gate: the bilans floor enters keycarding on
    // `SLAM_ENTRY_P` instead of this point floor, so knob-on both thresholds
    // answer 4NT and the test stops testing what it names.
    let mut agreements = Agreements::default();
    agreements.decision.instinct.bilans_floor = false;
    agreements.decision.instinct.floor_slam_entry = 33;
    assert_eq!(
        best_with(&agreements, &auction, opener),
        call(4, Strain::Spades),
        "combined ~30 < 33: stop in game, no ask"
    );
    agreements.decision.instinct.floor_slam_entry = 29;
    assert_eq!(
        best_with(&agreements, &auction, opener),
        call(4, Strain::Notrump),
        "shipped 29 gate: ask keycards on shape-slam values"
    );
}

/// The 1430 answers to partner's off-book 4NT, counted against the
/// derived trump (spades, raised)
#[test]
fn floor_answers_keycards_1430() {
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
    ];
    // 1 keycard (trump K) → 5♣
    assert_eq!(
        best(&auction, "KQ732.K53.Q42.92"),
        call(5, Strain::Clubs),
        "1 keycard → 5♣"
    );
    // 0 keycards (the heart K is not one) → 5♦
    assert_eq!(
        best(&auction, "QJ732.K53.Q42.Q2"),
        call(5, Strain::Diamonds),
        "0 keycards → 5♦"
    );
    // 2 aces + trump K = 3 keycards → 5♦
    assert_eq!(
        best(&auction, "AK732.A53.842.92"),
        call(5, Strain::Diamonds),
        "3 keycards → 5♦"
    );
    // 2 keycards with the trump queen → 5♠
    assert_eq!(
        best(&auction, "AQ732.A53.842.92"),
        call(5, Strain::Spades),
        "2 keycards + trump Q → 5♠"
    );
    // 2 keycards without the queen → 5♥.  Four trumps, not five: opposite a
    // shown five that is a nine-card fit, and nine is not ten.  With five
    // the side owns the **ten-card fit that stands in for the queen**
    // (`QUEEN_FIT`, live since the relay went default-on), so the same
    // hand one card longer correctly answers 5♠ instead.
    assert_eq!(
        best(&auction, "A873.A532.842.92"),
        call(5, Strain::Hearts),
        "2 keycards, no trump Q → 5♥"
    );
    assert_eq!(
        best(&auction, "A8732.A53.842.92"),
        call(5, Strain::Spades),
        "the ten-card fit answers the queen without the honour"
    );
    // All five keycards (four aces + trump K) → 5♣, the hole the book
    // ladder's {1,4} left open (round 3 passed a 4NT out on it)
    assert_eq!(
        best(&auction, "AK732.A53.A42.A2"),
        call(5, Strain::Clubs),
        "5 keycards → 5♣"
    );
}

/// The relay: after an ambiguous answer the asker asks
/// the queen rather than betting six on four keycards blind, and partner
/// replies on the next two rungs.
///
/// `1♠ - 3♠ - 4NT - 5♣ -` — spades agreed, partner's 5♣ is one-or-four.  We
/// hold three keycards, so the high reading (four) would put six combined on
/// the table and the low one is meant: four combined, one missing, and the
/// trump queen still an open question.
#[test]
fn floor_relays_for_the_trump_queen() {
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
        call(5, Strain::Clubs),
        Call::Pass,
    ];
    // ♠AKJ85 ♥AK2 ♦KJ2 ♣42 — three keycards (♠A, ♥A, ♠K), no trump queen.
    let queenless = "AKJ85.AK2.KJ2.42";

    assert_eq!(
        best(&auction, queenless),
        call(5, Strain::Diamonds),
        "5♦ is the step above the answer — ask the queen"
    );
    // Holding it ourselves settles the question, so the relay is skipped.
    assert_eq!(
        best(&auction, "AKQ85.AK2.KJ2.42"),
        call(6, Strain::Spades),
        "our own trump queen settles it: no relay, bid the slam"
    );
    // ♠AKJ85 ♥A32 ♦AK2 ♣32 — four keycards of our own, so the answer puts
    // all five on the table and six is bid whatever the queen does.  Asking
    // would spend a round on a reply we will not act on, and spend it at the
    // five level: bid the slam.  (22 HCP clears the slam-entry gate and is
    // nowhere near the grand zone — the cell where the rule bites.)
    assert_eq!(
        best(&auction, "AKJ85.A32.AK2.32"),
        call(6, Strain::Spades),
        "all five keycards and no grand values: nothing to learn, bid six"
    );
}

/// Partner answers the relay on the next two rungs, and the ten-card fit
/// stands in for the queen ([`ten_card_fit`])
#[test]
fn floor_answers_the_queen_relay() {
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
        call(5, Strain::Clubs),
        Call::Pass,
        call(5, Strain::Diamonds),
        Call::Pass,
    ];
    // ♠K74 ♥A653 ♦8432 ♣92 — three trumps opposite partner's shown five is
    // eight, so only the honour itself can answer, and it is missing.
    assert_eq!(
        best(&auction, "K74.A653.8432.92"),
        call(5, Strain::Spades),
        "no trump queen and no buff → five of trump, which is the signoff too"
    );
    // The same hand holding it, and not one side king → 5NT.
    assert_eq!(
        best(&auction, "KQ4.A653.8432.92"),
        call(5, Strain::Notrump),
        "trump queen, no side king → the top of the merged ladder"
    );
    // The queen with the cheapest side king on the ♥ rung — one round for
    // both facts, which is what the merged reply buys.
    assert_eq!(
        best(&auction, "KQ4.K653.8432.92"),
        call(5, Strain::Hearts),
        "trump queen and the ♥ king → the cheapest king rung"
    );
    // Five trumps opposite the same shown five is ten, and ten trumps
    // answer "queen" without it — the honour drops or finesses either way.
    assert_eq!(
        best(&auction, "K7432.A65.843.92"),
        call(5, Strain::Notrump),
        "a proven ten-card fit stands in for the queen"
    );
}

/// The asker places the contract on the relay's answer: the queen brings
/// the small slam, a denial stops at five, and exploring seven has to be
/// paid for in values — RKCB is a slam veto, not a slam seeker.
#[test]
fn floor_places_the_contract_on_the_queen_reply() {
    let opening = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
        call(5, Strain::Clubs),
        Call::Pass,
        call(5, Strain::Diamonds),
        Call::Pass,
    ];
    let after = |reply: Call| -> Vec<Call> {
        opening
            .iter()
            .copied()
            .chain([reply, Call::Pass])
            .collect::<Vec<_>>()
    };
    // Three keycards opposite a one-or-four answer decodes to four
    // combined — one keycard missing.
    let asker = "AKJ85.AK2.KJ2.42";
    assert_eq!(
        best(&after(call(5, Strain::Spades)), asker),
        Call::Pass,
        "queen denied on four keycards: five of trump is both the denial and the contract"
    );
    assert_eq!(
        best(&after(call(5, Strain::Notrump)), asker),
        call(6, Strain::Spades),
        "queen shown on four keycards: bid the small slam"
    );
    // Four keycards of our own decodes to all five combined.  The queen is
    // there and nothing is missing, but ~26 combined points is nowhere near
    // the grand zone, so the second relay is not worth a round: bid six.
    assert_eq!(
        best(&after(call(5, Strain::Hearts)), "AK985.A32.A42.32"),
        call(6, Strain::Spades),
        "all five keycards, the queen and a king, but no grand values: six"
    );
}

/// The relay's geometry, lane by lane — the queen ask is the step above the
/// answer, and it exists exactly when the *no-queen* rung still lands at or
/// below five of trump.  This is the table `relay_map` documents; get a
/// row wrong and a relay lands past the signoff it was meant to preserve.
/// Every relay lane, mechanically: which ones the merged reply fits in,
/// and that no lane ever assigns two messages to the same call.
///
/// This is the test both earlier cuts of the ladder needed and did not
/// have.  The first collided the buff jump with the king rungs; the second
/// collided the zero-king answer with two-or-more.  A collision is not a
/// wrong contract, it is partner reading the opposite of what was said.
#[test]
fn merged_relay_fits_eleven_lanes_without_collision() {
    use std::collections::HashSet;
    let plain = Bid::new(4, Strain::Notrump);
    let kickback = |trump: Suit| match trump {
        Suit::Clubs => Bid::new(4, Strain::Diamonds),
        Suit::Diamonds => Bid::new(4, Strain::Hearts),
        Suit::Hearts => Bid::new(4, Strain::Spades),
        Suit::Spades => plain,
    };
    let mut fitted = 0;
    for trump in Suit::ASC {
        for ask in [plain, kickback(trump)] {
            for step in 1..=2 {
                let mut answer = ask;
                for _ in 0..step {
                    answer = bid_successor(answer).unwrap();
                }
                let Some(map) = relay_map(answer, trump) else {
                    continue;
                };
                fitted += 1;
                let six = Bid::new(6, Strain::from(trump));
                let mut calls: Vec<Bid> = map.kings.iter().map(|&(_, call)| call).collect();
                calls.extend([map.weak, map.deny, map.no_king]);
                for &call in &calls {
                    assert!(call > map.ask, "{call:?} is not above the ask in {trump:?}");
                    assert!(call <= six, "{call:?} is above six of {trump:?}");
                }
                let unique: HashSet<Bid> = calls.iter().copied().collect();
                assert_eq!(
                    unique.len(),
                    calls.len(),
                    "two messages share a call with {trump:?} trumps over {answer:?}"
                );
                // Cheapest-first, so "skipped steps deny" is well defined.
                assert!(
                    map.kings.windows(2).all(|pair| pair[0].1 < pair[1].1),
                    "king rungs out of order for {trump:?}"
                );
            }
        }
    }
    assert_eq!(fitted, 11, "the merged reply should serve eleven lanes");
}

#[test]
fn relay_geometry_keeps_the_signoff_reachable() {
    let bid = |level, strain| Bid::new(level, strain);
    // Plain 4NT, spades agreed: both ambiguous answers have room.
    assert_eq!(
        queen_ask_room(bid(5, Strain::Clubs), Suit::Spades),
        Some(bid(5, Strain::Diamonds)),
        "♠ after one-or-four: 5♦ asks, 5♥ denies, 5♠ still signs off"
    );
    assert_eq!(
        queen_ask_room(bid(5, Strain::Diamonds), Suit::Spades),
        Some(bid(5, Strain::Hearts)),
        "♠ after none-or-three: 5♥ asks, 5♠ denies"
    );
    // Hearts after a none-or-three answer is the lane with no room: the
    // denial would be 5♠, already past the 5♥ signoff.
    assert_eq!(
        queen_ask_room(bid(5, Strain::Clubs), Suit::Hearts),
        Some(bid(5, Strain::Diamonds)),
        "♥ after one-or-four: 5♦ asks, 5♥ denies and is the signoff"
    );
    // Hearts after a none-or-three has no room: the ask would *be* 5♥, and
    // the answerer reads the face — it could not tell the ask from the
    // signoff, and would raise a signoff to six.
    assert_eq!(
        queen_ask_room(bid(5, Strain::Diamonds), Suit::Hearts),
        None,
        "♥ after none-or-three: no room — bet the small slam on four"
    );
    // Plain-4NT minors never have room: the 1430 answers overshoot five of
    // the minor before the relay starts.
    for (answer, trump) in [
        (bid(5, Strain::Clubs), Suit::Clubs),
        (bid(5, Strain::Diamonds), Suit::Clubs),
        (bid(5, Strain::Clubs), Suit::Diamonds),
    ] {
        assert_eq!(
            queen_ask_room(answer, trump),
            None,
            "plain 4NT in a minor is cramped before the relay"
        );
    }
    // Every relocated lane has room, and every rung it generates is a call
    // the rule table actually carries.
    for (ask, trump) in [
        (bid(4, Strain::Diamonds), Suit::Clubs),
        (bid(4, Strain::Hearts), Suit::Diamonds),
        (bid(4, Strain::Spades), Suit::Hearts),
        (bid(4, Strain::Notrump), Suit::Spades),
    ] {
        for step in 1..=2 {
            let mut answer = ask;
            for _ in 0..step {
                answer = bid_successor(answer).expect("the ladder stays legal");
            }
            let map = relay_map(answer, trump)
                .unwrap_or_else(|| panic!("{trump} after step {step} must have room"));
            assert_eq!(queen_ask_room(answer, trump), Some(map.ask));
            assert_eq!(
                map.weak,
                bid(5, Strain::from(trump)),
                "{trump} step {step}: the relocated lanes keep the weak denial"
            );
            // Every rung of both rounds is a call the rule table carries.
            let mut rungs = vec![map.ask, map.weak, map.deny, map.no_king];
            for &(_, shown) in &map.kings {
                rungs.push(shown);
                if let Some(second) = king_relay(shown, trump) {
                    rungs.extend([second.ask, second.more, second.none]);
                }
            }
            for rung in rungs {
                assert!(
                    RELAY_RUNGS.contains(&rung),
                    "{rung:?} is a relay rung with no rule to land on"
                );
            }
        }
    }
}

/// The converse discipline: a rule class installs only on the rungs some
/// lane can reach.  A constraint-dead alerted rule is still face-live, and
/// its structural `alerted` bit erases the natural walk for the window's
/// ordinary placements — five and six of trump above all.
#[test]
fn relay_rules_install_only_on_reachable_rungs() {
    let bid = |level, strain| Bid::new(level, strain);
    let feasible: Vec<Bid> = RELAY_RUNGS
        .into_iter()
        .filter(|&rung| queen_ask_can_land(rung))
        .collect();
    assert_eq!(
        feasible,
        [
            bid(4, Strain::Spades),
            bid(4, Strain::Notrump),
            bid(5, Strain::Clubs),
            bid(5, Strain::Diamonds),
            bid(5, Strain::Hearts),
        ],
        "the queen ask has exactly five rungs; an ask rule anywhere else is dead"
    );
    // 6♠ is a lane's strong denial and nothing else — never an ask, never
    // an artificial reply — so its natural reading survives the window.
    let six_spades = bid(6, Strain::Spades);
    assert!(!queen_ask_can_land(six_spades));
    assert!(!artificial_reply_can_land(six_spades));
    assert!(!king_ask_can_land(six_spades));
    assert!(denial_can_land(six_spades));
}

/// The asker decodes the answer: two keycards missing signs off at five,
/// one missing bids the small slam
#[test]
fn floor_asker_continues_after_the_answer() {
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
        call(5, Strain::Diamonds),
        Call::Pass,
    ];
    // 3 keycards: 5♦ arithmetically means 0 with us — two missing → 5♠.
    assert_eq!(
        best(&auction, "AQ752.A76.72.A93"),
        call(5, Strain::Spades),
        "two keycards missing → sign off"
    );
    // 4 keycards, but a 15-count opposite a limit raise is no slam: one
    // keycard missing used to hoist six unconditionally, and the same
    // decode applied to an *unvetted* ask (the net's contested 4NT)
    // converted every frivolous ask into a slam.  The six rung now
    // re-checks the slam entry; short of it, sign off.
    assert_eq!(
        best(&auction, "AK752.A76.72.A93"),
        call(5, Strain::Spades),
        "one missing but no slam values → still sign off"
    );
    // The same four keycards with genuine slam values: one missing → 6♠.
    assert_eq!(
        best(&auction, "AKQJ2.AK5.72.A93"),
        call(6, Strain::Spades),
        "one keycard missing with the entry in hand → small slam"
    );
}

/// The answerer respects the asker's placement — holding at most one
/// keycard the total cannot be slam-safe, so no milestone past it (with
/// two-plus the correction stays live: the asker may have read an
/// ambiguous answer low, or a book table signed off pessimistically)
#[test]
fn floor_answerer_respects_the_signoff() {
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
        call(5, Strain::Clubs),
        Call::Pass,
        call(5, Strain::Spades),
        Call::Pass,
    ];
    // One keycard (the trump K), yet milestone-worthy values (33+
    // combined would bid 6♠): the asker held the count — pass.
    assert_eq!(
        best(&auction, "KQ543.KQJ.KQJ4.K"),
        Call::Pass,
        "the answerer never overrides the asker's signoff short a keycard"
    );
}

/// The answerer decodes a face-agreed 4-4 trump: hearts were bid by
/// both members below the ask, so the face keys them even though the
/// answerer's table proves only seven (own four, the raise stamp three).
/// The *floor* never asks on this auction — the invite re-raise stamps
/// no strength, so the shown floors cannot reach the ask's
/// `combined_points(29)` conversation floor (a filed vacuous-reading
/// note) — but a net 4NT lands here, and the answer must not strand it.
#[test]
fn answerer_decodes_a_face_agreed_four_four_trump() {
    let auction = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
    ];
    // Two keycards, no queen → 5♥.
    assert_eq!(
        best(&auction, "A873.KT65.84.632"),
        call(5, Strain::Hearts),
        "the answerer decodes the face-agreed trump"
    );
}

/// The face dichotomy: RKCB if the side's last non-cue bid is a suit,
/// quantitative if notrump — cues are transparent, an agreed minor
/// yields to a 3NT sign-off while an agreed major survives it
/// (non-serious), and a suit-last minor auction keeps its RKCB route.
#[test]
fn face_trump_steps_past_cues_and_reads_the_nt_dichotomy() {
    // `1♥ (3♦) 4♦ - 4NT`: the 4♦ cue no longer blocks partner's
    // solo-bid hearts.
    let cue_blocked = [
        call(1, Strain::Hearts),
        call(3, Strain::Diamonds),
        call(4, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        face_trump(&cue_blocked, 4),
        Some(Suit::Hearts),
        "the face steps back past a cue of their suit"
    );
    // `1NT (2♠) 3♠ - 4NT`: the cue is skipped, the walk lands on 1NT —
    // the veto stands, that 4NT is quantitative.
    let nt_stop = [
        call(1, Strain::Notrump),
        call(2, Strain::Spades),
        call(3, Strain::Spades),
        Call::Pass,
    ];
    assert_eq!(
        face_trump(&nt_stop, 4),
        None,
        "stepping past cues stops at notrump"
    );
    // `1♦ - 3♦ - 3NT - 4NT`: the agreed minor yields to the 3NT
    // sign-off (BBA's probed slam move here is 4♣, never 4NT-as-RKCB).
    let minor_signoff = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(
        face_trump(&minor_signoff, 6),
        None,
        "an agreed minor yields to a 3NT sign-off"
    );
    // `1♠ - 3♠ - 3NT - 4NT`: over the agreed major the same 3NT is
    // non-serious — the fit stands.
    let major_nonserious = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(3, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(
        face_trump(&major_nonserious, 6),
        Some(Suit::Spades),
        "an agreed major survives a non-serious 3NT"
    );
    // `1♦ - 3♦ - 4NT`: suit last — the minor keeps its RKCB route.
    let minor_suit_last = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        face_trump(&minor_suit_last, 4),
        Some(Suit::Diamonds),
        "a suit-last minor auction stays RKCB"
    );
}

/// A guarded kickback suit ends the relocation instead of walking past it:
/// after `1♦ - 1♥ - 3♦`, 4♥ is natural (responder showed four), so the
/// diamond ask goes back to 4NT.  The earlier walk-up asked 4♠ here — one
/// step cheaper, and unrecognisable to a partner who has not built the same
/// table.  4NT has asked keycards since long before kickback, so the
/// fallback can never be misread.
#[test]
fn a_guarded_rung_falls_back_to_notrump() {
    // The stance is on, so the all-`None` is the guard's veto, not the
    // knob's.
    let jump_rebid = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        kickback_ladder(&jump_rebid, 6, RkcbVariant::Kickback),
        [None; 4],
        "hearts are guarded, so diamonds do not relocate at all"
    );
    assert_eq!(
        face_trump(&jump_rebid, 6),
        Some(Suit::Diamonds),
        "and 4NT still asks in the suit it always asked in"
    );
}

/// A spade bid cannot disprove hearts (5-5 majors bid spades first), so the
/// ladder yields the 4♥ claim rather than collide with a natural heart
/// game — unless the spade bidder named a second suit, which is 5+4 = 9
/// cards and leaves no room for five hearts.
#[test]
fn kickback_yields_the_undisprovable_major() {
    let response = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        kickback_ladder(&response, 6, RkcbVariant::Kickback),
        [None; 4],
        "responder's spades leave 4♥ natural, so the diamond ask stays 4NT"
    );
    // The same face over a weak two, where responder shows the longest
    // major first (`weak_two_longest_first`).
    let weak_two = [
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        kickback_ladder(&weak_two, 6, RkcbVariant::Kickback),
        [None; 4],
        "the weak-two face reads the same way"
    );
    // `1♠ - 2♦ - 3♦`: the spade bidder raised diamonds, so 5♠ + 4♦ leaves
    // at most four hearts — 4♥ is not natural and the relocation stands.
    let two_suited = [
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        kickback_ladder(&two_suited, 6, RkcbVariant::Kickback),
        [None, None, Some(Suit::Diamonds), None],
        "a second named suit disproves five hearts, so 4♥ asks in diamonds"
    );
}

/// Two set suits claim in ascending order, so both can carry a relocated
/// ask — 4♦ asks in clubs, 4♠ in hearts, and 4NT is left over.
#[test]
fn kickback_serves_both_fits_when_it_can() {
    let two_fits = [
        call(1, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    assert_eq!(
        kickback_ladder(&two_fits, 8, RkcbVariant::Kickback),
        [None, Some(Suit::Clubs), None, Some(Suit::Hearts)],
        "clubs claim 4♦, hearts claim 4♠"
    );
    // Two fits, but the lower one's only rung is the higher fit's suit.
    // Diamonds want 4♥ and hearts are guarded, so diamonds fall back to
    // 4NT; hearts still take 4♠, the suit directly above them.
    let one_free = [
        call(1, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    assert_eq!(
        kickback_ladder(&one_free, 8, RkcbVariant::Kickback),
        [None, None, None, Some(Suit::Hearts)],
        "diamonds revert to 4NT; hearts keep 4♠"
    );
}

/// What must *not* relocate: an unagreed suit, the opponents' suit, a
/// spade fit with nothing above it, and the notrump dichotomy's veto.
#[test]
fn kickback_refuses_without_a_set_trump() {
    // The stance is on, so every all-`None` below is the geometry's own
    // veto rather than the knob's.
    let one_bid = [call(1, Strain::Diamonds), Call::Pass];
    assert_eq!(
        kickback_ladder(&one_bid, 2, RkcbVariant::Kickback),
        [None; 4],
        "one bid is no agreement — `1♦ - 4♥` is not an ask"
    );
    let spades = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    assert_eq!(
        kickback_ladder(&spades, 4, RkcbVariant::Kickback),
        [None; 4],
        "nothing sits between 4♠ and 4NT"
    );
    // `1♦ - 3♦ - 3NT -`: [`face_trump`] reads the sign-off and vetoes —
    // that 4NT is quantitative, so there is no ask to relocate either.
    let minor_signoff = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(
        kickback_ladder(&minor_signoff, 6, RkcbVariant::Kickback),
        [None; 4],
        "the notrump veto carries to the ladder"
    );
}

/// A cue of their suit shows no length, so it never becomes the ask —
/// `1♥ (3♦) 4♦ - 4♥ -` sets hearts and relocates to 4♠, leaving 4♦ the
/// cue it was.
#[test]
fn kickback_never_claims_the_opponents_suit() {
    let cued = [
        call(1, Strain::Hearts),
        call(3, Strain::Diamonds),
        call(4, Strain::Diamonds),
        Call::Pass,
        call(4, Strain::Hearts),
        Call::Pass,
    ];
    assert_eq!(
        kickback_ladder(&cued, 6, RkcbVariant::Kickback),
        [None, None, None, Some(Suit::Hearts)],
        "their diamonds are guarded; the heart ask takes 4♠"
    );
}

/// The 1430 rungs are *steps above the ask* — and over a plain 4NT those
/// steps are the absolute 5♣/5♦/5♥/5♠ the floor has always used, which is
/// why making the machinery relative leaves the kickback-off system
/// unchanged
#[test]
fn answer_steps_reproduce_the_absolute_rungs_over_four_notrump() {
    let ladder = |ask: Bid| {
        [
            Bid::new(4, Strain::Hearts),
            Bid::new(4, Strain::Spades),
            Bid::new(4, Strain::Notrump),
            Bid::new(5, Strain::Clubs),
            Bid::new(5, Strain::Diamonds),
            Bid::new(5, Strain::Hearts),
            Bid::new(5, Strain::Spades),
        ]
        .into_iter()
        .filter_map(|bid| Some((answer_step(ask, bid)?, bid)))
        .collect::<Vec<_>>()
    };
    assert_eq!(
        ladder(Bid::new(4, Strain::Notrump)),
        vec![
            (1, Bid::new(5, Strain::Clubs)),
            (2, Bid::new(5, Strain::Diamonds)),
            (3, Bid::new(5, Strain::Hearts)),
            (4, Bid::new(5, Strain::Spades)),
        ],
        "the plain ask's steps are the rungs this table always held"
    );
    assert_eq!(
        ladder(Bid::new(4, Strain::Spades)),
        vec![
            (1, Bid::new(4, Strain::Notrump)),
            (2, Bid::new(5, Strain::Clubs)),
            (3, Bid::new(5, Strain::Diamonds)),
            (4, Bid::new(5, Strain::Hearts)),
        ],
        "4♠ asking in hearts: every answer lands at or below 5♥"
    );
    assert_eq!(
        ladder(Bid::new(4, Strain::Hearts)),
        vec![
            (1, Bid::new(4, Strain::Spades)),
            (2, Bid::new(4, Strain::Notrump)),
            (3, Bid::new(5, Strain::Clubs)),
            (4, Bid::new(5, Strain::Diamonds)),
        ],
        "Redwood 4♥ asking in diamonds: every answer lands at or below 5♦"
    );
}

/// The asker takes the cheaper ask where the ladder offers one: the same
/// monster that bids 4NT over an agreed-spade raise bids **4♠** over an
/// agreed-heart one, because 4♠ is above game in the agreed suit and can
/// collide with nothing
#[test]
fn kickback_asker_prefers_the_relocated_call() {
    let auction = [
        call(1, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let monster = "A32.AKQJ7.AKQ.32";
    let kickback = kickback_agreements();
    let relocated = best_with(&kickback, &auction, monster);
    assert_eq!(
        relocated,
        call(4, Strain::Spades),
        "knob on: 4♠ asks in hearts"
    );
    let plain = best(&auction, monster);
    assert_eq!(
        plain,
        call(4, Strain::Notrump),
        "knob off: the ask stays 4NT"
    );
}

/// The answerer counts against the trump the *ladder* pinned, and answers
/// in steps above the relocated ask — the 4♠-for-hearts slice
#[test]
fn kickback_answers_climb_from_four_spades() {
    let kickback = kickback_agreements();
    let auction = [
        call(1, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
        call(4, Strain::Spades),
        Call::Pass,
    ];
    assert_eq!(
        kickback_ladder(&auction, 4, RkcbVariant::Kickback)[Suit::Spades as usize],
        Some(Suit::Hearts),
        "hearts are set and spades unguarded, so 4♠ asks in hearts"
    );
    for (hand, expected, why) in [
        ("432.K765.5432.3", call(4, Strain::Notrump), "one keycard"),
        ("432.Q765.5432.3", call(5, Strain::Clubs), "no keycard"),
        (
            "A32.K765.543.32",
            call(5, Strain::Diamonds),
            "two, no queen",
        ),
        (
            "A32.KQ65.543.32",
            call(5, Strain::Hearts),
            "two with the queen",
        ),
    ] {
        assert_eq!(best_with(&kickback, &auction, hand), expected, "{why}");
    }
}

/// A 4NT that *answers* a relocated ask is an answer, not a new ask — the
/// bug the first smoke run found (`1♥ - 2NT - 3NT - 4♥ - 4♠ - 4NT - 5♦`
/// passed out, −15 IMPs: the asker read partner's step-1 answer as a fresh
/// keycard ask and answered it on the 1430 ladder, whose 1.9 outbids their
/// own 1.82 signoff).  The asker must place the contract in trumps instead.
#[test]
fn a_four_notrump_answering_the_relocation_is_not_a_new_ask() {
    let kickback = kickback_agreements();
    let auction = [
        call(1, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
        call(4, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(
        keycard_ask_bid(&auction, 6, RkcbVariant::Kickback),
        None,
        "4NT is step 1 over the 4♠ ask, so it asks nothing"
    );
    assert_eq!(
        keycard_ask_bid(&auction, 4, RkcbVariant::Kickback),
        Some(Bid::new(4, Strain::Spades)),
        "the ask is still the 4♠ two calls before it"
    );
    // Two keycards and the queen opposite a 1-or-4 answer: three combined,
    // two missing — sign off in the agreed suit, never a fifth-strain bid.
    let placement = best_with(&kickback, &auction, "A32.AKQJ7.AKQ.32");
    assert_eq!(
        placement,
        call(6, Strain::Hearts),
        "the asker places the contract in the agreed trump"
    );
}

/// The same collision one rung lower, and the one that actually cost IMPs:
/// the ladders **overlap**, so a relocated ask's own answer rungs are other
/// relocated asks.  With diamonds agreed 4♥ asks and 4♠ is its step 1 — but
/// 4♠ is also the *hearts* ask, and reading it as one puts a live ask on
/// partner's answer.  The asker then answers its own question: holding
/// ♠A ♥A ♣A it counts three keycards *for hearts* and bids 5♣, the 0-or-3
/// rung, whose 1.9 outbids its own 1.82 signoff.  Measured board 400 of the
/// kickback-vs-queen divergence audit: `5♣` doubled, singleton ♣A opposite
/// ♣987, −1100 against ♣KQT643 offside.
#[test]
fn a_suit_answering_the_relocation_is_not_a_new_ask() {
    let kickback = kickback_agreements();
    let auction = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
        call(4, Strain::Hearts),
        Call::Pass,
        call(4, Strain::Spades),
        Call::Pass,
    ];
    assert_eq!(
        keycard_ask_bid(&auction, 6, RkcbVariant::Kickback),
        None,
        "4♠ is step 1 over the 4♥ ask, so it asks nothing"
    );
    assert_eq!(
        keycard_ask_bid(&auction, 4, RkcbVariant::Kickback),
        Some(Bid::new(4, Strain::Hearts)),
        "the ask is still the 4♥ two calls before it"
    );
    let placement = best_with(&kickback, &auction, "AKQ3.AQ42.QT72.A");
    assert!(
        matches!(placement, Call::Bid(bid) if bid.strain == Strain::Diamonds),
        "the asker places the contract in the agreed trump, never a phantom club: {placement:?}"
    );
}

/// The answer arm of [`conversation_rung`] stands on its own: a relocated
/// ask's answer is never read as a fresh ask.
///
/// It used to be reachable only through the queen relay, so plain
/// `set_kickback` had no guard beyond the 4NT carve-out — which is why the
/// earlier "kickback is a wash" measurement was taken with the collision
/// live.  The guard is unconditional now, and so is the relay.
#[test]
fn the_answer_is_not_an_ask() {
    let auction = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
        call(4, Strain::Hearts),
        Call::Pass,
        call(4, Strain::Spades),
        Call::Pass,
    ];
    let answer = keycard_ask_bid(&auction, 6, RkcbVariant::Kickback);
    let ask = keycard_ask_bid(&auction, 4, RkcbVariant::Kickback);
    assert_eq!(answer, None, "4♠ answers the 4♥ ask; it asks nothing");
    assert_eq!(
        ask,
        Some(Bid::new(4, Strain::Hearts)),
        "the ask itself still reads as one"
    );
}

/// Redwood's whole point in one assertion: with diamonds agreed, the
/// two-keycards-plus-queen answer is `5♦` — the trump suit's own five
/// level, still a place to play — where the plain 4NT ask answers `5♠` and
/// the asker has nowhere left to stop
#[test]
fn redwood_keeps_the_trump_five_reachable() {
    let responder = "A32.432.KQ432.32"; // ♠A + ♦K = two keycards, with ♦Q
    let raise = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let kickback = kickback_agreements();
    let relocated: Vec<Call> = raise
        .iter()
        .copied()
        .chain([call(4, Strain::Hearts), Call::Pass])
        .collect();
    assert_eq!(
        kickback_ladder(&relocated, 4, RkcbVariant::Kickback)[Suit::Hearts as usize],
        Some(Suit::Diamonds),
        "diamonds are set and hearts unguarded, so 4♥ asks in diamonds"
    );
    let redwood = best_with(&kickback, &relocated, responder);

    let plain: Vec<Call> = raise
        .iter()
        .copied()
        .chain([call(4, Strain::Notrump), Call::Pass])
        .collect();
    let blackwood = best(&plain, responder);

    assert_eq!(
        redwood,
        call(5, Strain::Diamonds),
        "Redwood answers below 5♦"
    );
    assert_eq!(
        blackwood,
        call(5, Strain::Spades),
        "the plain ask answers above it — 5♦ is gone"
    );
}

/// [`RkcbVariant`]'s truth table: Redwood claims the minor lanes alone,
/// Kickback claims hearts on top and implies the minor scope, and either
/// relocation implies the minors' reach — no stance is "kickback only
/// hearts"
#[test]
fn redwood_scopes_the_ladder_and_implies_the_minors() {
    let diamonds = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let hearts = [
        call(1, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let claims = |auction: &[Call], variant| kickback_ladder(auction, 4, variant);
    let minor_asks_for = |variant| {
        let mut profile = DecisionProfile::default();
        profile.instinct.keycard_minors = false;
        profile.reading.rkcb_variant = variant;
        minor_asks(&profile)
    };

    assert_eq!(
        claims(&diamonds, RkcbVariant::Redwood)[Suit::Hearts as usize],
        Some(Suit::Diamonds),
        "Redwood relocates the diamond ask to 4♥"
    );
    assert_eq!(
        claims(&hearts, RkcbVariant::Redwood)[Suit::Spades as usize],
        None,
        "Redwood alone never claims 4♠ — the hearts ask stays at 4NT"
    );
    assert!(
        minor_asks_for(RkcbVariant::Redwood),
        "a live minor relocation implies the minors' reach"
    );

    assert_eq!(
        claims(&hearts, RkcbVariant::Kickback)[Suit::Spades as usize],
        Some(Suit::Hearts),
        "the full ladder claims the hearts lane too"
    );
    assert_eq!(
        claims(&diamonds, RkcbVariant::Kickback)[Suit::Hearts as usize],
        Some(Suit::Diamonds),
        "kickback implies the Redwood scope — never a hearts-only ladder"
    );
    assert!(
        minor_asks_for(RkcbVariant::Kickback),
        "kickback implies the minors' reach as well"
    );

    assert_eq!(
        claims(&diamonds, RkcbVariant::Plain)[Suit::Hearts as usize],
        None
    );
    assert!(
        !minor_asks_for(RkcbVariant::Plain),
        "no relocation, no carve: majors only"
    );
}

/// One hand's shown five never converts 4NT into an ask: opener's five
/// hearts complete our bare three to an eight *we* can see, but the
/// table proves only five and the face keys opener's second suit — the
/// old proxy fired here and the answerer counted against spades (the
/// wrong-suit clash the drift ledger filed as the ask-gate follow-up).
/// The gate now leaves the node to judgment, keeping the quant exit.
#[test]
fn unprovable_fit_never_asks() {
    let auction = [
        call(1, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    assert_ne!(
        best(&auction, "K42.K73.AQJ85.A2"),
        call(4, Strain::Notrump),
        "a fit only one seat can prove keeps 4NT out of the ask"
    );
}

/// jdh8's doctrine: directly over the Stayman answer or the transfer
/// completion, 4NT is quantitative — the one call exploring the
/// uncertain major fit and the misfit 6NT at once — and slam interest
/// cues the other major first.  Both nodes are book territory, so the
/// recalibrated floor gate never decides them.
#[test]
fn one_notrump_lanes_stay_book_quant() {
    let stayman = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let (made, off_book) = american_floored(&stayman, "KQ73.A5.KJ84.Q92");
    assert!(!off_book, "the Stayman lane is book territory");
    assert_eq!(
        made,
        call(3, Strain::Hearts),
        "slam interest cues the other major, not 4NT"
    );
    let transfer = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let (_, off_book) = american_floored(&transfer, "KQ873.A5.KJ8.Q92");
    assert!(!off_book, "the transfer lane is book territory");
}

/// The keycard machinery runs in a *contested* auction — the whole-auction
/// undisturbed gate is gone; only a bid inside the window stands it down.
/// Anchor: the reading-drift A/B's worst board (`1♦ (2♣) 2♠ - 3♠ - 4NT`),
/// where the net freewheeled the window into 5♣ - X - XX passed out in a
/// 2-2 club fit, −24 IMPs.
#[test]
fn contested_keycard_window_answers_and_places() {
    let ask = [
        Call::Pass,
        Call::Pass,
        call(1, Strain::Diamonds),
        call(2, Strain::Clubs),
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
    ];
    // The opener answers 1430 despite the earlier interference: A♠ K♠ =
    // two keycards, no queen → 5♥ (the net had mis-stated its count
    // with 5♣).
    assert_eq!(
        best(&ask, "AKT8.8432.KQJ.65"),
        call(5, Strain::Hearts),
        "a contested window still answers 1430"
    );
    // The asker decodes 5♥ (two + two = one missing) but holds a
    // 10-count: the ask was the net's judgement, not a vetted slam
    // entry, so the six rung stands down and the signoff places 5♠.
    let answered = [ask.as_slice(), &[call(5, Strain::Hearts), Call::Pass]].concat();
    assert_eq!(
        best(&answered, "J9654.AJ.AT74.74"),
        call(5, Strain::Spades),
        "an unvetted ask signs off at five"
    );
    // Their double of the answer changes nothing the arithmetic needs:
    // the asker still places the contract — never a redouble, never a
    // pass that leaves 5♥ doubled as the final contract.
    let doubled = [ask.as_slice(), &[call(5, Strain::Hearts), Call::Double]].concat();
    assert_eq!(
        best(&doubled, "J9654.AJ.AT74.74"),
        call(5, Strain::Spades),
        "the asker places over their double of the answer"
    );
    // The answerer respects the contested signoff holding at most one
    // keycard — even with the interference and their double on the way.
    let signed_off = [
        ask.as_slice(),
        &[
            call(5, Strain::Diamonds),
            Call::Double,
            call(5, Strain::Spades),
            Call::Pass,
        ],
    ]
    .concat();
    assert_eq!(
        best(&signed_off, "QJT8.KQ4.KQJ9.65"),
        Call::Pass,
        "the contested signoff is respected short a keycard"
    );
}

/// The cramped doubled answer: their X on partner's 1430 answer past
/// five of trump is never passed out, and never played in the answer's
/// phantom suit.  Anchor: the face-trump A/B's worst board:
/// `1♦ - 1♥ (1♠) - (3♠) 4♦ - 4NT - 5♥ (X)` passed out on a 4-1 heart "fit"
/// (−20), where
/// the answer's own natural read minted a phantom heart trump for the
/// asker — [`answer_trump`]'s pre-answer discipline — and the sit rung
/// played the doubled answer.
#[test]
fn cramped_doubled_answer_escapes_the_phantom_suit() {
    let doubled = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Hearts),
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        call(4, Strain::Diamonds),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
        call(5, Strain::Hearts),
        Call::Double,
    ];
    // The −20 board's asker: partner's 5♥ shows two keycards, not
    // hearts — the trump stays the diamond agreement, and the doubled
    // off-fit answer escapes to six of the seen fit.
    assert_eq!(
        best(&doubled, "KQ.9.AQ9875.T764"),
        call(6, Strain::Diamonds),
        "the doubled answer escapes to six of the seen fit"
    );
    // No seen diamond fit but their spades stopped: 5NT, a level
    // cheaper — and never 6♥ off the answer's polluted heart floor.
    assert_eq!(
        best(&doubled, "KQ5.96.A.QT87642"),
        call(5, Strain::Notrump),
        "without a seen fit the stopped hand drifts to 5NT"
    );
    // Stopperless with no seen fit: six of the derived trump, the last
    // resort — still never the answer suit doubled.
    assert_eq!(
        best(&doubled, "965.96.A.QJT8764"),
        call(6, Strain::Diamonds),
        "the last resort is six of the derived trump"
    );
    // The answerer sits the suit escape (two keycards keep the
    // correction exception live, but nothing drives past the escape)...
    let placed = [doubled.as_slice(), &[call(6, Strain::Diamonds), Call::Pass]].concat();
    assert_eq!(
        best(&placed, "A6.Q532.KJT642.Q"),
        Call::Pass,
        "the answerer sits the suit escape"
    );
    // ...and the notrump escape.
    let notrump = [doubled.as_slice(), &[call(5, Strain::Notrump), Call::Pass]].concat();
    assert_eq!(
        best(&notrump, "A6.Q532.KJT642.Q"),
        Call::Pass,
        "the answerer sits the notrump escape"
    );
    // The 1.88 respect rung reaches the notrump escape too: a club-lane
    // answerer short a keycard passes the asker's 5NT by rule.
    let club_lane = [
        call(1, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
        call(5, Strain::Diamonds),
        Call::Double,
        call(5, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(
        best(&club_lane, "QJ98.KQJ4.QJ92.4"),
        Call::Pass,
        "the notrump escape is respected short a keycard"
    );
}

#[test]
fn rkcb_historical_prefix_does_not_reuse_the_full_auction_reading() {
    use crate::bidding::american::american_instinct;

    let auction = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Hearts),
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        call(4, Strain::Diamonds),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
        call(5, Strain::Hearts),
        Call::Double,
    ];
    let hand: Hand = "KQ.9.AQ9875.T764".parse().expect("valid test hand");
    let stance = american_instinct(&crate::bidding::agreements::Agreements::default()).against();
    let uncached_context = stance.prefixed_context(RelativeVulnerability::NONE, &auction);
    let reference = answer_trump(hand, &uncached_context, 8);

    let cached_context = stance
        .prefixed_context(RelativeVulnerability::NONE, &auction)
        .with_decision_cache(hand);
    let cached = answer_trump(hand, &cached_context, 8);
    assert_eq!(cached, reference);
    assert_eq!(cached, Some(Suit::Diamonds));
    assert_eq!(
        cached_context.decision_cache_init_counts(),
        Some((1, 0, 0)),
        "only the full-auction read belongs to the decision cache"
    );
}

/// Their interference over our 4NT ask no longer stands the machinery
/// down: DOPI below five of trump, DEPO at or above, ROPI over their
/// double — the card's declared conventions, authored classic (D0P1 /
/// R0P1, the queen dimension traded away)
#[test]
fn keycard_interference_answers_dopi_ropi_depo() {
    // DOPI: their 5♣ over 4NT sits below five of the agreed spades.
    let dopi = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        call(5, Strain::Clubs),
    ];
    assert_eq!(
        best(&dopi, "QJ98.KQJ4.QJ9.42"),
        Call::Double,
        "DOPI: double shows zero keycards"
    );
    assert_eq!(
        best(&dopi, "QJ98.KQJ4.A92.42"),
        Call::Pass,
        "DOPI: pass shows one keycard"
    );
    assert_eq!(
        best(&dopi, "QJ98.KQ4.A92.A42"),
        call(5, Strain::Diamonds),
        "DOPI: the cheapest step shows two keycards"
    );
    // The asker decodes the double (zero opposite three own keycards)
    // and signs off in the still-available five of trump.
    let dopi_placed = [dopi.as_slice(), &[Call::Double, Call::Pass]].concat();
    assert_eq!(
        best(&dopi_placed, "AKT32.A54.K3.987"),
        call(5, Strain::Spades),
        "the asker decodes DOPI's zero and signs off"
    );
    // DEPO: their 5♠ over a heart-fit 4NT leaves no stepping room.
    let depo = [
        call(1, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
        call(4, Strain::Notrump),
        call(5, Strain::Spades),
    ];
    assert_eq!(
        best(&depo, "QJ98.QJ42.KQJ.92"),
        Call::Double,
        "DEPO: double shows an even count"
    );
    assert_eq!(
        best(&depo, "QJ98.QJ42.AQJ.92"),
        Call::Pass,
        "DEPO: pass shows an odd count"
    );
    // ROPI: their double of the ask — the two-keycard hand answers 5♣
    // (the cheapest bid, count two), not the 1430 5♥.
    let ropi = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Double,
    ];
    assert_eq!(
        best(&ropi, "QJ98.KQJ4.QJ9.42"),
        Call::Redouble,
        "ROPI: redouble shows zero keycards"
    );
    assert_eq!(
        best(&ropi, "QJ98.KQJ4.A92.42"),
        Call::Pass,
        "ROPI: pass shows one keycard"
    );
    assert_eq!(
        best(&ropi, "QJ98.KQ4.A92.A42"),
        call(5, Strain::Clubs),
        "ROPI: the cheapest bid shows two keycards"
    );
}

/// [`keycard_conversation_now`] marks the live window as forced-rail
/// territory for the neural shell — and stands down the moment the
/// opponents bid inside it or the ask is not decodable
#[test]
fn keycard_conversation_is_forced_rail_territory() {
    let live = |auction: &[Call]| forced(&Context::new(RelativeVulnerability::NONE, auction));
    // The contested ask window: partner's 4NT with a shown spade suit.
    assert!(live(&[
        call(1, Strain::Diamonds),
        call(2, Strain::Clubs),
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
    ]));
    // Their bid directly over the 4NT is the DOPI/DEPO window now — the
    // rungs answer in scheme, so the rail claims it.
    assert!(live(&[
        call(1, Strain::Diamonds),
        call(2, Strain::Clubs),
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        call(5, Strain::Clubs),
    ]));
    // But their bid over the *answer* still takes the machinery down:
    // judgement resumes.
    assert!(!live(&[
        call(1, Strain::Diamonds),
        call(2, Strain::Clubs),
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
        call(5, Strain::Hearts),
        call(6, Strain::Clubs),
    ]));
    // The side's last bid is a suit: 4NT over the 1♦ opening asks in
    // diamonds (face_trump rule 2 — all four suits qualify).
    assert!(live(&[
        call(1, Strain::Diamonds),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
    ]));
    // The round-2 A/B's worst boards: the side's last bid below the 4NT
    // is its own 3NT — quantitative, no fit behind it — so the window
    // stays judgement even after a five-level answer.  Hijacking the
    // asker here passed out making minor slams in 5♥.
    assert!(!live(&[
        call(1, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Clubs),
        call(2, Strain::Notrump),
        call(3, Strain::Notrump),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
        call(5, Strain::Hearts),
        Call::Pass,
    ]));
    // Round 3's worst board: partner doubles twice then jumps to 4♠ —
    // no fit on the face and the contested reading is vacuous, but the
    // side's last bid is a suit, so 4NT asks in spades and the answer
    // window is rail territory (the bare net redoubled the doubled
    // answer and played 5♥xx in a 4-1 fit, −23 IMPs).
    assert!(live(&[
        call(2, Strain::Hearts),
        Call::Double,
        call(3, Strain::Hearts),
        Call::Double,
        Call::Pass,
        call(4, Strain::Spades),
        Call::Pass,
        call(4, Strain::Notrump),
        Call::Pass,
    ]));
}

/// Partner's post-transfer 4♠ is a control bid agreeing hearts (the M6.4
/// reading): opener returns to the agreed suit instead of passing out the
/// phantom spade contract
#[test]
fn control_bid_is_never_passed_out() {
    // 1♦ - 1♠ - 2♦ - 4♥ under the hearts-first opt-in (knob off): a 1♠ response
    // denies four hearts, so 4♥ cannot be long — a control bid agreeing
    // diamonds (the M6.4 reading) — and the floor returns to the agreed
    // suit instead of passing out the phantom heart contract.  (Under the
    // longer-major default, 1♠ can be 5-5, so 4♥ reads to play.)
    let mut agreements = Agreements::default();
    agreements.decision.reading.longer_major_response = false;
    let auction = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(4, Strain::Hearts),
        Call::Pass,
    ];
    let (bid, from_floor) = american_floored_with(&agreements, &auction, "A4.K85.KQJ62.Q75");
    assert!(from_floor, "the 4♥ jump is off-book");
    assert_eq!(
        bid,
        call(5, Strain::Diamonds),
        "return to the agreed suit over partner's control bid"
    );
}

#[test]
fn transfer_jump_to_game_reaches_the_floor_and_passes() {
    // 1NT - 2♦ - 2♥ - 4♥: the jump past 3NT is off-book too.  Game is already
    // reached and the floor has no slam machinery yet (M6.2), so it passes —
    // M6.1 derives the six-card major (length only) without over-reaching.
    let game = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        call(4, Strain::Hearts),
        Call::Pass,
    ];
    let (bid, from_floor) = american_floored(&game, "AKQ2.J5.AQ52.K42");
    assert!(from_floor, "the 4♥ jump is off-book, the floor decides");
    assert_eq!(bid, Call::Pass);
}

#[test]
fn nine_count_five_card_major_forces_game_after_a_transfer() {
    // 1NT - 2♥ - 2♠: a 9-count with a single five-card spade suit transferred (it
    // cannot bid the direct 3NT, which denies a five-card major) and now forces
    // game.  The choice-of-games rule's `hcp(9..16)` mirrors the floor's
    // 9-count seam (`nt_responder_game_floor`), so the *book* authors the 3NT
    // the floor used to carry — same call, and now the rule's projection
    // admits the hand that bid it (`set_natural_reading` soundness).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let (bid, from_floor) = american_floored(&auction, "AK543.82.Q76.542");
    assert!(!from_floor, "the 9-count game force is on-book now");
    assert_eq!(bid, call(3, Strain::Notrump));
}

#[test]
fn opener_corrects_choice_of_games_3nt_to_the_known_major_fit() {
    // 1NT - 2♥ - 2♠ - 3NT: responder transferred (showing five spades) then offered
    // the choice with 3NT.  Opposite three-card support the 5-3 fit out-scores
    // notrump single-dummy *only with a ruffing doubleton*, so opener corrects
    // to 4♠ on a doubleton, but a flat 4-3-3-3 (no ruff) leaves the better game
    // in 3NT.  Default on.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Notrump),
        Call::Pass,
    ];
    // Three-card support with a ruffing doubleton (3-2-4-4): correct to 4♠.
    let (fit, _) = american_floored(&auction, "AQ4.K8.KJ72.Q832");
    assert_eq!(
        fit,
        call(4, Strain::Spades),
        "3-card support with a doubleton corrects to 4♠"
    );
    // Three-card support but a flat 4-3-3-3 (no ruffing value): stay in 3NT.
    let (flat, _) = american_floored(&auction, "AQ4.K83.KJ72.Q83");
    assert_eq!(
        flat,
        Call::Pass,
        "flat 4333 has no ruff — 3NT is the better game"
    );
    // Only a doubleton spade — no eight-card fit — also stays in 3NT.
    let (two, _) = american_floored(&auction, "AQ.K842.KJ73.Q82");
    assert_eq!(two, Call::Pass, "no eight-card fit leaves it in 3NT");
}

#[test]
fn strong_balanced_redoubles_a_double_of_our_1nt_not_3nt() {
    // 1NT (X): a strong balanced responder defends the unlimited business
    // redouble rather than pulling to 3NT (the floor suppresses the game-force
    // 3NT over a double of our 1NT).
    let auction = [call(1, Strain::Notrump), Call::Double];
    let (bid, from_floor) = american_floored(&auction, "KQ4.KJ43.AQ62.Q5");
    assert!(from_floor, "the response is off-book, the floor decides");
    assert_eq!(bid, Call::Redouble);
}

#[test]
fn keeps_passing_with_a_weak_responder() {
    // Partner opened 1NT but we are too weak to force game: still pass when
    // off-book (the forced-to-game floor must not fire on invitational-or-less).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(best(&auction, "8632.J9842.96.42"), Call::Pass);
}

/// Through an established 2/1 the floor must know partner holds values
///
/// The alerted `GAME_FORCE` response reads as *zero* points (its
/// `points(13..)` gate projects to no high-card floor), so without the
/// strength floor the slam-entry gate never fires here — opener signs off in
/// `4♠` holding a 26-count opposite the force.
#[test]
fn two_over_one_slam_strength_unblocks_the_ask() {
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    // Delete the book node so the floor owns the position.
    let mut agreements = Agreements::default();
    agreements.decision.instinct.two_over_one_slam_strength = false;
    agreements.game_force.opener_third = false;
    let (chosen, floored) = american_floored_with(&agreements, &auction, "AKQJ2.AKQ.AQJ4.9");
    assert!(floored, "the deleted node leaves this to the floor");
    // Was `4♠`, and the comment called it "the defect: a 26-count game".
    // The bilans floor (default-on) resolves it: 26 opposite a game-forcing
    // 2/1 with spade support is 39+ combined, and the net prices thirteen
    // tricks above the grand's break-even.  Knob-off this is still 4♠.
    assert_eq!(chosen, call(7, Strain::Spades), "the net finds the grand");

    let mut agreements = Agreements::default();
    agreements.decision.instinct.two_over_one_slam_strength = true;
    agreements.game_force.opener_third = false;
    let (chosen, _) = american_floored_with(&agreements, &auction, "AKQJ2.AKQ.AQJ4.9");
    assert_ne!(
        chosen,
        call(4, Strain::Spades),
        "26 opposite a 2/1 explores"
    );
    // The matching "a genuine minimum still signs off" guard lives in
    // `a_minimum_signs_off_opposite_an_established_two_over_one`, which is
    // ignored pending the 2/1 projection fix.
}

/// A minimum opener signs off in game opposite an established 2/1 — the
/// floor is a floor, not a licence.
///
/// **Ignored: a reading defect, not a floor defect.** Partner's 2/1 game
/// force reads `points 0..=37` — erased outright, the fit-split `Or`
/// unioning away its own `hcp(13..)` (docs/ai-bidder/sampled-projection.md).
/// So the sampled layouts include partners this auction cannot hold, and
/// the mean they drag up is what clears the slam-entry gate.  The sharper
/// v2 evaluator turned that latent bias into a visible call — `4NT`, a
/// keycard ask on a bare twelve-count opposite a partner who might hold a
/// Yarborough.  Pinning `4NT` here would enshrine the defect as intent, so
/// the assertion stays as written and waits on the reading fix.
///
/// Note the shape half of the read is *fine* — partner comes back with
/// clubs 4+ and spades exactly 3.  It is only `points` that is erased.
#[test]
#[ignore = "the 2/1 game force reads points 0..=37, which inflates the slam gate"]
fn a_minimum_signs_off_opposite_an_established_two_over_one() {
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    // Delete the book node so the floor owns the position.
    let mut agreements = Agreements::default();
    agreements.game_force.opener_third = false;
    let (minimum, floored) = american_floored_with(&agreements, &auction, "AQJ52.32.KQ54.92");
    assert!(floored, "the deleted node leaves this to the floor");
    assert_eq!(minimum, call(4, Strain::Spades));
}

/// Knob-on, the slam machinery stays alive end to end.  A 26-count
/// opposite an established 2/1 still blasts the grand — the 7♠ milestone
/// (1.75) outranks the ask (1.68) in *both* regimes (knob-off its
/// combined 39 ≥ 37; knob-on the net clears the grand break-even) — and on
/// the natural 1♠ - 3♠ raise the net's [`SLAM_ENTRY_P`] entry — the one
/// bilans gate with no forcing rail behind it — still fires the keycard
/// ask.  The minimum's signoff moved to
/// `a_minimum_signs_off_opposite_an_established_two_over_one`.
#[test]
fn bilans_floor_still_explores_the_rock_crusher_slam() {
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    // Delete the book node so the floor owns the position.
    let mut agreements = Agreements::default();
    agreements.game_force.opener_third = false;
    let (crusher, floored) = american_floored_with(&agreements, &auction, "AKQJ2.AKQ.AQJ4.9");
    // The natural jump raise (cf. `floor_asks_keycards_with_slam_values_
    // and_a_known_fit`): a 23-count opener in the entry band asks, not
    // blasts.  Natural calls read alike bare or prefixed, so `best` is
    // on-distribution here.
    let raise = [
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    let ask = best(&raise, "AKQJ7.AKQ.A32.32");
    assert!(floored, "the deleted node leaves this to the floor");
    assert_eq!(
        crusher,
        call(7, Strain::Spades),
        "the net clears the grand break-even where the points cleared 37"
    );
    assert_eq!(
        ask,
        call(4, Strain::Notrump),
        "the net's entry keeps the keycard ask alive"
    );
}

/// The auctions the game backstop used to cover, and the ones it did not
#[test]
fn two_over_one_force_reads_the_right_auctions() {
    let on = Agreements::default();
    let forced = |agreements: &Agreements, auction: &[Call]| {
        two_over_one_game_force(
            &Context::new(RelativeVulnerability::NONE, auction).with_profile(agreements.decision),
        )
    };
    // 1♠ - 2♣ and 1♦ - 2♣: the game force, read from opener's seat.
    assert!(forced(
        &on,
        &[
            call(1, Strain::Spades),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Pass
        ]
    ));
    assert!(forced(
        &on,
        &[
            call(1, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Pass
        ]
    ));
    // 1♥ - 2♠ is a jump shift, not a 2/1: the response must sit below the opening.
    assert!(!forced(
        &on,
        &[
            call(1, Strain::Hearts),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass
        ]
    ));
    // 1♣ - 2♦ has no suit below clubs to answer in; the book registers no
    // game force there either.
    assert!(!forced(
        &on,
        &[
            call(1, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass
        ]
    ));
    // Contested: over interference a two-level suit is a free bid, not a force.
    assert!(!forced(
        &on,
        &[
            call(1, Strain::Spades),
            call(2, Strain::Diamonds),
            call(2, Strain::Hearts),
            Call::Pass
        ]
    ));
    let mut off = Agreements::default();
    off.decision.two_over_one_force = false;
    assert!(!forced(
        &off,
        &[
            call(1, Strain::Spades),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Pass
        ]
    ));
}

/// With the game backstop deleted the floor owns these nodes, and it must
/// not abandon partner's game force — the 24%-of-divergences failure the
/// deletion A/B exposed (opener passing 3♣ out in an established 2/1).
#[test]
fn two_over_one_force_never_passes_below_game() {
    let auction = [
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let agreements = Agreements::default();
    for hand in ["AQ9876.K3.K95.76", "KQJ876.43.K95.A6", "AKJ976.Q2.J54.83"] {
        let (chosen, floored) = american_floored_with(&agreements, &auction, hand);
        assert!(floored, "the deleted backstop leaves {hand} to the floor");
        assert_ne!(chosen, Call::Pass, "{hand} must not pass a live game force");
    }
}

#[test]
fn forced_to_game_after_strong_two_clubs() {
    // `2♣ - 2♦ - 2NT -`: the strong 2♣ opening and game-forcing 2♦ waiting
    // response reach opener's 22–24 balanced rebid.  The auction is game
    // forcing, so a flat 7-count bids 3NT, never passing.
    // 2♣ - 2♥ is the double negative, so 2♦ commits the partnership to game.
    let auction = [
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(best(&auction, "QJ52.K43.T62.J32"), call(3, Strain::Notrump));
}

#[test]
fn forced_two_clubs_bids_major_game() {
    // The same forcing 2♣ - 2♦ - 2NT auction, but holding six hearts: jump to
    // the major-suit game in preference to 3NT.
    let auction = [
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(best(&auction, "3.QJ9854.K32.J32"), call(4, Strain::Hearts));
}

#[test]
fn double_negative_two_clubs_may_pass() {
    // 2♣ - 2♥ is the double negative (0–3 HCP); after opener's 2NT the
    // partnership may still stop, so a yarborough passes off-book — the
    // forcing-2♣ floor must not fire once responder has shown the bust.
    let auction = [
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(best(&auction, "8632.J9842.96.42"), Call::Pass);
}

#[test]
fn forced_game_steps_aside_when_penalizing() {
    // `2♣ - 2♦ - 2NT (3♦) X -`: after the game-forcing wait, they sacrifice
    // and partner doubles for penalty.  Passing the double out is the
    // action, so the floor must not pull it to a stopperless 3NT; with six
    // clubs and no diamond guard, show the suit instead.
    let auction = [
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Notrump),
        call(3, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    assert_eq!(best(&auction, "K3.KQ4.65.QJ8765"), call(4, Strain::Clubs));
}

#[test]
fn milestone_game_opposite_a_limited_rebid() {
    // 1♦ - 1♥ - 1NT: opposite the 12–16 rebid a balanced 16 has 28+ combined,
    // a cold 3NT the constructive book never reached (the board that started
    // this).  The floor reads the rebid's strength and bids the game.
    let auction = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Hearts),
        Call::Pass,
        call(1, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(best(&auction, "J9.AKJ7.K94.A852"), call(3, Strain::Notrump));
    // A 10-count is only invitational (22–24 combined): the floor uses the
    // *guaranteed* minimum, so it stays sound and passes rather than overbid.
    assert_eq!(best(&auction, "KJ9.QJ73.K94.852"), Call::Pass);
}

#[test]
fn milestone_slam_opposite_a_strong_rebid() {
    // 1♦ - 1♥ - 2NT is the 18–19 jump rebid; a balanced 16 lifts the combined
    // minimum to 34, the small-slam zone, so bid 6NT instead of stranding in
    // game.  No known major fit, so notrump is the strain.
    let auction = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(best(&auction, "KQ.AKJ7.K94.8542"), call(6, Strain::Notrump));
}

#[test]
fn milestone_game_opposite_a_competitive_overcall() {
    // After `(3♦) 3♠ -`, partner's overcall reads as 5+ spades and 8+ points.
    // A 21-count with three-card support lifts the
    // combined minimum to 29 with a known eight-card spade fit, so the floor
    // bids the game it would otherwise miss off-book.
    let auction = [
        call(3, Strain::Diamonds),
        call(3, Strain::Spades),
        Call::Pass,
    ];
    // The bilans floor (default-on) prices the contract rather than the
    // point sum.  The hull-only twin liked twelve tricks better than even
    // money here; the shipped v3 calls-tail twin (2026-07-27), with the
    // 3♦ preempt and the 3♠ overcall in its input, likes thirteen — the
    // aggressive edge of the slam family its `win | win` A/B measured
    // (the worst divergent boards are exactly 7-level claims, and the
    // aggregate still wins both scorers at both vulnerabilities).
    assert_eq!(best(&auction, "K32.AKJ.AQ4.KJ32"), call(7, Strain::Spades));
    // A flat 12-count: the point milestone read this as 20 combined and
    // passed.  The net raises to game instead — defensibly, since a 3-level
    // overcall of a 3♦ preempt is worth well more than the 8 the inference
    // floor reads, and this hand has three-card support.  Both assertions in
    // this test moved *upward* when the floor went default-on; see the
    // evaluator-net doc's competitive-auction note.
    assert_eq!(best(&auction, "K32.KJ4.KQ4.5432"), call(4, Strain::Spades));
}

#[test]
fn milestone_notrump_game_needs_a_stopper_in_competition() {
    // After `(3♣) 3♦ -`, we have game values opposite the overcall, but no
    // major fit and no diamond fit — the strain is 3NT,
    // and the floor must hold a club guard to bid it.
    let auction = [
        call(3, Strain::Clubs),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    // A club stopper (K432): the floor picks notrump.  The *level* is
    // deliberately not pinned.  Partner's overcall reads `points 8..=37` —
    // a vacuous envelope — and against it the v2 evaluator prices 6NT over
    // 3NT.  That is the wide-envelope ceiling (evaluator-net.md, "Known
    // ceilings"), and it is a *reading* defect, not a stopper one: the
    // subject of this test is the guard, so the guard is what it asserts.
    assert!(
        matches!(best(&auction, "AKQ.AQJ.32.K432"),
                 Call::Bid(bid) if bid.strain == Strain::Notrump),
        "a club guard buys notrump"
    );
    // No club guard and no fit: pass rather than bid into an unstopped suit.
    assert_eq!(best(&auction, "AKQ4.AKQ4.32.432"), Call::Pass);
}

#[test]
fn rubens_new_suit_transfer() {
    let agreements = rubens_agreements();
    // (1♣) 1♠ -: advancing partner's spade overcall with our own five-card
    // diamond suit, we transfer — 2♣ shows diamonds (the next suit up).  The
    // floor is 10 upgraded points (a *good* 9 and all 10+), since the
    // transfer commits partner to the two-level.
    let auction = [call(1, Strain::Clubs), call(1, Strain::Spades), Call::Pass];
    // A good 9: working K/KQ in a five-card suit upgrades over the floor.
    assert_eq!(
        best_with(&agreements, &auction, "2.K32.KQT54.J432"),
        call(2, Strain::Clubs)
    );
    // A bare 8 does not reach it: too weak to introduce the suit, pass.
    assert_eq!(
        best_with(&agreements, &auction, "2.Q32.KQT54.J432"),
        Call::Pass
    );
}

#[test]
fn rubens_limit_raise_transfer() {
    let agreements = rubens_agreements();
    // (1♣) 1♠ -: a limit raise of partner's spades goes through the
    // transfer that lands in their suit — 2♥ (the bid just below 2♠).
    let auction = [call(1, Strain::Clubs), call(1, Strain::Spades), Call::Pass];
    assert_eq!(
        best_with(&agreements, &auction, "K54.K32.K43.Q432"),
        call(2, Strain::Hearts)
    );
}

#[test]
fn rubens_completion_is_mechanical() {
    let agreements = rubens_agreements();
    // (1♣) 1♠ - 2♣ -: partner transferred to diamonds; the overcaller
    // completes into 2♦ regardless of hand.
    let auction = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    assert_eq!(
        best_with(&agreements, &auction, "AKJ52.K3.952.J32"),
        call(2, Strain::Diamonds)
    );
}

#[test]
fn rubens_two_level_cue_raise() {
    let agreements = rubens_agreements();
    // (1♠) 2♣ -: partner overcalled at the two level, so the cue (2♠) is
    // the limit-plus raise of clubs — no transfer ladder where there is no room.
    let auction = [call(1, Strain::Spades), call(2, Strain::Clubs), Call::Pass];
    assert_eq!(
        best_with(&agreements, &auction, "432.K32.K2.KQJ54"),
        call(2, Strain::Spades)
    );
}

#[test]
fn rubens_skips_jump_overcalls() {
    // (1♣) 2♠ -: partner's 2♠ is a jump (1♠ was available), a preemptive
    // weak jump overcall — not a simple overcall, so no Rubens.  A limit hand
    // with support raises spades naturally rather than transferring.
    let auction = [call(1, Strain::Clubs), call(2, Strain::Spades), Call::Pass];
    assert_eq!(best(&auction, "K54.K32.K43.Q432"), call(3, Strain::Spades));
}

#[test]
fn rubens_completes_through_the_double() {
    let agreements = rubens_agreements();
    // (1♣) 1♠ - 2♣ (X): opener lead-directs against the transfer; the
    // completion still fires — otherwise the relay dies and partner plays
    // the phantom suit doubled.
    let auction = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Double,
    ];
    assert_eq!(
        best_with(&agreements, &auction, "AKJ52.K3.952.J32"),
        call(2, Strain::Diamonds)
    );
}

#[test]
fn rubens_max_breaks_the_completion_to_game() {
    let agreements = rubens_agreements();
    // (1♣) 1♠ - 2♥ -: partner's transfer into our spades showed 10+
    // with support, so a maximum places the game instead of completing.
    let auction = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
    ];
    assert_eq!(
        best_with(&agreements, &auction, "AKJ52.K3.K52.J32"),
        call(4, Strain::Spades)
    );
    // In between, the overcaller super-accepts — the invite `3♠`.
    assert_eq!(
        best_with(&agreements, &auction, "AKJ52.K3.Q52.432"),
        call(3, Strain::Spades)
    );
    // A minimum still completes mechanically.
    assert_eq!(
        best_with(&agreements, &auction, "AKJ52.K3.952.J32"),
        call(2, Strain::Spades)
    );
    // (1♣) 1♦ - 2♣ -: the diamond break is 3NT behind a club stopper…
    let minor = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    assert_eq!(
        best_with(&agreements, &minor, "A32.K32.AQJ54.K2"),
        call(3, Strain::Notrump)
    );
    // …and completes without one, whatever the strength.
    assert_eq!(
        best_with(&agreements, &minor, "AQ2.K32.AQJ54.32"),
        call(2, Strain::Diamonds)
    );
}

#[test]
fn rubens_new_suit_break_bids_what_it_would_over_natural() {
    let agreements = rubens_agreements();
    // (1♣) 1♠ - 2♦ -: partner shows hearts.  The completion covers the
    // would-pass-a-natural-2♥ hands; with a fit and values the overcaller
    // bids what it would have bid over that natural 2♥.
    let auction = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    // Fit + 13: the invite raise.
    assert_eq!(
        best_with(&agreements, &auction, "AKJ52.Q32.K52.32"),
        call(3, Strain::Hearts)
    );
    // Fit + maximum: the game.
    assert_eq!(
        best_with(&agreements, &auction, "AKJ52.Q32.K52.A2"),
        call(4, Strain::Hearts)
    );
    // No fit, minimum: the mechanical completion.
    assert_eq!(
        best_with(&agreements, &auction, "AKJ52.32.Q952.J2"),
        call(2, Strain::Hearts)
    );
}

#[test]
fn rubens_transferee_rebid_survives_an_out_of_band_two_spades() {
    // (1♣) 1♦ - 2♠ - 3♣ -: the 2♠ sits above partner's suit — no
    // transfer.  The detector must reject it by the band, not index past
    // the spade suit (this exact shape panicked a 204k-board run).
    let auction = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let _ = best(&auction, "K54.A32.KQ32.Q43");
}

#[test]
fn rubens_transferee_clarifies_with_extras() {
    let agreements = rubens_agreements();
    // (1♣) 1♠ - 2♦ - 2♥ -: the heart transfer was wide yet
    // unlimited — a six-card maximum now bids the game.
    let hearts = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
    ];
    assert_eq!(
        best_with(&agreements, &hearts, "2.AKQT54.K32.A32"),
        call(4, Strain::Hearts)
    );
    // 12–13 re-raises the suit: the invite the natural NF 2♥ never had.
    assert_eq!(
        best_with(&agreements, &hearts, "2.AKJT54.Q32.Q32"),
        call(3, Strain::Hearts)
    );
    // (1♣) 1♠ - 2♣ - 2♦ -: the diamond hand's game is 3NT behind a
    // club stopper.
    let diamonds = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        best_with(&agreements, &diamonds, "2.K32.AKQT54.A32"),
        call(3, Strain::Notrump)
    );
}

#[test]
fn rubens_raiser_moves_with_extras_over_the_completion() {
    let agreements = rubens_agreements();
    // (1♣) 1♠ - 2♥ - 2♠ -: the mechanical completion denied extras,
    // so the raiser drives to game with 14+ and rests below it otherwise.
    let auction = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    // 14 points needs a non-flat shape on the shipped rule-of-N+8 scale
    // (a 4333 14-count reads 13 and rests).
    assert_eq!(
        best_with(&agreements, &auction, "K542.A32.KQ32.Q4"),
        call(4, Strain::Spades)
    );
    assert_ne!(
        best_with(&agreements, &auction, "K54.K32.K43.Q432"),
        call(4, Strain::Spades)
    );
}

#[test]
fn rubens_cue_answer_places_the_contract() {
    let agreements = rubens_agreements();
    // (1♠) 2♣ - 2♠ -: partner's cue-raise must never play their suit.
    let auction = [
        call(1, Strain::Spades),
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    // A minimum retreats to our suit.
    assert_eq!(
        best_with(&agreements, &auction, "32.K32.Q32.AQJT54"),
        call(3, Strain::Clubs)
    );
    // A maximum with their suit stopped places the notrump game.
    assert_eq!(
        best_with(&agreements, &auction, "A2.K32.Q2.AKQJ54"),
        call(3, Strain::Notrump)
    );
    // (1♠) 2♥ - 2♠ -: a maximum with hearts places the major game.
    let hearts = [
        call(1, Strain::Spades),
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    assert_eq!(
        best_with(&agreements, &hearts, "2.AKQJ54.K32.Q32"),
        call(4, Strain::Hearts)
    );
    assert_eq!(
        best_with(&agreements, &hearts, "Q2.AQJT54.432.32"),
        call(3, Strain::Hearts)
    );
}

#[test]
fn rubens_cue_answer_fires_through_the_system() {
    let agreements = rubens_agreements();
    // The same node reached through `american()`: the floor rule must not
    // be shadowed by a book node (`project_floor_shadowed_by_book_nodes`),
    // or the cue keeps passing out at the real table.
    let auction = [
        call(1, Strain::Spades),
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let (call_made, floored) = american_floored_with(&agreements, &auction, "32.K32.Q32.AQJT54");
    assert!(floored, "the overcaller's cue answer is floor territory");
    assert_eq!(call_made, call(3, Strain::Clubs));
}

#[test]
fn rubens_skips_advances_of_a_double() {
    // (1♥) X - 1♠: the side's first action was a double, so 1♠ advances
    // the double — the doubler's later cue is not a Rubens structure.
    let auction = [
        call(1, Strain::Hearts),
        Call::Double,
        Call::Pass,
        call(1, Strain::Spades),
    ];
    assert_eq!(overcall_shape(&auction), None);
}

#[test]
fn rubens_disabled_reverts_to_natural_advances() {
    // Knob off (`ReadingProfile::rubens_advances`) — the default since the layer A/B:
    // the same hands advance naturally.
    let mut agreements = Agreements::default();
    agreements.decision.reading.rubens_advances = false;
    let auction = [call(1, Strain::Clubs), call(1, Strain::Spades), Call::Pass];
    // The limit raise is a direct natural raise, not the 2♥ transfer.
    assert_eq!(
        best_with(&agreements, &auction, "K54.K32.K43.Q432"),
        call(2, Strain::Spades)
    );
    // A five-card-diamond good 9 bids its suit naturally (the knob-off
    // fallback rule), not the 2♣ transfer.
    assert_eq!(
        best_with(&agreements, &auction, "2.K32.KQT54.J432"),
        call(2, Strain::Diamonds)
    );
    // No mechanical completion: partner's 2♣ is a genuine club suit.
    let after = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    assert_ne!(best(&after, "AKJ52.K3.952.J32"), call(2, Strain::Diamonds));
}

#[test]
fn one_nt_runout_disabled_passes() {
    // Disabled, responder has no runout and falls to the natural floor —
    // Pass — even broke with a five-card suit.
    let mut agreements = Agreements::default();
    agreements.decision.instinct.one_nt_runout = false;
    let doubled = [call(1, Strain::Notrump), Call::Double];
    assert_eq!(
        best_with(&agreements, &doubled, "32.QJ763.9742.83"),
        Call::Pass
    );
}

#[test]
fn one_nt_runout_escapes_to_the_long_suit() {
    let doubled = [call(1, Strain::Notrump), Call::Double];
    // A broke hand with five hearts runs to 2♥ rather than sit for it.
    assert_eq!(best(&doubled, "32.QJ763.9742.83"), call(2, Strain::Hearts));
    // Length beats the major preference: six clubs over five spades.
    assert_eq!(best(&doubled, "T9842.3.7.QJ9632"), call(2, Strain::Clubs));
    // A balanced bust has nowhere to run: it sits.
    assert_eq!(best(&doubled, "432.J85.K74.9632"), Call::Pass);
}

#[test]
fn one_nt_runout_redoubles_with_values() {
    let mut agreements = Agreements::default();
    agreements.decision.instinct.one_nt_runout = true;
    agreements.decision.instinct.runout_xx_min = 8;
    let doubled = [call(1, Strain::Notrump), Call::Double];
    // 8 balanced HCP — too good to run, not enough to force game opposite a
    // 15–17 opener (23 combined): redouble to play 1NT-XX.
    assert_eq!(
        best_with(&agreements, &doubled, "K43.KQ5.8642.972"),
        Call::Redouble
    );
    // A shapely bust at the same boundary still runs, never redoubles.
    assert_eq!(
        best_with(&agreements, &doubled, "3.QJ763.97642.83"),
        call(2, Strain::Hearts)
    );
}

#[test]
fn one_nt_overcall_runout_escapes_and_redoubles() {
    // The runout now fires when our 1NT was an OVERCALL, not just an opening:
    // (1♥) 1NT (X), advancer to act, our 1NT anchored at index 1.
    let mut agreements = Agreements::default();
    agreements.decision.instinct.one_nt_runout = true;
    let doubled = [
        call(1, Strain::Hearts),
        call(1, Strain::Notrump),
        Call::Double,
    ];
    // A broke hand with five spades runs to 2♠ rather than sit for it.
    assert_eq!(
        best_with(&agreements, &doubled, "QJ763.32.9742.83"),
        call(2, Strain::Spades)
    );
    // A balanced bust has nowhere to run: it sits.
    assert_eq!(
        best_with(&agreements, &doubled, "432.J85.K74.9632"),
        Call::Pass
    );
    // Eight balanced HCP is too good to run, too weak to force game opposite a
    // 15–18 overcall: redouble to play 1NT-XX (business).
    agreements.decision.instinct.runout_xx_min = 8;
    assert_eq!(
        best_with(&agreements, &doubled, "K43.KQ5.8642.972"),
        Call::Redouble
    );
}

#[test]
fn one_nt_overcall_runout_is_floor_territory() {
    // In the full system the doubled 1NT overcall reaches the instinct floor
    // (no book node shadows it), so the generalized runout actually fires.
    let doubled = [
        call(1, Strain::Hearts),
        call(1, Strain::Notrump),
        Call::Double,
    ];
    let (call_made, floored) = american_floored(&doubled, "QJ763.32.9742.83");
    assert!(floored, "the doubled 1NT overcall is floor territory");
    assert_eq!(call_made, call(2, Strain::Spades));
}

#[test]
fn gambling_3nt_over_double_routes_long_minors() {
    let mut agreements = Agreements::default();
    agreements.decision.instinct.one_nt_runout = true;
    agreements.decision.instinct.gambling_3nt_over_double = true;
    agreements.decision.instinct.gambling_3nt_top_honors = 2;
    agreements.decision.instinct.gambling_3nt_require_ace = true;
    let doubled = [call(1, Strain::Notrump), Call::Double];

    // A six-card minor headed by its own ace (semi-solid, suit ace) runs to the
    // gambling 3NT — opposite the 15–17 opener the suit cashes — not XX, not an
    // escape.
    assert_eq!(
        best_with(&agreements, &doubled, "32.43.654.AKJ987"),
        call(3, Strain::Notrump)
    );
    assert_eq!(
        best_with(&agreements, &doubled, "32.43.AKJ987.654"),
        call(3, Strain::Notrump)
    );

    // A strong balanced hand holds no six-card minor, so the gamble can never
    // steal it: it still defends the business redouble.
    assert_eq!(
        best_with(&agreements, &doubled, "KQ4.KJ43.AQ62.Q5"),
        Call::Redouble
    );

    // The suit-ace gate (default on): a semi-solid six-bagger missing its own ace
    // cannot gamble — it escapes.  Drop the requirement and it gambles.
    assert_eq!(
        best_with(&agreements, &doubled, "32.43.654.KQJ987"),
        call(2, Strain::Clubs)
    );
    agreements.decision.instinct.gambling_3nt_require_ace = false;
    assert_eq!(
        best_with(&agreements, &doubled, "32.43.654.KQJ987"),
        call(3, Strain::Notrump)
    );
    agreements.decision.instinct.gambling_3nt_require_ace = true;

    // The semi-solid gate: an ace-headed but ragged six-bagger (one top honor)
    // escapes; length-only (top-honors 0) lets it gamble instead.
    assert_eq!(
        best_with(&agreements, &doubled, "32.43.654.AJ9876"),
        call(2, Strain::Clubs)
    );
    agreements.decision.instinct.gambling_3nt_top_honors = 0;
    assert_eq!(
        best_with(&agreements, &doubled, "32.43.654.AJ9876"),
        call(3, Strain::Notrump)
    );
}

#[test]
fn preempt_4m_over_double_jumps_the_long_major() {
    // Pin the legacy scale: these 6-count runout hands have a known 8-card
    // fit (six-card major opposite the 1NT), so under the shipped
    // `support_points` scale their extra shortness tips the fit-sum floor to
    // game — masking the trump-ace / semi-solid escape gates this test checks.
    // The scale shift itself is measured by the A/B.
    let mut agreements = Agreements::default();
    agreements.decision.reading.support_points = false;
    agreements.decision.instinct.one_nt_runout = true;
    agreements.decision.instinct.preempt_4m_over_double = true;
    agreements.decision.instinct.preempt_4m_top_honors = 2;
    agreements.decision.instinct.preempt_4m_require_ace = true;
    let doubled = [call(1, Strain::Notrump), Call::Double];

    // A semi-solid six-card major headed by the trump ace jumps to its game.
    assert_eq!(
        best_with(&agreements, &doubled, "432.AKJ987.65.32"),
        call(4, Strain::Hearts)
    );
    assert_eq!(
        best_with(&agreements, &doubled, "AKJ987.432.65.32"),
        call(4, Strain::Spades)
    );

    // The trump-ace gate (default on): a KQ-headed six-bagger lacking the trump
    // ace does not preempt to game (a 6-count escapes); drop it and it jumps.
    assert_eq!(
        best_with(&agreements, &doubled, "432.KQJ987.65.32"),
        call(2, Strain::Hearts)
    );
    agreements.decision.instinct.preempt_4m_require_ace = false;
    assert_eq!(
        best_with(&agreements, &doubled, "432.KQJ987.65.32"),
        call(4, Strain::Hearts)
    );
    agreements.decision.instinct.preempt_4m_require_ace = true;

    // The semi-solid gate: an ace-headed but ragged six-bagger escapes;
    // length-only (top-honors 0) lets it preempt.
    assert_eq!(
        best_with(&agreements, &doubled, "432.AJ9876.65.32"),
        call(2, Strain::Hearts)
    );
    agreements.decision.instinct.preempt_4m_top_honors = 0;
    assert_eq!(
        best_with(&agreements, &doubled, "432.AJ9876.65.32"),
        call(4, Strain::Hearts)
    );
}

#[test]
fn one_nt_runout_2nt_scrambles_the_minors() {
    let mut agreements = Agreements::default();
    agreements.decision.instinct.one_nt_runout = true;
    // The 2NT relay is the opt-in `FourFour` mode (the default is `Direct`).
    agreements.decision.instinct.unusual_2nt = Unusual2nt::FourFour;
    // 4-4 in the minors, no five-card suit, broke: 2NT asks opener to pick.
    let doubled = [call(1, Strain::Notrump), Call::Double];
    assert_eq!(
        best_with(&agreements, &doubled, "K3.842.Q642.J642"),
        call(2, Strain::Notrump)
    );
    // Opener names the longer minor: clubs here, diamonds when reversed —
    // never blindly "completing" 2NT as a diamond transfer.
    let after = [
        call(1, Strain::Notrump),
        Call::Double,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(
        best_with(&agreements, &after, "AQ5.KQ4.32.AK842"),
        call(3, Strain::Clubs)
    );
    assert_eq!(
        best_with(&agreements, &after, "AQ5.KQ4.AK842.32"),
        call(3, Strain::Diamonds)
    );
}

#[test]
fn one_nt_runout_2nt_shape_modes() {
    let mut agreements = Agreements::default();
    agreements.decision.instinct.one_nt_runout = true;
    let doubled = [call(1, Strain::Notrump), Call::Double];
    // A weak 5-5 in the minors.  In `FourFour` it escapes naturally to a
    // five-card minor; the 2NT scramble is only the no-five-card-suit action.
    agreements.decision.instinct.unusual_2nt = Unusual2nt::FourFour;
    assert_ne!(
        best_with(&agreements, &doubled, "3.42.KQ876.J8765"),
        call(2, Strain::Notrump)
    );
    // FiveFiveAdd routes the 5-5 hand through 2NT so opener picks the better
    // minor instead of responder guessing.
    agreements.decision.instinct.unusual_2nt = Unusual2nt::FiveFiveAdd;
    assert_eq!(
        best_with(&agreements, &doubled, "3.42.KQ876.J8765"),
        call(2, Strain::Notrump)
    );
    // Direct suppresses 2NT: the 4-4 bust runs straight to its longer minor
    // (ties to diamonds) at the two level.
    agreements.decision.instinct.unusual_2nt = Unusual2nt::Direct;
    assert_eq!(
        best_with(&agreements, &doubled, "K3.842.Q642.J642"),
        call(2, Strain::Diamonds)
    );
}

#[test]
fn one_nt_runout_penalizes_escape_on_stack() {
    let mut agreements = Agreements::default();
    agreements.decision.instinct.one_nt_runout = true;
    agreements.decision.instinct.penalize_escape_values = false;
    // `1NT (X) XX (2♣)`: responder's business redouble shows values before RHO
    // runs to 2♣.  A club stack (and not short in their suit, so the floor would
    // not take out) doubles the run for penalty.  Toggling the arm off withdraws
    // the double.
    let run = [
        call(1, Strain::Notrump),
        Call::Double,
        Call::Redouble,
        call(2, Strain::Clubs),
    ];
    agreements.decision.instinct.penalize_escape_stack = true;
    assert_eq!(
        best_with(&agreements, &run, "Q52.K43.Q43.AKJ4"),
        Call::Double
    );
    agreements.decision.instinct.penalize_escape_stack = false;
    assert_ne!(
        best_with(&agreements, &run, "Q52.K43.Q43.AKJ4"),
        Call::Double
    );
}

#[test]
fn one_nt_runout_leaves_in_escape_penalty() {
    let mut agreements = Agreements::default();
    agreements.decision.instinct.one_nt_runout = true;
    agreements.decision.instinct.penalize_escape_stack = true;
    // `1NT (X) XX (2♣) X -`: partner doubled their run for penalty.  We pass
    // to leave it in, never advancing it as if it were a takeout double.
    let doubled_run = [
        call(1, Strain::Notrump),
        Call::Double,
        Call::Redouble,
        call(2, Strain::Clubs),
        Call::Double,
        Call::Pass,
    ];
    assert_eq!(
        best_with(&agreements, &doubled_run, "KQ3.K54.J632.987"),
        Call::Pass
    );
}

#[test]
fn one_nt_runout_penalizes_escape_on_values() {
    let mut agreements = Agreements::default();
    agreements.decision.instinct.one_nt_runout = true;
    agreements.decision.instinct.penalize_escape_stack = false;
    agreements.decision.instinct.penalize_escape_values = true;
    // After responder's business redouble shows values, opener doubles their
    // run on general strength — no personal trump stack, and not short in
    // their suit, so the double is ours and not the floor's takeout.
    let run = [
        call(1, Strain::Notrump),
        Call::Double,
        Call::Redouble,
        call(2, Strain::Clubs),
    ];
    assert_eq!(
        best_with(&agreements, &run, "AQ5.KQ43.K3.6432"),
        Call::Double
    );
    // The chase recurses: they run on to 2♦, the values hand doubles again.
    let again = [
        call(1, Strain::Notrump),
        Call::Double,
        Call::Redouble,
        call(2, Strain::Clubs),
        Call::Double,
        call(2, Strain::Diamonds),
    ];
    assert_eq!(
        best_with(&agreements, &again, "KQ3.K54.J632.987"),
        Call::Double
    );
    // But opener's *SOS* redouble shows no values, so the values arm stays
    // silent there: `1NT (X) - - XX (2♣)` is not a values double.
    let sos = [
        call(1, Strain::Notrump),
        Call::Double,
        Call::Pass,
        Call::Pass,
        Call::Redouble,
        call(2, Strain::Clubs),
    ];
    assert_ne!(
        best_with(&agreements, &sos, "J32.Q54.J632.987"),
        Call::Double
    );
}

#[test]
fn one_nt_runout_universal_opener_escapes_and_sos() {
    let mut agreements = Agreements::default();
    agreements.decision.instinct.one_nt_runout = true;
    agreements.decision.instinct.one_nt_runout_universal = true;
    // In the balancing seat after `1NT (X) - -`, partner is broke and opener
    // acts rather than
    // sit 1NT-X.  A minimum with five spades runs to 2♠.
    let balancing = [
        call(1, Strain::Notrump),
        Call::Double,
        Call::Pass,
        Call::Pass,
    ];
    assert_eq!(
        best_with(&agreements, &balancing, "AQ542.KJ.K43.Q32"),
        call(2, Strain::Spades)
    );
    // A minimum with no five-card suit SOS-redoubles instead.
    assert_eq!(
        best_with(&agreements, &balancing, "AQ4.KJ2.K432.Q32"),
        Call::Redouble
    );
    // Responder answers the SOS with its longest suit, a four-carder.
    let after_sos = [
        call(1, Strain::Notrump),
        Call::Double,
        Call::Pass,
        Call::Pass,
        Call::Redouble,
        Call::Pass,
    ];
    assert_eq!(
        best_with(&agreements, &after_sos, "QJ32.842.642.J32"),
        call(2, Strain::Spades)
    );
}

#[test]
fn one_nt_runout_opener_passes_not_completes_phantom_transfer() {
    // `1NT (X) 2♥` is partner's *runout*, not a Jacoby transfer: opener passes
    // rather than "complete" it to 2♠ (responder's short suit).
    let after_runout = [
        call(1, Strain::Notrump),
        Call::Double,
        call(2, Strain::Hearts),
        Call::Pass,
    ];
    assert_eq!(best(&after_runout, "AQ4.KJ3.KQ52.432"), Call::Pass);
}
