//! The rollout harness of `docs/ai-bidder/logit-calibration.md` §4
//!
//! Session 2 put the book's order and the net's order side by side
//! (`probe-book-vs-net`) and found them disagreeing 8.7% of the time at
//! authored nodes.  A disagreement is not yet a verdict: neither order is
//! *evidence*.  This probe supplies the evidence, by rolling the disagreement
//! out and pricing it in IMPs.
//!
//! # Which decisions — `--population`
//!
//! The axis the session-4 handoff's **D1** turns on, and the reason this probe
//! has two arms rather than one:
//!
//! * **`authored`** (`--population authored`) — §4's census, kept so its numbers
//!   stay reproducible from one binary.  The book answered, and
//!   `Trie::classify_floored` takes the book's logits whenever they have mass —
//!   the floor is a *depth-0 root fallback* on the trie (`with_floors`), not an
//!   `OrElse` layer; `Bidder::or_else` has no non-test caller, so the handoff's
//!   `compose.rs` citation names the wrong mechanism for the same behaviour —
//!   so at **100%** of this population
//!   production bids the book and the net is *never evaluated*.  Relabelling
//!   here can only pay by generalising to nodes the floor serves.  The proposal
//!   is therefore the raw net — `classify_bba_v6` called directly, never
//!   through the floor shell, which at a constructive node delegates to the
//!   very rung ladder the odds replace.
//! * **`net-served`** (the default) — the nodes production actually consults
//!   the net at.  `!Provenance::is_authored` is necessary but not sufficient:
//!   the root fallback resolves to **three** floors (`common.rs`,
//!   `with_floors`), and the deterministic `instinct()` ladder answers both the
//!   constructive book and — as a rail inside the shell — every `forced`
//!   context.  So the harvest predicate is unauthored **and**
//!   `Phase::of != Constructive` **and** `!forced(context)`.  Harvesting either
//!   of the other two rows would repeat D1's mistake one layer down.
//!
//! At a net-served node the `Logits` `classify_with_provenance` returns **are**
//! the production distribution — the net after `mask_illegal`,
//! `competitive_gate` (`PASS_DEMOTION` included) and `new_suit_gate` — so they
//! are the proposal directly, and the census prices the shell as shipped.  A
//! call the gates veto is `-∞` and can never be a candidate, which is correct:
//! a retrained net's preference for it could not reach the table through those
//! same gates either.  Two columns go quiet as a result — the mask leaves the
//! admissible mass at 1, so nothing is ever `thin` and `--epsilon` never fires;
//! and with two finite calls and `k ≥ 2` the candidate set always holds an
//! alternative, so `flat` counts only gate-starved nodes.  Both are still
//! printed, so the two censuses read side by side.
//!
//! The raw-net-versus-shell question — are the gates costing IMPs? — is a
//! *separate* arm with its own retrain-free payoff, and is deliberately not
//! folded in here.
//!
//! # The measurement
//!
//! At every harvested decision of a self-play `american()` walk:
//!
//! 1. **The proposal**, softmaxed at `--temperature`, **restricted** to the
//!    admissible calls (finite logit, legal here) and renormalised over that
//!    set.  Below `--epsilon` of unrestricted mass the hook declines and the
//!    node is counted `thin`, never evaluated — the `authored` arm's fallback
//!    is the book's one-hot, which has no top-k.
//! 2. **The candidate set** is the proposal's top-`k` ∪ {the policy's own
//!    call}.  Restricting and rescaling cannot reorder, so this set — and every
//!    count below that is not `thin` — is **`T`-invariant**.
//! 3. **The rollout.** One `2 × --layouts` draw from `sample_layouts_replay`,
//!    split at the midpoint: select on the first half, validate on the second.
//!    `sampler::sample_with` is plain rejection sampling, so
//!    conditional on the draw filling, its accepted layouts are i.i.d. and the
//!    two halves are independent pools — a short draw is counted and skipped
//!    whole rather than topped up. One double-dummy solve per layout is
//!    **shared across every candidate**, each candidate seeded onto the real
//!    prefix and bid out.
//! 4. **The paired baseline** is the own call's contract on the *same* layout,
//!    so a candidate is credited only for beating what we would have done
//!    anyway.  The advantage is `mean_layouts imps(candidate − own)`, reported
//!    under **both** scorers of [`docs/measurement.md`] — plain DD
//!    (`ns_score_contract`) and perfect defense (`ns_score_pd`).
//!
//! The census then prices the target rule: a decision **relabels** when some
//! candidate's advantage clears `--margin` IMPs.  Three rules are counted at
//! once — plain DD alone, PD alone, and both — so the choice of arbiter is
//! priced rather than assumed.  Each rule also reports the **Pass share** of
//! its held-out relabel targets against the base rate at the same decisions:
//! the handoff's **D2** check, since a double-dummy oracle cannot price what a
//! bid conceals and so systematically prefers passing.
//!
//! ```sh
//! cargo run --release --example probe-rollout-label -- -c 400 -s 1 -m 8
//! cargo run --release --example probe-rollout-label -- -c 200 -s 1 -m 128 --vul both
//! ```
//!
//! # Scope choices
//!
//! * **Self-play by default, `--opponent bba` measured.** §4 says "roll each
//!   out with our policy against BBA".  BBA is FFI-bound and single-threaded by
//!   design — a fresh native bot is created and destroyed for every *call*
//!   ([`BbaOracle::with_bot`], `examples/common/oracle/mod.rs:457`), and
//!   `examples/bba-gen` parallelises across *processes* — so a BBA opponent
//!   costs `candidates × layouts × decisions × (opponent calls per bid-out)`
//!   serialised bot spawns.  **Measured** at `-c 400 -s 1 -m 8`: the rollout
//!   goes 0.3 s → 39.8 s, but double dummy still dominates, so wall clock is
//!   only 1.46× (92.2 s → 134.8 s).  It changes no conclusion — see §4 — and
//!   the default stays self-play, which is also the model the layouts were
//!   drawn under: `sample_layouts_replay` accepts a world by replaying **our**
//!   policy on the non-actors, so a BBA rollout scores worlds selected by a
//!   different opponent model.
//! * **`ns_score_pd`, not `ns_score_bid`.**  `ev_all` prices a bare *call* and
//!   so synthesises the double (`ns_score_bid`).  Here both branches are real
//!   auctions that may contain a real double, and `scoring.rs` is explicit that
//!   a duplicate comparison in which a side may defend by passing is scored
//!   with `ns_score_pd`.
//! * **No fitted `T`.**  No shipped weights sidecar carries a `temperature`
//!   (session 2 built the fitter; no net has been trained since). Temperature
//!   cannot change candidate order, but it changes which decisions clear the
//!   epsilon gate and therefore the evaluated population. A distilled net is
//!   expected to calibrate at `T > 1`, loosening every epsilon rung, so the
//!   census reports the sensitivity instead of waiting on a fitted value.
//! * **Vulnerability is a flag, not a phase.** `--vul` takes the measurement
//!   playbook's two cells. The net-served population is contested by
//!   construction, and both the competitive book and `competitive_gate` read
//!   vulnerability, so the axis matters more here than it did for the authored
//!   census — run `none` and `both` and keep them separate.
//! * **Our auctions and our own call, not BBA's corpus and BBA's label.** The
//!   walk is self-play `american()`, so the baseline displaced here is *the
//!   book's argmax* at *our* node distribution.  §4's target rule instead
//!   relabels **BBA's one-hot** on the BBA-generated corpus, and falls back to
//!   BBA off the margin.  This probe therefore prices the winner's curse in the
//!   selector — which is what refuted the in-sample rule — but its relabel
//!   *rates* are not the rates that rule would fire at.  Closing that needs a
//!   corpus-fed mode; see §6.

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::agreements::Agreements;
use pons::bidding::array::Logits;
use pons::bidding::context::relative;
use pons::bidding::features::{CompactConfig, ConventionCard, features_v6};
use pons::bidding::instinct::forced;
use pons::bidding::neural::classify_bba_v6;
use pons::bidding::sampler::sample_layouts_replay;
use pons::bidding::table::select_legal_call;
use pons::bidding::{Bidder, Phase, Table};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::collections::BTreeMap;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{auction_key, seat_to_act, seeded_deals};

/// The two brackets every result carries, in the order `docs/measurement.md`
/// reads them: plain DD first, perfect defense second.
const BRACKETS: [&str; 2] = ["plain DD", "perfect defense"];

/// The four floors a choice-bearing decision can resolve through, in the order
/// `harvest` counts them.  Only the last consults the net.
const SLICES: [&str; 4] = [
    "authored",
    "constructive-floor",
    "forced-rail",
    "net-served",
];

#[derive(Parser)]
struct Args {
    /// Deals to bid
    #[arg(short, long, default_value_t = 500)]
    count: usize,
    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,
    /// Layouts per selection/validation pool (the rollout's `M`; 2M are drawn)
    #[arg(short = 'm', long, default_value_t = 32)]
    layouts: usize,
    /// Proposal calls to roll out, before the union with the own call.  In the
    /// net-served arm the own call *is* the proposal's argmax (both are the
    /// shell's first-max over the same logits), so `k` buys `k - 1`
    /// alternatives and `k = 1` prices nothing; the authored arm's own call is
    /// the book's, which need not head the net's order at all.
    #[arg(short = 'k', long, default_value_t = 3)]
    top_k: usize,
    /// Admissible-mass rung below which the hook declines (the epsilon fallback)
    #[arg(short, long, default_value_t = 1e-4)]
    epsilon: f32,
    /// Softmax temperature. Proposal ordering is invariant; the epsilon
    /// fallback's fire rate and therefore the evaluated population move.
    #[arg(short, long, default_value_t = 1.0)]
    temperature: f32,
    /// Relabel margin in IMPs: a candidate displaces the label only by this much
    #[arg(long, default_value_t = 0.25)]
    margin: f64,
    /// Evaluate one authored decision in every `stride` (cost dial; the census
    /// rotates the retained auction position across four-dealer cycles)
    #[arg(long, default_value_t = 1)]
    stride: usize,
    /// Who bids the other side during a rollout
    #[arg(long, value_enum, default_value_t = Opponent::SelfPlay)]
    opponent: Opponent,
    /// Which decisions to harvest: the book's own nodes, or the ones the floor
    /// shell's net actually answers
    #[arg(long, value_enum, default_value_t = Population::NetServed)]
    population: Population,
    /// Board vulnerability; the playbook's two cells are `none` and `both`.
    /// `ns`/`ew` parse and are honest boards, but the walk harvests all four
    /// seats, so only the symmetric cells hold *relative* vulnerability fixed
    /// across the census.
    #[arg(long, default_value = "none")]
    vul: AbsoluteVulnerability,
    /// Relabelling nodes to list
    #[arg(long, default_value_t = 20)]
    nodes: usize,
    /// Who advances the harvested auction — us, or the corpus's own teacher
    #[arg(long, value_enum, default_value_t = Walk::SelfPlay)]
    walk: Walk,
}

/// Which auctions the census visits, and whose call it prices against
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Walk {
    /// Our policy at all four seats, `own` = its own call. §4/§4b's population.
    #[value(name = "self")]
    SelfPlay,
    /// BBA's legal argmax at all four seats, `own` = **BBA's call** — the walk
    /// `dump-teacher` runs, so this is the corpus population and the paired
    /// baseline is the label a relabel would actually overwrite.  Serialised
    /// alongside the rollout's own convention.
    Teacher,
}

impl Walk {
    /// The `--walk` spelling, for the report header
    const fn name(self) -> &'static str {
        match self {
            Self::SelfPlay => "self",
            Self::Teacher => "teacher",
        }
    }
}

/// Which decisions the census walks — the session-4 handoff's D1 axis
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Population {
    /// Nodes the book answers. Production bids the book here, so the net is
    /// never evaluated and a relabelling pays only by generalising elsewhere.
    Authored,
    /// Nodes the floor shell's net answers: unauthored, contested, and not a
    /// `forced` rail. The policy's own call is the shell's argmax.
    #[value(name = "net-served")]
    NetServed,
}

impl Population {
    /// The `--population` spelling, for the report header
    const fn name(self) -> &'static str {
        match self {
            Self::Authored => "authored",
            Self::NetServed => "net-served",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Opponent {
    /// Our own policy on both sides — `ev_all`'s assumption, and the model the
    /// replay sampler selected the layouts under
    #[value(name = "self")]
    SelfPlay,
    /// BBA's own 2/1 card through EPBot. Correct per the design, and serialised:
    /// the FFI is not assumed thread-safe, so the whole rollout runs one bot at
    /// a time.
    Bba,
}

/// One harvested decision, taken before any solver runs
struct Decision {
    /// Source deal, for deal-clustered uncertainty
    deal: usize,
    key: String,
    hand: Hand,
    seat: Seat,
    dealer: Seat,
    /// The real prefix this decision faced
    prefix: Vec<Call>,
    /// The policy's own call — the paired baseline, always a candidate
    own: Call,
    /// Own call first, then the proposal's top-`k` minus it. Empty when the
    /// hook declined (`thin`) or offered no alternative to the own call.
    candidates: Vec<Call>,
    /// The hook declined: the net put under `--epsilon` of its mass on the
    /// calls the book admits here
    thin: bool,
}

/// What one node accumulated
#[derive(Default)]
struct Bucket {
    /// In-population decisions seen — counted *after* `--stride`, since
    /// `harvest` only emits the decisions the stride kept
    seen: usize,
    /// Declined by the epsilon fallback
    thin: usize,
    /// The proposal's top-`k` held nothing but the policy's own call
    flat: usize,
    /// Rolled out
    priced: usize,
    /// Source deal for each entry in the two advantage vectors
    deals: Vec<usize>,
    /// Relabelled under [plain DD, PD, both], on the in-sample estimate
    relabel: [usize; 3],
    /// The same three rules read off the held-out half. The continuous estimate
    /// is unbiased for the first-half selector; thresholding it is not.
    held_out_relabel: [usize; 3],
    /// The D2 check, per rule: of the held-out relabels, how many put **Pass**
    /// in the target slot, and how many already had Pass as the own call.  A
    /// relabel needs `winner != own`, so the two are **disjoint** — read them as
    /// a flow into and out of Pass, against [`Bucket::priced_pass`], which is
    /// the population base rate neither column is.
    held_out_pass: [[usize; 2]; 3],
    /// Rolled-out decisions whose own call is Pass — the population base rate
    /// both D2 columns are read against.
    priced_pass: usize,
    /// `own -> winner` swaps under the `both` rule, by count
    swaps: BTreeMap<String, usize>,
    /// Best candidate advantage on the selection pool, per bracket, in IMPs
    in_sample_advantage: [Vec<f64>; 2],
    /// The same selected calls, reported on the independent validation pool
    held_out_advantage: [Vec<f64>; 2],
}

impl Bucket {
    fn merge(&mut self, other: Self) {
        self.seen += other.seen;
        self.thin += other.thin;
        self.flat += other.flat;
        self.priced += other.priced;
        self.priced_pass += other.priced_pass;
        self.deals.extend(other.deals);
        for (ours, theirs) in self.relabel.iter_mut().zip(other.relabel) {
            *ours += theirs;
        }
        for (ours, theirs) in self.held_out_relabel.iter_mut().zip(other.held_out_relabel) {
            *ours += theirs;
        }
        for (ours, theirs) in self.held_out_pass.iter_mut().zip(other.held_out_pass) {
            for (ours, theirs) in ours.iter_mut().zip(theirs) {
                *ours += theirs;
            }
        }
        for (swap, n) in other.swaps {
            *self.swaps.entry(swap).or_default() += n;
        }
        for (ours, theirs) in self
            .in_sample_advantage
            .iter_mut()
            .zip(other.in_sample_advantage)
        {
            ours.extend(theirs);
        }
        for (ours, theirs) in self
            .held_out_advantage
            .iter_mut()
            .zip(other.held_out_advantage)
        {
            ours.extend(theirs);
        }
    }
}

/// Decision-weighted mean and asymptotic 95% CI, with dependence clustered by
/// source deal. Several decisions can come from one deal, so treating them as
/// independent would make the interval too narrow.
#[allow(clippy::cast_precision_loss)]
fn clustered_mean_with_ci(values: &[(usize, f64)]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, f64::NAN);
    }
    let n = values.len() as f64;
    let mean = values.iter().map(|(_, value)| value).sum::<f64>() / n;
    let mut scores = BTreeMap::<usize, f64>::new();
    for &(deal, value) in values {
        *scores.entry(deal).or_default() += value - mean;
    }
    if scores.len() < 2 {
        return (mean, f64::NAN);
    }
    let clusters = scores.len() as f64;
    let variance = clusters / (clusters - 1.0)
        * scores.values().map(|score| score * score).sum::<f64>()
        / (n * n);
    (mean, 1.96 * variance.sqrt())
}

/// The proposal hook: `softmax(z / t)` restricted to `admissible` and
/// renormalised over it, paired with the share of the *unrestricted* mass that
/// set holds (what `--epsilon` thresholds).
fn restricted(logits: &Logits, admissible: &[Call], t: f32) -> (Vec<(Call, f32)>, f32) {
    let beta = 1.0 / t;
    let max = logits
        .iter()
        .map(|(_, &z)| z)
        .fold(f32::NEG_INFINITY, f32::max);
    let (mut inside, mut total) = (0.0, 0.0);
    let mut odds = Vec::with_capacity(admissible.len());
    for (call, &z) in logits.iter() {
        let w = (beta * (z - max)).exp();
        total += w;
        if admissible.contains(&call) {
            inside += w;
            odds.push((call, w));
        }
    }
    for (_, w) in &mut odds {
        *w /= inside;
    }
    odds.sort_by(|a, b| b.1.total_cmp(&a.1));
    (odds, inside / total)
}

/// Walk one deal's auction, harvesting every in-population decision whose
/// proposal offers a live alternative to the call that advances the walk.
///
/// Returns the decisions and the four-way slice histogram over every
/// choice-bearing node, in [`SLICES`] order — the denominator that prices a
/// corpus pass, since only the net-served slice is worth relabelling.
fn harvest(
    deal: &FullDeal,
    index: usize,
    args: &Args,
    ctx: &Shared,
    teacher: Option<&common::oracle::BbaOracle>,
) -> (Vec<Decision>, [u64; SLICES.len()]) {
    let dealer = Seat::ALL[index % 4];
    let mut auction = Auction::new();
    let mut out = Vec::new();
    let mut slices = [0u64; SLICES.len()];
    let mut kept = 0usize;
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let hand = deal[seat];
        let rel = relative(ctx.vul, seat);
        // Under a teacher walk the auction is BBA's, so a position our book
        // declines still has to advance by BBA's call, not by a pass.
        let advance = |auction: &Auction| match teacher {
            Some(t) => common::oracle::next_call(t, hand, seat, ctx.vul, auction),
            None => Call::Pass,
        };
        let Some((book, provenance)) = ctx.policy.classify_with_provenance(hand, rel, &auction)
        else {
            let call = advance(&auction);
            auction.push(call);
            continue;
        };
        let admissible: Vec<Call> = book
            .iter()
            .filter(|(call, logit)| logit.is_finite() && auction.can_push(*call).is_ok())
            .map(|(call, _)| call)
            .collect();
        // The paired baseline: production's own selector under a self-play
        // walk, BBA's own call — the label a relabel overwrites — under a
        // teacher walk.  BBA's call may sit outside `admissible`; that is
        // itself a finding, and `advantages` only needs it to be legal.
        let own = match teacher {
            Some(_) => advance(&auction),
            None => select_legal_call(Some(book), &auction),
        };
        // The slice. `is_authored` does not partition into "book" and "net":
        // an unauthored constructive decision, and an unauthored contested one
        // inside a `forced` rail, are both answered by the `instinct()` ladder,
        // so the net is not consulted there either.  Only slice 3 is net-served.
        let slice = (admissible.len() > 1).then(|| {
            if provenance.is_authored() {
                0
            } else if Phase::of(&auction) != Phase::Constructive {
                usize::from(!forced(&ctx.policy.prefixed_context(rel, &auction))) + 2
            } else {
                1
            }
        });
        if let Some(slice) = slice {
            slices[slice] += 1;
        }
        if slice
            == Some(match args.population {
                Population::Authored => 0,
                Population::NetServed => 3,
            })
        {
            let key = auction_key(&auction);
            // Rotate the retained auction position once per four-dealer cycle;
            // resetting the phase on every deal would always keep early calls.
            let take = (index / Seat::ALL.len() + kept).is_multiple_of(args.stride);
            kept += 1;
            let (odds, mass) = match args.population {
                // The book shadows the net here, so the net has to be asked
                // directly — the shell would answer with the book's own logits.
                Population::Authored => {
                    let context = ctx
                        .policy
                        .prefixed_context(rel, &auction)
                        .with_compact(&ctx.compact);
                    let net = classify_bba_v6(&features_v6(hand, &context));
                    restricted(&net, &admissible, args.temperature)
                }
                // `book` *is* the net here: masked, gated, demoted — production's
                // own distribution. Re-running the raw net would price a policy
                // that cannot reach the table.
                Population::NetServed => restricted(&book, &admissible, args.temperature),
            };
            // Three outcomes, all reported so the denominators stay honest:
            // the hook declines (`thin`); it proposes nothing the policy was
            // not already going to do (`flat`); or there is a live alternative.
            let thin = mass < args.epsilon;
            let mut candidates = Vec::new();
            if !thin {
                candidates.push(own);
                candidates.extend(
                    odds.iter()
                        .take(args.top_k)
                        .map(|&(call, _)| call)
                        .filter(|&call| call != own),
                );
                if candidates.len() < 2 {
                    candidates.clear();
                }
            }
            if take {
                out.push(Decision {
                    deal: index,
                    key,
                    hand,
                    seat,
                    dealer,
                    prefix: auction.iter().copied().collect(),
                    own,
                    candidates,
                    thin,
                });
            }
        }
        auction.push(own);
    }
    (out, slices)
}

/// Everything the walk and the rollout share, built once
struct Shared {
    policy: pons::bidding::Partnership,
    vul: AbsoluteVulnerability,
    compact: CompactConfig,
}

/// Draw the decision's layouts from the replay sampler.
fn layouts_for(decision: &Decision, layouts: usize, ctx: &Shared, seed: u64) -> Vec<FullDeal> {
    let rel = relative(ctx.vul, decision.seat);
    let inferences = ctx.policy.infer(rel, &decision.prefix);
    let mut rng = StdRng::seed_from_u64(seed);
    sample_layouts_replay(
        decision.hand,
        decision.seat,
        &ctx.policy,
        rel,
        &decision.prefix,
        &inferences,
        &mut rng,
        layouts,
    )
}

/// One candidate's advantage over the own call, in IMPs, on two independent
/// layout pools
///
/// Picking the largest of `k` estimates on `first` and reporting that same
/// estimate is upward-biased. Reporting that selected call on independent
/// `second` layouts is unbiased for the selector, and their gap is the winner's
/// curse at this `M`.
#[derive(Clone, Copy, Default)]
struct Advantage {
    first: [f64; 2],
    second: [f64; 2],
}

/// Every candidate's advantage over the own call, priced on the same layouts
/// and the same double-dummy solves.
///
/// One entry per candidate, own call first — so its own advantage is exactly
/// zero on every bracket and every half, by construction.
fn advantages(
    decision: &Decision,
    layouts: &[FullDeal],
    tables: &[ddss::TrickCountTable],
    ours: &dyn Bidder,
    theirs: &dyn Bidder,
    vul: AbsoluteVulnerability,
) -> Vec<Advantage> {
    let actor_is_ns = matches!(decision.seat, Seat::North | Seat::South);
    let (ns, ew): (&dyn Bidder, &dyn Bidder) = if actor_is_ns {
        (ours, theirs)
    } else {
        (theirs, ours)
    };
    let table = Table::new(ns, ew, decision.dealer, vul);
    let sign = if actor_is_ns { 1 } else { -1 };

    let priced: Vec<Vec<[i64; 2]>> = decision
        .candidates
        .iter()
        .map(|&call| {
            layouts
                .iter()
                .zip(tables)
                .map(|(layout, tricks)| {
                    let mut seed = Auction::new();
                    seed.try_extend(decision.prefix.iter().copied())
                        .expect("a prior table auction is legal");
                    // Every candidate came from `admissible`, which already
                    // tested `can_push` against this same prefix.
                    seed.try_push(call).expect("a candidate is legal here");
                    let reached =
                        final_contract(&table.bid_out_from(layout, seed), decision.dealer);
                    [
                        sign * ns_score_contract(reached, tricks, vul),
                        sign * ns_score_pd(reached, tricks, vul),
                    ]
                })
                .collect()
        })
        .collect();

    let base = priced[0].clone();
    let half = layouts.len() / 2;
    #[allow(clippy::cast_precision_loss)] // IMP sums are small integers
    let mean = |sum: i64, n: usize| if n == 0 { 0.0 } else { sum as f64 / n as f64 };
    priced
        .iter()
        .map(|candidate| {
            let mut advantage = Advantage::default();
            for bracket in 0..2 {
                let (mut lo, mut hi) = (0i64, 0i64);
                for (index, (got, want)) in candidate.iter().zip(&base).enumerate() {
                    let swing = imps(got[bracket] - want[bracket]);
                    if index < half {
                        lo += swing;
                    } else {
                        hi += swing;
                    }
                }
                advantage.first[bracket] = mean(lo, half);
                advantage.second[bracket] = mean(hi, layouts.len() - half);
            }
            advantage
        })
        .collect()
}

fn main() {
    let args = Args::parse();
    assert!(args.stride > 0, "--stride must be positive");
    assert!(args.layouts > 0, "--layouts must be positive");
    assert!(
        args.epsilon.is_finite() && args.epsilon > 0.0 && args.epsilon <= 1.0,
        "--epsilon must be finite and in (0, 1]"
    );
    assert!(
        args.temperature.is_finite() && args.temperature > 0.0,
        "--temperature must be finite and positive"
    );
    assert!(args.margin.is_finite(), "--margin must be finite");
    let draws_per_decision = args.layouts.checked_mul(2).expect("--layouts is too large");
    let base = args.seed.unwrap_or_else(rand::random);
    let agreements = Agreements::default();
    let ctx = Shared {
        policy: american(&agreements).bind(),
        vul: args.vul,
        compact: CompactConfig::symmetric(&ConventionCard::capture(&agreements, false)),
    };

    // One bot serves both the teacher walk and the BBA rollout arm.
    let rollout_bba = args.opponent == Opponent::Bba;
    let oracle = (rollout_bba || args.walk == Walk::Teacher).then(|| {
        common::oracle::BbaOracle::load(
            common::oracle::DEFAULT_LIB,
            common::oracle::SYSTEM_2_OVER_1,
            Vec::new(),
        )
        .expect("libEPBot.so loads (see examples/common/oracle)")
    });
    let teacher = (args.walk == Walk::Teacher).then(|| oracle.as_ref().expect("loaded above"));

    let deals = seeded_deals(base, args.count);
    let started = std::time::Instant::now();

    // Phase 1 — walk every auction and harvest the decisions.  Pure bidding, so
    // rayon owns the pool; nothing here touches the solver.  A teacher walk
    // runs one bot at a time, like the BBA rollout arm.
    let harvested: Vec<(Vec<Decision>, [u64; SLICES.len()])> = match teacher {
        Some(t) => deals
            .iter()
            .enumerate()
            .map(|(index, deal)| harvest(deal, index, &args, &ctx, Some(t)))
            .collect(),
        None => deals
            .par_iter()
            .enumerate()
            .map(|(index, deal)| harvest(deal, index, &args, &ctx, None))
            .collect(),
    };
    let mut slices = [0u64; SLICES.len()];
    let mut decisions: Vec<Decision> = Vec::new();
    for (found, counted) in harvested {
        decisions.extend(found);
        for (total, one) in slices.iter_mut().zip(counted) {
            *total += one;
        }
    }
    let decisions = decisions;
    let walked = started.elapsed();

    // Phase 2 — draw the layouts. Still no solver, still rayon. Layout streams
    // start after the source-deal seed range, so the two RNG domains never
    // reuse a stream; each decision index still replays exactly.
    let attempted: Vec<usize> = (0..decisions.len())
        .filter(|&i| !decisions[i].candidates.is_empty())
        .collect();
    let drawn: Vec<Vec<FullDeal>> = attempted
        .par_iter()
        .map(|&i| {
            layouts_for(
                &decisions[i],
                draws_per_decision,
                &ctx,
                base.wrapping_add(args.count as u64).wrapping_add(i as u64),
            )
        })
        .collect();
    let underfilled = drawn
        .iter()
        .filter(|layouts| layouts.len() < draws_per_decision)
        .count();
    let attempted_n = attempted.len();
    let (priced, sampled): (Vec<usize>, Vec<Vec<FullDeal>>) = attempted
        .into_iter()
        .zip(drawn)
        .filter(|(_, layouts)| layouts.len() == draws_per_decision)
        .unzip();
    let drawn = started.elapsed() - walked;

    // Phase 3 — one solve for every layout, **on the main thread**: `Solver::lock`
    // takes a global lock and fans out over the core pool itself.
    let all_layouts: Vec<FullDeal> = sampled.iter().flatten().copied().collect();
    let tables = Solver::lock(None).solve_deals(&all_layouts, NonEmptyStrainFlags::ALL);
    let solved = started.elapsed() - walked - drawn;

    // Phase 4 — bid out and price.  BBA's FFI is not thread-safe, so its arm
    // runs sequentially; self-play fans out.
    let mut offsets = Vec::with_capacity(sampled.len());
    let mut at = 0usize;
    for layouts in &sampled {
        offsets.push(at);
        at += layouts.len();
    }
    let price = |slot: usize| {
        let decision = &decisions[priced[slot]];
        let layouts = &sampled[slot];
        let start = offsets[slot];
        let theirs: &dyn Bidder = match oracle.as_ref().filter(|_| rollout_bba) {
            Some(bba) => bba,
            None => &ctx.policy,
        };
        let advantage = advantages(
            decision,
            layouts,
            &tables[start..start + layouts.len()],
            &ctx.policy,
            theirs,
            ctx.vul,
        );
        (priced[slot], advantage)
    };
    let scored: Vec<(usize, Vec<Advantage>)> = if rollout_bba {
        (0..priced.len()).map(price).collect()
    } else {
        (0..priced.len()).into_par_iter().map(price).collect()
    };
    let rolled = started.elapsed() - walked - drawn - solved;

    // ── The census ───────────────────────────────────────────────────────────
    let mut nodes: BTreeMap<String, Bucket> = BTreeMap::new();
    for decision in &decisions {
        let bucket = nodes.entry(decision.key.clone()).or_default();
        bucket.seen += 1;
        if decision.thin {
            bucket.thin += 1;
        } else if decision.candidates.is_empty() {
            bucket.flat += 1;
        }
    }
    for (index, advantage) in scored {
        let decision = &decisions[index];
        let mut bucket = Bucket {
            priced: 1,
            priced_pass: usize::from(decision.own == Call::Pass),
            ..Bucket::default()
        };
        bucket.deals.push(decision.deal);
        // Select on one M-layout pool and evaluate that same call on the
        // independent M-layout pool.
        let mut winner = [decision.own; 2];
        let mut in_sample = [0.0f64; 2];
        let mut held_out = [0.0f64; 2];
        for (call, adv) in decision.candidates.iter().zip(&advantage) {
            for bracket in 0..2 {
                if adv.first[bracket] > in_sample[bracket] {
                    in_sample[bracket] = adv.first[bracket];
                    held_out[bracket] = adv.second[bracket];
                    winner[bracket] = *call;
                }
            }
        }
        for bracket in 0..2 {
            bucket.in_sample_advantage[bracket].push(in_sample[bracket]);
            bucket.held_out_advantage[bracket].push(held_out[bracket]);
            bucket.relabel[bracket] =
                usize::from(winner[bracket] != decision.own && in_sample[bracket] > args.margin);
            bucket.held_out_relabel[bracket] =
                usize::from(winner[bracket] != decision.own && held_out[bracket] > args.margin);
        }
        bucket.held_out_relabel[2] = usize::from(
            winner[0] == winner[1]
                && winner[0] != decision.own
                && held_out[0] > args.margin
                && held_out[1] > args.margin,
        );
        // "Both" is the same call clearing the margin on each scorer — the
        // measurement playbook's non-inferiority reading, applied to a label.
        let agreed = winner[0] == winner[1]
            && winner[0] != decision.own
            && in_sample[0] > args.margin
            && in_sample[1] > args.margin;
        bucket.relabel[2] = usize::from(agreed);
        // D2: what the held-out relabels are *targeting*, against the base rate
        // of Pass among the own calls they displace.
        for rule in 0..3 {
            if bucket.held_out_relabel[rule] > 0 {
                // The `both` rule fired only with `winner[0] == winner[1]`.
                let target = winner[rule.min(1)];
                bucket.held_out_pass[rule][0] = usize::from(target == Call::Pass);
                bucket.held_out_pass[rule][1] = usize::from(decision.own == Call::Pass);
            }
        }
        if agreed {
            *bucket
                .swaps
                .entry(format!("{} -> {}", decision.own, winner[0]))
                .or_default() += 1;
        }
        nodes.entry(decision.key.clone()).or_default().merge(bucket);
    }

    let total = |f: &dyn Fn(&Bucket) -> usize| nodes.values().map(f).sum::<usize>();
    let (seen, thin, flat, priced_n) = (
        total(&|b: &Bucket| b.seen),
        total(&|b: &Bucket| b.thin),
        total(&|b: &Bucket| b.flat),
        total(&|b: &Bucket| b.priced),
    );
    println!(
        "seed {base}  deals {}  walk {}  population {}  M {} (2M draws)  k {}  eps {:.0e}  T {}  margin {:.2} IMPs  opponent {}  vulnerability {}",
        args.count,
        args.walk.name(),
        args.population.name(),
        args.layouts,
        args.top_k,
        args.epsilon,
        args.temperature,
        args.margin,
        if rollout_bba { "bba" } else { "self" },
        ctx.vul,
    );
    // The corpus-pass denominator: what fraction of a walk's choice-bearing
    // decisions the net actually answers, hence what fraction is worth pricing.
    let walked_slices: u64 = slices.iter().sum();
    #[allow(clippy::cast_precision_loss)] // a share, not money
    let share = |n: u64| 100.0 * n as f64 / walked_slices.max(1) as f64;
    #[allow(clippy::cast_precision_loss)] // a density, not money
    let per_deal = |n: u64| n as f64 / args.count.max(1) as f64;
    println!(
        "slice mix over {walked_slices} choice-bearing decisions ({:.2} per deal): {}",
        per_deal(walked_slices),
        SLICES
            .iter()
            .zip(slices)
            .map(|(name, n)| format!("{name} {n} ({:.1}%, {:.2}/deal)", share(n), per_deal(n)))
            .collect::<Vec<_>>()
            .join("  "),
    );
    #[allow(clippy::cast_precision_loss)] // a density, not money
    let density = seen as f64 / args.count.max(1) as f64;
    println!(
        "in-population decisions with a choice {seen} ({density:.2} per deal, stride {})  \
         thin {thin}  no alternative {flat}  rolled out {priced_n}  distinct nodes {}",
        args.stride,
        nodes.len(),
    );
    println!(
        "replay-sampler starvation: {underfilled} of {attempted_n} decisions \
         returned fewer than 2M layouts and were skipped ({:.2}%)",
        100.0 * underfilled as f64 / attempted_n.max(1) as f64,
    );
    println!(
        "walk {:.1}s  sample {:.1}s  solve {:.1}s ({} layouts)  roll {:.1}s",
        walked.as_secs_f64(),
        drawn.as_secs_f64(),
        solved.as_secs_f64(),
        all_layouts.len(),
        rolled.as_secs_f64(),
    );
    if priced_n == 0 {
        return;
    }

    #[allow(clippy::cast_precision_loss)] // census counts, not money
    let pct = |n: usize| 100.0 * n as f64 / priced_n as f64;
    println!(
        "\nrelabel rate at margin {:.2} IMPs — in-sample (chosen and scored on \
         the first M) vs held-out (same call scored on the second M):",
        args.margin
    );
    for (label, rule) in BRACKETS.iter().chain(["both"].iter()).zip(0..3) {
        let note = if rule == 2 {
            "  (same winning call clears both)"
        } else {
            ""
        };
        println!(
            "  {label:>15}  in-sample {:>6} {:>6.2}%   held-out {:>6} {:>6.2}%{note}",
            total(&|b: &Bucket| b.relabel[rule]),
            pct(total(&|b: &Bucket| b.relabel[rule])),
            total(&|b: &Bucket| b.held_out_relabel[rule]),
            pct(total(&|b: &Bucket| b.held_out_relabel[rule])),
        );
    }

    // D2 — a double-dummy oracle cannot price what a bid conceals, so it prefers
    // passing. Pass over-represented among the PD-only targets but not the
    // both-scorer ones is the doubling artifact; over-represented in both is the
    // blind spot, and a reason to distrust the sign whatever the interval says.
    let priced_pass = total(&|b: &Bucket| b.priced_pass);
    println!(
        "\nPass among held-out relabel targets, against its rate among the calls they \
         displace — {priced_pass} of {priced_n} rolled-out decisions ({:.2}%) already pass:",
        pct(priced_pass),
    );
    for (label, rule) in BRACKETS.iter().chain(["both"].iter()).zip(0..3) {
        let fired = total(&|b: &Bucket| b.held_out_relabel[rule]);
        #[allow(clippy::cast_precision_loss)] // census counts, not money
        let share = |n: usize| 100.0 * n as f64 / fired.max(1) as f64;
        let (target, displaced) = (
            total(&|b: &Bucket| b.held_out_pass[rule][0]),
            total(&|b: &Bucket| b.held_out_pass[rule][1]),
        );
        println!(
            "  {label:>15}  relabels {fired:>6}   target Pass {target:>6} {:>6.2}%   \
             own Pass {displaced:>6} {:>6.2}%",
            share(target),
            share(displaced),
        );
    }

    // The winner's curse, measured: select once, then compare that call's value
    // on the selection and independent validation pools.
    println!("\nbest-candidate advantage per rolled-out decision (95% CI clustered by deal):");
    for (label, bracket) in BRACKETS.iter().zip(0..2) {
        let in_sample: Vec<(usize, f64)> = nodes
            .values()
            .flat_map(|b| {
                b.deals
                    .iter()
                    .copied()
                    .zip(b.in_sample_advantage[bracket].iter().copied())
            })
            .collect();
        let held_out: Vec<(usize, f64)> = nodes
            .values()
            .flat_map(|b| {
                b.deals
                    .iter()
                    .copied()
                    .zip(b.held_out_advantage[bracket].iter().copied())
            })
            .collect();
        let (in_sample_mean, in_sample_ci) = clustered_mean_with_ci(&in_sample);
        let (held_out_mean, held_out_ci) = clustered_mean_with_ci(&held_out);
        println!(
            "  {label:>15}  in-sample {in_sample_mean:+.4} ± {in_sample_ci:.4}   \
             held-out {held_out_mean:+.4} ± {held_out_ci:.4}   curse {:+.4} IMPs",
            in_sample_mean - held_out_mean,
        );
    }

    let mut ranked: Vec<(&String, &Bucket)> =
        nodes.iter().filter(|(_, b)| b.relabel[2] > 0).collect();
    ranked.sort_by_key(|(key, b)| (std::cmp::Reverse(b.relabel[2]), key.as_str()));
    println!(
        "\n{:>7} {:>7} {:>7} {:>7}  {:<30} top swap",
        "seen", "flat", "priced", "relabel", "node"
    );
    for (key, bucket) in ranked.into_iter().take(args.nodes) {
        let top = bucket
            .swaps
            .iter()
            .max_by_key(|(swap, n)| (*n, std::cmp::Reverse(swap.as_str())))
            .map_or(String::new(), |(swap, n)| format!("{swap} x{n}"));
        println!(
            "{:>7} {:>7} {:>7} {:>7}  {:<30} {top}",
            bucket.seen, bucket.flat, bucket.priced, bucket.relabel[2], key,
        );
    }
}
