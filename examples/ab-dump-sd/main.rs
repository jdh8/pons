//! sd-lead paired delta of two aligned `bba-gen` dumps — the middle bracket.
//!
//! [`ab-dump-diff`](../ab-dump-diff/main.rs) scores each arm's reached contract
//! with double-dummy play (plain) or a perfect-defense doubler (pd).  This
//! third scorer prices the one information seam DD gets most wrong: the opening
//! lead.  The leader (declarer's LHO) chooses a card *single-dummy* — best over
//! `sd_worlds` layouts consistent with the auction as the scored arm's
//! agreements disclose it (an East–West leader decodes every one of our calls
//! off its authoring rule, [`common::TableReader`]; `--legacy-leader-reading`
//! restores the pre-2026-10-16 reader, which showed it our alerted calls only)
//! — and play thereafter is double-dummy on the actual deal (`single_dummy_leads`;
//! Pavlicek: a blind lead pays declarer ≈+0.3 tricks at the 1NT level).  Truth
//! sits between plain DD (under-punishes overbids) and pd (over-punishes); the
//! blind lead is the realistic middle, so a plain-win / pd-loss "doubling
//! artifact" that is *also* sd-positive is real value pd merely over-punishes.
//!
//! Compared at **auction** granularity, not contract: the same contract reached
//! through a different auction can draw a different blind lead, so an sd swing
//! exists wherever the two `table_a` auctions differ.
//!
//! `--sd-declarer` swaps in the **sd-declarer playout** bracket
//! (`single_dummy_playout`): after the blind lead, declarer plays every card
//! single-dummy over range-consistent worlds while the defense stays
//! double-dummy.  This prices the seam the lead bracket can't — declarer's
//! misguesses, which dominate at the slam level where every DD-play scorer is
//! optimistic for the arm bidding more slams.  Sequential per board (no
//! cross-board pooling), so reserve it for slam-sized divergent sets.
//!
//! Each arm also reports a `Sampling` line per scorer endpoint: worlds asked
//! for, how many the fallback tier filled because the auction reading starved
//! the constrained sample (unconstrained deals for the blind lead, hard-only
//! splits mid-play), the share of priced boards with any fallback world, and
//! the constrained sampler's draws and acceptance.  The `Delta`/`Verdict`
//! lines are unchanged; this only shows how much of them rests on fallback.
//!
//! `--by holding` splits the same delta into the `1NT (3x) X -` leave-in slice
//! (`common::holding_key`), so the sd bracket — the one that settled the v1
//! refutation — can be read per holding instead of as one number.
//!
//! ```text
//! cargo run --release --features serde --example ab-dump-sd -- on.json off.json
//! cargo run --release --features serde --example ab-dump-sd -- --sd-declarer on.json off.json
//! ```

use clap::Parser;
use contract_bridge::{AbsoluteVulnerability, Contract, Seat};
use pons::american;
use pons::bidding::SampleStats;
use pons::bidding::american::{FreeBidStyle, NegativeDoubleShape};
use pons::scoring::{imps, ns_score_pd_tricks, ns_score_tricks};
use pons::single_dummy::{LeadQuestion, single_dummy_leads_with_stats};
use rand::SeedableRng;
use rand::rngs::StdRng;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{TableReader, dump_density, mean_with_ci, verdict_line};

#[derive(Parser)]
struct Args {
    /// Dump bid with the feature ON (its `table_a` auction is scored).
    /// A directory folds its `shard-*.json` into one solve.
    on: String,
    /// Dump bid with the feature OFF, same seed/deals (the baseline).
    /// A directory folds its `shard-*.json` into one solve.
    off: String,
    /// Re-price at this vulnerability instead of the dump's
    #[arg(short, long)]
    vulnerability: Option<AbsoluteVulnerability>,
    /// Worlds sampled per blind lead (the validated GTO setting is 16)
    #[arg(long, default_value_t = 16)]
    sd_worlds: usize,
    /// Share of random deals the dumps' `--filter-*` accepted, for the
    /// verdict's δ / density margin; an enriched dump without it withholds
    /// the verdict.  Must be in `(0, 1]`
    #[arg(long, value_parser = common::parse_density)]
    density: Option<f64>,
    /// Seed for the world-sampling RNG (report it to reproduce a run)
    #[arg(long, default_value_t = 20_240_607)]
    sd_seed: u64,
    /// Score with the **sd-declarer playout** instead of the sd lead: after
    /// the blind lead, declarer plays every card single-dummy over
    /// `--declarer-worlds` range-consistent worlds while the defense stays
    /// double-dummy.  The pessimist bracket for slam aggression (plain DD is
    /// the optimist); far slower per board — use on slam-sized divergent sets
    #[arg(long, default_value_t = false)]
    sd_declarer: bool,
    /// Worlds sampled per declarer decision (with --sd-declarer)
    #[arg(long, default_value_t = 16)]
    declarer_worlds: usize,
    /// Read the ON arm's auctions with responder's free bids authored
    /// (`competition.free_bids`; opener's answers ride along)
    #[arg(long, default_value_t = false)]
    on_ns_free_bids: bool,
    /// Read the ON arm's auctions under this negative-double school:
    /// modern (shipped default) | both-majors | cachalot | sputnik
    /// (`competition.negative_double_shape`; all but both-majors imply the free bids)
    #[arg(long, default_value = "modern")]
    on_ns_negative_double_shape: String,
    /// Same for the OFF arm (the shipped `modern` unless a control arm plays
    /// another school, e.g. `both-majors` for the pre-Modern system)
    #[arg(long, default_value = "modern")]
    off_ns_negative_double_shape: String,
    /// Read the ON arm's auctions under this 2-level free-bid style:
    /// forcing (shipped default) | negative | transfer (`competition.free_bid_style`)
    #[arg(long, default_value = "forcing")]
    on_ns_free_bid_style: String,
    /// Read the ON arm's auctions with this free-1NT floor (`competition.free_1nt_floor`;
    /// discloses a tightened `1X (1Y) 1NT` band to the blind leader)
    #[arg(long, default_value_t = 6)]
    on_ns_free_1nt_floor: u8,
    /// Read the ON arm's auctions with the 2-level minor overcall tightened to
    /// 15+ (`defense.two_level_minor_overcall_tight`)
    #[arg(long, default_value_t = false)]
    on_ns_two_level_minor_overcall_tight: bool,
    /// Read the ON arm's auctions with the 2-level minor overcall tightened to
    /// 15+ when vulnerable only (`defense.two_level_minor_overcall_vul_tight`)
    #[arg(long, default_value_t = false)]
    on_ns_two_level_minor_overcall_vul_tight: bool,
    /// Read the ON arm's direct single-jump major overcall as weak and natural
    /// (`defense.direct_weak_jump_overcall`).
    #[arg(long, default_value_t = false)]
    on_ns_direct_weak_jump_overcall: bool,
    /// Read the OFF arm's direct single-jump major overcall as weak and natural.
    #[arg(long, default_value_t = false)]
    off_ns_direct_weak_jump_overcall: bool,
    /// Read the ON arm with the exact-six weak `(1♣) 2♦` extension.
    #[arg(long, default_value_t = false)]
    on_ns_direct_minor_weak_jump_overcall: bool,
    /// Read the OFF arm with the exact-six weak `(1♣) 2♦` extension.
    #[arg(long, default_value_t = false)]
    off_ns_direct_minor_weak_jump_overcall: bool,
    /// Read the ON arm's natural two-level overcall of their 1NT with this raw-HCP
    /// floor on top of the points band (`defense.natural_overcall_hcp_floor`;
    /// defaults to the shipped 0, so an arm with a floor must pass it explicitly).
    #[arg(long, default_value_t = 0)]
    on_ns_nt_overcall_hcp_floor: u8,
    /// Read the OFF arm's natural two-level overcall of their 1NT with this raw-HCP floor.
    #[arg(long, default_value_t = 0)]
    off_ns_nt_overcall_hcp_floor: u8,
    /// Read the ON arm with `(1NT) 2x - ?` left to the instinct floor
    /// (`defense.natural_overcall_advance_enabled`; the advance is shipped on, so
    /// this is the control arm's flag).
    #[arg(long, default_value_t = false)]
    on_no_ns_nt_overcall_advance: bool,
    /// Read the OFF arm with `(1NT) 2x - ?` left to the instinct floor.
    #[arg(long, default_value_t = false)]
    off_no_ns_nt_overcall_advance: bool,
    /// Read the ON arm with the takeout double barred on a six-card unbid minor
    /// (`defense.suppress_long_minor_takeout`).
    #[arg(long, default_value_t = false)]
    on_ns_suppress_long_minor_takeout: bool,
    /// Read the OFF arm with the six-card-minor takeout bar.
    #[arg(long, default_value_t = false)]
    off_ns_suppress_long_minor_takeout: bool,
    /// Read the ON arm with the level-split overcall/double seam and the authored
    /// doubler rebids (`defense.defensive_seam_split`).
    #[arg(long, default_value_t = false)]
    on_ns_defensive_seam_split: bool,
    /// Read the OFF arm with the level-split seam.
    #[arg(long, default_value_t = false)]
    off_ns_defensive_seam_split: bool,
    /// Read the ON arm with an unbid five-card major barred from the 1NT
    /// overcall (`defense.nt_overcall_no_major`).
    #[arg(long, default_value_t = false)]
    on_ns_nt_overcall_no_major: bool,
    /// Read the ON arm with that denial confined to 15–17 HCP
    /// (`defense.nt_overcall_major_band`).
    #[arg(long, default_value_t = false)]
    on_ns_nt_overcall_major_band: bool,
    /// Read the ON arm's auctions with this natural 1NT-defense suit-overcall
    /// band `LO:HI` (`ReadingProfile::natural_overcall_points`; the band sweep's arms differ
    /// only here, so the blind leader must know which one bid)
    #[arg(long, default_value = "8:14")]
    on_ns_overcall: String,
    /// Same for the OFF arm (the sweep's baseline is 8:14)
    #[arg(long, default_value = "8:14")]
    off_ns_overcall: String,
    /// Read the ON arm's auctions with this `hcp` band `LO:HI` on the 2NT
    /// overcall of their weak two (`defense.weak_two_notrump_points`; same reason
    /// as `--on-ns-overcall` — the leader must know which band bid 2NT)
    #[arg(long, default_value = "16:17")]
    on_ns_weak_two_nt_points: String,
    /// Same for the OFF arm (the shipped default is 16:17)
    #[arg(long, default_value = "16:17")]
    off_ns_weak_two_nt_points: String,
    /// Read the ON arm's auctions with the Gladiator advances of that 2NT
    /// authored (`defense.weak_two_notrump_advances_enabled`), which is what makes `3♣`
    /// a relay rather than natural to the leader
    #[arg(long, default_value_t = false)]
    on_ns_weak_two_nt_advances: bool,
    /// Same for the OFF arm
    #[arg(long, default_value_t = false)]
    off_ns_weak_two_nt_advances: bool,
    /// Read the ON arm's auctions with the defense to their Stayman
    /// (`defense.stayman_defense_enabled`), so `(1NT) - (2♣) X` reads as
    /// lead-directing clubs to the blind leader
    #[arg(long, default_value_t = false)]
    on_ns_stayman_defense: bool,
    /// Split the delta by `holding` — opener's holding in their suit over
    /// `1NT (3x) X -` (`common::holding_key`); ON must be the leave-in arm.
    /// `stayman` splits by our direct call over `(1NT) - (2♣)` in each arm
    /// (`common::stayman_key`); `stayman-x` by the first divergence after
    /// both arms double `2♣` (`common::stayman_x_key`)
    #[arg(long)]
    by: Option<String>,
    /// Show this many of the biggest swings (each way)
    #[arg(long, default_value_t = 8)]
    show: usize,
    /// Reproduce the pre-2026-10-16 reader, under which East–West (the
    /// foreign pair, and the leader whenever we declare) saw only our
    /// *alerted* calls, so a manipulation of an unalerted call's disclosure
    /// (`--on-ns-overcall`) never reached it.  Default: every NS call decodes
    /// off its authoring rule for an East–West reader (`common::TableReader`)
    #[arg(long, default_value_t = false)]
    legacy_leader_reading: bool,
}

/// One arm's sampling tally at one scorer endpoint: how the priced worlds
/// were dealt, summed over its boards
#[derive(Default)]
struct Sampling {
    /// Boards priced (a pass-out samples nothing and is not counted)
    boards: usize,
    /// Boards with any fallback world (or any unfilled one)
    degraded: usize,
    /// Every board's stats, summed
    stats: SampleStats,
}

impl Sampling {
    fn add(&mut self, stats: SampleStats) {
        self.boards += 1;
        self.degraded += usize::from(stats.degraded());
        self.stats += stats;
    }

    /// `Sampling <arm> <endpoint>: …` — one line, never starting with
    /// `Delta`/`Verdict`, so the scripts that grep those are unaffected
    #[allow(clippy::cast_precision_loss)]
    fn line(&self, arm: &str, endpoint: &str) -> String {
        let pct = |part: usize, whole: usize| 100.0 * part as f64 / whole.max(1) as f64;
        let s = &self.stats;
        format!(
            "Sampling {arm} {endpoint}: {} boards, {} worlds, {} fallback ({:.2}%), \
             {} boards with fallback ({:.2}%), {} draws (acceptance {:.3}%)",
            self.boards,
            s.requested,
            s.fallback,
            pct(s.fallback, s.requested),
            self.degraded,
            pct(self.degraded, self.boards),
            s.draws,
            pct(s.accepted, s.draws),
        )
    }
}

fn main() {
    let args = Args::parse();
    let on = common::load_dump(&args.on);
    let off = common::load_dump(&args.off);
    assert_eq!(on.boards.len(), off.boards.len(), "dumps must be aligned");
    let vul = args.vulnerability.unwrap_or(on.vulnerability);
    let n = on.boards.len();

    // Leader-view partnerships (knobs are read at book-construction time).  The OFF
    // arm always reads with the default book; the ON arm discloses whatever
    // knobs its auctions were bid with.
    let shape = |name: &str| match name {
        "both-majors" => NegativeDoubleShape::BothMajors,
        "modern" => NegativeDoubleShape::Modern,
        "cachalot" => NegativeDoubleShape::Cachalot,
        "sputnik" => NegativeDoubleShape::Sputnik,
        other => panic!("unknown negative-double shape {other}"),
    };
    let style = |name: &str| match name {
        "forcing" => FreeBidStyle::Forcing,
        "negative" => FreeBidStyle::Negative,
        "transfer" => FreeBidStyle::Transfer,
        other => panic!("unknown free-bid style {other}"),
    };
    let band = |spec: &str| -> (u8, u8) {
        spec.split_once(':')
            .and_then(|(lo, hi)| Some((lo.parse().ok()?, hi.parse().ok()?)))
            .unwrap_or_else(|| panic!("overcall band must be LO:HI, got {spec:?}"))
    };
    let (lo, hi) = band(&args.on_ns_overcall);
    // Each arm carries its own competitive, defensive, and reading values.
    let mut on_arm = pons::bidding::agreements::Agreements::default();
    on_arm.decision.reading.natural_overcall_points = (lo, hi);
    on_arm.competition.free_bids = args.on_ns_free_bids;
    on_arm.competition.negative_double_shape = shape(&args.on_ns_negative_double_shape);
    on_arm.competition.free_bid_style = style(&args.on_ns_free_bid_style);
    on_arm.competition.free_1nt_floor = args.on_ns_free_1nt_floor;
    on_arm.defense.two_level_minor_overcall_tight = args.on_ns_two_level_minor_overcall_tight;
    on_arm.defense.two_level_minor_overcall_vul_tight =
        args.on_ns_two_level_minor_overcall_vul_tight;
    on_arm.defense.direct_weak_jump_overcall = args.on_ns_direct_weak_jump_overcall;
    on_arm.defense.direct_minor_weak_jump_overcall = args.on_ns_direct_minor_weak_jump_overcall;
    on_arm.defense.natural_overcall_hcp_floor = args.on_ns_nt_overcall_hcp_floor;
    on_arm.defense.natural_overcall_advance_enabled = !args.on_no_ns_nt_overcall_advance;
    on_arm.defense.suppress_long_minor_takeout = args.on_ns_suppress_long_minor_takeout;
    on_arm.defense.defensive_seam_split = args.on_ns_defensive_seam_split;
    on_arm.defense.nt_overcall_no_major = args.on_ns_nt_overcall_no_major;
    on_arm.defense.nt_overcall_major_band = args.on_ns_nt_overcall_major_band;
    on_arm.defense.weak_two_notrump_points = band(&args.on_ns_weak_two_nt_points);
    on_arm.defense.weak_two_notrump_advances_enabled = args.on_ns_weak_two_nt_advances;
    on_arm.defense.stayman_defense_enabled = args.on_ns_stayman_defense;
    let read_arm = |arm: &pons::bidding::agreements::Agreements| {
        let partnership = american(arm).bind();
        if args.legacy_leader_reading {
            TableReader::legacy(partnership)
        } else {
            TableReader::new(partnership)
        }
    };
    let reader_on = read_arm(&on_arm);
    let (lo, hi) = band(&args.off_ns_overcall);
    // The OFF arm is the shipped pole, spelled out rather than inherited.
    let mut off_arm = pons::bidding::agreements::Agreements::default();
    off_arm.decision.reading.natural_overcall_points = (lo, hi);
    off_arm.competition.free_bids = false;
    off_arm.competition.negative_double_shape = shape(&args.off_ns_negative_double_shape);
    off_arm.competition.free_bid_style = FreeBidStyle::Forcing;
    off_arm.competition.free_1nt_floor = 6;
    off_arm.defense.two_level_minor_overcall_tight = false;
    off_arm.defense.two_level_minor_overcall_vul_tight = false;
    off_arm.defense.direct_weak_jump_overcall = args.off_ns_direct_weak_jump_overcall;
    off_arm.defense.direct_minor_weak_jump_overcall = args.off_ns_direct_minor_weak_jump_overcall;
    off_arm.defense.natural_overcall_hcp_floor = args.off_ns_nt_overcall_hcp_floor;
    off_arm.defense.natural_overcall_advance_enabled = !args.off_no_ns_nt_overcall_advance;
    off_arm.defense.suppress_long_minor_takeout = args.off_ns_suppress_long_minor_takeout;
    off_arm.defense.defensive_seam_split = args.off_ns_defensive_seam_split;
    off_arm.defense.weak_two_notrump_points = band(&args.off_ns_weak_two_nt_points);
    off_arm.defense.weak_two_notrump_advances_enabled = args.off_ns_weak_two_nt_advances;
    let reader_off = read_arm(&off_arm);

    let mut rng = StdRng::seed_from_u64(args.sd_seed);
    // Each entry is that board's `[plain, perfect-defense]` SD price — one
    // trick count scored twice (`ns_score_tricks` / `ns_score_pd_tricks`).
    let mut on_score = vec![[0i64; 2]; n];
    let mut off_score = vec![[0i64; 2]; n];
    let mut fired = 0usize;
    // `[ON, OFF]` arm × `[lead, playout]` endpoint; the lead scorer fills
    // only the lead column.
    let mut sampling: [[Sampling; 2]; 2] = Default::default();
    if args.sd_declarer {
        // sd-declarer playout: inherently sequential per board (each card's
        // choice depends on the last), so no cross-board pooling here — run
        // it on slam-sized divergent sets.
        for (i, (a, b)) in on.boards.iter().zip(&off.boards).enumerate() {
            assert_eq!(a.deal, b.deal, "dumps not seed-aligned");
            if a.table_a == b.table_a {
                continue; // identical auction ⇒ identical playout ⇒ swing 0
            }
            fired += 1;
            let [on_tally, off_tally] = &mut sampling;
            for (score, tally, board, reader) in [
                (&mut on_score[i], on_tally, a, &reader_on),
                (&mut off_score[i], off_tally, b, &reader_off),
            ] {
                let sd = common::sd_declarer_ns_score(
                    &board.table_a,
                    board.dealer,
                    &board.deal,
                    reader,
                    vul,
                    &mut rng,
                    args.sd_worlds,
                    args.declarer_worlds,
                );
                *score = sd.playout;
                if sd.level > 0 {
                    tally[0].add(sd.sampling[0]);
                    tally[1].add(sd.sampling[1]);
                }
            }
        }
    } else {
        // Build every blind-lead question on the boards whose auctions differ;
        // a pass-out contributes score 0 (its arm is simply omitted here).
        let mut pending: Vec<(usize, bool, Contract, Seat)> = Vec::new();
        let mut questions: Vec<LeadQuestion> = Vec::new();
        for (i, (a, b)) in on.boards.iter().zip(&off.boards).enumerate() {
            assert_eq!(a.deal, b.deal, "dumps not seed-aligned");
            if a.table_a == b.table_a {
                continue; // identical auction ⇒ identical blind lead ⇒ swing 0
            }
            fired += 1;
            for (arm_on, board, reader) in [(true, a, &reader_on), (false, b, &reader_off)] {
                if let Some((contract, declarer, inferences)) =
                    reader.lead_inputs(&board.table_a, board.dealer, vul)
                {
                    pending.push((i, arm_on, contract, declarer));
                    questions.push(LeadQuestion {
                        deal: board.deal,
                        strain: contract.bid.strain,
                        declarer,
                        inferences,
                    });
                }
            }
        }

        const CHUNK: usize = 4096;
        for (asked, chunk) in pending.chunks(CHUNK).zip(questions.chunks(CHUNK)) {
            let (answers, stats) = single_dummy_leads_with_stats(chunk, &mut rng, args.sd_worlds);
            for ((&(i, arm_on, contract, declarer), &(_, tricks)), &stats) in
                asked.iter().zip(&answers).zip(&stats)
            {
                sampling[usize::from(!arm_on)][0].add(stats);
                let t = u8::from(tricks);
                let score = [
                    ns_score_tricks(contract, declarer, t, vul),
                    ns_score_pd_tricks(contract, declarer, t, vul),
                ];
                if arm_on {
                    on_score[i] = score;
                } else {
                    off_score[i] = score;
                }
            }
        }
    }

    // `board_imps` stays the *plain* bracket: it also feeds the worst-boards
    // dump below, which wants the swing on the honest scorer.
    let board_imps: Vec<i64> = (0..n)
        .map(|i| imps(on_score[i][0] - off_score[i][0]))
        .collect();
    let pd_imps: Vec<i64> = (0..n)
        .map(|i| imps(on_score[i][1] - off_score[i][1]))
        .collect();
    #[allow(clippy::cast_precision_loss)]
    {
        let bracket = if args.sd_declarer {
            format!(
                "sd-declarer (lead {} × line {})",
                args.sd_worlds, args.declarer_worlds
            )
        } else {
            format!("sd-lead ({} worlds)", args.sd_worlds)
        };
        println!(
            "{bracket} ON {} vs OFF {} ({n} boards, vul {vul}): {fired} fired ({:.2}%)",
            on.our_label,
            off.our_label,
            100.0 * fired as f64 / n.max(1) as f64,
        );
        // Name the reader: outputs written before this line existed are
        // the legacy reader's, and their sd numbers do not price disclosure.
        println!("leader reading: {}", reader_on.mode());
        // Read the perfect-defense line as the arbiter: plain SD relaxes the
        // defenders' lead *and* never punishes the failures, so it flatters
        // aggression.  (Under `--sd-declarer` the defense already peeks, so
        // that line is a pessimism stress-test rather than a bracket.)
        let density = dump_density(&on, args.density);
        for (tag, scorer, swings) in [
            ("Delta plain           (run − sit)", "plain SD", &board_imps),
            ("Delta perfect-defense (run − sit)", "SD-PD", &pd_imps),
        ] {
            let (mean, ci) = mean_with_ci(swings);
            let total: i64 = swings.iter().sum();
            println!(
                "{tag}: {total:+} IMPs, {mean:+.4} IMPs/board [95% CI ±{ci:.4}], {:+.3} IMPs/fired",
                total as f64 / fired.max(1) as f64,
            );
            println!("{}", verdict_line(scorer, mean, ci, density));
        }
        for (arm, tally) in ["ON ", "OFF"].into_iter().zip(&sampling) {
            println!("{}", tally[0].line(arm, "sd-lead"));
            if args.sd_declarer {
                println!("{}", tally[1].line(arm, "sd-playout"));
            }
        }
    }

    #[allow(clippy::cast_precision_loss)]
    if let Some(by) = &args.by {
        // key -> (fired, sd-plain, sd-PD).  The key is `Some` on exactly the
        // auction-divergent boards, which is exactly this harness's `fired`
        // set, so the bucket counts must re-sum to it.
        let mut buckets: std::collections::HashMap<String, (usize, i64, i64)> =
            std::collections::HashMap::new();
        for (i, (a, b)) in on.boards.iter().zip(&off.boards).enumerate() {
            let key = match by.as_str() {
                "holding" => common::holding_key(a, b),
                "stayman" => common::stayman_key(a, b),
                "stayman-x" => common::stayman_x_key(a, b),
                other => panic!("unknown --by {other}"),
            };
            let Some(key) = key else { continue };
            let entry = buckets.entry(key).or_default();
            entry.0 += 1;
            entry.1 += board_imps[i];
            entry.2 += pd_imps[i];
        }
        let mut rows: Vec<_> = buckets.into_iter().collect();
        rows.sort_by_key(|(_, entry)| entry.1);
        let counted: usize = rows.iter().map(|(_, entry)| entry.0).sum();
        println!("--- --by {by} ({counted} keyed of {fired} fired) ---");
        println!(
            "{:>7} {:>8} {:>10} {:>8} {:>9}  bucket",
            "fired", "plain", "plain/fired", "PD", "PD/fired"
        );
        for (key, (count, plain, pd)) in &rows {
            let per = |total: i64| total as f64 / *count as f64;
            println!(
                "{count:>7} {plain:>+8} {:>+10.2} {pd:>+8} {:>+9.2}  {key}",
                per(*plain),
                per(*pd)
            );
        }
    }

    let mut swings: Vec<(usize, i64)> = board_imps
        .iter()
        .enumerate()
        .filter(|&(_, &imp)| imp != 0)
        .map(|(i, &imp)| (i, imp))
        .collect();
    swings.sort_by_key(|&(_, imp)| imp);
    let show = args.show.min(swings.len());
    if show > 0 {
        println!("--- {show} worst (for the feature) ---");
        for &(i, imp) in swings.iter().take(show) {
            println!(
                "[{imp:+} IMP] {}\n  on:  {}\n  off: {}",
                on.boards[i].deal, on.boards[i].table_a, off.boards[i].table_a
            );
        }
    }
}

#[cfg(test)]
mod tests;
