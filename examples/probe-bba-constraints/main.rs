//! Distill BBA/EPBot's Multi-Landy defense to a 1NT opening into the constraint
//! DSL by **sample-and-probe**: deal random actor hands, drive the real EPBot
//! engine for a fixed `(seat, auction)`, bucket each hand by the call it returns,
//! and summarise every bucket in DSL vocabulary (HCP / suit length / balanced).
//!
//! BBA's compiled 2/1 card answers a 1NT opening with **Multi-Landy**, whose
//! `2♦` is the *Multi* (an unknown single-suited major) and whose `2♥`/`2♠` are
//! *Muiderberg* (5+ major, 4+ minor).  Several `--mode`s read the seats of that
//! structure.  The `.so` ignores `vendor/bba/*.bbsa`, so this real-hand probe is
//! the only reliable read (see `probe-bba-1nt`); we force `Multi-Landy=1` (and
//! `Cappelletti=0`) on all seats so BBA both *bids* and *interprets* the calls:
//!
//! ```text
//! cargo run --release --example probe-bba-constraints -- --mode multi    # direct call over (1NT): X/2♣/2♦/2♥/2♠
//! cargo run --release --example probe-bba-constraints -- --mode advance  # advancer over the 2♦ Multi
//! cargo run --release --example probe-bba-constraints -- --mode muider-h # advancer over the 2♥ Muiderberg
//! cargo run --release --example probe-bba-constraints -- --mode muider-s # advancer over the 2♠ Muiderberg
//! cargo run --release --example probe-bba-constraints -- --mode rebid-d  # 2♦-overcaller's rebid (Multi → which major)
//! cargo run --release --example probe-bba-constraints -- --mode rebid-h  # 2♥-overcaller's rebid after the 2NT ask
//! cargo run --release --example probe-bba-constraints -- --mode rebid-s  # 2♠-overcaller's rebid after the 2NT ask
//! cargo run --release --example probe-bba-constraints -- --mode counter --vul none,both  # our-side counter-defense
//! cargo run --release --example probe-bba-constraints -- --mode weak2-h  # opener's rebid over 2♥-P-2NT-P
//! cargo run --release --example probe-bba-constraints -- --mode weak2-h --conv Ogust=1  # ...with BBA's Ogust on
//! ```
//!
//! The `weak2-*` modes read a node we author as **Ogust** and BBA does not: its
//! 2/1 default is `Ogust = 0` (confirmed against the live engine, since the `.so`
//! ignores the cards).  `libEPBot.so` does carry a full Ogust implementation
//! (`odzywka_OGUST`, and an `_interpretacja` for both sides), so `--conv Ogust=1`
//! reads the same node with it enabled — the two runs together decide whether to
//! match BBA's ladder or to teach BBA ours.
//!
//! Each `sketch:` line is a *candidate* constraint to verify and hand-author,
//! not a proof of BBA's internal logic.

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use anyhow::{Result, bail};
use clap::Parser;
use contract_bridge::deck::fill_deals;
use contract_bridge::eval::{self, HandEvaluator, SimpleEvaluator};
use contract_bridge::{Builder, Hand, Rank, Seat, Suit};
use libloading::Library;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::BTreeMap;
use std::ffi::{CString, c_char, c_int, c_void};
use std::fmt::Write as _;

const DEFAULT_LIB: &str = "vendor/bba/Native-libraries/linux/x64/libEPBot.so";

// EPBot bid codes: Pass = 0, X = 1; a bid is 5 + (level-1)*5 + strain (♣..NT).
const PASS: c_int = 0;
const ONE_NT: c_int = 9; // 5 + 0*5 + 4
const TWO_C: c_int = 10; // 5 + 1*5 + 0
const TWO_D: c_int = 11; // 5 + 1*5 + 1
const TWO_H: c_int = 12; // 5 + 1*5 + 2
const TWO_S: c_int = 13; // 5 + 1*5 + 3
const TWO_NT: c_int = 14; // 5 + 1*5 + 4

type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
type SetBidFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type SetConvFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int) -> c_int;

/// Cached EPBot entry points (copied out of the [`Library`], which we keep alive)
struct Bba {
    _lib: Library,
    create: CreateFn,
    destroy: DestroyFn,
    set_system: SetSystemFn,
    new_hand: NewHandFn,
    set_bid: SetBidFn,
    get_bid: GetBidFn,
    set_conv: SetConvFn,
    /// Named conventions forced on all four seats after `set_system`.
    overrides: Vec<(CString, c_int)>,
}

impl Bba {
    fn load(path: &str, overrides: Vec<(CString, c_int)>) -> Result<Self> {
        // SAFETY: loading a trusted native library; symbol signatures match the
        // ABI confirmed by `bba-match`/`probe-bba-1nt`.
        let lib = unsafe { Library::new(path) }?;
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
                overrides,
            })
        }
    }

    /// The call `actor` makes (system 0, 2/1 GF) holding `hand` after `prefix` is
    /// replayed from a dealer canonicalized to seat 0.  A fresh bot per call keeps
    /// this a pure function of its arguments.
    fn call(&self, actor: c_int, prefix: &[c_int], hand: Hand, vul: c_int) -> c_int {
        let suits = hand_to_suits(hand);
        let empty = c"".as_ptr();
        // SAFETY: a fresh bot used and destroyed here; all argument types match
        // the confirmed ABI; `suits` outlives the `new_hand` call.
        unsafe {
            let bot = (self.create)();
            assert!(!bot.is_null(), "epbot_create returned null");
            for seat in 0..4 {
                (self.set_system)(bot, seat, 0);
            }
            for (name, value) in &self.overrides {
                for seat in 0..4 {
                    (self.set_conv)(bot, seat, name.as_ptr(), *value);
                }
            }
            (self.new_hand)(bot, actor, suits.as_ptr(), 0, vul, 0, 0);
            for (index, &code) in prefix.iter().enumerate() {
                (self.set_bid)(bot, (index % 4) as c_int, code, empty);
            }
            let code = (self.get_bid)(bot);
            (self.destroy)(bot);
            code
        }
    }
}

/// The four holdings in EPBot's C,D,H,S order, newline-joined (13 cards).  See
/// `bba-match::hand_to_suits` — `Holding`'s `Display` is EPBot's canonical form.
fn hand_to_suits(hand: Hand) -> CString {
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

/// Decode an EPBot bid code into a label, or `None` for an error/illegal code
fn decode(code: c_int) -> Option<String> {
    const STRAIN: [&str; 5] = ["♣", "♦", "♥", "♠", "NT"];
    match code {
        0 => Some("Pass".into()),
        1 => Some("X".into()),
        2 => Some("XX".into()),
        5..=39 => {
            let i = code - 5;
            Some(format!("{}{}", i / 5 + 1, STRAIN[(i % 5) as usize]))
        }
        _ => None,
    }
}

/// The EPBot vulnerability code (bit 1 = N/S, bit 2 = E/W) for `we`/`they`
/// relative to `actor`; even seats are N/S.  Mirrors `bba-match::epbot_vulnerability`.
fn vul_code(token: &str, actor: c_int) -> Result<c_int> {
    let (we, they) = match token {
        "none" => (false, false),
        "we" => (true, false),
        "they" => (false, true),
        "both" => (true, true),
        other => bail!("--vul must be none|we|they|both, got {other:?}"),
    };
    let (ns, ew) = if actor % 2 == 0 {
        (we, they)
    } else {
        (they, we)
    };
    Ok(c_int::from(ns) | (c_int::from(ew) << 1))
}

/// Per-call accumulator: every probe hand BBA mapped to this call.
#[derive(Default)]
struct Bucket {
    hcp: Vec<u8>,
    /// Suit lengths in [`Suit::ASC`] order (♣ ♦ ♥ ♠).
    len: [Vec<u8>; 4],
    balanced: usize,
    /// A/K/Q count in the mode's trump suit; empty unless the mode names one.
    /// An Ogust-style ladder splits on suit *quality*, which neither the HCP
    /// nor the length columns can show.
    tops: Vec<u8>,
    /// Rival definitions of the same "good suit", to find which one BBA uses:
    /// A/K/Q/J count, A/K/Q/J/10 count, and plain HCP inside the trump suit.
    tops4: Vec<u8>,
    tops5: Vec<u8>,
    trump_hcp: Vec<u8>,
}

#[derive(Parser)]
#[command(about = "Distill BBA's Multi-Landy 2♦ defense (and its counter) into the DSL")]
struct Args {
    /// Which seat to probe: multi (2♦ overcaller) | advance (advancer relay) | counter (our-side defense)
    #[arg(long, default_value = "multi")]
    mode: String,

    /// Comma-separated vulnerabilities to report: none,we,they,both
    #[arg(long, default_value = "none")]
    vul: String,

    /// Probe hands per vulnerability
    #[arg(long, default_value_t = 5000)]
    samples: usize,

    /// Two-sided percentile trim for the reported HCP range (0.05 = 5th–95th)
    #[arg(long, default_value_t = 0.05)]
    trim: f64,

    /// Skip calls chosen on fewer than this fraction of probe hands
    #[arg(long, default_value_t = 0.01)]
    min_share: f64,

    /// RNG seed (fixed by default; the same hands are reused across vulnerabilities)
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Force a named convention: NAME=0|1 (repeatable). Default: Multi-Landy=1, Cappelletti=0
    #[arg(long = "conv")]
    conv: Vec<String>,

    /// Optional output file (default: stdout)
    #[arg(long)]
    out: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if !(0.0..0.5).contains(&args.trim) {
        bail!("--trim must be in [0.0, 0.5)");
    }

    // (actor seat, replayed prefix, self-consistency filter, heading) — dealer is
    // canonicalized to seat 0.  `filter` is the direct-seat call a probe hand must
    // make over (1NT) to be kept; `None` accepts every hand (the advancer's hand is
    // unconstrained, so its modes need no filter).  The `rebid-*` modes read the
    // overcaller's own seat, so they keep only hands BBA would actually overcall.
    let (actor, prefix, filter, what): (c_int, &[c_int], Option<c_int>, &str) = match args
        .mode
        .as_str()
    {
        "multi" => (
            1,
            &[ONE_NT],
            None,
            "BBA's direct call over (1NT) — the 2♦ bucket is the Multi",
        ),
        "advance" => (
            3,
            &[ONE_NT, TWO_D, PASS],
            None,
            "BBA advancer over 1NT-(2♦)-P — the pass-or-correct relay",
        ),
        "counter" => (
            2,
            &[ONE_NT, TWO_D],
            None,
            "BBA responder's counter-defense over 1NT-(2♦)",
        ),
        "muider-h" => (
            3,
            &[ONE_NT, TWO_H, PASS],
            None,
            "BBA advancer over 1NT-(2♥)-P — the Muiderberg advance (2NT/3♣/3♦ asks)",
        ),
        "muider-s" => (
            3,
            &[ONE_NT, TWO_S, PASS],
            None,
            "BBA advancer over 1NT-(2♠)-P — the Muiderberg advance (2NT/3♣/3♦ asks)",
        ),
        "rebid-d" => (
            1,
            &[ONE_NT, TWO_D, PASS, TWO_H, PASS],
            Some(TWO_D),
            "BBA 2♦-overcaller's rebid over 1NT-(2♦)-P-2♥-P — Pass=hearts, 2♠=spades",
        ),
        "rebid-d2s" => (
            1,
            &[ONE_NT, TWO_D, PASS, TWO_S, PASS],
            Some(TWO_D),
            "BBA 2♦-overcaller's rebid over 1NT-(2♦)-P-2♠-P — what the 2♠ advance forces",
        ),
        "rebid-h" => (
            1,
            &[ONE_NT, TWO_H, PASS, TWO_NT, PASS],
            Some(TWO_H),
            "BBA 2♥-overcaller's rebid over 1NT-(2♥)-P-2NT-P — what the 2NT ask wants",
        ),
        "rebid-s" => (
            1,
            &[ONE_NT, TWO_S, PASS, TWO_NT, PASS],
            Some(TWO_S),
            "BBA 2♠-overcaller's rebid over 1NT-(2♠)-P-2NT-P — what the 2NT ask wants",
        ),
        // Defense to the opponents' 1NT *response* (Stayman / Jacoby transfers),
        // 4th seat at [1NT, P, 2x].  No filter — the 4th-seat hand is unconstrained.
        "stayman" => (
            3,
            &[ONE_NT, PASS, TWO_C],
            None,
            "BBA 4th-seat over 1NT-P-(2♣ Stayman) — X=clubs, natural, no 2NT",
        ),
        "xfer-h" => (
            3,
            &[ONE_NT, PASS, TWO_D],
            None,
            "BBA 4th-seat over 1NT-P-(2♦ →♥) — X=diamonds, 2♥ cue=spades+minor",
        ),
        "xfer-s" => (
            3,
            &[ONE_NT, PASS, TWO_H],
            None,
            "BBA 4th-seat over 1NT-P-(2♥ →♠) — X=hearts, 2♠ cue=hearts+minor",
        ),
        // Opener's rebid after our own weak two is asked with 2NT.  We author
        // Ogust here; BBA's 2/1 default has `Ogust = 0` (verified against the live
        // engine by `bba-conv-probe`, not the card — the `.so` ignores the file),
        // so these read what its ladder means instead.  Pair with `--conv Ogust=1`
        // to read the same node with BBA's own Ogust switched on.
        "weak2-d" => (
            0,
            &[TWO_D, PASS, TWO_NT, PASS],
            Some(TWO_D),
            "BBA opener's rebid over 2♦-P-2NT-P — what the 2NT ask wants",
        ),
        "weak2-h" => (
            0,
            &[TWO_H, PASS, TWO_NT, PASS],
            Some(TWO_H),
            "BBA opener's rebid over 2♥-P-2NT-P — what the 2NT ask wants",
        ),
        "weak2-s" => (
            0,
            &[TWO_S, PASS, TWO_NT, PASS],
            Some(TWO_S),
            "BBA opener's rebid over 2♠-P-2NT-P — what the 2NT ask wants",
        ),
        other => bail!(
            "--mode must be multi|advance|counter|muider-h|muider-s|rebid-d|rebid-h|rebid-s|stayman|xfer-h|xfer-s|weak2-d|weak2-h|weak2-s, got {other:?}"
        ),
    };

    // The weak-two modes probe the OPENER, so their self-consistency filter replays
    // an empty prefix (the opening itself), not (1NT); `trump` names the suit whose
    // top-honor count to report, since an Ogust ladder splits on suit quality.
    let (filter_prefix, trump): (&[c_int], Option<Suit>) = match args.mode.as_str() {
        "weak2-d" => (&[], Some(Suit::Diamonds)),
        "weak2-h" => (&[], Some(Suit::Hearts)),
        "weak2-s" => (&[], Some(Suit::Spades)),
        _ => (&[ONE_NT], None),
    };

    let overrides = parse_conv(&args.conv)?;
    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let bba = Bba::load(&path, overrides)?;

    let mut report = String::new();
    let _ = writeln!(
        report,
        "# BBA Multi-Landy probe — mode `{}`\n\n{what}\nsystem: 0 (2/1 GF)  conventions: {}\nsamples per vul: {}\n",
        args.mode,
        conv_summary(&bba.overrides),
        args.samples,
    );
    let _ = writeln!(
        report,
        "Hands summarised in DSL vocabulary; `sketch` is a *candidate* to verify, not a proof.\n"
    );

    for token in args.vul.split(',').map(str::trim) {
        let vul = vul_code(token, actor)?;
        let buckets = run(
            &bba,
            actor,
            prefix,
            filter,
            filter_prefix,
            trump,
            vul,
            args.samples,
            args.seed,
        );
        render_vul(&mut report, token, &buckets, &args);
    }

    if let Some(out) = &args.out {
        std::fs::write(out, &report)?;
        eprintln!("wrote {out}");
    } else {
        print!("{report}");
    }
    Ok(())
}

/// Deal `samples` random hands, drive BBA, and bucket by its returned call.
// ponytail: a probe harness — a params struct would be more ceremony than the
// one call site is worth.  Bundle them if a third caller ever appears.
#[allow(clippy::too_many_arguments)]
fn run(
    bba: &Bba,
    actor: c_int,
    prefix: &[c_int],
    filter: Option<c_int>,
    filter_prefix: &[c_int],
    trump: Option<Suit>,
    vul: c_int,
    samples: usize,
    seed: u64,
) -> BTreeMap<c_int, Bucket> {
    let mut rng = StdRng::seed_from_u64(seed);
    let empty = Builder::new()
        .build_partial()
        .expect("an empty builder is a valid (all-unknown) partial deal");

    let mut buckets: BTreeMap<c_int, Bucket> = BTreeMap::new();
    for deal in fill_deals(&mut rng, empty).take(samples) {
        let hand = deal[Seat::North];
        // ponytail: rebid modes read the bidder's OWN seat, so a uniform random hand
        // is wrong — most never make the call.  Keep only hands whose call over
        // `filter_prefix` is the studied one; otherwise the rebid is from a hand that
        // never bid it.  (`filter_prefix` is (1NT) for the overcall modes and empty
        // for the weak-two modes, which probe the opener.)  Rejection is exact here.
        if let Some(want) = filter
            && bba.call(actor, filter_prefix, hand, vul) != want
        {
            continue;
        }
        let code = bba.call(actor, prefix, hand, vul);
        if decode(code).is_none() {
            continue; // EPBot error/illegal code — drop it
        }
        let entry = buckets.entry(code).or_default();
        entry.hcp.push(hcp(hand));
        let lengths = Suit::ASC.map(|suit| hand[suit].len() as u8);
        for (slot, &l) in entry.len.iter_mut().zip(&lengths) {
            slot.push(l);
        }
        if is_balanced(lengths) {
            entry.balanced += 1;
        }
        if let Some(suit) = trump {
            let holding = hand[suit];
            let count =
                |ranks: &[Rank]| ranks.iter().filter(|&&r| holding.contains(r)).count() as u8;
            entry.tops.push(count(&[Rank::A, Rank::K, Rank::Q]));
            entry
                .tops4
                .push(count(&[Rank::A, Rank::K, Rank::Q, Rank::J]));
            entry
                .tops5
                .push(count(&[Rank::A, Rank::K, Rank::Q, Rank::J, Rank::T]));
            let thcp = [(Rank::A, 4), (Rank::K, 3), (Rank::Q, 2), (Rank::J, 1)]
                .into_iter()
                .filter(|&(r, _)| holding.contains(r))
                .map(|(_, v)| v)
                .sum();
            entry.trump_hcp.push(thcp);
        }
    }
    buckets
}

/// One report section per vulnerability: the per-call buckets in DSL vocabulary.
fn render_vul(report: &mut String, vul: &str, buckets: &BTreeMap<c_int, Bucket>, args: &Args) {
    let probed: usize = buckets.values().map(|b| b.hcp.len()).sum();
    let _ = writeln!(report, "## vul: {vul}   (n={probed})\n");
    if probed == 0 {
        let _ = writeln!(report, "_no probe hands produced a legal call_\n");
        return;
    }

    let mut by_share: Vec<(&c_int, &Bucket)> = buckets.iter().collect();
    by_share.sort_by_key(|(_, b)| std::cmp::Reverse(b.hcp.len()));

    for (code, bucket) in by_share {
        let n = bucket.hcp.len();
        let share = n as f64 / probed as f64;
        if share < args.min_share {
            continue;
        }
        let call = decode(*code).expect("error codes were dropped in `run`");

        let mut hcp = bucket.hcp.clone();
        hcp.sort_unstable();
        let hcp_lo = pct(&hcp, args.trim);
        let hcp_hi = pct(&hcp, 1.0 - args.trim);

        let _ = writeln!(report, "### {call}   (chosen {:.1}%, n={n})", 100.0 * share);
        let _ = writeln!(
            report,
            "- hcp: {hcp_lo}–{hcp_hi} (median {})",
            pct(&hcp, 0.5)
        );
        if !bucket.tops.is_empty() {
            let good = bucket.tops.iter().filter(|&&t| t >= 2).count();
            // Sum in u32: 3 honors × a few hundred hands overflows u8, and release
            // builds wrap silently (a 2.02 mean printed as 0.01).
            let total: u32 = bucket.tops.iter().copied().map(u32::from).sum();
            let mean = f64::from(total) / bucket.tops.len() as f64;
            let _ = writeln!(
                report,
                "- trump A/K/Q: mean {mean:.2}, two-plus {:.0}% (Ogust \"good suit\")",
                100.0 * good as f64 / bucket.tops.len() as f64
            );
            // Histograms of rival "good suit" predicates.  A predicate BBA actually
            // uses separates the good/bad rungs with NO overlap, so read these as
            // "which row has an empty cell exactly where the other rung is full".
            for (label, col) in [
                ("A/K/Q  ", &bucket.tops),
                ("A/K/Q/J", &bucket.tops4),
                ("+ten   ", &bucket.tops5),
                ("trumpHC", &bucket.trump_hcp),
            ] {
                let hi = col.iter().copied().max().unwrap_or(0);
                let cells: Vec<String> = (0..=hi)
                    .map(|v| format!("{v}:{}", col.iter().filter(|&&x| x == v).count()))
                    .collect();
                let _ = writeln!(report, "  - {label} {}", cells.join(" "));
            }
        }

        let mut clauses = vec![format!("hcp({hcp_lo}..={hcp_hi})")];
        for (i, suit) in Suit::ASC.into_iter().enumerate() {
            let mut col = bucket.len[i].clone();
            col.sort_unstable();
            let (lo, mid, hi) = (pct(&col, 0.10), pct(&col, 0.5), pct(&col, 0.90));
            let _ = writeln!(report, "- {suit:?}: {lo}–{hi} (median {mid})");
            if lo >= 4 {
                clauses.push(format!("len({suit:?}, {lo}..)"));
            }
        }

        // Per-hand longest major / minor (columns are pushed in lockstep, so index
        // `k` is the same hand across all four).  These read the "6+ major" (Multi)
        // and "5-4 majors" answers directly, where the per-suit columns smear.
        let mut major: Vec<u8> = (0..n)
            .map(|k| bucket.len[2][k].max(bucket.len[3][k]))
            .collect();
        let mut minor: Vec<u8> = (0..n)
            .map(|k| bucket.len[0][k].max(bucket.len[1][k]))
            .collect();
        major.sort_unstable();
        minor.sort_unstable();
        let _ = writeln!(
            report,
            "- longest major: {}–{} (median {})",
            pct(&major, 0.10),
            pct(&major, 0.90),
            pct(&major, 0.5)
        );
        let _ = writeln!(
            report,
            "- longest minor: {}–{} (median {})",
            pct(&minor, 0.10),
            pct(&minor, 0.90),
            pct(&minor, 0.5)
        );

        let bal = bucket.balanced as f64 / n as f64;
        let _ = writeln!(report, "- balanced: {:.0}%", 100.0 * bal);
        if bal > 0.8 {
            clauses.push("balanced()".to_string());
        }
        let _ = writeln!(report, "- sketch: {}\n", clauses.join(" & "));
    }

    // Cheap invariant: HCP can never escape its range if the eval wiring is sound.
    assert!(
        buckets.values().flat_map(|b| &b.hcp).all(|&h| h <= 37),
        "HCP out of range — eval wiring is wrong"
    );
}

/// HCP via the simple evaluator (matches the DSL's `hcp(..)`).
fn hcp(hand: Hand) -> u8 {
    SimpleEvaluator(eval::hcp::<u8>).eval(hand)
}

/// Balanced: every suit ≥ 2 and at most one doubleton.
fn is_balanced(len: [u8; 4]) -> bool {
    len.iter().all(|&l| l >= 2) && len.iter().filter(|&&l| l == 2).count() <= 1
}

/// Value at quantile `q` of a pre-sorted slice (nearest-rank).
fn pct(sorted: &[u8], q: f64) -> u8 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Parse `--conv NAME=0|1` flags; default forces Multi-Landy on, Cappelletti off.
fn parse_conv(flags: &[String]) -> Result<Vec<(CString, c_int)>> {
    if flags.is_empty() {
        return Ok(vec![
            (CString::new("Multi-Landy").unwrap(), 1),
            (CString::new("Cappelletti").unwrap(), 0),
        ]);
    }
    flags
        .iter()
        .map(|flag| {
            let (name, value) = flag
                .split_once('=')
                .ok_or_else(|| anyhow::anyhow!("--conv must be NAME=0|1, got {flag:?}"))?;
            let on: c_int = value.trim().parse()?;
            Ok((CString::new(name.trim())?, on))
        })
        .collect()
}

fn conv_summary(overrides: &[(CString, c_int)]) -> String {
    overrides
        .iter()
        .map(|(name, value)| format!("{}={value}", name.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(", ")
}
