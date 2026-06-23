//! AI-bidder **Side-track S.1** — the external eval anchor.
//!
//! A duplicate A/B match of our deterministic [`american`] floor against
//! **BBA's own 2/1 Game Force card**, driven natively through EPBot's C ABI
//! (`libEPBot.so`, no Wine — first proven by the since-removed S.0 `bba-oracle`
//! spike).  The
//! two systems play the *same* 2/1 system, so every divergence is a pure
//! quality gap between our authored DSL and a mature engine, not a difference
//! of methods.  This turns "did we improve?" into "how far are we from BBA?",
//! calibrating the M1/M3 learned-floor gains.
//!
//! The harness mirrors `examples/ab-instinct-floor`: each board is bid twice
//! (our pair North/South at table A, East/West at table B), boards whose two
//! tables reach different contracts are scored double dummy with `ddss`, and
//! the swing is credited to our pair.  A negative IMPs/board means BBA's 2/1
//! out-bids ours; the divergence dump lists the boards we lost by the most —
//! concrete under-/over-bidding auctions to author against.
//!
//! EPBot ships in the `vendor/bba` git submodule (BBA is free for non-commercial
//! use and redistribution); `git submodule update --init vendor/bba` resolves the
//! default library path, or point `BBA_LIB` elsewhere:
//!
//! ```text
//! cargo run --release --example bba-match -- --count 1000
//! BBA_LIB=/path/to/libEPBot.so cargo run --release --example bba-match
//! ```
//!
//! `--our-system <index>` swaps our side for a *second* EPBot card, turning the
//! harness into a BBA-vs-BBA experiment (e.g. `--our-system 2` is WJ / Polish
//! Club, `--system 0` 2/1).  Left unset, our side stays the [`american`] floor —
//! the original S.1 anchor, unchanged.

use clap::Parser;
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Level, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use libloading::Library;
use pons::american;
use pons::bidding::american::DoubleShape;
use pons::bidding::array::{Array, Logits};
use pons::bidding::context::relative;
use pons::bidding::{Family, System};
use pons::scoring::{final_contract, imps, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::BTreeMap;
use std::ffi::{CString, c_char, c_int, c_void};

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";

/// System index 0 = "2/1GF - 2/1 Game Force" (verified via `epbot_system_name`)
const SYSTEM_2_OVER_1: c_int = 0;

/// Measure our 2/1 floor against BBA's 2/1: an A/B duplicate match
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "200")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Number of worst (most-lost) divergent boards to dump
    #[arg(short, long, default_value = "15")]
    top: usize,

    /// EPBot system index for *their* side (0 = 2/1 Game Force, 2 = WJ)
    #[arg(short, long, default_value_t = SYSTEM_2_OVER_1)]
    system: c_int,

    /// Drive *our* side with EPBot at this system index too (BBA-vs-BBA
    /// experiment); unset = our authored `american` floor
    #[arg(long)]
    our_system: Option<c_int>,

    /// Which of our authored systems to seat: `american` (default) or
    /// `neural-v3` (the restrictive disclosable distilled floor; requires the
    /// `neural-floor` feature).  Ignored when `--our-system` selects an EPBot card.
    #[arg(long, default_value = "american")]
    our_floor: String,

    /// Force a named BBA convention on/off on *our* side (repeatable), e.g.
    /// `--our-conv "Rubensohl after 1m=1"`.  Only meaningful with `--our-system`.
    #[arg(long = "our-conv", value_parser = parse_override, value_name = "NAME=0|1")]
    our_conv: Vec<(CString, c_int)>,

    /// Force a named BBA convention on/off on *their* side (repeatable), e.g.
    /// `--their-conv "Rubensohl after 1m=0"`.  Pair with `--our-conv` to isolate
    /// one toggle in a BBA-vs-BBA A/B.
    #[arg(long = "their-conv", value_parser = parse_override, value_name = "NAME=0|1")]
    their_conv: Vec<(CString, c_int)>,

    /// Only keep deals with a balanced 15-17 HCP hand somewhere (a 1NT-opener
    /// candidate), to raise the yield of 1NT boards.  Cheap shape gate, no
    /// bidding; `--count` then means *kept* boards.  Pairs with the per-subset
    /// 1NT report printed after the headline.
    #[arg(long, default_value_t = false)]
    filter_1nt: bool,

    /// Enable our Unusual-vs-Unusual structure over 1NT-(2NT) — BBA overcalls our
    /// 1NT with a both-minors 2NT (Multi-Landy), so this is the live test.  Sets
    /// the responder structure + the encircling chase at the given floors.
    #[arg(long, default_value_t = false)]
    uvu: bool,

    /// Responder's penalty-double HCP floor for `--uvu`
    #[arg(long, default_value_t = 9)]
    uvu_x_floor: u8,

    /// Responder's INV+ cue-bid points floor for `--uvu`
    #[arg(long, default_value_t = 8)]
    uvu_cue_floor: u8,

    /// Deal seed for reproducible boards (pairs an `--uvu` on/off comparison so
    /// the boards UvU does not touch are identical and cancel); unset = random
    #[arg(long)]
    seed: Option<u64>,

    /// Read a `(2♦)` overcall of our 1NT as a Multi (an unknown major) and use our
    /// Multi counter-defense.  BBA's 2/1 card overcalls 1NT with Multi-Landy, whose
    /// 2♦ *is* a Multi, so this is the live test — pair with
    /// `--their-conv "Multi-Landy=1"` to be sure BBA bids it.  Seed-pair an on/off
    /// run to isolate it (the boards it does not touch cancel).
    #[arg(long, default_value_t = false)]
    defense_2d_multi: bool,

    /// Suppress our *own* 1NT opening (those 15-17 balanced hands open a minor),
    /// so every 1NT in the match is BBA's and our pair is purely the defender.
    /// Removes the confound where the same deal has us opening 1NT at the other
    /// table — the "OUR defense vs their 1NT" report then measures defense alone.
    #[arg(long, default_value_t = false)]
    no_our_1nt: bool,

    /// Cleanly isolate our DEFENSE to BBA's 1NT.  Keep only boards where BBA (E/W)
    /// opens 1NT and our pair (N/S) defends, and score table A (our defense)
    /// against an ALL-BBA reference table — same BBA opener and responses, only
    /// the defender differs (ours vs BBA).  The swing is then pure defense quality,
    /// free of the other-table constructive confound that `--no-our-1nt` leaves.
    /// `--count` means kept (we-defend) boards.
    #[arg(long, default_value_t = false)]
    isolate_defense: bool,

    /// Restore the legacy fifths gauge for our 1NT opening (default = plain HCP
    /// 15-17).  Seed-pair an on/off run (standard mode) to re-A/B the change.
    #[arg(long, default_value_t = false)]
    nt_fifths: bool,

    /// Shape gate for our natural penalty double of their 1NT: any (default, matches
    /// the shipped `american()`) | semi | balanced.  Tunes our defense against BBA
    /// (`--isolate-defense`); pass `balanced` to A/B the X-only-balanced restriction.
    #[arg(long, default_value = "any")]
    ns_double_shape: String,

    /// HCP floor for our natural penalty double of their 1NT (default 15).
    #[arg(long, default_value_t = 15)]
    ns_double_floor: u8,

    /// Inclusive `points` range LO:HI for our natural two-level suit overcall of
    /// their 1NT (default 8:14).  Raising HI lets a strong one-suiter overcall
    /// instead of falling through to the penalty double.
    #[arg(long, default_value = "8:14")]
    ns_overcall: String,

    /// Logit weight of our natural penalty double of their 1NT (default 1.3, above
    /// the 1.0 suit overcall).  Set below 1.0 so suit overcalls take precedence —
    /// a strong one-suiter overcalls instead of doubling (realistic suit-vs-X).
    #[arg(long, default_value_t = 1.3)]
    ns_double_weight: f32,

    /// Extend our 1NT defense to the balancing seat (1NT) P P ? (default off);
    /// on replaces the instinct floor's undisciplined balancing doubles.
    #[arg(long, default_value_t = false)]
    ns_balancing: bool,

    /// Advertise that our defense to BBA's 1NT is natural.  At *our* table only
    /// (where we defend) the opponent bot's 1NT-defense conventions are disabled
    /// (`Multi-Landy`/`Cappelletti`/`Landy` off, atop `--their-conv`), so BBA reads
    /// our two-level overcalls as natural rather than as its own Multi-Landy.  The
    /// all-BBA reference table keeps BBA's genuine Multi-Landy.  Use with
    /// `--isolate-defense`.
    #[arg(long, default_value_t = false)]
    advertise_natural: bool,

    /// Disable the settle floor ("pass = play the top bid" over a takeout double,
    /// default on) to A/B the floor change's effect on defense.  Seed-pair an
    /// on/off run.
    #[arg(long, default_value_t = false)]
    no_settle_floor: bool,
}

/// Parse a `NAME=0|1` convention override for `--our-conv` / `--their-conv`
fn parse_override(spec: &str) -> Result<(CString, c_int), String> {
    let (name, value) = spec
        .rsplit_once('=')
        .ok_or("expected NAME=0|1 (e.g. \"Rubensohl after 1m=1\")")?;
    let on = match value.trim() {
        "0" => 0,
        "1" => 1,
        other => return Err(format!("value must be 0 or 1, got `{other}`")),
    };
    let name = CString::new(name.trim()).map_err(|_| "name has an interior NUL".to_string())?;
    Ok((name, on))
}

/// EPBot system label for the indices we use (the pinned `vendor/bba` build)
fn system_label(system: c_int) -> &'static str {
    match system {
        0 => "2/1 Game Force",
        2 => "WJ (Polish Club)",
        _ => "EPBot system",
    }
}

/// Render convention overrides for a side's label, e.g. ` [Rubensohl after 1m=1]`
fn label_overrides(overrides: &[(CString, c_int)]) -> String {
    overrides
        .iter()
        .map(|(name, value)| format!(" [{}={value}]", name.to_string_lossy()))
        .collect()
}

// ---------------------------------------------------------------------------
// The BBA oracle: EPBot driven as a pons `System`
// ---------------------------------------------------------------------------

// Confirmed C ABI (objdump + EPBotFFI decompile + empirical bid codes); the
// S.0 spike documents the discovery.  Handles are opaque pointers.
type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
// `epbot_set_bid(bot, position, bid, meaning)` — the 4th arg is the bid's
// meaning string (decompiled from EPBotFFI.SetBid); an empty string is fine,
// EPBot interprets each bid itself from the configured system.
type SetBidFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;
// `epbot_set_conventions(bot, seat, name, on)` — per-seat convention toggle.
// Addressing (seat + name, NOT index) recovered from objdump and validated
// against `21GF.bbsa` (240/258 boolean toggles round-trip via get_conventions);
// `get_bid` genuinely consults the flag.  Lets `--our-conv`/`--their-conv`
// isolate one named convention in a BBA-vs-BBA A/B.
type SetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int) -> c_int;

/// EPBot 2/1 bidder behind pons's [`System`] trait.
///
/// Each [`System::classify`] call drives a *fresh* bot: it configures all four
/// seats to the chosen system, deals the actor's hand, replays the auction so
/// far with `set_bid` (one call per seat, rotating from a canonical dealer at
/// position 0), and reads the actor's call with `get_bid`.  A fresh bot per
/// decision keeps `classify` a pure, stateless function of its arguments —
/// exactly what the [`System`] contract wants.
///
/// Cached raw function pointers (copied out of the [`Library`]) avoid a `dlsym`
/// per call; `_lib` is held so the pointers stay valid for the oracle's life.
struct BbaOracle {
    _lib: Library,
    create: CreateFn,
    destroy: DestroyFn,
    set_system: SetSystemFn,
    new_hand: NewHandFn,
    set_bid: SetBidFn,
    get_bid: GetBidFn,
    set_conv: SetConvFn,
    system: c_int,
    /// Named conventions forced to a value on all four seats of every fresh bot,
    /// applied after `set_system` (which loads the system's defaults).  This is
    /// the single-toggle lever for the BBA-vs-BBA A/B: load both sides at the
    /// same `system` and override one convention on one side only.
    overrides: Vec<(CString, c_int)>,
}

impl BbaOracle {
    /// Load the EPBot library and bind the `epbot_*` symbols
    fn load(path: &str, system: c_int, overrides: Vec<(CString, c_int)>) -> anyhow::Result<Self> {
        // SAFETY: loading a trusted native library; its initializers run here.
        let lib = unsafe { Library::new(path) }?;
        // SAFETY: each symbol has the signature confirmed in the S.0 spike;
        // `*sym` copies the function pointer (it is `Copy` and does not borrow
        // the library, which we keep alive in `_lib`).
        unsafe {
            Ok(Self {
                create: *lib.get::<CreateFn>(b"epbot_create\0")?,
                destroy: *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
                set_system: *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
                new_hand: *lib.get::<NewHandFn>(b"epbot_new_hand\0")?,
                set_bid: *lib.get::<SetBidFn>(b"epbot_set_bid\0")?,
                get_bid: *lib.get::<GetBidFn>(b"epbot_get_bid\0")?,
                set_conv: *lib.get::<SetConvFn>(b"epbot_set_conventions\0")?,
                _lib: lib,
                system,
                overrides,
            })
        }
    }
}

impl System for BbaOracle {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        // Canonicalize the dealer to position 0: the actor is the seat that has
        // bid `auction.len()` times after the dealer.  The relative seat
        // (1st/2nd/3rd/4th to speak, passed-hand status) is preserved by the
        // replayed calls, and the favorable/unfavorable vulnerability by the
        // mapping below, so the bid is identical to the true seating.
        let actor = (auction.len() % 4) as c_int;
        let suits = hand_to_suits(hand);
        let empty = c"".as_ptr();

        // SAFETY: a fresh bot used and destroyed within this call; all argument
        // types match the confirmed ABI.
        let code = unsafe {
            let bot = (self.create)();
            if bot.is_null() {
                return None;
            }
            for seat in 0..4 {
                (self.set_system)(bot, seat, self.system);
            }
            // Force any isolated convention(s) AFTER set_system loads defaults.
            for (name, value) in &self.overrides {
                for seat in 0..4 {
                    (self.set_conv)(bot, seat, name.as_ptr(), *value);
                }
            }
            (self.new_hand)(
                bot,
                actor,
                suits.as_ptr(),
                0,
                epbot_vulnerability(vul, actor),
                0,
                0,
            );
            for (index, &call) in auction.iter().enumerate() {
                (self.set_bid)(bot, (index % 4) as c_int, encode_call(call), empty);
            }
            let code = (self.get_bid)(bot);
            (self.destroy)(bot);
            code
        };

        decode_call(code).map(one_hot)
    }
}

/// The four holdings in EPBot's C,D,H,S order, newline-joined
///
/// [`Holding`][contract_bridge::Holding]'s `Display` renders ranks high-to-low
/// using `T` for the ten — exactly EPBot's canonical form (verified by reading
/// the hand back with `epbot_get_cards`).  EPBot counts characters as cards, so
/// every hand must be exactly 13; `full_deal` guarantees that.  A void suit is
/// an empty segment, which EPBot reads as zero cards.
fn hand_to_suits(hand: Hand) -> CString {
    use core::fmt::Write;
    let mut suits = String::with_capacity(20);
    for (index, suit) in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .enumerate()
    {
        if index > 0 {
            suits.push('\n');
        }
        write!(suits, "{}", hand[suit]).expect("writing to a String never fails");
    }
    CString::new(suits).expect("a holding string never contains a NUL byte")
}

/// EPBot vulnerability code from the actor-relative vulnerability
///
/// EPBot seats even (0, 2) are North/South and odd (1, 3) East/West; its
/// vulnerability bits are 1 = N/S, 2 = E/W.  With the dealer canonicalized to
/// position 0, the actor's side is N/S iff `actor` is even.  `none` maps to 0
/// and `both` to 3 regardless of direction; the N/S-vs-E/W direction (the only
/// unverified bit) matters solely for the half-vulnerable runs.
fn epbot_vulnerability(vul: RelativeVulnerability, actor: c_int) -> c_int {
    let we = vul.contains(RelativeVulnerability::WE);
    let they = vul.contains(RelativeVulnerability::THEY);
    let (ns, ew) = if actor % 2 == 0 {
        (we, they)
    } else {
        (they, we)
    };
    c_int::from(ns) | (c_int::from(ew) << 1)
}

/// Encode a [`Call`] into EPBot's integer bid code
///
/// `0/1/2 = Pass/X/XX`; a bid is `5 + (level - 1) * 5 + strain` with strain
/// `0 = ♣ … 4 = NT`, matching [`Strain`]'s discriminant order.
fn encode_call(call: Call) -> c_int {
    match call {
        Call::Pass => 0,
        Call::Double => 1,
        Call::Redouble => 2,
        Call::Bid(bid) => 5 + (c_int::from(bid.level.get()) - 1) * 5 + strain_index(bid.strain),
    }
}

/// Decode EPBot's bid code back into a [`Call`], or [`None`] on an error code
fn decode_call(code: c_int) -> Option<Call> {
    match code {
        0 => Some(Call::Pass),
        1 => Some(Call::Double),
        2 => Some(Call::Redouble),
        5..=39 => {
            let index = code - 5;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let level = Level::new((index / 5 + 1) as u8);
            Some(Call::Bid(Bid {
                level,
                strain: STRAINS[(index % 5) as usize],
            }))
        }
        _ => None,
    }
}

/// Strains in EPBot/[`Strain`] discriminant order (♣ ♦ ♥ ♠ NT)
const STRAINS: [Strain; 5] = [
    Strain::Clubs,
    Strain::Diamonds,
    Strain::Hearts,
    Strain::Spades,
    Strain::Notrump,
];

/// The 0..=4 index of a strain
fn strain_index(strain: Strain) -> c_int {
    STRAINS
        .iter()
        .position(|&s| s == strain)
        .expect("every strain is in STRAINS") as c_int
}

/// One-hot logits: the chosen call finite, everything else impossible
fn one_hot(call: Call) -> Logits {
    Logits(Array::from_fn(|c| {
        if c == call { 0.0 } else { f32::NEG_INFINITY }
    }))
}

// ---------------------------------------------------------------------------
// Driving the match (mirrors examples/instinct-floor)
// ---------------------------------------------------------------------------

/// The seat acting after `len` calls from `dealer`
const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

/// The highest-logit *legal* call, defaulting to a pass
fn next_call(
    system: &dyn System,
    hand: Hand,
    seat: Seat,
    vul: AbsoluteVulnerability,
    auction: &Auction,
) -> Call {
    let Some(logits) = system.classify(hand, relative(vul, seat), auction) else {
        return Call::Pass;
    };
    let mut scored: Vec<(Call, f32)> = logits
        .iter()
        .map(|(call, &logit)| (call, logit))
        .filter(|&(_, logit)| logit.is_finite())
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
    scored
        .into_iter()
        .map(|(call, _)| call)
        .find(|&call| auction.can_push(call).is_ok())
        .unwrap_or(Call::Pass)
}

/// Bid out one deal with our pair on `ours_is_ns`'s side, BBA on the other
fn bid_out(
    ours: &dyn System,
    bba: &dyn System,
    ours_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let system = if seat_is_ns == ours_is_ns { ours } else { bba };
        auction.push(next_call(system, deal[seat], seat, vul, &auction));
    }
    auction
}

/// Render an auction with leading passes kept, calls space-joined
fn show_auction(auction: &Auction) -> String {
    auction
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// Isolating the 1NT subset (openings and our defense), for --filter_1nt
// ---------------------------------------------------------------------------

/// Total HCP of a hand (the `examples/common` pattern; bba-match owns its helpers)
fn hand_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

/// Balanced (no singleton/void, at most one doubleton) with 15-17 HCP — a strict
/// 1NT-opener gate for the cheap pre-filter.
fn is_1nt_opener(hand: Hand) -> bool {
    let len = Suit::ASC.map(|s| hand[s].len());
    let balanced = len.iter().all(|&l| l >= 2) && len.iter().filter(|&&l| l == 2).count() <= 1;
    balanced && (15..=17).contains(&hand_hcp(hand))
}

/// If this auction's *opening* call is 1NT, its index and whether the opener is
/// North/South.  The opening requirement (all prior calls passes) excludes a
/// `1♣-P-1NT` rebid — we want 1NT *openings* only.
fn opening_1nt(auction: &[Call], dealer: Seat) -> Option<(usize, bool)> {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let index = auction.iter().position(|&call| call == one_nt)?;
    if auction[..index].iter().any(|&call| call != Call::Pass) {
        return None;
    }
    let opener_ns = matches!(seat_to_act(dealer, index), Seat::North | Seat::South);
    Some((index, opener_ns))
}

/// Our first call after the 1NT opening.  At table A our pair sits North/South,
/// so this is our action whether we opened (responder) or defended (overcaller),
/// skipping any opposing call in between.  Captures `Pass` too.
fn first_ns_call_after(auction: &[Call], dealer: Seat, nt_index: usize) -> Option<Call> {
    auction[nt_index + 1..]
        .iter()
        .enumerate()
        .find_map(|(off, &call)| {
            matches!(
                seat_to_act(dealer, nt_index + 1 + off),
                Seat::North | Seat::South
            )
            .then_some(call)
        })
}

/// The 1NT opener's partner's (responder's) first call after the opening — i.e.
/// what the opponents responded once we did *not* overcall.  Their partner sits
/// two seats after the opener (`+2` is partner in the N,E,S,W rotation).  `None`
/// if the responder never gets to call.
fn responder_call_after(auction: &[Call], dealer: Seat, nt_index: usize) -> Option<Call> {
    let responder = seat_to_act(dealer, nt_index + 2);
    auction[nt_index + 1..]
        .iter()
        .enumerate()
        .find_map(|(off, &call)| {
            (seat_to_act(dealer, nt_index + 1 + off) == responder).then_some(call)
        })
}

/// Short bucket label for a responder/defender call (`P`, `2♣`, `X`, …)
fn action_label(call: Call) -> String {
    match call {
        Call::Pass => "P".into(),
        Call::Double => "X".into(),
        Call::Redouble => "XX".into(),
        Call::Bid(bid) => bid.to_string(),
    }
}

/// Sample mean and the half-width of its 95% confidence interval
///
/// The mean is the headline IMPs/board; the half-width is `1.96 · SE` from the
/// per-board sample standard deviation, so a CI that excludes 0 is a result
/// distinguishable from noise.
#[allow(clippy::cast_precision_loss)]
fn mean_with_ci(values: &[i64]) -> (f64, f64) {
    let n = values.len();
    if n < 2 {
        return (0.0, 0.0);
    }
    let mean = values.iter().sum::<i64>() as f64 / n as f64;
    let variance = values
        .iter()
        .map(|&v| {
            let d = v as f64 - mean;
            d * d
        })
        .sum::<f64>()
        / (n - 1) as f64;
    (mean, 1.96 * (variance / n as f64).sqrt())
}

/// One board: the deal, the dealer, and both tables' auctions
struct Board {
    deal: FullDeal,
    dealer: Seat,
    /// Our pair North/South
    table_a: Auction,
    /// Our pair East/West
    table_b: Auction,
}

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let bba = match BbaOracle::load(&path, args.system, args.their_conv.clone()) {
        Ok(bba) => bba,
        Err(error) => {
            eprintln!(
                "could not load EPBot native lib at `{path}`: {error}\n\
                 Fetch it with `git submodule update --init vendor/bba`, or set BBA_LIB."
            );
            std::process::exit(1);
        }
    };
    // When advertising natural, the opponent bot at *our* table reads our 1NT
    // overcalls naturally: disable its 1NT-defense conventions on top of
    // `--their-conv`.  Used only where `ours` defends; the all-BBA reference keeps
    // the plain `bba` (BBA's genuine Multi-Landy).
    let bba_vs_natural = if args.advertise_natural {
        let mut conv = args.their_conv.clone();
        for name in ["Multi-Landy", "Cappelletti", "Landy"] {
            conv.push((CString::new(name).expect("a literal name has no NUL"), 0));
        }
        Some(BbaOracle::load(&path, args.system, conv)?)
    } else {
        None
    };
    // Our side: the authored floor by default, or a second EPBot card when
    // `--our-system` is given (the BBA-vs-BBA experiment).  Both live to the end
    // of `main`, so `ours` can borrow whichever is selected.
    if args.uvu {
        // Read at book construction (responder structure) and, for the encircling
        // chase, at classify time — fine here, the match is sequential (FFI).
        pons::bidding::american::set_uvu(true);
        pons::bidding::american::set_uvu_x_floor(args.uvu_x_floor);
        pons::bidding::american::set_uvu_cue_floor(args.uvu_cue_floor);
        pons::bidding::instinct::set_uvu_encircle(true);
    }
    // Book-construction TLS (responder structure over our overcalled 1NT), baked
    // into `our_floor` below — like `set_uvu`, no per-worker reset needed.
    pons::bidding::american::set_defense_to_2d_multi(args.defense_2d_multi);
    // Classify-time TLS, set once on this (sequential) main thread: the settle
    // floor governs partner's takeout-double continuations, including the
    // competitive seam after our defense to 1NT.  Off = the pre-9badc15 floor.
    pons::bidding::instinct::set_settle_floor(!args.no_settle_floor);
    // Read at book construction (the opening table): suppress our own 1NT opening
    // so the duplicate's other table can't reintroduce a we-open-1NT swing.
    pons::bidding::american::set_open_one_notrump(!args.no_our_1nt);
    pons::bidding::american::set_one_notrump_fifths(args.nt_fifths);
    // Defense tuning knobs (read at book construction, baked into `our_floor`).
    pons::bidding::american::set_natural_double_shape(match args.ns_double_shape.as_str() {
        "any" => DoubleShape::Any,
        "semi" => DoubleShape::SemiBalanced,
        "balanced" => DoubleShape::Balanced,
        other => anyhow::bail!("--ns-double-shape must be any|semi|balanced, got {other:?}"),
    });
    pons::bidding::american::set_natural_double_floor(args.ns_double_floor);
    pons::bidding::american::set_natural_double_weight(args.ns_double_weight);
    pons::bidding::american::set_notrump_balancing(args.ns_balancing);
    let (oc_lo, oc_hi) = args
        .ns_overcall
        .split_once(':')
        .and_then(|(lo, hi)| Some((lo.parse::<u8>().ok()?, hi.parse::<u8>().ok()?)))
        .ok_or_else(|| {
            anyhow::anyhow!("--ns-overcall must be LO:HI, got {:?}", args.ns_overcall)
        })?;
    pons::bidding::american::set_natural_overcall_points(oc_lo, oc_hi);
    let our_floor = match args.our_floor.as_str() {
        "american" => american().against(Family::NATURAL),
        #[cfg(feature = "neural-floor")]
        "neural-v3" => pons::american_neural_v3().against(Family::NATURAL),
        other => anyhow::bail!(
            "--our-floor must be american{}, got {other:?}",
            if cfg!(feature = "neural-floor") {
                " or neural-v3"
            } else {
                " (neural-v3 needs --features neural-floor)"
            }
        ),
    };
    let our_oracle = match args.our_system {
        Some(system) => Some(BbaOracle::load(&path, system, args.our_conv.clone())?),
        None => None,
    };
    let ours: &dyn System = match &our_oracle {
        Some(oracle) => oracle,
        None => &our_floor,
    };
    // The opponent our pair faces: the natural-advertised bot if `--advertise-natural`,
    // else plain `bba`.  The all-BBA reference table always uses plain `bba`.
    let opponent: &dyn System = match &bba_vs_natural {
        Some(oracle) => oracle,
        None => &bba,
    };
    let our_label = match args.our_system {
        Some(system) => format!(
            "BBA {}{}",
            system_label(system),
            label_overrides(&args.our_conv)
        ),
        None => format!("our {} floor", args.our_floor),
    };
    let their_label = format!(
        "BBA {}{}",
        system_label(args.system),
        label_overrides(&args.their_conv)
    );
    let seed = args.seed.unwrap_or_else(rand::random);
    let mut rng = StdRng::seed_from_u64(seed);

    // Bid every board at both tables, dealer rotating per board.  Sequential by
    // design: each EPBot decision creates/destroys a native bot through the FFI,
    // which we do not assume is thread-safe (only the DD solver parallelizes).
    // With --filter_1nt, keep dealing until `count` deals carry a 1NT-opener
    // candidate; `scanned` records how many were drawn to get there.
    let mut boards: Vec<Board> = Vec::with_capacity(args.count);
    let mut scanned = 0usize;
    while boards.len() < args.count {
        let deal = full_deal(&mut rng);
        scanned += 1;
        if args.filter_1nt && !Seat::ALL.iter().any(|&seat| is_1nt_opener(deal[seat])) {
            continue;
        }
        let dealer = Seat::ALL[boards.len() % 4];
        let table_a = bid_out(ours, opponent, true, dealer, args.vulnerability, &deal);
        let table_b = if args.isolate_defense {
            // Keep only boards where BBA (E/W) opened 1NT and our N/S defended,
            // and compare against an all-BBA table: same BBA opener + responses,
            // only the defender differs.  The swing is then pure defense quality.
            if !matches!(opening_1nt(&table_a, dealer), Some((_, false))) {
                continue;
            }
            bid_out(&bba, &bba, true, dealer, args.vulnerability, &deal)
        } else {
            bid_out(ours, opponent, false, dealer, args.vulnerability, &deal)
        };
        boards.push(Board {
            deal,
            dealer,
            table_a,
            table_b,
        });
    }

    // Only boards whose tables reach different contracts can swing; solve those
    // double dummy and credit the swing to our pair (NS at A, EW at B).
    let contracts: Vec<_> = boards
        .iter()
        .map(|board| {
            (
                final_contract(&board.table_a, board.dealer),
                final_contract(&board.table_b, board.dealer),
            )
        })
        .collect();
    let divergent: Vec<usize> = (0..boards.len())
        .filter(|&index| contracts[index].0 != contracts[index].1)
        .collect();
    let deals: Vec<FullDeal> = divergent.iter().map(|&index| boards[index].deal).collect();
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let mut total_points = 0i64;
    // Per-board IMP swing over *all* boards (0 for non-divergent), for the mean
    // and its confidence interval.
    let mut board_imps = vec![0i64; boards.len()];
    // Per divergent board: (board index, point swing, IMP swing) for the dump.
    let mut swings: Vec<(usize, i64, i64)> = Vec::with_capacity(divergent.len());
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let swing = ns_score_contract(contract_a, table, args.vulnerability)
            - ns_score_contract(contract_b, table, args.vulnerability);
        total_points += swing;
        board_imps[index] = imps(swing);
        swings.push((index, swing, imps(swing)));
    }
    let total_imps: i64 = board_imps.iter().sum();

    let (mean, half_width) = mean_with_ci(&board_imps);
    println!(
        "=== {} (us) vs {} (them): {} boards, vulnerability {} ===",
        our_label, their_label, args.count, args.vulnerability,
    );
    println!(
        "Divergent boards: {} of {} ({:.0}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "Our pair: {total_points:+} points, {total_imps:+} IMPs\n\
         IMPs/board: {mean:+.3}  (95% CI [{:+.3}, {:+.3}])",
        mean - half_width,
        mean + half_width,
    );
    if args.isolate_defense {
        println!(
            "(defense isolation: every board is BBA-opens-1NT / we-defend; \
             table B is an ALL-BBA reference, so the swing is our defense vs BBA's)"
        );
    }
    if args.advertise_natural {
        println!(
            "(advertise-natural: at our table BBA reads our overcalls as natural \
             — Multi-Landy/Cappelletti/Landy off; the reference keeps BBA's own)"
        );
    }
    if args.no_settle_floor {
        println!("(settle floor OFF: pre-9badc15 takeout-double continuations)");
    }

    // Isolate the 1NT subset, keyed on table A (where our pair sits NS): boards
    // whose opening call is 1NT, split by who opened (NS = our opening, EW = our
    // defense), and bucketed by our first call so a leak localizes to a single
    // continuation.  Divergence is still measured at both tables; this only
    // attributes the swing to the convention that ran at our table.
    let mut open = (0i64, 0i64);
    let mut open_by: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    let mut defend = (0i64, 0i64);
    let mut defend_by: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    // Same we-defend boards, re-bucketed to test the hypothesis that the leak is
    // in our defense to their *responses*, not our direct overcall/double: DIRECT
    // (we acted over 1NT), CONT <call> (we passed, they responded with <call>), or
    // QUIET (we passed, they passed).
    let mut defend_shape_by: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    // Focus subset: we open 1NT and the overcall is exactly 2NT — the auctions
    // the UvU counter-measures act on, bucketed by our response.
    let two_nt = Call::Bid(Bid::new(2, Strain::Notrump));
    let mut uvu = (0i64, 0i64);
    let mut uvu_by: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    for &(index, _points, imp) in &swings {
        let board = &boards[index];
        let Some((nt_index, opener_ns)) = opening_1nt(&board.table_a, board.dealer) else {
            continue;
        };
        let our_direct = first_ns_call_after(&board.table_a, board.dealer, nt_index);
        let key = our_direct.map_or_else(|| "(none)".into(), action_label);
        let is_uvu = opener_ns && board.table_a.get(nt_index + 1) == Some(&two_nt);
        let (sum, by) = if opener_ns {
            (&mut open, &mut open_by)
        } else {
            (&mut defend, &mut defend_by)
        };
        sum.0 += 1;
        sum.1 += imp;
        let entry = by.entry(key.clone()).or_default();
        entry.0 += 1;
        entry.1 += imp;
        // Hypothesis probe (we-defend only): did the swing come from our DIRECT
        // action over their 1NT, or from the CONTinuation after they responded?
        if !opener_ns {
            let shape = match our_direct {
                Some(call) if call != Call::Pass => "DIRECT (we bid over 1NT)".to_string(),
                _ => match responder_call_after(&board.table_a, board.dealer, nt_index) {
                    Some(call) if call != Call::Pass => format!("CONT {}", action_label(call)),
                    _ => "QUIET (we passed, they passed)".to_string(),
                },
            };
            let entry = defend_shape_by.entry(shape).or_default();
            entry.0 += 1;
            entry.1 += imp;
        }
        if is_uvu {
            uvu.0 += 1;
            uvu.1 += imp;
            let entry = uvu_by.entry(key).or_default();
            entry.0 += 1;
            entry.1 += imp;
        }
    }
    let report = |title: &str, sum: (i64, i64), by: &BTreeMap<String, (i64, i64)>| {
        println!(
            "\n=== {title} === ({} divergent boards, {:+} IMPs, {:+.3} IMPs/board)",
            sum.0,
            sum.1,
            sum.1 as f64 / sum.0.max(1) as f64,
        );
        for (action, &(boards_n, imps_won)) in by {
            println!(
                "  {action:<5} {boards_n:>5} boards  {imps_won:+6} IMPs  ({:+.3} IMPs/board)",
                imps_won as f64 / boards_n.max(1) as f64,
            );
        }
    };
    report("OUR 1NT openings (we open 1NT)", open, &open_by);
    report(
        "OUR defense vs their 1NT (they open 1NT)",
        defend,
        &defend_by,
    );
    report(
        "OUR defense vs their 1NT, by auction shape (DIRECT vs CONTinuation)",
        defend,
        &defend_shape_by,
    );
    report("OUR 1NT-(2NT) responses (focus)", uvu, &uvu_by);
    if args.filter_1nt {
        println!(
            "\n(pre-filtered to deals with a 15-17 balanced hand: kept {} of {scanned}, {:.1}%)",
            boards.len(),
            100.0 * boards.len() as f64 / scanned.max(1) as f64,
        );
    }

    // The boards we lost by the most: where their side out-bid ours.  Sort by
    // IMP swing ascending (most negative first), break ties by points.  One
    // renderer, used for the global ranking and the we-defend-1NT subset (the
    // latter cuts past the `--no-our-1nt` artifact boards so the defense auctions
    // are readable).
    let dump = |title: &str, rows: &[(usize, i64, i64)]| {
        println!("\n=== {title} ===");
        for &(index, points, imp) in rows {
            let board = &boards[index];
            let (contract_a, contract_b) = contracts[index];
            println!(
                "\n[board {index}] dealer {:?}, swing {points:+} pts / {imp:+} IMPs",
                board.dealer
            );
            println!("  {}", board.deal.display(Seat::North));
            println!(
                "  ours NS @ A: {}  -> {}",
                show_auction(&board.table_a),
                contract_a.map_or("passed out".into(), |(c, s)| format!("{c} by {s:?}")),
            );
            println!(
                "  ours EW @ B: {}  -> {}",
                show_auction(&board.table_b),
                contract_b.map_or("passed out".into(), |(c, s)| format!("{c} by {s:?}")),
            );
        }
    };
    swings.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.1.cmp(&b.1)));
    let worst: Vec<_> = swings.iter().take(args.top).copied().collect();
    dump(
        &format!("Worst {} divergent boards for us (their edge)", worst.len()),
        &worst,
    );
    // Same ranking, restricted to boards where BBA opened 1NT and we defended.
    let worst_defend: Vec<_> = swings
        .iter()
        .filter(|&&(index, ..)| {
            matches!(
                opening_1nt(&boards[index].table_a, boards[index].dealer),
                Some((_, false))
            )
        })
        .take(args.top)
        .copied()
        .collect();
    dump(
        &format!(
            "Worst {} we-defend-1NT boards (BBA opens 1NT, we defend)",
            worst_defend.len()
        ),
        &worst_defend,
    );
    Ok(())
}
