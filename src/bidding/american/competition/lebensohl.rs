//! Lebensohl after our `1NT` is overcalled
//!
//! The weak `2NT` relay to `3♣`, the cue as game-forcing Stayman, the
//! direct-versus-slow distinction, and the signoff raises.  The transfer
//! ("Rubensohl") variant is [`super::rubensohl`]; [`LebensohlStyle`] picks
//! between them.

use super::super::slam;
use super::penalty_double::{
    DoubleStyle, opener_cooperates_optional, opener_leaves_in_penalty_double, responder_double,
};
use super::rubensohl::{
    clubs_transfer_completion, cue_stayman_answer, kokish_kraft_delayed,
    kokish_kraft_doubler_major_answer, kokish_kraft_doubler_major_invite,
    kokish_kraft_doubler_rebid, kokish_kraft_invite_answer, kokish_kraft_minimum_notrump_answer,
    kokish_kraft_minor_completion, kokish_kraft_minors_answer, kokish_kraft_minors_overcalled,
    kokish_kraft_minors_place, kokish_kraft_responder, kokish_kraft_second_suits,
    kokish_kraft_slam_answer, kokish_kraft_transfer_overcalled, kokish_kraft_transfer_rebid,
    kokish_kraft_two_suiter_answer, lm_2d_both_majors_advance, lm_2d_clubs_ask, lm_2d_clubs_major,
    multi_2d_responder, multi_balance_double, multi_clubs_transfer_completion,
    multi_fit_search_place, multi_fit_search_rebid, multi_pass_answer, multi_penalty_answer,
    multi_quant_answer, multi_relay_rebid, multi_responder_rebid, multi_signoff_pass,
    multi_stopper_answer, multi_stopper_forcing_rebid, multi_stopper_over_four_spades,
    multi_takeout_answer, stayman_2d_answer, stayman_2d_fit_rebid, transfer_completion,
    transfer_lebensohl_responder, transfer_stayman_2d_responder, transfer_target,
};
use super::*;

/// Which Lebensohl package the competitive book carries over our overcalled
/// `1NT` (Section 5)
///
/// Terminology: *Rubensohl* proper makes `2NT` an artificial **club** transfer;
/// the transfer styles here keep the weak `2NT` **relay**, which makes them
/// *Transfer Lebensohl*.
///
/// - `Off` — no Lebensohl node; responder falls to the instinct floor.
/// - `Plain` — weak `2NT` relay / sign-off vs strong direct `3NT` / forcing
///   3-level; matches BBA's 21GF. The prior default (+0.26 IMPs/divergent vs the
///   floor, 200k boards).
/// - `Transfer` — **the default.** Larry Cohen's *Transfer Lebensohl*: 3-level
///   bids transfer up the line *through* the adverse suit, the cue is Stayman, and
///   a transfer to a suit above theirs is INV+ so opener is driven to game (the
///   anti-stranding fix for the earlier transfer-Lebensohl attempt that stranded
///   game hands in partscores). Over `(2♥)`/`(2♠)`/`(2♣)` that is the whole story;
///   it measures **+0.46/+1.24 IMPs/divergent (none/both) vs plain Lebensohl**
///   (`lebensohl-ab`, 200k boards each), and +0.35/+0.05 vs the bare floor. Over
///   `(2♦)` it additionally frees `3♣` for game-forcing Stayman (Smolen after
///   opener's `3♦` denial), reshuffles the 3-level transfers to direct Jacoby
///   (`3♦`→♥, `3♥`→♠, `3♠`→♣ — the `3♠`→♣ leg a forced game-force, its completion
///   `4♣`), and adds Leaping Michaels `4♦` (both majors) / `4♣` (clubs + a major).
///   That `(2♦)` Smolen package is worth **+0.020/+0.024 IMPs/board,
///   +2.286/+2.822 IMPs/divergent (none/both)** over Cohen-pure-over-`(2♦)`
///   (`lebensohl-ab`, 200k filtered each), and it also wins after a takeout double
///   of a weak `2♦` (**+0.014/+0.019 IMPs/board, +1.77/+2.52 IMPs/divergent**,
///   `sohl-after-double-ab`) — which is why the advancer carries it too.
///
/// (True Rubensohl — `2NT` an artificial **club** transfer, low transfers two-way —
/// was implemented and measured worse than `Transfer` (DD `−0.017/−0.046`,
/// perfect-defense `+0.001/−0.023 IMPs/board` none/both) and removed: its only edge
/// was DD-blind right-siding, and jdh8 prefers the Smolen+LM-over-minors /
/// Cohen-over-majors split that `Transfer` carries. See `docs/ai-bidder/21gf-ledger.md`.)
///
/// (An earlier standard-low-Stayman + Smolen hybrid over *both* `(2♦)` and `(2♥)`
/// — no Jacoby reshuffle, no Leaping Michaels — measured DD `−1.31/−1.76 IMPs/div`
/// and was reverted. The narrowed `(2♦)`-only package that `Transfer` now carries
/// *wins*: the Jacoby reshuffle plus Leaping Michaels add genuine fit-finding (5-3
/// major games through Stayman+Smolen, 5-5 major games through Leaping Michaels)
/// that the perfect-defense measure credits — unlike the reverted hybrid, whose
/// only gain was DD-blind right-siding. See `docs/ai-bidder/21gf-ledger.md`.)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LebensohlStyle {
    /// Responder falls to the instinct floor (no Lebensohl node)
    Off,
    /// Plain Lebensohl (weak relay vs forcing 3-level) — the prior default
    Plain,
    /// Transfer Lebensohl (Larry Cohen's `2NT`-relay transfers) — the default;
    /// over `(2♦)` it adds `3♣`-Stayman + Smolen, Jacoby transfers
    /// (`3♦`→♥/`3♥`→♠/`3♠`→♣), and Leaping Michaels `4♣`/`4♦`
    Transfer,
}

/// Author responder's direct `3NT` over the overcall at `weight`, honoring the
/// stopper ([`direct_3nt_stopper`]) and trap-pass ([`trap_pass`]) toggles. The
/// trap denies a too-good stopper (`suit_hcp(over, ..=4)`). The `&`-chained
/// constraints have distinct types, so each combination is authored in its own arm.
pub(super) fn author_direct_3nt(
    rules: Rules,
    weight: i16,
    over: Suit,
    agreements: &Agreements,
) -> Rules {
    let nt = Bid::new(3, Strain::Notrump);
    match (
        agreements.competition.direct_3nt_stopper,
        agreements.competition.trap_pass,
    ) {
        (true, true) => rules.rule(
            nt,
            weight,
            points(10..) & stopper_in(over) & suit_hcp(over, ..=4),
        ),
        (true, false) => rules.rule(nt, weight, points(10..) & stopper_in(over)),
        (false, true) => rules.rule(nt, weight, points(10..) & suit_hcp(over, ..=4)),
        (false, false) => rules.rule(nt, weight, points(10..)),
    }
}

/// Whether the weak natural escape is floored (and opener may raise it)
pub(super) fn natural_floor_on(agreements: &Agreements) -> bool {
    let natural_floor = agreements.competition.natural_floor;
    natural_floor.0 > 0 || natural_floor.1 > 0
}

/// The HCP floor on the weak natural escape (`0` = none) — a bound, so the
/// constraint type stays stable whether or not the floor is engaged.
pub(super) fn natural_floor_hcp(agreements: &Agreements) -> u8 {
    agreements.competition.natural_floor.0
}

/// The total-points floor on the weak natural escape (`0` = none)
pub(super) fn natural_floor_pts(agreements: &Agreements) -> u8 {
    agreements.competition.natural_floor.1
}

/// Whether the `(2♣)`-as-Landy counter-defense is engaged — a fact about the
/// opponents (their disclosed `2♣`), not a knob of ours
fn defense_2c_landy(agreements: &Agreements) -> bool {
    agreements.decision.their.two_clubs_landy
}

/// Whether their `(2♦)` is a Multi (campaign package N4) — the same channel:
/// a fact about the opponents' disclosed `2♦`.  Only the Transfer style has a
/// `(2♦)` leg to re-key; Plain keeps its natural table.
fn defense_2d_multi(agreements: &Agreements) -> bool {
    agreements.decision.their.two_diamonds_multi
        && agreements.competition.lebensohl_style == LebensohlStyle::Transfer
}

/// Whether the Kokish–Kraft variant owns the `(2♦)` Multi lane
/// ([`CompetitionKnobs::multi_kokish_kraft`][crate::bidding::agreements::CompetitionKnobs::multi_kokish_kraft])
///
/// The disclosure *and* the knob, so the knob is inert while their `2♦` is
/// undeclared, natural, or the Plain Lebensohl style is selected.
fn kokish_kraft(agreements: &Agreements) -> bool {
    defense_2d_multi(agreements) && agreements.competition.multi_kokish_kraft
}

/// Whether the Landy counter carries the N1b minor cues
///
/// N1c ([`CompetitionKnobs::defense_2c_landy_transfer`]) keeps the cues
/// verbatim and only re-rungs what sits below them, so it implies N1b.
fn landy_cues(agreements: &Agreements) -> bool {
    agreements.competition.defense_2c_landy_cues || landy_transfer(agreements)
}

/// Whether the Landy counter's minors are re-rung around the club transfer (N1c)
///
/// The N1d/N1e/N1f refinements below each imply it — they are increments over
/// the N1c structure, and the A/B arms stack them in that order.
fn landy_transfer(agreements: &Agreements) -> bool {
    agreements.competition.defense_2c_landy_transfer
        || landy_cue_floor(agreements)
        || landy_fit_answers(agreements)
        || landy_competition(agreements)
        || landy_low_minors(agreements)
        || landy_hcp_rungs(agreements)
}

/// Whether the Landy cues' floor is raised to `points(10..)` (N1d)
fn landy_cue_floor(agreements: &Agreements) -> bool {
    agreements.competition.defense_2c_landy_cue_floor
}

/// Whether the Landy counter's minor rungs are priced a point lower (N1h)
fn landy_low_minors(agreements: &Agreements) -> bool {
    agreements.competition.defense_2c_landy_low_minors
}

/// Whether the Landy counter's minor rungs are graded on `hcp` (N1i)
fn landy_hcp_rungs(agreements: &Agreements) -> bool {
    agreements.competition.defense_2c_landy_hcp_rungs
}

/// Whether opener answers a Landy cue in notrump on doubleton support (N1e)
fn landy_fit_answers(agreements: &Agreements) -> bool {
    agreements.competition.defense_2c_landy_fit_answers
}

/// Whether the Landy counter's interfered tails are authored (N1f)
fn landy_competition(agreements: &Agreements) -> bool {
    agreements.competition.defense_2c_landy_competition
}

/// Whether the Landy counter is the N1j BBA ladder
///
/// When on, responder's whole table and its continuations are
/// [`landy_bba_responder`]'s ([`landy_bba_entries`] registers instead of the
/// stack), and every N1b–N1i structure knob above is **inert** — this table
/// replaces the one they modify.
fn landy_bba(agreements: &Agreements) -> bool {
    agreements.competition.defense_2c_landy_bba
}

/// Whether the N1j ladder carries §N1-lia's deltas
/// ([`CompetitionKnobs::defense_2c_landy_lia`][crate::bidding::agreements::CompetitionKnobs::defense_2c_landy_lia])
///
/// A modifier of the BBA ladder, so it is inert unless [`landy_bba`] selects
/// that table in the first place.
fn landy_lia(agreements: &Agreements) -> bool {
    landy_bba(agreements) && agreements.competition.defense_2c_landy_lia
}

/// Whether the §N1-lia four-level rides South African Texas
/// ([`CompetitionKnobs::landy_texas`][crate::bidding::agreements::CompetitionKnobs::landy_texas])
///
/// A modifier of the jam rung — it moves that rung's call — so it is inert
/// unless the jam is on.  Independent of [`landy_lia`], which is why package C
/// still measured a win after that ladder lost: **default on since
/// 2026-08-31**, an eight-of-eight sweep bought almost entirely by
/// right-siding (96% of divergent boards are the same contract from the other
/// seat; the slam reroute reaches the five level on 5 of 2211).
fn landy_texas(agreements: &Agreements) -> bool {
    agreements.competition.landy_major_jam && agreements.competition.landy_texas
}

/// The single unbid major when `over` is itself a major (the other major)
///
/// `None` when `over` is a minor (then both majors are unbid) — the stopper-split
/// cue is only authored for the single-unbid-major contexts.
pub(super) fn unbid_major(over: Suit) -> Option<Suit> {
    match over {
        Suit::Hearts => Some(Suit::Spades),
        Suit::Spades => Some(Suit::Hearts),
        _ => None,
    }
}

/// The 2NT-relay shape over their `over` overcall: a 5+ suit (not their suit)
/// with 6+ HCP.
///
/// The 6-HCP floor is PD-distilled. A perfect-defense gate (relay only when
/// sampled double-dummy says our 3-level line out-scores defending) declines
/// nearly every sub-6 hand — pushing a near-bust to the 3 level loses on DD,
/// even with a 6-card suit — and this plain HCP floor recovers ~60–80% of that
/// gate's IMPs/board gain over relaying every 5-card suit (A/B, lebensohl-ab,
/// `--pd-relay`). Adverse-suit length/honors were *not* predictive; overall
/// weakness is the driver.
pub(super) fn lebensohl_relay_shape(over: Suit) -> Cons<impl Constraint + Clone> {
    let five = |s: Suit| len(s, 5..);
    let any5 = match over {
        Suit::Clubs => five(Suit::Diamonds) | five(Suit::Hearts) | five(Suit::Spades),
        Suit::Diamonds => five(Suit::Clubs) | five(Suit::Hearts) | five(Suit::Spades),
        Suit::Hearts => five(Suit::Clubs) | five(Suit::Diamonds) | five(Suit::Spades),
        Suit::Spades => five(Suit::Clubs) | five(Suit::Diamonds) | five(Suit::Hearts),
    };
    any5 & hcp(6..)
}

/// Responder's plain-Lebensohl actions after our `1NT` and a natural 2-level
/// overcall in `over`
///
/// A book node here *shadows* the instinct floor, so this table covers every
/// responder hand. The Lebensohl idea separates weak from strong: weak hands
/// relay through `2NT` to a `3♣` sign-off (or correct to a long suit), while game
/// hands bid a forcing 3-level suit or a to-play `3NT` directly — so a game is
/// never stranded in a partscore (the failure mode of the Rubensohl v1 attempt).
//
// ponytail: the cue (Stayman / stopper-ask, "slow shows / fast denies") is
// skipped — 4-4-major game hands bid 3NT. Author the cue + opener's reply if the
// A/B shows it matters.
//
// The Section-5 builders below are pure functions of `(over, hand)` — the auction
// prefix and the bidder's identity never enter — so `american/defense.rs` reuses
// them verbatim for "sohl after a takeout double" (advancing partner's takeout
// double of a weak two), where the opponents' suit is likewise at the two level.
pub(crate) fn lebensohl_responder(over: Suit, agreements: &Agreements) -> Rules {
    let mut rules = Rules::new();

    // Forcing 3-level new suit: game-forcing, 5+ cards. A jump (when the 2-level
    // was available) or the cheapest 3-level bid (suit at/below the overcall) —
    // either way 3-of-a-suit over the interference is forcing. (All 3-level bids
    // clear a 2-level overcall, so no min-level gate is needed.)
    for s in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if s == over {
            continue;
        }
        let strain = Strain::from(s);
        rules = rules.rule(Bid::new(3, strain), 180, len(s, 5..) & points(10..));
    }

    // Direct cue of their suit = Stayman: game-forcing with a 4-card unbid major
    // (no 5-card suit to bid forcingly — those use the 3-level above). Answered by
    // [`cue_stayman_answer`]. Stopper-agnostic, mirroring Transfer's default cue,
    // so a 4-4 major fit is found even with a stopper. Weight sits between the
    // natural forcing 3-level (1.8) and direct 3NT (1.7): a known 5-card suit is
    // bid naturally, a bare 4-card major cues, else 3NT.
    let cue = Bid::new(3, Strain::from(over));
    rules = match unbid_major(over) {
        Some(major) => rules
            .rule(cue, 175, len(major, 4..) & points(10..))
            .alert(LEBENSOHL_CUE),
        None => rules
            .rule(
                cue,
                175,
                (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..)) & points(10..),
            )
            .alert(LEBENSOHL_CUE),
    };

    // Direct 3NT to play: game values with their suit stopped (toggles: drop the
    // stopper requirement, and/or trap-pass with 4+ in their suit).
    rules = author_direct_3nt(rules, 170, over, agreements);

    // Responder's double of their overcall (penalty by default; see [`DoubleStyle`]).
    rules = responder_double(rules, over, agreements);

    // Natural new suit at the 2 level (above the overcall, below 2NT): weak,
    // competitive, to play.
    for s in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if s == over {
            continue;
        }
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(2, strain),
            150,
            min_level_is(2, strain)
                & len(s, 5..)
                & points(..=9)
                & hcp(natural_floor_hcp(agreements)..)
                & points(natural_floor_pts(agreements)..),
        );
    }

    // 2NT = Lebensohl relay to 3♣: a weak hand with a long suit not biddable
    // naturally at the 2 level (long clubs, or a suit below the overcall) — sign
    // off in 3♣ or correct (see [`lebensohl_relay_rebid`]). The natural 2-level
    // outranks this relay, so above-the-overcall suits are still bid naturally;
    // balanced weak hands pass. See [`lebensohl_relay_shape`] for the 6+/good-5
    // shape and the PD-distilled 6-HCP floor on the 5-card arm.
    let long_suit = lebensohl_relay_shape(over);
    rules = rules
        .rule(Bid::new(2, Strain::Notrump), 140, points(..=9) & long_suit)
        .alert(LEBENSOHL_RELAY);

    // Pass — weak, nothing constructive to say.
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Responder's counter-defense after `1NT (2♣)` when the `2♣` is read as
/// **Landy** (both majors, 5-4 or better), engaged by the opponents'
/// disclosure (`agreements.decision.their.two_clubs_landy`)
///
/// Systems-on — the default here — is the right treatment over a *natural* `2♣`,
/// which steals no room.  Over Landy it inverts: the stolen `X` asks for a
/// four-card major against a hand that has just shown **both**, and `2♦`/`2♥`
/// are Jacoby transfers *into* their suits.  So drop the whole structure and
/// play the hand for what it is — we hold 15-17 opposite a limited two-suiter,
/// which makes defending the live option:
///
/// - `X` = values (8+), penalty-oriented.  They will usually run to a major,
///   and that run is **unauthored**: the floor's penalty chase
///   (`penalize_escape_stack` / `penalize_escape_values`) gates on
///   `instinct::our_doubled_one_nt_escape`, which requires
///   `auction[opening + 1] == Call::Double`.  Here that slot holds their `2♣`,
///   so the chase never fires in this lane and the floor plays its ordinary
///   takeout ladder after our double.
/// - Natural bids in the suits they have **not** shown — `2♦`, `3♣`, `3♦` —
///   plus the natural `2NT` invite and a direct `3NT`.
/// - No major bid of our own: with 5-4 majors opposite, ours are dead.
///
/// The double is the **residual**, so it sits *below* every hand that has an
/// offensive direction (`3NT`, the forcing 3-level minors) and above the ones
/// that do not (the weak `2♦`, the `2NT` invite).  It is floored on `hcp`, not
/// `points`: defending does not care about distribution, and a `points` floor
/// would drag the shapely weak hands that belong in `2♦` into a double.
fn landy_responder(agreements: &Agreements) -> Rules {
    // No `over` binding: unlike every other node here, the call we sit over
    // names no suit at all — their `2♣` is the majors.
    let mut rules = Rules::new();

    // X = values, willing to defend whatever they run to.
    rules = rules
        .rule(Call::Double, 145, hcp(8..))
        .alert(LANDY_VALUES)
        .penalty();

    // The minor one-suiters, split by the N1b overlay (`defense_2c_landy_cues`):
    //
    // - **Cues off** (the base counter): natural *forcing* `3♣`/`3♦` — a
    //   **six-card** suit, ranked *above* 3NT.  Their 2♣ is artificial, so
    //   clubs are ours to bid; and with both majors against us, whether 3NT is
    //   playable turns on opener's major holdings, which only opener can see.
    //   Showing the source of tricks and letting opener choose beats guessing.
    //   (Five-card suits stay inside the 3NT/double partition: at 5-3-3-2
    //   there is nothing for opener to choose.)
    //
    // - **Cues on**: the full [`michaels_cue_responder`][super::two_suiters]
    //   skeleton — the cues (`2♥` = 5+♣, `2♠` = 5+♦, **invitational or
    //   better**) carry every minor one-suiter worth showing, six-carders
    //   included, and the direct `3♣`/`3♦` flip to natural **weak** escapes (a
    //   forcing 3m would be redundant with the cue below it).  The weak `3♦`
    //   survives below the 2♦ escape's floor and fired 9-11 times in the
    //   measured arm, so `2♦` does *not* shadow it.  `2♥` edges `2♠` so 5-5
    //   minors cue cheaper — which also means a hand with longer diamonds than
    //   clubs (6♦5♣) shows the clubs; rare enough to leave.
    //
    //   INV+ rather than game-forcing because opener's ask
    //   ([`landy_cue_answer`]) promises minor tolerance, so `3m`/`4m` are
    //   landing spots and opener can decline.  That costs a full accept/decline
    //   tree: `Inferences` carries no forcing channel (per-seat length/points
    //   envelopes only), so a rung left to the floor reads as bare "5+ ♣, 8+
    //   points" with no notion of an invitation — which is exactly how the
    //   sub-game cue answer cost −1.8 IMPs/fired.  Every rung below is
    //   authored for that reason.
    //
    // - **N1c on** (`defense_2c_landy_transfer`, which implies the cues): the
    //   two rungs *below* the cues are re-spent on what `probe-divergence`
    //   found inside N1b's aggregate wash.  The weak `3♣` was the package's
    //   engine (+3.41/+0.71 IMPs per fired board), so it moves down to a `2NT`
    //   transfer — cheaper, and right-sided into the `1NT` opener's hand, which
    //   is the whole reason a transfer exists.  The weak `3♦` measured −1.07 /
    //   −1.71 and was redundant with the weak `2♦` below it, so both direct 3m
    //   rungs are re-spent as **invitational** six-carders: the one hand the
    //   cue handles badly, since the cue's accept/decline tree is built for a
    //   five-card suit that may have no fit at all.
    if landy_cues(agreements) {
        // 3NT *above* the cues, and the only rung here that gates stoppers.
        // Opener bid 1NT, so opener declares any notrump contract (Law 54) —
        // responder's direct 3NT costs no siding, which is why it can outrank
        // a cue that would otherwise let opener choose.  Denying a six-card
        // minor is the 5-vs-6 split: a six-carder is a source of tricks with
        // slam play, so it always cues and lets opener place the contract,
        // where a 5-3-3-2 with both majors held has nothing to explore.
        //
        // The base arm's ungated 3NT (170, below) is untouched — this rule
        // only outranks it, so a cues-off arm bids exactly what it always did.
        rules = rules.rule(
            Bid::new(3, Strain::Notrump),
            180,
            points(10..)
                & stopper_in(Suit::Hearts)
                & stopper_in(Suit::Spades)
                & len(Suit::Clubs, ..=5)
                & len(Suit::Diamonds, ..=5),
        );
        if landy_transfer(agreements) {
            // The invitational six-carder, *above* the cue that would
            // otherwise take it: opener's cue answer promises tolerance and
            // hunts for stoppers on the assumption of a five-card suit, where
            // a six-bagger just wants a yes/no on 3NT.
            // N1h shifts the whole band down a point rather than just its
            // floor, so the 9-count six-carder falls through to the cue (INV+,
            // and opener places the contract) instead of overlapping it.
            let (inv_lo, inv_hi) = if landy_low_minors(agreements) {
                (7, 8)
            } else {
                (8, 9)
            };
            for (s, weight) in [(Suit::Clubs, 176), (Suit::Diamonds, 175)] {
                let strain = Strain::from(s);
                let bid = Bid::new(3, strain);
                // N1i regrades the rung on `hcp` — the scale the values double
                // it competes with already uses.
                rules = if landy_hcp_rungs(agreements) {
                    rules.rule(bid, weight, len(s, 6..) & hcp(7..=8))
                } else {
                    rules.rule(bid, weight, len(s, 6..) & points(inv_lo..=inv_hi))
                };
            }
        }
        // N1d raises the cues' floor to `points(10..)`: at `points(8..)` and
        // weight 173/172 against the values double's 145, the cue took every
        // 8+ point hand with a five-card minor, and the per-bid decomposition
        // priced each hand that migrated `X` → cue at −0.92/−2.53 PD per
        // fired.  With the floor up, 8-9 goes back to the double; sub-8-hcp
        // shapely hands fall to `2♦`/the transfer/Pass, where a values
        // double's doctrine wants them.
        // N1h takes a point off whatever floor is in force, so the two knobs
        // compose instead of one silently overriding the other.
        let cue_floor = if landy_cue_floor(agreements) { 10 } else { 8 }
            - if landy_low_minors(agreements) { 1 } else { 0 };
        rules = if landy_hcp_rungs(agreements) {
            rules
                .rule(
                    Bid::new(2, Strain::Hearts),
                    173,
                    len(Suit::Clubs, 5..) & hcp(9..),
                )
                .alert(LANDY_CUE)
                .rule(
                    Bid::new(2, Strain::Spades),
                    172,
                    len(Suit::Diamonds, 5..) & hcp(9..),
                )
                .alert(LANDY_CUE)
        } else {
            rules
                .rule(
                    Bid::new(2, Strain::Hearts),
                    173,
                    len(Suit::Clubs, 5..) & points(cue_floor..),
                )
                .alert(LANDY_CUE)
                .rule(
                    Bid::new(2, Strain::Spades),
                    172,
                    len(Suit::Diamonds, 5..) & points(cue_floor..),
                )
                .alert(LANDY_CUE)
        };
        if landy_transfer(agreements) {
            // The weak club escape, moved down a level and right-sided.  Same
            // weight the direct `3♣` carried, so the values double still takes
            // the 8-9 hcp hands and only the *rung* changes.  There is no
            // diamond twin: the weak `2♦` below already covers it, and N1b's
            // weak `3♦` measured negative trying to duplicate it.
            let transfer = Bid::new(2, Strain::Notrump);
            rules = if landy_hcp_rungs(agreements) {
                rules
                    .rule(transfer, 110, len(Suit::Clubs, 6..) & hcp(..=6))
                    .alert(LANDY_TRANSFER)
            } else {
                rules
                    .rule(transfer, 110, len(Suit::Clubs, 6..) & points(2..=9))
                    .alert(LANDY_TRANSFER)
            };
        } else {
            for s in [Suit::Clubs, Suit::Diamonds] {
                let strain = Strain::from(s);
                rules = rules.rule(Bid::new(3, strain), 110, len(s, 6..) & points(2..=9));
            }
        }
    } else {
        for s in [Suit::Clubs, Suit::Diamonds] {
            let strain = Strain::from(s);
            rules = rules.rule(Bid::new(3, strain), 175, len(s, 6..) & points(10..));
        }
    }

    // Direct 3NT on game values, with **no stopper gate** — deliberately not
    // `author_direct_3nt`, whose stopper and trap-pass tests both key on the
    // overcall's suit.  Their `2♣` is artificial: it promises the majors, not
    // clubs, so a club stopper is not the question and a club honour-stack is
    // not a reason to trap.  (Demanding a *major* stopper instead is no use
    // either — they hold both.)  This matches what systems-on already bid here,
    // which keeps the A/B a test of the double rather than of 3NT discipline.
    rules = rules.rule(Bid::new(3, Strain::Notrump), 170, points(10..));

    // Weak natural `2♦` — the one suit below their majors we can still play in.
    // N1i caps it on `hcp` instead, which is a *narrowing*: the 7-8 hcp
    // five-card-diamond hand it drops reaches neither the `X` (floor 8 hcp,
    // and only at 8) nor the cue, so it passes.  That hole is deliberate and
    // is the first row to read if the arm loses.
    let escape = Bid::new(2, Strain::Diamonds);
    let floors = hcp(natural_floor_hcp(agreements)..) & points(natural_floor_pts(agreements)..);
    rules = if landy_hcp_rungs(agreements) {
        rules.rule(escape, 140, len(Suit::Diamonds, 5..) & hcp(..=6) & floors)
    } else {
        rules.rule(
            escape,
            140,
            len(Suit::Diamonds, 5..) & points(..=9) & floors,
        )
    };

    // Natural invitational 2NT — dropped under N1c, which spends the call on
    // the club transfer.  It costs almost nothing: the values double outranks
    // it (145 vs 130) on every hand with 8+ hcp, so all it ever carried was the
    // 8-9 *point* hand with fewer than 8 hcp, which now passes unless it has a
    // suit to bid.  Shape points are exactly what a values double should not be
    // floored on, so that hand was never a happy invitation.
    if !landy_transfer(agreements) {
        rules = rules.rule(Bid::new(2, Strain::Notrump), 130, points(8..=9));
    }

    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to the Landy values double — sit for it
///
/// The double is values, not a question, so opener has nothing to answer and
/// every hand passes.  The node exists because the `X -` suffix would otherwise
/// reach `stayman_answers()` (the stolen-Stayman reply the counter replaces) and
/// bid a phantom major.  Total by construction: one rule, no gate.
fn landy_double_answer() -> Rules {
    Rules::new().rule(Call::Pass, 100, hcp(0..))
}

/// Which of §N1l's rungs an arm carries at the Landy doubler's rebid seat
///
/// The 2026-08-28 measurement was mixed **by rung**, not overall: the penalty
/// `X` carried the entire vulnerable plain win and the constructive family
/// carried the vulnerable loss.  So the flip is a choice of subset, and the
/// three knobs name three subsets rather than three tables.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DoublerLadder {
    /// [`CompetitionKnobs::landy_doubler_px`][crate::bidding::agreements::CompetitionKnobs::landy_doubler_px]
    /// — the penalty `X` and the catch-all, nothing else
    Px,
    /// [`CompetitionKnobs::landy_doubler_white`][crate::bidding::agreements::CompetitionKnobs::landy_doubler_white]
    /// — plus `3NT`, and the whole constructive family gated non-vulnerable
    White,
    /// [`CompetitionKnobs::landy_doubler_rebids`][crate::bidding::agreements::CompetitionKnobs::landy_doubler_rebids]
    /// — the ladder as measured, every rung
    Full,
}

/// The arm in force at the doubler's rebid seat, or [`None`] for the default
///
/// Most-to-least inclusive, so an arm that sets two knobs gets the bigger
/// ladder instead of an arbitrary one — the A/B never sets two, but a package
/// invariant crossing all three does.
fn landy_doubler_ladder(agreements: &Agreements) -> Option<DoublerLadder> {
    let knobs = &agreements.competition;
    if knobs.landy_doubler_rebids {
        Some(DoublerLadder::Full)
    } else if knobs.landy_doubler_white {
        Some(DoublerLadder::White)
    } else if knobs.landy_doubler_px {
        Some(DoublerLadder::Px)
    } else {
        None
    }
}

/// The Landy doubler's rebid once their advance has named the major
/// (`1NT (2♣) X (2♥) - -`, `X (2♠) - -`, `X (2♦) - (2♥)`, `X (2♦) - (2♠)`),
/// under
/// [`CompetitionKnobs::landy_doubler_rebids`][crate::bidding::agreements::CompetitionKnobs::landy_doubler_rebids]
///
/// [`kokish_kraft_doubler_rebid`]'s ladder ported one suit down — same four
/// rungs, same weights, same catch-all — with the two changes this lane forces.
///
/// **No `ran` fork.**  The Multi twin carries one because their advancer's
/// pass-or-correct is provisional and the overcaller pulls it, which is why
/// `kokish_kraft_entries` registers `X (2♥) - (2♠)`.  Probed at seat 1 with the
/// actor's hands filtered to the ones BBA actually overcalls `2♣` with, the
/// Landy overcaller passes the preference **94.5%** of the time over `(2♥)` and
/// **96.7%** over `(2♠)`: it has already shown both majors and lets partner
/// choose.  The preference is final, so there is no correction to fork on.
///
/// **A natural minor where the twin has the other major.**  The twin's
/// weight-100 rung is the major partner did not pick.  Here they hold *both*
/// majors, so there is no other major to bid, and the slot goes to a five-card
/// minor at the three level.  That rung is also the only route for an 8–9
/// one-suited minor: [`landy_bba_responder`]'s transfers above it are wide but
/// its `3NT`@168 outranks the double, so the hand that doubles first is the one
/// that cannot transfer.  Clubs first when both, the house's cheaper-minor
/// idiom.
///
/// Every rung answers a hole this seat has today, and the seat is the floor's
/// on every hand — `probe-decision` prints `fallback: Some(0)` across the band.
/// The floor bids `3NT` holding `KJ98` of *their* major instead of doubling
/// (`X` is not in its top six), and passes both the 8–9 invitation and the 8–9
/// five-card minor.  That is the 2026-08-27 census's "the auction dies after
/// our values double" (67 bd, −75 plain) stated hand by hand.
///
/// The escape legs take the same table: their `2♦` is **artificial** — BBA's
/// own label on 200/200 hands, a "pick a major" relay the overcaller corrects
/// 79.4% of the time and never passes — so it carries no diamond claim and the
/// `3♦` rung is as safe there as on the preference legs.
///
/// **The top two rungs are dead in self-play, and they are kept anyway.**
/// Unlike the Multi twin — whose `3NT`@150 needs *both* major stoppers, so a
/// one-stopper game hand really does double first — this lane's
/// [`landy_bba_responder`] carries an **ungated** `3NT`@168 on `points(10..)`.
/// Every 10-plus-point hand therefore bids `3NT` directly and never doubles,
/// which caps the double at nine points (`probe-call-reading` reads it back as
/// `points 8..9`, and the ordering says the same thing).  So `4NT`@160
/// (`hcp(16..)`) and `3NT`@150 (`points(10..)`) can only fire opposite a
/// partner who is not bidding this table.  They stay because the table is
/// **total**: with them deleted a strong hand arriving here would take the
/// `Pass`@0 catch-all, which is strictly worse than the floor this node
/// shadows.  What actually fires in self-play is `X` / `2NT` / `3♣` / `3♦` /
/// `Pass` — which is exactly the census's dying auction.
///
/// **The flip arms.**  The 2026-08-28 A/B measured this table mixed, and the
/// per-rung split says why: the penalty `X` is +7.489 (none) / +9.196 (both)
/// IMPs/fired on plain DD — the whole vulnerable plain win — while the
/// constructive rungs are positive non-vulnerable and negative vulnerable, the
/// `2NT` invitation worst (−3.695 PD/fired, and its declined half loses both
/// scorers).  [`DoublerLadder`] turns that into two smaller arms:
/// [`Px`][DoublerLadder::Px] keeps only the `X`, and
/// [`White`][DoublerLadder::White] keeps the whole constructive family with the
/// invitation and the natural minors gated `!vulnerable()`.  Both delete
/// `4NT`, which never fired once in either measured cell.
///
/// The gate — rather than deleting rungs — is what the divergence stream says
/// when it is re-read by first differing call: every constructive rung flips
/// sign with colour, and the natural minors are the *cheaper* half white (`3♦`
/// −0.607 PD per fired, `3♣` −0.797, against `2NT`'s −1.667), so a design that
/// deleted the minors would have kept the worst rung and dropped the best two.
///
/// **§N1-lia's three knobs, all rung edits on this one table.**
/// [`CompetitionKnobs::landy_doubler_catchall`][crate::bidding::agreements::CompetitionKnobs::landy_doubler_catchall]
/// (default **off** since 2026-08-30) restores the `Pass`@0; the shipped
/// default drops it, so the short hands fall through to the floor's
/// takeout-shaped values double — a measured +0.0036 NV / +0.0014 BV plain
/// IMPs/board, un-blocked by [`LANDY_PENALTY`]'s re-worded tag.
/// [`CompetitionKnobs::landy_doubler_three_honors`][crate::bidding::agreements::CompetitionKnobs::landy_doubler_three_honors]
/// and [`CompetitionKnobs::landy_doubler_three_small`][crate::bidding::agreements::CompetitionKnobs::landy_doubler_three_small]
/// (both default **on**, same A/B) add mutually exclusive three-card `X`
/// cells at 154/153 under the same tag and `.penalty()`, splitting
/// exactly-three trumps by `top_honors` — both cells measured plain wins at
/// both vulnerabilities, overturning the sibling lane's lone-honor caveat.
fn landy_doubler_rebid(major: Suit, ladder: DoublerLadder, agreements: &Agreements) -> Rules {
    let mut rules = Rules::new();
    if ladder == DoublerLadder::Full {
        rules = rules.rule(Bid::new(4, Strain::Notrump), 160, hcp(16..));
    }
    // The one rung every arm carries — the measurement's whole vulnerable
    // plain win, and the reason the flip exists.
    rules = rules
        .rule(Call::Double, 155, len(major, 4..))
        .alert(LANDY_PENALTY)
        .penalty();
    // §N1-lia's three-card cells: same call, lower weights (an authored
    // precedence, not a tie), same tag — the arms differ only in the rule.
    if agreements.competition.landy_doubler_three_honors {
        rules = rules
            .rule(
                Call::Double,
                154,
                len(major, 3..=3) & top_honors(major, 2..),
            )
            .alert(LANDY_PENALTY)
            .penalty();
    }
    if agreements.competition.landy_doubler_three_small {
        rules = rules
            .rule(
                Call::Double,
                153,
                len(major, 3..=3) & top_honors(major, ..=1),
            )
            .alert(LANDY_PENALTY)
            .penalty();
    }
    if ladder != DoublerLadder::Px {
        rules = rules.rule(
            Bid::new(3, Strain::Notrump),
            150,
            points(10..) & stopper_in(major),
        );
        // The invitation, and below it the naturals — the whole constructive
        // family, gated on colour outside the full ladder.  Spelled as paired
        // `rule` calls rather than one conditional constraint because the two
        // constraints are different types.
        //
        // The naturals sit below every rung above and above the catch-all, so
        // they fire on exactly the hands that pass today and cannot move a
        // call this lane already makes.
        let invite = hcp(8..=9) & stopper_in(major);
        let white = ladder == DoublerLadder::White;
        rules = if white {
            rules.rule(Bid::new(2, Strain::Notrump), 145, invite & !vulnerable())
        } else {
            rules.rule(Bid::new(2, Strain::Notrump), 145, invite)
        };
        for (minor, weight) in [(Suit::Clubs, 100), (Suit::Diamonds, 99)] {
            let call = Bid::new(3, Strain::from(minor));
            rules = if white {
                rules.rule(call, weight, len(minor, 5..) & !vulnerable())
            } else {
                rules.rule(call, weight, len(minor, 5..))
            };
        }
    }
    // §N1-lia package A: without the catch-all the node rejects what no rung
    // claims, which is *how* it hands those hands to the floor — the floor is
    // behind it by construction (the competitive book's learned floor).
    if agreements.competition.landy_doubler_catchall {
        rules = rules.rule(Call::Pass, 0, hcp(0..));
    }
    rules
}

/// **Opener's** own rebid once their advance has named the major
/// (`1NT (2♣) X (2♥)`, `X (2♠)`), under
/// [`CompetitionKnobs::landy_opener_px`][crate::bidding::agreements::CompetitionKnobs::landy_opener_px]
/// and its `rungs` companion
///
/// The seat one call *before* [`landy_doubler_rebid`]'s, and the seat §N1k
/// authored a `3NT` at, lost, and gave back to the floor.  The floor owns it
/// today and passes 98.5% (non-vulnerable) / 99.5% (vulnerable) of the time.
///
/// **The `X` is the whole idea, and the oracle draws its gate.**
/// `probe-landy-opener-oracle` priced every contract opener could steer to on
/// 103,653 + 81,023 seat boards taken off the §N1l base arms, against the
/// contract our live method actually reaches.  Defending their major
/// **doubled** wins every four-plus-trump bucket at both vulnerabilities —
/// +2.8…+6.8 IMPs/board white, +3.8…+8.1 red, rising with opener's HCP — with
/// a flat perfect-defense column (−1.2…+0.3), which is exactly the signature
/// of a real penalty double that plain DD sees and PD cannot
/// (docs/measurement.md's domain addendum).  On a **doubleton** it loses at
/// every strength (−0.7…−4.5); on **three** it is negative at 15, marginal at
/// 16, and positive only at 17, where `3NT` matches or beats it given a
/// stopper.  So `len(major, 4..)` is the entire gate: no HCP floor, no stopper
/// test, and no "three plus good defense" (the K–K reference allows it; the
/// oracle does not).  The one cell that would buy — three trumps, 17 HCP, *no*
/// stopper, +1.96 white / +2.94 red on ~1.3% of the seat — stays unbought,
/// because [`LANDY_PENALTY`] publishes four-plus of that major and a
/// three-card double under the same slug would make the alert false.
///
/// **Ordering does the capping.**  §N1k's `3NT` fired on `hcp(16..) &
/// has_stopper` with nothing above it, so it took the four-trump hands where
/// the oracle prices notrump −1.1…−4.5 and shadowed the floor's delayed
/// penalty double.  With `X`@150 on top, a four-trump maximum doubles and the
/// notrump rungs see only the two- and three-card holdings they win on — the
/// length cap `has_stopper` cannot express, supplied for free by the weights.
///
/// **What the oracle rejected.**  A natural `3m` is dominated by notrump on
/// the same boards at both vulnerabilities (+1.86 against `2NT`'s +1.99 and
/// `3NT`'s +2.19 white; +0.29 against +1.13 red), and the six-card slice that
/// might have changed that is *structurally absent* from the pool —
/// `--filter-landy`'s `is_1nt_opener` is strictly balanced, so 5m(422) and
/// 6m(322) openers never enter.  `3OM` in the major they did not name is the
/// worst of all seven candidates on its own 2.7% surface (−0.78 plain, −4.5
/// PD): they hold four-plus of it.  Both rungs are therefore absent, not
/// deferred.
fn landy_opener_rebid(major: Suit, rungs: bool) -> Rules {
    let mut rules = Rules::new()
        .rule(Call::Double, 150, len(major, 4..))
        .alert(LANDY_PENALTY)
        .penalty();
    if rungs {
        rules = rules
            .rule(
                Bid::new(3, Strain::Notrump),
                135,
                hcp(16..) & stopper_in(major),
            )
            // Fifteen by the ordering above, and white only: red, every
            // declaring candidate but the 16–17-with-a-stopper `3NT` collapses
            // (`2NT` −0.583 IMPs/board over the direct leg).
            .rule(
                Bid::new(2, Strain::Notrump),
                120,
                hcp(15..) & stopper_in(major) & !vulnerable(),
            );
    }
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to the doubler's natural three-level minor
/// (`1NT (2♣) X (2♥) - - 3♣ -` and its siblings)
///
/// The table §N1l owed and never built — the Multi-twin hole, left standing
/// because that measurement's `3♣`/`3♦` rungs ran with a floor-owned answer
/// above them.  Any arm that keeps the rungs has to pay for it: an authored
/// call whose continuation is the floor's is not a finished convention.
///
/// The arithmetic is the lane's, and it is tight.  Responder doubled on
/// `hcp 8+` and is capped at nine by [`landy_bba_responder`]'s ungated
/// `3NT`@168; it then bid a five-card minor *below* the `2NT`@145 invitation,
/// which denies the stopper that rung requires.  So the stopper has to be
/// opener's, and 16 opposite 9 is the 25 that bids the game — the same shape as
/// [`kokish_kraft_invite_answer`], one rung higher and carrying the stopper
/// test the invitation would have made.  Everything else passes the part-score
/// in the known eight-card-or-better minor fit.  Total.
fn landy_minor_rebid_answer(major: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            hcp(16..) & stopper_in(major),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to the counter's weak sign-offs — pass, always
///
/// One of the `landy_natural_answers` trio.  Covers the weak `2♦` (and, under
/// the N1b overlay, the weak `3♣`/`3♦`): responder is limited with a long
/// suit, the sign-off is a minor, and minor game is out of a weak hand's reach,
/// so there is no raise to probe for ([`lebensohl_signoff_raise`] excludes
/// minors by the same doctrine).  Like [`landy_double_answer`], the node exists
/// because the suffix would otherwise reach the floor, which cannot see the
/// counter's regime (the knob has no net input slot) and completes the retired
/// gadget instead — a phantom Jacoby `2♥` on 82% of audited `2♦` sign-offs.
fn landy_signoff_answer() -> Rules {
    Rules::new().rule(Call::Pass, 100, hcp(0..))
}

/// Opener's answer to the counter's natural `2NT` invite (`1NT (2♣) 2NT -`)
///
/// One of the `landy_natural_answers` trio.  The same size decision as the
/// uncontested invite, keyed on the same knob
/// ([`NotrumpKnobs::size_ask_accept_floor`][crate::bidding::agreements::NotrumpKnobs::size_ask_accept_floor],
/// default 16): accept at `3NT` from the top of the range, else pass.  Without
/// the node the floor answers the minor transfer the invite replaced (23%
/// phantom `3♦`/`3♣` in the audited dumps).
fn landy_invite_answer(agreements: &Agreements) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            hcp(agreements.notrump.size_ask_accept_floor..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to the base counter's game-forcing `3♣`/`3♦` (`1NT (2♣) 3m -`)
///
/// Responder shows a source of tricks and game values, and opener chooses —
/// the point of ranking the naturals above 3NT: `3NT` with both of their
/// majors stopped, else raise.  The raise doubles as the finite catch-all —
/// opener is balanced, so it never lands on fewer than two.  Without the node
/// the floor answers `3♣` as the Puppet Stayman it replaced (85% phantom
/// `3♦`).
///
/// The base arm only.  Under N1b the direct `3m` is a weak escape opener sits
/// for, under N1c it is invitational ([`landy_minor_invite_answer`]), and the
/// cue that carries the game force has its own tree ([`landy_cue_answer`]).
fn landy_minor_answer(minor: Suit) -> Rules {
    let strain = Strain::from(minor);
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            150,
            stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        )
        .rule(Bid::new(4, strain), 100, len(minor, 3..))
        .rule(Bid::new(4, strain), 20, hcp(0..))
}

/// Opener's answer to N1c's invitational `3♣`/`3♦` (`1NT (2♣) 3m -`)
///
/// Responder has 8-9 with a six-card minor and no more.  Minor game is the five
/// level, out of reach of a combined 23-26, so the only question on the table is
/// 3NT: accept from the top of the range with both of their majors stopped
/// (same knob as every other 1NT-size decision,
/// [`NotrumpKnobs::size_ask_accept_floor`][crate::bidding::agreements::NotrumpKnobs::size_ask_accept_floor],
/// default 16), else sit for the partscore.  The pass is the finite catch-all.
///
/// No stopper-hunting rung.  The cue below is what an invitational hand bids
/// when it wants opener's help finding one; choosing the direct `3m` instead is
/// choosing to be placed.
fn landy_minor_invite_answer(agreements: &Agreements) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            stopper_in(Suit::Hearts)
                & stopper_in(Suit::Spades)
                & hcp(agreements.notrump.size_ask_accept_floor..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// The cheapest bid of `strain` strictly above `floor`
///
/// The Landy cues sit at different heights (`2♥` for clubs, `2♠` for
/// diamonds), so every rung above them — opener's stopper ask, responder's
/// retreat to the minor — is one level higher on the diamond side.  Deriving
/// the rung instead of tabulating it keeps one table shape for both cues.
fn cheapest_above(strain: Strain, floor: Bid) -> Bid {
    let level = floor.level.get();
    if Bid::new(level, strain) > floor {
        Bid::new(level, strain)
    } else {
        Bid::new(level + 1, strain)
    }
}

/// Opener's answer to a Landy minor cue (`1NT (2♣) 2♥/2♠ -`)
///
/// The cue is invitational-or-better with 5+ in `minor`, and responder has
/// already denied the hand that belongs in 3NT (both majors stopped, no
/// six-card minor — it bids 3NT at its first turn).  So opener answers the two
/// questions responder cannot: are the majors stopped, and is this worth game?
///
/// | Opener | Shows |
/// | --- | --- |
/// | `3NT` | both majors stopped, maximum — accepts |
/// | `2NT` | both majors stopped, minimum — responder places it |
/// | the **unstopped major** | exactly one major stopped, 3+ in the minor — asks responder to supply the other |
/// | `4m` | neither major stopped, maximum |
/// | `3m` | neither major stopped, minimum (and the finite catch-all) |
///
/// The ask names the major opener *lacks*, and promises tolerance for the
/// minor, so responder without the stopper retreats to the minor rather than
/// guessing at notrump ([`landy_ask_answer`]).
///
/// **Every rung is authored down to the placing call, and none of the minimums
/// may collapse into 3NT.** The cue's first draft answered at `2NT`/`3m` and
/// stopped there, and `probe-divergence`'s post-mortem of the 2026-08-14 A/B
/// priced that at −1.8 IMPs per fired board on both scorers and both
/// vulnerabilities: `2NT` collapses into 3NT and the auction dies, where the
/// base arm's `4m` left a suit contract the floor could cue-bid over and
/// reached `6♦`.  Three of the five worst boards were exactly that swap, e.g.
/// `1NT (2♣) 2♥ - 2NT - 3NT` for `1NT (2♣) 3♦ - 3NT - 4♦ … 6♦`.  Hence the slam
/// try in [`landy_minimum_notrump_rebid`] / [`landy_minimum_minor_rebid`]:
/// `Inferences` carry no forcing channel, so an auction handed to the floor
/// below game reads as bare length-and-points.
///
/// Under N1e ([`CompetitionKnobs::defense_2c_landy_fit_answers`]) the two
/// notrump rungs also take **doubleton support** at their strength level —
/// *(both majors stopped, or ≤2-card support)* — and the catch-all flips to
/// `2NT`, so every raise and ask comes to promise 3+.  The base table raises
/// the cue's minor on two by design ("opener is balanced, so it never lands
/// on fewer than two"), and the fit forensic priced those 5-2 finals at
/// −10.0/−8.2 PD per fired board.  A stopper is then only guaranteed
/// alongside a fit, which responder knows from the very rung that denied one.
fn landy_cue_answer(minor: Suit, cue: Bid, agreements: &Agreements) -> Rules {
    let strain = Strain::from(minor);
    let max = agreements.notrump.size_ask_accept_floor;
    let fit_answers = landy_fit_answers(agreements);
    let both = stopper_in(Suit::Hearts) & stopper_in(Suit::Spades);
    let tolerance = len(minor, 3..);

    // Maximum: the 3-level and above.
    let mut rules = if fit_answers {
        Rules::new().rule(
            Bid::new(3, Strain::Notrump),
            160,
            (both.clone() | len(minor, ..=2)) & hcp(max..),
        )
    } else {
        Rules::new().rule(Bid::new(3, Strain::Notrump), 160, both.clone() & hcp(max..))
    };
    for (held, lacked) in [(Suit::Hearts, Suit::Spades), (Suit::Spades, Suit::Hearts)] {
        rules = rules
            .rule(
                Bid::new(3, Strain::from(lacked)),
                155,
                stopper_in(held) & !stopper_in(lacked) & tolerance.clone() & hcp(max..),
            )
            .alert(LANDY_CUE);
    }
    rules = rules.rule(Bid::new(4, strain), 150, tolerance.clone() & hcp(max..));

    // Minimum: 2NT, the cheap ask, or the minor.
    rules = if fit_answers {
        rules.rule(
            Bid::new(2, Strain::Notrump),
            145,
            (both | len(minor, ..=2)) & hcp(..max),
        )
    } else {
        rules.rule(Bid::new(2, Strain::Notrump), 145, both & hcp(..max))
    };
    // The club cue leaves `2♠` below the 3-level, so a *minimum* missing only
    // the spade stopper can still ask.  There is no cheap rung for a minimum
    // missing hearts (`3♥` would read as a maximum), and none at all over the
    // diamond cue — that asymmetry is the price of letting level mean strength.
    let cheap = Bid::new(2, Strain::Spades);
    if cheap > cue {
        rules = rules
            .rule(
                cheap,
                140,
                stopper_in(Suit::Hearts)
                    & !stopper_in(Suit::Spades)
                    & tolerance.clone()
                    & hcp(..max),
            )
            .alert(LANDY_CUE);
    }
    let rules = rules.rule(Bid::new(3, strain), 100, tolerance);
    if fit_answers {
        // The notrump rungs above took every doubleton and the 100-rung takes
        // every 3+ fit, so this formal catch-all is unreachable in practice —
        // but it must exist ("every table ends in a finite catch-all"), and
        // notrump is the one call that never manufactures a 5-2.
        rules.rule(Bid::new(2, Strain::Notrump), 20, hcp(0..))
    } else {
        rules.rule(Bid::new(3, strain), 20, hcp(0..))
    }
}

/// Opener's answer when the opponents raise over a Landy cue
/// (`1NT (2♣) 2♥ (2♠)` / `(3♥)` / `(3♠)`) — N1f only
///
/// The advancer's raise squeezes the clean ladder ([`landy_cue_answer`]) into
/// the room it leaves, and hands opener a call the clean table refuses on
/// purpose: **Pass**, which is safe here because responder is INV+ and
/// guaranteed another turn.  So the compressed ladder keeps only the calls
/// that say something Pass cannot — game with both of their majors stopped,
/// and the fit, split by size where a 3-level raise still exists (their `2♠`
/// over the club cue) and folded to a maximum `4m` where it does not.
/// Everything else passes and lets responder place it; without this node the
/// floor was bidding 3-5 card *majors* at the four level on these auctions
/// (−14…−18 PD, the worst boards of the whole divergent set).
fn landy_cue_overcalled(minor: Suit, over: Bid, agreements: &Agreements) -> Rules {
    let strain = Strain::from(minor);
    let max = agreements.notrump.size_ask_accept_floor;
    let both = stopper_in(Suit::Hearts) & stopper_in(Suit::Spades);
    let raise = cheapest_above(strain, over);
    let mut rules = Rules::new().rule(Bid::new(3, Strain::Notrump), 150, both & hcp(max..));
    if raise < Bid::new(4, strain) {
        rules = rules
            .rule(Bid::new(4, strain), 120, len(minor, 3..) & hcp(max..))
            .rule(raise, 100, len(minor, 3..));
    } else {
        rules = rules.rule(raise, 100, len(minor, 3..) & hcp(max..));
    }
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Responder's rebid over opener's minimum `2NT` (`1NT (2♣) 2♥ - 2NT -`)
///
/// Opener has shown both majors stopped and a minimum, so nothing is left to
/// ask: responder passes the invitation, bids the game it was hiding, or — with
/// a six-card source of tricks and enough for the combined 28+ — starts the
/// slam try in the minor ([`landy_slam_try`]).
fn landy_minimum_notrump_rebid(minor: Suit, agreements: &Agreements) -> Rules {
    let rules = Rules::new().rule(Bid::new(3, Strain::Notrump), 100, points(10..));
    landy_slam_try(rules, minor, 110, agreements).rule(Call::Pass, 0, hcp(0..))
}

/// Responder's `4♣`/`4♦` slam try over opener's minimum rebid
///
/// The cue is invitational-or-better with 5+ in the minor, and the ladder above
/// it topped out at 3NT — there was no rung at all for the hand with a six-card
/// source of tricks and slam values, so the cue could only ever land in game.
/// `4m` is that rung, ranked *above* the 3NT it displaces.
///
/// Opener's continuation is deliberately the floor's.  This is the one place
/// the measured evidence says so: the boards that cost the cue's first draft
/// −1.8 IMPs/fired were exactly the ones where a `4m` suit contract let the
/// floor cue-bid on to `6♦` while the notrump rung died in 3NT
/// ([`landy_cue_answer`]).
///
/// N1c only ([`CompetitionKnobs::defense_2c_landy_transfer`]) — these two
/// tables are shared with N1b, and the four-arm A/B only attributes cleanly if
/// the cues arm stays the structure that was measured.
fn landy_slam_try(rules: Rules, minor: Suit, weight: i16, agreements: &Agreements) -> Rules {
    if !landy_transfer(agreements) {
        return rules;
    }
    rules.rule(
        Bid::new(4, Strain::from(minor)),
        weight,
        points(13..) & len(minor, 6..),
    )
}

/// Responder's rebid over opener's minimum `3m` (`1NT (2♣) 2♥ - 3♣ -`)
///
/// This is the one minimum showing *no* stopper, so it is the only place
/// responder's own stopper worry can still be resolved — over `2NT` opener has
/// both, and over the cheap `2♠` ask responder is answering, not asking.  With
/// a game force responder bids `3NT` holding both majors itself, else cues the
/// major it lacks (the cheaper cue wins when both are missing).  Invitational
/// hands pass, which is what made the cue INV+ in the first place.
///
/// This is also where the slam try belongs most: opener has denied both
/// stoppers, so notrump is the dubious strain and the six-card minor is the
/// contract ([`landy_slam_try`]).
fn landy_minimum_minor_rebid(minor: Suit, agreements: &Agreements) -> Rules {
    let rules = Rules::new().rule(
        Bid::new(3, Strain::Notrump),
        120,
        points(10..) & stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
    );
    let mut rules = landy_slam_try(rules, minor, 130, agreements);
    for (major, weight) in [(Suit::Hearts, 110), (Suit::Spades, 109)] {
        rules = rules
            .rule(
                Bid::new(3, Strain::from(major)),
                weight,
                points(10..) & !stopper_in(major),
            )
            .alert(LANDY_CUE);
    }
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to responder's re-cue (`1NT (2♣) 2♥ - 3♣ - 3♠ -`)
///
/// Responder is game-forcing and missing `asked`.  Opener bids the game
/// holding it, else takes the minor — which opener's `3m` already promised.
fn landy_recue_answer(minor: Suit, asked: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, stopper_in(asked))
        .rule(Bid::new(4, Strain::from(minor)), 20, hcp(0..))
}

/// Responder's answer to opener's stopper ask (`1NT (2♣) 2♥ - 2♠ -`)
///
/// `asked` is the major opener lacks.  With it stopped responder shows
/// notrump — cheaply on a minimum so opener can still decline
/// ([`landy_invite_answer`] handles that), or straight to game with values
/// it has not yet shown.  Without it, retreat to the minor at the cheapest
/// level opener's ask left, which opener's tolerance made safe.
///
/// The cheap notrump rung only exists over the low ask (`2♠` over the club
/// cue); over a 3-level ask there is nothing between it and 3NT, so the
/// minimum and the maximum share the game bid and only the retreat splits.
fn landy_ask_answer(minor: Suit, asked: Suit, ask: Bid) -> Rules {
    let notrump = cheapest_above(Strain::Notrump, ask);
    let retreat = cheapest_above(Strain::from(minor), ask);
    let game = Bid::new(4, Strain::from(minor));
    let mut rules = Rules::new().rule(
        Bid::new(3, Strain::Notrump),
        150,
        stopper_in(asked) & points(10..),
    );
    if notrump < Bid::new(3, Strain::Notrump) {
        rules = rules.rule(notrump, 145, stopper_in(asked) & points(..=9));
    }
    if retreat < game {
        rules = rules.rule(game, 120, points(10..));
        rules = rules.rule(retreat, 100, points(0..));
    }
    rules.rule(retreat, 20, hcp(0..))
}

// --- N1j: the BBA-ladder counter (`defense_2c_landy_bba`) -------------------

/// Responder's N1j BBA-ladder table over `1NT (2♣)` Landy
///
/// The anchor's own counter structure
/// (`docs/ai-bidder/bba-1nt-counter-defense.md`: a notrump ladder, wide
/// forced minor transfers, no gadget cues) with two deliberate deviations,
/// both evidence-backed — the values `X` is kept byte-identical (BBA never
/// doubles Landy; three experiments defended the row), and a game-forcing
/// both-minors takeout/splinter family occupies the cues' former slots.  The
/// club transfer sits on `2NT` rather than BBA's `2♠` because the takeout
/// pair spends both major cues; the reading ceiling that creates is priced in
/// the campaign doc — BBA decodes this lane through *our disclosed
/// uncontested scheme* (`Transfers if RHO bids clubs`), so exact readability
/// was never on the table.
///
/// | Call | Meaning | Weight |
/// | --- | --- | --- |
/// | `3NT` | game values, both majors stopped, no six-card minor | 180 |
/// | `2♥`/`2♠` | **GF takeout**, 4+♦ 4+♣, exactly two in the bid major (2-2 bids `2♥`) | 178/177 |
/// | `3♥`/`3♠` | **GF splinter**, 4+♦ 4+♣, 0-1 in the bid major | 176/175 |
/// | `2NT` / `3♣` | transfers to ♣/♦, 6+, `points(2..)` (weak signoff through GF) | 174/173 |
/// | `4♠` / `4♥` | **§N1p jam**, natural game, 6+ of their major | 172/171 |
/// | `3NT` | game values, ungated | 168 |
/// | `X` | values, `hcp(8..)` — the stack's row verbatim | 145 |
/// | `2♦` | weak natural 5+ — verbatim (`hcp(..=6)` under the cap arm) | 140 |
/// | Pass | finite catch-all | 0 |
///
/// The two-suited family outranks the transfers so a 6-4 minor hand shows the
/// whole picture; the transfers outrank the values double so a six-carder
/// never defends; a hand with a doubleton in one major and 0-1 in the other
/// splinters — the shortness is the message with more play in it.  No `6NT`
/// blast rung (BBA's 2.9%): opposite our 15-17 with a live Landy overcall, an
/// 18+ responder is arithmetic-impossible in the lane.
///
/// **§N1p — the two knobs that widen the values double.**  Both `3NT` rungs are
/// ungated on major length, and the cheaper one is ungated on everything but
/// `points(10..)`, so every ten-plus-point hand bids `3NT` and the `X`@145 is
/// capped at nine points.  Under
/// [`CompetitionKnobs::landy_notrump_no_major`][crate::bidding::agreements::CompetitionKnobs::landy_notrump_no_major]
/// both gain `len(♥, ..=3) & len(♠, ..=3)`, so the game hands holding
/// four-plus of a major they showed reach the double instead; the stoppers, the
/// transfers and the two-suited family are untouched, so a six-card minor still
/// transfers and short stoppers still count.  Under
/// [`CompetitionKnobs::landy_major_jam`][crate::bidding::agreements::CompetitionKnobs::landy_major_jam]
/// as well, a *strong* six-card major jams the auction with `4M` instead — weak
/// six-carders keep defending.  The jam is **on by default** (it swept its
/// standalone A/B); `landy_notrump_no_major` stays off (measured loss).
///
/// **§N1-lia** ([`landy_lia`]) re-rungs this table to Lia's own counter, as
/// refined on the lia2 forensic — six-card minors invitational-or-better at
/// `2♠`/`2NT`, the values `X` narrowed to two-plus in each major, one
/// both-minors takeout in two bands *below* it at `2♥`, a re-gated
/// excessive-diamond sign-off at `3♦`@142, the weak club sign-off at
/// `3♣`@141, and the escape at `2♦`@140 with its ceiling raised to eight
/// HCP.  **`landy_texas`** ([`landy_texas`]) independently moves the jam onto
/// South African Texas with the direct major freed for the uncontested
/// slam-try tier.  The head (`3NT`@180) and `3NT`@168, the `4M` jam and
/// `Pass`@0 are shared verbatim across all four combinations; the splinters
/// are shared but re-weighted under lia, whose takeout is a superset of their
/// shape.
fn landy_bba_responder(agreements: &Agreements) -> Rules {
    let both_minors = len(Suit::Clubs, 4..) & len(Suit::Diamonds, 4..);

    // §N1p: `3NT` denies a four-card major, so the game hands holding
    // four-plus of a suit they showed fall through to the values `X`@145
    // instead of burying it.  Spelled as paired `rule` calls, like
    // `landy_doubler_rebid`'s colour gate, because the two constraints are
    // different types.
    let deny_major = agreements.competition.landy_notrump_no_major;
    let no_major = || len(Suit::Hearts, ..=3) & len(Suit::Spades, ..=3);

    // The gated 3NT — the stack's rung verbatim (see `landy_responder` for
    // why it outranks everything and takes no stopper gate on clubs).
    let game = Bid::new(3, Strain::Notrump);
    let gated = points(10..)
        & stopper_in(Suit::Hearts)
        & stopper_in(Suit::Spades)
        & len(Suit::Clubs, ..=5)
        & len(Suit::Diamonds, ..=5);
    let mut rules = if deny_major {
        Rules::new().rule(game, 180, gated & no_major())
    } else {
        Rules::new().rule(game, 180, gated)
    };

    // The both-minors family.  N1j: the takeout names the doubleton (so
    // `2♠` is exactly 2=3=4=4) and the splinter names the 0-1; the takeout
    // therefore requires 2+ in the *other* major too, or the splinter would
    // lose its singleton hands to the cheaper call.  §N1-lia frees `2♠` for
    // the club rung, so `2♥` is the **only** takeout — and it moves *below*
    // the values `X`@145, where it is authored with the rest of the low rungs
    // (see the §N1-lia block under the double).  Only the splinters stay
    // here, re-weighted: Lia's takeout shape contains theirs, so under that
    // ladder the two families overlap and the weights have to arbitrate — a
    // game-forcing 0-1 major makes the more descriptive call.  N1j's exact
    // doubletons stay disjoint from the splinters, so its order is free and
    // is left as shipped.
    let lia = landy_lia(agreements);
    let (splinter_hearts, splinter_spades) = if lia { (179, 178) } else { (176, 175) };
    if !lia {
        rules = rules
            .rule(
                Bid::new(2, Strain::Hearts),
                178,
                both_minors.clone()
                    & len(Suit::Hearts, 2..=2)
                    & len(Suit::Spades, 2..)
                    & points(10..),
            )
            .alert(LANDY_TKO)
            .rule(
                Bid::new(2, Strain::Spades),
                177,
                both_minors.clone()
                    & len(Suit::Spades, 2..=2)
                    & len(Suit::Hearts, 3..)
                    & points(10..),
            )
            .alert(LANDY_TKO);
    }
    rules = rules
        .rule(
            Bid::new(3, Strain::Hearts),
            splinter_hearts,
            both_minors.clone() & len(Suit::Hearts, ..=1) & points(10..),
        )
        .alert(LANDY_SPL)
        .rule(
            Bid::new(3, Strain::Spades),
            splinter_spades,
            both_minors.clone() & len(Suit::Spades, ..=1) & points(10..),
        )
        .alert(LANDY_SPL);

    // The one-suited minors.  N1j: BBA's wide transfers (`2NT`→♣ / `3♣`→♦),
    // completed 100% — weak sign-off through game force, the invitation
    // deliberately spent on right-siding (the measured N1h/N1i trade:
    // `3♣ ← 2NT` −2.19 PD).  §N1-lia: the ladder drops a full level —
    // `2♠`/`2NT` are the six-card minors, invitational or better, and the
    // weak hands sign off one step higher.
    //
    // Six cards at both ends, at both colours.  That is Lia's own rule ("6+,
    // rarely 5") and it is what package B's forensic priced the *first*
    // build's five-card weak leg at: exactly five clubs was **+1.405
    // IMPs/fired white and −0.803 red** — 45% of the arm's whole both-vul
    // perfect-defense deficit — while six (+0.993/+0.668) and seven-plus
    // (+1.849/+1.683) won at both.  The colour gate that finding bought is
    // gone with the rung it gated: exactly five now signs off through the
    // takeout or the escape, so nothing in this table reads `vulnerable()`
    // any more.
    //
    // **The lia2 A/B re-rung the answers, not these two rules.**  Responder
    // names the step below its suit and *opener* bids the suit, so the rung
    // right-sides like a transfer — which is the half of the N1h/N1i trade
    // the measurement kept: over 4.6M boards the same-contract-other-seat
    // bucket was 26,664 boards NV / 27,911 BV and cost **−0.0014 / −0.0023
    // IMPs per board**, its worst cell the club rung itself (`2NT → 3♣`,
    // −3,630 / −5,600).  Losing declarership was never the natural rung's
    // point, so [`landy_lia_max_break`] takes it back: the completion is
    // opener's minimum default, and the two rungs above it answer the
    // question length never could — whether this is a game.
    rules = if lia {
        rules
            .rule(
                Bid::new(2, Strain::Spades),
                174,
                len(Suit::Clubs, 6..) & points(8..),
            )
            .alert(LANDY_MINOR)
            .rule(
                Bid::new(2, Strain::Notrump),
                173,
                len(Suit::Diamonds, 6..) & points(8..),
            )
            .alert(LANDY_MINOR)
    } else {
        rules
            .rule(
                Bid::new(2, Strain::Notrump),
                174,
                len(Suit::Clubs, 6..) & points(2..),
            )
            .alert(LANDY_TRANSFER)
            .rule(
                Bid::new(3, Strain::Clubs),
                173,
                len(Suit::Diamonds, 6..) & points(2..),
            )
            .alert(LANDY_TRANSFER)
    };

    // The ungated 3NT, the values X and the weak 2♦: the stack's rows,
    // byte-identical — except the 2♦ band under the cap arm, the N1i
    // `2♦ → Pass` lead isolated (the dropped 7-9 point hands pass).
    // §N1p's jam: a strong six-card major bids the game rather than
    // defending, above `3NT`@168 and below the transfers so the transfers
    // keep outranking the double.  `4♠` outranks `4♥` because with 6-6 the
    // better game is `4♠`; nothing else satisfies both.
    //
    // Independent of `landy_notrump_no_major` since 2026-08-30, and default
    // on since the standalone A/B swept all eight cells: the jam over an
    // *ungated* `3NT`@168, where `4M` substitutes for the game rather than
    // for the double.  §N1p's `jam vs nt` win was measured against the
    // double and does not transfer; see the "Verdict" block in the campaign
    // doc.
    if agreements.competition.landy_major_jam {
        let floor = agreements.competition.landy_texas_floor;
        if landy_texas(agreements) {
            // §N1-lia package C: the jam rides South African Texas so opener
            // declares, and the freed direct major is the uncontested NF
            // slam-try tier verbatim (`hcp(15..=direct_4m_max)`, opener
            // launching RKCB from the top).  A 16+ six-carder transfers and
            // drives its own `4NT` above the completion, exactly as
            // uncontested.  `4♦` outranks `4♣` as `4♠` outranks `4♥` today —
            // with 6-6 (which the slam tries' `len(other, ..5)` excludes) the
            // better game is spades.
            let slam_try_max = direct_4m_max(agreements);
            for (major, other, weight) in [
                (Suit::Spades, Suit::Hearts, 172),
                (Suit::Hearts, Suit::Spades, 171),
            ] {
                rules = rules.rule(
                    Bid::new(4, Strain::from(major)),
                    weight,
                    len(major, 6..) & len(other, ..5) & hcp(15..=slam_try_max),
                );
            }
            for (target, transfer, weight) in [
                (Suit::Spades, Strain::Diamonds, 170),
                (Suit::Hearts, Strain::Clubs, 169),
            ] {
                rules = rules
                    .rule(
                        Bid::new(4, transfer),
                        weight,
                        len(target, 6..) & points(floor..),
                    )
                    .alert(TEXAS);
            }
        } else {
            for (major, weight) in [(Suit::Spades, 172), (Suit::Hearts, 171)] {
                rules = rules.rule(
                    Bid::new(4, Strain::from(major)),
                    weight,
                    len(major, 6..) & points(floor..),
                );
            }
        }
    }
    rules = if deny_major {
        rules.rule(game, 168, points(10..) & no_major())
    } else {
        rules.rule(game, 168, points(10..))
    };
    // The values double.  §N1-lia adds a **shape** term the base ladder has
    // no use for: two-plus in each major.  Under that ladder the both-minors
    // takeout sits one weight *below* this rung, so the double is what a
    // both-minors hand with defense in their suits bids and the takeout is
    // what it bids with a short one — the split Lia's own rung set implies
    // once the takeout stops naming a major.
    //
    // The lia2 forensic prices it, and the cell is genuinely mixed: the 8,113
    // NV / 6,236 BV boards this term hands back to the double were worth
    // **+1.854 / +1.154 plain IMPs per fired** to the takeout that had them,
    // and **−1.305 / −2.169** under perfect defense.  Plain says take out, PD
    // says double, and the runner's own arbitration note says a mechanism
    // that *removes our penalty doubles* is the one PD loss this lane does
    // not wave through — which is what taking out with values does.  So the
    // double keeps them, and the split is pre-registered as the first
    // falsifier of the next arm rather than settled here.
    rules = if lia {
        rules
            .rule(
                Call::Double,
                145,
                hcp(8..) & len(Suit::Hearts, 2..) & len(Suit::Spades, 2..),
            )
            .alert(LANDY_VALUES)
            .penalty()
    } else {
        rules
            .rule(Call::Double, 145, hcp(8..))
            .alert(LANDY_VALUES)
            .penalty()
    };
    let escape = Bid::new(2, Strain::Diamonds);
    // §N1-lia's low rungs, every one of them under the double.
    //
    // **The takeout, in two bands.**  Lia states it as shape in the minors
    // and says nothing about the majors, so neither band carries a major
    // term and the rung order does that work instead: 8+ with two-plus in
    // each major doubles (above), 8+ with a short one takes out here, and
    // 10+ with a singleton splinters (above again).  The weak band is ours,
    // not hers, and it exists for the hand no other rung reaches — five-plus
    // clubs and exactly four diamonds at 4-7, too short of diamonds for the
    // `2♦` escape and too short of clubs for `3♣`.  Its five-card club
    // guarantee is what makes opener's `3♣`-before-`3♦` answer priority safe
    // ([`landy_lia_takeout_answer`]): with both minors opener bids the one
    // responder is longer in.
    //
    // **The sign-offs straddle the weak `2♦`@140**, and that placement is the
    // whole diamond ladder: with a bust and six clubs there is nothing below
    // `3♣`, so it outranks the escape; with a bust and diamonds the escape is
    // a level cheaper and outranks `3♦`, which takes only what `2♦` cannot.
    // The lia2 A/B measured what "cannot" has to mean.  Its worst two cells
    // were both diamond ones — `3♣ → 2♦` (−36,875 NV / −32,773 BV) and
    // `3♣ → 3♦` (−36,119 / −31,269) — the base arm's wide transfer against a
    // sign-off in a suit that was merely long.  So `3♦` re-gates to
    // **excessive** diamonds, seven of them or six with two of the top three,
    // and everything else diamond-ish and weak takes the cheaper escape,
    // whose ceiling rises to eight HCP to accept them ("bid `2♦` if
    // possible").  The `weak_2d_cap` knob keeps governing the base arm only:
    // it caps a rung this ladder has re-cut, so crossing them would measure
    // two edits at once.
    if lia {
        rules = rules
            .rule(
                Bid::new(2, Strain::Hearts),
                144,
                both_minors.clone() & points(8..),
            )
            .alert(LANDY_TKO)
            .rule(
                Bid::new(2, Strain::Hearts),
                143,
                len(Suit::Clubs, 5..) & len(Suit::Diamonds, 4..=4) & points(4..=7),
            )
            .alert(LANDY_TKO)
            .rule(
                Bid::new(3, Strain::Diamonds),
                142,
                (len(Suit::Diamonds, 7..)
                    | (len(Suit::Diamonds, 6..) & top_honors(Suit::Diamonds, 2..)))
                    & points(..=7),
            )
            .rule(
                Bid::new(3, Strain::Clubs),
                141,
                len(Suit::Clubs, 6..) & points(..=7),
            );
    }
    let floors = hcp(natural_floor_hcp(agreements)..) & points(natural_floor_pts(agreements)..);
    rules = if lia {
        rules.rule(escape, 140, len(Suit::Diamonds, 5..) & hcp(..=8) & floors)
    } else if agreements.competition.defense_2c_landy_weak_2d_cap {
        rules.rule(escape, 140, len(Suit::Diamonds, 5..) & hcp(..=6) & floors)
    } else {
        rules.rule(
            escape,
            140,
            len(Suit::Diamonds, 5..) & points(..=9) & floors,
        )
    };
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Opener completes the N1j `3♣` diamond transfer with the forced `3♦`
///
/// The diamond twin of [`complete_lebensohl_relay`]: constrained `hcp(0..)`
/// it projects nothing, and under `reading.completion_alerts` the alert
/// suppresses the natural walk's diamond reading of a forced puppet.
fn landy_bba_diamond_completion(agreements: &Agreements) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 100, hcp(0..))
        .alert_if(
            agreements.decision.reading.completion_alerts,
            LEBENSOHL_COMPLETION,
        )
}

/// Responder's rebid after a completed N1j minor transfer
/// (`1NT (2♣) 2NT - 3♣ -` / `… 3♣ - 3♦ -`)
///
/// Pass is the weak sign-off — and the invitational hand's stop, the wide
/// band's deliberate trade of the invitation for right-siding.  With game
/// values: show the one major stopper held (`3♥`/`3♠`, a cue not length —
/// opener supplies 3NT holding the other, [`landy_recue_answer`]), bid `3NT`
/// holding both, or start the `4m` slam try on the six-card source of tricks.
/// With [`CompetitionKnobs::landy_minor_slam_answer`][crate::bidding::agreements::CompetitionKnobs::landy_minor_slam_answer]
/// on (the default), [`landy_slam_answer`] continues with RKCB on a maximum and
/// `5m` otherwise; the off arm restores the historical floor-owned seat.  A
/// game force with neither stopper takes its chances in `3NT`: opener opened a
/// balanced 15-17, and five of a minor needs more than most of these hands hold.
fn landy_bba_transfer_rebid(minor: Suit) -> Rules {
    let mut rules = Rules::new();
    for (held, other, weight) in [
        (Suit::Hearts, Suit::Spades, 150),
        (Suit::Spades, Suit::Hearts, 149),
    ] {
        rules = rules
            .rule(
                Bid::new(3, Strain::from(held)),
                weight,
                points(10..) & stopper_in(held) & !stopper_in(other),
            )
            .alert(LANDY_CUE);
    }
    rules
        .rule(
            Bid::new(4, Strain::from(minor)),
            130,
            points(13..) & len(minor, 6..),
        )
        .rule(Bid::new(3, Strain::Notrump), 120, points(10..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer to the N1j `4m` slam try (`1NT (2♣) 2NT - 3♣ - 4♣ -`)
///
/// Empty unless [`CompetitionKnobs::landy_minor_slam_answer`][crate::bidding::agreements::CompetitionKnobs::landy_minor_slam_answer]
/// is on.  The `4m` above has shipped default-on since N1 with **no answer at
/// all** — the doctrine of the day was that "opener's continuation is
/// deliberately the floor's, a `4m` suit contract lets the floor cue-bid on to
/// slam", and it is wrong here for a reason the doctrine could not have known:
/// `instinct`'s `4NT` keycard ask is gated on `Context::undisturbed`, and this
/// lane is disturbed by construction, so the floor can never ask.  It also
/// never *reads* the `4m`, which shows a floor of zero and buries the ask's
/// `combined_points(29)` a second time (`docs/minor-transfer-slam.md`).
///
/// The shape is [`competition::rubensohl::kokish_kraft_slam_answer`][crate::bidding::american::competition::rubensohl::kokish_kraft_slam_answer]'s:
/// a maximum asks keycard, anything else declines to game in the minor.  The
/// `16` is a constant, not a payload — the arm prices the *answer*, and this
/// lane's responder floor (`13`) is not in question.
fn landy_slam_answer(minor: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 160, hcp(16..))
        .alert(slam::RKCB)
        .rule(Bid::new(5, Strain::from(minor)), 100, hcp(..16))
}

/// Opener's answer to an N1j takeout or splinter (`1NT (2♣) 2M/3M -`)
///
/// Notrump = the bid (short) major stopped **or** no four-card minor:
/// responder knows its own holding in the unbid major, so opener answers only
/// the unknown, and the minor-less branch doubles as the forced catch-all —
/// it leaves opener with seven-plus major cards, stopper-rich.  A minor pick
/// is 4+ (cheaper first with both) and denies the short-major stopper.
fn landy_bba_takeout_answer(short: Suit, over: Bid) -> Rules {
    let notrump = cheapest_above(Strain::Notrump, over);
    let no_minor = len(Suit::Clubs, ..=3) & len(Suit::Diamonds, ..=3);
    let mut rules = Rules::new().rule(notrump, 150, stopper_in(short) | no_minor);
    for (minor, weight) in [(Suit::Clubs, 100), (Suit::Diamonds, 99)] {
        rules = rules.rule(
            cheapest_above(Strain::from(minor), over),
            weight,
            len(minor, 4..),
        );
    }
    // No catch-all: the two branches above are already **total**, and the
    // formal `notrump`@20 that used to restate their union was deleted
    // 2026-08-27 (research-doc discrepancy #7).  If `no_minor` is false then
    // some minor is four-plus, so a pick fires; if it is true the notrump rung
    // does.  Verified rather than argued: removing the row is byte-identical
    // on `smoke-default --count 20000 --seed 1` and does not move the
    // published reading at `1N (2C) 2H - 2N -`.
    rules
}

/// Responder's placement over opener's notrump answer (`1NT (2♣) 2M - 2NT -`)
///
/// Opener claimed the short-major stopper (or minor-less majors), and the
/// unbid major is the holding responder judges itself — so the one probe left
/// is the cue of *that* major without its stopper ([`landy_bba_ask_answer`]
/// resolves it).  Else place: `4m` with a fifth card and slam interest (the
/// floor continues), `3NT` otherwise — a game force has nowhere lower to
/// stop.
fn landy_bba_takeout_rebid(other: Suit) -> Rules {
    let mut rules = Rules::new()
        .rule(
            Bid::new(3, Strain::from(other)),
            150,
            !stopper_in(other) & points(10..),
        )
        .alert(LANDY_CUE);
    for (minor, weight) in [(Suit::Clubs, 130), (Suit::Diamonds, 129)] {
        rules = rules.rule(
            Bid::new(4, Strain::from(minor)),
            weight,
            points(14..) & len(minor, 5..),
        );
    }
    rules.rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Opener's answer to responder's stopper ask (`1NT (2♣) 2M - 2NT - 3M' -`)
///
/// `asked` is the unbid major responder cannot stop: `3NT` holding it, else
/// the cheapest four-card minor at the four level.  The game force is
/// committed, and responder's continuation over a `4m` suit contract is
/// deliberately the floor's (the slam-exploration doctrine).
fn landy_bba_ask_answer(asked: Suit) -> Rules {
    let mut rules = Rules::new().rule(Bid::new(3, Strain::Notrump), 100, stopper_in(asked));
    for (minor, weight) in [(Suit::Clubs, 60), (Suit::Diamonds, 59)] {
        rules = rules.rule(Bid::new(4, Strain::from(minor)), weight, len(minor, 4..));
    }
    rules.rule(Bid::new(4, Strain::Clubs), 20, hcp(0..))
}

/// Responder's placement over opener's three-level minor pick
/// (`1NT (2♣) 2M - 3m -`)
///
/// Opener denied the short-major stopper, so `3NT` needs both majors held in
/// responder's own hand; `4m` re-opens the slam zone with extras (the floor
/// continues); else the game force lands in `5m` on the guaranteed 4-4.
fn landy_bba_pick_rebid(minor: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            points(10..) & stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        )
        .rule(Bid::new(4, Strain::from(minor)), 110, points(14..))
        .rule(Bid::new(5, Strain::from(minor)), 100, hcp(0..))
}

/// Opener's game acceptance over a §N1-lia invitational rung
///
/// The one rung the invitational band needs that the game-forcing first build
/// did not: responder has promised 8+ and nothing more, so **opener** owns the
/// size decision, exactly as it does over the stack lane's own invitation
/// ([`landy_minor_invite_answer`], same knob and same gate — `3NT` from the
/// top of the range with both of their majors stopped).  Every other rung in
/// the two answer tables is a description, so this one is authored once and
/// shared: without it the ladder describes its way into a partscore opposite
/// a maximum, which is what "invitational" means it must not do.
fn landy_lia_accept(floor: u8) -> Cons<impl Constraint + Clone> {
    stopper_in(Suit::Hearts) & stopper_in(Suit::Spades) & hcp(floor..)
}

/// Opener's answer to the §N1-lia takeout (`1NT (2♣) 2♥ -`)
///
/// [`landy_bba_takeout_answer`]'s priority **reversed**: a four-card minor
/// comes first (cheapest with both — the guaranteed 4-4 fit is the point of
/// the takeout), then `2NT` with the spade stopper, then the `2♠` ask with
/// neither.  Lia's takeout names no short major, so the one question `2NT`
/// can answer is a *specific* stopper — spades, because nothing else in the
/// structure ever promises hearts and LHO leads their longer major; responder
/// resolves hearts with the `3♥` cue ([`landy_bba_takeout_rebid`] reused
/// verbatim one seat later).  Total by construction: `2♠`@100 is the vacuous
/// catch-all, read by exclusion and alerted by hand ([`LANDY_ASK`] — the
/// invariant's witness cannot see an `hcp(0..)` constraint).
///
/// `3NT`@160 on top is [`landy_lia_accept`]: the takeout is 8+, not a game
/// force, so a maximum with both majors stopped bids the game rather than
/// describing into `3♣` on 24 combined points.
///
/// **Known defect, deliberately left standing.**  The lia2 refinement gave
/// `2♥` a second, weak band (5+♣ / exactly 4♦, 4-7), and opener cannot tell
/// the two apart — so the accept can now land opposite four points, and
/// `2♥ - 3NT -` is an authored sit with no pull.  The alternative is to weight
/// the accept *below* the two minor picks, which protects the weak hand and
/// gives up the 8-9-opposite-maximum game; both are one weight and neither is
/// measured, so the iron rule says pick neither on analysis.  Pre-registered
/// as falsifier 2 of the refinement's arm
/// (`docs/one-notrump-competitive.md` §N1-lia).
fn landy_lia_takeout_answer(agreements: &Agreements) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            160,
            landy_lia_accept(agreements.notrump.size_ask_accept_floor),
        )
        .rule(Bid::new(3, Strain::Clubs), 150, len(Suit::Clubs, 4..))
        .rule(Bid::new(3, Strain::Diamonds), 149, len(Suit::Diamonds, 4..))
        .rule(Bid::new(2, Strain::Notrump), 120, stopper_in(Suit::Spades))
        .rule(Bid::new(2, Strain::Spades), 100, hcp(0..))
        .alert(LANDY_ASK)
}

/// Responder's placement over opener's minor pick (`1NT (2♣) 2♥ - 3♣ -`)
///
/// [`landy_bba_pick_rebid`] re-banded for the invitational takeout: the same
/// `3NT` on both majors held, the same `4m` slam move, but the `5m`@100 that
/// used to be a game-forcing catch-all is gated on game values and `Pass`@0
/// is the finite one.  Opener picked a minor in a guaranteed eight-card fit,
/// so the 8-9 hand has its contract and passing it is the whole point of
/// running the band down from a game force.
fn landy_lia_pick_rebid(minor: Suit) -> Rules {
    let strain = Strain::from(minor);
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            points(10..) & stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        )
        .rule(Bid::new(4, strain), 110, points(14..))
        .rule(Bid::new(5, strain), 100, points(10..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's placement over opener's `2NT` spade-stopper answer
/// (`1NT (2♣) 2♥ - 2NT -`)
///
/// [`landy_bba_takeout_rebid`] re-banded the same way: its `3NT`@100 was the
/// game force's catch-all, and under the invitational band a `Pass`@0 has to
/// be the finite rung instead — `2NT` is already a contract, and 8 opposite a
/// minimum 15 is not a game.  The `3♥` cue and the two `4m` slam moves are
/// unchanged, both already gated on values.
///
/// One rung answers the takeout's **weak** band: `3♣`@60 on five-plus clubs
/// and at most seven points.  That band is 5+♣ with exactly four diamonds, so
/// opener's `2NT` — which denied a four-card minor — is facing a five-two or
/// worse club fit and no values, and sitting it is the one thing the weak
/// hand must not do.  Opener passes the pull ([`multi_signoff_pass`]).
fn landy_lia_takeout_rebid() -> Rules {
    let mut rules = Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            150,
            !stopper_in(Suit::Hearts) & points(10..),
        )
        .alert(LANDY_CUE);
    for (minor, weight) in [(Suit::Clubs, 130), (Suit::Diamonds, 129)] {
        rules = rules.rule(
            Bid::new(4, Strain::from(minor)),
            weight,
            points(14..) & len(minor, 5..),
        );
    }
    rules
        .rule(Bid::new(3, Strain::Notrump), 100, points(10..))
        .rule(
            Bid::new(3, Strain::Clubs),
            60,
            len(Suit::Clubs, 5..) & points(..=7),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's placement over opener's §N1-lia `2♠` ask
/// (`1NT (2♣) 2♥ - 2♠ -`)
///
/// Opener denied a four-card minor and the spade stopper, so `3NT` needs
/// spades from responder's own hand: holding both majors with game values,
/// bid it; holding spades but not hearts, cue `3♥` for opener's heart stopper
/// ([`landy_bba_ask_answer`] resolves it — its minor rungs are dead there,
/// opener having denied four, so its `4♣`@20 carries the stopper-dead
/// endings).
///
/// The catch-all is `3♣`@20, **not** the first build's `4♣`@20, and the swap
/// fixes two things the invitational band turned from ugly into wrong.  That
/// rung was an unalerted artificial call — vacuous constraint, so the
/// invariant's witness could not see it name a suit responder need not hold —
/// and it committed to the five level opposite a hand that has promised only
/// eight points.  `3♣` names a suit responder does hold — 4+ by the takeout —
/// so it is natural and the alert goes.  It is not a promise of a *fit*: our
/// `1NT` caps both majors at four ([`NotrumpShape::Wide6322`][crate::bidding::agreements::NotrumpShape],
/// off-shape opt-in), so an opener that denied four in both minors is exactly
/// 3-3, 3-2 or 2-3 there, and the landing is a 4-3 at best and a 4-2 at worst.
/// Nothing better exists on responder's side of this auction: passing the ask
/// is not on offer (`2♠` cues a suit neither hand holds), and notrump needs a
/// spade stopper both hands have denied.  Opener repairs it instead,
/// correcting to `3♦` with a club doubleton ([`landy_lia_ask_landing`]) — the
/// same edit that gives this newly-reachable node below the game its authored
/// answer.
fn landy_lia_ask_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            150,
            points(10..) & stopper_in(Suit::Spades) & stopper_in(Suit::Hearts),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            140,
            points(10..) & stopper_in(Suit::Spades),
        )
        .alert(LANDY_CUE)
        .rule(Bid::new(3, Strain::Clubs), 20, hcp(0..))
}

/// Opener over the `2♠` ask's landing (`1NT (2♣) 2♥ - 2♠ - 3♣ -`)
///
/// A seat the first build did not have: its catch-all was `4♣`@20, above
/// `3NT`, so nothing could be bid between. Dropping the landing to `3♣`@20
/// opens a node below the game, and unauthored it is the floor's — which bids
/// **`3NT` on every probed hand**, in the strain opener's own `2♠` ask has
/// just denied a stopper in. So opener answers, and it has exactly one thing
/// to say: with a club doubleton and three diamonds, correct to the better
/// minor — responder holds 4+ of each by the takeout, so `3♦` turns the worst
/// case (4-2) into a 4-3. Otherwise pass; `Pass`@0 is the finite catch-all.
/// Both calls are natural, so neither takes an alert.
fn landy_lia_ask_landing() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Diamonds),
            100,
            len(Suit::Clubs, ..=2) & len(Suit::Diamonds, 3..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's **Max-break+** answer to a §N1-lia minor rung
/// (`1NT (2♣) 2♠ -` / `2NT -`)
///
/// Three rungs, one shape for both legs, and the ordering is the design:
///
/// | Answer | Meaning |
/// | --- | --- |
/// | `relay` — the step below the completion (`2NT` over `2♠`, `3♣` over `2NT`) | **super-accept**: maximum with three-card support ([`LANDY_SUPER`]) |
/// | `3NT` | maximum, no super-accept — [`landy_lia_accept`] verbatim |
/// | `completion` (`3♣` over `2♠`, `3♦` over `2NT`) | the minimum default, and the finite catch-all |
///
/// It replaces the by-length answer the lia2 A/B measured, and the swap is
/// two findings in one edit.  **Length answered the wrong question**: the rung
/// is invitational-or-better, so what responder needs to hear is whether this
/// is a game, and a three-versus-two split told it only which partscore to
/// pick.  **And the completion right-sides**: on the length table responder
/// declared its own suit, which cost the arm 26,664 NV / 27,911 BV
/// same-contract-other-seat boards at −0.0014 / −0.0023 IMPs per board — the
/// half of the N1h/N1i transfer trade that was worth keeping.  Here opener
/// bids the suit on every minimum, which is most of them.
///
/// Three-card support is the reversible default on the super-accept: opposite
/// the rung's six that is a nine-card fit, which is what a maximum can raise
/// on.  Flip it to `4..` if a probe says the ninth card is not enough.
///
/// The completion carries [`LEBENSOHL_COMPLETION`] under
/// `reading.completion_alerts`, exactly as the N1j transfer's does one lane
/// over: constrained `hcp(0..)` it projects nothing, so without the alert the
/// natural walk reads opener for the minor it just bid — a suit the rule says
/// nothing about, and one opener can hold two of.  The negative inference the
/// rung *does* carry (not a maximum, or a maximum with neither support nor
/// both stoppers) still falls out of `bid_exclusion` off its own siblings.
fn landy_lia_max_break(agreements: &Agreements, minor: Suit, relay: Bid, completion: Bid) -> Rules {
    let floor = agreements.notrump.size_ask_accept_floor;
    Rules::new()
        .rule(relay, 161, hcp(floor..) & len(minor, 3..))
        .alert(LANDY_SUPER)
        .rule(Bid::new(3, Strain::Notrump), 160, landy_lia_accept(floor))
        .rule(completion, 100, hcp(0..))
        .alert_if(
            agreements.decision.reading.completion_alerts,
            LEBENSOHL_COMPLETION,
        )
}

/// Responder's seat over opener's §N1-lia game acceptance
/// (`1NT (2♣) 2♠ - 3NT -`)
///
/// The rung [docs/minor-transfer-slam.md](../../../../docs/minor-transfer-slam.md)
/// owes every uncapped minor rung: `2♠`/`2NT` are invitational-or-**better**
/// with no ceiling, so opener's `3NT` can land opposite a hand that wants
/// slam, and an unauthored `4m` there reads as nothing while the floor's
/// keycard ask is gated on `Context::undisturbed` — which this lane never is.
/// So the `4m` is authored on the same gate the quiet ladder uses
/// ([`landy_bba_transfer_rebid`]'s slam try: game values and the six-card
/// source of tricks), and [`landy_slam_answer`] answers it.  `Pass`@0 is the
/// finite catch-all and the common case.
fn landy_lia_accept_rebid(minor: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::from(minor)),
            100,
            points(13..) & len(minor, 6..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's rebid over opener's §N1-lia super-accept
/// (`1NT (2♣) 2♠ - 2NT -` / `… 2NT - 3♣ -`)
///
/// Opener has a maximum and a nine-card fit, so the two committing rungs are
/// the point of the seat: `3NT` on both of their majors held, and the `4m`
/// slam try on the same gate every other lia leg uses.  The `3NT` gate is
/// `points(9..)`, one point below every other lia rung's, because opener has
/// already published `hcp(16..)` here and nowhere else — nine opposite a
/// declared maximum is the 25 the game wants, and leaving it at ten would
/// strand exactly the hand the super-accept exists to find.  Below them,
/// `retreat` — three of the minor — is the finite catch-all rather than a
/// `Pass`, because on the diamond leg the super-accept is `3♣` and passing it
/// would leave a six-one fit on the table; on the club leg it costs the 2NT
/// partscore, which a nine-card fit and 24+ combined points is not the hand
/// for.  Opener sits for it ([`multi_signoff_pass`]).
fn landy_lia_super_rebid(minor: Suit, retreat: Bid) -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::from(minor)),
            130,
            points(13..) & len(minor, 6..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            points(9..) & stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        )
        .rule(retreat, 0, hcp(0..))
}

/// The opponents' entries a §N1-lia contested tail is authored against
///
/// Every call above `floor` in the band the censused arm actually uses: their
/// two majors at every level they bid them, plus the two minors at the three
/// level.  The band stops at `4♠` because nothing above it appears — and it
/// **includes** the four level, though a first draft stopped at three on the
/// argument that the floor's delayed double there (47% of `2♠ (4♥)` boards,
/// 74% of `2♠ (4♠)`) might be right.  Step 0 priced it: those two cells are
/// −3.434 and −4.268 IMPs/fired, the worst per-fired cells in the whole `2♠`
/// bucket.  The floor's four-level double is not a judgement it earns; it is
/// the same blind push one level higher.
fn landy_lia_entries(floor: Bid) -> impl Iterator<Item = Bid> {
    [
        Bid::new(2, Strain::Hearts),
        Bid::new(2, Strain::Spades),
        Bid::new(3, Strain::Clubs),
        Bid::new(3, Strain::Diamonds),
        Bid::new(3, Strain::Hearts),
        Bid::new(3, Strain::Spades),
        Bid::new(4, Strain::Hearts),
        Bid::new(4, Strain::Spades),
    ]
    .into_iter()
    .filter(move |bid| *bid > floor)
}

/// Their major, if the call they just made named one
///
/// The §N1-lia contested tables key their penalty double on the suit doubled,
/// and [`LANDY_PENALTY`] publishes length in **their major** — so a minor
/// entry (`2♠ (3♦)`) and a double take no double of ours under that slug.
fn their_major(over: Option<Bid>) -> Option<Suit> {
    over?
        .strain
        .suit()
        .filter(|suit| matches!(suit, Suit::Hearts | Suit::Spades))
}

/// Opener's seat when they compete over a §N1-lia INV+ rung
/// (`1NT (2♣) 2♠ (3♠)`, `… 2NT (3♣)`, …)
///
/// **Three rungs and no catch-all**, which is the lia2 A/B's verdict on the
/// seat stated as a table.  The 2026-09-01 build authored `Pass`@0 here on the
/// census argument that the floor over-competes — it pushes `4♣` on 72% of
/// `2♠ (3♠)` boards — and the measurement says the census read the right
/// behaviour and drew the wrong conclusion.  A `Pass`@0 is a **decision to
/// sell out**, and where it went in over rungs that promised nothing it sold
/// out to a floor that was right: `1NT (2♣) 2♦ (2♥|2♠) -` alone cost
/// **−22,119 plain IMPs NV / −13,364 BV** over 14,717 boards, 14,699 of them
/// after our own escape, against a baseline whose floor competed to `3♦`.
/// Those registrations are gone; this table now rides only the rungs that do
/// promise something, and its residue reaches the floor by rejection —
/// **which needs an exact `Pattern::node`**, since a guarded fallback's
/// all-−∞ logits are returned unchecked and read as the same Pass (package
/// A's silent no-op, `Trie::resolve_floored`).
///
/// * `accept` — the invitation's own acceptance rung, where their call left
///   room for it below `3NT`.  It is [`landy_lia_accept`]'s `3NT` verbatim,
///   so an accept that was available uncontested does not vanish because they
///   competed.
/// * `X` — penalty on `len(major, 4..)` and nothing else, the gate
///   `probe-landy-opener-oracle` measured at opener's *other* seat in this
///   lane (`1NT (2♣) X (2♥)`): `2Mx` wins every four-plus-trump bucket at both
///   colours, on a minimum as well as a maximum, with or without a stopper,
///   while a doubleton loses at every strength.  Two narrowings of that gate
///   are deliberate and pre-registered as risks — the oracle priced the **two**
///   level, so this stops at the three; and there responder had shown
///   `hcp(8..)`, so here it rides only a rung that promises as much.
/// * `complete` — the rung's own completion, taken while it is still cheap
///   enough to be legal.  Responder is six-plus and invitational; opener
///   holding three is a nine-card fit, and competing to it right-sides the
///   contract as the quiet table does.  In practice this fires at
///   `2NT (3♣)` and nowhere else, because the club leg's completion is `3♣`
///   itself and every entry above `2♠` is already past it.
///
/// `over` is `None` for their **balancing double**, which names no suit: the
/// penalty rung drops, `3NT` stays available, and the seat is a two-rung
/// table.  That arm is why the parameter is an `Option` —
/// [`landy_lia_entries`] yields bids only.
fn landy_lia_overcalled(
    over: Option<Bid>,
    complete: Option<Bid>,
    agreements: &Agreements,
) -> Rules {
    let mut rules = Rules::new();
    let game = Bid::new(3, Strain::Notrump);
    if over.is_none_or(|bid| game > bid) {
        rules = rules.rule(
            game,
            100,
            landy_lia_accept(agreements.notrump.size_ask_accept_floor),
        );
    }
    if let Some(major) = over
        .filter(|bid| bid.level.get() <= 3)
        .and_then(|bid| their_major(Some(bid)))
    {
        rules = rules
            .rule(Call::Double, 90, len(major, 4..))
            .alert(LANDY_PENALTY)
            .penalty();
    }
    if let Some((bid, minor)) = complete
        .filter(|bid| over.is_none_or(|entry| *bid > entry))
        .and_then(|bid| Some((bid, bid.strain.suit()?)))
    {
        rules = rules.rule(bid, 80, len(minor, 3..));
    }
    rules
}

/// Responder's seat when the **overcaller** re-enters over opener's §N1-lia
/// length answer (`1NT (2♣) 2♠ - 3♣ (3♥)`, `… 2NT - 3♦ (X)`, …)
///
/// The seat the campaign doc's "the rest is opponents entering later" names,
/// and the seat rotation is worth stating because it inverts the obvious
/// reading: with `O L R A` clockwise, `2♠ - 3♣ (3♥)` indexes O L R A O **L**,
/// so the hand that re-enters is the **overcaller**, whose partner has already
/// passed.  It has shown 5-4-or-better in the majors and is bidding shape into
/// a known 15-17, which is the most double-worthy call in the lane — and
/// responder, sitting immediately after it, is the hand that plays over it.
///
/// Responder is also the only seat that knows where in the rung's uncapped
/// band it sits, so unlike [`landy_lia_overcalled`] this table carries the
/// committing calls:
///
/// * `3NT` — the game, on both of the majors they showed, exactly
///   [`landy_bba_pick_rebid`]'s gate; opener's 15-17 and responder's
///   `points(10..)` are 25+ between them.
/// * `X` — penalty, [`LANDY_PENALTY`]'s claim honoured by `len(major, 4..)`,
///   and gated on **`hcp`** rather than `points` because distribution does not
///   defend: the `2NT` rung reaches `points(10..)` on a seventh diamond and
///   eight HCP, and that hand must not double.
/// * `4m` — the finite game-forcing rung, so the top of the band **never
///   passes**.  This is the "no forcing channel" half of defect 1 stated as a
///   rung: today 32% of `2♠ - 3♣ (3♥)` boards see the floor push `4♣` with no
///   strength gate at all and 68% see it pass, game force or not.
/// * `Pass`@0 — the invitational hand defends, and the catch-all the guarded
///   registration requires.
///
/// * `signoff` — the super-accept leg's retreat, [`landy_lia_super_rebid`]'s
///   three-of-the-minor kept alive under interference.  Without it
///   `2♠ - 2NT (X)` strands the invitational hand in a doubled notrump it had
///   an authored rescue from one call earlier: a contested tail must not
///   delete a rung the quiet tail offers.
///
/// `over` is `None` for their double, which names no suit: the penalty rung
/// drops and the rest stands, [`multi_escape_overcalled`]'s `Option<Bid>`
/// idiom.
fn landy_lia_contested_rebid(
    minor: Suit,
    floor: Bid,
    over: Option<Bid>,
    signoff: Option<Bid>,
) -> Rules {
    let game = Bid::new(3, Strain::Notrump);
    let force = Bid::new(4, Strain::from(minor));
    let mut rules = Rules::new();
    if game > floor {
        rules = rules.rule(
            game,
            150,
            points(10..) & stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        );
    }
    if let Some(major) = their_major(over) {
        rules = rules
            .rule(Call::Double, 140, hcp(10..) & len(major, 4..))
            .alert(LANDY_PENALTY)
            .penalty();
    }
    if force > floor {
        rules = rules.rule(force, 120, points(10..));
    }
    if let Some(escape) = signoff.filter(|bid| *bid > floor && *bid < force) {
        rules = rules.rule(escape, 60, points(..=9));
    }
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer when the advancer raises over an N1j takeout or splinter
/// (`1NT (2♣) 2♥ (2♠/3♥/3♠) …`)
///
/// [`landy_cue_overcalled`]'s doctrine compressed further: notrump with the
/// short major stopped, the cheapest minor pick, and Pass for the rest — safe
/// because responder's game force guarantees another turn.
fn landy_bba_takeout_overcalled(short: Suit, over: Bid) -> Rules {
    let notrump = cheapest_above(Strain::Notrump, over);
    let mut rules = Rules::new();
    if notrump <= Bid::new(3, Strain::Notrump) {
        rules = rules.rule(notrump, 150, stopper_in(short));
    }
    for (minor, weight) in [(Suit::Clubs, 100), (Suit::Diamonds, 99)] {
        rules = rules.rule(
            cheapest_above(Strain::from(minor), over),
            weight,
            len(minor, 4..),
        );
    }
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// The N1j BBA-ladder registration — responder's table, opener's answers, the
/// transfer completions and rebids, and the interfered tails
///
/// Registered *instead of* the stack's entries ([`lebensohl_package`]
/// branches): the two tables disagree on nearly every rung, so an overlay
/// would leave stack rows shadowing ladder rows.  The tails follow the N1f
/// idiom — their `X` takes no room, so the immediate answer re-registers
/// verbatim and every deeper `X`-then-bid suffix rebases onto the clean
/// subtree; their raise gets the compressed ladder; the doubled transfers are
/// still completed.  The splinter's answers land at game or in the slam zone,
/// where the floor takes over, so only the two-level takeout carries authored
/// placements.
fn landy_bba_entries(agreements: &Agreements) -> Vec<Entry> {
    const OVER: &str = "P* 1NT (2♣)";
    let mut entries = Vec::new();

    entries.extend(rows_of(
        Pattern::after("P* 1NT", "(2♣)"),
        landy_bba_responder(agreements),
    ));
    entries.extend(rows_of(Pattern::after(OVER, "X -"), landy_double_answer()));

    // §N1p's jam is a sign-off: opener passes it.  The node is insurance
    // against a *measured* floor defect — §N1o's forensic caught the floor
    // cue-bidding this lane's four-level to `6♥` doubled — and it does forgo
    // slam on the fifteen-plus slice, so it is the first thing to relax if the
    // arm reads mixed.  Under §N1-lia's package C ([`landy_texas`]) the jam
    // rides the transfers instead: opener completes (their double takes no
    // room — complete anyway, and rebase the deeper X-then-bid suffixes), the
    // 16+ drive and its RKCB ladder sit above the completion, and the freed
    // direct majors take the uncontested slam-try answer with the ladder
    // above it.
    if agreements.competition.landy_major_jam {
        if landy_texas(agreements) {
            for (rung, into) in [("4♣", Suit::Hearts), ("4♦", Suit::Spades)] {
                let game = Bid::new(4, Strain::from(into));
                for suffix in [format!("{rung} -"), format!("{rung} (X)")] {
                    entries.extend(rows_of(
                        Pattern::after(OVER, &suffix),
                        complete_texas(into, agreements),
                    ));
                }
                entries.push(systems_on_over_double(
                    &format!("{OVER} {rung}"),
                    &game.to_string(),
                ));
                // Uncontested, the seat above the completion belongs to
                // `instinct()`, which never pulls a completed transfer.  This
                // lane's floor is the *learned* one, and §N1o's forensic
                // caught it cue-bidding a dead four-level to `6♥` doubled —
                // so the drive rebid gets the sit rail the jam node carries:
                // 16+ drives `4NT`, everything else passes by authorship.
                let completed = format!("{OVER} {rung} - {game} -");
                entries.extend(rows_of(
                    Pattern::node(&completed),
                    texas_slam_drive_rebid(agreements).rule(Call::Pass, 0, hcp(0..)),
                ));
                if agreements.notrump.texas_slam_drive {
                    entries.extend(slam::rkcb_rows(&completed, into));
                }
            }
            for major in [Suit::Hearts, Suit::Spades] {
                let node = format!("{OVER} {} -", Bid::new(4, Strain::from(major)));
                entries.extend(rows_of(Pattern::node(&node), slam_try_answer()));
                entries.extend(slam::rkcb_rows(&node, major));
            }
        } else {
            for path in ["4♠ -", "4♥ -"] {
                entries.extend(rows_of(Pattern::after(OVER, path), multi_signoff_pass()));
            }
        }
    }

    // The doubler's own rebid, once their advance has named the major.  Four
    // paths, no correction leg: the Landy overcaller passes the preference
    // 94.5%/96.7% of the time (probed at seat 1, filtered to hands it actually
    // overcalls `2♣` with), so `X (2♥) - -` and `X (2♠) - -` carry essentially
    // all of the preference traffic — and their artificial `2♦` escape is
    // pulled to a major 79.4% of the time and passed never, so the two escape
    // legs are live seats taking the same table with the major now named.
    //
    // `X (2NT)` and opener's own `X (2M)` seat are deliberately absent.  After
    // the strong advance the overcaller jumps to `4M` 54.3% of the time and to
    // slam another 13.5%, and BBA never doubles at opener's seat in either the
    // Landy or the Multi lane (`--mode opener-c-x2h`/`opener-d-x2h`, no `X`
    // bucket over 0.5% in eight cells) — the seat §N1k authored and lost.
    if let Some(ladder) = landy_doubler_ladder(agreements) {
        for (path, major) in [
            ("X (2♥) - -", Suit::Hearts),
            ("X (2♠) - -", Suit::Spades),
            ("X (2♦) - (2♥)", Suit::Hearts),
            ("X (2♦) - (2♠)", Suit::Spades),
        ] {
            // An exact node, not an `after` guard: only an exact node's
            // rejection falls through to the floor (`Trie::resolve_floored`'s
            // single fall-through returns a guarded fallback's logits without
            // checking mass), and rejection is precisely how the
            // `landy_doubler_catchall=false` arm hands three trumps or fewer
            // back to the floor.
            entries.extend(rows_of(
                Pattern::node(&format!("{OVER} {path}")),
                landy_doubler_rebid(major, ladder, agreements),
            ));
            // The repeated double is penalty by this lane's polarity rule, so
            // opener sits on it rather than answering a takeout.  Every arm
            // carries the `X`, so every arm carries this.
            entries.extend(rows_of(
                Pattern::after(OVER, &format!("{path} X -")),
                multi_signoff_pass(),
            ));
            // An answer table is registered only where its question exists: a
            // rung below a deleted rung is dead registration, and a live node
            // under a call no arm makes is a book node shadowing the floor for
            // no reason.
            if ladder != DoublerLadder::Px {
                entries.extend(rows_of(
                    Pattern::after(OVER, &format!("{path} 2NT -")),
                    kokish_kraft_invite_answer(),
                ));
                for minor in [Suit::Clubs, Suit::Diamonds] {
                    entries.extend(rows_of(
                        Pattern::after(
                            OVER,
                            &format!("{path} {} -", Bid::new(3, Strain::from(minor))),
                        ),
                        landy_minor_rebid_answer(major),
                    ));
                }
            }
            if ladder == DoublerLadder::Full {
                entries.extend(rows_of(
                    Pattern::after(OVER, &format!("{path} 4NT -")),
                    multi_quant_answer(),
                ));
            }
        }
    }
    // Opener's own seat, one call before the doubler's (§N1m).  Two legs only:
    // the relay's balancing analogue (`X (2♦) - (2♥) - -`) is 5.1% / 3.3% of
    // the seat's boards and the oracle prices **every** candidate there
    // negative at both vulnerabilities except par — opener is in the pass-out
    // seat and the live method already defends their `2♥`, so there is nothing
    // to buy.  Their runout over our double (`X (2♥) X (2♠)` and its siblings)
    // stays the floor's: the alert publishes opener's four-plus length, so the
    // floor decides on true information rather than a phantom, and the §N1l
    // twin one call later takes the same shape.
    if agreements.competition.landy_opener_px {
        for (path, major) in [("X (2♥)", Suit::Hearts), ("X (2♠)", Suit::Spades)] {
            entries.extend(rows_of(
                Pattern::after(OVER, path),
                landy_opener_rebid(major, agreements.competition.landy_opener_rungs),
            ));
            // The doubler sits for the penalty double, and passes both notrump
            // rungs: the ordering caps the `2NT` at fifteen, so it is a
            // sign-off, and `3NT` is already the game.
            let mut tails = vec![format!("{path} X -")];
            if agreements.competition.landy_opener_rungs {
                tails.push(format!("{path} 2NT -"));
                tails.push(format!("{path} 3NT -"));
            }
            for tail in tails {
                entries.extend(rows_of(Pattern::after(OVER, &tail), multi_signoff_pass()));
            }
        }
    }
    entries.extend(rows_of(
        Pattern::after(OVER, "2♦ -"),
        landy_signoff_answer(),
    ));

    // The minor one-suiters.  N1j: the wide transfers — forced completion
    // (doubled or not), responder's rebid, and opener's answer to the stopper
    // show.  §N1-lia: the ladder sits a level lower and the completion comes
    // **back** — [`landy_lia_max_break`] answers with a super-accept, a `3NT`
    // accept, or the completion as its minimum default, so opener declares on
    // the common branch exactly as it does over a transfer.  Responder rebids
    // off [`landy_lia_super_rebid`] over the super-accept and
    // [`landy_bba_transfer_rebid`] over the completion, and the N4-KK slam
    // machinery re-hangs byte-identical on both.
    if landy_lia(agreements) {
        for (minor, rung, completion, relay) in [
            (
                Suit::Clubs,
                Bid::new(2, Strain::Spades),
                Bid::new(3, Strain::Clubs),
                Bid::new(2, Strain::Notrump),
            ),
            (
                Suit::Diamonds,
                Bid::new(2, Strain::Notrump),
                Bid::new(3, Strain::Diamonds),
                Bid::new(3, Strain::Clubs),
            ),
        ] {
            let four = Bid::new(4, Strain::from(minor));
            let accept = Bid::new(3, Strain::Notrump);
            for suffix in [format!("{rung} -"), format!("{rung} (X)")] {
                entries.extend(rows_of(
                    Pattern::after(OVER, &suffix),
                    landy_lia_max_break(agreements, minor, relay, completion),
                ));
            }
            entries.push(systems_on_over_double(
                &format!("{OVER} {rung}"),
                &completion.to_string(),
            ));
            // Opener took the size decision the invitational band hands it,
            // and the game is the contract for all but one hand: the one that
            // wanted slam.  `docs/minor-transfer-slam.md`'s rule is that an
            // uncapped minor rung owes a `4m` rung above its `3NT` **and an
            // authored answer** — unauthored it reads as nothing, and the
            // floor's keycard ask is gated on `Context::undisturbed`, which
            // this lane never is.  [`landy_lia_accept_rebid`] is that rung;
            // its answer and RKCB ladder re-hang exactly as on the other two
            // legs.  The `(X)` tails rebase through `systems_on_over_double`.
            entries.extend(rows_of(
                Pattern::after(OVER, &format!("{rung} - {accept} -")),
                landy_lia_accept_rebid(minor),
            ));
            if agreements.competition.landy_minor_slam_answer {
                for tail in ["-", "(X)"] {
                    let path = format!("{OVER} {rung} - {accept} - {four} {tail}");
                    entries.extend(rows_of(Pattern::node(&path), landy_slam_answer(minor)));
                    entries.extend(slam::rkcb_rows(&path, minor));
                }
            }
            // **The contested surface, restricted to the rungs that promise
            // something.**  The 2026-09-01 build authored both seats over
            // every rung; the lia2 A/B says the seats over the *weak* rungs
            // were selling out to a floor that was competing correctly, so
            // they are gone (see [`landy_lia_overcalled`]) and only `2♠` and
            // `2NT` — invitational-or-better, so opener has a promise to act
            // on — carry a table here.
            //
            // The asymmetry between the two seats is the design: opener
            // cannot know where in the uncapped band responder sits, so its
            // table is three narrow rungs with **no catch-all** and the
            // residue reaches the floor; responder does know, so its table
            // ([`landy_lia_contested_rebid`]) carries every committing call
            // and a finite game-forcing rung so the top of the band is never
            // stranded.  Opener's registrations are exact `Pattern::node`s
            // for that reason — package A's finding, that only an exact
            // node's rejection reaches the floor.
            for over in landy_lia_entries(rung) {
                entries.extend(rows_of(
                    Pattern::node(&format!("{OVER} {rung} ({over})")),
                    landy_lia_overcalled(Some(over), Some(completion), agreements),
                ));
                // Opener sold out and so did the overcaller: responder, the
                // seat that knows its strength, places the contract.
                entries.extend(rows_of(
                    Pattern::after(OVER, &format!("{rung} ({over}) - -")),
                    landy_lia_contested_rebid(minor, over, Some(over), None),
                ));
                // Opener sits for responder's penalty double, and for the
                // game-forcing `4m` — the rail the §N1o forensic bought:
                // above a competitive four-level minor this lane's *learned*
                // floor cue-bids on to `6♥` doubled, and here opener has
                // published nothing that could make a slam try sound.
                for tail in ["X", &four.to_string()] {
                    entries.extend(rows_of(
                        Pattern::after(OVER, &format!("{rung} ({over}) - - {tail} -")),
                        multi_signoff_pass(),
                    ));
                }
                // …and responder sits for **opener's** penalty double, the
                // mirror image.  Responder described a six-card suit and a
                // point floor; opener took the money, and pulling is not on
                // offer.
                entries.extend(rows_of(
                    Pattern::after(OVER, &format!("{rung} ({over}) X -")),
                    multi_signoff_pass(),
                ));
                // …and the seat after opener takes the **completion** rung,
                // which fires at exactly one node in the whole package
                // (`2NT (3♣)` — the only entry that leaves a completion both
                // legal and cheap).  Responder is uncapped there, so without
                // this the top of the band is stranded on the floor at the
                // one node the new rung created: probed over the lia arm the
                // floor passes 8-11 and bids `3NT` on 12-14, which is sound,
                // but it also bids a phantom `4♥` on ~8% of the band, heart
                // voids included, and opener then answers that with `4♠` on
                // every hand.  Their double takes no room, so it answers
                // verbatim; opener sits for the game-forcing `4m`, the same
                // rail as one rung over.
                if completion > over {
                    let taken = format!("{rung} ({over}) {completion}");
                    for tail in ["-", "(X)"] {
                        entries.extend(rows_of(
                            Pattern::after(OVER, &format!("{taken} {tail}")),
                            landy_lia_contested_rebid(minor, completion, None, None),
                        ));
                    }
                    entries.extend(rows_of(
                        Pattern::after(OVER, &format!("{taken} - {four} -")),
                        multi_signoff_pass(),
                    ));
                }
                // Their runout from the penalty double this table authors,
                // and the overcaller bidding again over opener's sit before
                // responder has spoken.
                for again in landy_lia_entries(over) {
                    entries.extend(rows_of(
                        Pattern::after(OVER, &format!("{rung} ({over}) - - X ({again})")),
                        multi_signoff_pass(),
                    ));
                    entries.extend(rows_of(
                        Pattern::after(OVER, &format!("{rung} ({over}) - ({again})")),
                        landy_lia_contested_rebid(minor, again, Some(again), None),
                    ));
                }
            }
            for leg in [completion, relay] {
                let broken = leg == relay;
                let completed = format!("{rung} - {leg} -");
                let retreat = cheapest_above(Strain::from(minor), leg);
                let rebid = if broken {
                    landy_lia_super_rebid(minor, retreat)
                } else {
                    landy_bba_transfer_rebid(minor)
                };
                entries.extend(rows_of(Pattern::after(OVER, &completed), rebid));
                // The **overcaller** re-entering over opener's answer, its
                // partner having already passed: with `O L R A` clockwise,
                // `2♠ - 3♣ (3♥)` indexes `O L R A O` **`L`**.  Responder sits
                // immediately after it and is the seat that knows its
                // strength, so the whole committing table lives here.  Their
                // double takes no room and names no suit, so it rides the
                // same table with the penalty rung dropped —
                // [`multi_escape_overcalled`]'s `Option<Bid>`.
                for over in landy_lia_entries(leg).map(Some).chain([None]) {
                    let call = over.map_or_else(|| "X".to_owned(), |bid| bid.to_string());
                    let answered = format!("{rung} - {leg} ({call})");
                    entries.extend(rows_of(
                        Pattern::after(OVER, &answered),
                        landy_lia_contested_rebid(
                            minor,
                            over.unwrap_or(leg),
                            over,
                            broken.then_some(retreat),
                        ),
                    ));
                    for tail in ["X", &four.to_string()] {
                        entries.extend(rows_of(
                            Pattern::after(OVER, &format!("{answered} {tail} -")),
                            multi_signoff_pass(),
                        ));
                    }
                }
                if broken {
                    // The retreat to three of the minor is the super-accept
                    // leg's finite catch-all, so opener sits for it — and
                    // sits through their double of it too: step 0 priced
                    // `2♠ - 2NT - 3♣ (X)` at **−10.0 IMPs/fired**, the worst
                    // per-fired cell in the club rung outside the four-level
                    // runaways.
                    for tail in ["-", "(X)"] {
                        entries.extend(rows_of(
                            Pattern::after(OVER, &format!("{completed} {retreat} {tail}")),
                            multi_signoff_pass(),
                        ));
                    }
                } else {
                    // …and the seat one round later, where responder passed
                    // opener's completion and the **advancer** balances.
                    // Step 0 found it costs −5,509 (`2♠ - 3♣ - - (3♥)`) and
                    // −2,838 (`… (3♦)`) plain IMPs NV — a third of the club
                    // rung's whole contested deficit.  Opener acts, so it is
                    // the sit table, not the captain's; responder has already
                    // refused to compete once, which also caps it, so
                    // opener's value calls stay live.  Their **balancing
                    // double** rides it under `None`.
                    //
                    // The super-accept leg takes no such registration:
                    // [`landy_lia_super_rebid`] has no `Pass`, so responder
                    // cannot reach this seat there, and a node under a call
                    // no arm makes is a book node shadowing the floor for
                    // nothing.
                    for over in landy_lia_entries(leg).map(Some).chain([None]) {
                        let call = over.map_or_else(|| "X".to_owned(), |bid| bid.to_string());
                        let balanced = format!("{rung} - {leg} - - ({call})");
                        entries.extend(rows_of(
                            Pattern::node(&format!("{OVER} {balanced}")),
                            landy_lia_overcalled(over, None, agreements),
                        ));
                        entries.extend(rows_of(
                            Pattern::after(OVER, &format!("{balanced} - -")),
                            multi_signoff_pass(),
                        ));
                        if over.is_some() {
                            // Opener doubled their balancing bid; responder
                            // sits for it, as it does one round earlier.
                            entries.extend(rows_of(
                                Pattern::after(OVER, &format!("{balanced} X -")),
                                multi_signoff_pass(),
                            ));
                        }
                    }
                    // The re-cue answer rides the completion leg only:
                    // `landy_bba_transfer_rebid` is what carries the `3M`
                    // stopper cues, and the super-accept's rebid has none.
                    for (held, asked) in
                        [(Suit::Hearts, Suit::Spades), (Suit::Spades, Suit::Hearts)]
                    {
                        entries.extend(rows_of(
                            Pattern::after(
                                OVER,
                                &format!("{completed} {} -", Bid::new(3, Strain::from(held))),
                            ),
                            landy_recue_answer(minor, asked),
                        ));
                    }
                }
                // The `4m` slam try re-hung byte-identical on both legs (the
                // rebid tables carry the rung; this is its answer and RKCB
                // ladder).
                if agreements.competition.landy_minor_slam_answer {
                    for tail in ["-", "(X)"] {
                        let path = format!("{OVER} {completed} {four} {tail}");
                        entries.extend(rows_of(Pattern::node(&path), landy_slam_answer(minor)));
                        entries.extend(slam::rkcb_rows(&path, minor));
                    }
                }
            }
            // The direct six-card sign-off (`3♣` weak clubs, `3♦` excessive
            // diamonds) and the one seat around it that stays ours.  Opener
            // sits: responder named its own suit with at most seven points,
            // and the rung exists to end the auction.  Left to the floor this
            // seat answers the N1j gadget the natural `3m` replaced — a
            // phantom `3♦` transfer completion on every probed hand — because
            // the floor has no input slot for the lia regime.
            //
            // Its **contested** tail is deliberately not ours any more.  The
            // 2026-09-01 build sat there too, and the lia2 A/B convicted the
            // sit: see [`landy_lia_overcalled`] for the −22,119 IMPs.
            entries.extend(rows_of(
                Pattern::after(OVER, &format!("{completion} -")),
                landy_signoff_answer(),
            ));
        }
    } else {
        for (minor, transfer) in [(Suit::Clubs, "2NT"), (Suit::Diamonds, "3♣")] {
            let done = Bid::new(3, Strain::from(minor));
            for suffix in [format!("{transfer} -"), format!("{transfer} (X)")] {
                let completion = if minor == Suit::Clubs {
                    complete_lebensohl_relay(agreements)
                } else {
                    landy_bba_diamond_completion(agreements)
                };
                entries.extend(rows_of(Pattern::after(OVER, &suffix), completion));
            }
            entries.push(systems_on_over_double(
                &format!("{OVER} {transfer}"),
                &done.to_string(),
            ));
            let completed = format!("{transfer} - {done} -");
            entries.extend(rows_of(
                Pattern::after(OVER, &completed),
                landy_bba_transfer_rebid(minor),
            ));
            for (held, asked) in [(Suit::Hearts, Suit::Spades), (Suit::Spades, Suit::Hearts)] {
                entries.extend(rows_of(
                    Pattern::after(
                        OVER,
                        &format!("{completed} {} -", Bid::new(3, Strain::from(held))),
                    ),
                    landy_recue_answer(minor, asked),
                ));
            }
            // The `4m` slam try's answer and its RKCB ladder.  The rung itself
            // has shipped since N1; only this seat is new, and without it the
            // seat is the floor's — which cannot keycard in a disturbed
            // auction.  Their double takes no room, so it answers verbatim.
            if agreements.competition.landy_minor_slam_answer {
                let four = Bid::new(4, Strain::from(minor));
                for tail in ["-", "(X)"] {
                    let path = format!("{OVER} {completed} {four} {tail}");
                    entries.extend(rows_of(Pattern::node(&path), landy_slam_answer(minor)));
                    entries.extend(slam::rkcb_rows(&path, minor));
                }
            }
        }
    }

    // The both-minors family: answers, the doubled call verbatim plus the
    // rebase, and the compressed ladder over their raises.  §N1-lia keeps the
    // splinters (re-weighted, not re-ruled) and drops the `2♠` takeout, so
    // `2♥` is the only one; its answer priority reverses and gains the game
    // acceptance the invitational band needs ([`landy_lia_takeout_answer`]).
    let lia = landy_lia(agreements);
    let takeouts: &[(Suit, u8)] = if lia {
        &[(Suit::Hearts, 2), (Suit::Hearts, 3), (Suit::Spades, 3)]
    } else {
        &[
            (Suit::Hearts, 2),
            (Suit::Spades, 2),
            (Suit::Hearts, 3),
            (Suit::Spades, 3),
        ]
    };
    for &(short, level) in takeouts {
        let call = Bid::new(level, Strain::from(short));
        let lia_takeout = lia && level == 2;
        let answer = if lia_takeout {
            landy_lia_takeout_answer(agreements)
        } else {
            landy_bba_takeout_answer(short, call)
        };
        entries.extend(rows_of(
            Pattern::after(OVER, &format!("{call} -")),
            answer.clone(),
        ));
        entries.extend(rows_of(
            Pattern::after(OVER, &format!("{call} (X)")),
            answer,
        ));
        entries.push(systems_on_over_double(
            &format!("{OVER} {call}"),
            &cheapest_above(Strain::Notrump, call).to_string(),
        ));
        // Their raise: `(2♠)` exists over `2♥` only, the 3M raises over
        // whatever sits below them; the four level stays the floor's.  Lia's
        // takeout names no short major, so its compressed ladder keys the
        // stopper on the suit they actually raised instead.
        let mut raises = vec![
            (Bid::new(3, Strain::Hearts), Suit::Hearts),
            (Bid::new(3, Strain::Spades), Suit::Spades),
        ];
        let cheap = Bid::new(2, Strain::Spades);
        if cheap > call {
            raises.push((cheap, Suit::Spades));
        }
        for (over, raised) in raises.into_iter().filter(|(o, _)| *o > call) {
            let stopper = if lia_takeout { raised } else { short };
            entries.extend(rows_of(
                Pattern::after(OVER, &format!("{call} ({over})")),
                landy_bba_takeout_overcalled(stopper, over),
            ));
        }
    }

    // The two-level takeout's authored placements: over the notrump answer
    // (with the one remaining stopper ask) and over a minor pick.  Under
    // §N1-lia there is one takeout and the asked major flips — opener's `2NT`
    // claimed **spades**, so the live cue is hearts — plus the `2♠` ask's own
    // placement seat.
    if lia {
        entries.extend(rows_of(
            Pattern::after(OVER, "2♥ - 2NT -"),
            landy_lia_takeout_rebid(),
        ));
        entries.extend(rows_of(
            Pattern::after(OVER, "2♥ - 2NT - 3♥ -"),
            landy_bba_ask_answer(Suit::Hearts),
        ));
        // The weak band's pull of opener's `2NT` ([`landy_lia_takeout_rebid`]'s
        // `3♣`@60).  Opener sits: it denied a four-card minor to bid `2NT`, so
        // responder's five-plus clubs is the best trump suit either hand can
        // name, and the band caps at seven points.  Unauthored the floor bids
        // `3♦` on it — a suit responder has exactly four of and opener at most
        // three.
        entries.extend(rows_of(
            Pattern::after(OVER, "2♥ - 2NT - 3♣ -"),
            multi_signoff_pass(),
        ));
        entries.extend(rows_of(
            Pattern::after(OVER, "2♥ - 2♠ -"),
            landy_lia_ask_rebid(),
        ));
        entries.extend(rows_of(
            Pattern::after(OVER, "2♥ - 2♠ - 3♥ -"),
            landy_bba_ask_answer(Suit::Hearts),
        ));
        // The ask's landing, and responder's seat over opener's correction.
        // Both are new with the `3♣`@20 catch-all: the first build's `4♣`@20
        // sat above `3NT`, so no node existed between it and the game.
        entries.extend(rows_of(
            Pattern::after(OVER, "2♥ - 2♠ - 3♣ -"),
            landy_lia_ask_landing(),
        ));
        entries.extend(rows_of(
            Pattern::after(OVER, "2♥ - 2♠ - 3♣ - 3♦ -"),
            multi_signoff_pass(),
        ));
        // Opener accepted the invitation.  Responder sits: it has 4+ in both
        // minors and no major length, so the hands that could want more than
        // `3NT` need 33 combined points opposite a maximum that has already
        // bid the game — arithmetic this lane does not supply often enough to
        // author a slam try into.  Reversible by weight, and the seat is
        // named in the campaign doc's residue list.
        entries.extend(rows_of(
            Pattern::after(OVER, "2♥ - 3NT -"),
            multi_signoff_pass(),
        ));
        for minor in [Suit::Clubs, Suit::Diamonds] {
            entries.extend(rows_of(
                Pattern::after(
                    OVER,
                    &format!("2♥ - {} -", Bid::new(3, Strain::from(minor))),
                ),
                landy_lia_pick_rebid(minor),
            ));
        }
    } else {
        for (short, other) in [(Suit::Hearts, Suit::Spades), (Suit::Spades, Suit::Hearts)] {
            let tko = Bid::new(2, Strain::from(short));
            entries.extend(rows_of(
                Pattern::after(OVER, &format!("{tko} - 2NT -")),
                landy_bba_takeout_rebid(other),
            ));
            entries.extend(rows_of(
                Pattern::after(
                    OVER,
                    &format!("{tko} - 2NT - {} -", Bid::new(3, Strain::from(other))),
                ),
                landy_bba_ask_answer(other),
            ));
            for minor in [Suit::Clubs, Suit::Diamonds] {
                entries.extend(rows_of(
                    Pattern::after(
                        OVER,
                        &format!("{tko} - {} -", Bid::new(3, Strain::from(minor))),
                    ),
                    landy_bba_pick_rebid(minor),
                ));
            }
        }
    }

    entries
}

/// Opener completes responder's Lebensohl `2NT` relay with the forced `3♣`
///
/// Under `reading.completion_alerts` the completion is alerted: constrained
/// `hcp(0..)` it projects nothing, so the alert's whole effect is to suppress
/// the natural walk's club reading of a forced puppet.
pub(crate) fn complete_lebensohl_relay(agreements: &Agreements) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Clubs), 100, hcp(0..))
        .alert_if(
            agreements.decision.reading.completion_alerts,
            LEBENSOHL_COMPLETION,
        )
}

/// Responder's rebid after the `2NT` relay is completed at `3♣`
///
/// Pass to play clubs, or correct to the six-card suit (still a weak sign-off).
pub(crate) fn lebensohl_relay_rebid(over: Suit, agreements: &Agreements) -> Rules {
    let mut rules = Rules::new();
    for s in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if s == over {
            continue;
        }
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(3, strain),
            100,
            min_level_is(3, strain) & len(s, 5..),
        );
    }
    // Stopper-split on: the *delayed* cue of their suit — Stayman with a stopper,
    // game-forcing, exactly a 4-card unbid major (denies 5). Answered by
    // [`cue_stayman_answer`] (the stopper is guaranteed, so 3NT is safe).
    if let (true, Some(major)) = (agreements.competition.delayed_cue, unbid_major(over)) {
        rules = rules
            .rule(
                Bid::new(3, Strain::from(over)),
                150,
                points(10..) & stopper_in(over) & len(major, 4..) & len(major, ..5),
            )
            .alert(LEBENSOHL_CUE);
    }
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Opener's reply to responder's weak Lebensohl sign-off in a major
///
/// Responder's sign-off is a known weak hand with a 5+ suit, floored at
/// `resp_floor` points (the relay's PD-distilled 6, or the direct natural
/// escape's lower 5 — see [`lebensohl_relay_shape`] and `agreements.competition.natural_floor`).
/// A *maximum* 1NT opener with a fit stretches to game: the combined floor is
/// then high enough to reach the 4M zone with a long-trump dummy.
///
/// The gauge is points *plus* trump length, not points alone — a
/// Law-of-Total-Tricks dummy adjustment that trades one point per extra trump.
/// The combined target is 23 (a 17-max opposite the relay's 6-floor with an
/// 8-card fit); a lower responder floor raises opener's bar by the same amount,
/// and each trump beyond three lowers it by one.  Anything short passes the
/// sign-off.  Only majors — a minor sign-off's game is the 5 level, out of
/// reach for a weak hand.
pub(super) fn lebensohl_signoff_raise(signoff: Suit, resp_floor: u8) -> Rules {
    let game = Bid::new(4, Strain::from(signoff));
    let base = 23u8.saturating_sub(resp_floor); // opener points with bare 3-card support
    Rules::new()
        .rule(
            game,
            100,
            (len(signoff, 3..=3) & points(base..))
                | (len(signoff, 4..=4) & points(base.saturating_sub(1)..))
                | (len(signoff, 5..) & points(base.saturating_sub(2)..)),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's answer when they compete over responder's weak two-level escape
/// (`1NT (2♦ Multi) 2♥/2♠ (X | 2♠ | 2NT | 3x)`)
///
/// Wired only under [`CompetitionKnobs::multi_weak_escape`], whose whole point
/// is to route hands with **no HCP floor** into this escape — so the iron rule
/// ("complete the convention: both sides' continuations *and* the interfered
/// tails") makes it part of that package rather than a separate fix.  Left to
/// the floor, this seat bid *their* suit at the four level over partner's
/// escape (−1100 doubled) in the census dumps.
///
/// [`landy_cue_overcalled`]'s doctrine one level lower and opposite a far
/// weaker partner: Pass is the default and is *safe* — responder has shown a
/// long suit and at most eight, so no game is being stranded.  Above it sit
/// only the two calls Pass cannot make, the competitive raise on a fit with a
/// maximum and the values double when there is no raise to make.
///
/// `over` is their call; `None` is their double, which takes no room and gets a
/// pure sit.  Running a known 5-3 fit out of a doubled two-level partial is the
/// disaster the escape was authored to avoid, not a rescue.
fn multi_escape_overcalled(major: Suit, over: Option<Bid>, agreements: &Agreements) -> Rules {
    let mut rules = Rules::new();
    if let Some(over) = over {
        let max = agreements.notrump.size_ask_accept_floor;
        let raise = cheapest_above(Strain::from(major), over);
        if raise <= Bid::new(3, Strain::from(major)) {
            rules = rules.rule(raise, 100, len(major, 3..) & hcp(max..));
        }
        rules = rules.rule(Call::Double, 90, hcp(max..)).penalty();
    }
    rules.rule(Call::Pass, 0, hcp(0..))
}

/// The Kokish–Kraft registration — responder's table, the double family, the
/// neutral-pass family, the floorless minor transfers, `3♠` both minors, the
/// major transfers, Leaping Michaels, the direct `4M` slam try, and the weak
/// escape
///
/// Registered *instead of* the shipped N4 subtree ([`lebensohl_package`]
/// branches on [`kokish_kraft`]), for [`landy_bba_entries`]'s reason: the two
/// tables disagree on `2NT`, `3♣`, `3♠` and both delayed doubles, so an overlay
/// would leave v7's rows shadowing these.  What it *reuses* rather than forks —
/// [`multi_pass_answer`], [`multi_penalty_answer`], [`multi_takeout_answer`],
/// [`multi_quant_answer`], [`multi_signoff_pass`], [`transfer_completion`], the
/// `lm_2d_*` advances, the notrump book's [`slam_try_answer`]/`rkcb_rows`, and
/// the whole weak-escape family — is the half of the lane K–K does not move.
///
/// [`CompetitionKnobs::multi_balance`] composes (a different seat, opener
/// balancing over responder's pass); [`CompetitionKnobs::multi_stopper_ask`] is
/// inert, its `3♠` being the both-minors call here.
fn kokish_kraft_entries(agreements: &Agreements) -> Vec<Entry> {
    const NT: &str = "P* 1NT";
    const OVER: &str = "P* 1NT (2♦)";
    let sit = multi_signoff_pass();
    let mut entries = rows_of(
        Pattern::after(NT, "(2♦)"),
        kokish_kraft_responder(agreements),
    );

    // ---- the values double: opener's two answers, then the resolved round.
    entries.extend(rows_of(Pattern::after(OVER, "X -"), multi_pass_answer()));
    for major in [Suit::Hearts, Suit::Spades] {
        entries.extend(rows_of(
            Pattern::after(OVER, &format!("X (2{})", Strain::from(major))),
            multi_penalty_answer(major),
        ));
    }
    // Which paths carry `competition.multi_doubler_major`'s natural rung.  The
    // deciding fact is what opener's own action said about the *other* major,
    // and `multi_penalty_answer` makes that a hard fact: it doubles their
    // `(2M)` on `len(M, 4..)` at weight 150 against a weight-0 catch-all, so a
    // pass over `(2♥)` **denies** four hearts and a double of it **shows**
    // four.
    //
    // - `X (2♥) - -` — opener said nothing about spades, and `2♠` is the cheap
    //   rung.  This is the census's biggest single hole.
    // - `X (2♥) X (2♠)` — opener showed four-plus hearts, so `3♥` is a *known*
    //   4-4 at the three level with 23+ combined.  The strongest of the three.
    // - `X (2♥) - (2♠)` — opener's pass denied four hearts, so `3♥` could only
    //   ever find a 4-3.  Excluded.
    // - `X (2♠) - -` — opener said nothing about hearts, so `3♥` is a four-card
    //   suit at the three level opposite unknown support, firing only when the
    //   spade stopper is missing, i.e. on the misfits.  §N4-KK residue 4 named
    //   this leg (at `points(10..)`) and the review argued it out again; it is
    //   **withheld pending a ruling**, on 25 boards NV+both worth −30 IMPs
    //   plain and +8 PD — no measured loss to repair.  Flip the `dm` column to
    //   re-arm it.
    //
    // The `px` column is [`CompetitionKnobs::multi_px_split`]'s answer to the
    // same question, and it differs on exactly that leg.  Under the split the
    // 8–9 doubler is *guaranteed* a four-card major, so a hearts-only hand at
    // `X (2♠) - -` is no longer the misfit tail — it is the population that
    // used to sell out, and the withholding ruling was priced under the wider
    // `hcp 8+` double.  `X (2♥) - (2♠)` stays excluded on both columns: the
    // mechanism there is opener's *pass* denying four hearts, which no
    // responder-side split can change.
    for (path, major, ran, dm_leg, px_leg) in [
        ("X (2♥) - -", Suit::Hearts, false, true, true),
        ("X (2♥) - (2♠)", Suit::Spades, true, false, false),
        ("X (2♠) - -", Suit::Spades, false, false, true),
        ("X (2♥) X (2♠)", Suit::Spades, true, true, true),
    ] {
        // One rung when both knobs are on; `px_split` owns the weight.
        let natural_other = (agreements.competition.multi_doubler_major && dm_leg)
            || (agreements.competition.multi_px_split && px_leg);
        entries.extend(rows_of(
            Pattern::after(OVER, path),
            kokish_kraft_doubler_rebid(
                major,
                ran,
                natural_other,
                agreements.competition.multi_px_split,
            ),
        ));
        if natural_other {
            let other = if major == Suit::Hearts {
                Suit::Spades
            } else {
                Suit::Hearts
            };
            let bid = Bid::new(
                if other == Suit::Spades { 2 } else { 3 },
                Strain::from(other),
            );
            let notrump_out = agreements.competition.multi_px_split
                || agreements.competition.multi_doubler_notrump;
            // The minimum's `2NT` is a rung *below* the notrump out, so it is
            // incoherent without it: 15 would bid and 16 would pass.
            let minimum_notrump =
                notrump_out && agreements.competition.multi_doubler_minimum_notrump;
            entries.extend(rows_of(
                Pattern::after(OVER, &format!("{path} {bid} -")),
                kokish_kraft_doubler_major_answer(major, notrump_out, minimum_notrump),
            ));
            if other == Suit::Spades {
                entries.extend(rows_of(
                    Pattern::after(OVER, &format!("{path} {bid} - 3♠ -")),
                    kokish_kraft_doubler_major_invite(major),
                ));
                if minimum_notrump {
                    entries.extend(rows_of(
                        Pattern::after(OVER, &format!("{path} {bid} - 2NT -")),
                        kokish_kraft_minimum_notrump_answer(),
                    ));
                }
            }
        }
        // The repeated double is penalty at *every* one of these paths (the
        // K–K split), so opener sits on it rather than answering a takeout.
        entries.extend(rows_of(
            Pattern::after(OVER, &format!("{path} X -")),
            sit.clone(),
        ));
        entries.extend(rows_of(
            Pattern::after(OVER, &format!("{path} 2NT -")),
            kokish_kraft_invite_answer(),
        ));
        entries.extend(rows_of(
            Pattern::after(OVER, &format!("{path} 4NT -")),
            multi_quant_answer(),
        ));
    }
    // Responder, opener having doubled and the advancer sat: sit.  And their
    // `2NT` over the doubled/undoubled `2♠` is the overcaller's heart relay
    // (bba-1nt-defense.md) — nothing to say until they place it.
    for major in [Suit::Hearts, Suit::Spades] {
        entries.extend(rows_of(
            Pattern::after(OVER, &format!("X (2{}) X -", Strain::from(major))),
            sit.clone(),
        ));
    }
    for path in ["X (2♠) X (2NT)", "X (2♠) - (2NT)"] {
        entries.extend(rows_of(Pattern::after(OVER, path), sit.clone()));
    }

    // ---- the neutral pass: responder's delayed table and its takeout double.
    for (path, major) in [
        ("- (2♥) - -", Suit::Hearts),
        ("- (2♠) - -", Suit::Spades),
        ("- (2♥) - (2♠)", Suit::Spades),
    ] {
        entries.extend(rows_of(
            Pattern::after(OVER, path),
            kokish_kraft_delayed(major),
        ));
        entries.extend(rows_of(
            Pattern::after(OVER, &format!("{path} X -")),
            multi_takeout_answer(major),
        ));
        // The delayed `2NT` is natural and **competitive**, not invitational —
        // and responder's first-turn pass has already denied `hcp 8+`, so the
        // published reading is `hcp 7` and 3NT would be a 24-point punt.
        // Opener sits.  (The `3♣`/`3♦` rungs beside it are dead in self-play —
        // see [`kokish_kraft_delayed`] — so their answers stay the floor's.)
        //
        // [`CompetitionKnobs::multi_px_split`] is what makes it an invitation:
        // the split's first-turn pass denies 8–9 only *with* a four-card major,
        // so the 8–9-no-major band arrives here and opener accepts from the top
        // of the range.  The band still reaches down to `hcp 7`, so accepting
        // on 16 can reach `3NT` on 23 combined — a known cost of the swap, and
        // one of the two cells the split's forensic watches.
        entries.extend(rows_of(
            Pattern::after(OVER, &format!("{path} 2NT -")),
            if agreements.competition.multi_px_split {
                kokish_kraft_invite_answer()
            } else {
                sit.clone()
            },
        ));
    }

    // ---- the floorless minor transfers.
    for (minor, transfer) in [(Suit::Clubs, "2NT"), (Suit::Diamonds, "3♣")] {
        let done = Bid::new(3, Strain::from(minor));
        // Their double takes no room: opener completes anyway, and every
        // deeper `X`-then-bid suffix rebases onto the clean subtree.
        for suffix in [format!("{transfer} -"), format!("{transfer} (X)")] {
            entries.extend(rows_of(
                Pattern::after(OVER, &suffix),
                kokish_kraft_minor_completion(minor, agreements),
            ));
        }
        entries.push(systems_on_over_double(
            &format!("{OVER} {transfer}"),
            &done.to_string(),
        ));
        let completed = format!("{transfer} - {done}");
        for suffix in [format!("{completed} -"), format!("{completed} (X)")] {
            entries.extend(rows_of(
                Pattern::after(OVER, &suffix),
                kokish_kraft_transfer_rebid(minor, agreements),
            ));
        }
        for (second, step) in kokish_kraft_second_suits(minor) {
            entries.extend(rows_of(
                Pattern::after(OVER, &format!("{completed} - {} -", Bid::new(3, *step))),
                kokish_kraft_two_suiter_answer(*second),
            ));
        }
        // The `4m` slam try and its RKCB ladder.  Opener's answer is authored
        // rather than left to the floor — see `kokish_kraft_slam_answer` for
        // the probe that killed the doctrine at this seat.  Their double takes
        // no room, so it answers verbatim.
        let four = Bid::new(4, Strain::from(minor));
        if agreements.competition.multi_minor_slam_try.is_some() {
            for tail in ["-", "(X)"] {
                let path = format!("{OVER} {completed} - {four} {tail}");
                entries.extend(rows_of(
                    Pattern::node(&path),
                    kokish_kraft_slam_answer(minor),
                ));
                entries.extend(slam::rkcb_rows(&path, minor));
            }
        }
        // Their pass-or-correct above the completion: opener sits (the
        // transfer promised no values), and responder's — if it has any — act
        // again.  Deeper than that is the floor's.
        entries.extend(rows_of(
            Pattern::up_to(&format!("{OVER} {transfer}"), "7♠"),
            sit.clone(),
        ));
        // The same pair one round later, when they let the completion through
        // and compete over *it* instead.  The shipped relay authors this seat
        // (`multi_signoff_pass` over every sign-off, over their double of it,
        // and over their balance); the `continue` above skips that block, so
        // K–K re-registers its own — and responder's half is not a sit,
        // because a weak *and* a game-forcing hand both live behind the
        // floorless transfer and the game-forcing one has not spoken yet.
        for major in [Suit::Hearts, Suit::Spades] {
            let their = format!("(3{})", Strain::from(major));
            for suffix in [
                format!("{transfer} {their} - -"),
                format!("{completed} {their}"),
            ] {
                entries.extend(rows_of(
                    Pattern::after(OVER, &suffix),
                    kokish_kraft_transfer_overcalled(minor, major, agreements),
                ));
                // Opener has already passed once, so responder's values double
                // there stands as penalty: sit on it rather than let the floor
                // pull a partscore we chose to defend.
                entries.extend(rows_of(
                    Pattern::after(OVER, &format!("{suffix} X -")),
                    sit.clone(),
                ));
                // And the same for the contested `4m`, which is a *placement*
                // on shortness in their major, not a try: probed, the floor
                // answers it `4♥` — a contract in the suit they just named.
                if agreements.competition.multi_minor_slam_try.is_some() {
                    entries.extend(rows_of(
                        Pattern::after(OVER, &format!("{suffix} {four} -")),
                        sit.clone(),
                    ));
                }
            }
        }
        // And their *balance* after both of us have passed it out — their bid,
        // and their **double**, which `up_to` cannot see (it admits bids only)
        // and which is the shape the floor is on record pulling.
        entries.extend(rows_of(
            Pattern::up_to(&format!("{OVER} {completed} - -"), "7♠"),
            sit.clone(),
        ));
        entries.extend(rows_of(
            Pattern::after(OVER, &format!("{completed} - - (X)")),
            sit.clone(),
        ));
    }

    // ---- the major transfers keep the shipped INV+ completions.
    for (bid, target) in [("3♦", Suit::Hearts), ("3♥", Suit::Spades)] {
        entries.extend(rows_of(
            Pattern::after(OVER, &format!("{bid} -")),
            transfer_completion(target, Suit::Diamonds, agreements),
        ));
    }

    // ---- `3♠` both minors, game-forcing.
    for suffix in ["3♠ -", "3♠ (X)"] {
        entries.extend(rows_of(
            Pattern::after(OVER, suffix),
            kokish_kraft_minors_answer(),
        ));
    }
    entries.extend(rows_of(
        Pattern::up_to(&format!("{OVER} 3♠"), "7♠"),
        kokish_kraft_minors_overcalled(),
    ));
    for minor in [Suit::Clubs, Suit::Diamonds] {
        entries.extend(rows_of(
            Pattern::after(OVER, &format!("3♠ - 4{} -", Strain::from(minor))),
            kokish_kraft_minors_place(minor),
        ));
    }

    // ---- Leaping Michaels, unchanged.
    for (suffix, rules) in [
        ("4♦ -", lm_2d_both_majors_advance()),
        ("4♣ -", lm_2d_clubs_ask()),
        ("4♣ - 4♦ -", lm_2d_clubs_major()),
    ] {
        entries.extend(rows_of(Pattern::after(OVER, suffix), rules));
    }

    // ---- the direct `4M` slam try: the uncontested tier and its RKCB ladder.
    for major in [Suit::Hearts, Suit::Spades] {
        let prefix = format!("{OVER} {} -", Bid::new(4, Strain::from(major)));
        entries.extend(rows_of(Pattern::node(&prefix), slam_try_answer()));
        entries.extend(slam::rkcb_rows(&prefix, major));
    }

    // ---- the weak natural escape, verbatim from the shipped lane: opener's
    // sign-off raise fed the reading's own floor, plus the interfered tail.
    let weak_escape = agreements.competition.multi_weak_escape;
    if natural_floor_on(agreements) || weak_escape.is_some() {
        let resp_floor = if weak_escape.is_some() {
            0
        } else {
            natural_floor_hcp(agreements)
        };
        for signoff in [Suit::Hearts, Suit::Spades] {
            let escape = Bid::new(2, Strain::from(signoff));
            entries.extend(rows_of(
                Pattern::after(OVER, &format!("{escape} -")),
                lebensohl_signoff_raise(signoff, resp_floor),
            ));
            if weak_escape.is_none() {
                continue;
            }
            let overs = [
                Bid::new(2, Strain::Spades),
                Bid::new(2, Strain::Notrump),
                Bid::new(3, Strain::Clubs),
                Bid::new(3, Strain::Diamonds),
                Bid::new(3, Strain::Hearts),
                Bid::new(3, Strain::Spades),
            ];
            let theirs =
                std::iter::once(None).chain(overs.into_iter().filter(|b| *b > escape).map(Some));
            for bid in theirs {
                let call = bid.map_or_else(|| "X".to_string(), |b| b.to_string());
                entries.extend(rows_of(
                    Pattern::after(OVER, &format!("{escape} ({call})")),
                    multi_escape_overcalled(signoff, bid, agreements),
                ));
            }
        }
    }

    // ---- opener's balancing double over the neutral pass, when armed.
    if agreements.competition.multi_balance {
        for major in [Suit::Hearts, Suit::Spades] {
            let advance = format!("- (2{})", Strain::from(major));
            entries.extend(rows_of(
                Pattern::after(OVER, &advance),
                multi_balance_double(major),
            ));
            entries.extend(rows_of(
                Pattern::after(OVER, &format!("{advance} X -")),
                sit.clone(),
            ));
            entries.extend(rows_of(
                Pattern::up_to(&format!("{OVER} {advance} X"), "7♠"),
                sit.clone(),
            ));
        }
    }

    entries
}

/// Sections 5 / 5b / 5c as a row package: Lebensohl after our `1NT` is
/// overcalled at the 2 level (`agreements.competition.lebensohl_style`)
///
/// Purely additive — nothing else lands at `1NT` in the competitive book.
/// Plain or Transfer Lebensohl per [`LebensohlStyle`]; both keep the weak `2NT`
/// relay.  Over a natural `(2♣)` we play *systems on* instead (a rebase onto the
/// uncontested tree), so Lebensohl proper is wired only over the overcalls that
/// actually steal room.
pub(super) fn lebensohl_package() -> Package {
    Package {
        name: "lebensohl",
        gate: |agreements| agreements.competition.lebensohl_style != LebensohlStyle::Off,
        entries: |agreements| {
            const NT: &str = "P* 1NT";
            let style = agreements.competition.lebensohl_style;

            let mut entries = Vec::new();

            if defense_2c_landy(agreements) && landy_bba(agreements) {
                // N1j: the whole table swaps — see `landy_bba_entries`.
                entries.extend(landy_bba_entries(agreements));
            } else if defense_2c_landy(agreements) {
                // Landy counter: their 2♣ shows both majors, so systems-on is
                // exactly the wrong structure — see `landy_responder`.  This is
                // an *either/or* with the rebase below, not an overlay: leaving
                // the rebase registered would strip the 2♣ and remap our values
                // X back onto the stolen 2♣ Stayman one round later, sending
                // `1NT (2♣) X (2♥)` into the contested-Stayman package.  So the
                // book claims responder's first call and opener's one answer to
                // each of them, and every deeper auction is the floor's.  That
                // is not the same as being covered: `penalize_escape_stack` /
                // `penalize_escape_values` gate on
                // `instinct::our_doubled_one_nt_escape`, which requires
                // `auction[opening + 1] == Call::Double` — in `1NT (2♣) X (2♥)`
                // that slot holds their `2♣`, so the penalty chase cannot fire
                // here.  The seat after their run is unauthored and the floor
                // plays its ordinary takeout ladder there.
                entries.extend(rows_of(
                    Pattern::after(NT, "(2♣)"),
                    landy_responder(agreements),
                ));
                entries.extend(rows_of(
                    Pattern::after("P* 1NT (2♣)", "X -"),
                    landy_double_answer(),
                ));
                // `landy_natural_answers`: opener's one answer over each of the
                // counter's natural calls.  The floor cannot see the counter's
                // regime (no net input slot for the knob), so left to itself it
                // completes each call as the default-system gadget it replaced
                // — phantom Jacoby over `2♦`, phantom minor transfer over
                // `2NT`, phantom Puppet answer over `3♣`.  A direct `3NT`
                // needs no node: the audited dumps pass it out cleanly.
                entries.extend(rows_of(
                    Pattern::after("P* 1NT (2♣)", "2♦ -"),
                    landy_signoff_answer(),
                ));
                if landy_transfer(agreements) {
                    // N1c: `2NT` is the weak club transfer, so opener completes
                    // at `3♣` and responder — who has nothing more to say —
                    // passes.  Reusing the Lebensohl relay's completion keeps
                    // one forced-`3♣` table in the file.
                    entries.extend(rows_of(
                        Pattern::after("P* 1NT (2♣)", "2NT -"),
                        complete_lebensohl_relay(agreements),
                    ));
                    entries.extend(rows_of(
                        Pattern::after("P* 1NT (2♣)", "2NT - 3♣ -"),
                        landy_signoff_answer(),
                    ));
                    if landy_competition(agreements) {
                        // N1f: their X of the transfer takes no room — opener
                        // completes anyway, and responder still passes the
                        // completion.
                        entries.extend(rows_of(
                            Pattern::after("P* 1NT (2♣)", "2NT (X)"),
                            complete_lebensohl_relay(agreements),
                        ));
                        entries.extend(rows_of(
                            Pattern::after("P* 1NT (2♣)", "2NT (X) 3♣ -"),
                            landy_signoff_answer(),
                        ));
                    }
                } else {
                    entries.extend(rows_of(
                        Pattern::after("P* 1NT (2♣)", "2NT -"),
                        landy_invite_answer(agreements),
                    ));
                }
                for minor in [Suit::Clubs, Suit::Diamonds] {
                    entries.extend(rows_of(
                        Pattern::after("P* 1NT (2♣)", &format!("3{} -", Strain::from(minor))),
                        // The direct 3m means something different in all three
                        // arms (see `landy_responder`): forcing in the base
                        // counter, a weak escape opener sits for under N1b, and
                        // invitational under N1c.
                        if landy_transfer(agreements) {
                            landy_minor_invite_answer(agreements)
                        } else if agreements.competition.defense_2c_landy_cues {
                            landy_signoff_answer()
                        } else {
                            landy_minor_answer(minor)
                        },
                    ));
                }
                if landy_cues(agreements) {
                    // The INV+ cue's whole accept/decline tree, authored down
                    // to the placing call.  The floor cannot carry any of it:
                    // `Inferences` has no forcing channel, so every rung left
                    // to it reads as bare length-and-points — the defect that
                    // cost −1.8 IMPs/fired when opener's answer alone was
                    // sub-game.
                    for (minor, cue) in [
                        (Suit::Clubs, Bid::new(2, Strain::Hearts)),
                        (Suit::Diamonds, Bid::new(2, Strain::Spades)),
                    ] {
                        entries.extend(rows_of(
                            Pattern::after("P* 1NT (2♣)", &format!("{cue} -")),
                            landy_cue_answer(minor, cue, agreements),
                        ));
                        if landy_competition(agreements) {
                            // N1f: their X of the cue takes no room, so
                            // opener's immediate answer is the clean ladder
                            // verbatim — and every deeper X-then-bid suffix is
                            // stripped back onto the clean subtree by the
                            // systems-on rebase (the contested-Stayman idiom),
                            // so the asks, rebids and re-cues all answer as if
                            // undoubled.
                            entries.extend(rows_of(
                                Pattern::after("P* 1NT (2♣)", &format!("{cue} (X)")),
                                landy_cue_answer(minor, cue, agreements),
                            ));
                            entries.push(systems_on_over_double(
                                &format!("P* 1NT (2♣) {cue}"),
                                &cheapest_above(Strain::Notrump, cue).to_string(),
                            ));
                            // Their raise over the cue — `(2♠)` exists over
                            // the club cue only; `(4♠)` and higher stay the
                            // floor's, deliberately.
                            let mut overs =
                                vec![Bid::new(3, Strain::Hearts), Bid::new(3, Strain::Spades)];
                            let cheap_raise = Bid::new(2, Strain::Spades);
                            if cheap_raise > cue {
                                overs.push(cheap_raise);
                            }
                            for over in overs {
                                entries.extend(rows_of(
                                    Pattern::after("P* 1NT (2♣)", &format!("{cue} ({over})")),
                                    landy_cue_overcalled(minor, over, agreements),
                                ));
                            }
                        }
                        // Opener's minimums that are not asks: 2NT has both
                        // stoppers, 3m has none — the only rung that leaves
                        // responder's worry open, so the re-cue lives there.
                        entries.extend(rows_of(
                            Pattern::after("P* 1NT (2♣)", &format!("{cue} - 2NT -")),
                            landy_minimum_notrump_rebid(minor, agreements),
                        ));
                        let min_minor = Bid::new(3, Strain::from(minor));
                        entries.extend(rows_of(
                            Pattern::after("P* 1NT (2♣)", &format!("{cue} - {min_minor} -")),
                            landy_minimum_minor_rebid(minor, agreements),
                        ));
                        for asked in [Suit::Hearts, Suit::Spades] {
                            entries.extend(rows_of(
                                Pattern::after(
                                    "P* 1NT (2♣)",
                                    &format!(
                                        "{cue} - {min_minor} - {} -",
                                        Bid::new(3, Strain::from(asked))
                                    ),
                                ),
                                landy_recue_answer(minor, asked),
                            ));
                        }

                        // Opener's asks: the 3-level cue of the major opener
                        // lacks (a maximum), plus the cheap `2♠` a minimum can
                        // afford over the club cue only.
                        let mut asks = vec![
                            (Suit::Hearts, Bid::new(3, Strain::Hearts)),
                            (Suit::Spades, Bid::new(3, Strain::Spades)),
                        ];
                        let cheap = Bid::new(2, Strain::Spades);
                        if cheap > cue {
                            asks.push((Suit::Spades, cheap));
                        }
                        for (asked, ask) in asks {
                            let after = format!("{cue} - {ask} -");
                            entries.extend(rows_of(
                                Pattern::after("P* 1NT (2♣)", &after),
                                landy_ask_answer(minor, asked, ask),
                            ));
                            if landy_competition(agreements) {
                                // N1f: the doubled ask — the nine-board
                                // `3♥x`-passed-out defect.  Their X takes no
                                // room: responder answers the ask verbatim,
                                // and deeper X-then-bid suffixes rebase onto
                                // the clean subtree.
                                entries.extend(rows_of(
                                    Pattern::after("P* 1NT (2♣)", &format!("{cue} - {ask} (X)")),
                                    landy_ask_answer(minor, asked, ask),
                                ));
                                entries.push(systems_on_over_double(
                                    &format!("P* 1NT (2♣) {cue} - {ask}"),
                                    &cheapest_above(Strain::Notrump, ask).to_string(),
                                ));
                            }
                            // Responder's cheap notrump is a minimum with the
                            // stopper, so opener still judges game — the same
                            // question the natural 2NT invite asks, hence the
                            // same table.
                            let notrump = cheapest_above(Strain::Notrump, ask);
                            if notrump < Bid::new(3, Strain::Notrump) {
                                entries.extend(rows_of(
                                    Pattern::after("P* 1NT (2♣)", &format!("{after} {notrump} -")),
                                    landy_invite_answer(agreements),
                                ));
                            }
                            // Responder's retreat to the minor is a sign-off
                            // opener's ask already promised tolerance for.
                            let retreat = cheapest_above(Strain::from(minor), ask);
                            entries.extend(rows_of(
                                Pattern::after("P* 1NT (2♣)", &format!("{after} {retreat} -")),
                                landy_signoff_answer(),
                            ));
                        }
                    }
                }
            } else {
                // Over a natural (2♣) overcall we play *systems on*, not
                // Lebensohl: 2♣ steals no room (every transfer/relay still sits
                // above it), so responder keeps the uncontested 1NT structure
                // (Jacoby transfers, minor transfers, the 2NT invite, …) and
                // shows the now-unbiddable 2♣ Stayman with a Double.  Rather
                // than re-author all of that, rebase onto the uncontested tree:
                // the (2♣) overcall maps to the opponent's pass, and a Double
                // directly over it maps to the 2♣ Stayman it replaces.  (So
                // there is no natural 2♦/2♥/2♠ escape over 2♣ — those are
                // transfers.)
                let two_clubs = call(2, Strain::Clubs);
                entries.push(rebase(
                    Pattern::first(NT, "2♣"),
                    described_rewrite(
                        "systems on: their 2♣ is treated as a pass; X asks as the stolen 2♣ Stayman",
                        rewriter(move |auction: &[Call], depth: usize| {
                            if auction.get(depth) != Some(&two_clubs) {
                                return None;
                            }
                            let mut rewritten = auction.to_vec();
                            rewritten[depth] = Call::Pass; // (2♣) steals no room → systems on
                            if auction.get(depth + 1) == Some(&Call::Double) {
                                rewritten[depth + 1] = two_clubs; // stolen 2♣ Stayman = Double
                            }
                            Some(rewritten)
                        }),
                    ),
                ));

                // The rebase routes every *continuation*, but responder must be
                // handed a finite logit on Double to *choose* the stolen Stayman
                // (the rebase only offers the uncontested calls, where 2♣ is
                // illegal here).  So classify responder's own call with the
                // uncontested responses, moving the 2♣ Stayman logit onto
                // Double: X *is* the stolen 2♣ — same weight, same constraint,
                // nothing to drift if Stayman is retuned.  The empty-suffix
                // table claims only responder's first call; deeper calls fall
                // through to the rebase.
                let responses = notrump_responses(agreements);
                entries.push(classified(
                    Pattern::table("P* 1NT (2♣)"),
                    classifier(move |hand: Hand, context: &Context<'_>| {
                        let mut logits = responses.classify(hand, context);
                        let stayman = *logits.0.get(two_clubs);
                        *logits.0.get_mut(two_clubs) = f32::NEG_INFINITY; // 2♣ is stolen
                        *logits.0.get_mut(Call::Double) = stayman; // X inherits 2♣ exactly
                        logits
                    }),
                ));

                // Opener's penalty-pass of that Double: after `1NT (2♣) X -`
                // opener with good clubs sits to defend 2♣ doubled instead of
                // answering the stolen Stayman.  Authored at the same `1NT (2♣)`
                // node as the responder classifier (depth 2), so `resolve_at`
                // reaches it *before* the depth-1 systems-on rebase; the
                // disjoint suffix guard (`X -` vs the responder's empty suffix)
                // keeps the two from colliding.  `stayman_answers()` rides along
                // as the always-mass catch-all, so a hand failing the club gate
                // just answers Stayman exactly as the rebase would (no silent
                // pass).
                if let Some((min_len, min_hcp, over_major)) = agreements.competition.penalty_pass {
                    let pass_logit = if over_major { 150 } else { 75 };
                    entries.extend(rows_of(
                        Pattern::after("P* 1NT (2♣)", "X -"),
                        stayman_answers(agreements).rule(
                            Call::Pass,
                            pass_logit,
                            len(Suit::Clubs, min_len..) & suit_hcp(Suit::Clubs, min_hcp..),
                        ),
                    ));
                }
            }

            // Lebensohl proper applies only over (2♦/2♥/2♠) — the overcalls that
            // actually steal room.  (2♣) is the systems-on rebase above.
            for over in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let o = Strain::from(over);
                let their = format!("(2{o})");

                if over == Suit::Diamonds && kokish_kraft(agreements) {
                    // N4-KK: the whole table swaps — see `kokish_kraft_entries`.
                    entries.extend(kokish_kraft_entries(agreements));
                    continue;
                }

                // Responder's first action: the uncovered suffix is exactly
                // their overcall.
                entries.extend(rows_of(
                    Pattern::after(NT, &their),
                    match style {
                        // N4: their 2♦ is a Multi — the same constructive leg
                        // with every diamond-keyed gate re-keyed.  Either/or
                        // with the natural leg, never an overlay: the deleted
                        // first build gated the responder table and left the
                        // continuations natural, so opener answered a natural
                        // 3♦ with a transfer completion.
                        _ if over == Suit::Diamonds && defense_2d_multi(agreements) => {
                            multi_2d_responder(true, agreements)
                        }
                        LebensohlStyle::Transfer if over == Suit::Diamonds => {
                            // gate_4333 = true: our 1NT overcalled, partner is balanced.
                            transfer_stayman_2d_responder(true, agreements)
                        }
                        LebensohlStyle::Transfer => {
                            transfer_lebensohl_responder(over, true, agreements)
                        }
                        _ => lebensohl_responder(over, agreements),
                    },
                ));

                // Opener's reply to responder's double of the overcall.  The
                // penalty styles SIT (else the floor reads it as a takeout
                // advance and pulls — the documented leak); the optional style
                // cooperates (stand on a fit, run with a doubleton); takeout
                // keeps the floor's advance.  Gated on the leave-in knob.
                let multi = over == Suit::Diamonds && defense_2d_multi(agreements);
                if multi {
                    // N4: responder's double was values, waiting for them to
                    // name the major.  Over their pass (a seat BBA's advancer
                    // never gives — 0.0% at `advance-x`) opener shows a
                    // four-card major, else sits (v6: BBA's answer with its
                    // `3♦` cue replaced by the pass); over the pass-or-correct
                    // 2M opener doubles with four trumps, else waits.
                    // Structural, so it does not ride `penalty_double_leave_in`.
                    entries.extend(rows_of(
                        Pattern::after(NT, &format!("{their} X -")),
                        multi_pass_answer(),
                    ));
                    for major in [Suit::Hearts, Suit::Spades] {
                        entries.extend(rows_of(
                            Pattern::after(NT, &format!("{their} X (2{})", Strain::from(major))),
                            multi_penalty_answer(major),
                        ));
                    }
                    // N4f: the same double one branch over — responder
                    // *passed* the Multi and they named a major, the lane's
                    // one seat with no node at all (57% of the bucket).  The
                    // gate rises to five trumps because partner showed
                    // nothing; the anchor's own action here is exactly
                    // `len(M, 5..)` on 6-7% of hands and a pass on the rest.
                    if agreements.competition.multi_balance {
                        for major in [Suit::Hearts, Suit::Spades] {
                            let advance = format!("{their} - (2{})", Strain::from(major));
                            entries.extend(rows_of(
                                Pattern::after(NT, &advance),
                                multi_balance_double(major),
                            ));
                            // Responder sits, quiet or over their runout: the
                            // floor's documented failure in this lane is
                            // pulling exactly these penalty doubles.
                            entries.extend(rows_of(
                                Pattern::after(NT, &format!("{advance} X -")),
                                multi_signoff_pass(),
                            ));
                            entries.extend(rows_of(
                                Pattern::up_to(&format!("{NT} {advance} X"), "7♠"),
                                multi_signoff_pass(),
                            ));
                        }
                    }
                    // v3: the double family's continuations, all of them the
                    // same two-rule table or a sit.  The first two runs left
                    // every seat past opener's answer to the floor, which reads
                    // their 2♦ as diamonds and their 2M as a natural suit —
                    // and pulled the penalty doubles (responder's X of 2♥
                    // pulled to 4♥ by opener; opener's X of 2♠ pulled to 3♥
                    // by responder), the "consequent doubles are nominal
                    // penalty" structure the design named.  Their pass-or-
                    // correct resolves the major, so the resolved suit is the
                    // one the double keys on.
                    let sit = multi_signoff_pass();
                    // Responder, opener having sat: they passed (2♥ or 2♠ is
                    // theirs) or corrected 2♥ to 2♠.  v7: BBA's own second-turn
                    // table ([`multi_responder_rebid`]) less the rungs perfect
                    // defense refused — the takeout X of the resolved major,
                    // 3NT with a stopper, 4NT, the weak 2♠ — and opener's
                    // answers to each, so no seat of the family is floored.
                    let resolved = [
                        ("X (2♥) - -", Suit::Hearts, false),
                        ("X (2♥) - (2♠)", Suit::Spades, true),
                        ("X (2♠) - -", Suit::Spades, false),
                        ("X (2♥) X (2♠)", Suit::Spades, true),
                    ];
                    for (path, major, ran) in resolved {
                        entries.extend(rows_of(
                            Pattern::after(NT, &format!("{their} {path}")),
                            multi_responder_rebid(
                                major,
                                ran,
                                agreements.competition.multi_stopper_ask,
                            ),
                        ));
                        entries.extend(rows_of(
                            Pattern::after(NT, &format!("{their} {path} X -")),
                            if ran {
                                sit.clone()
                            } else {
                                multi_takeout_answer(major)
                            },
                        ));
                        entries.extend(rows_of(
                            Pattern::after(NT, &format!("{their} {path} 4NT -")),
                            multi_quant_answer(),
                        ));
                        let ask_mode = agreements.competition.multi_stopper_ask;
                        if ran && ask_mode != MultiStopperAsk::Off {
                            let ask = format!("{their} {path} 3♠");
                            let ask_key = format!("{NT} {ask}");
                            entries.extend(rows_of(
                                Pattern::after(NT, &format!("{ask} -")),
                                multi_stopper_answer(ask_mode),
                            ));
                            // Their double of the ask consumes no bidding room:
                            // strip it to a pass for this whole subtree.
                            entries.push(rebase(
                                Pattern::first(&ask_key, "X"),
                                ReplaceNext(Call::Pass),
                            ));

                            // Their 4♠ obstruction: double with a stopper or
                            // four trumps; otherwise Pass is forcing.
                            entries.extend(rows_of(
                                Pattern::after(NT, &format!("{ask} (4♠)")),
                                multi_stopper_over_four_spades(),
                            ));
                            entries.extend(rows_of(
                                Pattern::after(NT, &format!("{ask} (4♠) X -")),
                                sit.clone(),
                            ));
                            entries.extend(rows_of(
                                Pattern::after(NT, &format!("{ask} (4♠) - -")),
                                multi_stopper_forcing_rebid(),
                            ));
                            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                                let game = format!("5{}", Strain::from(suit));
                                for their_call in ["-", "(X)"] {
                                    entries.extend(rows_of(
                                        Pattern::after(
                                            NT,
                                            &format!("{ask} (4♠) - - {game} {their_call}"),
                                        ),
                                        sit.clone(),
                                    ));
                                }
                            }

                            if ask_mode == MultiStopperAsk::FitSearch {
                                // 3NT and 4♥ are already the final game.
                                for game in ["3NT", "4♥"] {
                                    for their_call in ["-", "(X)"] {
                                        entries.extend(rows_of(
                                            Pattern::after(
                                                NT,
                                                &format!("{ask} - {game} {their_call}"),
                                            ),
                                            sit.clone(),
                                        ));
                                    }
                                }
                                // A minor answer searches for responder's fit
                                // or longest remaining four-card side suit.
                                for shown in [Suit::Clubs, Suit::Diamonds] {
                                    let answer = format!("4{}", Strain::from(shown));
                                    entries.extend(rows_of(
                                        Pattern::after(NT, &format!("{ask} - {answer} -")),
                                        multi_fit_search_rebid(shown),
                                    ));
                                }
                                // Every search continuation except 4♣–4♦
                                // is game and terminal, even when doubled.
                                for continuation in
                                    ["4♣ - 5♣", "4♣ - 4♥", "4♦ - 5♦", "4♦ - 4♥", "4♦ - 5♣"]
                                {
                                    for their_call in ["-", "(X)"] {
                                        entries.extend(rows_of(
                                            Pattern::after(
                                                NT,
                                                &format!("{ask} - {continuation} {their_call}"),
                                            ),
                                            sit.clone(),
                                        ));
                                    }
                                }
                                entries.extend(rows_of(
                                    Pattern::after(NT, &format!("{ask} - 4♣ - 4♦ -")),
                                    multi_fit_search_place(),
                                ));
                                for game in ["5♣", "5♦"] {
                                    for their_call in ["-", "(X)"] {
                                        entries.extend(rows_of(
                                            Pattern::after(
                                                NT,
                                                &format!("{ask} - 4♣ - 4♦ - {game} {their_call}"),
                                            ),
                                            sit.clone(),
                                        ));
                                    }
                                }
                            } else {
                                // Opener placed the contract immediately.
                                for game in ["3NT", "4♥", "5♣", "5♦"] {
                                    for their_call in ["-", "(X)"] {
                                        entries.extend(rows_of(
                                            Pattern::after(
                                                NT,
                                                &format!("{ask} - {game} {their_call}"),
                                            ),
                                            sit.clone(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    entries.extend(rows_of(
                        Pattern::after(NT, &format!("{their} X (2♥) - - 2♠ -")),
                        sit.clone(),
                    ));
                    // Responder, opener having doubled and the advancer sat: sit.
                    for major in [Suit::Hearts, Suit::Spades] {
                        entries.extend(rows_of(
                            Pattern::after(
                                NT,
                                &format!("{their} X (2{}) X -", Strain::from(major)),
                            ),
                            sit.clone(),
                        ));
                    }
                    // Their 2NT over the doubled/undoubled 2♠ is the
                    // overcaller's heart relay (bba-1nt-defense.md): nothing
                    // to say until they place it — the floor cued 3♠.
                    for path in ["X (2♠) X (2NT)", "X (2♠) - (2NT)"] {
                        entries.extend(rows_of(
                            Pattern::after(NT, &format!("{their} {path}")),
                            sit.clone(),
                        ));
                    }
                }
                let opener_reply = match agreements.competition.double_style {
                    _ if multi => None,
                    // The armed `(2♦)` diamond penalty double promised the trumps,
                    // whatever the ambient style says, so opener sits on it.
                    _ if over == Suit::Diamonds
                        && agreements.competition.two_diamond_double.is_some() =>
                    {
                        Some(opener_leaves_in_penalty_double())
                    }
                    DoubleStyle::Penalty | DoubleStyle::PenaltyLight => {
                        Some(opener_leaves_in_penalty_double())
                    }
                    DoubleStyle::Optional => Some(opener_cooperates_optional(over)),
                    DoubleStyle::Takeout => None,
                };
                if let (true, Some(reply)) =
                    (agreements.competition.penalty_double_leave_in, opener_reply)
                {
                    entries.extend(rows_of(Pattern::after(NT, &format!("{their} X -")), reply));
                }

                // Opener completes the 2NT relay with 3♣, and responder rebids
                // over it (the weak relay sign-off).
                entries.extend(rows_of(
                    Pattern::after(NT, &format!("{their} 2NT -")),
                    complete_lebensohl_relay(agreements),
                ));
                let relay = format!("{their} 2NT - 3♣ -");
                entries.extend(rows_of(
                    Pattern::after(NT, &relay),
                    if multi {
                        // N4: their 2♦ held no diamonds, so 3♦ is ours to
                        // sign off in.
                        multi_relay_rebid()
                    } else {
                        lebensohl_relay_rebid(over, agreements)
                    },
                ));
                if multi {
                    // N4's interfered tail (the iron rule: they double it).
                    // The Multi leg sends weak diamond hands through the relay
                    // too, and the first run's worst board was responder
                    // passing their double of the forced 3♣ with five
                    // diamonds — the suffix was unauthored, so the floor sat.
                    // Their X of the relay takes no room: opener completes
                    // anyway; their X of the completion leaves responder the
                    // same sign-off table.
                    entries.extend(rows_of(
                        Pattern::after(NT, &format!("{their} 2NT (X)")),
                        complete_lebensohl_relay(agreements),
                    ));
                    for doubled in ["2NT (X) 3♣ -", "2NT - 3♣ (X)"] {
                        entries.extend(rows_of(
                            Pattern::after(NT, &format!("{their} {doubled}")),
                            multi_relay_rebid(),
                        ));
                    }
                    // Opener passes the sign-off — every relay path, every
                    // suit *of the Multi lane*.  Left to the floor in the first
                    // A/B, opener raised the weak 3♦ to 3NT on 45 of 52 relay
                    // boards (PD −2.9/−4.3 per board): the relay's whole loss
                    // was that seat.
                    //
                    // The natural-overcall lane below wires only opener's reply
                    // to a *major* sign-off ([`lebensohl_signoff_raise`]) and
                    // leaves its minor sign-off to the floor, where the same
                    // pathology cost 16 of 18 boards — N2 in
                    // docs/one-notrump-competitive.md.  That asymmetry is being
                    // fixed floor-side (Phase 0b), not by widening this loop:
                    // a node here would shadow the floor it is measured against.
                    for path in ["2NT - 3♣ -", "2NT (X) 3♣ -", "2NT - 3♣ (X)"] {
                        for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                            let signoff = format!("{their} {path} 3{}", Strain::from(suit));
                            entries.extend(rows_of(
                                Pattern::after(NT, &format!("{signoff} -")),
                                multi_signoff_pass(),
                            ));
                            // v3: and over their competition, both of us.  The
                            // second run's relay residue was `3♦ - - (3♠) 4♣ -
                            // 4♥` — responder correcting a weak sign-off to a
                            // four-level phantom, opener raising it.  Their X
                            // of the sign-off, their bid over it, and their
                            // balance after two passes: pass, pass, pass.
                            entries.extend(rows_of(
                                Pattern::after(NT, &format!("{signoff} (X)")),
                                multi_signoff_pass(),
                            ));
                            entries.extend(rows_of(
                                Pattern::up_to(&format!("{NT} {signoff}"), "7♠"),
                                multi_signoff_pass(),
                            ));
                            entries.extend(rows_of(
                                Pattern::up_to(&format!("{NT} {signoff} - -"), "7♠"),
                                multi_signoff_pass(),
                            ));
                        }
                    }
                }

                // Opener's reply to a weak major sign-off: pass, or stretch to
                // game with a maximum + fit (see [`lebensohl_signoff_raise`]).
                // Only a major *below* the overcall is reachable via the relay —
                // a higher major is bid naturally at the 2 level — so in
                // practice this wires only (2♠)→3♥.
                for signoff in [Suit::Hearts, Suit::Spades] {
                    if (signoff as u8) >= (over as u8) {
                        continue;
                    }
                    entries.extend(rows_of(
                        Pattern::after(NT, &format!("{relay} 3{} -", Strain::from(signoff))),
                        lebensohl_signoff_raise(signoff, 6),
                    ));
                }

                // Floored natural escape (only under `agreements.competition.natural_floor`):
                // opener's reply to a *direct* natural major sign-off — the
                // one-level-lower mirror of the relay sign-off raise above,
                // where 2M is a major *above* the overcall (a weak 5-card-suit
                // hand bids it naturally rather than relaying).  Same
                // [`lebensohl_signoff_raise`], but fed the natural floor (5, not
                // the relay's 6) so opener's game bar is one point higher to
                // compensate.
                //
                // The Multi lane's floorless escape
                // ([`CompetitionKnobs::multi_weak_escape`]) lets responder act
                // with no HCP at all, so opener's game bar moves with the
                // reading `project_authored` publishes: the same table, fed
                // `0`.  Getting that pair out of step is the reading-drift
                // failure mode, not a cosmetic detail.
                let weak_escape = if multi {
                    agreements.competition.multi_weak_escape
                } else {
                    None
                };
                if natural_floor_on(agreements) || weak_escape.is_some() {
                    let resp_floor = if weak_escape.is_some() {
                        0
                    } else {
                        natural_floor_hcp(agreements)
                    };
                    for signoff in [Suit::Hearts, Suit::Spades] {
                        if (signoff as u8) <= (over as u8) {
                            continue; // not above the overcall — no 2-level natural
                        }
                        let escape = Bid::new(2, Strain::from(signoff));
                        entries.extend(rows_of(
                            Pattern::after(NT, &format!("{their} {escape} -")),
                            lebensohl_signoff_raise(signoff, resp_floor),
                        ));
                        if weak_escape.is_none() {
                            continue;
                        }
                        // The interfered tail: their double, their advancer's
                        // pass-or-correct `2♠`, and every competitive call up
                        // to `3♠` (above that, and after opener's answer, the
                        // floor keeps the seat).
                        let overs = [
                            Bid::new(2, Strain::Spades),
                            Bid::new(2, Strain::Notrump),
                            Bid::new(3, Strain::Clubs),
                            Bid::new(3, Strain::Diamonds),
                            Bid::new(3, Strain::Hearts),
                            Bid::new(3, Strain::Spades),
                        ];
                        let theirs = std::iter::once(None)
                            .chain(overs.into_iter().filter(|b| *b > escape).map(Some));
                        for bid in theirs {
                            let call = bid.map_or_else(|| "X".to_string(), |b| b.to_string());
                            entries.extend(rows_of(
                                Pattern::after(NT, &format!("{their} {escape} ({call})")),
                                multi_escape_overcalled(signoff, bid, agreements),
                            ));
                        }
                    }
                }

                // Plain style: opener's reply to the direct cue (Stayman).
                // (Transfer wires its cue reply in the block below.)
                if style == LebensohlStyle::Plain {
                    entries.extend(rows_of(
                        Pattern::after(NT, &format!("{their} 3{o} -")),
                        cue_stayman_answer(over),
                    ));
                }

                // Transfer style: opener's reply to each 3-level transfer / cue.
                // Over (2♦) the Smolen block below owns the 3-level replies, so
                // this covers (2♥)/(2♠) only.
                if style == LebensohlStyle::Transfer && over != Suit::Diamonds {
                    for bid_suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                        let reply = if bid_suit == over {
                            cue_stayman_answer(over)
                        } else if let Some(target) = transfer_target(bid_suit, over) {
                            transfer_completion(target, over, agreements)
                        } else if over != Suit::Clubs {
                            clubs_transfer_completion(over, agreements) // top step → clubs (forced GF)
                        } else {
                            continue; // over (2♣): clubs is their suit — floored
                        };
                        entries.extend(rows_of(
                            Pattern::after(NT, &format!("{their} 3{} -", Strain::from(bid_suit))),
                            reply,
                        ));
                    }
                }

                // Recognize a delayed cue (2NT relay, then their suit) over
                // (2♥)/(2♠): Stayman with a stopper, answered like the direct
                // cue but with 3NT safe.  Always wired so a human partner who
                // plays it gets a sensible reply, even though the bot only
                // *bids* it under `agreements.competition.delayed_cue`.
                if style == LebensohlStyle::Transfer && unbid_major(over).is_some() {
                    entries.extend(rows_of(
                        Pattern::after(NT, &format!("{relay} 3{o} -")),
                        cue_stayman_answer(over),
                    ));
                }

                // Section 5c: Transfer over (2♦) — 3♣-Stayman + Smolen, the
                // Jacoby transfers (3♦→♥, 3♥→♠, 3♠→♣), and Leaping Michaels
                // 4♣/4♦.  (The 2♥/2♠ branches reuse the Transfer completions
                // above.)
                if style == LebensohlStyle::Transfer && over == Suit::Diamonds {
                    for (suffix, rules) in [
                        // 3♣ Stayman, opener's answer; then Smolen after the 3♦ denial.
                        ("3♣ -", stayman_2d_answer()),
                        ("3♣ - 3♦ -", smolen_at_three()),
                        (
                            "3♣ - 3♦ - 3♥ -",
                            smolen_completion(Suit::Spades, agreements),
                        ),
                        (
                            "3♣ - 3♦ - 3♠ -",
                            smolen_completion(Suit::Hearts, agreements),
                        ),
                        // Opener showed a 4-card major over Stayman; responder places.
                        ("3♣ - 3♥ -", stayman_2d_fit_rebid(Suit::Hearts)),
                        ("3♣ - 3♠ -", stayman_2d_fit_rebid(Suit::Spades)),
                        // Jacoby transfers: 3♦→♥, 3♥→♠ (auto-driven), 3♠→♣ (forced GF).
                        ("3♦ -", transfer_completion(Suit::Hearts, over, agreements)),
                        ("3♥ -", transfer_completion(Suit::Spades, over, agreements)),
                        (
                            "3♠ -",
                            if multi {
                                // N4: no diamond stopper to key on — 3NT outright.
                                multi_clubs_transfer_completion(agreements)
                            } else {
                                clubs_transfer_completion(over, agreements)
                            },
                        ),
                        // Leaping Michaels: 4♦ both majors, 4♣ clubs + a major (ask).
                        ("4♦ -", lm_2d_both_majors_advance()),
                        ("4♣ -", lm_2d_clubs_ask()),
                        ("4♣ - 4♦ -", lm_2d_clubs_major()),
                    ] {
                        entries.extend(rows_of(
                            Pattern::after(NT, &format!("{their} {suffix}")),
                            rules,
                        ));
                    }
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
