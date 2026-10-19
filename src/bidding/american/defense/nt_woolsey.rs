//! Woolsey ("Multi-Landy") over their `1NT`
//!
//! The equilibrium defense: `X` = exactly four cards in one major plus a longer
//! (5–6) minor, `2♣` = both majors (Landy's call), `2♦` = a Multi major,
//! `2♥`/`2♠` = Muiderberg (that major plus a minor).
//!
//! The first answer to every artificial call is authored over their pass and
//! their double (the `X` over their redouble), and so is the overcaller's
//! answer to each ask or pass-or-correct.  Deeper continuations and most of
//! their interference are **not** authored and fall to the instinct floor
//! (2,160 floored continuation boards of 204,800 kept in the 2026-10
//! isolate-defense forensics at `f8286a5a`).  Two opt-in knobs author the
//! costliest of those holes:
//! [`woolsey_muiderberg_sit`][field@crate::bidding::agreements::DefenseKnobs::woolsey_muiderberg_sit]
//! and [`woolsey_tails`][field@crate::bidding::agreements::DefenseKnobs::woolsey_tails].

use super::nt_defense::NotrumpDefense;
use super::nt_landy::sit;
use super::*;

/// Whether the Woolsey defense is the active system (read by the inference engine
/// to decode our artificial 2♣/2♦/2♥/2♠ overcalls — the alerted rules project
/// their own meaning; `inference::readers::landy_advance_suppress` still keys
/// its both-majors advance stub on this)
pub(super) fn woolsey_enabled(agreements: &Agreements) -> bool {
    agreements.decision.reading.notrump_defense == NotrumpDefense::Woolsey
}

/// Woolsey **Multi** `2♦`: a single 6+ card major (unknown which), nothing else long —
/// both minors at most four.  M6.2d simplified the shape and states it with `or`/`and`
/// so it projects straight off the rule: the strictly-longer-major and no-6-6 guards
/// are dropped, so a 6-5 or 6-6 major hand now qualifies as Multi.
pub(super) fn woolsey_multi() -> Cons<impl Constraint + Clone> {
    or([Suit::Hearts, Suit::Spades], 6..) & and([Suit::Clubs, Suit::Diamonds], ..=4)
}

/// Woolsey **Muiderberg** `2M`: exactly 5 in `major`, at most 3 in the other major, and
/// a 4+ card minor.  M6.2d states the minor side with `or` so it projects; the shape is
/// otherwise unchanged — `5..=5` keeps it disjoint from the 6+ Multi `2♦`, and the
/// other-major ≤3 cap keeps it disjoint from the 2♣ both-majors (the Woolsey structure
/// relies on disjoint shapes so its uniform 1.9 weights never tie).
pub(super) fn woolsey_muiderberg(major: Suit) -> Cons<impl Constraint + Clone> {
    let other = if major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    len(major, 5..=5) & len(other, ..=3) & or([Suit::Clubs, Suit::Diamonds], 4..)
}

/// Woolsey takeout `X`: exactly 4 in one major, at most 3 in the other, and a longer
/// (5-6) minor (a 7+ minor one-suiter passes — no natural minor overcall).  A 4-card
/// major can co-exist with at most a 5-card minor here, so the `or([♣,♦],5..=6)` needs
/// no upper cap on the second minor — the major half bars a 7+ minor anyway.
pub(super) fn woolsey_double_shape() -> Cons<impl Constraint + Clone> {
    ((len(Suit::Hearts, 4..=4) & len(Suit::Spades, ..=3))
        | (len(Suit::Spades, 4..=4) & len(Suit::Hearts, ..=3)))
        & or([Suit::Clubs, Suit::Diamonds], 5..=6)
}

/// Woolsey takeout `X`: a 4-card major + a longer (5-6) minor, `points(floor..)`.
pub(super) fn woolsey_x(agreements: &Agreements) -> Rules {
    Rules::new().rule(
        Call::Double,
        190,
        woolsey_double_shape() & points(agreements.decision.reading.woolsey_double_floor..),
    )
}

/// Woolsey `2♣` (Landy, inside the bundle): both majors, but neither major 6+
/// (`passed_two_suiter` caps each major at five, routing a 6-card major to the
/// Multi `2♦` and keeping the bundle's uniform 1.9 weights disjoint).  A distinct
/// block from [`landy_2c`] — same convention, load-bearing shape difference.
pub(super) fn woolsey_2c(agreements: &Agreements) -> Rules {
    let (lo, hi) = agreements.decision.reading.convention_points;
    Rules::new().rule(
        Bid::new(2, Strain::Clubs),
        190,
        passed_two_suiter(Suit::Hearts, Suit::Spades) & points(lo..=hi),
    )
}

/// Woolsey Multi `2♦`: a single 6+ major.
pub(super) fn multi_2d(agreements: &Agreements) -> Rules {
    let (lo, hi) = agreements.decision.reading.convention_points;
    Rules::new().rule(
        Bid::new(2, Strain::Diamonds),
        190,
        woolsey_multi() & points(lo..=hi),
    )
}

/// Woolsey Muiderberg `2♥`/`2♠`: exactly 5 in `major` + a 4+ minor.
pub(super) fn muiderberg(major: Suit, agreements: &Agreements) -> Rules {
    let (lo, hi) = agreements.decision.reading.convention_points;
    Rules::new().rule(
        Bid::new(2, Strain::from(major)),
        190,
        woolsey_muiderberg(major) & points(lo..=hi),
    )
}

// ---------------------------------------------------------------------------
// Woolsey "Multi-Landy" continuations: the Multi 2♦ (BBA's two-strength
// pass-or-correct — over the constructive 2♠ the overcaller bids 3♥ with hearts
// directly, no 2NT relay — plus a game-force ask), the Muiderberg 2♥/2♠ (raises
// + the 2NT minor-ask), and the takeout X (relay to the minor / own major /
// ask).  The both-majors 2♣ reuses the Landy advances in `nt_landy`.  The first
// answer to each artificial call is authored over their pass and their double
// (wired in `woolsey_package` below); deeper continuations are the floor's
// unless a `defense.woolsey_*` knob authors them.
// ---------------------------------------------------------------------------

/// Advancer over the Woolsey **Multi** `2♦` (`(1NT) 2♦ -` or `(1NT) 2♦ (X)`):
/// a major pass-or-correct in two strengths, plus a game-forcing ask.  Holds no
/// `Pass`, so over a double it always corrects rather than sitting in `2♦x` (the
/// overcaller has a major, never diamonds).  Thresholds track the overcall floor
/// `lo` (the `20-lo` / `22-lo` rule, as [`landy_advances`]).
fn multi_advances(lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);
    Rules::new()
        // Game-force: ask the overcaller to name its 6-card major (it jumps to 4M).
        .rule(Bid::new(2, Strain::Notrump), 100, points(game..))
        .alert(MULTI_ASK)
        // Constructive pass-or-correct: overcaller passes spades / bids 3♥ with hearts.
        .rule(Bid::new(2, Strain::Spades), 95, points(invite..game))
        .alert(MULTI_PC)
        // Weak pass-or-correct: overcaller passes hearts / corrects 2♠ / jumps with 7+.
        .rule(Bid::new(2, Strain::Hearts), 90, points(..invite))
        .alert(MULTI_PC)
}

/// Overcaller over the weak `2♥` pass-or-correct (`(1NT) 2♦ - 2♥ -`): pass
/// with six hearts, correct to `2♠` with six spades, jump to `3♥`/`3♠` with seven
fn multi_2h_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 110, len(Suit::Hearts, 7..))
        .rule(Bid::new(3, Strain::Spades), 110, len(Suit::Spades, 7..))
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 6..=6))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller over the constructive `2♠` pass-or-correct (`(1NT) 2♦ - 2♠ *`):
/// pass with spades, bid `3♥` with hearts.  Bidding the major directly (rather than
/// a 2NT heart-relay) keeps the rebid identical whether the `2♠` was passed or
/// doubled — over a double we must not be left to the floor.
fn multi_2s_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 6..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller over the game-forcing `2NT` ask (`(1NT) 2♦ - 2NT -`): jump to
/// game in the 6-card major
fn multi_2nt_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Hearts), 100, len(Suit::Hearts, 6..))
        .rule(Bid::new(4, Strain::Spades), 100, len(Suit::Spades, 6..))
}

/// Advancer over a **Muiderberg** `2M` (`(1NT) 2M -`): raise the known 5-card
/// major with support (`4M` game / `3M` invitational, or a `3M` preempt with
/// four-card support), or with no fit ask the 4+ minor via `2NT` (overcaller
/// answers `3♣`/`3♦`); a weak no-fit hand passes and plays `2M`.  `major` is the
/// overcaller's suit; thresholds track the overcall floor `lo`.
fn muiderberg_advances(major: Suit, lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);
    let strain = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, strain), 120, len(major, 3..) & points(game..))
        .rule(
            Bid::new(3, strain),
            110,
            (len(major, 4..) & points(..game)) | (len(major, 3..) & points(invite..game)),
        )
        // No major fit, invitational+: ask the 4+ minor (then place 3NT / minor game).
        .rule(
            Bid::new(2, Strain::Notrump),
            100,
            len(major, ..=2) & points(invite..),
        )
        .alert(MUIDERBERG_ASK)
        .rule(Call::Pass, 0, hcp(0..))
}

/// Advancer over a **doubled** Muiderberg `2M` (`(1NT) 2M (X)`): with a fit sit
/// for `2Mx` (a known 8+ card trump fit) or raise; without three-card support
/// escape via the `2NT` minor-ask — on a doubleton too, unless `sit`
///
/// `sit` is
/// [`woolsey_muiderberg_sit`][field@crate::bidding::agreements::DefenseKnobs::woolsey_muiderberg_sit]:
/// the escape then needs a void or singleton in the major (BBA's line), and
/// every other hand sits `2Mx` — a doubleton plays the 5-2 fit rather than a
/// doubled minor at the three level.  The raises are the same either way.
fn muiderberg_advances_doubled(major: Suit, lo: u8, sit: bool) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);
    let strain = Strain::from(major);
    let raises = Rules::new()
        .rule(Bid::new(4, strain), 120, len(major, 3..) & points(game..))
        .rule(
            Bid::new(3, strain),
            110,
            (len(major, 4..) & points(..game)) | (len(major, 3..) & points(invite..game)),
        );
    if sit {
        // Void or singleton → escape to the 4+ minor (any strength); else sit 2Mx.
        raises
            .rule(Bid::new(2, Strain::Notrump), 50, len(major, ..=1))
            .alert(MUIDERBERG_ASK)
            .rule(Call::Pass, 0, hcp(0..))
    } else {
        // Doubleton or shorter → escape to the 4+ minor (any strength); a fit sits 2Mx.
        raises
            .rule(Bid::new(2, Strain::Notrump), 50, len(major, ..=2))
            .alert(MUIDERBERG_ASK)
            .rule(Call::Pass, 0, len(major, 3..))
    }
}

/// Every bid the opener's side can make over `(1NT) 2M (X) -`: each bid above
/// our `2M`, up to `7NT`
fn bids_above(bid: Bid) -> impl Iterator<Item = Bid> {
    (2..=7u8)
        .flat_map(|level| Strain::ASC.map(|strain| Bid::new(level, strain)))
        .filter(move |&b| b > bid)
}

/// Advancer over their double of the weak pass-or-correct after the overcaller
/// passed it with hearts — the reopening `(1NT) 2♦ {-|(X)} 2♥ - - (X)` and the
/// direct `(1NT) 2♦ {-|(X)} 2♥ (X) - -`: run to a natural `2♠` only with six
/// spades, else sit `2♥x`.  The overcaller's pass showed exactly six hearts, so
/// even a singleton heart is a 6-1 fit — better than `2♠` opposite the 0–3
/// spades the heart Multi holds.  Part of
/// [`woolsey_tails`][field@crate::bidding::agreements::DefenseKnobs::woolsey_tails].
fn multi_2h_doubled_advance() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 6..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Advancer over their `2♥` interference of the Multi `2♦` (`(1NT) 2♦ (2♥)`):
/// `X` = values with game interest (`22 − lo`+ points, any shape); `2♠` =
/// pass-or-correct on three-card spade tolerance, whatever the hearts; else
/// pass.  Part of
/// [`woolsey_tails`][field@crate::bidding::agreements::DefenseKnobs::woolsey_tails].
///
/// Their `2♥` is natural (5+ hearts, BBA's 4–9 HCP), so the overcaller almost
/// always holds the spade Multi: the overcaller's `3♥` answer to this `2♠`
/// ([`multi_2s_rebid`] at `(1NT) 2♦ (2♥) 2♠ *`) is a near-dead catch-all, kept
/// so a six-heart Multi is never left to the floor.  No heart length is asked
/// of the `2♠` — a short-heart hand is exactly the one that most wants to
/// compete.
fn multi_over_2h_advance(lo: u8) -> Rules {
    let game = 22u8.saturating_sub(lo);
    Rules::new()
        // Near-dead (flagged, left as is): opener's 15–17, their 4+ HCP 2♥ and
        // the overcall leave the advancer ~13–15 HCP at the very most (~6
        // typical), so `22 − lo` (14 at the shipped 8) points needs shape too.
        .rule(Call::Double, 110, points(game..))
        .alert(MULTI_VALUES_X)
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 3..))
        .alert(MULTI_PC)
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller over the values double of their `2♥` (`(1NT) 2♦ (2♥) X -`): bid
/// `2♠` with the spade Multi, else sit for `2♥x` — the overcaller's six hearts
/// opposite a game-interest hand.  Part of
/// [`woolsey_tails`][field@crate::bidding::agreements::DefenseKnobs::woolsey_tails].
fn multi_values_x_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 6..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Advancer over their `2♠` interference of the Multi `2♦` (`(1NT) 2♦ (2♠)`):
/// `3♥` = pass-or-correct on three-card heart tolerance, whatever the spades;
/// else pass.  Part of
/// [`woolsey_tails`][field@crate::bidding::agreements::DefenseKnobs::woolsey_tails].
fn multi_over_2s_advance() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 3..))
        .alert(MULTI_PC)
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller over the `3♥` pass-or-correct (`(1NT) 2♦ (2♠) 3♥ *`): pass with
/// hearts, correct to `3♠` with spades.  Part of
/// [`woolsey_tails`][field@crate::bidding::agreements::DefenseKnobs::woolsey_tails].
///
/// Their `2♠` is natural (5+ spades), so the overcaller almost always holds the
/// heart Multi and the `3♠` correction is a near-dead catch-all, kept so a
/// six-spade Multi is never left to the floor.
fn multi_3h_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Spades), 100, len(Suit::Spades, 6..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller after their natural `2♥` over the Multi went `(1NT) 2♦ (2♥) - -`:
/// the advancer's pass denied three spades and game values, so balance a
/// natural `2♠` with the spade Multi, else pass.  Part of
/// [`woolsey_tails`][field@crate::bidding::agreements::DefenseKnobs::woolsey_tails].
fn multi_over_2h_balance() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 6..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller after their natural `2♠` over the Multi went `(1NT) 2♦ (2♠) - -`:
/// a natural `3♥` only with seven hearts, else pass.  Six would always qualify
/// (every heart Multi has six) and the advancer's pass denied three, so `3♥`
/// on six plays a 6-2 fit at best (jdh8, 2026-10-20).  Part of
/// [`woolsey_tails`][field@crate::bidding::agreements::DefenseKnobs::woolsey_tails].
fn multi_over_2s_balance() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 7..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller answering the Muiderberg `2NT` minor-ask (`(1NT) 2M … 2NT -`):
/// name the 4+ minor — `3♦` with diamonds (longer or equal), else `3♣`
fn muiderberg_2nt_rebid() -> Rules {
    let diamonds_longer = at_least_as_long(Suit::Diamonds, Suit::Clubs);
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 100, diamonds_longer)
        .rule(Bid::new(3, Strain::Clubs), 90, hcp(0..))
}

/// Advancer over the Woolsey takeout `X` (`(1NT) X -`): bid a 5+ major of your
/// own (to play), ask with a game-going hand, else relay `2♣` to the doubler's
/// long minor.  The catch-all `2♣` owns a finite logit so the floor never runs.
fn woolsey_x_advance(lo: u8) -> Rules {
    let game = 22u8.saturating_sub(lo);
    Rules::new()
        // Our own good major outranks the doubler's (its major may be the other one).
        .rule(Bid::new(2, Strain::Spades), 111, len(Suit::Spades, 5..))
        .rule(Bid::new(2, Strain::Hearts), 110, len(Suit::Hearts, 5..))
        // Game-going: ask the doubler to name its 4-card major.
        .rule(Bid::new(2, Strain::Notrump), 100, points(game..))
        .alert(WOOLSEY_X_ASK)
        // Weak / no major of our own: name your minor, I pass or correct.
        .rule(Bid::new(2, Strain::Clubs), 90, hcp(0..))
        .alert(WOOLSEY_X_RELAY)
}

/// Doubler over the `2♣` minor relay (`(1NT) X - 2♣ -`): pass with the club
/// minor, correct to `2♦` with the diamond minor (advancer denied a major)
fn woolsey_x_minor_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 100, len(Suit::Diamonds, 5..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Doubler over the `2NT` game-ask (`(1NT) X - 2NT -`): name the 4-card major
/// (the `X` always holds exactly one), leaving the advancer to place the game
fn woolsey_x_2nt_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(Bid::new(3, Strain::Spades), 100, len(Suit::Spades, 4..))
}

/// Woolsey "Multi-Landy" continuations ([`NotrumpDefense::Woolsey`])
///
/// The both-majors `2♣` reuses [`landy_advance_package`]'s wiring.  The first
/// answer to each artificial call is authored over their pass and their double
/// (the `X` over their redouble); deeper continuations fall to the floor unless
/// [`woolsey_muiderberg_sit`][field@crate::bidding::agreements::DefenseKnobs::woolsey_muiderberg_sit]
/// or [`woolsey_tails`][field@crate::bidding::agreements::DefenseKnobs::woolsey_tails]
/// authors them.
pub(super) fn woolsey_package() -> Package {
    Package {
        name: "woolsey",
        gate: |agreements| woolsey_enabled(agreements),
        entries: |agreements| {
            let lo = agreements.decision.reading.convention_points.0;
            let sit_doubled = agreements.defense.woolsey_muiderberg_sit;
            let tails = agreements.defense.woolsey_tails;
            let mut entries = Vec::new();

            // Multi 2♦.  The advance is the same over a pass or a double (it never
            // sits 2♦x — the overcaller has a major, not diamonds).  `rho` is the
            // opponents' call over our 2♦; `after` is their call over our
            // pass-or-correct — the overcaller names its major regardless of a
            // double, so we are never left to the floor in a doubled 2♥x/2♠x (the
            // dominant 2♦ leak vs BBA).
            for rho in ["-", "(X)"] {
                let base = format!("P* (1NT) 2♦ {rho}");
                entries.extend(rows_of(Pattern::node(&base), multi_advances(lo)));
                for after in ["-", "(X)"] {
                    for (bid, rebid) in [
                        // Weak 2♥ p/c → pass / correct 2♠ / jump 3M with seven.
                        ("2♥", multi_2h_rebid()),
                        // Constructive 2♠ p/c → pass spades / 3♥ with hearts.
                        ("2♠", multi_2s_rebid()),
                        // Game-force 2NT ask → overcaller jumps to game in its major.
                        ("2NT", multi_2nt_rebid()),
                    ] {
                        entries.extend(rows_of(
                            Pattern::node(&format!("{base} {bid} {after}")),
                            rebid,
                        ));
                    }
                }
                // Their double of the weak 2♥ the overcaller passed with
                // hearts, reopening or direct (woolsey_tails).
                if tails {
                    for doubled in ["2♥ - - (X)", "2♥ (X) - -"] {
                        entries.extend(rows_of(
                            Pattern::node(&format!("{base} {doubled}")),
                            multi_2h_doubled_advance(),
                        ));
                    }
                }
            }

            // Their 2♥/2♠ over the Multi (woolsey_tails): the advancer's
            // pass-or-correct (and values X over 2♥), and the overcaller's answers,
            // whether the pass-or-correct is passed or doubled.
            if tails {
                entries.extend(rows_of(
                    Pattern::node("P* (1NT) 2♦ (2♥)"),
                    multi_over_2h_advance(lo),
                ));
                entries.extend(rows_of(
                    Pattern::node("P* (1NT) 2♦ (2♥) X -"),
                    multi_values_x_rebid(),
                ));
                entries.extend(rows_of(
                    Pattern::node("P* (1NT) 2♦ (2♠)"),
                    multi_over_2s_advance(),
                ));
                // The advancer passed, so did opener: the overcaller balances.
                entries.extend(rows_of(
                    Pattern::node("P* (1NT) 2♦ (2♥) - -"),
                    multi_over_2h_balance(),
                ));
                entries.extend(rows_of(
                    Pattern::node("P* (1NT) 2♦ (2♠) - -"),
                    multi_over_2s_balance(),
                ));
                for after in ["-", "(X)"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* (1NT) 2♦ (2♥) 2♠ {after}")),
                        multi_2s_rebid(),
                    ));
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* (1NT) 2♦ (2♠) 3♥ {after}")),
                        multi_3h_rebid(),
                    ));
                }
            }

            // Muiderberg 2♥/2♠ — raises + the 2NT minor-ask (a doubled escape with
            // no fit).
            for (major, mbid) in [(Suit::Hearts, "2♥"), (Suit::Spades, "2♠")] {
                entries.extend(rows_of(
                    Pattern::node(&format!("P* (1NT) {mbid} -")),
                    muiderberg_advances(major, lo),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("P* (1NT) {mbid} (X)")),
                    muiderberg_advances_doubled(major, lo, sit_doubled),
                ));
                // The advancer sat 2Mx (woolsey_muiderberg_sit): the overcaller,
                // exactly five in the major, sits too when opener passes (no
                // escape — the structure defines none) and passes their run-out.
                if sit_doubled {
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* (1NT) {mbid} (X) - -")),
                        sit(),
                    ));
                    for run in bids_above(Bid::new(2, Strain::from(major))) {
                        entries.extend(rows_of(
                            Pattern::node(&format!("P* (1NT) {mbid} (X) - ({run})")),
                            sit(),
                        ));
                    }
                }
                // The 2NT minor-ask reaches the overcaller over either RHO action.
                for rho in ["-", "(X)"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* (1NT) {mbid} {rho} 2NT -")),
                        muiderberg_2nt_rebid(),
                    ));
                }
            }

            // Takeout X — advancer relays to the minor / bids its own major / asks
            // 2NT.  A redouble forces us to run (never sit 1NTxx): the same advance
            // applies.
            let xfloor = agreements.decision.reading.woolsey_double_floor;
            for adv in ["-", "(XX)"] {
                let base = format!("P* (1NT) X {adv}");
                entries.extend(rows_of(Pattern::node(&base), woolsey_x_advance(xfloor)));
                // The doubler names its 5-6 minor whether the 2♣ relay is passed or
                // doubled.
                for after in ["-", "(X)"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{base} 2♣ {after}")),
                        woolsey_x_minor_rebid(),
                    ));
                }
                // The 2NT game-ask → the doubler names its 4-card major.
                entries.extend(rows_of(
                    Pattern::node(&format!("{base} 2NT -")),
                    woolsey_x_2nt_rebid(),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
