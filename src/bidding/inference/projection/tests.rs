use super::*;
use crate::bidding::agreements::Agreements;
use crate::bidding::constraint::Constraint;
use crate::bidding::context::Context;
use crate::bidding::inference::EnvelopeUnion;
use crate::bidding::inference::tests::bid;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Hand, Strain, Suit};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct ObservableProjection {
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl Constraint for ObservableProjection {
    fn eval(&self, _: Hand, _: &Context<'_>) -> f32 {
        0.0
    }

    fn project(&self, _: &Context<'_>) -> EnvelopeUnion {
        self.events.lock().unwrap().push("project");
        EnvelopeUnion::unknown()
    }
}

#[test]
fn unread_compiled_effects_preserve_opaque_face_and_projection_hooks() {
    use crate::bidding::constraint::hcp;
    use crate::bidding::rules::Rules;

    let one_club = bid(1, Strain::Clubs);
    let mut agreements = Agreements::default();
    agreements.decision.reading.scope = ReadingScope::Alerted;
    agreements.decision.reading.pass_exclusion = false;
    agreements.decision.reading.announced = false;
    let context = Context::new(RelativeVulnerability::NONE, &[]).with_profile(agreements.decision);

    let face_events = Arc::new(Mutex::new(Vec::new()));
    let observed_face = Arc::clone(&face_events);
    let face_rules = Rules::new().rule(one_club, 0, hcp(0..)).face(move |_| {
        observed_face.lock().unwrap().push("face");
        true
    });
    let face_compiled = face_rules.compile(&context);
    assert!(authored_effect(one_club, &context, &face_rules, None, false, false).is_none());
    let legacy_face_events = face_events.lock().unwrap().clone();
    face_events.lock().unwrap().clear();
    assert!(
        authored_effect(
            one_club,
            &context,
            &face_rules,
            Some(&face_compiled),
            false,
            false,
        )
        .is_none()
    );
    assert_eq!(*face_events.lock().unwrap(), legacy_face_events);
    assert_eq!(legacy_face_events, ["face"]);

    let projection_events = Arc::new(Mutex::new(Vec::new()));
    let projection_rules = Rules::new().rule(
        one_club,
        0,
        ObservableProjection {
            events: Arc::clone(&projection_events),
        },
    );
    let projection_compiled = projection_rules.compile(&context);
    assert!(authored_effect(one_club, &context, &projection_rules, None, false, false,).is_none());
    let legacy_projection_events = projection_events.lock().unwrap().clone();
    projection_events.lock().unwrap().clear();
    assert!(
        authored_effect(
            one_club,
            &context,
            &projection_rules,
            Some(&projection_compiled),
            false,
            false,
        )
        .is_none()
    );
    assert_eq!(*projection_events.lock().unwrap(), legacy_projection_events);
    // Under the shipped `bid_exclusion` an undecoded call is never projected —
    // the fold defers every projection past the decode gate, in both paths.
    assert_eq!(legacy_projection_events, [] as [&str; 0]);

    let pass_events = Arc::new(Mutex::new(Vec::new()));
    let pass_rules = Rules::new().rule(
        Call::Pass,
        0,
        ObservableProjection {
            events: Arc::clone(&pass_events),
        },
    );
    let pass_compiled = pass_rules.compile(&context);
    assert!(authored_effect(Call::Pass, &context, &pass_rules, None, false, false).is_none());
    let legacy_pass_events = pass_events.lock().unwrap().clone();
    pass_events.lock().unwrap().clear();
    assert!(
        authored_effect(
            Call::Pass,
            &context,
            &pass_rules,
            Some(&pass_compiled),
            false,
            false,
        )
        .is_none()
    );
    assert_eq!(*pass_events.lock().unwrap(), legacy_pass_events);
    assert_eq!(legacy_pass_events, ["project"]);

    let profile = context.reading_profile();
    let pure_nonpass = Rules::new().rule(one_club, 0, hcp(0..));
    assert!(
        pure_nonpass
            .compile(&context)
            .can_skip_nonpass_effect(one_club, profile)
    );
    let pure_pass = Rules::new().rule(Call::Pass, 0, hcp(0..));
    assert!(pure_pass.compile(&context).can_skip_pass_effect(profile));
}

#[test]
fn deal_cache_rejects_observable_faces_and_projections_before_hooks_run() {
    use crate::bidding::book::System;
    use crate::bidding::rules::{Alert, Rules};
    use crate::bidding::trie::Classifier;

    let events = Arc::new(Mutex::new(Vec::new()));
    let face_events = Arc::clone(&events);
    let one_club = bid(1, Strain::Clubs);
    let classifier: Arc<dyn Classifier> = Arc::new(
        Rules::new()
            .rule(
                one_club,
                0,
                ObservableProjection {
                    events: Arc::clone(&events),
                },
            )
            .alert(Alert("deal-cache-observable-test"))
            .face(move |_| {
                face_events.lock().unwrap().push("face");
                true
            }),
    );
    let mut system = System {
        agreements: Agreements::default(),
        ..Default::default()
    };
    system.agreements.decision.reading.scope = ReadingScope::Alerted;
    system.agreements.decision.reading.pass = false;
    system.agreements.decision.reading.pass_exclusion = false;
    system.agreements.decision.reading.table_alerts = false;
    system.agreements.decision.reading.announced = false;
    system.agreements.decision.reading.probed = false;
    system.constructive.insert_arc(&[], classifier);
    let auction = [one_club, Call::Pass];

    for fallback_projection in [true, false] {
        system.agreements.decision.reading.fallback_projection = fallback_projection;
        let partnership = system.bind();
        let mut cache = AuthoringStepCache::new();

        assert!(
            cache
                .prepare(&partnership, RelativeVulnerability::NONE, &auction)
                .is_none(),
            "observable route was cached with fallback projection {fallback_projection}",
        );
        assert!(events.lock().unwrap().is_empty());
        assert!(
            cache
                .prepare(&partnership, RelativeVulnerability::NONE, &auction)
                .is_none(),
            "a disabled cache became live again",
        );
        assert!(events.lock().unwrap().is_empty());

        let context = partnership.prefixed_context(RelativeVulnerability::NONE, &auction);
        let expected = project_authored_legacy(&context);
        let expected_events = core::mem::take(&mut *events.lock().unwrap());
        // Shipped `bid_exclusion` order: the fold consults the face for the
        // live-rule list and again for the alert gate, then projects.
        assert_eq!(expected_events, ["face", "face", "project"]);

        for _ in 0..2 {
            let actual = project_authored(&context);
            let actual_events = core::mem::take(&mut *events.lock().unwrap());
            assert_eq!(actual, expected);
            assert_eq!(actual_events, expected_events);
        }
    }
}

#[test]
fn opaque_routes_keep_legacy_invocation_order_and_disable_step_cache() {
    use crate::bidding::book::System;
    use crate::bidding::fallback::{Fallback, guard};
    use crate::bidding::rules::{Alert, Rules};
    use crate::bidding::trie::Classifier;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = Arc::new(AtomicUsize::new(0));
    let classifier: Arc<dyn Classifier> = Arc::new(
        Rules::new()
            .rule(
                bid(1, Strain::Diamonds),
                0,
                crate::bidding::constraint::len(Suit::Hearts, 5..),
            )
            .alert(Alert("stateful opaque test")),
    );
    let make_guard = |calls: Arc<AtomicUsize>| {
        guard(move |_: &Context<'_>, _: &[Call]| {
            calls.fetch_add(1, Ordering::SeqCst).is_multiple_of(2)
        })
    };

    let mut system = System {
        agreements: Agreements::default(),
        ..Default::default()
    };
    system.agreements.decision.reading.scope = ReadingScope::Alerted;
    system.agreements.decision.reading.fallback_projection = true;
    system.agreements.decision.reading.pass = true;
    system.agreements.decision.reading.table_alerts = true;
    system.constructive.fallback_at(
        &[],
        make_guard(Arc::clone(&calls)),
        Fallback::Classify(Arc::clone(&classifier)),
    );
    system.defensive.fallback_at(
        &[],
        make_guard(Arc::clone(&calls)),
        Fallback::Classify(classifier),
    );
    let partnership = system.bind();

    let auction = [bid(1, Strain::Clubs), bid(1, Strain::Diamonds)];
    let context = partnership.prefixed_context(RelativeVulnerability::NONE, &auction);
    calls.store(0, Ordering::SeqCst);
    let compiled_entry = project_authored(&context);
    let compiled_calls = calls.load(Ordering::SeqCst);
    calls.store(0, Ordering::SeqCst);
    let legacy = project_authored_legacy(&context);
    let legacy_calls = calls.load(Ordering::SeqCst);
    assert_eq!(compiled_entry, legacy);
    assert_eq!(compiled_calls, legacy_calls);
    assert!(legacy_calls > 0);

    calls.store(0, Ordering::SeqCst);
    let mut cache = AuthoringStepCache::new();
    assert!(
        cache
            .prepare(&partnership, RelativeVulnerability::NONE, &auction)
            .is_none()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn later_opaque_route_does_not_speculatively_consult_an_earlier_face() {
    use crate::bidding::book::System;
    use crate::bidding::fallback::{Fallback, guard};
    use crate::bidding::rules::{Alert, Rules};
    use crate::bidding::trie::Classifier;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let face_calls = Arc::new(AtomicUsize::new(0));
    let observed_face = Arc::clone(&face_calls);
    let root: Arc<dyn Classifier> = Arc::new(
        Rules::new()
            .rule(
                bid(1, Strain::Clubs),
                0,
                crate::bidding::constraint::hcp(0..),
            )
            .alert(Alert("transactional opaque-route test"))
            // Public faces are deliberately observable on every consult.
            .face(move |_| {
                observed_face.fetch_add(1, Ordering::SeqCst);
                true
            }),
    );
    let guard_calls = Arc::new(AtomicUsize::new(0));
    let observed_guard = Arc::clone(&guard_calls);
    let opaque_target: Arc<dyn Classifier> = Arc::new(Rules::new());

    let mut system = System {
        agreements: Agreements::default(),
        ..Default::default()
    };
    system.agreements.decision.reading.scope = ReadingScope::Alerted;
    system.agreements.decision.reading.fallback_projection = true;
    system.agreements.decision.reading.pass = false;
    system.agreements.decision.reading.table_alerts = false;
    system.agreements.decision.reading.announced = false;
    system.constructive.insert_arc(&[], root);
    system.competitive.fallback_at(
        &[],
        guard(move |_: &Context<'_>, _: &[Call]| {
            observed_guard.fetch_add(1, Ordering::SeqCst);
            false
        }),
        Fallback::Classify(opaque_target),
    );
    let partnership = system.bind();
    // The same side's deal cache sees both calls in one append: the root
    // exact classifier authors 1♣, then the next prefix reaches the opaque
    // competitive fallback.
    let auction = [bid(1, Strain::Clubs), bid(1, Strain::Diamonds)];
    let context = partnership.prefixed_context(RelativeVulnerability::NONE, &auction);

    let expected = project_authored_legacy(&context);
    let expected_face_calls = face_calls.swap(0, Ordering::SeqCst);
    let expected_guard_calls = guard_calls.swap(0, Ordering::SeqCst);
    assert!(expected_face_calls > 0);
    assert!(expected_guard_calls > 0);

    let mut cache = AuthoringStepCache::new();
    assert!(
        cache
            .prepare(&partnership, RelativeVulnerability::NONE, &auction)
            .is_none()
    );
    assert_eq!(face_calls.load(Ordering::SeqCst), 0);
    assert_eq!(guard_calls.load(Ordering::SeqCst), 0);

    let actual = project_authored(&context);
    assert_eq!(actual, expected);
    assert_eq!(face_calls.load(Ordering::SeqCst), expected_face_calls);
    assert_eq!(guard_calls.load(Ordering::SeqCst), expected_guard_calls);
}

#[test]
fn opaque_route_on_unused_routed_prefix_is_never_invoked() {
    use crate::bidding::book::System;
    use crate::bidding::fallback::{Fallback, guard};
    use crate::bidding::rules::Rules;
    use crate::bidding::trie::Classifier;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let classifier: Arc<dyn Classifier> =
        Arc::new(Rules::new().rule(Call::Pass, 0, crate::bidding::constraint::hcp(0..)));
    let mut system = System {
        agreements: Agreements::default(),
        ..Default::default()
    };
    system.agreements.decision.reading.scope = ReadingScope::Alerted;
    system.agreements.decision.reading.fallback_projection = false;
    system.agreements.decision.reading.pass = true;
    system.agreements.decision.reading.table_alerts = true;
    system.constructive.fallback_at(
        &[],
        guard(move |_: &Context<'_>, _: &[Call]| {
            observed.fetch_add(1, Ordering::SeqCst);
            true
        }),
        Fallback::Classify(classifier),
    );
    let partnership = system.bind();
    let auction = [
        bid(1, Strain::Clubs),
        bid(1, Strain::Spades),
        Call::Double,
        Call::Pass,
    ];
    let context = partnership.prefixed_context(RelativeVulnerability::NONE, &auction);
    let compiled_entry = project_authored(&context);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let legacy = project_authored_legacy(&context);
    assert_eq!(compiled_entry, legacy);
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let mut cache = AuthoringStepCache::new();
    assert!(
        cache
            .prepare(&partnership, RelativeVulnerability::NONE, &auction)
            .is_some()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

/// Bid-exclusion soundness: wherever a table's argmax is (or ties with) a
/// non-Pass call, the knob-on reading of that call must admit the hand.
///
/// The made-bid twin of `passes_read_within_their_table`.  Both replay the
/// argmax rather than one rule, because the claim is about the **table** — "no
/// bidder of `C` holds a hand a strictly-heavier sibling gate accepts".  This
/// one also filters by legality, exactly as `table::select_with_legal_state`
/// does: an insufficient bid never reached the argmax, so its gate proves
/// nothing.  Ties count as wins (stricter than the driver, which keeps the
/// earliest), which is why the exclusion threshold is a strict `>` on weight.
#[test]
fn bids_read_within_their_table() {
    use crate::bidding::american::american;
    use crate::bidding::dutch::dutch;
    use rand::SeedableRng as _;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0x81D5);
    let mut hands: Vec<Hand> = crate::bidding::verify::random_hands(&mut rng)
        .take(128)
        .collect();
    hands.extend(
        ["AKQJ.AKQJ.AKQ.AK", "AKQ2.K53.QJ4.T92"]
            .map(|text| text.parse::<Hand>().unwrap_or_else(|_| unreachable!())),
    );

    let mut failures: Vec<String> = Vec::new();
    let check = |system: &str,
                 auction: &[Call],
                 context: &Context<'_>,
                 rules: &crate::bidding::rules::Rules,
                 failures: &mut Vec<String>| {
        // The serving path: a compiled plan interns each rule's complement, so
        // the per-rule sibling scan reads the pool instead of re-projecting.
        // The uncompiled twin is pinned by
        // `bid_exclusion_folds_only_live_legal_stronger_siblings`.
        let compiled = rules.compile(context);
        // A reading is hand-independent, so decode each call once per node and
        // replay every hand against the memo.
        let mut readings: std::collections::HashMap<Call, Option<EnvelopeUnion>> =
            std::collections::HashMap::new();
        for &hand in &hands {
            let logits = compiled.classify(rules, hand, context);
            let best = logits
                .iter()
                .filter(|(call, logit)| logit.is_finite() && context.allows(*call))
                .fold(f32::NEG_INFINITY, |best, (_, &logit)| best.max(logit));
            if !best.is_finite() {
                continue;
            }
            for (call, &logit) in logits.iter() {
                if call == Call::Pass || logit < best || !context.allows(call) {
                    continue;
                }
                let reading = readings.entry(call).or_insert_with(|| {
                    authored_effect(call, context, rules, Some(&compiled), false, false)
                        .map(|effect| effect.projection.into_owned())
                });
                let Some(reading) = reading else { continue };
                if !reading.boxes().iter().any(|box_| box_.accepts(hand)) && failures.len() < 16 {
                    failures.push(format!(
                        "{system}: [{}] {call} reading excludes the hand that bid it: {hand}",
                        contract_bridge::auction::display_calls(auction),
                    ));
                }
            }
        }
    };

    // The shipped regime only: ceilings on is the *tighter* reading, so it is
    // the binding cell — a hand this sweep admits with ceilings on is admitted
    // without them too.
    let mut agreements = Agreements::default();
    agreements.decision.reading.envelope_union = true;
    agreements.decision.reading.bid_exclusion = true;
    let american = american(&agreements);
    let dutch = dutch(&agreements);
    let tries: [(&str, &crate::bidding::trie::Trie); 4] = [
        ("american constructive", &american.constructive.0),
        ("american competitive", &american.competitive.0),
        ("american defensive", &american.defensive.0),
        ("dutch constructive", &dutch.constructive.0),
    ];
    // `Hand::EMPTY` arms the node's decision scope so one `Inferences::read`
    // serves every rule and every probe hand — the same saving `node_context`
    // buys the sibling sweeps in `inference/tests.rs`.  Without it the deep
    // keycard nodes, whose gates consult the reading, re-derive it per rule
    // per hand: 1.75s for a six-rule table.
    for (system, trie) in tries {
        for (auction, classifier) in trie {
            if let Some(rules) = classifier.as_rules() {
                let context = Context::new(RelativeVulnerability::NONE, &auction)
                    .with_prefixes(trie.common_prefixes(&auction))
                    .with_profile(agreements.decision)
                    .with_decision_cache(Hand::EMPTY);
                check(system, &auction, &context, rules, &mut failures);
            }
        }
        for (auction, _, fallback) in trie.fallbacks() {
            let crate::bidding::fallback::Fallback::Classify(classifier) = fallback else {
                continue;
            };
            if let Some(rules) = classifier.as_rules() {
                let context = Context::new(RelativeVulnerability::NONE, &auction)
                    .with_prefixes(trie.common_prefixes(&auction))
                    .with_profile(agreements.decision)
                    .with_decision_cache(Hand::EMPTY);
                check(system, &auction, &context, rules, &mut failures);
            }
        }
    }
    assert!(
        failures.is_empty(),
        "bid-exclusion excludes hands that bid:\n{}",
        failures.join("\n"),
    );
}

/// The exclusion fold's arithmetic, pinned rule by rule.
///
/// Two of these are the latent quirks the Pass-only path carried and this
/// phase fixed: a **face-dead** sibling never bids, and an **illegal** one was
/// filtered before the argmax ever saw it, so neither may be excluded.
#[test]
fn bid_exclusion_folds_only_live_legal_stronger_siblings() {
    use crate::bidding::constraint::{hcp, len, points, pred};
    use crate::bidding::inference::Range;
    use crate::bidding::rules::Rules;

    let one_heart = bid(1, Strain::Hearts);
    let one_spade = bid(1, Strain::Spades);
    let two_hearts = bid(2, Strain::Hearts);
    let three_clubs = bid(3, Strain::Clubs);
    // Their `1♠` is on the table, so `1♥` is insufficient and `2♥` is not.
    let auction = [one_spade];

    let mut agreements = Agreements::default();
    agreements.decision.reading.envelope_union = true;
    agreements.decision.reading.bid_exclusion = true;
    let context =
        Context::new(RelativeVulnerability::NONE, &auction).with_profile(agreements.decision);

    let read = |rules: &Rules| {
        authored_effect(two_hearts, &context, rules, None, false, false)
            .expect("2♥ is authored")
            .projection
            .into_owned()
    };

    // The catch-all alone reads nothing — today's identity.
    let bare = Rules::new().rule(two_hearts, 50, hcp(0..));
    assert_eq!(read(&bare), EnvelopeUnion::unknown());

    // A strictly-heavier legal sibling excludes its own gate.
    let excluded = bare.clone().rule(three_clubs, 100, points(17..));
    assert_eq!(read(&excluded).hull().strength.points, Range::new(0, 16));

    // ...but a face-dead one never bids,
    let face_dead = bare
        .clone()
        .rule(three_clubs, 100, points(17..))
        .face(|_| false);
    assert_eq!(read(&face_dead), EnvelopeUnion::unknown());

    // ...an insufficient one never reached the argmax,
    let illegal = bare.clone().rule(one_heart, 100, points(17..));
    assert_eq!(read(&illegal), EnvelopeUnion::unknown());

    // ...an equally-weighted one loses the tie to the earlier call,
    let tied = bare.clone().rule(three_clubs, 50, points(17..));
    assert_eq!(read(&tied), EnvelopeUnion::unknown());

    // ...and an opaque gate complements to ⊤, which is dropped before folding.
    let opaque = bare.clone().rule(three_clubs, 100, pred(|_, _| true));
    assert_eq!(read(&opaque), EnvelopeUnion::unknown());

    // The threshold is **per rule**: a second `2♥` rule heavier than the
    // sibling keeps its own arm of the union unexcluded, so the union reopens.
    let per_rule = excluded.clone().rule(two_hearts, 150, hcp(0..));
    assert_eq!(read(&per_rule), EnvelopeUnion::unknown());

    // Many disjunctive complements stay inside the budget without losing a
    // bidder that satisfies none of the declined gates.
    let mut wide = bare.clone().rule(three_clubs, 100, hcp(5..=10));
    for suit in Suit::ASC {
        wide = wide.rule(three_clubs, 100, len(suit, 2..=3));
    }
    let folded = read(&wide);
    assert!(folded.boxes().len() <= 16, "{} boxes", folded.boxes().len());
    let outside: Hand = "AKQJT.AKQJ.AKQJ.".parse().expect("a 5-4-4-0");
    assert!(folded.boxes().iter().any(|box_| box_.accepts(outside)));
}
