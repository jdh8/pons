//! Dutch's Multi `2♦` — Phase 3's Multi slice, in **two variants**
//!
//! One artificial opening replaces all three natural weak twos: `2♦!` is 4–10
//! HCP with exactly one six-card major, weak only, never in fourth seat
//! (`dutch::openings`, gated on `opening.multi_two_diamonds`).  This module is
//! everything after it — responder's table, opener's answers, and the
//! interfered tails.
//!
//! # Why two variants
//!
//! * **Base** ([`multi_two_diamonds`][crate::bidding::agreements::OpeningKnobs::multi_two_diamonds]
//!   alone) is **BBA's Multi book copied verbatim**, walked in
//!   `docs/ai-bidder/bba-multi-2d-opening.md`.  Copying buys two things a better
//!   table would cost: the WJ teacher net that floors Dutch's divergent subtrees
//!   was trained on these rows, and BBA — the anchor we measure against — reads
//!   our calls through its own book, so a verbatim lane is one BBA cannot
//!   misread.
//! * **Champion**
//!   ([`multi_two_diamonds_champion`][crate::bidding::agreements::OpeningKnobs::multi_two_diamonds_champion])
//!   is jdh8's own spec (<https://polish.club/2D.html>): the pass-or-correct
//!   ladder runs all the way to the three level, the ask is invitational rather
//!   than 16+, and the minors are natural forcing rather than one natural
//!   to-play bid and one artificial try.  It is **not `.bbsa`-expressible** —
//!   BBA reads a champion `3♥` as natural hearts — which is exactly why the
//!   verbatim base stays and stays pinned for anchor runs.
//!
//! The two share everything below responder's first call: the `2NT` ask ladder
//! (**max-first** — `3♣`/`3♦` are the 8–10 answers, `3♥`/`3♠` the 4–7 ones; both
//! BBA and the champion page answer this way, so there is no direction conflict
//! with the teacher), the `4♣` transfer machinery, the pass-or-correct rebids,
//! and every interfered tail.
//!
//! # What BBA's book does *not* say
//!
//! Three nodes came back as generic templates rather than rules when the walk
//! was re-run for this module (`probe-bba-book --conv "Multi=1"`), so they are
//! **ours**, not copies, in both variants: opener's rebid over the constructive
//! `2♠` (only the escape template `6+ <suit>` and a `P` = six spades),
//! responder's continuation after either ask answer (`bidable suit` at every
//! rung), and the responder table's own weight order — BBA states bands, not
//! precedence.
//!
//! The `2♦ (2♠)` hole is **inherited on purpose**: responder's table over their
//! spade overcall has no weak rung, so a weak responder passes them out in `2♠`.
//! Repairing it diverges from the teacher, so it is its own A/B — see the ledger
//! row in `docs/dutch-system.md`.

use crate::bidding::agreements::Agreements;
use crate::bidding::constraint::{and, hcp, len, or, points, stopper_in};
use crate::bidding::fallback::ReplaceNext;
use crate::bidding::rows::{Entry, Package, Pattern, rebase, rows_of};
use crate::bidding::{Alert, Rules};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// The Multi `2♦` opening itself — 4–10, one six-card major
pub(super) const MULTI_2D: Alert = Alert("dutch-multi:2d");
/// A pass-or-correct call — `2♥`, `2♠`, `4♦`, and the champion's `3♥`/`3♠`
/// sibling below carries its own tag
const PASS_OR_CORRECT: Alert = Alert("dutch-multi:pass-correct");
/// The `2NT` ask (and, in the base, its `XX` twin over their double)
const ASK: Alert = Alert("dutch-multi:ask");
/// One of the four answers to the ask
///
/// All four are alerted, the minimum answers included.  `3♥`/`3♠` name the real
/// suit, so they are not artificial — but the node they sit on is american's
/// Ogust key, and an unalerted call there decodes through the *hardcoded*
/// american reading rather than through its own projection.  The alert is what
/// routes it to projection; the Ogust answers are alerted for the same reason.
const ASK_ANSWER: Alert = Alert("dutch-multi:ask-answer");
/// The base's artificial `3♦` three-level try (10+, both majors 2+)
const TRY: Alert = Alert("dutch-multi:try");
/// The 15+ `4♣` ask, and the transfer answers it gets
const STRONG_ASK: Alert = Alert("dutch-multi:strong-ask");
/// Opener's `4♦`/`4♥` transfer answer to `4♣`, so the 15+ hand declares
const TRANSFER_ANSWER: Alert = Alert("dutch-multi:transfer-answer");
/// The cue-shaped limit-raise-or-better in the *other* major, over their overcall
const LIMIT_CUE: Alert = Alert("dutch-multi:limit-cue");
/// `2NT` as support for the other major over their overcall — a raise, not notrump
const SUPPORT: Alert = Alert("dutch-multi:support");
/// The champion's competitive `3♥`/`3♠` pass-or-correct
const PC_THREE: Alert = Alert("dutch-multi:pc-three");
/// The champion's `XX` over their double: name the major you do **not** hold
const WORSE_MAJOR: Alert = Alert("dutch-multi:worse-major");

/// Whether the champion structure is live (implies the Multi gate)
fn champion(agreements: &Agreements) -> bool {
    agreements.opening.multi_two_diamonds_champion
}

// ---------------------------------------------------------------------------
// Responder's first call
// ---------------------------------------------------------------------------

/// Responder over the Multi `2♦` — the one table the two variants disagree on
///
/// **Base** (`docs/ai-bidder/bba-multi-2d-opening.md` §2): `2♥` pass-or-correct
/// with no point floor, `2♠` pass-or-correct 12–17, `2NT` the 16+ ask, `3♣` and
/// `3♥`/`3♠` natural seven-card to-play hands at 10–14, `3♦` the artificial
/// three-level try, `4♣` the 15+ transfer ask, `4♦` the ≤14 pass-or-correct to
/// game, `4♥`/`4♠` natural, and `P` with a six-card diamond suit.
///
/// BBA declares the bands but **no precedence**, and several overlap, so the
/// weight order is ours: strongest and most specific first, the cheap
/// pass-or-correct last.  It resolves the overlaps the way the bridge reads —
/// 16+ asks rather than blasting `4♣`, a seven-card suit outranks the
/// pass-or-correct that would bury it, `2♠`'s 12–17 outranks the `3♦` try so the
/// try keeps the 10–11 slice that has no `2♠` rung.
///
/// **Champion** (<https://polish.club/2D.html>) rebuilds the ladder around
/// pass-or-correct at every level: `2♥` weak, `2♠` the constructive rung (heart
/// tolerance, below invitational), `3♥`/`3♠` the competitive rungs (`3♥` passes
/// or corrects to `3♠`, `3♠` passes or forces `4♥` — so `3♠` needs the fourth
/// heart), `4♦` the ten-card-fit blast, `2NT` an invitational-or-better ask,
/// `3♣`/`3♦` natural **forcing**, and the natural `4M` floor widened to 10+ to
/// re-house the seven-card 10–14 hands the base parks on `3♥`/`3♠`.
fn responses(agreements: &Agreements) -> Rules {
    let majors = [Suit::Hearts, Suit::Spades];
    if champion(agreements) {
        return Rules::new()
            // 3♣ / 3♦ — natural and forcing, six cards and real values.  They
            // outrank the `4♣` ask below: a hand with its own six-card suit
            // wants to name it, and leaving `4♣` on top would swallow every
            // 15+ holding one, which is most of them.
            .rule(
                Bid::new(3, Strain::Clubs),
                175,
                len(Suit::Clubs, 6..) & hcp(13..),
            )
            .rule(
                Bid::new(3, Strain::Diamonds),
                175,
                len(Suit::Diamonds, 6..) & hcp(13..),
            )
            // 4♣ — strong choice of games, by strength.  The rule is the base's
            // verbatim; only the rows around it move.
            .rule(
                Bid::new(4, Strain::Clubs),
                170,
                hcp(15..) & and(majors, 2..),
            )
            .alert(STRONG_ASK)
            // 2NT — the ask, invitational-or-better rather than the base's 16+.
            .rule(Bid::new(2, Strain::Notrump), 150, hcp(13..))
            .alert(ASK)
            // 4M natural — the floor drops to 10+, taking the seven-card hands
            // the base bids `3♥`/`3♠` with (that rung is a pass-or-correct here).
            .rule(
                Bid::new(4, Strain::Hearts),
                145,
                len(Suit::Hearts, 7..) & hcp(10..),
            )
            .rule(
                Bid::new(4, Strain::Spades),
                145,
                len(Suit::Spades, 7..) & hcp(10..),
            )
            // 4♦ — preemptive choice of games, by distribution: 4-4 in the
            // majors is a ten-card fit opposite either six-bagger.
            .rule(
                Bid::new(4, Strain::Diamonds),
                128,
                and(majors, 4..) & hcp(..=12),
            )
            .alert(PASS_OR_CORRECT)
            // 3♠ — pass-or-correct that *forces* 4♥, so it needs the fourth heart.
            .rule(
                Bid::new(3, Strain::Spades),
                125,
                len(Suit::Hearts, 4..) & len(Suit::Spades, 3..) & hcp(..=12),
            )
            .alert(PC_THREE)
            // 3♥ — pass-or-correct to `3♠`: three-card support either way.
            .rule(
                Bid::new(3, Strain::Hearts),
                120,
                and(majors, 3..) & hcp(..=12),
            )
            .alert(PC_THREE)
            // 2♠ — the constructive rung: happy in `2♠`, safe in `3♥`.
            .rule(
                Bid::new(2, Strain::Spades),
                110,
                len(Suit::Hearts, 3..) & hcp(8..=12),
            )
            .alert(PASS_OR_CORRECT)
            // Pass — a six-card diamond suit plays `2♦` (BBA's `minimum, 6+♦`).
            .rule(Call::Pass, 95, len(Suit::Diamonds, 6..))
            // 2♥ — the weak pass-or-correct, and the near catch-all.
            .rule(Bid::new(2, Strain::Hearts), 90, hcp(..=12))
            .alert(PASS_OR_CORRECT)
            .rule(Call::Pass, 0, hcp(0..));
    }
    Rules::new()
        // 2NT — the ask, 16+.
        .rule(Bid::new(2, Strain::Notrump), 170, hcp(16..))
        .alert(ASK)
        // 4♣ — the 15+ transfer ask (so only 15 exactly survives the ask above).
        .rule(
            Bid::new(4, Strain::Clubs),
            165,
            hcp(15..) & and(majors, 2..),
        )
        .alert(STRONG_ASK)
        // 4♥ / 4♠ natural — a seven-card suit of one's own, 13+.
        .rule(
            Bid::new(4, Strain::Hearts),
            160,
            len(Suit::Hearts, 7..) & hcp(13..),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            160,
            len(Suit::Spades, 7..) & hcp(13..),
        )
        // 3♥ / 3♠ natural to play — seven cards, 10–14.
        .rule(
            Bid::new(3, Strain::Hearts),
            150,
            len(Suit::Hearts, 7..) & hcp(10..=14),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            150,
            len(Suit::Spades, 7..) & hcp(10..=14),
        )
        // 3♣ natural to play — seven clubs, 10–14.
        .rule(
            Bid::new(3, Strain::Clubs),
            145,
            len(Suit::Clubs, 7..) & hcp(10..=14),
        )
        // 2♠ — the constructive pass-or-correct, 12–17.
        .rule(
            Bid::new(2, Strain::Spades),
            130,
            hcp(12..=17) & len(Suit::Hearts, 2..) & len(Suit::Spades, 1..),
        )
        .alert(PASS_OR_CORRECT)
        // 3♦ — the artificial three-level try, 10+ with both majors held.
        .rule(
            Bid::new(3, Strain::Diamonds),
            120,
            hcp(10..) & and(majors, 2..),
        )
        .alert(TRY)
        // 4♦ — pass-or-correct to game, ≤14 with three-card support either way.
        .rule(
            Bid::new(4, Strain::Diamonds),
            110,
            hcp(..=14) & and(majors, 3..),
        )
        .alert(PASS_OR_CORRECT)
        // Pass — a six-card diamond suit plays `2♦`.
        .rule(Call::Pass, 95, len(Suit::Diamonds, 6..))
        // 2♥ — the weak pass-or-correct, no point floor.
        .rule(Bid::new(2, Strain::Hearts), 90, hcp(..=17))
        .alert(PASS_OR_CORRECT)
        .rule(Call::Pass, 0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Opener's answers — shared by both variants
// ---------------------------------------------------------------------------

/// Opener over the weak `2♥` pass-or-correct: pass with hearts, `2♠` with
/// spades, and jump to the three level with the 10-count maximum
fn opener_over_two_hearts() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Hearts),
            120,
            len(Suit::Hearts, 6..) & hcp(10..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            120,
            len(Suit::Spades, 6..) & hcp(10..),
        )
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 6..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener over their double of the weak `2♥` pass-or-correct
///
/// The undisturbed decision without the maximum jump: pass with hearts (a
/// doubled `2♥` on a known six-two is where we wanted to play anyway), correct
/// to `2♠` with spades.  Authored because the floor is bad here — a knobs-on
/// generation trace had it jumping to `4♥` on eight HCP and being doubled.
fn opener_over_two_hearts_doubled() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 6..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener over the constructive `2♠` pass-or-correct: pass with spades, `3♥`
/// with hearts
///
/// BBA's book has no rule here — the walk returns only its escape template
/// (`6+ <suit>` at every rung) plus `P` = six spades — so this table is ours.
/// Naming the suit rather than relaying keeps the rebid identical whether the
/// `2♠` was passed or doubled.
fn opener_over_two_spades() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 6..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's four answers to the `2NT` ask — **maximum first**
///
/// `3♣` = hearts with 8–10, `3♦` = spades with 8–10, `3♥` = hearts with 4–7,
/// `3♠` = spades with 4–7.  Many European Multi cards answer minimum-first;
/// BBA and the champion page both answer maximum-first, so this ladder is
/// shared and matches the teacher.
fn ask_answers() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Clubs),
            130,
            len(Suit::Hearts, 6..) & hcp(8..),
        )
        .alert(ASK_ANSWER)
        .rule(
            Bid::new(3, Strain::Diamonds),
            130,
            len(Suit::Spades, 6..) & hcp(8..),
        )
        .alert(ASK_ANSWER)
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 6..))
        .alert(ASK_ANSWER)
        .rule(Bid::new(3, Strain::Spades), 100, len(Suit::Spades, 6..))
        .alert(ASK_ANSWER)
}

/// Responder's placement after one ask answer
///
/// BBA leaves this to its floor (every rung reads as the generic `bidable
/// suit`), so the table is ours and deliberately small: raise the now-known
/// six-card major to game with a fit and the values the answer justifies, sign
/// off in it otherwise, and take `3NT` when the major is unplayable.  Over a
/// **maximum** the game needs 14+; over a **minimum** it needs 17+, and the
/// minimum answer is natural, so passing it is a real option.
fn asker_after_answer(major: Suit, maximum: bool) -> Rules {
    let strain = Strain::from(major);
    if maximum {
        return Rules::new()
            .rule(Bid::new(4, strain), 110, len(major, 2..) & hcp(14..))
            .rule(Bid::new(3, strain), 100, len(major, 2..))
            .rule(Bid::new(3, Strain::Notrump), 0, hcp(0..));
    }
    Rules::new()
        .rule(Bid::new(4, strain), 110, len(major, 2..) & hcp(17..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener over the base's artificial `3♦` try: name the major, jumping to game
/// with the maximum
fn opener_over_three_diamond_try() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Hearts),
            120,
            len(Suit::Hearts, 6..) & hcp(10..),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            120,
            len(Suit::Spades, 6..) & hcp(10..),
        )
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 6..))
        .rule(Bid::new(3, Strain::Spades), 100, len(Suit::Spades, 6..))
}

/// Opener over one of the champion's natural forcing minors: name the major,
/// or fall back on `3NT`
fn opener_over_forcing_minor() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 6..))
        .rule(Bid::new(3, Strain::Spades), 100, len(Suit::Spades, 6..))
        .rule(Bid::new(3, Strain::Notrump), 0, hcp(0..))
}

/// Opener over the `4♣` ask — a **transfer**, so the 15+ hand declares:
/// `4♦` = hearts, `4♥` = spades
fn opener_over_four_clubs() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Diamonds), 100, len(Suit::Hearts, 6..))
        .alert(TRANSFER_ANSWER)
        .rule(Bid::new(4, Strain::Hearts), 100, len(Suit::Spades, 6..))
        .alert(TRANSFER_ANSWER)
}

/// Opener over the `4♦` pass-or-correct: bid the six-card major, unalerted
fn opener_over_four_diamonds() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Hearts), 100, len(Suit::Hearts, 6..))
        .rule(Bid::new(4, Strain::Spades), 100, len(Suit::Spades, 6..))
}

/// One row that plays the contract where it sits — pins the floor off a node
/// the book has already finished
fn play_it() -> Rules {
    Rules::new().rule(Call::Pass, 0, hcp(0..))
}

/// Responder over opener's maximum jump: raise to game with a fit and values
fn raise_the_maximum(major: Suit, floor: u8) -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::from(major)),
            100,
            len(major, 2..) & hcp(floor..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's completion of opener's transfer answer: bid the major
fn complete_the_transfer(major: Suit) -> Rules {
    Rules::new().rule(Bid::new(4, Strain::from(major)), 100, hcp(0..))
}

// ---------------------------------------------------------------------------
// Interfered tails
// ---------------------------------------------------------------------------

/// Responder over their **double** — the uncontested table plus one redouble
///
/// Base: `XX` is the same 16+ artificial ask as `2NT`, so the answer ladder is
/// reused unchanged.  Champion: `XX` asks for the major opener does **not**
/// hold, which is the call a strong responder with a long major of his own
/// wants — if the answer names his suit he passes and plays it from the right
/// side, and if it does not he now knows opener's real major and places the
/// contract.  (Published precedent for the treatment: the BBO forum thread
/// *Expert standard responses to weak only multi*,
/// <https://www.bridgebase.com/forums/topic/75286->.)
///
/// The rest of the table rides unchanged, and everything below responder's
/// first call rides the rebase that strips their double off the subtree.
fn responses_doubled(agreements: &Agreements) -> Rules {
    if champion(agreements) {
        return responses(agreements)
            .rule(
                Call::Redouble,
                175,
                or([Suit::Hearts, Suit::Spades], 5..) & points(13..),
            )
            .alert(WORSE_MAJOR);
    }
    responses(agreements)
        .rule(Call::Redouble, 175, hcp(16..))
        .alert(ASK)
}

/// Opener's answer to the champion's `XX`: name the major you do **not** hold
fn worse_major_answers() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Spades, 6..))
        .alert(WORSE_MAJOR)
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Hearts, 6..))
        .alert(WORSE_MAJOR)
}

/// Opener after they double our redouble: the ask is off, name the real suit
///
/// Shared by both variants — in a doubled auction the escape outranks whatever
/// question the redouble was asking.
fn opener_escape() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Hearts, 6..))
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 6..))
}

/// Responder over their **suit overcall**, copied verbatim from BBA's book
///
/// Responder resolves the Multi to *the other* major, on the inference that the
/// overcaller bid the one they hold: `their` is the major they named, `ours` is
/// the one opener is now assumed to have.  Both columns are BBA's
/// (`docs/ai-bidder/bba-multi-2d-opening.md` §4), including the hole:
///
/// **Over `(2♠)` there is no weak rung.**  `2♥` is gone, so support starts at
/// `2NT` and needs 14+, and a weak responder's only call is `P` — leaving the
/// opponents in `2♠` whenever responder is weak, which is most of the time.
/// The repair (a weak `2NT` relay, or `X` as pass-or-correct) diverges from the
/// teacher net, so it is its own A/B rather than a free fix; the hole is pinned
/// by a test and carried as a ledger row in `docs/dutch-system.md`.
fn responses_overcalled(their: Suit) -> Rules {
    let ours = if their == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    let majors = [Suit::Hearts, Suit::Spades];
    let mut rules = Rules::new()
        // A cue of their suit: limit raise or better in ours.  It outranks the
        // `4♣` ask below — BBA states both bands and no precedence, and a hand
        // with support belongs in the raise ladder, not in a blind game choice.
        .rule(
            Bid::new(3, Strain::from(their)),
            165,
            len(ours, 2..) & hcp(17..),
        )
        .alert(LIMIT_CUE)
        // 4♣ — the same 15+ ask as in the uncontested table.
        .rule(
            Bid::new(4, Strain::Clubs),
            160,
            hcp(15..) & and(majors, 2..),
        )
        .alert(STRONG_ASK)
        // 3NT with their suit stopped.
        .rule(
            Bid::new(3, Strain::Notrump),
            140,
            hcp(20..) & stopper_in(their),
        )
        // 2NT is a *raise*, not notrump: support with 14+.
        .rule(
            Bid::new(2, Strain::Notrump),
            130,
            len(ours, 2..) & hcp(14..),
        )
        .alert(SUPPORT);
    if their == Suit::Hearts {
        rules = rules
            // Natural 3♠ (11+, six cards) — available only over `(2♥)`.
            .rule(
                Bid::new(3, Strain::Spades),
                120,
                len(Suit::Spades, 6..) & hcp(11..),
            )
            // Penalty double, short in the major opener is assumed to hold.
            .rule(Call::Double, 115, hcp(14..) & len(Suit::Spades, ..=1))
            .penalty()
            // The cheap support rung — the one `(2♠)` does not have.
            .rule(
                Bid::new(2, Strain::Spades),
                110,
                len(Suit::Spades, 2..) & hcp(8..),
            );
    } else {
        rules = rules
            // Natural 3♥, but only at BBA's 20+ — this is the hole's other half.
            .rule(
                Bid::new(3, Strain::Hearts),
                120,
                len(Suit::Hearts, 4..) & hcp(20..),
            )
            .rule(Call::Double, 115, hcp(14..) & len(Suit::Hearts, ..=1))
            .penalty()
            .rule(
                Bid::new(4, Strain::Hearts),
                112,
                len(Suit::Hearts, 7..) & hcp(13..),
            );
    }
    rules
        // 4♦ pass-or-correct rides through unchanged.
        .rule(
            Bid::new(4, Strain::Diamonds),
            105,
            hcp(..=14) & and(majors, 3..),
        )
        .alert(PASS_OR_CORRECT)
        .rule(Call::Pass, 0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// The Multi `2♦` structure as one gated row package
///
/// Compiled only by [`dutch::book`][super::book], after american's, so every
/// weak-two and Ogust key under `P* 2♦` that american authored is **re-owned**
/// here: `-`, `- 2♥ -`, `- 2♠ -`, `- 3♣ -`, `- 2NT -` and the four
/// `- 2NT - 3x -` continuations.  The `P* 2♥` / `P* 2♠` subtrees american
/// compiled stay in the book and go dead — under this gate we never open either.
pub(super) fn package() -> Package {
    Package {
        name: "dutch-multi-2d",
        gate: |agreements| agreements.opening.multi_two_diamonds,
        entries: |agreements| {
            const OPEN: &str = "P* 2♦";
            let mut entries: Vec<Entry> = rows_of(Pattern::node("P* 2♦ -"), responses(agreements));

            // Opener's rebid over each pass-or-correct rung.
            entries.extend(rows_of(
                Pattern::node("P* 2♦ - 2♥ -"),
                opener_over_two_hearts(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 2♦ - 2♠ -"),
                opener_over_two_spades(),
            ));
            // Their double *of* the pass-or-correct: opener's decision is the
            // undisturbed one, minus the maximum jump — a jump into a doubled
            // auction buys nothing and the trace showed the floor taking `4♥`
            // on eight HCP here.  The rebase above routes `2♦ (X) 2♥ (X)` onto
            // these same two keys.
            entries.extend(rows_of(
                Pattern::node("P* 2♦ - 2♥ (X)"),
                opener_over_two_hearts_doubled(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 2♦ - 2♠ (X)"),
                opener_over_two_spades(),
            ));
            // …and responder's placement after it.
            entries.extend(rows_of(Pattern::node("P* 2♦ - 2♥ - 2♠ -"), play_it()));
            entries.extend(rows_of(
                Pattern::node("P* 2♦ - 2♥ - 3♥ -"),
                raise_the_maximum(Suit::Hearts, 12),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 2♦ - 2♥ - 3♠ -"),
                raise_the_maximum(Suit::Spades, 12),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 2♦ - 2♠ - 3♥ -"),
                raise_the_maximum(Suit::Hearts, 15),
            ));

            // The 2NT ask and its four answers, then responder's placement.
            // The base's `XX` twin needs its own placement keys: the opening
            // rebase below cannot serve them, because stripping their double
            // out of `2♦ (X) XX - 3♣ -` leaves an illegal `2♦ - XX` auction
            // that resolves nowhere — and the floor would sit for opener's
            // phantom `3♣`.
            entries.extend(rows_of(Pattern::node("P* 2♦ - 2NT -"), ask_answers()));
            for (answer, major, maximum) in [
                ("3♣", Suit::Hearts, true),
                ("3♦", Suit::Spades, true),
                ("3♥", Suit::Hearts, false),
                ("3♠", Suit::Spades, false),
            ] {
                entries.extend(rows_of(
                    Pattern::node(&format!("P* 2♦ - 2NT - {answer} -")),
                    asker_after_answer(major, maximum),
                ));
                if !champion(agreements) {
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* 2♦ (X) XX - {answer} -")),
                        asker_after_answer(major, maximum),
                    ));
                }
            }

            // The three-level rungs: the base's natural minors and artificial
            // try against the champion's forcing minors and pass-or-correct
            // majors.
            if champion(agreements) {
                for pattern in ["P* 2♦ - 3♣ -", "P* 2♦ - 3♦ -"] {
                    entries.extend(rows_of(Pattern::node(pattern), opener_over_forcing_minor()));
                }
                entries.extend(rows_of(
                    Pattern::node("P* 2♦ - 3♥ -"),
                    // Pass with hearts, correct to `3♠` with spades.
                    Rules::new()
                        .rule(Bid::new(3, Strain::Spades), 100, len(Suit::Spades, 6..))
                        .rule(Call::Pass, 0, hcp(0..)),
                ));
                entries.extend(rows_of(
                    Pattern::node("P* 2♦ - 3♠ -"),
                    // Pass with spades; with hearts the correction is forced to
                    // the four level.
                    Rules::new()
                        .rule(Bid::new(4, Strain::Hearts), 100, len(Suit::Hearts, 6..))
                        .rule(Call::Pass, 0, hcp(0..)),
                ));
            } else {
                entries.extend(rows_of(Pattern::node("P* 2♦ - 3♣ -"), play_it()));
                entries.extend(rows_of(
                    Pattern::node("P* 2♦ - 3♦ -"),
                    opener_over_three_diamond_try(),
                ));
                entries.extend(rows_of(Pattern::node("P* 2♦ - 3♥ -"), play_it()));
                entries.extend(rows_of(Pattern::node("P* 2♦ - 3♠ -"), play_it()));
            }

            // The four-level machinery, identical in both variants — and
            // registered again behind each overcall, because registration is
            // suffix-exact: `2♦ (2♠) 4♦` reaches no `2♦ - 4♦` key on its own,
            // the node falls to the floor, and the floor sits.  The pre-fix
            // A/B's worst boards were exactly that shape — `4♦` passed out
            // with no diamond suit, −21 IMP a board.
            for prefix in ["P* 2♦ -", "P* 2♦ (2♥)", "P* 2♦ (2♠)"] {
                entries.extend(rows_of(
                    Pattern::node(&format!("{prefix} 4♣ -")),
                    opener_over_four_clubs(),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("{prefix} 4♣ - 4♦ -")),
                    complete_the_transfer(Suit::Hearts),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("{prefix} 4♣ - 4♥ -")),
                    complete_the_transfer(Suit::Spades),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("{prefix} 4♦ -")),
                    opener_over_four_diamonds(),
                ));
            }
            for pattern in ["P* 2♦ - 4♥ -", "P* 2♦ - 4♠ -"] {
                entries.extend(rows_of(Pattern::node(pattern), play_it()));
            }

            // Their double: responder's table gains the redouble, opener answers
            // it, and everything deeper rides the rebase to the quiet auction.
            entries.extend(rows_of(
                Pattern::table("P* 2♦ (X)"),
                responses_doubled(agreements),
            ));
            // Exact nodes, not `Pattern::after`: opener always holds a
            // six-card major here, so the answer tables are total on the
            // reachable population and want to fall through rather than carry
            // a catch-all that would widen an answer's reading to "any hand".
            entries.extend(rows_of(
                Pattern::node("P* 2♦ (X) XX -"),
                if champion(agreements) {
                    worse_major_answers()
                } else {
                    ask_answers()
                },
            ));
            entries.extend(rows_of(Pattern::node("P* 2♦ (X) XX (X)"), opener_escape()));
            entries.push(rebase(Pattern::first(OPEN, "X"), ReplaceNext(Call::Pass)));

            // Their two-level overcall, verbatim — `(2♠)` hole included.
            entries.extend(rows_of(
                Pattern::node("P* 2♦ (2♥)"),
                responses_overcalled(Suit::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 2♦ (2♠)"),
                responses_overcalled(Suit::Spades),
            ));

            // Systems on over their double of every forced continuation: the
            // asks still get their answers, the transfers still complete, the
            // pass-or-corrects still correct.  Without these, the doubled
            // node falls to the floor and the floor sits — the `4♦ (X)`
            // disaster above, one seat over.  The `2♥`/`2♠` pass-or-corrects
            // keep their bespoke doubled tables and stay off this list; the
            // rebases chain, so a second double deeper in the tail resolves
            // through the same keys.
            let champion_corrections: &[&str] = if champion(agreements) {
                &["P* 2♦ - 3♣", "P* 2♦ - 3♥", "P* 2♦ - 3♠"]
            } else {
                &[]
            };
            for key in [
                "P* 2♦ - 2NT",
                "P* 2♦ - 2NT - 3♣",
                "P* 2♦ - 2NT - 3♦",
                "P* 2♦ - 2NT - 3♥",
                "P* 2♦ - 2NT - 3♠",
                "P* 2♦ - 3♦",
                "P* 2♦ - 4♣",
                "P* 2♦ - 4♣ - 4♦",
                "P* 2♦ - 4♣ - 4♥",
                "P* 2♦ - 4♦",
                "P* 2♦ (2♥) 4♣",
                "P* 2♦ (2♥) 4♣ - 4♦",
                "P* 2♦ (2♥) 4♣ - 4♥",
                "P* 2♦ (2♥) 4♦",
                "P* 2♦ (2♠) 4♣",
                "P* 2♦ (2♠) 4♣ - 4♦",
                "P* 2♦ (2♠) 4♣ - 4♥",
                "P* 2♦ (2♠) 4♦",
            ]
            .into_iter()
            .chain(champion_corrections.iter().copied())
            {
                entries.push(rebase(Pattern::first(key, "X"), ReplaceNext(Call::Pass)));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
