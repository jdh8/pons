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

use super::common::fallback_all_seats;
use super::constraint::Constraint;
use super::fallback::{Fallback, FirstIs, Guard, OvercallAtMost, Rewrite, SuffixIs};
use super::rules::{Alert, Rules};
use super::trie::{Classifier, Trie};
use contract_bridge::Bid;
use contract_bridge::auction::Call;
use core::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PATTERN_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_RULE_TABLE_ID: AtomicU64 = AtomicU64::new(1);

/// Stable identity of one preserved row pattern within a mutable book
///
/// IDs are opaque process-local tokens. They survive cloning, merging, and
/// grafting, so every concrete copy of one logical row retains one identity;
/// independently-authored rows cannot collide.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct PatternId(u64);

/// Stable identity of a classifier table, independent of labels and samples
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct RuleTableId(u64);

/// The shipped, enumerable part of the row pattern grammar
#[derive(Clone)]
pub(crate) enum PatternGrammar {
    Exact,
    First(Call),
    UpTo(Bid),
    Suffix(Box<[Call]>),
    Opaque {
        sample: Box<[Call]>,
        guard: Arc<dyn Guard>,
    },
}

impl fmt::Debug for PatternGrammar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact => f.write_str("Exact"),
            Self::First(call) => f.debug_tuple("First").field(call).finish(),
            Self::UpTo(bid) => f.debug_tuple("UpTo").field(bid).finish(),
            Self::Suffix(suffix) => f.debug_tuple("Suffix").field(suffix).finish(),
            Self::Opaque { sample, guard } => f
                .debug_struct("Opaque")
                .field("sample", sample)
                .field("guard", guard)
                .finish(),
        }
    }
}

/// The lowered object a preserved pattern selects
#[derive(Clone, Debug)]
pub(crate) enum PreservedTarget {
    Rules(Arc<dyn Classifier>),
    Computed(Arc<dyn Classifier>),
    Rebase(Arc<dyn Rewrite>),
}

/// Kind of lowered target retained after pointer validation at finalization
///
/// The concrete classifier/guard/rewrite `Arc`s already live in the finalized
/// trie and decoder.  Keeping them a second time in the catalog would retain
/// overwritten authoring objects and make every `Stance` clone heavier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreservedTargetKind {
    Rules,
    Computed,
    Rebase,
}

impl PreservedTarget {
    pub(crate) const fn kind(&self) -> PreservedTargetKind {
        match self {
            Self::Rules(_) => PreservedTargetKind::Rules,
            Self::Computed(_) => PreservedTargetKind::Computed,
            Self::Rebase(_) => PreservedTargetKind::Rebase,
        }
    }
}

impl PreservedTarget {
    fn is_table(&self) -> bool {
        matches!(self, Self::Rules(_))
    }

    pub(crate) fn classifier(&self) -> Option<&Arc<dyn Classifier>> {
        match self {
            Self::Rules(classifier) | Self::Computed(classifier) => Some(classifier),
            Self::Rebase(_) => None,
        }
    }
}

/// One row pattern retained beside all of its concrete lowered routes
#[derive(Clone, Debug)]
pub(crate) struct PreservedPattern {
    pub(crate) id: PatternId,
    pub(crate) table_id: Option<RuleTableId>,
    #[allow(dead_code)] // retained through finalization for diagnostics/catalog parity
    pub(crate) package: Arc<str>,
    #[allow(dead_code)] // retained through finalization for diagnostics/catalog parity
    pub(crate) source: Arc<str>,
    pub(crate) grammar: PatternGrammar,
    pub(crate) keys: Box<[Box<[Call]>]>,
    pub(crate) guard: Option<Arc<dyn Guard>>,
    pub(crate) target: PreservedTarget,
}

/// One ledger slot. Sharing the row payload keeps `Trie::clone` and merges
/// pointer-cheap until the whole-system finalization consumes the ledger.
#[derive(Clone, Debug)]
pub(crate) struct LedgerPattern {
    pub(crate) declaration: u64,
    pattern: Arc<PreservedPattern>,
}

impl core::ops::Deref for LedgerPattern {
    type Target = PreservedPattern;

    fn deref(&self) -> &Self::Target {
        &self.pattern
    }
}

/// One surviving concrete placement of a preserved pattern
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg(test)]
pub(crate) struct ConcreteSite {
    pub(crate) key: Box<[Call]>,
    pub(crate) placement: Placement,
}

/// Whether a concrete pattern owns the exact node or one fallback slot
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg(test)]
pub(crate) enum Placement {
    Exact,
    Fallback { index: usize },
}

/// A logical row pattern and every route that survived final mutation
#[derive(Clone, Debug)]
#[cfg(test)]
pub(crate) struct BoundPattern {
    pub(crate) id: PatternId,
    pub(crate) table_id: Option<RuleTableId>,
    pub(crate) package: Arc<str>,
    pub(crate) source: Arc<str>,
    pub(crate) grammar: PatternGrammar,
    pub(crate) declaration: u64,
    pub(crate) target: PreservedTargetKind,
    pub(crate) sites: Box<[ConcreteSite]>,
}

/// Finalized, mutation-free row catalog attached to one bound book
#[derive(Clone, Debug, Default)]
#[cfg(test)]
pub(crate) struct AuthoringCatalog {
    patterns: Box<[BoundPattern]>,
}

#[cfg(test)]
impl AuthoringCatalog {
    pub(crate) fn new(patterns: Vec<BoundPattern>) -> Self {
        Self {
            patterns: patterns.into_boxed_slice(),
        }
    }

    pub(crate) fn patterns(&self) -> &[BoundPattern] {
        &self.patterns
    }
}

/// Whole-book ledger of row patterns, retained until `Pair::against()`
#[derive(Clone, Debug)]
pub(crate) struct AuthoringLedger {
    patterns: Vec<LedgerPattern>,
    next_declaration: u64,
}

impl AuthoringLedger {
    pub(crate) const fn new() -> Self {
        Self {
            patterns: Vec::new(),
            next_declaration: 0,
        }
    }

    pub(crate) fn push(&mut self, pattern: PreservedPattern) {
        let declaration = self.next_declaration;
        self.next_declaration += 1;
        self.patterns.push(LedgerPattern {
            declaration,
            pattern: Arc::new(pattern),
        });
    }

    pub(crate) fn extend(&mut self, other: Self) {
        for mut pattern in other.patterns {
            pattern.declaration = self.next_declaration;
            self.next_declaration += 1;
            self.patterns.push(pattern);
        }
    }

    pub(crate) fn extend_graft(&mut self, other: &Self, dst_prefix: &[Call], src_prefix: &[Call]) {
        for pattern in &other.patterns {
            let keys: Vec<Box<[Call]>> = pattern
                .keys
                .iter()
                .filter_map(|key| {
                    key.strip_prefix(src_prefix).map(|suffix| {
                        dst_prefix
                            .iter()
                            .copied()
                            .chain(suffix.iter().copied())
                            .collect()
                    })
                })
                .collect();
            if keys.is_empty() {
                continue;
            }
            let mut copied = (**pattern).clone();
            copied.keys = keys.into_boxed_slice();
            self.push(copied);
        }
    }

    #[cfg(test)]
    pub(crate) fn patterns(&self) -> &[LedgerPattern] {
        &self.patterns
    }

    pub(crate) fn into_patterns(self) -> impl Iterator<Item = LedgerPattern> {
        self.patterns.into_iter()
    }
}

impl Default for AuthoringLedger {
    fn default() -> Self {
        Self::new()
    }
}

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

    fn grammar(&self) -> PatternGrammar {
        match self {
            Self::First(call) => PatternGrammar::First(*call),
            Self::UpTo(bid) => PatternGrammar::UpTo(*bid),
            Self::Suffix(calls) => PatternGrammar::Suffix(calls.clone().into_boxed_slice()),
            Self::Opaque { sample, guard } => PatternGrammar::Opaque {
                sample: sample.clone().into_boxed_slice(),
                guard: Arc::clone(guard),
            },
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
fn group(name: &str, entries: Vec<Entry>) -> Vec<(Pattern, Lowered)> {
    let mut groups: Vec<(Pattern, Lowered)> = Vec::new();
    for entry in entries {
        match entry {
            Entry::Guarded(pattern, fallback) => {
                assert!(
                    pattern.guard.is_some(),
                    "{name}: guarded entry at {:?} lacks a guard",
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
                        "{name}: pattern {:?} re-declared non-consecutively",
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
        if (package.gate)() {
            compile_entries(book, package.name, (package.entries)());
        }
    }
}

/// Lower one block of entries under a diagnostic `name`
///
/// The seam for row *producers* — a subtree authored under a prefix the caller
/// computes, which a [`Package`] cannot express: its `entries` is a bare `fn`
/// pointer, so it captures nothing, not even which suit is trumps.  RKCB's
/// `rkcb_rows` is the case that motivated it; a producer's rows still regroup
/// and lower exactly as a package's do.
///
/// # Panics
///
/// See [`group`].
pub(crate) fn compile_entries(book: &mut Trie, name: &str, entries: Vec<Entry>) {
    let package: Arc<str> = Arc::from(name);
    for (pattern, lowered) in group(name, entries) {
        let keys: Vec<Box<[Call]>> = (0..=pattern.fan)
            .map(|passes| {
                core::iter::repeat_n(Call::Pass, passes)
                    .chain(pattern.key.iter().copied())
                    .collect()
            })
            .collect();
        let source: Arc<str> = Arc::from(pattern.source.as_str());
        let grammar = pattern
            .guard
            .as_ref()
            .map_or(PatternGrammar::Exact, GuardSpec::grammar);
        let lowered_guard = pattern.guard.as_ref().map(GuardSpec::lower);

        let target = match (lowered, &pattern.guard) {
            (Lowered::Fallback(fallback), Some(_)) => {
                let target = match &fallback {
                    Fallback::Classify(classifier) => {
                        PreservedTarget::Computed(Arc::clone(classifier))
                    }
                    Fallback::Rebase(rewrite) => PreservedTarget::Rebase(Arc::clone(rewrite)),
                };
                fallback_all_seats(
                    book,
                    &pattern.key,
                    pattern.fan,
                    Arc::clone(lowered_guard.as_ref().expect("guarded pattern")),
                    fallback,
                );
                target
            }
            (Lowered::Fallback(_), None) => {
                unreachable!("group() rejects guardless entries")
            }
            (Lowered::Table(rules), None) => {
                let classifier: Arc<dyn Classifier> = Arc::new(rules);
                for key in &keys {
                    book.insert_arc(key, Arc::clone(&classifier));
                }
                PreservedTarget::Rules(classifier)
            }
            (Lowered::Table(rules), Some(_)) => {
                let classifier: Arc<dyn Classifier> = Arc::new(rules);
                fallback_all_seats(
                    book,
                    &pattern.key,
                    pattern.fan,
                    Arc::clone(lowered_guard.as_ref().expect("guarded pattern")),
                    Fallback::Classify(Arc::clone(&classifier)),
                );
                PreservedTarget::Rules(classifier)
            }
        };
        let table_id = target
            .is_table()
            .then(|| RuleTableId(NEXT_RULE_TABLE_ID.fetch_add(1, Ordering::Relaxed)));
        book.preserve_authoring(PreservedPattern {
            id: PatternId(NEXT_PATTERN_ID.fetch_add(1, Ordering::Relaxed)),
            table_id,
            package: Arc::clone(&package),
            source,
            grammar,
            keys: keys.into_boxed_slice(),
            guard: lowered_guard,
            target,
        });
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
        for (pattern, lowered) in group(package.name, (package.entries)()) {
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
        assert!(book.authoring().patterns().is_empty());
    }

    /// IDs name the logical row: clones and grafted concrete routes retain
    /// them, while the finalized catalog reports the destination key.
    #[test]
    fn clone_and_graft_preserve_pattern_identity() {
        let source = compiled(&[Package {
            name: "test",
            gate: || true,
            entries: || rows_of(Pattern::node("1NT (P) 2♣ (P)"), two_rule_table()),
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
        assert_eq!(&*catalog.patterns()[0].sites[0].key, calls("1♠ 1NT P 2♣ P"));
    }

    /// A later imperative overwrite is authoritative; displaced row metadata
    /// does not leak into finalization (the Dutch opening compatibility case).
    #[test]
    fn exact_overwrite_drops_displaced_metadata() {
        let mut book = compiled(&[Package {
            name: "test",
            gate: || true,
            entries: || rows_of(Pattern::node("1♥ (P)"), two_rule_table()),
        }]);
        assert_eq!(book.finalize_authoring().patterns().len(), 1);
        book.insert(&calls("1♥ P"), Rules::new().rule(Call::Pass, 0.0, hcp(0..)));
        assert!(book.finalize_authoring().patterns().is_empty());
    }

    /// Merge keeps the receiver's exact classifier at a collision, while
    /// retaining metadata from every non-colliding route in the other trie.
    #[test]
    fn merge_filters_exact_collision_and_keeps_disjoint_metadata() {
        let mut book = compiled(&[Package {
            name: "receiver",
            gate: || true,
            entries: || rows_of(Pattern::node("1♥ (P)"), two_rule_table()),
        }]);
        let receiver_id = book.authoring().patterns()[0].id;

        let other = compiled(&[Package {
            name: "other",
            gate: || true,
            entries: || {
                let mut entries = rows_of(Pattern::node("1♥ (P)"), two_rule_table());
                entries.extend(rows_of(Pattern::node("1♠ (P)"), two_rule_table()));
                entries
            },
        }]);
        let displaced_id = other.authoring().patterns()[0].id;
        let disjoint_id = other.authoring().patterns()[1].id;

        assert_eq!(
            book.merge(other),
            vec![calls("1♥ P").into_boxed_slice()],
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
        assert_eq!(&*disjoint.sites[0].key, calls("1♠ P"));
        assert_eq!(disjoint.sites[0].placement, Placement::Exact);
        assert_eq!(&*disjoint.package, "other");
    }

    /// Fallback lists concatenate receiver-first on merge, and finalized
    /// sites retain the same indices used by runtime provenance.
    #[test]
    fn merge_preserves_fallback_order_in_catalog_and_runtime() {
        let mut book = compiled(&[Package {
            name: "receiver",
            gate: || true,
            entries: || rows_of(Pattern::first("1♥", "X"), two_rule_table()),
        }]);
        let receiver_id = book.authoring().patterns()[0].id;
        let other = compiled(&[Package {
            name: "other",
            gate: || true,
            entries: || rows_of(Pattern::first("1♥", "X"), two_rule_table()),
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
            gate: || true,
            entries: || rows_of(Pattern::first("1♥", "X"), two_rule_table()),
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
            gate: || true,
            entries: || vec![rebase(Pattern::first("1NT", "X"), ReplaceNext(Call::Pass))],
        }]);
        source.insert(&calls("1NT P 2♣"), two_rule_table());
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
            gate: || true,
            entries: || rows_of(Pattern::first("", "1♣"), two_rule_table()),
        }]);
        book.fallback_at(
            &[],
            Always,
            Fallback::classify(Rules::new().rule(Call::Pass, 0.0, hcp(0..))),
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

        let floor_auction = calls("P");
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
            gate: || true,
            entries: || rows_of(Pattern::node("1♥ (P)"), two_rule_table()),
        }]);
        assert_eq!(book.authoring().patterns().len(), 1);

        let catalog = book.take_finalized_authoring();
        assert!(book.authoring().patterns().is_empty());
        assert_eq!(catalog.patterns().len(), 1);
        assert_eq!(&*catalog.patterns()[0].package, "P3:inventory-regression");
        assert_eq!(&*catalog.patterns()[0].source, "1♥ (P)");
        assert_eq!(catalog.patterns()[0].sites[0].placement, Placement::Exact);
        assert!(
            book.get(&calls("1♥ P")).is_some(),
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
