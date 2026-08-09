use super::super::constraint::hcp;
use super::super::context::Context;
use super::super::fallback::{Always, ReplaceNext, described_guard, guard};
use super::*;
use contract_bridge::auction::RelativeVulnerability;
use contract_bridge::{Hand, Strain};

fn calls(text: &str) -> Vec<Call> {
    text.split_whitespace()
        .map(|word| word.parse().expect("valid call"))
        .collect()
}

fn two_rule_table() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 100, hcp(10..))
        .rule(Call::Pass, 0, hcp(0..))
}

fn compiled(packages: &[Package]) -> Trie {
    let mut book = Trie::new();
    compile_into(&mut book, &Agreements::current(), packages);
    book
}

/// The tie census sees a rung claimed twice, and only then.
///
/// Three rules on one rung must report as one 3-way line, not the three
/// pairs it decomposes into, and not once per member.
#[test]
fn weight_ties_report_once_per_rung() {
    fn package(entries: fn(&Agreements) -> Vec<Entry>) -> Package {
        Package {
            name: "probe",
            gate: |_| true,
            entries,
        }
    }

    let distinct = package(|_| {
        rows_of(
            Pattern::node("1♥ -"),
            Rules::new()
                .rule(Bid::new(2, Strain::Hearts), 100, hcp(10..))
                .rule(Bid::new(2, Strain::Hearts), 90, hcp(6..))
                .rule(Call::Pass, 0, hcp(0..)),
        )
    });
    assert!(weight_tie_report(&Agreements::current(), &[distinct]).is_empty());

    let tied = package(|_| {
        rows_of(
            Pattern::node("1♥ -"),
            Rules::new()
                .rule(Bid::new(2, Strain::Hearts), 100, hcp(10..))
                .rule(Bid::new(2, Strain::Hearts), 100, hcp(6..))
                .rule(Bid::new(2, Strain::Hearts), 100, hcp(4..))
                // Same rung, different call: a mixed strategy, not a tie.
                .rule(Bid::new(2, Strain::Spades), 100, hcp(10..))
                .rule(Call::Pass, 0, hcp(0..)),
        )
    });
    let report = weight_tie_report(&Agreements::current(), &[tied]);
    assert_eq!(report.len(), 1, "one rung, one line: {report:?}");
    assert!(
        report[0].contains("2♥ at weight 100, 3 rules"),
        "{report:?}"
    );
}

/// Guarded rows lower onto every seat-fanned key, regrouped into one
/// table per pattern with the guard riding along.
#[test]
fn rows_regroup_and_fan() {
    let book = compiled(&[Package {
        name: "test",
        gate: |_| true,
        entries: |_| rows_of(Pattern::up_to("P* 1♥", "2♠"), two_rule_table()),
    }]);

    let entries = book.fallbacks();
    let keys: Vec<&[Call]> = entries.iter().map(|(key, ..)| &**key).collect();
    assert_eq!(
        keys,
        [
            &calls("1♥")[..],
            &calls("- 1♥"),
            &calls("- - 1♥"),
            &calls("- - - 1♥"),
        ],
        "one entry per seat, pass-less key first",
    );
    assert_eq!(
        entries[0].1.describe().as_deref(),
        Some("(overcall ≤2♠)"),
        "the guard lowered to OvercallAtMost",
    );

    let Fallback::Classify(classifier) = entries[0].2 else {
        panic!("rows lower to a classifying fallback");
    };
    let rules = classifier.as_rules().expect("rows regroup into Rules");
    assert_eq!(rules.rules().len(), 2, "both rows in one table");

    let hand: Hand = "AKQ2.KQ53.QJ4.92".parse().expect("valid hand");
    let auction = calls("1♥ 2♣");
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let (logits, provenance) = book
        .classify_floored(hand, &context, &auction)
        .expect("the table answers over their overcall");
    assert!(logits.has_mass());
    assert_eq!(provenance.depth, 1, "found at the [1♥] node");

    let authored = book.authoring().patterns();
    assert_eq!(authored.len(), 1, "one grouped pattern, not one per row");
    assert!(
        authored[0].table_id.is_some(),
        "the rule table has its own ID"
    );
    assert!(
        matches!(authored[0].grammar, PatternGrammar::UpTo(bid) if bid == Bid::new(2, Strain::Spades))
    );
    assert_eq!(
        authored[0].keys.len(),
        4,
        "all concrete seat-fanned keys retained"
    );
    let catalog = book.finalize_authoring();
    assert_eq!(catalog.patterns().len(), 1);
    assert_eq!(catalog.patterns()[0].sites.len(), 4);
    assert!(
        catalog.patterns()[0]
            .sites
            .iter()
            .all(|site| matches!(site.placement, Placement::Fallback { .. }))
    );
}

/// Exceptional legacy sites can set a partial seat fan without growing the
/// auction-string grammar.
#[test]
fn builder_sets_a_partial_fan() {
    let book = compiled(&[Package {
        name: "test",
        gate: |_| true,
        entries: |_| rows_of(Pattern::up_to("1♥", "2♠").with_fan(2), two_rule_table()),
    }]);

    let entries = book.fallbacks();
    let keys: Vec<&[Call]> = entries.iter().map(|(key, ..)| &**key).collect();
    assert_eq!(keys, [&calls("1♥")[..], &calls("- 1♥"), &calls("- - 1♥"),],);
    assert_eq!(book.authoring().patterns()[0].keys.len(), 3);
}

#[test]
#[should_panic(expected = "exceeds the three possible leading passes")]
fn builder_rejects_an_impossible_fan() {
    let _ = Pattern::node("1♥ -").with_fan(4);
}

/// A rebase entry lowers to `Fallback::Rebase` and re-resolves onto the
/// rewritten auction.
#[test]
fn rebase_lowers_and_reresolves() {
    let mut book = compiled(&[Package {
        name: "test",
        gate: |_| true,
        entries: |_| {
            vec![rebase(
                Pattern::first("P* 1♥", "X"),
                ReplaceNext(Call::Pass),
            )]
        },
    }]);
    book.insert(&calls("1♥ - 2♥"), two_rule_table());

    let auction = calls("1♥ X 2♥");
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let (_, provenance) = book
        .resolve(&context, &auction)
        .expect("the rebase reaches the systems-on node");
    assert_eq!(provenance.rebases, 1, "resolved through one rewrite");
    assert_eq!(provenance.depth, 3, "at the [1♥ - 2♥] node");
}

/// `after` splits the key from the guard suffix at the string boundary,
/// and fine-grained `row(...)` rows regroup with the alert riding along.
#[test]
fn after_splits_key_and_suffix() {
    let book = compiled(&[Package {
        name: "test",
        gate: |_| true,
        entries: |_| {
            vec![
                row(
                    Pattern::after("P* 1♥ (X)", "2NT -"),
                    Bid::new(4, Strain::Hearts),
                    100,
                    hcp(13..),
                )
                .alert(Alert("test:conv"))
                .into(),
                row(
                    Pattern::after("P* 1♥ (X)", "2NT -"),
                    Call::Pass,
                    0,
                    hcp(0..),
                )
                .into(),
            ]
        },
    }]);

    let entries = book.fallbacks();
    assert_eq!(
        &*entries[0].0,
        calls("1♥ X"),
        "their double keys, not guards"
    );
    let auction = calls("2NT -");
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    assert!(entries[0].1.admits(&context, &auction));
    assert!(!entries[0].1.admits(&context, &calls("2NT")));

    let Fallback::Classify(classifier) = entries[0].2 else {
        panic!("rows lower to a classifying fallback");
    };
    let rules = classifier.as_rules().expect("rows regroup into Rules");
    assert_eq!(rules.rules().len(), 2, "both rows in one table");
    assert!(rules.rules()[0].alert().is_some(), "the alert rode along");
}

/// A hand-written guard rides onto the trie verbatim — label and all —
/// and its sample is seat-checked against the key.
#[test]
fn guarded_carries_the_guard_verbatim() {
    let book = compiled(&[Package {
        name: "test",
        gate: |_| true,
        entries: |_| {
            vec![rebase(
                Pattern::guarded(
                    "P* 1NT - 2♣",
                    "(X) 2♦",
                    described_guard(
                        "X (bid) …",
                        guard(|_: &Context<'_>, suffix: &[Call]| {
                            suffix.first() == Some(&Call::Double)
                                && matches!(suffix.get(1), Some(Call::Bid(_)))
                        }),
                    ),
                ),
                ReplaceNext(Call::Pass),
            )]
        },
    }]);

    let entries = book.fallbacks();
    assert_eq!(&*entries[0].0, calls("1NT - 2♣"), "keyed below our Stayman");
    assert_eq!(
        entries[0].1.describe().as_deref(),
        Some("X (bid) …"),
        "the guard's own label survives, so render-book is unchanged",
    );

    let auction = calls("1NT - 2♣ X 2♦");
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    assert!(
        entries[0].1.admits(&context, &calls("X 2♦")),
        "wildcard tail"
    );
    assert!(
        !entries[0].1.admits(&context, &calls("X - -")),
        "the re-ask suffix is left to its own table — what FirstIs would swallow",
    );
}

/// A gated-off package compiles to nothing.
#[test]
fn gate_off_compiles_nothing() {
    let book = compiled(&[Package {
        name: "test",
        gate: |_| false,
        entries: |_| rows_of(Pattern::up_to("P* 1♥", "2♠"), two_rule_table()),
    }]);
    assert!(book.fallbacks().is_empty());
    assert_eq!(book.iter().count(), 0);
    assert!(book.authoring().patterns().is_empty());
}

/// IDs name the logical row: clones and grafted concrete routes retain
/// them, while the finalized catalog reports the destination key.
#[test]
fn clone_and_graft_preserve_pattern_identity() {
    let source = compiled(&[Package {
        name: "test",
        gate: |_| true,
        entries: |_| rows_of(Pattern::node("1NT - 2♣ -"), two_rule_table()),
    }]);
    let original = source.authoring().patterns()[0].id;
    assert_eq!(source.clone().authoring().patterns()[0].id, original);

    let mut grafted = Trie::new();
    assert!(
        grafted
            .graft(&calls("1♠ 1NT"), &source, &calls("1NT"))
            .is_empty()
    );
    let catalog = grafted.finalize_authoring();
    assert_eq!(catalog.patterns().len(), 1);
    assert_eq!(catalog.patterns()[0].id, original);
    assert_eq!(&*catalog.patterns()[0].sites[0].key, calls("1♠ 1NT - 2♣ -"));
}

/// A later imperative overwrite is authoritative; displaced row metadata
/// does not leak into finalization (the Dutch opening compatibility case).
#[test]
fn exact_overwrite_drops_displaced_metadata() {
    let mut book = compiled(&[Package {
        name: "test",
        gate: |_| true,
        entries: |_| rows_of(Pattern::node("1♥ -"), two_rule_table()),
    }]);
    assert_eq!(book.finalize_authoring().patterns().len(), 1);
    book.insert(&calls("1♥ -"), Rules::new().rule(Call::Pass, 0, hcp(0..)));
    assert!(book.finalize_authoring().patterns().is_empty());
}

/// Merge keeps the receiver's exact classifier at a collision, while
/// retaining metadata from every non-colliding route in the other trie.
#[test]
fn merge_filters_exact_collision_and_keeps_disjoint_metadata() {
    let mut book = compiled(&[Package {
        name: "receiver",
        gate: |_| true,
        entries: |_| rows_of(Pattern::node("1♥ -"), two_rule_table()),
    }]);
    let receiver_id = book.authoring().patterns()[0].id;

    let other = compiled(&[Package {
        name: "other",
        gate: |_| true,
        entries: |_| {
            let mut entries = rows_of(Pattern::node("1♥ -"), two_rule_table());
            entries.extend(rows_of(Pattern::node("1♠ -"), two_rule_table()));
            entries
        },
    }]);
    let displaced_id = other.authoring().patterns()[0].id;
    let disjoint_id = other.authoring().patterns()[1].id;

    assert_eq!(
        book.merge(other),
        vec![calls("1♥ -").into_boxed_slice()],
        "the shared exact node is the sole collision",
    );

    let catalog = book.finalize_authoring();
    assert_eq!(catalog.patterns().len(), 2);
    assert!(
        catalog
            .patterns()
            .iter()
            .any(|pattern| pattern.id == receiver_id)
    );
    assert!(
        catalog
            .patterns()
            .iter()
            .any(|pattern| pattern.id == disjoint_id)
    );
    assert!(
        catalog
            .patterns()
            .iter()
            .all(|pattern| pattern.id != displaced_id),
        "metadata for the classifier rejected by merge is stale",
    );

    let disjoint = catalog
        .patterns()
        .iter()
        .find(|pattern| pattern.id == disjoint_id)
        .expect("the non-colliding pattern survives");
    assert_eq!(&*disjoint.sites[0].key, calls("1♠ -"));
    assert_eq!(disjoint.sites[0].placement, Placement::Exact);
    assert_eq!(&*disjoint.package, "other");
}

/// Fallback lists concatenate receiver-first on merge, and finalized
/// sites retain the same indices used by runtime provenance.
#[test]
fn merge_preserves_fallback_order_in_catalog_and_runtime() {
    let mut book = compiled(&[Package {
        name: "receiver",
        gate: |_| true,
        entries: |_| rows_of(Pattern::first("1♥", "X"), two_rule_table()),
    }]);
    let receiver_id = book.authoring().patterns()[0].id;
    let other = compiled(&[Package {
        name: "other",
        gate: |_| true,
        entries: |_| rows_of(Pattern::first("1♥", "X"), two_rule_table()),
    }]);
    let other_id = other.authoring().patterns()[0].id;
    assert!(book.merge(other).is_empty());

    let catalog = book.finalize_authoring();
    let receiver = catalog
        .patterns()
        .iter()
        .find(|pattern| pattern.id == receiver_id)
        .expect("receiver metadata survives");
    let other = catalog
        .patterns()
        .iter()
        .find(|pattern| pattern.id == other_id)
        .expect("merged metadata survives");
    assert_eq!(
        receiver.sites[0].placement,
        Placement::Fallback { index: 0 }
    );
    assert_eq!(other.sites[0].placement, Placement::Fallback { index: 1 });

    let auction = calls("1♥ X");
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let (_, provenance) = book
        .resolve(&context, &auction)
        .expect("a fallback answers");
    assert_eq!(provenance.fallback, Some(0), "receiver fallback wins first");
}

/// Cloning and merging can deliberately create two pointer-identical live
/// fallback slots. Both indices belong to the same logical pattern.
#[test]
fn duplicate_live_fallback_slots_are_all_cataloged() {
    let mut book = compiled(&[Package {
        name: "test",
        gate: |_| true,
        entries: |_| rows_of(Pattern::first("1♥", "X"), two_rule_table()),
    }]);
    let id = book.authoring().patterns()[0].id;
    assert!(book.merge(book.clone()).is_empty());

    let catalog = book.finalize_authoring();
    assert_eq!(catalog.patterns().len(), 1, "one logical pattern ID");
    let pattern = &catalog.patterns()[0];
    assert_eq!(pattern.id, id);
    assert_eq!(pattern.sites.len(), 2, "both runtime slots are live");
    assert_eq!(
        pattern
            .sites
            .iter()
            .map(|site| site.placement)
            .collect::<Vec<_>>(),
        [
            Placement::Fallback { index: 0 },
            Placement::Fallback { index: 1 },
        ],
    );
    let expected = calls("1♥");
    assert!(
        pattern
            .sites
            .iter()
            .all(|site| site.key.as_ref() == expected.as_slice()),
    );
}

/// Grafting a rebase row carries its logical identity and fallback site
/// to the destination, and the copied runtime rewrite still resolves.
#[test]
fn graft_preserves_rebase_metadata_and_behavior() {
    let mut source = compiled(&[Package {
        name: "systems-on",
        gate: |_| true,
        entries: |_| vec![rebase(Pattern::first("1NT", "X"), ReplaceNext(Call::Pass))],
    }]);
    source.insert(&calls("1NT - 2♣"), two_rule_table());
    let id = source.authoring().patterns()[0].id;

    let mut grafted = Trie::new();
    assert!(
        grafted
            .graft(&calls("1♠ 1NT"), &source, &calls("1NT"))
            .is_empty()
    );

    let catalog = grafted.finalize_authoring();
    assert_eq!(catalog.patterns().len(), 1);
    let pattern = &catalog.patterns()[0];
    assert_eq!(pattern.id, id);
    assert_eq!(pattern.target, PreservedTargetKind::Rebase);
    assert_eq!(&*pattern.package, "systems-on");
    assert_eq!(&*pattern.sites[0].key, calls("1♠ 1NT"));
    assert_eq!(pattern.sites[0].placement, Placement::Fallback { index: 0 });

    let auction = calls("1♠ 1NT X 2♣");
    let context = Context::new(RelativeVulnerability::NONE, &auction);
    let (_, provenance) = grafted
        .resolve(&context, &auction)
        .expect("the grafted rebase reaches its copied exact node");
    assert_eq!(provenance.rebases, 1);
    assert_eq!(provenance.depth, 4);
}

/// An imperative root floor remains available to resolution but does not
/// become an authored pattern or disturb the row fallback's slot.
#[test]
fn floor_is_not_cataloged_or_confused_with_row_fallback() {
    let mut book = compiled(&[Package {
        name: "row",
        gate: |_| true,
        entries: |_| rows_of(Pattern::first("", "1♣"), two_rule_table()),
    }]);
    book.fallback_at(
        &[],
        Always,
        Fallback::classify(Rules::new().rule(Call::Pass, 0, hcp(0..))),
    );
    assert_eq!(book.fallbacks().len(), 2, "row plus imperative floor");

    let catalog = book.finalize_authoring();
    assert_eq!(catalog.patterns().len(), 1, "only the row is inventoried");
    assert_eq!(catalog.patterns()[0].sites.len(), 1);
    assert!(catalog.patterns()[0].sites[0].key.is_empty());
    assert_eq!(
        catalog.patterns()[0].sites[0].placement,
        Placement::Fallback { index: 0 }
    );

    let row_auction = calls("1♣");
    let row_context = Context::new(RelativeVulnerability::NONE, &row_auction);
    let (_, row_provenance) = book
        .resolve(&row_context, &row_auction)
        .expect("the authored row answers");
    assert_eq!(row_provenance.fallback, Some(0));

    let floor_auction = calls("-");
    let floor_context = Context::new(RelativeVulnerability::NONE, &floor_auction);
    let (_, floor_provenance) = book
        .resolve(&floor_context, &floor_auction)
        .expect("the floor answers outside the row guard");
    assert_eq!(floor_provenance.fallback, Some(1));
}

/// Production finalization transfers package provenance into the flat
/// catalog, drains pending metadata, and leaves runtime classifiers live.
#[test]
fn consuming_finalization_drains_ledger_and_retains_package() {
    let mut book = compiled(&[Package {
        name: "P3:inventory-regression",
        gate: |_| true,
        entries: |_| rows_of(Pattern::node("1♥ -"), two_rule_table()),
    }]);
    assert_eq!(book.authoring().patterns().len(), 1);

    let catalog = book.take_finalized_authoring();
    assert!(book.authoring().patterns().is_empty());
    assert_eq!(catalog.patterns().len(), 1);
    assert_eq!(&*catalog.patterns()[0].package, "P3:inventory-regression");
    assert_eq!(&*catalog.patterns()[0].source, "1♥ -");
    assert_eq!(catalog.patterns()[0].sites[0].placement, Placement::Exact);
    assert!(
        book.get(&calls("1♥ -")).is_some(),
        "draining metadata does not remove the runtime classifier",
    );
    assert!(
        book.take_finalized_authoring().patterns().is_empty(),
        "the pending ledger was consumed exactly once",
    );
}

/// Re-declaring a pattern non-consecutively is an authoring bug: the
/// second run would author a second fallback entry instead of extending
/// the first table.
#[test]
#[should_panic(expected = "re-declared non-consecutively")]
fn non_consecutive_pattern_panics() {
    compiled(&[Package {
        name: "test",
        gate: |_| true,
        entries: |_| {
            let mut entries = rows_of(Pattern::up_to("P* 1♥", "2♠"), two_rule_table());
            entries.push(rebase(
                Pattern::first("P* 1♥", "X"),
                ReplaceNext(Call::Pass),
            ));
            entries.extend(rows_of(Pattern::up_to("P* 1♥", "2♠"), two_rule_table()));
            entries
        },
    }]);
}

/// Parenthesisation is checked against seat alternation: our opening
/// written as theirs fails at build time.
#[test]
#[should_panic(expected = "call 1♥ is ours")]
fn wrong_side_panics() {
    let _ = Pattern::first("P* (1♥)", "X");
}

/// `P*` anywhere but leading is rejected.
#[test]
#[should_panic(expected = "P* is only valid leading")]
fn interior_fan_panics() {
    let _ = Pattern::up_to("1♥ P*", "2♠");
}

/// The sources a template expands to, in declaration order.
fn expansion_sources(source: &str) -> Vec<String> {
    group("test", expand(source, |_| true, |_| two_rule_table()))
        .into_iter()
        .map(|(pattern, _)| pattern.source)
        .collect()
}

/// A template expands to one exact node per surviving assignment; the
/// substituted source round-trips through `Pattern::node`, the binding
/// reaches the table, and the seat fan rides along.
#[test]
fn expand_substitutes_and_round_trips() {
    let mut book = Trie::new();
    compile_entries(
        &mut book,
        "weak-twos",
        expand(
            "P* 2x -",
            |bindings| bindings.suit('x') != Suit::Clubs,
            |bindings| {
                Rules::new()
                    .rule(Bid::new(4, bindings.suit('x').into()), 100, hcp(10..))
                    .rule(Call::Pass, 0, hcp(0..))
            },
        ),
    );
    assert!(
        book.get(&calls("2♣ -")).is_none(),
        "the domain closed ♣ out"
    );
    for suit in ["♦", "♥", "♠"] {
        let rules = book
            .get(&calls(&format!("2{suit} -")))
            .expect("an exact node per assignment")
            .as_rules()
            .expect("rows regroup into Rules");
        assert_eq!(
            rules.rules()[0].call().to_string(),
            format!("4{suit}"),
            "the binding reached the table",
        );
    }
    let patterns = book.authoring().patterns();
    let sources: Vec<&str> = patterns.iter().map(|pattern| &*pattern.source).collect();
    assert_eq!(sources, ["P* 2♦ -", "P* 2♥ -", "P* 2♠ -"]);
    assert_eq!(patterns[0].keys.len(), 4, "the fan rode through");
}

/// Non-ascending assignments are pruned: at one level the strain must
/// rise, leaving the six ascending pairs of sixteen.
#[test]
fn expand_prunes_non_ascending() {
    assert_eq!(expansion_sources("1x (1y)").len(), 6);
}

/// Notrump enters literally only: `iN` is notrump at a level variable.
#[test]
fn notrump_enters_literally() {
    let sources: Vec<String> = group(
        "test",
        expand(
            "P* 1♥ (iN)",
            |bindings| bindings.level('i').get() == 1,
            |_| two_rule_table(),
        ),
    )
    .into_iter()
    .map(|(pattern, _)| pattern.source)
    .collect();
    assert_eq!(sources, ["P* 1♥ (1NT)"]);
}

/// One letter binds once per row: `2x` raises the suit `1x` opened.
#[test]
fn one_letter_binds_once() {
    assert_eq!(
        expansion_sources("1x - 2x -"),
        ["1♣ - 2♣ -", "1♦ - 2♦ -", "1♥ - 2♥ -", "1♠ - 2♠ -"],
    );
}

/// `bid()` of a letter spelling several bids is ambiguous.
#[test]
#[should_panic(expected = "several bids")]
fn ambiguous_bid_lookup_panics() {
    let _ = expand(
        "1x - 2x -",
        |bindings| bindings.bid('x').level.get() == 1,
        |_| two_rule_table(),
    );
}

/// A letter absent from the row has no binding to read.
#[test]
#[should_panic(expected = "no level variable")]
fn missing_letter_panics() {
    let _ = expand(
        "2x -",
        |bindings| bindings.level('i').get() > 0,
        |_| two_rule_table(),
    );
}

/// Case is the variable/literal boundary: `2s` is a suit variable while
/// `2S` and `2♠` spell the same literal key.
#[test]
fn lowercase_is_a_variable_uppercase_a_literal() {
    assert_eq!(expansion_sources("2s -").len(), 4);
    let mut letters = Trie::new();
    compile_entries(
        &mut letters,
        "letters",
        rows_of(Pattern::node("2S -"), two_rule_table()),
    );
    let mut glyphs = Trie::new();
    compile_entries(
        &mut glyphs,
        "glyphs",
        rows_of(Pattern::node("2♠ -"), two_rule_table()),
    );
    assert!(letters.get(&calls("2♠ -")).is_some());
    assert!(glyphs.get(&calls("2♠ -")).is_some());
}

/// A lowercase word where a literal is expected fails loudly instead of
/// riding the case-insensitive upstream parse.
#[test]
#[should_panic(expected = "lowercase")]
fn lowercase_literal_call_panics() {
    let _ = Pattern::node("1♥ (x)");
}

/// `1n` is neither a literal (notrump is `N`) nor a variable word (`n`
/// binds levels).
#[test]
#[should_panic(expected = "level letter")]
fn lowercase_notrump_panics() {
    let _ = expand("1♥ (1n)", |_| true, |_| two_rule_table());
}

/// `M` and `m` are binding keywords over the majors and minors.
#[test]
fn major_minor_keywords_bind() {
    assert_eq!(
        expansion_sources("1m - 1M -"),
        ["1♣ - 1♥ -", "1♣ - 1♠ -", "1♦ - 1♥ -", "1♦ - 1♠ -"],
    );
}

/// `OM` derives from the row's `M` binding.
#[test]
fn other_major_derives_from_major() {
    assert_eq!(expansion_sources("1M - 2OM -"), ["1♥ - 2♠ -", "1♠ - 2♥ -"],);
}

/// A derived keyword without its base has nothing to derive from.
#[test]
#[should_panic(expected = "OM requires M")]
fn other_major_alone_panics() {
    let _ = expand("P* 2OM -", |_| true, |_| two_rule_table());
}

/// Each `.` is a fresh anonymous variable.
#[test]
fn anonymous_slots_are_fresh() {
    assert_eq!(expansion_sources("2. -").len(), 4);
    assert_eq!(expansion_sources("1. - 2. -").len(), 16);
}

/// `P+` is recognized, and deferred until a consumer exists.
#[test]
#[should_panic(expected = "deferred")]
fn leading_plus_is_recognized_but_deferred() {
    let _ = Pattern::node("P+ 2♥ -");
}

/// Quantifiers are leading-only: an internal pass run would flip the
/// side parity of every later call.
#[test]
#[should_panic(expected = "P+ is only valid leading")]
fn interior_plus_panics() {
    let _ = expand("2♥ P+ -", |_| true, |_| two_rule_table());
}

/// An expansion nothing survives is an authoring bug, not a package.
#[test]
#[should_panic(expected = "no assignment survives")]
fn empty_expansion_panics() {
    let _ = expand("1x -", |_| false, |_| two_rule_table());
}

/// Canonical `-` and legacy `P`/`(P)` each lower to the same pass call whether
/// the pass belongs to the opponents or to our side.
#[test]
fn canonical_and_legacy_passes_match_in_either_seat() {
    // Their pass after our bid.
    for source in ["1♥ -", "1♥ P", "1♥ (P)"] {
        let mut book = Trie::new();
        compile_entries(
            &mut book,
            "t",
            rows_of(Pattern::node(source), two_rule_table()),
        );
        assert!(book.get(&calls("1♥ -")).is_some(), "{source}");
    }

    // Our pass after their bid, before their partner's response.
    for source in ["(1♥) - (1♠)", "(1♥) P (1♠)", "(1♥) (P) (1♠)"] {
        let mut book = Trie::new();
        compile_entries(
            &mut book,
            "t",
            rows_of(Pattern::node(source), two_rule_table()),
        );
        assert!(book.get(&calls("1♥ - 1♠")).is_some(), "{source}");
    }
}
