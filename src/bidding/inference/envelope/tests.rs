use crate::bidding::context::Context;
use crate::bidding::inference::{Envelope, EnvelopeUnion, Range, ReadingProfile, Strength};
use contract_bridge::auction::RelativeVulnerability;
use contract_bridge::{Hand, Suit};

fn reading_with(configure: impl FnOnce(&mut ReadingProfile)) -> ReadingProfile {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    configure(&mut agreements.decision.reading);
    agreements.decision.reading
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

    fn disjoin(self, other: Self, profile: ReadingProfile) -> Self {
        if profile.envelope_union {
            self.union(other).tidy(profile)
        } else {
            Self(vec![self.hull().span(&other.hull())])
        }
    }

    fn intersect(&self, other: &Self, profile: ReadingProfile) -> Self {
        let mut out = Vec::new();
        for a in &self.0 {
            for b in &other.0 {
                if let Some(product) = a.intersect_nonempty(b, profile.point_scale()) {
                    out.push(product);
                }
            }
        }
        if out.is_empty() {
            out.push(self.hull().intersect(&other.hull()));
        }
        Self(out).tidy(profile)
    }

    fn tidy(mut self, profile: ReadingProfile) -> Self {
        if !profile.envelope_union {
            return self;
        }
        self.0.retain(Envelope::sum_feasible);
        if profile.sum_closure || profile.upgrade_closure {
            for box_ in &mut self.0 {
                if profile.sum_closure {
                    box_.narrow_to_sum();
                }
                if profile.upgrade_closure {
                    box_.narrow_to_upgrade(profile.point_scale());
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
                let profile = reading_with(|profile| {
                    profile.envelope_union = union;
                    profile.sum_closure = sum;
                    profile.upgrade_closure = upgrade;
                });

                let actual_left = EnvelopeUnion::from_boxes(left.clone());
                let actual_right = EnvelopeUnion::from_boxes(right.clone());
                let reference_left = VecEnvelopeUnion(left.clone());
                let reference_right = VecEnvelopeUnion(right.clone());

                assert_eq!(
                    actual_left.clone().tidy(profile).boxes(),
                    reference_left.clone().tidy(profile).0
                );
                assert_eq!(
                    actual_left.clone().union(actual_right.clone()).boxes(),
                    reference_left.clone().union(reference_right.clone()).0
                );
                assert_eq!(
                    actual_left
                        .clone()
                        .disjoin_with(actual_right.clone(), profile)
                        .boxes(),
                    reference_left
                        .clone()
                        .disjoin(reference_right.clone(), profile)
                        .0
                );
                assert_eq!(
                    actual_left.intersect_owned(&actual_right, profile).boxes(),
                    reference_left.intersect(&reference_right, profile).0
                );
            }
        }
    }
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

/// D1c: knob-on hygiene drops sum-infeasible ghosts and contained boxes,
/// leaving the union exact and short.
#[test]
fn tidy_prunes_ghosts_and_contained() {
    use crate::bidding::constraint::{Constraint as _, and, balanced, points};

    let profile = reading_with(|profile| profile.envelope_union = true);
    let decision = crate::bidding::context::DecisionProfile {
        reading: profile,
        ..Default::default()
    };
    let context = Context::new(RelativeVulnerability::NONE, &[]).with_profile(decision);

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

    let read_hcp = |on: bool| {
        let profile = reading_with(|profile| {
            profile.envelope_union = true;
            profile.upgrade_closure = on;
        });
        let decision = crate::bidding::context::DecisionProfile {
            reading: profile,
            ..Default::default()
        };
        let context = Context::new(RelativeVulnerability::NONE, &[]).with_profile(decision);
        let union = (balanced() & points(15..)).project(&context);
        union.hull().strength.hcp
    };

    assert_eq!(read_hcp(false), Range::new(13, Range::FULL_POINTS.max));
    assert_eq!(read_hcp(true), Range::new(15, Range::FULL_POINTS.max));
}

/// C2 is **not** membership-inert, unlike C1: it derives a bound on
/// `points` — an axis `admits` tests — from `hcp`, an axis it does not
/// (the write-only axis; see [`ReadingProfile::gauge_membership`]).  So the closure
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
    let loose = reading_with(|profile| profile.envelope_union = true);
    let decision = crate::bidding::context::DecisionProfile {
        reading: loose,
        ..Default::default()
    };
    let context = Context::new(RelativeVulnerability::NONE, &[]).with_profile(decision);
    let reading = (balanced() & hcp(..=8)).project_band(&context);

    assert!(reading.clone().tidy(loose).contains_on(hand, loose));
    let closed = reading_with(|profile| {
        profile.envelope_union = true;
        profile.upgrade_closure = true;
    });
    assert!(!reading.tidy(closed).contains_on(hand, closed));
}

/// Chop E: [`ReadingProfile::gauge_membership`] gives the raw-HCP and support-points
/// bands membership teeth; off (the default) they are inert.
#[test]
fn gauge_membership_teeth() {
    // 15 raw HCP, flat 4333 (no upgrade on any scale).
    let hand: Hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
    let mut envelope = Envelope::unknown();
    envelope.strength.hcp = Range::new(16, 17);

    // Off: the `points` gauge alone doesn't exclude it…
    let off = reading_with(|profile| profile.gauge_membership = false);
    assert!(envelope.admits_on(hand, off));

    // …on: the raw-HCP band does, and widening the band re-admits.
    let on = reading_with(|profile| profile.gauge_membership = true);
    assert!(!envelope.admits_on(hand, on));
    envelope.strength.hcp = Range::new(15, 17);
    assert!(envelope.admits_on(hand, on));
    envelope.strength.support_points = [Range::new(16, 37); 4];
    assert!(!envelope.admits_on(hand, on));
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
