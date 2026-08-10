use super::*;
use crate::bidding::constraint::{announced, balanced, hcp, len, pred, support};
use contract_bridge::auction::RelativeVulnerability;
use contract_bridge::{Bid, Strain, Suit};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

fn opening_rules() -> Rules {
    Rules::new()
        .rule(Bid::new(1, Strain::Notrump), 100, hcp(15..=17) & balanced())
        .rule(
            Bid::new(1, Strain::Spades),
            100,
            hcp(11..=21) & len(Suit::Spades, 5..),
        )
        .rule(Call::Pass, 0, hcp(..11))
}

fn best_call(logits: &Logits) -> Call {
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

fn assert_logits_bitwise_eq(left: &Logits, right: &Logits) {
    for (call, value) in &left.0 {
        assert_eq!(
            value.to_bits(),
            right.0.get(call).to_bits(),
            "different logit for {call}"
        );
    }
}

fn assert_explanations_bitwise_eq(left: &Map<(usize, f32)>, right: &Map<(usize, f32)>) {
    assert_eq!(
        left.keys().collect::<Vec<_>>(),
        right.keys().collect::<Vec<_>>()
    );
    for (call, &(index, value)) in left {
        let &(other_index, other_value) =
            right.get(call).expect("both explanations contain the call");
        assert_eq!(index, other_index, "different authored rule for {call}");
        assert_eq!(
            value.to_bits(),
            other_value.to_bits(),
            "different explained logit for {call}"
        );
    }
}

#[test]
fn test_classification() {
    let rules = opening_rules();
    let context = Context::new(RelativeVulnerability::NONE, &[]);

    let notrump = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
    assert_eq!(
        best_call(&rules.classify(notrump, &context)),
        Call::Bid(Bid::new(1, Strain::Notrump)),
    );

    let spades = "AKQ32.K532.QJ4.9".parse().expect("valid hand");
    assert_eq!(
        best_call(&rules.classify(spades, &context)),
        Call::Bid(Bid::new(1, Strain::Spades)),
    );

    let weak = "98432.K53.QJ4.92".parse().expect("valid hand");
    assert_eq!(best_call(&rules.classify(weak, &context)), Call::Pass);
}

#[test]
fn test_note_labels_last_rule_and_downcasts() {
    let rules = Rules::new()
        .rule(Bid::new(1, Strain::Notrump), 100, hcp(15..=17) & balanced())
        .note("15-17 BAL")
        .rule(Call::Pass, 0, hcp(..11));

    // note() labels the immediately preceding rule; the unlabeled one is "".
    assert_eq!(rules.rules()[0].label(), "15-17 BAL");
    assert_eq!(rules.rules()[1].label(), "");

    // The corpus hook recovers the authored rules through a type-erased ref.
    let erased: &dyn Classifier = &rules;
    let recovered = erased.as_rules().expect("Rules downcasts to itself");
    assert_eq!(recovered.rules().len(), 2);
    assert_eq!(recovered.rules()[0].label(), "15-17 BAL");
}

#[test]
fn test_alert_marks_block_and_gated_filters() {
    const PUPPET: Alert = Alert("puppet");
    const EUROPEAN: Alert = Alert("european");

    // Shared (unalerted) rule, then one alerted block per variant chained in.
    let rules = Rules::new()
        .rule(Call::Pass, 0, hcp(..8))
        .chain(
            Rules::new()
                .rule(Bid::new(3, Strain::Clubs), 100, hcp(9..))
                .alert(PUPPET),
        )
        .chain(
            Rules::new()
                .rule(Bid::new(3, Strain::Clubs), 100, hcp(9..))
                .alert(EUROPEAN),
        );

    assert_eq!(rules.rules()[0].alert(), None);
    assert_eq!(rules.rules()[1].alert(), Some(PUPPET));
    assert_eq!(rules.rules()[2].alert(), Some(EUROPEAN));

    // Gating to Puppet keeps the unalerted rule and the Puppet block only.
    let puppet = rules.clone().gated(|alert| alert == PUPPET);
    assert_eq!(puppet.rules().len(), 2);
    assert_eq!(puppet.rules()[0].alert(), None);
    assert_eq!(puppet.rules()[1].alert(), Some(PUPPET));

    // Gating to European keeps the unalerted rule and the European block.
    let european = rules.gated(|alert| alert == EUROPEAN);
    assert_eq!(european.rules().len(), 2);
    assert_eq!(european.rules()[1].alert(), Some(EUROPEAN));
}

#[test]
fn test_face_gate() {
    let rules = Rules::new()
        .rule(Bid::new(1, Strain::Notrump), 100, hcp(15..=17) & balanced())
        .face(|context| !context.auction().is_empty());
    let rule = &rules.rules()[0];
    let hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");

    // Gate false (opening seat): as-if-absent.
    let opening = Context::new(RelativeVulnerability::NONE, &[]);
    assert!(!rule.face_live(&opening));
    assert_eq!(rule.eval(hand, &opening), f32::NEG_INFINITY);

    // Gate true: normal evaluation.  Ungated rules default to live.
    let later = Context::new(RelativeVulnerability::NONE, &[Call::Pass]);
    assert!(rule.face_live(&later));
    assert!(rule.eval(hand, &later) > f32::NEG_INFINITY);
    assert!(opening_rules().rules()[0].face_live(&opening));
}

#[test]
fn test_explain() {
    let rules = opening_rules();
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let hand = "AKQ32.K532.QJ4.9".parse().expect("valid hand");
    let explanation = rules.explain(hand, &context);

    let spades = Call::Bid(Bid::new(1, Strain::Spades));
    assert_eq!(explanation.get(spades), Some(&(1, 1.0)));
    assert_eq!(explanation.get(Call::Pass), None);
    assert_eq!(explanation.get(Call::Double), None);
}

#[test]
fn compiled_classify_and_explain_are_bit_exact_and_keep_first_tie() {
    let one_spade = Call::Bid(Bid::new(1, Strain::Spades));
    let one_notrump = Call::Bid(Bid::new(1, Strain::Notrump));
    let rules = Rules::new()
        .rule(one_spade, 75, |_hand: Hand, _context: &Context<'_>| 0.25)
        // Equal same-call result: strict `>` explanation keeps index 0.
        .rule(one_spade, 100, |_hand: Hand, _context: &Context<'_>| 0.0)
        .rule(one_notrump, 0, hcp(15..=17) & balanced())
        .rule(Call::Double, 300, pred(|_, _| false))
        .rule(Call::Redouble, 900, pred(|_, _| true))
        .face(|context| !context.auction().is_empty());
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
    let compiled = rules.compile(&context);

    let legacy_logits = rules.classify(hand, &context);
    let compiled_logits = compiled.classify(&rules, hand, &context);
    assert_logits_bitwise_eq(&legacy_logits, &compiled_logits);

    let legacy_explanation = rules.explain(hand, &context);
    let compiled_explanation = compiled.explain(&rules, hand, &context);
    assert_explanations_bitwise_eq(&legacy_explanation, &compiled_explanation);
    assert_eq!(compiled_explanation.get(one_spade), Some(&(0, 1.0)));
    assert_eq!(compiled_explanation.get(Call::Redouble), None);
}

#[test]
fn explicit_shared_faces_are_lazy_ordered_and_once_per_decision() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let evaluations = Arc::new(AtomicUsize::new(0));
    let face_id = FaceId::new("test:shared-face", 0);

    let first_face_events = Arc::clone(&events);
    let first_face_evaluations = Arc::clone(&evaluations);
    let first_constraint_events = Arc::clone(&events);
    let second_face_events = Arc::clone(&events);
    let second_face_evaluations = Arc::clone(&evaluations);
    let second_constraint_events = Arc::clone(&events);
    let rules = Rules::new()
        .rule(
            Bid::new(1, Strain::Clubs),
            100,
            move |_hand: Hand, _context: &Context<'_>| {
                first_constraint_events.lock().unwrap().push("first");
                0.0
            },
        )
        .shared_face(face_id, move |_| {
            first_face_evaluations.fetch_add(1, Ordering::Relaxed);
            first_face_events.lock().unwrap().push("face");
            true
        })
        .rule(
            Bid::new(1, Strain::Diamonds),
            100,
            move |_hand: Hand, _context: &Context<'_>| {
                second_constraint_events.lock().unwrap().push("second");
                0.0
            },
        )
        .shared_face(face_id, move |_| {
            second_face_evaluations.fetch_add(1, Ordering::Relaxed);
            second_face_events.lock().unwrap().push("face");
            true
        });

    let hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
    let mut projections = ProjectionCache::default();
    let mut faces = FaceRegistry::default();
    let bare = Context::new(RelativeVulnerability::NONE, &[]);
    let compiled = CompiledRules::compile_with_cache(&rules, &bare, &mut projections, &mut faces);
    assert_eq!(faces.len(), 1);

    // The reader's at-the-time contexts deliberately carry no final
    // decision cache. Its effect-scoped memo still evaluates one shared
    // recognizer only once across projection/alert/announcement walks.
    let mut effect_faces = FaceMemo::new();
    assert!(compiled.face_live_memoized(&rules, 0, &bare, &mut effect_faces));
    assert!(compiled.face_live_memoized(&rules, 1, &bare, &mut effect_faces));
    assert!(compiled.face_live_memoized(&rules, 0, &bare, &mut effect_faces));
    assert_eq!(evaluations.load(Ordering::Relaxed), 1);
    evaluations.store(0, Ordering::Relaxed);
    events.lock().unwrap().clear();

    let context = bare.with_compiled_decision_cache(hand, faces.len());

    let _ = compiled.classify(&rules, hand, &context);
    assert_eq!(*events.lock().unwrap(), ["face", "first", "second"]);
    assert_eq!(evaluations.load(Ordering::Relaxed), 1);

    // Explanation is a second executor walk in the same immutable
    // decision. It reuses the face bit while retaining constraint order.
    let _ = compiled.explain(&rules, hand, &context);
    assert_eq!(
        *events.lock().unwrap(),
        ["face", "first", "second", "first", "second"]
    );
    assert_eq!(evaluations.load(Ordering::Relaxed), 1);
}

#[test]
fn opaque_public_faces_keep_every_consult() {
    let evaluations = Arc::new(AtomicUsize::new(0));
    let first = Arc::clone(&evaluations);
    let second = Arc::clone(&evaluations);
    let rules = Rules::new()
        .rule(Bid::new(1, Strain::Clubs), 100, pred(|_, _| true))
        .face(move |_| {
            first.fetch_add(1, Ordering::Relaxed);
            true
        })
        .rule(Bid::new(1, Strain::Diamonds), 100, pred(|_, _| true))
        .face(move |_| {
            second.fetch_add(1, Ordering::Relaxed);
            true
        });
    let hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let compiled = rules.compile(&context);

    let _ = compiled.classify(&rules, hand, &context);
    let _ = compiled.explain(&rules, hand, &context);
    assert_eq!(evaluations.load(Ordering::Relaxed), 4);
}

#[test]
fn compiled_groups_alerts_and_pass_exclusion_keep_authored_indices() {
    const ARTIFICIAL: Alert = Alert("test artificial");
    let one_club = Call::Bid(Bid::new(1, Strain::Clubs));
    let one_diamond = Call::Bid(Bid::new(1, Strain::Diamonds));
    let rules = Rules::new()
        .rule(Call::Pass, 100, pred(|_, _| true))
        .face(|_| false)
        .rule(one_club, 200, pred(|_, _| true))
        .rule(Call::Pass, 200, pred(|_, _| true))
        .rule(one_club, 300, pred(|_, _| true))
        .alert(ARTIFICIAL)
        .rule(one_diamond, 400, pred(|_, _| true))
        .rule(one_club, 200, pred(|_, _| true));
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let compiled = rules.compile(&context);

    assert_eq!(compiled.rule_indices(Call::Pass), [0, 2]);
    assert_eq!(compiled.rule_indices(one_club), [1, 3, 5]);
    assert_eq!(compiled.alerted_rule_indices(one_club), [3]);
    assert!(compiled.alerted_rule_indices(one_diamond).is_empty());
    assert_eq!(
        compiled.call_plan(one_club).map(CompiledCallPlan::call),
        Some(one_club)
    );
    assert_eq!(
        compiled
            .call_plan(one_club)
            .map(CompiledCallPlan::max_weight),
        Some(300)
    );

    let pass = compiled.pass_plan().expect("Pass was authored");
    // Face-dead Pass rule 0 remains in the reading plan by design.
    assert_eq!(compiled.pass_rule_indices(), [0, 2]);
    assert_eq!(pass.max_weight(), 200);
    assert_eq!(pass.stronger_nonpass_indices(), [3, 4]);
}

#[test]
fn projection_folds_compile_independently_and_profile_mismatch_falls_back() {
    let one_club = Bid::new(1, Strain::Clubs);
    let rules = Rules::new()
        .rule(one_club, 0, len(Suit::Spades, 4..=5))
        .rule(one_club, 0, support(3..))
        .rule(
            one_club,
            0,
            announced(pred(|_, _| true), len(Suit::Hearts, 5..)),
        )
        .alert(Alert("test announcement"));
    let auction = [Call::Bid(Bid::new(1, Strain::Hearts)), Call::Pass];
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.envelope_union = false;
    agreements.decision.reading.pass_exclusion = true;
    agreements.decision.reading.announced = true;
    let context =
        Context::new(RelativeVulnerability::NONE, &auction).with_profile(agreements.decision);
    let compiled = rules.compile(&context);
    assert!(compiled.projection_is_constant(0, ProjectionKind::Forward));
    assert!(compiled.projection_is_constant(0, ProjectionKind::Complement));
    assert!(!compiled.projection_is_constant(1, ProjectionKind::Forward));
    // A closure's default projection is the pure, vacuous fold.  The
    // announcement wrapper can therefore freeze it independently from
    // the concrete disclosure constraint below.
    assert!(compiled.projection_is_constant(2, ProjectionKind::Forward));
    assert!(compiled.projection_is_constant(2, ProjectionKind::Announcement));
    assert!(
        compiled
            .projection_dependencies(1, ProjectionKind::Forward)
            .intersects(ConstraintDependencies::CONTEXT)
    );

    assert_eq!(
        compiled.project_union(&rules, 0, &context),
        rules.rules()[0].project_union(&context)
    );
    assert_eq!(
        compiled.project_band_union(&rules, 0, &context),
        rules.rules()[0].project_band_union(&context)
    );
    assert_eq!(
        compiled.announce_union(&rules, 2, &context),
        rules.rules()[2].announce_union(&context)
    );

    // The complement was frozen knob-off, but a changed profile must use
    // the live virtual fold and retain both knob-on halves.
    agreements.decision.reading.envelope_union = true;
    let changed =
        Context::new(RelativeVulnerability::NONE, &auction).with_profile(agreements.decision);
    let expected = rules.rules()[0].project_complement_union(&changed);
    assert_eq!(expected.boxes().len(), 2);
    assert_eq!(
        compiled.project_complement_union(&rules, 0, &changed),
        expected
    );
}
