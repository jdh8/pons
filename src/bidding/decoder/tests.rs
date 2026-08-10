use super::*;
use crate::bidding::array::Logits;
use crate::bidding::constraint::hcp;
use crate::bidding::fallback::{Always, FirstIs, OvercallAtMost, ReplaceNext, SuffixIs};
use crate::bidding::rows::{Entry, Package, Pattern, compile_into, rows_of};
use crate::bidding::rules::Rules;
use crate::bidding::trie::classifier;
use contract_bridge::Strain;
use contract_bridge::auction::RelativeVulnerability;

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

fn marker(id: u8) -> Arc<dyn Classifier> {
    Arc::new(classifier(move |_, _| {
        let _ = id;
        Logits::new()
    }))
}

fn compile(trie: &Trie) -> AuthoringDecoder {
    AuthoringDecoder::compile(trie, &trie.finalize_authoring())
}

fn assert_same_resolution(
    trie: &Trie,
    decoder: &AuthoringDecoder,
    context: &Context<'_>,
    prefix: &[Call],
) {
    let expected = trie.resolve(context, prefix);
    let actual = decoder.resolve(context, prefix);
    match (expected, actual) {
        (None, None) => {}
        (Some((expected_classifier, expected_provenance)), Some(actual)) => {
            assert!(
                core::ptr::eq(expected_classifier, actual.classifier),
                "classifier identity differs at {prefix:?}",
            );
            assert_eq!(
                actual.provenance, expected_provenance,
                "provenance differs at {prefix:?}",
            );
        }
        (expected, actual) => panic!(
            "resolution presence differs at {prefix:?}: trie={}, decoder={}",
            expected.is_some(),
            actual.is_some(),
        ),
    }
}

fn assert_every_prefix(trie: &Trie, decoder: &AuthoringDecoder, auction: &[Call]) {
    for depth in 0..=auction.len() {
        let prefix = &auction[..depth];
        let context = Context::new(RelativeVulnerability::NONE, prefix);
        assert_same_resolution(trie, decoder, &context, prefix);
    }
}

#[test]
fn exact_and_structured_guards_match_trie() {
    let exact = marker(1);
    let first = marker(2);
    let up_to = marker(3);
    let suffix = marker(4);
    let undisturbed = marker(5);
    let mut trie = Trie::new();

    let exact_key = [bid(1, Strain::Clubs), Call::Pass];
    trie.insert_arc(&exact_key, Arc::clone(&exact));

    let first_key = [bid(1, Strain::Diamonds)];
    trie.fallback_at(
        &first_key,
        FirstIs(Call::Double),
        Fallback::Classify(Arc::clone(&first)),
    );

    let up_to_key = [bid(1, Strain::Hearts)];
    trie.fallback_at(
        &up_to_key,
        OvercallAtMost(Bid::new(2, Strain::Spades)),
        Fallback::Classify(Arc::clone(&up_to)),
    );

    let suffix_key = [bid(1, Strain::Spades)];
    trie.fallback_at(
        &suffix_key,
        SuffixIs(vec![Call::Double, Call::Pass]),
        Fallback::Classify(Arc::clone(&suffix)),
    );

    let undisturbed_key = [bid(1, Strain::Notrump)];
    trie.fallback_at(
        &undisturbed_key,
        super::super::fallback::Undisturbed,
        Fallback::Classify(Arc::clone(&undisturbed)),
    );

    let decoder = compile(&trie);
    let first_auction = [first_key[0], Call::Double, Call::Pass];
    let up_to_auction = [up_to_key[0], bid(2, Strain::Hearts)];
    let suffix_auction = [suffix_key[0], Call::Double, Call::Pass];
    let undisturbed_auction = [undisturbed_key[0], Call::Pass];

    for auction in [
        exact_key.as_slice(),
        first_auction.as_slice(),
        up_to_auction.as_slice(),
        suffix_auction.as_slice(),
        undisturbed_auction.as_slice(),
    ] {
        assert_every_prefix(&trie, &decoder, auction);
    }

    let exact_context = Context::new(RelativeVulnerability::NONE, &exact_key);
    let decoded = decoder
        .resolve(&exact_context, &exact_key)
        .expect("exact classifier");
    assert!(core::ptr::eq(decoded.classifier, exact.as_ref()));
    assert_eq!(
        decoded.provenance,
        Provenance {
            depth: exact_key.len(),
            fallback: None,
            rebases: 0,
        },
    );

    let disturbed_auction = [undisturbed_key[0], Call::Double];
    let disturbed_context = Context::new(RelativeVulnerability::NONE, &disturbed_auction);
    assert!(!disturbed_context.undisturbed());
    assert_same_resolution(&trie, &decoder, &disturbed_context, &disturbed_auction);
    assert!(
        decoder
            .resolve(&disturbed_context, &disturbed_auction)
            .is_none()
    );
}

#[test]
fn opaque_guard_matches_trie() {
    let classified = marker(10);
    let key = [bid(2, Strain::Clubs)];
    let auction = [key[0], Call::Redouble, Call::Pass];
    let mut trie = Trie::new();
    trie.fallback_at(
        &key,
        super::super::fallback::guard(|_: &Context<'_>, suffix: &[Call]| {
            suffix.first() == Some(&Call::Redouble)
        }),
        Fallback::Classify(Arc::clone(&classified)),
    );

    let decoder = compile(&trie);
    assert_eq!(decoder.opaque_sites(), 1);
    assert_every_prefix(&trie, &decoder, &auction);

    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let decoded = decoder.resolve(&context, &auction).expect("opaque guard");
    assert!(core::ptr::eq(decoded.classifier, classified.as_ref()));
    assert_eq!(
        decoded.provenance,
        Provenance {
            depth: key.len(),
            fallback: Some(0),
            rebases: 0,
        },
    );
    assert!(!decoded.fast);
}

#[test]
fn typed_and_opaque_rebases_match_trie() {
    let typed_target = marker(20);
    let opaque_target = marker(21);
    let typed_key = [bid(2, Strain::Diamonds)];
    let opaque_key = [bid(2, Strain::Hearts)];
    let typed_query = [typed_key[0], Call::Double, Call::Pass];
    let opaque_query = [opaque_key[0], Call::Double, Call::Pass];
    let mut trie = Trie::new();

    trie.insert_arc(
        &[typed_key[0], Call::Pass, Call::Pass],
        Arc::clone(&typed_target),
    );
    trie.fallback_at(
        &typed_key,
        FirstIs(Call::Double),
        Fallback::rebase(ReplaceNext(Call::Pass)),
    );

    trie.insert_arc(
        &[opaque_key[0], Call::Pass, Call::Pass],
        Arc::clone(&opaque_target),
    );
    trie.fallback_at(
        &opaque_key,
        FirstIs(Call::Double),
        Fallback::rebase(super::super::fallback::rewriter(
            |auction: &[Call], depth: usize| {
                (depth < auction.len()).then(|| {
                    let mut rewritten = auction.to_vec();
                    rewritten[depth] = Call::Pass;
                    rewritten
                })
            },
        )),
    );

    let decoder = compile(&trie);
    assert_eq!(decoder.opaque_sites(), 1);
    for auction in [&typed_query[..], &opaque_query[..]] {
        assert_every_prefix(&trie, &decoder, auction);
        let context = Context::new(RelativeVulnerability::NONE, auction);
        let decoded = decoder
            .resolve(&context, auction)
            .expect("rebased exact node");
        assert_eq!(
            decoded.provenance,
            Provenance {
                depth: auction.len(),
                fallback: None,
                rebases: 1,
            },
        );
    }

    let typed_context = Context::new(RelativeVulnerability::NONE, &typed_query);
    let typed = decoder
        .resolve(&typed_context, &typed_query)
        .expect("typed rebase");
    assert!(core::ptr::eq(typed.classifier, typed_target.as_ref()));
    assert!(typed.fast);

    let opaque_context = Context::new(RelativeVulnerability::NONE, &opaque_query);
    let opaque = decoder
        .resolve(&opaque_context, &opaque_query)
        .expect("opaque rebase");
    assert!(core::ptr::eq(opaque.classifier, opaque_target.as_ref()));
    assert!(!opaque.fast);
}

#[test]
fn fallback_depth_and_declaration_order_match_trie() {
    let root = marker(30);
    let first_deep = marker(31);
    let second_deep = marker(32);
    let exact = marker(33);
    let key = [Call::Pass];
    let mut trie = Trie::new();
    trie.fallback_at(&[], Always, Fallback::Classify(Arc::clone(&root)));
    trie.fallback_at(&key, Always, Fallback::Classify(Arc::clone(&first_deep)));
    trie.fallback_at(&key, Always, Fallback::Classify(Arc::clone(&second_deep)));
    trie.insert_arc(&key, Arc::clone(&exact));

    let decoder = compile(&trie);
    let auction = [Call::Pass, Call::Redouble];
    assert_every_prefix(&trie, &decoder, &auction);

    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let decoded = decoder.resolve(&context, &auction).expect("deep fallback");
    assert!(core::ptr::eq(decoded.classifier, first_deep.as_ref()));
    assert_eq!(
        decoded.provenance,
        Provenance {
            depth: 1,
            fallback: Some(0),
            rebases: 0,
        },
    );
}

#[test]
fn cursor_matches_trie_through_forward_scan() {
    let floor = marker(40);
    let contested = marker(41);
    let exact = marker(42);
    let mut trie = Trie::new();
    trie.fallback_at(&[], Always, Fallback::Classify(Arc::clone(&floor)));
    trie.fallback_at(
        &[Call::Pass],
        FirstIs(Call::Double),
        Fallback::Classify(Arc::clone(&contested)),
    );
    trie.insert_arc(&[Call::Pass, Call::Double, Call::Pass], Arc::clone(&exact));
    let decoder = compile(&trie);
    let auction = [Call::Pass, Call::Double, Call::Pass, bid(3, Strain::Clubs)];
    let mut cursor = decoder.cursor();

    for depth in 0..=auction.len() {
        let prefix = &auction[..depth];
        let context = Context::new(RelativeVulnerability::NONE, prefix);
        assert_same_resolution(&trie, &decoder, &context, prefix);

        let expected = trie.resolve(&context, prefix);
        let actual = cursor.resolve(&context, prefix);
        match (expected, actual) {
            (Some((expected_classifier, expected_provenance)), Some(actual)) => {
                assert!(
                    core::ptr::eq(expected_classifier, actual.classifier),
                    "cursor classifier identity differs at {prefix:?}",
                );
                assert_eq!(
                    actual.provenance, expected_provenance,
                    "cursor provenance differs at {prefix:?}",
                );
            }
            (None, None) => {}
            _ => panic!("cursor resolution presence differs at {prefix:?}"),
        }
    }
}

#[test]
fn deal_cursor_matches_trie_for_every_structured_guard_and_rebase() {
    let floor = marker(43);
    let first = marker(44);
    let up_to = marker(45);
    let suffix = marker(46);
    let rebased = marker(47);
    let exact = marker(48);
    let mut trie = Trie::new();
    trie.fallback_at(&[], Always, Fallback::Classify(Arc::clone(&floor)));
    trie.fallback_at(
        &[Call::Pass],
        FirstIs(Call::Double),
        Fallback::Classify(Arc::clone(&first)),
    );
    trie.fallback_at(
        &[Call::Pass, Call::Pass],
        OvercallAtMost(Bid::new(2, Strain::Spades)),
        Fallback::Classify(Arc::clone(&up_to)),
    );
    trie.fallback_at(
        &[Call::Pass, Call::Pass, Call::Pass],
        SuffixIs(vec![Call::Double, Call::Pass]),
        Fallback::Classify(Arc::clone(&suffix)),
    );
    let rebase_key = bid(1, Strain::Clubs);
    trie.insert_arc(&[rebase_key, Call::Pass, Call::Pass], Arc::clone(&rebased));
    trie.fallback_at(
        &[rebase_key],
        FirstIs(Call::Double),
        Fallback::rebase(ReplaceNext(Call::Pass)),
    );
    trie.insert_arc(
        &[
            Call::Pass,
            Call::Pass,
            Call::Pass,
            Call::Double,
            Call::Pass,
            Call::Pass,
        ],
        Arc::clone(&exact),
    );
    let decoder = compile(&trie);

    let auctions = [
        vec![Call::Pass, Call::Double, Call::Pass],
        vec![rebase_key, Call::Double, Call::Pass],
        vec![Call::Pass, Call::Pass, bid(2, Strain::Hearts), Call::Pass],
        vec![
            Call::Pass,
            Call::Pass,
            Call::Pass,
            Call::Double,
            Call::Pass,
            Call::Pass,
        ],
    ];
    for auction in auctions {
        let mut state = DecoderCursorState::default();
        for depth in 0..=auction.len() {
            let prefix = &auction[..depth];
            let context = Context::new(RelativeVulnerability::NONE, prefix);
            let expected = trie.resolve(&context, prefix);
            let actual = match decoder.resolve_checked_with_cursor(&mut state, &context, prefix) {
                CheckedResolution::Decoded(answer) => answer,
                CheckedResolution::Opaque => panic!("structured route became opaque"),
            };
            match (expected, actual) {
                (None, None) => {}
                (Some((expected_classifier, expected_provenance)), Some(actual)) => {
                    assert!(core::ptr::eq(expected_classifier, actual.classifier));
                    assert_eq!(expected_provenance, actual.provenance, "at {prefix:?}");
                }
                _ => panic!("deal cursor differs at {prefix:?}"),
            }
        }
    }
}

#[test]
fn deal_cursor_fallback_depth_visits_are_linear_not_triangular() {
    const DEPTH: usize = 32;
    let floor = marker(49);
    let exact = marker(50);
    let auction = vec![Call::Pass; DEPTH];
    let mut trie = Trie::new();
    trie.fallback_at(&[], Always, Fallback::Classify(Arc::clone(&floor)));
    trie.insert_arc(&auction, Arc::clone(&exact));
    let decoder = compile(&trie);
    let mut state = DecoderCursorState::default();

    for depth in 0..=auction.len() {
        let prefix = &auction[..depth];
        let context = Context::new(RelativeVulnerability::NONE, prefix);
        let expected = trie.resolve(&context, prefix).expect("root floor or exact");
        let actual = match decoder.resolve_checked_with_cursor(&mut state, &context, prefix) {
            CheckedResolution::Decoded(Some(answer)) => answer,
            _ => panic!("expected cache-safe structured answer at {prefix:?}"),
        };
        assert!(core::ptr::eq(expected.0, actual.classifier));
        assert_eq!(expected.1, actual.provenance);
    }

    assert_eq!(
        state.routes.depth_visits(),
        DEPTH,
        "each non-exact prefix should consult only the root candidate depth",
    );
}

#[test]
fn rejected_dynamic_guard_makes_cursor_result_uncacheable() {
    let selected = marker(50);
    let rejected = marker(51);
    let mut trie = Trie::new();
    trie.fallback_at(
        &[],
        super::super::fallback::guard(|_: &Context<'_>, _: &[Call]| false),
        Fallback::Classify(rejected),
    );
    trie.fallback_at(&[], Always, Fallback::Classify(Arc::clone(&selected)));

    let decoder = compile(&trie);
    let context = Context::new(RelativeVulnerability::NONE, &[]);
    let direct = decoder.resolve(&context, &[]).expect("later fallback");
    assert!(core::ptr::eq(direct.classifier, selected.as_ref()));
    assert!(!direct.fast);

    let mut state = DecoderCursorState::default();
    let cached = decoder
        .resolve_with_cursor(&mut state, &context, &[])
        .expect("later fallback through state cursor");
    assert!(core::ptr::eq(cached.classifier, selected.as_ref()));
    assert!(!cached.fast);
    assert!(!state.cache_stable());

    let mut no_answer = Trie::new();
    no_answer.fallback_at(
        &[],
        super::super::fallback::guard(|_: &Context<'_>, _: &[Call]| false),
        Fallback::Classify(marker(52)),
    );
    let no_answer = compile(&no_answer);
    let mut state = DecoderCursorState::default();
    assert!(
        no_answer
            .resolve_with_cursor(&mut state, &context, &[])
            .is_none()
    );
    assert!(!state.cache_stable());
}

#[test]
fn streaming_ledger_compile_matches_catalog_with_grafts_and_overwrites() {
    fn entries() -> Vec<Entry> {
        let mut entries = rows_of(
            Pattern::node("P* (1♣)"),
            Rules::new().rule(Call::Pass, 0, hcp(0..)),
        );
        entries.extend(rows_of(
            Pattern::first("P* 1♦", "X"),
            Rules::new().rule(Call::Pass, 0, hcp(0..)),
        ));
        entries
    }

    let mut source = Trie::new();
    compile_into(
        &mut source,
        &crate::bidding::agreements::Agreements::default(),
        &[Package {
            name: "decoder-streaming-test",
            gate: |_| true,
            entries: |_| entries(),
        }],
    );
    let mut trie = source.clone();
    let original = [bid(1, Strain::Clubs)];
    let grafted = [bid(2, Strain::Clubs)];
    assert!(trie.graft(&grafted, &source, &original).is_empty());

    // The original exact slot is stale, but its grafted copy with the same
    // PatternId remains live. Streaming finalization must allocate one
    // metadata entry and attach it only at the live copy.
    let replacement = marker(99);
    trie.insert_arc(&original, Arc::clone(&replacement));

    let catalog = trie.finalize_authoring();
    let catalog_decoder = AuthoringDecoder::compile(&trie, &catalog);
    let ledger = trie.take_authoring_ledger();
    let streaming_decoder = AuthoringDecoder::compile_ledger(&trie, ledger);
    assert!(trie.authoring().patterns().is_empty());
    assert_eq!(
        catalog_decoder.metadata.len(),
        streaming_decoder.metadata.len()
    );
    assert_eq!(streaming_decoder.metadata.len(), 2, "one entry per live ID");

    let fallback = [bid(1, Strain::Diamonds), Call::Double];
    for auction in [&original[..], &grafted[..], &fallback[..]] {
        let context = Context::new(RelativeVulnerability::NONE, auction);
        let expected = catalog_decoder.resolve(&context, auction);
        let actual = streaming_decoder.resolve(&context, auction);
        match (expected, actual) {
            (Some(expected), Some(actual)) => {
                assert!(core::ptr::eq(expected.classifier, actual.classifier));
                assert_eq!(expected.provenance, actual.provenance);
                assert_eq!(expected.pattern_id, actual.pattern_id);
                assert_eq!(expected.table_id, actual.table_id);
                assert_eq!(expected.fast, actual.fast);
            }
            (None, None) => {}
            _ => panic!("streaming/catalog presence differs at {auction:?}"),
        }
    }

    let context = Context::new(RelativeVulnerability::NONE, &original);
    let replaced = streaming_decoder
        .resolve(&context, &original)
        .expect("replacement exact classifier");
    assert!(core::ptr::eq(replaced.classifier, replacement.as_ref()));
    assert_eq!(
        replaced.pattern_id, None,
        "stale authoring was not attached"
    );
}
