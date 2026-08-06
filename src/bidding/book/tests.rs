use super::{DEAL_CACHE_MIN_DEPTH, Phase, StanceDealState, performance_support};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

#[test]
fn table_deal_state_activates_the_causal_cache_at_deep_prefixes() {
    use crate::bidding::System;
    use crate::bidding::american::american_book;
    use contract_bridge::auction::RelativeVulnerability;

    let stance = american_book().against();
    let hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
    let mut state = stance.new_deal_state().expect("stance deal state");
    assert!(
        state
            .downcast_ref::<StanceDealState>()
            .expect("typed stance state")
            .authoring
            .is_none()
    );

    let shallow = vec![Call::Pass; DEAL_CACHE_MIN_DEPTH - 1];
    let _ = stance.classify_in_deal(
        hand,
        RelativeVulnerability::NONE,
        &shallow,
        Some(state.as_mut()),
    );
    assert!(
        state
            .downcast_ref::<StanceDealState>()
            .expect("typed stance state")
            .authoring
            .is_none(),
        "shallow auctions should avoid cache setup",
    );

    let deep = vec![Call::Pass; DEAL_CACHE_MIN_DEPTH];
    let _ = stance.classify_in_deal(
        hand,
        RelativeVulnerability::NONE,
        &deep,
        Some(state.as_mut()),
    );
    assert!(
        state
            .downcast_ref::<StanceDealState>()
            .expect("typed stance state")
            .authoring
            .is_some(),
        "deep auctions should enter the append-only causal cache",
    );
}

#[test]
fn finalized_decoder_matches_legacy_resolution_on_frozen_corpus() {
    use crate::bidding::american::american_book;
    use crate::bidding::context::{Context, flipped};

    let stance = american_book().against();
    let corpus = performance_support::parse_corpus().expect("valid frozen corpus");
    for position in corpus {
        for depth in 0..=position.auction.len() {
            let prefix = &position.auction[..depth];
            let vul = if (position.auction.len() - depth).is_multiple_of(2) {
                position.vul
            } else {
                flipped(position.vul)
            };
            let context = Context::new(vul, prefix);
            let legacy = stance.trie_for(prefix).resolve(&context, prefix);
            let compiled = stance.decoder_for(prefix).resolve(&context, prefix);
            match (legacy, compiled) {
                (None, None) => {}
                (Some((legacy, legacy_provenance)), Some(compiled)) => {
                    assert!(
                        std::ptr::eq(legacy, compiled.classifier),
                        "classifier differs at corpus {} prefix {depth}",
                        position.id,
                    );
                    assert_eq!(
                        legacy_provenance, compiled.provenance,
                        "provenance differs at corpus {} prefix {depth}",
                        position.id,
                    );
                }
                (legacy, compiled) => panic!(
                    "resolution presence differs at corpus {} prefix {depth}: legacy={}, compiled={}",
                    position.id,
                    legacy.is_some(),
                    compiled.is_some(),
                ),
            }
        }
    }
}

#[test]
fn compiled_authored_projection_matches_legacy_on_frozen_corpus() {
    use crate::bidding::american::american_book;
    use crate::bidding::inference::{
        ReadingScope, assert_compiled_authoring_projection_parity, set_announced_reading,
        set_envelope_union_reading, set_fallback_projection, set_pass_exclusion_reading,
        set_pass_reading, set_reading_scope, set_table_alert_reading,
    };

    let corpus = performance_support::parse_corpus().expect("valid frozen corpus");
    let profiles = [
        (ReadingScope::Alerted, true, true, true, false, false, true),
        (ReadingScope::All, false, false, false, false, true, false),
        (ReadingScope::None, true, true, true, true, true, true),
    ];
    for (scope, union, fallback, pass, exclusion, announced, table) in profiles {
        set_reading_scope(scope);
        set_envelope_union_reading(union);
        set_fallback_projection(fallback);
        set_pass_reading(pass);
        set_pass_exclusion_reading(exclusion);
        set_announced_reading(announced);
        set_table_alert_reading(table);
        let stance = american_book().against();
        for position in &corpus {
            let context = stance.prefixed_context(position.vul, &position.auction);
            assert_compiled_authoring_projection_parity(&context);
        }
    }

    set_reading_scope(ReadingScope::Alerted);
    set_envelope_union_reading(true);
    set_fallback_projection(true);
    set_pass_reading(true);
    set_pass_exclusion_reading(false);
    set_announced_reading(false);
    set_table_alert_reading(true);
}

#[test]
fn append_only_step_cache_matches_from_scratch_frozen_prefixes() {
    use crate::bidding::american::american_book;
    use crate::bidding::context::flipped;
    use crate::bidding::inference::{AuthoringStepCache, assert_step_cache_projection_parity};

    let stance = american_book().against();
    let corpus = performance_support::parse_corpus().expect("valid frozen corpus");
    let mut cache_hits = 0usize;
    for position in corpus {
        let mut caches = [AuthoringStepCache::new(), AuthoringStepCache::new()];
        for depth in 0..=position.auction.len() {
            let vul = if (position.auction.len() - depth).is_multiple_of(2) {
                position.vul
            } else {
                flipped(position.vul)
            };
            cache_hits += usize::from(assert_step_cache_projection_parity(
                &stance,
                vul,
                &position.auction[..depth],
                &mut caches[depth % 2],
            ));
        }
    }
    assert!(
        cache_hits > 2_000,
        "too many prefixes dropped to the slow path"
    );
}

#[test]
fn step_cache_drops_to_legacy_after_middeal_profile_change() {
    use crate::bidding::american::american_book;
    use crate::bidding::inference::{AuthoringStepCache, set_envelope_union_reading};
    use contract_bridge::auction::RelativeVulnerability;

    let stance = american_book().against();
    let auction = [bid(1, Strain::Notrump), Call::Pass];
    let mut cache = AuthoringStepCache::new();
    assert!(
        cache
            .prepare(&stance, RelativeVulnerability::NONE, &[])
            .is_some()
    );
    set_envelope_union_reading(false);
    assert!(
        cache
            .prepare(&stance, RelativeVulnerability::NONE, &auction)
            .is_none(),
        "profile change must disable this deal cache"
    );
    set_envelope_union_reading(true);
}

#[test]
fn step_cache_is_bound_to_stance_identity_and_probe_revision() {
    use crate::bidding::american::american_book;
    use crate::bidding::inference::AuthoringStepCache;
    use contract_bridge::auction::RelativeVulnerability;

    let stance = american_book().against();
    let clone = stance.clone();
    let mut clone_cache = AuthoringStepCache::new();
    assert!(
        clone_cache
            .prepare(&stance, RelativeVulnerability::NONE, &[])
            .is_some()
    );
    assert!(
        clone_cache
            .prepare(
                &clone,
                RelativeVulnerability::NONE,
                &[Call::Pass, Call::Pass],
            )
            .is_some(),
        "a clone preserves the stance cache identity"
    );

    let other = american_book().against();
    let mut other_cache = AuthoringStepCache::new();
    assert!(
        other_cache
            .prepare(&stance, RelativeVulnerability::NONE, &[])
            .is_some()
    );
    assert!(
        other_cache
            .prepare(&other, RelativeVulnerability::NONE, &[])
            .is_none(),
        "a cache must not cross independently bound stances"
    );

    let mut probed = stance.clone();
    let mut probe_cache = AuthoringStepCache::new();
    assert!(
        probe_cache
            .prepare(&probed, RelativeVulnerability::NONE, &[])
            .is_some()
    );
    let _ = probed.probe(0, 0xCA_C4E);
    assert!(
        probe_cache
            .prepare(&probed, RelativeVulnerability::NONE, &[])
            .is_none(),
        "probing must replace the stance cache identity"
    );
}

/// [`Stance::probe`] stores boxes for high-traffic keys; the knob-on
/// reading tightens and the knob-off reading is byte-identical to an
/// unprobed stance.  The **floorless** book keeps the self-play cheap in
/// debug (rule evaluation only — the neural floor is ~75 ms/board
/// unoptimized); 2,000 boards give the `1♦ -` key ~240 expected samples,
/// clearing the [`Stance::MIN_SAMPLES`] floor.
#[test]
fn probe_stores_and_reads_high_traffic_keys() {
    use contract_bridge::auction::RelativeVulnerability;

    let mut stance = crate::bidding::american::american_book().against();
    let plain = crate::bidding::american::american_book().against();
    let report = stance.probe(2000, 0x9B0BE);
    assert!(report.keys > 0, "probe stored nothing");
    assert!(report.drifted <= report.keys);

    let auction = [bid(1, Strain::Diamonds), Call::Pass];
    // Knob off — byte-identical to an unprobed stance.
    let off = stance.infer(RelativeVulnerability::NONE, &auction);
    let unprobed = plain.infer(RelativeVulnerability::NONE, &auction);
    assert_eq!(off.rho(), unprobed.rho());

    crate::bidding::set_probed_reading(true);
    let on = stance.infer(RelativeVulnerability::NONE, &auction);
    crate::bidding::set_probed_reading(false);
    // The probed box only tightens the symbolic band — and it reads suit
    // lengths on the passer, which no symbolic path can (the pass gate is
    // points-only).
    assert!(on.rho().strength.points.max <= off.rho().strength.points.max);
    assert!(
        on.rho().length(Suit::Diamonds).max < 13,
        "no probed length ceiling on the passer"
    );
}

/// The vacuous-scoped fold serves coverage and nothing else, on the
/// ledger's own hole (`1♦ (2♣) 2♠` read partner ♠ `0..13`,
/// docs/reading-drift-handoff.md): partner's contested free bid fills
/// exactly the axes the symbolic reading left fully open, an opponent's
/// probed box never folds (v1's refuted tightening was the opponents
/// read as limited), and a key through a still-constructive prefix never
/// folds (filling constructive axes was the 2026-07-31 smoke's −0.67
/// IMPs/board of net-OOD grand blasts).
#[test]
fn probed_vacuous_fills_only_open_axes_on_contested_own_calls() {
    use contract_bridge::auction::RelativeVulnerability;

    use crate::bidding::{Envelope, Range};

    let mut stance = crate::bidding::american::american_book().against();
    let boxed = |spades: Range, points: Range| {
        let mut envelope = Envelope::unknown();
        envelope.lengths[Suit::Spades as usize] = spades;
        envelope.strength.points = points;
        envelope
    };
    // Partner's free bid in a contested prefix — the served key.
    stance.probed.insert(
        vec![ONE_DIAMOND, TWO_CLUBS, TWO_SPADES],
        boxed(Range::new(4, 8), Range::new(8, 16)),
    );
    // Their overcall — an opponent's key, never served.
    stance.probed.insert(
        vec![ONE_DIAMOND, TWO_CLUBS],
        boxed(Range::new(0, 2), Range::new(6, 14)),
    );
    // Our opening — a key through a still-constructive prefix, never served.
    stance.probed.insert(
        vec![ONE_DIAMOND],
        boxed(Range::new(0, 3), Range::new(10, 20)),
    );

    let auction = [ONE_DIAMOND, TWO_CLUBS, TWO_SPADES, P];
    let off = stance.infer(RelativeVulnerability::NONE, &auction);
    crate::bidding::set_probed_vacuous_reading(true);
    let on = stance.infer(RelativeVulnerability::NONE, &auction);
    crate::bidding::set_probed_vacuous_reading(false);

    assert_eq!(on.rho(), off.rho(), "an opponent's probed box folded");
    assert_eq!(on.me(), off.me(), "a constructive-prefix key folded");

    let (off_p, on_p) = (off.partner(), on.partner());
    // The hole this knob exists for: the free bid's suit reads nothing.
    assert_eq!(off_p.length(Suit::Spades), Range::FULL_LENGTH);
    assert_eq!(on_p.length(Suit::Spades), Range::new(4, 8));
    // Already-read axes stay byte-identical; open ones take the box.
    if off_p.strength.points == Range::FULL_POINTS {
        assert_eq!(on_p.strength.points, Range::new(8, 16));
    } else {
        assert_eq!(on_p.strength.points, off_p.strength.points);
    }
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        assert_eq!(on_p.length(suit), off_p.length(suit));
    }
}

const P: Call = Call::Pass;
const ONE_DIAMOND: Call = bid(1, Strain::Diamonds);
const ONE_HEART: Call = bid(1, Strain::Hearts);
const ONE_SPADE: Call = bid(1, Strain::Spades);
const TWO_CLUBS: Call = bid(2, Strain::Clubs);
const TWO_HEARTS: Call = bid(2, Strain::Hearts);
const TWO_SPADES: Call = bid(2, Strain::Spades);

#[test]
fn test_phase_before_any_opening() {
    assert_eq!(Phase::of(&[]), Phase::Constructive);
    assert_eq!(Phase::of(&[P]), Phase::Constructive);
    assert_eq!(Phase::of(&[P, P, P]), Phase::Constructive);
    assert_eq!(Phase::of(&[P, P, P, P]), Phase::Constructive);
}

#[test]
fn test_phase_when_we_opened_undisturbed() {
    assert_eq!(Phase::of(&[ONE_HEART, P]), Phase::Constructive);
    assert_eq!(
        Phase::of(&[ONE_HEART, P, TWO_HEARTS, P]),
        Phase::Constructive
    );
    assert_eq!(Phase::of(&[P, P, ONE_SPADE, P]), Phase::Constructive);
}

#[test]
fn test_phase_when_they_intervened() {
    assert_eq!(Phase::of(&[ONE_HEART, TWO_CLUBS]), Phase::Competitive);
    assert_eq!(Phase::of(&[ONE_HEART, Call::Double]), Phase::Competitive);
    assert_eq!(Phase::of(&[P, ONE_HEART, Call::Double]), Phase::Competitive);
    assert_eq!(
        Phase::of(&[ONE_HEART, P, TWO_HEARTS, TWO_SPADES]),
        Phase::Competitive
    );
    // Our own redouble is not a disturbance, but their double is.
    assert_eq!(
        Phase::of(&[ONE_SPADE, Call::Double, Call::Redouble, P]),
        Phase::Competitive
    );
}

#[test]
fn test_phase_when_they_opened() {
    assert_eq!(Phase::of(&[ONE_HEART]), Phase::Defensive);
    assert_eq!(Phase::of(&[P, P, ONE_SPADE]), Phase::Defensive);
    assert_eq!(Phase::of(&[ONE_HEART, TWO_CLUBS, P]), Phase::Defensive);
    assert_eq!(
        Phase::of(&[ONE_HEART, P, TWO_HEARTS, TWO_SPADES, P]),
        Phase::Defensive
    );
}

/// `explain_call` attributes a book call to its exact node and a floor
/// call to the instinct fallback, each with a renderable rule.
#[test]
fn explain_call_names_book_and_floor_rules() {
    use crate::bidding::american::american_instinct;
    use contract_bridge::Hand;
    use contract_bridge::auction::RelativeVulnerability;

    let stance = american_instinct().against();

    // A book decision: the routine 1♠ opening resolves at the exact root
    // node (no fallback taken) and names the rule that produced it.
    let opener: Hand = "AKJ84.K52.Q4.982".parse().expect("valid test hand");
    let (provenance, rule) = stance
        .explain_call(opener, RelativeVulnerability::NONE, &[], ONE_SPADE)
        .expect("an opening classifies");
    assert_eq!(provenance.fallback, None);
    let rule = rule.expect("the opening table is a Rules ladder");
    assert!(!rule.description.is_empty());

    // A floor decision: opener's competitive long-suit rebid comes from the
    // instinct floor (depth 0 + fallback), mirroring the provenance the
    // instinct tests assert, and its winning rule still renders.
    let auction = [
        bid(1, Strain::Diamonds),
        ONE_HEART,
        P,
        TWO_HEARTS, // they raise; opener holds a self-sufficient 7-card suit
    ];
    let one_suiter: Hand = "765.A.AKJT984.63".parse().expect("valid test hand");
    let (provenance, rule) = stance
        .explain_call(
            one_suiter,
            RelativeVulnerability::NONE,
            &auction,
            bid(3, Strain::Diamonds),
        )
        .expect("a legal auction classifies");
    assert_eq!(provenance.depth, 0);
    assert!(provenance.fallback.is_some());
    let rule = rule.expect("the instinct floor is a Rules ladder");
    assert!(!rule.description.is_empty());
}

/// Explanation re-evaluates the winning Rules ladder after resolution;
/// both passes must share the same causal decision cache.
#[test]
fn explanation_reuses_decision_initializers() {
    use crate::bidding::american::american_instinct;
    use contract_bridge::Hand;
    use contract_bridge::auction::RelativeVulnerability;

    let stance = american_instinct().against();
    let auction = [bid(1, Strain::Diamonds), ONE_HEART, P, TWO_HEARTS];
    let hand: Hand = "765.A.AKJT984.63".parse().expect("valid test hand");
    let context = stance.decision_context(hand, RelativeVulnerability::NONE, &auction);
    assert_eq!(context.decision_cache_init_counts(), Some((0, 0, 0)));

    let explained = stance
        .explain_call_in_context(hand, &context, bid(3, Strain::Diamonds))
        .expect("the scoped public helper explains the floor");
    assert!(explained.0.fallback.is_some());
    assert!(explained.1.is_some());
    let after_explanation = context
        .decision_cache_init_counts()
        .expect("decision cache attached");
    assert_eq!(
        after_explanation.0, 1,
        "resolution and explanation share inference"
    );
    assert!(after_explanation.0 <= 1);
    assert!(after_explanation.1 <= 1);
    assert!(after_explanation.2 <= 1);

    // Re-running the same helper cannot initialize anything again.  This
    // is the exact helper public `explain_call` invokes after entering its
    // decision scope, rather than a hand-copied approximation of that path.
    let repeated = stance
        .explain_call_in_context(hand, &context, bid(3, Strain::Diamonds))
        .expect("the scoped public helper explains twice");
    assert_eq!(repeated.0, explained.0);
    assert_eq!(
        context.decision_cache_init_counts(),
        Some(after_explanation)
    );
}

/// The fresh-context path is the in-process oracle for the scoped cache.
/// Compare float representations rather than `f32` equality so signed
/// zeroes, infinities, and any NaN payloads cannot drift unnoticed.
#[test]
fn public_paths_match_uncached_reference_bit_for_bit() {
    use crate::bidding::american::american_instinct;
    use contract_bridge::Hand;
    use contract_bridge::auction::RelativeVulnerability;

    let stance = american_instinct().against();
    let assert_classification = |hand: Hand, auction: &[Call]| {
        let (actual, actual_provenance) = stance
            .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
            .expect("the public path classifies");
        let (reference, reference_provenance) = stance
            .classify_with_provenance_uncached(hand, RelativeVulnerability::NONE, auction)
            .expect("the legacy path classifies");

        assert_eq!(actual_provenance, reference_provenance);
        for ((call, actual), (reference_call, reference)) in
            (&actual.0).into_iter().zip(&reference.0)
        {
            assert_eq!(call, reference_call);
            assert_eq!(
                actual.to_bits(),
                reference.to_bits(),
                "logit bits differ for {call:?} at {auction:?}"
            );
        }
    };

    // Exact authored node in the constructive book.
    let opener: Hand = "AKJ84.K52.Q4.982".parse().expect("valid test hand");
    assert_classification(opener, &[]);

    // Their opening routes through the defensive book.
    let overcaller: Hand = "AQJ9.K42.763.542".parse().expect("valid test hand");
    assert_classification(overcaller, &[ONE_HEART]);

    // Rejected/missing exact continuation falling through to the
    // deterministic floor in the competitive book.
    let auction = [bid(1, Strain::Diamonds), ONE_HEART, P, TWO_HEARTS];
    let one_suiter: Hand = "765.A.AKJT984.63".parse().expect("valid test hand");
    assert_classification(one_suiter, &auction);

    let assert_explanation = |hand: Hand, auction: &[Call], call: Call| {
        let (actual_provenance, actual) = stance
            .explain_call(hand, RelativeVulnerability::NONE, auction, call)
            .expect("the public path explains");
        let (reference_provenance, reference) = stance
            .explain_call_uncached(hand, RelativeVulnerability::NONE, auction, call)
            .expect("the legacy path explains");
        assert_eq!(actual_provenance, reference_provenance);

        match (actual, reference) {
            (Some(actual), Some(reference)) => {
                assert_eq!(actual.index, reference.index);
                assert_eq!(actual.label, reference.label);
                assert_eq!(actual.description, reference.description);
                assert_eq!(actual.alert, reference.alert);
            }
            (None, None) => {}
            (actual, reference) => panic!(
                "explanation presence differs at {auction:?}: actual={actual:?}, reference={reference:?}"
            ),
        }
    };

    assert_explanation(opener, &[], ONE_SPADE);
    assert_explanation(one_suiter, &auction, bid(3, Strain::Diamonds));
}

#[test]
fn cached_reference_parity_across_reading_and_evaluator_profiles() {
    use crate::bidding::american::{american, american_instinct};
    use crate::bidding::inference::ReadingScope;
    use contract_bridge::Hand;
    use contract_bridge::auction::RelativeVulnerability;

    struct Profile {
        scope: ReadingScope,
        union: bool,
        exclusion: bool,
        eval_auction: bool,
        eval_shape: bool,
        blind: bool,
        bilans: bool,
        collar: bool,
        configured: bool,
    }

    let profiles = [
        Profile {
            scope: ReadingScope::Alerted,
            union: true,
            exclusion: false,
            eval_auction: true,
            eval_shape: false,
            blind: false,
            bilans: false,
            collar: false,
            configured: false,
        },
        Profile {
            scope: ReadingScope::None,
            union: false,
            exclusion: false,
            eval_auction: false,
            eval_shape: false,
            blind: false,
            bilans: false,
            collar: false,
            configured: false,
        },
        Profile {
            scope: ReadingScope::All,
            union: true,
            exclusion: true,
            eval_auction: true,
            eval_shape: true,
            blind: false,
            bilans: true,
            collar: false,
            configured: false,
        },
        Profile {
            scope: ReadingScope::Alerted,
            union: true,
            exclusion: false,
            eval_auction: true,
            eval_shape: false,
            blind: false,
            bilans: true,
            collar: true,
            configured: false,
        },
        Profile {
            scope: ReadingScope::Alerted,
            union: true,
            exclusion: false,
            eval_auction: true,
            eval_shape: false,
            blind: false,
            bilans: false,
            collar: false,
            configured: true,
        },
        Profile {
            scope: ReadingScope::Alerted,
            union: true,
            exclusion: false,
            eval_auction: true,
            eval_shape: true,
            blind: true,
            bilans: false,
            collar: false,
            configured: true,
        },
    ];
    let vulnerabilities = [
        RelativeVulnerability::NONE,
        RelativeVulnerability::WE,
        RelativeVulnerability::THEY,
        RelativeVulnerability::ALL,
    ];
    let auction = [bid(1, Strain::Diamonds), ONE_HEART, P, TWO_HEARTS];
    let hand: Hand = "765.A.AKJT984.63".parse().expect("valid test hand");

    for (index, profile) in profiles.into_iter().enumerate() {
        crate::bidding::set_reading_scope(profile.scope);
        crate::bidding::set_envelope_union_reading(profile.union);
        crate::bidding::set_pass_exclusion_reading(profile.exclusion);
        crate::bidding::evaluator::set_eval_auction(profile.eval_auction);
        crate::bidding::evaluator::set_eval_shape(profile.eval_shape);
        crate::bidding::features::set_blind_inference(profile.blind);
        crate::bidding::instinct::set_bilans_floor(profile.bilans);
        crate::bidding::instinct::set_net_collar(profile.collar);

        let stance = if profile.configured {
            american().against()
        } else {
            american_instinct().against()
        };
        let vul = vulnerabilities[index % vulnerabilities.len()];
        let cached = stance
            .classify_with_provenance(hand, vul, &auction)
            .expect("cached profile classifies");
        let uncached = stance
            .classify_with_provenance_uncached(hand, vul, &auction)
            .expect("reference profile classifies");
        assert_eq!(cached.1, uncached.1, "profile {index}");
        for ((call, actual), (reference_call, reference)) in
            (&cached.0.0).into_iter().zip(&uncached.0.0)
        {
            assert_eq!(call, reference_call);
            assert_eq!(
                actual.to_bits(),
                reference.to_bits(),
                "profile {index}, call {call:?}"
            );
        }
    }

    // Restore this worker thread's shipped defaults for later tests.
    crate::bidding::set_reading_scope(ReadingScope::Alerted);
    crate::bidding::set_envelope_union_reading(true);
    crate::bidding::set_pass_exclusion_reading(false);
    crate::bidding::evaluator::set_eval_auction(true);
    crate::bidding::evaluator::set_eval_shape(false);
    crate::bidding::features::set_blind_inference(false);
    crate::bidding::instinct::set_bilans_floor(true);
    crate::bidding::instinct::set_net_collar(false);
}

/// Component-by-component release parity over the frozen stage-1 corpus.
///
/// Unlike the live-deal sweep below, this names every cached intermediate:
/// full inference unions (including announced-box order), every evaluator
/// feature generation, the forward-pass result, interpretation, routing,
/// legal selection, and explanation attribution.
#[test]
#[ignore = "release component parity over 512 frozen positions"]
fn cached_and_uncached_match_frozen_performance_corpus() {
    use super::ExplainedRule;
    use crate::bidding::american::american;
    use crate::bidding::array::Logits;
    use crate::bidding::evaluator::trick_estimates_with_auction;
    use crate::bidding::features::{features_eval, features_eval_v3, features_eval_v4};
    use crate::bidding::inference::{Inferences, ReadingScope};
    use crate::bidding::instinct::Interpretation;
    use crate::bidding::trie::Provenance;
    use contract_bridge::auction::Auction;

    fn set_shipped_profile() {
        crate::bidding::set_reading_scope(ReadingScope::Alerted);
        crate::bidding::set_envelope_union_reading(true);
        crate::bidding::set_pass_exclusion_reading(false);
        crate::bidding::evaluator::set_eval_auction(true);
        crate::bidding::evaluator::set_eval_shape(false);
        crate::bidding::features::set_blind_inference(false);
        crate::bidding::instinct::set_bilans_floor(true);
        crate::bidding::instinct::set_net_collar(false);
    }

    fn assert_float_bits(actual: &[f32], reference: &[f32], component: &str, position: u16) {
        assert_eq!(
            actual.len(),
            reference.len(),
            "{component} width at position {position}"
        );
        for (index, (&actual, &reference)) in actual.iter().zip(reference).enumerate() {
            assert_eq!(
                actual.to_bits(),
                reference.to_bits(),
                "{component}[{index}] bits at position {position}"
            );
        }
    }

    fn assert_logits_bits(actual: &Logits, reference: &Logits, auction: &[Call], position: u16) {
        for ((call, actual), (reference_call, reference)) in
            (&actual.0).into_iter().zip(&reference.0)
        {
            assert_eq!(call, reference_call);
            assert_eq!(
                actual.to_bits(),
                reference.to_bits(),
                "logit bits for {call:?} at position {position}, {auction:?}"
            );
        }
    }

    fn legal_call(logits: &Logits, auction: &[Call]) -> Call {
        let mut played = Auction::new();
        played
            .try_extend(auction.iter().copied())
            .expect("the frozen corpus contains legal prefixes");
        let mut scored: Vec<(Call, f32)> = logits
            .iter()
            .map(|(call, &logit)| (call, logit))
            .filter(|&(_, logit)| logit.is_finite())
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
        scored
            .into_iter()
            .map(|(call, _)| call)
            .find(|&call| played.can_push(call).is_ok())
            .unwrap_or(Call::Pass)
    }

    fn assert_explanation_identity(
        actual: Option<(Provenance, Option<ExplainedRule>)>,
        reference: Option<(Provenance, Option<ExplainedRule>)>,
        position: u16,
    ) {
        match (actual, reference) {
            (
                Some((actual_provenance, actual_rule)),
                Some((reference_provenance, reference_rule)),
            ) => {
                assert_eq!(
                    actual_provenance, reference_provenance,
                    "explanation provenance at position {position}"
                );
                match (actual_rule, reference_rule) {
                    (Some(actual), Some(reference)) => {
                        assert_eq!(actual.index, reference.index, "position {position}");
                        assert_eq!(actual.label, reference.label, "position {position}");
                        assert_eq!(
                            actual.description, reference.description,
                            "position {position}"
                        );
                        assert_eq!(actual.alert, reference.alert, "position {position}");
                    }
                    (None, None) => {}
                    (actual, reference) => panic!(
                        "explanation presence differs at position {position}: actual={actual:?}, reference={reference:?}"
                    ),
                }
            }
            (None, None) => {}
            (actual, reference) => panic!(
                "explanation resolution differs at position {position}: actual={actual:?}, reference={reference:?}"
            ),
        }
    }

    set_shipped_profile();
    let positions = performance_support::parse_corpus().expect("valid frozen corpus");
    assert_eq!(positions.len(), performance_support::POSITION_COUNT);
    let stance = american().against();

    for position in positions {
        let id = position.id;
        let hand = position.hand;
        let auction = position.auction.as_slice();
        let cached_context = stance.decision_context(hand, position.vul, auction);
        let uncached_context = stance.prefixed_context(position.vul, auction);

        let cached_inferences = cached_context.inferences();
        let uncached_inferences = Inferences::read(&uncached_context);
        assert_eq!(
            *cached_inferences, uncached_inferences,
            "full inference payload and box order at position {id}"
        );

        assert_float_bits(
            &features_eval(hand, &cached_inferences),
            &features_eval(hand, &uncached_inferences),
            "evaluator-v2-features",
            id,
        );
        assert_float_bits(
            &features_eval_v3(hand, &cached_inferences, auction),
            &features_eval_v3(hand, &uncached_inferences, auction),
            "evaluator-v3-features",
            id,
        );
        assert_float_bits(
            &features_eval_v4(hand, &cached_inferences, auction),
            &features_eval_v4(hand, &uncached_inferences, auction),
            "evaluator-v4-features",
            id,
        );

        let cached_tricks = cached_context.trick_estimates(hand);
        let uncached_tricks = trick_estimates_with_auction(hand, &uncached_inferences, auction);
        assert_eq!(
            cached_tricks.bit_pattern(),
            uncached_tricks.bit_pattern(),
            "trick-estimate bits at position {id}"
        );

        assert_eq!(
            cached_context.interpretation(),
            Interpretation::read(&uncached_context),
            "auction interpretation at position {id}"
        );

        let cached = stance
            .trie_for(auction)
            .classify_floored(hand, &cached_context, auction)
            .expect("the cached default stance is total");
        let uncached = stance
            .classify_with_provenance_uncached(hand, position.vul, auction)
            .expect("the uncached default stance is total");
        assert_eq!(cached.1, uncached.1, "provenance at position {id}");
        assert_logits_bits(&cached.0, &uncached.0, auction, id);

        let cached_call = legal_call(&cached.0, auction);
        let uncached_call = legal_call(&uncached.0, auction);
        assert_eq!(cached_call, uncached_call, "legal call at position {id}");
        assert_explanation_identity(
            stance.explain_call_in_context(hand, &cached_context, cached_call),
            stance.explain_call_uncached(hand, position.vul, auction, uncached_call),
            id,
        );

        let counts = cached_context
            .decision_cache_init_counts()
            .expect("the cached corpus path has a decision scope");
        assert!(
            counts.0 <= 1,
            "inference initialized {counts:?} at position {id}"
        );
        assert!(
            counts.1 <= 1,
            "evaluator initialized {counts:?} at position {id}"
        );
        assert!(
            counts.2 <= 1,
            "interpretation initialized {counts:?} at position {id}"
        );
    }

    // This ignored test is often selected beside other release checks on a
    // reused harness worker; leave every setting it pins at the shipped value.
    set_shipped_profile();
}

/// Same-process release sweep of the causal cached path against its
/// pre-cache oracle. Uses the identical deal/dealer/vulnerability schedule
/// as `examples/smoke-default` and checks every live decision before either
/// call is allowed to advance the auction.
#[test]
#[ignore = "release parity sweep over 20,000 complete deals"]
fn cached_and_uncached_match_over_twenty_thousand_deals() {
    use crate::bidding::american::american;
    use crate::bidding::array::Logits;
    use crate::bidding::context::relative;
    use contract_bridge::auction::Auction;
    use contract_bridge::deck::full_deal;
    use contract_bridge::{AbsoluteVulnerability, Seat};
    use rand::SeedableRng as _;

    fn legal_call(logits: &Logits, auction: &Auction) -> Call {
        let mut scored: Vec<(Call, f32)> = logits
            .iter()
            .map(|(call, &logit)| (call, logit))
            .filter(|&(_, logit)| logit.is_finite())
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
        scored
            .into_iter()
            .map(|(call, _)| call)
            .find(|&call| auction.can_push(call).is_ok())
            .unwrap_or(Call::Pass)
    }

    let stance = american().against();
    let vulnerabilities = [
        AbsoluteVulnerability::NONE,
        AbsoluteVulnerability::NS,
        AbsoluteVulnerability::EW,
        AbsoluteVulnerability::ALL,
    ];
    let mut decisions = 0usize;

    for board in 0..20_000usize {
        let deal = full_deal(&mut rand::rngs::StdRng::seed_from_u64(
            1u64.wrapping_add(board as u64),
        ));
        let dealer = Seat::ALL[board % 4];
        let table_vul = vulnerabilities[(board / 4) % 4];
        let mut auction = Auction::new();

        while !auction.has_ended() {
            let seat = Seat::ALL[(dealer as usize + auction.len()) % 4];
            let vul = relative(table_vul, seat);
            let cached = stance
                .classify_with_provenance(deal[seat], vul, &auction)
                .expect("the default stance is total");
            let uncached = stance
                .classify_with_provenance_uncached(deal[seat], vul, &auction)
                .expect("the reference stance is total");

            assert_eq!(cached.1, uncached.1, "provenance on board {board}");
            for ((call, actual), (reference_call, reference)) in
                (&cached.0.0).into_iter().zip(&uncached.0.0)
            {
                assert_eq!(call, reference_call);
                assert_eq!(
                    actual.to_bits(),
                    reference.to_bits(),
                    "logit bits for {call:?} on board {board} at {auction:?}"
                );
            }

            let cached_call = legal_call(&cached.0, &auction);
            let uncached_call = legal_call(&uncached.0, &auction);
            assert_eq!(
                cached_call, uncached_call,
                "legal selection on board {board} at {auction:?}"
            );
            auction.push(cached_call);
            decisions += 1;
        }
    }

    assert!(
        decisions >= 80_000,
        "unexpectedly shallow corpus: {decisions}"
    );
}
