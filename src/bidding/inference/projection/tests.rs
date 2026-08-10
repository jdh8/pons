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

    set_reading_scope(ReadingScope::Alerted);
    set_pass_exclusion_reading(false);
    set_announced_reading(false);

    let one_club = bid(1, Strain::Clubs);
    let context = Context::new(RelativeVulnerability::NONE, &[]);

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
    assert_eq!(legacy_projection_events, ["project"]);

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

    let pure_nonpass = Rules::new().rule(one_club, 0, hcp(0..));
    assert!(
        pure_nonpass
            .compile(&context)
            .can_skip_nonpass_effect(one_club)
    );
    let pure_pass = Rules::new().rule(Call::Pass, 0, hcp(0..));
    assert!(pure_pass.compile(&context).can_skip_pass_effect(false));
}

#[test]
fn deal_cache_rejects_observable_faces_and_projections_before_hooks_run() {
    use crate::bidding::book::Pair;
    use crate::bidding::rules::{Alert, Rules};
    use crate::bidding::trie::Classifier;

    set_reading_scope(ReadingScope::Alerted);
    set_pass_reading(false);
    set_pass_exclusion_reading(false);
    set_table_alert_reading(false);
    set_announced_reading(false);
    set_probed_reading(false);

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
    let mut pair = Pair {
        agreements: Agreements::current(),
        ..Default::default()
    };
    pair.constructive.insert_arc(&[], classifier);
    let auction = [one_club, Call::Pass];

    for fallback_projection in [true, false] {
        set_fallback_projection(fallback_projection);
        let stance = pair.against();
        let mut cache = AuthoringStepCache::new();

        assert!(
            cache
                .prepare(&stance, RelativeVulnerability::NONE, &auction)
                .is_none(),
            "observable route was cached with fallback projection {fallback_projection}",
        );
        assert!(events.lock().unwrap().is_empty());
        assert!(
            cache
                .prepare(&stance, RelativeVulnerability::NONE, &auction)
                .is_none(),
            "a disabled cache became live again",
        );
        assert!(events.lock().unwrap().is_empty());

        let context = stance.prefixed_context(RelativeVulnerability::NONE, &auction);
        let expected = project_authored_legacy(&context);
        let expected_events = core::mem::take(&mut *events.lock().unwrap());
        assert_eq!(expected_events, ["face", "project", "face"]);

        for _ in 0..2 {
            let actual = project_authored(&context);
            let actual_events = core::mem::take(&mut *events.lock().unwrap());
            assert_eq!(actual, expected);
            assert_eq!(actual_events, expected_events);
        }
    }

    set_fallback_projection(true);
    set_pass_reading(true);
    set_table_alert_reading(true);
}

#[test]
fn opaque_routes_keep_legacy_invocation_order_and_disable_step_cache() {
    use crate::bidding::book::Pair;
    use crate::bidding::fallback::{Fallback, guard};
    use crate::bidding::rules::{Alert, Rules};
    use crate::bidding::trie::Classifier;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    set_reading_scope(ReadingScope::Alerted);
    set_fallback_projection(true);
    set_pass_reading(true);
    set_table_alert_reading(true);

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

    let mut pair = Pair {
        agreements: Agreements::current(),
        ..Default::default()
    };
    pair.constructive.fallback_at(
        &[],
        make_guard(Arc::clone(&calls)),
        Fallback::Classify(Arc::clone(&classifier)),
    );
    pair.defensive.fallback_at(
        &[],
        make_guard(Arc::clone(&calls)),
        Fallback::Classify(classifier),
    );
    let stance = pair.against();

    let auction = [bid(1, Strain::Clubs), bid(1, Strain::Diamonds)];
    let context = stance.prefixed_context(RelativeVulnerability::NONE, &auction);
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
            .prepare(&stance, RelativeVulnerability::NONE, &auction)
            .is_none()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn later_opaque_route_does_not_speculatively_consult_an_earlier_face() {
    use crate::bidding::book::Pair;
    use crate::bidding::fallback::{Fallback, guard};
    use crate::bidding::rules::{Alert, Rules};
    use crate::bidding::trie::Classifier;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    set_reading_scope(ReadingScope::Alerted);
    set_fallback_projection(true);
    set_pass_reading(false);
    set_table_alert_reading(false);
    set_announced_reading(false);

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

    let mut pair = Pair {
        agreements: Agreements::current(),
        ..Default::default()
    };
    pair.constructive.insert_arc(&[], root);
    pair.competitive.fallback_at(
        &[],
        guard(move |_: &Context<'_>, _: &[Call]| {
            observed_guard.fetch_add(1, Ordering::SeqCst);
            false
        }),
        Fallback::Classify(opaque_target),
    );
    let stance = pair.against();
    // The same side's deal cache sees both calls in one append: the root
    // exact classifier authors 1♣, then the next prefix reaches the opaque
    // competitive fallback.
    let auction = [bid(1, Strain::Clubs), bid(1, Strain::Diamonds)];
    let context = stance.prefixed_context(RelativeVulnerability::NONE, &auction);

    let expected = project_authored_legacy(&context);
    let expected_face_calls = face_calls.swap(0, Ordering::SeqCst);
    let expected_guard_calls = guard_calls.swap(0, Ordering::SeqCst);
    assert!(expected_face_calls > 0);
    assert!(expected_guard_calls > 0);

    let mut cache = AuthoringStepCache::new();
    assert!(
        cache
            .prepare(&stance, RelativeVulnerability::NONE, &auction)
            .is_none()
    );
    assert_eq!(face_calls.load(Ordering::SeqCst), 0);
    assert_eq!(guard_calls.load(Ordering::SeqCst), 0);

    let actual = project_authored(&context);
    assert_eq!(actual, expected);
    assert_eq!(face_calls.load(Ordering::SeqCst), expected_face_calls);
    assert_eq!(guard_calls.load(Ordering::SeqCst), expected_guard_calls);

    set_pass_reading(true);
    set_table_alert_reading(true);
}

#[test]
fn opaque_route_on_unused_routed_prefix_is_never_invoked() {
    use crate::bidding::book::Pair;
    use crate::bidding::fallback::{Fallback, guard};
    use crate::bidding::rules::Rules;
    use crate::bidding::trie::Classifier;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    set_reading_scope(ReadingScope::Alerted);
    set_fallback_projection(false);
    set_pass_reading(true);
    set_table_alert_reading(true);

    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let classifier: Arc<dyn Classifier> =
        Arc::new(Rules::new().rule(Call::Pass, 0, crate::bidding::constraint::hcp(0..)));
    let mut pair = Pair {
        agreements: Agreements::current(),
        ..Default::default()
    };
    pair.constructive.fallback_at(
        &[],
        guard(move |_: &Context<'_>, _: &[Call]| {
            observed.fetch_add(1, Ordering::SeqCst);
            true
        }),
        Fallback::Classify(classifier),
    );
    let stance = pair.against();
    let auction = [
        bid(1, Strain::Clubs),
        bid(1, Strain::Spades),
        Call::Double,
        Call::Pass,
    ];
    let context = stance.prefixed_context(RelativeVulnerability::NONE, &auction);
    let compiled_entry = project_authored(&context);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let legacy = project_authored_legacy(&context);
    assert_eq!(compiled_entry, legacy);
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let mut cache = AuthoringStepCache::new();
    assert!(
        cache
            .prepare(&stance, RelativeVulnerability::NONE, &auction)
            .is_some()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    set_fallback_projection(true);
}
