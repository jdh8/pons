//! Shared helpers for the A/B / measurement harnesses (`ab-*`, some `probe-*`).
//!
//! Pulled in verbatim with
//! `#[path = "../common/mod.rs"] #[allow(dead_code)] mod common;` — a sibling
//! directory holding only `mod.rs` (no `main.rs`) is invisible to Cargo's example
//! auto-discovery, so this never compiles as a standalone example. Each harness
//! uses only the subset it needs, hence the `#[allow(dead_code)]` on the `mod`.

#[allow(dead_code)]
pub mod bba_book;
#[allow(dead_code)]
pub mod filters;
#[allow(dead_code)]
pub mod mass;
#[allow(dead_code)]
pub mod oracle;
#[allow(dead_code)]
pub mod rollout;
#[cfg(test)]
mod tests;

use contract_bridge::auction::{Auction, Call, display_calls};
use contract_bridge::deck::full_deal;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Rank, Seat, Strain, Suit,
};
use ddss::{NonEmptyStrainFlags, TrickCountTable};
use pons::bidding::agreements::Agreements;
use pons::bidding::card::{Card, american_card};
use pons::bidding::context::relative;
use pons::bidding::features::ConventionCard;
use pons::bidding::{Bidder, Partnership};
use pons::scoring::{
    final_contract, imps, ns_score_bid, ns_score_contract, ns_score_pd_tricks, ns_score_tricks,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// `--ns-preset NAME`: replace every knob with a named system variant
/// (`docs/system-variants.md`), keeping only the opponents' disclosures.
///
/// Applied **after** the `--ns-*` overlay, because every binary assigns each
/// flag unconditionally, defaults included, so a preset laid down first would
/// be clobbered back to the shipped system by the flags' own defaults.  A
/// preset therefore overrides the knob flags; "basic plus one gadget" is a
/// constructor edit, not a flag combination.
pub fn apply_preset(agreements: &mut Agreements, preset: &str) -> anyhow::Result<()> {
    let their = agreements.decision.their;
    *agreements = match preset {
        "basic" => Agreements::basic(),
        "advanced" => Agreements::advanced(),
        other => anyhow::bail!("--ns-preset must be basic or advanced, got {other:?}"),
    };
    agreements.decision.their = their;
    Ok(())
}

/// The vs-BBA disclosure corrections: our agreements as armed against the
/// EPBot 2/1 reference with no explicit `--their-conv` declaration.
///
/// Two corrections, one family.  BBA's 2/1 **card** declares its 1NT defense as
/// Cappelletti (`21GF.bbsa`: `Cappelletti = 1`, `Landy = 0`,
/// `Multi-Landy = 0`), but its measured **behavior** is Woolsey Multi-Landy —
/// the per-call census read 551 `(2♣)` overcalls of our 1NT as both-majors
/// Landy (docs/one-notrump-competitive.md), and the rows stay inert for its
/// own bidding even when set false.  The card lies, so the default read comes
/// from the census: their `2♣` is Landy and our counter-defense engages
/// (`Agreements::decision.their.two_clubs_landy` — the disclosure channel, engine
/// default *undeclared/natural* for unknown fields).  An explicit declaration
/// overrides this — we play to disclosure, and a bot that bids Landy behind a
/// declared no-Landy card commits *its* infraction.  Its `2♦` is the same
/// convention's Multi (`their.two_diamonds_multi`, N4 — shipped 2026-08-15
/// on the same census; `bba-gen`'s `their_2d_multi`).
///
/// Shared by `bba-gen` (arming, `their_2c_landy`) and `bba-decompose`
/// (replay) so the replay contract — decompose reproduces the generating
/// system bit for bit — survives the arming being derived rather than
/// flagged.
pub fn vs_bba_agreements(mut agreements: Agreements) -> Agreements {
    agreements.decision.their.two_clubs_landy = true;
    // N4 (v7, 2026-08-15): the same census reads its `2♦` as the Multi.
    agreements.decision.their.two_diamonds_multi = true;
    agreements
}

/// Total HCP of a hand
pub fn hand_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

/// The raw-hand statistic an enriched draw thresholds: over both partnerships,
/// the best `(combined HCP, longest combined non-spade fit)` available.
///
/// Both components are maxed across the two partnerships independently — the
/// question a filter asks is "does this deal contain a slam-ish non-spade fit
/// anywhere", and either side reaching one makes the deal worth bidding.
///
/// Non-spade because that is where a relocated keycard ask can differ at all: a
/// spade ask is 4NT under either card, so ♠ fits carry no configuration signal.
/// It reads **raw hands only**, which is what lets an enriched corpus draw
/// accept or reject a deal before the bidder ever sees it — the acceptance can
/// then never depend on, and so never bias, what the bidder does with it.
pub fn slam_ish(deal: &FullDeal) -> (u8, u8) {
    [(Seat::North, Seat::South), (Seat::East, Seat::West)]
        .into_iter()
        .map(|(one, two)| {
            let (a, b) = (deal[one], deal[two]);
            let fit = [Suit::Clubs, Suit::Diamonds, Suit::Hearts]
                .into_iter()
                .map(|suit| a[suit].len() + b[suit].len())
                .max()
                .unwrap_or(0);
            (
                hand_hcp(a) + hand_hcp(b),
                u8::try_from(fit).unwrap_or(u8::MAX),
            )
        })
        .fold((0, 0), |acc, x| (acc.0.max(x.0), acc.1.max(x.1)))
}

/// One display key per decision: leading passes stripped, calls joined
///
/// Stripping merges the four dealer rotations of one decision into one line —
/// leading passes only encode the seat, which the books already fan over.  The
/// result is directly greppable against `examples/render-book`.
pub fn auction_key(auction: &[Call]) -> String {
    let calls = auction
        .iter()
        .skip_while(|&&call| call == Call::Pass)
        .copied()
        .collect::<Vec<_>>();
    if calls.is_empty() {
        "(opening passes)".to_owned()
    } else {
        display_calls(&calls).to_string()
    }
}

/// The seat acting after `len` calls from `dealer`
pub const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

/// If this auction's *opening* call is 1NT, its index and whether the opener is
/// North/South.  The opening requirement (all prior calls passes) excludes a
/// `1♣ - 1NT` rebid — we want 1NT *openings* only.  `bba-score` splits its
/// report on it; `bba-gen`/`ben-gen` `--isolate-defense` keep only the boards
/// where E/W open 1NT and our N/S defend.
pub fn opening_1nt(auction: &[Call], dealer: Seat) -> Option<(usize, bool)> {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let index = auction.iter().position(|&call| call == one_nt)?;
    if auction[..index].iter().any(|&call| call != Call::Pass) {
        return None;
    }
    let opener_ns = matches!(seat_to_act(dealer, index), Seat::North | Seat::South);
    Some((index, opener_ns))
}

/// The `1NT (3x) X -` leave-in slice key: opener's holding in **their** suit
///
/// `nt_high_overcall_x_leave_in`'s only row is a `Pass` priced between the
/// four-card-major rungs and `3NT`, so every divergence of that A/B is the same
/// five-call window — `1NT (3x) X -`, ON passing, OFF pulling — and the
/// question the slice answers is *which* holdings in their seven-card suit are
/// worth defending.  Key: their suit x length bucket x top honors (A/K/Q) x
/// whether today's ladder already outbids the `Pass`.
///
/// `None` when the two auctions agree (nothing to bucket).  `"(other)"` when
/// they diverge anywhere else — on a dump of this experiment that bucket must
/// read **zero**, which is what makes the key a check and not just a label.
/// `on` must be the leave-in arm: the window demands ON pass at the seam.
///
/// The `4M` half is the boards **today's** ladder no longer passes.
/// `nt_high_overcall_x_major_at_four` shipped *after* these dumps were bid, so
/// over `(3♠)` four hearts now answer `4♥` at 140 and outrank the `Pass` at
/// 135 whatever this gate says.  Exclude those rows when judging a new gate.
pub fn holding_key(on: &Board, off: &Board) -> Option<String> {
    let (a, b) = (&on.table_a, &off.table_a);
    let i = (0..a.len().min(b.len())).find(|&index| a[index] != b[index])?;
    let other = || Some("(other)".to_owned());
    if i < 4 || a[i] != Call::Pass {
        return other();
    }
    let (Call::Bid(opening), Call::Bid(overcall)) = (a[i - 4], a[i - 3]) else {
        return other();
    };
    if opening.level.get() != 1
        || opening.strain != Strain::Notrump
        || overcall.level.get() != 3
        || a[i - 2] != Call::Double
        || a[i - 1] != Call::Pass
    {
        return other();
    }
    let Some(over) = overcall.strain.suit() else {
        return other();
    };
    // Opener bid the `1NT` at `i - 4` and answers the double at `i`, one full
    // round later — the same seat either way.
    let opener = seat_to_act(on.dealer, i - 4);
    let holding = on.deal[opener][over];
    let length = match holding.len() {
        0 | 1 => "len0-1",
        2 => "len2",
        3 => "len3",
        _ => "len4+",
    };
    let honors = match [Rank::A, Rank::K, Rank::Q]
        .into_iter()
        .filter(|&rank| holding.contains(rank))
        .count()
    {
        0 => "hon0",
        1 => "hon1",
        _ => "hon2+",
    };
    let outbid = [Suit::Hearts, Suit::Spades]
        .into_iter()
        .any(|major| major != over && on.deal[opener][major].len() >= 4);
    let outbid = if outbid { "4M" } else { "no4M" };
    Some(format!("({overcall}) {length} {honors} {outbid}"))
}

/// Bucket key for the Stayman-defense A/B: our direct call over
/// `(1NT) - (2♣)` in each arm, as `"on X / off -"`.
///
/// `None` when the two auctions agree; `"(other)"` when they diverge but
/// neither is a `(1NT) - (2♣)` auction.
pub fn stayman_key(on: &Board, off: &Board) -> Option<String> {
    let (a, b) = (&on.table_a, &off.table_a);
    if a[..] == b[..] {
        return None;
    }
    let call = |calls: &[Call]| {
        let k = calls.iter().position(|&call| call != Call::Pass)?;
        let Call::Bid(opening) = calls[k] else {
            return None;
        };
        let Call::Bid(ask) = *calls.get(k + 2)? else {
            return None;
        };
        (opening.level.get() == 1
            && opening.strain == Strain::Notrump
            && calls[k + 1] == Call::Pass
            && ask.level.get() == 2
            && ask.strain == Strain::Clubs)
            .then(|| {
                calls
                    .get(k + 3)
                    .map_or("end".to_owned(), ToString::to_string)
            })
    };
    match (call(a), call(b)) {
        (Some(x), Some(y)) => Some(format!("on {x} / off {y}")),
        _ => Some("(other)".to_owned()),
    }
}

/// Split key for `ab-dump-sd --by stayman-x`: on boards where **both** arms
/// reach `(1NT) - (2♣) X`, the first divergent call, keyed by the seat that
/// made it (opener, advancer, responder, doubler) and the calls since the `X`;
/// every other divergent board keys as `(other)`.
pub fn stayman_x_key(on: &Board, off: &Board) -> Option<String> {
    let (a, b) = (&on.table_a, &off.table_a);
    if a[..] == b[..] {
        return None;
    }
    let k = a.iter().position(|&call| call != Call::Pass)?;
    let stayman_x = |calls: &[Call]| {
        calls.get(k..k + 4).is_some_and(|w| {
            w[0] == Call::Bid(Bid::new(1, Strain::Notrump))
                && w[1] == Call::Pass
                && w[2] == Call::Bid(Bid::new(2, Strain::Clubs))
                && w[3] == Call::Double
        })
    };
    if !(stayman_x(a) && stayman_x(b)) {
        return Some("(other)".to_owned());
    }
    let i = (k..).find(|&i| a.get(i) != b.get(i))?;
    let role = ["opener", "advancer", "responder", "doubler"][(i - k) % 4];
    let show = |call: Option<&Call>| call.map_or("end".to_owned(), ToString::to_string);
    let since: Vec<_> = a[k + 4..i].iter().map(ToString::to_string).collect();
    Some(format!(
        "{role} after X {}: on {} / off {}",
        since.join(" "),
        show(a.get(i)),
        show(b.get(i))
    ))
}

/// The RNG for board `index` of the experiment seeded `base`: ChaCha keyed on
/// the pair, so boards are independent of thread order and two bases share no
/// boards.  The retired key `seed_from_u64(base + index)` slid instead —
/// `base + k` replayed board `i + k` of `base` as its board `i` (issue #6).
pub fn board_rng(base: u64, index: u64) -> StdRng {
    let mut key = [0; 32];
    key[..8].copy_from_slice(&base.to_le_bytes());
    key[8..16].copy_from_slice(&index.to_le_bytes());
    StdRng::from_seed(key)
}

/// `count` deals, board `i` drawn from [`board_rng`]`(base, i)`, so every arm of
/// an experiment replays the identical stream (the seed-hygiene invariant the
/// A/B scripts rely on — one `SEED_BASE` shared across arms).  Bidding is pure
/// and parallelizes downstream; only the deal generation is centralized here.
pub fn seeded_deals(base: u64, count: usize) -> Vec<FullDeal> {
    (0..count as u64)
        .map(|i| full_deal(&mut board_rng(base, i)))
        .collect()
}

/// The highest-logit *legal* call, defaulting to a pass
pub fn next_call(
    partnership: &Partnership,
    hand: Hand,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    auction: &Auction,
) -> Call {
    let seat = seat_to_act(dealer, auction.len());
    let Some(logits) = partnership.classify(hand, relative(vul, seat), auction) else {
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

/// Bid one deal with the convention pair on the side picked by `conv_is_ns`
pub fn bid_out(
    conv: &Partnership,
    baseline: &Partnership,
    conv_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let partnership = if seat_is_ns == conv_is_ns {
            conv
        } else {
            baseline
        };
        auction.push(next_call(partnership, deal[seat], dealer, vul, &auction));
    }
    auction
}

/// Bid one deal with the opponents (East/West) forced to pass throughout
pub fn bid_uncontested(
    partnership: &Partnership,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let call = if matches!(seat, Seat::East | Seat::West) {
            Call::Pass
        } else {
            next_call(partnership, deal[seat], dealer, vul, &auction)
        };
        auction.push(call);
    }
    auction
}

/// One board: the deal, the dealer, and both tables' auctions.  The interchange
/// unit between `bba-gen` (writes it) and `bba-score` (reads it).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Board {
    pub deal: FullDeal,
    pub dealer: Seat,
    /// Our pair North/South
    pub table_a: Auction,
    /// Our pair East/West
    pub table_b: Auction,
}

/// A serialized match: the bidder labels, the vulnerability the boards were bid
/// at, and every board.  `bba-gen` writes it; `bba-score` reads and scores it.
/// The serde derive is feature-gated so non-`serde` harnesses still compile this
/// shared module (the struct is then just dead code).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dump {
    pub our_label: String,
    pub their_label: String,
    pub vulnerability: AbsoluteVulnerability,
    /// The deal seed this shard was generated from, so the exact board stream
    /// is reproducible forever.  `None` in dumps written before the anchor
    /// campaign (serde-defaulted for backwards compatibility).
    #[cfg_attr(feature = "serde", serde(default))]
    pub seed: Option<u64>,
    /// The generating command line (`argv[1..]`), so a scorer can rebuild the
    /// exact book configuration instead of guessing knob state.  Empty in
    /// older dumps.
    #[cfg_attr(feature = "serde", serde(default))]
    pub gen_args: Vec<String>,
    /// Whether our pair read the opponents through
    /// [`european_minors_opponents`] while bidding this dump, so a scorer reads
    /// the auctions the same way.  `None` in dumps written before 2026-10-20,
    /// when `bba-gen` declared only on `--declare-their-european-minors`; see
    /// [`Dump::declares_european_minors`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub declared_european_minors: Option<bool>,
    pub boards: Vec<Board>,
}

impl Dump {
    /// Whether our pair read the opponents' `1NT` minors as European while
    /// bidding this dump: the recorded fact, else (older dumps) whether the
    /// generating command line opted in
    #[must_use]
    pub fn declares_european_minors(&self) -> bool {
        self.declared_european_minors.unwrap_or_else(|| {
            self.gen_args
                .iter()
                .any(|arg| arg == "--declare-their-european-minors")
        })
    }
}

/// Load a dump from `path`.  A plain file is read directly; a **directory**
/// globs its `shard-*.json` (sorted, so paired arms concatenate in the same
/// order) and folds every shard's boards into one dump.  Handing a harness the
/// whole board set lets it solve the entire divergent set in a single DDS
/// fan-out, instead of launching one solver process per shard — each of which
/// spins a full-core DDS pool and oversubscribes the box.
/// Every shard must name the same bidders and bidding vulnerability, and be
/// generated with the same arguments up to [`shard_config`]'s per-shard values
/// (seed, output path, board count, server port, chunk offset, the chunks an
/// assembled shard was built from).
#[cfg(feature = "serde")]
pub fn load_dump(path: &str) -> Dump {
    fn read(path: &std::path::Path) -> Dump {
        serde_json::from_reader(std::io::BufReader::new(
            std::fs::File::open(path)
                .unwrap_or_else(|e| panic!("open dump {}: {e}", path.display())),
        ))
        .unwrap_or_else(|e| panic!("parse dump {}: {e}", path.display()))
    }
    let root = std::path::Path::new(path);
    if !root.is_dir() {
        return read(root);
    }
    let mut shards: Vec<std::path::PathBuf> = std::fs::read_dir(root)
        .unwrap_or_else(|e| panic!("read dir {path}: {e}"))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("shard-") && n.ends_with(".json"))
        })
        .collect();
    assert!(!shards.is_empty(), "no shard-*.json in {path}");
    shards.sort();
    let mut dump = read(&shards[0]);
    for shard in &shards[1..] {
        let next = read(shard);
        assert_eq!(
            dump.vulnerability,
            next.vulnerability,
            "dump shard {} has a different bidding vulnerability",
            shard.display(),
        );
        assert_eq!(
            dump.our_label,
            next.our_label,
            "dump shard {} has a different bidder for our pair",
            shard.display(),
        );
        assert_eq!(
            dump.their_label,
            next.their_label,
            "dump shard {} has a different reference bidder",
            shard.display(),
        );
        assert_eq!(
            shard_config(&dump.gen_args),
            shard_config(&next.gen_args),
            "dump shard {} was generated with different arguments",
            shard.display(),
        );
        assert_eq!(
            dump.declares_european_minors(),
            next.declares_european_minors(),
            "dump shard {} read the opponents differently",
            shard.display(),
        );
        dump.boards.extend(next.boards);
    }
    dump
}

/// A shard's generating arguments less the values that legitimately differ
/// between the shards of one arm: `--seed` (`SEED_BASE + i`), `--output`/`-o`
/// (the shard's own path), `--count`/`-c` (a re-dealt chunk), `--port`/`-p`
/// (`ben-gen`'s server per shard) and `--start-board` (a split shard's chunk
/// offset).  Every other argument is configuration, so two shards of one dir
/// must agree on it.
///
/// `--note` is configuration too: it records the BEN runtime (the runners'
/// `ben-servers-sha256=`, older `*-conf-sha256=` and `BEN=` tokens), so a dir
/// whose shards were bid by different servers refuses to load.  Its
/// whitespace-separated tokens are kept except `assembled-from=…`, the chunk
/// list a reassembled shard appends, and a note left empty by that counts as
/// none.  Of the 46 noted shard dirs here and on dl02 (2026-10-18), only
/// `ben-anchor/2026-09-13-daa8bf4a/both` then disagrees: two of its shards
/// were recovered under the PASS-level patch, a mixed runtime
/// docs/ben-gap-campaign.md records.
///
/// Short forms are those of `bba-gen` and `ben-gen`, the generators that write
/// shard dirs; `-p` is `ben-gen`'s port and unused by `bba-gen`.
pub fn shard_config(gen_args: &[String]) -> Vec<&str> {
    const PER_SHARD: [&str; 8] = [
        "--seed",
        "--output",
        "-o",
        "--count",
        "-c",
        "--port",
        "-p",
        "--start-board",
    ];
    fn note<'a>(config: &mut Vec<&'a str>, note: &'a str) {
        let start = config.len();
        config.push("--note");
        config.extend(
            note.split_whitespace()
                .filter(|token| !token.starts_with("assembled-from=")),
        );
        if config.len() == start + 1 {
            config.pop();
        }
    }
    let mut config = Vec::with_capacity(gen_args.len());
    let mut args = gen_args.iter();
    while let Some(arg) = args.next() {
        match arg.split_once('=') {
            Some(("--note", value)) => note(&mut config, value),
            None if arg == "--note" => note(&mut config, args.next().map_or("", String::as_str)),
            Some((flag, _)) if PER_SHARD.contains(&flag) => {}
            None if PER_SHARD.contains(&arg.as_str()) => {
                args.next();
            }
            _ => config.push(arg.as_str()),
        }
    }
    config
}

/// `--verify`: `path` parses as a [`Dump`] of exactly `count` boards dealt
/// from `seed` (when given).  A truncated shard fails to parse.  The shard
/// check `scripts/{bba,ben}-gen-parallel.sh --verify` runs on every shard.
#[cfg(feature = "serde")]
pub fn verify_shard(path: &str, count: usize, seed: Option<u64>) -> anyhow::Result<()> {
    use anyhow::Context as _;
    let file = std::fs::File::open(path).with_context(|| format!("open shard {path}"))?;
    let dump: Dump = serde_json::from_reader(std::io::BufReader::new(file))
        .with_context(|| format!("parse shard {path}"))?;
    anyhow::ensure!(
        dump.boards.len() == count,
        "shard {path} holds {} boards, expected {count}",
        dump.boards.len(),
    );
    if let Some(seed) = seed {
        anyhow::ensure!(
            dump.seed == Some(seed),
            "shard {path} was dealt from seed {:?}, expected {seed}",
            dump.seed,
        );
    }
    Ok(())
}

/// Write `path` through a hidden temp file beside it, renamed into place after
/// a flush and an fsync, so a worker that dies mid-write (an error, a panic, a
/// kill) leaves no `shard-*.json` behind for a resume to count -- at most a
/// `.shard-*.tmp` that no shard glob matches.  On an error or a panic the temp
/// file is removed.
pub fn write_atomically(
    path: &str,
    write: impl FnOnce(&mut dyn std::io::Write) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    use anyhow::Context as _;
    /// Removes the temp file unless disarmed by a successful rename.
    struct Temp(Option<std::path::PathBuf>);
    impl Drop for Temp {
        fn drop(&mut self) {
            if let Some(path) = &self.0 {
                let _ = std::fs::remove_file(path);
            }
        }
    }
    let path = std::path::Path::new(path);
    let name = path
        .file_name()
        .with_context(|| format!("--output {} names no file", path.display()))?;
    let tmp = path.with_file_name(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let mut guard = Temp(Some(tmp.clone()));
    let mut out = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
    write(&mut out)?;
    // `into_inner` flushes and reports the error a dropped BufWriter swallows.
    out.into_inner()
        .map_err(std::io::IntoInnerError::into_error)?
        .sync_all()?;
    std::fs::rename(&tmp, path)?;
    guard.0 = None;
    Ok(())
}

/// The file this code was mapped from, for a generator's `--provenance`
///
/// [`std::env::current_exe`] names the kernel's executable, which is the
/// dynamic loader when a wrapper runs us as `ld-linux.so bba-gen` (`BBA_GEN` on
/// an old-glibc box), so read the mapping that holds this very function.
pub fn exe_image() -> std::io::Result<std::path::PathBuf> {
    let here = exe_image as *const () as usize;
    let maps = std::fs::read_to_string("/proc/self/maps").unwrap_or_default();
    for line in maps.lines() {
        let (Some((lo, hi)), Some(slash)) = (
            line.split_whitespace()
                .next()
                .and_then(|r| r.split_once('-')),
            line.find('/'),
        ) else {
            continue;
        };
        if let (Ok(lo), Ok(hi)) = (usize::from_str_radix(lo, 16), usize::from_str_radix(hi, 16))
            && (lo..hi).contains(&here)
        {
            return Ok(line[slash..].into());
        }
    }
    std::env::current_exe()
}

/// `--provenance`: the files whose bytes decide a generator's run, one
/// `role path` per line -- `exe` (the image this code was mapped from), `lib`
/// (the EPBot library) and one `weights` per `PONS_FLOOR_WEIGHTS` blob.
/// `scripts/ab-lib.sh` fingerprints each A/B arm by them.
pub fn print_provenance(lib: &str) -> anyhow::Result<()> {
    println!("exe {}", exe_image()?.display());
    println!("lib {lib}");
    if let Ok(paths) = std::env::var("PONS_FLOOR_WEIGHTS") {
        for path in paths.split(',') {
            println!("weights {path}");
        }
    }
    Ok(())
}

/// Sample mean and the half-width of its 95% confidence interval
///
/// The mean is the headline IMPs/board; the half-width is `1.96 · SE` from the
/// per-board sample standard deviation, so a CI that excludes 0 is a result
/// distinguishable from noise.  Under two values there is no sample variance:
/// the half-width is `+∞`, which [`verdict`] reads as inconclusive.
#[allow(clippy::cast_precision_loss)]
pub fn mean_with_ci(values: &[i64]) -> (f64, f64) {
    let n = values.len();
    if n < 2 {
        return (values.first().map_or(0.0, |&v| v as f64), f64::INFINITY);
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

/// One table's contract priced at **both** single-dummy endpoints of a single
/// composed run ([`pons::single_dummy_declarer_tricks`]): the lead endpoint
/// (blind lead, then double-dummy play — the optimist at slam level) and the
/// playout endpoint (the same lead, then a fallible declarer — the
/// pessimist).  Each endpoint is a `[plain, perfect-defense]` pair as in
/// [`ns_score_tricks`] / [`ns_score_pd_tricks`]; `level` selects the blend
/// weight in [`sd_blend_imps`].  A pass-out is all zeros with `level` 0.
#[derive(Clone, Copy, Default)]
pub struct SdScores {
    /// Contract level (1–7; 0 for a pass-out)
    pub level: u8,
    /// Declarer's tricks `[lead endpoint, playout endpoint]` — kept raw so a
    /// caller can read make indicators (`tricks >= level + 6`) for the
    /// blend's implied make%, which the signed NS scores obscure
    pub tricks: [u8; 2],
    /// `[plain, perfect-defense]` NS scores of the lead-endpoint tricks
    pub lead: [i64; 2],
    /// `[plain, perfect-defense]` NS scores of the playout tricks
    pub playout: [i64; 2],
    /// How the worlds behind `[lead endpoint, playout endpoint]` were dealt:
    /// the auction-constrained sample against its fallback tier
    /// ([`pons::bidding::SampleStats`]; default for a pass-out)
    pub sampling: [pons::bidding::SampleStats; 2],
}

/// NS scores of an auction's reached contract under **both sd endpoints**:
/// blind opening lead, then the deal priced once with double-dummy play after
/// the lead (the optimist at slam level — Pavlicek's after-lead table has DD
/// play +7pp of make-rate on slams) and once with a declarer who chooses
/// every card over range-consistent worlds instead of peeking
/// (the pessimist — its misguess haircut measures ≈1.5× the real one).
/// `reader` reads the auction for both modelled views (the leader's and
/// declarer's, [`TableReader::view`]); a pass-out scores 0.
///
/// Each endpoint is priced `[plain, perfect-defense]` ([`ns_score_tricks`]
/// and [`ns_score_pd_tricks`]).  Everything comes back from a single call
/// because the playout is expensive *and* draws from `rng`: calling twice
/// would run a second playout with different draws, pricing a *different*
/// trick count rather than the same one twice.
///
/// Read the playout's perfect-defense entry as a pessimism stress-test, not
/// an arbiter.  The defenders here already play double-dummy after the lead
/// ([`pons::single_dummy_playout`]), so the PD entry layers perfect doubling
/// on top of peeking defense and tracks the harness's existing DD-PD row; and
/// doubling a failing contract is realistic at partscore and game but not at
/// slam, where nobody doubles a voluntarily bid six.  The calibrated point
/// estimate between the endpoints is [`sd_blend_imps`].
// Each argument is a distinct fact of the board, as in `score_boards`.
#[allow(clippy::too_many_arguments)]
pub fn sd_declarer_ns_score(
    auction: &Auction,
    dealer: Seat,
    deal: &FullDeal,
    reader: &TableReader,
    vul: AbsoluteVulnerability,
    rng: &mut StdRng,
    lead_worlds: usize,
    line_worlds: usize,
) -> SdScores {
    let Some((contract, declarer)) = final_contract(auction, dealer) else {
        return SdScores::default();
    };
    let view = |seat: Seat| reader.view(auction, dealer, seat, vul);
    let (_, lead_tricks, line_tricks, sampling) = pons::single_dummy_declarer_tricks_with_stats(
        deal,
        contract.bid.strain,
        declarer,
        &view(declarer.lho()),
        &view(declarer),
        rng,
        lead_worlds,
        line_worlds,
    );
    let price = |tricks: u8| {
        [
            ns_score_tricks(contract, declarer, tricks, vul),
            ns_score_pd_tricks(contract, declarer, tricks, vul),
        ]
    };
    SdScores {
        level: contract.bid.level.get(),
        tricks: [u8::from(lead_tricks), u8::from(line_tricks)],
        lead: price(u8::from(lead_tricks)),
        playout: price(u8::from(line_tricks)),
        sampling,
    }
}

/// How the blind single-dummy players of one table read its auction: each
/// seat through its own pair's partnership, knowing what the *other* pair's
/// agreements disclose
///
/// What a defender may know of the declaring side's calls is what that side's
/// agreements disclose: every call it made, alerted or not, decoded off the
/// rule that authored it.  Reading through a plain partnership gives only the
/// alerted ones — the opponents' calls decode at `ReadingScope::Alerted`
/// unless an opponent is declared — so an unalerted natural call (the
/// `(1NT) 2♥` overcall band, a weak two's range, a free bid's strength) fell
/// to the natural walk, and a manipulation of its disclosure could not reach
/// the leader (docs/research-method-review.md §4).  A disclosed reader
/// therefore declares the other pair's partnership as its opponent
/// ([`Partnership::with_opponents`]), which decodes their calls at the full
/// reading scope through their books.
///
/// Two table shapes:
///
/// - **`bba-gen` dump** ([`new`][Self::new]): North–South is ours and
///   East–West the foreign engine.  East–West reads with our partnership
///   declared as its opponent; North–South reads as it always has, because
///   the foreign engine's system is not ours to disclose.
/// - **Self-play** ([`self_play`][Self::self_play]): both pairs are ours —
///   often the same partnership, sometimes different arms per pair.  Each
///   pair reads with the other pair's partnership declared as its opponent.
///
/// Every harness report prints [`mode`][Self::mode] on a `leader reading:`
/// line; a report without one predates 2026-10-16 and is legacy.
pub struct TableReader {
    ns: Partnership,
    ew: Partnership,
    mode: LeaderReading,
}

/// Which reader a [`TableReader`] is, for the report's `leader reading:` line
#[derive(Clone, Copy)]
enum LeaderReading {
    Dump,
    DumpLegacy,
    SelfPlay,
    SelfPlayLegacy,
}

impl LeaderReading {
    const fn label(self) -> &'static str {
        match self {
            Self::Dump => "disclosed (EW readers decode every NS call off its authoring rule)",
            Self::DumpLegacy => {
                "legacy (EW readers see only our alerted calls; pre-2026-10-16 outputs)"
            }
            Self::SelfPlay => {
                "disclosed (self-play: each pair decodes the other's calls off their authoring rules)"
            }
            Self::SelfPlayLegacy => {
                "legacy (self-play: each pair sees only the other's alerted calls; pre-2026-10-16 outputs)"
            }
        }
    }
}

impl TableReader {
    /// Read a `bba-gen` dump's `table_a` with our partnership disclosed to
    /// East–West
    pub fn new(ours: Partnership) -> Self {
        let ew = ours.clone().with_opponents(&ours);
        Self {
            ns: ours,
            ew,
            mode: LeaderReading::Dump,
        }
    }

    /// The pre-2026-10-16 dump reader: both pairs read through the plain
    /// partnership, so East–West sees only our alerted calls. Kept so old
    /// `ab-dump-sd` outputs can be reproduced.
    pub fn legacy(ours: Partnership) -> Self {
        Self {
            ew: ours.clone(),
            ns: ours,
            mode: LeaderReading::DumpLegacy,
        }
    }

    /// Read a self-play table whose North–South bid off `ns` and East–West
    /// off `ew` (the same partnership at an arm-pure table)
    ///
    /// `legacy` restores the pre-2026-10-16 reader — each seat through the
    /// partnership given for its pair, no opponent declared, so it sees only
    /// the other pair's *alerted* calls.  A harness that used to read every
    /// seat through one fixed book passes that book as both `ns` and `ew`.
    pub fn self_play(ns: &Partnership, ew: &Partnership, legacy: bool) -> Self {
        if legacy {
            Self {
                ns: ns.clone(),
                ew: ew.clone(),
                mode: LeaderReading::SelfPlayLegacy,
            }
        } else {
            Self {
                ns: ns.clone().with_opponents(ew),
                ew: ew.clone().with_opponents(ns),
                mode: LeaderReading::SelfPlay,
            }
        }
    }

    /// One line naming the leader-reading mode, for report headers
    pub const fn mode(&self) -> &'static str {
        self.mode.label()
    }

    /// [`mode`][Self::mode] of a [`self_play`][Self::self_play] reader, for a
    /// report that has none at hand
    pub const fn self_play_mode(legacy: bool) -> &'static str {
        if legacy {
            LeaderReading::SelfPlayLegacy.label()
        } else {
            LeaderReading::SelfPlay.label()
        }
    }

    /// What `seat` reads from `prefix` (the player to act after it)
    pub fn infer(
        &self,
        seat: Seat,
        vul: AbsoluteVulnerability,
        prefix: &[Call],
    ) -> pons::bidding::Inferences {
        let reader = match seat {
            Seat::North | Seat::South => &self.ns,
            Seat::East | Seat::West => &self.ew,
        };
        reader.infer(relative(vul, seat), prefix)
    }

    /// What `seat` reads from a complete `auction`, cut so that `seat` is the
    /// player to act: the last non-pass call sits within the final four
    /// calls, so exactly one of four consecutive prefix lengths keeps every
    /// non-pass call and puts `seat` to act
    pub fn view(
        &self,
        auction: &Auction,
        dealer: Seat,
        seat: Seat,
        vul: AbsoluteVulnerability,
    ) -> pons::bidding::Inferences {
        let cut = (auction.len().saturating_sub(3)..=auction.len())
            .find(|&len| seat_to_act(dealer, len) == seat)
            .expect("one of four consecutive lengths reaches every seat");
        self.infer(seat, vul, &auction[..cut])
    }

    /// The (contract, declarer, leader-view inferences) of one auction — the
    /// inputs of its blind-lead question; `None` for a pass-out (sd score 0)
    pub fn lead_inputs(
        &self,
        auction: &Auction,
        dealer: Seat,
        vul: AbsoluteVulnerability,
    ) -> Option<(Contract, Seat, pons::bidding::Inferences)> {
        let (contract, declarer) = final_contract(auction, dealer)?;
        Some((
            contract,
            declarer,
            self.view(auction, dealer, declarer.lho(), vul),
        ))
    }
}

/// Per-level weight of the playout endpoint in the **sd-blend**: with
/// probability `λ(level)` the table result is the playout's (fallible
/// declarer), else the lead endpoint's (double-dummy declarer after the same
/// blind lead).  Index by contract level; index 0 (pass-out) is 0 and never
/// matters (a pass-out's scores are 0 at both endpoints).
///
/// λ is fitted per level so the blend's make-rate shifts the lead endpoint's
/// make-logit by Pavlicek's **after-lead** log-odds (actual play vs
/// double-dummy play after the actual lead, rpbridge.net/8j45.htm — the exact
/// bias position of the lead endpoint): `p_target = σ(logit(p_lead) +
/// shift_L)`, `λ_L = (p_lead − p_target) / (p_lead − p_playout)`, from the
/// per-level columns of `probe-sd-calibration`.  Both endpoints share the
/// blind lead, so λ tunes exactly the declarer axis — at the 6-level the
/// after-lead shift is −0.334 log-odds (DD play makes 73.67% of slams that
/// actually made 66.70%), and the playout's own haircut is ≈1.5× that,
/// hence λ ≈ 0.66 rather than 1.
///
/// Level 7 keeps level 6's weight by design: Pavlicek's after-lead shift is
/// flat across the slam levels (−0.334 vs −0.339) and the grands' extra
/// full-deal bias lives on the *lead* side, which the shared lead endpoint
/// models itself; grands are additionally reported as a bracket, never a
/// point estimate (decision anchors, bba-kickback.md §7.15).
///
/// Fitted 2026-08-04 by `probe-sd-calibration --per-level 1000
/// --min-level-count 800` (seed 20260716, 16 worlds, 39,633 contracts), the
/// first run on the playout with the current-trick sequence-collapse fix —
/// pre-fix playout numbers are corrupted (CHANGELOG "Fixed").  The fit put
/// λ(7) = 0.696 on only 54 grands; it inherits λ(6) by design (the shifts
/// are flat, −0.334 vs −0.339, and n = 54 is noise).
///
/// **Holdout-validated 2026-08-04** on fresh seed 1785789480 (same settings,
/// 39,648 contracts): applying THIS λ to the holdout's endpoints reproduces
/// the holdout's Pavlicek targets within 0.6pp at every level ≤ 6 (binomial
/// noise ≈ 1.5pp at n = 1000), so the population check is no longer circular;
/// the refit λ column moves ≤ 0.02 except level 5 (0.47 → 0.39), and the
/// λ(4) > λ(5) inversion REPLICATES in both samples — 5-level contracts
/// genuinely carry a smaller misguess share, it is not sample noise.  λ is a
/// property of the 16-world emulation: re-fit before changing world count.
pub const SD_BLEND_LAMBDA: [f64; 8] = [0.0, 0.089, 0.242, 0.298, 0.539, 0.474, 0.664, 0.664];

/// The **sd-blend** IMP swing of one divergent board, bracket `k` (0 plain,
/// 1 perfect-defense): the expected IMPs of `a − b` when each table's result
/// is independently the playout outcome with probability `λ(level)` and the
/// lead-endpoint outcome otherwise.
///
/// The mixture is taken at the IMP level — four `imps` evaluations, one per
/// endpoint pairing — because the IMP table is nonlinear: blending scores
/// first would price an outcome no table ever produces.  Independence across
/// the two tables is the honest default for divergent boards (different
/// contracts, usually different declarers guessing different suits); the
/// same-contract case never reaches here (zero swing by construction).
#[allow(clippy::cast_precision_loss)]
pub fn sd_blend_imps(a: &SdScores, b: &SdScores, k: usize) -> f64 {
    let (la, lb) = (
        SD_BLEND_LAMBDA[usize::from(a.level)],
        SD_BLEND_LAMBDA[usize::from(b.level)],
    );
    let term = |x: i64, y: i64| imps(x - y) as f64;
    la * lb * term(a.playout[k], b.playout[k])
        + la * (1.0 - lb) * term(a.playout[k], b.lead[k])
        + (1.0 - la) * lb * term(a.lead[k], b.playout[k])
        + (1.0 - la) * (1.0 - lb) * term(a.lead[k], b.lead[k])
}

/// [`mean_with_ci`] for fractional per-board values (sd-blend swings are
/// λ-mixtures, so they are not integers)
#[allow(clippy::cast_precision_loss)]
pub fn mean_with_ci_f64(values: &[f64]) -> (f64, f64) {
    let n = values.len();
    if n < 2 {
        return (values.first().copied().unwrap_or(0.0), f64::INFINITY);
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let variance = values
        .iter()
        .map(|&v| {
            let d = v - mean;
            d * d
        })
        .sum::<f64>()
        / (n - 1) as f64;
    (mean, 1.96 * (variance / n as f64).sqrt())
}

/// The non-inferiority margin δ, in IMPs per **unconditional** board
///
/// Fixed in advance (2026-10-14, [#7](https://github.com/jdh8/pons/issues/7));
/// never tune it after seeing a result.  See `docs/measurement.md` § The
/// decision table.
pub const NI_MARGIN: f64 = 0.002;

/// Read one scorer's paired swing against [`NI_MARGIN`]
///
/// `mean ± half_width` is the 95% CI ([`mean_with_ci`]) of ON − OFF IMPs per
/// board over the measured population; `density` is that population's share of
/// random deals (1 unconditional, accepted/drawn for an enriched run), so the
/// margin per measured board is `δ / density`.  The lower bound is the
/// one-sided 97.5% bound.  A CI-clear negative reads `loss` even inside δ.
/// A CI with no finite width (under two boards, [`mean_with_ci`]) or a `NaN`
/// reads `inconclusive`: it rules nothing out.
///
/// # Panics
///
/// If `density` is not in `(0, 1]` — `0` would make the margin infinite and
/// prove anything non-inferior.  [`parse_density`] rejects it at the CLI.
pub fn verdict(mean: f64, half_width: f64, density: f64) -> &'static str {
    assert!(
        density > 0.0 && density <= 1.0,
        "density {density} is not a share of random deals in (0, 1]",
    );
    let lower = mean - half_width;
    if mean + half_width < 0.0 {
        "loss"
    } else if lower > 0.0 {
        "win"
    } else if lower > -NI_MARGIN / density {
        "non-inferior"
    } else {
        "inconclusive"
    }
}

/// Parse a `--density` flag: a share of random deals, finite and in `(0, 1]`
pub fn parse_density(s: &str) -> Result<f64, String> {
    let density: f64 = s.parse().map_err(|e| format!("{e}"))?;
    if density > 0.0 && density <= 1.0 {
        Ok(density)
    } else {
        Err(format!("{s} is not a share of random deals in (0, 1]"))
    }
}

/// The `density` [`verdict`] needs for `dump`: `explicit` if given, else 1 for
/// an unfiltered dump and `None` (withhold the verdict) for a `--filter-*` one,
/// whose acceptance rate the dump does not record
pub fn dump_density(dump: &Dump, explicit: Option<f64>) -> Option<f64> {
    explicit.or_else(|| (!dump.gen_args.iter().any(|a| a.starts_with("--filter"))).then_some(1.0))
}

/// The verdict line every A/B reporter prints under its delta
///
/// Names the scorer, because a verdict holds for that scorer only: a PD
/// `non-inferior` is no proof for plain DD.
pub fn verdict_line(scorer: &str, mean: f64, half_width: f64, density: Option<f64>) -> String {
    match density {
        Some(density) => format!(
            "Verdict ({scorer}, δ {NI_MARGIN} IMPs/board, density {density}): {}",
            verdict(mean, half_width, density),
        ),
        None => format!("Verdict ({scorer}): withheld — enriched dump, pass --density"),
    }
}

/// Print one scorer's whole-experiment line and its [`verdict_line`]
///
/// `board_imps` is ON − OFF per measured board, zeros included, so its mean is
/// IMPs per board over the population `density` describes (1 for random
/// deals).  `divergent` is the per-divergent figure's denominator.
#[allow(clippy::cast_precision_loss)]
pub fn report_swings(scorer: &str, board_imps: &[i64], divergent: usize, density: Option<f64>) {
    let (mean, ci) = mean_with_ci(board_imps);
    let total: i64 = board_imps.iter().sum();
    println!(
        "{scorer:>15}: {total:+} IMPs — {mean:+.4} ± {ci:.4} IMPs/board, {:+.3} IMPs/divergent",
        total as f64 / divergent.max(1) as f64,
    );
    println!("{}", verdict_line(scorer, mean, ci, density));
}

/// The outcome of scoring a board set against itself.
pub struct Scored {
    /// Indices (into the input) of boards whose two tables reached different
    /// contracts — the only ones that can swing.
    pub divergent: Vec<usize>,
    /// Per-board IMP swing, 0 for non-divergent boards (for the mean and its CI).
    pub board_imps: Vec<i64>,
    /// Per divergent board: `(index, point swing, IMP swing)`, for the dump.
    pub swings: Vec<(usize, i64, i64)>,
    pub total_points: i64,
    pub total_imps: i64,
    /// DD trick tables of the divergent boards, parallel to `divergent` — kept so
    /// callers can run further per-board analysis (e.g. a counterfactual line).
    pub tables: Vec<TrickCountTable>,
}

/// A reached contract and its declarer, or `None` for a pass-out — what
/// `final_contract` yields and what a `scorer` prices.
pub type Reached = Option<(Contract, Seat)>;

/// Solve the divergent boards double dummy and score each table's contract with
/// `scorer`, crediting the swing to the pair sitting NS at table A / EW at table
/// B.  This is the shared core of every A/B / BBA duplicate harness; only
/// `scorer` varies — `ns_score_contract` for plain DD, a perfect-defense closure
/// for PD.  `deals[i]` must be the deal of `contracts[i]`.
pub fn score_boards(
    contracts: &[(Reached, Reached)],
    deals: &[FullDeal],
    vul: AbsoluteVulnerability,
    scorer: impl Fn(Reached, &TrickCountTable, AbsoluteVulnerability) -> i64,
) -> Scored {
    let (divergent, tables) = solve_divergent(contracts, deals);
    score_solved(contracts, divergent, tables, vul, scorer)
}

/// Pick the boards whose two tables reached different contracts and solve just
/// those double dummy, on the **main thread** — `Solver::lock` takes a global
/// lock and fans out over the whole core pool internally, so it must never be
/// called from a rayon worker.
///
/// Split out of [`score_boards`] so a caller that wants the same boards under
/// more than one scorer pays for one solve instead of one per scorer.
///
/// Generic over what a side carries: a [`Reached`] for a direct A/B, an
/// [`ArmVsReference`] for a [`score_reference_solved`] gain — a board then
/// diverges when either its measured contract or its reference moved.
pub fn solve_divergent<T: PartialEq>(
    contracts: &[(T, T)],
    deals: &[FullDeal],
) -> (Vec<usize>, Vec<TrickCountTable>) {
    let divergent = divergent_boards(contracts);
    let solve: Vec<FullDeal> = divergent.iter().map(|&index| deals[index]).collect();
    let tables = pons::dd::solver().solve_deals(&solve, NonEmptyStrainFlags::ALL);
    (divergent, tables)
}

/// The boards whose two sides differ — the only ones [`solve_divergent`] solves
pub fn divergent_boards<T: PartialEq>(contracts: &[(T, T)]) -> Vec<usize> {
    (0..contracts.len())
        .filter(|&index| contracts[index].0 != contracts[index].1)
        .collect()
}

/// Price an already-solved divergent set with one `scorer`
///
/// `divergent` and `tables` are what [`solve_divergent`] returned, parallel to
/// each other.  Call it twice on clones of the same pair to read the plain-DD
/// and perfect-defense brackets off a single solve.
pub fn score_solved(
    contracts: &[(Reached, Reached)],
    divergent: Vec<usize>,
    tables: Vec<TrickCountTable>,
    vul: AbsoluteVulnerability,
    scorer: impl Fn(Reached, &TrickCountTable, AbsoluteVulnerability) -> i64,
) -> Scored {
    tally(contracts.len(), divergent, tables, |index, table| {
        let (a, b) = contracts[index];
        let (a, b) = (scorer(a, table, vul), scorer(b, table, vul));
        (a - b, direct_imps(a, b))
    })
}

/// **Direct A/B**, one board: `IMP(S_on − S_off)`
///
/// The two arms' scores IMPed against each other — the head-to-head that
/// every ship verdict reads.  It is *not* a gain against a reference: a
/// reference shared by both arms cancels in the raw points `S_on − S_off`,
/// but not across the nonlinear IMP table, so this differs from
/// [`reference_gain_imps`] even when the reference is fixed.
pub fn direct_imps(on: i64, off: i64) -> i64 {
    imps(on - off)
}

/// **Reference-relative IMP gain**, one board: `IMP(S_on − R_on) − IMP(S_off − R_off)`
///
/// Each arm is IMPed against its own reference *first*, then the two IMP
/// results are differenced — the move of a duplicate score against the
/// reference, which is what an anchor window measures.  With a fixed
/// reference (`R_on = R_off`) it is the gain against that reference; it is
/// never [`direct_imps`] in general (`S_on = 420`, `S_off = 400`, `R = 0`:
/// direct +1, gain 0).
pub fn reference_gain_imps(on: i64, on_reference: i64, off: i64, off_reference: i64) -> i64 {
    imps(on - on_reference) - imps(off - off_reference)
}

/// One arm's side of a board in a reference-relative comparison: the measured
/// contract, then the reference contract it is IMPed against
pub type ArmVsReference = (Reached, Reached);

/// Price an already-solved divergent set as a [`reference_gain_imps`] gain
///
/// `contracts[i]` is `(on, off)`, each an [`ArmVsReference`]; `divergent`
/// and `tables` come from [`solve_divergent`] on the same pairs, so a board
/// whose two sides agree — measured contract and reference both — scores an
/// exact 0 unsolved.  Every contract of a board is priced by the one `scorer`
/// on the one table, so the plain and PD gains share a solve and a reference
/// definition.  The `Scored` swing points are `(S_on − R_on) − (S_off − R_off)`.
pub fn score_reference_solved(
    contracts: &[(ArmVsReference, ArmVsReference)],
    divergent: Vec<usize>,
    tables: Vec<TrickCountTable>,
    vul: AbsoluteVulnerability,
    scorer: impl Fn(Reached, &TrickCountTable, AbsoluteVulnerability) -> i64,
) -> Scored {
    tally(contracts.len(), divergent, tables, |index, table| {
        let ((on, on_reference), (off, off_reference)) = contracts[index];
        let [on, on_reference, off, off_reference] =
            [on, on_reference, off, off_reference].map(|reached| scorer(reached, table, vul));
        (
            (on - on_reference) - (off - off_reference),
            reference_gain_imps(on, on_reference, off, off_reference),
        )
    })
}

/// Fold per-board `(points, IMPs)` swings over a solved divergent set; every
/// other board of the `count` scores 0
fn tally(
    count: usize,
    divergent: Vec<usize>,
    tables: Vec<TrickCountTable>,
    swing: impl Fn(usize, &TrickCountTable) -> (i64, i64),
) -> Scored {
    let mut total_points = 0i64;
    let mut board_imps = vec![0i64; count];
    let mut swings: Vec<(usize, i64, i64)> = Vec::with_capacity(divergent.len());
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (points, imp) = swing(index, table);
        total_points += points;
        board_imps[index] = imp;
        swings.push((index, points, imp));
    }
    let total_imps = board_imps.iter().sum();
    Scored {
        divergent,
        board_imps,
        swings,
        total_points,
        total_imps,
        tables,
    }
}

/// Print the measurement playbook's dual bracket for a divergent-only solved
/// A/B: the swing scored **both** ways — plain DD (`ns_score_contract`, the
/// contract's actual penalty) and perfect defense (`ns_score_bid`, a failing
/// contract priced as doubled).  `contracts[i]` is `[off, on]`, and `tables`
/// are the solved divergent boards, parallel to `divergent`.  Each line reports
/// total IMPs, IMPs/board (over all `count`) and IMPs/divergent, with 95% CIs.
///
/// Unlike [`score_boards`], this scores twice from one solve — the harness
/// solves the divergent set once and reads both brackets off the same tables.
/// Each line is followed by its [`verdict_line`] at density 1: every caller
/// deals unconditionally.
pub fn report_brackets(
    count: usize,
    divergent: &[usize],
    tables: &[TrickCountTable],
    contracts: &[[Option<(Contract, Seat)>; 2]],
    vul: AbsoluteVulnerability,
) {
    for (label, scorer) in [
        ("plain DD", ns_score_contract as fn(_, _, _) -> i64),
        ("perfect defense", ns_score_bid),
    ] {
        let mut per_board = vec![0i64; count];
        for (&i, table) in divergent.iter().zip(tables.iter()) {
            let off = scorer(contracts[i][0], table, vul);
            let on = scorer(contracts[i][1], table, vul);
            per_board[i] = imps(on - off);
        }
        let fired: Vec<i64> = divergent.iter().map(|&i| per_board[i]).collect();
        let (per_board_mean, per_board_ci) = mean_with_ci(&per_board);
        let (fired_mean, fired_ci) = mean_with_ci(&fired);
        println!(
            "{label:>15}: {:+} IMPs — {per_board_mean:+.4} ± {per_board_ci:.4} IMPs/board, \
             {fired_mean:+.3} ± {fired_ci:.3} IMPs/divergent",
            fired.iter().sum::<i64>(),
        );
        println!(
            "{}",
            verdict_line(label, per_board_mean, per_board_ci, Some(1.0))
        );
    }
}

/// The single-dummy sibling of [`report_brackets`]: one trick count per board,
/// reported under **both** SD brackets — plain (`ns_score_tricks`, the
/// contract's own penalty) and perfect defense (`ns_score_pd_tricks`, a
/// contract failing on those tricks priced doubled).
///
/// `on[i]` and `off[i]` are that board's `[plain, perfect-defense]` pair, as
/// produced beside every `single_dummy_leads` answer; the swing is `on − off`
/// within each bracket.  `divergent` is the divergent-board count, the
/// denominator of the per-divergent figure; `density` is the boards' share of
/// random deals, for each bracket's [`verdict_line`].
///
/// **Read the perfect-defense line as the arbiter, not the plain one.** Plain
/// SD relaxes the defenders' opening lead *and* never punishes the failures —
/// optimistic on both axes, so it flatters aggression; this pairs the
/// realistic lead with the doubled downside a real opponent exacts.
pub fn report_sd_brackets(
    label: &str,
    worlds: usize,
    seed: u64,
    on: &[[i64; 2]],
    off: &[[i64; 2]],
    divergent: usize,
    density: f64,
) {
    for (k, bracket) in ["plain          ", "perfect-defense"].iter().enumerate() {
        let board_imps: Vec<i64> = on.iter().zip(off).map(|(a, b)| imps(a[k] - b[k])).collect();
        let (mean, ci) = mean_with_ci(&board_imps);
        let total: i64 = board_imps.iter().sum();
        println!(
            "{label} {bracket} ({worlds} worlds, seed {seed}): {total:+} IMPs, \
             {mean:+.4} IMPs/board [95% CI ±{ci:.4}], {:+.3} IMPs/divergent",
            total as f64 / divergent.max(1) as f64,
        );
        let scorer = format!("{label} {}", bracket.trim_end());
        println!("{}", verdict_line(&scorer, mean, ci, Some(density)));
    }
}

/// Build one of our authored books by name, as `--our-floor`/`--their-floor`
/// spell it
///
/// Takes the agreements as a value, so a caller wanting a *perturbed* book
/// receives the complete [`Agreements`] value for the system being described,
/// edits the fields it should carry, and passes the result — two
/// differently-configured books can then coexist on one thread.
pub fn seat_floor(name: &str, agreements: &Agreements) -> anyhow::Result<Partnership> {
    Ok(match name {
        "american" => pons::american(agreements).bind(),
        // The authored book with no floor at all: a driver seating this passes
        // whenever the book runs out.  The floor ablation's other end.
        "american-book" => pons::bidding::american::american_book(agreements).bind(),
        // The deterministic pre-swap floor: the fixed baseline now that
        // `american` ships the BBA net.
        "american-instinct" => pons::american_instinct(agreements).bind(),
        // The book ablation: no authored book at all, the same floor wiring
        // `american` uses.  `american` − `american-floor` prices the book.
        "american-floor" => pons::american_floor(agreements).bind(),
        "american-v6" => pons::bidding::american::american_v6(agreements).bind(),
        "american-v6-their" => pons::bidding::american::american_v6_their(agreements).bind(),
        "american-v8" => pons::bidding::american::american_v8(agreements).bind(),
        // The floor sweep's candidate: a logit mean over the v6 blobs listed
        // in `PONS_FLOOR_WEIGHTS` (docs/archive/floor-sweep.md).
        "american-file" => {
            pons::bidding::american::american_mean(agreements, floor_blobs()?).bind()
        }
        // BBA's walked book as a mask over our floor (docs/bba-system.md):
        // the dump from `BBA_BOOK`, the net choosing at every walked node
        // (`bba`) or the floor partition's own owner (`bba-ladder`).
        #[cfg(feature = "serde")]
        "bba" | "bba-ladder" => {
            let chooser = bba_book::Chooser::parse(name).expect("matched above");
            bba_book::BbaBook::shared()?
                .system(agreements, chooser, bba_book::Mask::from_env())
                .bind()
        }
        #[cfg(not(feature = "serde"))]
        "bba" | "bba-ladder" => anyhow::bail!("the {name} floor needs --features serde"),
        other => anyhow::bail!(
            "floor must be american|american-book|american-instinct|american-floor|american-v6|american-v6-their|american-v8|american-file|bba|bba-ladder, got {other:?}"
        ),
    })
}

/// The comma-separated `.f32` blobs in `PONS_FLOOR_WEIGHTS`, decoded once
fn floor_blobs() -> anyhow::Result<std::sync::Arc<[Vec<f32>]>> {
    static BLOBS: std::sync::OnceLock<std::sync::Arc<[Vec<f32>]>> = std::sync::OnceLock::new();
    if let Some(blobs) = BLOBS.get() {
        return Ok(blobs.clone());
    }
    let paths = std::env::var("PONS_FLOOR_WEIGHTS")
        .map_err(|_| anyhow::anyhow!("american-file needs PONS_FLOOR_WEIGHTS=a.f32,b.f32,..."))?;
    let mut blobs = Vec::new();
    for path in paths.split(',') {
        let blob = pons::bidding::neural::decode(&std::fs::read(path)?);
        anyhow::ensure!(
            pons::bidding::neural::blob_shape(blob.len()).is_some(),
            "{path}: {} floats is no blob of inputs {:?} and width {:?}",
            blob.len(),
            pons::bidding::neural::BLOB_INPUTS,
            pons::bidding::neural::V6_WIDTHS
        );
        blobs.push(blob);
    }
    Ok(BLOBS.get_or_init(|| blobs.into()).clone())
}

/// The card a `--our-floor` / `--their-floor` name declares
///
/// Split on `-` like `bba-gen`'s `disclosure`: the `-instinct` / `-book` /
/// `-floor` variants differ only in the floor, which no card row expresses, so
/// they all declare their base system's card.
pub fn floor_card(name: &str, agreements: &Agreements) -> anyhow::Result<Card> {
    Ok(match name.split('-').next().unwrap_or_default() {
        "american" => american_card(agreements),
        // The `bba` pair plays EPBot's own defaults, which is what its card
        // says: no row moved (docs/bba-system.md, D5).
        "bba" => bba_book::card(),
        other => anyhow::bail!(
            "no card generator for system `{other}` (known: american, bba).  \
             Write one in `src/bidding/card.rs` rather than declaring another \
             system's card."
        ),
    })
}

/// [`seat_floor`], but with the opponents **declared** to the net
///
/// The books are identical — this moves only the net's regime input, whose
/// `theirs` half every other entry point fills with our own agreements
/// (`symmetric`).  Phase 2a of docs/declarative-rows.md: the floor channel
/// alone, so a measurement cannot confound a mixed net input with mixed book
/// rows.
///
/// **The name must reach the same net here as in [`seat_floor`]**, or the arms
/// of a `--declare-opponents` A/B measure the net swap rather than the
/// declaration — which is exactly what happened for one commit after the
/// 2026-08-08 v5 ship, at 3.5× the historical `ab-declared-opponents.sh`'s own
/// CI.  So the shipped name uses the honest-reading v6 floor on both paths.
///
/// Our own half rides `agreements`, captured by the caller once every `--ns-*`
/// cell is armed — the same "set every `--ns-*` first" rule that applies to
/// `bba-gen`'s `disclosure()`.  Theirs is projected
/// off the card with
/// [`ConventionCard::from_card`][pons::bidding::features::ConventionCard::from_card],
/// the only channel a foreign engine's card has; it is lossy exactly where that
/// function documents — the wide 1NT rungs collapse upward and a defense that is
/// neither Multi-Landy nor Landy reads as `Natural` — so a `--their-ns` arm on
/// one of those axes declares an approximation, not the row it armed.
///
/// Only the net-floored system accepts a declared opponent.  The other
/// floors are refused rather than silently ignored: an arm that quietly kept a
/// symmetric config would be incomparable to its sibling with nothing in the
/// output saying so.
pub fn seat_floor_vs(
    name: &str,
    theirs: &Card,
    agreements: &Agreements,
) -> anyhow::Result<Partnership> {
    Ok(match name {
        "american" => {
            pons::american_with_card(agreements, &ConventionCard::from_card(theirs)).bind()
        }
        other => anyhow::bail!(
            "--declare-opponents needs a net floor to declare them to: \
             floor must be american, got {other:?}"
        ),
    })
}

/// Apply the deviation-panel knobs, build a book under them, and reset
///
/// The B/C axes of docs/deviation-panel.md: `dial` is the antisymmetric
/// strength dial, the three flags are the shape-indiscipline knobs.  Only the
/// returned book carries them.
///
/// A deviation is *theirs alone*, so `theirs` — the card played by the seat
/// this book faces — is passed in from outside the transaction, while its own
/// half is read inside [`seat_floor_vs`].  [`seat_floor`] would have taken a
/// symmetric config and told this net the whole table deviates — a
/// misdisclosure the moment a deviation knob moves a disclosed row, which
/// `offshape_1nt` does.  The other three move no row, so with `theirs` equal to
/// this book's own card this is byte-identical.
///
/// Since the constant-input folds that honesty is **inert at the net on both
/// floors**: `one_notrump_offshape` is compact slot 12 under v5 and the
/// `1NT opening shape 4441` card column under v4, and both are folded to zero
/// (`docs/ai-bidder/card-manifold.md`).  The routing stays because honest
/// disclosure is the invariant, not because it currently moves a logit.
///
/// `base` supplies the [`OpeningKnobs`][pons::bidding::agreements::OpeningKnobs]
/// this seat was armed with — the two deviations that live there are written on
/// top of it, as is `overcall_four_card`, now a field of the value.  Everything
/// else comes from the current agreements snapshot.
pub fn deviant_floor(
    name: &str,
    theirs: &Card,
    base: &Agreements,
    dial: u8,
    overcall_four_card: bool,
    offshape_1nt: bool,
    wild_weak_two: bool,
) -> anyhow::Result<Partnership> {
    let mut agreements = Agreements::default();
    agreements.decision.reading.strength_dial = dial;
    agreements.opening = base.opening;
    agreements.opening.one_notrump_offshape = offshape_1nt;
    agreements.opening.weak_two_wild = wild_weak_two;
    agreements.defense.overcall_four_card = overcall_four_card;
    // Only the net-floored names take a config; the instinct and book-only
    // floors have no net to declare anything to.
    match name {
        "american" => seat_floor_vs(name, theirs, &agreements),
        _ => seat_floor(name, &agreements),
    }
}

/// A copy of `partnership` that reads with the *opponents'* readings blanked — the
/// `blind` arm of the deviation panel.
///
/// The setting is pinned into a partnership when it is built, so one side of the
/// table can go blind while a pons book on the *other* side keeps its readings:
/// the two partnerships hold different pins.  [`Partnership::profile_mut`] moves this
/// partnership's copy without rebuilding the book underneath, and without touching
/// any state the caller shares.
pub fn blinded(partnership: &Partnership) -> Partnership {
    let mut out = partnership.clone();
    out.profile_mut().reading.blind_opponents = true;
    out
}

/// Declare the opponents as our own system under `agreements` except for one
/// axis, their `1NT` minor-suit responses, which play the **European** scheme
/// (`2♠` clubs transfer, `2NT` invitation, `3♣` diamonds transfer)
///
/// EPBot's stock system-0 and BEN's declared card (`vendor/ben/BEN-21GF.bbsa`:
/// `1N-2S transfer to clubs=1`, `1N-3C transfer to diamonds=1`, `1N-3C Puppet
/// Stayman=0`) both play it, so reading their `(1NT) - (2NT)` as our Puppet
/// diamond transfer is simply false.  Only that axis: declaring a whole foreign
/// system is the Phase-2b treatment, which measured plain wash / PD −0.0070
/// (docs/declarative-rows.md §2b).  Reading only
/// ([`Partnership::with_opponents`]); our bidding table and the net's card
/// inputs are untouched.  `ben-gen` always reads BEN this way, and `bba-gen`
/// reads EPBot this way by default since 2026-10-20 (`--no-declare-their-
/// european-minors` opts out); each dump records it
/// ([`Dump::declared_european_minors`]) and `ab-dump-sd` reads each arm to
/// match.
///
/// Their **defense** is the shipped default, never `agreements.defense`: the
/// harnesses' `--ns-*` / `--on-ns-*` defense knobs move only our book, never
/// the one we read the opponent's calls off.  Copying them leaked the arm into
/// our reading of `1NT (2♥)` (2026-10-19: `ben-gen
/// --ns-nt-overcall-hcp-floor 8` vs `0`, 40 foreign boards at the gate).  The
/// same holds for the defensive systems that live in `decision.reading`
/// ([`ReadingProfile::with_house_defense`][pons::bidding::ReadingProfile::with_house_defense]):
/// copying `notrump_defense` read their calls over *our* 1NT as Woolsey
/// whenever we played Woolsey (found 2026-10-20).
pub fn european_minors_opponents(ours: Partnership, agreements: &Agreements) -> Partnership {
    let mut theirs = *agreements;
    theirs.decision.reading.notrump_minors = pons::bidding::american::EUROPEAN;
    theirs.defense = Agreements::default().defense;
    theirs.decision.reading = theirs.decision.reading.with_house_defense();
    ours.with_opponents(&pons::american(&theirs).bind())
}

/// CLI face of [`pons::bidding::american::NotrumpDefense`]
///
/// clap's `ValueEnum` cannot be derived for a foreign type, so every harness
/// that lets the operator pick a 1NT defense needs one of these. Shared here
/// rather than copied per binary: the engine holds the systems in a single
/// `Cell<NotrumpDefense>`, and one CLI enum per binary is how a harness ends up
/// re-inventing the read-time precedence cascade that cell exists to delete.
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum NtDefenseArg {
    Natural,
    DirectDont,
    Meckwell,
    Woolsey,
    DirectLandy,
    AlwaysPass,
    Off,
}

impl From<NtDefenseArg> for pons::bidding::american::NotrumpDefense {
    fn from(arg: NtDefenseArg) -> Self {
        match arg {
            NtDefenseArg::Natural => Self::Natural,
            NtDefenseArg::DirectDont => Self::DirectDont,
            NtDefenseArg::Meckwell => Self::Meckwell,
            NtDefenseArg::Woolsey => Self::Woolsey,
            NtDefenseArg::DirectLandy => Self::DirectLandy,
            NtDefenseArg::AlwaysPass => Self::AlwaysPass,
            NtDefenseArg::Off => Self::Off,
        }
    }
}

/// Field `family` as our 1NT defense, the card `bba-gen --ns-notrump-defense`
/// plays: the family itself, the unusual `2NT` on DONT and Meckwell's `8..=14`
/// band (`8..=13`, the crate default, elsewhere), and DONT's one-suiter
/// minimum.  Shared so that every harness that bids or reads an arm fields the
/// same card; `ben-gen` and `ab-dump-sd` once set the family alone, so a DONT
/// or Meckwell arm there played a narrower `2NT` than the same arm vs BBA.
pub fn set_notrump_defense(
    agreements: &mut Agreements,
    family: NtDefenseArg,
    dont_one_suiter_min: u8,
) {
    use pons::bidding::american::NotrumpDefense;
    let family = NotrumpDefense::from(family);
    agreements.decision.reading.notrump_defense = family;
    let wide = matches!(
        family,
        NotrumpDefense::DirectDont | NotrumpDefense::Meckwell
    );
    agreements.defense.unusual_notrump_range = Some(if wide { (8, 14) } else { (8, 13) });
    if family == NotrumpDefense::DirectDont {
        agreements.defense.direct_dont_one_suiter_min = dont_one_suiter_min;
    }
}

/// CLI face of [`pons::bidding::american::XxMajorFirst`], same reason as
/// [`NtDefenseArg`]
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum XxMajorFirstArg {
    Off,
    Longer,
    AtLeastAsLong,
}

impl From<XxMajorFirstArg> for pons::bidding::american::XxMajorFirst {
    fn from(arg: XxMajorFirstArg) -> Self {
        match arg {
            XxMajorFirstArg::Off => Self::Off,
            XxMajorFirstArg::Longer => Self::Longer,
            XxMajorFirstArg::AtLeastAsLong => Self::AtLeastAsLong,
        }
    }
}

/// CLI face of [`pons::bidding::ReadingScope`], same reason as [`NtDefenseArg`]
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ReadingScopeArg {
    /// Decode nothing authored — the natural walk alone
    None,
    /// Decode alerted calls only (the shipped default)
    Alerted,
    /// Decode every authored call, unalerted ones too (the held Phase-2 arm)
    All,
}

impl From<ReadingScopeArg> for pons::bidding::ReadingScope {
    fn from(arg: ReadingScopeArg) -> Self {
        match arg {
            ReadingScopeArg::None => Self::None,
            ReadingScopeArg::Alerted => Self::Alerted,
            ReadingScopeArg::All => Self::All,
        }
    }
}
