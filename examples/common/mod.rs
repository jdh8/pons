//! Shared helpers for the A/B / measurement harnesses (`ab-*`, some `probe-*`).
//!
//! Pulled in verbatim with
//! `#[path = "../common/mod.rs"] #[allow(dead_code)] mod common;` — a sibling
//! directory holding only `mod.rs` (no `main.rs`) is invisible to Cargo's example
//! auto-discovery, so this never compiles as a standalone example. Each harness
//! uses only the subset it needs, hence the `#[allow(dead_code)]` on the `mod`.

#[allow(dead_code)]
pub mod oracle;

use contract_bridge::auction::{Auction, Call, display_calls};
use contract_bridge::deck::full_deal;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Hand, Rank, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::bidding::agreements::Agreements;
use pons::bidding::card::{Card, american_card, dutch_card};
use pons::bidding::context::relative;
use pons::bidding::features::ConventionCard;
use pons::bidding::{Bidder, Partnership};
use pons::scoring::{
    final_contract, imps, ns_score_contract, ns_score_pd, ns_score_pd_tricks, ns_score_tricks,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

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
}

/// NS scores of an auction's reached contract under **both sd endpoints**:
/// blind opening lead, then the deal priced once with double-dummy play after
/// the lead (the optimist at slam level — Pavlicek's after-lead table has DD
/// play +7pp of make-rate on slams) and once with a declarer who chooses
/// every card over worlds consistent with the auction instead of peeking
/// (the pessimist — its misguess haircut measures ≈1.5× the real one).
/// `partnership` reads the auction for both modelled views (the leader's and
/// declarer's); a pass-out scores 0.
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
    partnership: &Partnership,
    vul: AbsoluteVulnerability,
    rng: &mut StdRng,
    lead_worlds: usize,
    line_worlds: usize,
) -> SdScores {
    let Some((contract, declarer)) = final_contract(auction, dealer) else {
        return SdScores::default();
    };
    // Align the read prefix so the viewing seat is the player to act: the
    // last non-pass call sits within the final four calls, so exactly one of
    // four consecutive prefix lengths keeps every non-pass call and puts that
    // seat to act (the `ab-dump-sd` lead idiom, applied to both views).
    let view = |seat: Seat| {
        let cut = (auction.len().saturating_sub(3)..=auction.len())
            .find(|&len| seat_to_act(dealer, len) == seat)
            .expect("one of four consecutive lengths reaches every seat");
        partnership.infer(relative(vul, seat), &auction[..cut])
    };
    let (_, lead_tricks, line_tricks) = pons::single_dummy_declarer_tricks(
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
        return (0.0, 0.0);
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
pub fn solve_divergent(
    contracts: &[(Reached, Reached)],
    deals: &[FullDeal],
) -> (Vec<usize>, Vec<TrickCountTable>) {
    let divergent: Vec<usize> = (0..contracts.len())
        .filter(|&index| contracts[index].0 != contracts[index].1)
        .collect();
    let solve: Vec<FullDeal> = divergent.iter().map(|&index| deals[index]).collect();
    let tables = Solver::lock(None).solve_deals(&solve, NonEmptyStrainFlags::ALL);
    (divergent, tables)
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
/// Takes the agreements as a value, so a caller wanting a *perturbed* book
/// receives the complete [`Agreements`] value for the system being described,
/// edits the fields it should carry, and passes the result — two
/// differently-configured books can then coexist on one thread.
pub fn seat_floor(name: &str, agreements: &Agreements) -> anyhow::Result<Partnership> {
    Ok(match name {
        "american" => pons::american(agreements).bind(),
        // The authored books with no floor at all: a driver seating this passes
        // whenever the books run out.  The floor ablation's other end.
        "american-book" => pons::bidding::american::american_book(agreements).bind(),
        "dutch" => pons::dutch(agreements).bind(),
        // The deterministic pre-swap floors: the fixed baselines now that
        // `american` and `dutch` both ship the BBA net.
        "american-instinct" => pons::american_instinct(agreements).bind(),
        "dutch-instinct" => pons::dutch_instinct(agreements).bind(),
        // The book ablation: no authored book at all, the same floor wiring
        // `american` uses.  `american` − `american-floor` prices the book.
        "american-floor" => pons::american_floor(agreements).bind(),
        "american-v6" => pons::bidding::american::american_v6(agreements).bind(),
        "american-v6-their" => pons::bidding::american::american_v6_their(agreements).bind(),
        "dutch-v6" => pons::bidding::dutch::dutch_v6(agreements).bind(),
        other => anyhow::bail!(
            "floor must be american|american-book|american-instinct|american-floor|american-v6|american-v6-their|dutch|dutch-instinct|dutch-v6, got {other:?}"
        ),
    })
}

/// The card a `--our-floor` / `--their-floor` name declares
///
/// Split on `-` like `bba-gen`'s `disclosure`: the `-instinct` / `-book` /
/// `-floor` variants differ only in the floor, which no card row expresses, so
/// they all declare their base system's card.
pub fn floor_card(name: &str, agreements: &Agreements) -> anyhow::Result<Card> {
    Ok(match name.split('-').next().unwrap_or_default() {
        "american" => american_card(agreements),
        "dutch" => dutch_card(agreements),
        other => anyhow::bail!(
            "no card generator for system `{other}` (known: american, dutch).  \
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
/// **Each name must reach the same net here as in [`seat_floor`]**, or the arms
/// of a `--declare-opponents` A/B measure the net swap rather than the
/// declaration — which is exactly what happened for one commit after the
/// 2026-08-08 v5 ship, at 3.5× `scripts/ab-declared-opponents.sh`'s own CI.  So
/// Both shipped names use the honest-reading v6 floor on both paths.
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
/// Only the two net-floored systems accept a declared opponent.  The other
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
        "dutch" => pons::dutch_with_card(agreements, &ConventionCard::from_card(theirs)).bind(),
        other => anyhow::bail!(
            "--declare-opponents needs a net floor to declare them to: \
             floor must be american|dutch, got {other:?}"
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
        "american" | "dutch" => seat_floor_vs(name, theirs, &agreements),
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
