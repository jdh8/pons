use super::*;
use crate::bidding::context::Context;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Level, Strain, Suit};

pub(super) const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

pub(super) fn read(auction: &[Call]) -> Inferences {
    Inferences::read(&Context::new(RelativeVulnerability::NONE, auction))
}

/// [`read`] under an explicit set of agreements.
pub(super) fn read_with(
    agreements: &crate::bidding::agreements::Agreements,
    auction: &[Call],
) -> Inferences {
    Inferences::read(
        &Context::new(RelativeVulnerability::NONE, auction).with_profile(agreements.decision),
    )
}

/// Read on a *prefixed* context, the trie access the projection pass needs to
/// read a convention off its authored rule — what the production search floor
/// hands `Inferences::read` (cf. `Partnership::prefixed_context`).  The plain `read`
/// above is keyless, so it sees no convention overlay.
pub(super) fn read_booked(auction: &[Call]) -> Inferences {
    read_booked_with(&crate::bidding::agreements::Agreements::default(), auction)
}

/// [`read_booked`], but under an explicit set of agreements — the shape a knob
/// that lives on [`Agreements`][crate::bidding::agreements::Agreements] rather
/// than in a thread-local cell has to be armed in.
pub(super) fn read_booked_with(
    agreements: &crate::bidding::agreements::Agreements,
    auction: &[Call],
) -> Inferences {
    let partnership = crate::american(agreements).bind();
    Inferences::read(&partnership.prefixed_context(RelativeVulnerability::NONE, auction))
}

/// The system's own choice at `auction` — the highest finite logit, book
/// and floor together (the in-crate twin of `examples/common::next_call`,
/// minus the legality filter: every call these tests expect is legal).
pub(super) fn chosen_call(
    partnership: &crate::bidding::Partnership,
    hand: Hand,
    auction: &[Call],
) -> Call {
    let (logits, _) = partnership
        .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
        .expect("the Gladiator node classifies");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

/// The regime grid every behavioural reading sweep runs: reading scope ×
/// whether the forward strength folds keep their ceilings.
///
/// Phase 1 of docs/authored-reading-handoff.md added the second axis.  A
/// forward ceiling can only *tighten* a box, so it is the axis that can newly
/// exclude a hand its own bidder holds — either because a hand-written walk
/// stamp contradicts the rule it sits on, or because a gate's own gauge and
/// the axis it projects into are different numbers.  All four cells must be
/// green before the knob is measured, let alone shipped; this grid is the
/// general soundness gate the N2 testbed waits on.
const READING_REGIMES: [(ReadingScope, bool); 4] = [
    (ReadingScope::Alerted, false),
    (ReadingScope::Alerted, true),
    (ReadingScope::All, false),
    (ReadingScope::All, true),
];

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

    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.nt_overcall_gladiator = true;
    agreements.decision.reading.envelope_union = true;
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
    let check = |partnership: &crate::bidding::Partnership,
                 failures: &mut Vec<String>,
                 regime: &str,
                 hand: Hand,
                 auction: &[Call],
                 made: Call| {
        let mut read: Vec<Call> = auction.to_vec();
        read.push(made);
        read.push(Call::Pass);
        let inferences = partnership.infer(RelativeVulnerability::NONE, &read);
        if !inferences.admits(Relative::Partner, hand) && failures.len() < 16 {
            failures.push(format!(
                "[{}] ({regime}) reading excludes the hand that bid it: {hand}",
                contract_bridge::auction::display_calls(&read),
            ));
        }
    };

    // Both reading regimes.  Knob-on, the natural advances project their
    // authoring rule *on top of* the walk's reading, so a walk claim that
    // contradicts the rule empties the box instead of quietly overriding it
    // — the sweep is how `set_natural_reading` gets adjudicated per node.
    for (scope, ceilings) in READING_REGIMES {
        agreements.decision.reading.scope = scope;
        agreements.decision.reading.strength_ceilings = ceilings;
        let regime = format!("{scope:?} scope, ceilings {ceilings}");
        let partnership = crate::american(&agreements).bind();
        for &hand in &hands {
            let made = chosen_call(&partnership, hand, &node);
            check(&partnership, &mut failures, &regime, hand, &node, made);
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
            let continued = chosen_call(&partnership, hand, &sorted);
            check(
                &partnership,
                &mut failures,
                &regime,
                hand,
                &sorted,
                continued,
            );
        }
        // The runout branch too — `(1♠) 1NT (X)` is authored, so its
        // escapes are read by the walk like any other natural call.
        let doubled = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Double,
        ];
        for &hand in &hands {
            let made = chosen_call(&partnership, hand, &doubled);
            check(&partnership, &mut failures, &regime, hand, &doubled, made);
        }
    }
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

    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.envelope_union = true;

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
        // cap used to exclude every to-make raiser of `3♥ - 4♥`.
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
        // Two of the three Phase 4 witnesses — nodes whose top tier is a
        // catch-all reading ⊤ until the exclusion fold hands it the heavier
        // tiers' complements.  (The third, opener's rebid over Jacoby, is red
        // in *both* exclusion arms under the legacy `Alerted` scope, where the
        // call is never decoded at all; it is pinned as a knob-off/knob-on
        // witness by `bid_exclusion_admits_the_jacoby_sign_off` instead.)
        (
            "asker's sign-off after the Ogust answer",
            &[
                bid(2, Strain::Hearts),
                Call::Pass,
                bid(2, Strain::Notrump),
                Call::Pass,
                bid(3, Strain::Clubs),
                Call::Pass,
            ],
        ),
        (
            "responder's action over the strong 2♣ opener's 2♥",
            &[
                bid(2, Strain::Clubs),
                Call::Pass,
                bid(2, Strain::Diamonds),
                Call::Pass,
                bid(2, Strain::Hearts),
                Call::Pass,
            ],
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
    // Phase 4's fold only ever *tightens* a box, so it is the third axis this
    // sweep has to gate — same discipline as the ceilings.
    for exclusion in [false, true] {
        agreements.decision.reading.bid_exclusion = exclusion;
        for (scope, ceilings) in READING_REGIMES {
            agreements.decision.reading.scope = scope;
            agreements.decision.reading.strength_ceilings = ceilings;
            let partnership = crate::american(&agreements).bind();
            for &(what, node) in nodes {
                for &hand in &hands {
                    // Honest route only: the seat's earlier calls in the
                    // script must be the ones this hand actually chooses.
                    if (node.len() % 4..node.len())
                        .step_by(4)
                        .any(|i| chosen_call(&partnership, hand, &node[..i]) != node[i])
                    {
                        continue;
                    }
                    let made = chosen_call(&partnership, hand, node);
                    // After `made` and a pass, the seat to act is the bidder's
                    // partner, so `Relative::Partner` is the seat replayed.
                    let mut read: Vec<Call> = node.to_vec();
                    read.push(made);
                    read.push(Call::Pass);
                    let inferences = partnership.infer(RelativeVulnerability::NONE, &read);
                    if !inferences.admits(Relative::Partner, hand) && failures.len() < 16 {
                        failures.push(format!(
                            "{what} [{}] ({scope:?} scope, ceilings {ceilings}, exclusion {exclusion}) excludes the hand that bid it: {hand}",
                            contract_bridge::auction::display_calls(&read),
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "readings exclude their own bidders:\n{}",
        failures.join("\n"),
    );
}

/// Phase 4's sharpest witness: opener's sign-off over Jacoby `2NT`.
///
/// `4♥` is authored `hcp(0..)` at weight 50 under four heavier tiers (the
/// `3x` splinters at 220/200, the `3M` extras rebid at 150, `3NT` at 140), so
/// its projection is ⊤ and the call falls back to the natural walk — which
/// guesses a Jacoby rebid at `points 16..21` and excludes every minimum that
/// actually signs off.  Under
/// [`bid_exclusion`][crate::bidding::ReadingProfile::bid_exclusion] it reads
/// what those tiers *denied* instead, and the minimum is admitted.
///
/// Not a row in [`readings_admit_the_bidder`]: the repair only reaches the
/// shipped [`ReadingScope::All`], and under the legacy `Alerted` scope this
/// call carries no alert, is never decoded, and keeps the walk's guess in
/// both arms.
#[test]
fn bid_exclusion_admits_the_jacoby_sign_off() {
    let node = [
        bid(1, Strain::Hearts),
        Call::Pass,
        bid(2, Strain::Notrump),
        Call::Pass,
    ];
    // Two 4-5-2-2 twelve-counts: sound openers, nowhere near a slam try.
    let hands: Vec<Hand> = ["AK72.JT873.A7.T9", "AT62.JT943.K5.K8"]
        .iter()
        .map(|text| text.parse().expect("a hand"))
        .collect();

    for exclusion in [false, true] {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.decision.reading.envelope_union = true;
        agreements.decision.reading.bid_exclusion = exclusion;
        let partnership = crate::american(&agreements).bind();
        for &hand in &hands {
            assert_eq!(
                chosen_call(&partnership, hand, &[]),
                bid(1, Strain::Hearts),
                "{hand} must open 1♥ for this witness to be about its rebid",
            );
            let made = chosen_call(&partnership, hand, &node);
            assert_eq!(made, bid(4, Strain::Hearts), "{hand} signs off in 4♥");
            let mut read: Vec<Call> = node.to_vec();
            read.push(made);
            read.push(Call::Pass);
            let inferences = partnership.infer(RelativeVulnerability::NONE, &read);
            assert_eq!(
                inferences.admits(Relative::Partner, hand),
                exclusion,
                "[{}] with exclusion {exclusion}: the walk's 16..21 guess must \
                 hold off and the fold must admit {hand} on",
                contract_bridge::auction::display_calls(&read),
            );
        }
    }
}

/// A cheap Michaels advance is preference for partner's shown five-card suit,
/// not a promise of three-card support.  Its authored reading is the exact
/// complement of the heavier game raise: weak with any length, or stronger
/// with at most a doubleton.
#[test]
fn michaels_preference_does_not_promise_three_cards() {
    let cases = [
        (
            Suit::Spades,
            bid(1, Strain::Hearts),
            bid(2, Strain::Hearts),
            bid(2, Strain::Spades),
            "AK.432.AQJ.98765",
            "AK2.432.AQJ.9876",
        ),
        (
            Suit::Hearts,
            bid(1, Strain::Spades),
            bid(2, Strain::Spades),
            bid(3, Strain::Hearts),
            "432.AK.AQJ.98765",
            "432.AK2.AQJ.9876",
        ),
    ];
    for (suit, opening, cue, preference, strong_two, strong_three) in cases {
        let auction = [opening, cue, Call::Pass, preference, Call::Pass];
        for exclusion in [false, true] {
            let mut agreements = crate::bidding::agreements::Agreements::default();
            agreements.decision.reading.envelope_union = true;
            agreements.decision.reading.bid_exclusion = exclusion;
            let inferences = read_booked_with(&agreements, &auction);
            assert_eq!(inferences.partner().length(suit), Range::FULL_LENGTH);
            let weak_three: Hand = "432.432.432.5432".parse().expect("a hand");
            assert!(inferences.admits(Relative::Partner, weak_three));
            assert!(inferences.admits(Relative::Partner, strong_two.parse().expect("a hand")));
            assert!(!inferences.admits(Relative::Partner, strong_three.parse().expect("a hand")));
        }
    }
}

/// Arm the node context's decision cache so one `Inferences::read` serves the
/// whole node
///
/// [`Inferences::read`] is hand-independent, but a bare [`Context`] has no
/// decision scope, so every `rule.eval(hand, context)` and every projection
/// re-derived the identical read — 1.26M of them across the book walks, ~91%
/// of the suite's wall clock.  Arming the scope at the node makes it one read
/// per node, shared by every rule and every probe hand.
///
/// [`Hand::EMPTY`] on purpose: the hand key gates only
/// `Context::trick_estimates`, and no 13-card probe hand equals it, so that
/// slot keeps computing live exactly as it did uncached.  The knob snapshot
/// the cache takes is likewise inert — every walker below flips knobs
/// *around* the walk, never inside it, and `assert_fixed_call` fails loudly in
/// debug if that ever stops being true.
fn node_context<'a>(
    trie: &'a crate::bidding::trie::Trie,
    auction: &'a [Call],
    profile: crate::bidding::context::DecisionProfile,
) -> Context<'a> {
    Context::new(RelativeVulnerability::NONE, auction)
        .with_prefixes(trie.common_prefixes(auction))
        .with_profile(profile)
        .with_decision_cache(Hand::EMPTY)
}

/// The memoised node read is the uncached read, and it happens once
///
/// [`node_context`]'s whole saving rests on two claims that no other test
/// would notice breaking: the memoised [`Inferences`] equals what the
/// uncached path derives (else every soundness net above goes *silently*
/// green against a stale reading), and the decision scope actually stays live
/// across the node's rules (else the walk quietly falls back to 1.26M reads
/// and the suite is back to twelve minutes).  Equality catches the first,
/// the init count catches the second.
#[test]
fn node_context_memoises_the_uncached_read() {
    use crate::bidding::american::american;

    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.envelope_union = true;
    let american = american(&agreements);
    let trie = &american.constructive.0;
    let mut checked = 0usize;
    for (auction, classifier) in trie {
        let auction: &[Call] = &auction;
        if classifier.as_rules().is_none() {
            continue;
        }
        let bare = Context::new(RelativeVulnerability::NONE, auction)
            .with_prefixes(trie.common_prefixes(auction))
            .with_profile(agreements.decision);
        let cached = node_context(trie, auction, agreements.decision);
        let where_ = || contract_bridge::auction::display_calls(auction);
        assert_eq!(
            *cached.inferences(),
            Inferences::read(&bare),
            "the memoised node read differs from the uncached read at [{}]",
            where_(),
        );
        // Second touch: the scope must serve, not re-derive.
        let _ = cached.inferences();
        assert_eq!(
            cached.decision_cache_init_counts().map(|counts| counts.0),
            Some(1),
            "the node decision scope stopped memoising at [{}]",
            where_(),
        );
        checked += 1;
        // ponytail: a prefix of the trie is plenty — the claim is structural,
        // not node-specific, and this test must stay off the critical path.
        if checked >= 256 {
            break;
        }
    }
    assert!(checked > 0, "the walk found no authored rule nodes");
}

/// Walk every authored rule of a book trie under its authoring-time context
///
/// The shared chassis of the book-wide invariant tests below: iterate the
/// trie's `(auction, classifier)` nodes, skip non-rule classifiers, build the
/// node's [`Context`] (with common prefixes), and visit each rule.
fn for_each_authored_rule(
    trie: &crate::bidding::trie::Trie,
    profile: crate::bidding::context::DecisionProfile,
    mut visit: impl FnMut(&[Call], &Context<'_>, &crate::bidding::rules::Rule),
) {
    for (auction, classifier) in trie {
        let auction: &[Call] = &auction;
        let Some(rules) = classifier.as_rules() else {
            continue;
        };
        let context = node_context(trie, auction, profile);
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
    profile: crate::bidding::context::DecisionProfile,
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
        let context = node_context(trie, auction, profile);
        for rule in rules.rules() {
            visit(auction, &context, rule);
        }
    }
}

/// The alert-invariant worklist for one trie: rules whose projection the
/// structural [`artificial`] detector flags but which carry no `.alert(...)`
///
/// Walks under the **legacy hull projection**
/// ([`ReadingProfile::envelope_union`] disabled):
/// the detector's "floors a suit it did not name" reading was defined
/// against hulls, and knob-on box unions (the fit-split's major floors,
/// `envelope_union_upgrade` boxes) legitimately carry other-suit information that
/// would false-positive it.
fn unalerted_artificial(
    label: &str,
    trie: &crate::bidding::trie::Trie,
    mut profile: crate::bidding::context::DecisionProfile,
) -> Vec<String> {
    profile.reading.envelope_union = false;
    let mut worklist = Vec::new();
    let mut visit =
        |auction: &[Call], context: &Context<'_>, rule: &crate::bidding::rules::Rule| {
            let made = rule.call();
            let doubled = context.last_bid().map(|last| last.strain);
            if super::artificial(&rule.project(context), made, doubled) && rule.alert().is_none() {
                worklist.push(format!(
                    "{label}: [{}] {made}  (label: {:?})",
                    contract_bridge::auction::display_calls(auction),
                    rule.label(),
                ));
            }
        };
    for_each_authored_rule(trie, profile, &mut visit);
    // Row packages lower `Pattern::after`/`table` entries to guarded
    // fallbacks, so a convention wired that way (the Landy counter, the
    // interference tails) is invisible to the exact-node walk above.  Two
    // exemptions, both structural:
    //
    // - the `(always)` rails are the instinct floor's rule tables, not book
    //   disclosure — their Stayman/transfer rules are deliberately unalerted
    //   (an always-present alerted rule would suppress the natural reading of
    //   every floor-classified call — the kickback §7.3.1 poison);
    // - `Double`/`Redouble` rules are checked at exact nodes only: the
    //   node-key context cannot witness the strain a suffix-guarded double
    //   actually doubles (a penalty X of their `(2♦)` overcall would read as
    //   doubling partner's `2♣` at the key), so `artificial`'s named suit is
    //   wrong exactly there.
    for (auction, guard, fallback) in trie.fallbacks() {
        if guard.describe().as_deref() == Some("(always)") {
            continue;
        }
        let crate::bidding::fallback::Fallback::Classify(classifier) = fallback else {
            continue;
        };
        let Some(rules) = classifier.as_rules() else {
            continue;
        };
        let auction: &[Call] = &auction;
        let context = node_context(trie, auction, profile);
        for rule in rules.rules() {
            if !matches!(rule.call(), Call::Double | Call::Redouble) {
                visit(auction, &context, rule);
            }
        }
    }
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

    let agreements = crate::bidding::agreements::Agreements::default();
    let system = american(&agreements);
    let mut worklist = Vec::new();
    for (phase, trie) in [
        ("constructive", &system.constructive.0),
        ("competitive", &system.competitive.0),
        ("defensive", &system.defensive.0),
    ] {
        worklist.extend(unalerted_artificial(phase, trie, agreements.decision));
    }
    assert_all_alerted("american", worklist);
}

#[test]
fn deviation_knobs_preserve_alert_invariant() {
    use crate::bidding::american::american;

    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.opening.one_notrump_offshape = true;
    agreements.opening.weak_two_wild = true;
    agreements.defense.overcall_four_card = true;
    let system = american(&agreements);

    let mut worklist = Vec::new();
    for (phase, trie) in [
        ("constructive", &system.constructive.0),
        ("competitive", &system.competitive.0),
        ("defensive", &system.defensive.0),
    ] {
        worklist.extend(unalerted_artificial(phase, trie, agreements.decision));
    }
    assert_all_alerted("american deviation knobs", worklist);
}

/// The same alert invariant over gated books the default walk never builds:
/// profiles a shipped arm can actually field.
///
/// [`artificial_calls_are_alerted`] walks `Agreements::default()`, so a rule
/// behind a non-default gate is invisible to it.  The proof this matters: the
/// Landy counter (`their.two_clubs_landy` — **true in the anchor**, derived
/// off BBA's measured behavior in `bba-gen`) shipped a whole alerted subtree
/// no default-build sweep ever visited.  Each 1NT-defense variant and the
/// RKCB relocation are the other gates a live arm fields; any gadget added
/// behind a new `TheirDisclosures` field belongs in this list.
#[test]
fn gated_profiles_preserve_alert_invariant() {
    use crate::bidding::agreements::Agreements;
    use crate::bidding::american::{NotrumpDefense, american};

    let mut profiles: Vec<(&str, Agreements)> = Vec::new();
    let base = Agreements::default();
    {
        let mut a = base;
        a.decision.their.two_clubs_landy = true;
        profiles.push(("their-landy", a));
    }
    {
        let mut a = base;
        a.decision.their.two_diamonds_multi = true;
        profiles.push(("their-multi", a));
    }
    {
        let mut a = base;
        a.decision.their.two_diamonds_multi = true;
        a.competition.multi_weak_escape = Some(6);
        profiles.push(("their-multi-escape", a));
    }
    for (name, defense) in [
        ("woolsey", NotrumpDefense::Woolsey),
        ("meckwell", NotrumpDefense::Meckwell),
        ("direct-dont", NotrumpDefense::DirectDont),
        ("direct-landy", NotrumpDefense::DirectLandy),
    ] {
        let mut a = base;
        a.decision.reading.notrump_defense = defense;
        profiles.push((name, a));
    }
    {
        let mut a = base;
        a.decision.reading.floor_rkcb = true;
        profiles.push(("kickback", a));
    }

    let mut worklist = Vec::new();
    for (name, agreements) in profiles {
        let system = american(&agreements);
        for (phase, trie) in [
            ("constructive", &system.constructive.0),
            ("competitive", &system.competitive.0),
            ("defensive", &system.defensive.0),
        ] {
            worklist.extend(unalerted_artificial(
                &format!("{name}/{phase}"),
                trie,
                agreements.decision,
            ));
        }
    }
    assert_all_alerted("gated profiles", worklist);
}

/// Under `completion_alerts`, every completion lane's reading admits the
/// hand that bid it — the completion-family twin of
/// [`readings_admit_the_bidder`]
///
/// The structural `artificial` witness cannot see a forced completion (its
/// constraint is vacuous, so nothing floors a foreign suit), which is exactly
/// how the Lebensohl `3♣` read as four clubs for months.  No predicate can
/// derive "this face is not a holding" from a `hcp(0..)` rule — so the check
/// is behavioural instead: replay the *bidder* through each completion lane
/// on the knob-on build and require the reading to admit the hand.  A future
/// completion authored without its `.alert_if(completion_alerts, ...)` tag
/// goes red here (the walk stamps its face suit; real hands get excluded)
/// rather than silently lying to every net downstream.
///
/// Knob-on only: at the default (off) the family deliberately keeps the old
/// readings until its A/B ships — this test is the repair's pin, not the
/// default's.
#[test]
fn completion_readings_admit_the_bidder() {
    use rand::SeedableRng as _;

    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.envelope_union = true;
    agreements.decision.reading.completion_alerts = true;

    let nodes: &[(&str, &[Call])] = &[
        (
            "Jacoby heart completion",
            &[
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(2, Strain::Diamonds),
                Call::Pass,
            ],
        ),
        (
            "Jacoby spade completion",
            &[
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(2, Strain::Hearts),
                Call::Pass,
            ],
        ),
        (
            "Jacoby completion over their double",
            &[
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(2, Strain::Diamonds),
                Call::Double,
            ],
        ),
        (
            "Stayman answer",
            &[
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(2, Strain::Clubs),
                Call::Pass,
            ],
        ),
        (
            "Texas completion",
            &[
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(4, Strain::Diamonds),
                Call::Pass,
            ],
        ),
        (
            "minor-transfer answer (2♠ → clubs)",
            &[
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(2, Strain::Spades),
                Call::Pass,
            ],
        ),
        (
            "diamond-transfer answer (2NT)",
            &[
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(2, Strain::Notrump),
                Call::Pass,
            ],
        ),
        (
            "Puppet answer",
            &[
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(3, Strain::Clubs),
                Call::Pass,
            ],
        ),
        (
            "advance-sohl completion (their weak two, our X)",
            &[
                bid(2, Strain::Spades),
                Call::Double,
                Call::Pass,
                bid(2, Strain::Notrump),
                Call::Pass,
            ],
        ),
        (
            "lebensohl-family completion over their 2♠",
            &[
                bid(1, Strain::Notrump),
                bid(2, Strain::Spades),
                bid(2, Strain::Notrump),
                Call::Pass,
            ],
        ),
    ];

    let mut rng = rand::rngs::StdRng::seed_from_u64(0xC0);
    let hands: Vec<Hand> = crate::bidding::verify::random_hands(&mut rng)
        .take(256)
        .collect();

    let mut failures: Vec<String> = Vec::new();
    for (scope, ceilings) in READING_REGIMES {
        agreements.decision.reading.scope = scope;
        agreements.decision.reading.strength_ceilings = ceilings;
        let partnership = crate::american(&agreements).bind();
        for &(what, node) in nodes {
            for &hand in &hands {
                // Honest route only, as in `readings_admit_the_bidder`.
                if (node.len() % 4..node.len())
                    .step_by(4)
                    .any(|i| chosen_call(&partnership, hand, &node[..i]) != node[i])
                {
                    continue;
                }
                let made = chosen_call(&partnership, hand, node);
                let mut read: Vec<Call> = node.to_vec();
                read.push(made);
                read.push(Call::Pass);
                let inferences = partnership.infer(RelativeVulnerability::NONE, &read);
                if !inferences.admits(Relative::Partner, hand) && failures.len() < 16 {
                    failures.push(format!(
                        "{what} [{}] ({scope:?} scope, ceilings {ceilings}) excludes the hand that bid it: {hand}",
                        contract_bridge::auction::display_calls(&read),
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} completion readings exclude their own bidders:\n{}",
        failures.len(),
        failures.join("\n"),
    );
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
/// `ReadingProfile::nt_splinter` shipped, and only the count moved.
///
/// The `[their-landy]` section is the **anchor delta**: the fixture's flat
/// list is the default build, but the anchor arms `their.two_clubs_landy`
/// and `their.two_diamonds_multi` (derived off BBA's measured 2♣/2♦ in
/// `bba-gen`), and a gadget gated on a `TheirDisclosures` field is invisible
/// to the default count — the Landy counter shipped three slugs this file
/// never carried, the Multi counter three more.  The section lists each slug
/// whose count moves under that gate, so the fielded system's disclosure
/// surface is what the tripwire actually watches.  `[multi-stopper]` is the
/// default-off stopper-ask delta; both continuation modes must expose the
/// same artificial ask sites.
#[test]
fn alerted_call_sites_match_the_disclosure_fixture() {
    use crate::bidding::agreements::Agreements;
    use crate::bidding::american::american;
    use std::collections::{BTreeMap, BTreeSet};

    fn alert_site_counts(agreements: &Agreements) -> BTreeMap<&'static str, usize> {
        let system = american(agreements);
        let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
        for trie in [
            &system.constructive.0,
            &system.competitive.0,
            &system.defensive.0,
        ] {
            let mut visit =
                |_auction: &[Call], _context: &Context<'_>, rule: &crate::bidding::rules::Rule| {
                    if let Some(alert) = rule.alert() {
                        *counts.entry(alert.0).or_default() += 1;
                    }
                };
            for_each_authored_rule(trie, agreements.decision, &mut visit);
            // Fallback-attached rules too: row packages lower their patterns
            // to guarded fallbacks, and an exact-node count under-describes
            // the fielded book (the Landy counter has no exact node at all).
            for_each_fallback_rule(trie, agreements.decision, &mut visit, |_auction, _guard| {});
        }
        counts
    }

    let default_counts = alert_site_counts(&Agreements::default());
    let mut anchor = Agreements::default();
    anchor.decision.their.two_clubs_landy = true;
    anchor.decision.their.two_diamonds_multi = true;
    let anchor_counts = alert_site_counts(&anchor);
    let mut search = anchor;
    search.competition.multi_stopper_ask = crate::bidding::american::MultiStopperAsk::FitSearch;
    let search_counts = alert_site_counts(&search);
    let mut place = anchor;
    place.competition.multi_stopper_ask = crate::bidding::american::MultiStopperAsk::OpenerPlaces;
    let place_counts = alert_site_counts(&place);
    assert_eq!(
        search_counts, place_counts,
        "both stopper continuations must expose the same artificial calls"
    );

    let mut found = default_counts
        .iter()
        .map(|(slug, count)| format!("{slug} {count}\n"))
        .collect::<String>();
    found.push_str("\n[their-landy]\n");
    let slugs: BTreeSet<&str> = default_counts
        .keys()
        .chain(anchor_counts.keys())
        .copied()
        .collect();
    for slug in slugs {
        let before = default_counts.get(slug).copied().unwrap_or_default();
        let after = anchor_counts.get(slug).copied().unwrap_or_default();
        if before != after {
            found.push_str(&format!("{slug} {before} -> {after}\n"));
        }
    }
    found.push_str("\n[multi-stopper]\n");
    let slugs: BTreeSet<&str> = anchor_counts
        .keys()
        .chain(search_counts.keys())
        .copied()
        .collect();
    for slug in slugs {
        let before = anchor_counts.get(slug).copied().unwrap_or_default();
        let after = search_counts.get(slug).copied().unwrap_or_default();
        if before != after {
            found.push_str(&format!("{slug} {before} -> {after}\n"));
        }
    }
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
    crate::bidding::context::DecisionProfile,
    &mut dyn FnMut(&[Call], &Context<'_>, &crate::bidding::rules::Rule),
);

fn axis_leaks(
    tries: &[(&str, &crate::bidding::trie::Trie)],
    profile: crate::bidding::context::DecisionProfile,
) -> std::collections::BTreeMap<&'static str, Vec<String>> {
    axis_leaks_with(tries, profile, |trie, profile, visit| {
        for_each_authored_rule(trie, profile, visit);
    })
}

fn axis_leaks_with(
    tries: &[(&str, &crate::bidding::trie::Trie)],
    profile: crate::bidding::context::DecisionProfile,
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
        walk(trie, profile, &mut |_, context, rule| {
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

    // ponytail: 128 hands is the whole cost dial.  The sweep is ~72s, and
    // measured, that is 110 368 (auction, rule) pairs × 132 hands: ~41s in
    // `Envelope::accepts` (13.9M calls, of which ~6s is re-reading the
    // reading profile out of thread-locals per box), ~29s in `Rule::eval`,
    // and under 1s each in projection and in `Inferences::read` — the reads
    // are memoised per node by `node_context` above, 5 042 of them rather
    // than 1.26M.  So the pool scales the sweep almost linearly and nothing
    // else here is cacheable.  Crank it when hunting a specific leak.
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

    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.envelope_union = true;
    let american = american(&agreements);
    let dutch = dutch(&agreements);
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
                        contract_bridge::auction::display_calls(auction),
                        rule.call(),
                    ));
                }
            }
        }
    }

    let mut failures: Vec<String> = Vec::new();
    for (system, trie) in tries {
        for_each_authored_rule(trie, agreements.decision, |auction, context, rule| {
            check(&mut failures, &hands, system, auction, context, rule);
        });
        // The same soundness claim for fallback-authored rules — the layer
        // the exact-node walk cannot see (`docs/ai-bidder/sampled-projection.md`
        // census: the meter blind spot).  Asserts, not pins: soundness has
        // no acceptable nonzero.
        for_each_fallback_rule(
            trie,
            agreements.decision,
            |auction, context, rule| {
                check(&mut failures, &hands, system, auction, context, rule);
            },
            |_, _| {},
        );
    }

    // Phase 1's general soundness gate: the same book-wide claim with the
    // forward strength folds two-sided
    // ([`strength_ceilings`][field@crate::bidding::ReadingProfile::strength_ceilings]).
    //
    // The algebra says this must hold — under the knob the three point
    // gauges' forward fold *is* `project_band`, which the sweep above
    // already replays book-wide, and every combinator builds its forward
    // fold out of its arms' — but "must hold" is exactly the reasoning that
    // put a floor-only ceiling in the book for two years.  A quarter of the
    // hand pool, because the sweep is hand-proportional and this axis only
    // moves the strength bounds: 36 hands over ~110k (auction, rule) pairs
    // still replays every ceiling in the book many times over.
    let ceilinged = &hands[..32.min(hands.len())]
        .iter()
        .copied()
        .chain(hands[hands.len() - 4..].iter().copied())
        .collect::<Vec<Hand>>();
    let mut with_ceilings = agreements.decision;
    with_ceilings.reading.strength_ceilings = true;
    for (system, trie) in tries {
        for_each_authored_rule(trie, with_ceilings, |auction, context, rule| {
            check(&mut failures, ceilinged, system, auction, context, rule);
        });
        for_each_fallback_rule(
            trie,
            with_ceilings,
            |auction, context, rule| {
                check(&mut failures, ceilinged, system, auction, context, rule);
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
/// - **union off** ([`ReadingProfile::envelope_union`]) — the legacy reading;
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

    let american = american(&crate::bidding::agreements::Agreements::default());
    let dutch = dutch(&crate::bidding::agreements::Agreements::default());
    let tries: [(&str, &crate::bidding::trie::Trie); 4] = [
        ("american constructive", &american.constructive.0),
        ("american competitive", &american.competitive.0),
        ("american defensive", &american.defensive.0),
        ("dutch constructive", &dutch.constructive.0),
    ];

    let mut off_profile = crate::bidding::context::DecisionProfile::default();
    off_profile.reading.envelope_union = false;
    let off = axis_leaks(&tries, off_profile);
    let mut on_profile = off_profile;
    on_profile.reading.envelope_union = true;
    let on = axis_leaks(&tries, on_profile);

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
        // 59 → 65 when the diamond splinters after `1NT - 2NT` went
        // default-on (2026-08-13).  The six are the same two calls (`3♥`,
        // `3♠`) in each of american constructive/defensive and dutch: they
        // gate on the `2NT` transfer's shape class, whose `6+ ♦ | 5+ ♦ & 4+
        // ♣` disjunction the legacy hull cannot pin on the length axis.
        // Knob-on stays 0 — the envelope union projects the union exactly,
        // which is the whole point of the DNF migration.
        // 65 → 67 and points 9 → 11 when the two Michaels preferences
        // acquired their exact `!(10+ points & 3+ trumps)` readings.  The
        // legacy hull cannot preserve either disjunct; envelope union does.
        // 67 → 75 when N3's `1NT (3x)` table shipped default-on (2026-08-18):
        // its seven natural-suit rules gate `at_least_as_long`, a comparative
        // staircase the legacy hull cannot pin on the length axis, and its
        // takeout `X` over a minor gates the `4+ ♥ | 4+ ♠` disjunction.  The
        // fielded reading is unaffected — `envelope_union` ships on and its
        // column stays 0, which is what the migration bought.
        // 75 → 85 when opener's three N3 answer tables took the same guard
        // (2026-08-19): five rules per major (`3+`/`4+`/`4+ & 17+ points`
        // tolerance and jump rungs of the double answer, the `5+` rung of the
        // four-level-minor answer) now gate `at_least_as_long` against the
        // rival major, so a 5-4 answers in its five-carder instead of losing
        // the cross-call weight tie to the call encoding.  Same staircase, same
        // blind spot, knob-on still 0.
        ("length", 85, 0),
        ("points", 11, 0),
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
        // exact `1x (overcall)` nodes where the support axis is live.
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

    let american = american(&crate::bidding::agreements::Agreements::default());
    let dutch = dutch(&crate::bidding::agreements::Agreements::default());
    let tries: [(&str, &crate::bidding::trie::Trie); 4] = [
        ("american constructive", &american.constructive.0),
        ("american competitive", &american.competitive.0),
        ("american defensive", &american.defensive.0),
        ("dutch constructive", &dutch.constructive.0),
    ];
    let walk: RuleWalk = |trie, profile, visit| {
        for_each_fallback_rule(trie, profile, visit, |_, _| {});
    };

    let mut off_profile = crate::bidding::context::DecisionProfile::default();
    off_profile.reading.envelope_union = false;
    let off = axis_leaks_with(&tries, off_profile, walk);
    let mut on_profile = off_profile;
    on_profile.reading.envelope_union = true;
    let on = axis_leaks_with(&tries, on_profile, walk);

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
            crate::bidding::context::DecisionProfile::default(),
            |_, _, _| {},
            |auction, label| {
                opaque.push(format!(
                    "{system}: [{}] guard: {}",
                    contract_bridge::auction::display_calls(auction),
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
    use crate::bidding::american::american;

    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.nt_overcall_gladiator = true;
    let system = american(&agreements);

    assert_all_alerted(
        "Gladiator",
        unalerted_artificial("defensive", &system.defensive.0, agreements.decision),
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

    let agreements = crate::bidding::agreements::Agreements::default();
    let system = dutch(&agreements);
    assert_all_alerted(
        "Dutch",
        unalerted_artificial("constructive", &system.constructive.0, agreements.decision),
    );
}

/// The same alert invariant for the opt-in New Minor Forcing book (off by
/// default, so the shipped-system walk never sees it).  Guards the one
/// artificial call NMF adds — responder's `2`-of-the-new-minor checkback —
/// against losing its `.alert(...)` and reading as a phantom minor suit.
#[test]
fn new_minor_forcing_artificial_calls_are_alerted() {
    use crate::bidding::american::american;

    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.rebid.new_minor_forcing = true;
    let system = american(&agreements);

    assert_all_alerted(
        "New Minor Forcing",
        unalerted_artificial("constructive", &system.constructive.0, agreements.decision),
    );
}

/// The same alert invariant for the opt-in choice-of-games 3NT and 2/1
/// fit-leg books (off by default, so the shipped-system walk never sees
/// them).
#[test]
fn choice_of_games_artificial_calls_are_alerted() {
    use crate::bidding::american::american;

    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.response.major_choice_of_games = true;
    let system = american(&agreements);

    assert_all_alerted(
        "choice-of-games",
        unalerted_artificial("constructive", &system.constructive.0, agreements.decision),
    );
}

/// The same alert invariant for the opt-in **European** 1NT minor scheme — the
/// opponent model in [`european`][crate::bidding::american::notrump::european].
///
/// `artificial_calls_are_alerted` walks `american()` at the Puppet default and
/// never sees a European row, which is how that scheme's club-transfer
/// continuations kept an unalerted rung for months.
#[test]
fn european_minors_artificial_calls_are_alerted() {
    use crate::bidding::american::{EUROPEAN, american};

    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.notrump_minors = EUROPEAN;
    let system = american(&agreements);

    assert_all_alerted(
        "European minors",
        unalerted_artificial("constructive", &system.constructive.0, agreements.decision),
    );
}

/// `reading.completion_alerts` (né `lebensohl_completion_alert`) — the forced `3♣` completion of a
/// sohl `2NT` relay is a puppet, but constrained `hcp(0..)` it projects
/// nothing, dodges the artificiality witness, and (unalerted) is read by the
/// natural walk as a **club holding**.  The knob alerts it, which decodes it
/// as ⊤ and suppresses the club read.
///
/// Asserted in the lane where the defect is *live*: advance-sohl, where our
/// side did not open and no notrump-structure blanket applies.  (After our
/// own `1NT` opening the whole structure is already walk-blanketed, so the
/// same completion is latent there under every minors scheme.)
#[test]
fn lebensohl_completion_alert_suppresses_the_club_reading() {
    use crate::bidding::inference::envelope::Range;

    // (2♠) X - 2NT - 3♣ -: their weak two, our takeout double, the advancer's
    // sohl relay, and partner's forced completion; advancer to act.
    let auction = [
        bid(2, Strain::Spades),
        Call::Double,
        Call::Pass,
        bid(2, Strain::Notrump),
        Call::Pass,
        bid(3, Strain::Clubs),
        Call::Pass,
    ];
    let mut arm = crate::bidding::agreements::Agreements::default();

    // On: the double claimed no shape, and the puppet claims none either.
    arm.decision.reading.completion_alerts = true;
    let on = read_booked_with(&arm, &auction);
    assert_eq!(on.partner().length(Suit::Clubs), Range::FULL_LENGTH);

    // Off — the shipped default — is the defect this knob exists to fix: the
    // forced completion reads as four real clubs.
    arm.decision.reading.completion_alerts = false;
    let off = read_booked_with(&arm, &auction);
    assert_eq!(off.partner().length(Suit::Clubs), Range::new(4, 13));
}

/// N4e's floorless escape must stay out of `(1x) 1NT (2♦)`.
///
/// Their `(2♦)` there is a response to their own one-suit opening, never the
/// Multi `their.two_diamonds_multi` declares — but the systems-on strip
/// re-reads the lane against the competition book, whose `1NT (2♦)` leg is
/// chosen Multi-or-natural at *build* time, so clearing the profile flag
/// cannot un-compile the Multi table.  It published the escape and the
/// inference-aware floor bid it: 26 of 260 divergent boards foreign on the
/// campaign's isolation gate, replicated 27/267 on a second seed
/// (`docs/one-notrump-competitive.md` §N4e).  The strip declines this one
/// shape now, so the knob is invisible here — and still live in the lane it
/// owns.
#[test]
fn multi_weak_escape_stays_out_of_the_overcall_lane() {
    let mut base = crate::bidding::agreements::Agreements::default();
    base.decision.their.two_diamonds_multi = true;
    base.competition.multi_weak_escape = None; // pinned: the escape is default-on since 2026-08-22
    let mut armed = base;
    armed.competition.multi_weak_escape = Some(6);

    // `(1♥) 1NT (2♦) 2♠ (3♦)` and its minor twin: both foreign-board shapes
    // off the gate. The knob must not move a single range.
    let overcall_lane: [&[Call]; 2] = [
        &[
            bid(1, Strain::Hearts),
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            bid(2, Strain::Spades),
            bid(3, Strain::Diamonds),
        ],
        &[
            bid(1, Strain::Clubs),
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            bid(2, Strain::Hearts),
            bid(3, Strain::Clubs),
        ],
    ];
    for auction in overcall_lane {
        assert_eq!(
            read_booked_with(&base, auction),
            read_booked_with(&armed, auction),
            "the escape leaked into the 1NT-overcall lane: {auction:?}",
        );
    }

    // The lane the package does own — our `1NT` *opening* — still moves.
    let opening_lane = [
        bid(1, Strain::Notrump),
        bid(2, Strain::Diamonds),
        bid(2, Strain::Spades),
        bid(3, Strain::Diamonds),
    ];
    assert_ne!(
        read_booked_with(&base, &opening_lane),
        read_booked_with(&armed, &opening_lane),
        "the escape stopped reading in the lane it owns",
    );
}
