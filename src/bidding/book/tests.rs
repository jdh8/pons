use super::{DEAL_CACHE_MIN_DEPTH, PartnershipDealState, Phase, performance_support};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

#[test]
fn table_deal_state_activates_the_causal_cache_at_deep_prefixes() {
    use crate::bidding::Bidder;
    use crate::bidding::american::american_book;
    use contract_bridge::auction::RelativeVulnerability;

    let partnership = american_book(&crate::bidding::agreements::Agreements::default()).bind();
    let hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
    let mut state = partnership
        .new_deal_state()
        .expect("partnership deal state");
    assert!(
        state
            .downcast_ref::<PartnershipDealState>()
            .expect("typed partnership state")
            .authoring
            .is_none()
    );

    let shallow = vec![Call::Pass; DEAL_CACHE_MIN_DEPTH - 1];
    let _ = partnership.classify_in_deal(
        hand,
        RelativeVulnerability::NONE,
        &shallow,
        Some(state.as_mut()),
    );
    assert!(
        state
            .downcast_ref::<PartnershipDealState>()
            .expect("typed partnership state")
            .authoring
            .is_none(),
        "shallow auctions should avoid cache setup",
    );

    let deep = vec![Call::Pass; DEAL_CACHE_MIN_DEPTH];
    let _ = partnership.classify_in_deal(
        hand,
        RelativeVulnerability::NONE,
        &deep,
        Some(state.as_mut()),
    );
    assert!(
        state
            .downcast_ref::<PartnershipDealState>()
            .expect("typed partnership state")
            .authoring
            .is_some(),
        "deep auctions should enter the append-only causal cache",
    );
}

#[test]
fn finalized_decoder_matches_legacy_resolution_on_frozen_corpus() {
    use crate::bidding::american::american_book;
    use crate::bidding::context::{Context, flipped};

    let partnership = american_book(&crate::bidding::agreements::Agreements::default()).bind();
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
            let legacy = partnership.trie_for(prefix).resolve(&context, prefix);
            let compiled = partnership.decoder_for(prefix).resolve(&context, prefix);
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
    use crate::bidding::inference::{ReadingScope, assert_compiled_authoring_projection_parity};

    let corpus = performance_support::parse_corpus().expect("valid frozen corpus");
    let profiles = [
        (ReadingScope::Alerted, true, true, true, false, true),
        (ReadingScope::All, false, false, false, true, false),
        (ReadingScope::None, true, true, true, true, true),
    ];
    for (scope, union, fallback, pass, announced, table) in profiles {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.decision.reading.scope = scope;
        agreements.decision.reading.envelope_union = union;
        agreements.decision.reading.fallback_projection = fallback;
        agreements.decision.reading.pass = pass;
        agreements.decision.reading.announced = announced;
        agreements.decision.reading.table_alerts = table;
        let partnership = american_book(&agreements).bind();
        for position in &corpus {
            let context = partnership.prefixed_context(position.vul, &position.auction);
            assert_compiled_authoring_projection_parity(&context);
        }
    }
}

#[test]
fn append_only_step_cache_matches_from_scratch_frozen_prefixes() {
    use crate::bidding::american::american_book;
    use crate::bidding::context::flipped;
    use crate::bidding::inference::{AuthoringStepCache, assert_step_cache_projection_parity};

    let partnership = american_book(&crate::bidding::agreements::Agreements::default()).bind();
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
                &partnership,
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

/// A partnership pins its reading profile, so a mid-deal setting flip reaches this
/// cache only through [`Partnership::profile_mut`] — which invalidates the partnership's
/// cache identity, and with it the deal cache.  The profile arm of `prepare`'s
/// guard is therefore belt-and-braces: no path moves a pinned profile without
/// also bumping the identity checked just before it.
#[test]
fn step_cache_drops_to_legacy_after_middeal_edit() {
    use crate::bidding::american::american_book;
    use crate::bidding::inference::AuthoringStepCache;
    use contract_bridge::auction::RelativeVulnerability;

    let mut partnership = american_book(&crate::bidding::agreements::Agreements::default()).bind();
    let auction = [bid(1, Strain::Notrump), Call::Pass];
    let mut cache = AuthoringStepCache::new();
    assert!(
        cache
            .prepare(&partnership, RelativeVulnerability::NONE, &[])
            .is_some()
    );
    assert!(
        cache
            .prepare(&partnership, RelativeVulnerability::NONE, &auction)
            .is_some(),
        "extending the auction must not disturb this deal cache"
    );
    partnership.profile_mut().reading.envelope_union = false;
    assert!(
        cache
            .prepare(&partnership, RelativeVulnerability::NONE, &auction)
            .is_none(),
        "moving the pinned profile must disable this deal cache"
    );
}

#[test]
fn step_cache_is_bound_to_partnership_identity_and_probe_revision() {
    use crate::bidding::american::american_book;
    use crate::bidding::inference::AuthoringStepCache;
    use contract_bridge::auction::RelativeVulnerability;

    let partnership = american_book(&crate::bidding::agreements::Agreements::default()).bind();
    let clone = partnership.clone();
    let mut clone_cache = AuthoringStepCache::new();
    assert!(
        clone_cache
            .prepare(&partnership, RelativeVulnerability::NONE, &[])
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
        "a clone preserves the partnership cache identity"
    );

    let other = american_book(&crate::bidding::agreements::Agreements::default()).bind();
    let mut other_cache = AuthoringStepCache::new();
    assert!(
        other_cache
            .prepare(&partnership, RelativeVulnerability::NONE, &[])
            .is_some()
    );
    assert!(
        other_cache
            .prepare(&other, RelativeVulnerability::NONE, &[])
            .is_none(),
        "a cache must not cross independently bound partnerships"
    );

    let mut probed = partnership.clone();
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
        "probing must replace the partnership cache identity"
    );
}

/// [`Partnership::probe`] stores boxes for high-traffic keys; the knob-on
/// reading tightens and the knob-off reading is byte-identical to an
/// unprobed partnership.  The **floorless** book keeps the self-play cheap in
/// debug (rule evaluation only — the neural floor is ~75 ms/board
/// unoptimized); 2,000 boards give the `1♦ -` key ~240 expected samples,
/// clearing the [`Partnership::MIN_SAMPLES`] floor.
#[test]
fn probe_stores_and_reads_high_traffic_keys() {
    use contract_bridge::auction::RelativeVulnerability;

    let mut partnership =
        crate::bidding::american::american_book(&crate::bidding::agreements::Agreements::default())
            .bind();
    let plain =
        crate::bidding::american::american_book(&crate::bidding::agreements::Agreements::default())
            .bind();
    let report = partnership.probe(2000, 0x9B0BE);
    assert!(report.keys > 0, "probe stored nothing");
    assert!(report.drifted <= report.keys);

    let auction = [bid(1, Strain::Diamonds), Call::Pass];
    // Knob off — byte-identical to an unprobed partnership.
    let off = partnership.infer(RelativeVulnerability::NONE, &auction);
    let unprobed = plain.infer(RelativeVulnerability::NONE, &auction);
    assert_eq!(off.rho(), unprobed.rho());

    partnership.profile_mut().reading.probed = true;
    let on = partnership.infer(RelativeVulnerability::NONE, &auction);
    partnership.profile_mut().reading.probed = false;
    // The probed box only tightens the symbolic band — and it reads suit
    // lengths on the passer, which no symbolic path can (the pass gate is
    // points-only).
    assert!(on.rho().strength.points.max <= off.rho().strength.points.max);
    assert!(
        on.rho().length(Suit::Diamonds).max < 13,
        "no probed length ceiling on the passer"
    );
}

/// The per-key harvest fold is a count sum and a per-axis span, so the order
/// boards arrive in cannot move it.  That is the whole licence for the
/// `rayon` feature reducing in whatever order its pool finishes: break this
/// and a parallel probe stops matching a sequential one.
#[test]
fn observed_merge_is_order_insensitive() {
    use super::Observed;
    use crate::bidding::constraint::PointScale;

    let hands: Vec<contract_bridge::Hand> = [
        "AKQ2.K53.QJ4.T92",
        "5.AQJT98.K7.A543",
        "QJT98.4.A65432.7",
        "A2.K3.QJ54.J8765",
    ]
    .iter()
    .map(|hand| hand.parse().expect("valid hand"))
    .collect();

    // One board's contribution, then folded in two unrelated orders — the
    // seed is empty, exactly as a fresh shard's accumulator is.
    let fold = |order: &[usize]| {
        order.iter().fold(Observed::new(), |mut acc, &index| {
            let mut one = Observed::new();
            one.add(PointScale::Hcp, hands[index]);
            acc.merge(&one);
            acc
        })
    };
    let straight = fold(&[0, 1, 2, 3]);
    assert!(straight.boxed().is_some(), "nothing widened to compare");
    assert_eq!(straight.count, fold(&[3, 1, 0, 2]).count);
    assert_eq!(straight.boxed(), fold(&[3, 1, 0, 2]).boxed());
    assert_eq!(straight.boxed(), fold(&[2, 0, 3, 1]).boxed());
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

    let mut partnership =
        crate::bidding::american::american_book(&crate::bidding::agreements::Agreements::default())
            .bind();
    // The hole this knob fills exists under the alert-gated reading; under the
    // shipped `ReadingScope::All` the free bid's own rule already publishes
    // its suit, so pin the regime the knob was built for.
    partnership.profile_mut().reading.scope = crate::bidding::inference::ReadingScope::Alerted;
    let boxed = |spades: Range, points: Range| {
        let mut envelope = Envelope::unknown();
        envelope.lengths[Suit::Spades as usize] = spades;
        envelope.strength.points = points;
        envelope
    };
    // Partner's free bid in a contested prefix — the served key.
    partnership.probed.insert(
        vec![ONE_DIAMOND, TWO_CLUBS, TWO_SPADES],
        boxed(Range::new(4, 8), Range::new(8, 16)),
    );
    // Their overcall — an opponent's key, never served.
    partnership.probed.insert(
        vec![ONE_DIAMOND, TWO_CLUBS],
        boxed(Range::new(0, 2), Range::new(6, 14)),
    );
    // Our opening — a key through a still-constructive prefix, never served.
    partnership.probed.insert(
        vec![ONE_DIAMOND],
        boxed(Range::new(0, 3), Range::new(10, 20)),
    );

    let auction = [ONE_DIAMOND, TWO_CLUBS, TWO_SPADES, P];
    let off = partnership.infer(RelativeVulnerability::NONE, &auction);
    partnership.profile_mut().reading.probed_vacuous = true;
    let on = partnership.infer(RelativeVulnerability::NONE, &auction);
    partnership.profile_mut().reading.probed_vacuous = false;

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

    let partnership = american_instinct(&crate::bidding::agreements::Agreements::default()).bind();

    // A book decision: the routine 1♠ opening resolves at the exact root
    // node (no fallback taken) and names the rule that produced it.
    let opener: Hand = "AKJ84.K52.Q4.982".parse().expect("valid test hand");
    let (provenance, rule) = partnership
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
    let (provenance, rule) = partnership
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

    let partnership = american_instinct(&crate::bidding::agreements::Agreements::default()).bind();
    let auction = [bid(1, Strain::Diamonds), ONE_HEART, P, TWO_HEARTS];
    let hand: Hand = "765.A.AKJT984.63".parse().expect("valid test hand");
    let context = partnership.decision_context(hand, RelativeVulnerability::NONE, &auction);
    assert_eq!(context.decision_cache_init_counts(), Some((0, 0, 0)));

    let explained = partnership
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
    let repeated = partnership
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

    let partnership = american_instinct(&crate::bidding::agreements::Agreements::default()).bind();
    let assert_classification = |hand: Hand, auction: &[Call]| {
        let (actual, actual_provenance) = partnership
            .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
            .expect("the public path classifies");
        let (reference, reference_provenance) = partnership
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
        let (actual_provenance, actual) = partnership
            .explain_call(hand, RelativeVulnerability::NONE, auction, call)
            .expect("the public path explains");
        let (reference_provenance, reference) = partnership
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
        eval_auction: bool,
        eval_shape: bool,
        blind: bool,
        accountant: bool,
        collar: bool,
        configured: bool,
    }

    let profiles = [
        Profile {
            scope: ReadingScope::Alerted,
            union: true,
            eval_auction: true,
            eval_shape: false,
            blind: false,
            accountant: false,
            collar: false,
            configured: false,
        },
        Profile {
            scope: ReadingScope::None,
            union: false,
            eval_auction: false,
            eval_shape: false,
            blind: false,
            accountant: false,
            collar: false,
            configured: false,
        },
        Profile {
            scope: ReadingScope::All,
            union: true,
            eval_auction: true,
            eval_shape: true,
            blind: false,
            accountant: true,
            collar: false,
            configured: false,
        },
        Profile {
            scope: ReadingScope::Alerted,
            union: true,
            eval_auction: true,
            eval_shape: false,
            blind: false,
            accountant: true,
            collar: true,
            configured: false,
        },
        Profile {
            scope: ReadingScope::Alerted,
            union: true,
            eval_auction: true,
            eval_shape: false,
            blind: false,
            accountant: false,
            collar: false,
            configured: true,
        },
        Profile {
            scope: ReadingScope::Alerted,
            union: true,
            eval_auction: true,
            eval_shape: true,
            blind: true,
            accountant: false,
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
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.decision.eval_auction = profile.eval_auction;
        agreements.decision.eval_shape = profile.eval_shape;
        agreements.decision.blind_inference = profile.blind;
        agreements.decision.reading.scope = profile.scope;
        agreements.decision.reading.envelope_union = profile.union;
        agreements.decision.instinct.accountant_floor = profile.accountant;
        agreements.decision.instinct.net_collar = profile.collar;

        let partnership = if profile.configured {
            american(&agreements).bind()
        } else {
            american_instinct(&agreements).bind()
        };
        let vul = vulnerabilities[index % vulnerabilities.len()];
        let cached = partnership
            .classify_with_provenance(hand, vul, &auction)
            .expect("cached profile classifies");
        let uncached = partnership
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
    use crate::bidding::inference::Inferences;
    use crate::bidding::instinct::Interpretation;
    use crate::bidding::trie::Provenance;
    use contract_bridge::auction::Auction;

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

    let positions = performance_support::parse_corpus().expect("valid frozen corpus");
    assert_eq!(positions.len(), performance_support::POSITION_COUNT);
    let partnership = american(&crate::bidding::agreements::Agreements::default()).bind();

    for position in positions {
        let id = position.id;
        let hand = position.hand;
        let auction = position.auction.as_slice();
        let cached_context = partnership.decision_context(hand, position.vul, auction);
        let uncached_context = partnership.prefixed_context(position.vul, auction);

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

        let cached = partnership
            .trie_for(auction)
            .classify_floored(hand, &cached_context, auction)
            .expect("the cached default partnership is total");
        let uncached = partnership
            .classify_with_provenance_uncached(hand, position.vul, auction)
            .expect("the uncached default partnership is total");
        assert_eq!(cached.1, uncached.1, "provenance at position {id}");
        assert_logits_bits(&cached.0, &uncached.0, auction, id);

        let cached_call = legal_call(&cached.0, auction);
        let uncached_call = legal_call(&uncached.0, auction);
        assert_eq!(cached_call, uncached_call, "legal call at position {id}");
        assert_explanation_identity(
            partnership.explain_call_in_context(hand, &cached_context, cached_call),
            partnership.explain_call_uncached(hand, position.vul, auction, uncached_call),
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

    let partnership = american(&crate::bidding::agreements::Agreements::default()).bind();
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
            let cached = partnership
                .classify_with_provenance(deal[seat], vul, &auction)
                .expect("the default partnership is total");
            let uncached = partnership
                .classify_with_provenance_uncached(deal[seat], vul, &auction)
                .expect("the reference partnership is total");

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

/// Rows Phase 2b: the opponents' calls read off *their* books
///
/// `(1♣) - (2♦)`, seen from the 4th seat: both opponents' calls are theirs to
/// disclose.  American reads the 2♦ response as its own jump shift (6+♦, no
/// strength floor); the Dutch book that actually bid it shows 5+♦, no 4-card
/// major, game-forcing.  Declaring the opponent must move the reading of RHO
/// and leave our own side alone.
#[test]
fn a_declared_opponent_reads_their_calls_in_their_books() {
    use crate::bidding::american::american_book;
    use crate::bidding::dutch::dutch_book;
    use crate::bidding::inference::Relative;
    use contract_bridge::auction::RelativeVulnerability;

    let auction = [bid(1, Strain::Clubs), Call::Pass, bid(2, Strain::Diamonds)];
    let ours = american_book(&crate::bidding::agreements::Agreements::default()).bind();
    let dutch = dutch_book(&crate::bidding::agreements::Agreements::default()).bind();

    let read = |partnership: &super::Partnership| {
        *partnership
            .infer(RelativeVulnerability::NONE, &auction)
            .get(Relative::Rho)
    };
    let undeclared = read(&ours);
    let declared = read(
        &american_book(&crate::bidding::agreements::Agreements::default())
            .bind()
            .with_opponents(&dutch),
    );

    assert_eq!(undeclared.lengths[Suit::Diamonds as usize].min, 6);
    assert_eq!(undeclared.strength.points.min, 0);
    assert_eq!(declared.lengths[Suit::Diamonds as usize].min, 5);
    assert_eq!(declared.lengths[Suit::Hearts as usize].max, 3);
    assert_eq!(declared.strength.points.min, 13);

    // Declaring our own books changes nothing: that is the shipped default,
    // spelled out.
    assert_eq!(
        format!(
            "{:?}",
            read(
                &american_book(&crate::bidding::agreements::Agreements::default())
                    .bind()
                    .with_opponents(&ours)
            )
        ),
        format!("{undeclared:?}"),
    );
}

#[test]
fn all_scope_reads_unalerted_calls_from_both_declared_partnerships() {
    use crate::bidding::american::american_book;
    use crate::bidding::inference::{Range, ReadingScope, Relative};
    use contract_bridge::auction::RelativeVulnerability;

    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(3, Strain::Diamonds),
    ];
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.scope = ReadingScope::All;
    agreements.decision.reading.nt_overcall_gladiator = true;
    let their_books = american_book(&agreements).bind();

    let read = |partnership: &super::Partnership| {
        *partnership
            .infer(RelativeVulnerability::NONE, &auction)
            .get(Relative::Rho)
    };
    let undeclared = read(&american_book(&agreements).bind());
    let declared = read(
        &american_book(&agreements)
            .bind()
            .with_opponents(&their_books),
    );

    assert_eq!(undeclared.strength.points, Range::FULL_POINTS);
    assert!(declared.lengths[Suit::Diamonds as usize].min >= 5);
    assert!(declared.strength.points.min >= 10);
}

/// The keystone of the pin-at-build campaign: two systems built from one armed
/// [`Agreements`][crate::bidding::agreements::Agreements] answer identically on
/// the building thread and a second thread. Any classify-time read that
/// bypasses the pinned [`DecisionProfile`][super::DecisionProfile] diverges
/// here.
///
/// Live since stage 5, armed over all three layers: `ReadingProfile`'s 51
/// fields, `InstinctProfile`'s 34 fields, and the nine scalar fields
/// `DecisionProfile` holds directly.  (The counts drift as knobs land — they
/// are prose, not an assertion; the earlier "44 = 24 former foreign cells plus
/// 20 value-owned" split stopped adding up long ago and is dropped rather than
/// re-guessed.)  The values are deliberately meaningless — this arms a system
/// nobody plays — because what is under test is only that both threads see the
/// *same* one.
#[test]
fn partnership_pins_knobs_across_threads() {
    use crate::bidding::american;
    use crate::bidding::table::Table;
    use contract_bridge::auction::AbsoluteVulnerability;
    use contract_bridge::{FullDeal, Seat};
    use rand::SeedableRng as _;

    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading = crate::bidding::inference::ReadingProfile::nondefault();
    agreements.decision.instinct = crate::bidding::instinct::InstinctProfile::nondefault();
    agreements.decision.eval_auction = false;
    agreements.decision.eval_shape = true;
    agreements.decision.blind_inference = true;
    agreements.decision.two_over_one_force = false;
    agreements.decision.fuzzy_fifths = true;
    agreements.decision.fifths_companion = crate::bidding::constraint::FifthsCompanion::Hcp;
    agreements.decision.stayman_net_force = true;
    agreements.decision.transfer_gf_majors = false;
    agreements.decision.transfer_gf_hearts = false;

    let ns = american(&agreements);
    let ew = american(&agreements);
    let table = Table::of_systems(&ns, &ew, Seat::North, AbsoluteVulnerability::NONE);
    let deals: Vec<FullDeal> = (0..24)
        .map(|seed| contract_bridge::deck::full_deal(&mut rand::rngs::StdRng::seed_from_u64(seed)))
        .collect();

    let first_thread: Vec<String> = deals
        .iter()
        .map(|deal| table.bid_out(deal).to_string())
        .collect();
    let second_thread: Vec<String> = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                deals
                    .iter()
                    .map(|deal| table.bid_out(deal).to_string())
                    .collect()
            })
            .join()
            .expect("second bidding thread panicked")
    });
    assert_eq!(
        first_thread, second_thread,
        "a classify-time knob read escaped the pin"
    );
}
