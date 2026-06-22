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
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Level, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use libloading::Library;
use pons::american;
use pons::bidding::array::{Array, Logits};
use pons::bidding::context::relative;
use pons::bidding::{Family, System};
use pons::scoring::{final_contract, imps, ns_score_contract};
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

    /// Force a named BBA convention on/off on *our* side (repeatable), e.g.
    /// `--our-conv "Rubensohl after 1m=1"`.  Only meaningful with `--our-system`.
    #[arg(long = "our-conv", value_parser = parse_override, value_name = "NAME=0|1")]
    our_conv: Vec<(CString, c_int)>,

    /// Force a named BBA convention on/off on *their* side (repeatable), e.g.
    /// `--their-conv "Rubensohl after 1m=0"`.  Pair with `--our-conv` to isolate
    /// one toggle in a BBA-vs-BBA A/B.
    #[arg(long = "their-conv", value_parser = parse_override, value_name = "NAME=0|1")]
    their_conv: Vec<(CString, c_int)>,
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
    // Our side: the authored floor by default, or a second EPBot card when
    // `--our-system` is given (the BBA-vs-BBA experiment).  Both live to the end
    // of `main`, so `ours` can borrow whichever is selected.
    let our_floor = american().against(Family::NATURAL);
    let our_oracle = match args.our_system {
        Some(system) => Some(BbaOracle::load(&path, system, args.our_conv.clone())?),
        None => None,
    };
    let ours: &dyn System = match &our_oracle {
        Some(oracle) => oracle,
        None => &our_floor,
    };
    let our_label = match args.our_system {
        Some(system) => format!(
            "BBA {}{}",
            system_label(system),
            label_overrides(&args.our_conv)
        ),
        None => "our american floor".into(),
    };
    let their_label = format!(
        "BBA {}{}",
        system_label(args.system),
        label_overrides(&args.their_conv)
    );
    let mut rng = rand::rng();

    // Bid every board at both tables, dealer rotating per board.
    let boards: Vec<Board> = (0..args.count)
        .map(|index| {
            let dealer = Seat::ALL[index % 4];
            let deal = full_deal(&mut rng);
            let table_a = bid_out(ours, &bba, true, dealer, args.vulnerability, &deal);
            let table_b = bid_out(ours, &bba, false, dealer, args.vulnerability, &deal);
            Board {
                deal,
                dealer,
                table_a,
                table_b,
            }
        })
        .collect();

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

    // The boards we lost by the most: where their side out-bid ours.  Sort by
    // IMP swing ascending (most negative first), break ties by points.
    swings.sort_by(|a, b| a.2.cmp(&b.2).then_with(|| a.1.cmp(&b.1)));
    println!(
        "\n=== Worst {} divergent boards for us (their edge) ===",
        args.top.min(swings.len()),
    );
    for &(index, points, imp) in swings.iter().take(args.top) {
        let board = &boards[index];
        let (contract_a, contract_b) = contracts[index];
        println!(
            "\n[board {index}] dealer {:?}, swing {points:+} pts / {imp:+} IMPs",
            board.dealer,
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
    Ok(())
}
