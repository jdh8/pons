//! Declarative book layer: entry rows compiled into the existing [`Trie`]
//!
//! The three books already have their runtime structure; what was scattered is
//! the *assembly* — mega-functions spelling wiring imperatively.  This module
//! turns a book section into data: a [`Package`] (name, knob gate, entries), a
//! list of [`Entry`] rows each tying an auction [`Pattern`] to one rule or one
//! rebase, and one fold — [`compile_into`] — that lowers them onto a [`Trie`]
//! through the existing verbs ([`insert_all_seats`], [`fallback_all_seats`]).
//!
//! Every pattern construct **lowers** to an existing expander or guard; a
//! construct with no lowering does not enter the grammar, so resolution
//! precedence, renderability, and the floor discipline are inherited, not
//! reimplemented:
//!
//! | Pattern construct | Lowers to |
//! | --- | --- |
//! | leading `P*` | seat fan, keys under `0..=3` leading passes |
//! | [`Pattern::node`] | exact trie key |
//! | [`Pattern::first`] | [`FirstIs`] guarded fallback |
//! | [`Pattern::up_to`] | [`OvercallAtMost`] guarded fallback |
//! | [`Pattern::table`], [`Pattern::after`] | [`SuffixIs`] guarded fallback |
//! | [`Pattern::guarded`] | a hand-written [`Guard`], carried verbatim |
//! | [`rebase`] entry | [`Fallback::Rebase`] |
//! | [`classified`] entry | [`Fallback::Classify`] of a computed table |
//!
//! The grammar grows only with its consumers.
//!
//! Auction strings write our calls bare and their calls in parentheses —
//! `"P* 1♥ (X)"` — and the parser checks that parenthesisation against seat
//! alternation, so a row authored on the wrong side of the table fails at
//! build time rather than bidding for the opponents.
//!
//! The exact-node/guarded-table distinction is load-bearing, not cosmetic: an
//! exact node that rejects a hand (all-−∞) falls through to the floor
//! ([`Trie::classify_floored`]), while a guarded table is consulted again on
//! the fall-through pass and therefore must stay total — keep a finite
//! catch-all in every guarded table.

use super::common::{fallback_all_seats, insert_all_seats};
use super::constraint::Constraint;
use super::fallback::{Fallback, FirstIs, Guard, OvercallAtMost, Rewrite, SuffixIs};
use super::rules::{Alert, Rules};
use super::trie::{Classifier, Trie};
use contract_bridge::Bid;
use contract_bridge::auction::Call;
use std::sync::Arc;

/// Where a row lives: a trie key, a seat fan, and an optional guard
///
/// Constructed from auction strings by [`node`][Self::node],
/// [`table`][Self::table], [`after`][Self::after], [`first`][Self::first],
/// and [`up_to`][Self::up_to]; see the [module docs][self] for the grammar
/// and each constructor for its lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Pattern {
    /// The auction string(s) as authored, for build-time diagnostics
    source: String,
    /// The trie key the entry attaches to
    key: Vec<Call>,
    /// Leading-pass fan: keys under `0..=fan` leading passes share one entry
    fan: usize,
    /// [`None`] is an exact classifier node; a guard is a fallback entry
    guard: Option<GuardSpec>,
}

/// The guard constructs the grammar admits, each naming its lowering
#[derive(Clone)]
enum GuardSpec {
    /// Any continuation starting with this call → [`FirstIs`]
    First(Call),
    /// Exactly one uncovered bid at most this one → [`OvercallAtMost`]
    UpTo(Bid),
    /// Exactly this continuation → [`SuffixIs`]
    Suffix(Vec<Call>),
    /// A hand-written guard, carried verbatim → itself
    Opaque {
        /// One continuation the guard admits, for probing and diagnostics
        sample: Vec<Call>,
        /// The guard as the imperative site wrote it
        guard: Arc<dyn Guard>,
    },
}

/// Compared by shape alone: an [`Arc<dyn Guard>`][Guard] is a closure and has
/// no equality.  [`Pattern`]'s derived comparison also covers `source`, which
/// for an [`Opaque`][GuardSpec::Opaque] spec carries the guard's own
/// `describe()` — so two rows authored from one call regroup into one table,
/// and two differently-labelled guards stay apart.  The residual: two guards
/// sharing a label *and* a sample would merge under the first one's `Arc`.
impl PartialEq for GuardSpec {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::First(a), Self::First(b)) => a == b,
            (Self::UpTo(a), Self::UpTo(b)) => a == b,
            (Self::Suffix(a), Self::Suffix(b)) => a == b,
            (Self::Opaque { sample: a, .. }, Self::Opaque { sample: b, .. }) => a == b,
            _ => false,
        }
    }
}

impl Eq for GuardSpec {}

impl std::fmt::Debug for GuardSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::First(call) => write!(f, "First({call})"),
            Self::UpTo(bid) => write!(f, "UpTo({bid})"),
            Self::Suffix(calls) => write!(f, "Suffix({calls:?})"),
            Self::Opaque { sample, guard } => {
                write!(f, "Opaque({:?}, sample {sample:?})", guard.describe())
            }
        }
    }
}

impl GuardSpec {
    fn lower(&self) -> Arc<dyn Guard> {
        match self {
            Self::First(call) => Arc::new(FirstIs(*call)),
            Self::UpTo(bid) => Arc::new(OvercallAtMost(*bid)),
            Self::Suffix(calls) => Arc::new(SuffixIs(calls.clone())),
            Self::Opaque { guard, .. } => Arc::clone(guard),
        }
    }
}

/// One parsed auction token: the call and whether it is theirs
struct Token {
    call: Call,
    theirs: bool,
}

/// Parse an auction string into tokens plus the leading `P*` fan
///
/// # Panics
///
/// Panics on an unparsable call so a typo fails at book build, with the
/// offending string in the message.
fn parse(source: &str) -> (Vec<Token>, usize) {
    let mut fan = 0;
    let mut tokens = Vec::new();
    for (index, word) in source.split_whitespace().enumerate() {
        if word == "P*" {
            assert_eq!(index, 0, "pattern {source:?}: P* is only valid leading");
            fan = 3;
            continue;
        }
        let (theirs, text) = match word.strip_prefix('(').and_then(|w| w.strip_suffix(')')) {
            Some(inner) => (true, inner),
            None => (false, word),
        };
        let call = text
            .parse()
            .unwrap_or_else(|_| panic!("pattern {source:?}: unparsable call {word:?}"));
        tokens.push(Token { call, theirs });
    }
    (tokens, fan)
}

/// Check parenthesisation against seat alternation
///
/// `our_index` is an index (one past the last token is allowed) where **we**
/// are to act; sides strictly alternate, so every token's side follows.  The
/// leading-pass fan shifts every index equally and cancels out.
fn check_sides(source: &str, tokens: &[Token], our_index: usize) {
    for (index, token) in tokens.iter().enumerate() {
        let theirs = (our_index - index) % 2 == 1;
        assert_eq!(
            token.theirs,
            theirs,
            "pattern {source:?}: call {} is {} — write theirs in parentheses, ours bare",
            token.call,
            if theirs { "theirs" } else { "ours" },
        );
    }
}

impl Pattern {
    /// A guard at the `key` admitting any continuation starting with
    /// `their_call`
    ///
    /// Lowers to [`FirstIs`]; the canonical carrier of a systems-on
    /// [`rebase`] — `Pattern::first("P* 1♥", "X")` with
    /// [`ReplaceNext`][super::fallback::ReplaceNext]`(Pass)` strips their
    /// double off the whole subtree.
    pub(crate) fn first(key: &str, their_call: &str) -> Self {
        let (tokens, fan) = parse(key);
        check_sides(key, &tokens, tokens.len() + 1);
        let call = their_call
            .parse()
            .unwrap_or_else(|_| panic!("pattern {key:?}: unparsable call {their_call:?}"));
        Self {
            source: format!("{key} ({their_call}) …"),
            key: tokens.into_iter().map(|token| token.call).collect(),
            fan,
            guard: Some(GuardSpec::First(call)),
        }
    }

    /// A guard at the `key` admitting exactly one uncovered bid ≤ `their_bid`
    ///
    /// Lowers to [`OvercallAtMost`]: the natural guard for a competitive
    /// package handling the call directly over an overcall.
    pub(crate) fn up_to(key: &str, their_bid: &str) -> Self {
        let (tokens, fan) = parse(key);
        check_sides(key, &tokens, tokens.len() + 1);
        let bid = match their_bid.parse() {
            Ok(Call::Bid(bid)) => bid,
            _ => panic!("pattern {key:?}: {their_bid:?} is not a bid"),
        };
        Self {
            source: format!("{key} (≤{their_bid})"),
            key: tokens.into_iter().map(|token| token.call).collect(),
            fan,
            guard: Some(GuardSpec::UpTo(bid)),
        }
    }

    /// An exact classifier node at the `key`
    ///
    /// Lowers to a plain trie insert.  Unlike a guarded table, an exact node
    /// may reject a hand (all-−∞) and fall through to the floor — the idiom
    /// for a defense whose no-sound-action default belongs to the floor.
    pub(crate) fn node(key: &str) -> Self {
        let (tokens, fan) = parse(key);
        check_sides(key, &tokens, tokens.len());
        Self {
            source: key.to_string(),
            key: tokens.into_iter().map(|token| token.call).collect(),
            fan,
            guard: None,
        }
    }

    /// A total re-authoring table at the `key` itself
    ///
    /// Lowers to [`SuffixIs`]`([])`: the whole next call is re-authored at a
    /// deeper key (winning structurally over any guard at the parent), while
    /// every longer suffix stays with whatever the parent authored — the
    /// idiom for "meanings change here, continuations ride the rebase".
    pub(crate) fn table(key: &str) -> Self {
        let (tokens, fan) = parse(key);
        check_sides(key, &tokens, tokens.len());
        Self {
            source: key.to_string(),
            key: tokens.into_iter().map(|token| token.call).collect(),
            fan,
            guard: Some(GuardSpec::Suffix(Vec::new())),
        }
    }

    /// A table behind the exact continuation `suffix` after the `key`
    ///
    /// Lowers to [`SuffixIs`]`(suffix)`: our answer after one specific
    /// partner call and their pass — `Pattern::after("P* 1♥ (X)", "2NT (P)")`
    /// is opener's rebid over the Jordan raise.
    pub(crate) fn after(key: &str, suffix: &str) -> Self {
        let (tokens, fan) = parse(key);
        let (rest, rest_fan) = parse(suffix);
        assert_eq!(rest_fan, 0, "pattern {suffix:?}: P* is only valid leading");
        let key_len = tokens.len();
        let source = format!("{key} {suffix}");
        let all: Vec<Token> = tokens.into_iter().chain(rest).collect();
        check_sides(&source, &all, all.len());
        let mut calls = all.into_iter().map(|token| token.call);
        Self {
            key: calls.by_ref().take(key_len).collect(),
            source,
            fan,
            guard: Some(GuardSpec::Suffix(calls.collect())),
        }
    }

    /// A hand-written guard at the `key`, with a `sample` continuation it
    /// admits
    ///
    /// The escape hatch for guards the constructs above cannot spell — a
    /// prefix wildcard (`X (bid) …`, our stopper-bid after their double of
    /// our Stayman), a relational overcall/cue pair.  The guard is carried
    /// **verbatim**, so the renderers and resolution see exactly what the
    /// imperative site wrote; wrap it in
    /// [`described_guard`][super::fallback::described_guard] as that site did,
    /// or the book renders an opaque entry.
    ///
    /// `sample` is what buys back the checks the named constructs get for
    /// free: it is seat-checked like any suffix, it is the probe auction the
    /// totality invariant needs, and
    /// `assert_package_invariants` asserts the guard really does admit it —
    /// so a sample that drifts from its guard fails the test rather than
    /// silently probing the wrong auction.
    pub(crate) fn guarded(key: &str, sample: &str, guard: impl Guard + 'static) -> Self {
        let (tokens, fan) = parse(key);
        let (rest, rest_fan) = parse(sample);
        assert_eq!(rest_fan, 0, "pattern {sample:?}: P* is only valid leading");
        let key_len = tokens.len();
        let seats = format!("{key} {sample}");
        let all: Vec<Token> = tokens.into_iter().chain(rest).collect();
        // Unlike an exact suffix, a wildcard sample need not end just before
        // our turn — so anchor on the key instead: every key starts with our
        // call, hence an even token index is ours.  Rounding the length up to
        // an even `our_index` says exactly that without underflowing.
        check_sides(&seats, &all, all.len().next_multiple_of(2));
        let mut calls = all.into_iter().map(|token| token.call);
        Self {
            key: calls.by_ref().take(key_len).collect(),
            source: match guard.describe() {
                Some(label) => format!("{key} {label}"),
                None => seats,
            },
            fan,
            guard: Some(GuardSpec::Opaque {
                sample: calls.collect(),
                guard: Arc::new(guard),
            }),
        }
    }

    /// The auction this pattern's table actually answers: the key completed
    /// by the guard's admitted continuation
    ///
    /// ponytail: `UpTo` samples only its bound — the level-tightest admitted
    /// auction; probe per admitted bid if a legality-anchored table ever
    /// hides behind one.
    #[cfg(test)]
    fn probe_auction(&self) -> Vec<Call> {
        let mut auction = self.key.clone();
        match &self.guard {
            None => {}
            Some(GuardSpec::First(call)) => auction.push(*call),
            Some(GuardSpec::UpTo(bid)) => auction.push(Call::Bid(*bid)),
            Some(GuardSpec::Suffix(calls)) => auction.extend_from_slice(calls),
            Some(GuardSpec::Opaque { sample, .. }) => auction.extend_from_slice(sample),
        }
        auction
    }
}

/// One rule at one pattern — the row of the declarative layer
///
/// The payload is a singleton [`Rules`] so the existing builder columns
/// (weight, alert) carry over unchanged, and [`compile_into`] regroups
/// consecutive same-pattern rows into one table.
pub(crate) struct Row {
    pattern: Pattern,
    rules: Rules,
}

/// Author one rule at one pattern — the fine-grained form
///
/// The row-native twin of [`Rules::rule`]; chain [`Row::alert`] for the
/// artificial-call column, then `.into()` for the [`Entry`].
pub(crate) fn row(
    pattern: Pattern,
    call: impl Into<Call>,
    weight: f32,
    when: impl Constraint + 'static,
) -> Row {
    Row {
        pattern,
        rules: Rules::new().rule(call, weight, when),
    }
}

impl Row {
    /// Alert this row's call as the artificial convention `alert`
    ///
    /// Mirrors [`Rules::alert`]; the invariant test
    /// [`assert_artificial_fallback_rows_alerted`] checks the column is
    /// filled wherever the projection says it must be.
    pub(crate) fn alert(mut self, alert: Alert) -> Self {
        self.rules = self.rules.alert(alert);
        self
    }
}

/// A whole authored [`Rules`] table as rows at one pattern
///
/// The bridge for tables shared across sections (a cue-raise ladder used by
/// several packages): the table keeps its single authoring function and each
/// site lifts it into rows.  [`compile_into`]'s regrouping reassembles the
/// exact same rule list, so the port is byte-identical.
pub(crate) fn rows_of(pattern: Pattern, rules: Rules) -> Vec<Entry> {
    rules
        .rules()
        .iter()
        .map(|rule| {
            Entry::Row(Row {
                pattern: pattern.clone(),
                rules: Rules::of(rule.clone()),
            })
        })
        .collect()
}

/// A guarded rebase entry: rewrite the auction and resolve again
///
/// The `pattern` must carry a guard ([`Pattern::first`] in practice); the
/// rewrite lowers to [`Fallback::Rebase`] behind it.
pub(crate) fn rebase(pattern: Pattern, rewrite: impl Rewrite + 'static) -> Entry {
    assert!(
        pattern.guard.is_some(),
        "rebase pattern {:?} needs a guard",
        pattern.source,
    );
    Entry::Guarded(pattern, Fallback::rebase(rewrite))
}

/// A guarded entry whose table is computed at classify time
///
/// The escape hatch for a table the row grammar cannot spell as [`Rules`]:
/// one that reads the [`Context`][super::context::Context] (opener's raise at
/// the level *their* intervention forces) or transplants another table's
/// logits (the stolen-relay sites).  The classifier is carried verbatim into
/// [`Fallback::Classify`], so resolution and the renderers see exactly what
/// the imperative site wrote.
///
/// Being opaque, it pays a disclosure price: [`assert_package_invariants`]
/// still probes such an entry for totality (that needs only
/// [`Classifier::classify`]) but **cannot check its alerts**, which the
/// authored-rule column gets for free.
pub(crate) fn classified(pattern: Pattern, classifier: impl Classifier + 'static) -> Entry {
    assert!(
        pattern.guard.is_some(),
        "classified pattern {:?} needs a guard",
        pattern.source,
    );
    Entry::Guarded(pattern, Fallback::classify(classifier))
}

/// One line of a package: a rule row, or a guarded fallback carried verbatim
pub(crate) enum Entry {
    /// One rule at one pattern
    Row(Row),
    /// A guarded auction rewrite ([`rebase`]) or computed table
    /// ([`classified`])
    Guarded(Pattern, Fallback),
}

impl From<Row> for Entry {
    fn from(row: Row) -> Self {
        Self::Row(row)
    }
}

/// A named, knob-gated block of entries — the unit of the package ledger
pub(crate) struct Package {
    /// Ledger name (`P4:jordan-truscott`, …), used in diagnostics
    pub name: &'static str,
    /// The `set_*` knob, read at book build exactly as the `if` blocks it
    /// replaces; `|| true` for shipped base sections
    pub gate: fn() -> bool,
    /// The rows; a `fn` rather than a value because rows read range knobs
    pub entries: fn() -> Vec<Entry>,
}

/// One lowered unit at one pattern: a regrouped rule table, or a fallback
/// (rebase or computed table) carried verbatim
enum Lowered {
    Table(Rules),
    Fallback(Fallback),
}

/// Group a package's entries: consecutive same-pattern rows into one table
///
/// The regrouping seam [`compile_into`] and the invariant checks share, so
/// what is checked is exactly what is inserted.
///
/// # Panics
///
/// Panics when a pattern re-appears non-consecutively within a package (the
/// second group would author a second fallback entry, or silently replace an
/// exact node, instead of extending the first table — an authoring bug either
/// way), and on a guardless [`Entry::Guarded`].
fn group(package: &Package) -> Vec<(Pattern, Lowered)> {
    let mut groups: Vec<(Pattern, Lowered)> = Vec::new();
    for entry in (package.entries)() {
        match entry {
            Entry::Guarded(pattern, fallback) => {
                assert!(
                    pattern.guard.is_some(),
                    "{}: guarded entry at {:?} lacks a guard",
                    package.name,
                    pattern.source,
                );
                groups.push((pattern, Lowered::Fallback(fallback)));
            }
            Entry::Row(row) => match groups.last_mut() {
                Some((pattern, Lowered::Table(rules))) if *pattern == row.pattern => {
                    *rules = std::mem::replace(rules, Rules::new()).chain(row.rules);
                }
                _ => {
                    assert!(
                        !groups.iter().any(|(pattern, _)| *pattern == row.pattern),
                        "{}: pattern {:?} re-declared non-consecutively",
                        package.name,
                        row.pattern.source,
                    );
                    groups.push((row.pattern, Lowered::Table(row.rules)));
                }
            },
        }
    }
    groups
}

/// Lower packages onto a trie — the only writer the row layer uses
///
/// Consecutive [`Entry::Row`]s sharing a pattern regroup into one [`Rules`]
/// table (one classifier [`Arc`] fanned across seats, exactly like the
/// hand-written verbs), then lower per the pattern's guard.  Declaration
/// order within a package is preserved node-by-node, which is the resolution
/// precedence for fallback entries.
///
/// # Panics
///
/// See [`group`].
pub(crate) fn compile_into(book: &mut Trie, packages: &[Package]) {
    for package in packages {
        if !(package.gate)() {
            continue;
        }
        for (pattern, lowered) in group(package) {
            match (lowered, &pattern.guard) {
                (Lowered::Fallback(fallback), Some(spec)) => {
                    fallback_all_seats(book, &pattern.key, pattern.fan, spec.lower(), fallback);
                }
                (Lowered::Fallback(_), None) => {
                    unreachable!("group() rejects guardless entries")
                }
                (Lowered::Table(rules), None) => {
                    insert_all_seats(book, &pattern.key, pattern.fan, rules);
                }
                (Lowered::Table(rules), Some(spec)) => fallback_all_seats(
                    book,
                    &pattern.key,
                    pattern.fan,
                    spec.lower(),
                    Fallback::classify(rules),
                ),
            }
        }
    }
}

/// Assert the row invariants over whole packages
///
/// Walks the same [`group`]ing [`compile_into`] inserts, probing each table
/// under the auction it actually answers — the pattern's key completed by its
/// guard's admitted continuation — so legality-anchored rules
/// (`min_level_is`) see the real bidding space.  Two invariants:
///
/// * **Totality**, guarded entries only (the 7NT rule): a guarded table
///   survives the floor's fall-through pass, so a hand it rejects would be
///   left with no call at all.  Probed rather than proved — a spread of hands
///   from yarborough to rock — which catches the classic omission (no
///   catch-all `Pass`) without deciding totality symbolically.  Exact nodes
///   may reject-to-floor by design and are exempt.  A [`classified`] entry is
///   probed too: totality needs only [`Classifier::classify`], and a computed
///   table is exactly where mass goes missing (the stolen-relay transplants
///   push a call to −∞ and rely on `Double` inheriting it).
/// * **Alerts**, authored tables only: an artificial call (witness: a
///   projection flooring 4+ in a suit the call did not name) must carry an
///   alert — the fallback-row extension of `artificial_calls_are_alerted`,
///   which walks exact trie nodes only.  A [`classified`] entry has no
///   readable rules and goes unchecked here; its alerts ride on the tables it
///   computes from.
///
/// Gates are ignored: opt-in packages must satisfy the invariants too.
#[cfg(test)]
pub(crate) fn assert_package_invariants(packages: &[Package]) {
    use super::context::Context;
    use super::inference::artificial;
    use super::trie::Classifier;
    use contract_bridge::auction::RelativeVulnerability;

    let probes: Vec<contract_bridge::Hand> = [
        "98432.K53.QJ4.92", // yarborough-ish
        "AKQ2.KQ5.AQJ4.92", // 21 balanced
        "AQ2.K53.QJ42.T92", // 12 flat
        "2.98653.QJ742.92", // weak two-suiter
        "72.AQJ983.K4.J32", // heart single-suiter
        "AKQJ32.2.AK42.93", // strong shapely
    ]
    .iter()
    .map(|hand| hand.parse().expect("valid probe hand"))
    .collect();

    for package in packages {
        for (pattern, lowered) in group(package) {
            let auction = pattern.probe_auction();
            let context = Context::new(RelativeVulnerability::NONE, &auction);
            if let Some(spec @ GuardSpec::Opaque { sample, .. }) = &pattern.guard {
                assert!(
                    spec.lower().admits(&context, sample),
                    "{}: hand-written guard at {:?} rejects its own sample — \
                     the probe auction and the guard have drifted apart",
                    package.name,
                    pattern.source,
                );
            }
            // Totality reads whatever the guard classifies with — an authored
            // table or a computed one; only the alert probe needs `Rules`.
            let table: Option<&dyn Classifier> = match &lowered {
                Lowered::Table(rules) => Some(rules),
                Lowered::Fallback(Fallback::Classify(classifier)) => Some(&**classifier),
                Lowered::Fallback(_) => None,
            };
            if let (Some(table), true) = (table, pattern.guard.is_some()) {
                for &hand in &probes {
                    assert!(
                        table.classify(hand, &context).has_mass(),
                        "{}: guarded table at {:?} rejects {hand} — \
                         guarded tables cannot fall through to the floor",
                        package.name,
                        pattern.source,
                    );
                }
            }
            let Lowered::Table(rules) = lowered else {
                continue;
            };
            // The doubled strain for an X/XX row: the last bid before it.
            let doubled = auction.iter().rev().find_map(|call| match call {
                Call::Bid(bid) => Some(bid.strain),
                _ => None,
            });
            for rule in rules.rules() {
                if artificial(&rule.project(&context), rule.call(), doubled) {
                    assert!(
                        rule.alert().is_some(),
                        "{}: artificial call {} at {:?} has no alert",
                        package.name,
                        rule.call(),
                        pattern.source,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::constraint::hcp;
    use super::super::context::Context;
    use super::super::fallback::{ReplaceNext, described_guard, guard};
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
            .rule(Bid::new(2, Strain::Hearts), 1.0, hcp(10..))
            .rule(Call::Pass, 0.0, hcp(0..))
    }

    fn compiled(packages: &[Package]) -> Trie {
        let mut book = Trie::new();
        compile_into(&mut book, packages);
        book
    }

    /// Guarded rows lower onto every seat-fanned key, regrouped into one
    /// table per pattern with the guard riding along.
    #[test]
    fn rows_regroup_and_fan() {
        let book = compiled(&[Package {
            name: "test",
            gate: || true,
            entries: || rows_of(Pattern::up_to("P* 1♥", "2♠"), two_rule_table()),
        }]);

        let entries = book.fallbacks();
        let keys: Vec<&[Call]> = entries.iter().map(|(key, ..)| &**key).collect();
        assert_eq!(
            keys,
            [
                &calls("1♥")[..],
                &calls("P 1♥"),
                &calls("P P 1♥"),
                &calls("P P P 1♥"),
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
    }

    /// A rebase entry lowers to `Fallback::Rebase` and re-resolves onto the
    /// rewritten auction.
    #[test]
    fn rebase_lowers_and_reresolves() {
        let mut book = compiled(&[Package {
            name: "test",
            gate: || true,
            entries: || {
                vec![rebase(
                    Pattern::first("P* 1♥", "X"),
                    ReplaceNext(Call::Pass),
                )]
            },
        }]);
        book.insert(&calls("1♥ P 2♥"), two_rule_table());

        let auction = calls("1♥ X 2♥");
        let context = Context::new(RelativeVulnerability::NONE, &auction);
        let (_, provenance) = book
            .resolve(&context, &auction)
            .expect("the rebase reaches the systems-on node");
        assert_eq!(provenance.rebases, 1, "resolved through one rewrite");
        assert_eq!(provenance.depth, 3, "at the [1♥ P 2♥] node");
    }

    /// `after` splits the key from the guard suffix at the string boundary,
    /// and fine-grained `row(...)` rows regroup with the alert riding along.
    #[test]
    fn after_splits_key_and_suffix() {
        let book = compiled(&[Package {
            name: "test",
            gate: || true,
            entries: || {
                vec![
                    row(
                        Pattern::after("P* 1♥ (X)", "2NT (P)"),
                        Bid::new(4, Strain::Hearts),
                        1.0,
                        hcp(13..),
                    )
                    .alert(Alert("test:conv"))
                    .into(),
                    row(
                        Pattern::after("P* 1♥ (X)", "2NT (P)"),
                        Call::Pass,
                        0.0,
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
        let auction = calls("2NT P");
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
            gate: || true,
            entries: || {
                vec![rebase(
                    Pattern::guarded(
                        "P* 1NT (P) 2♣",
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
        assert_eq!(&*entries[0].0, calls("1NT P 2♣"), "keyed below our Stayman");
        assert_eq!(
            entries[0].1.describe().as_deref(),
            Some("X (bid) …"),
            "the guard's own label survives, so render-book is unchanged",
        );

        let auction = calls("1NT P 2♣ X 2♦");
        let context = Context::new(RelativeVulnerability::NONE, &auction);
        assert!(
            entries[0].1.admits(&context, &calls("X 2♦")),
            "wildcard tail"
        );
        assert!(
            !entries[0].1.admits(&context, &calls("X P P")),
            "the re-ask suffix is left to its own table — what FirstIs would swallow",
        );
    }

    /// A gated-off package compiles to nothing.
    #[test]
    fn gate_off_compiles_nothing() {
        let book = compiled(&[Package {
            name: "test",
            gate: || false,
            entries: || rows_of(Pattern::up_to("P* 1♥", "2♠"), two_rule_table()),
        }]);
        assert!(book.fallbacks().is_empty());
        assert_eq!(book.iter().count(), 0);
    }

    /// Re-declaring a pattern non-consecutively is an authoring bug: the
    /// second run would author a second fallback entry instead of extending
    /// the first table.
    #[test]
    #[should_panic(expected = "re-declared non-consecutively")]
    fn non_consecutive_pattern_panics() {
        compiled(&[Package {
            name: "test",
            gate: || true,
            entries: || {
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
}
