//! Lebensohl after our `1NT` is overcalled
//!
//! The weak `2NT` relay to `3♣`, the cue as game-forcing Stayman, the
//! direct-versus-slow distinction, and the signoff raises.  The transfer
//! ("Rubensohl") variant is [`super::rubensohl`]; [`LebensohlStyle`] picks
//! between them.

use super::penalty_double::{
    DoubleStyle, opener_cooperates_optional, opener_leaves_in_penalty_double, responder_double,
};
use super::rubensohl::{
    clubs_transfer_completion, cue_stayman_answer, lm_2d_both_majors_advance, lm_2d_clubs_ask,
    lm_2d_clubs_major, stayman_2d_answer, stayman_2d_fit_rebid, transfer_completion,
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

/// Whether the `(2♦)`-as-Multi counter-defense is engaged
fn defense_2d_multi(agreements: &Agreements) -> bool {
    agreements.competition.defense_2d_multi
}

/// Whether the `(2♣)`-as-Landy counter-defense is engaged — a fact about the
/// opponents (their disclosed `2♣`), not a knob of ours
fn defense_2c_landy(agreements: &Agreements) -> bool {
    agreements.their.two_clubs_landy
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

/// Responder's counter-defense after `1NT (2♦)` when the `2♦` is read as a
/// **Multi** (an unknown single-suited major), engaged by
/// `agreements.competition.defense_2d_multi`
///
/// Distilled from BBA's Multi-Landy counter (`docs/ai-bidder/bba-multi-2d.md`):
/// **double = values / takeout** of the unknown major (BBA's 41% workhorse), and
/// everything else **natural**. Unlike the natural-diamond treatments, both
/// majors are biddable naturally at the 2 level and `2♦` steals no major room, so
/// there is no Stayman cue — the diamond bid that would be the cue is just natural
/// diamonds. The `2NT` relay and its `3♣` completion are the shared Lebensohl
/// machinery (registered for `(2♦)` regardless of this toggle), so weak
/// club/diamond one-suiters keep their sign-off.
fn multi_responder(agreements: &Agreements) -> Rules {
    let over = Suit::Diamonds; // the call we sit over; their real suit is a major
    let mut rules = Rules::new();

    // X = values / takeout of the unknown major — BBA's backbone (41%). Floored
    // at 8 (a touch above BBA's loose ~5) for doubled-contract discipline.
    rules = rules
        .rule(Call::Double, 155, points(8..))
        .alert(MULTI_TAKEOUT);

    // Natural forcing 3-level single-suiter (incl. natural 3♦ — diamonds is not
    // their suit, so no cue).
    for s in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(s);
        rules = rules.rule(Bid::new(3, strain), 180, len(s, 5..) & points(10..));
    }

    // Direct 3NT to play (default toggles → plain game values).
    rules = author_direct_3nt(rules, 170, over, agreements);

    // Natural weak 2-level major — both majors clear the `2♦` overcall.
    for s in [Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(2, strain),
            150,
            len(s, 5..)
                & points(..=9)
                & hcp(natural_floor_hcp(agreements)..)
                & points(natural_floor_pts(agreements)..),
        );
    }

    // 2NT = Lebensohl relay to 3♣ (weak long minor / suit below the majors).
    let long_suit = lebensohl_relay_shape(over);
    rules = rules
        .rule(Bid::new(2, Strain::Notrump), 140, points(..=9) & long_suit)
        .alert(LEBENSOHL_RELAY);

    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Responder's counter-defense after `1NT (2♣)` when the `2♣` is read as
/// **Landy** (both majors, 5-4 or better), engaged by the opponents'
/// disclosure (`agreements.their.two_clubs_landy`)
///
/// Systems-on — the default here — is the right treatment over a *natural* `2♣`,
/// which steals no room.  Over Landy it inverts: the stolen `X` asks for a
/// four-card major against a hand that has just shown **both**, and `2♦`/`2♥`
/// are Jacoby transfers *into* their suits.  So drop the whole structure and
/// play the hand for what it is — we hold 15-17 opposite a limited two-suiter,
/// which makes defending the live option:
///
/// - `X` = values (8+), penalty-oriented.  They will usually run to a major,
///   and the floor already encircles that escape (`penalize_escape_stack` /
///   `penalize_escape_values`), so this hands a job to machinery that exists.
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
    rules = rules.rule(Call::Double, 145, hcp(8..)).alert(LANDY_VALUES);

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
    //   skeleton — the cues (`2♥` = 5+♣, `2♠` = 5+♦, game-forcing) carry
    //   *every* GF minor one-suiter, six-carders included, and the direct
    //   `3♣`/`3♦` flip to natural **weak** escapes (a forcing 3m would be
    //   redundant with the cue below it).  The weak `3♦` is mostly shadowed by
    //   the cheaper `2♦` (140 > 110); it survives only below the 2♦ escape's
    //   hcp floor.  `2♥` edges `2♠` so 5-5 minors cue cheaper — which also
    //   means a GF hand with longer diamonds than clubs (6♦5♣) shows the
    //   clubs; rare enough to leave.
    if agreements.competition.defense_2c_landy_cues {
        rules = rules
            .rule(
                Bid::new(2, Strain::Hearts),
                173,
                len(Suit::Clubs, 5..) & points(10..),
            )
            .alert(LANDY_CUE)
            .rule(
                Bid::new(2, Strain::Spades),
                172,
                len(Suit::Diamonds, 5..) & points(10..),
            )
            .alert(LANDY_CUE);
        for s in [Suit::Clubs, Suit::Diamonds] {
            let strain = Strain::from(s);
            rules = rules.rule(Bid::new(3, strain), 110, len(s, 6..) & points(2..=9));
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
    rules = rules.rule(
        Bid::new(2, Strain::Diamonds),
        140,
        len(Suit::Diamonds, 5..)
            & points(..=9)
            & hcp(natural_floor_hcp(agreements)..)
            & points(natural_floor_pts(agreements)..),
    );

    // Natural invitational 2NT.
    rules = rules.rule(Bid::new(2, Strain::Notrump), 130, points(8..=9));

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

/// Opener's answer to the base counter's forcing `3♣`/`3♦` (`1NT (2♣) 3m -`)
///
/// One of the `landy_natural_answers` trio, wired only with the cues *off*
/// (under N1b the direct minors are weak and take [`landy_signoff_answer`]).
/// Responder shows a six-card source of tricks and game values, and opener
/// chooses — the point of ranking the naturals above 3NT: `3NT` with both of
/// their majors stopped, else raise.  The raise doubles as the finite
/// catch-all — opener is balanced, so it never lands on fewer than two, and
/// responder has six.  [`landy_cue_answer`] one level up; without the node the
/// floor answers `3♣` as the Puppet Stayman it replaced (85% phantom `3♦`).
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

/// Opener's answer to a Landy GF minor cue (`1NT (2♣) 2♥/2♠ -`)
///
/// The cue is game-forcing with 5+ in the named minor, so opener describes:
/// `2NT` with both majors stopped (keeping 3NT live from the strong side),
/// else a raise of the minor.  The raise doubles as the finite catch-all —
/// opener is balanced, so it never lands on fewer than two — and everything
/// deeper is the floor's.
fn landy_cue_answer(minor: Suit) -> Rules {
    let strain = Strain::from(minor);
    Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            150,
            stopper_in(Suit::Hearts) & stopper_in(Suit::Spades),
        )
        .rule(Bid::new(3, strain), 100, len(minor, 3..))
        .rule(Bid::new(3, strain), 20, hcp(0..))
}

/// Opener completes responder's Lebensohl `2NT` relay with the forced `3♣`
pub(crate) fn complete_lebensohl_relay() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Clubs), 100, hcp(0..))
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

            if defense_2c_landy(agreements) {
                // Landy counter: their 2♣ shows both majors, so systems-on is
                // exactly the wrong structure — see `landy_responder`.  This is
                // an *either/or* with the rebase below, not an overlay: leaving
                // the rebase registered would strip the 2♣ and remap our values
                // X back onto the stolen 2♣ Stayman one round later, sending
                // `1NT (2♣) X (2♥)` into the contested-Stayman package.  So the
                // book claims responder's first call and opener's one answer to
                // each of them, and every deeper auction is the floor's — which
                // is where we want it: the floor already encircles their escape
                // from our double (`penalize_escape_stack`/`_values`).
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
                entries.extend(rows_of(
                    Pattern::after("P* 1NT (2♣)", "2NT -"),
                    landy_invite_answer(agreements),
                ));
                for minor in [Suit::Clubs, Suit::Diamonds] {
                    entries.extend(rows_of(
                        Pattern::after("P* 1NT (2♣)", &format!("3{} -", Strain::from(minor))),
                        // Under N1b the direct 3m is a weak escape (see
                        // `landy_responder`), so opener sits; the forcing
                        // 3NT-or-raise answer belongs to the base arm only.
                        if agreements.competition.defense_2c_landy_cues {
                            landy_signoff_answer()
                        } else {
                            landy_minor_answer(minor)
                        },
                    ));
                }
                if agreements.competition.defense_2c_landy_cues {
                    entries.extend(rows_of(
                        Pattern::after("P* 1NT (2♣)", "2♥ -"),
                        landy_cue_answer(Suit::Clubs),
                    ));
                    entries.extend(rows_of(
                        Pattern::after("P* 1NT (2♣)", "2♠ -"),
                        landy_cue_answer(Suit::Diamonds),
                    ));
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
                        stayman_answers().rule(
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

                // Responder's first action: the uncovered suffix is exactly
                // their overcall.
                entries.extend(rows_of(
                    Pattern::after(NT, &their),
                    match style {
                        _ if over == Suit::Diamonds && defense_2d_multi(agreements) => {
                            multi_responder(agreements)
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
                let opener_reply = match agreements.competition.double_style {
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
                    complete_lebensohl_relay(),
                ));
                let relay = format!("{their} 2NT - 3♣ -");
                entries.extend(rows_of(
                    Pattern::after(NT, &relay),
                    lebensohl_relay_rebid(over, agreements),
                ));

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
                if natural_floor_on(agreements) {
                    for signoff in [Suit::Hearts, Suit::Spades] {
                        if (signoff as u8) <= (over as u8) {
                            continue; // not above the overcall — no 2-level natural
                        }
                        entries.extend(rows_of(
                            Pattern::after(NT, &format!("{their} 2{} -", Strain::from(signoff))),
                            lebensohl_signoff_raise(signoff, natural_floor_hcp(agreements)),
                        ));
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
                            transfer_completion(target, over)
                        } else if over != Suit::Clubs {
                            clubs_transfer_completion(over) // top step → clubs (forced GF)
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
                        ("3♣ - 3♦ - 3♥ -", smolen_completion(Suit::Spades)),
                        ("3♣ - 3♦ - 3♠ -", smolen_completion(Suit::Hearts)),
                        // Opener showed a 4-card major over Stayman; responder places.
                        ("3♣ - 3♥ -", stayman_2d_fit_rebid(Suit::Hearts)),
                        ("3♣ - 3♠ -", stayman_2d_fit_rebid(Suit::Spades)),
                        // Jacoby transfers: 3♦→♥, 3♥→♠ (auto-driven), 3♠→♣ (forced GF).
                        ("3♦ -", transfer_completion(Suit::Hearts, over)),
                        ("3♥ -", transfer_completion(Suit::Spades, over)),
                        ("3♠ -", clubs_transfer_completion(over)),
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
