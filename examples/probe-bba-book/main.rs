//! The BBA book walk: reverse-engineering EPBot's **rule book**, call by call.
//!
//! BBA is the anchor `docs/bba-gap-campaign.md` measures against, and every
//! decode of it so far has been an island — `1NT` lanes, the keycard family,
//! the bilans arithmetic (`docs/ai-bidder/bba-*.md`).  This walks the whole
//! system instead, and it reads the **book** rather than the floor: EPBot
//! exports a prose label for every call it interprets
//! ([`BbaOracle::interpret`]), so a hand-free bot replaying an auction says
//! *what the rule is called* without ever running the bilans engine.  The label
//! `calculated bid` is the boundary — it means no rule matched and the floor
//! chose the call.
//!
//! # What bounds the walk
//!
//! Three cuts, and it takes all three.
//!
//! **Ceilings.**  The walk stops where the bridge stops being interesting rather
//! than at a uniform depth: **constructive auctions expand to `4♠`, contested
//! ones to `3NT`**, judged by the *child's* family (how many sides have made a
//! non-pass call).  `1♠ - 4♠` is inside the constructive ceiling; `1♠ (4♥)` is a
//! contested dead end.  A dead end's meaning is still read and recorded — only
//! its subtree is skipped.
//!
//! **The floor gate.**  A `calculated bid` child is a dead end unless a BBA
//! self-play corpus actually reaches it (`--selfplay`, `--corpus`,
//! `--min-reach`): the floor's continuations are worth recording where BBA's own
//! play goes, and nowhere else.
//!
//! **The reach gate** (`--reach-depth`, default 4).  The first two are not
//! enough — they cap the *level*, not the *length*.  Under the `3NT` contested
//! ceiling a single depth-6 node still has millions of descendants, because
//! every pass keeps the auction alive and every remaining bid below `3NT`
//! branches again; a probe of `1♠ (1NT) 2♣ (2♦) 2♥ (2♠) 2NT` passed 2000 nodes
//! without finishing.  So past `--reach-depth` calls, the corpus gate applies to
//! **every** child, not just the floor's: the book stays exhaustive to that
//! length and below it the walk follows where BBA actually goes.
//!
//! # Modes
//!
//! ```text
//! # (1) settle the ABI and the invariances
//! cargo run --release --features serde --example probe-bba-book -- --self-check
//!
//! # (2) a census of one opening, unbounded, to size a lane
//! cargo run --release --features serde --example probe-bba-book -- \
//!     --prefix "1♠" --vuls none --reach-depth 99 --max-depth 8 --output census.jsonl
//!
//! # (3) the reach corpus: BBA against BBA, every auction prefix counted
//! cargo run --release --features serde --example probe-bba-book -- \
//!     --selfplay 25000 --seed "$SEED_BASE" --output reach-0.jsonl
//!
//! # (4) cross-check the interpreter against hands BBA actually bids
//! cargo run --release --features serde --example probe-bba-book -- \
//!     --crosscheck 1000 --seed "$SEED_BASE"
//!
//! # (5) the full run, sharded (see scripts/bba-book.sh)
//! scripts/idle-run.sh scripts/bba-book.sh ab-results/bba-book/$(date +%F)-$(git rev-parse --short HEAD)
//!
//! # (6) look one lane up
//! cargo run --release --features serde --example probe-bba-book -- \
//!     --render ab-results/bba-book/latest --prefix "1♠ (2♥)"
//! ```
//!
//! EPBot is single-threaded and thread-unsafe, so this is one process on the
//! main thread; parallelism is by process, one shard per `--prefix`.

use clap::Parser;
use contract_bridge::auction::{Auction, Call, display_calls};
use contract_bridge::{AbsoluteVulnerability, Bid, Hand, Seat, Strain, Suit};
use std::collections::{BTreeMap, HashMap};
use std::ffi::{CString, c_int};
use std::io::Write;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::oracle::{
    BbaOracle, DEFAULT_LIB, Interpretation, SYSTEM_2_OVER_1, SeatInfo, load_bbsa, next_call,
};
use common::{hand_hcp, seat_to_act, seeded_deals};

// ───────────────────────────── arguments ─────────────────────────────

#[derive(Parser)]
#[command(about = "Walk EPBot's rule book: what every call means, by its rules")]
struct Args {
    /// Start the walk here, e.g. `"1♠"` or `"1NT (2♦) X"` (house format —
    /// `-` for pass, parentheses optional and ignored)
    ///
    /// `allow_hyphen_values` because a third- or fourth-seat prefix *begins*
    /// with a pass (`- - 1♠`, where Drury lives), and clap would otherwise read
    /// the leading `-` as a flag.
    #[arg(long, allow_hyphen_values = true, default_value = "")]
    prefix: String,

    /// Vulnerabilities to read every call under; identical readings collapse
    #[arg(long, value_delimiter = ',', default_value = "none,ns,ew,both")]
    vuls: Vec<AbsoluteVulnerability>,

    /// Convention card to put on **all four** EPBot seats, or `none` for the
    /// engine's own 2/1 defaults
    #[arg(long, default_value = "cards/American.bbsa")]
    card: String,

    /// Force one convention on top of the card, `--conv "Kickback 1430=1"`
    #[arg(long)]
    conv: Vec<String>,

    /// Directory of `reach-*.jsonl` files from `--selfplay`; gates which
    /// `calculated bid` children are expanded
    #[arg(long)]
    corpus: Option<String>,

    /// Expand a corpus-gated child once the corpus reaches it this often
    #[arg(long, default_value_t = 1)]
    min_reach: u64,

    /// Walk exhaustively to this auction length; deeper, follow only where the
    /// corpus goes
    ///
    /// **The ceilings alone do not bound the walk.** They cap the *level*, not
    /// the *length*: under the `3NT` contested ceiling a single depth-6 node
    /// still has millions of descendants, because every pass keeps the auction
    /// alive and every remaining bid below `3NT` branches again (measured — a
    /// probe of `1♠ (1NT) 2♣ (2♦) 2♥ (2♠) 2NT` passed 2000 nodes without
    /// finishing).  Past this length a child is expanded only when the corpus
    /// reaches it `--min-reach` times, which is the same gate `calculated bid`
    /// children already pass through, applied to all of them.
    ///
    /// The default keeps the whole book exhaustive through the advancer's first
    /// call and follows BBA's own play below that.  Raise it for a lane you want
    /// complete; `--reach-depth 99` restores the unbounded walk.
    #[arg(long, default_value_t = 4)]
    reach_depth: usize,

    /// Expand every `calculated bid` child, corpus or not (a census flag —
    /// the floor's tree is unbounded in practice)
    #[arg(long)]
    expand_calculated: bool,

    /// Safety stop: never expand an auction this long
    #[arg(long, default_value_t = 30)]
    max_depth: usize,

    /// Emit, rather than expand, nodes at this auction length; their keys go to
    /// `--frontier-out` for one shard each
    #[arg(long)]
    frontier: Option<usize>,

    /// Where the frontier key list goes (one key per line)
    #[arg(long, default_value = "frontier.txt")]
    frontier_out: String,

    /// Keep EPBot's prose `extended` reading in the dump
    ///
    /// Off by default because the census showed it is exactly `h` plus `n`
    /// re-spelled in English, at ~150 bytes a child — half the dump for nothing
    /// a reader cannot reconstruct.  Worth turning on for one lane when reading
    /// by eye, or to re-check that derivability after an EPBot upgrade.
    #[arg(long)]
    extended: bool,

    /// Bid out N boards BBA-vs-BBA and count every auction prefix instead of
    /// walking
    #[arg(long)]
    selfplay: Option<usize>,

    /// Bid out N boards and check whether the hand that made each call fits the
    /// hand-free interpreter's HCP and suit ranges
    #[arg(long)]
    crosscheck: Option<usize>,

    /// Deal-stream seed for `--selfplay`/`--crosscheck`; board `i` is seeded
    /// `seed + i`
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Print the tree map of a dump directory instead of walking
    #[arg(long)]
    render: Option<String>,

    /// Print dump statistics instead of walking
    #[arg(long)]
    stats: Option<String>,

    /// Keep dead ends the renderer hides, and show each call's extended reading
    #[arg(long)]
    verbose: bool,

    /// Print EPBot's convention table (ids 0..N) and exit
    #[arg(long)]
    conventions: Option<i32>,

    /// Audit the card: every engine id with the value the card wrote (`-` if
    /// the card has no row for it) and the value BBA actually plays, then exit
    #[arg(long)]
    effective: Option<i32>,

    /// Settle the ABI and the invariances, then exit
    #[arg(long)]
    self_check: bool,

    /// Path to `libEPBot.so`
    #[arg(long, default_value = DEFAULT_LIB)]
    lib: String,

    /// Write here instead of stdout
    #[arg(short, long)]
    output: Option<String>,
}

// ───────────────────────────── dump schema ─────────────────────────────

/// One expanded node: its key, its family, and every legal child's reading
#[derive(serde::Serialize, serde::Deserialize)]
struct Node {
    /// `display_calls` of the auction — house format, no parentheses
    k: String,
    /// `o` no side has bid, `c` one side has (constructive), `x` both (contested)
    f: char,
    /// How often a BBA self-play corpus reached this prefix
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    n: u64,
    c: Vec<Child>,
}

/// One legal call out of a node
#[derive(serde::Serialize, serde::Deserialize)]
struct Child {
    /// The call itself, `Display`-rendered (`2♦`, `X`, `XX`, `P`)
    c: String,
    /// Why the walk stopped here; absent means the subtree is its own node
    #[serde(default, skip_serializing_if = "Option::is_none")]
    x: Option<String>,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    n: u64,
    /// One entry per distinct reading, with the vulnerabilities it holds under
    r: Vec<Reading>,
}

/// What BBA says the call means, as a delta against the node's own state
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct Reading {
    /// Bitmask over `AbsoluteVulnerability`: none 1, ns 2, ew 4, both 8
    v: u8,
    /// BBA's own name for the rule, or `calculated bid` for the floor
    l: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    e: String,
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    a: i32,
    /// The caller's HCP band after the call, from `feature[402]`/`[403]`
    h: [i32; 2],
    /// Suit length bands the call **changed**, keyed `C`/`D`/`H`/`S`
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    n: BTreeMap<char, [i32; 2]>,
    /// Stopper flags the call changed
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    st: BTreeMap<char, i32>,
    /// `feature` slots the call changed, minus the HCP pair carried by `h`
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    f: BTreeMap<u16, i32>,
    /// Bitmask of the *other* positions whose public block the call moved
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    o: u8,
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

// ───────────────────────────── auction facts ─────────────────────────────

/// How many sides have made a non-pass call: 0 opening passes, 1 constructive,
/// 2 contested
///
/// A side is a position's parity, the dealer being canonicalized to position 0.
/// A double counts — `1♠ (X)` is contested.
fn family(auction: &[Call]) -> u8 {
    let mut sides = 0_u8;
    for (index, &call) in auction.iter().enumerate() {
        if call != Call::Pass {
            sides |= 1 << (index % 2);
        }
    }
    sides.count_ones() as u8
}

/// `o` / `c` / `x` for the three families
fn family_char(family: u8) -> char {
    ['o', 'c', 'x'][family as usize]
}

/// Highest bid the walk expands past, by the **child's** family
///
/// Constructive auctions run to game in the higher major; contested ones stop
/// at `3NT`, above which the interesting decision is the accountant's, not the
/// book's.  Judging by the child's family is what makes `1♠ (4♥)` a dead end
/// while `1♠ - 4♠` is not.
fn ceiling(family: u8) -> Bid {
    if family <= 1 {
        Bid::new(4, Strain::Spades)
    } else {
        Bid::new(3, Strain::Notrump)
    }
}

/// Every call, in the order the dump lists them: bids ascending, then X, XX, P
fn candidates() -> Vec<Call> {
    let mut calls = Vec::with_capacity(38);
    for level in 1..=7 {
        for strain in [
            Strain::Clubs,
            Strain::Diamonds,
            Strain::Hearts,
            Strain::Spades,
            Strain::Notrump,
        ] {
            calls.push(Call::Bid(Bid::new(level, strain)));
        }
    }
    calls.extend([Call::Double, Call::Redouble, Call::Pass]);
    calls
}

/// Parse a house-format auction, tolerating the renderer's parentheses
///
/// `common::auction_key` is deliberately not used: it strips leading passes, and
/// `- - 1♠` (a third-seat opening, where Drury lives) is a different node from
/// `1♠`.
fn parse_auction(text: &str) -> anyhow::Result<Auction> {
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

/// The house rendering: pass as `-`, the non-opening side's calls parenthesized
fn house(auction: &[Call]) -> String {
    use core::fmt::Write;
    let opener = auction.iter().position(|&call| call != Call::Pass);
    let mut out = String::new();
    for (index, &call) in auction.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        let theirs = opener.is_some_and(|first| index % 2 != first % 2);
        match call {
            Call::Pass => out.push('-'),
            call if theirs => write!(out, "({call})").expect("writing to a String never fails"),
            call => write!(out, "{call}").expect("writing to a String never fails"),
        }
    }
    out
}

/// Who is about to act, named relative to the opening side
fn role(auction: &[Call]) -> &'static str {
    let actor = auction.len() % 4;
    match auction.iter().position(|&call| call != Call::Pass) {
        None => ["1st", "2nd", "3rd", "4th"][actor],
        Some(first) => {
            ["opener", "overcaller", "responder", "advancer"][(actor + 4 - first % 4) % 4]
        }
    }
}

/// The bit this vulnerability owns in a [`Reading`]'s mask
fn vul_bit(vul: AbsoluteVulnerability) -> u8 {
    1 << vul.bits()
}

/// Render a reading's vulnerability mask back to names
fn vul_names(mask: u8) -> String {
    let names: Vec<&str> = [
        AbsoluteVulnerability::NONE,
        AbsoluteVulnerability::NS,
        AbsoluteVulnerability::EW,
        AbsoluteVulnerability::ALL,
    ]
    .into_iter()
    .filter(|&vul| mask & vul_bit(vul) != 0)
    .map(|vul| match vul {
        AbsoluteVulnerability::NS => "ns",
        AbsoluteVulnerability::EW => "ew",
        AbsoluteVulnerability::ALL => "both",
        _ => "none",
    })
    .collect();
    names.join("/")
}

/// EPBot's convention table, ids 0..173, baked from the vendored library
///
/// `feature[id]` is that convention's flag and `feature[511]` names the id that
/// last fired, so this is what turns a raw slot delta into `Kickback 1430`.  It
/// is baked rather than read so the renderer works on a dump alone; `--conventions`
/// diffs it against the live library and fails loudly if the vendored `.so` ever
/// moves under it.  Ids 173.. read `Not defined` — the filler slots `card.rs`'s
/// `PONS_SCHEMA` deliberately parks our own convention names on.
const CONVENTIONS: [&str; 173] = [
    "1D opening with 4 cards",
    "1D opening with 5 cards",
    "1m opening allows 5M",
    "1M-3M blocking",
    "1M-3M inviting",
    "1N-2S Minor Suit Stayman",
    "1N-2S transfer to clubs",
    "1N-2N transfer to clubs",
    "1N-2N transfer to diamonds",
    "1N-3C transfer to diamonds",
    "1N-3C Puppet Stayman",
    "1N-3D majors",
    "1N-3D minors",
    "1N-3D natural",
    "1N-3D splinter",
    "1N-3M splinter",
    "1NT opening allows less 1HCP",
    "1NT opening natural",
    "1NT opening NT style",
    "1NT opening range 12-14",
    "1NT opening range 13-15",
    "1NT opening range 14-16",
    "1NT opening range 15-17",
    "1NT opening shape 4441",
    "1NT opening shape 5422",
    "1NT opening shape 5 major",
    "1NT opening shape 6 minor",
    "(1X)-1Y-(1Z)-2Z natural",
    "1X-(Y)-2Z forcing",
    "1X-(1Y)-2Z strong",
    "1X-(1Y)-2Z weak",
    "2N-3C-3N both majors",
    "2N-3C Puppet Stayman",
    "2N-3S transfer to clubs",
    "2N-4C transfer to diamonds",
    "4NT opening",
    "5431 after 1NT",
    "5NT pick a slam",
    "Benjamin 2D",
    "Bergen",
    "Blackwood 0123",
    "Blackwood 0314",
    "Blackwood 1430",
    "Blackwood without K and Q",
    "BROMAD",
    "Cappelletti",
    "Checkback",
    "Collante",
    "Count signals",
    "Crosswood 0123",
    "Crosswood 0314",
    "Crosswood 1430",
    "Cue bid",
    "DEPO",
    "DOPI",
    "Drury",
    "Exclusion",
    "Extended Stayman",
    "Extended acceptance after NT",
    "Flannery",
    "Fit showing jumps",
    "Fitted 2NT",
    "Forcing 1NT",
    "Fourth suit",
    "Fourth suit game force",
    "French 2D",
    "Gambling",
    "Garbage Stayman",
    "Gazzilli",
    "Gerber",
    "Gerber only for NT openings",
    "Ghestem",
    "Grand Slam Force",
    "Imposible 2S",
    "Inverted count signals",
    "Inverted minors",
    "Inviting Jump Shifts",
    "Jacoby 2NT",
    "Jordan Truscott 2NT",
    "Jordan Truscott 2NT defence",
    "Kickback 0123",
    "Kickback 0314",
    "Kickback 1430",
    "King ask by 5NT",
    "King ask by 5NT inviting",
    "King ask by available bid",
    "Kokish Relay",
    "Landy",
    "Lavinthal from void",
    "Lavinthal on ace",
    "Lavinthal to void",
    "Leaping Michaels",
    "Lebensohl after 1NT",
    "Lebensohl after 1m",
    "Lebensohl after double",
    "Major Direct Jump Cuebid Gambling",
    "Major Direct Jump Cuebid Minor",
    "Major Direct Jump Cuebid Strong",
    "Mark on queen",
    "Mark on king",
    "Minor Direct Jump Cuebid Gambling",
    "Minor Direct Jump Cuebid Majors",
    "Minor Direct Jump Cuebid Preempt",
    "Maximal Doubles",
    "Michaels Cuebid",
    "Mini Splinter",
    "Minor Suit Slam Try after 2NT",
    "Minor Suit Stayman after 2NT",
    "Minor Suit Transfers after 2NT",
    "Mixed raise",
    "Multi",
    "Multi-Landy",
    "Namyats",
    "Natural 3N entering style",
    "Niemeijer",
    "New Minor Forcing",
    "NMF after 2NT rebid",
    "NMF by passed hand",
    "Non-Leaping Michaels",
    "Ogust",
    "Polish two suiters",
    "Precision 2D",
    "Quantitative 4NT",
    "Raptor 1NT",
    "Responsive double",
    "Reverse Bergen",
    "Reverse drury",
    "Reverse Flannery 2H",
    "Reverse Flannery 2S",
    "Reverse Smith Echo",
    "Rodrigue",
    "ROPI",
    "Roudi",
    "Rubensohl after 1NT",
    "Rubensohl after 1m",
    "Rubensohl after double",
    "Scrambling 2NT",
    "Semi forcing 1NT",
    "Shape Bergen structure",
    "Smith Echo",
    "SMOLEN",
    "Snapdragon Double",
    "Soloway Jump Shifts",
    "Soloway Jump Shifts Extended",
    "Splinter",
    "Strength Lawrence structure",
    "Strong natural 2D",
    "Strong natural 2M",
    "Strong jump shifts 2",
    "Strong jump shifts 3",
    "Super acceptance after NT",
    "Support 1NT",
    "Support double redouble",
    "Surplus pass",
    "Texas",
    "Transfers if RHO doubles",
    "Transfers if RHO bids clubs",
    "Two suit takeout double",
    "Two way game tries",
    "Two Way New Minor Forcing",
    "TWNMF by passed hand",
    "Unusual 1NT",
    "Unusual 2NT",
    "Unusual 3NT",
    "Unusual 4NT",
    "Unusual vs. Unusual",
    "Weak Jump Shifts 2",
    "Weak Jump Shifts 3",
    "Weak natural 2D",
    "Weak natural 2M",
    "Walsh style",
    "Wilkosz",
    "WJ 1C_1D allows majors",
];

/// `feature` slot names, from the `F_*` constants of EPBot's `ModuleCommon`
///
/// Only the two named regions are here: 300..325 (per-call bidding facts) and
/// 400..512 (per-hand and per-auction state).  Slots 0..173 are the convention
/// flags — [`CONVENTIONS`] names those — and 173..300 / 325..400 are dead space
/// the engine never touches.
///
/// Names are EPBot's own constants, `F_` stripped and lowercased, Polish and
/// all, so a slot in a dump greps straight back to the decompile.  The glossary
/// is in `docs/ai-bidder/bba-book.md`: `odzywka` = call, `kolor` = suit,
/// `pierwszy`/`ostatnia` = first/last, `sekwencja` = sequence, `zadane pytanie`
/// = question asked, `zgloszone` = shown, `krotkosc` = shortness, `dlugosc` =
/// length, `bilansowa` = of the balance engine (the floor).
const FEATURE_NAMES: [(u16, &str); 135] = [
    (300, "passed_first_bid"),
    (301, "odwrotka"),
    (302, "defensive_bid"),
    (303, "waiting_bid"),
    (304, "rubensohl_partner_suit"),
    (305, "weak_gazzilli"),
    (306, "one_two_over_1m"),
    (307, "ofensive_bid"),
    (308, "preemptive"),
    (309, "multi_landy_54"),
    (310, "otwarcie_czwartoreczne"),
    (311, "sign_off"),
    (312, "unusual_4nt"),
    (313, "surplus_cue"),
    (314, "alerting"),
    (315, "weak_natural_3x"),
    (316, "fit_showing_jumps"),
    (317, "otwarcie_strong_2nt"),
    (318, "pierwszy_kolor"),
    (319, "trump_queen"),
    (320, "showed_aces"),
    (321, "showed_kings"),
    (322, "showed_queens"),
    (323, "showed_jacks"),
    (324, "sekwencja_wj_1c_1m_2c_2d"),
    (400, "odzywka"),
    (401, "pierwsza_odzywka"),
    (402, "min_hcp"),
    (403, "max_hcp"),
    (404, "min_pkt"),
    (405, "max_pkt"),
    (406, "zgloszone_asy"),
    (407, "zgloszone_krole"),
    (408, "zgloszona_krotkosc"),
    (409, "dwukolorowka"),
    (410, "nt_style"),
    (411, "force_partner"),
    (412, "forcing_21"),
    (413, "nt_natural"),
    (414, "ending_bid"),
    (415, "balancing_double"),
    (416, "wywiad_bezatutowy"),
    (417, "odzywka_bilansowa"),
    (418, "penalty_double_redouble"),
    (419, "pierwszy_mlodszy"),
    (420, "pierwszy_starszy"),
    (421, "pozycja_sforsowana"),
    (422, "reopening_double"),
    (423, "rubensohl_forcing"),
    (424, "kolor_domniemany"),
    (425, "zadane_pytanie_o_asy"),
    (426, "potencjalne_pytanie_o_dame"),
    (427, "zadane_pytanie_o_dame"),
    (428, "potencjalne_pytanie_o_krole"),
    (429, "zadane_pytanie_o_krole"),
    (430, "omitted_cue_bid"),
    (431, "two_way_game_tries_void"),
    (432, "poprzednia_odzywka"),
    (433, "negat_after_polish_1c"),
    (434, "negat_after_splinter"),
    (435, "takeout_double"),
    (436, "strong_double"),
    (437, "alert"),
    (438, "pytanie_po_precision_2c"),
    (439, "polish_club_1c"),
    (440, "passed_suit"),
    (441, "rodzaj_zadanego_pytania_o_asy"),
    (442, "przeskok"),
    (443, "game_forcing"),
    (444, "artificial_bid"),
    (445, "sekwencja_checkback"),
    (446, "sekwencja_twnmf"),
    (447, "limit_raise_or_better"),
    (448, "first_fourth_suit"),
    (449, "second_fourth_suit"),
    (450, "sekwencja_wj_1c_1d_1m"),
    (451, "sekwencja_wj_1c_1d_2d"),
    (452, "sekwencja_wj_1c_1d_2d_2h"),
    (453, "sekwencja_wj_1c_1d_2d_2s"),
    (454, "sekwencja_wj_1c_1d_2d_3c"),
    (455, "sekwencja_wj_1c_1d_2d_3c_3d"),
    (456, "otwarcie_precision_2c"),
    (457, "sekwencja_precision_2c_2d"),
    (458, "sekwencja_precision_2c_2nt"),
    (459, "sekwencja_precision_2c_2d_2m"),
    (460, "precision_1c"),
    (461, "precision_1d"),
    (462, "otwarcie_strong_2c"),
    (463, "trap_pass_situation"),
    (464, "sekwencja_mini_splinter_3nt"),
    (465, "sekwencja_1nt_2c_2d"),
    (466, "sekwencja_2nt_3c_3d"),
    (467, "sekwencja_1nt_23d23h_23h23s"),
    (468, "precision_canape"),
    (469, "precision_4441"),
    (470, "precision_ask_4441"),
    (471, "negat_after_precision_1c"),
    (472, "sekwencja_multi_landy_1nt_2c_2d"),
    (473, "sekwencja_staymana"),
    (474, "sekwencja_wj_1c_1m_2c"),
    (475, "sekwencja_raptor_1nt_1x"),
    (477, "fourth_suit_hcp"),
    (478, "sekwencja_pc_1c_1h"),
    (479, "odzywka_po_nt_style"),
    (480, "blocking_bid"),
    (481, "gloszona_dlugosc"),
    (482, "gloszony_kolor_opponents"),
    (484, "negative_double"),
    (485, "2nt_response_to_flannery"),
    (486, "rebid_2nt_otwierajacego"),
    (487, "showed_hcp"),
    (488, "penalty_double_not_accepted"),
    (489, "lebensohl_2nt"),
    (490, "semi_natural_two_over_one"),
    (491, "en_passant"),
    (492, "otwarcie_acol"),
    (493, "sos_redouble"),
    (494, "sekwencja_1x_y_2z"),
    (495, "major_slam_try"),
    (496, "minor_slam_try"),
    (497, "aces_and_kings"),
    (498, "sekwencja_nmf"),
    (499, "strength_cue_bid"),
    (500, "sekwencja_garbage_stayman"),
    (501, "pominieto_1h"),
    (502, "excluded_figures"),
    (503, "marked_suit"),
    (504, "sekwencja_roudi"),
    (505, "sekwencja_wj_1d_1x_2n_or_3c"),
    (506, "potencjalne_pkt_partnera"),
    (507, "transfer_after_nt_style"),
    (508, "sekwencja_nmf_after_2nt_rebid"),
    (509, "ostatnia_odzywka"),
    (510, "triggered_convention"),
    (511, "used_convention"),
];

/// The `F_*` slot a name belongs to, or the raw number when it is unnamed
fn feature_name(slot: u16) -> String {
    if let Some((_, name)) = FEATURE_NAMES.iter().find(|&&(index, _)| index == slot) {
        return (*name).to_string();
    }
    match CONVENTIONS.get(slot as usize) {
        Some(convention) => format!("conv:{convention}"),
        None => format!("f{slot}"),
    }
}

/// `feature[511]` (and its two siblings) hold a convention id, not a count
const CONVENTION_VALUED: [u16; 3] = [441, 510, 511];

/// Render one `feature` delta as `name=value`, naming the convention where the
/// value *is* one
fn feature_delta(slot: u16, value: i32) -> String {
    let name = feature_name(slot);
    if CONVENTION_VALUED.contains(&slot)
        && let Ok(index) = usize::try_from(value)
        && let Some(convention) = CONVENTIONS.get(index)
    {
        return format!("{name}={convention}");
    }
    format!("{name}={value}")
}

/// EPBot's suit letters, in its own C, D, H, S array order
const SUIT_KEYS: [char; 4] = ['C', 'D', 'H', 'S'];

/// `!S` → `♠`, and the rest of EPBot's suit escapes
fn suit_symbols(label: &str) -> String {
    label
        .replace("!C", "♣")
        .replace("!D", "♦")
        .replace("!H", "♥")
        .replace("!S", "♠")
        .replace("!N", "NT")
}

// ───────────────────────────── the walk ─────────────────────────────

/// The full HCP band, which EPBot uses to mean "unconstrained"
const UNCONSTRAINED_HCP: [i32; 2] = [0, 37];

/// `feature` slots carried by [`Reading::h`], so the delta map need not repeat
/// them
const HCP_SLOTS: [u16; 2] = [402, 403];

/// Turn one interpretation into a [`Reading`], diffed against the node's own state
fn reading(
    before: &[SeatInfo; 4],
    after: &Interpretation,
    vul: AbsoluteVulnerability,
    keep_extended: bool,
) -> Reading {
    let caller = after.caller as usize;
    let (was, now) = (&before[caller], &after.public[caller]);
    let (lo, hi) = now.hcp_range();

    let mut lengths = BTreeMap::new();
    let mut stoppers = BTreeMap::new();
    for (index, key) in SUIT_KEYS.into_iter().enumerate() {
        if (was.min_length[index], was.max_length[index])
            != (now.min_length[index], now.max_length[index])
        {
            lengths.insert(key, [now.min_length[index], now.max_length[index]]);
        }
        if was.stoppers[index] != now.stoppers[index] {
            stoppers.insert(key, now.stoppers[index]);
        }
    }

    // A slot the call cleared is as much of a delta as one it set, so walk the
    // union of the two key sets rather than only the new nonzero entries.
    let mut features = BTreeMap::new();
    for slot in was.features.keys().chain(now.features.keys()) {
        if HCP_SLOTS.contains(slot) {
            continue;
        }
        let value = now.features.get(slot).copied().unwrap_or(0);
        if was.features.get(slot).copied().unwrap_or(0) != value {
            features.insert(*slot, value);
        }
    }

    let mut others = 0_u8;
    for (position, was) in before.iter().enumerate() {
        if position != caller && *was != after.public[position] {
            others |= 1 << position;
        }
    }

    Reading {
        v: vul_bit(vul),
        l: after.label.clone(),
        e: if keep_extended {
            after.extended.clone()
        } else {
            String::new()
        },
        a: after.alert,
        h: [lo, hi],
        n: lengths,
        st: stoppers,
        f: features,
        o: others,
    }
}

/// Fold a reading into a child's list, merging it with an identical earlier one
///
/// Vulnerability is the only axis the walk reads every node on, and most calls
/// mean the same thing at all four — collapsing them is what keeps the dump at
/// kilobytes per node instead of four times that.
fn merge(readings: &mut Vec<Reading>, mut new: Reading) {
    let bit = new.v;
    new.v = 0;
    for existing in readings.iter_mut() {
        let mask = existing.v;
        existing.v = 0;
        let same = *existing == new;
        existing.v = mask;
        if same {
            existing.v |= bit;
            return;
        }
    }
    new.v = bit;
    readings.push(new);
}

struct Walk<'a> {
    oracle: &'a BbaOracle,
    vuls: &'a [AbsoluteVulnerability],
    calls: Vec<Call>,
    reach: HashMap<String, u64>,
    args: &'a Args,
    frontier: Vec<String>,
    nodes: usize,
    bots: usize,
}

impl Walk<'_> {
    /// Read every legal child of one node, and say which of them to expand
    fn visit(&mut self, auction: &Auction) -> (Node, Vec<Call>) {
        let key = display_calls(auction).to_string();
        let before: Vec<[SeatInfo; 4]> = self
            .vuls
            .iter()
            .map(|&vul| {
                self.oracle
                    .public(vul, auction)
                    .expect("EPBot must allocate a bot")
            })
            .collect();
        self.bots += self.vuls.len();

        let mut children = Vec::new();
        let mut expand = Vec::new();
        let mut child = auction.clone();
        for &call in &self.calls {
            if child.try_push(call).is_err() {
                continue;
            }
            let child_key = display_calls(&child).to_string();
            let child_reach = self.reach.get(&child_key).copied().unwrap_or(0);

            let mut readings: Vec<Reading> = Vec::new();
            for (index, &vul) in self.vuls.iter().enumerate() {
                let read = self
                    .oracle
                    .interpret(vul, &child)
                    .expect("EPBot must allocate a bot");
                merge(
                    &mut readings,
                    reading(&before[index], &read, vul, self.args.extended),
                );
            }
            self.bots += self.vuls.len();

            let calculated = readings
                .iter()
                .all(|read| read.l == Interpretation::CALCULATED);
            let verdict: Option<&str> = if child.has_ended() {
                Some("end")
            } else if matches!(call, Call::Bid(bid) if bid > ceiling(family(&child))) {
                Some("ceil")
            } else if child.len() >= self.args.max_depth {
                Some("depth")
            } else if calculated
                && !self.args.expand_calculated
                && child_reach < self.args.min_reach
            {
                Some("calc")
            } else if child.len() > self.args.reach_depth && child_reach < self.args.min_reach {
                Some("reach")
            } else {
                None
            };

            if verdict.is_none() {
                // A frontier run stops here and hands the key to a shard; the
                // child record still says "expanded", because it is — elsewhere.
                if self.args.frontier.is_some_and(|at| child.len() >= at) {
                    self.frontier.push(child_key.clone());
                } else {
                    expand.push(call);
                }
            }

            children.push(Child {
                c: call.to_string(),
                x: verdict.map(str::to_owned),
                n: child_reach,
                r: readings,
            });
            child.pop();
        }

        self.nodes += 1;
        let node = Node {
            f: family_char(family(auction)),
            n: self.reach.get(&key).copied().unwrap_or(0),
            k: key,
            c: children,
        };
        (node, expand)
    }

    /// Depth-first, pre-order, one JSONL line per expanded node
    fn run(&mut self, root: Auction, out: &mut dyn Write) -> anyhow::Result<()> {
        let start = std::time::Instant::now();
        let mut stack = vec![root];
        while let Some(auction) = stack.pop() {
            let (node, expand) = self.visit(&auction);
            writeln!(out, "{}", serde_json::to_string(&node)?)?;
            // Reversed, so the stack pops the children back in dump order.
            for &call in expand.iter().rev() {
                let mut next = auction.clone();
                next.push(call);
                stack.push(next);
            }
            if self.nodes.is_multiple_of(200) {
                let seconds = start.elapsed().as_secs_f64();
                eprintln!(
                    "probe-bba-book: {} nodes, {} bots, {:.0} nodes/s, {} queued",
                    self.nodes,
                    self.bots,
                    self.nodes as f64 / seconds,
                    stack.len(),
                );
            }
        }
        eprintln!(
            "probe-bba-book: {} nodes, {} bots, {:.1}s",
            self.nodes,
            self.bots,
            start.elapsed().as_secs_f64(),
        );
        Ok(())
    }
}

// ───────────────────────────── the reach corpus ─────────────────────────────

/// One prefix and how often BBA's own play reached it
#[derive(serde::Serialize, serde::Deserialize)]
struct Reach {
    k: String,
    n: u64,
}

/// Bid out `count` boards BBA-vs-BBA and count every auction prefix
///
/// `bid_out` already seats the same oracle on both sides, so the corpus needs no
/// generator of its own.  Dealer and vulnerability rotate so no seat's or
/// vulnerability's book is over-weighted.
fn selfplay(
    oracle: &BbaOracle,
    count: usize,
    seed: u64,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    const VULS: [AbsoluteVulnerability; 4] = [
        AbsoluteVulnerability::NONE,
        AbsoluteVulnerability::NS,
        AbsoluteVulnerability::EW,
        AbsoluteVulnerability::ALL,
    ];
    let mut reach: HashMap<String, u64> = HashMap::new();
    let start = std::time::Instant::now();
    for (board, deal) in seeded_deals(seed, count).iter().enumerate() {
        let dealer = Seat::ALL[board % 4];
        let vul = VULS[(board / 4) % 4];
        let auction = common::oracle::bid_out(oracle, oracle, true, dealer, vul, deal);
        for length in 0..=auction.len() {
            *reach
                .entry(display_calls(&auction[..length]).to_string())
                .or_default() += 1;
        }
        if (board + 1) % 500 == 0 {
            eprintln!(
                "probe-bba-book: {} boards, {} prefixes, {:.1}s",
                board + 1,
                reach.len(),
                start.elapsed().as_secs_f64(),
            );
        }
    }
    let mut rows: Vec<Reach> = reach.into_iter().map(|(k, n)| Reach { k, n }).collect();
    rows.sort_by(|a, b| b.n.cmp(&a.n).then_with(|| a.k.cmp(&b.k)));
    for row in &rows {
        writeln!(out, "{}", serde_json::to_string(row)?)?;
    }
    eprintln!(
        "probe-bba-book: {count} boards, {} prefixes, {:.1}s",
        rows.len(),
        start.elapsed().as_secs_f64(),
    );
    Ok(())
}

#[derive(Default)]
struct CrosscheckLabel {
    decisions: usize,
    mismatches: usize,
    example: Option<String>,
}

/// Rotate table vulnerability into the walk's dealer-as-position-0 frame.
fn canonical_vulnerability(vul: AbsoluteVulnerability, dealer: Seat) -> AbsoluteVulnerability {
    if matches!(dealer, Seat::North | Seat::South) {
        vul
    } else if vul == AbsoluteVulnerability::NS {
        AbsoluteVulnerability::EW
    } else if vul == AbsoluteVulnerability::EW {
        AbsoluteVulnerability::NS
    } else {
        vul
    }
}

/// Why a real hand falls outside the same fields the renderer prints.
fn containment_failure(hand: Hand, read: &Reading) -> Option<String> {
    let mut failures = Vec::new();
    let hcp = i32::from(hand_hcp(hand));
    if !(read.h[0]..=read.h[1]).contains(&hcp) {
        failures.push(format!("HCP {hcp} outside {}..={}", read.h[0], read.h[1]));
    }
    for (&key, &[lo, hi]) in &read.n {
        let suit = match key {
            'C' => Suit::Clubs,
            'D' => Suit::Diamonds,
            'H' => Suit::Hearts,
            'S' => Suit::Spades,
            _ => unreachable!("the reader emits only C/D/H/S length keys"),
        };
        let length = hand[suit].len() as i32;
        if !(lo..=hi).contains(&length) {
            failures.push(format!("{key} length {length} outside {lo}..={hi}"));
        }
    }
    (!failures.is_empty()).then(|| failures.join(", "))
}

/// Does BBA's hand-free interpreter describe the hands its bidder actually uses?
fn crosscheck(
    oracle: &BbaOracle,
    count: usize,
    seed: u64,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    anyhow::ensure!(count != 0, "--crosscheck must be greater than zero");
    const VULS: [AbsoluteVulnerability; 4] = [
        AbsoluteVulnerability::NONE,
        AbsoluteVulnerability::NS,
        AbsoluteVulnerability::EW,
        AbsoluteVulnerability::ALL,
    ];
    let mut labels: BTreeMap<String, CrosscheckLabel> = BTreeMap::new();
    let mut decisions = 0;
    let mut mismatches = 0;
    let start = std::time::Instant::now();

    for (board, deal) in seeded_deals(seed, count).iter().enumerate() {
        let dealer = Seat::ALL[board % 4];
        let vul = VULS[(board / 4) % 4];
        let auction = common::oracle::bid_out(oracle, oracle, true, dealer, vul, deal);
        let canonical_vul = canonical_vulnerability(vul, dealer);
        let root = oracle
            .public(canonical_vul, &[])
            .ok_or_else(|| anyhow::anyhow!("EPBot failed to allocate a bot"))?;
        let path = oracle
            .interpret_path(canonical_vul, &auction)
            .ok_or_else(|| anyhow::anyhow!("EPBot failed to allocate a bot"))?;
        debug_assert_eq!(path.len(), auction.len());

        for (index, read) in path.iter().enumerate() {
            debug_assert_eq!(read.caller as usize, index % 4);
            let before = if index == 0 {
                &root
            } else {
                &path[index - 1].public
            };
            let rendered = reading(before, read, canonical_vul, false);
            let hand = deal[seat_to_act(dealer, index)];
            let stats = labels.entry(read.label.clone()).or_default();
            stats.decisions += 1;
            decisions += 1;
            if let Some(reason) = containment_failure(hand, &rendered) {
                stats.mismatches += 1;
                mismatches += 1;
                stats.example.get_or_insert_with(|| {
                    format!("{} | hand {hand} | {reason}", house(&auction[..=index]))
                });
            }
        }

        if (board + 1) % 100 == 0 {
            eprintln!(
                "probe-bba-book: cross-checked {} boards, {decisions} decisions, {:.1}s",
                board + 1,
                start.elapsed().as_secs_f64(),
            );
        }
    }

    writeln!(out, "boards: {count}  seed: {seed}")?;
    writeln!(
        out,
        "containment disagreements: {mismatches}/{decisions} ({:.3}%)",
        100.0 * mismatches as f64 / decisions as f64
    )?;
    writeln!(
        out,
        "bidder-label disagreements: unavailable — the outgoing meaning slot is empty or stale after get_bid; set_bid refreshes it through the interpreter"
    )?;

    let mut worst: Vec<_> = labels
        .iter()
        .filter(|(_, stats)| stats.mismatches != 0)
        .collect();
    worst.sort_by(|a, b| {
        b.1.mismatches
            .cmp(&a.1.mismatches)
            .then_with(|| b.1.decisions.cmp(&a.1.decisions))
            .then_with(|| a.0.cmp(b.0))
    });
    if worst.is_empty() {
        writeln!(out, "worst containment labels: none")?;
    } else {
        writeln!(out, "worst containment labels (by mismatches):")?;
        for (label, stats) in worst.into_iter().take(20) {
            writeln!(
                out,
                "  {:>6}/{:<6} {:>6.2}%  {}",
                stats.mismatches,
                stats.decisions,
                100.0 * stats.mismatches as f64 / stats.decisions as f64,
                if label.is_empty() {
                    "(unlabelled)"
                } else {
                    label
                },
            )?;
            if let Some(example) = &stats.example {
                writeln!(out, "                  {example}")?;
            }
        }
    }
    Ok(())
}

/// Sum every `reach-*.jsonl` in a directory
fn load_reach(dir: &str) -> anyhow::Result<HashMap<String, u64>> {
    let mut reach: HashMap<String, u64> = HashMap::new();
    let mut files = 0;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !name.starts_with("reach-") || !name.ends_with(".jsonl") {
            continue;
        }
        files += 1;
        for line in std::fs::read_to_string(&path)?.lines() {
            let row: Reach = serde_json::from_str(line)?;
            *reach.entry(row.k).or_default() += row.n;
        }
    }
    eprintln!(
        "probe-bba-book: corpus {dir}: {files} shard(s), {} prefixes",
        reach.len()
    );
    Ok(reach)
}

// ───────────────────────────── reading a dump back ─────────────────────────────

/// Every node of every `*.jsonl` shard, keyed by auction, plus the byte total
fn load_dump(dir: &str) -> anyhow::Result<(HashMap<String, Node>, u64)> {
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

/// A short English gloss of what the call promised, built from the arrays alone
fn gloss(read: &Reading) -> String {
    let mut parts = Vec::new();
    if read.h != UNCONSTRAINED_HCP {
        parts.push(match read.h {
            [lo, 37] => format!("{lo}+"),
            [0, hi] => format!("≤{hi}"),
            [lo, hi] if lo == hi => format!("{lo}"),
            [lo, hi] => format!("{lo}-{hi}"),
        });
    }
    for (index, key) in SUIT_KEYS.into_iter().enumerate() {
        let symbol = ['♣', '♦', '♥', '♠'][index];
        if let Some(&[min, max]) = read.n.get(&key) {
            match (min, max) {
                (0, 13) => {}
                (min, 13) => parts.push(format!("{min}+{symbol}")),
                (0, max) => parts.push(format!("≤{max}{symbol}")),
                (min, max) if min == max => parts.push(format!("{min}{symbol}")),
                (min, max) => parts.push(format!("{min}-{max}{symbol}")),
            }
        }
        if read.st.get(&key).is_some_and(|&value| value != 0) {
            parts.push(format!("{symbol}stop"));
        }
    }
    parts.join(" ")
}

/// The convention EPBot recorded as having fired on this call
///
/// `feature[511]` (`F_USED_CONVENTION`) is stamped by `ustaw_konwencje`, the one
/// routine that turns a convention on, so it names the rule by *id* where the
/// prose label only describes it.  `Blackwood 1430` and `Kickback 1430` share
/// the label `Blackwood 1430, for !x`; they do not share this.
fn fired(read: &Reading) -> Option<&'static str> {
    let id = usize::try_from(*read.f.get(&511)?).ok()?;
    CONVENTIONS.get(id).copied()
}

/// One child rendered as `2♠(3+♠)[support]`, with the flag suffixes
fn render_child(child: &Child) -> Option<String> {
    let first = child.r.first()?;
    let mut text = child.c.clone();
    let constraint = gloss(first);
    if !constraint.is_empty() {
        text.push_str(&format!("({constraint})"));
    }
    if !first.l.is_empty() && first.l != Interpretation::CALCULATED {
        text.push_str(&format!("[{}]", suit_symbols(&first.l)));
    }
    if let Some(convention) = fired(first) {
        text.push_str(&format!("{{{convention}}}"));
    }
    if first.l == Interpretation::CALCULATED {
        text.push('~');
    }
    if first.a != 0 {
        text.push('!');
    }
    if child.r.len() > 1 {
        text.push('†');
    }
    if let Some(verdict) = child.x.as_deref()
        && !matches!(verdict, "end" | "ceil")
    {
        text.push_str(&format!(" ⟨{verdict}⟩"));
    }
    Some(text)
}

/// The label a ceiling row is grouped by — its meaning, not its level
///
/// Above the ceiling BBA repeats itself: `4♣`, `5♣`, `6♣` and `7♣` all read
/// `preemptive`, four spellings of one fact.  Grouping on the label alone (not
/// the point band, which drifts a little with the level) turns 31 dead ends into
/// three or four, and loses nothing the dump does not still hold.
fn ceiling_key(child: &Child) -> Option<String> {
    Some(child.r.first()?.l.clone())
}

/// Print the house tree map of a dump, depth-first from `prefix`
fn render(nodes: &HashMap<String, Node>, prefix: &Auction, verbose: bool) {
    let mut dangling = 0_usize;
    let mut printed = 0_usize;
    let mut stack = vec![prefix.clone()];
    while let Some(auction) = stack.pop() {
        let key = display_calls(&auction).to_string();
        let Some(node) = nodes.get(&key) else {
            dangling += 1;
            eprintln!("probe-bba-book: dangling child `{key}` — a shard is missing");
            continue;
        };
        let (over, live): (Vec<&Child>, Vec<&Child>) = node
            .c
            .iter()
            .partition(|child| child.x.as_deref() == Some("ceil"));
        let entries: Vec<String> = live
            .iter()
            .filter_map(|child| render_child(child))
            .collect();
        if !entries.is_empty() {
            println!(
                "{:<34}{}: {}",
                house(&auction),
                role(&auction),
                entries.join(" / "),
            );
            printed += 1;
        }

        // Above the ceiling: one row per distinct meaning, lowest call first,
        // `+n` counting the higher calls that repeat it.  A meaningless dead end
        // (no label, or the floor's) is dropped entirely — `--verbose` keeps
        // every one of them.
        let mut seen: Vec<(String, String, usize)> = Vec::new();
        for child in over {
            let Some(key) = ceiling_key(child) else {
                continue;
            };
            let empty = child
                .r
                .first()
                .is_some_and(|read| read.l.is_empty() || read.l == Interpretation::CALCULATED);
            if empty && !verbose {
                continue;
            }
            match seen.iter_mut().find(|(seen, _, _)| *seen == key) {
                Some((_, _, count)) if !verbose => *count += 1,
                _ => {
                    if let Some(text) = render_child(child) {
                        seen.push((key, text, 0));
                    }
                }
            }
        }
        if !seen.is_empty() {
            let row: Vec<String> = seen
                .into_iter()
                .map(|(_, text, count)| {
                    if count == 0 {
                        text
                    } else {
                        format!("{text} +{count}")
                    }
                })
                .collect();
            println!("{:<34}  ⌃ {}", "", row.join(" / "));
        }
        for child in &node.c {
            for read in child.r.iter().skip(1) {
                println!(
                    "{:<34}  † {} {} = {}{}",
                    "",
                    child.c,
                    vul_names(read.v),
                    if read.l.is_empty() {
                        "(no label)".to_string()
                    } else {
                        format!("[{}]", suit_symbols(&read.l))
                    },
                    {
                        let constraint = gloss(read);
                        if constraint.is_empty() {
                            String::new()
                        } else {
                            format!(" {constraint}")
                        }
                    },
                );
            }
            if verbose {
                for read in &child.r {
                    if !read.e.is_empty() {
                        println!("{:<34}  · {} {}", "", child.c, suit_symbols(&read.e));
                    }
                    if !read.f.is_empty() {
                        let deltas: Vec<String> = read
                            .f
                            .iter()
                            .map(|(&slot, &value)| feature_delta(slot, value))
                            .collect();
                        println!("{:<34}  · {} {}", "", child.c, deltas.join(" "));
                    }
                }
            }
        }
        let mut next: Vec<Auction> = Vec::new();
        for child in &node.c {
            if child.x.is_some() {
                continue;
            }
            let Ok(call) = child.c.parse::<Call>() else {
                continue;
            };
            let mut deeper = auction.clone();
            if deeper.try_push(call).is_ok() {
                next.push(deeper);
            }
        }
        stack.extend(next.into_iter().rev());
    }
    eprintln!("probe-bba-book: {printed} node(s) printed, {dangling} dangling");
}

/// Nodes per depth and family, verdicts, labels, and how much of it is the floor
fn stats(nodes: &HashMap<String, Node>, bytes: u64) {
    let mut by_depth: BTreeMap<usize, [usize; 3]> = BTreeMap::new();
    let mut verdicts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut labels: HashMap<String, usize> = HashMap::new();
    let mut calls_by_family: [[usize; 2]; 3] = [[0; 2]; 3];
    let (mut readings, mut invariant) = (0_usize, 0_usize);
    let mut vuls = 0_u8;

    for node in nodes.values() {
        let depth = if node.k.is_empty() {
            0
        } else {
            node.k.split_whitespace().count()
        };
        let family = "ocx".find(node.f).unwrap_or(0);
        by_depth.entry(depth).or_default()[family] += 1;
        for child in &node.c {
            *verdicts
                .entry(child.x.as_deref().unwrap_or("expand"))
                .or_default() += 1;
            readings += 1;
            if child.r.len() == 1 {
                invariant += 1;
            }
            for read in &child.r {
                *labels.entry(read.l.clone()).or_default() += 1;
                vuls |= read.v;
            }
            let calculated = child
                .r
                .first()
                .is_some_and(|read| read.l == Interpretation::CALCULATED);
            calls_by_family[family][usize::from(calculated)] += 1;
        }
    }

    let total: usize = nodes.len();
    println!(
        "nodes {total}, bytes {bytes} ({:.0} B/node)",
        bytes as f64 / total as f64
    );
    println!("\ndepth  opening  constructive  contested");
    for (depth, counts) in &by_depth {
        println!(
            "{depth:>5}  {:>7}  {:>12}  {:>9}",
            counts[0], counts[1], counts[2]
        );
    }
    println!("\nverdicts");
    for (verdict, count) in &verdicts {
        println!("  {verdict:<8} {count:>8}");
    }
    println!("\nbook vs floor, by family");
    for (index, name) in ["opening", "constructive", "contested"]
        .into_iter()
        .enumerate()
    {
        let [book, floor] = calls_by_family[index];
        let total = book + floor;
        if total > 0 {
            println!(
                "  {name:<13} {book:>7} book  {floor:>7} calculated  ({:.1}% floor)",
                100.0 * floor as f64 / total as f64
            );
        }
    }
    // Vacuous unless the dump was read at more than one vulnerability: with a
    // single one every child has exactly one reading by construction.
    if vuls.count_ones() > 1 {
        println!(
            "\nvulnerability-invariant readings: {invariant}/{readings} ({:.1}%)",
            100.0 * invariant as f64 / readings as f64
        );
    } else {
        println!(
            "\nvulnerability: one only ({}) — invariance not measured here",
            vul_names(vuls)
        );
    }
    let mut histogram: Vec<(String, usize)> = labels.into_iter().collect();
    histogram.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    println!("\nlabels ({} distinct)", histogram.len());
    for (label, count) in histogram.iter().take(80) {
        println!(
            "  {count:>8}  {}",
            if label.is_empty() {
                "(pass — no label)"
            } else {
                label
            }
        );
    }
}

// ───────────────────────────── the self-check ─────────────────────────────

/// The battery: one auction per shape of the walk, including the two facts
/// `docs/ai-bidder/bba-kickback.md` and `docs/ai-bidder/bba-1nt-counter-defense.md`
/// already record.
const BATTERY: [&str; 15] = [
    "1♠",
    "1♠ - 2♠",
    "1♠ - 2♠ - 3♣",
    "1♠ - 2NT",
    "1♠ - 4NT",
    "1NT",
    "1NT - 2♣",
    "1NT 2♣",
    "1NT 2♦",
    "- - 1♠ - 2♣",
    "1♥ 1♠ X",
    "1♠ 2♥ - 3♥",
    "2♦",
    "- - - 1NT",
    "1♣ - 1♥ - 1♠ - 2♣",
];

fn self_check(oracle: &BbaOracle) -> anyhow::Result<()> {
    const VULS: [AbsoluteVulnerability; 4] = [
        AbsoluteVulnerability::NONE,
        AbsoluteVulnerability::NS,
        AbsoluteVulnerability::EW,
        AbsoluteVulnerability::ALL,
    ];
    let none = AbsoluteVulnerability::NONE;
    let hands: Vec<_> = seeded_deals(20_260_823, 8)
        .into_iter()
        .map(|deal| deal[Seat::North])
        .collect();

    let auctions: Vec<Auction> = BATTERY
        .iter()
        .map(|text| parse_auction(text))
        .collect::<anyhow::Result<_>>()?;

    // (a) hand-free vs dealt-hand.  If these disagree, the whole walk is reading
    // a system EPBot does not bid.
    let (mut same, mut prose, mut total) = (0, 0, 0);
    for auction in &auctions {
        let free = oracle
            .interpret(none, auction)
            .ok_or_else(|| anyhow::anyhow!("no bot"))?;
        let reader = (auction.len() % 4) as c_int;
        for &hand in &hands {
            let dealt = oracle
                .interpret_as(reader, Some(hand), none, auction)
                .ok_or_else(|| anyhow::anyhow!("no bot"))?;
            total += 1;
            same += usize::from(dealt.label == free.label);
            prose += usize::from(dealt.extended == free.extended);
            if dealt.label != free.label {
                println!(
                    "(a) LABEL MISMATCH {:<24} free `{}` vs dealt `{}`",
                    house(auction),
                    free.label,
                    dealt.label
                );
            } else if dealt.extended != free.extended {
                // Expected on `calculated bid`: with no rule to quote, the
                // extended reading paraphrases the hand model, and a dealt hand
                // is part of that model.  The label — the thing the walk records
                // — stays put, which is the invariance that matters.
                println!(
                    "(a) prose only  [{}] {:<24}\n      free  {}\n      dealt {}",
                    free.label,
                    house(auction),
                    free.extended,
                    dealt.extended
                );
            }
        }
    }
    println!("(a) hand-free == dealt: labels {same}/{total}, extended {prose}/{total}");

    // (b) reader-seat invariance.
    let (mut same, mut total) = (0, 0);
    for auction in &auctions {
        let base = oracle
            .interpret_as(0, None, none, auction)
            .ok_or_else(|| anyhow::anyhow!("no bot"))?;
        for reader in 1..4 {
            let other = oracle
                .interpret_as(reader, None, none, auction)
                .ok_or_else(|| anyhow::anyhow!("no bot"))?;
            total += 1;
            if other.label == base.label && other.public == base.public {
                same += 1;
            } else {
                println!(
                    "(b) MISMATCH {:<24} seat 0 `{}` vs seat {reader} `{}`",
                    house(auction),
                    base.label,
                    other.label
                );
            }
        }
    }
    println!("(b) reader-seat invariant: {same}/{total}");

    // (c) vulnerability invariance — how much four-vul reading actually buys.
    let mut invariant = 0;
    for auction in &auctions {
        let readings: Vec<_> = VULS
            .iter()
            .map(|&vul| oracle.interpret(vul, auction))
            .collect::<Option<_>>()
            .ok_or_else(|| anyhow::anyhow!("no bot"))?;
        if readings.windows(2).all(|pair| pair[0] == pair[1]) {
            invariant += 1;
        } else {
            println!(
                "(c) vul-dependent {:<24} {:?}",
                house(auction),
                readings.iter().map(|read| &read.label).collect::<Vec<_>>()
            );
        }
    }
    println!(
        "(c) vulnerability-invariant: {invariant}/{}",
        auctions.len()
    );

    // (d) what `extended` and the convention getters actually say.
    println!("(d) extended readings and convention getters");
    for auction in &auctions {
        let read = oracle
            .interpret(none, auction)
            .ok_or_else(|| anyhow::anyhow!("no bot"))?;
        println!(
            "    {:<24} alert {} [{}]\n        {}",
            house(auction),
            read.alert,
            read.label,
            read.extended
        );
        let used = oracle
            .convention_usage(none, auction, CONVENTIONS.len() as c_int)
            .ok_or_else(|| anyhow::anyhow!("no bot"))?;
        println!(
            "        conventions fired: {}",
            if used.is_empty() {
                "(none)".to_string()
            } else {
                used.iter()
                    .map(|(id, name, count)| format!("{id}:{name}×{count}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );
    }

    // (e) the buffer-too-small status, provoked deliberately.
    for bytes in [4_usize, 32, 1024] {
        let (status, text) = oracle
            .meaning_with_buffer(none, &auctions[0], bytes)
            .ok_or_else(|| anyhow::anyhow!("no bot"))?;
        println!("(e) buffer {bytes:>5} B → status {status:>3}, text `{text}`");
    }

    // (f) one bot replayed vs a fresh bot per prefix.
    let (mut same, mut total) = (0, 0);
    for auction in &auctions {
        let path = oracle
            .interpret_path(none, auction)
            .ok_or_else(|| anyhow::anyhow!("no bot"))?;
        for length in 1..=auction.len() {
            let fresh = oracle
                .interpret(none, &auction[..length])
                .ok_or_else(|| anyhow::anyhow!("no bot"))?;
            total += 1;
            if path[length - 1] == fresh {
                same += 1;
            } else {
                println!(
                    "(f) MISMATCH {:<24} path `{}` vs fresh `{}`",
                    house(&auction[..length]),
                    path[length - 1].label,
                    fresh.label
                );
            }
        }
    }
    println!("(f) replayed == fresh: {same}/{total}");

    // (g) does the vulnerability argument reach the engine at all?
    //
    // The walk measures every reading as vulnerability-invariant, which is only
    // a finding if the argument is live.  BBA's *bidding* is famously not
    // vul-blind — a marginal preempt is a different call red against green — so
    // driving `classify` at both vulnerabilities over a batch of hands separates
    // "EPBot's reader ignores vulnerability" from "our vul code is dead".
    let mut moved = 0;
    let deals = seeded_deals(20_260_824, 400);
    let empty = Auction::new();
    for deal in &deals {
        let hand = deal[Seat::North];
        let green = next_call(
            oracle,
            hand,
            Seat::North,
            AbsoluteVulnerability::NONE,
            &empty,
        );
        let red = next_call(
            oracle,
            hand,
            Seat::North,
            AbsoluteVulnerability::ALL,
            &empty,
        );
        moved += usize::from(green != red);
    }
    println!(
        "(g) opening call moves with vulnerability: {moved}/{} hands{}",
        deals.len(),
        if moved == 0 {
            "   ← ALARM: the vulnerability argument is not reaching EPBot"
        } else {
            ""
        }
    );
    Ok(())
}

// ───────────────────────────── main ─────────────────────────────

fn open(path: Option<&String>) -> anyhow::Result<Box<dyn Write>> {
    Ok(match path {
        Some(path) => Box::new(std::io::BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(std::io::BufWriter::new(std::io::stdout())),
    })
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Rendering and statistics never touch EPBot, so they run without the
    // library present.
    if let Some(dir) = &args.render {
        let (nodes, _) = load_dump(dir)?;
        render(&nodes, &parse_auction(&args.prefix)?, args.verbose);
        return Ok(());
    }
    if let Some(dir) = &args.stats {
        let (nodes, bytes) = load_dump(dir)?;
        stats(&nodes, bytes);
        return Ok(());
    }

    let (system, mut overrides) = if args.card == "none" {
        (SYSTEM_2_OVER_1, Vec::new())
    } else {
        let card = load_bbsa(&args.card)?;
        (card.system, card.toggles)
    };
    for row in &args.conv {
        let (name, value) = row
            .rsplit_once('=')
            .ok_or_else(|| anyhow::anyhow!("--conv wants `Name=value`, got `{row}`"))?;
        overrides.push((CString::new(name)?, value.trim().parse()?));
    }
    // Both sides get the same card: `with_opponents` is left unset, which is
    // exactly "the book BBA plays when told our agreements", on all four seats.
    let written: HashMap<String, c_int> = overrides
        .iter()
        .map(|(name, value)| {
            (
                name.to_str().expect("card rows are UTF-8").to_owned(),
                *value,
            )
        })
        .collect();
    let oracle = BbaOracle::load(&args.lib, system, overrides)?;

    if let Some(count) = args.conventions {
        let names = oracle
            .convention_names(count)
            .ok_or_else(|| anyhow::anyhow!("EPBot failed to allocate a bot"))?;
        let mut drift = 0;
        for (id, name) in names.iter().enumerate() {
            let baked = CONVENTIONS.get(id).copied().unwrap_or("Not defined");
            let flag = if baked == name {
                ""
            } else {
                drift += 1;
                "   ← DRIFT, baked: "
            };
            println!(
                "{id:>4}  {name}{flag}{}",
                if baked == name { "" } else { baked }
            );
        }
        anyhow::ensure!(
            drift == 0,
            "{drift} convention id(s) differ from the table baked into this example. \
             The vendored EPBot has moved under the dumps; re-bake CONVENTIONS."
        );
        return Ok(());
    }

    if let Some(count) = args.effective {
        let values = oracle
            .convention_values(count)
            .ok_or_else(|| anyhow::anyhow!("EPBot failed to allocate a bot"))?;
        for (id, name, value) in &values {
            let wrote = written
                .get(name.as_str())
                .map_or("-".to_owned(), |w| w.to_string());
            let flag = match written.get(name.as_str()) {
                None if *value != 0 => "   ← ON without a row",
                Some(w) if w != value => "   ← DIFFERS from the row",
                _ => "",
            };
            println!("{id:>4}  {name:<44} wrote {wrote:>2}  plays {value:>2}{flag}");
        }
        let orphans: Vec<_> = written
            .keys()
            .filter(|name| !values.iter().any(|(_, n, _)| n == *name))
            .collect();
        println!("rows the engine has no id for (inert): {orphans:?}");
        return Ok(());
    }

    if args.self_check {
        return self_check(&oracle);
    }

    let mut out = open(args.output.as_ref())?;
    if let Some(count) = args.selfplay {
        return selfplay(&oracle, count, args.seed, &mut out);
    }
    if let Some(count) = args.crosscheck {
        return crosscheck(&oracle, count, args.seed, &mut out);
    }

    let reach = match &args.corpus {
        Some(dir) => load_reach(dir)?,
        None => HashMap::new(),
    };
    let mut walk = Walk {
        oracle: &oracle,
        vuls: &args.vuls,
        calls: candidates(),
        reach,
        args: &args,
        frontier: Vec::new(),
        nodes: 0,
        bots: 0,
    };
    walk.run(parse_auction(&args.prefix)?, &mut out)?;
    out.flush()?;
    if args.frontier.is_some() {
        let list = walk.frontier.join("\n");
        std::fs::write(&args.frontier_out, list + "\n")?;
        eprintln!(
            "probe-bba-book: {} frontier key(s) → {}",
            walk.frontier.len(),
            args.frontier_out
        );
    }
    Ok(())
}
