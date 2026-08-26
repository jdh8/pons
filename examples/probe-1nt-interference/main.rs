//! Census: how much does each interference call over **our 1NT** cost us?
//!
//! The anchor report's ranked buckets stop at `Competitive / book / round-1` —
//! one number over every contested opening.  This splits the 1NT slice of that
//! blend by **RHO's call**, so the competitive-1NT campaign
//! ([docs/one-notrump-competitive.md](../../docs/one-notrump-competitive.md))
//! picks its next package from measured cost instead of from how wrong each
//! mismatch looks on paper.  It reads an existing anchor arm — no generation,
//! no new DD beyond the arm's own divergent set.
//!
//! Our pair sits NS at table A, so a board counts when `table_a` opens 1NT from
//! North or South and East/West act over it.  The bucket is that action: `X`,
//! `2♣`…`2NT`, one of the four three-level suits, or `4+` (`3NT` and above).
//! Both brackets are reported off one solve.
//!
//! ```text
//! cargo run --release --features serde --example probe-1nt-interference -- \
//!     ab-results/anchor/2026-08-12-ea2cde9-dirty/american-none \
//!     --dd-cache ab-results/anchor/dd-cache.json
//! ```
//!
//! Pass the anchor's `--dd-cache` and the census costs seconds instead of the
//! arm's whole DD fan-out — the tables key on the *deal*, so the anchor already
//! solved every board this reads.  Without it, every divergent board is solved
//! fresh (`bba-decompose`'s idiom, same key function).
//!
//! One call deeper — `--bucket 2♠ --responses 8` — splits that bucket by our
//! response (table A) and by BBA's response to *our* `2♠` overcall of its 1NT
//! (table B), then through responder's next turn, and classifies the hands
//! that passed by which authored call they fell between.  `--show N --next
//! "P P P"` dumps the worst boards of one continuation with the dealer,
//! responder's hand and the DD makes.
//!
//! `--table b` re-aims `--show`/`--next` at the **mirror** lane — *they* open
//! 1NT and we overcall — cut like the B panel to boards where our own 1NT was
//! uncontested or absent, with the acting hand printed as our overcaller;
//! `--show-score pd` ranks by the perfect-defense swing.  Each dumped board
//! carries both contracts with their plain and PD scores, so the gap between
//! the two columns reads off directly as perfect defense's synthetic double.
//!
//! `--table b` re-aims `--show`/`--next` at the **mirror** lane — *they* open
//! 1NT and we overcall — cut like the B panel to boards where our own 1NT was
//! uncontested or absent, with the acting hand printed as our overcaller;
//! `--show-score pd` ranks by the perfect-defense swing.  Each dumped board
//! carries both contracts with their plain and PD scores, so the gap between
//! the two columns reads off directly as perfect defense's synthetic double.
//!
//! **What this can and cannot say.**  The swing is the *board's* IMPs, not the
//! interference decision's: the same deal may also have BBA opening 1NT at
//! table B with us defending, and every later call is in there too.  So the
//! buckets **rank**, they do not isolate — isolation is the package's own A/B
//! (`bba-gen --filter-1nt`, one knob).  The confound is broadly common to all
//! buckets, which is what leaves the ranking usable.

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat, Strain};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use std::collections::HashMap;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, Dump, Reached, mean_with_ci, seat_to_act};

#[derive(Parser)]
struct Args {
    /// Directory of `shard-*.json` from one anchor arm
    dir: String,
    /// Re-price at this vulnerability instead of the dump's
    #[arg(short, long)]
    vulnerability: Option<AbsoluteVulnerability>,
    /// A deal-keyed DD table cache, created if absent and written back with
    /// every new solve — the artifact that makes the second cut of an arm cost
    /// seconds instead of its whole DD fan-out
    #[arg(long)]
    dd_cache: Option<String>,
    /// Dump this many worst plain-DD boards from `--bucket`
    #[arg(long, default_value_t = 0)]
    show: usize,
    /// Which bucket `--show` dumps, e.g. `2♣`, `X`, `3♠`, `4+`
    #[arg(long, default_value = "")]
    bucket: String,
    /// Split `--bucket` by the next calls — responder / advancer / opener — at
    /// table A (our response) and at table B (BBA's response to *our* overcall
    /// of its 1NT), through responder's next turn.  Sub-rows with fewer boards
    /// than this fold into `other`.
    #[arg(long)]
    responses: Option<usize>,
    /// With `--show`: only boards whose calls after the overcall start with
    /// this, e.g. `"P P P"` or `"2NT"`, on the `--table` being shown
    #[arg(long, default_value = "")]
    next: String,
    /// Which table `--show`/`--next` read: `a` (we open 1NT, they interfere)
    /// or `b` — the mirror lane, *they* open 1NT and we overcall, cut like the
    /// `-only` panel to boards where our own 1NT was uncontested or absent
    #[arg(long, default_value = "a")]
    table: String,
    /// Sort `--show` by `plain` double-dummy or `pd` perfect-defense swing
    #[arg(long, default_value = "plain")]
    show_score: String,
}

/// A deal's cache key: its serde string, as `bba-decompose` writes it
fn deal_key(deal: &FullDeal) -> String {
    serde_json::to_string(deal).expect("a deal serializes")
}

/// Solve every deal in `idxs` the cache is missing.  The census only needs the
/// divergent set; `--show` also prints boards whose two tables agreed, and those
/// need a table too if the dump is to carry their scores.
fn solve_missing(cache: &mut HashMap<String, TrickCountTable>, boards: &[Board], idxs: &[usize]) {
    let missing: Vec<usize> = idxs
        .iter()
        .copied()
        .filter(|&i| !cache.contains_key(&deal_key(&boards[i].deal)))
        .collect();
    for chunk in missing.chunks(4096) {
        let deals: Vec<FullDeal> = chunk.iter().map(|&i| boards[i].deal).collect();
        let solved = Solver::lock(None).solve_deals(&deals, NonEmptyStrainFlags::ALL);
        for (&i, table) in chunk.iter().zip(solved) {
            cache.insert(deal_key(&boards[i].deal), table);
        }
    }
}

/// Load every `shard-*.json` in a dir, concatenated, plus the dump vulnerability.
fn load_dir(dir: &str) -> (AbsoluteVulnerability, Vec<Board>) {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .expect("read arm dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with("shard-") && s.ends_with(".json"))
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no shard-*.json in {dir}");
    let mut vul = None;
    let mut boards = Vec::new();
    for path in paths {
        let dump: Dump = serde_json::from_reader(std::io::BufReader::new(
            std::fs::File::open(&path).expect("open shard"),
        ))
        .expect("parse shard");
        vul = Some(dump.vulnerability);
        boards.extend(dump.boards);
    }
    (vul.expect("at least one shard"), boards)
}

/// What a board contributes to the census.
enum Class {
    /// We did not open 1NT at table A — an opponent opened, or we opened
    /// something else.  A 1NT *overcall* lands here too.
    Other,
    /// We opened 1NT and RHO passed.
    Uncontested,
    /// We opened 1NT and RHO acted; the label is that action, the index the
    /// opening's position in the auction.
    Contested(String, usize),
}

/// Classify a board by RHO's action over our table-A 1NT opening.
fn classify(board: &Board) -> Class {
    classify_auction(&board.table_a, board.dealer)
}

/// Classify one table's auction by RHO's action over a North/South 1NT opening.
fn classify_auction(auction: &[Call], dealer: Seat) -> Class {
    let Some(open) = auction.iter().position(|&c| c != Call::Pass) else {
        return Class::Other;
    };
    if !matches!(seat_to_act(dealer, open), Seat::North | Seat::South) {
        return Class::Other;
    }
    let Call::Bid(bid) = auction[open] else {
        return Class::Other;
    };
    if bid.level.get() != 1 || bid.strain.is_suit() {
        return Class::Other;
    }
    match auction.get(open + 1) {
        None | Some(&Call::Pass) => Class::Uncontested,
        Some(&Call::Redouble) => Class::Contested("XX".to_owned(), open),
        Some(&Call::Double) => Class::Contested("X".to_owned(), open),
        // The three-level *suits* split per suit — N3 authors one table per
        // overcall, so the census has to price them apart.  `3NT` and the
        // four-level calls stay one `4+` bucket: still floor-only, still too
        // rare to rank.
        Some(&Call::Bid(rho)) if rho.level.get() == 3 && rho.strain.is_suit() => {
            Class::Contested(rho.to_string(), open)
        }
        Some(&Call::Bid(rho)) if rho.level.get() > 2 => Class::Contested("4+".to_owned(), open),
        Some(&Call::Bid(rho)) => Class::Contested(rho.to_string(), open),
    }
}

/// The `n` calls after position `at`, space-joined; `.` past the auction's end.
fn next_calls(auction: &[Call], at: usize, n: usize) -> String {
    (1..=n)
        .map(|k| {
            auction
                .get(at + k)
                .map_or(".".to_owned(), ToString::to_string)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// One sub-table of `(key, plain, pd)` rows, sorted by plain total, keys with
/// fewer than `min` boards folded into `other`.
#[allow(clippy::cast_precision_loss)]
fn sub_table(title: &str, rows: &[(String, i64, i64)], min: usize) {
    /// The plain and PD swings of one key's boards
    type Swings = (Vec<i64>, Vec<i64>);
    let mut buckets: std::collections::BTreeMap<&str, Swings> = std::collections::BTreeMap::new();
    for (key, p, d) in rows {
        let e = buckets.entry(key.as_str()).or_default();
        e.0.push(*p);
        e.1.push(*d);
    }
    let mut other: Swings = Default::default();
    let mut kept: Vec<(&str, &Swings)> = Vec::new();
    for (k, v) in &buckets {
        if v.0.len() < min {
            other.0.extend(&v.0);
            other.1.extend(&v.1);
        } else {
            kept.push((k, v));
        }
    }
    kept.sort_by_key(|(_, v)| v.0.iter().sum::<i64>());
    println!("\n--- {title}: {} boards ---", rows.len());
    println!(
        "{:<44} {:>6} {:>7} {:>18} {:>18} {:>9} {:>9}",
        "next calls", "boards", "share", "plain/bd", "PD/bd", "plain tot", "PD tot"
    );
    let n = rows.len().max(1) as f64;
    let line = |label: &str, p: &[i64], d: &[i64]| {
        let (pm, pc) = mean_with_ci(p);
        let (dm, dc) = mean_with_ci(d);
        println!(
            "{label:<44} {:>6} {:>6.1}% {:>10.3} ±{:.3} {:>10.3} ±{:.3} {:>9} {:>9}",
            p.len(),
            100.0 * p.len() as f64 / n,
            pm,
            pc,
            dm,
            dc,
            p.iter().sum::<i64>(),
            d.iter().sum::<i64>(),
        );
    };
    for (k, v) in kept {
        line(k, &v.0, &v.1);
    }
    if !other.0.is_empty() {
        line("other", &other.0, &other.1);
    }
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let (dump_vul, boards) = load_dir(&args.dir);
    let vul = args.vulnerability.unwrap_or(dump_vul);

    let contracts: Vec<(Reached, Reached)> = boards
        .iter()
        .map(|b| {
            (
                final_contract(&b.table_a, b.dealer),
                final_contract(&b.table_b, b.dealer),
            )
        })
        .collect();
    let divergent: Vec<usize> = (0..boards.len())
        .filter(|&i| contracts[i].0 != contracts[i].1)
        .collect();

    let mut cache: HashMap<String, TrickCountTable> = match args.dd_cache.as_deref() {
        Some(path) if std::path::Path::new(path).exists() => serde_json::from_reader(
            std::io::BufReader::new(std::fs::File::open(path).expect("open dd cache")),
        )
        .expect("parse dd cache"),
        _ => HashMap::new(),
    };
    let hits = divergent
        .iter()
        .filter(|&&i| cache.contains_key(&deal_key(&boards[i].deal)))
        .count();
    eprintln!(
        "dd cache: {hits}/{} divergent boards hit, {} to solve",
        divergent.len(),
        divergent.len() - hits
    );
    solve_missing(&mut cache, &boards, &divergent);

    let mut plain = vec![0i64; boards.len()];
    let mut pd = vec![0i64; boards.len()];
    for &i in &divergent {
        let table = &cache[&deal_key(&boards[i].deal)];
        let (a, b) = contracts[i];
        plain[i] = imps(ns_score_contract(a, table, vul) - ns_score_contract(b, table, vul));
        pd[i] = imps(ns_score_pd(a, table, vul) - ns_score_pd(b, table, vul));
    }

    let mut buckets: std::collections::BTreeMap<String, (Vec<i64>, Vec<i64>)> =
        std::collections::BTreeMap::new();
    let mut quiet: Vec<i64> = Vec::new();
    let mut shown: Vec<usize> = Vec::new();
    for (index, board) in boards.iter().enumerate() {
        match classify(board) {
            Class::Other => {}
            Class::Uncontested => quiet.push(plain[index]),
            Class::Contested(label, _) => {
                if label == args.bucket {
                    shown.push(index);
                }
                let entry = buckets.entry(label).or_default();
                entry.0.push(plain[index]);
                entry.1.push(pd[index]);
            }
        }
    }

    // `--table b` re-aims `--show` at the mirror lane: BBA opens 1NT and *we*
    // overcall.  Same `-only` cut as the panel, so the board's swing is one
    // lane's, not a blend of both tables' 1NT fights.
    let mirror = args.table == "b";
    if mirror {
        shown = boards
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                matches!(classify_auction(&b.table_b, b.dealer), Class::Contested(ref l, _) if *l == args.bucket)
                    && !matches!(classify_auction(&b.table_a, b.dealer), Class::Contested(..))
            })
            .map(|(i, _)| i)
            .collect();
    }

    let uncontested = quiet.len();
    let contested: usize = buckets.values().map(|(p, _)| p.len()).sum();
    let opened = contested + uncontested;
    println!("=== interference over our 1NT — {} ===", args.dir);
    println!("vulnerability {vul:?}, {} boards", boards.len());
    println!(
        "we opened 1NT: {opened} ({:.2}% of boards); contested {contested} ({:.1}% of our 1NTs)",
        100.0 * opened as f64 / boards.len() as f64,
        100.0 * contested as f64 / opened.max(1) as f64,
    );
    println!(
        "\nIMPs/board are the BOARD's swing, not the node's — these RANK, they do not isolate.\n"
    );
    println!(
        "{:<6} {:>8} {:>8} {:>20} {:>20} {:>10}",
        "RHO", "boards", "share", "plain IMPs/bd", "PD IMPs/bd", "plain tot"
    );
    let mut rows: Vec<_> = buckets.iter().collect();
    rows.sort_by(|a, b| {
        let total = |v: &Vec<i64>| v.iter().sum::<i64>();
        total(&a.1.0).cmp(&total(&b.1.0))
    });
    for (label, (p, d)) in rows {
        let (pm, pc) = mean_with_ci(p);
        let (dm, dc) = mean_with_ci(d);
        println!(
            "{label:<6} {:>8} {:>7.2}% {:>12.4} ±{:.4} {:>12.4} ±{:.4} {:>10}",
            p.len(),
            100.0 * p.len() as f64 / opened.max(1) as f64,
            pm,
            pc,
            dm,
            dc,
            p.iter().sum::<i64>(),
        );
    }
    let (all_p, all_d): (Vec<i64>, Vec<i64>) = buckets
        .values()
        .flat_map(|(p, d)| p.iter().copied().zip(d.iter().copied()))
        .unzip();
    let (pm, pc) = mean_with_ci(&all_p);
    let (dm, dc) = mean_with_ci(&all_d);
    println!(
        "{:<6} {:>8} {:>7.2}% {:>12.4} ±{:.4} {:>12.4} ±{:.4} {:>10}",
        "ALL",
        all_p.len(),
        100.0,
        pm,
        pc,
        dm,
        dc,
        all_p.iter().sum::<i64>(),
    );
    let (um, uc) = mean_with_ci(&quiet);
    println!(
        "\nreference — our uncontested 1NT: {uncontested} boards, plain {um:.4} ±{uc:.4} IMPs/bd"
    );

    if let Some(min) = args.responses {
        // Table A: we opened 1NT and RHO made `--bucket`; split by our
        // response, then by response / advancer / opener.  Table B: BBA
        // opened 1NT and *we* (EW) made the same call; split by BBA's
        // response.  A board can sit in both, so the "-only" cuts drop the
        // boards where the other table also contested a 1NT.  IMPs stay
        // ours (table-A NS) throughout — a negative table-B row is BBA's gain.
        let contested =
            |a: &[Call], d: Seat| matches!(classify_auction(a, d), Class::Contested(..));
        let mut a1 = Vec::new();
        let mut a3 = Vec::new();
        let mut a5 = Vec::new();
        let mut a1_only = Vec::new();
        let mut a3_only = Vec::new();
        let mut a5_only = Vec::new();
        let mut b1 = Vec::new();
        let mut b3 = Vec::new();
        let mut b5 = Vec::new();
        let mut b1_only = Vec::new();
        let mut b3_only = Vec::new();
        let mut b5_only = Vec::new();
        for (i, b) in boards.iter().enumerate() {
            let row =
                |a: &[Call], open: usize, n: usize| (next_calls(a, open + 1, n), plain[i], pd[i]);
            if let Class::Contested(label, open) = classify_auction(&b.table_a, b.dealer)
                && label == args.bucket
            {
                a1.push(row(&b.table_a, open, 1));
                a3.push(row(&b.table_a, open, 3));
                a5.push(row(&b.table_a, open, 5));
                if !contested(&b.table_b, b.dealer) {
                    a1_only.push(row(&b.table_a, open, 1));
                    a3_only.push(row(&b.table_a, open, 3));
                    a5_only.push(row(&b.table_a, open, 5));
                }
            }
            if let Class::Contested(label, open) = classify_auction(&b.table_b, b.dealer)
                && label == args.bucket
            {
                b1.push(row(&b.table_b, open, 1));
                b3.push(row(&b.table_b, open, 3));
                b5.push(row(&b.table_b, open, 5));
                if !contested(&b.table_a, b.dealer) {
                    b1_only.push(row(&b.table_b, open, 1));
                    b3_only.push(row(&b.table_b, open, 3));
                    b5_only.push(row(&b.table_b, open, 5));
                }
            }
        }
        let bk = &args.bucket;
        println!(
            "\n=== responses over ({bk}) — IMPs are OUR (table-A NS) swing on both tables ==="
        );
        sub_table(
            &format!("table A, we open 1NT ({bk}): OUR response"),
            &a1,
            min,
        );
        sub_table(
            "table A, A-only (BBA's 1NT uncontested/absent): OUR response",
            &a1_only,
            min,
        );
        sub_table("table A: our response / advancer / opener", &a3, min);
        sub_table("table A: through responder's next turn", &a5, min);
        sub_table(
            "table A, A-only: our response / advancer / opener",
            &a3_only,
            min,
        );
        sub_table(
            "table A, A-only: through responder's next turn",
            &a5_only,
            min,
        );
        sub_table(
            &format!("table B, BBA opens 1NT, we overcall ({bk}): BBA's response"),
            &b1,
            min,
        );
        sub_table(
            "table B, B-only (our 1NT uncontested/absent): BBA's response",
            &b1_only,
            min,
        );
        sub_table(
            "table B: BBA's response / our advance / BBA opener",
            &b3,
            min,
        );
        sub_table("table B: through BBA responder's next turn", &b5, min);
        sub_table(
            "table B, B-only: BBA's response / our advance / BBA opener",
            &b3_only,
            min,
        );
        sub_table(
            "table B, B-only: through BBA responder's next turn",
            &b5_only,
            min,
        );
    }

    if args.responses.is_some() {
        // Why did responder pass?  Classify responder's hand on the boards
        // where our first call over `--bucket` was Pass, by which of the
        // book's calls it fell between (over = their suit).
        let over = args.bucket.chars().last().and_then(|c| match c {
            '♣' => Some(contract_bridge::Suit::Clubs),
            '♦' => Some(contract_bridge::Suit::Diamonds),
            '♥' => Some(contract_bridge::Suit::Hearts),
            '♠' => Some(contract_bridge::Suit::Spades),
            _ => None,
        });
        if let Some(over) = over {
            let mut rows = Vec::new();
            for (i, b) in boards.iter().enumerate() {
                let Class::Contested(label, open) = classify(b) else {
                    continue;
                };
                if label != args.bucket || b.table_a.get(open + 2) != Some(&Call::Pass) {
                    continue;
                }
                let hand = b.deal[seat_to_act(b.dealer, open + 2)];
                let suits = [
                    contract_bridge::Suit::Clubs,
                    contract_bridge::Suit::Diamonds,
                    contract_bridge::Suit::Hearts,
                    contract_bridge::Suit::Spades,
                ];
                let hcp: u8 = suits
                    .iter()
                    .map(|&s| contract_bridge::eval::hcp::<u8>(hand[s]))
                    .sum();
                let pts = pons::bidding::constraint::point_count(hand);
                let long = suits
                    .iter()
                    .filter(|&&s| s != over)
                    .map(|&s| hand[s].len())
                    .max()
                    .unwrap_or(0);
                let theirs = hand[over].len();
                let their_hcp = contract_bridge::eval::hcp::<u8>(hand[over]);
                let key = if hcp >= 10 && their_hcp >= 5 {
                    "trap-pass (10+, 5+ hcp in theirs)"
                } else if hcp >= 8 && theirs <= 1 {
                    "8+ hcp, 0-1 in theirs: X needs 2-3"
                } else if hcp >= 8 && theirs >= 4 {
                    "8+ hcp, 4+ in theirs: X needs 2-3"
                } else if hcp >= 8 {
                    "8+ hcp, 2-3 in theirs — X was available?"
                } else if hcp <= 5 && long >= 6 {
                    "≤5 hcp, 6+ suit: relay floor is hcp 6"
                } else if hcp <= 5 && long == 5 {
                    "≤5 hcp, 5-card suit: relay floor"
                } else if (6..=7).contains(&hcp) && long >= 5 && pts >= 9 {
                    "6-7 hcp, 5+ suit, points 9+: relay ceiling"
                } else if (6..=7).contains(&hcp) && long >= 5 {
                    "6-7 hcp, 5+ suit, points ≤8 — relay was available?"
                } else if hcp <= 7 {
                    "≤7 hcp, no 5-card suit: nothing to say"
                } else {
                    "other"
                };
                rows.push((key.to_owned(), plain[i], pd[i]));
            }
            sub_table(
                &format!("table A: WHY responder passed over ({})", args.bucket),
                &rows,
                1,
            );
        }
    }

    if args.show > 0 {
        let by_pd = args.show_score == "pd";
        fn pick(b: &Board, mirror: bool) -> &[Call] {
            if mirror { &b.table_b } else { &b.table_a }
        }
        shown.sort_by_key(|&i| if by_pd { pd[i] } else { plain[i] });
        println!(
            "\n--- {} worst {} boards in {} (table {}) ---",
            args.show, args.show_score, args.bucket, args.table
        );
        let picked: Vec<usize> = shown
            .iter()
            .copied()
            .filter(|&i| {
                let Class::Contested(_, open) =
                    classify_auction(pick(&boards[i], mirror), boards[i].dealer)
                else {
                    return false;
                };
                next_calls(pick(&boards[i], mirror), open + 1, 3).starts_with(&args.next)
            })
            .take(args.show)
            .collect();
        solve_missing(&mut cache, &boards, &picked);
        for &i in &picked {
            let b = &boards[i];
            let Class::Contested(_, open) = classify_auction(pick(b, mirror), b.dealer) else {
                unreachable!()
            };
            // Table A's focus hand is our responder over their interference; the
            // mirror's is our overcaller, one seat earlier.
            let focus = seat_to_act(b.dealer, open + if mirror { 1 } else { 2 });
            let hand = b.deal[focus];
            let hcp: u8 = [
                contract_bridge::Suit::Clubs,
                contract_bridge::Suit::Diamonds,
                contract_bridge::Suit::Hearts,
                contract_bridge::Suit::Spades,
            ]
            .into_iter()
            .map(|s| contract_bridge::eval::hcp::<u8>(hand[s]))
            .sum();
            let pts = pons::bidding::constraint::point_count(hand);
            let table = cache.get(&deal_key(&b.deal));
            let dd = |seat: Seat| {
                table.map_or("?".to_owned(), |t| {
                    [
                        Strain::Notrump,
                        Strain::Spades,
                        Strain::Hearts,
                        Strain::Diamonds,
                        Strain::Clubs,
                    ]
                    .into_iter()
                    .map(|s| format!("{}{}", s, t[s].get(seat).get()))
                    .collect::<Vec<_>>()
                    .join(" ")
                })
            };
            let (ca, cb) = contracts[i];
            let name =
                |c: Reached| c.map_or("passed out".to_owned(), |(k, s)| format!("{k} by {s:?}"));
            // Plain/PD side by side per table: the gap between them IS perfect
            // defense's synthetic double of a failing undoubled contract.
            let score = |c: Reached| {
                table.map_or("?/?".to_owned(), |t| {
                    format!(
                        "{}/{}",
                        ns_score_contract(c, t, vul),
                        ns_score_pd(c, t, vul)
                    )
                })
            };
            println!(
                "[{:+} plain / {:+} PD] dealer {:?}, {} {focus:?} {hand} ({hcp} hcp, {pts} pts)\n  {}\n  us:  {}\n  bba: {}\n  A: {} {}   B: {} {}\n  DD N: {}   E: {}\n  DD S: {}   W: {}",
                plain[i],
                pd[i],
                b.dealer,
                if mirror { "overcaller" } else { "responder" },
                b.deal,
                b.table_a,
                b.table_b,
                name(ca),
                score(ca),
                name(cb),
                score(cb),
                dd(Seat::North),
                dd(Seat::East),
                dd(Seat::South),
                dd(Seat::West),
            );
        }
        let p: Vec<i64> = picked.iter().map(|&i| plain[i]).collect();
        let d: Vec<i64> = picked.iter().map(|&i| pd[i]).collect();
        println!(
            "shown {}: plain {} / PD {} total",
            picked.len(),
            p.iter().sum::<i64>(),
            d.iter().sum::<i64>()
        );
    }

    if let Some(path) = args.dd_cache.as_deref() {
        serde_json::to_writer(
            std::io::BufWriter::new(std::fs::File::create(path).expect("create dd cache")),
            &cache,
        )
        .expect("write dd cache");
        eprintln!("dd cache {path} now holds {} tables", cache.len());
    }
}
