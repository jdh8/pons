//! Declarative book layer: entry rows compiled into the existing [`Trie`]
//!
//! The three books already have their runtime structure; what was scattered is
//! the *assembly* — mega-functions spelling wiring imperatively.  This module
//! turns a book section into data: a [`Package`] (name, knob gate, entries), a
//! list of [`Entry`] rows each tying an auction [`Pattern`] to one rule or one
//! rebase, and one fold — [`compile_into`] — that lowers them onto a [`Trie`]
//! through exact inserts and the existing [`fallback_all_seats`] guard helper.
//!
//! Every pattern construct **lowers** to an existing expander or guard; a
//! construct with no lowering does not enter the grammar, so resolution
//! precedence, renderability, and the floor discipline are inherited, not
//! reimplemented:
//!
//! | Pattern construct | Lowers to |
//! | --- | --- |
//! | leading `P*` | seat fan, keys under `0..=3` leading passes |
//! | [`Pattern::with_fan`] | explicit seat fan, keys under `0..=fan` leading passes |
//! | [`Pattern::node`] | exact trie key |
//! | [`Pattern::first`] | [`FirstIs`] guarded fallback |
//! | [`Pattern::up_to`] | [`OvercallAtMost`] guarded fallback |
//! | [`Pattern::table`], [`Pattern::after`] | [`SuffixIs`] guarded fallback |
//! | [`Pattern::guarded`] | a hand-written [`Guard`], carried verbatim |
//! | [`rebase`] entry | [`Fallback::Rebase`] |
//! | [`classified`] entry | [`Fallback::Classify`] of a computed table |
//! | [`expand`] template | one [`Pattern::node`] row per assignment |
//!
//! The grammar grows only with its consumers.
//!
//! Auction strings write our calls bare and their calls in parentheses —
//! `"P* 1♥ (X)"` — and the parser checks that parenthesisation against seat
//! alternation, so a row authored on the wrong side of the table fails at
//! build time rather than bidding for the opponents.  `-` is a **pass by
//! whoever is to act** — side-agnostic, for the routine passes that pad a
//! constructive auction; explicit `P`/`(P)` stays worthwhile in contested
//! tails where seat-tracking earns its parens.  Pick one spelling per
//! package: [`Pattern`] equality includes the source string, and two
//! spellings of one key regroup as two patterns.
//!
//! ## Variable rows
//!
//! [`expand`] turns one template auction string into many exact rows: it
//! cross-products every variable's domain, keeps the assignments the caller's
//! domain closure accepts and whose auctions are strictly ascending, and
//! emits [`Pattern::node`] rows from a table closure handed each assignment's
//! [`Bindings`].  Variables are **binders, not matchers**: the result is a
//! finite family of exact keys (infinite tails stay with the guard verbs),
//! and `P`/`X`/`XX` stay literal — a table for their overcall is never the
//! table for their double.
//!
//! A variable word is a **level slot** then a **strain slot**, implicitly
//! typed by spelling, and **case is the variable/literal boundary**: a single
//! lowercase letter is a variable; uppercase words and glyphs are literals.
//! The lexer rejects lowercase literals, pre-empting the case-insensitive
//! upstream `FromStr` that would silently read `"2s"` as 2♠.
//!
//! | Slot | Literals | Variables | Keywords |
//! | --- | --- | --- | --- |
//! | level | `1`–`7` | `i j k l n` bind a level; `.` = fresh anonymous | — |
//! | strain | `♣ ♦ ♥ ♠`, `C D H S`, `N`/`NT` | any other lowercase letter binds a [`Suit`]; `.` = fresh anonymous | `M` a major, `m` a minor (binding); `OM`/`om` the other one (derived) |
//!
//! * Notrump enters **literally only** (`iN` is notrump at level `i`):
//!   letters bind suits, never five-strain domains — notrump meanings are
//!   systemically special and get their own rows.
//! * `M`, `OM`, `m`, `om` are reserved strain words like `NT`, not letters;
//!   a row using `OM` must also use `M` (`om` likewise `m`) or the build
//!   panics.
//! * One letter binds once per row — `"1x (P) 2x (P)"` is a raise — while each
//!   `.` is fresh.
//! * Quantifiers apply to `P` alone, and only leading: no other call can
//!   repeat in an ascending auction, and an internal pass run would flip
//!   which side later calls fall on.  `P*` is the seat fan above; `P+` is
//!   recognized but deferred until a consumer exists.
//! * A legacy site with a deliberately smaller leading-pass ceiling uses
//!   [`Pattern::with_fan`] after construction; this keeps exceptional fan
//!   counts out of the auction-string grammar.
//!
//! Style: prefer `x y z a b c` for suit letters — `o` reads as the `om` tail,
//! `l` as `1`, `n` as the `N` literal.
//!
//! The exact-node/guarded-table distinction is load-bearing, not cosmetic: an
//! exact node that rejects a hand (all-−∞) falls through to the floor
//! ([`Trie::classify_floored`]), while a guarded table is consulted again on
//! the fall-through pass and therefore must stay total — keep a finite
//! catch-all in every guarded table.

use super::common::{fallback_all_seats, other_major, other_minor};
use super::constraint::Constraint;
use super::fallback::{Fallback, FirstIs, Guard, OvercallAtMost, Rewrite, SuffixIs};
use super::rules::{Alert, Rules};
use super::trie::{Classifier, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Level, Strain, Suit};
use core::fmt;
use std::collections::{BTreeMap, BTreeSet};
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
///
/// `theirs` is [`None`] for `-`, the side-agnostic pass — it takes whichever
/// seat its position implies and [`check_sides`] does not second-guess it.
struct Token {
    call: Call,
    theirs: Option<bool>,
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
        if word == "P+" {
            assert_eq!(index, 0, "pattern {source:?}: P+ is only valid leading");
            panic!("pattern {source:?}: P+ is recognized but deferred — it has no consumer yet");
        }
        if word == "-" {
            tokens.push(Token {
                call: Call::Pass,
                theirs: None,
            });
            continue;
        }
        let (theirs, text) = match word.strip_prefix('(').and_then(|w| w.strip_suffix(')')) {
            Some(inner) => (true, inner),
            None => (false, word),
        };
        let call = parse_call(source, word, text);
        tokens.push(Token {
            call,
            theirs: Some(theirs),
        });
    }
    (tokens, fan)
}

/// Parse one concrete call word, enforcing the case boundary
///
/// Case is the variable/literal boundary (see the [module docs][self]), but
/// the upstream `FromStr` is case-*insensitive* — left alone it would read
/// `"2s"` as 2♠ where the grammar means a suit variable.  Rejecting lowercase
/// wherever a literal is expected keeps the boundary crisp at build time.
fn parse_call(source: &str, word: &str, text: &str) -> Call {
    assert!(
        !text.contains(|c: char| c.is_ascii_lowercase()),
        "pattern {source:?}: {word:?} is lowercase — a lowercase letter is a variable; \
         write literals uppercase ({})",
        text.to_ascii_uppercase(),
    );
    text.parse()
        .unwrap_or_else(|_| panic!("pattern {source:?}: unparsable call {word:?}"))
}

/// Check parenthesisation against seat alternation
///
/// `our_index` is an index (one past the last token is allowed) where **we**
/// are to act; sides strictly alternate, so every token's side follows.  The
/// leading-pass fan shifts every index equally and cancels out.
fn check_sides(source: &str, tokens: &[Token], our_index: usize) {
    for (index, token) in tokens.iter().enumerate() {
        let Some(annotated) = token.theirs else {
            continue;
        };
        let theirs = (our_index - index) % 2 == 1;
        assert_eq!(
            annotated,
            theirs,
            "pattern {source:?}: call {} is {} — write theirs in parentheses, ours bare",
            token.call,
            if theirs { "theirs" } else { "ours" },
        );
    }
}

impl Pattern {
    /// Set the maximum number of leading passes this pattern fans across
    ///
    /// `P*` remains the readable shorthand for the ordinary all-seat fan of
    /// `0..=3`.  This builder carries the exceptional legacy ceiling that is
    /// neither zero nor three without adding another auction-string token.
    #[must_use]
    pub(crate) fn with_fan(mut self, fan: usize) -> Self {
        assert!(
            fan <= 3,
            "seat fan {fan} exceeds the three possible leading passes"
        );
        self.fan = fan;
        self
    }

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
        let call = parse_call(key, their_call, their_call);
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
        let bid = match parse_call(key, their_bid, their_bid) {
            Call::Bid(bid) => bid,
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

/// The level slot of one template word
#[derive(Clone, Copy)]
enum LevelSlot {
    Lit(Level),
    Var(char),
    Anon(usize),
}

/// The strain slot of one template word
///
/// `M`/`m`/`OM`/`om` are reserved words, not letters — they live here rather
/// than in `Var` so the case rule needs no exception.
#[derive(Clone, Copy)]
enum StrainSlot {
    Lit(Strain),
    Var(char),
    Major,
    Minor,
    OtherMajor,
    OtherMinor,
    Anon(usize),
}

/// One word of a variable-row template
enum TemplateWord {
    /// Passed through verbatim; a bid word joins the ascension check
    Literal { text: String, bid: Option<Bid> },
    /// Substituted per assignment, parenthesised as authored
    Variable {
        parens: bool,
        level: LevelSlot,
        strain: StrainSlot,
    },
}

/// Type one template word per the [module grammar][self]
fn lex_word(
    source: &str,
    word: &str,
    anon_levels: &mut usize,
    anon_suits: &mut usize,
) -> TemplateWord {
    let (parens, text) = match word.strip_prefix('(').and_then(|w| w.strip_suffix(')')) {
        Some(inner) => (true, inner),
        None => (false, word),
    };
    if matches!(text, "P*" | "P+") {
        // Quantifier position and P+ deferral are `parse`'s panics.
        return TemplateWord::Literal {
            text: word.into(),
            bid: None,
        };
    }
    let mut chars = text.chars();
    let level = match chars.next() {
        Some(digit @ '1'..='7') => LevelSlot::Lit(Level::new(digit as u8 - b'0')),
        Some(letter @ ('i' | 'j' | 'k' | 'l' | 'n')) => LevelSlot::Var(letter),
        Some('.') => {
            *anon_levels += 1;
            LevelSlot::Anon(*anon_levels - 1)
        }
        // No level slot: a concrete call word (`P`, `X`, `-`, …)
        _ => {
            return TemplateWord::Literal {
                text: word.into(),
                bid: match parse_call(source, word, text) {
                    Call::Bid(bid) => Some(bid),
                    _ => None,
                },
            };
        }
    };
    let strain = match chars.as_str() {
        "" => panic!("template {source:?}: {word:?} has an empty strain slot"),
        "." => {
            *anon_suits += 1;
            StrainSlot::Anon(*anon_suits - 1)
        }
        "M" => StrainSlot::Major,
        "m" => StrainSlot::Minor,
        "OM" => StrainSlot::OtherMajor,
        "om" => StrainSlot::OtherMinor,
        rest => {
            let mut singleton = rest.chars();
            match (singleton.next(), singleton.next()) {
                (Some(letter @ ('i' | 'j' | 'k' | 'l' | 'n')), None) => panic!(
                    "template {source:?}: {word:?} puts level letter {letter:?} in the \
                     strain slot — notrump is the literal `N`",
                ),
                (Some(letter @ 'a'..='z'), None) => StrainSlot::Var(letter),
                _ => {
                    assert!(
                        !rest.contains(|c: char| c.is_ascii_lowercase()),
                        "template {source:?}: {word:?} is lowercase — literals are \
                         uppercase; write {}",
                        text.to_ascii_uppercase(),
                    );
                    StrainSlot::Lit(rest.parse().unwrap_or_else(|_| {
                        panic!("template {source:?}: unparsable strain in {word:?}")
                    }))
                }
            }
        }
    };
    match (level, strain) {
        // A fully literal bid word is just a literal.
        (LevelSlot::Lit(level), StrainSlot::Lit(strain)) => TemplateWord::Literal {
            text: word.into(),
            bid: Some(Bid { level, strain }),
        },
        (level, strain) => TemplateWord::Variable {
            parens,
            level,
            strain,
        },
    }
}

/// The enumerable variables of one template, in deterministic order
struct Variables {
    /// Named level letters, sorted, each over `1..=7`
    levels: Vec<char>,
    /// Fresh anonymous levels, each over `1..=7`
    anon_levels: usize,
    /// Named strain keys, sorted (`'M'`/`'m'` carry the keywords)
    suits: Vec<char>,
    /// Fresh anonymous suits, each over the four suits
    anon_suits: usize,
}

impl Variables {
    fn of(source: &str, words: &[TemplateWord]) -> Self {
        let mut levels = BTreeSet::new();
        let mut suits = BTreeSet::new();
        let (mut anon_levels, mut anon_suits) = (0, 0);
        let (mut derived_major, mut derived_minor) = (false, false);
        for word in words {
            let TemplateWord::Variable { level, strain, .. } = word else {
                continue;
            };
            match level {
                LevelSlot::Var(letter) => {
                    levels.insert(*letter);
                }
                LevelSlot::Anon(index) => anon_levels = anon_levels.max(index + 1),
                LevelSlot::Lit(_) => {}
            }
            match strain {
                StrainSlot::Var(letter) => {
                    suits.insert(*letter);
                }
                StrainSlot::Major => {
                    suits.insert('M');
                }
                StrainSlot::Minor => {
                    suits.insert('m');
                }
                StrainSlot::OtherMajor => derived_major = true,
                StrainSlot::OtherMinor => derived_minor = true,
                StrainSlot::Anon(index) => anon_suits = anon_suits.max(index + 1),
                StrainSlot::Lit(_) => {}
            }
        }
        assert!(
            !derived_major || suits.contains(&'M'),
            "template {source:?}: OM requires M in the row",
        );
        assert!(
            !derived_minor || suits.contains(&'m'),
            "template {source:?}: om requires m in the row",
        );
        Self {
            levels: levels.into_iter().collect(),
            anon_levels,
            suits: suits.into_iter().collect(),
            anon_suits,
        }
    }

    fn suit_domain(key: char) -> &'static [Suit] {
        match key {
            'M' => &[Suit::Hearts, Suit::Spades],
            'm' => &[Suit::Clubs, Suit::Diamonds],
            _ => &Suit::ASC,
        }
    }

    /// Domain sizes, one axis per variable
    fn dims(&self) -> Vec<usize> {
        core::iter::repeat_n(7, self.levels.len() + self.anon_levels)
            .chain(self.suits.iter().map(|&key| Self::suit_domain(key).len()))
            .chain(core::iter::repeat_n(4, self.anon_suits))
            .collect()
    }

    /// Materialize the assignment at one odometer state
    fn bind(&self, state: &[usize]) -> Assignment {
        let mut cursor = state.iter().copied();
        let level = |index: usize| Level::new(u8::try_from(index + 1).expect("level fits"));
        Assignment {
            levels: self
                .levels
                .iter()
                .map(|&letter| (letter, level(cursor.next().expect("state covers levels"))))
                .collect(),
            anon_levels: (0..self.anon_levels)
                .map(|_| level(cursor.next().expect("state covers anonymous levels")))
                .collect(),
            suits: self
                .suits
                .iter()
                .map(|&key| {
                    let domain = Self::suit_domain(key);
                    (key, domain[cursor.next().expect("state covers suits")])
                })
                .collect(),
            anon_suits: (0..self.anon_suits)
                .map(|_| Suit::ASC[cursor.next().expect("state covers anonymous suits")])
                .collect(),
        }
    }
}

/// One concrete valuation of a template's variables
struct Assignment {
    levels: BTreeMap<char, Level>,
    anon_levels: Vec<Level>,
    suits: BTreeMap<char, Suit>,
    anon_suits: Vec<Suit>,
}

impl Assignment {
    fn level(&self, slot: LevelSlot) -> Level {
        match slot {
            LevelSlot::Lit(level) => level,
            LevelSlot::Var(letter) => self.levels[&letter],
            LevelSlot::Anon(index) => self.anon_levels[index],
        }
    }

    fn strain(&self, slot: StrainSlot) -> Strain {
        match slot {
            StrainSlot::Lit(strain) => strain,
            StrainSlot::Var(letter) => self.suits[&letter].into(),
            StrainSlot::Major => self.suits[&'M'].into(),
            StrainSlot::Minor => self.suits[&'m'].into(),
            StrainSlot::OtherMajor => other_major(self.suits[&'M']).into(),
            StrainSlot::OtherMinor => other_minor(self.suits[&'m']).into(),
            StrainSlot::Anon(index) => self.anon_suits[index].into(),
        }
    }
}

/// One assignment of a template's variables, handed to the domain and table
/// closures of [`expand`]
pub(crate) struct Bindings {
    levels: BTreeMap<char, Level>,
    suits: BTreeMap<char, Suit>,
    /// Every variable-strain word's substituted bid, for [`bid`][Self::bid]
    bids: Vec<(char, Bid)>,
}

#[allow(dead_code)] // the grammar lands one commit before its first consumer
impl Bindings {
    /// The level bound to a level letter
    pub(crate) fn level(&self, letter: char) -> Level {
        *self
            .levels
            .get(&letter)
            .unwrap_or_else(|| panic!("no level variable {letter:?} in the row"))
    }

    /// The suit bound to a suit letter, or to the `'M'`/`'m'` keyword
    pub(crate) fn suit(&self, letter: char) -> Suit {
        *self
            .suits
            .get(&letter)
            .unwrap_or_else(|| panic!("no suit variable {letter:?} in the row"))
    }

    /// The bid at a strain letter's unique occurrence
    ///
    /// # Panics
    ///
    /// When the letter spells no bid in the row, or several — read an
    /// ambiguous letter's bids off their level letters instead.
    pub(crate) fn bid(&self, letter: char) -> Bid {
        let mut hits = self
            .bids
            .iter()
            .filter(|(key, _)| *key == letter)
            .map(|&(_, bid)| bid);
        let bid = hits
            .next()
            .unwrap_or_else(|| panic!("no bid spells strain letter {letter:?} in the row"));
        assert!(
            hits.next().is_none(),
            "strain letter {letter:?} spells several bids in the row — \
             read the one you mean off its level letter",
        );
        bid
    }
}

/// Substitute one assignment into the template: the concrete source, its
/// bindings, and whether its bids strictly ascend
fn substitute(words: &[TemplateWord], assignment: &Assignment) -> (String, Bindings, bool) {
    let mut texts = Vec::new();
    let mut bids = Vec::new();
    let mut letter_bids = Vec::new();
    for word in words {
        match word {
            TemplateWord::Literal { text, bid } => {
                texts.push(text.clone());
                bids.extend(*bid);
            }
            TemplateWord::Variable {
                parens,
                level,
                strain,
            } => {
                let bid = Bid {
                    level: assignment.level(*level),
                    strain: assignment.strain(*strain),
                };
                match strain {
                    StrainSlot::Var(letter) => letter_bids.push((*letter, bid)),
                    StrainSlot::Major => letter_bids.push(('M', bid)),
                    StrainSlot::Minor => letter_bids.push(('m', bid)),
                    _ => {}
                }
                texts.push(if *parens {
                    format!("({bid})")
                } else {
                    bid.to_string()
                });
                bids.push(bid);
            }
        }
    }
    let ascending = bids.windows(2).all(|pair| pair[0] < pair[1]);
    let bindings = Bindings {
        levels: assignment.levels.clone(),
        suits: assignment.suits.clone(),
        bids: letter_bids,
    };
    (texts.join(" "), bindings, ascending)
}

/// Expand a variable-row template into exact rows, one table per assignment
///
/// Cross-products every variable's domain (see the [module grammar][self]),
/// keeps the assignments `domain` accepts whose bids strictly ascend, and for
/// each survivor emits [`Pattern::node`] rows carrying `table`'s [`Rules`].
/// Exact nodes only: a wildcard tail is the guard verbs' native shape, not a
/// template's.
///
/// # Panics
///
/// On a malformed template (see [`lex_word`] and [`Variables::of`]), and when
/// no assignment survives the filters — an empty expansion is an authoring
/// bug, not a package.
pub(crate) fn expand(
    source: &str,
    domain: impl Fn(&Bindings) -> bool,
    table: impl Fn(&Bindings) -> Rules,
) -> Vec<Entry> {
    let (mut anon_levels, mut anon_suits) = (0, 0);
    let words: Vec<TemplateWord> = source
        .split_whitespace()
        .map(|word| lex_word(source, word, &mut anon_levels, &mut anon_suits))
        .collect();
    let variables = Variables::of(source, &words);
    let dims = variables.dims();
    let mut state = vec![0usize; dims.len()];
    let mut entries = Vec::new();
    'assignments: loop {
        let (concrete, bindings, ascending) = substitute(&words, &variables.bind(&state));
        if ascending && domain(&bindings) {
            entries.extend(rows_of(Pattern::node(&concrete), table(&bindings)));
        }
        let mut axis = 0;
        loop {
            if axis == dims.len() {
                break 'assignments;
            }
            state[axis] += 1;
            if state[axis] < dims[axis] {
                break;
            }
            state[axis] = 0;
            axis += 1;
        }
    }
    assert!(
        !entries.is_empty(),
        "template {source:?}: no assignment survives the domain and legality filters",
    );
    entries
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
#[track_caller]
pub(crate) fn row(
    pattern: Pattern,
    call: impl Into<Call>,
    weight: i16,
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
///   which walks exact trie nodes only.  The probe uses the same legacy hull
///   reading as that invariant: union-profile boxes can legitimately carry
///   another suit's information, and context-dead generic-table arms can replay
///   `support` against a different live partner suit.  A [`classified`] entry
///   has no readable rules and goes unchecked here; its alerts ride on the
///   tables it computes from.
///
/// The weight ties tolerated by [`assert_package_invariants`]
///
/// It emptied at the per-overcall direct-seat conversion: the two clusters the
/// census launched with — the Modern negative double's four
/// `min_level_is`/`they_bid` arms and the invitational `2NT`'s cheapest/jump
/// pair, both long-hand forms of one generic row partitioned across the
/// `(≤2♠)` guard's auction cases — dissolved when that guard became one exact
/// table per overcall, each column keeping a single arm.  Any new tie fails
/// the build; a deliberate one enters here as a substring of its report line.
///
/// The one entry is the Stayman `2♣` partition, surfaced (not created) by the
/// N1 port, which put [`notrump_responses`][super::american::notrump_responses]
/// under this probe for the first time.  Four rules claim `2♣` at weight 150 —
/// the constructive Stayman (`notrump.rs:382`), garbage Stayman's broke and
/// weak tiers (`:468`, `:481`) and crawling Stayman (`:508`) — and the report
/// cannot tell a redundancy from a partition, but this one is provably the
/// latter: every pair separates on the HCP band (`8..` against `..5`, `5..8`,
/// `..8`; `..5` against `5..8`) or on diamond length (`3+`/`4+` against `..=1`).
/// At most one ever matches, so `Rules::explain`'s strict `>` never chooses,
/// and all four carry `STAYMAN` in any case.  Splitting the weights instead
/// would move the reading, which is a bidding change and not this campaign's.
// ponytail: substring match, not a structured key — a new tie differing in
// call, weight, or arity still fails, which is the whole job.  The arity is in
// the entry deliberately: a *fifth* `2♣` rule at 150 is a new claim on the
// partition and must come back for its own disjointness argument.
#[cfg(test)]
const KNOWN_WEIGHT_TIES: [&str; 1] = ["one-nt-base: \"P* 1NT (P)\" — 2♣ at weight 150, 4 rules"];

/// Every pair of rules in one table justifying the same call at the same weight
///
/// Such a pair is redundant, not expressive.  The logit of a call is the
/// **maximum** over its rules and constraints are crisp, so two rules
/// `(call, w, C₁)` and `(call, w, C₂)` evaluate to exactly `w + crisp(C₁ ∨ C₂)`
/// — and the reader already disjoins every matching rule's projection, so it
/// reads the union too.  One rule whose constraint is the authored
/// [`EnvelopeUnion`][super::inference::EnvelopeUnion] says the same thing once.
///
/// The pair is also actively lossy: `Rules::explain` breaks the tie with a
/// strict `>`, so it names only the *first* rule while the reader unions both.
/// Which alert and which label describe the call is then decided by authoring
/// order alone.
///
/// Differing weights are the opposite and are not reported — that is an
/// authored precedence, where the lower rule speaks only for the hands the
/// higher one rejects.
///
/// Each report line carries both rules' [`Rule::origin`], because a tie between
/// two authoring sites is a collision while a tie inside one is a local
/// redundancy.  A pair that is *deliberately* two named justifications shows up
/// as `both labeled` — merging it into a union would fold the two
/// [`Rules::note`] labels into one, the only content such a pair actually has.
#[cfg(test)]
fn weight_tie_report(packages: &[Package]) -> Vec<String> {
    let mut report = Vec::new();
    for package in packages {
        for (pattern, lowered) in group(package.name, (package.entries)()) {
            let Lowered::Table(rules) = lowered else {
                continue;
            };
            let rules = rules.rules();
            for (index, rule) in rules.iter().enumerate() {
                // One line per rung, listing every rule on it: an N-way tie is
                // one fact, not the N(N-1)/2 pairs it decomposes into.
                let tied: Vec<&super::rules::Rule> = rules[index..]
                    .iter()
                    .filter(|other| other.call() == rule.call() && other.weight() == rule.weight())
                    .collect();
                if tied.len() < 2
                    || rules[..index].iter().any(|earlier| {
                        earlier.call() == rule.call() && earlier.weight() == rule.weight()
                    })
                {
                    continue;
                }
                let labeled = tied.iter().all(|rule| !rule.label().is_empty());
                let sites: Vec<String> = tied
                    .iter()
                    .map(|rule| {
                        format!(
                            "    {}:{}  {}",
                            rule.origin().file(),
                            rule.origin().line(),
                            rule.describe(),
                        )
                    })
                    .collect();
                report.push(format!(
                    "{}: {:?} — {} at weight {}, {} rules{}\n{}",
                    package.name,
                    pattern.source,
                    rule.call(),
                    rule.weight(),
                    tied.len(),
                    if labeled { " (all labeled)" } else { "" },
                    sites.join("\n"),
                ));
            }
        }
    }
    report
}

/// * **Weight ties**, authored tables only: see [`weight_tie_report`].
///
/// Gates are ignored: opt-in packages must satisfy the invariants too.
#[cfg(test)]
pub(crate) fn assert_package_invariants(packages: &[Package]) {
    let ties: Vec<String> = weight_tie_report(packages)
        .into_iter()
        .filter(|tie| !KNOWN_WEIGHT_TIES.iter().any(|known| tie.contains(known)))
        .collect();
    assert!(
        ties.is_empty(),
        "{} new same-call weight tie(s) — two rules claim one call at one rung, \
         so which alert and reading describe it is decided by authoring order \
         alone (see `weight_tie_report`):\n{}",
        ties.len(),
        ties.join("\n"),
    );

    use super::context::Context;
    use super::inference::{artificial, envelope_union_reading};
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
            // Match `artificial_calls_are_alerted`'s legacy-hull definition.
            // Restore the thread-local before asserting so a failed invariant
            // cannot leak its profile into another test on this worker.
            let prior_union_reading = envelope_union_reading();
            super::set_envelope_union_reading(false);
            let unalerted = rules
                .rules()
                .iter()
                .find(|rule| {
                    artificial(&rule.project(&context), rule.call(), doubled)
                        && rule.alert().is_none()
                })
                .map(super::rules::Rule::call);
            super::set_envelope_union_reading(prior_union_reading);
            assert!(
                unalerted.is_none(),
                "{}: artificial call {} at {:?} has no alert",
                package.name,
                unalerted.expect("checked above"),
                pattern.source,
            );
        }
    }
}

#[cfg(test)]
mod tests;
