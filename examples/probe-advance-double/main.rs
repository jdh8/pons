//! Throwaway probe: distill BBA/EPBot's 2/1-GF **advance of a takeout double**
//! into the constraint DSL by sample-and-probe.  Opener (seat 0) opens
//! `1♣/1♦/1♥/1♠`, our partner (seat 1) doubles for takeout, opener's partner
//! (seat 2) passes, and we (seat 3, the *advancer*) act.  Deal random advancer
//! hands, drive the real EPBot engine for the fixed `(seat, auction)`, bucket
//! each hand by the call it returns, and summarise every bucket.
//!
//! Configured **identically** to how `bba-gen` configures BBA (`--their-conv`)
//! in the anchor: system index 0 ("2/1GF"), no convention overrides — the very
//! engine the anchor measured against.
//!
//! ```text
//! cargo run --release --example probe-advance-double
//! cargo run --release --example probe-advance-double -- --samples 40000 --out /tmp/report.md
//! ```
//!
//! Each `sketch:` line is a *candidate* constraint to verify and hand-author,
//! not a proof of BBA's internal logic.  A one-off analysis tool — no `src/`.

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

/// System index 0 = "2/1GF - 2/1 Game Force" — matches `bba-gen`'s default.
const SYSTEM_2_OVER_1: c_int = 0;

// EPBot bid codes: Pass = 0, X = 1, XX = 2; a bid is 5 + (level-1)*5 + strain,
// strain order ♣ ♦ ♥ ♠ NT (matches `Suit::ASC` for the first four).
const ONE_C: c_int = 5;
const ONE_D: c_int = 6;
const ONE_H: c_int = 7;
const ONE_S: c_int = 8;
const X: c_int = 1;
const PASS: c_int = 0;

type CreateFn = unsafe extern "C" fn() -> *mut c_void;
type DestroyFn = unsafe extern "C" fn(*mut c_void);
type SetSystemFn = unsafe extern "C" fn(*mut c_void, c_int, c_int);
type NewHandFn =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_char, c_int, c_int, c_int, c_int);
type SetBidFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, *const c_char);
type GetBidFn = unsafe extern "C" fn(*mut c_void) -> c_int;

/// Cached EPBot entry points (copied out of the [`Library`], which we keep alive)
struct Bba {
    _lib: Library,
    create: CreateFn,
    destroy: DestroyFn,
    set_system: SetSystemFn,
    new_hand: NewHandFn,
    set_bid: SetBidFn,
    get_bid: GetBidFn,
}

impl Bba {
    fn load(path: &str) -> Result<Self> {
        // SAFETY: loading a trusted native library; symbol signatures match the
        // ABI confirmed by `bba-gen`.
        let lib = unsafe { Library::new(path) }?;
        unsafe {
            Ok(Self {
                create: *lib.get::<CreateFn>(b"epbot_create\0")?,
                destroy: *lib.get::<DestroyFn>(b"epbot_destroy\0")?,
                set_system: *lib.get::<SetSystemFn>(b"epbot_set_system_type\0")?,
                new_hand: *lib.get::<NewHandFn>(b"epbot_new_hand\0")?,
                set_bid: *lib.get::<SetBidFn>(b"epbot_set_bid\0")?,
                get_bid: *lib.get::<GetBidFn>(b"epbot_get_bid\0")?,
                _lib: lib,
            })
        }
    }

    /// The call `actor` makes (system 0, 2/1 GF) holding `hand` after `prefix`
    /// is replayed from a dealer canonicalized to seat 0.  A fresh bot per call
    /// keeps this a pure function of its args.
    fn call(&self, actor: c_int, prefix: &[c_int], hand: Hand, vul: c_int) -> c_int {
        let suits = hand_to_suits(hand);
        let empty = c"".as_ptr();
        // SAFETY: a fresh bot used and destroyed here; all argument types match
        // the confirmed ABI; `suits` outlives the `new_hand` call.
        unsafe {
            let bot = (self.create)();
            assert!(!bot.is_null(), "epbot_create returned null");
            for seat in 0..4 {
                (self.set_system)(bot, seat, SYSTEM_2_OVER_1);
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

/// The four holdings in EPBot's C,D,H,S order, newline-joined (13 cards).
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

/// If `code` is a suit bid (not NT/Pass/X/XX), its [`Suit::ASC`] index.
fn call_suit_index(code: c_int) -> Option<usize> {
    if (5..=39).contains(&code) {
        let strain = ((code - 5) % 5) as usize;
        (strain < 4).then_some(strain)
    } else {
        None
    }
}

/// The EPBot vulnerability code (bit 1 = N/S, bit 2 = E/W); `none`/`both` are
/// symmetric so the actor's seat parity doesn't matter for the two tokens we use.
fn vul_code(token: &str) -> Result<c_int> {
    match token {
        "none" => Ok(0),
        "both" => Ok(3),
        other => bail!("vul must be none|both, got {other:?}"),
    }
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
#[command(about = "Distill BBA's advance of a takeout double ([1t, X, P]) into the DSL")]
struct Args {
    /// Probe hands per (opening, vulnerability) combination
    #[arg(long, default_value_t = 40_000)]
    samples: usize,

    /// Skip calls chosen on fewer than this fraction of probe hands
    #[arg(long, default_value_t = 0.003)]
    min_share: f64,

    /// RNG seed (fixed by default; the same hands are reused across every combo)
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Optional output file (default: stdout)
    #[arg(long)]
    out: Option<String>,
}

/// One opening to probe: label, bid code, its [`Suit::ASC`] index.
const OPENINGS: [(&str, c_int, usize); 4] = [
    ("1♣", ONE_C, 0),
    ("1♦", ONE_D, 1),
    ("1♥", ONE_H, 2),
    ("1♠", ONE_S, 3),
];
const VULS: [&str; 2] = ["none", "both"];

fn main() -> Result<()> {
    let args = Args::parse();

    let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
    let bba = match Bba::load(&path) {
        Ok(bba) => bba,
        Err(error) => {
            eprintln!(
                "could not load EPBot native lib at `{path}`: {error}\n\
                 Fetch it with `git submodule update --init vendor/bba`, or set BBA_LIB."
            );
            std::process::exit(1);
        }
    };

    let mut report = String::new();
    let _ = writeln!(
        report,
        "# BBA advance-of-takeout-double probe — [1t, X, P], advancer (seat 3)\n\n\
         system: 0 (2/1 GF), no convention overrides (matches the anchor's BBA config)\n\
         samples per (opening, vul): {}\n",
        args.samples
    );

    // actor = seat 3: opener (0) opens, partner (1) doubles, seat 2 passes, we act.
    const ACTOR: c_int = 3;
    for (opening_name, opening_code, opener_suit) in OPENINGS {
        for vul_token in VULS {
            let vul = vul_code(vul_token)?;
            let prefix = [opening_code, X, PASS];
            let buckets = run(&bba, ACTOR, &prefix, vul, args.samples, args.seed);
            render(
                &mut report,
                opening_name,
                opener_suit,
                vul_token,
                &buckets,
                &args,
            );
        }
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

/// One report section per (opening, vulnerability): the per-call buckets.
fn render(
    report: &mut String,
    opening: &str,
    opener_suit: usize,
    vul: &str,
    buckets: &BTreeMap<c_int, Bucket>,
    args: &Args,
) {
    let probed: usize = buckets.values().map(|b| b.hcp.len()).sum();
    let _ = writeln!(report, "## {opening} opening, vul: {vul}   (n={probed})\n");
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
        let tag = match call_suit_index(*code) {
            Some(idx) if idx == opener_suit => " (CUEBID of opener's suit)",
            Some(_) => " (natural suit call)",
            None => "",
        };

        let mut hcp = bucket.hcp.clone();
        hcp.sort_unstable();
        let (hcp_min, hcp_max) = (pct(&hcp, 0.0), pct(&hcp, 1.0));
        let (hcp_10, hcp_50, hcp_90) = (pct(&hcp, 0.10), pct(&hcp, 0.5), pct(&hcp, 0.90));

        let _ = writeln!(
            report,
            "### {call}{tag}   (chosen {:.1}%, n={n})",
            100.0 * share
        );
        let _ = writeln!(
            report,
            "- hcp: min {hcp_min} / p10 {hcp_10} / median {hcp_50} / p90 {hcp_90} / max {hcp_max}"
        );

        let mut clauses = vec![format!("hcp({hcp_10}..={hcp_90})")];
        for (i, suit) in Suit::ASC.into_iter().enumerate() {
            let mut col = bucket.len[i].clone();
            col.sort_unstable();
            let (lo, mid, hi) = (pct(&col, 0.10), pct(&col, 0.5), pct(&col, 0.90));
            let mark = if i == opener_suit {
                " [opener's suit]"
            } else {
                ""
            };
            let _ = writeln!(report, "- {suit:?}{mark}: {lo}–{hi} (median {mid})");
            if lo >= 4 && i != opener_suit {
                clauses.push(format!("len({suit:?}, {lo}..)"));
            }
        }

        let bal = bucket.balanced as f64 / n as f64;
        let _ = writeln!(report, "- balanced: {:.0}%", 100.0 * bal);
        if bal > 0.8 {
            clauses.push("balanced()".to_string());
        }
        let _ = writeln!(report, "- sketch: {}", clauses.join(" & "));
        let _ = writeln!(report);
    }

    assert!(
        buckets.values().flat_map(|b| &b.hcp).all(|&h| h <= 37),
        "HCP out of range — eval wiring is wrong"
    );
}

/// HCP via the simple evaluator (matches the DSL's `hcp(..)`).
fn hcp(hand: Hand) -> u8 {
    SimpleEvaluator(eval::hcp::<u8>).eval(hand)
}

/// Balanced: every suit >= 2 and at most one doubleton.
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
