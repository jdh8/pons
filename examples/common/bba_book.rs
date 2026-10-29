//! The `bba` system — BBA's walked book as an admissibility mask over our floor
//!
//! `docs/bba-system.md` is the design.  `probe-bba-book` walks EPBot's
//! hand-free interpreter and writes one [`Node`] per auction with every legal
//! call's reading (the schema below, which that example shares); this module
//! reads a dump back, folds each caller's deltas into cumulative [`Band`]s,
//! and builds a [`System`] whose every walked node is a [`Rules`] table of
//! those bands with the *choice* delegated to our own floor
//! ([`Rules::choose_by`]).  Off the walk the floors are the ones `american()`
//! ships: the v6 net on the contested books, the instinct ladder on the
//! constructive one.
//!
//! Nothing here reads a knob: the compiled book is a pure function of the
//! dump, and the floor takes the same [`Agreements`] value `american()` would
//! (the regime input and the ladder's profile), so `bba` − `american` on paired
//! deals is the agreements term under one judgement.
//!
//! The dump is named by `BBA_BOOK=<run dir>` ([`BbaBook::shared`]); the mask
//! knobs of the design's §6 are `BBA_BOOK_SLACK=<hcp>` (trap 4, widen every
//! HCP band), `BBA_BOOK_STOPPERS=1` (trap 3, read the stopper flags too) and
//! `BBA_BOOK_NO_CEILING_FLOOR=1` (trap 11, drop the bilans' glosses of calls
//! above the ceiling), all read by [`Mask::from_env`].

use super::hand_hcp;
use super::oracle::{Interpretation, SYSTEM_2_OVER_1};
use contract_bridge::auction::{Auction, Call, RelativeVulnerability, display_calls};
use contract_bridge::{Hand, Suit};
use pons::bidding::agreements::Agreements;
use pons::bidding::card::Card;
use pons::bidding::constraint::{Cons, Constraint, hcp, len, stopper_in};
use pons::bidding::context::Context;
use pons::bidding::fallback::{Always, Fallback};
use pons::bidding::features::{CompactConfig, ConventionCard};
use pons::bidding::neural_floor::ConfiguredFloorV6;
use pons::bidding::trie::Classifier;
use pons::bidding::{
    Alert, Competitive, Constructive, Defensive, Phase, Rules, System, Trie, instinct,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

// ───────────────────────────── dump schema ─────────────────────────────

/// One expanded node: its key, its family, and every legal child's reading
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Node {
    /// `display_calls` of the auction — house format, no parentheses
    pub k: String,
    /// `o` no side has bid, `c` one side has (constructive), `x` both (contested)
    pub f: char,
    /// How often a BBA self-play corpus reached this prefix
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_zero_u64"))]
    pub n: u64,
    pub c: Vec<Child>,
}

/// One legal call out of a node
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Child {
    /// The call itself, `Display`-rendered (`2♦`, `X`, `XX`, `P`)
    pub c: String,
    /// Why the walk stopped here; absent means the subtree is its own node
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub x: Option<String>,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_zero_u64"))]
    pub n: u64,
    /// One entry per distinct reading, with the vulnerabilities it holds under
    pub r: Vec<Reading>,
}

/// What BBA says the call means, as a delta against the node's own state
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Eq)]
pub struct Reading {
    /// Bitmask over `AbsoluteVulnerability`: none 1, ns 2, ew 4, both 8
    pub v: u8,
    /// BBA's own name for the rule, or `calculated bid` for the floor
    pub l: String,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "String::is_empty")
    )]
    pub e: String,
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_zero_i32"))]
    pub a: i32,
    /// The caller's HCP band after the call, from `feature[402]`/`[403]`
    pub h: [i32; 2],
    /// Suit length bands the call **changed**, keyed `C`/`D`/`H`/`S`
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "BTreeMap::is_empty")
    )]
    pub n: BTreeMap<char, [i32; 2]>,
    /// Stopper flags the call changed
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "BTreeMap::is_empty")
    )]
    pub st: BTreeMap<char, i32>,
    /// `feature` slots the call changed, minus the HCP pair carried by `h`
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "BTreeMap::is_empty")
    )]
    pub f: BTreeMap<u16, i32>,
    /// Bitmask of the *other* positions whose public block the call moved
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_zero_u8"))]
    pub o: u8,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero_u64(n: &u64) -> bool {
    *n == 0
}
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero_u8(n: &u8) -> bool {
    *n == 0
}
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero_i32(n: &i32) -> bool {
    *n == 0
}

/// EPBot's suit letters, in its own C, D, H, S array order
pub const SUIT_KEYS: [char; 4] = ['C', 'D', 'H', 'S'];

/// The same four suits as [`Suit`]s
pub const SUITS: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

/// Parse a house-format auction, tolerating the renderer's parentheses
///
/// `common::auction_key` is deliberately not used: it strips leading passes, and
/// `- - 1♠` (a third-seat opening, where Drury lives) is a different node from
/// `1♠`.
pub fn parse_auction(text: &str) -> anyhow::Result<Auction> {
    let mut auction = Auction::new();
    for token in text.split_whitespace() {
        let token = token
            .strip_prefix('(')
            .and_then(|token| token.strip_suffix(')'))
            .unwrap_or(token);
        let call: Call = token
            .parse()
            .map_err(|_| anyhow::anyhow!("`{token}` is not a call"))?;
        auction
            .try_push(call)
            .map_err(|error| anyhow::anyhow!("`{text}`: {error}"))?;
    }
    Ok(auction)
}

/// Every node of every `*.jsonl` shard, keyed by auction, plus the byte total
#[cfg(feature = "serde")]
pub fn load_dump(dir: &str) -> anyhow::Result<(HashMap<String, Node>, u64)> {
    let mut nodes = HashMap::new();
    let mut bytes = 0_u64;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !name.ends_with(".jsonl") || name.starts_with("reach-") {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        bytes += text.len() as u64;
        for line in text.lines() {
            let node: Node = serde_json::from_str(line)
                .map_err(|error| anyhow::anyhow!("{}: {error}", path.display()))?;
            nodes.insert(node.k.clone(), node);
        }
    }
    anyhow::ensure!(!nodes.is_empty(), "no nodes found under {dir}");
    Ok((nodes, bytes))
}

// ───────────────────────────── the compiled book ─────────────────────────────

/// How a band is read as a mask: the design's two §6 knobs
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mask {
    /// Widen every HCP band by this much at both ends (trap 4; `BBA_BOOK_SLACK`)
    pub hcp_slack: i32,
    /// Require a textbook stopper (A, Kx, Qxx, Jxxx) in every suit the reading
    /// flags one in (trap 3; `BBA_BOOK_STOPPERS=1`).  Off by default: the flag's
    /// scale (`1`, `2`, `-1`, `-2`) is not ours, and the hand-free crosscheck
    /// never measured it
    pub stoppers: bool,
    /// Drop a `calculated bid` child above the walk's ceiling from the
    /// admissible set (trap 11; `BBA_BOOK_NO_CEILING_FLOOR=1`).  Above the
    /// ceiling BBA has no rule and the walk no subtree, and the reading is the
    /// bilans' post-hoc gloss of a call it reaches only by arithmetic: `6♠`
    /// over `1♠ -` reads *9+ HCP, 3+♠*, and `4NT..7NT` over `1♣ (3NT)` all
    /// copy the gambling `3NT`'s band.  Of the 151,699 such children in the
    /// 2026-10-29 walk, 72,504 floor no suit at five and start at or below
    /// 12 HCP.  Off by default: a loose admission only leaves the chooser as
    /// free as it is off-book, while dropping them would also take the
    /// contested four-level — the accountant's own ground — out of every
    /// walked node whose `4M` BBA reads as its floor's
    pub no_ceiling_floor: bool,
}

impl Mask {
    /// The mask the environment asks for, defaults otherwise
    #[must_use]
    pub fn from_env() -> Self {
        let hcp_slack = std::env::var("BBA_BOOK_SLACK")
            .ok()
            .and_then(|text| text.trim().parse().ok())
            .unwrap_or(0);
        let flag = |name: &str| {
            std::env::var(name).is_ok_and(|text| matches!(text.trim(), "1" | "true" | "on"))
        };
        Self {
            hcp_slack,
            stoppers: flag("BBA_BOOK_STOPPERS"),
            no_ceiling_floor: flag("BBA_BOOK_NO_CEILING_FLOOR"),
        }
    }

    /// The HCP band after widening, clamped to the engine's own `0..=37`
    fn hcp_band(self, band: [i32; 2]) -> (u8, u8) {
        let clamp = |value: i32| u8::try_from(value.clamp(0, 37)).expect("clamped to 0..=37");
        (
            clamp(band[0] - self.hcp_slack),
            clamp(band[1] + self.hcp_slack),
        )
    }
}

/// What BBA's reader says the caller holds after one call: the cumulative
/// bands, which the dump stores as deltas and [`BbaBook::compile`] folds
///
/// `h` is already cumulative in the dump; the length and stopper deltas are
/// overlaid on the caller's state at the parent.  No reading of the shipped
/// walk ever moved another seat's block (`o` is zero on all 1,375,446
/// children), so the caller's own calls are the whole of its state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Band {
    /// The HCP band, `0..=37` when unconstrained
    pub hcp: [i32; 2],
    /// Min/max length per suit in EPBot's C, D, H, S order; `0..=13` when free
    pub lengths: [[i32; 2]; 4],
    /// Stopper flags per suit, same order; `0` is nothing claimed
    pub stoppers: [i32; 4],
}

impl Band {
    /// A fresh seat: every band at its full width
    pub const UNCONSTRAINED: Self = Self {
        hcp: [0, 37],
        lengths: [[0, 13]; 4],
        stoppers: [0; 4],
    };

    /// This band after one more reading by the same caller
    #[must_use]
    pub fn after(mut self, reading: &Reading) -> Self {
        self.hcp = reading.h;
        for (index, key) in SUIT_KEYS.iter().enumerate() {
            if let Some(&band) = reading.n.get(key) {
                self.lengths[index] = band;
            }
            if let Some(&flag) = reading.st.get(key) {
                self.stoppers[index] = flag;
            }
        }
        self
    }

    /// Whether the hand lies inside the band, read as `mask` reads it
    #[must_use]
    pub fn admits(&self, hand: Hand, mask: Mask) -> bool {
        let (lo, hi) = mask.hcp_band(self.hcp);
        let hcp = hand_hcp(hand);
        if hcp < lo || hcp > hi {
            return false;
        }
        for (index, suit) in SUITS.iter().enumerate() {
            let length = i32::try_from(hand[*suit].len()).expect("a suit has at most 13 cards");
            let [lo, hi] = self.lengths[index];
            if length < lo || length > hi {
                return false;
            }
        }
        if mask.stoppers {
            let context = Context::new(RelativeVulnerability::NONE, &[]);
            for (index, suit) in SUITS.iter().enumerate() {
                if self.stoppers[index] > 0 && !stopper_in(*suit).eval(hand, &context).is_finite() {
                    return false;
                }
            }
        }
        true
    }

    /// The band as an authored constraint: the same primitives `american()`
    /// uses, so its projection is the house reading of the same facts
    #[must_use]
    pub fn constraint(&self, mask: Mask) -> Box<dyn Constraint> {
        let (lo, hi) = mask.hcp_band(self.hcp);
        let mut when: Box<dyn Constraint> = Box::new(hcp(lo..=hi));
        for (index, suit) in SUITS.iter().enumerate() {
            let [lo, hi] = self.lengths[index];
            if [lo, hi] != [0, 13] {
                let (lo, hi) = (lo.clamp(0, 13) as usize, hi.clamp(0, 13) as usize);
                when = Box::new(Cons(when) & len(*suit, lo..=hi));
            }
        }
        if mask.stoppers {
            for (index, suit) in SUITS.iter().enumerate() {
                if self.stoppers[index] > 0 {
                    when = Box::new(Cons(when) & stopper_in(*suit));
                }
            }
        }
        when
    }

    /// A short gloss, the renderer's spelling (`7-9 3+♠`)
    #[must_use]
    pub fn gloss(&self) -> String {
        let mut parts = Vec::new();
        if self.hcp != [0, 37] {
            parts.push(match self.hcp {
                [lo, 37] => format!("{lo}+"),
                [0, hi] => format!("≤{hi}"),
                [lo, hi] if lo == hi => format!("{lo}"),
                [lo, hi] => format!("{lo}-{hi}"),
            });
        }
        for (index, suit) in SUITS.iter().enumerate() {
            match self.lengths[index] {
                [0, 13] => {}
                [lo, 13] => parts.push(format!("{lo}+{suit}")),
                [0, hi] => parts.push(format!("≤{hi}{suit}")),
                [lo, hi] if lo == hi => parts.push(format!("{lo}{suit}")),
                [lo, hi] => parts.push(format!("{lo}-{hi}{suit}")),
            }
            if self.stoppers[index] > 0 {
                parts.push(format!("{suit}stop"));
            }
        }
        parts.join(" ")
    }
}

/// One legal call at a walked node, with everything the book keeps about it
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub call: Call,
    /// The caller's cumulative band after the call
    pub band: Band,
    /// BBA's own name for the rule, `calculated bid` for its floor
    pub label: &'static str,
    /// BBA alerts the call
    pub alert: bool,
    /// The call is above the walk's ceiling: read, never expanded
    pub ceiling: bool,
}

impl Entry {
    /// Whether BBA's book has a rule or a reading for this call at all
    ///
    /// `Not defined` is neither: the engine names no rule and reads no band
    /// (`5NT` as an opening, 44 children of the 2026-10-29 walk), so the call
    /// is outside the book rather than loosely inside it.  A `calculated bid`
    /// above the ceiling is the bilans' gloss and goes with
    /// [`Mask::no_ceiling_floor`].
    #[must_use]
    pub fn in_book(&self, mask: Mask) -> bool {
        self.label != "Not defined"
            && !(mask.no_ceiling_floor && self.ceiling && is_calculated(self.label))
    }

    /// Whether the book admits `hand` for this call under `mask`
    #[must_use]
    pub fn admits(&self, hand: Hand, mask: Mask) -> bool {
        self.in_book(mask) && self.band.admits(hand, mask)
    }
}

/// Which floor chooses among the admitted calls at a walked node — the
/// design's D1
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chooser {
    /// The v6 floor shell everywhere (`bba`): our judgement, one net, at
    /// every walked node
    Net,
    /// The floor partition's own owner (`bba-ladder`): the instinct ladder at
    /// constructive nodes, the net elsewhere.  The ladder is a natural
    /// precedence, so it may put no mass on BBA's admitted calls and fall
    /// through; that rate is a number this arm measures
    Partition,
}

impl Chooser {
    /// The `--our-floor` name that selects this chooser
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Net => "bba",
            Self::Partition => "bba-ladder",
        }
    }

    /// The chooser a `--our-floor` name selects, if it is one of ours
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "bba" => Some(Self::Net),
            "bba-ladder" => Some(Self::Partition),
            _ => None,
        }
    }
}

/// BBA's walked book, compiled: every walked auction with its admissible calls
#[derive(Debug)]
pub struct BbaBook {
    nodes: HashMap<Box<[Call]>, Vec<Entry>>,
    /// Where the dump came from, for labels
    pub source: String,
}

impl BbaBook {
    /// Read a walk run directory and compile it
    #[cfg(feature = "serde")]
    pub fn load(dir: &str) -> anyhow::Result<Self> {
        let (nodes, _) = load_dump(dir)?;
        let mut book = Self::compile(&nodes)?;
        book.source = dir.to_owned();
        Ok(book)
    }

    /// Fold the dump's deltas into cumulative bands, node by node from the root
    ///
    /// Depth-first through the expanded children, carrying all four seats'
    /// state; a child's band is its caller's state overlaid with the child's
    /// reading.  Every node of a complete dump is reached from the root (the
    /// frontier shards are the root's own descendants); one that is not is
    /// reported, not silently dropped.
    pub fn compile(nodes: &HashMap<String, Node>) -> anyhow::Result<Self> {
        anyhow::ensure!(nodes.contains_key(""), "the dump has no root node");
        let mut book: HashMap<Box<[Call]>, Vec<Entry>> = HashMap::with_capacity(nodes.len());
        let mut dangling = Vec::new();
        let mut stack: Vec<(Vec<Call>, [Band; 4])> = vec![(Vec::new(), [Band::UNCONSTRAINED; 4])];
        while let Some((auction, state)) = stack.pop() {
            let key = display_calls(&auction).to_string();
            let Some(node) = nodes.get(&key) else {
                dangling.push(key);
                continue;
            };
            let caller = auction.len() % 4;
            let mut entries = Vec::with_capacity(node.c.len());
            for child in &node.c {
                let call: Call = child
                    .c
                    .parse()
                    .map_err(|_| anyhow::anyhow!("`{key}`: `{}` is not a call", child.c))?;
                // The walk reads every node at one vulnerability (readings
                // measured invariant across all four), so a child has one.
                let Some(reading) = child.r.first() else {
                    anyhow::bail!("`{key}`: child `{}` has no reading", child.c);
                };
                let band = state[caller].after(reading);
                entries.push(Entry {
                    call,
                    band,
                    label: intern(&reading.l),
                    alert: reading.a != 0,
                    ceiling: child.x.as_deref() == Some("ceil"),
                });
                if child.x.is_none() {
                    let mut next = auction.clone();
                    next.push(call);
                    let mut next_state = state;
                    next_state[caller] = band;
                    stack.push((next, next_state));
                }
            }
            book.insert(auction.into_boxed_slice(), entries);
        }
        anyhow::ensure!(
            dangling.is_empty(),
            "{} expanded child(ren) have no node — a shard is missing; first: `{}`",
            dangling.len(),
            dangling[0]
        );
        if book.len() != nodes.len() {
            eprintln!(
                "bba-book: {} of {} nodes are not reachable from the root and were not compiled",
                nodes.len() - book.len(),
                nodes.len()
            );
        }
        Ok(Self {
            nodes: book,
            source: String::new(),
        })
    }

    /// The book named by `BBA_BOOK`, loaded once per process
    ///
    /// # Errors
    ///
    /// When `BBA_BOOK` is unset or names no readable walk.
    #[cfg(feature = "serde")]
    pub fn shared() -> anyhow::Result<Arc<Self>> {
        static BOOK: OnceLock<Result<Arc<BbaBook>, String>> = OnceLock::new();
        BOOK.get_or_init(|| {
            let dir = std::env::var("BBA_BOOK").map_err(|_| {
                "the bba floor needs BBA_BOOK=<probe-bba-book run directory> \
                 (docs/bba-system.md)"
                    .to_owned()
            })?;
            Self::load(&dir)
                .map(Arc::new)
                .map_err(|error| format!("BBA_BOOK={dir}: {error}"))
        })
        .clone()
        .map_err(|error| anyhow::anyhow!(error))
    }

    /// How many auctions the book has a node for
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the book has no node at all
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The walked node for an auction, in dump order (bids ascending, then
    /// X, XX, P)
    #[must_use]
    pub fn get(&self, auction: &[Call]) -> Option<&[Entry]> {
        self.nodes.get(auction).map(Vec::as_slice)
    }

    /// The calls BBA's book admits for `hand` at `auction`, or [`None`] when
    /// the auction is off the walk
    #[must_use]
    pub fn admissible(&self, auction: &[Call], hand: Hand, mask: Mask) -> Option<Vec<Call>> {
        self.get(auction).map(|entries| {
            entries
                .iter()
                .filter(|entry| entry.admits(hand, mask))
                .map(|entry| entry.call)
                .collect()
        })
    }

    /// Every walked auction, for a census
    pub fn auctions(&self) -> impl Iterator<Item = &[Call]> {
        self.nodes.keys().map(AsRef::as_ref)
    }

    /// Build the system: one masked table per walked node over `american()`'s
    /// floor wiring under `agreements`
    ///
    /// Each node is a [`Rules`] table with one rule per legal call — weight 0,
    /// the band as its constraint, BBA's label as its note and, where BBA
    /// alerts, as its alert — whose choice is delegated to `chooser`.  The
    /// tables are routed into the three books by [`Phase::of`] exactly as the
    /// authored book's keys are, and the root fallbacks are the ones
    /// `with_floors` attaches (`src/bidding/common.rs`): the v6 shell on both
    /// contested books, the instinct ladder on the constructive one.
    #[must_use]
    pub fn system(&self, agreements: &Agreements, chooser: Chooser, mask: Mask) -> System {
        let agreements = *agreements;
        let ladder = Arc::new(instinct(&agreements));
        let net: Arc<dyn Classifier> = Arc::new(ConfiguredFloorV6::new(
            CompactConfig::symmetric(&ConventionCard::capture(&agreements)),
            Arc::clone(&ladder),
        ));
        let ladder: Arc<dyn Classifier> = ladder;

        let mut constructive = Constructive::new();
        let mut competitive = Competitive::new();
        let mut defensive = Defensive::new();
        for (auction, entries) in &self.nodes {
            let phase = Phase::of(auction);
            let choice = match (chooser, phase) {
                (Chooser::Partition, Phase::Constructive) => Arc::clone(&ladder),
                _ => Arc::clone(&net),
            };
            let mut rules = Rules::new();
            for entry in entries.iter().filter(|entry| entry.in_book(mask)) {
                rules = rules
                    .rule(entry.call, 0, entry.band.constraint(mask))
                    .note(entry.label);
                if entry.alert {
                    rules = rules.alert(Alert(entry.label));
                }
            }
            let rules = Arc::new(rules.choose_by(choice));
            let book: &mut Trie = match phase {
                Phase::Constructive => &mut constructive,
                Phase::Competitive => &mut competitive,
                Phase::Defensive => &mut defensive,
            };
            book.insert_arc(auction, rules);
        }
        competitive.fallback_at(&[], Always, Fallback::Classify(Arc::clone(&net)));
        defensive.fallback_at(&[], Always, Fallback::Classify(net));
        constructive.fallback_at(&[], Always, Fallback::Classify(ladder));
        System::new(constructive, competitive, defensive, agreements)
    }
}

/// The card the `bba` pair plays: EPBot's own 2/1 defaults, no row moved
///
/// The truthful disclosure (design D5): the walk that compiled the book was
/// `--card none`, so BBA reading us as a stock BBA is reading us right.
#[must_use]
pub fn card() -> Card {
    Card {
        system: SYSTEM_2_OVER_1,
        rows: Vec::new(),
    }
}

/// Whether a reading's label is BBA's floor rather than a rule
#[must_use]
pub fn is_calculated(label: &str) -> bool {
    label == Interpretation::CALCULATED
}

/// One `&'static str` per distinct label, for [`Alert`] and [`Rules::note`]
///
/// A few hundred distinct labels over a million readings, leaked once each.
fn intern(label: &str) -> &'static str {
    static LABELS: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut guard = LABELS.lock().expect("the label table is never poisoned");
    let labels = guard.get_or_insert_with(HashSet::new);
    if let Some(&leaked) = labels.get(label) {
        return leaked;
    }
    let leaked: &'static str = Box::leak(label.to_owned().into_boxed_str());
    labels.insert(leaked);
    leaked
}
