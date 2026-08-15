use super::*;
use crate::bidding::card::american_card;
use crate::bidding::trie::Trie;
use crate::bidding::{Range, Relative};

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

fn test_hand() -> Hand {
    "AKQ2.KQ53.QJ4.92".parse().expect("valid test hand")
}

#[test]
fn test_relative_vulnerability() {
    assert_eq!(
        relative(AbsoluteVulnerability::NS, Seat::North),
        RelativeVulnerability::WE,
    );
    assert_eq!(
        relative(AbsoluteVulnerability::NS, Seat::East),
        RelativeVulnerability::THEY,
    );
    assert_eq!(
        relative(AbsoluteVulnerability::ALL, Seat::West),
        RelativeVulnerability::ALL,
    );
    assert_eq!(
        relative(AbsoluteVulnerability::NONE, Seat::South),
        RelativeVulnerability::NONE,
    );
}

#[test]
fn test_contested_auction_facts() {
    // `1♠ - 2♣ (X)`: we act next.
    let auction = [
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Double,
    ];
    let context = Context::new(RelativeVulnerability::NONE, &auction);

    assert!(context.we_bid(Strain::Spades));
    assert!(context.we_bid(Strain::Clubs));
    assert!(!context.they_bid(Strain::Spades));
    assert_eq!(context.partner_last_bid(), Some(Bid::new(2, Strain::Clubs)));
    assert_eq!(context.partner_last_suit(), Some(Suit::Clubs));
    assert_eq!(context.last_bid(), Some(Bid::new(2, Strain::Clubs)));
    assert_eq!(context.penalty(), Penalty::Doubled);
    assert!(!context.undisturbed());
    assert!(!context.passed_hand());
    assert!(!context.partner_passed_hand());
    assert_eq!(context.opener_seat(), Some(1));
}

#[test]
fn incremental_turn_contexts_match_fresh_construction() {
    let auction = [
        bid(1, Strain::Clubs),
        Call::Double,
        Call::Redouble,
        Call::Pass,
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Hearts),
        Call::Pass,
    ];
    for final_vul in [
        RelativeVulnerability::NONE,
        RelativeVulnerability::WE,
        RelativeVulnerability::THEY,
        RelativeVulnerability::ALL,
    ] {
        let timeline = Context::at_each_turn(final_vul, &auction);
        assert_eq!(timeline.len(), auction.len() + 1);
        for (depth, actual) in timeline.iter().enumerate() {
            let vul = if (auction.len() - depth).is_multiple_of(2) {
                final_vul
            } else {
                flipped(final_vul)
            };
            let expected = Context::new(vul, &auction[..depth]);
            assert_eq!(
                format!("{actual:?}"),
                format!("{expected:?}"),
                "context differs at depth {depth}",
            );
        }
    }

    let mut cursor = ContextCursor::new();
    for depth in 0..=auction.len() {
        assert_eq!(
            cursor.phase(),
            super::super::book::Phase::of(&auction[..depth])
        );
        if let Some(&call) = auction.get(depth) {
            cursor.push(call);
        }
    }
}

/// The eleven mechanical facts are derived **twice, by two different
/// algorithms** — [`ContextCursor`] maintains them incrementally, O(1) a turn,
/// while [`Context::new`] rescans the whole auction.  Nothing makes them agree
/// by construction, so this walks the frozen corpus and holds them to the same
/// answer at every prefix of every auction; the sibling test above covers one
/// hand-picked auction across all four vulnerabilities.
///
/// A drift here is a silent wrong-bidding bug: the reading walk uses the
/// cursor and a fresh classification uses the scan.
#[test]
fn incremental_and_rescanned_facts_agree_on_the_frozen_corpus() {
    use crate::bidding::book::performance_support::parse_corpus;

    let positions = parse_corpus().expect("valid frozen corpus");
    assert!(positions.len() > 100, "the corpus was found");
    for position in &positions {
        for (depth, incremental) in Context::at_each_turn(position.vul, &position.auction)
            .iter()
            .enumerate()
        {
            let vul = if (position.auction.len() - depth).is_multiple_of(2) {
                position.vul
            } else {
                flipped(position.vul)
            };
            assert_eq!(
                format!("{incremental:?}"),
                format!("{:?}", Context::new(vul, &position.auction[..depth])),
                "position {} diverges at depth {depth}",
                position.id,
            );
        }
    }
}

#[test]
fn test_their_suits_and_min_level() {
    // They opened 1♥ and raised to 2♥ over partner's 1♠ overcall.
    let auction = [
        bid(1, Strain::Hearts),
        bid(1, Strain::Spades),
        bid(2, Strain::Hearts),
    ];
    let context = Context::new(RelativeVulnerability::NONE, &auction);

    assert_eq!(context.their_suits().collect::<Vec<_>>(), [Suit::Hearts]);
    assert!(context.we_bid(Strain::Spades));
    assert_eq!(context.min_level(Strain::Hearts), Some(Level::new(3)));
    assert_eq!(context.min_level(Strain::Spades), Some(Level::new(2)));
    assert_eq!(context.min_level(Strain::Clubs), Some(Level::new(3)));
}

#[test]
fn test_min_level_exhausted() {
    let auction = [bid(7, Strain::Notrump)];
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    assert_eq!(context.min_level(Strain::Spades), None);
    assert_eq!(context.min_level(Strain::Notrump), None);
}

#[test]
fn test_passed_hands() {
    // `- - 1♥ (1♠)`: we act next.
    let auction = [
        Call::Pass,
        Call::Pass,
        bid(1, Strain::Hearts),
        bid(1, Strain::Spades),
    ];
    let context = Context::new(RelativeVulnerability::NONE, &auction);

    assert!(context.passed_hand());
    assert!(!context.partner_passed_hand());
    assert_eq!(context.leading_passes(), 2);
    assert_eq!(context.opener_seat(), Some(3));
    assert_eq!(context.seat_to_open(), None);
    // Past the leading passes, and their 1♠ overcall does not overwrite it.
    assert_eq!(context.opening_bid(), Some(Bid::new(1, Strain::Hearts)));
}

#[test]
fn test_opening_bid() {
    assert_eq!(
        Context::new(RelativeVulnerability::NONE, &[]).opening_bid(),
        None,
    );
    assert_eq!(
        Context::new(RelativeVulnerability::NONE, &[Call::Pass; 3]).opening_bid(),
        None,
    );
}

#[test]
fn test_seat_to_open() {
    let passes = [Call::Pass; 4];

    for len in 0..=3 {
        let context = Context::new(RelativeVulnerability::NONE, &passes[..len]);
        // SAFETY: `len` is at most 3, so the cast is safe.
        #[allow(clippy::cast_possible_truncation)]
        let seat = len as u8 + 1;
        assert_eq!(context.seat_to_open(), Some(seat));
    }

    let passed_out = Context::new(RelativeVulnerability::NONE, &passes);
    assert_eq!(passed_out.seat_to_open(), None);
}

#[test]
fn bare_context_stays_uncached() {
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    assert!(context.decision_cache.is_none());
    assert!(matches!(context.inferences(), Cow::Owned(_)));
    assert!(matches!(context.inferences(), Cow::Owned(_)));
}

#[test]
fn decision_values_initialize_once() {
    let hand = test_hand();
    let context = Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(hand);

    let uncached_inferences = Inferences::read(&context);
    let cached_inferences = context.inferences();
    assert!(matches!(cached_inferences, Cow::Borrowed(_)));
    assert_eq!(*cached_inferences, uncached_inferences);
    assert!(matches!(context.inferences(), Cow::Borrowed(_)));
    let first = context.trick_estimates(hand);
    let second = context.trick_estimates(hand);
    let uncached_tricks = trick_estimates_with_auction_on(
        &context.decision_profile(),
        hand,
        &Inferences::read(&context),
        context.auction(),
    );
    assert_eq!(first.bit_pattern(), second.bit_pattern());
    assert_eq!(first.bit_pattern(), uncached_tricks.bit_pattern());
    let first_interpretation = context.interpretation();
    let second_interpretation = context.interpretation();
    assert_eq!(first_interpretation, Interpretation::read(&context));
    assert_eq!(first_interpretation, second_interpretation);

    assert_eq!(context.decision_cache_init_counts(), Some((1, 1, 1)));
}

#[test]
fn configured_clone_preserves_decision_cache() {
    let context = Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(test_hand());
    let cache = Arc::clone(context.decision_cache.as_ref().expect("attached cache"));
    let config = Config::symmetric(&american_card(
        &crate::bidding::agreements::Agreements::default(),
    ));
    let configured = context.with_config(&config);

    assert_eq!(configured.revision, cache.revision);
    assert!(Arc::ptr_eq(
        configured.decision_cache.as_ref().expect("preserved cache"),
        &cache,
    ));
    assert!(configured.active_decision_cache().is_some());
}

#[test]
fn structural_builders_reject_an_attached_cache() {
    let auction = [];
    let trie = Trie::new();
    let partnership = Partnership::default();
    let context =
        Context::new(RelativeVulnerability::NONE, &auction).with_decision_cache(test_hand());
    let cache = Arc::clone(context.decision_cache.as_ref().expect("attached cache"));

    let prefixed = context
        .clone()
        .with_prefixes(trie.common_prefixes(&auction));
    assert_eq!(prefixed.revision, cache.revision + 1);
    assert!(prefixed.active_decision_cache().is_none());
    assert!(Arc::ptr_eq(
        prefixed
            .decision_cache
            .as_ref()
            .expect("logically retained"),
        &cache,
    ));

    let opposed = context.with_system(&partnership);
    assert_eq!(opposed.revision, cache.revision + 1);
    assert!(opposed.active_decision_cache().is_none());
}

#[test]
fn wrong_hand_does_not_fill_scoped_trick_cache() {
    let owner = test_hand();
    let other = "98432.K53.QJ4.92".parse().expect("valid test hand");
    let context = Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(owner);

    let _ = context.trick_estimates(other);
    let cache = context.decision_cache.as_deref().expect("attached cache");
    // `trick_estimates` reads through `net_inferences`, so under the shipped
    // `legacy_view` the one read lands in the legacy slot rather than the
    // plain one.  Either way it is exactly one, and the *trick* cache — the
    // hand-scoped one this test is about — stays empty for a foreign hand.
    let (plain, tricks, interpretations) = context
        .decision_cache_init_counts()
        .expect("attached cache");
    assert_eq!(plain + context.legacy_inference_inits().unwrap_or(0), 1);
    assert_eq!((tricks, interpretations), (0, 0));
    assert!(cache.trick_estimates.get().is_none());
}

#[test]
fn debug_omits_cache_mechanics() {
    let context = Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(test_hand());
    let debug = format!("{context:?}");
    assert!(!debug.contains("revision"));
    assert!(!debug.contains("decision_cache"));
}

#[cfg(debug_assertions)]
#[test]
fn decision_cache_rejects_cross_thread_use() {
    let context = Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(test_hand());

    std::thread::scope(|scope| {
        let result = scope
            .spawn(move || {
                let _ = context.inferences();
            })
            .join();
        assert!(result.is_err());
    });
}

#[test]
fn threads_use_their_explicit_profiles() {
    let handles = [false, true].map(|eval_auction| {
        std::thread::spawn(move || {
            let profile = DecisionProfile {
                eval_auction,
                ..DecisionProfile::default()
            };
            let context = Context::new(RelativeVulnerability::NONE, &[])
                .with_profile(profile)
                .with_decision_cache(test_hand());
            let uncached = Inferences::read(&context);
            assert_eq!(*context.inferences(), uncached);
            assert_eq!(context.decision_cache_init_counts(), Some((1, 0, 0)));
            context
                .decision_cache
                .as_deref()
                .expect("decision cache attached")
                .profile
                .eval_auction
        })
    });

    let profiles = handles.map(|handle| handle.join().expect("profile thread"));
    assert_eq!(profiles, [false, true]);
}

/// `legacy_view` hands the **nets** the pre-ceilings reading while every other
/// consumer keeps the true one.
///
/// The fixture is the call that found the whole problem (N2 of
/// docs/one-notrump-competitive.md): over their Muiderberg `2♠`, our `2♠`
/// lebensohl relay is gated `points(..=8)`, and until
/// [`strength_ceilings`][field@crate::bidding::ReadingProfile::strength_ceilings]
/// partner read it as `6..=37` — unlimited.  With the ceilings on it reads
/// `6..=8`; with `legacy_view` on as well, `net_inferences` still says
/// `6..=37`, because every shipped net was trained on floor-only boxes.
#[test]
fn legacy_view_serves_the_nets_the_pre_ceilings_reading() {
    // `1NT (2♠) 2NT (P)` — the seat two back (`Relative::Partner`) relayed.
    let auction = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Spades),
        bid(2, Strain::Notrump),
        Call::Pass,
    ];
    let hand = test_hand();
    let partner_points = |legacy_view: bool, ceilings: bool| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.decision.reading.strength_ceilings = ceilings;
        agreements.decision.legacy_view = legacy_view;
        let partnership = crate::american(&agreements).bind();
        // The prefix-bearing context `Partnership::infer` builds — without
        // the trie's common prefixes the relay never routes to its rule.
        let context = partnership
            .prefixed_context(RelativeVulnerability::NONE, &auction)
            .with_decision_cache(hand);
        let truth = context
            .inferences()
            .announced(Relative::Partner)
            .strength
            .points;
        let served = context
            .net_inferences()
            .announced(Relative::Partner)
            .strength
            .points;
        // Memoised: two more reads must not re-walk the auction.
        let _ = context.net_inferences();
        let _ = context.net_inferences();
        let inits = context.legacy_inference_inits().expect("an active cache");
        assert_eq!(inits, usize::from(legacy_view), "legacy reads");
        (truth, served)
    };

    // Ceilings off: the knob is inert, and the view has nothing to hold back.
    assert_eq!(
        partner_points(false, false),
        (Range::new(6, 37), Range::new(6, 37))
    );
    assert_eq!(
        partner_points(true, false),
        (Range::new(6, 37), Range::new(6, 37))
    );
    // Ceilings on: the truth carries the eight.
    assert_eq!(
        partner_points(false, true),
        (Range::new(6, 8), Range::new(6, 8))
    );
    // …and the view hands the nets the pre-ceilings box while it stays true.
    assert_eq!(
        partner_points(true, true),
        (Range::new(6, 8), Range::new(6, 37))
    );
}
