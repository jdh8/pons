use super::*;
use crate::bidding::agreements::Agreements;
use contract_bridge::auction::RelativeVulnerability;
use contract_bridge::{Bid, Strain};

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The shelled net's logits under the card the knobs currently describe
///
/// The card is read per call, not once, because several tests below arm a
/// knob and re-shell to assert the logits moved.
fn shelled(auction: &[Call], hand: &str) -> Logits {
    shelled_with(&Agreements::default(), auction, hand)
}

fn shelled_with(agreements: &Agreements, auction: &[Call], hand: &str) -> Logits {
    let hand: Hand = hand.parse().expect("valid test hand");
    let floor = ConfiguredFloorBba::new(
        Config::symmetric(&crate::bidding::card::american_card(agreements)),
        Arc::new(crate::bidding::instinct(agreements)),
    );
    let context =
        Context::new(RelativeVulnerability::NONE, auction).with_profile(agreements.decision);
    floor.classify(hand, &context)
}

fn configured_with(agreements: &Agreements, auction: &[Call], hand: &str) -> Vec<f32> {
    shelled_with(agreements, auction, hand)
        .iter()
        .map(|(_, logit)| *logit)
        .collect()
}

/// The card block reaches the net: flipping one row moves the logits.
///
/// The in-crate echo of `scripts/pair-flip-diagnostic.py`, and the check
/// that would have caught a v4 floor wired to a config it never attaches —
/// which would look exactly like "the convention is worth nothing" at gate
/// 2, with no other symptom.  It asserts the logits *move*, not that the
/// chosen call does: `Kickback 1430` decides about one auction in 700, so a
/// call-level assertion on an arbitrary hand would be asserting noise.
#[test]
fn the_configured_floor_reads_its_card() {
    let auction = [call(1, Strain::Spades), Call::Pass];
    let hand = "AQ32.K53.QJ4.A92";
    let mut agreements = Agreements::default();
    agreements.decision.reading.rkcb_variant = crate::bidding::instinct::RkcbVariant::Plain;
    let off = configured_with(&agreements, &auction, hand);
    agreements.decision.reading.rkcb_variant = crate::bidding::instinct::RkcbVariant::Kickback;
    let on = configured_with(&agreements, &auction, hand);
    assert_ne!(off, on, "the kickback row must reach the feature vector");
}

/// The forced rail answers off **this shell's** ladder, not a process-global
/// one
///
/// [`instinct`][crate::bidding::instinct] reads the pinned profile at build
/// time: its RKCB fields install the kickback answer table instead of the plain
/// one. While the ladder was a `LazyLock` static, the whole process shared
/// whichever arm happened to classify first — so `ab-kickback`, which holds
/// both arms at once under rayon, could answer a relocated `4♠` ask off a plain
/// ladder, nondeterministically.  Both shells here are built in one process and
/// must disagree.
///
/// `4♠` over agreed hearts asks in hearts (`kickback_answers_climb_from_four_spades`);
/// the ambient knob stays on so `forced` opens the window for both.
#[test]
fn the_configured_floor_answers_off_its_own_ladder() {
    use crate::bidding::instinct::{RkcbVariant, instinct};

    let auction = [
        call(1, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
        call(4, Strain::Spades),
        Call::Pass,
    ];
    let hand: Hand = "432.K765.5432.3".parse().expect("valid test hand");
    let card = Config::symmetric(&crate::bidding::card::american_card(
        &crate::bidding::agreements::Agreements::default(),
    ));

    let mut relocated_agreements = Agreements::default();
    relocated_agreements.decision.reading.rkcb_variant = RkcbVariant::Kickback;
    let relocated = Arc::new(instinct(&relocated_agreements));
    let mut plain_agreements = Agreements::default();
    plain_agreements.decision.reading.rkcb_variant = RkcbVariant::Plain;
    let plain = Arc::new(instinct(&plain_agreements));

    // Ambient on: `forced` recognizes the relocated ask for both shells, so the
    // only difference left is the ladder each was handed.
    let context = Context::new(RelativeVulnerability::NONE, &auction)
        .with_profile(relocated_agreements.decision);
    let answer = |ladder: Arc<Rules>| {
        let logits = ConfiguredFloorBba::new(card.clone(), ladder).classify(hand, &context);
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    };
    let on_relocated = answer(relocated);
    let on_plain = answer(plain);
    assert_eq!(
        on_relocated,
        call(4, Strain::Notrump),
        "one keycard is step 2 above the 4♠ ask"
    );
    assert_ne!(
        on_plain, on_relocated,
        "a plain ladder has no rule on the relocated rungs"
    );
}

/// Attaching the configured floor's card clones Context, but must retain
/// the decision scope shared by the surrounding trie resolution.
#[test]
fn configured_floor_clone_reuses_the_decision_cache() {
    let auction = [call(1, Strain::Spades), Call::Pass];
    let hand: Hand = "AQ32.K53.QJ4.A92".parse().expect("valid test hand");
    let floor = ConfiguredFloorBba::new(
        Config::symmetric(&crate::bidding::card::american_card(
            &crate::bidding::agreements::Agreements::default(),
        )),
        Arc::new(crate::bidding::instinct(&Agreements::default())),
    );
    let context = Context::new(RelativeVulnerability::NONE, &auction).with_decision_cache(hand);

    let first = floor.classify(hand, &context);
    let after_first = context
        .decision_cache_init_counts()
        .expect("decision cache attached");
    let second = floor.classify(hand, &context);
    assert_eq!(
        (&first.0)
            .into_iter()
            .map(|(_, x)| x.to_bits())
            .collect::<Vec<_>>(),
        (&second.0)
            .into_iter()
            .map(|(_, x)| x.to_bits())
            .collect::<Vec<_>>()
    );
    assert_eq!(context.decision_cache_init_counts(), Some(after_first));
    assert_eq!(after_first.0, 1, "configured features read inference once");
    assert!(after_first.1 <= 1);
    assert!(after_first.2 <= 1);
}

/// The shelled net's highest-logit call
fn best(auction: &[Call], hand: &str) -> Call {
    let logits = shelled(auction, hand);
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

/// The board that priced the whole kickback campaign.  After `1♦ - 1♠ - 2♦`
/// the ladder claims 4♥ as the diamond keycard ask — and responder here is
/// 6-6 in the majors and **void in diamonds**, so a 4♥ from this hand can
/// only mean hearts.  The kickback-blind net jumped to 4♥ anyway (it had
/// never seen 4♥ mean keycards), partner answered the phantom ask, and the
/// auction was passed out in a 4-1 fit on 171 of 2090 divergent boards.
///
/// BBA escapes by bidding the second suit cheaply instead, and the
/// mixed-regime net was distilled from a teacher that does exactly that.
/// The fix is not that 4♥ became legal-but-unattractive; it is that the net
/// stops offering it at all once the readings say the suit is spoken for.
#[test]
fn the_six_six_hand_stops_jumping_into_the_relocated_ask() {
    let auction = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let hand = "AQJT83.QT9875..6"; // ♠AQJT83 ♥QT9875 ♦— ♣6, board 229
    let mut agreements = Agreements::default();
    agreements.decision.reading.rkcb_variant = crate::bidding::instinct::RkcbVariant::Kickback;
    let logits = shelled_with(&agreements, &auction, hand);
    let with_kickback = (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty");
    assert_ne!(
        with_kickback,
        call(4, Strain::Hearts),
        "4♥ is the diamond ask on this face — the net must not bid it naturally"
    );
}

// The five §0.4 safety properties, enforced by the shell against the learned
// net.  The four forced rails delegate to `instinct`, so they reproduce
// its tested calls exactly; the legality rail exercises the net + mask.

#[test]
fn advancing_a_double_delegates_to_instinct() {
    // Partner doubled their 3♣ for takeout; the shell delegates to instinct,
    // reproducing its calls — advance a bust with an outside suit, defend with
    // four cards behind their suit (the settle floor, default on).
    let auction = [call(3, Strain::Clubs), Call::Double, Call::Pass];
    assert_eq!(best(&auction, "96432.J85.9742.2"), call(3, Strain::Spades));
    assert_eq!(best(&auction, "964.J85.974.9632"), Call::Pass);
}

#[test]
fn keycard_window_delegates_in_competition() {
    // The reading-drift A/B's worst board: inside this live contested
    // keycard window the bare net mis-answered 5♣, redoubled the doubled
    // answer, and left 5♣XX to play in a 2-2 club fit (−24 IMPs).  A
    // keycard conversation is forced-rail territory: the shell delegates
    // to instinct, which answers 1430 and places the contract.
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
    assert_eq!(
        best(&ask, "AKT8.8432.KQJ.65"),
        call(5, Strain::Hearts),
        "two keycards, no queen → 5♥, interference notwithstanding"
    );
    let doubled = [ask.as_slice(), &[call(5, Strain::Hearts), Call::Double]].concat();
    let placed = best(&doubled, "J9654.AJ.AT74.74");
    assert_ne!(
        placed,
        Call::Redouble,
        "a doubled 1430 answer is never redoubled"
    );
    assert_eq!(
        placed,
        call(5, Strain::Spades),
        "the asker places the contract over their double"
    );
}

#[test]
fn forced_to_game_never_passes_below_game() {
    // `2♣ - 2♦ - 2NT -`: the strong 2♣ opening and game-forcing 2♦ waiting
    // response force game, so the shell delegates to instinct and never passes
    // below game.
    let auction = [
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    assert_eq!(best(&auction, "QJ52.K43.T62.J32"), call(3, Strain::Notrump));
    assert_eq!(best(&auction, "3.QJ9854.K32.J32"), call(4, Strain::Hearts));
}

#[test]
fn completes_partners_transfer_over_notrump() {
    // We opened 1NT and partner transferred 2♦ (hearts): the shell delegates
    // to instinct and completes with 2♥.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(best(&auction, "AQ32.KJ5.KQ4.Q92"), call(2, Strain::Hearts));
}

#[test]
fn forced_game_steps_aside_when_penalizing() {
    // `2♣ - 2♦ - 2NT (3♦) X -`: they sacrifice after the forcing wait and
    // partner doubles for penalty.  The auction still forces game, so the shell
    // delegates to instinct, which shows the six-card suit rather than a
    // stopperless 3NT.
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
    let chosen = best(&auction, "K3.KQ4.65.QJ8765");
    assert_eq!(chosen, call(4, Strain::Clubs));
    assert_ne!(chosen, call(3, Strain::Notrump));
}

#[test]
fn doubles_only_their_live_bids() {
    // Not a forced auction → the net + legality mask.  The call to beat is
    // our own 2♠ (partner raised our overcall); the net emits a finite Double
    // logit, but doubling our own side is illegal, so the mask zeroes it —
    // while Pass stays finite so a distribution always exists.
    let auction = [
        call(1, Strain::Hearts),
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let logits = shelled(&auction, "92.K53.AQJ42.962");
    assert_eq!(*logits.0.get(Call::Double), f32::NEG_INFINITY);
    assert!(logits.0.get(Call::Pass).is_finite());
}

/// The competitive accountant vetoes a hopeless five-level save, and only for
/// the hand that deserves it
///
/// `1♠ (5♥)` to the advancer: with a bust the economics price `5♠` below
/// defending `5♥` by more than `COMPETITIVE_MARGIN`, so the gate masks it —
/// while Pass keeps a finite logit, because a distribution must survive every
/// stage.  Knob-off is byte-identical by construction (the stage returns before
/// reading anything), which the `off` row pins at the firing node itself.  The
/// same auction with a real hand is left alone: this is a priced decision, not
/// an auction-shaped rule.
#[test]
fn the_competitive_gate_vetoes_the_phantom_save() {
    let auction = [call(1, Strain::Spades), call(5, Strain::Hearts)];
    let five_spades = call(5, Strain::Spades);
    let mut agreements = Agreements::default();
    agreements.decision.instinct.competitive_accountant = false;

    let off = shelled_with(&agreements, &auction, "43.9862.7532.J864");
    assert!(
        off.0[five_spades].is_finite(),
        "knob-off leaves the net's own ranking alone"
    );

    agreements.decision.instinct.competitive_accountant = true;
    let before = crate::bidding::instinct::competitive_counts()[0];
    let bust = shelled_with(&agreements, &auction, "43.9862.7532.J864");
    assert!(
        crate::bidding::instinct::competitive_counts()[0] > before,
        "the veto is attributed"
    );
    assert_eq!(
        bust.0[five_spades],
        f32::NEG_INFINITY,
        "the save is priced out"
    );
    assert!(bust.0[Call::Pass].is_finite(), "Pass is always legal");
    assert!(bust.has_mass(), "a distribution survives the stage");

    let values = shelled_with(&agreements, &auction, "AQ32.5.QJ42.A932");
    assert!(
        values.0[five_spades].is_finite(),
        "the veto prices the hand, not the auction"
    );
}

/// The accountant pushes a double at the five-level but not over a slam
///
/// `1♠ (5♥)` with a defensive 15-count: the economics beat passing by more
/// than `COMPETITIVE_MARGIN`, so Pass is charged.  Lift the same auction to
/// `6♥` — where the case for doubling is only stronger — and the gate keeps
/// quiet, because past `DOUBLE_PUSH_CEILING` a double is an information
/// transfer that runs them into the slam that makes (the first A/B's whole
/// loss tail).  Pass must land exactly on its knob-off logit: the cap skips
/// the action, it does not scale it.
#[test]
fn the_accountant_pushes_no_double_over_a_slam() {
    let hand = "AQ.K94.AT42.KQ76";
    let agreements = Agreements::default();
    let mut silent = agreements;
    silent.decision.instinct.competitive_accountant = false;

    let five = [call(1, Strain::Spades), call(5, Strain::Hearts)];
    let before = crate::bidding::instinct::competitive_counts()[2];
    let pushed = shelled_with(&agreements, &five, hand);
    assert!(
        crate::bidding::instinct::competitive_counts()[2] > before,
        "the five-level demotion is attributed"
    );
    assert!(
        pushed.0[Call::Pass] < shelled_with(&silent, &five, hand).0[Call::Pass],
        "Pass is charged when the double is the better bet"
    );
    assert!(pushed.has_mass(), "a distribution survives the stage");

    let slam = [call(1, Strain::Spades), call(6, Strain::Hearts)];
    let before = crate::bidding::instinct::competitive_counts()[2];
    let capped = shelled_with(&agreements, &slam, hand);
    assert_eq!(
        crate::bidding::instinct::competitive_counts()[2],
        before,
        "no double is pushed over a slam"
    );
    assert_eq!(
        capped.0[Call::Pass],
        shelled_with(&silent, &slam, hand).0[Call::Pass],
        "the cap skips the action rather than scaling it"
    );
}
