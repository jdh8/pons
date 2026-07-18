//! The **BEN gap campaign**'s generation half — bids a duplicate A/B match of
//! our [`american`] floor against **BEN** (lorserker/ben, pinned v0.8.8.4)
//! over its REST `/bid` endpoint, writing the same `Dump` every downstream
//! consumer of `bba-gen` already reads (`bba-score`, `ab-dump-diff`,
//! `ab-dump-sd`, `bba-decompose`).  See `docs/ben-gen-design.md` for the
//! protocol facts and `docs/ben-gap-campaign.md` for the campaign.
//!
//! Start servers first (`scripts/ben-servers.sh start N`); one ben-gen process
//! per server instance, mirroring `bba-gen-parallel.sh`'s process sharding:
//!
//! ```text
//! cargo run --release --features serde --example ben-gen -- \
//!   --count 100 --port 8085 --tier f -o boards.json
//! ```
//!
//! `--calibrate-epbot` seats the vendored EPBot (via `common::oracle`) at
//! *our* chairs instead of pons — zero pons code in the loop — to reproduce
//! BBA's published EPBot-vs-BEN Table 1 row as the harness validation.
//!
//! Error policy: any transport error, non-200, or response without a `bid`
//! key retries 3× with backoff and then **aborts the shard loudly** — never a
//! silent Pass, which would bias the measurement.  `"Bidding is over"` or an
//! `{"error"}` reply means our auction desynced from BEN's view: abort at
//! once, no retry.

use clap::Parser;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, Hand, Level, Seat, Strain, Suit};
use pons::american;
use pons::bidding::array::Logits;
use pons::bidding::{Family, System};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::oracle::{BbaOracle, DEFAULT_LIB, SYSTEM_2_OVER_1, bid_out, one_hot};
use common::{Board, Dump};

/// The pinned BEN release the servers must be running (recorded in labels;
/// re-pinning is a campaign decision — see docs/ben-gap-campaign.md).
const BEN_TAG: &str = "v0.8.8.4";

/// Bid our 2/1 floor against BEN's 21GF card over REST and write the boards
/// (the generation half of the A/B duplicate match; `bba-score` scores them)
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "100")]
    count: usize,

    /// Write the bid boards as JSON here; default is stdout
    #[arg(short, long)]
    output: Option<String>,

    /// Vulnerability the boards are bid at: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal-stream seed; unset = random (print it via gen_args for replays)
    #[arg(long)]
    seed: Option<u64>,

    /// Port of the BEN server instance this shard talks to
    #[arg(short, long, default_value_t = 8085)]
    port: u16,

    /// Which server tier the port is running: `f` (fast, pure policy) or `s`
    /// (strong, stock search).  Label-only — the tier really lives in the
    /// server's config, so this MUST match what `ben-servers.sh` started.
    #[arg(short, long, default_value = "f")]
    tier: String,

    /// Seat the vendored EPBot at our chairs instead of pons (harness
    /// validation vs BBA's published EPBot-vs-BEN table; no pons in the loop)
    #[arg(long, default_value_t = false)]
    calibrate_epbot: bool,

    /// Disable the cue reading of the natural walk (shipped default-on
    /// 2026-07-18, bid-inert): a bid of a suit only the opponents have
    /// naturally shown is a cue, never a holding.
    #[arg(long, default_value_t = false)]
    no_ns_cue_reading: bool,

    /// Disable sound natural length floors (shipped default-on 2026-07-18:
    /// plain wash + PD win on both references): opener's immediate two-level
    /// rebid of the opened suit reads 5+ not 6+, an agreed-suit re-raise adds
    /// no length, and a doubler's later jump is never a weak six-card jump.
    #[arg(long, default_value_t = false)]
    no_ns_length_soundness: bool,

    /// Disable table-wide alert reading (shipped default-on 2026-07-18,
    /// bid-inert): the opponents' alerted calls decode off their authoring
    /// rules — modeling them as playing our books, an approximation against
    /// BEN — instead of falling to the natural walk.
    #[arg(long, default_value_t = false)]
    no_ns_table_alert_reading: bool,

    /// Disable the pass reading (shipped default-on 2026-07-18, bid-inert):
    /// each pass at an authored node reads as its table's own Pass gate — the
    /// negative inference of declining every other call (no-open ≤ 11 points,
    /// silent responder ≤ 5 HCP, direct seat ≤ 17 HCP).  Opponents' passes
    /// also need table-wide alert reading on.
    #[arg(long, default_value_t = false)]
    no_ns_pass_reading: bool,

    /// Free-form provenance recorded in gen_args (the launcher passes the
    /// server conf's sha256 here)
    #[arg(long)]
    note: Option<String>,

    /// Distillation mode: instead of the A/B match, seat BEN at all four
    /// chairs and write a `probe-brl-book` self-play corpus (JSONL) here.
    /// `--count` deals, each bid under all four vulnerabilities, dealer fixed
    /// North.  Tier F is a deterministic argmax, so the corpus `top3`/`ent`
    /// are synthetic (one fully-confident candidate, zero entropy) — the
    /// confident-row and entropy columns of the probe are not meaningful.
    #[arg(long)]
    self_play: Option<String>,

    /// Self-play corpus: base value for the `deal` field (shard offset).  Each
    /// fleet shard passes a distinct `--first-deal i*count` so `deal` ids stay
    /// globally unique — the probe pairs vul-flips and splits train/test by it.
    #[arg(long, default_value_t = 0)]
    first_deal: usize,
}

// ---------------------------------------------------------------------------
// Wire encoding (kept together so a BEN dialect change is a local edit;
// tokens confirmed live at v0.8.8.4 — docs/ben-gen-design.md "Wire protocol")
// ---------------------------------------------------------------------------

/// BEN suit letters in [`Strain`] discriminant order (♣ ♦ ♥ ♠ NT)
const STRAIN_CHARS: [char; 5] = ['C', 'D', 'H', 'S', 'N'];

/// A call as a BEN `ctx` request token (`P`, `X`, `XX`, `1C`…`7N`)
fn call_token(call: Call) -> String {
    match call {
        Call::Pass => "P".into(),
        Call::Double => "X".into(),
        Call::Redouble => "XX".into(),
        Call::Bid(bid) => format!("{}{}", bid.level.get(), STRAIN_CHARS[bid.strain as usize]),
    }
}

/// A call as a `probe-brl-book` corpus token (`P`, `X`, `XX`, `1C`…`1NT`…`7NT`).
/// Same as [`call_token`] but notrump is `NT`, matching the probe's `TOKENS`.
fn corpus_token(call: Call) -> String {
    match call {
        Call::Pass => "P".into(),
        Call::Double => "X".into(),
        Call::Redouble => "XX".into(),
        Call::Bid(bid) => {
            let strain = ["C", "D", "H", "S", "NT"][bid.strain as usize];
            format!("{}{strain}", bid.level.get())
        }
    }
}

/// A BEN response `bid` token (`PASS`, `X`, `XX`, `1C`…`7N`) as a [`Call`]
fn parse_token(token: &str) -> Option<Call> {
    match token {
        "PASS" => Some(Call::Pass),
        "X" => Some(Call::Double),
        "XX" => Some(Call::Redouble),
        _ => {
            let mut chars = token.chars();
            let level = chars.next()?.to_digit(10).filter(|l| (1..=7).contains(l))?;
            let strain = match chars.next()? {
                'C' => Strain::Clubs,
                'D' => Strain::Diamonds,
                'H' => Strain::Hearts,
                'S' => Strain::Spades,
                'N' => Strain::Notrump,
                _ => return None,
            };
            #[allow(clippy::cast_possible_truncation)]
            Some(Call::Bid(Bid {
                level: Level::new(level as u8),
                strain,
            }))
        }
    }
}

/// The hand in BEN's PBN order (`S.H.D.C`, ranks high-to-low, `T` for ten)
fn hand_pbn(hand: Hand) -> String {
    use core::fmt::Write;
    let mut pbn = String::with_capacity(20);
    for (index, suit) in [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs]
        .into_iter()
        .enumerate()
    {
        if index > 0 {
            pbn.push('.');
        }
        write!(pbn, "{}", hand[suit]).expect("writing to a String never fails");
    }
    pbn
}

/// BEN's absolute `vul` value, dealer canonicalized to North.
///
/// The actor after `len` calls from a North dealer sits N,E,S,W cyclically, so
/// the actor's side is N/S iff `actor` is even — the same mapping
/// `common::oracle` uses for EPBot.  Relative vulnerability is preserved.
fn ben_vulnerability(vul: RelativeVulnerability, actor: usize) -> &'static str {
    let we = vul.contains(RelativeVulnerability::WE);
    let they = vul.contains(RelativeVulnerability::THEY);
    let (ns, ew) = if actor.is_multiple_of(2) {
        (we, they)
    } else {
        (they, we)
    };
    match (ns, ew) {
        (false, false) => "",
        (true, false) => "NS",
        (false, true) => "EW",
        (true, true) => "Both",
    }
}

// ---------------------------------------------------------------------------
// The BEN oracle: the REST `/bid` endpoint driven as a pons `System`
// ---------------------------------------------------------------------------

/// BEN behind pons's [`System`] trait — one blocking HTTP GET per call.
///
/// Like [`BbaOracle`], the dealer is canonicalized (to North); each `/bid`
/// request is stateless and, with a fixed server version + config + startup
/// seed, a pure function of its query string, so same seed ⇒ identical dump.
struct BenOracle {
    port: u16,
}

impl BenOracle {
    /// One `/bid` request; `Ok` is the raw JSON body of a 200 response.
    ///
    /// HTTP/1.0 keeps the reply un-chunked and EOF-delimited — no response
    /// framing to parse beyond the status line and the blank line.
    fn get(&self, query: &str) -> anyhow::Result<String> {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port))?;
        stream.set_read_timeout(Some(Duration::from_secs(600)))?;
        write!(stream, "GET {query} HTTP/1.0\r\nHost: localhost\r\n\r\n")?;
        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        let (head, body) = response
            .split_once("\r\n\r\n")
            .ok_or_else(|| anyhow::anyhow!("malformed HTTP response: {response:?}"))?;
        let status = head.lines().next().unwrap_or_default();
        anyhow::ensure!(
            status.split_whitespace().nth(1) == Some("200"),
            "BEN returned {status:?}: {body}"
        );
        Ok(body.to_string())
    }

    /// The bid for this request, retrying transport errors 3× with backoff.
    ///
    /// Panics (aborting the shard loudly) when retries are exhausted or the
    /// server's reply says our auction desynced — a silent Pass fallback
    /// would bias the measurement, and a shard is cheaply re-run by seed.
    fn bid(&self, query: &str) -> Call {
        let mut last = None;
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(Duration::from_secs(1 << attempt));
            }
            match self.get(query) {
                Err(error) => last = Some(error),
                Ok(body) => {
                    let json: serde_json::Value = match serde_json::from_str(&body) {
                        Ok(json) => json,
                        Err(error) => {
                            last = Some(error.into());
                            continue;
                        }
                    };
                    let Some(token) = json["bid"].as_str() else {
                        // "Bidding is over" / {"error": ...}: our loop and
                        // BEN disagree about the auction — never retry.
                        panic!("ben-gen desynced from BEN on {query}: {body}");
                    };
                    let Some(call) = parse_token(token) else {
                        panic!("ben-gen cannot parse BEN's bid {token:?} on {query}");
                    };
                    return call;
                }
            }
        }
        panic!(
            "ben-gen: BEN on port {} unreachable for {query}: {}",
            self.port,
            last.expect("three failed attempts leave an error")
        );
    }
}

impl System for BenOracle {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        let actor = auction.len() % 4;
        let seat = ["N", "E", "S", "W"][actor];
        let ctx = auction
            .iter()
            .map(|&call| call_token(call))
            .collect::<Vec<_>>()
            .join("-");
        let query = format!(
            "/bid?hand={}&seat={seat}&dealer=N&vul={}&ctx={ctx}",
            hand_pbn(hand),
            ben_vulnerability(vul, actor),
        );
        Some(one_hot(self.bid(&query)))
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    anyhow::ensure!(
        matches!(args.tier.as_str(), "f" | "s"),
        "--tier must be f or s"
    );
    // Classify-time inference knobs; the board loop stays on this thread.
    pons::bidding::set_cue_reading(!args.no_ns_cue_reading);
    pons::bidding::set_length_soundness(!args.no_ns_length_soundness);
    pons::bidding::set_table_alert_reading(!args.no_ns_table_alert_reading);
    pons::bidding::set_pass_reading(!args.no_ns_pass_reading);
    let ben = BenOracle { port: args.port };

    // Health-probe the server before dealing: a fixed opening-bid request.
    ben.bid("/bid?hand=AK97543.K.T3.AK7&seat=N&dealer=N&vul=&ctx=");

    // Distillation mode: BEN at all four chairs, each deal under all four
    // vulnerabilities (paired for the probe's vul-flip test), dealer fixed
    // North so the corpus `seats[0]` is the dealer.  One JSONL row per
    // (deal, vul) in probe-brl-book's schema; top3/ent synthetic (Tier F is a
    // deterministic argmax — see --self-play).
    if let Some(path) = args.self_play.as_deref() {
        let seed = args.seed.unwrap_or_else(rand::random);
        let mut rng = StdRng::seed_from_u64(seed);
        let vuls: [(&str, AbsoluteVulnerability); 4] = ["none", "ns", "ew", "both"]
            .map(|s| (s, s.parse().expect("valid vulnerability label")));
        let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
        for index in 0..args.count {
            let deal = full_deal(&mut rng);
            let pbn = format!(
                "N:{} {} {} {}",
                hand_pbn(deal[Seat::North]),
                hand_pbn(deal[Seat::East]),
                hand_pbn(deal[Seat::South]),
                hand_pbn(deal[Seat::West]),
            );
            for (label, vul) in vuls {
                let auction = bid_out(&ben, &ben, true, Seat::North, vul, &deal);
                let calls: Vec<String> = auction.iter().map(|&c| corpus_token(c)).collect();
                let top3: Vec<serde_json::Value> = calls
                    .iter()
                    .map(|t| serde_json::json!([[t.as_str(), 1.0]]))
                    .collect();
                let ent = vec![0.0_f64; calls.len()];
                let row = serde_json::json!({
                    "deal": args.first_deal + index, "vul": label, "pbn": pbn,
                    "calls": calls, "top3": top3, "ent": ent,
                });
                writeln!(out, "{row}")?;
            }
        }
        out.flush()?;
        eprintln!(
            "ben-gen: self-play corpus — {} deals × 4 vul = {} boards to {path} (seed {seed})",
            args.count,
            args.count * 4,
        );
        return Ok(());
    }

    let our_floor = american().against(Family::NATURAL);
    let epbot = if args.calibrate_epbot {
        let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
        Some(BbaOracle::load(&path, SYSTEM_2_OVER_1, Vec::new())?)
    } else {
        None
    };
    let (ours, our_label): (&dyn System, &str) = match &epbot {
        Some(oracle) => (oracle, "EPBot 2/1 (vendored)"),
        None => (&our_floor, "our american floor"),
    };
    let their_label = format!("BEN {BEN_TAG} 21GF/{}", args.tier.to_uppercase());

    let seed = args.seed.unwrap_or_else(rand::random);
    let mut rng = StdRng::seed_from_u64(seed);

    // Bid every board at both tables, dealer rotating per board.  Sequential
    // by design: the server serializes bids behind a lock anyway — parallelism
    // is one ben-gen process per server instance (ports 8085+i).
    let boards = (0..args.count)
        .map(|index| {
            let deal = full_deal(&mut rng);
            let dealer = Seat::ALL[index % 4];
            Board {
                table_a: bid_out(ours, &ben, true, dealer, args.vulnerability, &deal),
                table_b: bid_out(ours, &ben, false, dealer, args.vulnerability, &deal),
                deal,
                dealer,
            }
        })
        .collect();

    let dump = Dump {
        our_label: our_label.into(),
        their_label,
        vulnerability: args.vulnerability,
        seed: Some(seed),
        gen_args: std::env::args().skip(1).collect(),
        boards,
    };
    match args.output.as_deref() {
        Some(path) => {
            serde_json::to_writer(std::io::BufWriter::new(std::fs::File::create(path)?), &dump)?;
        }
        None => serde_json::to_writer(std::io::stdout().lock(), &dump)?,
    }
    eprintln!(
        "ben-gen: {} (us) vs {} (them), vulnerability {} — wrote {} boards{}",
        dump.our_label,
        dump.their_label,
        dump.vulnerability,
        dump.boards.len(),
        match args.output.as_deref() {
            Some(path) => format!(" to {path}"),
            None => " to stdout".into(),
        },
    );
    Ok(())
}
