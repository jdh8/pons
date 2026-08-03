//! Shared helpers for the A/B / measurement harnesses (`ab-*`, some `probe-*`).
//!
//! Pulled in verbatim with
//! `#[path = "../common/mod.rs"] #[allow(dead_code)] mod common;` — a sibling
//! directory holding only `mod.rs` (no `main.rs`) is invisible to Cargo's example
//! auto-discovery, so this never compiles as a standalone example. Each harness
//! uses only the subset it needs, hence the `#[allow(dead_code)]` on the `mod`.

#[allow(dead_code)]
pub mod oracle;

use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Hand, Seat, Suit};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::bidding::context::relative;
use pons::bidding::{Stance, System};
use pons::scoring::{
    final_contract, imps, ns_score_contract, ns_score_pd, ns_score_pd_tricks, ns_score_tricks,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

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
    let key = auction
        .iter()
        .skip_while(|&&call| call == Call::Pass)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    if key.is_empty() {
        "(opening passes)".to_owned()
    } else {
        key
    }
}

/// The seat acting after `len` calls from `dealer`
pub const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

/// `count` deals, board `i` seeded `base + i`, so every arm of an experiment
/// replays the identical stream (the seed-hygiene invariant the A/B scripts
/// rely on — one `SEED_BASE` shared across arms).  Bidding is pure and
/// parallelizes downstream; only the deal generation is centralized here.
pub fn seeded_deals(base: u64, count: usize) -> Vec<FullDeal> {
    (0..count)
        .map(|i| full_deal(&mut StdRng::seed_from_u64(base.wrapping_add(i as u64))))
        .collect()
}

/// The highest-logit *legal* call, defaulting to a pass
pub fn next_call(
    stance: &Stance,
    hand: Hand,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    auction: &Auction,
) -> Call {
    let seat = seat_to_act(dealer, auction.len());
    let Some(logits) = stance.classify(hand, relative(vul, seat), auction) else {
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
    conv: &Stance,
    baseline: &Stance,
    conv_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let stance = if seat_is_ns == conv_is_ns {
            conv
        } else {
            baseline
        };
        auction.push(next_call(stance, deal[seat], dealer, vul, &auction));
    }
    auction
}

/// Bid one deal with the opponents (East/West) forced to pass throughout
pub fn bid_uncontested(
    stance: &Stance,
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
            next_call(stance, deal[seat], dealer, vul, &auction)
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
    pub boards: Vec<Board>,
}

/// Load a dump from `path`.  A plain file is read directly; a **directory**
/// globs its `shard-*.json` (sorted, so paired arms concatenate in the same
/// order) and folds every shard's boards into one dump.  Handing a harness the
/// whole board set lets it solve the entire divergent set in a single DDS
/// fan-out, instead of launching one solver process per shard — each of which
/// spins a full-core DDS pool and oversubscribes the box.
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
        dump.boards.extend(read(shard).boards);
    }
    dump
}

/// Sample mean and the half-width of its 95% confidence interval
///
/// The mean is the headline IMPs/board; the half-width is `1.96 · SE` from the
/// per-board sample standard deviation, so a CI that excludes 0 is a result
/// distinguishable from noise.
#[allow(clippy::cast_precision_loss)]
pub fn mean_with_ci(values: &[i64]) -> (f64, f64) {
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

/// NS scores of an auction's reached contract under the **sd-declarer
/// playout** ([`pons::single_dummy_declarer_tricks`]): blind opening lead,
/// then a declarer who chooses every card over worlds consistent with the
/// auction instead of peeking.  This is the pessimist bracket for slam
/// aggression — plain DD is the optimist (a DD declarer never misguesses),
/// and perfect defense only prices doubling, not guessing.  `stance` reads
/// the auction for both modelled views (the leader's and declarer's); a
/// pass-out scores 0.
///
/// Returns `[plain, perfect-defense]` — the one trick count priced both ways
/// ([`ns_score_tricks`] and [`ns_score_pd_tricks`]).  Both come back from a
/// single call because the playout is expensive *and* draws from `rng`:
/// calling twice would run a second playout with different draws, pricing a
/// *different* trick count rather than the same one twice.
///
/// Read the perfect-defense entry as a pessimism stress-test, not an arbiter.
/// The defenders here already play double-dummy after the lead
/// ([`pons::single_dummy_playout`]), so the PD entry layers perfect doubling
/// on top of peeking defense and tracks the harness's existing DD-PD row; and
/// doubling a failing contract is realistic at partscore and game but not at
/// slam, where nobody doubles a voluntarily bid six.
// Each argument is a distinct fact of the board, as in `score_boards`.
#[allow(clippy::too_many_arguments)]
pub fn sd_declarer_ns_score(
    auction: &Auction,
    dealer: Seat,
    deal: &FullDeal,
    stance: &Stance,
    vul: AbsoluteVulnerability,
    rng: &mut StdRng,
    lead_worlds: usize,
    line_worlds: usize,
) -> [i64; 2] {
    let Some((contract, declarer)) = final_contract(auction, dealer) else {
        return [0; 2];
    };
    // Align the read prefix so the viewing seat is the player to act: the
    // last non-pass call sits within the final four calls, so exactly one of
    // four consecutive prefix lengths keeps every non-pass call and puts that
    // seat to act (the `ab-dump-sd` lead idiom, applied to both views).
    let view = |seat: Seat| {
        let cut = (auction.len().saturating_sub(3)..=auction.len())
            .find(|&len| seat_to_act(dealer, len) == seat)
            .expect("one of four consecutive lengths reaches every seat");
        stance.infer(relative(vul, seat), &auction[..cut])
    };
    let (_, tricks) = pons::single_dummy_declarer_tricks(
        deal,
        contract.bid.strain,
        declarer,
        &view(declarer.lho()),
        &view(declarer),
        rng,
        lead_worlds,
        line_worlds,
    );
    let tricks = u8::from(tricks);
    [
        ns_score_tricks(contract, declarer, tricks, vul),
        ns_score_pd_tricks(contract, declarer, tricks, vul),
    ]
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
    let divergent: Vec<usize> = (0..contracts.len())
        .filter(|&index| contracts[index].0 != contracts[index].1)
        .collect();
    let solve: Vec<FullDeal> = divergent.iter().map(|&index| deals[index]).collect();
    let tables = Solver::lock(None).solve_deals(&solve, NonEmptyStrainFlags::ALL);

    let mut total_points = 0i64;
    let mut board_imps = vec![0i64; contracts.len()];
    let mut swings: Vec<(usize, i64, i64)> = Vec::with_capacity(divergent.len());
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let swing = scorer(contract_a, table, vul) - scorer(contract_b, table, vul);
        total_points += swing;
        board_imps[index] = imps(swing);
        swings.push((index, swing, imps(swing)));
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
/// contract's actual penalty) and perfect defense (`ns_score_pd`, a failing
/// contract priced as doubled).  `contracts[i]` is `[off, on]`, and `tables`
/// are the solved divergent boards, parallel to `divergent`.  Each line reports
/// total IMPs, IMPs/board (over all `count`) and IMPs/divergent, with 95% CIs.
///
/// Unlike [`score_boards`], this scores twice from one solve — the harness
/// solves the divergent set once and reads both brackets off the same tables.
pub fn report_brackets(
    count: usize,
    divergent: &[usize],
    tables: &[TrickCountTable],
    contracts: &[[Option<(Contract, Seat)>; 2]],
    vul: AbsoluteVulnerability,
) {
    for (label, scorer) in [
        ("plain DD", ns_score_contract as fn(_, _, _) -> i64),
        ("perfect defense", ns_score_pd),
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
/// denominator of the per-divergent figure.
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
    }
}

/// Build one of our authored books by name, as `--our-floor`/`--their-floor`
/// spell it
///
/// Reads the live knob state, so a caller wanting a *perturbed* book sets every
/// `set_*` it should carry before calling and resets them straight after — the
/// knobs are captured at construction, so two differently-configured books can
/// then coexist on one thread.
pub fn seat_floor(name: &str) -> anyhow::Result<Stance> {
    Ok(match name {
        "american" => pons::american().against(),
        // The authored books with no floor at all: a driver seating this passes
        // whenever the books run out.  The floor ablation's other end.
        "american-book" => pons::bidding::american::american_book().against(),
        "dutch" => pons::dutch().against(),
        // The deterministic pre-swap floors: the fixed baselines now that
        // `american` and `dutch` both ship the BBA net.
        "american-instinct" => pons::american_instinct().against(),
        "dutch-instinct" => pons::dutch_instinct().against(),
        // The book ablation: no authored book at all, the same floor wiring
        // `american` uses.  `american` − `american-floor` prices the book.
        "american-floor" => pons::american_floor().against(),
        other => anyhow::bail!(
            "floor must be american|american-book|american-instinct|american-floor|dutch|dutch-instinct, got {other:?}"
        ),
    })
}

/// Apply the deviation-panel knobs, build a book under them, and reset
///
/// The B/C axes of docs/deviation-panel.md: `dial` is the antisymmetric
/// strength dial, the three flags are the shape-indiscipline knobs.  Only the
/// returned book carries them.
pub fn deviant_floor(
    name: &str,
    dial: u8,
    overcall_four_card: bool,
    offshape_1nt: bool,
    wild_weak_two: bool,
) -> anyhow::Result<Stance> {
    pons::bidding::constraint::set_strength_dial(dial);
    pons::bidding::american::set_overcall_four_card(overcall_four_card);
    pons::bidding::american::set_one_notrump_offshape(offshape_1nt);
    pons::bidding::american::set_weak_two_wild(wild_weak_two);
    let book = seat_floor(name);
    pons::bidding::constraint::set_strength_dial(0);
    pons::bidding::american::set_overcall_four_card(false);
    pons::bidding::american::set_one_notrump_offshape(false);
    pons::bidding::american::set_weak_two_wild(false);
    book
}

/// A [`System`] that classifies with the *opponents'* readings blanked
///
/// Wraps [`pons::bidding::set_blind_opponent_reading`] around `classify` and
/// restores the previous state, so one side of the table can go blind while a
/// pons book on the *other* side, sharing the thread, keeps its readings.  The
/// `blind` arm of the deviation panel.
pub struct Blinded<'a>(pub &'a dyn System);

impl System for Blinded<'_> {
    fn classify(
        &self,
        hand: Hand,
        vul: contract_bridge::auction::RelativeVulnerability,
        auction: &[Call],
    ) -> Option<pons::bidding::array::Logits> {
        let was = pons::bidding::blind_opponent_reading();
        pons::bidding::set_blind_opponent_reading(true);
        let out = self.0.classify(hand, vul, auction);
        pons::bidding::set_blind_opponent_reading(was);
        out
    }

    fn authored_at(
        &self,
        vul: contract_bridge::auction::RelativeVulnerability,
        auction: &[Call],
    ) -> bool {
        let was = pons::bidding::blind_opponent_reading();
        pons::bidding::set_blind_opponent_reading(true);
        let out = self.0.authored_at(vul, auction);
        pons::bidding::set_blind_opponent_reading(was);
        out
    }
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
