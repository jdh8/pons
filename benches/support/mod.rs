//! Shared parser and invariants for the frozen bidding-performance corpus.

#![allow(dead_code)]

use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Level, Strain, Suit};
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub const CORPUS_TEXT: &str = include_str!("../fixtures/bidding-performance.tsv");
pub const POSITION_COUNT: usize = 512;
pub const PER_ORIGIN_BIN: usize = 64;

/// Rust-global allocation-count and requested-byte telemetry for one workload.
/// Native allocations inside EPBot are outside this counter.
pub struct CountingAllocator {
    enabled: AtomicBool,
    allocations: AtomicU64,
    bytes: AtomicU64,
}

impl CountingAllocator {
    pub const fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            allocations: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
        }
    }

    pub fn reset(&self) {
        self.allocations.store(0, Ordering::SeqCst);
        self.bytes.store(0, Ordering::SeqCst);
        self.enabled.store(true, Ordering::SeqCst);
    }

    pub fn snapshot(&self) -> AllocationSnapshot {
        self.enabled.store(false, Ordering::SeqCst);
        AllocationSnapshot {
            allocations: self.allocations.load(Ordering::SeqCst),
            bytes: self.bytes.load(Ordering::SeqCst),
        }
    }
}

// SAFETY: every request is forwarded unchanged to `System`; the atomics only
// observe allocation requests and do not participate in ownership.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if self.enabled.load(Ordering::Relaxed) {
            self.allocations.fetch_add(1, Ordering::Relaxed);
            self.bytes
                .fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        // SAFETY: forwarding the caller's valid layout to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if self.enabled.load(Ordering::Relaxed) {
            self.allocations.fetch_add(1, Ordering::Relaxed);
            self.bytes
                .fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        // SAFETY: forwarding the caller's valid layout to the system allocator.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: forwarding the allocation and its original layout.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if self.enabled.load(Ordering::Relaxed) {
            self.allocations.fetch_add(1, Ordering::Relaxed);
            self.bytes.fetch_add(new_size as u64, Ordering::Relaxed);
        }
        // SAFETY: forwarding the allocation, original layout, and requested size.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AllocationSnapshot {
    pub allocations: u64,
    pub bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    Pons,
    Bba,
}

impl Origin {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pons => "pons",
            Self::Bba => "bba",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DepthBin {
    Two,
    Four,
    Eight,
    Twelve,
}

impl DepthBin {
    pub const ALL: [Self; 4] = [Self::Two, Self::Four, Self::Eight, Self::Twelve];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Two => "2",
            Self::Four => "4",
            Self::Eight => "8",
            Self::Twelve => "12",
        }
    }

    pub const fn contains(self, depth: usize) -> bool {
        match self {
            Self::Two => depth == 2,
            Self::Four => depth == 4,
            Self::Eight => depth == 8,
            Self::Twelve => depth == 12,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Representative,
    AuthoredShallow,
    AuthoredDeep,
    NeuralFloor,
    ConstructiveFloor,
    ForcedInstinctFloor,
    RkcbSlamTail,
    SystemsOn,
    Fallback,
}

impl Category {
    pub const REQUIRED: [Self; 8] = [
        Self::AuthoredShallow,
        Self::AuthoredDeep,
        Self::NeuralFloor,
        Self::ConstructiveFloor,
        Self::ForcedInstinctFloor,
        Self::RkcbSlamTail,
        Self::SystemsOn,
        Self::Fallback,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Representative => "representative",
            Self::AuthoredShallow => "authored-shallow",
            Self::AuthoredDeep => "authored-deep",
            Self::NeuralFloor => "neural-floor",
            Self::ConstructiveFloor => "constructive-floor",
            Self::ForcedInstinctFloor => "forced-instinct-floor",
            Self::RkcbSlamTail => "rkcb-slam-tail",
            Self::SystemsOn => "systems-on",
            Self::Fallback => "fallback",
        }
    }
}

/// Whether this decision is the asker decoding partner's plain-4NT 1430
/// answer.  That is the RKCB shape which makes the floor re-read the auction
/// immediately before the artificial answer; merely containing a 4NT call is
/// not enough (it may have been quantitative, or made by an opponent).
pub fn is_rkcb_historical_decode(auction: &[Call]) -> bool {
    let n = auction.len();
    let Some(ask_index) = n.checked_sub(4) else {
        return false;
    };
    if auction[ask_index] != Call::Bid(Bid::new(4, Strain::Notrump))
        || !matches!(auction[ask_index + 1], Call::Pass | Call::Double)
        || !matches!(auction[ask_index + 3], Call::Pass | Call::Double)
    {
        return false;
    }
    let Call::Bid(answer) = auction[ask_index + 2] else {
        return false;
    };
    if answer.level != Level::new(5) || answer.strain.suit().is_none() {
        return false;
    }
    // A notrump call by the asker immediately before 4NT makes the ask
    // quantitative.  Requiring a suit named by both members of the asking
    // side pins a face-recognizable agreement for this frozen plain-RKCB case.
    if ask_index >= 2
        && matches!(auction[ask_index - 2], Call::Bid(bid) if bid.strain == Strain::Notrump)
    {
        return false;
    }
    Suit::ASC.into_iter().any(|suit| {
        let mut named = [false; 2];
        for (index, call) in auction[..ask_index].iter().enumerate() {
            if index % 2 != ask_index % 2 {
                continue;
            }
            if matches!(call, Call::Bid(bid) if bid.strain.suit() == Some(suit)) {
                named[usize::from(index % 4 == ask_index % 4)] = true;
            }
        }
        named == [true, true]
    })
}

#[derive(Clone, Debug)]
pub struct Position {
    pub id: u16,
    pub origin: Origin,
    pub depth_bin: DepthBin,
    pub category: Category,
    pub vul: RelativeVulnerability,
    pub hand: Hand,
    pub auction: Vec<Call>,
}

pub fn parse_corpus() -> Result<Vec<Position>, String> {
    let mut positions = Vec::with_capacity(POSITION_COUNT);
    for (line_index, raw) in CORPUS_TEXT.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 7 {
            return Err(format!(
                "line {line_number}: expected 7 tab-separated fields"
            ));
        }
        let id = fields[0]
            .parse::<u16>()
            .map_err(|error| format!("line {line_number}: invalid id: {error}"))?;
        let origin = match fields[1] {
            "pons" => Origin::Pons,
            "bba" => Origin::Bba,
            value => return Err(format!("line {line_number}: invalid origin `{value}`")),
        };
        let depth_bin = match fields[2] {
            "2" => DepthBin::Two,
            "4" => DepthBin::Four,
            "8" => DepthBin::Eight,
            "12" => DepthBin::Twelve,
            value => return Err(format!("line {line_number}: invalid depth bin `{value}`")),
        };
        let category = match fields[3] {
            "representative" => Category::Representative,
            "authored-shallow" => Category::AuthoredShallow,
            "authored-deep" => Category::AuthoredDeep,
            "neural-floor" => Category::NeuralFloor,
            "constructive-floor" => Category::ConstructiveFloor,
            "forced-instinct-floor" => Category::ForcedInstinctFloor,
            "rkcb-slam-tail" => Category::RkcbSlamTail,
            "systems-on" => Category::SystemsOn,
            "fallback" => Category::Fallback,
            value => return Err(format!("line {line_number}: invalid category `{value}`")),
        };
        let vul = parse_vulnerability(fields[4])
            .ok_or_else(|| format!("line {line_number}: invalid vulnerability `{}`", fields[4]))?;
        let hand = fields[5]
            .parse::<Hand>()
            .map_err(|error| format!("line {line_number}: invalid hand: {error}"))?;
        if hand.len() != 13 {
            return Err(format!(
                "line {line_number}: hand has {} cards, expected 13",
                hand.len()
            ));
        }
        let auction =
            parse_auction(fields[6]).map_err(|error| format!("line {line_number}: {error}"))?;
        if !depth_bin.contains(auction.len()) {
            return Err(format!(
                "line {line_number}: depth {} is outside bin {}",
                auction.len(),
                depth_bin.as_str()
            ));
        }
        let mut legal = Auction::new();
        legal
            .try_extend(auction.iter().copied())
            .map_err(|error| format!("line {line_number}: illegal auction: {error}"))?;
        if legal.has_ended() {
            return Err(format!(
                "line {line_number}: position is after an ended auction"
            ));
        }
        positions.push(Position {
            id,
            origin,
            depth_bin,
            category,
            vul,
            hand,
            auction,
        });
    }
    validate(&positions)?;
    Ok(positions)
}

fn validate(positions: &[Position]) -> Result<(), String> {
    if positions.len() != POSITION_COUNT {
        return Err(format!(
            "corpus has {} positions, expected {POSITION_COUNT}",
            positions.len()
        ));
    }
    let ids: BTreeSet<_> = positions.iter().map(|position| position.id).collect();
    if ids.len() != POSITION_COUNT || ids.first() != Some(&0) || ids.last() != Some(&511) {
        return Err("position ids must be unique and contiguous from 0 through 511".into());
    }
    if positions
        .iter()
        .enumerate()
        .any(|(expected, position)| usize::from(position.id) != expected)
    {
        return Err("position rows must be ordered by their contiguous id".into());
    }
    let mut cells = BTreeMap::<(Origin, DepthBin), usize>::new();
    let mut categories = BTreeSet::new();
    let mut unique_positions = BTreeSet::new();
    for position in positions {
        *cells
            .entry((position.origin, position.depth_bin))
            .or_default() += 1;
        categories.insert(position.category);
        if position.category != Category::Representative && position.origin != Origin::Pons {
            return Err(format!(
                "position {} has targeted metadata but is not Pons-origin",
                position.id
            ));
        }
        let key = format!(
            "{}|{}|{}|{}|{}",
            position.origin.as_str(),
            position.depth_bin.as_str(),
            format_vulnerability(position.vul),
            position.hand,
            format_auction(&position.auction),
        );
        if !unique_positions.insert(key) {
            return Err(format!(
                "position {} duplicates an earlier corpus row",
                position.id
            ));
        }
        match position.category {
            Category::AuthoredShallow if position.auction.len() > 4 => {
                return Err(format!(
                    "position {} has a non-shallow authored label",
                    position.id
                ));
            }
            Category::AuthoredDeep if position.auction.len() < 8 => {
                return Err(format!(
                    "position {} has a non-deep authored label",
                    position.id
                ));
            }
            Category::RkcbSlamTail if !is_rkcb_historical_decode(&position.auction) => {
                return Err(format!(
                    "position {} is not an asker decoding partner's RKCB answer",
                    position.id
                ));
            }
            _ => {}
        }
    }
    for origin in [Origin::Pons, Origin::Bba] {
        for bin in DepthBin::ALL {
            let count = cells.get(&(origin, bin)).copied().unwrap_or(0);
            if count != PER_ORIGIN_BIN {
                return Err(format!(
                    "{} depth {} has {count} positions, expected {PER_ORIGIN_BIN}",
                    origin.as_str(),
                    bin.as_str()
                ));
            }
        }
    }
    for category in Category::REQUIRED {
        if !categories.contains(&category) {
            return Err(format!(
                "corpus is missing required category {}",
                category.as_str()
            ));
        }
    }
    Ok(())
}

pub fn parse_auction(text: &str) -> Result<Vec<Call>, String> {
    text.split_ascii_whitespace().map(parse_call).collect()
}

pub fn format_auction(calls: &[Call]) -> String {
    calls
        .iter()
        .map(|&call| format_call(call))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn parse_vulnerability(text: &str) -> Option<RelativeVulnerability> {
    match text {
        "none" => Some(RelativeVulnerability::NONE),
        "we" => Some(RelativeVulnerability::WE),
        "they" => Some(RelativeVulnerability::THEY),
        "both" => Some(RelativeVulnerability::ALL),
        _ => None,
    }
}

pub fn format_vulnerability(vul: RelativeVulnerability) -> &'static str {
    match (
        vul.contains(RelativeVulnerability::WE),
        vul.contains(RelativeVulnerability::THEY),
    ) {
        (false, false) => "none",
        (true, false) => "we",
        (false, true) => "they",
        (true, true) => "both",
    }
}

fn parse_call(text: &str) -> Result<Call, String> {
    match text {
        "P" => return Ok(Call::Pass),
        "X" => return Ok(Call::Double),
        "XX" => return Ok(Call::Redouble),
        _ => {}
    }
    let bytes = text.as_bytes();
    if bytes.len() != 2 || !(b'1'..=b'7').contains(&bytes[0]) {
        return Err(format!("invalid call `{text}`"));
    }
    let strain = match bytes[1] {
        b'C' => Strain::Clubs,
        b'D' => Strain::Diamonds,
        b'H' => Strain::Hearts,
        b'S' => Strain::Spades,
        b'N' => Strain::Notrump,
        _ => return Err(format!("invalid call `{text}`")),
    };
    Ok(Call::Bid(Bid {
        level: Level::new(bytes[0] - b'0'),
        strain,
    }))
}

fn format_call(call: Call) -> String {
    match call {
        Call::Pass => "P".into(),
        Call::Double => "X".into(),
        Call::Redouble => "XX".into(),
        Call::Bid(bid) => format!(
            "{}{}",
            bid.level.get(),
            match bid.strain {
                Strain::Clubs => 'C',
                Strain::Diamonds => 'D',
                Strain::Hearts => 'H',
                Strain::Spades => 'S',
                Strain::Notrump => 'N',
            }
        ),
    }
}
