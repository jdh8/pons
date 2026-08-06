//! The rich advance of a takeout double — transfers, cues, and minor jumps
//!
//! Opt-in ([`set_rich_advance_double`]): advancer transfers so the doubler
//! declares, cues to force, and jumps in a minor with a stopper ask above it.
//! [`set_advance_rubens`], [`set_longest_first_advance`],
//! [`set_advance_pass_yield_major`], [`set_advance_sit_hcp_gate`] and
//! [`set_advance_minor_jump`] each tune one rung.

use super::advance_double::{cheapest_forced, natural_advance, no_unbid_major};
use super::*;

thread_local! {
    /// Whether the *rich* advance of partner's takeout double of a one-opening
    /// (`(1t) X -`) is authored — the cue + notrump ladder that gives the
    /// advancer an invite/force channel; see [`set_rich_advance_double`].
    static RICH_ADVANCE_DOUBLE: Cell<bool> = const { Cell::new(true) };
    /// Whether the **jump-cue Rubens transfer** layer sits on top of the rich
    /// advance — a jump-cue transfer to a 5+ unbid major; see
    /// [`set_advance_rubens`].  No effect unless [`RICH_ADVANCE_DOUBLE`] is on.
    static ADVANCE_RUBENS: Cell<bool> = const { Cell::new(false) };
    /// Whether the advance of partner's takeout double bids the **longest** suit
    /// (weight climbing with length) rather than the highest-ranking 4+ suit;
    /// see [`set_longest_first_advance`].
    static LONGEST_FIRST_ADVANCE: Cell<bool> = const { Cell::new(true) };
    /// Whether the advancer's three-level jump in a **minor** shows an
    /// invitational one-suiter (5+, 10–12, denying a 4-card unbid major); see
    /// [`set_advance_minor_jump`].  No effect unless [`RICH_ADVANCE_DOUBLE`] is on.
    static ADVANCE_MINOR_JUMP: Cell<bool> = const { Cell::new(true) };
    /// Whether the advancer's **weak** penalty pass yields to a 4+ unbid major
    /// (below the cue band the hand bids the ladder instead of sitting); see
    /// [`set_advance_pass_yield_major`].
    static ADVANCE_PASS_YIELD_MAJOR: Cell<bool> = const { Cell::new(false) };
    /// The advancer's 4-card penalty-pass quality gate as a `suit_hcp` floor —
    /// `None` keeps the shipped `top_honors(t, 2..)`; see
    /// [`set_advance_sit_hcp_gate`].
    static ADVANCE_SIT_HCP_GATE: Cell<Option<u8>> = const { Cell::new(None) };
    /// Whether the doubler answers the advancer's invitational `2NT` with an
    /// authored accept/decline instead of falling to the instinct floor (which
    /// passes even game-going hands); see [`set_advance_2nt_continuation`].  **On
    /// by default** — a wash-positive A/B fix to a strict floor-pass in the
    /// default-on rich advance.  No effect unless [`RICH_ADVANCE_DOUBLE`] is on.
    static ADVANCE_2NT_CONTINUATION: Cell<bool> = const { Cell::new(true) };
}

/// Toggle the **rich advance** of partner's takeout double of a one-of-a-suit
/// opening (`(1t) X - ?`) for books built *after* this call (thread-local,
/// read once at book-construction time)
///
/// **On by default** (the shipped behavior); pass `false` (`bba-gen
/// --no-ns-rich-advance`) to drop back to the flat [`advance_double`] ladder.
/// The advancer gets a rich ladder: a cue of opener's suit asking for a 4-card
/// major (invitational 10–11 — the Stayman-ask; game hands blast 4M), a notrump
/// ladder (`1NT` 7–10 / `2NT` 11–12 / `3NT` 13+), weak shapely game jumps, and a
/// forced 3-card-suit response when broke — filling the invite/force gap the
/// flat floor leaves. Measured a constructive win vs the flat book (see
/// `docs/ai-bidder/21gf-ledger.md`).
pub fn set_rich_advance_double(on: bool) {
    RICH_ADVANCE_DOUBLE.with(|cell| cell.set(on));
}

/// Whether the rich advance of a takeout double is currently authored
pub(super) fn rich_advance_double_enabled() -> bool {
    RICH_ADVANCE_DOUBLE.with(Cell::get)
}

/// Toggle the **jump-cue Rubens transfer** layer on top of the rich advance for
/// books built *after* this call (thread-local, read at book-construction time)
///
/// **Off by default**, and a no-op unless [`set_rich_advance_double`] is also on.
/// When on, the advancer's jump-cue (and, over `(1♠)`, a natural `3♥`) becomes a
/// **transfer to a 5+ unbid major** (invitational-or-better) — the doubler
/// completes and *declares*, right-siding the strong hand.  Right-siding is
/// invisible to double-dummy (the trick count is the same whoever declares), so
/// its value shows up under the single-dummy lead scorer, not the DD A/B; this
/// knob (`bba-gen --ns-advance-rubens`) exists to confirm no DD *regression* and
/// as an sd-lead re-measure candidate.  See `docs/ai-bidder/21gf-ledger.md`.
pub fn set_advance_rubens(on: bool) {
    ADVANCE_RUBENS.with(|cell| cell.set(on));
}

/// Whether the jump-cue Rubens transfer layer is currently authored
fn advance_rubens_enabled() -> bool {
    ADVANCE_RUBENS.with(Cell::get)
}

/// Toggle the **longest-first** suit discipline for the flat advance of partner's
/// takeout double of a one-of-a-suit opening (`(1t) X - ?`) for books built
/// *after* this call (thread-local, read at book-construction time)
///
/// **On by default** (the shipped behavior); pass `false` (`bba-gen
/// --no-ns-longest-advance`) to score every eligible 4+ suit alike, whereupon
/// the argmax tie-break bids the **highest-ranking** one regardless of length —
/// holding five clubs and four spades it advances `1♠`, not `2♣`. On, the
/// natural-advance weight climbs with suit length, so the advancer bids the
/// **longest** suit and breaks equal-length ties toward the higher-ranking suit
/// (a major over a minor, spades over hearts) — standard takeout-double
/// advancing.
pub fn set_longest_first_advance(on: bool) {
    LONGEST_FIRST_ADVANCE.with(|cell| cell.set(on));
}

/// Whether the longest-first advance discipline is currently authored
pub(super) fn longest_first_advance_enabled() -> bool {
    LONGEST_FIRST_ADVANCE.with(Cell::get)
}

/// Toggle the weak advancer's **pass-yield to a 4-card major** over partner's
/// takeout double (`(1t) X - ?`) for books built *after* this call
/// (thread-local, read at book-construction time)
///
/// **Off by default.**  On (`bba-gen --ns-advance-pass-yield`), the penalty
/// pass's trump-stack legs yield when the hand is *below the cue band*
/// (`hcp ≤ 9`) **and** holds a 4+ unbid major: instead of converting the
/// double to penalty, the hand advances on the normal longest-first ladder
/// (which may still land in a longer minor).  Strong sits (10+) stand
/// regardless — restricting *them* is the refuted cap migration
/// (`ab-results/advance-penalty-pass/`, −2 IMPs/fired on both scorers).  The
/// A/B knob for `scripts/ab-advance-pass-yield.sh`.
pub fn set_advance_pass_yield_major(on: bool) {
    ADVANCE_PASS_YIELD_MAJOR.with(|cell| cell.set(on));
}

/// Whether the weak penalty pass yields to a 4-card unbid major
pub(super) fn advance_pass_yield_major_enabled() -> bool {
    ADVANCE_PASS_YIELD_MAJOR.with(Cell::get)
}

/// Swap the advancer's **4-card penalty-pass quality gate** over partner's
/// takeout double (`(1t) X - ?`) to a per-suit HCP floor for books built
/// *after* this call (thread-local, read at book-construction time)
///
/// **`None` by default** (the shipped behavior): a 4-card trump stack sits
/// with two of the top three honors.  `Some(n)` (`bba-gen
/// --ns-advance-sit-hcp N`) replaces that gate with `suit_hcp(t, n..)` in the
/// **rich** advance only — the flat book, which is also the weak-two advance
/// node, keeps the honor gate.  The candidate gates nest,
/// {6+} ⊂ {top2} ⊂ {5+}:
/// - `Some(5)` admits exactly one new class, **AJxx** — KQ = 5 is the
///   cheapest two of A/K/Q, so nothing is removed (the same subset relation
///   probed for BBA's Ogust "good suit"; see [`suit_hcp`]);
/// - `Some(6)` instead drops exactly **bare KQxx** (no jack ⇒ 5) while
///   keeping KQJx/AKxx/AQxx; AJxx stays out (5 is the most a single top
///   honor can carry).
///
/// Composes with [`set_advance_pass_yield_major`]: the yield wraps whichever
/// sit gate is live (both default-off, so the default system is untouched).
/// The sweep knob for `scripts/ab-advance-sit-hcp.sh`.
pub fn set_advance_sit_hcp_gate(gate: Option<u8>) {
    ADVANCE_SIT_HCP_GATE.with(|cell| cell.set(gate));
}

/// The advancer's 4-card sit quality gate override, if any
fn advance_sit_hcp_gate() -> Option<u8> {
    ADVANCE_SIT_HCP_GATE.with(Cell::get)
}

/// Toggle the advancer's **invitational minor jump** on the rich advance of a
/// takeout double for books built *after* this call (thread-local, read at
/// book-construction time)
///
/// **On by default**, and a no-op unless [`set_rich_advance_double`] is on. When
/// on, a three-level jump in a *minor* (`(1♥) X - 3♣`, `(1♠) X - 3♦`, …)
/// shows an invitational one-suiter — a real 5-card suit, 10–12, **denying a
/// 4-card unbid major** (with one the advancer cues opener's suit to find the
/// 4-4 major fit).  It ranks *below* the notrump ladder, so a stopper still
/// prefers `1NT`/`2NT`/`3NT`; the jump is the residual for the no-stopper shapely
/// invite that would otherwise have to cue.  Game-forcing minors (13+) are capped
/// out and still cue or bid a stopped `3NT`.  The doubler, strong but stopperless,
/// re-asks for a stopper by cueing their suit (a Western cue); the advancer bids
/// the right-sided `3NT` with a stopper, else the minor game.  Two-seed A/B: SIG+
/// in all four cells (plain ≥ PD → constructive).  Turn off with
/// `bba-gen --no-ns-advance-minor-jump`.
pub fn set_advance_minor_jump(on: bool) {
    ADVANCE_MINOR_JUMP.with(|cell| cell.set(on));
}

/// Whether the invitational minor jump is currently authored
fn advance_minor_jump_enabled() -> bool {
    ADVANCE_MINOR_JUMP.with(Cell::get)
}

/// Toggle the doubler's **accept/decline of the advancer's invitational `2NT`**
/// on the rich advance of a takeout double for books built *after* this call
/// (thread-local, read at book-construction time)
///
/// **On by default**, and a no-op unless [`set_rich_advance_double`] is on. The
/// advancer's `2NT` (`(1t) X - 2NT`) is a limited balanced 11–12 invite with a
/// stopper, but with no authored continuation the doubler falls to the instinct
/// floor, which treats `2NT` as non-forcing and *passes it even holding a game*.
/// When on, the doubler answers the invite naturally: **Pass** declines with a
/// minimum, **`3NT`** accepts to play, and a **new 5-card major** accepts
/// game-forcing so the advancer can pick the 4-4/5-3 major game.  Fixing this
/// floor-pass measured wash-positive on all four cells (NV/vul × plain/PD),
/// which earns the default-on flip.  Off-switch `bba-gen
/// --no-ns-advance-2nt-continuation`.
pub fn set_advance_2nt_continuation(on: bool) {
    ADVANCE_2NT_CONTINUATION.with(|cell| cell.set(on));
}

/// Whether the doubler's answer to the advancer's `2NT` invite is authored
fn advance_2nt_continuation_enabled() -> bool {
    ADVANCE_2NT_CONTINUATION.with(Cell::get)
}

/// Rich advance of partner's takeout double of a one-of-a-suit `their_opening`
/// (`(1t) X - ?`), gated by [`set_rich_advance_double`]
///
/// The flat [`advance_double`] ladder gives the advancer only a cheapest natural
/// suit, a `3NT`, and a penalty pass — so the whole 10+ invitational-and-up
/// band collapses into "bid your cheapest suit," flat, with no way to invite or
/// force.  This adds the missing structure:
///
/// - **cue of opener's suit** (`2t`) — *invitational-or-better*, forcing one
///   round: the residual for any 10+ hand with no natural limited bid (a
///   4-card major seeking the fit, a stopperless hand, a slam try).  Advancer
///   then clarifies — simple rebid = invite, jump = game force
///   ([`advance_cue_rebid`]).  *Artificial* (`ADVANCE_CUE`); `hcp(10..)`.
/// - **natural notrump ladder** — `1NT` 8–10, `2NT` 11–12 balanced, `3NT`
///   limited 13–17, each with a stopper in their suit.
/// - **new-suit jumps** — a *major* two-level jump is *constructive* (8–10, 4+)
///   and a three-level jump is *invitational* (10–12, 5+); a *minor* three-level
///   jump (only under [`set_advance_minor_jump`]) is an invitational one-suiter
///   (10–12, 5+) ranked below the notrump ladder and denying a 4-card unbid
///   major.  The cheapest new suit is natural weak (0–7, 4+).
/// - **major game jump** (`4M`, 5+) — always *limited* (slam tries cue):
///   two-way (shapely-weak or minimum game force, 11–15 points) when no Rubens
///   transfer exists, or purely preemptive (0–10) when a transfer carries the
///   strong hands.
/// - **forced 3-card suit** when broke with no 4-card suit outside their suit —
///   a takeout double cannot be passed for want of a bid; the cheapest such
///   bid, keeping the forced auction low.
/// - **penalty pass** with a trump stack (5+ of their suit, or 4 with two top
///   honors — a swept `suit_hcp` floor under [`set_advance_sit_hcp_gate`]).
#[must_use]
pub(super) fn advance_double_rich(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let t = theirs.suit().expect("their opening is always a suit bid");
    let level = their_opening.level.get();
    let cue = Bid::new(level + 1, theirs);

    // Penalty pass: a trump stack sits for the double — 5+ of their suit
    // (length alone is enough to convert), or 4 with two top honors (under
    // `set_advance_sit_hcp_gate`, a swept `suit_hcp` floor instead).  A weak
    // 5-card holding in their suit passes rather than being forced into a
    // three-card minor that the field doubles at the game level.  Under
    // `set_advance_pass_yield_major`, a hand below the 10+ cue band holding a
    // 4+ unbid major bids the ladder instead of sitting.
    fn sit_pass(t: Suit, quality: Cons<impl Constraint + Clone + 'static>) -> Rules {
        let sit = len(t, 5..) | (len(t, 4..) & quality);
        if advance_pass_yield_major_enabled() {
            Rules::new().rule(Call::Pass, 160, sit & (hcp(10..) | no_unbid_major(t)))
        } else {
            Rules::new().rule(Call::Pass, 160, sit)
        }
    }
    let mut rules = match advance_sit_hcp_gate() {
        Some(gate) => sit_pass(t, suit_hcp(t, gate..)),
        None => sit_pass(t, top_honors(t, 2..)),
    };

    // Cue of opener's suit — *invitational-or-better*, forcing for one round
    // (the standard advancer force).  It is the residual for any 10+ hand with
    // no natural limited bid to name — a 4-card-major invite/force seeking the
    // fit, a stopperless hand, a two-suiter, or a slam try.  Deliberately the
    // *lowest-weighted* action above the weak natural suit (1.0): every specific
    // limited bid (a jump, a notrump, a game) outranks it, so only the genuinely
    // shapeless invite-or-better lands here.  The advancer then clarifies —
    // simple rebid = invite (partner may pass), jump = game force
    // ([`advance_cue_rebid`]).  One rule (M6.2d).  Artificial → `ADVANCE_CUE`.
    rules = rules.rule(cue, 105, hcp(10..)).alert(ADVANCE_CUE);

    rules = rules
        // 3NT to play: a *limited* balanced-ish game (13–17) with a stopper and
        // no five-card major.  Bigger hands cue (slam try); shapelier ones bid
        // the suit.  Weighted just over the cue so a clear 3NT is not diverted.
        .rule(
            Bid::new(3, Strain::Notrump),
            145,
            hcp(13..=17) & stopper_in_their_suits(),
        )
        // 2NT: invitational (11–12) balanced with a stopper — almost denies a
        // 4-card major, which would have cued.
        .rule(
            Bid::new(level + 1, Strain::Notrump),
            115,
            hcp(11..=12) & balanced() & stopper_in_their_suits(),
        )
        // Natural 1NT: 8–10 with a stopper — the same invitational band as the
        // two-level constructive suit jump, offered in notrump.
        .rule(
            Bid::new(level, Strain::Notrump),
            110,
            hcp(8..=10) & stopper_in_their_suits(),
        )
        // Final fallback.
        .rule(Call::Pass, 0, hcp(0..));

    // Majors that a Rubens transfer can reach (only when the transfer layer is
    // on).  For these the strong hands transfer, so the direct `4M` jump is
    // freed up to be purely preemptive; for the rest `4M` is the limited game
    // force.  Over `(1♠)` hearts is *not* here (it sits below the jump-cue), so
    // The direct 4♥ advance over 1♠ doubled stays the minimum game force.
    let transfer_majors: Vec<Suit> = if advance_rubens_enabled() {
        advance_major_transfers(theirs)
            .into_iter()
            .map(|(_, target)| target)
            .collect()
    } else {
        Vec::new()
    };

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain == theirs {
            continue;
        }
        let bid_level = if strain > theirs { level } else { level + 1 };
        if longest_first_advance_enabled() {
            // Natural advance at the cheapest legal level (weak, 0–7): the
            // longest unbid suit, an equal-length tie to the higher rank.
            rules = natural_advance(rules, t, suit, bid_level, 100, 4);
            // Forced 3-card suit: a takeout double cannot be passed for want of
            // a bid — but with no 4-card suit outside their suit the priority
            // flips from highest-ranking to **cheapest bid**, keeping the
            // forced auction as low as possible.  No HCP cap — the
            // higher-weight cue, notrump, jump, and pass rules take every hand
            // with a better call, leaving only the genuinely stuck ones here.
            rules = rules.rule(
                Bid::new(bid_level, strain),
                30,
                len(suit, 3..) & cheapest_forced(suit, t, level),
            );
        } else {
            // Natural advance at the cheapest legal level (weak, 0–7).
            rules = natural_advance(rules, t, suit, bid_level, 100, 4);
            // Forced 3-card suit: a takeout double cannot be passed for want of
            // a bid, so any hand with no 4-card suit and no notrump/cue home
            // still introduces its highest-ranking 3-card suit (no HCP cap —
            // the higher-weight cue, notrump, and 4-card-suit rules take every
            // hand that has a better call, leaving only the genuinely stuck
            // ones here).
            rules = natural_advance(rules, t, suit, bid_level, 30, 3);
        }
        // Jump in a new *major*: a cheap two-level jump is *constructive*
        // (8–10, 4+); the more committal three-level jump is *invitational* and
        // wants a real 5-card suit.  (A game-forcing hand cues or blasts `4M` —
        // see below — so both are capped.)
        let jump = bid_level + 1;
        if matches!(suit, Suit::Hearts | Suit::Spades) {
            if jump == 2 {
                rules = rules.rule(Bid::new(2, strain), 120, hcp(8..=10) & len(suit, 4..));
            } else if jump == 3 {
                rules = rules.rule(Bid::new(3, strain), 125, hcp(10..=12) & len(suit, 5..));
            }
        } else if jump == 3 && advance_minor_jump_enabled() {
            // Three-level jump in a *minor* — an invitational one-suiter (5+,
            // 10–12) that DENIES a 4-card unbid major: with one the advancer cues
            // opener's suit to find the 4-4 major fit rather than burying it under
            // the minor.  It does *not* deny a stopper — the rule carries no
            // stopper term; it is simply weighted *below* the notrump ladder, so
            // a hand that fits a natural notrump invite (balanced, in the
            // `1NT`/`2NT` band) prefers that, while a shapely hand outside the
            // notrump band (a 6-card minor, a stiff) still jumps.  The 10–12 cap
            // keeps game-forcing minors cueing or bidding `3NT`, so — unlike the
            // old high-weighted minor jump — it never abandons a makeable game.
            // The doubler then accepts or declines ([`answer_advance_minor_jump`]).
            // At most three in each *unbid* major (opener's own major is not
            // constrained — `..=13` is vacuously true).
            let no_unbid_major = len(
                Suit::Hearts,
                ..=if theirs == Strain::Hearts { 13 } else { 3 },
            ) & len(
                Suit::Spades,
                ..=if theirs == Strain::Spades { 13 } else { 3 },
            );
            rules = rules.rule(
                Bid::new(3, strain),
                108,
                hcp(10..=12) & len(suit, 5..) & no_unbid_major,
            );
        }
        // Major-suit game jump `4M` (5+ — a 4-card major cues to check the fit).
        // A game jump is always *limited*: slam tries cue.  When a Rubens
        // transfer carries the strong hands it is purely preemptive (weak, long
        // trumps).  Without a transfer there is nowhere else for a shapely weak
        // hand to compete to game, so `4M` stays two-way — shapely-weak *or*
        // minimum game force — capped at 15 (points, distribution-aware) so slam
        // hands still cue.  Measured: the pure-MIN-FG gate stranded weak 6-card
        // majors below a makeable game (advance-double-v5, −0.005/bd DD).
        if matches!(suit, Suit::Hearts | Suit::Spades) {
            let bid = Bid::new(4, strain);
            rules = if transfer_majors.contains(&suit) {
                rules.rule(bid, 150, len(suit, 5..) & hcp(0..=10))
            } else {
                rules.rule(bid, 150, len(suit, 5..) & points(11..=15))
            };
        }
    }

    // Jump-cue Rubens transfers: a 5+ unbid major (invitational-or-better) shows
    // via a transfer one rank below it, so the doubler declares (right-siding).
    // Weighted above the cue and the game-blast so a 5+ major routes here.
    if advance_rubens_enabled() {
        for (bid, target) in advance_major_transfers(theirs) {
            rules = rules
                .rule(bid, 160, hcp(10..) & len(target, 5..))
                .alert(ADVANCE_TRANSFER);
        }
        // Over (1♠) the sole unbid major (hearts) sits *below* the jump-cue, so
        // there is nothing to transfer into: a 5-card heart hand is already shown
        // by the natural three-level `3♥` jump (invitational) in the suit loop
        // above, and a game-forcing one cues `2♠` or blasts `4♥`.
    }

    rules
}

/// The advancer's jump-cue major transfers over a one-of-`theirs` opening:
/// `(transfer bid, the 5+ unbid major it shows)`.  A transfer is the rank
/// immediately below its target major, at the three level.  Over `(1♠)` the sole
/// unbid major (hearts, `3♥`) is below the jump-cue (`3♠`), so it is shown by the
/// natural invitational `3♥` jump in [`advance_double_rich`] instead and is not
/// returned here.
fn advance_major_transfers(theirs: Strain) -> Vec<(Bid, Suit)> {
    if theirs == Strain::Spades {
        return Vec::new();
    }
    let mut out = Vec::new();
    for target in [Suit::Hearts, Suit::Spades] {
        if Strain::from(target) == theirs {
            continue;
        }
        let below = match target {
            Suit::Hearts => Suit::Diamonds,
            Suit::Spades => Suit::Hearts,
            _ => unreachable!("only hearts and spades are majors"),
        };
        out.push((Bid::new(3, Strain::from(below)), target));
    }
    out
}

/// Doubler's completion of the advancer's Rubens transfer
/// (`(1t) X - transfer { - | (X) } ?`, gated by [`set_advance_rubens`])
///
/// The transfer promised a 5+ `target` major; the doubler bids it (declaring —
/// the right-siding point), jumping to game (`4M`) with a maximum and support.
/// The completion is a finite catch-all so the artificial transfer is never
/// passed out.  Both bids are natural (`target`), so neither is alerted.
fn complete_advance_transfer(target: Suit) -> Rules {
    let strain = Strain::from(target);
    Rules::new()
        // Super-accept: maximum with support jumps to game.
        .rule(Bid::new(4, strain), 130, len(target, 4..) & points(15..))
        // Complete the transfer (always) — never pass the artificial call.
        .rule(Bid::new(3, strain), 100, hcp(0..))
}

/// Advancer's rebid after the doubler completed the transfer
/// (`(1t) X - transfer { - | (X) } 3M - ?`)
///
/// The transfer was invitational-or-better; opposite the doubler's minimum
/// completion a game-forcing advancer (12+) raises to game, an invitational one
/// (10–11) rests in the three-level partscore.
fn advance_transfer_rebid(target: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(target)), 100, hcp(12..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Doubler's answer to the advancer's cue (`(1t) X - cue - ?`, gated by
/// [`set_rich_advance_double`])
///
/// The cue ([`advance_double_rich`]) is invitational-or-better and forcing for
/// one round, asking the doubler to describe.  With a minimum the doubler bids
/// its cheapest 4-card unbid major (or the `2NT` catch-all); with extras (15+)
/// it jumps — `4M` with a major, `3NT` with a stopper.  The advancer then
/// clarifies invite-vs-force ([`advance_cue_rebid`]).  The `2NT` catch-all
/// guarantees a bid so the artificial cue is **never passed out**, which would
/// strand us declaring the opponents' suit (the M6.3 "passed-out cue" trap).
/// Every bid here is natural, so none is alerted.
fn answer_advance_cue(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let level = their_opening.level.get();

    let mut rules = Rules::new()
        // Extras and a stopper, no major to raise: 3NT to play.
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            hcp(15..) & stopper_in_their_suits(),
        )
        // Always-legal non-pass catch-all: never leave the artificial cue in.
        .rule(Bid::new(level + 1, Strain::Notrump), 20, hcp(0..));

    for major in [Suit::Hearts, Suit::Spades] {
        let m = Strain::from(major);
        if m == theirs {
            continue;
        }
        // The cheapest legal bid of the unbid major, above the cue at (level+1, theirs).
        let cheap = if m > theirs { level + 1 } else { level + 2 };
        // Show the 4-4 major fit: cheapest with a minimum, game with extras.
        rules = rules.rule(Bid::new(cheap, m), 130, len(major, 4..));
        rules = rules.rule(Bid::new(4, m), 150, len(major, 4..) & points(15..));
    }
    rules
}

/// The doubler's *non-game* answers to the advancer's cue — the ones over which
/// the advancer still has to clarify invite-vs-force ([`advance_cue_rebid`]).
///
/// These are exactly the minimum descriptions from [`answer_advance_cue`]: the
/// cheapest bid of each unbid major and the `2NT` catch-all.  (A `3NT`/`4M`
/// answer is already game — the advancer passes it or moves toward slam, which
/// the floor handles.)
fn advance_cue_answers(their_opening: Bid) -> Vec<Bid> {
    let theirs = their_opening.strain;
    let level = their_opening.level.get();
    let mut out = vec![Bid::new(level + 1, Strain::Notrump)];
    for major in [Suit::Hearts, Suit::Spades] {
        let m = Strain::from(major);
        if m == theirs {
            continue;
        }
        let cheap = if m > theirs { level + 1 } else { level + 2 };
        out.push(Bid::new(cheap, m));
    }
    out
}

/// Advancer's clarifying rebid after the cue and the doubler's minimum `answer`
/// (`(1t) X - cue { - | (X) } answer { - | (X) } ?`, gated by
/// [`set_rich_advance_double`])
///
/// The cue was invitational-or-better ([`advance_double_rich`]); here the
/// advancer resolves it against the doubler's minimum.  A *game-forcing* advancer
/// (13+) must reach game: raise the doubler's shown major with support, else bid
/// `3NT` (a stopper preferred, but forced even without — the game is on).  An
/// *invitational* advancer (10–12) has heard a minimum and stops (`Pass`).  This
/// is the "simple rebid = invite, jump = force" split, authored so a game force
/// cannot stall below game (the cue projects only `hcp(10..)`, so the floor
/// alone could read it as a mere invite and pass out).
fn advance_cue_rebid(answer: Bid) -> Rules {
    let mut rules = Rules::new();
    // Game force with a fit: raise the doubler's suit to game.
    if let Some(s) = answer.strain.suit() {
        rules = rules.rule(Bid::new(4, answer.strain), 100, len(s, 3..) & hcp(13..));
    }
    rules
        // Game force, no raise: notrump game (stopper preferred, else a punt).
        .rule(
            Bid::new(3, Strain::Notrump),
            60,
            hcp(13..) & stopper_in_their_suits(),
        )
        .rule(Bid::new(3, Strain::Notrump), 20, hcp(13..))
        // Invitational: partner showed a minimum — stop.
        .rule(Call::Pass, 0, hcp(0..))
}

/// Doubler's accept-or-decline of the advancer's invitational minor jump
/// (`(1t) X - 3m { - | (X) } ?`, gated by [`set_advance_minor_jump`])
///
/// The jump is a *limited* natural invite (10–12, 5+ `minor`, no 4-card unbid
/// major) that does **not** promise a stopper, so — unlike the forcing cue,
/// which the doubler may never pass — the continuation is a natural-invite
/// accept/decline: **Pass** declines (too weak for game), a **new 5+ suit**
/// accepts game-forcing (the advancer places it — [`advance_minor_jump_rebid`]),
/// and **`3NT`** accepts to play *with the doubler's own stopper*.  With game
/// values but **no** stopper and no biddable side suit the doubler instead
/// **cues their suit** — a Western stopper-ask; the advancer supplies the
/// notrump from its side ([`advance_minor_stopper_ask_answer`]), right-siding
/// `3NT` when it holds the stopper.  The cue is the only artificial call here
/// (`ADVANCE_CUE`); the rest are natural.
fn answer_advance_minor_jump(their_opening: Bid, minor: Suit) -> Rules {
    let theirs = their_opening.strain;
    let m = Strain::from(minor);
    let mut rules = Rules::new()
        // Accept to play: 3NT with values and a stopper.
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            hcp(15..) & stopper_in_their_suits(),
        )
        // Too weak for game: decline (the invite is limited, so Pass is safe).
        .rule(Call::Pass, 0, hcp(0..));
    // Accept by showing a new 5+ suit (game-forcing) — any unbid suit above the
    // jump, biddable at the three level.
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let s = Strain::from(suit);
        if s == theirs || s <= m {
            continue;
        }
        rules = rules.rule(Bid::new(3, s), 130, points(15..) & len(suit, 5..));
    }
    // Game values but no stopper and no 5-card side suit: cue their suit to ask
    // the advancer for the stopper (a Western cue).  Lowest-weighted of the game
    // tries, so a hand with its own stopper (`3NT`) or a biddable side suit (a
    // new suit) is routed there first; only the shapeless stopperless 15+ lands
    // here.  Always legal — the minor jump exists only *below* their suit, so
    // 3-of-their-suit sits above `3m` and below `3NT`.  Artificial → `ADVANCE_CUE`.
    rules = rules
        .rule(Bid::new(3, theirs), 100, hcp(15..))
        .alert(ADVANCE_CUE);
    rules
}

/// Advancer's placement after the doubler accepts the minor jump with a forcing
/// new suit (`(1t) X - 3m { - | (X) } 3S { - | (X) } ?`, gated by
/// [`set_advance_minor_jump`])
///
/// The doubler forced to game showing a 5+ `shown` suit; the advancer (already
/// limited to 10–12) places it: raise to game with three-card support, else
/// `3NT` (a stopper preferred, but the game is on either way).
fn advance_minor_jump_rebid(shown: Suit) -> Rules {
    let s = Strain::from(shown);
    let game = if matches!(shown, Suit::Hearts | Suit::Spades) {
        4
    } else {
        5
    };
    Rules::new()
        // Support: raise the doubler's suit to game.
        .rule(Bid::new(game, s), 100, len(shown, 3..))
        // No support: notrump game (stopper preferred, else forced — game is on).
        .rule(Bid::new(3, Strain::Notrump), 60, stopper_in_their_suits())
        .rule(Bid::new(3, Strain::Notrump), 20, hcp(0..))
}

/// Advancer's answer to the doubler's stopper-ask cue after the minor jump
/// (`(1t) X - 3m { - | (X) } 3t { - | (X) } ?`, gated by [`set_advance_minor_jump`])
///
/// The doubler cued their suit holding game values but no stopper (and no 5-card
/// side suit); the advancer supplies the notrump decision.  With a stopper the
/// advancer bids **`3NT`** — right-siding it, so the opening lead runs up to the
/// advancer's tenace — otherwise no stopper sits on either side, so the advancer
/// signs off in the **minor game** (both hands have shown game values).  Natural;
/// nothing to alert.
fn advance_minor_stopper_ask_answer(minor: Suit) -> Rules {
    let m = Strain::from(minor);
    Rules::new()
        // Stopper: the right-sided notrump game (the lead comes up to us).
        .rule(Bid::new(3, Strain::Notrump), 130, stopper_in_their_suits())
        // No stopper anywhere: play the minor game (game values are established).
        .rule(Bid::new(5, m), 50, hcp(0..))
}

/// Doubler's accept-or-decline of the advancer's invitational `2NT`
/// (`(1t) X - 2NT { - | (X) } ?`, gated by [`set_advance_2nt_continuation`])
///
/// The `2NT` invite is a limited balanced 11–12 with a stopper (the advancer
/// supplies the notrump stopper), so the doubler — sitting on the wide takeout
/// range — simply answers a natural invite: **Pass** declines with a minimum,
/// **`3NT`** accepts to play, and a **new 5-card major** accepts game-forcing so
/// the advancer can choose the 4-4/5-3 major game over `3NT` (the advancer places
/// it — [`advance_minor_jump_rebid`], the same accept-a-forcing-suit logic).  A
/// 5-card *minor* is not shown: with the advancer's stopper `3NT` is almost
/// always right, so only the fit-seeking majors are worth the detour.  All
/// natural; nothing artificial to alert.
fn answer_advance_2nt(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let mut rules = Rules::new()
        // Accept to play: 3NT with a maximum (the advancer holds the stopper).
        .rule(Bid::new(3, Strain::Notrump), 120, hcp(14..))
        // Minimum: decline the invite, play 2NT.
        .rule(Call::Pass, 0, hcp(0..));
    // Accept game-forcing by showing a 5-card major to seek the fit.
    for major in [Suit::Hearts, Suit::Spades] {
        let s = Strain::from(major);
        if s == theirs {
            continue;
        }
        rules = rules.rule(Bid::new(3, s), 130, points(14..) & len(major, 5..));
    }
    rules
}

/// Continuations of the rich advance of partner's takeout double
///
/// Four sub-ladders, each authored for both RHO branches — RHO may pass *or*
/// double our artificial call, and the obligation to answer is the same either
/// way (leaving the doubled branch to the floor lets it pass out an artificial
/// cue):
///
/// * the doubler's answer to the advancer's cue, then the advancer's
///   invite-vs-force clarification over each minimum answer,
/// * the Rubens transfer completion and the advancer's rebid over it
///   ([`set_advance_rubens`]),
/// * the doubler's accept/decline of the invitational minor jump, the
///   advancer's placement over the forcing new suit, and the answer to the
///   stopper-ask cue ([`set_advance_minor_jump`]),
/// * the same accept/decline over the invitational `2NT`
///   ([`set_advance_2nt_continuation`]) — without it the doubler falls to the
///   floor, which passes `2NT` holding a game.
pub(super) fn rich_advance_double_package() -> Package {
    Package {
        name: "rich-advance-of-double",
        gate: rich_advance_double_enabled,
        entries: || {
            let mut entries = Vec::new();
            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let theirs = Strain::from(suit);
                let opening = Bid::new(1, theirs);
                let base = format!("P* ({opening}) X -");

                let cue = Bid::new(2, theirs);
                for rho in ["-", "(X)"] {
                    let after_cue = format!("{base} {cue} {rho}");
                    entries.extend(rows_of(
                        Pattern::node(&after_cue),
                        answer_advance_cue(opening),
                    ));
                    for answer in advance_cue_answers(opening) {
                        for rho2 in ["-", "(X)"] {
                            entries.extend(rows_of(
                                Pattern::node(&format!("{after_cue} {answer} {rho2}")),
                                advance_cue_rebid(answer),
                            ));
                        }
                    }
                }

                // Rubens transfers: the doubler completes the transfer
                // (declaring), and the advancer raises to game or rests over the
                // completion — so the artificial transfer is never left in.
                if advance_rubens_enabled() {
                    for (bid, target) in advance_major_transfers(theirs) {
                        let completion = Bid::new(3, Strain::from(target));
                        for rho in ["-", "(X)"] {
                            let after_xfer = format!("{base} {bid} {rho}");
                            entries.extend(rows_of(
                                Pattern::node(&after_xfer),
                                complete_advance_transfer(target),
                            ));
                            entries.extend(rows_of(
                                Pattern::node(&format!("{after_xfer} {completion} -")),
                                advance_transfer_rebid(target),
                            ));
                        }
                    }
                }

                // The natural jump is limited, so — like a `2NT` invite — the
                // doubler passes to decline; only the accepting branches (and the
                // advancer's rebid over them) need authoring.
                if advance_minor_jump_enabled() {
                    for minor in [Suit::Clubs, Suit::Diamonds] {
                        let m = Strain::from(minor);
                        // A three-level minor jump exists only below their suit.
                        if m >= theirs {
                            continue;
                        }
                        let jump = Bid::new(3, m);
                        for rho in ["-", "(X)"] {
                            let after_jump = format!("{base} {jump} {rho}");
                            entries.extend(rows_of(
                                Pattern::node(&after_jump),
                                answer_advance_minor_jump(opening, minor),
                            ));
                            // The advancer places game over each forcing new suit
                            // the doubler can show (any unbid suit above the jump).
                            for shown in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                                let s = Strain::from(shown);
                                if s == theirs || s <= m {
                                    continue;
                                }
                                let bid = Bid::new(3, s);
                                for rho2 in ["-", "(X)"] {
                                    entries.extend(rows_of(
                                        Pattern::node(&format!("{after_jump} {bid} {rho2}")),
                                        advance_minor_jump_rebid(shown),
                                    ));
                                }
                            }
                            // The advancer answers the doubler's stopper-ask cue
                            // (3 of their suit): 3NT with a stopper (right-sided),
                            // else the minor game.
                            let ask = Bid::new(3, theirs);
                            for rho2 in ["-", "(X)"] {
                                entries.extend(rows_of(
                                    Pattern::node(&format!("{after_jump} {ask} {rho2}")),
                                    advance_minor_stopper_ask_answer(minor),
                                ));
                            }
                        }
                    }
                }

                if advance_2nt_continuation_enabled() {
                    for rho in ["-", "(X)"] {
                        let after_2nt = format!("{base} 2NT {rho}");
                        entries.extend(rows_of(
                            Pattern::node(&after_2nt),
                            answer_advance_2nt(opening),
                        ));
                        // The advancer places game over each forcing major the
                        // doubler can show (an unbid major at the three level).
                        for major in [Suit::Hearts, Suit::Spades] {
                            let s = Strain::from(major);
                            if s == theirs {
                                continue;
                            }
                            let bid = Bid::new(3, s);
                            for rho2 in ["-", "(X)"] {
                                entries.extend(rows_of(
                                    Pattern::node(&format!("{after_2nt} {bid} {rho2}")),
                                    advance_minor_jump_rebid(major),
                                ));
                            }
                        }
                    }
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
