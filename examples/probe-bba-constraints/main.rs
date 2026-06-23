//! Distill BBA/EPBot's Multi-Landy defense to a 1NT opening into the constraint
//! DSL by **sample-and-probe**: deal random actor hands, drive the real EPBot
//! engine for a fixed `(seat, auction)`, bucket each hand by the call it returns,
//! and summarise every bucket in DSL vocabulary (HCP / suit length / balanced).
//!
//! BBA's compiled 2/1 card answers a 1NT opening with **Multi-Landy**, whose
//! `2♦` is the *Multi*: an unknown single-suited major.  Three `--mode`s read
//! the three seats of that structure.  The `.so` ignores `vendor/bba/*.bbsa`, so
//! this real-hand probe is the only reliable read (see `probe-bba-1nt`); we force
//! `Multi-Landy=1` (and `Cappelletti=0`) on all seats so BBA both *bids* and
//! *interprets* the 2♦ as a Multi:
//!
//! ```text
//! cargo run --release --example probe-bba-constraints -- --mode multi    # the 2♦ overcaller
//! cargo run --release --example probe-bba-constraints -- --mode advance  # the advancer relay
//! cargo run --release --example probe-bba-constraints -- --mode counter --vul none,both  # our-side counter-defense
//! ```
//!
//! Each `sketch:` line is a *candidate* constraint to verify and hand-author,
//! not a proof of BBA's internal logic.

#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use anyhow::{Result, bail};
use clap::Parser;
use contract_bridge::deck::fill_deals;
use contract_bridge::eval::{self, HandEvaluator, SimpleEvaluator};
use contract_bridge::{Builder, Hand, Seat, Suit};
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
const TWO_D: c_int = 11; // 5 + 1*5 + 1

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

    // (actor seat, replayed prefix, heading) — dealer is canonicalized to seat 0.
    let (actor, prefix, what): (c_int, &[c_int], &str) = match args.mode.as_str() {
        "multi" => (
            1,
            &[ONE_NT],
            "BBA's direct call over (1NT) — the 2♦ bucket is the Multi",
        ),
        "advance" => (
            3,
            &[ONE_NT, TWO_D, PASS],
            "BBA advancer over 1NT-(2♦)-P — the pass-or-correct relay",
        ),
        "counter" => (
            2,
            &[ONE_NT, TWO_D],
            "BBA responder's counter-defense over 1NT-(2♦)",
        ),
        other => bail!("--mode must be multi|advance|counter, got {other:?}"),
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
        let buckets = run(&bba, actor, prefix, vul, args.samples, args.seed);
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
fn run(
    bba: &Bba,
    actor: c_int,
    prefix: &[c_int],
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
