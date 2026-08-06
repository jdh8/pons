use super::*;
use crate::bidding::constraint::{Constraint, point_count};
use contract_bridge::auction::RelativeVulnerability;
use contract_bridge::{Bid, Hand, Level};
use proptest::prelude::*;
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

const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

fn read(auction: &[Call]) -> Inferences {
    Inferences::read(&Context::new(RelativeVulnerability::NONE, auction))
}

/// Read on a *prefixed* context, the trie access the projection pass needs to
/// read a convention off its authored rule — what the production search floor
/// hands `Inferences::read` (cf. `Stance::prefixed_context`).  The plain `read`
/// above is keyless, so it sees no convention overlay.
fn read_booked(auction: &[Call]) -> Inferences {
    let stance = crate::american().against();
    Inferences::read(&stance.prefixed_context(RelativeVulnerability::NONE, auction))
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
    let mut pair = Pair::default();
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

    let mut pair = Pair::default();
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

    let mut pair = Pair::default();
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
    let mut pair = Pair::default();
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

/// Pins the skew bound `support_band_to_points` is derived from: at every
/// fit-known trump (three-plus cards), `point_count` lies within the
/// image of the hand's own support count — so a support band's image
/// always contains the legacy count of every hand the band admits.
#[test]
fn support_band_points_image_is_sound() {
    use rand::SeedableRng as _;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0x5B);
    let hands = crate::bidding::verify::random_hands(&mut rng)
        .take(4096)
        // Extremes the random pool cannot deal: two side voids attain
        // the +5 skew; working doubletons alone attain the −1 side.
        .chain(
            ["432.AKQJT98765..", "..432.AKQJT98765", "AQJT9.KQJT.A2.K2"]
                .map(|text| text.parse::<Hand>().unwrap_or_else(|_| unreachable!())),
        );
    for hand in hands {
        for trump in Suit::ASC {
            if hand[trump].len() < 3 {
                continue;
            }
            let support =
                crate::bidding::constraint::support_point_count_in(hand, trump).min(POINTS_CAP);
            let image = support_band_to_points(Range::new(support, support));
            let points = point_count(hand);
            assert!(
                image.contains(points),
                "{hand}: trump {trump}, support {support}, points {points}"
            );
        }
    }
}

/// The `EnvelopeUnion` box algebra: `union` retains alternatives,
/// `intersect` distributes and **drops** the empty
/// products, so a disjunctive reading stays tight instead of hulling to the
/// bounding box.  The worked example is `1NT ∩ 4-5♥` (opener's Stayman `2♥`).
#[derive(Clone)]
struct VecEnvelopeUnion(Vec<Envelope>);

impl VecEnvelopeUnion {
    fn hull(&self) -> Envelope {
        self.0
            .iter()
            .copied()
            .reduce(|a, b| a.span(&b))
            .unwrap_or_else(Envelope::unknown)
    }

    fn union(mut self, mut other: Self) -> Self {
        self.0.append(&mut other.0);
        self
    }

    fn disjoin(self, other: Self) -> Self {
        if envelope_union_reading() {
            self.union(other).tidy()
        } else {
            Self(vec![self.hull().span(&other.hull())])
        }
    }

    fn intersect(&self, other: &Self) -> Self {
        let mut out = Vec::new();
        for a in &self.0 {
            for b in &other.0 {
                if let Some(product) = a.intersect_nonempty(b) {
                    out.push(product);
                }
            }
        }
        if out.is_empty() {
            out.push(self.hull().intersect(&other.hull()));
        }
        Self(out).tidy()
    }

    fn tidy(mut self) -> Self {
        if !envelope_union_reading() {
            return self;
        }
        self.0.retain(Envelope::sum_feasible);
        if sum_closure() || upgrade_closure() {
            for box_ in &mut self.0 {
                if sum_closure() {
                    box_.narrow_to_sum();
                }
                if upgrade_closure() {
                    box_.narrow_to_upgrade();
                }
            }
        }
        let mut kept = Vec::with_capacity(self.0.len());
        'boxes: for (i, a) in self.0.iter().enumerate() {
            for (j, b) in self.0.iter().enumerate() {
                if i != j && a.subset_of(b) && (!b.subset_of(a) || j < i) {
                    continue 'boxes;
                }
            }
            kept.push(*a);
        }
        if kept.is_empty() {
            kept.push(Envelope::unknown());
        }
        Self(kept)
    }
}

#[test]
fn inline_union_matches_the_vec_oracle_in_every_closure_profile() {
    let envelope = |lengths: [(u8, u8); 4], points: (u8, u8)| Envelope {
        lengths: lengths.map(|(min, max)| Range::new(min, max)),
        strength: Strength {
            points: Range::new(points.0, points.1),
            ..Strength::unknown()
        },
    };
    let left = vec![
        envelope([(2, 6), (2, 6), (2, 4), (2, 4)], (15, 17)),
        envelope([(2, 3), (2, 3), (2, 3), (5, 5)], (15, 17)),
        envelope([(0, 1), (0, 1), (0, 1), (0, 1)], (0, 37)),
    ];
    let right = vec![
        envelope([(0, 13), (0, 13), (4, 5), (0, 13)], (0, 37)),
        envelope([(0, 13), (0, 13), (5, 5), (0, 13)], (8, 20)),
    ];

    for union in [false, true] {
        for sum in [false, true] {
            for upgrade in [false, true] {
                set_envelope_union_reading(union);
                set_sum_closure(sum);
                set_upgrade_closure(upgrade);

                let actual_left = EnvelopeUnion::from_boxes(left.clone());
                let actual_right = EnvelopeUnion::from_boxes(right.clone());
                let reference_left = VecEnvelopeUnion(left.clone());
                let reference_right = VecEnvelopeUnion(right.clone());

                assert_eq!(
                    actual_left.clone().tidy().boxes(),
                    reference_left.clone().tidy().0
                );
                assert_eq!(
                    actual_left.clone().union(actual_right.clone()).boxes(),
                    reference_left.clone().union(reference_right.clone()).0
                );
                assert_eq!(
                    actual_left.clone().disjoin(actual_right.clone()).boxes(),
                    reference_left.clone().disjoin(reference_right.clone()).0
                );
                assert_eq!(
                    actual_left.intersect(&actual_right).boxes(),
                    reference_left.intersect(&reference_right).0
                );
            }
        }
    }

    set_envelope_union_reading(true);
    set_sum_closure(false);
    set_upgrade_closure(false);
}

#[test]
fn envelope_union_algebra_preserves_exact_alternatives() {
    // A box literal: [♣, ♦, ♥, ♠] length ranges (ASC order) and points.
    let box_ = |c: (u8, u8), d: (u8, u8), h: (u8, u8), s: (u8, u8), p: (u8, u8)| Envelope {
        lengths: [
            Range::new(c.0, c.1),
            Range::new(d.0, d.1),
            Range::new(h.0, h.1),
            Range::new(s.0, s.1),
        ],
        strength: Strength {
            points: Range::new(p.0, p.1),
            ..Strength::unknown()
        },
    };

    // 1NT as three shapes, all 15-17: balanced, then each 5-card major.
    let one_nt = EnvelopeUnion::from_boxes(vec![
        box_((2, 6), (2, 6), (2, 4), (2, 4), (15, 17)), // balanced
        box_((2, 3), (2, 3), (2, 3), (5, 5), (15, 17)), // 5=♠
        box_((2, 3), (2, 3), (5, 5), (2, 3), (15, 17)), // 5=♥
    ]);
    // Opener's `2♥` over Stayman = 1NT ∩ {4-5 hearts}, other suits free.
    let four_five_hearts = EnvelopeUnion::from(box_((0, 13), (0, 13), (4, 5), (0, 13), (0, 37)));

    let two_hearts = one_nt.intersect(&four_five_hearts);

    // The 5=♠ box (hearts 2-3) contradicts 4-5♥ and is dropped: 2 boxes, not 3.
    assert_eq!(two_hearts.boxes().len(), 2, "empty product not dropped");
    // The survivors pin hearts to exactly 4 (from balanced) and exactly 5.
    let hearts: Vec<Range> = two_hearts
        .boxes()
        .iter()
        .map(|b| b.length(Suit::Hearts))
        .collect();
    assert!(hearts.contains(&Range::new(4, 4)) && hearts.contains(&Range::new(5, 5)));

    // The hull re-widens to the bounding box — the slop the union avoids: it
    // admits ♠4♥5, a hand *neither* surviving box holds (balanced caps ♠ at 4
    // only with ≤4♥; the 5♥ box caps ♠ at 3).
    let hull = two_hearts.hull();
    let folded_span = two_hearts
        .boxes()
        .iter()
        .copied()
        .reduce(|a, b| a.span(&b))
        .unwrap_or_else(Envelope::unknown);
    assert_eq!(hull, folded_span);
    assert_eq!(hull.length(Suit::Hearts), Range::new(4, 5));
    assert_eq!(hull.length(Suit::Spades), Range::new(2, 4));
    assert!(
        two_hearts.boxes().iter().all(|b| {
            !(b.length(Suit::Spades).contains(4) && b.length(Suit::Hearts).contains(5))
        })
    );

    // Fully-contradictory intersect falls back to the widened hull, never empty.
    let empty = EnvelopeUnion::from(box_((0, 0), (0, 13), (0, 13), (0, 13), (0, 37)));
    let clubs = EnvelopeUnion::from(box_((5, 13), (0, 13), (0, 13), (0, 13), (0, 37)));
    let widened = empty.intersect(&clubs);
    let expected_widening = EnvelopeUnion::from(empty.hull().span(&clubs.hull()));
    assert_eq!(widened, expected_widening);

    let exact = empty.union(clubs);
    assert_eq!(exact.boxes().len(), 2, "exact union must retain both boxes");
}

/// `set_envelope_union_reading` gates the `Or` wall: off,
/// `or([♥, ♠], 6..)` hulls to one
/// box that admits a 5-4 hand with no six-card major; on, it keeps the two
/// boxes and rejects that hand while still admitting each true one-suiter.
#[test]
fn envelope_union_reading_pins_the_two_suiter() {
    use crate::bidding::constraint::{Constraint, or};
    assert!(
        std::thread::spawn(envelope_union_reading).join().unwrap(),
        "the envelope-union reading must default on"
    );

    // Holdings are spades.hearts.diamonds.clubs.
    let six_spades: Hand = "AKQJ32.KQ4.32.32".parse().unwrap();
    let six_hearts: Hand = "KQ4.AKQJ32.32.32".parse().unwrap();
    let five_four: Hand = "AKQJ3.KQ42.32.32".parse().unwrap(); // no six-card major
    let ctx = Context::new(RelativeVulnerability::NONE, &[]);
    let reading = or([Suit::Hearts, Suit::Spades], 6..);

    set_envelope_union_reading(true);
    let boxes = reading.project(&ctx);
    let expected_legacy_hull = EnvelopeUnion::from(boxes.hull());
    assert_eq!(boxes.boxes().len(), 2, "on: one box per major");
    assert!(boxes.contains(six_spades) && boxes.contains(six_hearts));
    assert!(
        !boxes.contains(five_four),
        "on: neither box holds the 5-4 hand"
    );

    set_envelope_union_reading(false);
    let hull = reading.project(&ctx);
    assert_eq!(hull, expected_legacy_hull, "off: the legacy span");
    assert_eq!(hull.boxes().len(), 1, "off: one bounding box");
    assert!(
        hull.contains(five_four),
        "off: the hull admits the 5-4 slop"
    );

    set_envelope_union_reading(true);
}

/// `set_blind_opponent_reading` blanks LHO and RHO and *only* those: the
/// deviation panel's blind arm must leave partner and our own reading
/// intact, or it stops measuring what reading *their* calls is worth.
#[test]
fn blind_opponent_reading_spares_our_side() {
    // 1♦ (me) - 1♥ (LHO) - 1♠ (partner) - 2♥ (RHO): all four seats have
    // shown something, so blanking two of them is visible.
    let auction = [
        bid(1, Strain::Diamonds),
        bid(1, Strain::Hearts),
        bid(1, Strain::Spades),
        bid(2, Strain::Hearts),
    ];
    let seen = read(&auction);
    set_blind_opponent_reading(true);
    let blind = read(&auction);
    set_blind_opponent_reading(false);

    for who in [Relative::Lho, Relative::Rho] {
        assert_eq!(*blind.get(who), Envelope::unknown(), "{who:?} not blanked");
        assert_eq!(blind.announced_union(who), &EnvelopeUnion::unknown());
    }
    assert_ne!(
        *seen.get(Relative::Rho),
        Envelope::unknown(),
        "the fixture must read RHO's 1♥, else the test proves nothing"
    );
    for who in [Relative::Me, Relative::Partner] {
        assert_eq!(*blind.get(who), *seen.get(who), "{who:?} moved");
        assert_eq!(blind.announced_union(who), seen.announced_union(who));
    }
    // Knob off is byte-identical to never having set it.
    let after = read(&auction);
    for who in [
        Relative::Me,
        Relative::Lho,
        Relative::Partner,
        Relative::Rho,
    ] {
        assert_eq!(*after.get(who), *seen.get(who), "{who:?} moved after reset");
        assert_eq!(after.announced_union(who), seen.announced_union(who));
    }
}

#[test]
fn opening_shapes() {
    // [1♥]: the opener sits to our right (the call just before ours).
    let one_heart = read(&[bid(1, Strain::Hearts)]);
    assert_eq!(one_heart.rho().length(Suit::Hearts), Range::new(5, 13));
    // `points(12..)` is the Rule of 20, which opens sound 10-11 HCP counts,
    // so the floor is 10.
    assert_eq!(one_heart.rho().strength.points, Range::new(10, 21));

    // A strong notrump is balanced-or-6322-minor (the shipped Wide6322): a
    // major stays 2–5 (a balanced 5332 major), a minor widens to 2–6 (the
    // 6322's six-card minor); an artificial 2♣ says only "strong".
    let one_nt = read(&[bid(1, Strain::Notrump)]);
    assert_eq!(one_nt.rho().length(Suit::Spades), Range::new(2, 5));
    assert_eq!(one_nt.rho().length(Suit::Diamonds), Range::new(2, 6));
    // Plain HCP 15–17: no downgrade on the shipped floored scale, a
    // semi-balanced 5422/6322 reads one over → 15–18.
    assert_eq!(one_nt.rho().strength.points, Range::new(15, 18));

    let two_clubs = read(&[bid(2, Strain::Clubs)]);
    assert_eq!(two_clubs.rho().length(Suit::Spades), Range::FULL_LENGTH);
    assert_eq!(two_clubs.rho().strength.points, Range::new(20, 37));

    // Weak two: exactly six; three-level preempt: seven-plus.
    let weak_two = read(&[bid(2, Strain::Spades)]);
    assert_eq!(weak_two.rho().length(Suit::Spades), Range::new(6, 6));
    assert_eq!(weak_two.rho().strength.points, Range::new(5, 10));
    let preempt = read(&[bid(3, Strain::Diamonds)]);
    assert_eq!(preempt.rho().length(Suit::Diamonds), Range::new(7, 13));

    // A 1♣ opening denies a five-card major.
    let one_club = read(&[bid(1, Strain::Clubs)]);
    assert_eq!(one_club.rho().length(Suit::Clubs), Range::new(3, 13));
    assert_eq!(one_club.rho().length(Suit::Hearts), Range::new(0, 4));
}

/// A two-over-one denies four-card support, and the reading now says so.
///
/// `Flip` had no projection at all, so `!support(4..)` — a plain box, "at
/// most three of partner's suit" — read as ⊤ and responder's spades came
/// back `0..=13` after `1♠–2♣`.  The strength half of the same rule is
/// still blind (`Or::project` unions `hcp(13..)` away; see
/// `docs/ai-bidder/sampled-projection.md`), which is why only the length
/// axis is asserted here.
#[test]
fn two_over_one_denies_four_card_support() {
    let auction = [bid(1, Strain::Spades), Call::Pass, bid(2, Strain::Clubs)];
    let read = read_booked(&auction);
    let responder = read.rho();
    assert_eq!(responder.length(Suit::Spades), Range::new(0, 3));
    assert_eq!(responder.length(Suit::Clubs), Range::new(4, 13));
}

#[test]
fn pass_reading_caps_the_no_open_pass() {
    let p = Call::Pass;
    // Knob off — the pre-ship identity: a pass reads nothing.
    set_pass_reading(false);
    assert_eq!(
        read_booked(&[p, p]).partner().strength.points,
        Range::FULL_POINTS
    );

    set_pass_reading(true);
    set_table_alert_reading(false);
    // Partner's no-open pass reads the opening table's own gate,
    // `points(..12)`; an opponent's pass stays unread until table-wide
    // disclosure is on too.
    let own = read_booked(&[p, p]);
    assert_eq!(own.partner().strength.points, Range::new(0, 11));
    assert_eq!(own.rho().strength.points, Range::FULL_POINTS);
    set_table_alert_reading(true);
    assert_eq!(read_booked(&[p]).rho().strength.points, Range::new(0, 11));
    // A capped passer leaves the opener's own band alone.
    let opened = read_booked(&[p, bid(1, Strain::Hearts)]);
    assert_eq!(opened.partner().strength.points, Range::new(0, 11));
    assert_eq!(opened.rho().strength.points, Range::new(10, 21));
}

#[test]
fn pass_reading_caps_the_failed_compete() {
    let auction = [bid(1, Strain::Hearts), Call::Pass, Call::Pass];
    set_pass_reading(false);
    assert_eq!(
        read_booked(&auction).partner().strength.points,
        Range::FULL_POINTS
    );

    set_pass_reading(true);
    set_table_alert_reading(false);
    // Partner's direct-seat pass: the authored complement of the strong
    // tier ("strong hands double first regardless") — at most 17 raw HCP,
    // 19 on the point-count scale (17 + max upgrade 2).  Their responder's
    // pass stays unread until table-wide disclosure is on.
    let own = read_booked(&auction);
    assert_eq!(own.partner().strength.points, Range::new(0, 19));
    assert_eq!(own.rho().strength.points, Range::FULL_POINTS);
    set_table_alert_reading(true);
    // Their responder's pass: the response table's `hcp(..6)` gate — at
    // most 5 raw HCP, 7 on the point-count scale (5 + max upgrade 2).
    assert_eq!(
        read_booked(&auction).rho().strength.points,
        Range::new(0, 7)
    );
}

#[test]
fn pass_reading_caps_the_silent_responder() {
    set_pass_reading(true);
    // Our 1♥, silent partner: the response table's `hcp(..6)` gate —
    // at most 5 raw HCP, 7 on the point-count scale (5 + max upgrade 2).
    let caps = read_booked(&[bid(1, Strain::Hearts), Call::Pass, Call::Pass, Call::Pass]);
    assert_eq!(caps.partner().strength.points, Range::new(0, 7));
}

#[test]
fn pass_reading_caps_the_notrump_signoff() {
    set_pass_reading(true);
    // Pass of partner's 1NT: the authored union of the weak arm and the
    // flat-eight arm — at most 10 points (the flat-eight arm's 8 HCP + the
    // point-count max upgrade 2), no six-card major.
    let nt = read_booked(&[bid(1, Strain::Notrump), Call::Pass, Call::Pass, Call::Pass]);
    assert_eq!(nt.partner().strength.points, Range::new(0, 10));
    assert!(nt.partner().length(Suit::Hearts).max <= 5);
    assert!(nt.partner().length(Suit::Spades).max <= 5);
}

#[test]
fn pass_reading_skips_trap_and_trivial_passes() {
    set_pass_reading(true);
    set_table_alert_reading(true);
    // The advance of a takeout double authors genuine strong sits (the
    // penalty conversion), so its pass-gate union is trivial: nothing is
    // claimed about the advancer even with every reading knob on.
    let trap = read_booked(&[bid(1, Strain::Hearts), Call::Double, Call::Pass, Call::Pass]);
    assert_eq!(trap.rho().strength.points, Range::FULL_POINTS);
}

/// Pass-exclusion (`set_pass_exclusion_reading`) caps the direct-seat pass
/// over their weak two off the *declined* shape-free double tier
/// (`points(17..)`, weight 1.2) — the catch-all `hcp(0..)` Pass gate says
/// nothing on its own, which is why this key read 100% blind in the census.
/// Shaped siblings (the overcalls, the 2NT arm) complement to unions or ⊤
/// and are skipped by the single-box filter, so the lengths stay ⊤.
#[test]
fn pass_exclusion_caps_the_weak_two_defender() {
    let auction = [bid(2, Strain::Spades), Call::Pass, Call::Pass];
    set_pass_reading(true);
    set_table_alert_reading(false);

    // Knob off — today's identity: the catch-all gate reads nothing.
    set_pass_exclusion_reading(false);
    let off = read_booked(&auction);
    assert_eq!(off.partner().strength.points, Range::FULL_POINTS);

    // Knob on — declining the 17+ double caps the passer.
    set_pass_exclusion_reading(true);
    let on = read_booked(&auction);
    assert_eq!(on.partner().strength.points, Range::new(0, 16));
    // The overcall complements are multi-box and skipped: no length claim.
    assert_eq!(on.partner().length(Suit::Hearts), Range::new(0, 13));

    // Off again is byte-identical to never having been on.
    set_pass_exclusion_reading(false);
    assert_eq!(read_booked(&auction).partner(), off.partner());
    set_table_alert_reading(true);
}

#[test]
fn opener_extras_ladder_reads_extras() {
    use crate::bidding::american::set_opener_extras_ladder;
    let d = bid(1, Strain::Diamonds);
    let s = bid(1, Strain::Spades);
    let p = Call::Pass;
    set_opener_extras_ladder(true);
    // Opener (partner of the hero to act) after 1♦ – 1♠ – X.
    // Jump-rebid 3♦: a self-sufficient six-plus diamonds, 16+.
    let jr = read(&[d, p, s, p, bid(3, Strain::Diamonds), p]);
    assert!(jr.partner().length(Suit::Diamonds).min >= 6);
    assert!(jr.partner().strength.points.min >= 16);
    // Reverse 2♥: five-plus diamonds, four-plus hearts, 17+.
    let rev = read(&[d, p, s, p, bid(2, Strain::Hearts), p]);
    assert!(rev.partner().length(Suit::Diamonds).min >= 5);
    assert!(rev.partner().length(Suit::Hearts).min >= 4);
    assert!(rev.partner().strength.points.min >= 17);
    // Jump-shift 3♣: five-plus diamonds, 18+, and clubs read as the strong
    // 4+ second suit — NOT the weak-jump six (the phantom-suit fix).
    let js = read(&[d, p, s, p, bid(3, Strain::Clubs), p]);
    assert!(js.partner().length(Suit::Diamonds).min >= 5);
    assert!(js.partner().strength.points.min >= 18);
    assert_eq!(
        js.partner().length(Suit::Clubs),
        Range::at_least(4, LENGTH_CAP)
    );
    set_opener_extras_ladder(true);
}

#[test]
fn opener_major_jump_rebid_reads_extras() {
    use crate::bidding::american::set_opener_major_jump_rebid;
    let h = bid(1, Strain::Hearts);
    let s = bid(1, Strain::Spades);
    let p = Call::Pass;
    set_opener_major_jump_rebid(true);
    // Opener after 1♥ – 1♠ – 3♥: jump-rebid of a six-plus major, 16+.
    let jr = read(&[h, p, s, p, bid(3, Strain::Hearts), p]);
    assert!(jr.partner().length(Suit::Hearts).min >= 6);
    assert!(jr.partner().strength.points.min >= 16);
    set_opener_major_jump_rebid(true);
}

/// The M6.4 deterministic rule on its canonical auctions: a
/// four-plus-level new suit is a control bid iff the bidder *bypassed*
/// it (available below their first-shown suit at the same level);
/// everything else stays to play — suppressed, nothing floored.
#[test]
fn high_bid_control_vs_natural() {
    use crate::bidding::american::set_longer_major_response;
    // Pin the historic hearts-first reading (knob off): these
    // minor-response verdicts are the knob-off ones — the longer-major
    // default is covered by `high_bid_under_longer_major_response`, and the
    // 1NT-transfer sub-cases below are knob-independent.
    set_longer_major_response(false);
    // 1♦–1♠–2♦–4♥: responder bid spades first, so hearts cannot be their
    // longest — a control bid agreeing diamonds.  Hearts stays unfloored;
    // diamond support and slam-try values are recorded instead.
    let control = read(&[
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(4, Strain::Hearts),
        Call::Pass,
    ]);
    assert_eq!(control.partner().length(Suit::Hearts).min, 0);
    assert!(control.partner().length(Suit::Diamonds).min >= 3);
    assert!(control.partner().strength.points.min >= 13);

    // 1♦–1♠–2♦–4♠: rebidding one's own suit is natural — six-plus spades.
    let rebid = read(&[
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(4, Strain::Spades),
        Call::Pass,
    ]);
    assert!(rebid.partner().length(Suit::Spades).min >= 6);

    // 1♦–4♥: the bidder has shown nothing, so hearts can be their
    // longest — to play, no control machinery (and no phantom floor:
    // the honest envelope of an unread jump stays wide).
    let preempt = read(&[
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(4, Strain::Hearts),
        Call::Pass,
    ]);
    assert!(preempt.control_bid().is_none());

    // 1♣–1♥–2♣–4♠: spades sit *above* the first-shown hearts, so they were
    // never denied — this system's response and transfer styles bid the
    // cheaper suit first holding a longer higher one (the first M6.4 A/B
    // bled six IMPs a fired board pulling these to the "agreed" minor).
    // To play, not a control bid.
    let above = read(&[
        bid(1, Strain::Clubs),
        Call::Pass,
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(4, Strain::Spades),
        Call::Pass,
    ]);
    assert!(above.control_bid().is_none());

    // 1NT–2♦–2♥–4♠: same shape through a transfer (the overlay attributes
    // the hearts to the bidder) — spades were never denied, so to play.
    let post_transfer = read_booked(&[
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(4, Strain::Spades),
        Call::Pass,
    ]);
    assert!(post_transfer.control_bid().is_none());
    assert!(post_transfer.partner().length(Suit::Hearts).min >= 5);

    // 1NT–2♥–2♠–4♥ — the mirror: hearts sit *below* the transferred
    // spades and the cheaper heart transfer was bypassed, so 4♥ cannot be
    // long — a control bid agreeing spades, promising a sixth.
    let mirror = read_booked(&[
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Spades),
        Call::Pass,
        bid(4, Strain::Hearts),
        Call::Pass,
    ]);
    assert_eq!(mirror.partner().length(Suit::Hearts).min, 0);
    assert!(mirror.partner().length(Suit::Spades).min >= 6);
    set_longer_major_response(true); // restore the shipped default
}

/// The longer-major response discipline swaps the M6.4 verdicts on the
/// two major-response auctions: a 1♥ response denies longer spades (so
/// the spade jump becomes a control bid), and a 1♠ response may conceal
/// equal-length five-plus hearts (so the heart jump reads to play).
#[test]
fn high_bid_under_longer_major_response() {
    use crate::bidding::american::set_longer_major_response;

    // 1♣–1♥–2♣–4♠, discipline on: 1♥ denied longer spades, so 4♠ is a
    // bypass — a control bid agreeing clubs, spades left unfloored.
    set_longer_major_response(true);
    let control = read(&[
        bid(1, Strain::Clubs),
        Call::Pass,
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(4, Strain::Spades),
        Call::Pass,
    ]);
    // The mirror 1♣–1♠–2♣–4♥: a 1♠ response no longer proves short
    // hearts (5-5 responds 1♠), so the heart jump reads to play.
    let to_play = read(&[
        bid(1, Strain::Clubs),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(4, Strain::Hearts),
        Call::Pass,
    ]);
    assert_eq!(control.partner().length(Suit::Spades).min, 0);
    assert!(control.partner().length(Suit::Clubs).min >= 3);
    assert!(control.partner().strength.points.min >= 13);
    assert!(to_play.control_bid().is_none());

    // Knob off (the historic hearts-first opt-in): the original verdicts
    // stand — the spade jump above the 1♥ response is to play.
    set_longer_major_response(false);
    let above = read(&[
        bid(1, Strain::Clubs),
        Call::Pass,
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(4, Strain::Spades),
        Call::Pass,
    ]);
    set_longer_major_response(true); // restore the shipped default
    assert!(above.control_bid().is_none());
}

#[test]
fn gambling_3nt_over_double_reads_unbalanced() {
    use crate::bidding::instinct::set_gambling_3nt_over_double;
    // [1NT,(X),3NT,P]: opener reads partner's gambling 3NT.  The floor alerts the
    // call as the long-minor gamble, so the natural balanced-3NT reading is
    // suppressed and a six-card minor stays within range — the search sampler must
    // be free to deal responder its running suit, not pin it to a flat hand.
    set_gambling_3nt_over_double(true);
    let read = read_booked(&[
        bid(1, Strain::Notrump),
        Call::Double,
        bid(3, Strain::Notrump),
        Call::Pass,
    ]);
    assert!(read.partner().length(Suit::Clubs).contains(6));
    assert!(read.partner().length(Suit::Diamonds).contains(6));
    set_gambling_3nt_over_double(false);
}

#[test]
fn leaping_michaels_conditions_partner() {
    use crate::bidding::american::set_leaping_michaels;

    // (2♥)–4♣–(P): the advancer reads partner's two-suiter — five-plus clubs
    // AND five-plus spades, game-forcing — so the search sampler deals partner
    // the right shape rather than a natural club one-suiter.
    set_leaping_michaels(true);
    let advance = read_booked(&[bid(2, Strain::Hearts), bid(4, Strain::Clubs), Call::Pass]);
    assert_eq!(advance.partner().length(Suit::Clubs), Range::new(5, 13));
    assert_eq!(advance.partner().length(Suit::Spades), Range::new(5, 13));
    assert_eq!(advance.partner().strength.points, Range::new(14, 37));

    // Over 2♦, the 4♦ cue shows both majors; 4♣ shows clubs + an unknown
    // major, so only clubs is pinned.
    let cue = read_booked(&[
        bid(2, Strain::Diamonds),
        bid(4, Strain::Diamonds),
        Call::Pass,
    ]);
    assert_eq!(cue.partner().length(Suit::Hearts), Range::new(5, 13));
    assert_eq!(cue.partner().length(Suit::Spades), Range::new(5, 13));

    // Disabled (the default): a 4♣ jump reads as a natural one-suiter, so
    // spades stay unconstrained — the convention must not leak when off.
    set_leaping_michaels(false);
    let off = read_booked(&[bid(2, Strain::Hearts), bid(4, Strain::Clubs), Call::Pass]);
    assert_eq!(off.partner().length(Suit::Spades), Range::FULL_LENGTH);
    set_leaping_michaels(true);
}

#[test]
fn landy_conditions_partner() {
    use crate::bidding::american::{set_landy, set_unusual_notrump_defense};

    // (1NT)–2♣–(P): the advancer reads partner's both-majors two-suiter (at
    // least 4-4 in the majors, 8+ points) rather than a natural club suit.
    set_landy(Some((8, 15)));
    set_unusual_notrump_defense(Some((8, 15)));
    let advance = read_booked(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
    assert_eq!(advance.partner().length(Suit::Hearts), Range::new(4, 13));
    assert_eq!(advance.partner().length(Suit::Spades), Range::new(4, 13));
    assert_eq!(advance.partner().length(Suit::Clubs), Range::FULL_LENGTH);
    assert_eq!(advance.partner().strength.points, Range::new(8, 37));

    // (1NT)–2NT–(P): both minors, 5-5 (the independent unusual-2NT toggle).
    let minors = read_booked(&[bid(1, Strain::Notrump), bid(2, Strain::Notrump), Call::Pass]);
    assert_eq!(minors.partner().length(Suit::Clubs), Range::new(5, 13));
    assert_eq!(minors.partner().length(Suit::Diamonds), Range::new(5, 13));

    // The advancer's 2♦ relay is artificial — read from the overcaller's seat,
    // partner's (the relayer's) diamonds stay unconstrained.
    let relay = read_booked(&[
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
    ]);
    assert_eq!(relay.partner().length(Suit::Diamonds), Range::FULL_LENGTH);

    // Disabled: 2♣ reads as a natural club one-suiter, so spades stay
    // unconstrained — the convention must not leak when off.
    set_landy(None);
    set_unusual_notrump_defense(None);
    let off = read_booked(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
    assert_eq!(off.partner().length(Suit::Spades), Range::FULL_LENGTH);

    // Restore the shipped defaults so sibling tests on this thread are unaffected
    // (unusual 2NT ships on; Landy 2♣ ships off).
    set_unusual_notrump_defense(Some((8, 13)));
}

#[test]
fn woolsey_conditions_partner() {
    use crate::bidding::american::{
        NotrumpDefense, set_landy, set_notrump_defense, set_unusual_notrump_defense,
        set_woolsey_points,
    };
    // Landy off, Woolsey on: the 2♣ must read through the Woolsey path.
    set_landy(None);
    set_unusual_notrump_defense(None);
    set_notrump_defense(NotrumpDefense::Woolsey);
    set_woolsey_points(10, 19);

    // (1NT)–2♣–(P): Woolsey's 2♣ is both majors, 10+, never a natural club suit.
    // Read off the authored rule's projection (on a prefixed/booked context),
    // which pins each major to 4-5 exactly — Woolsey sends a six-card major to
    // the Multi/Muiderberg calls, a distinction the old loose reader missed.
    let two_c = read_booked(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
    assert_eq!(two_c.partner().length(Suit::Hearts), Range::new(4, 5));
    assert_eq!(two_c.partner().length(Suit::Spades), Range::new(4, 5));
    assert_eq!(two_c.partner().length(Suit::Clubs), Range::FULL_LENGTH);
    assert_eq!(two_c.partner().strength.points, Range::new(10, 37));

    // (1NT)–2♦–(P): the Multi names diamonds it does NOT hold, so the natural
    // ≥5 reading is suppressed and BOTH minors narrow to ≤4 — the floor can no
    // longer "raise diamonds" into a doubled 5♦ (the 6+ major falls out of the
    // residual the per-suit framework cannot pin).
    let multi = read(&[
        bid(1, Strain::Notrump),
        bid(2, Strain::Diamonds),
        Call::Pass,
    ]);
    assert_eq!(multi.partner().length(Suit::Diamonds), Range::new(0, 4));
    assert_eq!(multi.partner().length(Suit::Clubs), Range::new(0, 4));

    // (1NT)–2♥–(P): Muiderberg — exactly 5 hearts, ≤3 spades.
    let muiderberg = read(&[bid(1, Strain::Notrump), bid(2, Strain::Hearts), Call::Pass]);
    assert_eq!(muiderberg.partner().length(Suit::Hearts), Range::new(5, 5));
    assert_eq!(muiderberg.partner().length(Suit::Spades), Range::new(0, 3));

    // The advancer's 2♥/2♠ over 2♣ (both majors) or 2♦ (Multi) is a PREFERENCE
    // among partner's two majors — not own length — so its natural ≥4 reading is
    // suppressed throughout (here, read from the advancer's seat as partner).
    let pref_2c = read(&[
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
    ]);
    assert_eq!(pref_2c.partner().length(Suit::Hearts), Range::FULL_LENGTH);
    let pref_2d = read(&[
        bid(1, Strain::Notrump),
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Spades),
        Call::Pass,
    ]);
    assert_eq!(pref_2d.partner().length(Suit::Spades), Range::FULL_LENGTH);

    // Off: the Multi 2♦ reads as a natural diamond one-suiter again (≥5) — the
    // convention must not leak when disabled.
    set_notrump_defense(NotrumpDefense::Natural);
    let off = read(&[
        bid(1, Strain::Notrump),
        bid(2, Strain::Diamonds),
        Call::Pass,
    ]);
    assert_eq!(off.partner().length(Suit::Diamonds), Range::new(5, 13));

    // Restore the shipped default (unusual 2NT ships on).
    set_unusual_notrump_defense(Some((8, 13)));
    set_woolsey_points(8, 19);
}

#[test]
fn artificial_witness_covers_doubles() {
    // A projection that floors a suit it would not name — the witness a transfer
    // or two-suiter trips (5+ hearts).
    let mut floors_hearts = Envelope::unknown();
    floors_hearts.narrow_length(Suit::Hearts, Range::at_least(5, LENGTH_CAP));

    // A *bid* that did not name hearts is artificial (Jacoby 2♦ → 5+♥); a bid
    // naming its own suit is natural (1♥ → 5+♥).
    assert!(artificial(&floors_hearts, bid(2, Strain::Diamonds), None));
    assert!(!artificial(&floors_hearts, bid(1, Strain::Hearts), None));

    // A pass redirects from nothing → never artificial, even flooring a suit.
    assert!(!artificial(
        &floors_hearts,
        Call::Pass,
        Some(Strain::Spades)
    ));

    // A double/redouble "names" the *doubled strain*.  Doubling spades while the
    // projection floors hearts is takeout — it points partner at hearts → artificial;
    // doubling hearts while flooring hearts defends the doubled strain → natural
    // (penalty).  A redouble inherits the same doubled strain.
    assert!(artificial(
        &floors_hearts,
        Call::Double,
        Some(Strain::Spades)
    ));
    assert!(!artificial(
        &floors_hearts,
        Call::Double,
        Some(Strain::Hearts)
    ));
    assert!(artificial(
        &floors_hearts,
        Call::Redouble,
        Some(Strain::Spades)
    ));
    assert!(!artificial(
        &floors_hearts,
        Call::Redouble,
        Some(Strain::Hearts)
    ));

    // A double of notrump defends no suit, so any floored side suit is takeout.
    assert!(artificial(
        &floors_hearts,
        Call::Double,
        Some(Strain::Notrump)
    ));
}

#[test]
fn woolsey_double_and_advances_read() {
    use crate::bidding::american::{
        NotrumpDefense, set_landy, set_notrump_defense, set_unusual_notrump_defense,
        set_woolsey_double_floor, set_woolsey_points,
    };
    set_landy(None);
    set_unusual_notrump_defense(None);
    set_notrump_defense(NotrumpDefense::Woolsey);
    set_woolsey_points(10, 19);
    set_woolsey_double_floor(12);

    // (1NT)–X–(P): the takeout double names no suit, so nothing is misread — but
    // the doubler's strength (12+) is recorded, where a bare double of 1NT would
    // otherwise read as nothing.
    let x = read(&[bid(1, Strain::Notrump), Call::Double, Call::Pass]);
    assert_eq!(x.partner().strength.points, Range::new(12, 37));

    // (1NT)–X–(P)–2♣–(P): the advancer's 2♣ is a "name your minor" relay, not own
    // clubs, so its natural ≥4 reading is suppressed (read from the advancer seat).
    let relay = read(&[
        bid(1, Strain::Notrump),
        Call::Double,
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
    ]);
    assert_eq!(relay.partner().length(Suit::Clubs), Range::FULL_LENGTH);

    // (1NT)–2♥–(P)–2NT–(P): the Muiderberg minor-ask 2NT is a relay in a
    // COMPETITIVE auction (our side already overcalled), so it is never read as a
    // natural notrump invite — the advancer's points stay unconstrained.
    let ask = read(&[
        bid(1, Strain::Notrump),
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Notrump),
        Call::Pass,
    ]);
    assert_eq!(ask.partner().strength.points, Range::new(0, 37));

    // Off: the Woolsey 12+ reading must not leak — the double now falls through to
    // the default-on natural penalty reading (15+), not Woolsey's 12+.
    set_notrump_defense(NotrumpDefense::Natural);
    let off = read(&[bid(1, Strain::Notrump), Call::Double, Call::Pass]);
    assert_eq!(off.partner().strength.points, Range::new(15, 37));

    set_unusual_notrump_defense(Some((8, 13)));
    set_woolsey_points(8, 19);
}

#[test]
fn dont_overcalls_and_advances_read() {
    use crate::bidding::american::{
        NotrumpDefense, set_landy, set_notrump_defense, set_unusual_notrump_defense,
    };
    set_landy(None);
    set_unusual_notrump_defense(None);
    set_notrump_defense(NotrumpDefense::DirectDont);

    // (1NT)–X–(P): a one-suiter in ♣/♦/♥ — spades short (≤3, the one sound fact),
    // strength recorded (the default 8+ overcall floor) where a bare double of 1NT
    // would otherwise read as nothing.
    let x = read(&[bid(1, Strain::Notrump), Call::Double, Call::Pass]);
    assert_eq!(x.partner().length(Suit::Spades), Range::new(0, 3));
    assert_eq!(x.partner().strength.points, Range::new(8, 37));

    // (1NT)–X–(P)–2♣–(P): the advancer's 2♣ is a "name your suit" relay, not own
    // clubs, so its natural ≥4 reading is suppressed (read from the advancer seat).
    let relay = read(&[
        bid(1, Strain::Notrump),
        Call::Double,
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
    ]);
    assert_eq!(relay.partner().length(Suit::Clubs), Range::FULL_LENGTH);

    // (1NT)–2♣–(P): a real ≥4 club suit + an unknown major.  The natural ≥5 reading
    // is suppressed (a 4-club / 5-major DONT hand makes this call), re-pinned to ≥4.
    let two_c = read(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
    assert_eq!(two_c.partner().length(Suit::Clubs), Range::new(4, 13));
    assert_eq!(two_c.partner().strength.points, Range::new(8, 37));

    // (1NT)–2♣–(P)–2♦–(P): the advancer's 2♦ is a "name your higher suit" relay,
    // not own diamonds — suppressed.
    let pref = read(&[
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
    ]);
    assert_eq!(pref.partner().length(Suit::Diamonds), Range::FULL_LENGTH);

    // (1NT)–2♥–(P): both majors, ≥4-4 — exactly a Landy two-suiter on the 2♥ bid.
    let two_h = read(&[bid(1, Strain::Notrump), bid(2, Strain::Hearts), Call::Pass]);
    assert_eq!(two_h.partner().length(Suit::Hearts), Range::new(4, 13));
    assert_eq!(two_h.partner().length(Suit::Spades), Range::new(4, 13));

    // Off: the 2♣ reads as a natural club one-suiter again (≥5) — no leak.
    set_notrump_defense(NotrumpDefense::Natural);
    let off = read(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
    assert_eq!(off.partner().length(Suit::Clubs), Range::new(5, 13));
}

#[test]
fn meckwell_overcalls_and_advances_read() {
    use crate::bidding::american::{
        NotrumpDefense, set_landy, set_notrump_defense, set_unusual_notrump_defense,
    };
    set_landy(None);
    set_unusual_notrump_defense(None);
    set_notrump_defense(NotrumpDefense::Meckwell);

    // (1NT)–X–(P): the two-way double (single 6+ minor OR both majors) shares no
    // sound per-suit fact, so ONLY the points floor is recorded — no length is
    // narrowed (unlike DONT's X, which pins spades ≤ 3).
    let x = read(&[bid(1, Strain::Notrump), Call::Double, Call::Pass]);
    assert_eq!(x.partner().strength.points, Range::new(8, 37));
    assert_eq!(x.partner().length(Suit::Spades), Range::FULL_LENGTH);
    assert_eq!(x.partner().length(Suit::Hearts), Range::FULL_LENGTH);

    // (1NT)–X–(P)–2♣–(P): the advancer's 2♣ is a "name your suit" relay, not own
    // clubs, so its natural ≥ 4 reading is suppressed.
    let relay = read(&[
        bid(1, Strain::Notrump),
        Call::Double,
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
    ]);
    assert_eq!(relay.partner().length(Suit::Clubs), Range::FULL_LENGTH);

    // (1NT)–2♣–(P): a real ≥ 4 club suit + an unknown major.  The natural ≥ 5
    // reading is suppressed (a 4-club / 5-major hand makes this call), re-pinned ≥ 4.
    let two_c = read(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
    assert_eq!(two_c.partner().length(Suit::Clubs), Range::new(4, 13));
    assert_eq!(two_c.partner().strength.points, Range::new(8, 37));

    // (1NT)–2♦–(P): diamonds + a major, real ≥ 4.
    let two_d = read(&[
        bid(1, Strain::Notrump),
        bid(2, Strain::Diamonds),
        Call::Pass,
    ]);
    assert_eq!(two_d.partner().length(Suit::Diamonds), Range::new(4, 13));

    // (1NT)–2♥–(P): NATURAL hearts (Meckwell's 2♥ is a single-suiter, not DONT's
    // both-majors), so spades are not floored — the DONT-vs-Meckwell fork.
    let two_h = read(&[bid(1, Strain::Notrump), bid(2, Strain::Hearts), Call::Pass]);
    assert_eq!(
        two_h.partner().length(Suit::Spades).min,
        0,
        "natural 2♥ shows no spades",
    );

    // Off: the 2♣ reads as a natural club one-suiter again (≥ 5) — no leak.
    set_notrump_defense(NotrumpDefense::Natural);
    let off = read(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
    assert_eq!(off.partner().length(Suit::Clubs), Range::new(5, 13));

    set_unusual_notrump_defense(Some((8, 13)));
}

#[test]
fn narrowed_points_intersects_one_player() {
    // 1NT shows 15-18; narrow the opener (here our RHO) to the upper half.
    let inf = read(&[bid(1, Strain::Notrump)]);
    assert_eq!(inf.rho().strength.points, Range::new(15, 18));

    let upper = inf.narrowed_points(Relative::Rho, Range::new(17, 18));
    assert_eq!(
        upper.rho().strength.points,
        Range::new(17, 18),
        "narrowed to the half"
    );
    assert_eq!(
        inf.rho().strength.points,
        Range::new(15, 18),
        "original unchanged"
    );
    // Shape and the other players are untouched.
    assert_eq!(
        upper.rho().length(Suit::Spades),
        inf.rho().length(Suit::Spades)
    );
    assert_eq!(
        upper.partner().strength.points,
        inf.partner().strength.points
    );

    // Intersection, not replacement: a wider request cannot widen what was shown.
    let clamped = inf.narrowed_points(Relative::Rho, Range::new(0, POINTS_CAP));
    assert_eq!(clamped.rho().strength.points, Range::new(15, 18));
}

#[test]
fn third_seat_openings_are_light() {
    // [P, P, 1♠]: a third-seat opener may be down to nine points.
    let third = read(&[Call::Pass, Call::Pass, bid(1, Strain::Spades)]);
    assert_eq!(third.rho().strength.points, Range::new(9, 21));
}

#[test]
fn responses_narrow_partner_and_opener() {
    // [1♥, P, 2♣, P]: we opened 1♥ (partner is us at index 0... no — at
    // len 4, index 0 is Me), partner responded 2♣ (game-forcing 2/1).
    let auction = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
    ];
    let inf = read(&auction);
    // Index 0 (1♥) is four before the actor → Me, the opener.
    assert_eq!(inf.me().length(Suit::Hearts), Range::new(5, 13));
    // Index 2 (2♣) is two before → Partner, the 2/1 responder.
    assert_eq!(inf.partner().length(Suit::Clubs), Range::new(4, 13));
    assert_eq!(inf.partner().strength.points, Range::new(13, 37));
}

#[test]
fn opener_rebid_reads_five_plus_by_default() {
    // [1♥, P, 1♠, P, 2♥, P]: the opener (who bid 1♥ and rebid 2♥) sits as
    // partner, and the 1♠ responder is us.  The shipped sound reading
    // keeps the rebid at five-plus (the floor routinely rebids a good
    // five); the legacy six-card claim needs the knob off.
    let auction = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read(&auction);
    assert_eq!(inf.partner().length(Suit::Hearts), Range::new(5, 13));
    // Our 1♠ response showed four spades and six-plus points.
    assert_eq!(inf.me().length(Suit::Spades), Range::new(4, 13));
    assert_eq!(inf.me().strength.points, Range::new(6, 37));
    set_length_soundness(false);
    let legacy = read(&auction);
    assert_eq!(legacy.partner().length(Suit::Hearts), Range::new(6, 13));
    set_length_soundness(true);
}

#[test]
fn competitive_opener_rebid_shows_sixth_card() {
    // [1♦, 1♥, P, 2♥, 3♦, P]: partner opened 1♦ and, over the opponents'
    // heart auction, rebid 3♦ (the opt-in `set_competitive_rebid` floor).
    // The natural length reading applies in competition too — only the
    // *strength* reading is suppressed when opponents act — so partner is
    // still read with six-plus diamonds, keeping the sampler and any further
    // interference sound.  Knob-independent: `read` interprets the auction.
    let auction = [
        bid(1, Strain::Diamonds),
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Hearts),
        bid(3, Strain::Diamonds),
        Call::Pass,
    ];
    let inf = read(&auction);
    assert_eq!(inf.partner().length(Suit::Diamonds), Range::new(6, 13));
}

#[test]
fn overcall_shows_five_cards() {
    // [1♦, 1♠]: their 1♦ opening, our partner's... no — at len 2, index 1
    // (1♠) is RHO.  Their 1♦ is two before → Partner? recompute below.
    let auction = [bid(1, Strain::Diamonds), bid(1, Strain::Spades)];
    let inf = read(&auction);
    // Index 0 (1♦ opening) → Partner; index 1 (1♠ overcall) → Rho.
    assert_eq!(inf.partner().length(Suit::Diamonds), Range::new(3, 13));
    assert_eq!(inf.rho().length(Suit::Spades), Range::new(5, 13));
    assert_eq!(inf.rho().strength.points, Range::new(8, 37));
}

#[test]
fn transfers_are_not_read_as_natural() {
    // [1NT, P, 2♦, P]: 2♦ is a Jacoby transfer, not diamonds — the
    // opening side's artificial response leaves shape unknown.
    let auction = [
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
    ];
    let inf = read(&auction);
    assert_eq!(inf.partner().length(Suit::Diamonds), Range::FULL_LENGTH);
}

#[test]
fn three_level_suit_over_one_notrump_is_natural() {
    // [1NT, P, 3♥, P]: with the splinter *not* authored, a three-level suit
    // bid over 1NT is forcing and natural in the instinct reading —
    // five-plus hearts.  This is the knob-off control for
    // `nt_splinter_is_read_as_shortness_not_length`; the splinter is on by
    // default, so the walk has to be asked for explicitly.
    crate::bidding::american::set_nt_splinter(false);
    let auction = [
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read(&auction);
    crate::bidding::american::set_nt_splinter(true);
    assert_eq!(inf.partner().length(Suit::Hearts), Range::new(5, 13));
}

#[test]
fn nt_splinter_is_read_as_shortness_not_length() {
    // [1NT, P, 3♥, P] with the splinter authored: the *same* call that reads
    // as five-plus hearts above now decodes off its alert into the pinned
    // shape — short hearts, 2-3 spades, exactly four diamonds, 5-6 clubs.
    // The natural walk would floor a phantom heart suit responder is void in.
    crate::bidding::american::set_nt_splinter(true);
    let auction = [
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read_booked(&auction);
    crate::bidding::american::set_nt_splinter(false);

    let partner = inf.partner();
    assert!(partner.length(Suit::Hearts).max <= 1);
    assert_eq!(partner.length(Suit::Spades), Range::new(2, 3));
    assert_eq!(partner.length(Suit::Diamonds), Range::new(4, 4));
    assert_eq!(partner.length(Suit::Clubs), Range::new(5, 6));

    // Knob off, the book has no 3♥ rule and the walk is back: five-plus.
    let off = read_booked(&auction);
    crate::bidding::american::set_nt_splinter(true); // restore the default
    assert_eq!(off.partner().length(Suit::Hearts), Range::new(5, 13));
}

#[test]
fn systems_on_overcall_transfer_is_not_read_as_diamonds() {
    // [1♦, 1NT, P, 2♦, P]: their 1♦, our 1NT overcall, the advancer's 2♦ is a
    // Jacoby transfer (grafted opening-1NT structure), not natural diamonds.
    // Stripping their opening reads it as [1NT, P, 2♦, P], so the floor never
    // raises a phantom diamond suit into a doubled disaster (the iron rule).
    let auction = [
        bid(1, Strain::Diamonds),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
    ];
    let inf = read(&auction);
    assert_eq!(inf.partner().length(Suit::Diamonds), Range::FULL_LENGTH);
}

#[test]
fn systems_on_stripped_read_is_separate_from_the_full_decision_cache() {
    let auction = [
        bid(1, Strain::Diamonds),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
    ];
    let hand: Hand = "AQ32.K53.QJ4.A92".parse().expect("valid test hand");
    let stance = crate::american().against();
    let uncached = stance.infer(RelativeVulnerability::NONE, &auction);
    let context = stance
        .prefixed_context(RelativeVulnerability::NONE, &auction)
        .with_decision_cache(hand);
    let cached = context.inferences();

    assert_eq!(*cached, uncached);
    assert_eq!(context.decision_cache_init_counts(), Some((1, 0, 0)));
    assert_eq!(cached.partner().length(Suit::Diamonds), Range::FULL_LENGTH);
}

#[test]
fn gladiator_cue_is_not_read_as_their_major() {
    // [1♠, 1NT, P, 2♠, P]: our 1NT overcall of their 1♠; the advancer's 2♠ is
    // Gladiator Stayman for hearts (exactly 4, INV+) — NOT a natural spade
    // suit.  The major-strip is suppressed for Gladiator, so `gladiator_reading`
    // reads the cue.
    crate::bidding::american::set_nt_overcall_gladiator(true);
    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Spades),
        Call::Pass,
    ];
    let inf = read(&auction);
    crate::bidding::american::set_nt_overcall_gladiator(false);
    // Their major is never floored into the advancer's hand (the iron rule)...
    assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
    // ...and the cue pins the four-card heart holding it promised.
    assert_eq!(inf.partner().length(Suit::Hearts), Range::new(4, 13));
}

#[test]
fn gladiator_relay_is_not_read_as_clubs() {
    // [1♠, 1NT, P, 2♣, P]: the advancer's 2♣ is the Gladiator relay (weak /
    // invitational, any suit), not a natural club suit.
    crate::bidding::american::set_nt_overcall_gladiator(true);
    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
    ];
    let inf = read(&auction);
    crate::bidding::american::set_nt_overcall_gladiator(false);
    assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
}

#[test]
fn gladiator_delayed_cue_is_read_as_exactly_three_not_spades() {
    // [1♠,1NT,P,2♣,P,2♦,P,2♠,P]: the advancer's SECOND 2♠ (after the 2♣ relay
    // and forced 2♦) is the Gladiator delayed cue — exactly 3 hearts, INV+ —
    // NOT a natural spade suit.  The suppression must cover it too, else the
    // floor raises a phantom spade suit into a doubled disaster (the iron rule).
    crate::bidding::american::set_nt_overcall_gladiator(true);
    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Spades),
        Call::Pass,
    ];
    let inf = read(&auction);
    crate::bidding::american::set_nt_overcall_gladiator(false);
    // Their major is never floored into the advancer's hand...
    assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
    // ...and the delayed cue pins exactly 3 hearts.
    assert_eq!(inf.partner().length(Suit::Hearts), Range::new(3, 3));
}

#[test]
fn gladiator_stolen_relay_double_is_read_as_the_relay() {
    // [1♠, 1NT, (2♣), X, P]: over RHO's systems-on 2♣, the advancer's Double is
    // the stolen Gladiator relay (weak-or-invitational, any suit) — NOT a
    // penalty double naming clubs.  The reader mirrors the book rebase.
    crate::bidding::american::set_nt_overcall_gladiator(true);
    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        Call::Double,
        Call::Pass,
    ];
    let inf = read(&auction);
    crate::bidding::american::set_nt_overcall_gladiator(false);
    // No phantom club suit raised from the doubled strain...
    assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
    // ...and no point cap: the relay's third arm is game-forcing, so the
    // `0..=9` this used to assert excluded hands the agreement admits (see
    // the `Relay` arm of the post-walk block).
    assert_eq!(inf.partner().strength.points, Range::FULL_POINTS);
}

/// The system's own choice at `auction` — the highest finite logit, book
/// and floor together (the in-crate twin of `examples/common::next_call`,
/// minus the legality filter: every call these tests expect is legal).
fn chosen_call(stance: &crate::bidding::Stance, hand: Hand, auction: &[Call]) -> Call {
    let (logits, _) = stance
        .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
        .expect("the Gladiator node classifies");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

/// Do we play the card we claim to play?
///
/// Our Gladiator (`set_nt_overcall_gladiator`) adapts the Crowborough card
/// — <https://www.bridgewebs.com/crowborough/NT%20Responses.htm> — from a
/// 1NT *opening* to our 1NT *overcall*, where `2♦` is natural and the cue
/// is Stayman, so the relay must also park the hands that card's `2♦`
/// Extended Stayman takes.  This replays the **bidder** (not the rule
/// table) over one representative hand per advance and per relay
/// continuation, so a floor that drifts under the structure shows up as a
/// red test rather than as a convention that quietly stops firing.
#[test]
fn gladiator_advances_follow_the_card() {
    crate::bidding::american::set_nt_overcall_gladiator(true);
    let stance = crate::american().against();
    let node = [bid(1, Strain::Spades), bid(1, Strain::Notrump), Call::Pass];
    // After the relay and its forced 2♦ puppet: the XYZ-style sort.
    let sorted: Vec<Call> = node
        .iter()
        .copied()
        .chain([
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
        ])
        .collect();

    // (hand, auction, expected call, what the hand is)
    let rows: &[(&str, &[Call], Call, &str)] = &[
        // Their major is ♠, so the one unbid major `o` is ♥ throughout.
        (
            "873.93.KJ973.T94",
            &node,
            bid(2, Strain::Clubs),
            "weak with 5+♦ — the relay's weak takeout arm",
        ),
        (
            "K872.Q93.J84.Q93",
            &node,
            bid(2, Strain::Clubs),
            "invitational, nothing to bid directly — the relay's INV arm",
        ),
        (
            "K3.Q876.KJ84.972",
            &node,
            bid(2, Strain::Spades),
            "INV with exactly 4♥, not 4333 — the cue, Stayman for ♥",
        ),
        (
            "K3.972.KJ864.Q93",
            &node,
            bid(2, Strain::Diamonds),
            "INV with exactly 5♦ — natural",
        ),
        (
            "93.KJ864.K73.Q92",
            &node,
            bid(2, Strain::Hearts),
            "INV with exactly 5♥ — natural",
        ),
        (
            "93.874.J6.KQ9764",
            &node,
            bid(2, Strain::Notrump),
            "weak with 6+♣ — the transfer to clubs",
        ),
        (
            "3.KQ86.AJ84.K976",
            &node,
            bid(3, Strain::Spades),
            "GF raise of ♥ with a singleton spade — the splinter",
        ),
        // The relay's continuations over the forced 2♦.
        (
            "873.93.KJ973.T94",
            &sorted,
            Call::Pass,
            "weak with ♦ — pass the puppet",
        ),
        (
            "93.KJ864.T73.972",
            &sorted,
            bid(2, Strain::Hearts),
            "weak with 5+♥ — the takeout",
        ),
        (
            "K872.Q93.J84.Q93",
            &sorted,
            bid(2, Strain::Notrump),
            "balanced INV (flat 4333: no delayed cue)",
        ),
        (
            "K872.Q93.KJ84.9",
            &sorted,
            bid(2, Strain::Spades),
            "INV with exactly 3♥, not 4333 — the delayed cue",
        ),
        (
            "932.7.QJ9764.KJ2",
            &sorted,
            bid(3, Strain::Diamonds),
            "INV with a good 6-card suit",
        ),
        // The relay's *third* arm — a game-forcing balanced hand with
        // exactly 3♥ — is authored but weight-shadowed: at 0.5 it loses to
        // `3NT` (1.2) and to the 3-level naturals (1.3), so no hand plays
        // it.  Deliberate (the box is too confined to adjudicate an A/B on),
        // and pinned here so the divergence is documented rather than
        // hidden: the arm is read, never played.
        (
            "K942.Q76.AJ83.K4",
            &node,
            bid(3, Strain::Notrump),
            "GF balanced with exactly 3♥ — arm 3 is shadowed by 3NT",
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for &(text, auction, expected, what) in rows {
        let hand: Hand = text.parse().expect("a hand");
        let made = chosen_call(&stance, hand, auction);
        if made != expected {
            failures.push(format!("{text} ({what}): bid {made}, carded {expected}"));
        }
    }
    crate::bidding::american::set_nt_overcall_gladiator(false);

    assert!(
        failures.is_empty(),
        "Gladiator diverges from the card:\n{}",
        failures.join("\n"),
    );
}

/// Every Gladiator reading admits the hand that actually made the call.
///
/// The behavioural analogue of `authored_rules_eval_within_projection`,
/// which cannot cover this table: that sweep walks the shipped tries, and
/// `gladiator_advances` is only in one when the knob is on.  It also covers
/// what no static sweep can — the hand-written stamps in the post-walk
/// block, which may narrow past what the rules promise (this test is what
/// caught the relay's `0..=9` band deleting the game-forcing box).
#[test]
fn gladiator_readings_admit_the_bidder() {
    use rand::SeedableRng as _;

    crate::bidding::american::set_nt_overcall_gladiator(true);
    set_envelope_union_reading(true);
    let stance = crate::american().against();
    let node = [bid(1, Strain::Spades), bid(1, Strain::Notrump), Call::Pass];

    let mut rng = rand::rngs::StdRng::seed_from_u64(0x61AD);
    let hands: Vec<Hand> = crate::bidding::verify::random_hands(&mut rng)
        .take(256)
        .collect();

    let mut failures: Vec<String> = Vec::new();
    // The advancer sits two seats back once a pass follows their call, so
    // `Relative::Partner` is the seat that just bid.
    //
    // Every advance, not just the ones `gladiator_reading` decodes: the
    // card's *natural* advances are read by the walk, and the walk used to
    // read the game-forcing `3♣`/`3♦`/`3O` — authored `len(suit, 5..)` — as
    // a weak six-card jump, excluding every five-card advancer from its own
    // box.  Fixed by teaching the walk that our 1NT *overcall* takes the
    // same three-level reading as an opening 1NT (`over_one_notrump`), and
    // pinned here so the two layers cannot drift apart again.
    let check = |failures: &mut Vec<String>, hand: Hand, auction: &[Call], made: Call| {
        let mut read: Vec<Call> = auction.to_vec();
        read.push(made);
        read.push(Call::Pass);
        let inferences = stance.infer(RelativeVulnerability::NONE, &read);
        if !inferences.admits(Relative::Partner, hand) && failures.len() < 16 {
            failures.push(format!(
                "[{}] reading excludes the hand that bid it: {hand}",
                read.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" "),
            ));
        }
    };

    // Both reading regimes.  Knob-on, the natural advances project their
    // authoring rule *on top of* the walk's reading, so a walk claim that
    // contradicts the rule empties the box instead of quietly overriding it
    // — the sweep is how `set_natural_reading` gets adjudicated per node.
    for natural in [false, true] {
        set_reading_scope(if natural {
            ReadingScope::All
        } else {
            ReadingScope::Alerted
        });
        for &hand in &hands {
            let made = chosen_call(&stance, hand, &node);
            check(&mut failures, hand, &node, made);
            // Relayers carry on through the forced 2♦ — the only route to
            // the delayed cue, whose stamp is the other narrowing one.
            if made != bid(2, Strain::Clubs) {
                continue;
            }
            let sorted: Vec<Call> = node
                .iter()
                .copied()
                .chain([
                    bid(2, Strain::Clubs),
                    Call::Pass,
                    bid(2, Strain::Diamonds),
                    Call::Pass,
                ])
                .collect();
            let continued = chosen_call(&stance, hand, &sorted);
            check(&mut failures, hand, &sorted, continued);
        }
        // The runout branch too — `[1♠, 1NT, (X)]` is authored, so its
        // escapes are read by the walk like any other natural call.
        let doubled = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Double,
        ];
        for &hand in &hands {
            let made = chosen_call(&stance, hand, &doubled);
            check(&mut failures, hand, &doubled, made);
        }
    }
    set_reading_scope(ReadingScope::Alerted);
    crate::bidding::american::set_nt_overcall_gladiator(false);

    assert!(
        failures.is_empty(),
        "Gladiator readings exclude their own bidders:\n{}",
        failures.join("\n"),
    );
}

/// Every reading admits the hand that actually made the call — the
/// table-driven regime-2 invariant of `docs/reading-drift-handoff.md`.
///
/// At each node the *bidder* is replayed over seeded hands and partner's
/// reading of the chosen call must admit the hand, in both reading regimes
/// — the only check that catches an authored-natural rule contradicting the
/// walk's shape-guess (`authored_rules_eval_within_projection` compares a
/// rule to *its own* projection and is blind to the walk).  Default knobs;
/// the knob-gated twin is `gladiator_readings_admit_the_bidder`.
///
/// A row lands **together with the repair that makes it green** — the
/// unrepaired queue lives in the handoff doc's ledger, not here.
#[test]
fn readings_admit_the_bidder() {
    use rand::SeedableRng as _;

    set_envelope_union_reading(true);
    let stance = crate::american().against();

    // (what the node is, the auction up to the seat replayed).  Multi-call
    // seats are route-filtered below: a hand counts only when replaying
    // the seat's *earlier* decisions reproduces the script, so the reading
    // of the whole lane is tested against hands that actually bid it.
    let nodes: &[(&str, &[Call])] = &[
        ("opening", &[]),
        ("second-seat opening", &[Call::Pass]),
        ("response to 1♠", &[bid(1, Strain::Spades), Call::Pass]),
        ("response to 1♥", &[bid(1, Strain::Hearts), Call::Pass]),
        // A raise of a preempt is two-way (furthering or to-make), so the
        // walk stamps no band and no support floor on it — the `1..=11`
        // cap used to exclude every to-make raiser of `[3♥ P 4♥]`.
        (
            "raise of a 3♥ preempt",
            &[bid(3, Strain::Hearts), Call::Pass],
        ),
        (
            "raise of a 3♠ preempt",
            &[bid(3, Strain::Spades), Call::Pass],
        ),
        ("raise of a weak 2♥", &[bid(2, Strain::Hearts), Call::Pass]),
        // Delayed preferences/raises of a shown 5-6 suit floor at two (the
        // false preference on Hx is the norm) — the blanket 3-card stamp
        // excluded 81% of the actual preference bidders.
        (
            "preference after forcing NT, 2♦ rebid",
            &[
                bid(1, Strain::Spades),
                Call::Pass,
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(2, Strain::Diamonds),
                Call::Pass,
            ],
        ),
        (
            "preference after forcing NT, 2♥ rebid",
            &[
                bid(1, Strain::Spades),
                Call::Pass,
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(2, Strain::Hearts),
                Call::Pass,
            ],
        ),
        (
            "raise of the jump rebid",
            &[
                bid(1, Strain::Spades),
                Call::Pass,
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(3, Strain::Spades),
                Call::Pass,
            ],
        ),
        (
            "raise of opener's rebid suit",
            &[
                bid(1, Strain::Hearts),
                Call::Pass,
                bid(1, Strain::Spades),
                Call::Pass,
                bid(2, Strain::Hearts),
                Call::Pass,
            ],
        ),
        // The XYZ 2M rebid is authored five-plus on both routes; the
        // walk's sixth-card stamp excluded every 5-carder.
        (
            "XYZ relay then 2♠ invite",
            &[
                bid(1, Strain::Diamonds),
                Call::Pass,
                bid(1, Strain::Spades),
                Call::Pass,
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(2, Strain::Clubs),
                Call::Pass,
                bid(2, Strain::Diamonds),
                Call::Pass,
            ],
        ),
        (
            "XYZ direct 2♠ sign-off",
            &[
                bid(1, Strain::Diamonds),
                Call::Pass,
                bid(1, Strain::Spades),
                Call::Pass,
                bid(1, Strain::Notrump),
                Call::Pass,
            ],
        ),
        // Post-transfer continuations fall under the notrump-structure
        // blanket — the artificial 2♦ used to count as a first diamond
        // bid, reading responder's 3♦ as a six-card rebid.
        (
            "responder's second suit after a transfer",
            &[
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(2, Strain::Diamonds),
                Call::Pass,
                bid(2, Strain::Hearts),
                Call::Pass,
            ],
        ),
        // The support double's `support(3..=3)` projects under the
        // bidder's at-the-time context — the reader-context skew used to
        // stamp the exactly-3 on the opened minor (100% exclusion).
        (
            "opener's support double",
            &[
                bid(1, Strain::Diamonds),
                Call::Pass,
                bid(1, Strain::Hearts),
                bid(1, Strain::Spades),
            ],
        ),
        // Cue raises: the same skew put the `support(n..)` atom on the
        // cue suit itself, excluding every cue-bidder over a minor.
        (
            "cue raise over their 1♠",
            &[bid(1, Strain::Hearts), bid(1, Strain::Spades)],
        ),
        (
            "cue raise over their 2♦",
            &[bid(1, Strain::Spades), bid(2, Strain::Diamonds)],
        ),
        (
            "cue raise over their 1♦",
            &[bid(1, Strain::Clubs), bid(1, Strain::Diamonds)],
        ),
        (
            "advance of our 1NT overcall (systems on)",
            &[bid(1, Strain::Spades), bid(1, Strain::Notrump), Call::Pass],
        ),
        (
            "runout of our doubled 1NT overcall (systems on)",
            &[
                bid(1, Strain::Spades),
                bid(1, Strain::Notrump),
                Call::Double,
            ],
        ),
    ];

    // The four 5-5-major witnesses that caught the strip's keyless re-read
    // (each bids the authored both-majors 3♦ off `points(8..)` on the
    // upgrade scale), then a random sweep.
    let mut rng = rand::rngs::StdRng::seed_from_u64(0x5EAD);
    let hands: Vec<Hand> = [
        "Q9632.AT985.T53.",
        "QJ862.96543.K5.Q",
        "KJT84.AQ653.87.T",
        "KQ853.T7542.9.QJ",
    ]
    .iter()
    .map(|text| text.parse().expect("a hand"))
    .chain(crate::bidding::verify::random_hands(&mut rng).take(256))
    .collect();

    let mut failures: Vec<String> = Vec::new();
    for natural in [false, true] {
        set_reading_scope(if natural {
            ReadingScope::All
        } else {
            ReadingScope::Alerted
        });
        for &(what, node) in nodes {
            for &hand in &hands {
                // Honest route only: the seat's earlier calls in the
                // script must be the ones this hand actually chooses.
                if (node.len() % 4..node.len())
                    .step_by(4)
                    .any(|i| chosen_call(&stance, hand, &node[..i]) != node[i])
                {
                    continue;
                }
                let made = chosen_call(&stance, hand, node);
                // After `made` and a pass, the seat to act is the bidder's
                // partner, so `Relative::Partner` is the seat replayed.
                let mut read: Vec<Call> = node.to_vec();
                read.push(made);
                read.push(Call::Pass);
                let inferences = stance.infer(RelativeVulnerability::NONE, &read);
                if !inferences.admits(Relative::Partner, hand) && failures.len() < 16 {
                    failures.push(format!(
                        "{what} [{}] (natural-reading {natural}) excludes the hand that bid it: {hand}",
                        read.iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" "),
                    ));
                }
            }
        }
    }
    set_reading_scope(ReadingScope::Alerted);

    assert!(
        failures.is_empty(),
        "readings exclude their own bidders:\n{}",
        failures.join("\n"),
    );
}

/// A doubled 1NT overcall runs out — it does not jump to the three level.
///
/// Gladiator turns off `systems_on_overcall_strip`, which is what let the
/// floor read `[1M, 1NT, X]` as a doubled *opening* 1NT.  Without it the
/// distilled net escaped a 1-count to `3♥`; `gladiator_doubled_runout` is
/// the book node that shadows it.
#[test]
fn gladiator_runs_out_of_the_doubled_overcall() {
    crate::bidding::american::set_nt_overcall_gladiator(true);
    let stance = crate::american().against();
    let node = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Double,
    ];

    // (hand, expected, what it is)
    let rows: &[(&str, Call, &str)] = &[
        ("873.93.KJ973.T94", bid(2, Strain::Diamonds), "bust, 5♦"),
        ("93.KJ864.T73.972", bid(2, Strain::Hearts), "bust, 5♥"),
        ("93.874.J6.KQ9764", bid(2, Strain::Clubs), "bust, 6♣"),
        (
            "8732.932.J973.T4",
            Call::Pass,
            "1-count, no five-bagger: sit",
        ),
        (
            "T9843.93.J973.T4",
            Call::Pass,
            "bust with five of THEIR major: sit, never run into it",
        ),
        ("K872.Q93.J84.Q93", Call::Redouble, "values: play 1NT××"),
    ];

    let mut failures: Vec<String> = Vec::new();
    for &(text, expected, what) in rows {
        let hand: Hand = text.parse().expect("a hand");
        let made = chosen_call(&stance, hand, &node);
        if made != expected {
            failures.push(format!("{text} ({what}): bid {made}, carded {expected}"));
        }
    }
    crate::bidding::american::set_nt_overcall_gladiator(false);

    assert!(
        failures.is_empty(),
        "the doubled 1NT overcall misplays its runout:\n{}",
        failures.join("\n"),
    );
}

/// `set_natural_reading` publishes what an unalerted authored rule promises.
///
/// `gladiator_advances` authors the game-forcing `3♦` as
/// `len(♦, 5..) & points(game..)`.  It is natural, so it carries no alert and
/// the projection pass skips it: the walk supplies a length floor and the
/// game force is simply lost.  Knob-on the rule's own box is intersected in.
#[test]
fn natural_reading_publishes_an_unalerted_rules_promise() {
    crate::bidding::american::set_nt_overcall_gladiator(true);
    set_envelope_union_reading(true);
    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(3, Strain::Diamonds),
        Call::Pass,
    ];

    set_reading_scope(ReadingScope::Alerted);
    let off = read_booked(&auction);
    set_reading_scope(ReadingScope::All);
    let on = read_booked(&auction);
    set_reading_scope(ReadingScope::Alerted);
    crate::bidding::american::set_nt_overcall_gladiator(false);

    assert_eq!(
        off.partner().strength.points,
        Range::FULL_POINTS,
        "knob-off the game force is unread"
    );
    assert!(
        on.partner().strength.points.min >= 10,
        "knob-on the rule's `points(game..)` reaches the reading, got {:?}",
        on.partner().strength.points,
    );
    // The walk's natural reading survives: the call is not suppressed, so
    // the diamond suit is still read from the auction, not only from the box.
    assert!(on.partner().length(Suit::Diamonds).min >= 5);
}

/// Every Gladiator continuation ends where the card says, not where the
/// floor guesses.
///
/// Authoring a node **shadows** the floor, so this sweep is also the record
/// of what is deliberately *not* authored: every "advancer passes the game
/// opposite a limited hand" leaf below is answered by the floor and answered
/// right, and a bare `Pass` node there would only cost the floor its slam
/// machinery.  The three that are authored are the ones the floor got wrong
/// — it raised a weak signoff on three trumps, bid `3NT` opposite a hand
/// that had denied 8 points, and answered Leaping Michaels `4♣` with `5NT`.
#[test]
fn gladiator_continuations_are_authored_to_the_leaf() {
    crate::bidding::american::set_nt_overcall_gladiator(true);
    let stance = crate::american().against();
    let p = Call::Pass;
    let base = [bid(1, Strain::Spades), bid(1, Strain::Notrump), p];
    let seq =
        |tail: &[Call]| -> Vec<Call> { base.iter().copied().chain(tail.iter().copied()).collect() };
    let relay = bid(2, Strain::Clubs);
    let forced = bid(2, Strain::Diamonds);

    // (auction, hand, expected, what)
    let rows: Vec<(Vec<Call>, &str, Call, &str)> = vec![
        // --- authored: the floor was wrong here ---
        (
            seq(&[relay, p, forced, p, bid(2, Strain::Hearts), p]),
            "AQ8.AK9.Q852.A93",
            p,
            "16 with three hearts: pass the weak signoff (floor raised)",
        ),
        (
            seq(&[relay, p, forced, p, bid(2, Strain::Hearts), p]),
            "AQ86.AKJ.Q85.A93",
            p,
            "17 with three hearts: pass (floor bid 3NT opposite a bust)",
        ),
        (
            seq(&[relay, p, forced, p, bid(2, Strain::Hearts), p]),
            "AQ8.AKJ2.Q85.A9",
            bid(3, Strain::Hearts),
            "18 with four hearts: the one sound push",
        ),
        (
            seq(&[bid(4, Strain::Clubs), p]),
            "AQ8.AK9.Q852.A93",
            bid(4, Strain::Hearts),
            "Leaping 4♣ (5-5 hearts+clubs GF), three-card fit (floor bid 5NT)",
        ),
        (
            seq(&[bid(4, Strain::Diamonds), p]),
            "AQ86.AKJ.Q85.A93",
            bid(4, Strain::Hearts),
            "Leaping 4♦, three-card fit",
        ),
        (
            seq(&[bid(4, Strain::Spades), p]),
            "AQ8.AK9.Q852.A93",
            bid(5, Strain::Diamonds),
            "Leaping 4♠ (both minors), diamonds the longer",
        ),
        // --- deliberately left to the floor, and it answers right ---
        (
            seq(&[bid(2, Strain::Notrump), p, bid(3, Strain::Clubs), p]),
            "93.874.J6.KQ9764",
            p,
            "weak club transfer completed: pass",
        ),
        (
            seq(&[forced, p, bid(3, Strain::Notrump), p]),
            "K3.972.KJ864.Q93",
            p,
            "invitational 2♦ accepted to 3NT: pass",
        ),
        (
            seq(&[bid(2, Strain::Hearts), p, bid(4, Strain::Hearts), p]),
            "93.KJ864.K73.Q92",
            p,
            "invitational 2♥ raised to game: pass",
        ),
        (
            seq(&[
                relay,
                p,
                forced,
                p,
                bid(2, Strain::Notrump),
                p,
                bid(3, Strain::Notrump),
                p,
            ]),
            "K872.Q93.J84.Q93",
            p,
            "balanced invitation accepted: pass",
        ),
        (
            seq(&[bid(3, Strain::Spades), p, bid(4, Strain::Hearts), p]),
            "3.KQ86.AJ84.K976",
            p,
            "splinter raised to game: pass",
        ),
        (
            seq(&[bid(3, Strain::Diamonds), p, bid(3, Strain::Notrump), p]),
            "KQT.K8.AJT64.QJ4",
            p,
            "game-forcing 3♦ placed in 3NT: pass",
        ),
    ];

    let mut failures: Vec<String> = Vec::new();
    for (auction, text, expected, what) in rows {
        let hand: Hand = text.parse().expect("a hand");
        let made = chosen_call(&stance, hand, &auction);
        if made != expected {
            failures.push(format!("{text} ({what}): bid {made}, wanted {expected}"));
        }
    }
    crate::bidding::american::set_nt_overcall_gladiator(false);

    assert!(
        failures.is_empty(),
        "Gladiator continuations land in the wrong place:\n{}",
        failures.join("\n"),
    );
}

/// Gladiator keeps the systems-on strip where it has no structure of its own.
///
/// Over RHO's **X** and over 3-level-or-higher interference, Gladiator and
/// systems-on play the same auction (a natural runout, then the floor), so
/// the strip identity still holds and the inference-aware floor keeps the
/// picture it was distilled on.  Over a pass or a 2-level bid it does not —
/// the advances, the stolen relay and Transfer Lebensohl all diverge.
#[test]
fn gladiator_keeps_the_strip_where_it_has_no_structure() {
    crate::bidding::american::set_nt_overcall_gladiator(true);
    let p = Call::Pass;
    let one_s = bid(1, Strain::Spades);
    let one_nt = bid(1, Strain::Notrump);
    // (auction after [1♠, 1NT], stripped?)
    let rows: &[(&[Call], bool, &str)] = &[
        (&[Call::Double], true, "their X — a runout in both systems"),
        (
            &[bid(3, Strain::Clubs)],
            true,
            "3-level — the floor in both",
        ),
        (
            &[bid(4, Strain::Hearts)],
            true,
            "4-level — the floor in both",
        ),
        (&[], false, "quiet — the Gladiator advances"),
        (&[p], false, "quiet — the Gladiator advances"),
        (
            &[bid(2, Strain::Clubs)],
            false,
            "their 2♣ — the stolen relay",
        ),
        (
            &[bid(2, Strain::Hearts)],
            false,
            "their 2♥ — Transfer Lebensohl",
        ),
    ];
    let mut failures: Vec<String> = Vec::new();
    for &(tail, want, what) in rows {
        let auction: Vec<Call> = [one_s, one_nt]
            .into_iter()
            .chain(tail.iter().copied())
            .collect();
        let got = super::systems_on_overcall_strip(&auction).is_some();
        if got != want {
            failures.push(format!("{what}: stripped = {got}, wanted {want}"));
        }
    }
    crate::bidding::american::set_nt_overcall_gladiator(false);
    assert!(
        failures.is_empty(),
        "strip scope wrong:\n{}",
        failures.join("\n")
    );
}

#[test]
fn gladiator_contested_transfer_lebensohl_pins_the_target() {
    // [1♠, 1NT, (2♥), 3♦, P]: over RHO's 2♥ there is no room for the relay
    // tree, so advancer plays Transfer Lebensohl; 3♦ transfers up through their
    // hearts (showing spades), read via the builders' alerts — opener must not
    // raise a phantom diamond suit.
    crate::bidding::american::set_nt_overcall_gladiator(true);
    let auction = [
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        bid(2, Strain::Hearts),
        bid(3, Strain::Diamonds),
        Call::Pass,
    ];
    let inf = read_booked(&auction);
    crate::bidding::american::set_nt_overcall_gladiator(false);
    assert!(
        inf.partner().length(Suit::Spades).min >= 5,
        "transfer target pinned"
    );
    assert!(
        inf.partner().length(Suit::Diamonds).min < 5,
        "phantom suit not read"
    );
}

#[test]
fn completed_major_transfer_shows_five() {
    // [1NT, P, 2♦, P, 2♥, P]: partner transferred to hearts and we
    // completed; at length 6 the responder is us (Me).  The transfer shows a
    // five-card major even before a jump confirms the sixth, while the
    // transferred-*from* suit stays unread.
    let auction = [
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read_booked(&auction);
    assert_eq!(inf.me().length(Suit::Hearts), Range::new(5, 13));
    assert_eq!(inf.me().length(Suit::Diamonds), Range::FULL_LENGTH);
}

#[test]
fn transfer_jump_to_game_shows_at_least_five() {
    // [1NT, P, 2♦, P, 2♥, P, 4♥, P]: partner transferred then jumped to 4♥.
    // The projection reads the 2♦ transfer's authored rule — a five-card floor;
    // the old reader's six-card upgrade off the jump is dropped (soundness over
    // tightness, M6.2c).  At length 8 the responder sits as Partner.
    let auction = [
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(4, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read_booked(&auction);
    assert_eq!(inf.partner().length(Suit::Hearts), Range::new(5, 13));
}

#[test]
fn transfer_then_three_major_shows_at_least_five() {
    // [1NT, P, 2♦, P, 2♥, P, 3♥, P]: a raise of the transferred suit.  The
    // projection pins the transfer's five-card floor; the old reader's six-card
    // upgrade and the 8–9 invitational points are dropped (soundness over
    // tightness, M6.2c).
    let auction = [
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read_booked(&auction);
    assert!(inf.partner().length(Suit::Hearts).min >= 5);
}

#[test]
fn transfer_projection_covers_spades_and_two_notrump() {
    // Spade transfer (2♥ → 2♠) jumped to 4♠: the 2♥ transfer rule projects a
    // five-card spade floor.
    let spades = read_booked(&[
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Spades),
        Call::Pass,
        bid(4, Strain::Spades),
        Call::Pass,
    ]);
    assert_eq!(spades.partner().length(Suit::Spades), Range::new(5, 13));

    // The same shape over a 2NT opening (3♦ → 3♥, jump 4♥).
    let two_nt = read_booked(&[
        bid(2, Strain::Notrump),
        Call::Pass,
        bid(3, Strain::Diamonds),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
        bid(4, Strain::Hearts),
        Call::Pass,
    ]);
    assert_eq!(two_nt.partner().length(Suit::Hearts), Range::new(5, 13));
}

#[test]
fn contested_transfer_auction_is_not_specially_read() {
    // [1NT, 2♣, 2♦, P, 2♥, P, 4♥, P]: with the opponents in, the transfer
    // positions shift, so the special reading must not pin a six-card suit.
    let auction = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Clubs),
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(4, Strain::Hearts),
        Call::Pass,
    ];
    let inf = read(&auction);
    assert!(inf.partner().length(Suit::Hearts).min < 6);
}

#[test]
fn contested_transfer_lebensohl_reads_the_target_under_intervention() {
    // Board 881510: [1NT, (2♠), 3♦, (3♠)] — responder's 3♦ is a Transfer-
    // Lebensohl transfer to hearts (up the line through their spade suit).  RHO's
    // (3♠) skips opener's completion node; the default-on fallback projection
    // re-resolves 3♦'s authoring rule and pins hearts, so opener does not read it
    // as natural diamonds and raise the phantom suit to 5♦x.  Needs the prefixed
    // `read_booked` (the projection reads the rule off the book).
    let auction = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Spades),
        bid(3, Strain::Diamonds),
        bid(3, Strain::Spades),
    ];
    let inf = read_booked(&auction);
    assert!(
        inf.partner().length(Suit::Hearts).min >= 5,
        "transfer target pinned"
    );
    assert!(
        inf.partner().length(Suit::Diamonds).min < 5,
        "phantom suit not read"
    );
}

#[test]
fn fallback_projection_decodes_contested_leaping_michaels() {
    // [1NT, (2♦), 4♦, (P)]: Leaping Michaels = both majors 5-5, authored as a
    // *guarded fallback* in the (2♦) Transfer block — invisible to the exact-node
    // projection, and with no hand reader.  The default-on fallback projection
    // re-resolves its authoring rule and pins both majors (no reader involved).
    let auction = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Diamonds),
        bid(4, Strain::Diamonds),
        Call::Pass,
    ];
    let inf = read_booked(&auction);
    assert!(
        inf.partner().length(Suit::Hearts).min >= 5 && inf.partner().length(Suit::Spades).min >= 5,
        "fallback projection pins both majors for contested Leaping Michaels"
    );
}

/// The §7.3.1 union poison (docs/ai-bidder/bba-kickback.md): with
/// `set_kickback` on, the relocated-ask and answer rules on 4♥/4♠ were
/// structurally alerted, so a **natural** 4♠'s box was unioned with the
/// ask's ⊤ projection — partner's `length(Spades).min` collapsed to 0 and
/// the natural walk's lane bookkeeping was suppressed on top.  The face
/// gate makes those rules as-if-absent on faces where `kickback_ladder`
/// claims nothing (here no suit is bid twice by one side, so the ladder is
/// all-`None`): the knob-on reading must equal the knob-off one.
#[test]
fn kickback_face_gate_keeps_natural_four_spades_natural() {
    use crate::bidding::instinct::{RkcbVariant, set_rkcb_variant};
    // The audited C−B shape: 1♦ P 1♠ P 2♦ P 4♠ P — the reader is the
    // opener, partner is the natural 4♠ bidder.
    let auction = [
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(4, Strain::Spades),
        Call::Pass,
    ];
    let baseline = read_booked(&auction).partner().length(Suit::Spades).min;
    set_rkcb_variant(RkcbVariant::Kickback);
    let gated = read_booked(&auction).partner().length(Suit::Spades).min;
    set_rkcb_variant(RkcbVariant::Plain); // restore the default (off) for the rest of the suite
    assert!(baseline >= 4, "the natural walk floors responder's spades");
    assert_eq!(gated, baseline, "kickback must not erase the natural floor");
}

/// The face gate's positive control: where the ladder *does* claim the
/// call (hearts agreed, spades unguarded → 4♠ asks), the rule stays live —
/// alerted, so the ask is not read as a natural spade suit.
#[test]
fn kickback_relocated_ask_still_reads_as_the_convention() {
    use crate::bidding::instinct::{RkcbVariant, set_rkcb_variant};
    let auction = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
        bid(4, Strain::Spades),
        Call::Pass,
    ];
    set_rkcb_variant(RkcbVariant::Kickback);
    let spades = read_booked(&auction).partner().length(Suit::Spades).min;
    set_rkcb_variant(RkcbVariant::Plain); // restore the default (off) for the rest of the suite
    assert!(spades < 4, "the relocated ask is not a natural spade suit");
}

/// The default-system twin of the kickback poison: the plain 1430 answers
/// (5♣–5♠) and DOPI/ROPI/DEPO on X/XX are present in every stance and
/// always alerted, so a **natural** floor 5♦ — no ask anywhere on the
/// face — reads as a keycard answer: the union with the answer rules' ⊤
/// projection erases partner's diamond floor and the `alerted` bit
/// suppresses the natural walk.  The `Rules::face` gates confine the
/// rules to a live ask window, so the natural reading survives.
///
/// This was a differential test against `set_keycard_answer_gates`.  That
/// knob is gone — its off arm was the poison itself, not an agreement any
/// partnership could play — so the guard is now absolute: partner's
/// diamond floor must not be erased.  Remove the gates and it goes to
/// nothing, which is exactly the regression being pinned.
#[test]
fn answer_gates_spare_a_natural_five_diamonds() {
    use crate::bidding::instinct::{RkcbVariant, set_rkcb_variant};
    // The plain arm on purpose (also the default): the poison this pins is
    // the *default system's* five-level answers, not the relocated
    // ladder's.
    set_rkcb_variant(RkcbVariant::Plain);
    let auction = [
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Diamonds),
        Call::Pass,
        bid(5, Strain::Diamonds),
        Call::Pass,
    ];
    let diamonds = read_booked(&auction).partner().length(Suit::Diamonds).min;
    set_rkcb_variant(RkcbVariant::Plain); // restore the default (off) for the rest of the suite
    assert!(
        diamonds >= 2,
        "a natural 5♦ with no ask anywhere on the face must keep its \
         diamond floor, got {diamonds}"
    );
}

/// The gates' positive control: inside a live ask window the answer is
/// still alerted — a 5♦ answering 4NT is a keycard count, not diamonds.
#[test]
fn answer_gates_keep_the_live_window_alerted() {
    let auction = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
        bid(4, Strain::Notrump),
        Call::Pass,
        bid(5, Strain::Diamonds),
        Call::Pass,
    ];
    // The gates are the default: the in-window answer must stay alerted.
    let diamonds = read_booked(&auction).partner().length(Suit::Diamonds).min;
    assert!(
        diamonds < 4,
        "the in-window answer is not a natural diamond suit"
    );
}

#[test]
fn contested_transfer_lebensohl_direct_jacoby_over_2d() {
    // Over (2♦) the transfers are direct Jacoby: 3♦→♥.  [1NT, (2♦), 3♦, (X)].
    let auction = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Diamonds),
        bid(3, Strain::Diamonds),
        Call::Double,
    ];
    let inf = read_booked(&auction);
    assert!(inf.partner().length(Suit::Hearts).min >= 5);
}

#[test]
fn contested_transfer_lebensohl_cue_is_not_a_transfer() {
    // The cue of their suit is Stayman (a 4-card unbid major), not a 5+ transfer:
    // [1NT, (2♠), 3♠, (P)] projects hearts as only 4-card interest, and the
    // natural-spades reading of the cue is suppressed (not a long spade suit).
    let auction = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Spades),
        bid(3, Strain::Spades),
        Call::Pass,
    ];
    let inf = read_booked(&auction);
    assert!(inf.partner().length(Suit::Hearts).min < 5);
    assert!(inf.partner().length(Suit::Spades).min < 5);
}

#[test]
fn relative_seat_tracks_the_actor() {
    // The same 1♥ opening lands on a different relative seat as the
    // auction grows by one call.
    assert_eq!(
        read(&[bid(1, Strain::Hearts)]).rho().strength.points,
        Range::new(10, 21)
    );
    assert_eq!(
        read(&[bid(1, Strain::Hearts), Call::Pass])
            .partner()
            .strength
            .points,
        Range::new(10, 21)
    );
}

#[test]
fn limited_notrump_rebids_narrow_strength() {
    // [1♦, P, 1♥, P, 1NT, P]: the opener (partner) showed a 12–16 minimum.
    let one_nt = read(&[
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(1, Strain::Notrump),
        Call::Pass,
    ]);
    assert_eq!(one_nt.partner().strength.points, Range::new(12, 16));

    // A jump to 2NT is the strong 18–19 rebid (sound bound 18–21).
    let two_nt = read(&[
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Notrump),
        Call::Pass,
    ]);
    assert_eq!(two_nt.partner().strength.points, Range::new(18, 21));
}

#[test]
fn cheapest_two_notrump_over_a_response_is_not_strong() {
    // [1♦, P, 2♣, P, 2NT, P]: 2NT is the *cheapest* notrump over a 2/1, a
    // minimum — it must not be read as the 18–19 jump.  Opener stays at the
    // opening floor (10–21).
    let inf = read(&[
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(2, Strain::Notrump),
        Call::Pass,
    ]);
    assert_eq!(inf.partner().strength.points, Range::new(10, 21));
}

#[test]
fn raises_and_one_notrump_response_narrow_the_responder() {
    // [1♥, P, 2♥, P]: a single raise is 6–10 — a support-scale band, so
    // the dedicated gauge carries it exactly and the legacy axis holds
    // only its sound image (4-point shapely raises are measured fact:
    // the `1♠ P 2♠` divergence-meter defect).
    let single = read(&[
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
    ]);
    let hearts = Suit::Hearts as usize;
    assert_eq!(
        single.partner().strength.support_points[hearts],
        Range::new(6, 10)
    );
    assert_eq!(single.partner().strength.points, Range::new(1, 11));
    assert_eq!(single.partner().strength.shown_floor(), 6);
    // [1♥, P, 3♥, P]: a limit (jump) raise is 10–12.
    let limit = read(&[
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(3, Strain::Hearts),
        Call::Pass,
    ]);
    assert_eq!(
        limit.partner().strength.support_points[hearts],
        Range::new(10, 12)
    );
    assert_eq!(limit.partner().strength.points, Range::new(5, 13));
    // [1♥, P, 1NT, P]: a 1NT response is 6–12.
    let one_nt = read(&[
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(1, Strain::Notrump),
        Call::Pass,
    ]);
    assert_eq!(one_nt.partner().strength.points, Range::new(6, 12));
}

#[test]
fn competition_suppresses_the_limited_rebid_reading() {
    // [1♦, P, 1♥, 1♠, 1NT, P]: with the opponents in, opener's 1NT is not
    // the quiet 12–16 rebid — leave the strength at the opening floor
    // (10–21).
    let inf = read(&[
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Hearts),
        bid(1, Strain::Spades),
        bid(1, Strain::Notrump),
        Call::Pass,
    ]);
    assert_eq!(inf.partner().strength.points, Range::new(10, 21));
}

#[test]
fn rubens_cue_raise_shows_support() {
    // (1♠) 2♣ (P) 2♠ (P): we overcalled 2♣, partner cue-raised 2♠ — a
    // limit-plus club raise.  The overcaller reads three-plus clubs and
    // ten-plus points, but no spade length (the cue is a relay).
    let inf = read(&[
        bid(1, Strain::Spades),
        bid(2, Strain::Clubs),
        Call::Pass,
        bid(2, Strain::Spades),
        Call::Pass,
    ]);
    assert!(inf.partner().length(Suit::Clubs).min >= 3);
    // A support-scale promise: exact on the club slot, only its sound
    // image on the legacy axis.
    assert!(inf.partner().strength.support_points[Suit::Clubs as usize].min >= 10);
    assert!(inf.partner().strength.points.min >= 5);
    assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
}

#[test]
fn rubens_transfer_is_not_read_as_natural() {
    // (1♣) 1♠ (P) 2♣ (P): we overcalled 1♠, partner transferred 2♣ (a relay
    // to diamonds).  The bid suit must not be read as a club holding.
    let inf = read(&[
        bid(1, Strain::Clubs),
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
    ]);
    assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
}

#[test]
fn rubens_reading_respects_the_knob() {
    // With Rubens advances off — the default since the layer A/B — the same
    // 2♣ is a genuine club suit: the suppression lifts and it reads naturally.
    crate::bidding::instinct::set_rubens_advances(false);
    set_cue_reading(false);
    let inf = read(&[
        bid(1, Strain::Clubs),
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
    ]);
    assert!(inf.partner().length(Suit::Clubs).min >= 4);
    set_cue_reading(true);
}

#[test]
fn their_minor_cue_reads_as_michaels() {
    // (1♣) 2♣: the direct cue of their minor opening is Michaels — both
    // majors, five-five, and no club length (the probe caught a club void
    // read as five clubs).  Off, the old overcall reading returns.
    set_cue_reading(true);
    let inf = read(&[bid(1, Strain::Clubs), bid(2, Strain::Clubs)]);
    assert_eq!(inf.rho().length(Suit::Clubs), Range::FULL_LENGTH);
    assert!(inf.rho().length(Suit::Hearts).min >= 5);
    assert!(inf.rho().length(Suit::Spades).min >= 5);
    set_cue_reading(false);
    let off = read(&[bid(1, Strain::Clubs), bid(2, Strain::Clubs)]);
    assert!(off.rho().length(Suit::Clubs).min >= 5);
    set_cue_reading(true);
}

#[test]
fn their_jump_cue_over_a_weak_two_is_leaping_michaels() {
    // (2♦) 4♦: the jump cue of a weak-two minor is Leaping Michaels — both
    // majors, no diamond length (the probe: a diamond void read as six).
    set_cue_reading(true);
    let inf = read(&[
        Call::Pass,
        bid(2, Strain::Diamonds),
        bid(4, Strain::Diamonds),
    ]);
    assert_eq!(inf.rho().length(Suit::Diamonds), Range::FULL_LENGTH);
    assert!(inf.rho().length(Suit::Hearts).min >= 5);
    assert!(inf.rho().length(Suit::Spades).min >= 5);
}

#[test]
fn their_cue_of_our_overcall_is_a_raise() {
    // 1♥ (2♦) 3♦: responder's cue of the overcalled suit is the limit-plus
    // heart raise — three-plus hearts, ten-plus points, and no diamond
    // length (the probe: two diamonds read as four).
    set_cue_reading(true);
    let inf = read(&[
        Call::Pass,
        Call::Pass,
        bid(1, Strain::Hearts),
        bid(2, Strain::Diamonds),
        bid(3, Strain::Diamonds),
    ]);
    assert_eq!(inf.rho().length(Suit::Diamonds), Range::FULL_LENGTH);
    assert!(inf.rho().length(Suit::Hearts).min >= 3);
    assert!(inf.rho().strength.support_points[Suit::Hearts as usize].min >= 10);
    assert!(inf.rho().strength.points.min >= 5);
}

#[test]
fn a_doublers_jump_is_not_a_weak_jump() {
    // 2♠ (X) P (3♦) P (4♥): the doubler's jump to game is strength, made
    // on as few as three hearts — never a weak six-card jump.
    set_length_soundness(true);
    let auction = [
        bid(2, Strain::Spades),
        Call::Double,
        Call::Pass,
        bid(3, Strain::Diamonds),
        Call::Pass,
        bid(4, Strain::Hearts),
    ];
    let inf = read(&auction);
    assert_eq!(inf.rho().length(Suit::Hearts), Range::FULL_LENGTH);
    set_length_soundness(false);
    let off = read(&auction);
    assert!(off.rho().length(Suit::Hearts).min >= 6);
    set_length_soundness(true);
}

#[test]
fn an_agreed_suit_re_raise_adds_no_length() {
    // 1♥ (P) 2♥ (P) 3♥: opener's game-try re-raise of the agreed suit adds
    // no length — the five from the opening stands, not a phantom sixth.
    set_length_soundness(true);
    let auction = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        bid(3, Strain::Hearts),
    ];
    let inf = read(&auction);
    assert_eq!(inf.rho().length(Suit::Hearts).min, 5);
    set_length_soundness(false);
    let off = read(&auction);
    assert_eq!(off.rho().length(Suit::Hearts).min, 6);
    set_length_soundness(true);
}

#[test]
fn opener_minor_rebid_reads_five_plus() {
    // 1♦ (P) 1♠ (P) 2♦: opener's two-level rebid of the opened minor is
    // routinely a good five-card suit, not six (the probe: five of eight
    // rebids were made on five).
    set_length_soundness(true);
    let auction = [
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Diamonds),
    ];
    let inf = read(&auction);
    assert_eq!(inf.rho().length(Suit::Diamonds).min, 5);
    set_length_soundness(false);
    let off = read(&auction);
    assert_eq!(off.rho().length(Suit::Diamonds).min, 6);
    set_length_soundness(true);
}

#[test]
fn their_splinter_is_disclosed_to_the_table() {
    // 1♠ (P) 4♦ read by a defender: their splinter is alerted and
    // explained at the table, so it decodes off their authoring rule —
    // diamond shortness with spade support, never diamond length.
    set_table_alert_reading(true);
    let auction = [bid(1, Strain::Spades), Call::Pass, bid(4, Strain::Diamonds)];
    let inf = read_booked(&auction);
    assert!(inf.rho().length(Suit::Diamonds).max <= 1);
    set_table_alert_reading(false);
    let off = read_booked(&auction);
    assert_eq!(off.rho().length(Suit::Diamonds).max, 13);
}

#[test]
fn their_michaels_is_disclosed_to_the_table() {
    // 1♠ (2♠) read by the opening side: their Michaels cue resolves in
    // *their* phase-routed book (defensive at their turn) and decodes off
    // the authored rule — five-plus hearts *with the rule's strength
    // floor*, which the retired `two_suiter_reading` never knew (chop 1,
    // `docs/reader-retirement.md`).  This knob is now the only owner of
    // the reading, so its off arm is the honest record of what the
    // retirement gives up: the shape floor goes too.
    set_table_alert_reading(true);
    let auction = [bid(1, Strain::Spades), bid(2, Strain::Spades)];
    let inf = read_booked(&auction);
    assert!(inf.rho().length(Suit::Hearts).min >= 5);
    assert!(inf.rho().strength.points.min >= 8);
    assert_eq!(inf.rho().length(Suit::Spades).min, 0);
    set_table_alert_reading(false);
    let off = read_booked(&auction);
    assert_eq!(off.rho().strength.points.min, 0);
    assert_eq!(off.rho().length(Suit::Hearts).min, 0);
    set_table_alert_reading(true);
}

#[test]
fn their_checkback_is_disclosed_to_the_table() {
    // 1♦ (P) 1♠ (P) 1NT (P) 2♣ read by a defender: their artificial
    // checkback 2♣ promises no clubs — the natural walk floored four (the
    // probe: four-plus clubs read on a singleton).
    set_table_alert_reading(true);
    let auction = [
        bid(1, Strain::Diamonds),
        Call::Pass,
        bid(1, Strain::Spades),
        Call::Pass,
        bid(1, Strain::Notrump),
        Call::Pass,
        bid(2, Strain::Clubs),
    ];
    let inf = read_booked(&auction);
    assert!(inf.rho().length(Suit::Clubs).min < 4);
    set_table_alert_reading(false);
    let off = read_booked(&auction);
    assert!(off.rho().length(Suit::Clubs).min >= 4);
    set_table_alert_reading(true);
}

#[test]
fn rubens_limit_raise_transfer_records_support() {
    crate::bidding::instinct::set_rubens_advances(true);
    // (1♣) 1♠ (P) 2♥ (P): partner's transfer into our spades is the
    // limit-plus raise — the overcaller reads three-plus spades and
    // ten-plus points, while the named hearts stay unread (a relay).
    let inf = read(&[
        bid(1, Strain::Clubs),
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
    ]);
    assert!(inf.partner().length(Suit::Spades).min >= 3);
    assert!(inf.partner().strength.points.min >= 10);
    assert_eq!(inf.partner().length(Suit::Hearts), Range::FULL_LENGTH);
}

#[test]
fn rubens_new_suit_transfer_records_the_target() {
    crate::bidding::instinct::set_rubens_advances(true);
    // (1♣) 1♠ (P) 2♣ (P): the new-suit transfer shows the advancer's own
    // five-card diamond suit and ten-plus points; clubs stay unread.
    let inf = read(&[
        bid(1, Strain::Clubs),
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Clubs),
        Call::Pass,
    ]);
    assert!(inf.partner().length(Suit::Diamonds).min >= 5);
    assert!(inf.partner().strength.points.min >= 10);
    assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
}

#[test]
fn rubens_transfer_records_despite_intervention() {
    crate::bidding::instinct::set_rubens_advances(true);
    // (1♣) 1♠ (P) 2♥ (X): opener doubles the transfer — the completion
    // never comes, but the shown limit raise is exactly what the
    // overcaller needs for the competitive decision.
    let inf = read(&[
        bid(1, Strain::Clubs),
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Double,
    ]);
    assert!(inf.partner().length(Suit::Spades).min >= 3);
    assert!(inf.partner().strength.points.min >= 10);
}

#[test]
fn rubens_transfer_is_not_read_for_the_opponents() {
    // Same auction read from the opening side (the advancer is now our
    // LHO): the opponents' agreement is not assumed — an in-band advance
    // from the other side may be a genuine suit, so nothing is recorded.
    let inf = read(&[
        bid(1, Strain::Clubs),
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
        Call::Pass,
    ]);
    assert_eq!(inf.lho().length(Suit::Spades), Range::FULL_LENGTH);
    assert_eq!(inf.lho().strength.points, Range::FULL_POINTS);
}

/// Their Michaels cue of our opened major, post-retirement (chop 1)
///
/// The reading now comes from the authored `.alert(MICHAELS)` rule's own
/// projection, so the auction must be read **keyed** (`read_booked`) and
/// the knob that owns the reading is `set_table_alert_reading`, not
/// `set_uvu_over_majors` (which kept only its book half).  The projection
/// also carries the rule's strength floor, which the retired reader never
/// did.
#[test]
fn michaels_cue_over_our_major_reads_the_other_major() {
    // [1♥, (2♥)]: their direct cue of our opened major is Michaels — 5+
    // spades with the rule's 8+ floor, and NOT a natural heart suit (the
    // walk's misread suppressed by the alert).
    let inf = read_booked(&[bid(1, Strain::Hearts), bid(2, Strain::Hearts)]);
    assert!(inf.rho().length(Suit::Spades).min >= 5, "the shown major");
    assert!(inf.rho().strength.points.min >= 8, "the rule's floor");
    assert_eq!(
        inf.rho().length(Suit::Hearts),
        Range::FULL_LENGTH,
        "the cue is not natural hearts"
    );

    // Table-wide disclosure and the shipped cue reading both off: the
    // pre-package natural reading is preserved verbatim.
    set_table_alert_reading(false);
    set_cue_reading(false);
    let inf = read_booked(&[bid(1, Strain::Hearts), bid(2, Strain::Hearts)]);
    assert!(inf.rho().length(Suit::Hearts).min >= 5);
    assert_eq!(inf.rho().length(Suit::Spades), Range::FULL_LENGTH);
    set_cue_reading(true);
    set_table_alert_reading(true);
}

/// Their unusual `(2NT)` over our major, post-retirement (chop 1) — as
/// above, but the authored rule is a single box, so it pins both minors
/// *and* the strength floor.
#[test]
fn unusual_2nt_over_our_major_reads_both_minors() {
    let inf = read_booked(&[bid(1, Strain::Spades), bid(2, Strain::Notrump)]);
    assert!(inf.rho().length(Suit::Clubs).min >= 5);
    assert!(inf.rho().length(Suit::Diamonds).min >= 5);
    assert!(inf.rho().strength.points.min >= 8, "the rule's floor");

    // Table-wide disclosure off: nothing recorded for their 2NT (a notrump
    // bid never entered the natural suit walk either).
    set_table_alert_reading(false);
    let inf = read_booked(&[bid(1, Strain::Spades), bid(2, Strain::Notrump)]);
    assert_eq!(inf.rho().length(Suit::Clubs), Range::FULL_LENGTH);
    assert_eq!(inf.rho().length(Suit::Diamonds), Range::FULL_LENGTH);
    assert_eq!(inf.rho().strength.points, Range::FULL_POINTS);
    set_table_alert_reading(true);
}

/// The retirement guard for chop 1 (`docs/reader-retirement.md`)
///
/// `two_suiter_reading` claimed `other_major >= 5` for their Michaels cue
/// and `♣ >= 5 && ♦ >= 5` for their unusual `(2NT)`.  Every one of those
/// claims is a **subset** of the authoring rule's projection on every
/// auction the reader used to fire on — both seat-fans of the opening and
/// both reading seats (the opponents' call decoded by the table-alert
/// walk, and the same call decoded own-side at the advancer's turn) — and
/// the projection adds the rule's `points >= 8` on top.  That subset
/// property is why the chop needed no A/B: the reader's `narrow_length`
/// was already an idempotent intersect against a hull folded in before it.
#[test]
fn retired_two_suiter_reader_is_subsumed_by_the_projection() {
    let michaels: [(&[Call], Relative); 3] = [
        (
            &[bid(1, Strain::Hearts), bid(2, Strain::Hearts)],
            Relative::Rho,
        ),
        (
            &[Call::Pass, bid(1, Strain::Hearts), bid(2, Strain::Hearts)],
            Relative::Rho,
        ),
        // The advancer's turn: index 1 is now our own side, decoded by the
        // exact-node walk rather than the table-alert one.
        (
            &[bid(1, Strain::Hearts), bid(2, Strain::Hearts), Call::Pass],
            Relative::Partner,
        ),
    ];
    for (auction, who) in michaels {
        let inf = read_booked(auction);
        let shown = inf.get(who);
        assert!(
            shown.length(Suit::Spades).min >= 5,
            "{auction:?}: the retired reader's other-major floor"
        );
        assert!(
            shown.strength.points.min >= 8,
            "{auction:?}: the floor the reader never carried"
        );
        assert_eq!(
            shown.length(Suit::Hearts),
            Range::FULL_LENGTH,
            "{auction:?}: the cue is not natural hearts"
        );
    }

    let unusual: [(&[Call], Relative); 2] = [
        (
            &[bid(1, Strain::Spades), bid(2, Strain::Notrump)],
            Relative::Rho,
        ),
        (
            &[bid(1, Strain::Spades), bid(2, Strain::Notrump), Call::Pass],
            Relative::Partner,
        ),
    ];
    for (auction, who) in unusual {
        let inf = read_booked(auction);
        let shown = inf.get(who);
        assert!(
            shown.length(Suit::Clubs).min >= 5,
            "{auction:?}: the retired reader's club floor"
        );
        assert!(
            shown.length(Suit::Diamonds).min >= 5,
            "{auction:?}: the retired reader's diamond floor"
        );
        assert!(
            shown.strength.points.min >= 8,
            "{auction:?}: the floor the reader never carried"
        );
    }
}

#[test]
fn uvu_major_cue_projects_the_raise() {
    use crate::bidding::american::set_uvu_over_majors;

    // [1♥, (2NT), 3♣, (P)] from opener's seat: partner's cheap cue is the
    // alerted limit-plus raise — decoded off its authored rule's
    // projection (3+ hearts, 10+), not as natural clubs.
    set_uvu_over_majors(true);
    let inf = read_booked(&[
        bid(1, Strain::Hearts),
        bid(2, Strain::Notrump),
        bid(3, Strain::Clubs),
        Call::Pass,
    ]);
    let cue_bidder = inf.partner();
    assert!(
        cue_bidder.length(Suit::Hearts).min >= 3,
        "the projected fit"
    );
    assert!(
        cue_bidder.strength.points.min >= 10,
        "the projected strength"
    );
    assert_eq!(
        cue_bidder.length(Suit::Clubs),
        Range::FULL_LENGTH,
        "not natural clubs"
    );
}

#[test]
fn rubens_transfer_reading_knob_recovers_suppress_only() {
    crate::bidding::instinct::set_rubens_advances(true);
    // Stage-2 knob off: the transfer is still suppressed (not natural
    // hearts) but records nothing — the pre-fix shape.
    set_rubens_transfer_reading(false);
    let inf = read(&[
        bid(1, Strain::Clubs),
        bid(1, Strain::Spades),
        Call::Pass,
        bid(2, Strain::Hearts),
        Call::Pass,
    ]);
    assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
    assert_eq!(inf.partner().length(Suit::Hearts), Range::FULL_LENGTH);
    assert_eq!(inf.partner().strength.points, Range::FULL_POINTS);
    set_rubens_transfer_reading(true);
}

/// D1c: knob-on hygiene drops sum-infeasible ghosts and contained boxes,
/// leaving the union exact and short.
#[test]
fn tidy_prunes_ghosts_and_contained() {
    use crate::bidding::constraint::{Constraint as _, and, balanced, points};

    set_envelope_union_reading(true);
    let context = Context::new(RelativeVulnerability::NONE, &[]);

    // `balanced & {3..}⁴`: the four 5(332) pan-handles intersect to
    // sum-infeasible 5-3-3-3 ghosts; only the {3..=4}⁴ flat cube survives.
    let flat = (balanced() & and(Suit::ASC, 3..)).project_band(&context);
    let mut expected = Envelope::unknown();
    expected.lengths = [Range::new(3, 4); 4];
    assert_eq!(flat.boxes(), &[expected]);

    // A strength-only `Or` duplicates the five shape boxes across its two
    // arms; the wider-points copy encloses the narrower, so five remain.
    let dup = (balanced() & (points(8..) | points(10..))).project_band(&context);
    assert_eq!(dup.boxes().len(), 5);

    set_envelope_union_reading(true);
}

/// The 560 ordered shapes — every 4-tuple of suit lengths summing to 13.
fn all_shapes() -> Vec<[u8; 4]> {
    (0..=13u8)
        .flat_map(|a| {
            (0..=13 - a).flat_map(move |b| (0..=13 - a - b).map(move |c| [a, b, c, 13 - a - b - c]))
        })
        .collect()
}

fn shape_fits(lengths: &[Range; 4], shape: &[u8; 4]) -> bool {
    lengths
        .iter()
        .zip(shape)
        .all(|(range, &len)| range.contains(len))
}

/// C1: `narrow_to_sum` is **exact** — every narrowed bound is attained by a
/// real 13-card shape inside the box — and **membership-inert**: the same
/// shapes lie in the box before and after.  Idempotent, too.
#[test]
fn sum_closure_is_exact_and_inert() {
    let shapes = all_shapes();
    assert_eq!(shapes.len(), 560);

    // Deterministic xorshift — the point is coverage, not randomness.
    let mut state = 0x2545_f491_4f6c_dd1d_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut tested = 0_u32;
    for _ in 0..8000 {
        let mut lengths = [Range::FULL_LENGTH; 4];
        for range in &mut lengths {
            let min = u8::try_from(next() % 8).expect("under 8");
            let max = min + u8::try_from(next() % u64::from(14 - min)).expect("under 14");
            *range = Range::new(min, max);
        }
        let mut envelope = Envelope::unknown();
        envelope.lengths = lengths;
        if !envelope.sum_feasible() {
            continue;
        }
        tested += 1;

        let inside: Vec<_> = shapes.iter().filter(|s| shape_fits(&lengths, s)).collect();
        assert!(
            !inside.is_empty(),
            "sum-feasible box {lengths:?} holds no shape"
        );
        envelope.narrow_to_sum();

        for (suit, range) in envelope.lengths.iter().enumerate() {
            let low = inside.iter().map(|s| s[suit]).min().expect("nonempty");
            let high = inside.iter().map(|s| s[suit]).max().expect("nonempty");
            assert_eq!(
                (range.min, range.max),
                (low, high),
                "suit {suit} of {lengths:?} narrowed to {range:?}, truth {low}..={high}"
            );
        }
        assert!(
            shapes
                .iter()
                .all(|s| shape_fits(&lengths, s) == shape_fits(&envelope.lengths, s)),
            "closure moved membership on {lengths:?}"
        );

        let once = envelope.lengths;
        envelope.narrow_to_sum();
        assert_eq!(envelope.lengths, once, "not idempotent on {lengths:?}");
    }
    assert!(tested > 1000, "only {tested} feasible boxes sampled");
}

/// C2: a box whose lengths force balanced reads `points == hcp`, because a
/// balanced hand never upgrades.  Knob-off the HCP floor carries the
/// scale's *global* worst-case slack instead.
#[test]
fn upgrade_closure_crisps_the_balanced_band() {
    use crate::bidding::constraint::{Constraint as _, balanced, points};

    set_envelope_union_reading(true);
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let read_hcp = |on: bool| {
        set_upgrade_closure(on);
        let union = (balanced() & points(15..)).project(&context);
        set_upgrade_closure(false);
        union.hull().strength.hcp
    };

    assert_eq!(read_hcp(false), Range::new(13, Range::FULL_POINTS.max));
    assert_eq!(read_hcp(true), Range::new(15, Range::FULL_POINTS.max));
}

/// C2 is **not** membership-inert, unlike C1: it derives a bound on
/// `points` — an axis `admits` tests — from `hcp`, an axis it does not
/// (the write-only axis; see [`set_gauge_membership`]).  So the closure
/// gives an otherwise unenforced HCP claim teeth *through* `points`.
///
/// Found by `examples/probe-closure-features.rs`, which cross-tested
/// sampled layouts against the other arm's reading: C1 rejected 0 of
/// 409,708, C2 rejected 249 of 8,576.  The narrowing is exact relative to
/// what the box *claims*; it is the sampler's acceptance that widens
/// without it.
#[test]
fn upgrade_closure_gives_hcp_teeth() {
    use crate::bidding::constraint::{Constraint as _, balanced, hcp};

    // Flat 4333, 10 raw HCP: balanced ⇒ no upgrade ⇒ `points` == `hcp`.
    // Outside the `hcp(..=8)` claim, yet the loose reading admits it,
    // because `points` was slacked to `hcp + hcp_ceiling_slack()`.
    let hand: Hand = "AKQ2.J43.432.432".parse().expect("valid hand");
    set_envelope_union_reading(true);
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let reading = (balanced() & hcp(..=8)).project_band(&context);

    assert!(reading.clone().tidy().contains(hand));
    set_upgrade_closure(true);
    assert!(!reading.tidy().contains(hand));
    set_upgrade_closure(false);
}

/// Chop E: `set_gauge_membership` gives the raw-HCP and support-points
/// bands membership teeth; off (the default) they are inert.
#[test]
fn gauge_membership_teeth() {
    // 15 raw HCP, flat 4333 (no upgrade on any scale).
    let hand: Hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
    let mut envelope = Envelope::unknown();
    envelope.strength.hcp = Range::new(16, 17);

    // Off: the `points` gauge alone doesn't exclude it…
    assert!(envelope.admits(hand));

    // …on: the raw-HCP band does, and widening the band re-admits.
    set_gauge_membership(true);
    assert!(!envelope.admits(hand));
    envelope.strength.hcp = Range::new(15, 17);
    assert!(envelope.admits(hand));
    envelope.strength.support_points = [Range::new(16, 37); 4];
    assert!(!envelope.admits(hand));
    set_gauge_membership(false);
}

#[test]
fn range_intersect_widens_on_conflict() {
    // Disjoint ranges cannot both hold; widen to the span, never empty.
    assert_eq!(
        Range::new(5, 13).intersect(Range::new(6, 13)),
        Range::new(6, 13)
    );
    assert_eq!(
        Range::new(0, 3).intersect(Range::new(6, 13)),
        Range::new(0, 13)
    );
}

/// Walk every authored rule of a book trie under its authoring-time context
///
/// The shared chassis of the book-wide invariant tests below: iterate the
/// trie's `(auction, classifier)` nodes, skip non-rule classifiers, build the
/// node's [`Context`] (with common prefixes), and visit each rule.
fn for_each_authored_rule(
    trie: &crate::bidding::trie::Trie,
    mut visit: impl FnMut(&[Call], &Context<'_>, &crate::bidding::rules::Rule),
) {
    for (auction, classifier) in trie {
        let auction: &[Call] = &auction;
        let Some(rules) = classifier.as_rules() else {
            continue;
        };
        let context = Context::new(RelativeVulnerability::NONE, auction)
            .with_prefixes(trie.common_prefixes(auction));
        for rule in rules.rules() {
            visit(auction, &context, rule);
        }
    }
}

/// The fallback sibling of [`for_each_authored_rule`]: walk every authored
/// rule wired through a guarded [`Fallback::Classify`][crate::bidding::fallback::Fallback]
///
/// Iterates [`Trie::fallbacks`][crate::bidding::trie::Trie::fallbacks],
/// keeps the classifiers that expose authored
/// [`Rules`][crate::bidding::rules::Rules] via `as_rules`, and visits each
/// rule under the **node-key context** — the same authoring-time
/// approximation the exact-node chassis makes (the fallback actually fires
/// on longer auctions; the sniffer's `claims()` filters already exclude
/// context-dependent atoms).  Classifiers with `as_rules() == None` are
/// reported to `opaque` with their guard label: that list is the residue no
/// rule walk can meter, and the conversion worklist for the pass-reading
/// campaign (`docs/ai-bidder/sampled-projection.md`).
fn for_each_fallback_rule(
    trie: &crate::bidding::trie::Trie,
    mut visit: impl FnMut(&[Call], &Context<'_>, &crate::bidding::rules::Rule),
    mut opaque: impl FnMut(&[Call], Option<String>),
) {
    for (auction, guard, fallback) in trie.fallbacks() {
        let crate::bidding::fallback::Fallback::Classify(classifier) = fallback else {
            continue;
        };
        let auction: &[Call] = &auction;
        let Some(rules) = classifier.as_rules() else {
            opaque(auction, guard.describe());
            continue;
        };
        let context = Context::new(RelativeVulnerability::NONE, auction)
            .with_prefixes(trie.common_prefixes(auction));
        for rule in rules.rules() {
            visit(auction, &context, rule);
        }
    }
}

/// The alert-invariant worklist for one trie: rules whose projection the
/// structural [`artificial`] detector flags but which carry no `.alert(...)`
///
/// Walks under the **legacy hull projection**
/// (`set_envelope_union_reading(false)`):
/// the detector's "floors a suit it did not name" reading was defined
/// against hulls, and knob-on box unions (the fit-split's major floors,
/// `envelope_union_upgrade` boxes) legitimately carry other-suit information that
/// would false-positive it.
fn unalerted_artificial(label: &str, trie: &crate::bidding::trie::Trie) -> Vec<String> {
    set_envelope_union_reading(false);
    let mut worklist = Vec::new();
    for_each_authored_rule(trie, |auction, context, rule| {
        let made = rule.call();
        let doubled = context.last_bid().map(|last| last.strain);
        if super::artificial(&rule.project(context), made, doubled) && rule.alert().is_none() {
            worklist.push(format!(
                "{label}: [{}] {made}  (label: {:?})",
                auction
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" "),
                rule.label(),
            ));
        }
    });
    set_envelope_union_reading(true);
    worklist
}

/// Assert an alert worklist is empty, listing the offenders
fn assert_all_alerted(what: &str, mut worklist: Vec<String>) {
    worklist.sort();
    worklist.dedup();
    assert!(
        worklist.is_empty(),
        "{} {what} artificial calls lack an alert:\n{}",
        worklist.len(),
        worklist.join("\n"),
    );
}

/// Retirement invariant for [`artificial`]: every call the structural
/// detector would read as artificial is *also* alerted by its authoring rule.
///
/// `artificial(project(rule), call) ⟹ rule.alert().is_some()`, walked over
/// every authored rule in the shipped `american()` book (all three phase
/// tries).  This now holds with zero counterexamples, so `|| artificial(p,
/// made)` has been dropped from the decode gate: alerts alone carry the "decode
/// this call" signal (alert-by-disclosed-meaning, the move modern bridge made
/// retiring "X is self-alerting").
///
/// Kept as a **permanent regression guard**: a future artificial bid added
/// without an `.alert(...)` makes this fail (the panic lists the exact call),
/// rather than silently losing its decoding now that the structural fallback is
/// gone.
#[test]
fn artificial_calls_are_alerted() {
    use crate::bidding::american::american;

    let pair = american();
    let mut worklist = Vec::new();
    for (phase, trie) in [
        ("constructive", &pair.constructive.0),
        ("competitive", &pair.competitive.0),
        ("defensive", &pair.defensive.0),
    ] {
        worklist.extend(unalerted_artificial(phase, trie));
    }
    assert_all_alerted("american", worklist);
}

#[test]
fn deviation_knobs_preserve_alert_invariant() {
    use crate::bidding::american::{
        american, set_one_notrump_offshape, set_overcall_four_card, set_weak_two_wild,
    };

    set_one_notrump_offshape(true);
    set_overcall_four_card(true);
    set_weak_two_wild(true);
    let pair = american();
    set_one_notrump_offshape(false);
    set_overcall_four_card(false);
    set_weak_two_wild(false);

    let mut worklist = Vec::new();
    for (phase, trie) in [
        ("constructive", &pair.constructive.0),
        ("competitive", &pair.competitive.0),
        ("defensive", &pair.defensive.0),
    ] {
        worklist.extend(unalerted_artificial(phase, trie));
    }
    assert_all_alerted("american deviation knobs", worklist);
}

/// Disclosure tripwire: the alerted call sites of the default `american()`
/// book, counted per alert slug, against `tests/fixtures/alert-sites.txt`
///
/// [`card`][crate::bidding::card] generates our `.bbsa` disclosure from the
/// live knob state, so a row that *has* a knob can no longer drift.  What
/// generation cannot catch is authoring a convention and never giving it a
/// row at all — the card then silently under-describes us to BBA.  This is
/// the artifact that fires on that: any new (or deleted) alerted rule moves
/// a count, and the failure sends the author to the generator.
///
/// Counts, not the call-site list: the list runs to four figures and would
/// make every unrelated node edit an unreviewable diff, which is how a
/// fixture degrades into a rubber stamp.  Counts are also the granularity
/// that *works* — `Alert("splinter")` is shared by the major-raise splinter
/// and the 1NT splinter, so the slug **set** was unchanged when
/// `set_nt_splinter` shipped, and only the count moved.
#[test]
fn alerted_call_sites_match_the_disclosure_fixture() {
    use crate::bidding::american::american;
    use std::collections::BTreeMap;

    let pair = american();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for trie in [&pair.constructive.0, &pair.competitive.0, &pair.defensive.0] {
        for_each_authored_rule(trie, |_auction, _context, rule| {
            if let Some(alert) = rule.alert() {
                *counts.entry(alert.0).or_default() += 1;
            }
        });
    }
    let found = counts
        .iter()
        .map(|(slug, count)| format!("{slug} {count}\n"))
        .collect::<String>();
    assert_eq!(
        found,
        include_str!("../../../tests/fixtures/alert-sites.txt"),
        "the book's alerted call sites moved.  If you authored or retired a \
         convention, give it a row in `src/bidding/card.rs` (or record there \
         why BBA's schema cannot express it), then bless this fixture:\n\n{found}",
    );
}

/// Per-column reading-leak lists over a set of book tries
///
/// A **leak** is an authored rule whose [`Constraint::describe`] names an
/// axis while **no box** of its [`Rule::project_band_union`] band constrains
/// that axis.  Per-box (not hull) on purpose: a disjunction that constrains
/// the axis in every arm — the fit-split's `points | support points` — is a
/// *sound* reading knob-on even though its hull is full, but knob-off the
/// band is a single hull box, so the same predicate degenerates to the
/// original hull check.
///
/// Columns: one per strength gauge (`HCP`, `points`, `support points` —
/// each noun checked against **its own** gauge), `length` (suit-symbol
/// atoms), `suit HCP` ("HCP in ♠" atoms against the per-suit HCP axis),
/// and `support` ("card support for partner", resolved through
/// [`Context::partner_last_suit`]).
///
/// "Names an axis" is sniffed off the rendered atoms — `describe_int_range`
/// puts the noun last, so the describe strings are **load-bearing test
/// infrastructure**: reword a noun and this sniffer must follow.  The
/// exclusions that keep the signal usable: per-suit gauges read "… in ♠"
/// (excluded from `length`; "HCP in ♠" meters on its own `suit HCP`
/// column), partner-facing atoms end in "partner"
/// (excluded from every gauge column), vacuous `0+` floors are ⊤
/// *correctly*, and `points` awards an atom to the most specific noun
/// (`support points` is not a `points` claim).
/// The rule walk `axis_leaks_with` meters over — exact-node or fallback
type RuleWalk = fn(
    &crate::bidding::trie::Trie,
    &mut dyn FnMut(&[Call], &Context<'_>, &crate::bidding::rules::Rule),
);

fn axis_leaks(
    tries: &[(&str, &crate::bidding::trie::Trie)],
) -> std::collections::BTreeMap<&'static str, Vec<String>> {
    axis_leaks_with(tries, |trie, visit| for_each_authored_rule(trie, visit))
}

fn axis_leaks_with(
    tries: &[(&str, &crate::bidding::trie::Trie)],
    walk: RuleWalk,
) -> std::collections::BTreeMap<&'static str, Vec<String>> {
    use crate::bidding::constraint::Description;

    /// Flatten a description tree into its leaf atoms.
    fn atoms(description: &Description, out: &mut Vec<String>) {
        match description {
            Description::Atom(text) => out.push(text.to_string()),
            Description::Not(inner) => atoms(inner, out),
            Description::All(parts) | Description::Any(parts) => {
                for part in parts {
                    atoms(part, out);
                }
            }
            Description::Opaque => {}
        }
    }

    /// A non-vacuous claim of `noun`: `describe_int_range` puts the noun last.
    fn claims(atom: &str, noun: &str) -> bool {
        atom.ends_with(noun) && !atom.starts_with("0+")
    }

    let mut leaks = std::collections::BTreeMap::<&'static str, Vec<String>>::new();
    for &(system, trie) in tries {
        walk(trie, &mut |_, context, rule| {
            let mut leaves = Vec::new();
            atoms(&rule.describe(), &mut leaves);
            let band = rule.project_band_union(context);
            let boxes = band.boxes();
            let text = leaves.join(" | ");
            let entry = format!("{system}: {} :: {text}", rule.call());

            type Vacuous = fn(&Strength) -> bool;
            let gauges: [(&'static str, Vacuous); 3] = [
                ("HCP", |s| s.hcp == Range::FULL_POINTS),
                ("points", |s| s.points == Range::FULL_POINTS),
                ("support points", |s| {
                    s.support_points
                        .iter()
                        .all(|slot| *slot == Range::FULL_POINTS)
                }),
            ];
            for (noun, vacuous) in gauges {
                let named = leaves.iter().any(|atom| {
                    claims(atom, noun) && (noun != "points" || !claims(atom, "support points"))
                });
                if named && boxes.iter().all(|b| vacuous(&b.strength)) {
                    leaks.entry(noun).or_default().push(entry.clone());
                }
            }

            for suit in Suit::ASC {
                let symbol = suit.to_string();
                let named = leaves.iter().any(|atom| {
                    claims(atom, &symbol)
                        // Per-suit gauges read "… in ♠" and meter on their
                        // own columns; "partner's last suit is ♠" is a
                        // *context* claim, not a hand one; "≤13 ♠" is a
                        // deliberate no-op cap (`len(x, ..14)` for gating
                        // symmetry) — all vacuous on the length axis.
                        && !atom.contains(" in ")
                        && !atom.contains("last suit is")
                        && !atom.starts_with("≤13 ")
                });
                if named && boxes.iter().all(|b| b.length(suit) == Range::FULL_LENGTH) {
                    leaks
                        .entry("length")
                        .or_default()
                        .push(format!("{system}: {symbol} {} :: {text}", rule.call()));
                    break;
                }
            }

            for suit in Suit::ASC {
                let noun = format!("HCP in {suit}");
                let named = leaves.iter().any(|atom| claims(atom, &noun));
                if named
                    && (boxes.iter())
                        .all(|b| b.strength.suit_hcp[suit as usize] == Range::FULL_SUIT_HCP)
                {
                    leaks
                        .entry("suit HCP")
                        .or_default()
                        .push(format!("{system}: {suit} {} :: {text}", rule.call()));
                    break;
                }
            }

            if let Some(suit) = context.partner_last_suit() {
                let named = leaves
                    .iter()
                    .any(|atom| claims(atom, "card support for partner"));
                if named && boxes.iter().all(|b| b.length(suit) == Range::FULL_LENGTH) {
                    leaks.entry("support").or_default().push(entry.clone());
                }
            }
        });
    }
    for column in leaks.values_mut() {
        column.sort();
        column.dedup();
    }
    leaks
}

/// E0: book-wide soundness — a finite `eval` implies strict membership of
/// the knob-on projection, forward and band, for every authored rule of
/// the shipped systems.
///
/// This is the safety net under the whole DNF wave: every projection
/// upgrade (complement halves, De Morgan, shape unions, `Support`'s
/// forward box, `tidy`'s pruning) claims *at most* what its gate enforces,
/// and here each claim is replayed against random hands — a hand the rule
/// accepts must lie in some box of the rule's own reading, on **every**
/// gauge ([`Envelope::accepts`]).  A few extreme hands ride along to probe
/// the gauge ceilings (a 37-HCP maximum, a 13-0-0-0 freak).
#[test]
fn authored_rules_eval_within_projection() {
    use crate::bidding::american::american;
    use crate::bidding::dutch::dutch;
    use rand::SeedableRng as _;

    // ponytail: 128 hands keeps the sweep under ~10s in the default test
    // run; the deep-auction rules re-walk `Inferences::read` per eval and
    // dominate the cost.  Crank the pool when hunting a specific leak.
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xE0);
    let mut hands: Vec<Hand> = crate::bidding::verify::random_hands(&mut rng)
        .take(128)
        .collect();
    hands.extend(
        [
            "AKQJ.AKQJ.AKQ.AK",
            "AKQJT98765432...",
            "..AKQJT98765432.",
            "AKQ2.K53.QJ4.T92",
        ]
        .map(|text| text.parse::<Hand>().unwrap_or_else(|_| unreachable!())),
    );

    set_envelope_union_reading(true);
    let american = american();
    let dutch = dutch();
    let tries: [(&str, &crate::bidding::trie::Trie); 4] = [
        ("american constructive", &american.constructive.0),
        ("american competitive", &american.competitive.0),
        ("american defensive", &american.defensive.0),
        ("dutch constructive", &dutch.constructive.0),
    ];

    fn check(
        failures: &mut Vec<String>,
        hands: &[Hand],
        system: &str,
        auction: &[Call],
        context: &Context<'_>,
        rule: &crate::bidding::rules::Rule,
    ) {
        let forward = rule.project_union(context);
        let band = rule.project_band_union(context);
        for &hand in hands {
            if !rule.eval(hand, context).is_finite() {
                continue;
            }
            for (fold, union) in [("project", &forward), ("band", &band)] {
                if !union.boxes().iter().any(|envelope| envelope.accepts(hand))
                    && failures.len() < 16
                {
                    failures.push(format!(
                        "{system}: [{}] {} {fold} excludes accepted hand {hand}",
                        auction
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" "),
                        rule.call(),
                    ));
                }
            }
        }
    }

    let mut failures: Vec<String> = Vec::new();
    for (system, trie) in tries {
        for_each_authored_rule(trie, |auction, context, rule| {
            check(&mut failures, &hands, system, auction, context, rule);
        });
        // The same soundness claim for fallback-authored rules — the layer
        // the exact-node walk cannot see (`docs/ai-bidder/sampled-projection.md`
        // census: the meter blind spot).  Asserts, not pins: soundness has
        // no acceptable nonzero.
        for_each_fallback_rule(
            trie,
            |auction, context, rule| {
                check(&mut failures, &hands, system, auction, context, rule);
            },
            |_, _| {},
        );
    }

    assert!(
        failures.is_empty(),
        "unsound projections (eval ⊄ reading):\n{}",
        failures.join("\n"),
    );
}

/// Pass-exclusion soundness: wherever a table's argmax is (or ties with)
/// Pass, the knob-on pass projection must admit the hand.
///
/// [`authored_rules_eval_within_projection`] replays each rule against its
/// *own* reading; the exclusion reading is a claim about the **table** —
/// "no passer holds a hand a strictly-heavier sibling gate accepts" — so
/// this sweep replays the argmax itself.  Ties count as passes (stricter
/// than the drivers, whose `max_by` keeps the later call), which is why
/// the exclusion threshold is a strict `>` on weight.
#[test]
fn passes_read_within_their_table() {
    use crate::bidding::american::american;
    use crate::bidding::dutch::dutch;
    use crate::bidding::trie::Classifier as _;
    use rand::SeedableRng as _;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0x9A55);
    let mut hands: Vec<Hand> = crate::bidding::verify::random_hands(&mut rng)
        .take(128)
        .collect();
    hands.extend(
        ["AKQJ.AKQJ.AKQ.AK", "AKQ2.K53.QJ4.T92"]
            .map(|text| text.parse::<Hand>().unwrap_or_else(|_| unreachable!())),
    );

    set_envelope_union_reading(true);
    set_pass_exclusion_reading(true);
    let american = american();
    let dutch = dutch();
    let tries: [(&str, &crate::bidding::trie::Trie); 4] = [
        ("american constructive", &american.constructive.0),
        ("american competitive", &american.competitive.0),
        ("american defensive", &american.defensive.0),
        ("dutch constructive", &dutch.constructive.0),
    ];

    let mut failures: Vec<String> = Vec::new();
    let mut check = |system: &str,
                     auction: &[Call],
                     context: &Context<'_>,
                     rules: &crate::bidding::rules::Rules| {
        let Some(projection) = super::project_pass(rules, None, context) else {
            return;
        };
        for &hand in &hands {
            let logits = rules.classify(hand, context);
            let pass = *logits.0.get(Call::Pass);
            let best_other = (&logits.0)
                .into_iter()
                .filter(|(call, _)| *call != Call::Pass)
                .map(|(_, logit)| *logit)
                .fold(f32::NEG_INFINITY, f32::max);
            if !pass.is_finite() || pass < best_other {
                continue;
            }
            if !projection
                .as_union()
                .boxes()
                .iter()
                .any(|b| b.accepts(hand))
                && failures.len() < 16
            {
                failures.push(format!(
                    "{system}: [{}] pass reading excludes passing hand {hand}",
                    auction
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" "),
                ));
            }
        }
    };

    for (system, trie) in tries {
        for (auction, classifier) in trie {
            if let Some(rules) = classifier.as_rules() {
                let context = Context::new(RelativeVulnerability::NONE, &auction)
                    .with_prefixes(trie.common_prefixes(&auction));
                check(system, &auction, &context, rules);
            }
        }
        for (auction, _, fallback) in trie.fallbacks() {
            let crate::bidding::fallback::Fallback::Classify(classifier) = fallback else {
                continue;
            };
            if let Some(rules) = classifier.as_rules() {
                let context = Context::new(RelativeVulnerability::NONE, &auction)
                    .with_prefixes(trie.common_prefixes(&auction));
                check(system, &auction, &context, rules);
            }
        }
    }
    set_pass_exclusion_reading(false);

    assert!(
        failures.is_empty(),
        "pass-exclusion excludes hands that pass:\n{}",
        failures.join("\n"),
    );
}

/// Sibling invariant to [`artificial_calls_are_alerted`]: an authored rule that
/// *gates* on an axis must not *read* as ⊤ on that axis.
///
/// The fit-split bug is the motivating case (see
/// `docs/ai-bidder/sampled-projection.md`): `hcp(13..) | (support(3..) &
/// support_points(13..))` is a correct bidding rule that measured as a win, yet
/// its projection says nothing about points at all — `Or::project` is the union,
/// and one box holding a union is the bounding box, so the union is `0..=37`.
/// Nothing errored and no test went red; the reading simply stopped knowing
/// anything and kept a straight face.  The principle this pins down: the
/// machinery may be *imprecise*, but never imprecise **invisibly**.
///
/// The leak notion and its describe-sniffing caveats live on [`axis_leaks`].
/// The walk covers the shipped `american()` books plus `dutch()`'s
/// constructive trie (Dutch reuses american's competitive and defensive
/// books), and runs **twice**:
///
/// - **knob-off** (`set_envelope_union_reading(false)`) — the legacy reading;
///   the
///   byte-identity guard.  These counts must not move *in either direction*:
///   a fall means a knob-off hull tightened, which is a bidding change that
///   must ship through measurement, not slip in as a refactor.
/// - **knob-on** — the migration meter.  DNF-wave chops drive these toward
///   zero; each re-pin is recorded in `docs/dnf-migration.md`'s ledger.
///
/// **Pinned exactly, not as a `<=` ratchet**: a fix-one-add-one swap cannot
/// hide, at the price of consciously re-pinning (same commit, ledger row)
/// whenever authoring legitimately moves a count.
#[test]
fn authored_calls_read_what_they_gate() {
    use crate::bidding::american::american;
    use crate::bidding::dutch::dutch;

    let american = american();
    let dutch = dutch();
    let tries: [(&str, &crate::bidding::trie::Trie); 4] = [
        ("american constructive", &american.constructive.0),
        ("american competitive", &american.competitive.0),
        ("american defensive", &american.defensive.0),
        ("dutch constructive", &dutch.constructive.0),
    ];

    set_envelope_union_reading(false);
    let off = axis_leaks(&tries);
    set_envelope_union_reading(true);
    let on = axis_leaks(&tries);

    // (column, knob-off pin, knob-on pin) — re-pins go in the
    // docs/dnf-migration.md ledger.  Chop G drove every knob-on column to
    // **zero**: comparative staircases, reroute `envelope_union_upgrade` boxes,
    // `top_honors`/`Points` gauge floors, and `Balanced`'s unbalanced
    // complement.  Knob-off pins are the byte-identity guard; `length`
    // dropped 71 → 59 when the sniffer stopped counting context claims
    // ("partner's last suit is ♠") and deliberate no-op caps ("≤13 ♠") —
    // a meter-precision change, not a reading change (the dump diff
    // stayed clean).  The 2026-07-25 `Points13` gate default (the major
    // no-fit 2/1 now gauges `points(13..)`, not `hcp(13..)`) swaps six
    // legacy-`Or` leaks from HCP (17 → 11) to points (3 → 9); the knob-on
    // The envelope-union box pins both axes exactly, so both knob-on columns stay 0.
    let pinned: [(&str, usize, usize); 6] = [
        // 11/0 → 20/9 when the queen relay went default-on (2026-08-02).
        // The nine new leaks are the same three calls in each column —
        // the asker's continuations over a 1430 answer, which *gate* on
        // `19+ HCP` (the grand-zone strength bar) but *read* as keycard
        // counts and "the queen cannot change the call".  The reading is
        // the honest one; the HCP conjunct is a strength floor that the
        // reading deliberately does not project, so the meter scores it a
        // leak.  **Recorded, not resolved** — closing it means either
        // projecting the strength bar (which would over-narrow partner's
        // hand at every keycard answer) or dropping it (which would let
        // the relay fire without the values).  See
        // docs/ai-bidder/bba-kickback.md §7.7.
        ("HCP", 20, 9),
        ("length", 59, 0),
        ("points", 9, 0),
        // 0/0 measured at birth (2026-07-25): every `suit_hcp` gate the
        // walk reaches (Ogust, the Lebensohl trap pass) is `&`-chained, and
        // the exact base-axis projection is ungated, so even the knob-off
        // hull keeps the band.  The `Or`-shaped gates (UVU double, penalty
        // X, SOS runouts) are wired as `Fallback::classify` and the walk
        // never sees them — a pre-existing meter blind spot on EVERY
        // column, recorded in docs/dnf-migration.md.
        ("suit HCP", 0, 0),
        // 84 → 107 when the direct-seat `(≤2♠)` guard became one exact
        // table per overcall (2026-08-06): its 23 cue/raise rules — the
        // same rules, the same hulls — moved from the fallback layer
        // (where this walk never saw them, and where the fallback sibling
        // metered them under the guard-key context whose
        // `partner_last_suit()` is `None`, sniffing no support atom) onto
        // exact `[1x (overcall)]` nodes where the support axis is live.
        // The knob-on column stays 0: the envelope union projects support
        // exactly.  107 → 115 when the C6 batch (negX answer, strong-two
        // competition, high-overcall, free-bid answer) followed the same
        // guard-to-exact path: eight raise rules of the answer tables
        // surface identically.  Ledger rows in docs/dnf-migration.md.
        ("support", 115, 0),
        ("support points", 18, 0),
    ];
    let count = |leaks: &std::collections::BTreeMap<&str, Vec<String>>, column| {
        leaks.get(column).map_or(0, Vec::len)
    };
    let dump = |leaks: &std::collections::BTreeMap<&str, Vec<String>>, column| {
        leaks.get(column).map_or_else(String::new, |v| v.join("\n"))
    };
    let mut mismatches = Vec::new();
    for (column, pin_off, pin_on) in pinned {
        let (got_off, got_on) = (count(&off, column), count(&on, column));
        if got_off != pin_off || got_on != pin_on {
            mismatches.push(format!(
                "{column}: knob-off {got_off} (pinned {pin_off}), \
                 knob-on {got_on} (pinned {pin_on})\n\
                 --- knob-off ---\n{}\n--- knob-on ---\n{}",
                dump(&off, column),
                dump(&on, column),
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "axis leak counts moved:\n{}",
        mismatches.join("\n\n"),
    );
}

/// The fallback-layer twin of [`authored_calls_read_what_they_gate`]: the
/// same axis-leak meter over every rule wired through a guarded
/// [`Fallback::classify`][crate::bidding::fallback::Fallback::classify] —
/// the layer the exact-node walk cannot see (every contested convention:
/// UVU, penalty-X and SOS runouts, transfer competition).
///
/// Pinned exactly like its sibling, in a **separate table** so the
/// exact-node pins never re-pin for fallback churn.  Pin-first discipline:
/// the initial nonzero counts *are* the worklist
/// (`docs/ai-bidder/sampled-projection.md`), not failures to fix before
/// landing the meter.  The opaque census below is the residue even this
/// walk cannot meter — closures with no `as_rules()` — pinned with labels
/// so a new dark classifier is a conscious act; that list is the
/// conversion worklist for the pass-reading campaign.
#[test]
fn fallback_rules_read_what_they_gate() {
    use crate::bidding::american::american;
    use crate::bidding::dutch::dutch;

    let american = american();
    let dutch = dutch();
    let tries: [(&str, &crate::bidding::trie::Trie); 4] = [
        ("american constructive", &american.constructive.0),
        ("american competitive", &american.competitive.0),
        ("american defensive", &american.defensive.0),
        ("dutch constructive", &dutch.constructive.0),
    ];
    let walk: RuleWalk = |trie, visit| for_each_fallback_rule(trie, visit, |_, _| {});

    set_envelope_union_reading(false);
    let off = axis_leaks_with(&tries, walk);
    set_envelope_union_reading(true);
    let on = axis_leaks_with(&tries, walk);

    // Pinned at birth (2026-07-27) — the meter getting honest, not a
    // regression: these are the worklist the exact-node walk never saw.
    // The knob-on residue (14 HCP, 19 length, 2 points) is dominated by
    // the competitive free-bid/responsive-double package and the 4NT
    // quantitative fallback; `suit HCP`'s two knob-off leaks (the UVU
    // double) already close knob-on.  Re-pins ride the
    // docs/dnf-migration.md ledger like the sibling's.
    //
    // `points` went 2 → 8 → **0** over 2026-08-02.  All three numbers are
    // one mechanism: the keycard ask carried
    // `announced(slam_entry_reached(), points(11..))`, whose *agreement*
    // half is pure disclosure — the judgment is the support-point entry
    // bar, so the 11 was never a gate on anything.  Two leaks while only
    // 4NT asked, eight once kickback added three more asks across the two
    // constructive columns, and none at all once `set_rkcb_announce` was
    // deleted for announcing a floor the ask does not honour.  Deleting a
    // false announcement closed the leak outright rather than deferring
    // it, which is why this row is not on the §7.7 worklist with the
    // sibling's nine HCP-axis leaks.
    let pinned: [(&str, usize, usize); 6] = [
        ("HCP", 14, 14),
        // 28/19 → 9/0 when the direct-seat `(≤2♠)` guard became exact
        // per-overcall nodes (2026-08-06) and left this walk: 19 of the
        // knob-off leaks and the *entire* knob-on residue were its
        // negative-double/free-bid arms — the named OR-projection wall.
        // Per column each table keeps a single arm, so the wall does not
        // reappear in the exact-node sibling's length row (59 holds).
        // Ledger row in docs/dnf-migration.md.
        ("length", 9, 0),
        ("points", 0, 0),
        ("suit HCP", 2, 0),
        ("support", 0, 0),
        ("support points", 0, 0),
    ];
    let count = |leaks: &std::collections::BTreeMap<&str, Vec<String>>, column| {
        leaks.get(column).map_or(0, Vec::len)
    };
    let dump = |leaks: &std::collections::BTreeMap<&str, Vec<String>>, column| {
        leaks.get(column).map_or_else(String::new, |v| v.join("\n"))
    };
    let mut mismatches = Vec::new();
    for (column, pin_off, pin_on) in pinned {
        let (got_off, got_on) = (count(&off, column), count(&on, column));
        if got_off != pin_off || got_on != pin_on {
            mismatches.push(format!(
                "{column}: knob-off {got_off} (pinned {pin_off}), \
                 knob-on {got_on} (pinned {pin_on})\n\
                 --- knob-off ---\n{}\n--- knob-on ---\n{}",
                dump(&off, column),
                dump(&on, column),
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "fallback axis leak counts moved:\n{}",
        mismatches.join("\n\n"),
    );

    // The opaque census: `Fallback::classify` installations whose
    // classifier exposes no rules.  Counts installations (a shared entry
    // under seat-fanned prefixes rows once per node key), labelled by the
    // guard's describe().
    let mut opaque = Vec::new();
    for (system, trie) in tries {
        for_each_fallback_rule(
            trie,
            |_, _, _| {},
            |auction, label| {
                opaque.push(format!(
                    "{system}: [{}] guard: {}",
                    auction
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" "),
                    label.unwrap_or_else(|| "<unlabelled>".into()),
                ));
            },
        );
    }
    opaque.sort();
    // Census at birth (2026-07-27), the residue worklist for the
    // pass-reading campaign: the seat-fanned `[1NT 2♣]`
    // competition-over-Stayman closure (×4), and the two root `(always)`
    // catch-alls — the competitive and defensive floor layers, exactly the
    // `Fallback::classify` blind spot the ⊤-census named.  Converting one
    // to `Rules` shrinks this pin and grows the metered tables above.
    assert_eq!(
        opaque.len(),
        6,
        "opaque classify-fallback census moved (re-pin consciously):\n{}",
        opaque.join("\n"),
    );
}

/// The same alert invariant, but for the opt-in Gladiator book (off by default,
/// so the walk above never sees it).  A Gladiator artificial call added without
/// `.alert(...)` fails here.
#[test]
fn gladiator_artificial_calls_are_alerted() {
    use crate::bidding::american::{american, set_nt_overcall_gladiator};

    set_nt_overcall_gladiator(true);
    let pair = american();
    set_nt_overcall_gladiator(false);

    assert_all_alerted(
        "Gladiator",
        unalerted_artificial("defensive", &pair.defensive.0),
    );
}

/// The same alert invariant for the [`dutch`][crate::bidding::dutch] system's
/// constructive book.  Dutch reuses american's competitive and defensive
/// books (covered by `artificial_calls_are_alerted`) and overrides only the
/// opening table, so this walks the constructive trie — guarding the strong
/// 2♣ alert and any artificial call a future Dutch phase adds.
#[test]
fn dutch_artificial_calls_are_alerted() {
    use crate::bidding::dutch::dutch;

    let pair = dutch();
    assert_all_alerted(
        "Dutch",
        unalerted_artificial("constructive", &pair.constructive.0),
    );
}

/// The same alert invariant for the opt-in New Minor Forcing book (off by
/// default, so the shipped-system walk never sees it).  Guards the one
/// artificial call NMF adds — responder's `2`-of-the-new-minor checkback —
/// against losing its `.alert(...)` and reading as a phantom minor suit.
#[test]
fn new_minor_forcing_artificial_calls_are_alerted() {
    use crate::bidding::american::{american, set_new_minor_forcing};

    set_new_minor_forcing(true);
    let pair = american();
    set_new_minor_forcing(false);

    assert_all_alerted(
        "New Minor Forcing",
        unalerted_artificial("constructive", &pair.constructive.0),
    );
}

/// The same alert invariant for the opt-in choice-of-games 3NT and 2/1
/// fit-leg books (off by default, so the shipped-system walk never sees
/// them).
#[test]
fn choice_of_games_artificial_calls_are_alerted() {
    use crate::bidding::american::{american, set_major_choice_of_games};

    // ponytail: `two_over_one_fit` now defaults on, so the old set/restore
    // pair here was stale (and restored to the *non*-default).
    set_major_choice_of_games(true);
    let pair = american();
    set_major_choice_of_games(false);

    assert_all_alerted(
        "choice-of-games",
        unalerted_artificial("constructive", &pair.constructive.0),
    );
    set_major_choice_of_games(true);
}

/// The alerted choice-of-games 3NT decodes: opener reads responder as
/// (4333) with 3+ in every suit (so the 5-3 major fit is known), exactly
/// three spades over 1♥, and 12+ points.
#[test]
fn choice_of_games_three_notrump_reads_support() {
    use crate::bidding::american::set_major_choice_of_games;

    set_major_choice_of_games(true);
    let stance = crate::american().against();
    set_major_choice_of_games(false);

    let auction = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(3, Strain::Notrump),
        Call::Pass,
    ];
    let read = Inferences::read(&stance.prefixed_context(RelativeVulnerability::NONE, &auction));
    assert!(read.partner().length(Suit::Hearts).min >= 3);
    assert!(read.partner().length(Suit::Diamonds).min >= 3);
    assert!(read.partner().length(Suit::Clubs).min >= 3);
    assert_eq!(read.partner().length(Suit::Spades), Range::new(3, 3));
    assert!(read.partner().strength.points.min >= 12);
    set_major_choice_of_games(true);
}

proptest! {
    /// Soundness: a hand that opens the book's choice falls within the
    /// opening inference.  Tests rule 1 (the opening table) over random hands.
    #[test]
    fn opening_inference_contains_the_opener(seed in any::<u64>()) {
        use crate::bidding::trie::Classifier;
        use crate::bidding::american::openings;
        use contract_bridge::deck::full_deal;
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let deal = full_deal(&mut rng);
        let hand: Hand = deal[contract_bridge::Seat::North];

        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let logits = openings().classify(hand, &context);
        let Some((call, _)) = (&logits.0)
            .into_iter()
            .filter(|(_, l)| l.is_finite())
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("not NaN"))
        else {
            return Ok(());
        };
        let Call::Bid(_) = call else { return Ok(()); };

        // The opener sits to the actor's right after a single call.
        let inf = read(&[call]);
        let opener = inf.rho();
        let points = point_count(hand);
        prop_assert!(
            opener.strength.points.contains(points),
            "{call} opener with {points} points outside {:?}",
            opener.strength.points
        );
        for suit in Suit::ASC {
            let length = hand[suit].len();
            // SAFETY: a suit length is at most 13.
            #[allow(clippy::cast_possible_truncation)]
            let length = length as u8;
            prop_assert!(
                opener.length(suit).contains(length),
                "{call} opener with {length} {suit:?} outside {:?}",
                opener.length(suit)
            );
        }
    }

    /// The load-bearing C1/C2 pin: closing the boxes is **membership-inert**
    /// on the real reading path, so the sampler cannot move.  Every hand a
    /// reading admitted knob-off it still admits knob-on, and vice versa —
    /// on the lenient `EnvelopeUnion::contains` the sampler uses *and* the strict
    /// `Envelope::accepts` gate.  If this ever fires, the closure is
    /// dropping legal hands and the A/B verdict means nothing.
    #[test]
    fn closure_is_membership_inert(seed in any::<u64>()) {
        use crate::bidding::constraint::{Constraint as _, and, balanced, hcp, len, or, points};
        use contract_bridge::deck::full_deal;
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let deal = full_deal(&mut rng);
        let hand: Hand = deal[contract_bridge::Seat::North];

        set_envelope_union_reading(true);
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let readings = [
            (balanced() & points(15..17)).project_band(&context),
            (or([Suit::Hearts, Suit::Spades], 5..) & points(8..)).project(&context),
            (and([Suit::Hearts, Suit::Spades], 5..) & hcp(6..11)).project_band(&context),
            (len(Suit::Spades, 6..) & points(13..)).project(&context),
            (!balanced() & points(12..)).project(&context),
        ];

        for reading in readings {
            let loose = reading.clone().tidy();
            set_sum_closure(true);
            set_upgrade_closure(true);
            let closed = reading.tidy();
            set_sum_closure(false);
            set_upgrade_closure(false);

            prop_assert_eq!(
                loose.contains(hand), closed.contains(hand),
                "contains moved: {:?} vs {:?}", loose, closed
            );
            prop_assert_eq!(
                loose.boxes().iter().any(|b| b.accepts(hand)),
                closed.boxes().iter().any(|b| b.accepts(hand)),
                "accepts moved: {:?} vs {:?}", loose, closed
            );
        }
    }
}
